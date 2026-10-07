extends Control
## Linked, equal-relative-scale comparison of candidate and reference.

signal closed
const PANE = preload("res://showcase_extensions/head_compare_pane.gd")
var panes: Array = []
var state := {"yaw": 0.0, "pitch": 0.0, "zoom": 1.8, "light": -45.0, "mode": "render", "hair": false, "body": false, "mouth": 0.0, "pan_x": 0.0, "pan_y": 0.0}
var dragging := false
var panning := false
var angle_label: Label
var capture_pending := false
var status: Label
var zoom_slider: HSlider
var candidate_path := ""
var reference_path := ""

func configure(candidate: NPRCharacterDefinition, reference: NPRCharacterDefinition) -> void:
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	mouse_filter = Control.MOUSE_FILTER_STOP
	candidate_path = candidate.resource_path
	reference_path = reference.resource_path
	var background := ColorRect.new()
	background.color = Color("7995ac")
	background.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	background.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(background)
	var margin := MarginContainer.new()
	margin.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	for side in ["left", "right", "top", "bottom"]:
		margin.add_theme_constant_override("margin_" + side, 14)
	add_child(margin)
	var column := VBoxContainer.new()
	column.add_theme_constant_override("separation", 9)
	margin.add_child(column)
	var title_row := HBoxContainer.new()
	column.add_child(title_row)
	var title := Label.new()
	title.text = "模型对比 · 同角度 / 同取景比例 / 同光照"
	title.add_theme_font_size_override("font_size", 21)
	title.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	title_row.add_child(title)
	button(title_row, "返回审查", func(): closed.emit())
	var toolbar := HBoxContainer.new()
	column.add_child(toolbar)
	for pair in [["渲染", "render"], ["白模", "white"], ["线框", "edges"]]:
		button(toolbar, pair[0], set_mode.bind(pair[1]))
	for pair in [["正面", 0], ["左45°", -45], ["右45°", 45], ["左侧", -90], ["右侧", 90], ["背面", 180]]:
		button(toolbar, pair[0], set_angle.bind(float(pair[1]), 0.0))
	button(toolbar, "俯视", set_angle.bind(0.0, 55.0))
	var angle_row := HBoxContainer.new()
	column.add_child(angle_row)
	button(angle_row, "左转10°", step_angle.bind(-10.0))
	button(angle_row, "右转10°", step_angle.bind(10.0))
	button(angle_row, "取景复位", reset_framing)
	angle_label = Label.new()
	angle_row.add_child(angle_label)
	var controls := HBoxContainer.new()
	column.add_child(controls)
	zoom_slider = slider(controls, "取景", 0.25, 3.0, 1.8, set_zoom)
	slider(controls, "主光", -180.0, 180.0, -45.0, set_light)
	var hair := CheckButton.new()
	hair.text = "头发"
	hair.toggled.connect(set_visibility.bind("hair"))
	controls.add_child(hair)
	var body := CheckButton.new()
	body.text = "身体"
	body.toggled.connect(set_visibility.bind("body"))
	controls.add_child(body)
	var mouth_row := HBoxContainer.new()
	column.add_child(mouth_row)
	slider(mouth_row, "候选张嘴", 0.0, 1.0, 0.0, set_mouth)
	var mouth_note := Label.new()
	mouth_note.text = "仅驱动候选 MouthOpen；参考保持中立"
	mouth_row.add_child(mouth_note)
	var pair_row := HBoxContainer.new()
	pair_row.add_theme_constant_override("separation", 12)
	pair_row.size_flags_vertical = Control.SIZE_EXPAND_FILL
	column.add_child(pair_row)
	var success := true
	for entry in [[candidate, "Aster · " + candidate_path.get_base_dir().get_file(), false], [reference, "银狼 · 参考（头部中立姿态）", true]]:
		var box := VBoxContainer.new()
		box.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		box.size_flags_stretch_ratio = 1.0
		pair_row.add_child(box)
		var label := Label.new()
		label.text = entry[1]
		label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		label.add_theme_font_size_override("font_size", 18)
		box.add_child(label)
		var pane = PANE.new()
		box.add_child(pane)
		var loaded: bool = pane.configure(entry[0], entry[2])
		success = success and loaded
		pane.drive_mouth = not entry[2]
		pane.gui_input.connect(pane_input)
		panes.append(pane)
	status = Label.new()
	status.text = "拖动同步旋转；Shift+拖动同步平移；滚轮缩放；10°按钮逐档检查；F12 保存。仅预览归一化，不修改源模型。"
	column.add_child(status)
	if success:
		apply()
		print("MODEL_COMPARE_READY=" + JSON.stringify({"candidate": candidate_path, "reference": reference_path, "face_heights": [panes[0].source_face_height, panes[1].source_face_height], "state": state}))
	else:
		status.text = "比较载入失败，请返回审查查看日志；不能将此画面视为有效对比。"
		push_error("Comparison failed to initialize both models")

func button(row: Control, text: String, callback: Callable) -> void:
	var control := Button.new()
	control.text = text
	control.pressed.connect(callback)
	row.add_child(control)

func slider(row: Control, title: String, low: float, high: float, value: float, callback: Callable) -> HSlider:
	var label := Label.new()
	label.custom_minimum_size.x = 94
	label.text = title + " " + str(snappedf(value, 0.01))
	row.add_child(label)
	var control := HSlider.new()
	control.custom_minimum_size.x = 150
	control.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	control.min_value = low
	control.max_value = high
	control.step = 0.01
	control.value = value
	control.scrollable = false
	control.value_changed.connect(callback)
	control.value_changed.connect(func(v: float): label.text = title + " " + str(snappedf(v, 0.01)))
	row.add_child(control)
	return control

func apply() -> void:
	if is_instance_valid(angle_label):
		angle_label.text = "水平角 %.1f° / 俯仰 %.1f°" % [state.yaw, state.pitch]
	for pane in panes:
		pane.apply_state(state)

func set_visibility(value: bool, key: String) -> void:
	state[key] = value
	apply()

func set_mode(value: String) -> void:
	state.mode = value
	apply()

func set_angle(yaw: float, pitch: float) -> void:
	dragging = false
	panning = false
	state.yaw = yaw
	state.pitch = pitch
	apply()

func step_angle(delta: float) -> void:
	set_angle(wrapf(state.yaw + delta, -180.0, 180.0), state.pitch)

func reset_framing() -> void:
	state.pan_x = 0.0
	state.pan_y = 0.0
	zoom_slider.value = 1.8
	apply()

func set_zoom(value: float) -> void:
	state.zoom = clampf(value, 0.25, 3.0)
	apply()

func set_mouth(value: float) -> void:
	state.mouth = value
	apply()

func set_light(value: float) -> void:
	state.light = value
	apply()

func pane_input(event: InputEvent) -> void:
	if event is InputEventMouseButton:
		if event.button_index == MOUSE_BUTTON_LEFT:
			dragging = event.pressed
			panning = event.pressed and event.shift_pressed
		elif event.pressed and event.button_index in [MOUSE_BUTTON_WHEEL_UP, MOUSE_BUTTON_WHEEL_DOWN]:
			zoom_slider.value *= 0.9 if event.button_index == MOUSE_BUTTON_WHEEL_UP else 1.1
	elif event is InputEventMouseMotion and dragging:
		if panning:
			var scale_per_pixel: float = state.zoom / maxf(panes[0].size.y, 1.0)
			state.pan_x = clampf(state.pan_x - event.relative.x * scale_per_pixel, -1.5, 1.5)
			state.pan_y = clampf(state.pan_y + event.relative.y * scale_per_pixel, -1.5, 1.5)
		else:
			state.yaw = wrapf(state.yaw + event.relative.x * 0.4, -180, 180)
			state.pitch = clampf(state.pitch + event.relative.y * 0.25, -60, 60)
		apply()

func _input(event: InputEvent) -> void:
	if event is InputEventMouseButton and not event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
		dragging = false
		panning = false
	if event is InputEventKey and event.pressed and not event.echo:
		if event.keycode == KEY_ESCAPE:
			closed.emit()
		elif event.keycode == KEY_F12:
			save_capture()

func _notification(what: int) -> void:
	if what == NOTIFICATION_WM_WINDOW_FOCUS_OUT:
		dragging = false
		panning = false

func save_capture() -> void:
	if capture_pending:
		return
	capture_pending = true
	await RenderingServer.frame_post_draw
	var saved_state: Dictionary = state.duplicate(true)
	var folder := "res://artifacts/model_comparison"
	DirAccess.make_dir_recursive_absolute(folder)
	var stem := folder + "/compare-" + str(Time.get_unix_time_from_system()).replace(".", "-")
	var error := get_viewport().get_texture().get_image().save_png(stem + ".png")
	capture_pending = false
	if error != OK:
		status.text = "对比截图保存失败"
		return
	var file := FileAccess.open(stem + ".json", FileAccess.WRITE)
	if file != null:
		file.store_string(JSON.stringify({"candidate": candidate_path, "reference": reference_path, "state": saved_state}, "  "))
	status.text = "已保存同屏对比与参数；请等待保存后再切换视角。"
	print("MODEL_COMPARE_CAPTURE=" + stem + ".png")
