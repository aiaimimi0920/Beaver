# Run only through an authorized Beaver native validation job with NPR and GUT.
# Scope: affine of the actual imported same-job baseline; no full-performance profiles.
extends GutTest
const PROBE_PATH := "res://tests/node_affine_probe.json"
const REPORT_PATH := "user://morph_node_affine_report.json"

func vector3(value: Array) -> Vector3:
	return Vector3(value[0], value[1], value[2])

func max3(value: Vector3) -> float:
	return maxf(absf(value.x), maxf(absf(value.y), absf(value.z)))

func within(actual: Vector3, expected: Vector3, atol: float, rtol: float) -> bool:
	var d := (actual - expected).abs()
	var limit := expected.abs() * rtol + Vector3.ONE * atol
	return d.x <= limit.x and d.y <= limit.y and d.z <= limit.z

func mesh_nodes(root: Node) -> Array:
	var result := []
	if root is MeshInstance3D:
		result.append(root)
	for child in root.get_children():
		result.append_array(mesh_nodes(child))
	return result

func transform_values(value: Transform3D) -> Array:
	return [value.basis.x.x, value.basis.x.y, value.basis.x.z, value.basis.y.x, value.basis.y.y, value.basis.y.z, value.basis.z.x, value.basis.z.y, value.basis.z.z, value.origin.x, value.origin.y, value.origin.z]

func test_same_job_imported_baseline_affine() -> void:
	var probe = JSON.parse_string(FileAccess.get_file_as_string(PROBE_PATH))
	assert_not_null(probe, "Node-affine probe manifest")
	if probe == null:
		return
	assert_eq(probe.schema, "native-node-affine-probe-v1")
	assert_eq(probe.contract, "affine-of-actual-imported-same-job-baseline")
	var report := {"engine": Engine.get_version_info(), "test_sha256": FileAccess.get_sha256(get_script().resource_path), "probe_sha256": FileAccess.get_sha256(PROBE_PATH), "contract": probe.contract, "cases": [], "raw_glb_engine_note": "Raw GLB endpoint-length packing is a separate contract; this compares two actual imports of identical baseline streams. No engine packing tolerance is added."}
	for case_data in probe.cases:
		var result := {"name": case_data.name, "failures": [], "weights": []}
		report.cases.append(result)
		var hashes_ok := true
		for entry in [[case_data.base_resource, case_data.base_sha256], [case_data.variant_resource, case_data.variant_sha256], [case_data.intent_resource, case_data.intent_sha256], [case_data.definition_resource, case_data.definition_sha256], [case_data.report_resource, case_data.report_sha256]]:
			var actual_hash := FileAccess.get_sha256(entry[0])
			assert_eq(actual_hash, entry[1], "Actual resource hash: " + str(entry[0]))
			hashes_ok = hashes_ok and actual_hash == entry[1]
		result.base_sha256 = case_data.base_sha256
		result.variant_sha256 = case_data.variant_sha256
		if not hashes_ok:
			result.failures.append("Resource hash mismatch")
			continue
		# The expected affine comes from intent, never the tested variant node.
		var intent = JSON.parse_string(FileAccess.get_file_as_string(case_data.intent_resource))
		var scales := Vector3(intent.proportions.width, intent.proportions.height, intent.proportions.depth)
		assert_eq(scales, vector3(case_data.declared_scale), "Intent scale binding")
		var generation_report = JSON.parse_string(FileAccess.get_file_as_string(case_data.report_resource))
		var authored_pivot := vector3(generation_report.general_face.pivot_blender_xyz)
		var pivot := Vector3(authored_pivot.x, authored_pivot.z, -authored_pivot.y)
		assert_eq(pivot, vector3(case_data.declared_pivot), "Generation report pivot binding")
		var affine := Transform3D(Basis.from_scale(scales), pivot - scales * pivot)
		var base_scene: PackedScene = load(case_data.base_resource)
		assert_not_null(base_scene, "Independent same-job imported Face scene")
		if base_scene == null:
			result.failures.append("Baseline scene missing")
			continue
		var base_root := base_scene.instantiate() as Node3D
		add_child_autofree(base_root)
		var base_meshes := mesh_nodes(base_root)
		assert_eq(base_meshes.size(), 1, "Face-only baseline")
		if base_meshes.size() != 1:
			result.failures.append("Baseline not Face-only")
			continue
		var base_face: MeshInstance3D = base_meshes[0]
		var actor := NPRCharacter.new()
		actor.definition = load(case_data.definition_resource)
		var head_only := actor.definition != null
		if head_only:
			for field in ["face_motion_profile", "face_atlas_profile", "eye_geometry_profile"]:
				head_only = head_only and actor.definition.get(field) == null
			head_only = head_only and actor.definition.performance_data_path.is_empty()
		assert_true(head_only, "Bounded head-only definition excludes optional full-performance paths")
		if not head_only:
			result.failures.append("Unsupported optional profile")
			actor.free()
			continue
		add_child_autofree(actor)
		await get_tree().process_frame
		assert_true(actor.initialized, "Actual NPRCharacter initialized")
		if not actor.initialized:
			result.failures.append("Variant initialization failed")
			continue
		var face: MeshInstance3D = actor.meshes[1]
		var baseline := base_face.mesh as ArrayMesh
		var variant := face.mesh as ArrayMesh
		assert_not_null(baseline, "Baseline ArrayMesh")
		assert_not_null(variant, "Variant ArrayMesh")
		if baseline == null or variant == null:
			result.failures.append("ArrayMesh unavailable")
			continue
		# Cancel display-height alignment while retaining imported Face transform.
		var variant_relative := actor.character.global_transform.affine_inverse() * face.global_transform
		var base_relative := base_root.global_transform.affine_inverse() * base_face.global_transform
		result.actual_relative_transform = transform_values(variant_relative)
		result.expected_relative_transform = transform_values(affine)
		result.actor_display_transform = transform_values(actor.character.global_transform)
		var transform_ok := true
		for axis in [Vector3.ZERO, Vector3.RIGHT, Vector3.UP, Vector3.BACK]:
			transform_ok = transform_ok and within(base_relative * axis, axis, probe.position_atol, probe.rtol)
			transform_ok = transform_ok and within(variant_relative * axis, affine * axis, probe.position_atol, probe.rtol)
		assert_true(transform_ok, "Declared affine matches actual Face transform")
		if not transform_ok:
			result.failures.append("Node transform mismatch")
			continue
		var identity_ok := baseline.get_surface_count() == variant.get_surface_count() and baseline.get_blend_shape_count() == variant.get_blend_shape_count() and baseline.get_blend_shape_mode() == variant.get_blend_shape_mode()
		result.base_blend_mode = baseline.get_blend_shape_mode()
		result.variant_blend_mode = variant.get_blend_shape_mode()
		result.surface_formats = []
		if identity_ok:
			for surface in baseline.get_surface_count():
				var a := baseline.surface_get_arrays(surface)
				var b := variant.surface_get_arrays(surface)
				var am := baseline.surface_get_blend_shape_arrays(surface)
				var bm := variant.surface_get_blend_shape_arrays(surface)
				result.surface_formats.append([baseline.surface_get_format(surface), variant.surface_get_format(surface)])
				identity_ok = identity_ok and a == b and am == bm
		result.exhaustive_native_joint_arrays_and_indices_exact = identity_ok
		assert_true(identity_ok, "Complete native joint arrays, morph arrays and indices match exactly")
		if not identity_ok:
			result.failures.append("Native arrays/order differ; index correspondence refused")
			continue
		var base_mouth := base_face.find_blend_shape_by_name("MouthOpen")
		var variant_mouth := face.find_blend_shape_by_name("MouthOpen")
		assert_gte(base_mouth, 0)
		assert_eq(base_mouth, variant_mouth, "Target identity")
		if base_mouth < 0 or base_mouth != variant_mouth:
			result.failures.append("Target mismatch")
			continue
		var expected_normal_matrix := affine.basis.inverse().transposed()
		var actual_normal_matrix := variant_relative.basis.inverse().transposed()
		for weight in probe.weights:
			base_face.set_blend_shape_value(base_mouth, weight)
			face.set_blend_shape_value(variant_mouth, weight)
			await get_tree().process_frame
			var a := base_face.bake_mesh_from_current_blend_shape_mix()
			var b := face.bake_mesh_from_current_blend_shape_mix()
			assert_not_null(a, "Baseline engine bake")
			assert_not_null(b, "Variant engine bake")
			if a == null or b == null:
				result.failures.append("Engine bake unavailable")
				continue
			var stats := {"weight": weight, "vertices": 0, "position_max_error": 0.0, "normal_max_error": 0.0, "world_position_max_error": 0.0, "world_normal_max_error": 0.0, "violations": 0, "baked_local_arrays_exact": true}
			for surface in a.get_surface_count():
				var aa := a.surface_get_arrays(surface)
				var bb := b.surface_get_arrays(surface)
				stats.baked_local_arrays_exact = stats.baked_local_arrays_exact and aa == bb
				var ap: PackedVector3Array = aa[Mesh.ARRAY_VERTEX]
				var bp: PackedVector3Array = bb[Mesh.ARRAY_VERTEX]
				var an: PackedVector3Array = aa[Mesh.ARRAY_NORMAL]
				var bn: PackedVector3Array = bb[Mesh.ARRAY_NORMAL]
				for vertex in ap.size():
					var expected_position := affine * ap[vertex]
					var actual_position := variant_relative * bp[vertex]
					var expected_normal := (expected_normal_matrix * an[vertex]).normalized()
					var actual_normal := (actual_normal_matrix * bn[vertex]).normalized()
					var world_expected := actor.character.global_transform * expected_position
					var world_actual := face.global_transform * bp[vertex]
					var world_expected_normal := ((actor.character.global_transform.basis * affine.basis).inverse().transposed() * an[vertex]).normalized()
					var world_actual_normal := (face.global_transform.basis.inverse().transposed() * bn[vertex]).normalized()
					stats.vertices += 1
					stats.position_max_error = maxf(stats.position_max_error, max3(actual_position - expected_position))
					stats.normal_max_error = maxf(stats.normal_max_error, max3(actual_normal - expected_normal))
					stats.world_position_max_error = maxf(stats.world_position_max_error, max3(world_actual - world_expected))
					stats.world_normal_max_error = maxf(stats.world_normal_max_error, max3(world_actual_normal - world_expected_normal))
					if not within(actual_position, expected_position, probe.position_atol, probe.rtol) or not within(actual_normal, expected_normal, probe.normal_atol, probe.rtol) or not within(world_actual, world_expected, probe.position_atol, probe.rtol) or not within(world_actual_normal, world_expected_normal, probe.normal_atol, probe.rtol):
						stats.violations += 1
			result.weights.append(stats)
			assert_true(stats.baked_local_arrays_exact, "Engine baked local arrays exact")
			assert_eq(stats.violations, 0, "All vertices affine at weight " + str(weight))
			if stats.violations > 0 or not stats.baked_local_arrays_exact:
				result.failures.append("Affine blend failed at " + str(weight))
		base_face.set_blend_shape_value(base_mouth, 0.0)
		face.set_blend_shape_value(variant_mouth, 0.0)
		result.status = "PASS" if result.failures.is_empty() else "FAIL"
	var file := FileAccess.open(REPORT_PATH, FileAccess.WRITE)
	assert_not_null(file, "Native report output")
	if file != null:
		file.store_string(JSON.stringify(report, "\t"))
		file.close()
	var summary := []
	for result in report.cases:
		summary.append({"name": result.name, "status": result.get("status", "FAIL"), "failures": result.failures})
	print("MORPH_NODE_AFFINE_REPORT " + REPORT_PATH + " " + JSON.stringify(summary))
