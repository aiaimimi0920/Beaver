extends GutTest

const CAPTURE = preload("res://showcase_extensions/faye_capture_probe.gd")
const ATLAS := "25969224699a1bcec09e2df78f48922cf91bd652c3e855cf939cc436073b6223"
var fixture_paths: Array[String] = []
var probe


func before_each() -> void:
	probe = CAPTURE.new()


func after_each() -> void:
	probe.free()
	for path in fixture_paths:
		DirAccess.remove_absolute(path)
	fixture_paths.clear()


func _report(value: Dictionary) -> String:
	var path := "user://faye-capture-test-%s.json" % Time.get_ticks_usec()
	var file := FileAccess.open(path, FileAccess.WRITE)
	file.store_string(JSON.stringify(value))
	file.close()
	fixture_paths.append(path)
	return path


func _valid_report() -> Dictionary:
	return {"new_glb_sha256": CAPTURE.EXPECTED_MODEL, "face_components": {"composition_sha256": CAPTURE.EXPECTED_COMPOSITION},
		"semantic_eyes": {"sdf": {"atlas_sha256": ATLAS}}}


func test_accepts_exact_model_composition_and_atlas() -> void:
	assert_true(probe._valid_identity(_report(_valid_report()),
		{"model": CAPTURE.EXPECTED_MODEL, "atlas": ATLAS}))


func test_rejects_wrong_model_or_atlas() -> void:
	var report := _report(_valid_report())
	assert_false(probe._valid_identity(report, {"model": "wrong", "atlas": ATLAS}))
	assert_false(probe._valid_identity(report, {"model": CAPTURE.EXPECTED_MODEL, "atlas": "wrong"}))


func test_rejects_wrong_composition_and_missing_report_fields() -> void:
	var report := _valid_report()
	report.face_components.composition_sha256 = "another-composition"
	var hashes := {"model": CAPTURE.EXPECTED_MODEL, "atlas": ATLAS}
	assert_false(probe._valid_identity(_report(report), hashes))
	assert_false(probe._valid_identity(_report({}), hashes))
	assert_false(probe._valid_identity(_report({"face_components": "invalid", "semantic_eyes": {}}), hashes))


func test_hash_collection_cannot_be_empty() -> void:
	assert_false(probe._valid_hashes({}))
	assert_true(probe._valid_hashes({"model": CAPTURE.EXPECTED_MODEL, "atlas": ATLAS}))


func test_rejects_report_bound_to_another_model() -> void:
	var report := _valid_report()
	report.new_glb_sha256 = "another-model"
	assert_false(probe._valid_identity(_report(report),
		{"model": CAPTURE.EXPECTED_MODEL, "atlas": ATLAS}))


func test_binding_includes_renderer_implementation_and_project_settings() -> void:
	var bindings: Dictionary = probe._bindings()
	for path in ["res://project.godot", "res://addons/npr_character_frame/npr_character.gd",
		"res://addons/npr_character_frame/shaders/face/sdf360/sdf_materials.gd",
		"res://addons/npr_character_frame/shaders/face/sdf360/face_base.gdshader"]:
		assert_eq(bindings.get(path), FileAccess.get_sha256(path))
		assert_eq(str(bindings.get(path)).length(), 64)


func _materials():
	return load("res://assets/aster/general_face_eyes_combofaye01/aster_definition.tres").material_set.duplicate()


func test_texture_snapshot_covers_required_optional_and_aliased_maps() -> void:
	var materials = _materials()
	var snapshot: Dictionary = probe._texture_snapshot(materials)
	for field in CAPTURE.REQUIRED_TEXTURES:
		assert_eq(snapshot[field].sha256, FileAccess.get_sha256(materials.get(field).resource_path))
	assert_true(snapshot.has("face_expression"))
	assert_null(snapshot.face_expression)
	assert_eq(snapshot.face_outline_control, snapshot.face_map)
	assert_true(snapshot.has("dissolve_noise"))


func test_texture_snapshot_rejects_missing_required_and_pathless_maps() -> void:
	var materials = _materials()
	materials.face_base = null
	assert_true(probe._texture_snapshot(materials).is_empty())
	materials = _materials()
	var image := Image.create(1, 1, false, Image.FORMAT_RGBA8)
	materials.face_expression = ImageTexture.create_from_image(image)
	assert_true(probe._texture_snapshot(materials).is_empty())


func test_texture_snapshot_detects_optional_changes_and_reassignment() -> void:
	var materials = _materials()
	var before: Dictionary = probe._texture_snapshot(materials)
	materials.face_expression = materials.face_base
	assert_true(probe._texture_snapshot(materials) != before)
	materials = _materials()
	materials.face_base = materials.face_map
	assert_true(probe._texture_snapshot(materials) != before)
