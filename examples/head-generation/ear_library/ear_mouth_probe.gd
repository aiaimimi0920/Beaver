extends "res://tests/restored_mouth_probe.gd"
## Reuse the existing capture schedule without occluding ear silhouettes.

const EAR_CAPTURE_VIEW = preload("res://showcase_extensions/ear_capture_view.gd")


func _ready() -> void:
	super._ready()
	EAR_CAPTURE_VIEW.prepare(self)
