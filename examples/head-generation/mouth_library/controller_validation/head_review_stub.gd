extends Control
## Test-only host for the real mouth_review.gd, not the NPR wardrobe application.


class PreviewStub:
	extends RefCounted
	var meshes: Array[MeshInstance3D] = []


class DisplayStub:
	extends RefCounted
	var face: MeshInstance3D
	var sync_calls := 0
	var last_values: Dictionary = {}

	func sync_morphs() -> void:
		sync_calls += 1
		last_values.clear()
		for index in face.mesh.get_blend_shape_count():
			var name: String = face.mesh.get_blend_shape_name(index)
			last_values[name] = face.get_blend_shape_value(index)


var test_face: MeshInstance3D
var preview := PreviewStub.new()
var display := DisplayStub.new()
var _status: Label


func _ready() -> void:
	preview.meshes.assign([null, test_face])
	display.face = test_face
	_status = Label.new()
	add_child(_status)
