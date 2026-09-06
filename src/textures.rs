use crate::GpuInstance;

#[cfg(feature = "image")]
use crate::file_name;
#[cfg(feature = "image")]
use anyhow::{Context, Ok, Result, anyhow};
use std::fs;
#[cfg(feature = "image")]
use std::path::Path;



/// Holds a wgpu texture and all its related data (including a bind group for the texture)
pub struct Texture {
	/// Holds the most basic data about the texture
	pub wgpu_texture: wgpu::Texture,
	/// Can be given to bind groups, which are then bound to the render pipeline
	pub wgpu_view: wgpu::TextureView,
	/// This is a bind group with just one binding, which is a view to this texture. The layout for this is taken from [`GpuInstance::wgpu_texture_bind_group_layout`]
	pub wgpu_bind_group: wgpu::BindGroup,
	/// Specifies the format of the texture's texels (aka pixels)
	pub wgpu_format: wgpu::TextureFormat,
}

/// Creates a new texture with a given size and format. More:
///
/// - If `is_render_target` is true, [`wgpu::TextureUsages::RENDER_ATTACHMENT`] is given instead of [`wgpu::TextureUsages::COPY_DST`], and [`wgpu::TextureUsages::TEXTURE_BINDING`] is always given
#[must_use]
#[inline]
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
					resource: wgpu::BindingResource::Sampler(&gpu_instance.wgpu_filtering_sampler),
				},
				wgpu::BindGroupEntry {
					binding: 2,
					resource: wgpu::BindingResource::Sampler(
						&gpu_instance.wgpu_non_filtering_sampler,
					),
				},
			],
		});

	Texture {
		wgpu_texture: texture,
		wgpu_view: view,
		wgpu_bind_group: bind_group,
		wgpu_format: format,
	}
}

/// Creates a new depth texture with a given size
#[must_use]
#[inline]
pub fn create_depth_texture(name: &str, size: (u32, u32), gpu_instance: &GpuInstance) -> Texture {
	let format = wgpu::TextureFormat::Depth24Plus;

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
					resource: wgpu::BindingResource::Sampler(&gpu_instance.wgpu_filtering_sampler),
				},
				wgpu::BindGroupEntry {
					binding: 2,
					resource: wgpu::BindingResource::Sampler(
						&gpu_instance.wgpu_non_filtering_sampler,
					),
				},
			],
		});

	Texture {
		wgpu_texture: texture,
		wgpu_view: view,
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
			bytes_per_row: Some(width * u32::from(texture.wgpu_format.block_copy_size(None).unwrap_or_else(|| panic!("Failed to get the byte size of the given texture format: {:?}", texture.wgpu_format)))),
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
/// - The result always uses the format [`wgpu::TextureFormat::Rgba8Unorm`].
/// - This is only available when the "image" feature is enabled
/// - By default, only png, jpeg, webp, bmp, and tga formats are enabled. If needed, you can enable more image formats by adding this to your Cargo.toml: `image = { version = "...", features = [ .. ] }` (note: it needs to be the same version that this crate uses for the features to combine)
///
/// # Errors
///
/// This errors if [`image::open()`] errors or if it cannot get the file name from the path
#[cfg(feature = "image")]
pub fn load_texture_from_path(path: &Path, gpu_instance: &GpuInstance) -> Result<Texture> {
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

/// Represents a location within a generated texture atlas
#[cfg(feature = "atlas")]
pub struct AtlasLocation {
	/// The position within the atlas
	pub pos: (u32, u32),
	/// The size of the texture within the atlas
	pub size: (u32, u32),
}

/// Builds an atlas out of many textures
/// 
/// The input is a list of textures, with each item having a specified width, height, and pixel data. Also, the mip value works in the same way as in texture samplers, `map_mip` being 0 means no mip levels, 1 means there's one extra mip level that's half the texture width & height, etc
#[cfg(feature = "atlas")]
#[must_use]
pub fn create_texture_atlas<Data: AsRef<[u8]>>(name: &str, format: wgpu::TextureFormat, max_mip: u32, textures: &[(u32, u32, Data)], gpu_instance: &GpuInstance) -> (Texture, Vec<AtlasLocation>) {
    use std::collections::BTreeMap;
	
	let bytes_per_pixel = format.block_copy_size(None).unwrap_or_else(|| panic!("Failed to get the byte size of the given texture format: {format:?}"));
	let fit_mip = |v: u32| -> u32 {
		(((v - 1) >> max_mip) + 1) << max_mip
	};

	let mut rects_to_place: rectangle_pack::GroupedRectsToPlace<u32, u8> = rectangle_pack::GroupedRectsToPlace::new();
	let mut total_pixels = 0;
	for (i, (tex_width, tex_height, _tex_data)) in textures.iter().enumerate() {
		if *tex_width == 0 || *tex_height == 0 { continue; }
		let (tex_width, tex_height) = (fit_mip(*tex_width) >> max_mip, fit_mip(*tex_height) >> max_mip);
		rects_to_place.push_rect(i as u32, None, rectangle_pack::RectToInsert::new(tex_width, tex_height, 1));
		total_pixels += tex_width as u64 * tex_height as u64;
	}
	
	let mut atlas_size = (total_pixels.isqrt() as u32) * 16 / 15;
	atlas_size = fit_mip(atlas_size);
	
	loop {
		
		let mut target_bins = BTreeMap::new();
		target_bins.insert(0u8, rectangle_pack::TargetBin::new(atlas_size, atlas_size, 1));
		
		let results = rectangle_pack::pack_rects(&rects_to_place, &mut target_bins, &rectangle_pack::volume_heuristic, &rectangle_pack::contains_smallest_box);
		let Result::Ok(results) = results else {
			atlas_size = fit_mip(atlas_size * 16 / 15 + (1 << max_mip));
			continue;
		};
		let results = results.packed_locations();
		
		let atlas_size  = atlas_size << max_mip;
		
		let texture = create_texture(name, (atlas_size, atlas_size), format, gpu_instance, false);
		
		let mut rects = Vec::with_capacity(textures.len());
		let mut atlas_data = vec![0; atlas_size as usize * atlas_size as usize * bytes_per_pixel as usize];
		
		for (i, (width, height, data)) in textures.iter().enumerate() {
			let data = data.as_ref();
			let Some((_bin, loc)) = results.get(&(i as u32)) else {
				rects.push(AtlasLocation {
					pos: (0, 0),
					size: (0, 0),
				});
				continue;
			};
			// note: fit_mip() is intentionally not done
			let (loc_x, loc_y) = (loc.x() << max_mip, loc.y() << max_mip);
			rects.push(AtlasLocation {
				pos: (loc_x, loc_y),
				size: (*width, *height),
			});
			let mip_fitted_width = fit_mip(*width);
			// copy texture data into atlas data
			for row_y in 0..*height {
				// get source row
				let src = &data[(row_y * width * bytes_per_pixel) as usize ..][.. (width * bytes_per_pixel) as usize];
				// get destination row
				let mut dst = &mut atlas_data[(loc_x * bytes_per_pixel + (loc_y + row_y) * atlas_size * bytes_per_pixel) as usize ..][.. (mip_fitted_width * bytes_per_pixel) as usize];
				// copy
				dst[.. (*width * bytes_per_pixel) as usize].copy_from_slice(src);
				// add mipmap padding (+x)
				dst = &mut dst[(*width * bytes_per_pixel) as usize ..];
				let src = &src[src.len() - bytes_per_pixel as usize ..];
				loop {
					if dst.is_empty() { break; }
					dst[.. bytes_per_pixel as usize].copy_from_slice(src);
					dst[0] /= 2;
					dst[1] /= 2;
					dst[2] /= 2;
					dst = &mut dst[bytes_per_pixel as usize ..];
				}
			}
			// add mipmap padding (+y)
			let mip_fitted_height = fit_mip(*height); // note: don't use fit_mip() here because we want the lower value
			if *height != mip_fitted_height {
				// split it so that we can copy part of the data into another part of the data
				let (src, dst) = atlas_data.split_at_mut(((loc_y + *height) * atlas_size * bytes_per_pixel) as usize);
				// select the bottom row of the texture
				let src = &src[src.len() - ((atlas_size - loc_x) * bytes_per_pixel) as usize .. ][.. (mip_fitted_width * bytes_per_pixel) as usize];
				// select the start of the row to copy to
				let mut dst = &mut dst[(loc_x * bytes_per_pixel) as usize ..];
				// copy each row
				for row_y in *height..mip_fitted_height {
					// select the area within the row to copy to
					dst[.. (mip_fitted_width * bytes_per_pixel) as usize].copy_from_slice(src);
					for i in 0 .. (*width * bytes_per_pixel) as usize {
						if i % 4 == 3 {continue;}
						dst[i] /= 2;
					}
					// move selected area forward
					if row_y != mip_fitted_height - 1 {
						dst = &mut dst[(atlas_size * bytes_per_pixel) as usize ..];
					}
				}
			}
		}
		
		update_texture(&texture, &atlas_data, gpu_instance);
		
		return (texture, rects);
	}
}

/// Creates a texture atlas from a given folder
#[cfg(all(feature = "atlas", feature = "image"))]
#[must_use]
pub fn create_texture_atlas_from_path(name: &str, path: &Path, recursive: bool, max_mip: u32, gpu_instance: &GpuInstance) -> Result<(Texture, Vec<AtlasLocation>)> {
	let mut textures = vec![];
	let mut paths = fs::read_dir(path).with_context(|| format!("Failed to read contents of folder at {}", path.display()))?.collect::<std::result::Result<Vec<_>, std::io::Error>>()?;
	
	loop {
		let Some(curr_path) = paths.pop() else { break; };
		let curr_path = curr_path.path();
		
		if recursive {
			for child in fs::read_dir(&curr_path).with_context(|| format!("Failed to read contents of folder at {}", curr_path.display()))? {
				paths.push(child?);
			}
		}
		
		let image = image::open(curr_path)?;
		textures.push((image.width(), image.height(), image.to_rgba8().into_raw()));
		
	}
	
	let output = create_texture_atlas(name, wgpu::TextureFormat::Rgba8Unorm, max_mip, &textures, gpu_instance);
	Ok(output)
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
		..Default::default()
	})
}
