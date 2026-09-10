@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
	var output: VertexOutput;

	var position = vec4<f32>(1.0);

	if (vertex_index == 0u) { position = vec4<f32>(-1.0, -1.0, 0.5, 1.0); }
	if (vertex_index == 1u) { position = vec4<f32>( 1.0, -1.0, 0.5, 1.0); }
	if (vertex_index == 2u) { position = vec4<f32>(-1.0,  1.0, 0.5, 1.0); }
	if (vertex_index == 3u) { position = vec4<f32>( 1.0,  1.0, 0.5, 1.0); }

	output.position = position;
	output.texcoord = position.xy * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5);

	return output;
}

struct VertexOutput {
	@builtin(position) position: vec4<f32>,
	@location(0) texcoord: vec2<f32>,
};

@group(0) @binding(0) var higher_mip: texture_2d<f32>;
@group(0) @binding(1) var filtering_sampler: sampler;

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
	return textureSample(higher_mip, filtering_sampler, input.texcoord);
}
