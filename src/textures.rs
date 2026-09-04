use crate::GpuInstance;

#[cfg(feature = "image")]
use crate::file_name;
#[cfg(feature = "image")]
use anyhow::{Context, Ok, Result, anyhow};
#[cfg(feature = "image")]
use std::path::Path;



/// Holds a wgpu texture and all its related data (including a bind group for the texture)
pub struct Texture {
	/// Holds the most basic data about the texture
	pub wgpu_texture: wgpu::Texture,
	/// Can be given to bind groups, which are then bound to the render pipeline
	pub wgpu_view: wgpu::TextureView,
	/// Can be given to bind groups, and specifies how the texture is sampled. The first is a filtering sampler, and the second is non-filtering
	pub wgpu_samplers: (wgpu::Sampler, wgpu::Sampler),
	/// This is a bind group with just one binding, which is a view to this texture. The layout for this is taken from `gpu_instance.wgpu_texture_bind_group_layout`
	pub wgpu_bind_group: wgpu::BindGroup,
	/// Specifies the format of the texture's texels (aka pixels)
	pub wgpu_format: wgpu::TextureFormat,
}

/// Creates a new texture with a given size and format. More:
///
/// - If `is_filtered` is true, the sampler and bind group will be set created with bilinear filtering enabled (and no filtering if false)'
/// - If `is_render_target` is true, `wgpu::TextureUsages::RENDER_ATTACHMENT` is given instead of `wgpu::TextureUsages::COPY_DST`
#[inline]
#[must_use]
pub fn create_texture(
	name: &str,
	size: (u32, u32),
	format: wgpu::TextureFormat,
	gpu_instance: &GpuInstance,
	is_render_target: bool,
) -> Texture {
	let texture = gpu_instance
		.wgpu_device
		.create_texture(&wgpu::TextureDescriptor {
			label: Some(name),
			size: wgpu::Extent3d {
				width: size.0,
				height: size.1,
				depth_or_array_layers: 1,
			},
			mip_level_count: 1,
			sample_count: 1,
			dimension: wgpu::TextureDimension::D2,
			format,
			usage: if is_render_target {
				wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT
			} else {
				wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST
			},
			view_formats: &[],
		});

	let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

	let filtering_sampler = make_sampler(
		wgpu::AddressMode::ClampToEdge,
		wgpu::FilterMode::Linear,
		gpu_instance,
	);
	let non_filtering_sampler = make_sampler(
		wgpu::AddressMode::ClampToEdge,
		wgpu::FilterMode::Nearest,
		gpu_instance,
	);

	let bind_group = gpu_instance
		.wgpu_device
		.create_bind_group(&wgpu::BindGroupDescriptor {
			label: Some(name),
			layout: &gpu_instance.wgpu_texture_bind_group_layout,
			entries: &[
				wgpu::BindGroupEntry {
					binding: 0,
					resource: wgpu::BindingResource::TextureView(&view),
				},
				wgpu::BindGroupEntry {
					binding: 1,
					resource: wgpu::BindingResource::Sampler(&filtering_sampler),
				},
				wgpu::BindGroupEntry {
					binding: 2,
					resource: wgpu::BindingResource::Sampler(&non_filtering_sampler),
				},
			],
		});

	Texture {
		wgpu_texture: texture,
		wgpu_view: view,
		wgpu_samplers: (filtering_sampler, non_filtering_sampler),
		wgpu_bind_group: bind_group,
		wgpu_format: format,
	}
}

/// Creates a new depth texture with a given size
#[inline]
#[must_use]
pub fn create_depth_texture(name: &str, size: (u32, u32), gpu_instance: &GpuInstance) -> Texture {
	let format = wgpu::TextureFormat::Depth32Float;

	let texture = gpu_instance
		.wgpu_device
		.create_texture(&wgpu::TextureDescriptor {
			label: Some(name),
			size: wgpu::Extent3d {
				width: size.0,
				height: size.1,
				depth_or_array_layers: 1,
			},
			mip_level_count: 1,
			sample_count: 1,
			dimension: wgpu::TextureDimension::D2,
			format,
			usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
			view_formats: &[],
		});

	let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

	let filtering_sampler = make_sampler(
		wgpu::AddressMode::ClampToEdge,
		wgpu::FilterMode::Linear,
		gpu_instance,
	);
	let non_filtering_sampler = make_sampler(
		wgpu::AddressMode::ClampToEdge,
		wgpu::FilterMode::Nearest,
		gpu_instance,
	);
	
	let bind_group = gpu_instance
		.wgpu_device
		.create_bind_group(&wgpu::BindGroupDescriptor {
			label: Some(name),
			layout: &gpu_instance.wgpu_depth_texture_bind_group_layout,
			entries: &[
				wgpu::BindGroupEntry {
					binding: 0,
					resource: wgpu::BindingResource::TextureView(&view),
				},
				wgpu::BindGroupEntry {
					binding: 1,
					resource: wgpu::BindingResource::Sampler(&filtering_sampler),
				},
				wgpu::BindGroupEntry {
					binding: 2,
					resource: wgpu::BindingResource::Sampler(&non_filtering_sampler),
				},
			],
		});

	Texture {
		wgpu_texture: texture,
		wgpu_view: view,
		wgpu_samplers: (filtering_sampler, non_filtering_sampler),
		wgpu_bind_group: bind_group,
		wgpu_format: format,
	}
}



/// Overwrites a texture with new data
pub fn update_texture(texture: &Texture, new_data: &[u8], gpu_instance: &GpuInstance) {
	let (width, height) = (texture.wgpu_texture.width(), texture.wgpu_texture.height());
	gpu_instance.wgpu_queue.write_texture(
		wgpu::TexelCopyTextureInfo {
			texture: &texture.wgpu_texture,
			mip_level: 0,
			origin: wgpu::Origin3d::ZERO,
			aspect: wgpu::TextureAspect::All,
		},
		new_data,
		wgpu::TexelCopyBufferLayout {
			offset: 0,
			bytes_per_row: Some(width * u32::from(texture.wgpu_format.components())),
			rows_per_image: Some(height),
		},
		wgpu::Extent3d {
			width,
			height,
			depth_or_array_layers: 1,
		},
	);
}



/// Creates a texture from a given file path. More:
///
/// - The result always uses the format `wgpu::TextureFormat::Rgba8Unorm`.
/// - If `is_filtered` is true, the sampler and bind group will be set created with bilinear filtering enabled (and no filtering if false)'
///
/// # Errors
///
/// This only errors if it fails to read the file
#[cfg(feature = "image")]
pub fn load_texture_from_path(
	path: &Path,
	gpu_instance: &GpuInstance,
) -> Result<Texture> {
	let texture_image =
		image::open(path).with_context(|| format!("Failed to read file {}", path.display()))?;
	let size = (texture_image.width(), texture_image.height());
	let texture_data = texture_image.into_rgba8();
	let file_name = file_name(path)
		.ok_or_else(|| anyhow!("Failed to get file name from path {}", path.display()))?;
	let texture = create_texture(
		&file_name,
		size,
		wgpu::TextureFormat::Rgba8Unorm,
		gpu_instance,
		false,
	);
	update_texture(&texture, &texture_data, gpu_instance);
	Ok(texture)
}



/// Creates a basic `wgpu::Sampler` with a specified wrapping mode and filtering mode
#[must_use]
pub fn make_sampler(
	wrapping: wgpu::AddressMode,
	filter: wgpu::FilterMode,
	gpu_instance: &GpuInstance,
) -> wgpu::Sampler {
	gpu_instance
		.wgpu_device
		.create_sampler(&wgpu::SamplerDescriptor {
			address_mode_u: wrapping,
			address_mode_v: wrapping,
			address_mode_w: wrapping,
			mag_filter: filter,
			min_filter: filter,
			..Default::default()
		})
}
