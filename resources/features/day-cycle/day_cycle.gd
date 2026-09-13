class_name BeaverDayCycle
extends RefCounted

signal changed

var total_minutes: int = 0
var paused: bool = false


func advance(minutes: int) -> bool:
	if paused or minutes <= 0:
		return false
	total_minutes += minutes
	changed.emit()
	return true


func day() -> int:
	return floori(float(total_minutes) / 1440.0) + 1


func hour() -> int:
	return floori(float(total_minutes % 1440) / 60.0)


func minute() -> int:
	return total_minutes % 60


func phase() -> String:
	if hour() < 6 or hour() >= 20:
		return "night"
	if hour() < 12:
		return "morning"
	if hour() < 17:
		return "afternoon"
	return "evening"


func snapshot() -> Dictionary:
	return {"total_minutes": total_minutes, "paused": paused}


func restore(data: Dictionary) -> void:
	total_minutes = maxi(0, int(data.get("total_minutes", 0)))
	paused = bool(data.get("paused", false))
	changed.emit()
