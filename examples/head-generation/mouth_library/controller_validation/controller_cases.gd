extends RefCounted
## Exercises the controller's real _ready, public API, signals and engine frames.

const CHECKS = preload("res://assertions.gd")
const CURVE := [[0.0, 0.0], [0.25, 0.75], [0.5, 1.0], [0.75, 0.75], [1.0, 0.0]]
var checks
var tree: SceneTree
var samples: Array[Dictionary] = []


func run(review, host: SceneTree, assertions, has_corrective: bool) -> void:
	tree = host
	checks = assertions
	checks.check(review.mouth_ready, "controller initialized through real _ready")
	checks.check(review.MOUTH_NAMES == CHECKS.NAMES, "public input channel names unchanged")
	checks.check(review.mouth_sliders.size() == 4, "no corrective slider added")
	for pair in [["blink.L", 0.25], ["blink.R", 0.75]]:
		var index: int = review.mouth_face.find_blend_shape_by_name(pair[0])
		checks.check(index >= 0, "blink available before tests")
		if index >= 0:
			review.mouth_face.set_blend_shape_value(index, pair[1])
	await _curve_and_persistence(review, has_corrective)
	_rejections(review, has_corrective)
	await _sliders(review, has_corrective)
	await _partial_and_reset(review, has_corrective)


func _curve_and_persistence(review, has_corrective: bool) -> void:
	for pair in CURVE:
		var state := {
			"MouthOpen": pair[0], "MouthRound": 0.25, "MouthSpread": 0.75, "MouthPucker": 1.0
		}
		checks.check(review.set_mouth_state(state), "accept mixed state at " + str(pair[0]))
		checks.state(review, state, pair[1], has_corrective)
		samples.append(
			{
				"open": pair[0],
				"actual": review.actual_mouth_state(),
				"corrective": review.actual_mouth_corrective_state()
			}
		)
		for name in CHECKS.NAMES:
			review.mouth_face.set_blend_shape_value(review.mouth_indices[name], 0.13)
		if has_corrective:
			var index: int = review.mouth_face.find_blend_shape_by_name(CHECKS.CORRECTIVE)
			review.mouth_face.set_blend_shape_value(index, 0.17)
			checks.near(
				review.actual_mouth_corrective_state()[CHECKS.CORRECTIVE],
				0.17,
				"corrective inspection reads real mesh, not formula"
			)
		var sync_before: int = review.display.sync_calls
		await tree.process_frame
		await tree.process_frame
		checks.check(
			review.display.sync_calls > sync_before, "engine process frame reaches display sync"
		)
		checks.state(review, state, pair[1], has_corrective)


func _rejections(review, has_corrective: bool) -> void:
	var baseline := {"MouthOpen": 0.5, "MouthRound": 0.25, "MouthSpread": 0.75, "MouthPucker": 1.0}
	checks.check(review.set_mouth_state(baseline), "set atomicity baseline")
	var before: Dictionary = checks.snapshot(review)
	for value in [true, false, "1", null, [], {}, -0.1, 1.1, NAN, INF, -INF]:
		checks.check(
			not review.set_mouth_state({"MouthOpen": 0.0, "MouthRound": value}),
			"reject invalid value " + str(value)
		)
		checks.check(
			checks.snapshot(review) == before,
			"invalid value is atomic, including sliders, corrective and sync"
		)
	for invalid in [{"MouthOpen": 0.0, "unknown": 0.5}, {"MouthOvalMidCorrective": 0.2}, {42: 0.5}]:
		checks.check(not review.set_mouth_state(invalid), "reject unknown or derived-only channel")
		checks.check(checks.snapshot(review) == before, "invalid key is atomic")
	checks.state(review, baseline, 1.0, has_corrective)
	checks.check(review.set_mouth_state({"MouthOpen": 1}), "integer input remains accepted")
	checks.state(review, {"MouthOpen": 1.0}, 0.0, has_corrective)


func _sliders(review, has_corrective: bool) -> void:
	var state := {"MouthOpen": 0.0, "MouthRound": 0.25, "MouthSpread": 0.75, "MouthPucker": 1.0}
	checks.check(review.set_mouth_state(state), "set slider baseline")
	for pair in CURVE:
		review.mouth_sliders["MouthOpen"].value = pair[0]
		state["MouthOpen"] = pair[0]
		checks.state(review, state, pair[1], has_corrective)
		await tree.process_frame
		await tree.process_frame
		checks.state(review, state, pair[1], has_corrective)
	checks.check(
		review.set_mouth_state(
			{"MouthOpen": 0.5, "MouthRound": 0.25, "MouthSpread": 0.75, "MouthPucker": 1.0}
		),
		"set secondary slider baseline"
	)
	state["MouthOpen"] = 0.5
	for name in ["MouthRound", "MouthSpread", "MouthPucker"]:
		review.mouth_sliders[name].value = 0.6
		state[name] = 0.6
		checks.state(review, state, 1.0, has_corrective)


func _partial_and_reset(review, has_corrective: bool) -> void:
	checks.check(review.set_mouth_state({"MouthSpread": 0.5}), "accept partial state")
	checks.state(review, {"MouthSpread": 0.5}, 0.0, has_corrective)
	checks.check(review.set_mouth_state({"MouthOpen": 0.5}), "set nonzero corrective before reset")
	checks.check(review.set_mouth_state({}), "public reset accepted")
	checks.state(review, {}, 0.0, has_corrective)
	checks.check(
		review.set_mouth_state({"MouthOpen": 0.5, "MouthPucker": 0.75}),
		"set nonzero state before button reset"
	)
	var reset: Button
	for button in review.find_children("*", "Button", true, false):
		if button.text == "恢复中性":
			reset = button
	checks.check(reset != null, "actual reset button found")
	if reset != null:
		reset.pressed.emit()
		checks.state(review, {}, 0.0, has_corrective)
		await tree.process_frame
		await tree.process_frame
		checks.state(review, {}, 0.0, has_corrective)
