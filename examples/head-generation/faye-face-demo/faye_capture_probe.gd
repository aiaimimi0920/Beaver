extends "res://showcase_extensions/researched_mouth_review.gd"
## Nine source-bound views of one generated face using the existing NPR stage.

const CAPTURE_VIEW = preload("res://showcase_extensions/ear_capture_view.gd")
const SCHEDULE := "res://showcase_extensions/faye_capture_schedule.json"
const BINDINGS := [
	"res://showcase/faye_face_capture.tscn",
	"res://showcase_extensions/faye_capture_probe.gd", SCHEDULE,
	"res://showcase_extensions/ear_capture_view.gd",
	"res://showcase_extensions/researched_mouth_review.gd",
	"res://showcase_extensions/head_review.gd",
	"res://showcase_extensions/head_stage.gd",
	"res://showcase_extensions/head_display.gd",
	"res://showcase_extensions/head_edges.gdshader",
	"res://showcase_extensions/head_facets.gdshader",
	"res://addons/npr_character_frame/showcase/wardrobe.gd",
]


func _bindings() -> Dictionary:
	var result := {}
	for path in BINDINGS:
		result[path] = FileAccess.get_sha256(path)
	return result


func _valid_hashes(values: Dictionary) -> bool:
	for value in values.values():
		if not value is String or value.length() != 64:
			push_error("FAYE_CAPTURE_MISSING_PROVENANCE")
			return false
	return not values.is_empty()


func _ready() -> void:
	super._ready()
	if not mouth_ready or not is_instance_valid(preview) or not preview.initialized:
		push_error("FAYE_CAPTURE_NOT_READY")
		return
	var args := OS.get_cmdline_user_args()
	if args.is_empty() or character_definition.resource_path != args[0]:
		push_error("FAYE_CAPTURE_EXPLICIT_DEFINITION_REQUIRED")
		return
	var entry_scene: String = get_tree().current_scene.scene_file_path
	var entry_sha := FileAccess.get_sha256(entry_scene)
	if entry_sha.length() != 64 or entry_sha != FileAccess.get_sha256("res://showcase/faye_face_capture.tscn"):
		push_error("FAYE_CAPTURE_ENTRY_SCENE_MISMATCH")
		return
	set_process_input(false)
	set_process_unhandled_input(false)
	set_process_unhandled_key_input(false)
	_lock_controls(self)
	CAPTURE_VIEW.prepare(self)
	var schedule = JSON.parse_string(FileAccess.get_file_as_string(SCHEDULE))
	if not schedule is Array or schedule.size() != 9:
		push_error("FAYE_CAPTURE_INVALID_SCHEDULE")
		return
	var source: String = character_definition.model_scene.resource_path
	var definition: String = character_definition.resource_path
	var report_path := definition.get_base_dir() + "/generation_report.json"
	var before := {"model": FileAccess.get_sha256(source),
		"definition": FileAccess.get_sha256(definition), "report": FileAccess.get_sha256(report_path)}
	var bindings := _bindings()
	if not _valid_hashes(before) or not _valid_hashes(bindings):
		return
	var left := mouth_face.find_blend_shape_by_name("blink.L")
	var right := mouth_face.find_blend_shape_by_name("blink.R")
	if left < 0 or right < 0:
		push_error("FAYE_CAPTURE_MISSING_BLINKS")
		return
	var folder := "res://artifacts/faye_face/capture-" + str(Time.get_unix_time_from_system()).replace(".", "-")
	if DirAccess.make_dir_recursive_absolute(folder) != OK:
		push_error("FAYE_CAPTURE_DIRECTORY_FAILED")
		return
	var captures: Array[Dictionary] = []
	for item in schedule:
		_head_angle(item.yaw, item.pitch)
		camera.set_orthogonal(_head_view_size * item.scale, 0.03, 1000.0)
		_set_display_mode(item.mode)
		if not set_mouth_state(item.mouth):
			push_error("FAYE_CAPTURE_INVALID_MOUTH")
			return
		mouth_face.set_blend_shape_value(left, 0.0)
		mouth_face.set_blend_shape_value(right, 0.0)
		preview.meshes[0].visible = false
		preview.meshes[2].visible = false
		display.sync_morphs()
		var expected_target := [_target.x, _target.y, _target.z]
		var expected_position := [camera.position.x, camera.position.y, camera.position.z]
		await get_tree().process_frame
		await RenderingServer.frame_post_draw
		await get_tree().process_frame
		await RenderingServer.frame_post_draw
		var path := folder + "/frame_%03d.png" % captures.size()
		if get_viewport().get_texture().get_image().save_png(path) != OK:
			push_error("FAYE_CAPTURE_IMAGE_FAILED")
			return
		var sidecar := FileAccess.open(path + ".import", FileAccess.WRITE)
		if sidecar == null:
			push_error("FAYE_CAPTURE_IMPORT_METADATA_FAILED")
			return
		sidecar.store_string("[remap]\n\nimporter=\"skip\"\n")
		sidecar.close()
		captures.append({"requested": item, "actual_mouth": actual_mouth_state(),
			"actual_blink": [mouth_face.get_blend_shape_value(left), mouth_face.get_blend_shape_value(right)],
			"expected_target": expected_target, "expected_position": expected_position,
			"actual_view": {"target": [_target.x, _target.y, _target.z],
				"position": [camera.position.x, camera.position.y, camera.position.z], "mode": display.mode, "projection": camera.projection,
				"size": camera.size, "yaw": turntable.rotation_degrees.y, "pitch": _camera_pitch,
				"camera_pitch": rad_to_deg(atan2(camera.global_transform.basis.z.y, camera.global_transform.basis.z.z)),
				"viewport_size": [get_viewport().size.x, get_viewport().size.y],
				"face_visible": mouth_face.is_visible_in_tree(), "body_visible": preview.meshes[0].visible,
				"hair_visible": preview.meshes[2].visible}, "image": path, "sha256": FileAccess.get_sha256(path)})
	var after := {"model": FileAccess.get_sha256(source),
		"definition": FileAccess.get_sha256(definition), "report": FileAccess.get_sha256(report_path)}
	var bindings_after := _bindings()
	var entry_sha_after := FileAccess.get_sha256(entry_scene)
	if before != after or bindings != bindings_after or entry_sha != entry_sha_after:
		push_error("FAYE_CAPTURE_PROVENANCE_CHANGED")
		return
	var manifest := {"schema_version": 1, "source": source, "definition": definition,
		"entry_scene_at_capture": entry_scene, "entry_scene_sha256": entry_sha_after,
		"entry_scene_sha256_before": entry_sha, "entry_scene_sha256_after": entry_sha_after,
		"report": report_path, "source_before": before, "source_after": after,
		"bindings_before": bindings, "bindings_after": bindings_after, "base_view_size": _head_view_size,
		"captures": captures, "expected": 9, "speech_aligned": false, "continuous_animation": false}
	var file := FileAccess.open(folder + "/manifest.json", FileAccess.WRITE)
	if file == null:
		push_error("FAYE_CAPTURE_MANIFEST_FAILED")
		return
	file.store_string(JSON.stringify(manifest, "\t"))
	file.close()
	set_mouth_state({})
	_set_display_mode("render")
	_reset_view()
	camera.set_orthogonal(_head_view_size, 0.03, 1000.0)
	print("FAYE_CAPTURE_PROBE=" + folder + "/manifest.json")


func _lock_controls(node: Node) -> void:
	if node is Control:
		node.mouse_filter = Control.MOUSE_FILTER_IGNORE
	if node is BaseButton:
		node.disabled = true
	if node is Slider:
		node.editable = false
	for child in node.get_children():
		_lock_controls(child)
