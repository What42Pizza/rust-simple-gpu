use crate::{AtlasLocation, GpuInstance, get_gpu_limits, place_texture_in_atlas};
use anyhow::{Result, anyhow};
use guillotiere::{AtlasAllocator, size2};
use std::collections::HashMap;
use swash::{
	CacheKey, FontRef,
	scale::{Render, ScaleContext, Source, StrikeWith},
	zeno::{Format, Placement},
};



/// Holds all the data needed for rendering text
pub struct TextRenderer {
	/// Holds the raw font data
	pub font_data: Vec<u8>,
	/// This is the size at which font is rendered for storage within the character atlas
	pub rasterize_size: u32,
	/// This is the atlas that all rendered characters are placed in
	pub atlas_data: Vec<u8>,
	/// This is the allocator used for placing characters into the atlas
	pub atlas_allocator: AtlasAllocator,
	/// Maps characters to both their location within the atlas and their placements
	pub char_to_atlas_mappings: HashMap<char, (AtlasLocation, Placement)>,
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
///
/// # Panics
///
/// This panics if it a glyph fails to render
pub fn create_text_renderer(
	font: impl Into<Vec<u8>>,
	rasterize_size: u32,
	max_atlas_size: Option<u32>,
	gpu_instance: &GpuInstance,
) -> Result<TextRenderer> {
	let font_data = font.into();
	let font_ref = FontRef {
		data: &font_data,
		offset: 0,
		key: CacheKey::new(),
	};

	let mut scale_context = ScaleContext::new();
	let mut font_scaler = scale_context
		.builder(font_ref)
		.size((rasterize_size * 3) as f32)
		.hint(true)
		.build();

	let atlas_size = get_gpu_limits(gpu_instance).max_texture_dimension_2d;
	let max_atlas_size = max_atlas_size.unwrap_or(rasterize_size * 6);
	let atlas_size = atlas_size.min(max_atlas_size);

	let mut character_datas_to_place = vec![];
	for c in '!'..='~' {
		let glyph_id = font_ref.charmap().map(c);
		let mut bitmap = Render::new(&[Source::Outline, Source::Bitmap(StrikeWith::BestFit)])
			.format(Format::Alpha)
			.render(&mut font_scaler, glyph_id)
			.expect("Failed to render glyph for character");
		let data = generate_sdf(
			&bitmap.data,
			bitmap.placement.width,
			0.3 / rasterize_size as f32,
		);
		bitmap.placement.width = (bitmap.placement.width) / 3 + 2;
		bitmap.placement.height = (bitmap.placement.height) / 3 + 2;
		bitmap.placement.left = (bitmap.placement.left + 1) / 3;
		bitmap.placement.top = (bitmap.placement.top + 1) / 3;
		character_datas_to_place.push((c, data, bitmap.placement));
	}

	let mut char_to_atlas_mappings = HashMap::new();
	let mut atlas_data = vec![255; atlas_size as usize * atlas_size as usize];
	let mut atlas_allocator = AtlasAllocator::new(size2(atlas_size as i32, atlas_size as i32));
	character_datas_to_place
		.sort_by_key(|(_c, _data, placement)| u32::MAX - placement.width * placement.height);

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
		font_data,
		rasterize_size,
		atlas_data,
		atlas_allocator,
		char_to_atlas_mappings,
		atlas_texture,
		atlas_tex_view,
		atlas_bind_group,
	})
}



/// Generates an sdf texture from an alpha texture. More:
///
/// - The input is expected to be 3 times larger than the output in both dimensions
/// - The input is `R8Unorm`, where 0 is fully transparent and 1 is fully opaque
/// - The output is `R8Unorm`, where 0.5 is at the glyph edge, 0-0.5 is inside, and 0.5-1.0 is outside
#[must_use]
pub fn generate_sdf(input: &[u8], input_width: u32, dist_scale: f32) -> Vec<u8> {
	let input_height = input.len() as u32 / input_width;
	let output_width = input_width / 3 + 2;
	let output_height = input_height / 3 + 2;
	let mut output = vec![0; (output_width * output_height) as usize];

	let mut edge_points = vec![];
	for (i, v) in input.iter().copied().enumerate() {
		if v < 127 {
			continue;
		}
		let x = i as u32 % input_width;
		if (x > 0 && input[i - 1] < 127)
			|| (x < input_width - 1 && input[i + 1] < 127)
			|| (i >= input_width as usize && input[i - input_width as usize] < 127)
			|| (i < input.len() - input_width as usize && input[i + input_width as usize] < 127)
		{
			edge_points.push((i as i32 % input_width as i32, i as i32 / input_width as i32));
		}
	}

	for x in 0..output_width {
		for y in 0..output_height {
			let (x_2, y_2) = (x as i32 * 3 - 2, y as i32 * 3 - 2);
			let mut closest_dist_squared = i32::MAX;
			for edge_point in &edge_points {
				let x_dist = edge_point.0 - x_2;
				let y_dist = edge_point.1 - y_2;
				let dist_squared = x_dist * x_dist + y_dist * y_dist;
				closest_dist_squared = closest_dist_squared.min(dist_squared);
			}
			let mut dist = (closest_dist_squared as f32).sqrt();
			if x != 0 && y != 0 && x != output_width - 1 && y != output_height - 1 {
				let is_positive = input[x_2 as usize + y_2 as usize * input_width as usize] < 127;
				if !is_positive {
					dist *= -1.0;
				}
			}
			let unorm_value = (dist.mul_add(dist_scale, 0.5).clamp(0.0, 1.0) * 255.0) as u8;
			output[x as usize + y as usize * output_width as usize] = unorm_value;
		}
	}

	output
}
