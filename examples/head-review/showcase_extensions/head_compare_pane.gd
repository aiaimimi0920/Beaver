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
	print("MODEL_COMPARE_MORPH=" + JSON.stringify({"source": source_path, "MouthOpen": mouth_index, "count": face.mesh.get_blend_shape_count()}))
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
	var bounds: AABB = normalized.global_transform.affine_inverse() * face.global_transform * base_bounds
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
	return true

func apply_state(state: Dictionary) -> void:
	if not is_instance_valid(actor) or not actor.initialized:
		return
	pivot.rotation_degrees = Vector3(state.pitch, state.yaw, 0)
	camera.set_orthogonal(state.zoom, 0.01, 100.0)
	actor.light_yaw = state.light
	actor.meshes[0].visible = state.body
	actor.meshes[2].visible = state.hair
	if display.mode != state.mode:
		display.set_mode(state.mode)

	if drive_mouth and mouth_index >= 0:
		actor.meshes[1].set_blend_shape_value(mouth_index, state.get("mouth", 0.0))
		display.sync_morphs()
