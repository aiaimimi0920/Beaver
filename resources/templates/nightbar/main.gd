extends Control
## Original short narrative game. No original VA-11 HALL-A content is included.

const INGREDIENTS := ["星盐", "青柠", "余烬"]
const GUESTS := [
	{
		"name": "洛伊 / 夜班信使",
		"line": "雨把整座城的霓虹揉在了一起。我需要一杯清醒的东西，送完最后一封信就回家。",
		"recipe": [1, 2, 0],
		"hint": "清醒航线：星盐 1 · 青柠 2",
		"reply": "就是这个味道。谢谢你，让回家的路短了一点。"
	},
	{
		"name": "栖岚 / 旧港领航员",
		"line": "明天是旧港最后一次鸣笛。给我一点温暖吧，但不要太甜。",
		"recipe": [0, 1, 2],
		"hint": "港口微光：青柠 1 · 余烬 2",
		"reply": "像灯塔。也许有些地方拆掉以后，仍会留在人的心里。"
	},
	{
		"name": "阿澈 / 电台修理师",
		"line": "我修好了停播三年的电台。今晚想庆祝一下，就来那杯三种味道都有的。",
		"recipe": [1, 1, 1],
		"hint": "晚安频率：星盐 1 · 青柠 1 · 余烬 1",
		"reply": "喂，听得到吗？这座城还醒着。今晚的第一首歌送给你。"
	}
]

var guest_index := 0
var score := 0
var mixture: Array[int] = [0, 0, 0]
var speaker: Label
var dialogue: Label
var recipe_label: Label
var mixture_label: Label
var status_label: Label
var served := false


func _ready() -> void:
	var theme_resource := Theme.new()
	theme_resource.default_font_size = 22
	theme = theme_resource
	var margin := MarginContainer.new()
	margin.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	for edge in ["left", "top", "right", "bottom"]:
		margin.add_theme_constant_override("margin_" + edge, 36)
	add_child(margin)
	var column := VBoxContainer.new()
	column.add_theme_constant_override("separation", 18)
	margin.add_child(column)
	var title := Label.new()
	title.text = "夜 航 调 饮 室   /   NIGHT SHIFT 01"
	title.add_theme_color_override("font_color", Color("d9ff38"))
	title.add_theme_font_size_override("font_size", 32)
	column.add_child(title)
	var subtitle := Label.new()
	subtitle.text = "原创对话调饮模板 · 让每一次倾听，都有回声"
	subtitle.modulate = Color("929a9f")
	column.add_child(subtitle)
	column.add_child(HSeparator.new())
	speaker = Label.new()
	speaker.add_theme_color_override("font_color", Color("22c55e"))
	column.add_child(speaker)
	dialogue = Label.new()
	dialogue.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	dialogue.custom_minimum_size.y = 130
	column.add_child(dialogue)
	recipe_label = Label.new()
	recipe_label.add_theme_color_override("font_color", Color("06b6d4"))
	column.add_child(recipe_label)
	mixture_label = Label.new()
	column.add_child(mixture_label)
	var ingredient_row := HBoxContainer.new()
	ingredient_row.add_theme_constant_override("separation", 12)
	column.add_child(ingredient_row)
	for index in range(INGREDIENTS.size()):
		var button := make_button("+ " + INGREDIENTS[index], ingredient_row)
		button.pressed.connect(add_ingredient.bind(index))
	var actions := HBoxContainer.new()
	actions.add_theme_constant_override("separation", 12)
	column.add_child(actions)
	make_button("调制并递上", actions).pressed.connect(serve)
	make_button("清空配方", actions).pressed.connect(clear_mix)
	make_button("下一位客人", actions).pressed.connect(next_guest)
	make_button("保存进度", actions).pressed.connect(save_progress)
	make_button("读取进度", actions).pressed.connect(load_progress)
	status_label = Label.new()
	status_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	column.add_child(status_label)
	show_guest()


func make_button(text: String, parent: Node) -> Button:
	var button := Button.new()
	button.text = text
	button.custom_minimum_size = Vector2(150, 50)
	parent.add_child(button)
	return button


func show_guest() -> void:
	var guest: Dictionary = GUESTS[guest_index]
	speaker.text = guest["name"]
	dialogue.text = guest["line"]
	recipe_label.text = guest["hint"]
	served = false
	clear_mix()
	status_label.text = "满意的客人：%d / 3" % score


func add_ingredient(index: int) -> void:
	if not served and mixture[index] < 5:
		mixture[index] += 1
	update_mix()


func update_mix() -> void:
	mixture_label.text = "当前杯中  /  星盐 %d   青柠 %d   余烬 %d" % mixture


func clear_mix() -> void:
	mixture = [0, 0, 0]
	update_mix()


func serve() -> void:
	if served:
		return
	var guest: Dictionary = GUESTS[guest_index]
	if mixture == guest["recipe"]:
		served = true
		score += 1
		dialogue.text = guest["reply"]
		status_label.text = "客人很满意。听完这段话，再迎接下一位。"
	else:
		status_label.text = "味道似乎不太对，再看看配方提示吧。"


func next_guest() -> void:
	if not served:
		status_label.text = "先为眼前的客人调好这杯饮品。"
	elif guest_index + 1 < GUESTS.size():
		guest_index += 1
		show_guest()
	else:
		dialogue.text = "天快亮了。你关掉吧台的灯，城市里还有三个人记得今晚。"
		status_label.text = "第一夜结束 · 本轮满意度 %d / 3。可让 Codex 继续扩展角色、配方和剧情。" % score


func save_progress() -> void:
	var file := FileAccess.open("user://nightbar.save", FileAccess.WRITE)
	if file:
		file.store_var({"guest": guest_index, "score": score - int(served)})
		status_label.text = "已保存当前客人开始前的进度。"


func load_progress() -> void:
	if FileAccess.file_exists("user://nightbar.save"):
		var file := FileAccess.open("user://nightbar.save", FileAccess.READ)
		var data: Dictionary = file.get_var()
		guest_index = clampi(int(data.get("guest", 0)), 0, GUESTS.size() - 1)
		score = clampi(int(data.get("score", 0)), 0, 3)
		show_guest()
