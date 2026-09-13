class_name NPRMaterialProfile
extends Resource
## Full float material-slot parameters, with explicit provenance. No fake HDR recovery.

@export_multiline var source_note := "Artist-authored; not recovered capture constants."
@export var capture_verified := false
@export var specular_exponents := PackedFloat32Array([24, 24, 24, 24, 24, 24, 24, 24])


func is_valid() -> bool:
	if specular_exponents.size() != 8:
		return false
	for value in specular_exponents:
		if not is_finite(value) or value < 0.01 or value > 4096.0:
			return false
	return true


func apply_to(material: ShaderMaterial) -> bool:
	if not is_valid():
		return false
	material.set_shader_parameter("u_npr_specular_exponents", specular_exponents)
	material.set_shader_parameter("u_npr_material_profile_enabled", true)
	material.set_shader_parameter("u_use_specular_exponent_override", false)
	return true
