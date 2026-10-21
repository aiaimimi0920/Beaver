extends GutTest
## Install through Beaver into tests; executes the actual generated-GLB consumer.

const REVIEW = preload("res://showcase_extensions/researched_mouth_review.gd")
const DEFINITION = "res://assets/aster/general_face_eyes_combo892/aster_definition.tres"
var review
var previous_max_fps: int


func before_each() -> void:
	previous_max_fps = Engine.max_fps
	review = REVIEW.new()
	review.character_definition = load(DEFINITION)
	add_child_autofree(review)
	await wait_frames(2)
	assert_true(review.mouth_ready)


func after_each() -> void:
	Engine.max_fps = previous_max_fps


func assert_weights(expected: Dictionary) -> void:
	var actual: Dictionary = review.actual_mouth_state()
	for name in review.MOUTH_NAMES:
		assert_almost_eq(float(actual[name]), float(expected.get(name, 0.0)), 0.000001)
		assert_almost_eq(float(review.mouth_sliders[name].value), float(expected.get(name, 0.0)), 0.000001)
		assert_almost_eq(float(review.mouth_weights[name]), float(expected.get(name, 0.0)), 0.000001)


func test_combined_state_survives_process_frame() -> void:
	var state := {"MouthOpen": 0.5, "MouthRound": 1.0, "MouthSpread": 0.25, "MouthPucker": 0.75}
	assert_true(review.set_mouth_state(state))
	await wait_frames(2)
	assert_weights(state)


func test_invalid_values_are_atomic() -> void:
	var state := {"MouthOpen": 0.5, "MouthRound": 0.25, "MouthSpread": 0.75, "MouthPucker": 1.0}
	assert_true(review.set_mouth_state(state))
	for value in [true, "1", -0.1, 1.1, NAN, INF, -INF]:
		assert_false(review.set_mouth_state({"MouthOpen": 0.0, "MouthRound": value}))
		assert_weights(state)
	assert_false(review.set_mouth_state({"MouthOpen": 0.0, "unknown": 0.5}))
	assert_weights(state)


func test_slider_signal_preserves_other_channels() -> void:
	var state := {"MouthOpen": 0.5, "MouthRound": 0.25, "MouthSpread": 0.75, "MouthPucker": 1.0}
	assert_true(review.set_mouth_state(state))
	review.mouth_sliders["MouthRound"].value = 0.6
	state["MouthRound"] = 0.6
	await wait_frames(1)
	assert_weights(state)


func test_partial_values_clear_omitted_channels() -> void:
	assert_true(review.set_mouth_state({"MouthOpen": 1.0, "MouthRound": 1.0, "MouthSpread": 1.0, "MouthPucker": 1.0}))
	assert_true(review.set_mouth_state({"MouthSpread": 0.5}))
	assert_weights({"MouthSpread": 0.5})


func test_actual_reset_button_keeps_blinks() -> void:
	var left: int = review.mouth_face.find_blend_shape_by_name("blink.L")
	var right: int = review.mouth_face.find_blend_shape_by_name("blink.R")
	assert_gte(left, 0)
	assert_gte(right, 0)
	review.mouth_face.set_blend_shape_value(left, 0.25)
	review.mouth_face.set_blend_shape_value(right, 0.75)
	assert_true(review.set_mouth_state({"MouthOpen": 1.0, "MouthPucker": 0.5}))
	var reset: Button
	for button in review.find_children("*", "Button", true, false):
		if button.text == "恢复中性":
			reset = button
	assert_not_null(reset)
	if reset == null:
		return
	reset.pressed.emit()
	await wait_frames(1)
	assert_weights({})
	assert_almost_eq(review.mouth_face.get_blend_shape_value(left), 0.25, 0.000001)
	assert_almost_eq(review.mouth_face.get_blend_shape_value(right), 0.75, 0.000001)


func test_unready_instance_rejects_without_mutation() -> void:
	var unready = REVIEW.new()
	assert_false(unready.set_mouth_state({"MouthOpen": 1.0}))
	assert_eq(unready.mouth_weights, {})
	assert_eq(unready.actual_mouth_state(), {})
	unready.free()
