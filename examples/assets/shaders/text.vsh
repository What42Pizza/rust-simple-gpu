#version 450

layout(location = 0) out vec2 texcoord;

void main() {
	gl_Position = vec4(1.0);
	if (gl_VertexIndex == 0) gl_Position = vec4(-1.0, -1.0, 0.5, 1.0);
	if (gl_VertexIndex == 1) gl_Position = vec4( 1.0, -1.0, 0.5, 1.0);
	if (gl_VertexIndex == 2) gl_Position = vec4(-1.0,  1.0, 0.5, 1.0);
	if (gl_VertexIndex == 3) gl_Position = vec4( 1.0,  1.0, 0.5, 1.0);
	texcoord = gl_Position.xy * vec2(0.5, -0.5) + 0.5;
}
