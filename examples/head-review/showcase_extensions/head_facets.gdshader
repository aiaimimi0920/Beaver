shader_type spatial;
render_mode cull_disabled, unshaded;
varying vec3 view_position;
void vertex() {
	view_position = (MODELVIEW_MATRIX * vec4(VERTEX, 1.0)).xyz;
}
void fragment() {
	vec3 n = normalize(cross(dFdx(view_position), dFdy(view_position)));
	float shade = 0.45 + 0.55 * abs(dot(n, normalize(vec3(-0.3, 0.7, 1.0))));
	ALBEDO = vec3(0.72, 0.76, 0.80) * shade;
}
