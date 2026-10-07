extends GutTest

const COMPARE = preload("res://showcase_extensions/head_compare.gd")
const CANDIDATE = preload("res://assets/aster/head_recovery_188/aster_definition.tres")
const REFERENCE = preload("res://assets/aster/head_recovery_900/aster_definition.tres")

func make_comparison():
	var compare = COMPARE.new()
	add_child_autofree(compare)
	compare.configure(CANDIDATE, REFERENCE)
	return compare

func test_close_framing_remains_linked_and_can_reset() -> void:
	var compare = make_comparison()
	await get_tree().process_frame
	assert_eq(compare.panes.size(), 2)
	assert_almost_eq(compare.zoom_slider.min_value, 0.25, 0.0001)
	compare.zoom_slider.value = 0.35
	compare.state.pan_x = 0.25
	compare.state.pan_y = -0.1
	compare.apply()
	for pane in compare.panes:
		assert_almost_eq(pane.camera.size, 0.35, 0.0001)
		assert_almost_eq(pane.camera.position.x, 0.25, 0.0001)
		assert_almost_eq(pane.camera.position.y, -0.1, 0.0001)
	compare.reset_framing()
	assert_almost_eq(compare.state.zoom, 1.8, 0.0001)
	assert_almost_eq(compare.state.pan_x, 0.0, 0.0001)
	assert_almost_eq(compare.state.pan_y, 0.0, 0.0001)
	compare.set_zoom(-2.0)
	assert_almost_eq(compare.state.zoom, 0.25, 0.0001)

func test_exact_ten_degree_steps_preserve_other_view_state() -> void:
	var compare = make_comparison()
	await get_tree().process_frame
	for side in [-1.0, 1.0]:
		compare.set_angle(0.0, 0.0)
		for step in range(1, 10):
			compare.step_angle(side * 10.0)
			assert_almost_eq(compare.state.yaw, side * step * 10.0, 0.0001)
			for pane in compare.panes:
				assert_almost_eq(pane.pivot.rotation_degrees.y, side * step * 10.0, 0.001)
			assert_almost_eq(compare.state.pitch, 0.0, 0.0001)
			assert_almost_eq(compare.state.mouth, 0.0, 0.0001)
			assert_almost_eq(compare.state.light, -45.0, 0.0001)
	compare.set_angle(175.0, 0.0)
	compare.step_angle(10.0)
	assert_almost_eq(compare.state.yaw, -175.0, 0.0001)


func test_saved_lash_collision_evidence_matches_runtime_asset() -> void:
	var root := "res://assets/aster/head_recovery_188/"
	var report = JSON.parse_string(FileAccess.get_file_as_string(root + "generation_report.json"))
	assert_true(report is Dictionary)
	if not report is Dictionary:
		return
	assert_eq(FileAccess.get_sha256(root + "aster_head.glb"), report.get("new_glb_sha256", ""))
	var fit: Dictionary = report.get("eye_accent_surface_fit", {})
	for side in [-1, 1]:
		for label in ["Upper lash ", "Inner forked lash blade ", "Outer lateral eye liner "]:
			var evidence: Dictionary = fit.get(label + str(side), {})
			assert_true(evidence.get("includes_generated_ocular_obstacles", false))
			assert_gte(float(evidence.get("minimum_sampled_obstacle_clearance_m", -1)), 0.000349)
			assert_gt(int(evidence.get("sampled_roots", 0)), 0)
