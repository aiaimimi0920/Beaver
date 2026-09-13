class_name NPRDepthViewTable
extends RefCounted
## Main-thread RGBA32F view metadata. Publish count only after update succeeds.
## Camera matrices use the same float32 conversion as material uniforms.

const WIDTH := 12
var texture := ImageTexture.new()
var view_count := 0
var upload_count := 0
## Actual encoding work, including a valid prefix of a later-rejected batch.
var encoded_rows := 0
var _bytes := PackedByteArray()
var _height := 0
var _cached_rows: Array[Dictionary] = []
var _cached_bytes: Array[PackedByteArray] = []


func update_views(rows: Array[Dictionary]) -> Error:
	# The shared array's device layer limit is checked by its allocation owner.
	# Bound metadata allocation independently; this is not a two-view protocol.
	if rows.size() > 4096:
		return ERR_INVALID_PARAMETER
	var bytes := PackedByteArray()
	var cache_rows: Array[Dictionary] = []
	var cache_bytes: Array[PackedByteArray] = []
	for index in range(rows.size()):
		var row := rows[index]
		# Only validated, immutable records may bypass validation and encoding.
		# Identity, not Dictionary value equality, preserves types and signed zero.
		if row.is_read_only() and index < _cached_rows.size() and is_same(row, _cached_rows[index]):
			bytes.append_array(_cached_bytes[index])
			cache_rows.append(row)
			cache_bytes.append(_cached_bytes[index])
			continue
		var columns := _pack_row(row)
		if columns.size() != WIDTH:
			return ERR_INVALID_PARAMETER
		for column in columns:
			if not column.is_finite():
				return ERR_INVALID_PARAMETER
		# Single-precision Vector4 already contains the shader columns. Pack a
		# whole row natively instead of allocating two arrays per metadata texel.
		var packed := PackedVector4Array(columns).to_byte_array()
		if packed.size() != WIDTH * 16:
			# Preserve the RGBA32F contract even in a double-precision engine.
			var values := PackedFloat32Array()
			for column in columns:
				values.append_array(PackedFloat32Array([column.x, column.y, column.z, column.w]))
			packed = values.to_byte_array()
		bytes.append_array(packed)
		encoded_rows += 1
		cache_rows.append(row if row.is_read_only() else {})
		cache_bytes.append(packed)
	var height := maxi(rows.size(), 1)
	if rows.is_empty():
		bytes.resize(WIDTH * 16)
	# Commit cache only after the entire candidate has passed validation.
	_cached_rows = cache_rows
	_cached_bytes = cache_bytes
	if bytes == _bytes and height == _height and view_count == rows.size():
		return OK
	var image := Image.create_from_data(WIDTH, height, false, Image.FORMAT_RGBAF, bytes)
	if _height == height:
		texture.update(image)
	else:
		texture.set_image(image)
	_bytes = bytes
	_height = height
	view_count = rows.size()
	upload_count += 1
	return OK


func _pack_row(row: Dictionary) -> Array[Vector4]:
	# Fetch once without coercion: float integer-fields must still be rejected.
	var world_value: Variant = row.get("world")
	var projection_value: Variant = row.get("projection")
	var size_value: Variant = row.get("viewport_size")
	var pixel_value: Variant = row.get("pixel_size")
	var sizes_value: Variant = row.get("sizes")
	var layers: Variant = row.get("layers")
	var enabled: Variant = row.get("enabled")
	var array_base: Variant = row.get("array_base")
	var ready_index: Variant = row.get("ready_index")
	var steps: Variant = row.get("steps")
	var depth_range: Variant = row.get("range")
	var height: Variant = row.get("height")
	if (
		not world_value is Transform3D
		or not projection_value is Projection
		or not size_value is Vector2
		or not pixel_value is Vector2
		or not sizes_value is Vector4
		or not layers is int
		or not enabled is int
		or not array_base is int
		or not ready_index is int
		or not steps is int
		or not (depth_range is float or depth_range is int)
		or not (height is float or height is int)
	):
		return []
	var size: Vector2 = size_value
	var pixel: Vector2 = pixel_value
	var sizes: Vector4 = sizes_value
	if (
		layers < 0
		or layers > 0xffffffff
		or enabled < 0
		or enabled > 3
		or array_base < 0
		or array_base > 16777214
		or ready_index < 0
		or ready_index > 16777215
		or steps < 2
		or steps > 4
		or depth_range <= 0
		or height <= 0
		or size.x <= 0
		or size.y <= 0
		or pixel.x <= 0
		or pixel.y <= 0
		or sizes.x <= 0
		or sizes.y <= 0
		or sizes.z <= 0
		or sizes.w <= 0
	):
		return []
	var world := Projection(world_value)
	var projection: Projection = projection_value
	return [
		world.x,
		world.y,
		world.z,
		world.w,
		projection.x,
		projection.y,
		projection.z,
		projection.w,
		Vector4(size.x, size.y, layers & 0xffff, (layers >> 16) & 0xffff),
		Vector4(enabled, depth_range, height, steps),
		Vector4(pixel.x, pixel.y, array_base, ready_index),
		sizes
	]
