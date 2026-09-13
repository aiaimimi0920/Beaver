extends SceneTree


func _initialize() -> void:
	var missing := PackedStringArray()
	if not TriangleMesh.new().has_method("update_from_indexed_surfaces"):
		missing.append("TriangleMesh.update_from_indexed_surfaces")
	if not RenderingServer.has_method("mesh_surface_get_geometry_arrays"):
		missing.append("RenderingServer.mesh_surface_get_geometry_arrays")
	if not missing.is_empty():
		print("BEAVER_NPR_INCOMPATIBLE=" + JSON.stringify(missing))
		quit(1)
		return
	var installer: Script = load("res://addons/npr_characters/config/install_settings.gd")
	var errors: PackedStringArray = installer.install()
	if not errors.is_empty():
		print("BEAVER_NPR_INSTALL_ERRORS=" + JSON.stringify(errors))
		quit(1)
		return
	ProjectSettings.set_setting("rendering/renderer/rendering_method", "forward_plus")
	ProjectSettings.set_setting("rendering/renderer/rendering_method.mobile", "forward_plus")
	var enabled: PackedStringArray = ProjectSettings.get_setting(
		"editor_plugins/enabled", PackedStringArray()
	)
	var plugin := "res://addons/npr_characters/plugin.cfg"
	if not enabled.has(plugin):
		enabled.append(plugin)
	ProjectSettings.set_setting("editor_plugins/enabled", enabled)
	var result := ProjectSettings.save()
	if result != OK:
		push_error("Cannot save NPR project settings: " + error_string(result))
		quit(1)
		return
	print("BEAVER_NPR_INSTALL_OK")
	quit(0)
