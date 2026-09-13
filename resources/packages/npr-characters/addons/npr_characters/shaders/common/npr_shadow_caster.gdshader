shader_type spatial;
render_mode unshaded, cull_back;
#include "res://addons/npr_characters/shaders/common/npr_vertex.gdshaderinc"
uniform ivec3 geometry_mask = ivec3(255, 0, 0);

void vertex() {
	POSITION = npr_clip_position(PROJECTION_MATRIX, MODELVIEW_MATRIX, VERTEX);
	if (!npr_part_visible(COLOR, geometry_mask.x, geometry_mask.y != 0, geometry_mask.z != 0)) {
		POSITION = vec4(-99.0, -99.0, -99.0, 1.0);
	}
}
