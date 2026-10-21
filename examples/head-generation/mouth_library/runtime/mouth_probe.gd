extends "res://showcase_extensions/researched_mouth_review.gd"
## Native rendered evidence through the same public manual controller API.

const SCHEDULE_PATH := "res://showcase_extensions/mouth_capture_schedule.json"


func _ready() -> void:
	super._ready()
	if not mouth_ready:
		return
	set_process_input(false)
	set_process_unhandled_input(false)
	set_process_unhandled_key_input(false)
	_lock_controls(self)
	var parsed = JSON.parse_string(FileAccess.get_file_as_string(SCHEDULE_PATH))
	if not parsed is Array or parsed.size() != 92:
		push_error("MOUTH_PROBE_BAD_SCHEDULE")
		return
	var left := mouth_face.find_blend_shape_by_name("blink.L")
	var right := mouth_face.find_blend_shape_by_name("blink.R")
	if left < 0 or right < 0:
		push_error("MOUTH_PROBE_MISSING_BLINK")
		return
	var source: String = character_definition.model_scene.resource_path
	var before := FileAccess.get_sha256(source)
	var folder := "res://artifacts/mouth_motion/probe-" + str(Time.get_unix_time_from_system()).replace(".", "-")
	if DirAccess.make_dir_recursive_absolute(folder) != OK:
		push_error("MOUTH_PROBE_FOLDER_FAILED")
		return
	var captures: Array[Dictionary] = []
	for item in parsed:
		preview.meshes[0].visible = false
		preview.meshes[2].visible = false
		camera.set_orthogonal(_head_view_size, 0.03, 1000.0)
		_head_angle(item.yaw, item.pitch)
		_set_display_mode(item.mode)
		if not set_mouth_state(item.mouth):
			push_error("MOUTH_PROBE_INVALID_STATE")
			return
		mouth_face.set_blend_shape_value(left, item.blink[0])
		mouth_face.set_blend_shape_value(right, item.blink[1])
		display.sync_morphs()
		_status.text = "嘴部状态 %d / 92（非音素）" % (captures.size() + 1)
		await get_tree().process_frame
		await RenderingServer.frame_post_draw
		await get_tree().process_frame
		await RenderingServer.frame_post_draw
		var path := folder + "/frame_%03d.png" % captures.size()
		if get_viewport().get_texture().get_image().save_png(path) != OK:
			push_error("MOUTH_PROBE_CAPTURE_FAILED")
			return
		captures.append({
			"requested": item,
			"actual_mouth": actual_mouth_state(),
			"actual_blink": [mouth_face.get_blend_shape_value(left), mouth_face.get_blend_shape_value(right)],
			"actual_view": {"mode": display.mode, "projection": camera.projection,
				"size": camera.size, "yaw": turntable.rotation_degrees.y, "pitch": _camera_pitch,
				"camera_pitch": rad_to_deg(atan2(camera.global_transform.basis.z.y, camera.global_transform.basis.z.z)),
				"viewport_size": [get_viewport().size.x, get_viewport().size.y],
				"face_visible": mouth_face.is_visible_in_tree(), "body_visible": preview.meshes[0].visible,
				"hair_visible": preview.meshes[2].visible},
			"image": path, "sha256": FileAccess.get_sha256(path)
		})
	set_mouth_state({})
	mouth_face.set_blend_shape_value(left, 0.0)
	mouth_face.set_blend_shape_value(right, 0.0)
	_set_display_mode("render")
	_reset_view()
	var report := {"source": source, "source_before": before, "source_after": FileAccess.get_sha256(source),
		"schedule_sha256": FileAccess.get_sha256(SCHEDULE_PATH),
		"controller_sha256": FileAccess.get_sha256("res://showcase_extensions/researched_mouth_review.gd"),
		"probe_sha256": FileAccess.get_sha256(get_script().resource_path),
		"captures": captures, "expected": parsed.size(), "speech_aligned": false}
	var file := FileAccess.open(folder + "/manifest.json", FileAccess.WRITE)
	if file == null:
		push_error("MOUTH_PROBE_MANIFEST_FAILED")
		return
	file.store_string(JSON.stringify(report, "\t"))
	file.close()
	_status.text = "92 帧嘴部状态已捕获；美术与语音验收未完成"
	print("MOUTH_MOTION_PROBE=" + folder + "/manifest.json")


func _lock_controls(node: Node) -> void:
	if node is Control:
		node.mouse_filter = Control.MOUSE_FILTER_IGNORE
	if node is BaseButton:
		node.disabled = true
	if node is Slider:
		node.editable = false
	for child in node.get_children():
		_lock_controls(child)
