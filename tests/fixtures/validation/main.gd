extends Control

var count: int = 0


func _on_pressed() -> void:
	count += 1
	$Button.text = "Count: %d" % count
	$Color.color = Color.GREEN


func _input(event: InputEvent) -> void:
	if event.is_action_pressed("ui_right"):
		$Color.position.x += 30
