use crate::GpuInstance;
use std::{
	marker::PhantomData,
	ops::{Deref, DerefMut},
};



/// Holds the data for the vertices of a mesh, or the instances of mesh
///
///
///
/// Note: this can be automatically dereferenced to its `cpu_buffer` field
pub struct VertexBuffer<VertexRawData: BufferItemRawData> {
	/// Holds a cpu-side copy of the vertex buffer's data
	pub cpu_buffer: Vec<VertexRawData>,
	/// A handle to the gpu buffer
	pub wgpu_buffer: wgpu::Buffer,
	/// The number of items currently stored in the gpu buffer
	pub wgpu_buffer_len: u32,
	/// The maximum number of items the gpu buffer can hold. The actual byte size of the buffer is `wgpu_buffer_capacity * size_of::<VertexRawData>()`
	pub wgpu_buffer_capacity: u32,
	/// Holds the name of the buffer, only used when reallocating the wgpu buffer
	pub name: String,
}

impl<VertexRawData: BufferItemRawData> Deref for VertexBuffer<VertexRawData> {
	type Target = Vec<VertexRawData>;
	fn deref(&self) -> &Self::Target {
		&self.cpu_buffer
	}
}

impl<VertexRawData: BufferItemRawData> DerefMut for VertexBuffer<VertexRawData> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.cpu_buffer
	}
}

/// Creates a new vertex buffer (which can also be used for instance datas). Note: the byte size of the resulting wgpu buffer is `count * size_of::<VertexRawData>()`
pub fn create_vertex_buffer<VertexRawData: BufferItemRawData>(
	name: impl Into<String>,
	wgpu_buffer_capacity: u32,
	gpu_instance: &GpuInstance,
) -> VertexBuffer<VertexRawData> {
	let name = name.into();
	let buffer = gpu_instance
		.wgpu_device
		.create_buffer(&wgpu::BufferDescriptor {
			label: Some(&name),
			size: u64::from(wgpu_buffer_capacity) * std::mem::size_of::<VertexRawData>() as u64,
			usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
			mapped_at_creation: false,
		});
	VertexBuffer {
		cpu_buffer: vec![],
		wgpu_buffer: buffer,
		wgpu_buffer_len: 0,
		wgpu_buffer_capacity,
		name,
	}
}

/// Similar to `create_vertex_buffer()`, but also initializes the buffer with values
pub fn init_vertex_buffer<VertexRawData: BufferItemRawData>(
	name: impl Into<String>,
	items: impl Into<Vec<VertexRawData>>,
	gpu_instance: &GpuInstance,
) -> VertexBuffer<VertexRawData> {
	let items = items.into();
	#[allow(clippy::cast_possible_truncation)]
	let items_len = items.len() as u32;
	let name = name.into();
	let buffer = gpu_instance
		.wgpu_device
		.create_buffer(&wgpu::BufferDescriptor {
			label: Some(&name),
			size: u64::from(items_len) * std::mem::size_of::<VertexRawData>() as u64,
			usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
			mapped_at_creation: false,
		});
	gpu_instance
		.wgpu_queue
		.write_buffer(&buffer, 0, bytemuck::cast_slice(&items));
	VertexBuffer {
		cpu_buffer: items,
		wgpu_buffer: buffer,
		wgpu_buffer_len: items_len,
		wgpu_buffer_capacity: items_len,
		name,
	}
}

/// Sends the data in a [`VertexBuffer`]'s cpu-side buffer into its wgpu buffer
///
/// Note: the wgpu buffer is automatically reallocated if it is not large enough
#[allow(clippy::cast_possible_truncation)]
pub fn sync_vertex_buffer<VertexRawData: BufferItemRawData>(
	vertex_buffer: &mut VertexBuffer<VertexRawData>,
	gpu_instance: &GpuInstance,
) {
	if vertex_buffer.wgpu_buffer_capacity < vertex_buffer.cpu_buffer.len() as u32 {
		let new_capacity = (vertex_buffer.cpu_buffer.len() * 3 / 2) as u32;
		let buf_desc = wgpu::BufferDescriptor {
			label: Some(&vertex_buffer.name),
			size: u64::from(new_capacity) * std::mem::size_of::<VertexRawData>() as u64,
			usage: vertex_buffer.wgpu_buffer.usage(),
			mapped_at_creation: false,
		};
		vertex_buffer.wgpu_buffer = gpu_instance.wgpu_device.create_buffer(&buf_desc);
		vertex_buffer.wgpu_buffer_capacity = new_capacity;
	}
	gpu_instance.wgpu_queue.write_buffer(
		&vertex_buffer.wgpu_buffer,
		0,
		bytemuck::cast_slice(&vertex_buffer.cpu_buffer),
	);
	vertex_buffer.wgpu_buffer_len = vertex_buffer.cpu_buffer.len() as u32;
}



/// Holds the list of indices that are used to connect vertices into triangles when rendering. Note: it is always assumed that indices are u16 values
pub struct IndexBuffer {
	/// A handle to the gpu buffer
	pub wgpu_buffer: wgpu::Buffer,
	/// The number of indices this holds
	pub count: u32,
}

/// Creates a new index buffer
#[must_use]
pub fn create_index_buffer(name: &str, indices: &[u16], gpu_instance: &GpuInstance) -> IndexBuffer {
	let buffer = gpu_instance
		.wgpu_device
		.create_buffer(&wgpu::BufferDescriptor {
			label: Some(name),
			size: indices.len() as u64 * 2,
			usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
			mapped_at_creation: false,
		});
	gpu_instance
		.wgpu_queue
		.write_buffer(&buffer, 0, bytemuck::cast_slice(indices));
	#[allow(clippy::cast_possible_truncation)]
	IndexBuffer {
		wgpu_buffer: buffer,
		count: indices.len() as u32,
	}
}



/// Represents a type that can be put in a [`VertexBuffer`], and can be used for either vertex datas or instance datas, and should be created with [`crate::make_vertex_buffer_type!()`]
pub trait BufferItemRawData: bytemuck::Pod {
	/// Defines the data layout of each item
	const WGPU_LAYOUT: wgpu::VertexBufferLayout<'static>;
}

/// Creates a type that represents an item in a vertex / instance buffer
///
/// The first token needs to be either `Vertex` or `Instance`, which directly corresponds to [`wgpu::VertexStepMode`]. After that, you define the struct, where each field has a name, type, shader location, and shader format.
///
/// Example:
///
/// ```
/// // makes a vertex buffer item type called "VertexData"
/// make_vertex_buffer_type!(Vertex, struct VertexData {
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
/// pub struct VertexData {
///     pub pos:   [f32; 3],
///     pub uv:    [f32; 2],
///     pub color: [f32; 4],
/// }
/// impl simple_gpu::BufferItemRawData for VertexData {
///     const WGPU_LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
///         array_stride: std::mem::size_of::<VertexData>() as u64,
///         step_mode: wgpu::VertexStepMode::Vertex,
///         attributes: &[
///             wgpu::VertexAttribute {
///                 format: wgpu::VertexFormat::Float32x3,
///                 offset: 0,
///                 shader_location: 0,
///             },
///             wgpu::VertexAttribute {
///                 format: wgpu::VertexFormat::Float32x2,
///                 offset: (0 + wgpu::VertexFormat::Float32x3.size()),
///                 shader_location: 1,
///             },
///             wgpu::VertexAttribute {
///                 format: wgpu::VertexFormat::Float32x4,
///                 offset: ((0 + wgpu::VertexFormat::Float32x3.size()) + wgpu::VertexFormat::Float32x2.size()),
///                 shader_location: 2,
///             },
///         ],
///     };
/// }
/// ```
#[macro_export]
macro_rules! make_vertex_buffer_type {
	($step_mode:ident, struct $struct_name:ident { $( $field_name:ident : $field_type:ty as location $field_loc:tt : $field_data:ident , )+ }) => {
		#[derive(Copy, Clone, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
		#[repr(C)]
		pub struct $struct_name {
			$(
				pub $field_name: $field_type,
			)+
		}

		impl $crate::BufferItemRawData for $struct_name {
			const WGPU_LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
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
#[must_use]
#[inline]
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
