extends "res://showcase_extensions/head_review.gd"
## Native regression for the dedicated face-review presentation.

var scope_checks: Array[Dictionary] = []


func verify(value: bool, label: String) -> void:
	scope_checks.append({"check": label, "passed": value})


func _ready() -> void:
	super._ready()
	await RenderingServer.frame_post_draw
	var source: String = character_definition.model_scene.resource_path
	var before := FileAccess.get_sha256(source)
	verify(preview.initialized, "Production actor initialized")
	verify(preview.meshes[1].visible, "Face visible")
	verify(not preview.meshes[0].visible, "Frozen Body hidden")
	verify(not preview.meshes[2].visible, "Frozen Hair hidden")
	for name in ["Navigation", "Save"]:
		verify(not get_node(name).visible, name + " hidden")
	verify(get_node("Save").disabled, "Save remains disabled")
	for name in ["FullView", "HalfView", "HairToggle"]:
		verify(not find_child(name, true, false).visible, name + " hidden")
	verify(find_child("ModelCompare", true, false).is_visible_in_tree(), "Comparison accessible")
	verify(find_child("ResetView", true, false).is_visible_in_tree(), "Camera reset accessible")
	var slider_count := 0
	for control in _content.get_children():
		if control is HSlider:
			slider_count += 1
	verify(slider_count == 2, "Only face light and outline sliders")
	for mode in ["white", "edges", "render"]:
		_set_display_mode(mode)
		verify(display.mode == mode, "Actual display mode " + mode)
		verify(display._wire_nodes[0].visible == (mode == "edges"), "Actual Face wire " + mode)
	_head_angle(45, 0)
	verify(is_equal_approx(turntable.rotation_degrees.y, 45), "Angle control")
	_reset_view()
	verify(is_equal_approx(turntable.rotation_degrees.y, 0), "Reset front")
	verify(camera.projection == Camera3D.PROJECTION_ORTHOGONAL, "Orthographic retained")
	_open_comparison()
	await RenderingServer.frame_post_draw
	verify(is_instance_valid(_compare_overlay), "Comparison opened")
	if is_instance_valid(_compare_overlay):
		verify(_compare_overlay.panes.size() == 2, "Both comparison panes")
		verify(
			is_instance_valid(_compare_overlay.expression_controls.row), "Blink controls available"
		)
		verify(not is_processing_input(), "Parent input suspended")
		_close_comparison()
		await get_tree().process_frame
		verify(is_processing_input(), "Parent input restored")
		verify(
			viewport.render_target_update_mode == SubViewport.UPDATE_ALWAYS,
			"Preview updates restored"
		)
	verify(FileAccess.get_sha256(source) == before, "Source geometry unchanged")
	var folder := "res://artifacts/face_review_scope"
	DirAccess.make_dir_recursive_absolute(folder)
	await RenderingServer.frame_post_draw
	var image := folder + "/preview.png"
	verify(get_viewport().get_texture().get_image().save_png(image) == OK, "Screenshot saved")
	var failures := 0
	for row in scope_checks:
		if not row.passed:
			failures += 1
	var report := {
		"source": source,
		"source_sha256": before,
		"checks": scope_checks,
		"failures": failures,
		"image": image,
		"image_sha256": FileAccess.get_sha256(image)
	}
	var file := FileAccess.open(folder + "/result.json", FileAccess.WRITE)
	file.store_string(JSON.stringify(report, "  "))
	file.close()
	print(
		"FACE_REVIEW_SCOPE=" + JSON.stringify({"checks": scope_checks.size(), "failures": failures})
	)
