extends Control
## Linked, equal-relative-scale comparison of candidate and reference.

signal closed
const PANE = preload("res://showcase_extensions/head_compare_pane.gd")
var panes: Array = []
var state := {"yaw": 0.0, "pitch": 0.0, "zoom": 1.8, "light": -45.0, "mode": "render", "hair": false, "body": false}
var dragging := false
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
	var controls := HBoxContainer.new()
	column.add_child(controls)
	zoom_slider = slider(controls, "取景", 1.0, 3.0, 1.8, set_zoom)
	slider(controls, "主光", -180.0, 180.0, -45.0, set_light)
	var hair := CheckButton.new()
	hair.text = "头发"
	hair.toggled.connect(set_visibility.bind("hair"))
	controls.add_child(hair)
	var body := CheckButton.new()
	body.text = "身体"
	body.toggled.connect(set_visibility.bind("body"))
	controls.add_child(body)
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
		pane.gui_input.connect(pane_input)
		panes.append(pane)
	status = Label.new()
	status.text = "拖动任一模型同步旋转；滚轮同步缩放；F12 保存对比。仅预览归一化，不修改源模型。"
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
	state.yaw = yaw
	state.pitch = pitch
	apply()

func set_zoom(value: float) -> void:
	state.zoom = value
	apply()

func set_light(value: float) -> void:
	state.light = value
	apply()

func pane_input(event: InputEvent) -> void:
	if event is InputEventMouseButton:
		if event.button_index == MOUSE_BUTTON_LEFT:
			dragging = event.pressed
		elif event.pressed and event.button_index in [MOUSE_BUTTON_WHEEL_UP, MOUSE_BUTTON_WHEEL_DOWN]:
			zoom_slider.value *= 0.9 if event.button_index == MOUSE_BUTTON_WHEEL_UP else 1.1
	elif event is InputEventMouseMotion and dragging:
		state.yaw = wrapf(state.yaw + event.relative.x * 0.4, -180, 180)
		state.pitch = clampf(state.pitch + event.relative.y * 0.25, -60, 60)
		apply()

func _input(event: InputEvent) -> void:
	if event is InputEventMouseButton and not event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
		dragging = false
	if event is InputEventKey and event.pressed and not event.echo:
		if event.keycode == KEY_ESCAPE:
			closed.emit()
		elif event.keycode == KEY_F12:
			save_capture()

func _notification(what: int) -> void:
	if what == NOTIFICATION_WM_WINDOW_FOCUS_OUT:
		dragging = false

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
