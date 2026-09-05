use crate::{GpuInstance, IndexBuffer, Texture};



/// Creates a basic 2d rendering pipeline, with no back-face culling and the vertex list treated as a triangles list
#[must_use]
pub fn create_2d_pipeline(
	name: &str,
	vertex_buffer_layouts: &[Option<wgpu::VertexBufferLayout>],
	vertex_shader: &wgpu::ShaderModule,
	fragment_shader: &wgpu::ShaderModule,
	output_format: &wgpu::TextureFormat,
	gpu_instance: &GpuInstance,
) -> wgpu::RenderPipeline {
	gpu_instance
		.wgpu_device
		.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
			label: Some(name),
			layout: Some(&gpu_instance.wgpu_pipeline_layout),
			vertex: wgpu::VertexState {
				module: vertex_shader,
				entry_point: None,
				buffers: vertex_buffer_layouts,
				compilation_options: wgpu::PipelineCompilationOptions::default(),
			},
			fragment: Some(wgpu::FragmentState {
				module: fragment_shader,
				entry_point: None,
				targets: &[Some(wgpu::ColorTargetState {
					format: *output_format,
					blend: Some(wgpu::BlendState::ALPHA_BLENDING),
					write_mask: wgpu::ColorWrites::ALL,
				})],
				compilation_options: wgpu::PipelineCompilationOptions::default(),
			}),
			primitive: wgpu::PrimitiveState::default(),
			depth_stencil: None,
			multisample: wgpu::MultisampleState::default(),
			multiview_mask: None,
			cache: None,
		})
}

/// Creates a basic 3d rendering pipeline with back-face culling, counter-clockwise triangles, a 24-bit float depth buffer expected, and the vertex list treated as a triangles list
#[must_use]
pub fn create_3d_pipeline(
	name: &str,
	vertex_buffer_layouts: &[Option<wgpu::VertexBufferLayout>],
	vertex_shader: &wgpu::ShaderModule,
	fragment_shader: &wgpu::ShaderModule,
	output_format: &wgpu::TextureFormat,
	gpu_instance: &GpuInstance,
) -> wgpu::RenderPipeline {
	gpu_instance
		.wgpu_device
		.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
			label: Some(name),
			layout: Some(&gpu_instance.wgpu_pipeline_layout),
			vertex: wgpu::VertexState {
				module: vertex_shader,
				entry_point: None,
				buffers: vertex_buffer_layouts,
				compilation_options: wgpu::PipelineCompilationOptions::default(),
			},
			fragment: Some(wgpu::FragmentState {
				module: fragment_shader,
				entry_point: None,
				targets: &[Some(wgpu::ColorTargetState {
					format: *output_format,
					blend: Some(wgpu::BlendState::ALPHA_BLENDING),
					write_mask: wgpu::ColorWrites::ALL,
				})],
				compilation_options: wgpu::PipelineCompilationOptions::default(),
			}),
			primitive: wgpu::PrimitiveState {
				topology: wgpu::PrimitiveTopology::TriangleList,
				strip_index_format: None,
				front_face: wgpu::FrontFace::default(),
				cull_mode: Some(wgpu::Face::Back),
				polygon_mode: wgpu::PolygonMode::Fill,
				unclipped_depth: false,
				conservative: false,
			},
			depth_stencil: Some(wgpu::DepthStencilState {
				format: wgpu::TextureFormat::Depth24Plus,
				depth_write_enabled: Some(true),
				depth_compare: Some(wgpu::CompareFunction::Less),
				stencil: wgpu::StencilState::default(),
				bias: wgpu::DepthBiasState::default(),
			}),
			multisample: wgpu::MultisampleState::default(),
			multiview_mask: None,
			cache: None,
		})
}



/// Starts a 2d render pass (note: this one render pass can be used by multiple pipelines)
pub fn start_2d_render_pass<'a>(
	name: &str,
	texture_output: &wgpu::TextureView,
	clear_color: Option<wgpu::Color>,
	command_encoder: &'a mut wgpu::CommandEncoder,
) -> wgpu::RenderPass<'a> {
	let load_od = if let Some(clear_color) = clear_color {
		wgpu::LoadOp::Clear(clear_color)
	} else {
		wgpu::LoadOp::Load
	};
	command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
		label: Some(name),
		color_attachments: &[Some(wgpu::RenderPassColorAttachment {
			view: texture_output,
			depth_slice: None,
			resolve_target: None,
			ops: wgpu::Operations {
				load: load_od,
				store: wgpu::StoreOp::Store,
			},
		})],
		depth_stencil_attachment: None,
		timestamp_writes: None,
		occlusion_query_set: None,
		multiview_mask: None,
	})
}

/// Starts a 3d render pass (note: this one render pass can be used by multiple pipelines)
pub fn start_3d_render_pass<'a>(
	name: &str,
	texture_output: &wgpu::TextureView,
	depth_tex: &wgpu::TextureView,
	clear_color: Option<wgpu::Color>,
	clear_depth: bool,
	command_encoder: &'a mut wgpu::CommandEncoder,
) -> wgpu::RenderPass<'a> {
	let load_od = if let Some(clear_color) = clear_color {
		wgpu::LoadOp::Clear(clear_color)
	} else {
		wgpu::LoadOp::Load
	};
	command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
		label: Some(name),
		color_attachments: &[Some(wgpu::RenderPassColorAttachment {
			view: texture_output,
			depth_slice: None,
			resolve_target: None,
			ops: wgpu::Operations {
				load: load_od,
				store: wgpu::StoreOp::Store,
			},
		})],
		depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
			view: depth_tex,
			depth_ops: Some(wgpu::Operations {
				load: if clear_depth {
					wgpu::LoadOp::Clear(1.0)
				} else {
					wgpu::LoadOp::Load
				},
				store: wgpu::StoreOp::Store,
			}),
			stencil_ops: None,
		}),
		timestamp_writes: None,
		occlusion_query_set: None,
		multiview_mask: None,
	})
}



/// Finishes a render pass
///
/// This is the same as `drop(render_pass)`, it just has a nicer name
#[allow(unused_variables)]
#[allow(clippy::needless_pass_by_value)]
pub fn finish_render_pass(render_pass: wgpu::RenderPass) {}



/// Renders 2d or 3d data with a given pipeline, texture, and uniforms
///
/// Notes:
/// - this works with both 2d and 3d pipelines
/// - the given texture can be a uniform that is reused for multiple renders
/// - this assumes that the index buffer is completely full
pub fn render(
	render_pass: &mut wgpu::RenderPass,
	pipeline: &wgpu::RenderPipeline,
	vertex_buffers: &[&wgpu::Buffer],
	index_buffer: Option<&IndexBuffer>,
	texture_input: &Texture,
	uniforms: &wgpu::BindGroup,
	vertex_count: u32,
	instance_count: u32,
) {
	render_pass.set_pipeline(pipeline);
	render_pass.set_bind_group(0, uniforms, &[]);
	render_pass.set_bind_group(1, &texture_input.wgpu_bind_group, &[]);

	for (i, buffer) in vertex_buffers.iter().enumerate() {
		#[allow(clippy::cast_possible_truncation)]
		render_pass.set_vertex_buffer(i as u32, buffer.slice(..));
	}
	if let Some(index_buffer) = index_buffer {
		render_pass.set_index_buffer(
			index_buffer.wgpu_buffer.slice(..),
			wgpu::IndexFormat::Uint16,
		);
		render_pass.draw_indexed(0..index_buffer.count, 0, 0..instance_count);
	} else {
		render_pass.draw(0..vertex_count, 0..instance_count);
	}
}
