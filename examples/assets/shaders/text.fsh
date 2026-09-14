#version 450

layout(location = 0) in vec2 texcoord;

layout(location = 0) out vec4 frag_color;

layout(set = 0, binding = 0) uniform all_uniforms {
	uvec2 screenSize;
};

layout(set = 1, binding = 0) uniform texture2D atlas;
layout(set = 1, binding = 1) uniform sampler filtering_sampler;

float samplePos(vec2 pos) {
	float v = texture(sampler2D(atlas, filtering_sampler), pos).r;
	v -= 0.5;
	v *= screenSize.x;
	v /= 8.0;
	v = clamp(v, 0.0, 1.0);
	return pow(v, 0.75);
}

void main() {
	vec2 texSize = textureSize(sampler2D(atlas, filtering_sampler), 0);
	float pixelOverThree = 1.0 / 4.0 / screenSize.x;
	float left = samplePos(texcoord - vec2(pixelOverThree, 0.0));
	float right = samplePos(texcoord + vec2(pixelOverThree, 0.0));
	frag_color.rgb = vec3(left, (left + right) * 0.5, right);
	frag_color.a = 1.0 - (left + right) / 2.0;
}
