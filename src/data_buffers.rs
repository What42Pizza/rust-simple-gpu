use crate::GpuInstance;
#[cfg(doc)]
use crate::vertex_buffer_item_type;
use std::{
	marker::PhantomData,
	ops::{Deref, DerefMut},
};



/// The common [`wgpu::BufferUsages`] flags needed for vertex buffers
pub const USAGE_VERTEX_BUFFER: wgpu::BufferUsages = wgpu::BufferUsages::from_bits_truncate(
	wgpu::BufferUsages::VERTEX.bits() | wgpu::BufferUsages::COPY_DST.bits(),
);
/// The common [`wgpu::BufferUsages`] flags needed for index buffers
pub const USAGE_INDEX_BUFFER: wgpu::BufferUsages = wgpu::BufferUsages::from_bits_truncate(
	wgpu::BufferUsages::INDEX.bits() | wgpu::BufferUsages::COPY_DST.bits(),
);
/// The common [`wgpu::BufferUsages`] flags needed for instance buffers (which are the same as vertex buffers)
pub const USAGE_INSTANCE_BUFFER: wgpu::BufferUsages = USAGE_VERTEX_BUFFER;



/// Holds the data for the vertices of a mesh, or the instances of mesh
///
/// Notes:
/// - This can be automatically dereferenced to its [`cpu_copy`](Self::cpu_copy) field
/// - For vertex and instance buffers, consider using [`vertex_buffer_item_type!()`] for creating the item type
pub struct GpuBuffer<ItemRawData: bytemuck::Pod> {
	/// Holds the name of the buffer, only used when reallocating the wgpu buffer
	pub name: String,
	/// Holds a cpu-side copy of the vertex buffer's data
	pub cpu_copy: Vec<ItemRawData>,
	/// A handle to the gpu buffer
	pub wgpu_buffer: wgpu::Buffer,
	/// The number of items currently stored in the gpu buffer
	pub wgpu_buffer_len: u32,
	/// The maximum number of items the gpu buffer can hold. The actual byte size of the buffer is `wgpu_buffer_capacity * size_of::<ItemRawData>()`
	pub wgpu_buffer_capacity: u32,
	/// Lists the usages that this buffer was created with
	pub wgpu_usages: wgpu::BufferUsages,
}

impl<ItemRawData: bytemuck::Pod> Deref for GpuBuffer<ItemRawData> {
	type Target = Vec<ItemRawData>;
	#[inline]
	fn deref(&self) -> &Self::Target {
		&self.cpu_copy
	}
}

impl<ItemRawData: bytemuck::Pod> DerefMut for GpuBuffer<ItemRawData> {
	#[inline]
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.cpu_copy
	}
}

/// Creates a new vertex buffer (which can also be used for instance datas). Note: the byte size of the resulting wgpu buffer is `count * size_of::<ItemRawData>()`
#[inline]
#[must_use]
pub fn create_buffer<ItemRawData: bytemuck::Pod>(
	name: impl Into<String>,
	initial_buffer_capacity: u32,
	usages: wgpu::BufferUsages,
	gpu_instance: &GpuInstance,
) -> GpuBuffer<ItemRawData> {
	let name = name.into();
	let buffer = gpu_instance
		.wgpu_device
		.create_buffer(&wgpu::BufferDescriptor {
			label: Some(&name),
			size: u64::from(initial_buffer_capacity) * std::mem::size_of::<ItemRawData>() as u64,
			usage: usages,
			mapped_at_creation: false,
		});
	GpuBuffer {
		name,
		cpu_copy: vec![],
		wgpu_buffer: buffer,
		wgpu_buffer_len: 0,
		wgpu_buffer_capacity: initial_buffer_capacity,
		wgpu_usages: usages,
	}
}

/// Similar to [`create_buffer()`], but also initializes the buffer with values
#[inline]
#[must_use]
pub fn init_buffer<ItemRawData: bytemuck::Pod>(
	name: impl Into<String>,
	items: impl Into<Vec<ItemRawData>>,
	usages: wgpu::BufferUsages,
	gpu_instance: &GpuInstance,
) -> GpuBuffer<ItemRawData> {
	let items = items.into();
	let items_len = items.len() as u32;
	let name = name.into();
	let buffer = gpu_instance
		.wgpu_device
		.create_buffer(&wgpu::BufferDescriptor {
			label: Some(&name),
			size: u64::from(items_len) * std::mem::size_of::<ItemRawData>() as u64,
			usage: usages,
			mapped_at_creation: false,
		});
	gpu_instance
		.wgpu_queue
		.write_buffer(&buffer, 0, bytemuck::cast_slice(&items));
	GpuBuffer {
		name,
		cpu_copy: items,
		wgpu_buffer: buffer,
		wgpu_buffer_len: items_len,
		wgpu_buffer_capacity: items_len,
		wgpu_usages: usages,
	}
}

/// Sends the data in a [`GpuBuffer`]'s cpu-side buffer into its wgpu buffer
///
/// Note: the wgpu buffer is automatically reallocated if it is not large enough
#[inline]
pub fn sync_buffer<ItemRawData: bytemuck::Pod>(
	vertex_buffer: &mut GpuBuffer<ItemRawData>,
	gpu_instance: &GpuInstance,
) {
	if vertex_buffer.wgpu_buffer_capacity < vertex_buffer.cpu_copy.len() as u32 {
		let new_capacity = (vertex_buffer.cpu_copy.len() * 3 / 2) as u32;
		let buf_desc = wgpu::BufferDescriptor {
			label: Some(&vertex_buffer.name),
			size: u64::from(new_capacity) * std::mem::size_of::<ItemRawData>() as u64,
			usage: vertex_buffer.wgpu_buffer.usage(),
			mapped_at_creation: false,
		};
		vertex_buffer.wgpu_buffer = gpu_instance.wgpu_device.create_buffer(&buf_desc);
		vertex_buffer.wgpu_buffer_capacity = new_capacity;
	}
	gpu_instance.wgpu_queue.write_buffer(
		&vertex_buffer.wgpu_buffer,
		0,
		bytemuck::cast_slice(&vertex_buffer.cpu_copy),
	);
	vertex_buffer.wgpu_buffer_len = vertex_buffer.cpu_copy.len() as u32;
}



/// Creates a type that represents an item in a vertex / instance buffer, with an associated const field `WGPU_LAYOUT: wgpu::VertexBufferLayout<'static>`
///
/// The first token needs to be either `Vertex` or `Instance`, which directly corresponds to [`wgpu::VertexStepMode`]. After that, you define the struct, where each field has a name, type, shader location, and shader format.
///
/// Note: the shader format needs to be a value of [`wgpu::VertexFormat`].
///
/// Example:
///
/// ```
/// // makes a vertex buffer item type called "VertexData"
/// simple_gpu::vertex_buffer_item_type!(Vertex, struct VertexData {
///     pos:   [f32; 3] as location 0: Float32x3,
///     uv:    [f32; 2] as location 1: Float32x2,
///     color: [f32; 4] as location 2: Float32x4,
/// });
/// ```
///
/// This expands to:
///
/// ```
/// #[derive(Copy, Clone, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
/// #[repr(C)]
/// #[allow(missing_docs, clippy::derive_partial_eq_without_eq)]
/// pub struct VertexData {
///    pub pos: [f32; 3],
///    pub uv: [f32; 2],
///    pub color: [f32; 4],
/// }
/// impl VertexData {
///     #[allow(missing_docs)]
///     pub const WGPU_LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
///         array_stride: std::mem::size_of::<VertexData>() as u64,
///         step_mode: wgpu::VertexStepMode::Vertex,
///         attributes: &[
///             (wgpu::VertexAttribute {
///                 format: wgpu::VertexFormat::Float32x3,
///                 offset: 0,
///                 shader_location: 0,
///             }),
///             (wgpu::VertexAttribute {
///                 format: wgpu::VertexFormat::Float32x2,
///                 offset: (0 + wgpu::VertexFormat::Float32x3.size()),
///                 shader_location: 1,
///             }),
///             (wgpu::VertexAttribute {
///                 format: wgpu::VertexFormat::Float32x4,
///                 offset: ((0 + wgpu::VertexFormat::Float32x3.size())
///                     + wgpu::VertexFormat::Float32x2.size()),
///                 shader_location: 2,
///             }),
///         ],
///     };
/// }
/// ```
#[macro_export]
macro_rules! vertex_buffer_item_type {
	($step_mode:ident, struct $struct_name:ident { $( $field_name:ident : $field_type:ty as location $field_loc:tt : $field_data:ident , )+ }) => {
		#[derive(Copy, Clone, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
		#[repr(C)]
		#[allow(missing_docs, clippy::derive_partial_eq_without_eq)]
		pub struct $struct_name {
			$(
				pub $field_name: $field_type,
			)+
		}

		impl $struct_name {
			#[allow(missing_docs)]
			pub const WGPU_LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
				array_stride: std::mem::size_of::<$struct_name>() as u64,
				step_mode: wgpu::VertexStepMode::$step_mode,
				attributes: &wgpu::vertex_attr_array![
					$(
						$field_loc => $field_data,
					)+
				],
			};
		}
	};
}



/// Holds all the uniform data. It is suggested that only one of these is made, and that it is updated exactly once per frame
pub struct UniformsBuffer<UniformsRawData> {
	/// A handle to the gpu buffer
	pub wgpu_buffer: wgpu::Buffer,
	/// This is a bind group with just one binding, which is a link to this struct's [`wgpu::Buffer`]. The layout for this is taken from [`GpuInstance::wgpu_uniforms_bind_group_layout`]
	pub wgpu_bind_group: wgpu::BindGroup,
	#[doc(hidden)]
	pub _phantom: PhantomData<UniformsRawData>,
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
