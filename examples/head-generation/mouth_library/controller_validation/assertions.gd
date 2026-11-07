extends RefCounted
## Assertions compare real MeshInstance3D state, UI state and display-sync order.

const NAMES := ["MouthOpen", "MouthRound", "MouthSpread", "MouthPucker"]
const CORRECTIVE := "MouthOvalMidCorrective"
var checks := 0
var failures: Array[String] = []
var context := ""


func check(condition: bool, message: String) -> void:
	checks += 1
	if not condition:
		var failure := context + ": " + message
		failures.append(failure)
		printerr("VALIDATION_FAILURE: " + failure)


func near(actual: float, expected: float, message: String) -> void:
	check(
		is_finite(actual) and absf(actual - expected) <= 0.000001,
		"%s: got %s, expected %s" % [message, actual, expected]
	)


func mesh_values(review) -> Dictionary:
	var result := {}
	for index in review.mouth_face.mesh.get_blend_shape_count():
		result[str(review.mouth_face.mesh.get_blend_shape_name(index))] = (
			review.mouth_face.get_blend_shape_value(index)
		)
	return result


func snapshot(review) -> Dictionary:
	var sliders := {}
	for name in NAMES:
		sliders[name] = review.mouth_sliders[name].value
	return {
		"actual": review.actual_mouth_state().duplicate(),
		"corrective": review.actual_mouth_corrective_state().duplicate(),
		"weights": review.mouth_weights.duplicate(),
		"sliders": sliders,
		"mesh": mesh_values(review),
		"sync_calls": review.display.sync_calls,
		"last_sync": review.display.last_values.duplicate(),
	}


func state(review, expected: Dictionary, corrective: float, has_corrective: bool) -> void:
	var actual: Dictionary = review.actual_mouth_state()
	check(actual.size() == 4, "public output still has four mouth channels")
	for name in NAMES:
		var value := float(expected.get(name, 0.0))
		near(float(actual.get(name, -1.0)), value, name + " mesh")
		near(float(review.mouth_weights.get(name, -1.0)), value, name + " stored")
		near(float(review.mouth_sliders[name].value), value, name + " slider")
		near(float(review.display.last_values.get(name, -1.0)), value, name + " display sync")
	var corrective_state: Dictionary = review.actual_mouth_corrective_state()
	if has_corrective:
		check(corrective_state.size() == 1, "corrective output contains one available shape")
		near(float(corrective_state.get(CORRECTIVE, -1.0)), corrective, "corrective readback")
		var index: int = review.mouth_face.find_blend_shape_by_name(CORRECTIVE)
		near(review.mouth_face.get_blend_shape_value(index), corrective, "corrective real mesh")
		near(
			float(review.display.last_values.get(CORRECTIVE, -1.0)),
			corrective,
			"corrective before display sync"
		)
	else:
		check(corrective_state.is_empty(), "legacy corrective output stays empty")
	blink_state(review)


func blink_state(review) -> void:
	for pair in [["blink.L", 0.25], ["blink.R", 0.75]]:
		var index: int = review.mouth_face.find_blend_shape_by_name(pair[0])
		check(index >= 0, "blink target exists: " + pair[0])
		if index >= 0:
			near(
				review.mouth_face.get_blend_shape_value(index),
				pair[1],
				"blink unchanged: " + pair[0]
			)
