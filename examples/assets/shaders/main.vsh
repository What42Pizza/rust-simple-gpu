#version 450

layout(location = 0) in vec3 position;
layout(location = 1) in vec2 uv;
layout(location = 2) in vec4 color;

layout(location = 0) out vec4 vert_color;
layout(location = 1) out vec2 vert_uv;

layout(set = 0, binding = 0) uniform Camera {
	mat4 view_mat;
	mat4 proj_mat;
	mat4 view_proj_mat;
	mat4 inv_view_mat;
	mat4 inv_proj_mat;
	mat4 inv_view_proj_mat;
};

void main() {
	gl_Position = view_proj_mat * vec4(position, 1.0);
	gl_Position = vec4(gl_Position.xy, gl_Position.z * 0.5 + 0.5 * gl_Position.w, gl_Position.w);
	vert_color = color;
	vert_uv = uv;
}
