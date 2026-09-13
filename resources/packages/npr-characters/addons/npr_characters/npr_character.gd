class_name NPRCharacter
extends Node3D
## Isolated display instance. Never modifies the imported meshes or source materials.

const OUTLINE = preload("res://addons/npr_characters/shaders/face/face_outline_pixels.gdshader")
const SHADOW_RIG = preload("res://addons/npr_characters/runtime/character_shadow.gd")
const VALIDATOR = preload("res://addons/npr_characters/runtime/npr_model_validator.gd")
const DEPTH_PASS = preload("res://addons/npr_characters/runtime/character_depth.gd")
const GEOMETRY = preload("res://addons/npr_characters/runtime/npr_geometry_state.gd")
const ACTOR_LAYERS = preload("res://addons/npr_characters/runtime/npr_actor_layers.gd")
const MAX_ACTOR_LAYERS := ACTOR_LAYERS.MAX_ACTOR_LAYERS

@export var definition: NPRCharacterDefinition

var material_profile: NPRMaterialProfile
var validation_errors := PackedStringArray()
var initialized := false

var character: Node3D
var meshes: Array[MeshInstance3D] = []
var materials: Array[ShaderMaterial] = []
var outlines: Array[ShaderMaterial] = []
var model_bounds: AABB
var light_yaw := -45.0
var light_elevation := 45.0
var depth_pass: Node
var fill_light: OmniLight3D
var isolation_available: bool:
	get:
		return _light_layer != 0
var _geometry: Array[NPRGeometryState] = []
var _light_layer: int = 0
var _isolation_ready := false
var _applied_light_angles := Vector2(INF, INF)
var _initial_style: Dictionary[StringName, float] = {
	&"set_ramp_mix": 0.45,
	&"set_sdf_feather": 0.015,
	&"set_shadow_strength": 0.28,
	&"set_hair_highlight": 0.30,
	&"set_hair_contact": 0.35,
	&"set_fill_strength": 0.0,
	&"set_rim_strength": 0.10,
	&"set_outline_width": 1.0,
	&"set_depth_quality": 2.0
}


func _ready() -> void:
	process_priority = -20
	if definition == null:
		_fail(PackedStringArray(["NPRCharacter.definition is required"]))
		return
	validation_errors = definition.validate()
	if not validation_errors.is_empty():
		_fail(validation_errors)
		return
	var instance := definition.model_scene.instantiate()
	if not instance is Node3D:
		instance.free()
		_fail(PackedStringArray(["model_scene root must be Node3D"]))
		return
	var model := instance as Node3D
	_strip_capture_nodes(model)
	validation_errors = VALIDATOR.validate(
		model,
		definition.mesh_paths(),
		definition.use_source_materials,
		definition.material_set.sdf_on_uv2 if definition.material_set != null else false
	)
	if not validation_errors.is_empty():
		model.free()
		_fail(validation_errors)
		return
	material_profile = definition.material_profile
	character = _create_rig(model)
	ACTOR_LAYERS.register(self, _apply_isolation_layer)
	var built: Array[ShaderMaterial] = []
	if not definition.use_source_materials:
		built = definition.material_set.build()
	for path in character.mesh_paths:
		var mesh := character.get_node(path) as MeshInstance3D
		meshes.append(mesh)
		var material: ShaderMaterial
		if definition.use_source_materials:
			material = _copy_chain(mesh.get_active_material(0) as ShaderMaterial)
		else:
			material = built[materials.size()]
		mesh.material_override = null
		mesh.set_surface_override_material(0, material)
		materials.append(material)
	add_child(character)
	for index in range(meshes.size()):
		_geometry.append(GEOMETRY.attach(meshes[index], index))
	_align_character()
	_apply_display_preset()
	_setup_light_isolation()
	_apply_light()
	character.share_key_light(_light_layer)
	depth_pass = DEPTH_PASS.new()
	add_child(depth_pass)
	depth_pass.setup(meshes, materials)
	initialized = true
	for method in _initial_style:
		call(
			method,
			(
				int(_initial_style[method])
				if method == &"set_depth_quality"
				else _initial_style[method]
			)
		)


func _create_rig(model: Node3D) -> Node3D:
	if model.get_script() == SHADOW_RIG:
		model.mesh_paths = definition.mesh_paths()
		return model
	var rig := SHADOW_RIG.new()
	model.name = "Model"
	rig.add_child(model)
	for path in definition.mesh_paths():
		rig.mesh_paths.append(NodePath("Model/" + str(path)))
	return rig


func _fail(errors: PackedStringArray) -> void:
	validation_errors = errors
	set_process(false)
	push_error("NPR character rejected: " + "; ".join(errors))


func _process(_delta: float) -> void:
	# Keep the key in world space while the turntable rotates the complete character.
	if initialized:
		_apply_light()


func _notification(what: int) -> void:
	if what == NOTIFICATION_ENTER_WORLD or what == NOTIFICATION_EXIT_WORLD:
		ACTOR_LAYERS.invalidate()
	# A detached/reparented preview still owns its private layers. Releasing on
	# exit-tree lets the next actor reuse a live actor's fill/shadow isolation mask.
	if what == NOTIFICATION_PREDELETE:
		ACTOR_LAYERS.unregister(self)


func _strip_capture_nodes(node: Node) -> void:
	for child in node.get_children():
		if child is Light3D or child is Camera3D or child is WorldEnvironment:
			child.free()
		else:
			_strip_capture_nodes(child)


func _copy_chain(source: ShaderMaterial) -> ShaderMaterial:
	var copy := source.duplicate() as ShaderMaterial
	if source.next_pass is ShaderMaterial:
		copy.next_pass = _copy_chain(source.next_pass as ShaderMaterial)
	return copy


func _align_character() -> void:
	var to_local := global_transform.affine_inverse()
	var bounds := to_local * meshes[0].global_transform * meshes[0].get_aabb()
	for mesh in meshes.slice(1):
		bounds = bounds.merge(to_local * mesh.global_transform * mesh.get_aabb())
	var factor := definition.display_height / bounds.size.y
	if definition.display_height > 0.0:
		character.scale *= factor
		# The pivot is at ground level, horizontally centered on the whole character.
		character.position = (
			(
				character.position
				- Vector3(bounds.get_center().x, bounds.position.y, bounds.get_center().z)
			)
			* factor
		)
	model_bounds = bounds
	character.shadow_light.shadow_bias = 0.1
	character.shadow_light.shadow_normal_bias = 2.0
	character.shadow_light.directional_shadow_max_distance = 15.0
	character.shadow_light.directional_shadow_blend_splits = true
	character.shadow_light.light_energy = 1.1
	character.shadow_light.light_angular_distance = 0.75


func _apply_display_preset() -> void:
	# Set only duplicated material values, never global shader state.
	for material in [materials[0], materials[1], materials[2], materials[2].next_pass]:
		material.set_shader_parameter("u_npr_studio_enabled", true)
	var body := materials[0]
	for feature in ["height_lerp", "bloom", "special_effect", "fog", "dithering"]:
		body.set_shader_parameter("u_debug_frag_%s_enabled" % feature, false)
	body.set_shader_parameter("u_new_local_light", Vector4.ZERO)
	body.set_shader_parameter("u_contrast_switch", false)
	body.set_shader_parameter("u_enable_metal_reflection_flag", false)
	body.set_shader_parameter("u_character_toon_ramp_mode_compensation", 0.0)
	# The imported RGBA8 LUT clipped its exponent row to 1. Use an explicit
	# display-only artistic exponent, retaining the material's other LUT channels.
	# assert expressions are not executed in release builds. Apply the profile
	# unconditionally so release keeps the same authored exponents as the editor.
	if not material_profile.apply_to(body):
		push_error("Invalid eight-slot NPR material profile")
	body.next_pass = _make_outline(Color(0.13, 0.09, 0.20))
	var face := materials[1]
	# Authored face SDF owns the facial terminator. Generic mesh self-shadows
	# break the graphic nose/cheek shapes; keep CSM on body, hair and the stage.
	meshes[1].material_overlay = null
	face.set_shader_parameter("u_bloom_no_mid_baked_mask_intensity", 0.0)
	# Both at one disable the two vertex-alpha remaps (zero intensity darkens skin).
	face.set_shader_parameter("u_emission_threshold", 1.0)
	face.set_shader_parameter("u_emission_intensity", 1.0)
	face.next_pass.next_pass = _make_outline(Color(0.24, 0.12, 0.18))
	var hair := materials[2]
	var eye_hair := hair.next_pass as ShaderMaterial
	for material in [hair, eye_hair]:
		material.set_shader_parameter("u_npr_native_shadow_contrast", true)
		material.set_shader_parameter("g_u_level_adjust_on", false)
		material.set_shader_parameter("u_accent_split_tone_strength", 0.0)
		material.set_shader_parameter("u_accent_additive_highlight_strength", 0.0)
		material.set_shader_parameter("u_rim_shadow_width", 0.0)
	eye_hair.next_pass = _make_outline(Color(0.16, 0.13, 0.25))


func _make_outline(color: Color) -> ShaderMaterial:
	var material := ShaderMaterial.new()
	material.shader = OUTLINE
	material.set_shader_parameter("outline_color", color)
	outlines.append(material)
	return material


func _setup_light_isolation() -> void:
	# Actor/fill layers and real directional-light slots are different budgets.
	# Compatible actors share a key; the pool accounts for the eight-light cap.
	fill_light = OmniLight3D.new()
	fill_light.name = "ArtFillLight"
	fill_light.position = Vector3(1.0, 2.3, 1.0)
	fill_light.light_color = Color(0.52, 0.68, 1.0)
	fill_light.light_energy = 2.0
	fill_light.omni_range = 4.0
	fill_light.light_cull_mask = _light_layer
	fill_light.hide()
	add_child(fill_light)
	_isolation_ready = true
	_apply_isolation_layer(_light_layer)
	character.shadow_light.light_cull_mask = _light_layer | 1
	character.shadow_light.shadow_caster_mask = _light_layer | 1


func _apply_isolation_layer(layer: int) -> void:
	_light_layer = layer
	if not _isolation_ready:
		return
	for mesh in meshes:
		mesh.layers = layer
		mesh.get_node("ShadowCaster").layers = layer
	fill_light.light_cull_mask = layer
	character.update_shared_layer(layer)


func _apply_light() -> void:
	var angles := Vector2(light_yaw, light_elevation)
	if angles == _applied_light_angles:
		return
	_applied_light_angles = angles
	var yaw := deg_to_rad(light_yaw)
	var elevation := deg_to_rad(light_elevation)
	var direction := Vector3(sin(yaw) * cos(elevation), sin(elevation), cos(yaw) * cos(elevation))
	meshes[0].set_instance_shader_parameter(
		"u_custom_main_light_dir", Vector4(direction.x, direction.y, direction.z, 1.0)
	)


func _defer_style(method: StringName, value: float) -> bool:
	if initialized:
		return false
	_initial_style[method] = value
	return true


func set_ramp_mix(value: float) -> void:
	if _defer_style(&"set_ramp_mix", value):
		return
	for material in [materials[0], materials[2], materials[2].next_pass]:
		material.set_shader_parameter("u_npr_ramp_mix", value)


func set_fill_strength(value: float) -> void:
	if _defer_style(&"set_fill_strength", value):
		return
	for material in [materials[0], materials[2], materials[2].next_pass]:
		material.set_shader_parameter("u_npr_fill_strength", value)
	fill_light.visible = value > 0.0


func set_sdf_feather(value: float) -> void:
	if _defer_style(&"set_sdf_feather", value):
		return
	materials[1].set_shader_parameter("u_sdf_feather_radius", value)


func set_shadow_strength(value: float) -> void:
	if _defer_style(&"set_shadow_strength", value):
		return
	character.shadow_strength = value
	character.set_shadows_enabled(value > 0.0)


func set_outline_width(value: float) -> void:
	if _defer_style(&"set_outline_width", value):
		return
	for material in outlines:
		material.set_shader_parameter("outline_width_pixels", value)


func set_hair_highlight(value: float) -> void:
	if _defer_style(&"set_hair_highlight", value):
		return
	for material in [materials[2], materials[2].next_pass]:
		material.set_shader_parameter("u_hair_highlight_strength", value)


func set_hair_contact(value: float) -> void:
	if _defer_style(&"set_hair_contact", value):
		return
	materials[1].set_shader_parameter("u_npr_contact_strength", value)


func set_depth_quality(value: int) -> void:
	if _defer_style(&"set_depth_quality", value):
		return
	depth_pass.quality = clampi(value, DEPTH_PASS.Quality.PERFORMANCE, DEPTH_PASS.Quality.FULL)


func set_rim_strength(value: float) -> void:
	if _defer_style(&"set_rim_strength", value):
		return
	for material in [materials[0], materials[2], materials[2].next_pass]:
		material.set_shader_parameter("u_npr_rim_strength", value)


func get_world_bounds() -> AABB:
	var bounds := AABB()
	var initialized := false
	for state in _geometry:
		if not is_instance_valid(state.source) or not state.source.is_visible_in_tree():
			continue
		state.refresh()
		if state.source.mesh == null:
			continue
		var current := state.source.global_transform * state.local_bounds
		bounds = bounds.merge(current) if initialized else current
		initialized = true
	return bounds


func pick_surface(origin: Vector3, direction: Vector3) -> Dictionary:
	var nearest := {}
	var distance := INF
	var camera := get_viewport().get_camera_3d()
	var cull_mask := camera.cull_mask if camera != null else 0xFFFFF
	for state in _geometry:
		var mesh := state.source
		if (
			not is_instance_valid(mesh)
			or not mesh.is_visible_in_tree()
			or (mesh.layers & cull_mask) == 0
		):
			continue
		var hit := state.intersect_ray(origin, direction)
		if hit.is_empty():
			continue
		if hit.distance < distance:
			distance = hit.distance
			nearest = hit
	return nearest
