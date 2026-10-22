extends RefCounted
## Hide only review chrome; preserve the live 3D stage and backdrop.


static func prepare(host: Control) -> void:
	var requested := OS.get_cmdline_user_args()
	if requested.is_empty() or host.character_definition == null:
		push_error("EAR_CAPTURE_MISSING_DEFINITION")
		host.queue_free()
		return
	if host.character_definition.resource_path != requested[0]:
		push_error("EAR_CAPTURE_DEFINITION_MISMATCH")
		host.queue_free()
		return
	var hidden: Array[String] = []
	for child in host.get_children():
		if child is Control and child != host._stage and not child is ColorRect:
			child.hide()
			hidden.append(str(child.name))
	print("EAR_CAPTURE_VIEW=" + JSON.stringify({
		"definition": requested[0], "hidden_review_controls": hidden,
		"stage_visible": host._stage.is_visible_in_tree()
	}))
