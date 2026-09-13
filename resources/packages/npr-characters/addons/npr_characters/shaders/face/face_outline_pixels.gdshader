shader_type spatial;
#include "res://addons/npr_characters/shaders/common/npr_vertex.gdshaderinc"
render_mode unshaded, cull_front, depth_draw_opaque, shadows_disabled, fog_disabled;

// Pixel width, independent of the source FBX units and undocumented vertex colors.
uniform float outline_width_pixels : hint_range(0.0, 4.0, 0.1) = 1.2;
uniform vec4 outline_color : source_color = vec4(0.22, 0.09, 0.13, 1.0);
uniform ivec3 geometry_mask = ivec3(255, 0, 0);

void vertex() {
	vec4 clip = npr_clip_position(PROJECTION_MATRIX, MODELVIEW_MATRIX, VERTEX);
	vec3 normal_view = MODELVIEW_NORMAL_MATRIX * NORMAL;
	vec4 normal_clip = PROJECTION_MATRIX * vec4(normal_view, 0.0);
	// Quotient derivative of clip.xy / clip.w, normalized in pixel rather than NDC space.
	vec2 pixel_direction = (normal_clip.xy * clip.w - clip.xy * normal_clip.w) * VIEWPORT_SIZE;
	float direction_length = length(pixel_direction);
	if (direction_length > 0.00001 && clip.w > 0.00001) {
		clip.xy += pixel_direction / direction_length * (2.0 * outline_width_pixels / VIEWPORT_SIZE) * clip.w;
	}
	// Do not change Z or W: preserve the engine's Reverse-Z depth convention.
	POSITION = clip;
	if (!npr_part_visible(COLOR, geometry_mask.x, geometry_mask.y != 0, geometry_mask.z != 0)) {
		POSITION = vec4(-99.0, -99.0, -99.0, 1.0);
	}
}

void fragment() {
	ALBEDO = outline_color.rgb;
}
