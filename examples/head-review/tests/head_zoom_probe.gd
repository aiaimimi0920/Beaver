extends "res://showcase_extensions/head_review.gd"
## Temporary native GUI regression scene; never used by the ordinary viewer.


func _ready() -> void:
	super._ready()
	await RenderingServer.frame_post_draw
	var failures: Array[String] = []
	var cases: Array[Dictionary] = []
	for pose in [Vector2(0, 0), Vector2(-45, 0), Vector2(45, 0), Vector2(90, 0), Vector2(0, 55)]:
		var yaw: float = pose.x
		var pitch: float = pose.y
		for fraction in [Vector2(0.5, 0.5), Vector2(0.48, 0.5), Vector2(0.52, 0.48)]:
			_reset_view()
			_head_angle(yaw, pitch)
			await RenderingServer.frame_post_draw
			var stage_point: Vector2 = _stage.size * fraction
			var point: Vector2 = Vector2(viewport.size) * fraction
			for steps in [1.0, 2.0, -1.0, 50.0, -100.0]:
				var hit: Dictionary = preview.pick_surface(
					camera.project_ray_origin(point), camera.project_ray_normal(point)
				)
				if hit.is_empty():
					failures.append("Missing surface at yaw %.1f" % yaw)
					break
				var focus: Vector3 = hit.position
				var expected := clampf(
					camera.size * pow(0.88, steps), _head_view_size / 6.0, _head_view_size * 2.0
				)
				_zoom_at(stage_point, steps)
				await RenderingServer.frame_post_draw
				var error := camera.unproject_position(focus).distance_to(point)
				if absf(camera.size - expected) > 0.000001 or error > 0.05:
					failures.append("Zoom size/anchor mismatch at yaw %.1f" % yaw)
				cases.append(
					{
						"yaw": yaw,
						"pitch": pitch,
						"cursor_fraction": [fraction.x, fraction.y],
						"steps": steps,
						"anchor_error_px": error,
						"magnification": _head_view_size / camera.size
					}
				)
	_reset_view()
	if absf(camera.size - _head_view_size) > 0.000001:
		failures.append("Reset failed to restore orthographic size")
	if absf(_model_yaw_degrees) + absf(_camera_pitch) > 0.000001:
		failures.append("Reset failed to restore yaw/pitch")
	if _target.distance_to(turntable.transform * _head_focus) > 0.000001:
		failures.append("Reset failed to restore framing target")
	camera.set_perspective(36.0, 0.03, 1000.0)
	await RenderingServer.frame_post_draw
	var before := _distance
	_zoom_at(_stage.size * Vector2(0.5, 0.5), 1.0)
	if absf(_distance - clampf(before * 0.88, 0.35, 30.0)) > 0.000001:
		failures.append("Perspective zoom behavior changed")
	camera.set_orthogonal(_head_view_size, 0.03, 1000.0)
	_reset_view()
	print(
		(
			"HEAD_ZOOM_PROBE="
			+ JSON.stringify(
				{
					"passed": failures.is_empty(),
					"cases": cases,
					"failures": failures,
					"perspective_checked": true
				}
			)
		)
	)
