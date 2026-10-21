extends Control
## Four authored lower-face presets; no full-skull or speech acceptance implied.

const REVIEW = preload("res://showcase_extensions/researched_mouth_review.gd")
const CANDIDATES := [905, 898, 899, 900]
const LABELS := ["紧凑圆弧", "柔和收束", "较直收束", "短钝颏变体"]
var current_review: Control
var current_index := -1
var face_ready := false
var selector: OptionButton
var status_label: Label


func _ready() -> void:
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	var panel := VBoxContainer.new()
	panel.position = Vector2(820, 710)
	panel.custom_minimum_size = Vector2(320, 80)
	panel.z_index = 100
	add_child(panel)
	selector = OptionButton.new()
	for label in LABELS:
		selector.add_item(label)
	selector.item_selected.connect(select_face)
	panel.add_child(selector)
	status_label = Label.new()
	status_label.text = "下脸样式；切换会恢复中性表情和视角"
	panel.add_child(status_label)
	select_face(0)


func select_face(index: int) -> bool:
	if index < 0 or index >= CANDIDATES.size():
		if current_index >= 0:
			selector.select(current_index)
		return false
	var path := "res://assets/aster/general_face_eyes_combo%d/aster_definition.tres" % CANDIDATES[index]
	var definition = load(path)
	if definition == null or not definition.validate().is_empty():
		status_label.text = "脸型资源不可用，保留当前模型"
		if current_index >= 0:
			selector.select(current_index)
		return false
	var next = REVIEW.new()
	next.character_definition = definition
	next.set_meta("explicit_definition", true)
	add_child(next)
	move_child(next, 0)
	next.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	if not next.mouth_ready:
		next.queue_free()
		status_label.text = "脸型初始化失败，保留当前模型"
		if current_index >= 0:
			selector.select(current_index)
		return false
	if is_instance_valid(current_review):
		current_review.queue_free()
	current_review = next
	current_index = index
	face_ready = true
	selector.select(index)
	status_label.text = "下脸样式；切换会恢复中性表情和视角"
	return true
