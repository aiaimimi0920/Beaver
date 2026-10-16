extends "res://showcase_extensions/head_review.gd"
## Temporary native GUI control regression. Restore the ordinary scene afterward.

const PANE = preload("res://showcase_extensions/head_compare_pane.gd")
const SWEEP = preload("res://showcase_extensions/head_sdf_sweep.gd")
var failures: Array[String] = []
var checks := 0
var late_cancel: Dictionary = {}


func check(value: bool, message: String) -> void:
	checks += 1
	if not value:
		failures.append(message)


func _ready() -> void:
	super._ready()
	await RenderingServer.frame_post_draw
	check(not PANE.supports_sdf_controls(null), "Null material")
	check(not PANE.supports_sdf_controls(ShaderMaterial.new()), "Null shader")
	for declaration in [
		"uniform bool u_npr_sdf360_enabled;",
		"uniform float u_npr_sdf360_enabled; uniform bool u_npr_sdf360_debug;",
		"uniform bool u_npr_sdf360_enabled; uniform bool u_npr_sdf360_debug;"
	]:
		var material := ShaderMaterial.new()
		material.shader = Shader.new()
		material.shader.code = "shader_type spatial; " + declaration
		check(
			(
				PANE.supports_sdf_controls(material)
				== declaration.begins_with("uniform bool u_npr_sdf360_enabled; uniform")
			),
			"Uniform contract"
		)
	for path in [
		"res://showcase_extensions/sdf360/face_base.gdshader",
		"res://addons/npr_character_frame/shaders/face/sdf360/face_base.gdshader"
	]:
		var material := ShaderMaterial.new()
		material.shader = load(path)
		check(PANE.supports_sdf_controls(material), "Supported shader " + path)
	_open_comparison()
	await RenderingServer.frame_post_draw
	check(
		is_instance_valid(_compare_overlay) and _compare_overlay.panes.size() == 2,
		"Actual comparison created"
	)
	if not is_instance_valid(_compare_overlay):
		finish([])
		return
	var host = _compare_overlay
	var states: Array[Dictionary] = []
	for enabled in [false, true, false, true]:
		for debug in [false, true]:
			host.state.mode = "render"
			host.state.sdf360 = enabled
			host.state.sdf_debug = debug
			host.apply()
			await RenderingServer.frame_post_draw
			for pane in host.panes:
				var observed: Dictionary = pane.sdf_control_state()
				check(observed.get("available", false), "Actual shader capable")
				check(
					observed.get("enabled") == enabled and observed.get("debug") == debug,
					"Actual uniform roundtrip"
				)
				check(
					(
						observed.get("atlas_bound", false)
						and observed.get("render_material_active", false)
					),
					"Actual texture/material bound"
				)
				states.append(observed)
	host.state.mode = "white"
	host.apply()
	for pane in host.panes:
		check(
			not pane.sdf_control_state().get("render_material_active", true),
			"White mode is not SDF evidence"
		)
	host.state.mode = "render"
	host.state.sdf360 = true
	host.state.sdf_debug = false
	host.apply()
	var actual: Dictionary = host.panes[0].sdf_control_state()
	check(
		SWEEP.controls_match([actual, {"available": false}], false),
		"Unsupported reference explicitly permitted"
	)
	check(
		not SWEEP.controls_match([{"available": false}, actual], false),
		"Unsupported candidate rejected"
	)
	var missing := actual.duplicate()
	missing.atlas_bound = false
	check(not SWEEP.controls_match([missing, actual], false), "Missing atlas rejected")
	var helper = preload("res://showcase_extensions/head_eye_sweep.gd").new(host)
	var locked: Array = helper.lock_controls("光照采集")
	for control in host.find_children("*", "Button", true, false):
		if control.text == "停止采集":
			check(not control.disabled, "SDF cancel button reachable")
		elif control.text == "右眼5°采集":
			check(control.disabled, "Other sweep disabled")
	helper.unlock_controls(locked)
	host.sdf_sweep = SWEEP.new(host)
	host.sdf_sweep.running = true
	var prior: Dictionary = host.state.duplicate(true)
	var motion := InputEventMouseMotion.new()
	motion.relative = Vector2(100, 100)
	host.dragging = true
	host.pane_input(motion)
	check(host.state == prior, "Drag blocked during SDF sweep")
	var escape := InputEventKey.new()
	escape.pressed = true
	escape.keycode = KEY_ESCAPE
	host._input(escape)
	check(host.sdf_sweep.cancel_requested and is_instance_valid(host), "Escape cancels sweep")
	host.sdf_sweep.running = false
	host.dragging = false
	host.panning = false
	var before_folders := DirAccess.get_directories_at("res://artifacts/sdf_sweep")
	host.start_sdf_sweep()
	while host.sdf_sweep.running:
		await get_tree().process_frame
		if host.status.text.contains("250 / 250"):
			host._input(escape)
	var folders := DirAccess.get_directories_at("res://artifacts/sdf_sweep")
	var new_folders: Array[String] = []
	for folder in folders:
		if folder not in before_folders:
			new_folders.append(folder)
	check(new_folders.size() == 1, "Exactly one owned sweep folder")
	if new_folders.size() != 1:
		finish(states)
		return
	var result = JSON.parse_string(
		FileAccess.get_file_as_string(
			"res://artifacts/sdf_sweep/" + new_folders[0] + "/manifest.json"
		)
	)
	check(result is Dictionary, "Late cancellation manifest exists")
	if result is Dictionary:
		check(result.identity_preserved, "Cancelled sweep preserves source identity")
		late_cancel = {
			"status": result.status,
			"captures": result.captures.size(),
			"identity_preserved": result.identity_preserved
		}
		check(
			result.status == "cancelled" and result.captures.size() == 249,
			"Final awaited frame cancellation never completes"
		)
	finish(states)


func finish(states: Array) -> void:
	var report := {
		"checks": checks, "failures": failures, "actual_states": states, "late_cancel": late_cancel
	}
	DirAccess.make_dir_recursive_absolute("res://artifacts/head_sdf_probe")
	var file := FileAccess.open("res://artifacts/head_sdf_probe/result.json", FileAccess.WRITE)
	file.store_string(JSON.stringify(report, "  "))
	file.close()
	print("HEAD_SDF_CONTROLS_PROBE=" + JSON.stringify(report))
