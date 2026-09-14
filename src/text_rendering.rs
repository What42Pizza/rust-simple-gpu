use crate::{AtlasLocation, GpuInstance, get_gpu_limits, place_texture_in_atlas};
use anyhow::{Result, anyhow};
use fontdue::{Font, FontSettings, Metrics};
use guillotiere::{AtlasAllocator, size2};
use std::collections::HashMap;



/// Holds all the data needed for rendering text
pub struct TextRenderer {
	/// Holds the [`fontdue::Font`]
	pub font: Font,
	/// This is the size at which font is rendered for storage within the character atlas
	pub rasterize_size: u32,
	/// This is the atlas that all rendered characters are placed in
	pub atlas_data: Vec<u8>,
	/// This is the allocator used for placing characters into the atlas
	pub atlas_allocator: AtlasAllocator,
	/// Maps characters to both their location within the atlas and their metrics
	pub char_to_atlas_mappings: HashMap<char, (AtlasLocation, Metrics)>,
	/// The actual texture for the character atlas
	pub atlas_texture: wgpu::Texture,
	/// The view for the character atlas
	pub atlas_tex_view: wgpu::TextureView,
	/// The view for the character atlas
	pub atlas_bind_group: wgpu::BindGroup,
}



/// Creates a [`TextRenderer`] with a specific font, rasterize
///
/// # Errors
///
/// This returns an error if:
/// - The font fails to load
/// - The full ascii set of characters cannot be placed within the atlas
pub fn create_text_renderer(
	font: &[u8],
	rasterize_size: u32,
	max_atlas_size: Option<u32>,
	gpu_instance: &GpuInstance,
) -> Result<TextRenderer> {
	let font = Font::from_bytes(
		font,
		FontSettings {
			collection_index: 0,
			scale: rasterize_size as f32,
			load_substitutions: true,
		},
	)
	.map_err(|v| anyhow!(v))?;

	let atlas_size = get_gpu_limits(gpu_instance).max_texture_dimension_2d;
	let max_atlas_size = max_atlas_size.unwrap_or(rasterize_size * 6);
	let atlas_size = atlas_size.min(max_atlas_size);

	let mut character_datas_to_place = vec![];
	for c in '!'..='~' {
		let (metrics, rasterized) = font.rasterize(c, rasterize_size as f32);
		character_datas_to_place.push((c, rasterized, metrics));
	}

	let mut char_to_atlas_mappings = HashMap::new();
	let mut atlas_data = vec![0; atlas_size as usize * atlas_size as usize];
	let mut atlas_allocator = AtlasAllocator::new(size2(atlas_size as i32, atlas_size as i32));
	character_datas_to_place
		.sort_by_key(|(_c, _data, metrics)| usize::MAX - metrics.width * metrics.height);

	for (c, data, metrics) in character_datas_to_place {
		let allocation = atlas_allocator
			.allocate(size2(metrics.width as i32, metrics.height as i32))
			.ok_or_else(|| {
				anyhow!(
					"Failed to place all ascii characters within the characters atlas. Atlas size: {atlas_size}"
				)
			})?;
		let x = allocation.rectangle.x_range().start as u32;
		let y = allocation.rectangle.y_range().start as u32;
		let w = allocation.rectangle.x_range().len() as u32;
		let h = allocation.rectangle.y_range().len() as u32;
		place_texture_in_atlas(
			&data,
			(x, y),
			(w, h),
			&mut atlas_data,
			(atlas_size, atlas_size),
			1,
			1,
		);
		let atlas_loc = AtlasLocation {
			pos: (x, y),
			size: (w, h),
			alloc_id: allocation.id,
		};
		char_to_atlas_mappings.insert(c, (atlas_loc, metrics));
	}

	let atlas_texture = gpu_instance
		.wgpu_device
		.create_texture(&wgpu::TextureDescriptor {
			label: Some("text_atlas"),
			size: wgpu::Extent3d {
				width: atlas_size,
				height: atlas_size,
				depth_or_array_layers: 1,
			},
			mip_level_count: 1,
			sample_count: 1,
			dimension: wgpu::TextureDimension::D2,
			format: wgpu::TextureFormat::R8Unorm,
			usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
			view_formats: &[],
		});

	gpu_instance.wgpu_queue.write_texture(
		wgpu::TexelCopyTextureInfoBase {
			texture: &atlas_texture,
			mip_level: 0,
			origin: wgpu::Origin3d::ZERO,
			aspect: wgpu::TextureAspect::All,
		},
		&atlas_data,
		wgpu::TexelCopyBufferLayout {
			offset: 0,
			bytes_per_row: Some(atlas_size),
			rows_per_image: Some(atlas_size),
		},
		wgpu::Extent3d {
			width: atlas_size,
			height: atlas_size,
			depth_or_array_layers: 1,
		},
	);

	let atlas_tex_view = atlas_texture.create_view(&wgpu::TextureViewDescriptor {
		format: Some(wgpu::TextureFormat::R8Unorm),
		..Default::default()
	});

	let atlas_bind_group = gpu_instance
		.wgpu_device
		.create_bind_group(&wgpu::BindGroupDescriptor {
			label: Some("text_atlas_bind_group"),
			layout: &gpu_instance.wgpu_texture_bind_group_layout,
			entries: &[
				wgpu::BindGroupEntry {
					binding: 0,
					resource: wgpu::BindingResource::TextureView(&atlas_tex_view),
				},
				wgpu::BindGroupEntry {
					binding: 1,
					resource: wgpu::BindingResource::Sampler(&gpu_instance.wgpu_linear_sampler),
				},
			],
		});

	Ok(TextRenderer {
		font,
		rasterize_size,
		atlas_data,
		atlas_allocator,
		char_to_atlas_mappings,
		atlas_texture,
		atlas_tex_view,
		atlas_bind_group,
	})
}



/// judges the blurriness of a character atlas
#[must_use]
pub fn approximate_char_atlas_quality(atlas_data: &[u8], width: u32) -> f64 {
	let width = width as usize;
	let mut total_partials = 0;
	let mut total_linear_partials = 0;
	for (i, v) in atlas_data.iter().copied().enumerate() {
		if v == 0 || v == 255 {
			continue;
		}
		total_partials += 1;
		if let Some(left) = atlas_data.get(i - 1)
			&& *left == v
		{
			total_linear_partials += 1;
		} else if let Some(right) = atlas_data.get(i + 1)
			&& *right == v
		{
			total_linear_partials += 1;
		} else if let Some(up) = atlas_data.get(i - width)
			&& *up == v
		{
			total_linear_partials += 1;
		} else if let Some(down) = atlas_data.get(i + width)
			&& *down == v
		{
			total_linear_partials += 1;
		}
	}
	1.0 - f64::from(total_linear_partials) / f64::from(total_partials)
}
