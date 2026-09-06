#version 450

layout(location = 0) in vec4 vert_color;
layout(location = 1) in vec2 vert_uv;

layout(location = 0) out vec4 frag_color;

layout(set = 1, binding = 0) uniform texture2D tex;
layout(set = 1, binding = 1) uniform sampler tex_sampler_filtering;
layout(set = 1, binding = 2) uniform sampler tex_sampler_non_filtering;

#define sample_linear(tex, tex_sampler, uv) texture(sampler2D(tex, tex_sampler_filtering), uv)
#define sample_nearest(tex, tex_sampler, uv) texture(sampler2D(tex, tex_sampler_non_filtering), uv)

void main() {
	frag_color = sample_nearest(tex, tex_sampler, vert_uv) * vert_color;
}
