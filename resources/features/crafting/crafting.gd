class_name BeaverCrafting
extends RefCounted

var recipes: Dictionary = {}


func define(id: String, costs: Dictionary, rewards: Dictionary) -> bool:
	if id.is_empty() or costs.is_empty() or rewards.is_empty():
		return false
	for item in costs:
		if item.is_empty() or int(costs[item]) <= 0:
			return false
	for item in rewards:
		if item.is_empty() or int(rewards[item]) <= 0:
			return false
	recipes[id] = {"costs": costs.duplicate(true), "rewards": rewards.duplicate(true)}
	return true


func craft(id: String, inventory: RefCounted) -> bool:
	if not recipes.has(id) or inventory == null or not inventory.has_method("exchange"):
		return false
	var recipe: Dictionary = recipes[id]
	return bool(inventory.call("exchange", recipe["costs"], recipe["rewards"]))
