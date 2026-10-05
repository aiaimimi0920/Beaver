shader_type spatial;
render_mode cull_disabled, unshaded, depth_draw_never;
varying vec3 barycentric;
void vertex() {
	barycentric = COLOR.rgb;
	POSITION = PROJECTION_MATRIX * MODELVIEW_MATRIX * vec4(VERTEX, 1.0);
	POSITION.z += 0.00001 * POSITION.w;
}
void fragment() {
	vec3 pixel = fwidth(barycentric);
	vec3 interior = smoothstep(vec3(0.0), pixel * 1.1, barycentric);
	float edge = 1.0 - min(interior.x, min(interior.y, interior.z));
	if (edge < 0.15) discard;
	ALBEDO = vec3(0.10, 0.15, 0.19);
	ALPHA = edge;
}
