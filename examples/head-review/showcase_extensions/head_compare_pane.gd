extends SubViewportContainer
## One normalized model viewport. Source resources are never edited.

const DISPLAY = preload("res://showcase_extensions/head_display.gd")
var actor: NPRCharacter
var camera: Camera3D
var pivot: Node3D
var display
var source_path := ""
var drive_mouth := false
var mouth_index := -1
var blink_indices := {}
var source_face_height := 0.0


func configure(definition: NPRCharacterDefinition, neutralize_head: bool) -> bool:
	stretch = true
	mouse_filter = Control.MOUSE_FILTER_STOP
	size_flags_horizontal = Control.SIZE_EXPAND_FILL
	size_flags_vertical = Control.SIZE_EXPAND_FILL
	custom_minimum_size = Vector2(120, 220)
	source_path = definition.resource_path
	var view := SubViewport.new()
	view.own_world_3d = true
	view.transparent_bg = true
	view.msaa_3d = Viewport.MSAA_4X
	view.render_target_update_mode = SubViewport.UPDATE_ALWAYS
	add_child(view)
	var studio := Node3D.new()
	view.add_child(studio)
	var world := WorldEnvironment.new()
	var environment := Environment.new()
	environment.background_mode = Environment.BG_COLOR
	environment.background_color = Color(0, 0, 0, 0)
	environment.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	environment.ambient_light_color = Color(0.66, 0.7, 0.85)
	environment.ambient_light_energy = 0.6
	world.environment = environment
	studio.add_child(world)
	camera = Camera3D.new()
	camera.position = Vector3(0, 0, 3)
	camera.near = 0.01
	camera.far = 100.0
	studio.add_child(camera)
	camera.look_at(Vector3.ZERO, Vector3.UP)
	camera.make_current()
	pivot = Node3D.new()
	studio.add_child(pivot)
	var normalized := Node3D.new()
	pivot.add_child(normalized)
	actor = NPRCharacter.new()
	actor.definition = definition.duplicate()
	normalized.add_child(actor)
	if not actor.initialized:
		return false
	if neutralize_head:
		var head := actor.character.get_node_or_null("Head") as Node3D
		if head != null:
			head.rotation = Vector3.ZERO
	var face: MeshInstance3D = actor.meshes[1]
	mouth_index = face.find_blend_shape_by_name("MouthOpen")
	for key in ["blink.L", "blink.R"]:
		blink_indices[key] = face.find_blend_shape_by_name(key)
	print(
		(
			"MODEL_COMPARE_MORPH="
			+ JSON.stringify(
				{
					"source": source_path,
					"MouthOpen": mouth_index,
					"count": face.mesh.get_blend_shape_count()
				}
			)
		)
	)
	var base_bounds := AABB()
	var has_vertex := false
	for surface in face.mesh.get_surface_count():
		var positions: PackedVector3Array = face.mesh.surface_get_arrays(surface)[Mesh.ARRAY_VERTEX]
		for position in positions:
			if not has_vertex:
				base_bounds = AABB(position, Vector3.ZERO)
				has_vertex = true
			else:
				base_bounds = base_bounds.expand(position)
	if not has_vertex:
		return false
	var bounds: AABB = (
		normalized.global_transform.affine_inverse() * face.global_transform * base_bounds
	)
	source_face_height = bounds.size.y
	if not is_finite(source_face_height) or source_face_height <= 0.0:
		return false
	actor.position = -bounds.get_center()
	normalized.scale = Vector3.ONE / source_face_height
	actor.meshes[0].visible = false
	actor.meshes[2].visible = false
	actor.set_sdf_feather(0.06)
	display = DISPLAY.new()
	actor.add_child(display)
	display.setup(actor, null)
	preload("res://showcase_extensions/head_geometry_audit.gd").summarize(self)
	return true


func apply_state(state: Dictionary) -> void:
	if not is_instance_valid(actor) or not actor.initialized:
		return
	pivot.rotation_degrees = Vector3(state.pitch, state.yaw, 0)
	camera.set_orthogonal(state.zoom, 0.01, 100.0)
	camera.position.x = state.get("pan_x", 0.0)
	camera.position.y = state.get("pan_y", 0.0)
	actor.light_yaw = state.light
	actor.light_elevation = state.get("elevation", 25.0)
	if supports_sdf_controls(actor.materials[1]):
		actor.materials[1].set_shader_parameter("u_npr_sdf360_enabled", state.get("sdf360", true))
		actor.materials[1].set_shader_parameter("u_npr_sdf360_debug", state.get("sdf_debug", false))
	actor.meshes[0].visible = state.body
	actor.meshes[2].visible = state.hair
	if display.mode != state.mode:
		display.set_mode(state.mode)

	if drive_mouth:
		if mouth_index >= 0:
			actor.meshes[1].set_blend_shape_value(mouth_index, state.get("mouth", 0.0))
		if supports_blinks():
			for key in blink_indices:
				actor.meshes[1].set_blend_shape_value(blink_indices[key], state.get(key, 0.0))
		display.sync_morphs()


static func supports_sdf_controls(material: ShaderMaterial) -> bool:
	if material == null or material.shader == null:
		return false
	var names := {}
	for entry in material.shader.get_shader_uniform_list():
		if entry.type == TYPE_BOOL:
			names[String(entry.name)] = true
	return names.has("u_npr_sdf360_enabled") and names.has("u_npr_sdf360_debug")


func sdf_control_state() -> Dictionary:
	if not is_instance_valid(actor) or not actor.initialized:
		return {"available": false}
	var material: ShaderMaterial = actor.materials[1]
	if not supports_sdf_controls(material):
		return {"available": false}
	var atlas = material.get_shader_parameter("u_npr_sdf360_atlas")
	return {
		"available": true,
		"atlas_bound": atlas is Texture2D and atlas.get_width() > 0,
		"atlas_path": atlas.resource_path if atlas is Texture2D else "",
		"render_material_active":
		display.mode == "render" and actor.meshes[1].get_active_material(0) == material,
		"enabled": material.get_shader_parameter("u_npr_sdf360_enabled"),
		"debug": material.get_shader_parameter("u_npr_sdf360_debug")
	}


func supports_blinks() -> bool:
	return blink_indices.get("blink.L", -1) >= 0 and blink_indices.get("blink.R", -1) >= 0


func morph_state() -> Dictionary:
	if not is_instance_valid(actor) or not actor.initialized:
		return {}
	var face: MeshInstance3D = actor.meshes[1]
	var result := {}
	for key in ["MouthOpen", "blink.L", "blink.R"]:
		var index := face.find_blend_shape_by_name(key)
		if index >= 0:
			result[key] = face.get_blend_shape_value(index)
	return result
