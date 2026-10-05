extends RefCounted
## Stage assembly reuses the NPR wardrobe's production actor and presentation.

static func build(host: Control) -> bool:
	var backdrop := ColorRect.new()
	backdrop.mouse_filter = Control.MOUSE_FILTER_IGNORE
	backdrop.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	host._backdrop_material = ShaderMaterial.new()
	host._backdrop_material.shader = load("res://addons/npr_character_frame/showcase/shaders/wardrobe_background.gdshader")
	backdrop.material = host._backdrop_material
	host.add_child(backdrop)
	host._stage = SubViewportContainer.new()
	host._stage.name = "Stage"
	host._stage.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	host._stage.stretch = true
	host._stage.gui_input.connect(host._stage_input)
	host.add_child(host._stage)
	host.viewport = SubViewport.new()
	host.viewport.own_world_3d = true
	host.viewport.transparent_bg = true
	host.viewport.msaa_3d = Viewport.MSAA_4X
	host.viewport.render_target_update_mode = SubViewport.UPDATE_ALWAYS
	host._stage.add_child(host.viewport)
	var studio := Node3D.new()
	host.viewport.add_child(studio)
	var world := WorldEnvironment.new()
	host._environment = Environment.new()
	host._environment.background_mode = Environment.BG_COLOR
	host._environment.background_color = Color(0, 0, 0, 0)
	host._environment.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	host._environment.ambient_light_color = Color(0.66, 0.7, 0.85)
	host._environment.ambient_light_energy = 0.6
	world.environment = host._environment
	studio.add_child(world)
	host.camera = Camera3D.new()
	host.camera.fov = 36.0
	host.camera.near = 0.03
	studio.add_child(host.camera)
	host.turntable = Node3D.new()
	studio.add_child(host.turntable)
	host.preview = NPRCharacter.new()
	host.preview.definition = host.character_definition.duplicate()
	host.turntable.add_child(host.preview)
	if not host.preview.initialized:
		return false
	host.preview.set_sdf_feather(0.06)
	host.display = load("res://showcase_extensions/head_display.gd").new()
	host.preview.add_child(host.display)
	host.display.setup(host.preview, null)
	return true
