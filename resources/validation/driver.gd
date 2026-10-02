extends Node

const Capture = preload("res://beaver_validation_runtime/capture.gd")
var spec: Dictionary
var record := {"complete": false, "completedSteps": 0, "evidence": [], "events": [], "error": ""}
var captures: RefCounted
var held_actions: Array[String] = []
var held_keys: Array[int] = []


func _enter_tree() -> void:
	spec = JSON.parse_string(
		FileAccess.get_file_as_string("res://beaver_validation_runtime/run.json")
	)
	seed(int(spec.config.seed))
	TranslationServer.set_locale(spec.config.locale)
	Engine.max_fps = int(spec.config.fps)
	var viewport_size := Vector2i(spec.config.width, spec.config.height)
	var window := get_tree().root
	# Resize the output window without replacing the game's logical design canvas.
	window.size = viewport_size
	for path in spec.config.saves:
		var destination := "user://" + str(path)
		DirAccess.make_dir_recursive_absolute(destination.get_base_dir())
		var file := FileAccess.open(destination, FileAccess.WRITE)
		if file == null:
			record.error = "Cannot prepare isolated save: " + destination
			continue
		file.store_buffer(FileAccess.get_file_as_bytes("res://" + str(path)))
		file.close()


func _ready() -> void:
	captures = Capture.new(self, spec, record)
	call_deferred("run_flow")


func write_record() -> void:
	var temporary: String = spec.output + "/record.tmp"
	var file := FileAccess.open(temporary, FileAccess.WRITE)
	if file == null:
		get_tree().quit(3)
		return
	file.store_string(JSON.stringify(record))
	file.close()
	DirAccess.rename_absolute(temporary, spec.output + "/record.json")


func run_flow() -> void:
	await get_tree().process_frame
	if not record.error.is_empty():
		finish(false)
		return
	record.userData = OS.get_user_data_dir()
	for step in spec.steps:
		record.events.append({"step": step.id, "time": captures.seconds(), "action": step})
		var error: String = await execute_step(step)
		if not error.is_empty():
			record.error = "Step " + str(step.id) + ": " + error
			finish(false)
			return
		record.completedSteps += 1
		if spec.video and step.kind != "capture":
			await captures.capture(step, "keyframe")
		write_record()
	finish(true)


func execute_step(step: Dictionary) -> String:
	var error := ""
	match step.kind:
		"wait":
			for index in int(step.frames):
				await get_tree().process_frame
				if spec.video and index % int(spec.config.fps) == 0:
					await captures.capture(step, "sample-" + str(index))
		"action":
			if not InputMap.has_action(step.name):
				return "Unknown input action: " + str(step.name)
			var event := InputEventAction.new()
			event.action = step.name
			event.pressed = step.pressed
			event.strength = 1.0 if step.pressed else 0.0
			Input.parse_input_event(event)
			if step.pressed:
				Input.action_press(step.name)
				held_actions.append(step.name)
			else:
				Input.action_release(step.name)
				held_actions.erase(step.name)
			await get_tree().process_frame
		"key":
			var event := InputEventKey.new()
			event.keycode = int(step.code)
			event.physical_keycode = int(step.code)
			event.pressed = step.pressed
			Input.parse_input_event(event)
			if step.pressed:
				held_keys.append(int(step.code))
			else:
				held_keys.erase(int(step.code))
			await get_tree().process_frame
		"click":
			var node := get_node_or_null(NodePath(step.node))
			if not node is Control or not node.is_visible_in_tree():
				return "Click requires a visible Control: " + str(step.node)
			var canvas_position: Vector2 = node.get_global_transform_with_canvas() * (node.size / 2)
			var location: Vector2 = get_viewport().get_final_transform() * canvas_position
			var motion := InputEventMouseMotion.new()
			motion.position = location
			Input.parse_input_event(motion)
			for pressed in [true, false]:
				var event := InputEventMouseButton.new()
				event.position = location
				event.button_index = MOUSE_BUTTON_LEFT
				event.pressed = pressed
				Input.parse_input_event(event)
				await get_tree().process_frame
		"waitFor":
			for index in int(step.timeout):
				var node := get_node_or_null(NodePath(step.node))
				if node != null and node.get_indexed(NodePath(step.property)) == step.equals:
					return ""
				await get_tree().process_frame
				if spec.video and index % int(spec.config.fps) == 0:
					await captures.capture(step, "waiting-" + str(index))
			return "State wait timed out: " + str(step.node) + ":" + str(step.property)
		"capture":
			error = await captures.capture(step, "capture")
		_:
			error = "Unknown flow step"
	return error


func finish(success: bool) -> void:
	for action in held_actions:
		Input.action_release(action)
	for code in held_keys:
		var event := InputEventKey.new()
		event.keycode = code
		event.pressed = false
		Input.parse_input_event(event)
	record.complete = success and record.error.is_empty()
	record.duration = captures.seconds()
	write_record()
	get_tree().quit(0 if record.complete else 2)
