class_name BeaverLocalization
extends RefCounted

signal changed

var locale: String = "zh"
var fallback_locale: String = "en"
var tables: Dictionary = {}


func set_locale(value: String) -> bool:
	if not tables.has(value):
		return false
	locale = value
	changed.emit()
	return true


func text(key: String, parameters: Dictionary = {}) -> String:
	var primary: Dictionary = tables.get(locale, {})
	var fallback: Dictionary = tables.get(fallback_locale, {})
	var result: String = str(primary.get(key, fallback.get(key, key)))
	for name in parameters:
		result = result.replace("{" + name + "}", str(parameters[name]))
	return result
