use crate::GpuInstance;
use std::ops::{Deref, DerefMut};



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
	let items_len = items.len() as u32;
	let name = name.into();
	let buffer = gpu_instance
		.wgpu_device
		.create_buffer(&wgpu::BufferDescriptor {
			label: Some(&name),
			size: items_len as u64 * std::mem::size_of::<VertexRawData>() as u64,
			usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
			mapped_at_creation: false,
		});
	gpu_instance
		.wgpu_queue
		.write_buffer(&buffer, 0, bytemuck::cast_slice(&*items));
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
		bytemuck::cast_slice(&*vertex_buffer.cpu_buffer),
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
	IndexBuffer {
		wgpu_buffer: buffer,
		count: indices.len() as u32,
	}
}



/// Represents a type that can be put in a [`VertexBuffer`], and can be used for either vertex datas or instance datas
///
/// Example usage:
///
/// ```
/// #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
/// #[repr(C)]
/// struct VertexRawData {
///     pub pos: [f32; 3],
///     pub uv: [f32; 2],
///     pub color: [f32; 4],
/// }
///
/// impl simple_gpu::BufferItemRawData for VertexRawData {
///     // specifies the fields that VertexRawData has
///     const FIELDS: &[wgpu::VertexAttribute] = &wgpu::vertex_attr_array![
///         0 => Float32x3,
///         1 => Float32x2,
///         2 => Float32x4,
///     ];
///     // specifies that this contains vertex data, not instance data
///     const STEP_MODE: wgpu::VertexStepMode = wgpu::VertexStepMode::Vertex;
///     // there is another const field, but it is automatically generated from the other two
/// }
/// ```
pub trait BufferItemRawData: bytemuck::Pod {
	/// Lists the fields that are in `Self`, should be constructed using `wgpu::vertex_attr_array![]`
	const FIELDS: &[wgpu::VertexAttribute];
	/// Defines if this is a vertex type or an instance type
	const STEP_MODE: wgpu::VertexStepMode;
	/// Lists this as a [`wgpu::VertexBufferLayout`] (note: this is automatically generated!)
	const BUFFER_LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
		array_stride: std::mem::size_of::<Self>() as u64,
		step_mode: Self::STEP_MODE,
		attributes: Self::FIELDS,
	};
}
