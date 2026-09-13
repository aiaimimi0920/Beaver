class_name NPRCharacterMaterials
extends Resource
## Authored inputs. Textures are shared; every actor owns its material instances.

const BODY = preload("res://addons/npr_characters/materials/body.tres")
const FACE = preload("res://addons/npr_characters/materials/face.tres")
const EYE = preload("res://addons/npr_characters/materials/eye.tres")
const HAIR = preload("res://addons/npr_characters/materials/hair.tres")
const EYE_HAIR = preload("res://addons/npr_characters/materials/eye_hair.tres")
const TEXTURE_FIELDS := [
	"body_base",
	"body_ilm",
	"body_lut",
	"body_ramp",
	"body_cool_ramp",
	"face_base",
	"face_ilm",
	"face_map",
	"face_ramp",
	"hair_base",
	"hair_ilm",
	"hair_ramp",
	"hair_cool_ramp"
]

@export_group("Body and equipment")
@export var body_base: Texture2D
@export var body_ilm: Texture2D
@export var body_lut: Texture2D
@export var body_ramp: Texture2D
@export var body_cool_ramp: Texture2D
@export_group("Face")
@export var face_base: Texture2D
@export var face_ilm: Texture2D
@export var face_map: Texture2D
@export var face_ramp: Texture2D
@export var sdf_on_uv2 := false
## Local mesh axes, after Godot import. +Z faces the viewer in the standard.
@export var face_forward := Vector3(0, 0, 1)
@export var face_right := Vector3(1, 0, 0)
@export_group("Hair")
@export var hair_base: Texture2D
@export var hair_ilm: Texture2D
@export var hair_ramp: Texture2D
@export var hair_cool_ramp: Texture2D
## Local view direction used by the eye-reveal alpha, not the highlight ribbon.
@export var hair_sheen_axis := Vector3(0, 0, 1)


func validate() -> PackedStringArray:
	var errors := PackedStringArray()
	for field in TEXTURE_FIELDS:
		var texture := get(field) as Texture2D
		if texture == null or texture.get_width() < 1 or texture.get_height() < 1:
			errors.append(field + " requires a nonempty Texture2D")
	if body_lut != null and (body_lut.get_width() != 8 or body_lut.get_height() != 8):
		errors.append("body_lut must be 8 x 8, including reserved rim/bloom rows")
	for texture in [body_ramp, body_cool_ramp, hair_ramp, hair_cool_ramp]:
		if texture != null and (texture.get_width() < 2 or texture.get_height() != 16):
			errors.append("Body/hair ramps require 16 rows and at least two columns")
	if not _unit_axis(face_forward) or not _unit_axis(face_right):
		errors.append("Face axes must be finite unit vectors")
	elif absf(face_forward.dot(face_right)) > 0.001:
		errors.append("Face forward and right must be perpendicular")
	if not _unit_axis(hair_sheen_axis):
		errors.append("hair_sheen_axis must be a finite unit vector")
	return errors


func build() -> Array[ShaderMaterial]:
	if not validate().is_empty():
		return []
	var body := BODY.duplicate() as ShaderMaterial
	var face := FACE.duplicate() as ShaderMaterial
	var eye := EYE.duplicate() as ShaderMaterial
	var hair := HAIR.duplicate() as ShaderMaterial
	var eye_hair := EYE_HAIR.duplicate() as ShaderMaterial
	_bind(
		body,
		{
			"u_texture_base_map": body_base,
			"u_texture_light_map": body_ilm,
			"u_material_values_pack_lut": body_lut,
			"u_texture_diffuse_ramp": body_ramp,
			"u_texture_diffuse_cool_ramp": body_cool_ramp
		}
	)
	_bind(
		face,
		{
			"u_texture_base_map": face_base,
			"u_texture_ilm_map": face_ilm,
			"u_texture_face": face_map,
			"u_texture_diffuse_ramp": face_ramp,
			"u_sdf_on_uv2": sdf_on_uv2,
			"u_npr_sdf_basis_enabled": true,
			"u_npr_face_forward": face_forward,
			"u_npr_face_right": face_right,
			"u_npr_capture_fog_enabled": false
		}
	)
	# Eye coverage uses the base UV atlas, independently of the face SDF UV set.
	_bind(eye, {"u_texture_ilm": face_ilm, "u_sdf_on_uv2": true})
	for material in [hair, eye_hair]:
		_bind(
			material,
			{
				"u_texture_base_map": hair_base,
				"u_texture_ilm_map": hair_ilm,
				"u_texture_diffuse_ramp": hair_ramp,
				"u_texture_diffuse_cool_ramp": hair_cool_ramp,
				"u_npr_hair_sheen_axis": hair_sheen_axis
			}
		)
	face.next_pass = eye
	hair.next_pass = eye_hair
	return [body, face, hair]


static func _bind(material: ShaderMaterial, values: Dictionary) -> void:
	for key in values:
		material.set_shader_parameter(key, values[key])


static func _unit_axis(axis: Vector3) -> bool:
	return axis.is_finite() and absf(axis.length_squared() - 1.0) < 0.001
