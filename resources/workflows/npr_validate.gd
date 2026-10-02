extends SceneTree
## Beaver metadata/capture adapter. Structural compliance belongs to check_model.gd.

var report: Dictionary = {
	"status": "error", "errors": [], "screenshots": [], "grayscaleScreenshots": []
}


func _initialize() -> void:
	_run.call_deferred()


func _finish(code: int) -> void:
	print("BEAVER_WORKFLOW_RESULT=" + JSON.stringify(report))
	quit(code)


func _run() -> void:
	var args := OS.get_cmdline_user_args()
	if args.size() != 4 or args[1] not in ["metadata", "preview"]:
		report.errors.append("Expected definition, metadata/preview, output and options")
		_finish(2)
		return
	var options: Variant = JSON.parse_string(args[3])
	if not options is Dictionary:
		report.errors.append("Preview options must be an object")
		_finish(2)
		return
	var definition: Resource = load("res://" + args[0])
	if definition == null or not definition.has_method("validate"):
		report.errors.append("Cannot load NPRCharacterDefinition")
		_finish(1)
		return
	if definition.model_scene == null:
		report.errors.append("Definition has no model_scene")
		_finish(1)
		return
	report.definition = args[0]
	report.model = definition.model_scene.resource_path
	report.modelSource = str(report.model).get_slice("::", 0)
	report.modelSha256 = FileAccess.get_sha256(report.modelSource)
	report.engine = Engine.get_version_info().string
	if args[1] == "metadata":
		report.status = "metadata"
		_finish(0)
		return
	var actor_script: Script = load("res://addons/npr_character_frame/npr_character.gd")
	var actor: Node3D = actor_script.new()
	actor.definition = definition
	root.add_child(actor)
	await process_frame
	report.errors = Array(actor.validation_errors)
	if not actor.initialized or not report.errors.is_empty():
		report.errors.append("NPR actor could not initialize for rendering")
		_finish(1)
		return
	var environment := WorldEnvironment.new()
	environment.environment = load(
		"res://addons/npr_character_frame/materials/studio_environment.tres"
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
		"target": [center.x, center.y, center.z], "distance": distance,
		"fov": camera.fov, "angles": angles,
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
			_finish(2)
			return
		report.screenshots.append(file)
		if options.get("grayscale", false):
			image.convert(Image.FORMAT_L8)
			var gray_file := file.trim_suffix(".png") + "-gray.png"
			if image.save_png("res://" + gray_file) != OK:
				report.errors.append("Cannot save grayscale preview")
				_finish(2)
				return
			report.grayscaleScreenshots.append(gray_file)
	report.status = "rendered"
	_finish(0)
