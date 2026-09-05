#![warn(clippy::all, clippy::nursery, clippy::pedantic)]
#![warn(missing_docs)]
#![allow(clippy::option_if_let_else)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::large_enum_variant)]

//! # Simple Gpu
//!
//! This is a tiny abstraction over WGPU that is inspired by the simplicity and directness of multimedia libraries like raylib and SDL. The main goal of this crate is to provide a simple and convenient way of rendering custom data with custom shaders.
//! 
//! ## See the [example program](https://github.com/What42Pizza/rust-simple-gpu/blob/main/examples/basic.rs)



use anyhow::{Ok, Result};
use std::{borrow::Cow, ffi::OsStr, path::Path};



/// Utilities for managing a window's surface (aka output texture)
pub mod window_surface;
pub use window_surface::*;
/// Utilities for creating and updating textures
pub mod textures;
pub use textures::*;
/// Utilities for creating and updating the uniforms buffer
pub mod uniforms_buffer;
pub use uniforms_buffer::*;
/// Utilities for creating and updating vertex buffers, index buffers, and instance buffers
pub mod data_buffers;
pub use data_buffers::*;
/// Utilities for creating pipelines
pub mod pipelines;
pub use pipelines::*;
/// Utilities for loading shaders
pub mod shaders;
pub use shaders::*;



/// Holds everything needed for simple wgpu functionality
pub struct GpuInstance {
	/// This is for retrieving the adapter and window surface
	pub wgpu_instance: wgpu::Instance,
	/// This is for querying data about the gpu
	pub wgpu_adapter: wgpu::Adapter,
	/// This is the gpu's interface, allowing you to create buffers, register shaders, etc
	pub wgpu_device: wgpu::Device,
	/// This is where you send the commands for the gpu to execute
	pub wgpu_queue: wgpu::Queue,
	/// This is just a basic sampler with bilinear filtering and coordinate clamping enabled
	pub wgpu_filtering_sampler: wgpu::Sampler,
	/// This is just a basic sampler with filtering disabled and coordinate clamping enabled
	pub wgpu_non_filtering_sampler: wgpu::Sampler,
	/// This specifies the layout for the uniforms bind group. More:
	///
	/// - Binding 0: buffer (type: uniforms)
	pub wgpu_uniforms_bind_group_layout: wgpu::BindGroupLayout,
	/// This specifies the layout for each texture's bind group. More:
	///
	/// - Binding 0: texture view
	/// - Binding 1: texture sampler (filtering)
	/// - Binding 2: texture sampler (non-filtering)
	pub wgpu_texture_bind_group_layout: wgpu::BindGroupLayout,
	/// This specifies the layout for each depth texture's bind group. More:
	///
	/// - Binding 0: texture view
	/// - Binding 1: texture sampler (filtering)
	/// - Binding 2: texture sampler (non-filtering)
	pub wgpu_depth_texture_bind_group_layout: wgpu::BindGroupLayout,
	/// This is the default pipeline layout used to render everything. More:
	///
	/// - Its bindings are:
	/// - Bind group 0 binding 0: buffer (type: uniforms)
	/// - Bind group 1 binding 0: texture view
	/// - Bind group 1 binding 1: texture sampler (filtering)
	/// - Bind group 1 binding 2: texture sampler (non-filtering)
	pub wgpu_pipeline_layout: wgpu::PipelineLayout,
}



/// Creates a new [`GpuInstance`]
///
/// # Errors
///
/// This returns an error if [`wgpu::Instance::request_adapter()`] errors or if [`wgpu::Adapter::request_device()`] errors
#[inline]
pub fn init(min_limits: wgpu::Limits, memory_hint: wgpu::MemoryHints) -> Result<GpuInstance> {
	let wgpu_instance =
		wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());

	let wgpu_adapter =
		pollster::block_on(wgpu_instance.request_adapter(&wgpu::RequestAdapterOptions::default()))?;

	let (wgpu_device, wgpu_queue) =
		pollster::block_on(wgpu_adapter.request_device(&wgpu::DeviceDescriptor {
			label: Some("main wgpu device"),
			required_limits: min_limits,
			required_features: wgpu::Features::empty(),
			memory_hints: memory_hint,
			experimental_features: wgpu::ExperimentalFeatures::disabled(),
			trace: wgpu::Trace::Off,
		}))?;

	let filtering_sampler = make_sampler(
		wgpu::AddressMode::ClampToEdge,
		wgpu::FilterMode::Linear,
		&wgpu_device,
	);
	let non_filtering_sampler = make_sampler(
		wgpu::AddressMode::ClampToEdge,
		wgpu::FilterMode::Nearest,
		&wgpu_device,
	);

	let uniforms_bind_group_layout =
		wgpu_device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
			label: Some("uniforms_bind_group_layout"),
			entries: &[wgpu::BindGroupLayoutEntry {
				binding: 0,
				visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
				ty: wgpu::BindingType::Buffer {
					ty: wgpu::BufferBindingType::Uniform,
					has_dynamic_offset: false,
					min_binding_size: None,
				},
				count: None,
			}],
		});

	let texture_bind_group_layout =
		wgpu_device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
			label: Some("texture_bind_group_layout"),
			entries: &[
				wgpu::BindGroupLayoutEntry {
					// texture
					binding: 0,
					visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
					ty: wgpu::BindingType::Texture {
						multisampled: false,
						view_dimension: wgpu::TextureViewDimension::D2,
						sample_type: wgpu::TextureSampleType::Float { filterable: true },
					},
					count: None,
				},
				wgpu::BindGroupLayoutEntry {
					// filtering sampler
					binding: 1,
					visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
					ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
					count: None,
				},
				wgpu::BindGroupLayoutEntry {
					// non-filtering sampler
					binding: 2,
					visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
					ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
					count: None,
				},
			],
		});

	let depth_texture_bind_group_layout =
		wgpu_device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
			label: Some("texture_bind_group_layout"),
			entries: &[
				wgpu::BindGroupLayoutEntry {
					// texture
					binding: 0,
					visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
					ty: wgpu::BindingType::Texture {
						multisampled: false,
						view_dimension: wgpu::TextureViewDimension::D2,
						sample_type: wgpu::TextureSampleType::Depth,
					},
					count: None,
				},
				wgpu::BindGroupLayoutEntry {
					// sampler
					binding: 1,
					visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
					ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
					count: None,
				},
				wgpu::BindGroupLayoutEntry {
					// sampler
					binding: 2,
					visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
					ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
					count: None,
				},
			],
		});

	let pipeline_layout = wgpu_device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
		label: Some("main_pipeline_layout"),
		bind_group_layouts: &[
			Some(&uniforms_bind_group_layout),
			Some(&texture_bind_group_layout),
		],
		immediate_size: 0,
	});

	Ok(GpuInstance {
		wgpu_instance,
		wgpu_adapter,
		wgpu_device,
		wgpu_queue,
		wgpu_filtering_sampler: filtering_sampler,
		wgpu_non_filtering_sampler: non_filtering_sampler,
		wgpu_uniforms_bind_group_layout: uniforms_bind_group_layout,
		wgpu_texture_bind_group_layout: texture_bind_group_layout,
		wgpu_depth_texture_bind_group_layout: depth_texture_bind_group_layout,
		wgpu_pipeline_layout: pipeline_layout,
	})
}



/// Allows you to start rendering a frame by preparing instructions for the gpu
#[must_use]
#[inline]
pub fn start_command_encoder(name: &str, gpu_instance: &GpuInstance) -> wgpu::CommandEncoder {
	gpu_instance
		.wgpu_device
		.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some(name) })
}

/// Submits a command encoder to the gpu, which starts the actual rendering of the next frame
#[inline]
pub fn submit_gpu_commands(command_encoder: wgpu::CommandEncoder, gpu_instance: &GpuInstance) {
	gpu_instance
		.wgpu_queue
		.submit(std::iter::once(command_encoder.finish()));
}



/// Represents whether A: a texture was retrieved, B: no texture was given, or C: there was an error requiring reconfiguring
///
/// This is the same as [`SurfaceTextureResult`], but with a [`wgpu::CommandEncoder`] added to the [`Some`] variant
pub enum StartFrameResult {
	/// Surface texture was successfully acquired
	Some(
		wgpu::SurfaceTexture,
		wgpu::TextureView,
		wgpu::CommandEncoder,
	),
	/// Surface timed out or is occluded (no rendering is needed)
	None,
	/// Surface textured errored and requires [`reconfigure_window_surface()`] to be called
	Error,
}

/// Combines [`get_surface_texture()`] and [`start_command_encoder()`] into a single function
#[must_use]
#[inline]
pub fn start_frame(
	name: &str,
	surface: &WindowSurface,
	gpu_instance: &GpuInstance,
) -> StartFrameResult {
	match get_surface_texture(surface) {
		SurfaceTextureResult::Some(tex, view) => {
			StartFrameResult::Some(tex, view, start_command_encoder(name, gpu_instance))
		}
		SurfaceTextureResult::None => StartFrameResult::None,
		SurfaceTextureResult::Error => StartFrameResult::Error,
	}
}

/// Combines [`submit_gpu_commands()`] and [`present_frame()`] into a single function
#[inline]
pub fn finish_frame(
	command_encoder: wgpu::CommandEncoder,
	surface_tex: wgpu::SurfaceTexture,
	gpu_instance: &GpuInstance,
) {
	submit_gpu_commands(command_encoder, gpu_instance);
	present_frame(surface_tex, gpu_instance);
}



/// Returns the limits (maximum texture sizes, max bindings per group, etc) for the current gpu
#[must_use]
#[inline]
pub fn get_gpu_limits(gpu_instance: &GpuInstance) -> wgpu::Limits {
	gpu_instance.wgpu_adapter.limits()
}



pub(crate) fn file_name(path: &Path) -> Option<Cow<'_, str>> {
	path.file_name().map(OsStr::to_string_lossy)
}
