#version 450

layout(location = 0) in vec2 texcoord;

layout(location = 0) out vec4 frag_color;

layout(set = 0, binding = 0) uniform texture2D higher_mip;
layout(set = 0, binding = 1) uniform sampler filtering_sampler;

void main() {
	frag_color = texture(sampler2D(higher_mip, filtering_sampler), texcoord);
}
