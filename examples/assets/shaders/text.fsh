#version 450

layout(location = 0) in vec2 texcoord;

layout(location = 0) out vec4 frag_color;

layout(set = 0, binding = 0) uniform all_uniforms {
	uvec2 targetSize;
	uint textColorPacked;
	uint backgroundColorPacked;
	uvec2 mousePos;
};

layout(set = 1, binding = 0) uniform texture2D atlas_tex;
layout(set = 1, binding = 1) uniform sampler filtering_sampler;
#define atlas sampler2D(atlas_tex, filtering_sampler)

float invMix(float low, float high, float v) {
	return (v - low) / (high - low);
}

float samplePos(vec2 pos, float blur) {
	
	vec2 posScaled = pos * textureSize(atlas, 0) - 0.5;
	ivec2 posInt = ivec2(posScaled);
	
	vec2 vec_LL = texelFetch(atlas, posInt + ivec2(0, 0), 0).rg * 255.0 - 128.0;
	vec2 vec_LH = texelFetch(atlas, posInt + ivec2(0, 1), 0).rg * 255.0 - 128.0;
	vec2 vec_HL = texelFetch(atlas, posInt + ivec2(1, 0), 0).rg * 255.0 - 128.0;
	vec2 vec_HH = texelFetch(atlas, posInt + ivec2(1, 1), 0).rg * 255.0 - 128.0;
	
	vec2 posWithinTexel = posScaled - posInt;
	vec2 invPosWithinTexel = 1.0 - posWithinTexel;
	float weight_LL = invPosWithinTexel.x * invPosWithinTexel.y;
	float weight_LH = invPosWithinTexel.x * posWithinTexel.y;
	float weight_HL = posWithinTexel.x * invPosWithinTexel.y;
	float weight_HH = posWithinTexel.x * posWithinTexel.y;
	
	vec2 mainSample;
	if (posWithinTexel.x > 0.5) {
		if (posWithinTexel.y > 0.5) {
			mainSample = vec_HH;
		} else {
			mainSample = vec_HL;
		}
	} else {
		if (posWithinTexel.y > 0.5) {
			mainSample = vec_LH;
		} else {
			mainSample = vec_LL;
		}
	}
	
	if (abs(mainSample.x - vec_LL.x) > 4 || abs(mainSample.y - vec_LL.y) > 4) weight_LL = 0.0;
	if (abs(mainSample.x - vec_LH.x) > 4 || abs(mainSample.y - vec_LH.y) > 4) weight_LH = 0.0;
	if (abs(mainSample.x - vec_HL.x) > 4 || abs(mainSample.y - vec_HL.y) > 4) weight_HL = 0.0;
	if (abs(mainSample.x - vec_HH.x) > 4 || abs(mainSample.y - vec_HH.y) > 4) weight_HH = 0.0;
	
	vec2 filteredVec =
		vec_LL * weight_LL +
		vec_LH * weight_LH +
		vec_HL * weight_HL +
		vec_HH * weight_HH;
	filteredVec /= weight_LL + weight_LH + weight_HL + weight_HH;
	
	return clamp(invMix(2.0 + blur, 2.0, length(filteredVec)), 0.0, 1.0);
	
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
	
	vec2 texelCoordFloat = texcoord * textureSize(atlas, 0);
	ivec2 texelCoord = ivec2(texelCoordFloat);
	
	float subPixSize = 0.28 / targetSize.x;
	float blurSize = 2.0 / targetSize.x * textureSize(atlas, 0).x;
	vec3 mixFactors = vec3(
		samplePos(texcoord - vec2(subPixSize, 0.0), blurSize),
		0.0,
		samplePos(texcoord + vec2(subPixSize, 0.0), blurSize)
	);
	mixFactors.g = (mixFactors.r + mixFactors.b) * 0.5;
	
	frag_color = vec4(
		mix(backgroundColor.r, foregroundColor.r, clamp(mixFactors.r, 0.0, 1.0)),
		mix(backgroundColor.g, foregroundColor.g, clamp(mixFactors.g, 0.0, 1.0)),
		mix(backgroundColor.b, foregroundColor.b, clamp(mixFactors.b, 0.0, 1.0)),
		1.0
	);
	
	if (gl_FragCoord.x > mousePos.x) {
		frag_color = texelFetch(atlas, texelCoord, 0);
		frag_color.rg = abs(frag_color.rg * 255.0 - 128.0) / 255.0;
	}
	
}
