extends GutTest

const Loader = preload("res://addons/gut/gut_loader.gd")
const PREFIX: String = "debug/gdscript/warnings/"


class SettingsDouble:
	extends RefCounted
	var values: Dictionary = {}

	func has_setting(key: String) -> bool:
		return values.has(key)

	func get_setting(key: String) -> Variant:
		return values.get(key)

	func set_setting(key: String, value: Variant) -> void:
		values[key] = value


func test_legacy_true_and_false_are_restored() -> void:
	for original: bool in [true, false]:
		var settings := SettingsDouble.new()
		settings.values[PREFIX + "exclude_addons"] = original
		var snapshot: Dictionary = Loader.capture_warning_setting(settings)
		Loader.suppress_addon_warnings(settings, snapshot)
		assert_eq(settings.values[PREFIX + "exclude_addons"], true)
		Loader.restore_warning_setting(settings, snapshot)
		assert_eq(settings.values[PREFIX + "exclude_addons"], original)
		assert_eq(settings.values.size(), 1)


func test_directory_policies_survive_suppression_and_restore() -> void:
	for original: Dictionary in [
		{},
		{"res://addons": 0},
		{"res://addons": 1, "res://addons/custom": 1, "res://scripts": 0},
	]:
		var settings := SettingsDouble.new()
		settings.values[PREFIX + "directory_rules"] = original.duplicate(true)
		var snapshot: Dictionary = Loader.capture_warning_setting(settings)
		Loader.suppress_addon_warnings(settings, snapshot)
		assert_eq(settings.values[PREFIX + "directory_rules"]["res://addons"], 0)
		assert_eq(snapshot["value"], original)
		for key: String in original:
			if key != "res://addons":
				assert_eq(settings.values[PREFIX + "directory_rules"][key], original[key])
		Loader.restore_warning_setting(settings, snapshot)
		assert_eq(settings.values[PREFIX + "directory_rules"], original)
		assert_false(settings.has_setting(PREFIX + "exclude_addons"))
		settings.values[PREFIX + "directory_rules"]["res://scripts"] = 1
		assert_eq(snapshot["value"], original, "restored dictionaries must not alias the snapshot")


func test_modern_setting_takes_precedence_without_rewriting_legacy() -> void:
	var settings := SettingsDouble.new()
	settings.values = {PREFIX + "directory_rules": {}, PREFIX + "exclude_addons": false}
	var snapshot: Dictionary = Loader.capture_warning_setting(settings)
	assert_eq(snapshot["name"], "directory_rules")
	Loader.suppress_addon_warnings(settings, snapshot)
	Loader.restore_warning_setting(settings, snapshot)
	assert_eq(settings.values[PREFIX + "directory_rules"], {})
	assert_false(settings.values[PREFIX + "exclude_addons"])


func test_real_loader_and_warnings_manager_restore_project_policy() -> void:
	var snapshot: Dictionary = Loader.original_warning_setting
	var active: Variant = ProjectSettings.get_setting(PREFIX + str(snapshot["name"]))
	assert_eq(active, snapshot["value"])
	var manager: Script = load("res://addons/gut/warnings_manager.gd")
	assert_eq(manager.get("project_warnings")[snapshot["name"]], snapshot["value"])
	if snapshot["name"] == "directory_rules":
		assert_false(ProjectSettings.has_setting(PREFIX + "exclude_addons"))
		assert_eq(active, {"res://addons": 1, "res://addons/custom": 1, "res://scripts": 0})
