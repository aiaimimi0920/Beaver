# Beaver compatibility overlay for the pinned GUT 9.4.0 loader.
# Only disposable validation copies use this file; the upstream ZIP is unchanged.
# Godot 4.8 replaces exclude_addons with directory_rules. Preserve the complete
# original setting, including custom directory policies, before loading GUT.
const WARNING_PATH: String = "debug/gdscript/warnings/"

static var original_warning_setting: Dictionary = {}

@warning_ignore("unsafe_method_access")
@warning_ignore("unsafe_property_access")
@warning_ignore("untyped_declaration")
static func _static_init() -> void:
	original_warning_setting = capture_warning_setting(ProjectSettings)
	suppress_addon_warnings(ProjectSettings, original_warning_setting)
	var WarningsManager = load("res://addons/gut/warnings_manager.gd")
	if WarningsManager.disabled:
		restore_warning_setting(ProjectSettings, original_warning_setting)

	# Load GUT only after warnings are suppressed, as in the upstream loader.
	var _utils: Object = load("res://addons/gut/utils.gd")
	_utils.LazyLoader.load_all()
	WarningsManager._project_warnings[original_warning_setting["name"]] = copy_setting_value(
		original_warning_setting["value"]
	)


static func copy_setting_value(value: Variant) -> Variant:
	if value is Dictionary:
		return value.duplicate(true)
	return value


@warning_ignore("unsafe_method_access")
static func capture_warning_setting(settings: Object) -> Dictionary:
	var name: String = "directory_rules"
	if not settings.has_setting(WARNING_PATH + name):
		name = "exclude_addons"
	var value: Variant = settings.get_setting(WARNING_PATH + name)
	# Missing or malformed settings must fail, not silently fall back to success.
	assert(
		(
			(name == "directory_rules" and value is Dictionary)
			or (name == "exclude_addons" and value is bool)
		),
		"Unsupported GUT warning setting: " + name
	)
	return {"name": name, "value": copy_setting_value(value)}


@warning_ignore("unsafe_method_access")
static func suppress_addon_warnings(settings: Object, snapshot: Dictionary) -> void:
	var value: Variant = copy_setting_value(snapshot["value"])
	if value is Dictionary:
		value["res://addons"] = 0  # Godot WarningDirectoryRule::DECISION_EXCLUDE.
	else:
		value = true
	settings.set_setting(WARNING_PATH + str(snapshot["name"]), value)


@warning_ignore("unsafe_method_access")
static func restore_warning_setting(settings: Object, snapshot: Dictionary) -> void:
	settings.set_setting(
		WARNING_PATH + str(snapshot["name"]), copy_setting_value(snapshot["value"])
	)


# GUT calls this before running tests. Never leave its temporary policy active.
static func restore_ignore_addons() -> void:
	restore_warning_setting(ProjectSettings, original_warning_setting)

# The MIT License (MIT)
# Copyright (c) 2025 Tom "Butch" Wesley
#
# Permission is hereby granted, free of charge, to any person obtaining a copy
# of this software and associated documentation files (the "Software"), to deal
# in the Software without restriction, including without limitation the rights
# to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
# copies of the Software, and to permit persons to whom the Software is
# furnished to do so, subject to the following conditions:
#
# The above copyright notice and this permission notice shall be included in
# all copies or substantial portions of the Software.
#
# THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
# IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
# FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
# AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
# LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
# OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
# THE SOFTWARE.
