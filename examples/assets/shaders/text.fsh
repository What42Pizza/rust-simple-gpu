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
	//if (abs(low - high) < 0.000001) return 0.5;
	return (v - low) / (high - low);
}

float distToLine(vec2 linePoint1, vec2 linePoint2, vec2 offset) {
	linePoint1 -= offset;
	linePoint2 -= offset;
	return abs(linePoint1.x * linePoint2.y - linePoint1.y * linePoint2.x) / length(linePoint1 - linePoint2);
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
	
	// holds pos.x, pos.y, sample
	vec3 sampleA = vec3(0.0, 0.4, 0.0);
	vec3 sampleB = vec3(-0.5, 0.2, 0.0);
	vec3 sampleC = vec3(0.5, 0.2, 0.0);
	
	vec2 invTargetSize = 1.0 / targetSize;
	sampleA.z = texture(atlas, texcoord + sampleA.xy * invTargetSize).r;
	sampleB.z = texture(atlas, texcoord + sampleB.xy * invTargetSize).r;
	sampleC.z = texture(atlas, texcoord + sampleC.xy * invTargetSize).r;
	
	float diffAB = abs(sampleA.z - sampleB.z);
	float diffBC = abs(sampleB.z - sampleC.z);
	float diffCA = abs(sampleC.z - sampleA.z);
	
	vec3 centerPos = sampleC, offset1 = sampleA, offset2 = sampleB;
	if (diffBC < diffAB && diffBC < diffCA) {
		centerPos = sampleA;
		offset1 = sampleB;
		offset2 = sampleC;
	}
	if (diffCA < diffAB && diffCA < diffBC) {
		centerPos = sampleB;
		offset1 = sampleA;
		offset2 = sampleC;
	}
	vec2 linePoint1 = mix(offset1.xy, centerPos.xy, invMix(offset1.z, centerPos.z, 0.49));
	vec2 linePoint2 = mix(offset2.xy, centerPos.xy, invMix(offset2.z, centerPos.z, 0.49));
	
	vec3 mixFactors = vec3(
		1.0 - 0.75 * distToLine(linePoint1, linePoint2, vec2(-0.25, 0.0)),
		1.0 - 0.75 * distToLine(linePoint1, linePoint2, vec2(0.0, 0.0)),
		1.0 - 0.75 * distToLine(linePoint1, linePoint2, vec2(0.25, 0.0))
	);
	if (texture(atlas, texcoord).r < 0.5) {
		mixFactors = vec3(1.0);
	}
	
	
	
	//float leftSample = texture(atlas, leftPos).r;
	//float rightSample = texture(atlas, rightPos).r;
	//if (leftSample < 0.5 && rightSample < 0.5) {
	//	frag_color = foregroundColor;
	//	return;
	//}
	//if (leftSample > 0.5 && rightSample > 0.5) {
	//	discard;
	//	return;
	//}
	
	//float lineYIntercept = invMix(leftSample, rightSample, 0.5) * 2.0 - 1.0;
	//float lineXIntercept = mix(-1.0, 1.0, lineYIntercept);
	//float m = sign(leftSample - rightSample) * 0.15;
	//float b = 0.5 - lineXIntercept * m;
	//vec3 mixFactors = vec3(
	//	invMix(0.0, 0.5, b - m),
	//	invMix(0.0, 0.5, b),
	//	invMix(0.0, 0.5, b + m)
	//);
	
	frag_color = vec4(
		mix(backgroundColor.r, foregroundColor.r, clamp(mixFactors.r, 0.0, 1.0)),
		mix(backgroundColor.g, foregroundColor.g, clamp(mixFactors.g, 0.0, 1.0)),
		mix(backgroundColor.b, foregroundColor.b, clamp(mixFactors.b, 0.0, 1.0)),
		1.0
	);
	
	if (gl_FragCoord.x > mousePos.x) {
		frag_color = texture(atlas, texcoord);
	}
	
}
