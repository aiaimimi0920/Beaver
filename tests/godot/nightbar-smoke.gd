extends SceneTree


func _initialize() -> void:
	_run.call_deferred()


func _run() -> void:
	var scene := load("res://main.tscn") as PackedScene
	var game := scene.instantiate() as Control
	root.add_child(game)
	await process_frame
	game.call("serve")
	if not _check(game.get("score") == 0, "Wrong recipe must not score"):
		return
	var recipes := [[1, 2, 0], [0, 1, 2], [1, 1, 1]]
	for guest in range(3):
		for ingredient in range(3):
			for dose in range(recipes[guest][ingredient]):
				game.call("add_ingredient", ingredient)
		game.call("serve")
		game.call("serve")
		if not _check(game.get("score") == guest + 1, "Serve must score once"):
			return
		game.call("next_guest")
	if not _check(game.get("score") == 3, "All guests must be satisfied"):
		return
	game.call("save_progress")
	game.set("score", 0)
	game.call("load_progress")
	if not _check(game.get("score") == 2, "Save restores start of last guest"):
		return
	game.queue_free()
	await process_frame
	print("BEAVER_GAMEPLAY_OK: recipes, duplicate serve, story progression, save/load")
	quit(0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		push_error(message)
		quit(1)
	return condition
