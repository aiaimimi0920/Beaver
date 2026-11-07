extends SceneTree
## Focused controller integration only; the NPR host and rendering are not tested.

const REVIEW = preload("res://showcase_extensions/researched_mouth_review.gd")
const CHECKS = preload("res://assertions.gd")
const CASES = preload("res://controller_cases.gd")
var checks := CHECKS.new()
var cases: Array[Dictionary] = []


func _initialize() -> void:
	_run.call_deferred()


func _run() -> void:
	var paths := OS.get_cmdline_user_args()
	if paths.size() != 3:
		printerr("Expected V15, V11 and reordered V15 local GLB paths")
		quit(2)
		return
	_unready()
	for index in paths.size():
		await _model(paths[index], ["v15", "v11_legacy", "v15_reordered"][index], index != 1)
	print(
		(
			"MOUTH_CONTROLLER_VALIDATION="
			+ (
				JSON
				. stringify(
					{
						"engine": Engine.get_version_info().string,
						"scope":
						"Real mouth_review.gd and GLTFDocument imports; test-only head review/display stand-ins, not full NPR app or visual acceptance",
						"checks": checks.checks,
						"failures": checks.failures,
						"cases": cases,
						"passed": checks.failures.is_empty(),
					}
				)
			)
		)
	)
	quit(0 if checks.failures.is_empty() else 1)


func _unready() -> void:
	checks.context = "unready"
	var review = REVIEW.new()
	checks.check(not review.set_mouth_state({"MouthOpen": 1.0}), "unready setter rejects")
	checks.check(review.mouth_weights.is_empty(), "unready stored state is untouched")
	checks.check(review.actual_mouth_state().is_empty(), "unready public inspection empty")
	checks.check(
		review.actual_mouth_corrective_state().is_empty(), "unready corrective inspection empty"
	)
	review.free()


func _model(path: String, label: String, has_corrective: bool) -> void:
	checks.context = label
	var document := GLTFDocument.new()
	var gltf := GLTFState.new()
	var error := document.append_from_file(path, gltf)
	checks.check(error == OK, "GLTFDocument import succeeded")
	if error != OK:
		return
	var imported := document.generate_scene(gltf)
	checks.check(imported != null, "GLTFDocument generated actual model scene")
	if imported == null:
		return
	var face := _find_face(imported)
	checks.check(face != null, "real imported face has all four public mouth shapes")
	if face == null:
		imported.free()
		return
	var corrective_index := face.find_blend_shape_by_name(CHECKS.CORRECTIVE)
	checks.check(
		(corrective_index >= 0) == has_corrective, "corrective availability matches model case"
	)
	if label == "v15_reordered":
		checks.check(corrective_index == 0, "reordered actual GLB corrective is slot zero, not six")
	var names: Array[String] = []
	for index in face.mesh.get_blend_shape_count():
		names.append(str(face.mesh.get_blend_shape_name(index)))
	var review = REVIEW.new()
	review.test_face = face
	review.add_child(imported)
	root.add_child(review)
	checks.check(
		review.mouth_corrective_index == corrective_index,
		"controller cached name-resolved actual index"
	)
	var runner := CASES.new()
	var before := checks.checks
	await runner.run(review, self, checks, has_corrective)
	cases.append(
		{
			"name": label,
			"blend_shapes": names,
			"corrective_index": corrective_index,
			"checks": checks.checks - before,
			"samples": runner.samples
		}
	)
	review.queue_free()
	await process_frame


func _find_face(node: Node) -> MeshInstance3D:
	if node is MeshInstance3D:
		var matches := true
		for name in CHECKS.NAMES:
			matches = matches and node.find_blend_shape_by_name(name) >= 0
		if matches:
			return node
	for child in node.get_children():
		var candidate := _find_face(child)
		if candidate != null:
			return candidate
	return null
