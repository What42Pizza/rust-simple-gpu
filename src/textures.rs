use crate::GpuInstance;
use anyhow::{Context, Ok, Result, anyhow};

#[cfg(feature = "image")]
use crate::file_name;
#[cfg(feature = "image")]
use std::path::Path;
#[cfg(all(feature = "atlas", feature = "image"))]
use std::{collections::HashMap, path::PathBuf};



/// Holds a wgpu texture and all its related data (including a bind group for the texture)
pub struct Texture {
	/// Holds the most basic data about the texture
	pub wgpu_texture: wgpu::Texture,
	/// Can be given to bind groups, which are then bound to the render pipeline. More:
	///
	/// Note: this can be used within shaders to see all mipmap levels
	pub wgpu_view: wgpu::TextureView,
	/// Similar to `wgpu_view`, but contains one view for each mipmap level This is not needed for most operations and is only used for refilling mipmaps
	pub wgpu_mipmap_views: Vec<wgpu::TextureView>,
	/// Similar to `wgpu_bind_group`, but contains corresponding views from `wgpu_mipmap_view`. This is not needed for most operations and is only used for refilling mipmaps
	pub wgpu_mipmap_bind_groups: Vec<wgpu::BindGroup>,
	/// This is a bind group with just one binding, which is a view to this texture. The layout for this is taken from [`GpuInstance::wgpu_texture_bind_group_layout`]
	pub wgpu_bind_group: wgpu::BindGroup,
	/// Specifies the format of the texture's texels (aka pixels)
	pub wgpu_format: wgpu::TextureFormat,
	/// Specified the number of mip levels this texture contains (must be at least 1)
	pub mip_count: u32,
	/// The name of the texture, used for recreating the texture's bind group
	pub name: String,
}

/// Creates a new texture with a given size and format. More:
///
/// - If `samplers` is `None`, it will be created with the default samplers:
///   - A bilinear sampler that clamps coordinates
///   - A nearest sampler that clamps coordinates
/// - `mip_count` must be at least 1
#[must_use]
#[inline]
pub fn create_texture(
	name: impl Into<String>,
	size: (u32, u32),
	format: wgpu::TextureFormat,
	filter_mode: wgpu::FilterMode,
	mip_count: u32,
	gpu_instance: &GpuInstance,
) -> Texture {
	let name = name.into();

	let texture = gpu_instance
		.wgpu_device
		.create_texture(&wgpu::TextureDescriptor {
			label: Some(&name),
			size: wgpu::Extent3d {
				width: size.0,
				height: size.1,
				depth_or_array_layers: 1,
			},
			mip_level_count: mip_count,
			sample_count: 1,
			dimension: wgpu::TextureDimension::D2,
			format,
			usage: wgpu::TextureUsages::TEXTURE_BINDING
				| wgpu::TextureUsages::COPY_DST
				| wgpu::TextureUsages::RENDER_ATTACHMENT,
			view_formats: &[],
		});

	let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
	let mut mipmap_views = vec![];
	let mut mipmap_bind_groups = vec![];
	for i in 0..mip_count {
		let view = texture.create_view(&wgpu::TextureViewDescriptor {
			base_mip_level: i,
			mip_level_count: Some(1),
			..Default::default()
		});
		let bind_group = gpu_instance
			.wgpu_device
			.create_bind_group(&wgpu::BindGroupDescriptor {
				label: Some("mipmap_bind_group"),
				layout: &gpu_instance.wgpu_mipmap_bind_group_layout,
				entries: &[
					wgpu::BindGroupEntry {
						binding: 0,
						resource: wgpu::BindingResource::TextureView(&view),
					},
					wgpu::BindGroupEntry {
						binding: 1,
						resource: wgpu::BindingResource::Sampler(
							&gpu_instance.wgpu_filtering_sampler,
						),
					},
				],
			});
		mipmap_views.push(view);
		mipmap_bind_groups.push(bind_group);
	}

	let bind_group = gpu_instance
		.wgpu_device
		.create_bind_group(&wgpu::BindGroupDescriptor {
			label: Some(&name),
			layout: &gpu_instance.wgpu_texture_bind_group_layout,
			entries: &[
				wgpu::BindGroupEntry {
					binding: 0,
					resource: wgpu::BindingResource::TextureView(&view),
				},
				wgpu::BindGroupEntry {
					binding: 1,
					resource: wgpu::BindingResource::Sampler(
						if filter_mode == wgpu::FilterMode::Linear {
							&gpu_instance.wgpu_filtering_sampler
						} else {
							&gpu_instance.wgpu_non_filtering_sampler
						},
					),
				},
			],
		});

	Texture {
		wgpu_texture: texture,
		wgpu_view: view,
		wgpu_mipmap_views: mipmap_views,
		wgpu_mipmap_bind_groups: mipmap_bind_groups,
		wgpu_bind_group: bind_group,
		wgpu_format: format,
		mip_count,
		name,
	}
}



/// Same as [`Texture`], but specifically for depth textures, which need to be used and configured differently
pub struct DepthTexture {
	/// Holds the most basic data about the texture
	pub wgpu_texture: wgpu::Texture,
	/// Can be given to bind groups, which are then bound to the render pipeline.
	pub wgpu_view: wgpu::TextureView,
	/// This is a bind group with just one binding, which is a view to this texture. The layout for this is taken from [`GpuInstance::wgpu_texture_bind_group_layout`]
	pub wgpu_bind_group: wgpu::BindGroup,
}

/// Creates a new depth texture with a given size. More:
///
/// - This always uses the default samplers
/// - The mip level is always 1
#[must_use]
#[inline]
pub fn create_depth_texture(
	name: impl Into<String>,
	size: (u32, u32),
	filter_mode: wgpu::FilterMode,
	gpu_instance: &GpuInstance,
) -> DepthTexture {
	let name = name.into();
	let format = wgpu::TextureFormat::Depth24Plus;

	let texture = gpu_instance
		.wgpu_device
		.create_texture(&wgpu::TextureDescriptor {
			label: Some(&name),
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

	let bind_group = gpu_instance
		.wgpu_device
		.create_bind_group(&wgpu::BindGroupDescriptor {
			label: Some(&name),
			layout: &gpu_instance.wgpu_depth_texture_bind_group_layout,
			entries: &[
				wgpu::BindGroupEntry {
					binding: 0,
					resource: wgpu::BindingResource::TextureView(&view),
				},
				wgpu::BindGroupEntry {
					binding: 1,
					resource: wgpu::BindingResource::Sampler(
						if filter_mode == wgpu::FilterMode::Linear {
							&gpu_instance.wgpu_filtering_sampler
						} else {
							&gpu_instance.wgpu_non_filtering_sampler
						},
					),
				},
			],
		});

	DepthTexture {
		wgpu_texture: texture,
		wgpu_view: view,
		wgpu_bind_group: bind_group,
	}
}



/// Overwrites a texture with new data
///
/// # Panics
///
/// This will panic if the given format does not have a known block copy size (aka pixel byte size), see [`wgpu::TextureFormat::block_copy_size()`] for more.
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
			bytes_per_row: Some(
				width
					* texture
						.wgpu_format
						.block_copy_size(None)
						.unwrap_or_else(|| {
							panic!(
								"Failed to get the byte size of the given texture format: {:?}",
								texture.wgpu_format
							)
						}),
			),
			rows_per_image: Some(height),
		},
		wgpu::Extent3d {
			width,
			height,
			depth_or_array_layers: 1,
		},
	);
}

/// Sets the filter mode for a texture
pub fn set_filter_mode(
	texture: &mut Texture,
	filter_mode: wgpu::FilterMode,
	gpu_instance: &GpuInstance,
) {
	let new_bind_group = gpu_instance
		.wgpu_device
		.create_bind_group(&wgpu::BindGroupDescriptor {
			label: Some(&texture.name),
			layout: &gpu_instance.wgpu_texture_bind_group_layout,
			entries: &[
				wgpu::BindGroupEntry {
					binding: 0,
					resource: wgpu::BindingResource::TextureView(&texture.wgpu_view),
				},
				wgpu::BindGroupEntry {
					binding: 1,
					resource: wgpu::BindingResource::Sampler(
						if filter_mode == wgpu::FilterMode::Linear {
							&gpu_instance.wgpu_filtering_sampler
						} else {
							&gpu_instance.wgpu_non_filtering_sampler
						},
					),
				},
			],
		});
	texture.wgpu_bind_group = new_bind_group;
}



/// Creates a texture from a given file path. More:
///
/// - If `samplers` is `None`, it will be created with the default samplers:
///   - A bilinear sampler that clamps coordinates
///   - A nearest sampler that clamps coordinates
/// - `mip_count` must be at least 1
/// - The result always uses the format [`wgpu::TextureFormat::Rgba8Unorm`]
/// - This is only available when the "image" feature is enabled
/// - By default, only png, jpeg, webp, bmp, and tga formats are enabled. If needed, you can enable more image formats by adding this to your Cargo.toml: `image = { version = "...", features = [ .. ] }` (note: it needs to be the same version that this crate uses for the features to combine)
///
/// # Errors
///
/// This errors if [`image::open()`] errors or if it cannot get the file name from the path.
#[cfg(feature = "image")]
pub fn load_texture_from_path(
	path: &Path,
	filter_mode: wgpu::FilterMode,
	mip_count: u32,
	gpu_instance: &GpuInstance,
) -> Result<Texture> {
	let texture_image =
		image::open(path).with_context(|| format!("Failed to read file {}", path.display()))?;
	let size = (texture_image.width(), texture_image.height());
	let texture_data = texture_image.into_rgba8();
	let file_name = file_name(path)
		.ok_or_else(|| anyhow!("Failed to get file name from path {}", path.display()))?;
	let texture = create_texture(
		file_name,
		size,
		wgpu::TextureFormat::Rgba8Unorm,
		filter_mode,
		mip_count,
		gpu_instance,
	);
	update_texture(&texture, &texture_data, gpu_instance);
	Ok(texture)
}



/// Represents a location within a generated texture atlas
#[cfg(feature = "atlas")]
#[derive(Copy, Clone, Debug)]
pub struct AtlasLocation {
	/// The position within the atlas
	pub pos: (u32, u32),
	/// The size of the texture within the atlas
	pub size: (u32, u32),
	/// The id of the allocation within the [`guillotiere::AtlasAllocator`]. Note: if this equals `AllocId::deserialize(u32::MAX)`, the texture was not placed because its width or height is 0.
	pub alloc_id: guillotiere::AllocId,
}

/// A simple wrapper around [`guillotiere::AtlasAllocator`] that ensures all positions are re-expanded to match their actual texture position
///
/// When a texture atlas is created, the allocator for it has its coordinates scaled down by `2 ^ max_mip`. This is to ensure that all positions are automatically aligned to mip boundaries, but it can also create some confusion. To make it more obvious that this is the case (and also for some extra convenience), the allocator is wrapped in this struct along with the max mip level it was created with
#[cfg(feature = "atlas")]
pub struct AtlasAllocator {
	/// The actual allocator
	pub allocator: guillotiere::AtlasAllocator,
	/// The mip level this allocator uses. The allocator must have a size that is a multiple of `2 ^ map_mip`, and the outputs must be scaled up by `2 ^ max_mip` (which is automatically done with the methods on this struct)
	pub max_mip: u32,
}

/// Builds an atlas out of many textures
///
/// The input is a list of textures, with each item having a specified width, height, and pixel data. You also have to specify the pixel format and mip count
///
/// Important note: the returned allocator is scaled down by `2 ^ (mip_count - 1)` so that the allocated positions are automatically aligned to a mip boundary. This means that if you want to use the allocator yourself, you need to shift the output locations right by `mip_count - 1`
///
/// - If `samplers` is `None`, it will be created with the default samplers:
///   - A bilinear sampler that clamps coordinates
///   - A nearest sampler that clamps coordinates
/// - `mip_count` must be at least 1
/// - The result always uses the format [`wgpu::TextureFormat::Rgba8Unorm`]
/// - This is only available when the "atlas" feature is enabled
///
/// # Panics
///
/// This will panic if the given format does not have a known block copy size (aka pixel byte size), see [`wgpu::TextureFormat::block_copy_size()`] for more.
#[cfg(feature = "atlas")]
#[must_use]
pub fn create_texture_atlas<Data: AsRef<[u8]>>(
	name: &str,
	textures: &[(u32, u32, Data)],
	format: wgpu::TextureFormat,
	filter_mode: wgpu::FilterMode,
	mip_count: u32,
	min_size: Option<(u32, u32)>,
	gpu_instance: &GpuInstance,
) -> (Texture, Vec<AtlasLocation>, AtlasAllocator) {
	let mut textures = textures
		.iter()
		.enumerate()
		.map(|(i, (width, height, data))| (*width, *height, data, i))
		.collect::<Vec<_>>();
	textures.sort_by_key(|(width, height, _data, _i)| u32::MAX - width * height);
	let max_mip = mip_count - 1;

	let bytes_per_pixel = format.block_copy_size(None).unwrap_or_else(|| {
		panic!("Failed to get the byte size of the given texture format: {format:?}")
	});

	let mut total_allocator_pixels = 0;
	for (tex_width, tex_height, _tex_data, _i) in &textures {
		if *tex_width == 0 || *tex_height == 0 {
			continue;
		}
		let (tex_width, tex_height) = (
			fit_mip(*tex_width, max_mip) >> max_mip,
			fit_mip(*tex_height, max_mip) >> max_mip,
		);
		total_allocator_pixels += u64::from(tex_width) * u64::from(tex_height);
	}
	#[allow(clippy::cast_possible_truncation)]
	let atlas_size = total_allocator_pixels.isqrt() as u32 + 2;
	let (mut atlas_width, mut atlas_height) = if let Some((min_width, min_height)) = min_size {
		(atlas_size.max(min_width), atlas_size.max(min_height))
	} else {
		(atlas_size, atlas_size)
	};

	let mut locations = vec![
		AtlasLocation {
			pos: (0, 0),
			size: (0, 0),
			alloc_id: guillotiere::AllocId::deserialize(u32::MAX),
		};
		textures.len()
	];
	#[allow(clippy::cast_sign_loss, clippy::cast_possible_wrap)]
	'try_alloc: loop {
		//println!("trying size {atlas_size}");

		let mut allocator = guillotiere::AtlasAllocator::new(guillotiere::size2(
			atlas_width as i32,
			atlas_height as i32,
		));

		for (width, height, _data, i) in &textures {
			let (width, height) = (
				fit_mip(*width, max_mip) >> max_mip,
				fit_mip(*height, max_mip) >> max_mip,
			);
			let Some(loc) = allocator.allocate(guillotiere::size2(width as i32, height as i32))
			else {
				//println!("did not fit");
				atlas_width = atlas_width * 32 / 31 + 1;
				atlas_height = atlas_height * 32 / 31 + 1;
				continue 'try_alloc;
			};
			locations[*i] = AtlasLocation {
				pos: (
					(loc.rectangle.x_range().start as u32) << max_mip,
					(loc.rectangle.y_range().start as u32) << max_mip,
				),
				size: (width << max_mip, height << max_mip),
				alloc_id: loc.id,
			};
		}

		let (atlas_width, atlas_height) = (atlas_width << max_mip, atlas_height << max_mip);

		let texture = create_texture(
			name,
			(atlas_width, atlas_height),
			format,
			filter_mode,
			mip_count,
			gpu_instance,
		);

		// note: fills with fully transparent black
		let mut atlas_tex_data =
			vec![0; atlas_width as usize * atlas_height as usize * bytes_per_pixel as usize];

		for (width, height, data, i) in &textures {
			let data = data.as_ref();
			place_texture_in_atlas(
				data,
				locations[*i].pos,
				(*width, *height),
				&mut atlas_tex_data,
				(atlas_width, atlas_height),
				mip_count,
				bytes_per_pixel,
			);
		}

		update_texture(&texture, &atlas_tex_data, gpu_instance);
		//println!("efficiency: {}", total_allocator_pixels as f32 / ((atlas_width >> max_mip) * (atlas_height >> max_mip)) as f32);

		let allocator = AtlasAllocator { allocator, max_mip };

		return (texture, locations, allocator);
	}
}

/// Places a texture's pixel data inside the pixel data of a texture atlas, with padding for mip mapping
#[cfg(feature = "atlas")]
pub fn place_texture_in_atlas(
	tex_data: &[u8],
	pos: (u32, u32),
	size: (u32, u32),
	atlas_data: &mut [u8],
	atlas_size: (u32, u32),
	mip_count: u32,
	bytes_per_pixel: u32,
) {
	let max_mip = mip_count - 1;
	debug_assert_eq!(
		tex_data.len(),
		(size.0 * size.1 * bytes_per_pixel) as usize,
		"Texture data is not the correct size"
	);
	debug_assert_eq!(
		atlas_data.len(),
		(atlas_size.0 * atlas_size.1 * bytes_per_pixel) as usize,
		"Atlas texture data is not the correct size"
	);
	debug_assert_eq!(
		pos.0,
		fit_mip(pos.0, max_mip),
		"Texture is not positioned on a mip boundary (this is likely due to incorrect usage of the guillotiere allocator)"
	);
	debug_assert_eq!(
		pos.1,
		fit_mip(pos.1, max_mip),
		"Texture is not positioned on a mip boundary (this is likely due to incorrect usage of the guillotiere allocator)"
	);
	debug_assert_eq!(
		atlas_size.0,
		atlas_size.0 & (u32::MAX << max_mip),
		"Atlas size does not line up with the max mip level"
	);
	debug_assert_eq!(
		atlas_size.1,
		atlas_size.1 & (u32::MAX << max_mip),
		"Atlas size does not line up with the max mip level"
	);

	let (x, y) = pos;
	let (width, height) = size;
	let (atlas_width, _atlas_height) = atlas_size;
	let mip_fitted_width = fit_mip(width, max_mip);
	let mip_fitted_height = fit_mip(height, max_mip);

	for row_y in 0..height {
		// get source row
		let src = &tex_data[(row_y * width * bytes_per_pixel) as usize..]
			[..(width * bytes_per_pixel) as usize];
		// get destination row
		let mut dst = &mut atlas_data
			[(x * bytes_per_pixel + (y + row_y) * atlas_width * bytes_per_pixel) as usize..]
			[..(mip_fitted_width * bytes_per_pixel) as usize];
		// copy
		dst[..(width * bytes_per_pixel) as usize].copy_from_slice(src);
		// add mipmap padding (+x)
		dst = &mut dst[(width * bytes_per_pixel) as usize..];
		let src = &src[src.len() - bytes_per_pixel as usize..];
		loop {
			if dst.is_empty() {
				break;
			}
			dst[..bytes_per_pixel as usize].copy_from_slice(src);
			//dst[0] /= 2; // testing
			//dst[1] /= 2; // testing
			//dst[2] /= 2; // testing
			dst = &mut dst[bytes_per_pixel as usize..];
		}
	}

	// add mipmap padding (+y)
	if height != mip_fitted_height {
		// split it so that we can copy part of the data into another part of the data
		let (src, dst) =
			atlas_data.split_at_mut(((y + height) * atlas_width * bytes_per_pixel) as usize);
		// select the bottom row of the texture
		let src = &src[src.len() - ((atlas_width - x) * bytes_per_pixel) as usize..]
			[..(mip_fitted_width * bytes_per_pixel) as usize];
		// select the start of the row to copy to
		let mut dst = &mut dst[(x * bytes_per_pixel) as usize..];
		// copy each row
		for row_y in height..mip_fitted_height {
			// select the area within the row to copy to
			dst[..(mip_fitted_width * bytes_per_pixel) as usize].copy_from_slice(src);
			//for i in 0 .. (width * bytes_per_pixel) as usize { // testing
			//	if i % 4 == 3 {continue;}
			//	dst[i] /= 2;
			//}
			// move selected area forward
			if row_y != mip_fitted_height - 1 {
				dst = &mut dst[(atlas_width * bytes_per_pixel) as usize..];
			}
		}
	}
}

/// Rounds a value up to the nearest `1 << max_mip`. For example, `fit_mip(20, 3)` will return 32 because `1 << 3` is 16 and 32 is the lowest multiple of 16 that can fit 20
#[cfg(feature = "atlas")]
#[must_use]
pub const fn fit_mip(v: u32, max_mip: u32) -> u32 {
	if v == 0 {
		return 0;
	}
	(((v - 1) >> max_mip) + 1) << max_mip
}



/// Creates a texture atlas from a given folder
///
/// - If `samplers` is `None`, it will be created with the default samplers:
///   - A bilinear sampler that clamps coordinates
///   - A nearest sampler that clamps coordinates
/// - `mip_count` must be at least 1
/// - The result always uses the format [`wgpu::TextureFormat::Rgba8Unorm`]
/// - This is only available when the both the "image" and "atlas" features are enabled
/// - By default, only png, jpeg, webp, bmp, and tga formats are enabled. If needed, you can enable more image formats by adding this to your Cargo.toml: `image = { version = "...", features = [ .. ] }` (note: it needs to be the same version that this crate uses for the features to combine)
///
/// # Errors
///
/// This only returns an error if [`std::fs::read_dir()`] errors.
#[cfg(all(feature = "atlas", feature = "image"))]
pub fn create_texture_atlas_from_path(
	name: &str,
	path: &Path,
	recursive: bool,
	filter_mode: wgpu::FilterMode,
	mip_count: u32,
	min_size: Option<(u32, u32)>,
	gpu_instance: &GpuInstance,
) -> Result<(Texture, HashMap<PathBuf, AtlasLocation>, AtlasAllocator)> {
	let mut textures = vec![];
	let mut texture_paths = vec![];
	let mut paths = std::fs::read_dir(path)
		.with_context(|| format!("Failed to read contents of folder at {}", path.display()))?
		.collect::<std::result::Result<Vec<_>, std::io::Error>>()?;

	loop {
		let Some(curr_path) = paths.pop() else {
			break;
		};
		let curr_path = curr_path.path();

		if recursive {
			for child in std::fs::read_dir(&curr_path).with_context(|| {
				format!(
					"Failed to read contents of folder at {}",
					curr_path.display()
				)
			})? {
				paths.push(child?);
			}
		}

		let image = image::open(&curr_path)?;
		textures.push((image.width(), image.height(), image.to_rgba8().into_raw()));
		texture_paths.push(curr_path);
	}

	let (atlas_texture, locations, allocator) = create_texture_atlas(
		name,
		&textures,
		wgpu::TextureFormat::Rgba8Unorm,
		filter_mode,
		mip_count,
		min_size,
		gpu_instance,
	);

	let mut mapped_locations = HashMap::new();
	for (i, path) in texture_paths.into_iter().enumerate() {
		mapped_locations.insert(path, locations[i]);
	}

	Ok((atlas_texture, mapped_locations, allocator))
}



/// Creates a basic [`wgpu::Sampler`] with a specified wrapping mode and filtering mode
#[must_use]
pub fn make_sampler(
	wrapping: wgpu::AddressMode,
	filter: wgpu::FilterMode,
	wgpu_device: &wgpu::Device,
) -> wgpu::Sampler {
	wgpu_device.create_sampler(&wgpu::SamplerDescriptor {
		address_mode_u: wrapping,
		address_mode_v: wrapping,
		address_mode_w: wrapping,
		mag_filter: filter,
		min_filter: filter,
		mipmap_filter: wgpu::MipmapFilterMode::Linear,
		..Default::default()
	})
}



/// Regenerates the mipmap data for a texture
pub fn refill_mipmaps(
	texture: &Texture,
	command_encoder: &mut wgpu::CommandEncoder,
	gpu_instance: &mut GpuInstance,
) {
	if texture.mip_count < 2 {
		return;
	}
	let pipeline = get_mipmap_pipeline(texture.wgpu_format, gpu_instance);
	let dont_load = unsafe { wgpu::LoadOpDontCare::enabled() }; // safety: the entire output will be overwritten without blending

	for i in 1..texture.mip_count {
		let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
			label: Some("render_mipmap"),
			color_attachments: &[Some(wgpu::RenderPassColorAttachment {
				view: &texture.wgpu_mipmap_views[i as usize],
				depth_slice: None,
				resolve_target: None,
				ops: wgpu::Operations {
					load: wgpu::LoadOp::DontCare(dont_load),
					store: wgpu::StoreOp::Store,
				},
			})],
			depth_stencil_attachment: None,
			timestamp_writes: None,
			occlusion_query_set: None,
			multiview_mask: None,
		});
		render_pass.set_pipeline(pipeline);
		let higher_mipmap_bind_group = &texture.wgpu_mipmap_bind_groups[i as usize - 1];
		render_pass.set_bind_group(0, higher_mipmap_bind_group, &[]);
		render_pass.draw(0..4, 0..1);
	}
}

/// Returns the pipeline needed for rendering a texture's mipmaps
pub fn get_mipmap_pipeline(
	texture_format: wgpu::TextureFormat,
	gpu_instance: &mut GpuInstance,
) -> &wgpu::RenderPipeline {
	gpu_instance
		.wgpu_mipmap_pipelines
		.entry(texture_format)
		.or_insert_with(|| {
			let desc = wgpu::RenderPipelineDescriptor {
				label: Some("mipmap_pipeline"),
				layout: Some(&gpu_instance.wgpu_mipmap_pipeline_layout),
				vertex: wgpu::VertexState {
					module: &gpu_instance.wgpu_full_quad_vertex_shader,
					entry_point: None,
					buffers: &[],
					compilation_options: wgpu::PipelineCompilationOptions::default(),
				},
				fragment: Some(wgpu::FragmentState {
					module: &gpu_instance.wgpu_mipmap_fragment_shader,
					entry_point: None,
					targets: &[Some(wgpu::ColorTargetState {
						format: texture_format,
						blend: Some(wgpu::BlendState::REPLACE),
						write_mask: wgpu::ColorWrites::ALL,
					})],
					compilation_options: wgpu::PipelineCompilationOptions::default(),
				}),
				primitive: wgpu::PrimitiveState {
					topology: wgpu::PrimitiveTopology::TriangleStrip,
					..Default::default()
				},
				depth_stencil: None,
				multisample: wgpu::MultisampleState::default(),
				multiview_mask: None,
				cache: None,
			};
			gpu_instance.wgpu_device.create_render_pipeline(&desc)
		})
}
