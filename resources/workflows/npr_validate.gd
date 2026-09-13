extends SceneTree

var report: Dictionary = {
	"initialized": false, "errors": [], "screenshots": [], "grayscaleScreenshots": []
}


func _initialize() -> void:
	_run.call_deferred()


func _finish(code: int) -> void:
	print("BEAVER_WORKFLOW_RESULT=" + JSON.stringify(report))
	quit(code)


func _run() -> void:
	var args := OS.get_cmdline_user_args()
	if args.size() != 4:
		report.errors.append("Expected definition, mode, output directory and preview options")
		_finish(1)
		return
	var options: Dictionary = JSON.parse_string(args[3])
	var definition: Resource = load("res://" + args[0])
	if definition == null or not definition.has_method("validate"):
		report.errors.append("Cannot load NPRCharacterDefinition")
		_finish(1)
		return
	report.definition = args[0]
	report.errors = Array(definition.validate())
	if not report.errors.is_empty():
		_finish(1)
		return
	var actor_script: Script = load("res://addons/npr_characters/npr_character.gd")
	var actor: Node3D = actor_script.new()
	actor.definition = definition
	root.add_child(actor)
	await process_frame
	report.initialized = actor.initialized
	report.errors = Array(actor.validation_errors)
	report.model = definition.model_scene.resource_path
	report.modelSource = str(report.model).get_slice("::", 0)
	report.modelSha256 = FileAccess.get_sha256(report.modelSource)
	report.engine = Engine.get_version_info().string
	if not actor.initialized or not report.errors.is_empty():
		_finish(1)
		return
	if args[1] == "preview":
		var environment := WorldEnvironment.new()
		environment.environment = load(
			"res://addons/npr_characters/materials/studio_environment.tres"
		)
		root.add_child(environment)
		root.msaa_3d = Viewport.MSAA_4X
		var camera := Camera3D.new()
		root.add_child(camera)
		camera.current = true
		var bounds: AABB = actor.get_world_bounds()
		var center := bounds.get_center()
		var distance: float = maxf(bounds.size.length() * 1.6, 2.0)
		var angles: Array = [0.0, 90.0, 180.0]
		if options.get("camera") is Dictionary:
			var framing: Dictionary = options.camera
			center = Vector3(framing.target[0], framing.target[1], framing.target[2])
			distance = float(framing.distance)
			camera.fov = float(framing.fov)
			angles = framing.angles
		report.camera = {
			"target": [center.x, center.y, center.z],
			"distance": distance,
			"fov": camera.fov,
			"angles": angles,
		}
		for degrees in angles:
			var angle := deg_to_rad(float(degrees))
			camera.position = center + Vector3(sin(angle), 0.1, cos(angle)) * distance
			camera.look_at(center, Vector3.UP)
			for frame in range(12):
				await process_frame
			await RenderingServer.frame_post_draw
			var image: Image = root.get_texture().get_image()
			var file := args[2] + "/view-" + str(report.screenshots.size()) + ".png"
			if image == null or image.is_empty() or image.save_png("res://" + file) != OK:
				report.errors.append("Cannot save preview image")
				_finish(1)
				return
			report.screenshots.append(file)
			if options.get("grayscale", false):
				image.convert(Image.FORMAT_L8)
				var gray_file := file.trim_suffix(".png") + "-gray.png"
				if image.save_png("res://" + gray_file) != OK:
					report.errors.append("Cannot save grayscale preview")
					_finish(1)
					return
				report.grayscaleScreenshots.append(gray_file)
	_finish(0)
