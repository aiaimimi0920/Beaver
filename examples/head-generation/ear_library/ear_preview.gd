extends "res://showcase_extensions/researched_mouth_review.gd"
## Preview the requested ear candidate, never a selector's historical default.

func _ready() -> void:
	super._ready()
	var requested := OS.get_cmdline_user_args()
	if requested.is_empty() or not mouth_ready:
		push_error("EAR_PREVIEW_REQUIRES_READY_EXPLICIT_DEFINITION")
		queue_free()
		return
	if character_definition == null or character_definition.resource_path != requested[0]:
		push_error("EAR_PREVIEW_DEFINITION_MISMATCH")
		queue_free()
		return
	print("EAR_PREVIEW_DEFINITION=" + character_definition.resource_path)
