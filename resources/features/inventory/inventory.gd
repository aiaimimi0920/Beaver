class_name BeaverInventory
extends RefCounted

signal changed

var items: Dictionary = {}


func count(item: String) -> int:
	return int(items.get(item, 0))


func add(item: String, amount: int = 1) -> bool:
	if item.is_empty() or amount <= 0:
		return false
	items[item] = count(item) + amount
	changed.emit()
	return true


func exchange(costs: Dictionary, rewards: Dictionary) -> bool:
	for item in costs:
		if item.is_empty() or int(costs[item]) <= 0 or count(item) < int(costs[item]):
			return false
	for item in rewards:
		if item.is_empty() or int(rewards[item]) <= 0:
			return false
	for item in costs:
		items[item] = count(item) - int(costs[item])
		if count(item) == 0:
			items.erase(item)
	for item in rewards:
		items[item] = count(item) + int(rewards[item])
	changed.emit()
	return true


func remove(item: String, amount: int = 1) -> bool:
	return exchange({item: amount}, {})


func snapshot() -> Dictionary:
	return items.duplicate(true)


func restore(data: Dictionary) -> void:
	items.clear()
	for item in data:
		if not item.is_empty() and int(data[item]) > 0:
			items[item] = int(data[item])
	changed.emit()
