extends RefCounted

var owner_node: Node
var spec: Dictionary
var record: Dictionary


func _init(node: Node, config: Dictionary, result: Dictionary) -> void:
	owner_node = node
	spec = config
	record = result


func seconds() -> float:
	return float(Engine.get_process_frames()) / float(spec.config.fps)


func references(step: Dictionary) -> Array:
	var result: Array = spec.references.duplicate(true)
	result.append_array(step.get("references", []))
	var scene := owner_node.get_tree().current_scene
	if scene == null:
		return result
	if not scene.scene_file_path.is_empty():
		result.append(
			{
				"path": scene.scene_file_path,
				"node": str(scene.get_path()),
				"symbol": "",
				"source": "runtime"
			}
		)
	var queue: Array[Node] = [scene]
	var count := 0
	while not queue.is_empty() and count < 500:
		var node: Node = queue.pop_front()
		count += 1
		var script: Script = node.get_script()
		if script != null and not script.resource_path.is_empty():
			result.append(
				{
					"path": script.resource_path,
					"node": str(node.get_path()),
					"symbol": "",
					"source": "runtime"
				}
			)
		queue.append_array(node.get_children())
	return result


func capture(step: Dictionary, suffix: String) -> String:
	await RenderingServer.frame_post_draw
	var window := owner_node.get_window()
	var image := window.get_texture().get_image()
	if image == null or image.is_empty():
		record.error = "Renderer returned an empty image"
		return record.error
	# Viewport textures omit window letterboxing (and may use a design resolution).
	# Remove canvas transforms to recover the actual texture-to-window presentation.
	var canvas := window.get_stretch_transform() * window.global_canvas_transform
	var presentation := window.get_final_transform() * canvas.affine_inverse()
	var content := Rect2i(presentation * Rect2(Vector2.ZERO, Vector2(image.get_size())))
	var layout := {
		"window": [window.size.x, window.size.y],
		"viewport": [image.get_width(), image.get_height()],
		"content": [content.position.x, content.position.y, content.size.x, content.size.y]
	}
	if window.size != Vector2i(spec.config.width, spec.config.height):
		record.error = "Window size changed during capture"
		return record.error
	if record.has("presentation") and record.presentation != layout:
		record.error = "Window presentation changed during capture"
		return record.error
	record.presentation = layout
	image.convert(Image.FORMAT_RGBA8)
	if window.use_hdr_2d:
		image.linear_to_srgb()
	if image.get_size() != content.size:
		image.resize(content.size.x, content.size.y, Image.INTERPOLATE_BILINEAR)
	var output := Image.create(window.size.x, window.size.y, false, Image.FORMAT_RGBA8)
	output.fill(Color.BLACK)
	output.blit_rect(image, Rect2i(Vector2i.ZERO, image.get_size()), content.position)
	var index: int = record.evidence.size()
	var file := "frame-%05d.png" % index
	if output.save_png(spec.output + "/" + file) != OK:
		record.error = "Cannot save captured frame"
		return record.error
	var state := {"scene": "", "step": step.id, "frame": Engine.get_process_frames()}
	if owner_node.get_tree().current_scene != null:
		state.scene = owner_node.get_tree().current_scene.scene_file_path
	record.evidence.append(
		{
			"file": file,
			"point": str(step.id) + ":" + suffix,
			"start": seconds(),
			"end": seconds(),
			"references": references(step),
			"state": state
		}
	)
	owner_node.write_record()
	return ""
