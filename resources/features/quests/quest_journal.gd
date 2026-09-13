class_name BeaverQuestJournal
extends RefCounted

signal changed

var quests: Dictionary = {}


func register(id: String, target: int) -> bool:
	if id.is_empty() or target <= 0 or quests.has(id):
		return false
	quests[id] = {"target": target, "progress": 0, "claimed": false}
	changed.emit()
	return true


func advance(id: String, amount: int = 1) -> bool:
	if not quests.has(id) or amount <= 0:
		return false
	var quest: Dictionary = quests[id]
	quest["progress"] = mini(int(quest["progress"]) + amount, int(quest["target"]))
	changed.emit()
	return true


func completed(id: String) -> bool:
	if not quests.has(id):
		return false
	return int(quests[id]["progress"]) >= int(quests[id]["target"])


func claim(id: String) -> bool:
	if not completed(id) or bool(quests[id]["claimed"]):
		return false
	quests[id]["claimed"] = true
	changed.emit()
	return true


func snapshot() -> Dictionary:
	return quests.duplicate(true)


func restore(data: Dictionary) -> void:
	quests.clear()
	for id in data:
		if id.is_empty() or not data[id] is Dictionary:
			continue
		var target: int = int(data[id].get("target", 0))
		if target <= 0:
			continue
		var progress: int = clampi(int(data[id].get("progress", 0)), 0, target)
		quests[id] = {
			"target": target,
			"progress": progress,
			"claimed": progress == target and bool(data[id].get("claimed", false)),
		}
	changed.emit()
