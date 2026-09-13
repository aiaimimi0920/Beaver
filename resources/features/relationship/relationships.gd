class_name BeaverRelationships
extends RefCounted

signal changed

var values: Dictionary = {}


func value(character: String) -> int:
	return int(values.get(character, 0))


func adjust(character: String, amount: int) -> int:
	if character.is_empty():
		return 0
	values[character] = clampi(value(character) + amount, -100, 100)
	changed.emit()
	return value(character)


func stage(character: String) -> String:
	if value(character) >= 50:
		return "friendly"
	if value(character) <= -50:
		return "hostile"
	return "neutral"


func snapshot() -> Dictionary:
	return values.duplicate(true)


func restore(data: Dictionary) -> void:
	values.clear()
	for character in data:
		if not character.is_empty():
			values[character] = clampi(int(data[character]), -100, 100)
	changed.emit()
