class_name BeaverProgression
extends RefCounted

signal level_changed(level: int)

var level: int = 1
var experience: int = 0
var max_level: int = 100


func required() -> int:
	return level * 100


func gain(amount: int) -> int:
	if amount <= 0 or level >= max_level:
		return 0
	experience += amount
	var gained: int = 0
	while level < max_level and experience >= required():
		experience -= required()
		level += 1
		gained += 1
		level_changed.emit(level)
	if level >= max_level:
		experience = 0
	return gained


func snapshot() -> Dictionary:
	return {"level": level, "experience": experience}


func restore(data: Dictionary) -> void:
	level = clampi(int(data.get("level", 1)), 1, max_level)
	experience = clampi(int(data.get("experience", 0)), 0, required() - 1)
	if level >= max_level:
		experience = 0
