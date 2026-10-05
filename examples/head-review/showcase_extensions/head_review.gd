extends "res://addons/npr_character_frame/showcase/wardrobe.gd"
## Head-only authoring review in the framework wardrobe layout.
## Full wardrobe validation is unchanged and remains a separate unmet gate.

const HEAD_STAGE = preload("res://showcase_extensions/head_stage.gd")
var _head_focus := Vector3.ZERO
var _head_view_size := 1.0
var _compare_overlay
var _compare_previous_update_mode := SubViewport.UPDATE_ALWAYS

const HEAD_MODES := ["render", "white", "facet", "edges"]

func _ready() -> void:
	Engine.max_fps = 30
	var requested := OS.get_cmdline_user_args()
	if not requested.is_empty():
		var definition_path: String = requested[0]
		if not definition_path.begins_with("res://assets/aster/") or not definition_path.ends_with("/aster_definition.tres"):
			push_error("Unsupported head review definition")
			return
		character_definition = load(definition_path)
	if character_definition == null:
		push_error("Head review requires a character definition")
		return
	var errors := character_definition.validate()
	if not errors.is_empty():
		push_error("Invalid head review resource: " + "; ".join(errors))
		return
	_camera_profile = NPRShowcaseCameraProfile.new()
	_camera_profile.view_heights = PackedFloat64Array([0.95, 1.4, 1.66])
	_camera_profile.view_distances = PackedFloat64Array([3.5, 1.5, 0.70])
	_camera_profile.field_of_view = 36.0
	_camera_profile.horizontal_offset = 0.0
	_camera_profile.vertical_offset = 0.0
	get_window().min_size = Vector2i(1152, 720)
	theme = UI.theme()
	if not HEAD_STAGE.build(self):
		return
	# Head-only comparison: visibility changes are preview-only.
	preview.meshes[0].visible = false
	var reference_head := preview.character.get_node_or_null("Head") as Node3D
	if character_definition.resource_path.contains("head_recovery_900/") and reference_head != null:
		reference_head.rotation = Vector3.ZERO
	var face_mesh: MeshInstance3D = preview.meshes[1]
	var face_bounds: AABB = turntable.global_transform.affine_inverse() * face_mesh.global_transform * face_mesh.get_aabb()
	_head_focus = face_bounds.get_center()
	_head_view_size = face_bounds.size.y / 0.55
	camera.set_orthogonal(_head_view_size, 0.03, 1000.0)
	_camera_profile.view_heights = PackedFloat64Array([0.95, 1.4, _head_focus.y])
	_camera_profile.view_distances = PackedFloat64Array([3.5, 1.5, face_bounds.size.y / (2.0 * tan(deg_to_rad(36.0) / 2.0) * 0.55)])
	var is_reference := character_definition.resource_path.contains("head_recovery_900/")
	if not is_reference:
		preview.set_hair_highlight(0.32)
	character_display_name = "银狼 · 参考" if is_reference else "Aster"
	_build_ui()
	for child in get_children():
		if child is Label and child.text.begins_with("V"):
			child.text = "NPR 1.3.0 · 头部审查"
	for i in _navigation.size():
		_navigation[i].disabled = i not in [2, 4, 6, 7, 8]
		if _navigation[i].disabled:
			_navigation[i].tooltip_text = "本轮仅审查头部；旧备份的此项作者数据尚未恢复"
	get_node("Save").disabled = true
	get_node("Save").tooltip_text = "头部审查模式不写入完整装扮方案"
	_status.text = "头部审查 · 动态表情与完整装扮待恢复"
	select_section(6)
	set_view("face")
	print("HEAD_REVIEW_READY=" + JSON.stringify({"definition": character_definition.resource_path, "engine": Engine.get_version_info().string, "full_wardrobe_accepted": false, "body_visible": preview.meshes[0].visible, "orthographic": true, "reference_head_pose_neutralized": is_reference}))

func _input(event: InputEvent) -> void:
	super._input(event)
	if event is InputEventKey and event.pressed and not event.echo and event.keycode == KEY_B:
		preview.meshes[0].visible = not preview.meshes[0].visible
	if event is InputEventKey and event.pressed and not event.echo and event.keycode == KEY_O:
		if camera.projection == Camera3D.PROJECTION_ORTHOGONAL:
			camera.set_perspective(36.0, 0.03, 1000.0)
		else:
			camera.set_orthogonal(_head_view_size, 0.03, 1000.0)
	if event is InputEventKey and event.pressed and not event.echo and event.keycode == KEY_H:
		_toggle_hair()
	if event is InputEventKey and event.pressed and not event.echo and event.keycode == KEY_F12:
		_save_head_capture()

func _save_head_capture() -> void:
	await RenderingServer.frame_post_draw
	var folder := "res://artifacts/head_review"
	DirAccess.make_dir_recursive_absolute(folder)
	var filename := folder + "/head-" + str(Time.get_unix_time_from_system()).replace(".", "-") + ".png"
	var error := get_viewport().get_texture().get_image().save_png(filename)
	if error == OK:
		_status.text = "已保存当前画面 · F12 截图"
		print("HEAD_REVIEW_CAPTURE=" + filename)
	else:
		push_error("Head review screenshot failed: " + str(error))

func _build_display_toolbar() -> void:
	var panel := PanelContainer.new()
	panel.name = "DisplayModes"
	panel.set_anchors_and_offsets_preset(Control.PRESET_TOP_WIDE)
	panel.offset_left = 240
	panel.offset_right = -394
	panel.offset_top = 20
	panel.offset_bottom = 86
	panel.add_theme_stylebox_override("panel", UI.box(Color(0.16, 0.25, 0.36, 0.72)))
	add_child(panel)
	var row := HBoxContainer.new()
	panel.add_child(row)
	var group := ButtonGroup.new()
	for i in HEAD_MODES.size():
		var button := UI.button(["渲染", "白模", "面片", "线框"][i], "HeadMode%d" % i, _set_display_mode.bind(HEAD_MODES[i]))
		button.toggle_mode = true
		button.button_group = group
		button.button_pressed = i == 0
		button.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		row.add_child(button)
		_display_buttons.append(button)
	var hair_button := UI.button("头发开", "HairToggle", _toggle_hair)
	hair_button.tooltip_text = "H：临时隐藏头发，检查脸壳与耳根；不修改模型"
	row.add_child(hair_button)
	row.add_child(UI.button("模型对比", "ModelCompare", _open_comparison))

func _toggle_hair() -> void:
	preview.meshes[2].visible = not preview.meshes[2].visible
	var button := find_child("HairToggle", true, false) as Button
	if button != null:
		button.text = "头发开" if preview.meshes[2].visible else "头发关"

func _set_display_mode(value: String) -> void:
	display.set_mode(value)
	for i in _display_buttons.size():
		_display_buttons[i].set_pressed_no_signal(HEAD_MODES[i] == value)

func select_section(index: int) -> void:
	section = index
	for child in _content.get_children():
		_content.remove_child(child)
		child.queue_free()
	for i in _navigation.size():
		_navigation[i].set_pressed_no_signal(i == index)
	_section_title.text = "头部制作审查" if index != 4 else "灯光渲染"
	_paragraph("仅看头部，默认正交。B：身体；H：头发；O：透视切换。完整人物尚未验收。")
	for row in [["正面", 0.0, 0.0], ["左45°", -45.0, 0.0], ["右45°", 45.0, 0.0], ["左侧", -90.0, 0.0], ["右侧", 90.0, 0.0], ["后脑", 180.0, 0.0], ["俯视", 0.0, 55.0]]:
		_content.add_child(UI.button(row[0], "HeadAngle" + str(row[1]) + str(row[2]), _head_angle.bind(row[1], row[2])))
	var text := "运行时三角面数（不等同 Blender 作者四边面）\n"
	for role in [1, 2]:
		var a: Array = preview.meshes[role].mesh.surface_get_arrays(0)
		var indices: PackedInt32Array = a[Mesh.ARRAY_INDEX]
		text += "%s：%d\n" % [["Body", "Face", "Hair"][role], indices.size() / 3]
	_paragraph(text)
	_add_slider("主光方位", -180.0, 180.0, preview.light_yaw, func(v: float): preview.light_yaw = v)
	_add_slider("发丝高光", 0.0, 1.0, 0.32, preview.set_hair_highlight)
	_add_slider("头发接触阴影", 0.0, 1.0, 0.35, preview.set_hair_contact)
	_add_slider("轮廓线宽", 0.0, 2.0, 1.0, preview.set_outline_width)

func _add_slider(title: String, low: float, high: float, value: float, callback: Callable) -> void:
	_content.add_child(UI.label(title, 14))
	var slider := HSlider.new()
	slider.min_value = low
	slider.max_value = high
	slider.step = 0.01
	slider.value = value
	slider.scrollable = false
	slider.value_changed.connect(callback)
	_content.add_child(slider)

func _head_angle(yaw: float, pitch: float) -> void:
	_model_yaw_degrees = yaw
	_camera_pitch = pitch
	turntable.rotation_degrees.y = yaw
	set_view("face")

func set_view(value: String) -> void:
	var i: int = NPRShowcaseCameraProfile.VIEW_INDICES[value]
	_target = turntable.transform * _head_focus if value == "face" else Vector3(0, _camera_profile.view_heights[i], 0)
	_distance = _camera_profile.view_distances[i]
	camera.h_offset = 0.0
	_update_camera()

func _reset_view() -> void:
	_head_angle(0, 0)

func reset_scheme() -> void:
	_set_display_mode("render")
	_reset_view()

func save_scheme() -> void:
	_status.text = "本轮不写入完整装扮方案"

func _stage_input(event: InputEvent) -> void:
	if event is InputEventMouseButton:
		if event.button_index == MOUSE_BUTTON_LEFT:
			_dragging = event.pressed
		elif event.button_index == MOUSE_BUTTON_MIDDLE:
			_middle_dragging = event.pressed
		elif event.pressed and event.button_index in [MOUSE_BUTTON_WHEEL_UP, MOUSE_BUTTON_WHEEL_DOWN]:
			_zoom_at(event.position, 1.0 if event.button_index == MOUSE_BUTTON_WHEEL_UP else -1.0)
	elif event is InputEventMouseMotion:
		if _middle_dragging:
			_target.y += event.relative.y * 0.0028 * _distance
			_update_camera()
		elif _dragging:
			_model_yaw_degrees = wrapf(_model_yaw_degrees + event.relative.x * 0.4, -180, 180)
			turntable.rotation_degrees.y = _model_yaw_degrees
			_camera_pitch = clampf(_camera_pitch + event.relative.y * 0.25, -70.0, 70.0)
			_update_camera()


func _open_comparison() -> void:
	if is_instance_valid(_compare_overlay):
		return
	var candidate: NPRCharacterDefinition = character_definition
	if candidate.resource_path.contains("head_recovery_900/"):
		candidate = load("res://assets/aster/head_recovery_49/aster_definition.tres")
	var reference: NPRCharacterDefinition = load("res://assets/aster/head_recovery_900/aster_definition.tres")
	if candidate == null or reference == null or not candidate.validate().is_empty() or not reference.validate().is_empty():
		_status.text = "模型对比资源未通过验证"
		return
	_compare_overlay = load("res://showcase_extensions/head_compare.gd").new()
	_compare_overlay.name = "ModelComparison"
	add_child(_compare_overlay)
	_compare_overlay.closed.connect(_close_comparison)
	_compare_previous_update_mode = viewport.render_target_update_mode
	viewport.render_target_update_mode = SubViewport.UPDATE_DISABLED
	set_process_input(false)
	_compare_overlay.configure(candidate, reference)

func _close_comparison() -> void:
	if is_instance_valid(_compare_overlay):
		_compare_overlay.queue_free()
	_compare_overlay = null
	viewport.render_target_update_mode = _compare_previous_update_mode
	set_process_input(true)
	_dragging = false
	_middle_dragging = false
