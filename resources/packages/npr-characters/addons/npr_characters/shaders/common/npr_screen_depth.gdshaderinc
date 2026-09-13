uniform bool u_npr_depth_enabled = false;
// Explicit opt-in contract, not bit-equivalence with hardware-normalized nearest.
uniform bool u_npr_depth_integer_sampling = false;
uniform sampler2D u_npr_character_depth : filter_nearest, repeat_disable;
uniform sampler2D u_npr_hair_depth : filter_nearest, repeat_disable;
uniform float u_npr_depth_range = 16.0;
uniform float u_npr_character_height = 3.0;
uniform float u_npr_depth_rim_pixels : hint_range(0.0, 6.0) = 2.0;
uniform float u_npr_contact_strength : hint_range(0.0, 1.0) = 0.0;
// Logical screen pixels, NOT packed-depth texels; zero preserves standalone use.
uniform vec2 u_npr_depth_pixel_size = vec2(0.0);
uniform int u_npr_contact_steps : hint_range(2, 4) = 4;
// Each texture pair belongs to one camera-space mapping, not every draw of this material.
uniform mat4 u_npr_depth_camera_world = mat4(1.0);
uniform mat4 u_npr_depth_projection = mat4(1.0);
uniform vec2 u_npr_depth_viewport_size = vec2(0.0);
uniform uint u_npr_depth_layers = 0u;
// Explicit second view; fixed samplers avoid dynamically indexed descriptor arrays.
uniform bool u_npr_depth_enabled_secondary = false;
uniform sampler2D u_npr_character_depth_secondary : filter_nearest, repeat_disable;
uniform sampler2D u_npr_hair_depth_secondary : filter_nearest, repeat_disable;
uniform float u_npr_depth_range_secondary = 16.0;
uniform float u_npr_character_height_secondary = 3.0;
uniform vec2 u_npr_depth_pixel_size_secondary = vec2(0.0);
uniform int u_npr_contact_steps_secondary = 4;
uniform mat4 u_npr_depth_camera_world_secondary = mat4(1.0);
uniform mat4 u_npr_depth_projection_secondary = mat4(1.0);
uniform vec2 u_npr_depth_viewport_size_secondary = vec2(0.0);
uniform uint u_npr_depth_layers_secondary = 0u;
// Optional array transport. Negative bases retain the direct viewport path.
// Each view owns consecutive character/hair layers and their unpadded sizes.
// The producer must publish only after both storage and view metadata are ready.
uniform sampler2DArray u_npr_depth_array : filter_nearest, repeat_disable;
uniform bool u_npr_depth_array_compact = false;
uniform int u_npr_depth_array_base = -1;
uniform int u_npr_depth_array_base_secondary = -1;
uniform vec4 u_npr_depth_array_sizes = vec4(1.0);
uniform vec4 u_npr_depth_array_sizes_secondary = vec4(1.0);
uniform bool u_npr_depth_array_guard = false;
uniform sampler2D u_npr_depth_array_ready : filter_nearest, repeat_disable;
// Shared per-view guard. Negative indices retain the legacy single-view guard.
uniform sampler2DArray u_npr_depth_view_ready : filter_nearest, repeat_disable;
uniform int u_npr_depth_view_index = -1;
uniform int u_npr_depth_view_index_secondary = -1;
// Twelve RGBA32F texels per registered view; zero rows keeps the old protocol.
uniform sampler2D u_npr_depth_view_table : filter_nearest, repeat_disable;
uniform int u_npr_depth_view_count = 0;
uniform int u_npr_depth_view_channel = 0; // 0: character rim, 1: hair contact.
// 0 = unavailable, 1/2 = legacy views, 3 + row = dynamic metadata view.
varying flat float npr_depth_view_slot;
varying flat vec2 npr_depth_uv_offset;

#ifdef HAS_UNJITTERED_PROJECTION_MATRIX
#define NPR_DEPTH_PROJECTION UNJITTERED_PROJECTION_MATRIX
#else
#define NPR_DEPTH_PROJECTION PROJECTION_MATRIX
#endif

vec2 npr_depth_projection_offset(mat4 raster_projection, mat4 depth_projection) {
	// Temporal jitter is a constant clip-XY translation after perspective division.
	// Use a finite point for both perspective and orthographic cameras. Keep the
	// original raster UV exactly when jitter is absent, including on older engines.
	vec4 point = vec4(0.0, 0.0, -1.0, 1.0);
	vec4 raster = raster_projection * point;
	vec4 stable = depth_projection * point;
	return (stable.xy / stable.w - raster.xy / raster.w) * 0.5;
}

bool npr_same_depth_view(mat4 camera_world, mat4 projection, vec2 dimensions, uint layers,
		mat4 depth_camera_world, mat4 depth_projection, vec2 depth_dimensions, uint depth_layers) {
	// Exact float32 uniforms: do not merge nearby cameras with an arbitrary epsilon.
	// Run per vertex and pass one flat scalar, rather than compare matrices per fragment.
	return layers == depth_layers && all(equal(dimensions, depth_dimensions))
		&& all(equal(camera_world[0], depth_camera_world[0]))
		&& all(equal(camera_world[1], depth_camera_world[1]))
		&& all(equal(camera_world[2], depth_camera_world[2]))
		&& all(equal(camera_world[3], depth_camera_world[3]))
		&& all(equal(projection[0], depth_projection[0]))
		&& all(equal(projection[1], depth_projection[1]))
		&& all(equal(projection[2], depth_projection[2]))
		&& all(equal(projection[3], depth_projection[3]));
}

bool npr_view_array_ready(int view_index) {
	if (view_index < 0 || view_index >= textureSize(u_npr_depth_view_ready, 0).z) {
		return false;
	}
	return texelFetch(u_npr_depth_view_ready, ivec3(0, 0, view_index), 0).r > 0.5;
}

vec4 npr_view_record(int column, int row) {
	return texelFetch(u_npr_depth_view_table, ivec2(column, row), 0);
}

float npr_select_depth_view(mat4 camera_world, mat4 projection, vec2 dimensions, uint layers) {
	if (u_npr_depth_view_count > 0 && textureSize(u_npr_depth_view_table, 0).x == 12) {
		int count = min(u_npr_depth_view_count, textureSize(u_npr_depth_view_table, 0).y);
		ivec3 array_dimensions = textureSize(u_npr_depth_array, 0);
		for (int row = 0; row < count; row++) {
			vec4 parameters = npr_view_record(9, row);
			int enabled = int(parameters.x);
			int channel = clamp(u_npr_depth_view_channel, 0, 1);
			if ((enabled & (1 << channel)) == 0) { continue; }
			vec4 mapping = npr_view_record(10, row);
			vec4 sizes = npr_view_record(11, row);
			if (mapping.z < 0.0 || mapping.z + 1.0 >= float(array_dimensions.z)
					|| any(lessThanEqual(sizes, vec4(0.0)))
					|| any(greaterThan(sizes.xy, vec2(array_dimensions.xy)))
					|| any(greaterThan(sizes.zw, vec2(array_dimensions.xy)))) { continue; }
			vec2 source_dimensions = channel == 1 ? sizes.zw : sizes.xy;
			// Preserve the existing normalized-nearest contract for legacy callers.
			// Padded multi-view layers require the explicitly selected integer mode.
			if (!u_npr_depth_integer_sampling
					&& any(notEqual(source_dimensions, vec2(array_dimensions.xy)))) { continue; }
			if (!npr_view_array_ready(int(mapping.w))) { continue; }
			vec4 identity = npr_view_record(8, row);
			uint view_layers = uint(identity.z) | (uint(identity.w) << 16u);
			mat4 world = mat4(npr_view_record(0, row), npr_view_record(1, row), npr_view_record(2, row), npr_view_record(3, row));
			mat4 view_projection = mat4(npr_view_record(4, row), npr_view_record(5, row), npr_view_record(6, row), npr_view_record(7, row));
			if (npr_same_depth_view(camera_world, projection, dimensions, layers,
					world, view_projection, identity.xy, view_layers)) {
				return float(row + 3);
			}
		}
	}
	if (u_npr_depth_enabled && npr_same_depth_view(camera_world, projection, dimensions, layers,
			u_npr_depth_camera_world, u_npr_depth_projection, u_npr_depth_viewport_size, u_npr_depth_layers)) {
		return 1.0;
	}
	if (u_npr_depth_enabled_secondary && npr_same_depth_view(camera_world, projection, dimensions, layers,
			u_npr_depth_camera_world_secondary, u_npr_depth_projection_secondary, u_npr_depth_viewport_size_secondary, u_npr_depth_layers_secondary)) {
		return 2.0;
	}
	return 0.0;
}

ivec2 npr_depth_texel(vec2 uv, ivec2 dimensions) {
	return clamp(ivec2(floor(uv * vec2(dimensions))), ivec2(0), dimensions - ivec2(1));
}

float npr_read_depth(sampler2D source, vec2 uv, float depth_range) {
	if (any(lessThan(uv, vec2(0.0))) || any(greaterThan(uv, vec2(1.0)))) {
		return depth_range + 1.0;
	}
	vec3 data;
	if (u_npr_depth_integer_sampling) {
		data = texelFetch(source, npr_depth_texel(uv, textureSize(source, 0)), 0).rgb;
	} else {
		data = textureLod(source, uv, 0.0).rgb;
	}
	return data.b > 0.5 ? dot(data.rg, vec2(1.0, 1.0 / 256.0)) * depth_range : depth_range + 1.0;
}

float npr_unpack_array_depth(vec3 data, float depth_range) {
	if (u_npr_depth_array_compact) {
		return data.r >= 0.0 ? data.r * depth_range : depth_range + 1.0;
	}
	return data.b > 0.5 ? dot(data.rg, vec2(1.0, 1.0 / 256.0)) * depth_range : depth_range + 1.0;
}

float npr_read_array_depth(vec2 uv, int layer, vec2 source_size, float depth_range) {
	if (any(lessThan(uv, vec2(0.0))) || any(greaterThan(uv, vec2(1.0)))) {
		return depth_range + 1.0;
	}
	if (u_npr_depth_integer_sampling) {
		vec3 data = texelFetch(u_npr_depth_array, ivec3(npr_depth_texel(uv, ivec2(source_size)), layer), 0).rgb;
		return npr_unpack_array_depth(data, depth_range);
	}
	// Clamp to the source rectangle's texel centers, not the padded array edge.
	// In particular UV == 1 must retain the direct sampler's clamp semantics.
	vec2 half_texel = vec2(0.5) / source_size;
	vec2 scale = source_size / vec2(textureSize(u_npr_depth_array, 0).xy);
	// Divide dimensions before scaling UV: equal-sized layers must keep UV
	// unchanged, not round it through multiply/divide across a nearest boundary.
	vec2 array_uv = clamp(uv, half_texel, vec2(1.0) - half_texel) * scale;
	vec3 data = textureLod(u_npr_depth_array, vec3(array_uv, float(layer)), 0.0).rgb;
	return npr_unpack_array_depth(data, depth_range);
}

bool npr_array_ready(bool secondary) {
	int view_index = secondary ? u_npr_depth_view_index_secondary : u_npr_depth_view_index;
	if (view_index >= 0) {
		return npr_view_array_ready(view_index);
	}
	return !u_npr_depth_array_guard || textureLod(u_npr_depth_array_ready, vec2(0.5), 0.0).r > 0.5;
}

float npr_character_depth_at(vec2 uv, bool secondary, float depth_range) {
	if (npr_depth_view_slot >= 2.5) {
		int row = int(npr_depth_view_slot) - 3;
		vec4 mapping = npr_view_record(10, row);
		if (!npr_view_array_ready(int(mapping.w))) { return depth_range + 1.0; }
		return npr_read_array_depth(uv, int(mapping.z), npr_view_record(11, row).xy, depth_range);
	}
	int base = secondary ? u_npr_depth_array_base_secondary : u_npr_depth_array_base;
	if (base >= 0 && npr_array_ready(secondary)) {
		vec2 size = secondary ? u_npr_depth_array_sizes_secondary.xy : u_npr_depth_array_sizes.xy;
		return npr_read_array_depth(uv, base, size, depth_range);
	}
	if (secondary) {
		return npr_read_depth(u_npr_character_depth_secondary, uv, depth_range);
	}
	return npr_read_depth(u_npr_character_depth, uv, depth_range);
}

float npr_hair_depth_at(vec2 uv, bool secondary, float depth_range) {
	if (npr_depth_view_slot >= 2.5) {
		int row = int(npr_depth_view_slot) - 3;
		vec4 mapping = npr_view_record(10, row);
		if (!npr_view_array_ready(int(mapping.w))) { return depth_range + 1.0; }
		return npr_read_array_depth(uv, int(mapping.z) + 1, npr_view_record(11, row).zw, depth_range);
	}
	int base = secondary ? u_npr_depth_array_base_secondary : u_npr_depth_array_base;
	if (base >= 0 && npr_array_ready(secondary)) {
		vec2 size = secondary ? u_npr_depth_array_sizes_secondary.zw : u_npr_depth_array_sizes.zw;
		return npr_read_array_depth(uv, base + 1, size, depth_range);
	}
	if (secondary) {
		return npr_read_depth(u_npr_hair_depth_secondary, uv, depth_range);
	}
	return npr_read_depth(u_npr_hair_depth, uv, depth_range);
}

float npr_depth_rim(vec2 uv, vec3 normal_vs, float view_depth) {
	uv += npr_depth_uv_offset;
	vec2 direction = vec2(normal_vs.x, -normal_vs.y);
	direction /= max(length(direction), 0.001);
	bool secondary = npr_depth_view_slot > 1.5;
	float depth_range = secondary ? u_npr_depth_range_secondary : u_npr_depth_range;
	vec2 pixel = secondary ? u_npr_depth_pixel_size_secondary : u_npr_depth_pixel_size;
	float height = secondary ? u_npr_character_height_secondary : u_npr_character_height;
	if (npr_depth_view_slot >= 2.5) {
		int row = int(npr_depth_view_slot) - 3;
		vec4 parameters = npr_view_record(9, row);
		depth_range = parameters.y;
		height = parameters.z;
		pixel = npr_view_record(10, row).xy;
	}
	if (pixel.x <= 0.0) {
		pixel = secondary ? 1.0 / vec2(textureSize(u_npr_character_depth_secondary, 0)) : 1.0 / vec2(textureSize(u_npr_character_depth, 0));
	}
	float threshold = height * 0.008;
	float rim = 0.0;
	for (int tap = 1; tap <= 3; tap++) {
		vec2 sample_uv = uv + direction * pixel * u_npr_depth_rim_pixels * float(tap) / 3.0;
		float gap = npr_character_depth_at(sample_uv, secondary, depth_range) - view_depth;
		rim += smoothstep(threshold, threshold * 2.0, gap) / 3.0;
	}
	return rim;
}

float npr_hair_contact(vec2 uv, vec3 position_vs, vec3 light_vs, mat4 projection) {
	if (npr_depth_view_slot < 0.5 || u_npr_contact_strength <= 0.0) {
		return 0.0;
	}
	uv += npr_depth_uv_offset;
	bool secondary = npr_depth_view_slot > 1.5;
	float depth_range = secondary ? u_npr_depth_range_secondary : u_npr_depth_range;
	float height = secondary ? u_npr_character_height_secondary : u_npr_character_height;
	vec2 pixel = secondary ? u_npr_depth_pixel_size_secondary : u_npr_depth_pixel_size;
	int steps = clamp(secondary ? u_npr_contact_steps_secondary : u_npr_contact_steps, 2, 4);
	if (npr_depth_view_slot >= 2.5) {
		int row = int(npr_depth_view_slot) - 3;
		vec4 parameters = npr_view_record(9, row);
		depth_range = parameters.y;
		height = parameters.z;
		steps = clamp(int(parameters.w), 2, 4);
		pixel = npr_view_record(10, row).xy;
	}
	float reach = height * 0.012;
	float bias = height * 0.001;
	if (pixel.x <= 0.0) {
		pixel = secondary ? 1.0 / vec2(textureSize(u_npr_hair_depth_secondary, 0)) : 1.0 / vec2(textureSize(u_npr_hair_depth, 0));
	}
	vec2 focal = vec2(abs(projection[0][0]), -abs(projection[1][1]));
	float perspective = abs(projection[3][3]) < 0.5 ? max(-position_vs.z, 0.01) : 1.0;
	vec2 offset = focal * light_vs.xy * reach / (2.0 * perspective);
	float occlusion = 0.0;
	for (int tap = 1; tap <= steps; tap++) {
		float t = float(tap) / float(steps);
		vec2 sample_uv = uv + offset * t;
		float probe_depth = -position_vs.z - light_vs.z * reach * t;
		// A small cross filter smooths the hair silhouette, without blending packed depth.
		for (int side = -1; side <= 1; side++) {
			float hair_depth = npr_hair_depth_at(sample_uv + vec2(float(side) * pixel.x, 0.0), secondary, depth_range);
			float gap = probe_depth - hair_depth;
			float close_enough = 1.0 - smoothstep(height * 0.045, height * 0.065, gap);
			occlusion += smoothstep(bias, bias * 3.0, gap) * close_enough / float(steps * 3);
		}
	}
	return occlusion * u_npr_contact_strength;
}
