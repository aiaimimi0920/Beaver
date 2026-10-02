extends Node2D

var clicks := 0


func _ready() -> void:
	var layer := CanvasLayer.new()
	layer.name = "UI"
	add_child(layer)
	var button := Button.new()
	button.name = "Button"
	button.position = Vector2(1050, 28)
	button.size = Vector2(182, 50)
	button.text = "Click"
	button.pressed.connect(func(): clicks += 1)
	layer.add_child(button)
	print("VIEWPORT_FIXTURE base=", get_window().content_scale_size)
	print("VIEWPORT_FIXTURE final=", get_viewport().get_final_transform())
	print("VIEWPORT_FIXTURE button=", button.get_global_transform_with_canvas())
	print("VIEWPORT_FIXTURE screen=", button.get_screen_transform())


func _draw() -> void:
	draw_rect(Rect2(0, 0, 1280, 720), Color(0.1, 0.2, 0.3))
	draw_rect(Rect2(1200, 660, 30, 30), Color.GREEN)
