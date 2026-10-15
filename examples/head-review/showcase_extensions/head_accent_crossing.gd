extends RefCounted


## Read-only aggregate non-coplanar crossings; source mesh arrays never leave memory.
static func segment_hit(a: Vector3, b: Vector3, p: Vector3, q: Vector3, r: Vector3) -> Variant:
	var direction := b - a
	var e1 := q - p
	var e2 := r - p
	var h := direction.cross(e2)
	var det := e1.dot(h)
	if absf(det) < 1e-10:
		return null
	var f := 1.0 / det
	var s := a - p
	var u := f * s.dot(h)
	if u < -1e-8 or u > 1.00000001:
		return null
	var cross_q := s.cross(e1)
	var v := f * direction.dot(cross_q)
	var t := f * e2.dot(cross_q)
	if v < -1e-8 or u + v > 1.00000001 or t < 0 or t > 1:
		return null
	return a + t * direction


static func summarize(
	positions: Array[Vector3],
	triangles: Array[Vector3i],
	parents: Array[int],
	groups: Dictionary,
	visibility: Array[Dictionary]
) -> Array[Dictionary]:
	var chosen := {}
	for item in visibility:
		var component: int = item.component
		if not chosen.has(component):
			chosen[component] = {}
		for blocker in item.occluders:
			chosen[component][int(blocker)] = (
				int(chosen[component].get(int(blocker), 0)) + int(item.occluders[blocker])
			)
	var labels: Array[int] = []
	for tri in triangles:
		var label := tri.x
		while parents[label] != label:
			label = parents[label]
		labels.append(label)
	var result: Array[Dictionary] = []
	for component in chosen:
		var main := -1
		var score := -1
		for blocker in chosen[component]:
			if int(chosen[component][blocker]) > score:
				main = int(blocker)
				score = int(chosen[component][blocker])
		if main < 0:
			continue
		var ids: Array = groups[component]
		var box := AABB(positions[int(ids[0])], Vector3.ZERO)
		for id in ids:
			box = box.expand(positions[int(id)])
		var leaf: Array[Vector3i] = []
		var support: Array[Vector3i] = []
		for i in triangles.size():
			var tri: Vector3i = triangles[i]
			if labels[i] == int(component):
				leaf.append(tri)
			elif labels[i] == main:
				var tb := AABB(positions[tri.x], Vector3.ZERO).expand(positions[tri.y]).expand(
					positions[tri.z]
				)
				if tb.grow(.0001).intersects(box):
					support.append(tri)
		var samples := 0
		var minimum := INF
		var maximum := -INF
		var beyond := 0
		for direction in [0, 1]:
			var first: Array[Vector3i] = leaf if direction == 0 else support
			var second: Array[Vector3i] = support if direction == 0 else leaf
			for tri in first:
				var vertices := [tri.x, tri.y, tri.z]
				for edge in range(3):
					for other in second:
						var hit: Variant = segment_hit(
							positions[vertices[edge]],
							positions[vertices[(edge + 1) % 3]],
							positions[other.x],
							positions[other.y],
							positions[other.z]
						)
						if hit != null:
							var progress: float = (hit.x - box.position.x) / box.size.x
							samples += 1
							minimum = minf(minimum, progress)
							maximum = maxf(maximum, progress)
							if progress > .1:
								beyond += 1
		(
			result
			. append(
				{
					"leaf_component": component,
					"support_component": main,
					"intersection_samples": samples,
					"beyond_inner_tenth_samples": beyond,
					"minimum_progress": minimum if samples > 0 else null,
					"maximum_progress": maximum if samples > 0 else null,
					"scope":
					"noncoplanar crossings against principal visible occluder; attachment intersections may be intentional"
				}
			)
		)
	return result
