extends RefCounted

# Copy geometry at the rendered frame boundary; never query a later live scene.
const MAX_TRIANGLES := 100000
var meshes: Array[Dictionary] = []
var transform: Transform3D
var inverse_projection: Projection
var skipped := 0
var triangles := 0


func freeze(root: Node, camera: Camera3D) -> Dictionary:
	meshes.clear()
	skipped = 0
	triangles = 0
	transform = camera.global_transform
	inverse_projection = camera.get_camera_projection().inverse()
	collect(root, camera.cull_mask)
	return {"capability": "frozen-static-mesh-ray", "triangles": triangles, "skipped": skipped}


func collect(node: Node, mask: int) -> void:
	if node is MeshInstance3D and node.is_visible_in_tree() and node.layers & mask:
		var mesh: Mesh = node.mesh
		var supported := mesh != null and node.skin == null and node.material_overlay == null
		if supported and mesh is ArrayMesh:
			supported = mesh.get_blend_shape_count() == 0
		if supported:
			for surface in mesh.get_surface_count():
				var material: Material = node.get_active_material(surface)
				if material != null and material.next_pass != null:
					supported = false
				if material is ShaderMaterial:
					supported = false
				if material is BaseMaterial3D:
					if (
						material.transparency != BaseMaterial3D.TRANSPARENCY_DISABLED
						or material.billboard_mode != BaseMaterial3D.BILLBOARD_DISABLED
						or material.grow
					):
						supported = false
		if supported:
			var faces := mesh.get_faces()
			if triangles + faces.size() / 3 > MAX_TRIANGLES:
				skipped += 1
			else:
				for index in faces.size():
					faces[index] = node.global_transform * faces[index]
				triangles += faces.size() / 3
				meshes.append({"path": str(root_path(node)), "faces": faces})
		else:
			skipped += 1
	elif node is GeometryInstance3D and node.is_visible_in_tree():
		skipped += 1
	for child in node.get_children():
		collect(child, mask)


func root_path(node: Node) -> NodePath:
	return node.get_tree().current_scene.get_path_to(node)


func unproject(point: Dictionary, depth: float) -> Vector3:
	var clip := inverse_projection * Vector4(2.0 * point.x - 1.0, 1.0 - 2.0 * point.y, depth, 1.0)
	return transform * (Vector3(clip.x, clip.y, clip.z) / clip.w)


func pick(point: Dictionary) -> Dictionary:
	var start := unproject(point, -1.0)
	var end := unproject(point, 1.0)
	var nearest := INF
	var hit: Variant = null
	for item in meshes:
		var faces: PackedVector3Array = item.faces
		for index in range(0, faces.size(), 3):
			var position: Variant = Geometry3D.segment_intersects_triangle(
				start, end, faces[index], faces[index + 1], faces[index + 2]
			)
			if position == null:
				continue
			var distance: float = start.distance_to(position)
			if distance < nearest:
				nearest = distance
				var normal := (
					(faces[index + 2] - faces[index])
					. cross(faces[index + 1] - faces[index])
					. normalized()
				)
				hit = {
					"nodePath": item.path,
					"triangle": index / 3,
					"position": vector(position),
					"normal": vector(normal),
					"distance": distance
				}
	return {
		"capability": "frozen-static-mesh-ray",
		"hit": hit,
		"skipped": skipped,
		"triangles": triangles
	}


func vector(value: Vector3) -> Array:
	return [value.x, value.y, value.z]


func pick_box(rectangle: Dictionary) -> Dictionary:
	var matrix := inverse_projection.inverse() * Projection(transform.affine_inverse())
	var result: Dictionary = preload("res://beaver_live_preview/box_picking.gd").select(
		meshes, matrix, rectangle
	)
	result.skipped = skipped
	result.triangles = triangles
	return result
