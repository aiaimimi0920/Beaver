extends "res://showcase_extensions/head_review.gd"
## Paired imported/triangle-normal diagnostic; identical geometry, lighting and BRDF.


func _ready() -> void:
	super._ready()
	set_process_input(false)
	set_process_unhandled_input(false)
	set_process_unhandled_key_input(false)
	_lock_probe_controls(self)
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
		"res://artifacts/normal_ab_probe/paired-"
		+ str(Time.get_unix_time_from_system()).replace(".", "-")
	)
	DirAccess.make_dir_recursive_absolute(folder)
	var shader := Shader.new()
	shader.code = """shader_type spatial;
render_mode cull_disabled, diffuse_burley, specular_schlick_ggx;
uniform bool geometric = false;
varying vec3 view_position;
void vertex() { view_position = (MODELVIEW_MATRIX * vec4(VERTEX, 1.0)).xyz; }
void fragment() {
    ALBEDO = vec3(0.6653873, 0.7083758, 0.7605245);
    ROUGHNESS = 1.0;
    METALLIC = 0.0;
    SPECULAR = 0.5;
    vec3 face_normal = normalize(cross(dFdx(view_position), dFdy(view_position)));
    if (dot(face_normal, NORMAL) < 0.0) { face_normal = -face_normal; }
    if (geometric) { NORMAL = face_normal; }
}
"""
	var material := ShaderMaterial.new()
	material.shader = shader
	var captures: Array[Dictionary] = []
	var schedule: Array[Dictionary] = []
	for yaw in [0.0, -45.0, 45.0]:
		for weight in [0.0, 1.0]:
			for mode in ["imported_normals", "triangle_normals"]:
				schedule.append({
					"yaw": yaw, "pitch": 0.0, "left": 0.0,
					"right": 0.0, "mouth": weight, "mode": mode
				})
	for item in schedule:
		preview.meshes[0].visible = false
		preview.meshes[2].visible = false
		camera.set_orthogonal(_head_view_size, 0.03, 1000.0)
		_head_angle(item.yaw, item.pitch)
		_set_display_mode("white")
		material.set_shader_parameter("geometric", item.mode == "triangle_normals")
		face.material_override = material
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
				"actual_view": {
					"mode": item.mode,
					"geometric_uniform": material.get_shader_parameter("geometric"),
					"viewport_size": [get_viewport().get_visible_rect().size.x,
						get_viewport().get_visible_rect().size.y],
					"face_visible": face.is_visible_in_tree(),
					"camera_pitch": rad_to_deg(atan2(
						camera.global_transform.basis.z.y, camera.global_transform.basis.z.z)),
					"body_visible": preview.meshes[0].visible,
					"hair_visible": preview.meshes[2].visible,
					"projection": camera.projection,
					"size": camera.size,
					"yaw": turntable.rotation_degrees.y,
					"pitch": _camera_pitch
				},
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
		"comparison": "same shader, geometry, camera and lights; imported normals versus triangle derivative normals",
		"limitations": "Triangle normals are not smoothed normals. This isolates normal-field dependence, not a production shading fix.",
		"source_before": before,
		"source_after": FileAccess.get_sha256(source),
		"captures": captures,
		"expected": schedule.size()
	}
	var file := FileAccess.open(folder + "/manifest.json", FileAccess.WRITE)
	file.store_string(JSON.stringify(report, "  "))
	file.close()
	_status.text = "Paired normal diagnostic captured"
	print("NORMAL_AB_PROBE=" + JSON.stringify({"folder": folder, "captures": captures.size()}))


func _input(_event: InputEvent) -> void:
	pass


func _stage_input(_event: InputEvent) -> void:
	pass


func _lock_probe_controls(node: Node) -> void:
	if node is Control:
		node.mouse_filter = Control.MOUSE_FILTER_IGNORE
		node.focus_mode = Control.FOCUS_NONE
	for child in node.get_children():
		_lock_probe_controls(child)
