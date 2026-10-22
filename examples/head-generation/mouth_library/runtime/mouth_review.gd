extends "res://showcase_extensions/head_review.gd"
## Manual generated-GLB consumer. Not the separate NPR performance JSON speech path.

const MOUTH_NAMES := ["MouthOpen", "MouthRound", "MouthSpread", "MouthPucker"]
const MOUTH_LABELS := ["开口", "圆唇", "横展", "前噘"]
var mouth_weights: Dictionary = {}
var mouth_sliders: Dictionary = {}
var mouth_indices: Dictionary = {}
var mouth_face: MeshInstance3D
var mouth_ready := false


func _ready() -> void:
	super._ready()
	if preview == null or preview.meshes.size() < 2:
		push_error("MOUTH_REVIEW_NO_FACE")
		return
	mouth_face = preview.meshes[1]
	for name in MOUTH_NAMES:
		var index := mouth_face.find_blend_shape_by_name(name)
		if index < 0:
			push_error("MOUTH_REVIEW_MISSING_TARGET: " + name)
			return
		mouth_indices[name] = index
		mouth_weights[name] = 0.0
	mouth_ready = true
	var navigation := get_node_or_null("Navigation")
	if navigation != null:
		navigation.hide()
	var panel := PanelContainer.new()
	panel.position = Vector2(20, 130)
	panel.custom_minimum_size = Vector2(190, 250)
	var background := StyleBoxFlat.new()
	background.bg_color = Color("243d56")
	background.set_corner_radius_all(12)
	background.content_margin_left = 12
	background.content_margin_right = 12
	background.content_margin_top = 12
	background.content_margin_bottom = 12
	panel.add_theme_stylebox_override("panel", background)
	panel.z_index = 50
	add_child(panel)
	var box := VBoxContainer.new()
	panel.add_child(box)
	var title := Label.new()
	title.text = "嘴部状态（非音素）"
	box.add_child(title)
	for i in MOUTH_NAMES.size():
		var name: String = MOUTH_NAMES[i]
		var label := Label.new()
		label.text = MOUTH_LABELS[i]
		box.add_child(label)
		var slider := HSlider.new()
		slider.min_value = 0.0
		slider.max_value = 1.0
		slider.step = 0.05
		slider.custom_minimum_size.x = 175
		slider.value_changed.connect(_mouth_slider_changed.bind(name))
		box.add_child(slider)
		mouth_sliders[name] = slider
	var reset := Button.new()
	reset.text = "恢复中性"
	reset.pressed.connect(func() -> void: set_mouth_state({}))
	box.add_child(reset)
	_status.text = "手动口部状态；未接入语音对齐"
	print("MOUTH_REVIEW_READY=" + JSON.stringify(mouth_indices))


func _process(_delta: float) -> void:
	# Explicit manual ownership only in this head-authoring consumer.
	# No hidden mapping from these visual channels to phonemes is performed.
	if not mouth_ready or not is_instance_valid(mouth_face):
		return
	for name in MOUTH_NAMES:
		mouth_face.set_blend_shape_value(mouth_indices[name], mouth_weights[name])
	display.sync_morphs()


func _mouth_slider_changed(value: float, name: String) -> void:
	var next := mouth_weights.duplicate()
	next[name] = value
	set_mouth_state(next)


func set_mouth_state(values: Dictionary) -> bool:
	if not mouth_ready:
		return false
	for name in values:
		if name not in MOUTH_NAMES:
			return false
		var value = values[name]
		if typeof(value) not in [TYPE_INT, TYPE_FLOAT]:
			return false
		if not is_finite(float(value)) or value < 0.0 or value > 1.0:
			return false
	for name in MOUTH_NAMES:
		mouth_weights[name] = float(values.get(name, 0.0))
		mouth_face.set_blend_shape_value(mouth_indices[name], mouth_weights[name])
		mouth_sliders[name].set_value_no_signal(mouth_weights[name])
	display.sync_morphs()
	return true


func actual_mouth_state() -> Dictionary:
	var result := {}
	if mouth_ready:
		for name in MOUTH_NAMES:
			result[name] = mouth_face.get_blend_shape_value(mouth_indices[name])
	return result
