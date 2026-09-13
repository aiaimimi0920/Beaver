extends Node
## One actor's shared depth storage, camera transactions and metadata publication.
## Producers are synchronized before this frame_pre_draw callback.

const ARRAY = preload("res://addons/npr_characters/runtime/npr_depth_array.gd")
const FLAGS = preload("res://addons/npr_characters/runtime/npr_depth_view_flags.gd")
const TABLE = preload("res://addons/npr_characters/runtime/npr_depth_view_table.gd")
const UPDATE = preload("res://addons/npr_characters/runtime/npr_depth_view_update.gd")
const HISTORY_RESET = preload("res://addons/npr_characters/runtime/npr_taa_history_reset.gd")
const NATIVE_VIEW = preload("res://addons/npr_characters/runtime/npr_native_view.gd")
var depth: Node
var integer_sampling := true
var compact_storage := false
## Logical array payload budgets; source viewports and driver overhead are separate.
var max_payload_bytes := 128 * 1024 * 1024
var max_live_payload_bytes := 256 * 1024 * 1024
var _array: RefCounted
var _flags: RefCounted
var _table: RefCounted
var _entries: Array[Dictionary] = []
var _mutex := Mutex.new()
var _build_result: Dictionary = {}
var _pending := false
var _built_signature: Array = []
var _attempt_signature: Array = []
var _retry_after := 0
var _published_count := -1
var _previous_integer: Array = []
var _generation := 0
var _build_attempts := 0
var _admission_signature: Array = []
var _admission_result: Dictionary = {}
var _admission_pending := false
var _admission_retry_after := 0
var _admission_attempts := 0
var _retaining := false
var _live_result: Dictionary = {}
var _candidate: Dictionary = {}
var _staging := false
var _retained_signature: Array = []
var _preserved_history: Array[Dictionary] = []


func _enter_tree() -> void:
	_mutex.lock()
	_generation += 1
	_build_result = {}
	_admission_result = {}
	_candidate = {}
	_mutex.unlock()
	_array = ARRAY.new()
	_array.compact = compact_storage
	_flags = FLAGS.new()
	_table = TABLE.new()
	_pending = false
	_build_attempts = 0
	_admission_signature.clear()
	_admission_pending = false
	_admission_attempts = 0
	_retaining = false
	_live_result = {}
	_staging = false
	_published_count = -1
	_retry_after = 0
	_built_signature.clear()
	_attempt_signature.clear()
	_previous_integer.clear()
	for material in depth.consumers:
		_previous_integer.append(material.get_shader_parameter("u_npr_depth_integer_sampling"))
		material.set_shader_parameter("u_npr_depth_integer_sampling", integer_sampling)
	if _entries.is_empty():
		_entries = [_entry(depth.get_viewport(), depth, false)]
	else:
		_entries[0].target = weakref(depth.get_viewport())
	_reconnect()
	# On subtree reentry, producer siblings reconnect after this child.
	_reconnect.call_deferred()


func _entry(viewport: Viewport, producer: Node, owned: bool) -> Dictionary:
	return {
		"target": weakref(viewport),
		"producer": producer,
		"owned": owned,
		"camera": null,
		"effect": null,
		"previous": null,
		"installed": null,
		"parents": []
	}


func _reconnect() -> void:
	if RenderingServer.frame_pre_draw.is_connected(_synchronize):
		RenderingServer.frame_pre_draw.disconnect(_synchronize)
	RenderingServer.frame_pre_draw.connect(_synchronize)


func register_view(viewport: Viewport) -> bool:
	if not is_instance_valid(viewport) or viewport == depth.get_viewport():
		return false
	if viewport is NATIVE_VIEW and not viewport.has_consumer():
		return false
	if get_producer(viewport) != null:
		return true
	var owned: bool = not depth._view_producers.has(viewport.get_instance_id())
	var producer: Node = depth.register_view_producer(viewport)
	if producer == null:
		return false
	var entry := _entry(viewport, producer, owned)
	# A texture copied by RD does not itself establish a viewport dependency.
	# Parent the producer viewports under their consumer to enforce child-first draw.
	for source_view in producer.viewports:
		entry.parents.append(source_view.get_parent())
		source_view.reparent(viewport)
	_entries.append(entry)
	if viewport is NATIVE_VIEW:
		viewport.consumer_exiting.connect(_native_exiting.bind(entry))
		_bind_native_sources(entry)
	_reconnect()
	return true


func get_producer(viewport: Viewport) -> Node:
	for entry in _entries:
		if entry.target.get_ref() == viewport and is_instance_valid(entry.producer):
			return entry.producer
	return null


func unregister_view(viewport: Viewport = null) -> void:
	for index in range(_entries.size() - 1, 0, -1):
		var entry := _entries[index]
		if viewport == null or entry.target.get_ref() == viewport:
			_remove_entry(entry)
			_entries.remove_at(index)


func _remove_entry(entry: Dictionary, deferred_free := false) -> void:
	_detach(entry)
	var target: Viewport = entry.target.get_ref()
	if target is NATIVE_VIEW and target.consumer_exiting.is_connected(_native_exiting.bind(entry)):
		target.consumer_exiting.disconnect(_native_exiting.bind(entry))
	if is_instance_valid(entry.producer):
		for index in range(entry.parents.size()):
			var source_view: SubViewport = entry.producer.viewports[index]
			if is_instance_valid(source_view) and is_instance_valid(entry.parents[index]):
				source_view.reparent(entry.parents[index])
		if entry.owned:
			if deferred_free:
				if is_instance_valid(target):
					depth._view_producers.erase(target.get_instance_id())
				entry.producer.queue_free()
			elif is_instance_valid(target):
				depth.unregister_view_producer(target)
			else:
				entry.producer.free()


func _synchronize() -> void:
	for index in range(_entries.size() - 1, 0, -1):
		var target: Viewport = _entries[index].target.get_ref()
		if (
			not is_instance_valid(target)
			or target.is_queued_for_deletion()
			or not is_instance_valid(_entries[index].producer)
		):
			_remove_entry(_entries[index])
			_entries.remove_at(index)
	if not depth.active:
		_suspend()
		return
	var rows: Array[Dictionary] = []
	var inputs: Array[RID] = []
	var sizes: Array[Vector2i] = []
	var signature: Array = [max_payload_bytes, max_live_payload_bytes]
	var active_entries: Array[Dictionary] = []
	for entry in _entries:
		var target: Viewport = entry.target.get_ref()
		if target is NATIVE_VIEW and not target.has_consumer():
			_detach(entry, false)
			continue
		_bind_native_sources(entry)
		var record: Dictionary = entry.producer.get_view_snapshot(rows.size() * 2, rows.size())
		if record.is_empty():
			_detach(entry)
			continue
		active_entries.append(entry)
		rows.append(record)
		signature.append(_consumer_identity(entry))
		for source_view in entry.producer.viewports:
			inputs.append(source_view.get_texture().get_rid())
			sizes.append(source_view.size)
			signature.append([inputs.back(), sizes.back()])
	if rows.is_empty():
		_suspend()
		return
	_mutex.lock()
	var result := _build_result.duplicate(true)
	_mutex.unlock()
	_retaining = false
	if not _consume_build_result(result, signature, active_entries, rows):
		return
	# Replacement preparation preserves any sources still mapped to live storage.
	if signature != _built_signature:
		var retained := _retained_sources(signature, active_entries, _live_result)
		if not retained.is_empty():
			for entry in active_entries:
				if not retained.entries.has(entry):
					_detach(entry)
			_retained_signature = signature.duplicate(true)
			var retry_wait := (
				signature == _attempt_signature and Time.get_ticks_msec() < _retry_after
			)
			if (
				not _pending
				and not retry_wait
				and _admit_growth(signature, inputs, sizes, rows.size())
			):
				_queue_build(signature, inputs, sizes, rows.size(), true)
			_retaining = true
			rows.assign(retained.rows)
			inputs.assign(retained.inputs)
			active_entries.assign(retained.entries)
			signature = _built_signature.duplicate(true)
			result = _live_result.duplicate(true)
		elif _pending:
			_suspend()
			return
	if (
		signature != _built_signature
		or (not result.get("capacity_rejection", "").is_empty() and signature != _attempt_signature)
	):
		_suspend()
		if (
			signature == _attempt_signature
			and (
				not result.get("capacity_rejection", "").is_empty()
				or Time.get_ticks_msec() < _retry_after
			)
		):
			return
		_queue_build(signature, inputs, sizes, rows.size(), false)
		return
	_render_views(rows, active_entries, inputs, result)


func _render_views(
	rows: Array[Dictionary],
	active_entries: Array[Dictionary],
	inputs: Array[RID],
	result: Dictionary
) -> void:
	if _table.update_views(rows) != OK:
		_suspend()
		return
	_publish(rows.size())
	for index in range(active_entries.size()):
		var entry := active_entries[index]
		if rows[index].enabled == 0:
			_detach(entry)
			continue
		_mount(entry, rows[index].ready_index, result.epoch)
		var state: Dictionary = entry.effect.result()
		if (
			state.completed == 0
			or state.error != OK
			or state.completed < entry.get("submitted", 0)
			or entry.producer.requested_this_frame.has(true)
		):
			var sources: Array[RID] = [inputs[index * 2], inputs[index * 2 + 1]]
			var accepted: int = entry.effect.queue_update(sources, PackedInt32Array([0, 1]))
			if accepted > 0:
				entry.submitted = accepted
			entry.effect.enabled = true
		else:
			# Retain valid layers and readiness without dispatching an empty
			# render-thread callback every frame for each static camera.
			entry.effect.enabled = false
	_flush_preserved_history()


func _consume_build_result(
	result: Dictionary, signature: Array, entries: Array[Dictionary], rows: Array[Dictionary]
) -> bool:
	if not _pending:
		return true
	if not _staging:
		_suspend()
	if result.is_empty():
		return _staging
	_pending = false
	if result.error == OK:
		if _staging:
			if signature != _attempt_signature:
				# A completed GPU allocation is not permission to publish an old request.
				_mutex.lock()
				var obsolete := _candidate
				_candidate = {}
				_build_result = {}
				_mutex.unlock()
				if not obsolete.is_empty():
					RenderingServer.call_on_render_thread(obsolete.flags.release)
					RenderingServer.call_on_render_thread(obsolete.array.release)
				_staging = false
				return true
			_commit_candidate(signature, entries, rows)
		_built_signature = _attempt_signature.duplicate(true)
		_live_result = result.duplicate(true)
	else:
		_retry_after = Time.get_ticks_msec() + 500
		if not _staging:
			_live_result = {}
	return true


func _queue_build(
	signature: Array, inputs: Array[RID], sizes: Array[Vector2i], count: int, staged: bool
) -> void:
	_attempt_signature = signature.duplicate(true)
	_mutex.lock()
	_build_result = {}
	_mutex.unlock()
	_pending = true
	_staging = staged
	_build_attempts += 1
	RenderingServer.call_on_render_thread(
		_build.bind(
			inputs.duplicate(),
			sizes.duplicate(),
			count,
			_array,
			_flags,
			_generation,
			max_payload_bytes,
			max_live_payload_bytes,
			false,
			staged
		)
	)


func _commit_candidate(
	signature: Array, entries: Array[Dictionary], rows: Array[Dictionary]
) -> void:
	_mutex.lock()
	var candidate := _candidate
	_candidate = {}
	_mutex.unlock()
	var old_array: RefCounted = _array
	var old_flags: RefCounted = _flags
	_publish(0)
	for entry in _entries:
		var position := entries.find(entry)
		var preserve := false
		if (
			signature == _attempt_signature
			and position >= 0
			and rows[position].enabled != 0
			and entry.effect != null
			and entry.camera == entry.producer.camera
		):
			var offset: int = 2 + entry.effect.view_index * 3
			var current_offset := 2 + position * 3
			var state: Dictionary = entry.effect.result()
			preserve = (
				(
					_built_signature.slice(offset, offset + 3)
					== signature.slice(current_offset, current_offset + 3)
				)
				and state.error == OK
				and state.completed > 0
				and state.completed >= entry.get("submitted", 0)
			)
		_detach(entry, not preserve)
		if preserve:
			_preserved_history.append(entry)
	_array = candidate.array
	_flags = candidate.flags
	RenderingServer.call_on_render_thread(old_flags.release)
	RenderingServer.call_on_render_thread(old_array.release)


func _retained_sources(
	signature: Array, entries: Array[Dictionary], result: Dictionary
) -> Dictionary:
	var count := (_built_signature.size() - 2) / 3
	if (
		count < 1
		or result.get("error", ERR_UNCONFIGURED) != OK
		or result.get("resident_payload_bytes", 0) > max_payload_bytes
		or result.get("resident_payload_bytes", 0) > max_live_payload_bytes
	):
		return {}
	var rows: Array[Dictionary] = []
	var inputs: Array[RID] = []
	var retained: Array[Dictionary] = []
	for index in range(count):
		var offset := 2 + index * 3
		var found := -1
		for candidate in range(entries.size()):
			var start := 2 + candidate * 3
			if signature.slice(start, start + 3) == _built_signature.slice(offset, offset + 3):
				found = candidate
				break
		if found == -1:
			continue
		var entry := entries[found]
		# Table rows are compact, but retained depth/readiness keep old physical slots.
		var record: Dictionary = entry.producer.get_view_snapshot(index * 2, index)
		if record.is_empty():
			continue
		rows.append(record)
		retained.append(entry)
		for source_view in entry.producer.viewports:
			inputs.append(source_view.get_texture().get_rid())
	return {} if rows.is_empty() else {"rows": rows, "inputs": inputs, "entries": retained}


func _admit_growth(
	signature: Array, inputs: Array[RID], sizes: Array[Vector2i], count: int
) -> bool:
	_mutex.lock()
	var result := _admission_result.duplicate(true)
	_mutex.unlock()
	if _admission_pending:
		if result.is_empty():
			return false
		_admission_pending = false
		_admission_retry_after = Time.get_ticks_msec() + 500
	if signature == _admission_signature and not result.is_empty():
		if result.error == OK:
			return true
		if (
			not result.capacity_rejection.is_empty()
			or Time.get_ticks_msec() < _admission_retry_after
		):
			return false
	_admission_signature = signature.duplicate(true)
	_mutex.lock()
	_admission_result = {}
	_mutex.unlock()
	_admission_pending = true
	_admission_attempts += 1
	RenderingServer.call_on_render_thread(
		_build.bind(
			inputs.duplicate(),
			sizes.duplicate(),
			count,
			_array,
			_flags,
			_generation,
			max_payload_bytes,
			max_live_payload_bytes,
			true
		)
	)
	return false


func _build(
	inputs: Array[RID],
	sizes: Array[Vector2i],
	count: int,
	array: RefCounted,
	flags: RefCounted,
	generation: int,
	payload_limit: int,
	live_limit: int,
	preflight_only := false,
	staged := false
) -> void:
	var rd := RenderingServer.get_rendering_device()
	var sources: Array[RID] = []
	var error := OK
	var extent := Vector2i.ZERO
	var pixel_bytes := 0
	var source_format := -1
	for index in range(inputs.size()):
		var source := RenderingServer.texture_get_rd_texture(inputs[index])
		if not rd.texture_is_valid(source):
			error = ERR_UNCONFIGURED
			break
		var format := rd.texture_get_format(source)
		if Vector2i(format.width, format.height) != sizes[index]:
			error = ERR_UNCONFIGURED
			break
		# Capacity accounting is meaningful only for a valid homogeneous copy job.
		# Keep unready/incompatible source failures on the existing retry path.
		if (
			not ARRAY.PIXEL_BYTES.has(format.format)
			or format.texture_type != RenderingDevice.TEXTURE_TYPE_2D
			or format.samples != RenderingDevice.TEXTURE_SAMPLES_1
			or not (format.usage_bits & RenderingDevice.TEXTURE_USAGE_CAN_COPY_FROM_BIT)
			or (source_format != -1 and source_format != format.format)
		):
			error = ERR_INVALID_PARAMETER
			break
		source_format = format.format
		sources.append(source)
		extent = Vector2i(maxi(extent.x, format.width), maxi(extent.y, format.height))
		pixel_bytes = maxi(pixel_bytes, ARRAY.PIXEL_BYTES.get(format.format, 0))
	var requested_bytes: int = (
		extent.x * extent.y * inputs.size() * (4 if array.compact else pixel_bytes)
	)
	var live_bytes: int = array.resident_payload_bytes + requested_bytes
	var max_layers := rd.limit_get(RenderingDevice.LIMIT_MAX_TEXTURE_ARRAY_LAYERS)
	var rejection := ""
	if error == OK:
		if inputs.size() > max_layers:
			rejection = "device_layers"
		elif count > 4096:
			rejection = "metadata_rows"
		elif requested_bytes > payload_limit:
			rejection = "payload_budget"
		elif live_bytes > live_limit:
			rejection = "replacement_overlap_budget"
		if not rejection.is_empty():
			error = ERR_OUT_OF_MEMORY
	var previous_array: RefCounted = array
	var next: Dictionary = {}
	if error == OK and staged and not preflight_only:
		array = ARRAY.new()
		array.compact = previous_array.compact
		array.allocation_count = previous_array.allocation_count
		array.copy_count = previous_array.copy_count
		array.peak_live_payload_bytes = previous_array.peak_live_payload_bytes
		var previous_epoch: int = flags.epoch
		flags = FLAGS.new()
		flags.epoch = previous_epoch
		next = {"array": array, "flags": flags}
	if error == OK and not preflight_only:
		array.max_payload_bytes = payload_limit
		array.max_live_payload_bytes = (
			live_limit - previous_array.resident_payload_bytes if staged else live_limit
		)
		error = array.replace(sources)
	if error == OK and not preflight_only:
		error = flags.reset(count)
	if not next.is_empty() and error == OK:
		array.max_live_payload_bytes = live_limit
		array.peak_live_payload_bytes = maxi(array.peak_live_payload_bytes, live_bytes)
	_mutex.lock()
	if generation == _generation:
		var report := {
			"error": error,
			"epoch": flags.epoch,
			"allocations": array.allocation_count,
			"copies": array.copy_count,
			"capacity_rejection": rejection,
			"requested_layers": inputs.size(),
			"device_max_layers": max_layers,
			"requested_payload_bytes": requested_bytes,
			"requested_live_payload_bytes": live_bytes,
			"resident_payload_bytes": array.resident_payload_bytes,
			"peak_live_payload_bytes": array.peak_live_payload_bytes
		}
		if preflight_only:
			_admission_result = report
		else:
			_build_result = report
			if error == OK and not next.is_empty():
				_candidate = next
				next = {}
	_mutex.unlock()
	if not next.is_empty():
		next.flags.release()
		next.array.release()


func _publish(count: int) -> void:
	if _published_count == count:
		return
	_published_count = count
	for index in range(depth.consumers.size()):
		var material: ShaderMaterial = depth.consumers[index]
		material.set_shader_parameter("u_npr_depth_view_count", count)
		material.set_shader_parameter("u_npr_depth_view_channel", 1 if index == 1 else 0)
		material.set_shader_parameter("u_npr_depth_array_base", -1)
		material.set_shader_parameter("u_npr_depth_array_base_secondary", -1)
		material.set_shader_parameter("u_npr_depth_array", _array.resource if count > 0 else null)
		material.set_shader_parameter("u_npr_depth_array_compact", count > 0 and _array.compact)
		material.set_shader_parameter(
			"u_npr_depth_view_ready", _flags.resource if count > 0 else null
		)
		material.set_shader_parameter(
			"u_npr_depth_view_table", _table.texture if count > 0 else null
		)


func _suspend() -> void:
	_publish(0)
	for entry in _entries:
		_detach(entry)
	_flush_preserved_history()


func _flush_preserved_history() -> void:
	# A retained camera that did not remount must discard history before fallback.
	for entry in _preserved_history:
		if is_instance_valid(entry.producer):
			_reset_history(entry)
	_preserved_history.clear()


func _mount(entry: Dictionary, index: int, epoch: int) -> void:
	var camera: Camera3D = entry.producer.camera
	if (
		entry.camera == camera
		and entry.effect != null
		and entry.effect.target == _array
		and entry.effect.flags == _flags
		and entry.effect.view_index == index
		and entry.effect.view_epoch == epoch
		and camera.compositor != null
		and camera.compositor.compositor_effects.has(entry.effect)
	):
		return
	_detach(entry)
	var effect := UPDATE.new()
	effect.target = _array
	effect.reset_taa_on_ready = true
	# Equivalent storage changes are not a loss/recovery of this view's depth.
	# A failed first copy still transitions true -> false and rejects old history.
	effect._last_success = _preserved_history.has(entry)
	_preserved_history.erase(entry)
	effect.flags = _flags
	effect.view_index = index
	effect.view_epoch = epoch
	effect.destinations = PackedInt32Array([index * 2, index * 2 + 1])
	var effects: Array[CompositorEffect] = []
	entry.previous = camera.compositor
	if camera.compositor != null:
		effects.assign(camera.compositor.compositor_effects)
	effects.append(effect)
	var compositor := Compositor.new()
	compositor.compositor_effects = effects
	camera.compositor = compositor
	entry.camera = camera
	entry.installed = compositor
	entry.effect = effect
	entry.submitted = 0
	_publish_native_compositor(entry)


func _detach(entry: Dictionary, reset_history := true) -> void:
	# A destroyed target frees its camera before the next frame_pre_draw purge.
	# Validate the Variant before assigning it to a typed Object local.
	var camera: Camera3D = entry.camera if is_instance_valid(entry.camera) else null
	var effect: CompositorEffect = entry.effect
	if effect == null:
		return
	if is_instance_valid(camera) and camera.compositor != null:
		var current := camera.compositor
		if current.compositor_effects.has(effect):
			var remaining: Array[CompositorEffect] = []
			for item in current.compositor_effects:
				if item != effect:
					remaining.append(item)
			var previous: Array[CompositorEffect] = []
			if entry.previous != null:
				previous.assign(entry.previous.compositor_effects)
			if current == entry.installed and remaining == previous:
				camera.compositor = entry.previous
			else:
				var retained := Compositor.new()
				retained.compositor_effects = remaining
				camera.compositor = retained
	RenderingServer.call_on_render_thread(effect.release)
	if reset_history:
		_reset_history(entry)
	_publish_native_compositor(entry)
	entry.effect = null
	entry.camera = null
	entry.previous = null
	entry.installed = null


func _consumer_identity(entry: Dictionary) -> Variant:
	var target: Viewport = entry.target.get_ref()
	if target is NATIVE_VIEW:
		return [target.get_instance_id(), target.consumer_viewport, target.consumer_camera]
	return target.get_instance_id()


func _bind_native_sources(entry: Dictionary) -> void:
	var target: Viewport = entry.target.get_ref()
	if not target is NATIVE_VIEW or not target.consumer_viewport.is_valid():
		return
	var identity: Variant = _consumer_identity(entry)
	if entry.get("native_identity") == identity:
		return
	for source_view in entry.producer.viewports:
		RenderingServer.viewport_set_parent_viewport(
			source_view.get_viewport_rid(), target.consumer_viewport
		)
	entry.native_identity = identity


func _native_exiting(entry: Dictionary) -> void:
	_detach(entry, false)
	if is_instance_valid(entry.producer):
		for source_view in entry.producer.viewports:
			if is_instance_valid(source_view):
				RenderingServer.viewport_set_parent_viewport(source_view.get_viewport_rid(), RID())
	entry.erase("native_identity")


func _publish_native_compositor(entry: Dictionary) -> void:
	var target: Viewport = entry.target.get_ref()
	if target is NATIVE_VIEW:
		target.publish_compositor()


func _reset_history(entry: Dictionary) -> void:
	var target: Viewport = entry.target.get_ref()
	if target is NATIVE_VIEW:
		target.reset_history()
	else:
		HISTORY_RESET.schedule(entry.producer.camera)


func status() -> Dictionary:
	_mutex.lock()
	var value := (_live_result if _build_result.is_empty() else _build_result).duplicate(true)
	if _retaining and not (_staging and _attempt_signature == _retained_signature):
		value.merge(_admission_result, true)
	_mutex.unlock()
	value.merge(
		{
			"pending": _pending,
			"published_views": maxi(_published_count, 0),
			"registered_views": _entries.size(),
			"build_attempts": _build_attempts,
			"admission_attempts": _admission_attempts,
			"admission_pending": _admission_pending,
			"retained_views": maxi(_published_count, 0) if _retaining else 0
		}
	)
	return value


func _exit_tree() -> void:
	_mutex.lock()
	_generation += 1
	var candidate := _candidate
	_candidate = {}
	_mutex.unlock()
	RenderingServer.frame_pre_draw.disconnect(_synchronize)
	_suspend()
	for index in range(depth.consumers.size()):
		depth.consumers[index].set_shader_parameter(
			"u_npr_depth_integer_sampling", _previous_integer[index]
		)
	RenderingServer.call_on_render_thread(_flags.release)
	RenderingServer.call_on_render_thread(_array.release)
	if not candidate.is_empty():
		RenderingServer.call_on_render_thread(candidate.flags.release)
		RenderingServer.call_on_render_thread(candidate.array.release)


func _notification(what: int) -> void:
	if what == NOTIFICATION_PREDELETE:
		for entry in _entries:
			_detach(entry)
		# A child exit callback runs while its parent's child list is locked.
		# Never free sibling producers synchronously from that callback.
		for index in range(_entries.size() - 1, 0, -1):
			_remove_entry(_entries[index], true)
		_entries.clear()
