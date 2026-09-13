class_name BeaverSaveSlot
extends RefCounted
## Adapt this block to the project's data model before use.


static func save_data(data: Dictionary, slot: int = 0) -> Error:
	var file := FileAccess.open("user://save_%d.json" % slot, FileAccess.WRITE)
	if not file:
		return FileAccess.get_open_error()
	file.store_string(JSON.stringify(data))
	return OK


static func load_data(slot: int = 0) -> Dictionary:
	var name := "user://save_%d.json" % slot
	if not FileAccess.file_exists(name):
		return {}
	var value: Variant = JSON.parse_string(FileAccess.get_file_as_string(name))
	return value if value is Dictionary else {}
