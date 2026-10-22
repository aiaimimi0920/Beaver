extends Control
## Switch only existing whole-model combinations, not an ungenerated slot matrix.

const REVIEW = preload("res://showcase_extensions/researched_mouth_review.gd")
const CHARACTER_DEFINITION = preload("res://addons/npr_character_frame/npr_character_definition.gd")
const CATALOG_PATH := "res://showcase_extensions/researched_ear_catalog.json"
const SLOTS := ["eyes", "brows", "nose", "mouth", "ears", "face"]
var current_review
var current_index := -1
var ear_ready := false
var entries: Array = []
var selector: OptionButton
var status_label: Label
var header_panel: VBoxContainer


func _ready() -> void:
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	_build_controls()
	var data = JSON.parse_string(FileAccess.get_file_as_string(CATALOG_PATH))
	if not data is Dictionary or data.get("schema_version") != 1:
		_fail("组合目录不可用")
		return
	var records = data.get("entries")
	if not records is Array or records.is_empty():
		_fail("尚无已生成组合")
		return
	var ids := {}
	for entry in records:
		if not _valid_entry(entry) or ids.has(entry["id"]):
			_fail("组合目录格式错误")
			return
		ids[entry["id"]] = true
	entries = records
	for entry in entries:
		selector.add_item(entry["label"])
	var requested := _requested_definition()
	var initial := 0
	if not requested.is_empty():
		initial = -1
		for i in entries.size():
			if entries[i]["definition"] == requested:
				initial = i
				break
		if initial < 0:
			_fail("指定模型不在目录内，不回退到旧默认模型")
			return
	select_entry(initial)


func _requested_definition() -> String:
	var args := OS.get_cmdline_user_args()
	return "" if args.is_empty() else args[0]


func _build_controls() -> void:
	var panel := VBoxContainer.new()
	header_panel = panel
	panel.set_anchors_and_offsets_preset(Control.PRESET_TOP_WIDE)
	panel.offset_left = 12
	panel.offset_right = -12
	panel.offset_top = 12
	panel.z_index = 100
	add_child(panel)
	var title := Label.new()
	title.text = "已生成整套组合（非独立部件切换）"
	panel.add_child(title)
	selector = OptionButton.new()
	selector.item_selected.connect(select_entry)
	panel.add_child(selector)
	status_label = Label.new()
	status_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	panel.add_child(status_label)


func _valid_entry(value) -> bool:
	if not value is Dictionary:
		return false
	for key in ["id", "label", "definition", "definition_sha256", "model_sha256", "report_sha256"]:
		if not value.get(key) is String or value[key].is_empty():
			return false
	var path: String = value["definition"]
	if not path.begins_with("res://assets/aster/general_face_eyes_combo") or ".." in path:
		return false
	if not path.ends_with("/aster_definition.tres"):
		return false
	var selection = value.get("selection")
	if not selection is Dictionary or selection.size() != SLOTS.size():
		return false
	for slot in SLOTS:
		if not selection.get(slot) is String or selection[slot].is_empty():
			return false
	return true


func _fail(message: String) -> bool:
	status_label.text = message + "；保留当前模型"
	if current_index >= 0:
		selector.select(current_index)
	else:
		selector.select(-1)
	return false


func _report_matches(report, selection: Dictionary, model_hash: String) -> bool:
	var components = report.get("face_components") if report is Dictionary else null
	return components is Dictionary and components.get("selection") == selection \
		and report.get("new_glb_sha256") == model_hash


func _definition_matches(definition, model_path: String) -> bool:
	return definition is CHARACTER_DEFINITION and definition.validate().is_empty() \
		and definition.model_scene != null and definition.model_scene.resource_path == model_path


func _create_review():
	return REVIEW.new()


func select_entry(index: int) -> bool:
	if index < 0 or index >= entries.size():
		return _fail("组合索引无效")
	var entry = entries[index]
	if not _valid_entry(entry):
		return _fail("组合记录无效")
	var path: String = entry["definition"]
	var prefix := path.get_base_dir()
	var model_path := prefix + "/aster_head.glb"
	var report_path := prefix + "/generation_report.json"
	for required_path in [path, model_path, report_path]:
		if not FileAccess.file_exists(required_path):
			return _fail("组合文件缺失")
	if FileAccess.get_sha256(path) != entry["definition_sha256"]:
		return _fail("组合定义已变化或缺失")
	if FileAccess.get_sha256(model_path) != entry["model_sha256"]:
		return _fail("组合模型已变化或缺失")
	if FileAccess.get_sha256(report_path) != entry["report_sha256"]:
		return _fail("组合来源报告已变化或缺失")
	var report = JSON.parse_string(FileAccess.get_file_as_string(report_path))
	if not _report_matches(report, entry["selection"], entry["model_sha256"]):
		return _fail("组合标签与真实部件不一致")
	var definition = load(path)
	if not _definition_matches(definition, model_path):
		return _fail("组合资源不可用")
	var next = _create_review()
	next.character_definition = definition
	next.set_meta("explicit_definition", true)
	next.hide()
	add_child(next)
	move_child(next, 0)
	next.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	next.offset_top = 112.0
	if not is_instance_valid(next.preview) or not next.preview.initialized \
		or not next.mouth_ready or next.mouth_sliders.size() != next.MOUTH_NAMES.size() \
		or next.character_definition.resource_path != path:
		next.queue_free()
		return _fail("组合初始化失败")
	if next.preview.definition.model_scene.resource_path != model_path:
		next.queue_free()
		return _fail("实际预览模型不一致")
	if is_instance_valid(current_review):
		current_review.hide()
		current_review.queue_free()
	current_review = next
	current_index = index
	ear_ready = true
	selector.select(index)
	next._stage.offset_left = 225.0
	next._stage.offset_right = -380.0
	next.show()
	status_label.text = "脸：%s / 眼：%s；切换重置表情、视角和显示模式" % [entry["selection"]["face"], entry["selection"]["eyes"]]
	print("EAR_COMBINATION_READY=" + JSON.stringify({"id": entry["id"], "definition": path, "model_scene": model_path, "selection": entry["selection"]}))
	return true
