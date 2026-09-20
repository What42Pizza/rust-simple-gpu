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
	
	ivec2 sample_LL_Pos = ivec2(0, 0);
	ivec2 sample_HL_Pos = ivec2(1, 0);
	ivec2 sample_LH_Pos = ivec2(0, 1);
	
	vec2 texelCoordFloat = texcoord * textureSize(atlas, 0);
	ivec2 texelCoord = ivec2(texelCoordFloat);
	float sample_LL = texelFetch(atlas, texelCoord + sample_LL_Pos, 0).r;
	float sample_HL = texelFetch(atlas, texelCoord + sample_HL_Pos, 0).r;
	float sample_LH = texelFetch(atlas, texelCoord + sample_LH_Pos, 0).r;
	const float MAX_STEP = 24.0 / 255.0;
	sample_HL = clamp(sample_HL, sample_LL - MAX_STEP, sample_LL + MAX_STEP);
	sample_LH = clamp(sample_LH, sample_LL - MAX_STEP, sample_LL + MAX_STEP);
	
	vec3 sampleA = vec3((sample_LL_Pos - fract(texelCoordFloat) + 0.5) / textureSize(atlas, 0) * targetSize, sample_LL);
	vec3 sampleB = vec3((sample_HL_Pos - fract(texelCoordFloat) + 0.5) / textureSize(atlas, 0) * targetSize, sample_HL);
	vec3 sampleC = vec3((sample_LH_Pos - fract(texelCoordFloat) + 0.5) / textureSize(atlas, 0) * targetSize, sample_LH);
	
	float diffAB = abs(sampleA.z - sampleB.z);
	float diffBC = abs(sampleB.z - sampleC.z);
	float diffAC = abs(sampleA.z - sampleC.z);
	
	vec3 centerPos = sampleC, offset1 = sampleA, offset2 = sampleB;
	if (diffBC < diffAB && diffBC < diffAC) {
		centerPos = sampleA;
		offset1 = sampleB;
		offset2 = sampleC;
	}
	if (diffAC < diffAB && diffAC < diffBC) {
		centerPos = sampleB;
		offset1 = sampleA;
		offset2 = sampleC;
	}
	vec2 linePoint1 = mix(offset1.xy, centerPos.xy, invMix(offset1.z, centerPos.z, 0.5));
	vec2 linePoint2 = mix(offset2.xy, centerPos.xy, invMix(offset2.z, centerPos.z, 0.5));
	
	vec3 mixFactors = vec3(
		1.0 - 0.75 * distToLine(linePoint1, linePoint2, vec2(-0.25, 0.0)),
		1.0 - 0.75 * distToLine(linePoint1, linePoint2, vec2( 0.0 , 0.0)),
		1.0 - 0.75 * distToLine(linePoint1, linePoint2, vec2( 0.25, 0.0))
	);
	if (sample_LL < 0.5) {
		mixFactors = vec3(1.0);
	}
	
	frag_color = vec4(
		mix(backgroundColor.r, foregroundColor.r, clamp(mixFactors.r, 0.0, 1.0)),
		mix(backgroundColor.g, foregroundColor.g, clamp(mixFactors.g, 0.0, 1.0)),
		mix(backgroundColor.b, foregroundColor.b, clamp(mixFactors.b, 0.0, 1.0)),
		1.0
	);
	
	if (gl_FragCoord.x > mousePos.x) {
		frag_color = texelFetch(atlas, texelCoord, 0);
	}
	
}
