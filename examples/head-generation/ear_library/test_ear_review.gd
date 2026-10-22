extends GutTest

const EAR_REVIEW = preload("res://showcase_extensions/researched_ear_review.gd")
const REQUIRED_EARS := ["standard-oval", "compact-round", "slender-oval", "short-point", "long-leaf", "side-droop"]

var saved_fps: int


func before_each() -> void:
	saved_fps = Engine.max_fps


func after_each() -> void:
	Engine.max_fps = saved_fps


class WrongDefinition:
	extends Resource
	func validate() -> Array:
		return []


class BrokenMouthReview:
	extends "res://showcase_extensions/researched_mouth_review.gd"
	func _ready() -> void:
		super._ready()
		preview.initialized = false


class RollbackReview:
	extends "res://showcase_extensions/researched_ear_review.gd"
	var reject_next := false
	func _create_review():
		return BrokenMouthReview.new() if reject_next else super._create_review()


func test_failed_child_initialization_keeps_the_live_previous_view() -> void:
	var view = RollbackReview.new()
	add_child_autofree(view)
	await get_tree().process_frame
	assert_true(view.ear_ready)
	if not view.ear_ready:
		return
	var previous = view.current_review
	var index: int = view.current_index
	var count: int = view.get_child_count()
	view.reject_next = true
	assert_false(view.select_entry((index + 1) % view.entries.size()))
	await wait_frames(2)
	assert_eq(view.current_review, previous)
	assert_eq(view.current_index, index)
	assert_eq(view.selector.selected, index)
	assert_true(previous.visible)
	assert_eq(view.get_child_count(), count)


func test_malformed_report_and_wrong_resource_are_controlled_rejections() -> void:
	var view = EAR_REVIEW.new()
	autofree(view)
	for report in [null, "bad", [], {"face_components": null}, {"face_components": "bad"}, {"face_components": []}]:
		assert_false(view._report_matches(report, {}, "a".repeat(64)))
	for digest in [null, "", "b".repeat(64)]:
		assert_false(view._report_matches({"face_components": {"selection": {}}, "new_glb_sha256": digest}, {}, "a".repeat(64)))
	assert_true(view._report_matches({"face_components": {"selection": {}}, "new_glb_sha256": "a".repeat(64)}, {}, "a".repeat(64)))
	assert_false(view._definition_matches(Resource.new(), "res://missing.glb"))
	assert_false(view._definition_matches(WrongDefinition.new(), "res://missing.glb"))


func test_all_six_actual_ear_models_and_existing_face_examples_select() -> void:
	var view = EAR_REVIEW.new()
	add_child_autofree(view)
	await get_tree().process_frame
	assert_true(view.ear_ready)
	if not view.ear_ready:
		return
	var found := {}
	var definitions := {}
	var model_hashes := {}
	var vertex_hashes := {}
	for i in view.entries.size():
		var entry: Dictionary = view.entries[i]
		assert_true(view.select_entry(i), entry["id"])
		await get_tree().process_frame
		assert_eq(view.current_index, i)
		assert_eq(view.selector.selected, i)
		assert_eq(view.current_review.character_definition.resource_path, entry["definition"])
		assert_eq(view.current_review.preview.definition.model_scene.resource_path,
			entry["definition"].get_base_dir() + "/aster_head.glb")
		assert_true(view.current_review.mouth_ready)
		assert_true(view.current_review.preview.initialized)
		assert_true(view.current_review.character_definition.material_set.sdf_on_uv2)
		assert_not_null(view.current_review.character_definition.material_set.face_sdf360)
		if entry["id"].begins_with("ear-"):
			found[entry["selection"]["ears"].trim_prefix("research-")] = true
			definitions[entry["definition"]] = true
			model_hashes[entry["model_sha256"]] = true
			var vertices: PackedVector3Array = view.current_review.mouth_face.mesh.surface_get_arrays(0)[Mesh.ARRAY_VERTEX]
			var context := HashingContext.new()
			context.start(HashingContext.HASH_SHA256)
			context.update(vertices.to_byte_array())
			vertex_hashes[context.finish().hex_encode()] = true
	for kind in REQUIRED_EARS:
		assert_true(found.has(kind), kind)
	assert_eq(definitions.size(), 6)
	assert_eq(model_hashes.size(), 6)
	assert_eq(vertex_hashes.size(), 6)
	var target: int = (view.current_index + 1) % view.entries.size()
	view.selector.item_selected.emit(target)
	await get_tree().process_frame
	assert_eq(view.current_index, target)
	assert_gt(view.current_review.size.x, 100.)
	assert_gt(view.current_review.size.y, 100.)
	assert_gt(view.current_review._stage.size.x, 100.)
	assert_gt(view.current_review.viewport.size.x, 100)
	assert_false(view.header_panel.get_global_rect().intersects(view.current_review.get_global_rect()))
	assert_gte(view.current_review._stage.get_global_rect().position.x, 225.)
	assert_lte(view.current_review._stage.get_global_rect().end.x, view.get_global_rect().end.x - 380.)
	assert_almost_eq(float(view.current_review.viewport.size.x), view.current_review._stage.size.x, 1.)
	for candidate in [905, 898, 899, 900]:
		var path := "res://assets/aster/general_face_eyes_combo%d/aster_definition.tres" % candidate
		assert_true(view.entries.any(func(entry): return entry["definition"] == path))


func test_invalid_selection_preserves_current_model_and_index() -> void:
	var view = EAR_REVIEW.new()
	add_child_autofree(view)
	await get_tree().process_frame
	assert_true(view.ear_ready)
	if not view.ear_ready:
		return
	var previous = view.current_review
	var previous_index: int = view.current_index
	assert_false(view.select_entry(-1))
	assert_false(view.select_entry(view.entries.size()))
	var saved: Dictionary = view.entries[0].duplicate(true)
	view.entries[0]["model_sha256"] = "changed"
	assert_false(view.select_entry(0))
	view.entries[0] = saved
	assert_eq(view.current_review, previous)
	assert_eq(view.current_index, previous_index)
	assert_eq(view.selector.selected, previous_index)
	assert_true(view.current_review.visible)


func test_switching_explicitly_resets_mouth_blink_view_and_display() -> void:
	var view = EAR_REVIEW.new()
	add_child_autofree(view)
	await get_tree().process_frame
	assert_true(view.ear_ready)
	if not view.ear_ready:
		return
	view.current_review.set_mouth_state({"MouthOpen": .8, "MouthPucker": .3})
	view.current_review._head_angle(45.0, 10.0)
	view.current_review._set_display_mode("white")
	var face: MeshInstance3D = view.current_review.mouth_face
	for name in ["blink.L", "blink.R"]:
		var index := face.find_blend_shape_by_name(name)
		assert_gte(index, 0)
		if index >= 0:
			face.set_blend_shape_value(index, .7 if name == "blink.L" else .2)
	await get_tree().process_frame
	assert_almost_eq(view.current_review.actual_mouth_state()["MouthOpen"], .8, .00001)
	assert_true(view.select_entry((view.current_index + 1) % view.entries.size()))
	await get_tree().process_frame
	assert_almost_eq(view.current_review._model_yaw_degrees, 0., .00001)
	assert_almost_eq(view.current_review._camera_pitch, 0., .00001)
	assert_eq(view.current_review.display.mode, "render")
	for value in view.current_review.actual_mouth_state().values():
		assert_almost_eq(value, 0., .00001)
	face = view.current_review.mouth_face
	for name in ["blink.L", "blink.R"]:
		var index := face.find_blend_shape_by_name(name)
		assert_gte(index, 0)
		if index >= 0:
			assert_almost_eq(face.get_blend_shape_value(index), 0., .00001)
