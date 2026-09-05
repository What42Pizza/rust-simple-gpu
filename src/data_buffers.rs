use crate::GpuInstance;
use std::marker::PhantomData;



/// Holds the data for the vertices of a mesh, or the instances of mesh
pub struct VertexBuffer<VertexRawData: BufferItemRawData> {
	/// A handle to the gpu buffer
	pub wgpu_buffer: wgpu::Buffer,
	/// The number of vertices this can hold. The actual size of the wgpu buffer is `self.count * size_of::<VertexRawData>()`
	pub count: u32,
	/// Holds the name of the buffer, only used when resizing (aka recreating) the wgpu buffer
	pub name: String,
	#[doc(hidden)]
	pub _phantom: PhantomData<VertexRawData>,
}

/// Creates a new vertex buffer (which can also be used for instance datas). Note: the byte size of the resulting wgpu buffer is `count * size_of::<VertexRawData>()`
pub fn create_vertex_buffer<VertexRawData: BufferItemRawData>(
	name: impl Into<String>,
	item_count: u32,
	gpu_instance: &GpuInstance,
) -> VertexBuffer<VertexRawData> {
	let name = name.into();
	let buffer = gpu_instance
		.wgpu_device
		.create_buffer(&wgpu::BufferDescriptor {
			label: Some(&name),
			size: u64::from(item_count) * std::mem::size_of::<VertexRawData>() as u64,
			usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
			mapped_at_creation: false,
		});
	VertexBuffer {
		wgpu_buffer: buffer,
		count: item_count,
		name,
		_phantom: PhantomData,
	}
}

/// Reallocates the wgpu buffer, but does not copy the previous data
pub fn resize_vertex_buffer<VertexRawData: BufferItemRawData>(
	vertex_buffer: &mut VertexBuffer<VertexRawData>,
	new_vertex_count: u32,
	gpu_instance: &GpuInstance,
) {
	vertex_buffer.wgpu_buffer = gpu_instance
		.wgpu_device
		.create_buffer(&wgpu::BufferDescriptor {
			label: Some(&vertex_buffer.name),
			size: u64::from(new_vertex_count) * std::mem::size_of::<VertexRawData>() as u64,
			usage: vertex_buffer.wgpu_buffer.usage(),
			mapped_at_creation: false,
		});
	vertex_buffer.count = new_vertex_count;
}

/// Replaces the data in a vertex buffer
pub fn update_vertex_buffer<VertexRawData: BufferItemRawData>(
	vertex_buffer: &mut VertexBuffer<VertexRawData>,
	new_data: &[VertexRawData],
	gpu_instance: &GpuInstance,
) {
	gpu_instance.wgpu_queue.write_buffer(
		&vertex_buffer.wgpu_buffer,
		0,
		bytemuck::cast_slice(new_data),
	);
}



/// Holds the list of indices that are used to connect vertices into triangles when rendering. Note: it is always assumed that indices are u16 values
pub struct IndexBuffer {
	/// A handle to the gpu buffer
	pub wgpu_buffer: wgpu::Buffer,
	/// The number of indices this can hold. The actual size of the wgpu buffer is `self.count * 2`
	pub count: u32,
	/// Holds the name of the buffer, only used when resizing (aka recreating) the wgpu buffer
	pub name: String,
}

/// Creates a new index buffer
pub fn create_index_buffer(
	name: impl Into<String>,
	index_count: u32,
	gpu_instance: &GpuInstance,
) -> IndexBuffer {
	let name = name.into();
	let buffer = gpu_instance
		.wgpu_device
		.create_buffer(&wgpu::BufferDescriptor {
			label: Some(&name),
			size: u64::from(index_count) * 2,
			usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
			mapped_at_creation: false,
		});
	IndexBuffer {
		wgpu_buffer: buffer,
		count: index_count,
		name,
	}
}

/// Reallocates the wgpu buffer, but does not copy the previous data
pub fn resize_index_buffer(
	index_buffer: &mut IndexBuffer,
	new_index_count: u32,
	gpu_instance: &GpuInstance,
) {
	index_buffer.wgpu_buffer = gpu_instance
		.wgpu_device
		.create_buffer(&wgpu::BufferDescriptor {
			label: Some(&index_buffer.name),
			size: u64::from(new_index_count) * 2,
			usage: index_buffer.wgpu_buffer.usage(),
			mapped_at_creation: false,
		});
	index_buffer.count = new_index_count;
}

/// Replaces the data in a vertex buffer
pub fn update_index_buffer(
	index_buffer: &mut IndexBuffer,
	new_data: &[u16],
	gpu_instance: &GpuInstance,
) {
	gpu_instance.wgpu_queue.write_buffer(
		&index_buffer.wgpu_buffer,
		0,
		bytemuck::cast_slice(new_data),
	);
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
