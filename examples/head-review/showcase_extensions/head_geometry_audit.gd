extends RefCounted
## Aggregate-only diagnostics. Never changes or exports source vertices/UVs.


static func root(parents: Array[int], index: int) -> int:
	var current := index
	while parents[current] != current:
		current = parents[current]
	return current


static func summarize(pane) -> void:
	var face: MeshInstance3D = pane.actor.meshes[1]
	var transform: Transform3D = (
		pane.pivot.global_transform.affine_inverse() * face.global_transform
	)
	var positions: Array[Vector3] = []
	var triangles: Array[Vector3i] = []
	var corner_normals: Array[PackedVector3Array] = []
	var slots := {}
	var parents: Array[int] = []
	for surface in face.mesh.get_surface_count():
		var arrays: Array = face.mesh.surface_get_arrays(surface)
		var vertices: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
		var indices: PackedInt32Array = arrays[Mesh.ARRAY_INDEX]
		var normals: PackedVector3Array = arrays[Mesh.ARRAY_NORMAL]
		var normal_basis: Basis = transform.basis.inverse().transposed()
		var local_ids: Array[int] = []
		for vertex in vertices:
			var point: Vector3 = transform * vertex
			var key := Vector3i(
				roundi(point.x * 100000), roundi(point.y * 100000), roundi(point.z * 100000)
			)
			if not slots.has(key):
				slots[key] = positions.size()
				parents.append(positions.size())
				positions.append(point)
			local_ids.append(int(slots[key]))
		if indices.is_empty():
			for i in vertices.size():
				indices.append(i)
		for i in range(0, indices.size() - 2, 3):
			var a := local_ids[indices[i]]
			var b := local_ids[indices[i + 1]]
			var c := local_ids[indices[i + 2]]
			parents[root(parents, b)] = root(parents, a)
			parents[root(parents, c)] = root(parents, a)
			triangles.append(Vector3i(a, b, c))
			corner_normals.append(
				PackedVector3Array(
					[
						(normal_basis * normals[indices[i]]).normalized(),
						(normal_basis * normals[indices[i + 1]]).normalized(),
						(normal_basis * normals[indices[i + 2]]).normalized()
					]
				)
			)
	var groups := {}
	for i in positions.size():
		var component := root(parents, i)
		if not groups.has(component):
			groups[component] = []
		groups[component].append(i)
	var results: Array[Dictionary] = []
	for component in groups:
		var ids: Array = groups[component]
		var bounds := AABB(positions[int(ids[0])], Vector3.ZERO)
		for index in ids:
			bounds = bounds.expand(positions[int(index)])
		if (
			bounds.end.x < 0.08
			or bounds.position.x > 0.43
			or bounds.end.y < -0.2
			or bounds.position.y > 0.18
		):
			continue
		var bins: Array[Dictionary] = []
		for bin_index in range(6):
			var selected: Array[Vector3] = []
			var x_min: float = 0.08 + bin_index * 0.055
			for index in ids:
				var p: Vector3 = positions[int(index)]
				if p.x >= x_min and p.x < x_min + 0.055 and p.y > -0.2 and p.y < 0.18:
					selected.append(p)
			if selected.is_empty():
				continue
			var box := AABB(selected[0], Vector3.ZERO)
			var front: Vector3 = selected[0]
			var back: Vector3 = selected[0]
			for p in selected:
				box = box.expand(p)
				if p.z > front.z:
					front = p
				if p.z < back.z:
					back = p
			bins.append(
				{
					"x_band": [x_min, x_min + 0.055],
					"count": selected.size(),
					"height_range": [box.position.y, box.end.y],
					"depth_range": [box.position.z, box.end.z],
					"frontmost_height": front.y,
					"backmost_height": back.y
				}
			)
		var projected_bounds: Array[Dictionary] = []
		for degrees in [0, 45, 90, 135, 145]:
			var angle: float = deg_to_rad(float(-degrees))
			var left: float = INF
			var right: float = -INF
			for index in ids:
				var p: Vector3 = positions[int(index)]
				var horizontal: float = p.x * cos(angle) + p.z * sin(angle)
				left = minf(left, horizontal)
				right = maxf(right, horizontal)
			projected_bounds.append({"angle_degrees": degrees, "width_H": right - left})
		# Area-weighted aggregate moments, never raw reference vertices.
		var total_area: float = 0.0
		var first_moment: Vector3 = Vector3.ZERO
		var second_moment: Array[float] = []
		second_moment.resize(9)
		second_moment.fill(0.0)
		for tri in triangles:
			if root(parents, tri.x) != int(component):
				continue
			var pa: Vector3 = positions[tri.x]
			var pb: Vector3 = positions[tri.y]
			var pc: Vector3 = positions[tri.z]
			var area: float = (pb - pa).cross(pc - pa).length() * 0.5
			var sum_p: Vector3 = pa + pb + pc
			total_area += area
			first_moment += sum_p * (area / 3.0)
			for row in range(3):
				for col in range(3):
					second_moment[row * 3 + col] += (
						area
						* (
							sum_p[row] * sum_p[col]
							+ pa[row] * pa[col]
							+ pb[row] * pb[col]
							+ pc[row] * pc[col]
						)
						/ 12.0
					)
		var center: Vector3 = first_moment / maxf(total_area, 1e-12)
		for row in range(3):
			for col in range(3):
				second_moment[row * 3 + col] = (
					second_moment[row * 3 + col] / maxf(total_area, 1e-12)
					- center[row] * center[col]
				)
		results.append(
			{
				"component": component,
				"welded_vertex_count": ids.size(),
				"bounds_min": [bounds.position.x, bounds.position.y, bounds.position.z],
				"surface_area_H2": total_area,
				"surface_centroid_H": [center.x, center.y, center.z],
				"surface_covariance_H2": second_moment,
				"bounds_max": [bounds.end.x, bounds.end.y, bounds.end.z],
				"bands": bins,
				"projected_widths": projected_bounds
			}
		)
	var sections: Array[Dictionary] = []
	var probes: Array[Vector2] = []
	for px in [0.14, 0.18, 0.22, 0.24, 0.26, 0.28, 0.30, 0.34]:
		for py in [
			-0.07,
			-0.06,
			-0.05,
			-0.045,
			-0.04,
			-0.035,
			-0.03,
			-0.025,
			-0.02,
			-0.015,
			-0.01,
			0.0,
			0.01
		]:
			probes.append(Vector2(px, py))
	for px in [0.30, 0.32, 0.34, 0.36]:
		for py in [-0.06, -0.08, -0.10, -0.12]:
			probes.append(Vector2(px, py))
	for px in [0.10, 0.12, 0.14, 0.16]:
		for py in [-0.08, -0.07, -0.06, -0.05, -0.04, -0.03, -0.02]:
			probes.append(Vector2(px, py))
	for probe in probes:
		var px: float = probe.x
		var py: float = probe.y
		var front_z := -INF
		var normal := Vector3.ZERO
		var shading_normal := Vector3.ZERO
		var layers := {}
		for triangle_index in triangles.size():
			var triangle := triangles[triangle_index]
			var a: Vector3 = positions[triangle.x]
			var b: Vector3 = positions[triangle.y]
			var c: Vector3 = positions[triangle.z]
			var denominator: float = (b.y - c.y) * (a.x - c.x) + (c.x - b.x) * (a.y - c.y)
			if absf(denominator) < 1e-12:
				continue
			var u: float = ((b.y - c.y) * (px - c.x) + (c.x - b.x) * (py - c.y)) / denominator
			var v: float = ((c.y - a.y) * (px - c.x) + (a.x - c.x) * (py - c.y)) / denominator
			if u < 0 or v < 0 or u + v > 1:
				continue
			var z: float = u * a.z + v * b.z + (1 - u - v) * c.z
			var component_id := root(parents, triangle.x)
			if not layers.has(component_id) or z > float(layers[component_id]):
				layers[component_id] = z
			if z > front_z:
				front_z = z
				normal = (b - a).cross(c - a).normalized()
				var ns: PackedVector3Array = corner_normals[triangle_index]
				shading_normal = (ns[0] * u + ns[1] * v + ns[2] * (1 - u - v)).normalized()
		if is_finite(front_z):
			sections.append(
				{
					"x_H": px,
					"y_H": py,
					"front_depth_H": front_z,
					"component_front_depths_H": layers,
					"shading_vs_geometric_angle_degrees":
					rad_to_deg(acos(clampf(absf(shading_normal.dot(normal)), 0.0, 1.0))),
					"shading_front_angle_degrees":
					rad_to_deg(acos(clampf(shading_normal.dot(Vector3(0, 0, 1)), -1.0, 1.0))),
					"geometric_front_angle_degrees":
					rad_to_deg(acos(clampf(absf(normal.z), 0.0, 1.0)))
				}
			)
	var visibility: Array[Dictionary] = (
		preload("res://showcase_extensions/head_accent_visibility.gd")
		. summarize(positions, triangles, parents, groups)
	)
	var crossings: Array[Dictionary] = (
		preload("res://showcase_extensions/head_accent_crossing.gd")
		. summarize(positions, triangles, parents, groups, visibility)
	)
	var folder := "res://artifacts/eye_inner_audit"
	DirAccess.make_dir_recursive_absolute(folder)
	var tag: String = (
		"reference"
		if pane.source_path.contains("silver_wolf")
		else pane.source_path.get_base_dir().get_file()
	)
	var output := FileAccess.open(folder.path_join(tag + ".json"), FileAccess.WRITE)
	if output != null:
		output.store_string(
			JSON.stringify(
				{
					"source": pane.source_path,
					"source_face_height": pane.source_face_height,
					"measurement_only_weld_tolerance_H": 0.00001,
					"coordinates": "normalized display: x lateral, y height centered, z forward",
					"components": results,
					"front_surface_probes": sections,
					"short_leaf_visibility": visibility,
					"short_leaf_crossings": crossings
				},
				"  "
			)
		)
