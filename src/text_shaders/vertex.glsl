#version 450

layout(location = 0) in ivec2 pos;
layout(location = 1) in ivec2 size;
layout(location = 2) in uvec4 texCoords;
layout(location = 3) in uint stringId;

layout(location = 0) out vec2 texcoord;

layout(set = 1, binding = 0) uniform all_uniforms {
	ivec2 targetSize;
	ivec2 atlasSize;
};

void main() {
	
	//if (gl_VertexIndex == 0) gl_Position = vec4(-1.0, -1.0, 0.5, 1.0);
	//if (gl_VertexIndex == 1) gl_Position = vec4( 1.0, -1.0, 0.5, 1.0);
	//if (gl_VertexIndex == 2) gl_Position = vec4(-1.0,  1.0, 0.5, 1.0);
	//if (gl_VertexIndex == 3) gl_Position = vec4( 1.0,  1.0, 0.5, 1.0);
	//texcoord = gl_Position.xy * vec2(0.5, -0.5) + 0.5;
	
	gl_Position = vec4(pos, 0.5, 1.0);
	if (gl_VertexIndex % 2 == 0) gl_Position.x += size.x;
	if (gl_VertexIndex % 4 < 2) gl_Position.y += size.y;
	gl_Position.xy /= targetSize;
	gl_Position.xy = gl_Position.xy * 2.0 - 1.0;
	gl_Position.y *= -1.0;
	
	texcoord = texCoords.xy;
	if (gl_VertexIndex % 2 == 0) texcoord.x += texCoords.z;
	if (gl_VertexIndex % 4 < 2) texcoord.y += texCoords.w;
	texcoord /= atlasSize;
	
}
