extends "res://showcase_extensions/head_review.gd"
## Actual native pose captures for the isolated eyelid prototype.


func _ready() -> void:
	super._ready()
	await RenderingServer.frame_post_draw
	var face: MeshInstance3D = preview.meshes[1]
	var left := face.find_blend_shape_by_name("blink.L")
	var right := face.find_blend_shape_by_name("blink.R")
	var mouth := face.find_blend_shape_by_name("MouthOpen")
	if min(left, right, mouth) < 0:
		push_error("BLINK_PROBE_MISSING_SHAPES")
		return
	var source: String = character_definition.model_scene.resource_path
	var before := FileAccess.get_sha256(source)
	var folder := (
		"res://artifacts/blink_probe/blink-"
		+ str(Time.get_unix_time_from_system()).replace(".", "-")
	)
	DirAccess.make_dir_recursive_absolute(folder)
	var captures: Array[Dictionary] = []
	var schedule: Array[Dictionary] = []
	for pose in [
		Vector2(0, 0),
		Vector2(-45, 0),
		Vector2(45, 0),
		Vector2(90, 0),
		Vector2(0, -20),
		Vector2(0, 20)
	]:
		for weight in [0.0, 0.25, 0.5, 0.75, 1.0]:
			for mode in ["render", "white"]:
				schedule.append(
					{
						"yaw": pose.x,
						"pitch": pose.y,
						"left": weight,
						"right": weight,
						"mouth": 0.0,
						"mode": mode
					}
				)
	for pair in [Vector2(1, 0), Vector2(0, 1), Vector2(1, 1)]:
		for yaw in [0.0, -45.0, 45.0]:
			schedule.append(
				{
					"yaw": yaw,
					"pitch": 0.0,
					"left": pair.x,
					"right": pair.y,
					"mouth": 0.6,
					"mode": "render"
				}
			)
	for item in schedule:
		_head_angle(item.yaw, item.pitch)
		_set_display_mode(item.mode)
		face.set_blend_shape_value(left, item.left)
		face.set_blend_shape_value(right, item.right)
		face.set_blend_shape_value(mouth, item.mouth)
		display.sync_morphs()
		_status.text = (
			"Blink %.2f / %.2f | mouth %.2f | %s" % [item.left, item.right, item.mouth, item.mode]
		)
		await get_tree().process_frame
		await RenderingServer.frame_post_draw
		await get_tree().process_frame
		await RenderingServer.frame_post_draw
		var image := folder + "/frame_%03d.png" % captures.size()
		if get_viewport().get_texture().get_image().save_png(image) != OK:
			push_error("BLINK_PROBE_CAPTURE_FAILED")
			return
		captures.append(
			{
				"requested": item,
				"actual_weights":
				[
					face.get_blend_shape_value(left),
					face.get_blend_shape_value(right),
					face.get_blend_shape_value(mouth)
				],
				"image": image,
				"sha256": FileAccess.get_sha256(image)
			}
		)
	face.set_blend_shape_value(left, 0.0)
	face.set_blend_shape_value(right, 0.0)
	face.set_blend_shape_value(mouth, 0.0)
	display.sync_morphs()
	_set_display_mode("render")
	_reset_view()
	var report := {
		"source": source,
		"source_before": before,
		"source_after": FileAccess.get_sha256(source),
		"captures": captures,
		"expected": schedule.size()
	}
	var file := FileAccess.open(folder + "/manifest.json", FileAccess.WRITE)
	file.store_string(JSON.stringify(report, "  "))
	file.close()
	_status.text = "Blink prototype captured; visual acceptance pending"
	print("BLINK_POSE_PROBE=" + JSON.stringify({"folder": folder, "captures": captures.size()}))
