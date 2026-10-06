extends GutTest
## Current head-review integration; full wardrobe acceptance stays a separate gate.

const REVIEW = preload("res://showcase/aster_head_review.tscn")
const ORIGINAL = preload("res://assets/aster/head_recovery_94/aster_definition.tres")
const WARDROBE_CONFIG = preload("res://addons/npr_character_frame/showcase/wardrobe_configuration.gd")

func test_head_review_initializes_without_mutating_authoring_definition() -> void:
	var original_height: float = ORIGINAL.display_height
	var original_scene: PackedScene = ORIGINAL.model_scene
	var review = REVIEW.instantiate()
	review.character_definition = ORIGINAL
	add_child_autofree(review)
	await get_tree().process_frame
	assert_not_null(review.preview)
	assert_true(review.preview.initialized)
	assert_ne(review.preview.definition, ORIGINAL)
	assert_eq(review.preview.definition.model_scene, original_scene)
	assert_eq(ORIGINAL.model_scene, original_scene)
	assert_eq(ORIGINAL.display_height, original_height)
	assert_false(review.preview.meshes[0].visible)
	assert_eq(review.camera.projection, Camera3D.PROJECTION_ORTHOGONAL)
	assert_true(review.get_node("Save").disabled)
	var slider_count := 0
	for control in review._content.get_children():
		if control is HSlider:
			slider_count += 1
	assert_eq(slider_count, 4)
	review._head_angle(15.0, 0.0)
	assert_almost_eq(review.turntable.rotation_degrees.y, 15.0, 0.001)
	review.set_view("full")
	assert_almost_eq(review._distance, 3.5, 0.001)
	review._reset_view()
	assert_almost_eq(review.turntable.rotation_degrees.y, 0.0, 0.001)
	assert_almost_eq(review._target.y, review._head_focus.y, 0.001)
	assert_eq(ORIGINAL.display_height, original_height)

func test_head_only_definition_does_not_pass_full_wardrobe_acceptance() -> void:
	var errors: PackedStringArray = WARDROBE_CONFIG.validate(ORIGINAL, "aster", "Aster", "")
	assert_false(errors.is_empty())
	assert_true(errors.has("Full showcase requires performance_data_path"))
	assert_true(errors.has("Full showcase requires soft_tissue_data_path"))
	assert_true(errors.has("Full showcase requires hair_dynamics_data_path"))
	assert_true(errors.has("Full showcase requires rig_layout_data_path"))
