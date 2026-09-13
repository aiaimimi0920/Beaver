class_name NPRCharacterDefinition
extends Resource
## Three explicit rendering roles; body includes clothing and equipment.

@export var model_scene: PackedScene
@export var body_path := NodePath("Body")
@export var face_path := NodePath("Face")
@export var hair_path := NodePath("Hair")
@export var material_set: NPRCharacterMaterials
## For authored Godot scenes whose three material chains already use this module.
@export var use_source_materials := false
@export var material_profile: NPRMaterialProfile = NPRMaterialProfile.new()
## Zero preserves authored size and pivot. Positive height grounds/centers the model.
@export_range(0.0, 100.0) var display_height := 3.0


func mesh_paths() -> Array[NodePath]:
	return [body_path, face_path, hair_path]


func validate() -> PackedStringArray:
	var errors := PackedStringArray()
	if model_scene == null:
		errors.append("model_scene is required")
	var paths := mesh_paths()
	for path in paths:
		if path.is_empty() or path.is_absolute() or ".." in str(path).split("/"):
			errors.append("Mesh paths must be nonempty relative descendant paths")
	if paths[0] == paths[1] or paths[0] == paths[2] or paths[1] == paths[2]:
		errors.append("Body, face and hair must bind distinct nodes")
	if not is_finite(display_height) or display_height < 0.0:
		errors.append("display_height must be finite and nonnegative")
	if material_profile == null or not material_profile.is_valid():
		errors.append("material_profile requires eight finite exponents in [0.01, 4096]")
	if not use_source_materials:
		if material_set == null:
			errors.append("material_set is required unless use_source_materials is enabled")
		else:
			errors.append_array(material_set.validate())
	return errors
