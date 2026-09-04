#version 450

layout(location = 0) in vec4 vert_color;
layout(location = 1) in vec2 vert_uv;

layout(location = 0) out vec4 frag_color;

layout(set = 1, binding = 0) uniform texture2D tex;
layout(set = 1, binding = 1) uniform sampler tex_sampler;

#define sample(tex, tex_sampler, uv) texture(sampler2D(tex, tex_sampler), uv)

void main() {
	frag_color = sample(tex, tex_sampler, vert_uv) * vert_color;
}
