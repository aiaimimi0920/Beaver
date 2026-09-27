extends Node

const CameraController = preload("res://beaver_live_preview/camera.gd")
const Picking = preload("res://beaver_live_preview/picking.gd")
var picking = Picking.new()
var pick_metadata: Dictionary = {}
var last_pick := ""
var config: Dictionary
var controller: RefCounted
var sequence := 0
var revision := 0
var frozen := false
var captured_revision := -1


func _enter_tree() -> void:
	process_mode = Node.PROCESS_MODE_ALWAYS
	config = JSON.parse_string(
		FileAccess.get_file_as_string("res://beaver_live_preview/config.json")
	)
	get_tree().root.size = Vector2i(int(config.width), int(config.height))
	Engine.max_fps = 30


func _ready() -> void:
	call_deferred("run_preview")


func run_preview() -> void:
	for index in 30:
		await get_tree().process_frame
	controller = CameraController.new()
	var error: String = controller.initialize(self)
	if not error.is_empty():
		write_record({"sessionId": config.sessionId, "error": error})
		push_error(error)
		get_tree().quit(3)
		return
	while true:
		var path: String = config.output + "/command.json"
		if FileAccess.file_exists(path):
			var command: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
			if command is Dictionary and command.get("active", false):
				if frozen and int(command.revision) == captured_revision:
					process_pick(command.get("pick", {}))
					await get_tree().create_timer(0.25).timeout
					continue
				frozen = bool(command.get("frozen", false))
				get_tree().paused = frozen
				revision = int(command.revision)
				config.width = int(command.width)
				config.height = int(command.height)
				get_tree().root.size = Vector2i(config.width, config.height)
				controller.apply_view(command.camera)
				await get_tree().process_frame
				await RenderingServer.frame_pre_draw
				pick_metadata = (
					picking.freeze(get_tree().current_scene, controller.camera) if frozen else {}
				)
				last_pick = ""
				await RenderingServer.frame_post_draw
				await capture()
				captured_revision = revision
		await get_tree().create_timer(0.25).timeout


func capture() -> void:
	var image := get_viewport().get_texture().get_image()
	if image.get_width() != int(config.width) or image.get_height() != int(config.height):
		write_record({"sessionId": config.sessionId, "error": "PREVIEW_VIEWPORT_SIZE_CHANGED"})
		get_tree().quit(3)
		return
	sequence += 1
	var path: String = config.output + "/frame-" + str(sequence) + ".png"
	if image.save_png(path) != OK:
		write_record({"sessionId": config.sessionId, "error": "PREVIEW_FRAME_WRITE_FAILED"})
		get_tree().quit(3)
		return
	write_record(
		{
			"sessionId": config.sessionId,
			"sequence": sequence,
			"revision": revision,
			"frozen": frozen,
			"width": image.get_width(),
			"height": image.get_height(),
			"camera": controller.metadata(),
			"picking": pick_metadata,
			"engine": Engine.get_version_info().string
		}
	)
	if sequence > 3:
		DirAccess.remove_absolute(config.output + "/frame-" + str(sequence - 3) + ".png")


func write_record(record: Dictionary) -> void:
	var path: String = config.output + "/frame.tmp"
	var file := FileAccess.open(path, FileAccess.WRITE)
	if file == null:
		get_tree().quit(3)
		return
	file.store_string(JSON.stringify(record))
	file.close()
	if DirAccess.rename_absolute(path, config.output + "/frame.json") != OK:
		get_tree().quit(3)


func process_pick(request: Dictionary) -> void:
	if request.is_empty() or str(request.requestId) == last_pick:
		return
	if int(request.sequence) != sequence or int(request.revision) != revision:
		return
	var result: Dictionary = (
		picking.pick_box(request.rectangle)
		if request.has("rectangle")
		else picking.pick(request.point)
	)
	result.merge(request)
	# JSON.parse_string reads request numbers as floats. Keep receipt IDs integral.
	result.revision = revision
	result.sequence = sequence
	var path: String = config.output + "/pick.tmp"
	var file := FileAccess.open(path, FileAccess.WRITE)
	if file == null:
		return
	file.store_string(JSON.stringify(result))
	file.close()
	if DirAccess.rename_absolute(path, config.output + "/pick.json") == OK:
		last_pick = str(request.requestId)
