extends RefCounted
## Read-only, exact five-degree paired eye diagnostic capture.

const MODES = ["render", "white", "edges"]
const FOCUS = Vector3(0.22, -0.07, 0.16)
const EYE_ZOOM = 0.42
var host
var running := false
var cancel_requested := false


func _init(control) -> void:
	host = control


static func capture_schedule() -> Array[Dictionary]:
	var entries: Array[Dictionary] = []
	for angle in range(0, 181, 5):
		for mode in MODES:
			entries.append({"angle": angle, "mode": mode})
	return entries


static func focus_at(angle: float) -> Vector2:
	var focus := FOCUS.rotated(Vector3.UP, deg_to_rad(-angle))
	return Vector2(focus.x, focus.y)


func source_identities() -> Array[Dictionary]:
	var result: Array[Dictionary] = []
	for pane in host.panes:
		var model: String = pane.actor.definition.model_scene.resource_path
		result.append(
			{
				"definition": pane.source_path,
				"definition_sha256": FileAccess.get_sha256(pane.source_path),
				"model": model,
				"model_sha256": FileAccess.get_sha256(model),
				"normalized_source_face_height": pane.source_face_height
			}
		)
	return result


func lock_controls() -> Array:
	var previous: Array = []
	for control in host.find_children("*", "Control", true, false):
		if control is Button:
			previous.append([control, "disabled", control.disabled])
			if control.text == "右眼5°采集":
				control.text = "停止采集"
			else:
				control.disabled = true
		elif control is Slider:
			previous.append([control, "editable", control.editable])
			control.editable = false
	return previous


func unlock_controls(previous: Array) -> void:
	for entry in previous:
		if is_instance_valid(entry[0]):
			entry[0].set(entry[1], entry[2])
			if entry[0] is Button and entry[0].text == "停止采集":
				entry[0].text = "右眼5°采集"


func write_json(path: String, value: Dictionary) -> bool:
	var file := FileAccess.open(path, FileAccess.WRITE)
	if file == null:
		return false
	file.store_string(JSON.stringify(value, "  "))
	file.close()
	return true


func run_or_cancel() -> void:
	if running:
		cancel_requested = true
		return
	if host.panes.size() != 2:
		return
	running = true
	cancel_requested = false
	var saved: Dictionary = host.state.duplicate(true)
	var controls := lock_controls()
	host.dragging = false
	host.panning = false
	var before := source_identities()
	var identities_valid := true
	for identity in before:
		identities_valid = (
			identities_valid
			and identity.definition_sha256.length() == 64
			and identity.model_sha256.length() == 64
		)
	var folder := (
		"res://artifacts/eye_sweep/eye-" + str(Time.get_unix_time_from_system()).replace(".", "-")
	)
	DirAccess.make_dir_recursive_absolute(folder)
	var report := {
		"schema_version": 1,
		"status": "running",
		"candidate": host.candidate_path,
		"reference": host.reference_path,
		"sources_before": before,
		"captures": [],
		"angle_step_degrees": 5,
		"angle_start": 0,
		"angle_end": 180,
		"view_direction": "negative yaw; frontal screen-right eye (+X)",
		"focus_normalized": [FOCUS.x, FOCUS.y, FOCUS.z],
		"zoom": EYE_ZOOM,
		"expected_captures": 111,
		"occlusion_policy": "preserved; no removal of surrounding geometry"
	}
	host.state.hair = false
	host.state.body = false
	host.state.mouth = 0.0
	host.state.pitch = 0.0
	host.state.light = -45.0
	host.zoom_slider.value = EYE_ZOOM
	write_json(folder + "/manifest.json", report)
	for entry in capture_schedule():
		if not identities_valid:
			report["error"] = "Source identity unavailable"
			break
		if cancel_requested:
			break
		var focus := focus_at(float(entry.angle))
		host.state.yaw = -float(entry.angle)
		host.state.pan_x = focus.x
		host.state.pan_y = focus.y
		host.state.mode = entry.mode
		host.apply()
		host.status.text = (
			"右眼采集 %d/111 · %d° · %s；保持遮挡，不修改模型。"
			% [report.captures.size() + 1, entry.angle, entry.mode]
		)
		await host.get_tree().process_frame
		await RenderingServer.frame_post_draw
		await host.get_tree().process_frame
		await RenderingServer.frame_post_draw
		if cancel_requested:
			break
		var stem := folder + ("/angle_%03d_%s" % [entry.angle, entry.mode])
		var error: Error = host.get_viewport().get_texture().get_image().save_png(stem + ".png")
		if error != OK:
			report["error"] = "PNG save failed: " + str(error)
			break
		var record: Dictionary = entry.duplicate(true)
		record["state"] = host.state.duplicate(true)
		record["image"] = stem + ".png"
		record["sha256"] = FileAccess.get_sha256(stem + ".png")
		record["cameras"] = []
		for pane in host.panes:
			record.cameras.append(
				{
					"position":
					[pane.camera.position.x, pane.camera.position.y, pane.camera.position.z],
					"size": pane.camera.size
				}
			)
		if not write_json(stem + ".json", record):
			report["error"] = "Capture metadata save failed"
			break
		report.captures.append(record)
		if not write_json(folder + "/manifest.json", report):
			report["error"] = "Manifest save failed"
			break
	report["sources_after"] = source_identities()
	report["source_identity_preserved"] = identities_valid and before == report.sources_after
	report.status = (
		"completed"
		if (
			report.captures.size() == 111
			and report.source_identity_preserved
			and not report.has("error")
		)
		else "cancelled" if cancel_requested else "failed"
	)
	write_json(folder + "/manifest.json", report)
	host.state = saved
	host.zoom_slider.value = float(saved.zoom)
	host.apply()
	unlock_controls(controls)
	running = false
	host.status.text = "右眼采集 %s：%d/111，已恢复取景。" % [report.status, report.captures.size()]
	print(
		(
			"EYE_SWEEP_RESULT="
			+ JSON.stringify(
				{
					"folder": folder,
					"status": report.status,
					"captures": report.captures.size(),
					"source_identity_preserved": report.source_identity_preserved
				}
			)
		)
	)
