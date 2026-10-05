extends "res://addons/npr_character_frame/showcase/wardrobe_display.gd"
## Explicit static-head diagnostic mode. No rig or dynamic acceptance is implied.

var _wire_nodes: Array[MeshInstance3D] = []
var _flat: ShaderMaterial
var _original_visibility: Dictionary = {}

func setup(actor: NPRCharacter, _unused_driver: Node) -> void:
	_actor = actor
	white_material = StandardMaterial3D.new()
	white_material.albedo_color = Color("d5dbe2")
	white_material.roughness = 1.0
	white_material.cull_mode = BaseMaterial3D.CULL_DISABLED
	_flat = ShaderMaterial.new()
	_flat.shader = load("res://showcase_extensions/head_facets.gdshader")
	for role in [1, 2]:
		var source: MeshInstance3D = actor.meshes[role]
		var arrays: Array = source.mesh.surface_get_arrays(0)
		var positions: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
		var indices: PackedInt32Array = arrays[Mesh.ARRAY_INDEX]
		if indices.is_empty():
			for i in positions.size():
				indices.append(i)
		var vertices := PackedVector3Array()
		var colors := PackedColorArray()
		for i in indices.size():
			vertices.append(positions[indices[i]])
			colors.append([Color(1, 0, 0), Color(0, 1, 0), Color(0, 0, 1)][i % 3])
		var wire_arrays := []
		wire_arrays.resize(Mesh.ARRAY_MAX)
		wire_arrays[Mesh.ARRAY_VERTEX] = vertices
		wire_arrays[Mesh.ARRAY_COLOR] = colors
		var mesh := ArrayMesh.new()
		mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, wire_arrays)
		var wire := MeshInstance3D.new()
		wire.mesh = mesh
		var wire_material := ShaderMaterial.new()
		wire_material.shader = load("res://showcase_extensions/head_edges.gdshader")
		wire.material_override = wire_material
		wire.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		source.add_child(wire)
		wire.visible = false
		_wire_nodes.append(wire)

func set_mode(value: String) -> void:
	if value not in ["render", "white", "facet", "edges"]:
		return
	restore_materials()
	for node in _original_visibility:
		if is_instance_valid(node):
			node.visible = _original_visibility[node]
	_original_visibility.clear()
	mode = value
	for wire in _wire_nodes:
		wire.visible = value == "edges"
	if value == "render":
		return
	for node in _actor.find_children("*", "GeometryInstance3D", true, false):
		if node in _wire_nodes:
			continue
		if node in _actor.meshes:
			material_snapshots.append({"geometry": node, "override": node.material_override, "overlay": node.material_overlay})
			node.material_override = _flat if value in ["facet", "edges"] else white_material
			node.material_overlay = null
		else:
			_original_visibility[node] = node.visible
			node.visible = false

func refresh() -> void:
	set_mode(mode)
