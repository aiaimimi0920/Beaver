extends RefCounted


# Clip triangles against the rectangle's camera frustum, including near/far.
# This is a through-selection: occluded static meshes are intentionally included.
static func select(
	meshes: Array[Dictionary], matrix: Projection, rectangle: Dictionary
) -> Dictionary:
	var left: float = 2.0 * rectangle.x - 1.0
	var right: float = 2.0 * (rectangle.x + rectangle.width) - 1.0
	var top: float = 1.0 - 2.0 * rectangle.y
	var bottom: float = 1.0 - 2.0 * (rectangle.y + rectangle.height)
	var planes: Array[Vector4] = [
		Vector4(1, 0, 0, -left),
		Vector4(-1, 0, 0, right),
		Vector4(0, 1, 0, -bottom),
		Vector4(0, -1, 0, top),
		Vector4(0, 0, 1, 1),
		Vector4(0, 0, -1, 1)
	]
	var paths: Array[String] = []
	var truncated := false
	var path_bytes := 0
	for item in meshes:
		var faces: PackedVector3Array = item.faces
		for index in range(0, faces.size(), 3):
			var polygon: Array[Vector4] = []
			for offset in 3:
				var v := faces[index + offset]
				polygon.append(matrix * Vector4(v.x, v.y, v.z, 1))
			for plane in planes:
				polygon = clip(polygon, plane)
				if polygon.is_empty():
					break
			if polygon.is_empty():
				continue
			var bytes: int = item.path.to_utf8_buffer().size()
			if paths.size() == 32 or bytes > 4096 or path_bytes + bytes > 32768:
				truncated = true
			else:
				paths.append(item.path)
				path_bytes += bytes
			break
	return {
		"capability": "frozen-static-mesh-frustum",
		"hit": null,
		"nodePaths": paths,
		"truncated": truncated
	}


static func clip(polygon: Array[Vector4], plane: Vector4) -> Array[Vector4]:
	var result: Array[Vector4] = []
	if polygon.is_empty():
		return result
	var previous := polygon[-1]
	var previous_distance := plane.dot(previous)
	for current in polygon:
		var distance := plane.dot(current)
		if (distance >= 0.0) != (previous_distance >= 0.0):
			result.append(
				previous.lerp(current, previous_distance / (previous_distance - distance))
			)
		if distance >= 0.0:
			result.append(current)
		previous = current
		previous_distance = distance
	return result
