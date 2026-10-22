extends GutTest
## Install through Beaver after generating these actual candidates.
## Runtime checks do not replace visual or continuous-collision acceptance.

const CANDIDATES := {
	905: "compact-rounded",
	898: "soft-tapered",
	899: "straight-tapered",
	900: "blunt-chin",
	901: "compact-rounded",  # Different eyes, brow, nose and mouth composition.
}
const REVIEW = preload("res://showcase_extensions/researched_mouth_review.gd")
const TARGETS := ["blink.L", "blink.R", "MouthOpen", "MouthRound", "MouthSpread", "MouthPucker"]


func prefix(candidate: int) -> String:
	return "res://assets/aster/general_face_eyes_combo%d/" % candidate


func test_native_presets_have_distinct_bound_geometry() -> void:
	var digests: Array[String] = []
	var geometry_signatures: Array[String] = []
	for candidate in CANDIDATES:
		var base := prefix(candidate)
		var report = JSON.parse_string(FileAccess.get_file_as_string(base + "generation_report.json"))
		assert_not_null(report)
		if report == null:
			continue
		assert_eq(report.face_components.face_design.kind, CANDIDATES[candidate])
		assert_eq(report.facial_surface.face_design.parameters.kind, CANDIDATES[candidate])
		var digest := FileAccess.get_sha256(base + "aster_head.glb")
		assert_eq(digest.length(), 64)
		assert_false(digests.has(digest), "Each preset must have actual distinct geometry")
		digests.append(digest)
		var definition = load(base + "aster_definition.tres")
		assert_not_null(definition)
		if definition == null:
			continue
		assert_eq(definition.validate().size(), 0)
		assert_not_null(definition.material_set.face_sdf360)
		assert_true(definition.material_set.sdf_on_uv2)
		var actor := NPRCharacter.new()
		actor.definition = definition
		add_child_autofree(actor)
		await wait_frames(2)
		assert_true(actor.initialized)
		if not actor.initialized:
			continue
		assert_eq(actor.meshes.size(), 3)
		var face: MeshInstance3D = actor.meshes[1]
		var vertices: PackedVector3Array = face.mesh.surface_get_arrays(0)[Mesh.ARRAY_VERTEX]
		var context := HashingContext.new()
		context.start(HashingContext.HASH_SHA256)
		context.update(vertices.to_byte_array())
		var signature := context.finish().hex_encode()
		assert_false(geometry_signatures.has(signature), "Displayed face vertices differ")
		geometry_signatures.append(signature)
		for target in TARGETS:
			assert_gte(face.find_blend_shape_by_name(target), 0, target)
		actor.queue_free()
		await wait_frames(1)


func test_face_presets_keep_mouth_and_blinks_independent() -> void:
	var saved_fps := Engine.max_fps
	for candidate in CANDIDATES:
		var review = REVIEW.new()
		review.character_definition = load(prefix(candidate) + "aster_definition.tres")
		review.set_meta("explicit_definition", true)
		add_child_autofree(review)
		await wait_frames(2)
		assert_true(review.mouth_ready)
		if not review.mouth_ready:
			review.queue_free()
			continue
		assert_eq(review.preview.definition.model_scene.resource_path, prefix(candidate) + "aster_head.glb")
		var left: int = review.mouth_face.find_blend_shape_by_name("blink.L")
		var right: int = review.mouth_face.find_blend_shape_by_name("blink.R")
		review.mouth_face.set_blend_shape_value(left, 0.25)
		review.mouth_face.set_blend_shape_value(right, 0.75)
		var requested := {"MouthOpen": 0.5, "MouthRound": 1.0, "MouthSpread": 0.25, "MouthPucker": 0.75}
		assert_true(review.set_mouth_state(requested))
		await wait_frames(2)
		var actual: Dictionary = review.actual_mouth_state()
		for target in requested:
			assert_almost_eq(float(actual[target]), float(requested[target]), 0.000001)
		assert_almost_eq(review.mouth_face.get_blend_shape_value(left), 0.25, 0.000001)
		assert_almost_eq(review.mouth_face.get_blend_shape_value(right), 0.75, 0.000001)
		assert_true(review.set_mouth_state({}))
		await wait_frames(1)
		for value in review.actual_mouth_state().values():
			assert_almost_eq(float(value), 0.0, 0.000001)
		assert_almost_eq(review.mouth_face.get_blend_shape_value(left), 0.25, 0.000001)
		assert_almost_eq(review.mouth_face.get_blend_shape_value(right), 0.75, 0.000001)
		review.queue_free()
		await wait_frames(1)
	Engine.max_fps = saved_fps


func test_selector_changes_actual_definition_and_rejects_invalid_index() -> void:
	var saved_fps := Engine.max_fps
	var controller = load("res://showcase_extensions/researched_face_review.gd").new()
	add_child_autofree(controller)
	await wait_frames(2)
	assert_true(controller.face_ready)
	for index in range(4):
		controller.selector.item_selected.emit(index)
		await wait_frames(2)
		assert_eq(controller.current_index, index)
		assert_eq(controller.current_review.character_definition.resource_path, prefix(controller.CANDIDATES[index]) + "aster_definition.tres")
		assert_true(controller.current_review.mouth_ready)
		assert_gt(controller.current_review.size.x, 100.0)
		assert_gt(controller.current_review.size.y, 100.0)
		assert_gt(controller.current_review._stage.size.x, 100.0)
		assert_gt(controller.current_review.viewport.size.x, 100)
		assert_eq(controller.current_review.preview.definition.model_scene.resource_path, prefix(controller.CANDIDATES[index]) + "aster_head.glb")
		for value in controller.current_review.actual_mouth_state().values():
			assert_almost_eq(float(value), 0.0, 0.000001)
	var previous = controller.current_review
	assert_false(controller.select_face(-1))
	assert_false(controller.select_face(4))
	assert_eq(controller.current_review, previous)
	assert_eq(controller.current_index, 3)
	Engine.max_fps = saved_fps
