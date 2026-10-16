extends "res://showcase_extensions/head_review.gd"
## Native test of candidate-only slider application and wireframe synchronization.

var failures: Array[String] = []
var checks := 0


func check(value: bool, message: String) -> void:
	checks += 1
	if not value:
		failures.append(message)


func _ready() -> void:
	super._ready()
	await RenderingServer.frame_post_draw
	var source: String = character_definition.model_scene.resource_path
	var before := FileAccess.get_sha256(source)
	_open_comparison()
	await RenderingServer.frame_post_draw
	var host = _compare_overlay
	check(is_instance_valid(host) and host.panes.size() == 2, "Comparison ready")
	if not is_instance_valid(host) or host.panes.size() != 2:
		return
	var identities := {}
	for pane in host.panes:
		var model: String = pane.actor.definition.model_scene.resource_path
		identities[model] = FileAccess.get_sha256(model)
		check(identities[model].length() == 64, "Actual pane model hash available")
	var capable: bool = host.panes[0].supports_blinks()
	var ui = host.expression_controls
	check(is_instance_valid(ui.row) == capable, "Controls match actual asset capability")
	if capable:
		var reference: Dictionary = host.panes[1].morph_state()
		for mode in ["render", "white", "edges"]:
			host.set_mode(mode)
			for weights in [
				Vector2(0, 0), Vector2(1, 0), Vector2(0, 1), Vector2(.5, .25), Vector2(1, 1)
			]:
				ui.sliders["blink.L"].value = weights.x
				ui.sliders["blink.R"].value = weights.y
				host.set_mouth(.6)
				await RenderingServer.frame_post_draw
				var actual: Dictionary = host.panes[0].morph_state()
				check(is_equal_approx(actual["blink.L"], weights.x), "Left weight")
				check(is_equal_approx(actual["blink.R"], weights.y), "Right weight")
				check(is_equal_approx(actual["MouthOpen"], .6), "Independent mouth")
				check(host.panes[1].morph_state() == reference, "Reference remains neutral")
				var display = host.panes[0].display
				for wire in display._wire_nodes:
					var original: MeshInstance3D = wire.get_parent()
					for index in wire.mesh.get_blend_shape_count():
						check(
							is_equal_approx(
								wire.get_blend_shape_value(index),
								original.get_blend_shape_value(index)
							),
							"Wire morph sync"
						)
		ui.reset()
		check(host.panes[0].morph_state()["blink.L"] == 0, "Reset left")
		check(host.panes[0].morph_state()["blink.R"] == 0, "Reset right")
		check(host.panes[0].morph_state()["MouthOpen"] > .59, "Reset preserves mouth")
		host.state["blink.L"] = .35
		host.state["blink.R"] = .7
		host.apply()
		check(is_equal_approx(ui.sliders["blink.L"].value, .35), "Restored state syncs slider")
		check(is_equal_approx(ui.sliders["blink.R"].value, .7), "Restored right slider")
		var helper = host.EYE_SWEEP.new(host)
		var locked: Array = helper.lock_controls()
		check(
			not ui.sliders["blink.L"].editable and not ui.sliders["blink.R"].editable,
			"Capture locks sliders"
		)
		helper.unlock_controls(locked)
		check(
			ui.sliders["blink.L"].editable and ui.sliders["blink.R"].editable,
			"Capture unlocks sliders"
		)
		var pane = host.panes[0]
		var saved_right: int = pane.blink_indices["blink.R"]
		pane.blink_indices["blink.R"] = -1
		var face: MeshInstance3D = pane.actor.meshes[1]
		face.set_blend_shape_value(pane.blink_indices["blink.L"], .42)
		host.apply()
		check(is_equal_approx(pane.morph_state()["blink.L"], .42), "Partial capability untouched")
		pane.blink_indices["blink.R"] = saved_right
		host.apply()
		var saved: Dictionary = host.state.duplicate(true)
		for kind in ["eye", "sdf"]:
			var capture_root: String = "res://artifacts/" + kind + "_sweep"
			var old_folders := DirAccess.get_directories_at(capture_root)
			if kind == "eye":
				host.start_eye_sweep()
			else:
				host.start_sdf_sweep()
			var captured := {}
			var capture_deadline := Time.get_ticks_msec() + 15000
			while (
				captured.is_empty()
				and host._sweep_running()
				and Time.get_ticks_msec() < capture_deadline
			):
				await get_tree().process_frame
				for folder_name in DirAccess.get_directories_at(capture_root):
					if folder_name not in old_folders:
						var manifest_path := capture_root + "/" + folder_name + "/manifest.json"
						if FileAccess.file_exists(manifest_path):
							var manifest = JSON.parse_string(
								FileAccess.get_file_as_string(manifest_path)
							)
							if (
								manifest is Dictionary
								and not manifest.get("captures", []).is_empty()
							):
								captured = manifest.captures[0]
			check(not captured.is_empty(), "Actual sweep saved a frame")
			if not captured.is_empty():
				var weights: Array = captured.get("actual_morphs", [])
				check(weights.size() == 2, "Sweep actual weights recorded")
				if weights.size() == 2:
					for key in ["blink.L", "blink.R"]:
						check(
							(
								absf(float(weights[0].get(key, -99)) - captured.state.get(key, 0.0))
								< .000001
							),
							"Sweep applied blink matches state"
						)
			var escape := InputEventKey.new()
			escape.pressed = true
			escape.keycode = KEY_ESCAPE
			host._input(escape)
			var deadline := Time.get_ticks_msec() + 15000
			while host._sweep_running() and Time.get_ticks_msec() < deadline:
				await get_tree().process_frame
			check(not host._sweep_running(), "Actual sweep cancelled")
			check(host.state == saved, "Actual sweep restores expression state")
			check(
				is_equal_approx(ui.sliders["blink.L"].value, saved["blink.L"]),
				"Actual restored slider"
			)
		var folder := "res://artifacts/model_comparison"
		var previous := DirAccess.get_files_at(folder)
		host.save_capture()
		while host.capture_pending:
			await get_tree().process_frame
		var found := false
		for filename in DirAccess.get_files_at(folder):
			if filename.ends_with(".json") and filename not in previous:
				var metadata = JSON.parse_string(
					FileAccess.get_file_as_string(folder + "/" + filename)
				)
				if metadata is Dictionary:
					var observed: Array = metadata.get("actual_morphs", [])
					found = observed.size() == 2
					if found:
						for i in range(2):
							var expected: Dictionary = host.panes[i].morph_state()
							found = found and observed[i].size() == expected.size()
							for key in expected:
								found = (
									found
									and (
										absf(float(observed[i].get(key, -99)) - expected[key])
										< 0.000001
									)
								)
		check(found, "Saved actual morph metadata")
		ui.reset()
		host.set_mouth(0.0)
		host.set_mode("render")
	for model in identities:
		check(identities[model] == FileAccess.get_sha256(model), "Actual pane source unchanged")
	check(before == FileAccess.get_sha256(source), "Source identity unchanged")
	var result := {
		"checks": checks,
		"failures": failures,
		"supports_blinks": capable,
		"source": source,
		"source_unchanged": before == FileAccess.get_sha256(source)
	}
	DirAccess.make_dir_recursive_absolute("res://artifacts/blink_controls")
	var file := FileAccess.open("res://artifacts/blink_controls/result.json", FileAccess.WRITE)
	file.store_string(JSON.stringify(result, "  "))
	file.close()
	print("BLINK_CONTROLS_PROBE=" + JSON.stringify(result))
