class_name BeaverWallet
extends RefCounted

signal changed

var balances: Dictionary = {}


func balance(currency: String = "coin") -> int:
	return int(balances.get(currency, 0))


func credit(amount: int, currency: String = "coin") -> bool:
	if amount <= 0 or currency.is_empty():
		return false
	balances[currency] = balance(currency) + amount
	changed.emit()
	return true


func spend(amount: int, currency: String = "coin") -> bool:
	if amount <= 0 or currency.is_empty() or balance(currency) < amount:
		return false
	balances[currency] = balance(currency) - amount
	changed.emit()
	return true


func snapshot() -> Dictionary:
	return balances.duplicate(true)


func restore(data: Dictionary) -> void:
	balances.clear()
	for currency in data:
		if not currency.is_empty():
			balances[currency] = maxi(0, int(data[currency]))
	changed.emit()
