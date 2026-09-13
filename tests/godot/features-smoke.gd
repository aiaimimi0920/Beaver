extends SceneTree

const Inventory = preload("res://features/inventory/inventory.gd")
const Quests = preload("res://features/quests/quest_journal.gd")
const Relationships = preload("res://features/relationship/relationships.gd")
const Wallet = preload("res://features/economy/wallet.gd")
const Crafting = preload("res://features/crafting/crafting.gd")
const DayCycle = preload("res://features/day-cycle/day_cycle.gd")
const Progression = preload("res://features/progression/progression.gd")
const Localization = preload("res://features/localization/localization.gd")

var checks: int = 0
var failures: int = 0


func _initialize() -> void:
	_test_inventory_and_crafting()
	_test_quests()
	_test_relationships_and_wallet()
	_test_clock_and_progression()
	_test_localization()
	if failures == 0:
		print("BEAVER_FEATURES_OK checks=", checks, " modules=8")
	quit(0 if failures == 0 else 1)


func _check(condition: bool, label: String) -> void:
	checks += 1
	if not condition:
		failures += 1
		push_error(label)


func _test_inventory_and_crafting() -> void:
	var inventory = Inventory.new()
	_check(inventory.add("tea", 3), "inventory add")
	_check(not inventory.add("tea", -1), "inventory rejects negative add")
	var before: Dictionary = inventory.snapshot()
	_check(not inventory.exchange({"tea": 2, "water": 1}, {"drink": 1}), "missing ingredient")
	_check(inventory.snapshot() == before, "atomic failure")
	var crafting = Crafting.new()
	_check(crafting.define("tea-drink", {"tea": 2}, {"drink": 1}), "recipe define")
	_check(crafting.craft("tea-drink", inventory), "craft success")
	_check(inventory.count("tea") == 1 and inventory.count("drink") == 1, "craft quantities")
	_check(not crafting.craft("tea-drink", inventory), "cannot craft for free")
	inventory.restore(before)
	_check(inventory.count("tea") == 3 and inventory.count("drink") == 0, "inventory restore")
	_check(not inventory.remove("tea", 4), "reject insufficient removal")


func _test_quests() -> void:
	var journal = Quests.new()
	_check(journal.register("serve", 3), "quest register")
	_check(not journal.claim("serve"), "incomplete quest")
	journal.advance("serve", 5)
	_check(journal.completed("serve"), "quest completion")
	_check(journal.claim("serve") and not journal.claim("serve"), "claim once")
	var restored = Quests.new()
	restored.restore(journal.snapshot())
	_check(restored.completed("serve") and not restored.claim("serve"), "claimed persistence")


func _test_relationships_and_wallet() -> void:
	var relations = Relationships.new()
	_check(relations.adjust("mira", 200) == 100, "relationship upper bound")
	_check(relations.stage("mira") == "friendly", "relationship stage")
	relations.adjust("mira", -300)
	_check(relations.value("mira") == -100, "relationship lower bound")
	var restored = Relationships.new()
	restored.restore(relations.snapshot())
	_check(restored.value("mira") == -100, "relationship persistence")
	var wallet = Wallet.new()
	_check(wallet.credit(10), "wallet credit")
	_check(not wallet.spend(11) and wallet.balance() == 10, "wallet insufficient funds")
	_check(wallet.spend(6) and wallet.balance() == 4, "wallet spend")
	_check(not wallet.credit(-1) and not wallet.spend(-1), "wallet rejects negatives")
	var second = Wallet.new()
	second.restore(wallet.snapshot())
	_check(second.balance() == 4, "wallet persistence")


func _test_clock_and_progression() -> void:
	var clock = DayCycle.new()
	clock.advance(1439)
	_check(
		clock.day() == 1 and clock.hour() == 23 and clock.minute() == 59, "clock before midnight"
	)
	clock.advance(1)
	_check(clock.day() == 2 and clock.hour() == 0, "clock midnight")
	clock.paused = true
	_check(not clock.advance(60) and clock.hour() == 0, "clock paused")
	var restored = DayCycle.new()
	restored.restore(clock.snapshot())
	_check(restored.day() == 2 and restored.paused, "clock persistence")
	var levels = Progression.new()
	_check(
		levels.gain(350) == 2 and levels.level == 3 and levels.experience == 50, "multi level gain"
	)
	_check(levels.gain(-1) == 0 and levels.experience == 50, "negative experience")
	var second = Progression.new()
	second.restore(levels.snapshot())
	_check(second.level == 3 and second.experience == 50, "progression persistence")
	levels.max_level = 4
	levels.gain(10000)
	_check(levels.level == 4 and levels.experience == 0, "level cap")


func _test_localization() -> void:
	var localization = Localization.new()
	localization.tables = {
		"zh": {"hello": "你好，{name}"}, "en": {"hello": "Hello, {name}", "fallback": "Fallback"}
	}
	_check(localization.text("hello", {"name": "Mira"}) == "你好，Mira", "localized parameters")
	_check(localization.text("fallback") == "Fallback", "fallback locale")
	_check(localization.text("missing") == "missing", "missing key remains visible")
	_check(
		(
			localization.set_locale("en")
			and localization.text("hello", {"name": "Mira"}) == "Hello, Mira"
		),
		"language switch"
	)
	_check(not localization.set_locale("invalid"), "unsupported locale")
