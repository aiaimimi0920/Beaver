@tool
extends EditorPlugin


func _enter_tree() -> void:
	var installer = load("res://addons/npr_characters/config/install_settings.gd")
	var errors: PackedStringArray = installer.install()
	if errors.is_empty():
		print("NPR character settings installed. Read addons/npr_characters/README.md.")
	else:
		for error in errors:
			push_error(error)
