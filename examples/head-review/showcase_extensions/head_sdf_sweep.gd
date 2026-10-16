extends RefCounted
## Capture actual directional lighting, preserving source geometry and restoring state.
var host
var running := false
var cancel_requested := false


func _init(control) -> void:
	host = control


func run_or_cancel() -> void:
	if running:
		cancel_requested = true
		return
	running = true
	cancel_requested = false
	var helper = preload("res://showcase_extensions/head_eye_sweep.gd").new(host)
	host.dragging = false
	host.panning = false
	var saved: Dictionary = host.state.duplicate(true)
	var controls: Array = helper.lock_controls("光照采集")
	var before: Array = helper.source_identities()
	var folder := (
		"res://artifacts/sdf_sweep/sdf-" + str(Time.get_unix_time_from_system()).replace(".", "-")
	)
	DirAccess.make_dir_recursive_absolute(folder)
	var report := {"status": "running", "source_before": before, "captures": [], "expected": 250}
	var schedule: Array[Dictionary] = []
	for elevation in [-90.0, -60.0, 0.0, 60.0, 90.0]:
		for angle in range(-180, 181, 15):
			for debug in [false, true]:
				schedule.append({"light": float(angle), "elevation": elevation, "debug": debug})
	host.state.mode = "render"
	host.state.zoom = 1.5
	host.state.yaw = 0.0
	host.state.pitch = 0.0
	host.state.pan_x = 0.0
	host.state.pan_y = 0.0
	host.state.sdf360 = true
	for item in schedule:
		if cancel_requested:
			break
		host.state.light = item.light
		host.state.elevation = item.elevation
		host.state.sdf_debug = item.debug
		host.apply()
		var applied: Array[Dictionary] = []
		for pane in host.panes:
			applied.append(pane.sdf_control_state())
		if not controls_match(applied, item.debug):
			report["error"] = "Candidate SDF material unavailable or requested state not applied"
			break
		host.status.text = "SDF 光照采集 %d / 250" % (report.captures.size() + 1)
		await host.get_tree().process_frame
		await RenderingServer.frame_post_draw
		await host.get_tree().process_frame
		await RenderingServer.frame_post_draw
		if cancel_requested:
			break
		applied.clear()
		for pane in host.panes:
			applied.append(pane.sdf_control_state())
		if not controls_match(applied, item.debug):
			report["error"] = "Applied SDF material state changed before capture"
			break
		var path := folder + "/frame_%03d.png" % report.captures.size()
		if host.get_viewport().get_texture().get_image().save_png(path) != OK:
			report["error"] = "Screenshot write failed"
			break
		report.captures.append(
			{
				"state": host.state.duplicate(true),
				"applied_sdf_controls": applied,
				"actual_morphs": [host.panes[0].morph_state(), host.panes[1].morph_state()],
				"image": path,
				"sha256": FileAccess.get_sha256(path)
			}
		)
		helper.write_json(folder + "/manifest.json", report)
	report["source_after"] = helper.source_identities()
	report["identity_preserved"] = before == report.source_after
	report.status = (
		"completed"
		if report.captures.size() == 250 and report.identity_preserved and not cancel_requested
		else (
			"cancelled"
			if cancel_requested and report.identity_preserved and not report.has("error")
			else "failed"
		)
	)
	helper.write_json(folder + "/manifest.json", report)
	host.state = saved
	host.apply()
	helper.unlock_controls(controls)
	running = false
	host.status.text = "SDF 光照采集 %s：%d / 250" % [report.status, report.captures.size()]
	print("SDF_SWEEP_RESULT=" + JSON.stringify({"folder": folder, "status": report.status}))


static func controls_match(applied: Array[Dictionary], debug: bool) -> bool:
	if applied.size() != 2 or not applied[0].get("available", false):
		return false
	for value in applied:
		if not value.get("available", false):
			continue
		if not value.get("enabled", false) or value.get("debug") != debug:
			return false
		if not value.get("atlas_bound", false) or not value.get("render_material_active", false):
			return false
	return true
