extends GutTest
const PROBE := "res://tests/semantic_eyes_probe.json"
const REPORT := "user://semantic_eyes_import_report.json"

func find_mesh(root: Node, wanted: String) -> MeshInstance3D:
	if root is MeshInstance3D and root.name == wanted:
		return root
	for child in root.get_children():
		var found := find_mesh(child, wanted)
		if found != null:
			return found
	return null

func finite_value(value: Variant) -> bool:
	if value == null:
		return true
	if value is float:
		return is_finite(value)
	if value is Vector2:
		return value.is_finite()
	if value is Vector3:
		return value.is_finite()
	if value is Color:
		return is_finite(value.r) and is_finite(value.g) and is_finite(value.b) and is_finite(value.a)
	if value is int:
		return true
	if value is Array or value is PackedFloat32Array or value is PackedFloat64Array or value is PackedVector3Array or value is PackedVector2Array or value is PackedColorArray or value is PackedByteArray or value is PackedInt32Array or value is PackedInt64Array:
		for item in value:
			if not finite_value(item):
				return false
		return true
	return false

func required_face_arrays(arrays: Array, expected_count: int, require_indices: bool) -> bool:
	if arrays.size() != Mesh.ARRAY_MAX or expected_count <= 0:
		return false
	if not arrays[Mesh.ARRAY_VERTEX] is PackedVector3Array or not arrays[Mesh.ARRAY_NORMAL] is PackedVector3Array:
		return false
	if arrays[Mesh.ARRAY_VERTEX].size() != expected_count or arrays[Mesh.ARRAY_NORMAL].size() != expected_count:
		return false
	if require_indices and (not arrays[Mesh.ARRAY_TANGENT] is PackedFloat32Array or arrays[Mesh.ARRAY_TANGENT].size() != 4 * expected_count):
		return false
	if require_indices:
		if not arrays[Mesh.ARRAY_INDEX] is PackedInt32Array or arrays[Mesh.ARRAY_INDEX].is_empty():
			return false
		for index in arrays[Mesh.ARRAY_INDEX]:
			if index < 0 or index >= expected_count:
				return false
	return finite_value(arrays)

func exact_mesh(a: Mesh, b: Mesh) -> bool:
	if a.get_surface_count() != b.get_surface_count() or a.get_blend_shape_count() != b.get_blend_shape_count():
		return false
	for i in a.get_surface_count():
		if a.surface_get_arrays(i) != b.surface_get_arrays(i) or a.surface_get_blend_shape_arrays(i) != b.surface_get_blend_shape_arrays(i):
			return false
	return true

func vec(value: Array) -> Vector3:
	return Vector3(value[0], value[1], value[2])

func max_component(value: Vector3) -> float:
	return maxf(absf(value.x), maxf(absf(value.y), absf(value.z)))

func test_native_semantic_eye_imports() -> void:
	var probe = JSON.parse_string(FileAccess.get_file_as_string(PROBE))
	assert_not_null(probe, "Probe available")
	if probe == null:
		return
	var report := {"engine": Engine.get_version_info(), "scope": probe.scope, "test_sha256": FileAccess.get_sha256(get_script().resource_path), "probe_sha256": FileAccess.get_sha256(PROBE), "hashes": [], "cases": []}
	var hashes_ok := true
	for entry in probe.files:
		var actual := FileAccess.get_sha256(entry.path)
		assert_eq(actual, entry.sha256, "Bound native input: " + str(entry.path))
		hashes_ok = hashes_ok and actual == entry.sha256
		report.hashes.append({"path": entry.path, "actual_sha256": actual, "expected_sha256": entry.sha256})
	if not hashes_ok:
		write_report(report)
		return
	var original_scene: PackedScene = load(probe.original_resource)
	assert_not_null(original_scene)
	if original_scene == null:
		return
	var original := original_scene.instantiate()
	add_child_autofree(original)
	var counts := []
	for item in probe.cases:
		var actor := NPRCharacter.new()
		actor.definition = load(item.definition)
		assert_eq(actor.definition.model_scene.resource_path, item.model, "Definition points to bound model")
		add_child_autofree(actor)
		await get_tree().process_frame
		assert_true(actor.initialized)
		if not actor.initialized:
			continue
		var result := {"name": item.name, "weights": [], "frozen": {}}
		report.cases.append(result)
		for role in [0, 2]:
			var role_name: String = ["Body", "Face", "Hair"][role]
			var reference := find_mesh(original, role_name)
			assert_not_null(reference, "Original49 " + role_name)
			if reference != null:
				var frozen_ok := exact_mesh(reference.mesh, actor.meshes[role].mesh)
				result.frozen[role_name] = frozen_ok
				assert_true(frozen_ok, "Exhaustive imported original49 " + role_name + " arrays and topology")
		var face: MeshInstance3D = actor.meshes[1]
		assert_eq(face.mesh.get_surface_count(), 1)
		assert_eq(face.mesh.get_blend_shape_count(), 1)
		var mouth := face.find_blend_shape_by_name("MouthOpen")
		assert_gte(mouth, 0)
		if mouth < 0:
			continue
		var basis_arrays := face.mesh.surface_get_arrays(0)
		var morph_arrays := face.mesh.surface_get_blend_shape_arrays(0)
		assert_true(finite_value(basis_arrays), "All imported Face streams finite")
		assert_true(finite_value(morph_arrays), "All imported Face morph streams finite")
		var basis_ok := required_face_arrays(basis_arrays, int(item.vertex_count), true)
		assert_true(basis_ok, "Nonempty complete indexed Face basis arrays")
		assert_eq(morph_arrays.size(), 1, "One nonempty target array")
		if not basis_ok or morph_arrays.size() != 1:
			continue
		var morph_ok := required_face_arrays(morph_arrays[0], int(item.vertex_count), false)
		assert_true(morph_ok, "Nonempty complete Face morph arrays")
		if not morph_ok:
			continue
		var count: int = basis_arrays[Mesh.ARRAY_VERTEX].size()
		counts.append(count)
		result.vertex_count = count
		result.blend_shape_mode = face.mesh.get_blend_shape_mode()
		assert_eq(count, int(item.vertex_count), "Native vertex count matches audited GLB")
		assert_eq(basis_arrays[Mesh.ARRAY_INDEX].size(), int(item.index_count))
		var neutral_positions := PackedVector3Array()
		for bounds in item.bounds:
			face.set_blend_shape_value(mouth, bounds.weight)
			await get_tree().process_frame
			var baked := face.bake_mesh_from_current_blend_shape_mix()
			assert_not_null(baked, "Native blended mesh")
			if baked == null:
				continue
			var arrays := baked.surface_get_arrays(0)
			var finite := required_face_arrays(arrays, count, true)
			assert_true(finite, "Finite baked arrays at weight " + str(bounds.weight))
			if not finite:
				result.weights.append({"weight": bounds.weight, "finite": false})
				continue
			var points: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
			assert_eq(points.size(), count)
			var low := Vector3(INF, INF, INF)
			var high := Vector3(-INF, -INF, -INF)
			var moved := 0.0
			for i in points.size():
				low = low.min(points[i])
				high = high.max(points[i])
				if neutral_positions.size() == points.size():
					moved = maxf(moved, points[i].distance_to(neutral_positions[i]))
			if bounds.weight == 0:
				neutral_positions = points.duplicate()
			var error := maxf(max_component(low - vec(bounds.min)), max_component(high - vec(bounds.max)))
			assert_true(is_finite(error) and error <= float(probe.position_atol_m), "Actual bounds vs nonlinear recipe GLB, fixed3e-6m")
			if bounds.weight > 0:
				assert_gt(moved, 0.001 * float(bounds.weight), "MouthOpen produces nontrivial actual motion")
			result.weights.append({"weight": bounds.weight, "finite": finite, "vertices": points.size(), "bounds_error_m": error, "max_motion_m": moved})
		face.set_blend_shape_value(mouth, 0.0)
	assert_eq(counts.size(), 2)
	if counts.size() == 2:
		assert_gt(counts[0], counts[1], "Target has fewer vertices after optional layer removal")
	write_report(report)

func write_report(report: Dictionary) -> void:
	var file := FileAccess.open(REPORT, FileAccess.WRITE)
	assert_not_null(file)
	if file != null:
		file.store_string(JSON.stringify(report, "\t"))
		file.close()
	print("SEMANTIC_EYES_IMPORT_REPORT " + REPORT)
