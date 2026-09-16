#version 450

layout(location = 0) in vec2 texcoord;

layout(location = 0) out vec4 frag_color;

layout(set = 0, binding = 0) uniform all_uniforms {
	uvec2 targetSize;
	uint textColorPacked;
	uint backgroundColorPacked;
};

layout(set = 1, binding = 0) uniform texture2D atlas_tex;
layout(set = 1, binding = 1) uniform sampler filtering_sampler;
#define atlas sampler2D(atlas_tex, filtering_sampler)

//float samplePos(vec2 pos) {
//	float v = texture(atlas, pos).r;
//	v -= 0.5;
//	v *= targetSize.x;
//	v /= 6.0;
//	v = clamp(v, 0.0, 1.0);
//	return pow(v, 0.75);
//}

float invMix(float low, float high, float v) {
	return (v - low) /  (high - low);
}

void main() {
	
	vec4 foregroundColor = vec4(
		(textColorPacked & 0xFF) >> 0,
		(textColorPacked & 0xFF00) >> 8,
		(textColorPacked & 0xFF0000) >> 16,
		(textColorPacked & 0xFF000000) >> 24
	) / 255.0;
	vec4 backgroundColor = vec4(
		(backgroundColorPacked & 0xFF) >> 0,
		(backgroundColorPacked & 0xFF00) >> 8,
		(backgroundColorPacked & 0xFF0000) >> 16,
		(backgroundColorPacked & 0xFF000000) >> 24
	) / 255.0;
	
	//vec2 texSize = textureSize(atlas, 0);
	//float pixelOverThree = 1.0 / 3.0 / targetSize.x;
	//float left = samplePos(texcoord - vec2(pixelOverThree, 0.0));
	//float right = samplePos(texcoord + vec2(pixelOverThree, 0.0));
	//frag_color.rgb *= 1.0 - vec3(left, (left + right) * 0.5, right);
	//frag_color.a *= 1.0 - (left + right) / 2.0;
	
	float pixelWidth = 1.0 / targetSize.x;
	vec2 leftPos = texcoord - vec2(pixelWidth * 0.5, 0.0);
	vec2 rightPos = texcoord + vec2(pixelWidth * 0.5, 0.0);
	
	float leftSample = texture(atlas, leftPos).r;
	float rightSample = texture(atlas, rightPos).r;
	if (leftSample < 0.5 && rightSample < 0.5) {
		frag_color = foregroundColor;
		return;
	}
	if (leftSample > 0.5 && rightSample > 0.5) {
		discard;
		return;
	}
	
	float lineYIntercept = invMix(leftSample, rightSample, 0.5) * 2.0 - 1.0;
	float lineXIntercept = mix(-1.0, 1.0, lineYIntercept);
	float m = sign(leftSample - rightSample) * 0.25;
	float b = 0.5 - lineYIntercept * m;
	vec3 mixFactors = vec3(
		invMix(0.0, 0.5, m * -0.75 + b),
		invMix(0.0, 0.5, m * 0.0 + b),
		invMix(0.0, 0.5, m * 0.75 + b)
	);
	
	frag_color = vec4(
		mix(backgroundColor.r, foregroundColor.r, clamp(mixFactors.r, 0.0, 1.0)),
		mix(backgroundColor.g, foregroundColor.g, clamp(mixFactors.g, 0.0, 1.0)),
		mix(backgroundColor.b, foregroundColor.b, clamp(mixFactors.b, 0.0, 1.0)),
		mixFactors.g
	);
	
	//frag_color = vec4(0.0);
	//float v = texture(atlas, texcoord).r;
	//frag_color.a = step(0.5, v);
	
	//frag_color = texture(atlas, texcoord);
	
}
