use crate::GpuInstance;
use std::marker::PhantomData;



/// Holds all the uniform data. It is suggested that only one of these is made, and that it is updated exactly once per frame
pub struct UniformsBuffer<UniformsRawData> {
	/// A handle to the gpu buffer
	pub wgpu_buffer: wgpu::Buffer,
	/// This is a bind group with just one binding, which is a link to this struct's `wgpu::Buffer`. The layout for this is taken from `gpu_instance.wgpu_uniforms_bind_group_layout`
	pub wgpu_bind_group: wgpu::BindGroup,
	#[doc(hidden)]
	_phantom: PhantomData<UniformsRawData>,
}

/// Creates the buffer that stores uniform data
#[inline]
#[must_use]
pub fn create_uniforms_buffer<UniformsRawData>(
	gpu_instance: &GpuInstance,
) -> UniformsBuffer<UniformsRawData> {
	debug_assert!(
		(std::mem::size_of::<UniformsRawData>() as u64).is_multiple_of(wgpu::COPY_BUFFER_ALIGNMENT),
		"Size of uniforms data MUST be divisible by `wgpu::COPY_BUFFER_ALIGNMENT`, which is {}",
		wgpu::COPY_BUFFER_ALIGNMENT
	);

	let buffer = gpu_instance
		.wgpu_device
		.create_buffer(&wgpu::BufferDescriptor {
			label: Some("uniforms"),
			size: std::mem::size_of::<UniformsRawData>() as u64,
			usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
			mapped_at_creation: false,
		});

	let bind_group = gpu_instance
		.wgpu_device
		.create_bind_group(&wgpu::BindGroupDescriptor {
			label: Some("uniforms"),
			layout: &gpu_instance.wgpu_uniforms_bind_group_layout,
			entries: &[wgpu::BindGroupEntry {
				binding: 0,
				resource: wgpu::BindingResource::Buffer(buffer.as_entire_buffer_binding()),
			}],
		});

	UniformsBuffer {
		wgpu_buffer: buffer,
		wgpu_bind_group: bind_group,
		_phantom: PhantomData,
	}
}

/// Updates the uniforms buffer with new data
#[inline]
pub fn update_uniforms_buffer<UniformsRawData: bytemuck::Pod>(
	uniforms_buffer: &UniformsBuffer<UniformsRawData>,
	uniforms_raw_data: &UniformsRawData,
	gpu_instance: &GpuInstance,
) {
	gpu_instance.wgpu_queue.write_buffer(
		&uniforms_buffer.wgpu_buffer,
		0,
		bytemuck::bytes_of(uniforms_raw_data),
	);
}
