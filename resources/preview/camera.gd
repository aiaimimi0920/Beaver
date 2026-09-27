extends RefCounted

var camera: Camera3D
var initial: Transform3D
var distance := 10.0
var initial_size := 1.0


func initialize(host: Node) -> String:
	var source := host.get_viewport().get_camera_3d()
	if source == null:
		return "PREVIEW_REQUIRES_ACTIVE_CAMERA3D"
	initial = source.global_transform
	camera = Camera3D.new()
	camera.projection = source.projection
	camera.fov = source.fov
	camera.size = source.size
	initial_size = source.size
	camera.near = source.near
	camera.far = source.far
	camera.keep_aspect = source.keep_aspect
	camera.frustum_offset = source.frustum_offset
	camera.h_offset = source.h_offset
	camera.v_offset = source.v_offset
	camera.cull_mask = source.cull_mask
	camera.environment = source.environment
	camera.attributes = source.attributes
	host.add_child(camera)
	camera.global_transform = initial
	camera.make_current()
	return ""


func apply_view(view: Dictionary) -> void:
	var yaw := deg_to_rad(float(view.yaw))
	var pitch := deg_to_rad(float(view.pitch))
	var rotation := Basis(initial.basis.y.normalized(), yaw) * initial.basis
	rotation = Basis(rotation.x.normalized(), pitch) * rotation
	var scale_factor := exp(float(view.zoom))
	var pivot := initial.origin - initial.basis.z * distance
	pivot += initial.basis.x * float(view.panX) * distance
	pivot += initial.basis.y * float(view.panY) * distance
	camera.global_transform = Transform3D(rotation, pivot + rotation.z * distance * scale_factor)
	camera.size = initial_size * scale_factor
	camera.make_current()


func metadata() -> Dictionary:
	var transform := camera.global_transform
	var projection := camera.get_camera_projection()
	return {
		"transform":
		[
			vector(transform.basis.x),
			vector(transform.basis.y),
			vector(transform.basis.z),
			vector(transform.origin)
		],
		"projection":
		[plane(projection.x), plane(projection.y), plane(projection.z), plane(projection.w)],
		"near": camera.near,
		"far": camera.far,
		"mode": camera.projection
	}


func vector(value: Vector3) -> Array:
	return [value.x, value.y, value.z]


func plane(value: Vector4) -> Array:
	return [value.x, value.y, value.z, value.w]
