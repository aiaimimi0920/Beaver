extends RefCounted


## Coarse aggregate visibility only. No source vertices or images are exported.
static func summarize(
	positions: Array[Vector3], triangles: Array[Vector3i], parents: Array[int], groups: Dictionary
) -> Array[Dictionary]:
	var selected: Array[int] = []
	for component in groups:
		var ids: Array = groups[component]
		if ids.size() < 15 or ids.size() > 60:
			continue
		var box := AABB(positions[int(ids[0])], Vector3.ZERO)
		for index in ids:
			box = box.expand(positions[int(index)])
		if (
			box.position.x > 0.20
			and box.end.x < 0.31
			and box.position.y > -0.06
			and box.end.y < 0.03
		):
			selected.append(int(component))
	var labels: Array[int] = []
	for triangle in triangles:
		var label: int = triangle.x
		while parents[label] != label:
			label = parents[label]
		labels.append(label)
	var result: Array[Dictionary] = []
	for degrees in [0, 45, 90]:
		var angle: float = deg_to_rad(float(-degrees))
		var rotated: Array[Vector3] = []
		for p in positions:
			rotated.append(
				Vector3(
					p.x * cos(angle) + p.z * sin(angle), p.y, -p.x * sin(angle) + p.z * cos(angle)
				)
			)
		for component in selected:
			var ids: Array = groups[component]
			var box := AABB(rotated[int(ids[0])], Vector3.ZERO)
			for index in ids:
				box = box.expand(rotated[int(index)])
			var covered := 0
			var visible := 0
			var blockers := {}
			for ix in range(16):
				for iy in range(16):
					var x: float = box.position.x + box.size.x * (ix + 0.5) / 16.0
					var y: float = box.position.y + box.size.y * (iy + 0.5) / 16.0
					var top := -INF
					var target := -INF
					var top_label := -1
					for index in triangles.size():
						var tri: Vector3i = triangles[index]
						var a: Vector3 = rotated[tri.x]
						var b: Vector3 = rotated[tri.y]
						var c: Vector3 = rotated[tri.z]
						if (
							x < minf(a.x, minf(b.x, c.x))
							or x > maxf(a.x, maxf(b.x, c.x))
							or y < minf(a.y, minf(b.y, c.y))
							or y > maxf(a.y, maxf(b.y, c.y))
						):
							continue
						var den: float = (b.y - c.y) * (a.x - c.x) + (c.x - b.x) * (a.y - c.y)
						if absf(den) < 1e-12:
							continue
						var u: float = ((b.y - c.y) * (x - c.x) + (c.x - b.x) * (y - c.y)) / den
						var v: float = ((c.y - a.y) * (x - c.x) + (a.x - c.x) * (y - c.y)) / den
						if u < 0 or v < 0 or u + v > 1:
							continue
						var z: float = u * a.z + v * b.z + (1 - u - v) * c.z
						if labels[index] == component:
							target = maxf(target, z)
						if z > top:
							top = z
							top_label = labels[index]
					if is_finite(target):
						covered += 1
						if top - target < 0.0001:
							visible += 1
						else:
							blockers[top_label] = int(blockers.get(top_label, 0)) + 1
			var cell_area: float = box.size.x * box.size.y / 256.0
			result.append(
				{
					"component": component,
					"angle": degrees,
					"grid": 16,
					"covered_cells": covered,
					"visible_cells": visible,
					"projected_area_H2": covered * cell_area,
					"visible_area_H2": visible * cell_area,
					"occluders": blockers
				}
			)
	return result
