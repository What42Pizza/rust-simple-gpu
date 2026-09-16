use crate::{
	AtlasAllocator, AtlasLocation, CreatedAtlasResult, GpuInstance, Texture, create_texture_atlas,
	get_gpu_limits,
};
use anyhow::Result;
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
	/// The atlas for character textures
	pub atlas: Texture,
	/// The raw data for the atlas texture
	pub atlas_data: Vec<u8>,
	/// This is the allocator used for placing characters into the atlas
	pub atlas_allocator: AtlasAllocator,
	/// Stores the atlas location, glyph placement, and rasterized sdf (signed distance field)
	pub char_datas: HashMap<char, (AtlasLocation, Placement, Vec<u8>)>,
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
	gpu_instance: &mut GpuInstance,
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

	let mut char_datas = HashMap::new();
	let mut char_textures = vec![];
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
		bitmap.placement.width = (bitmap.placement.width) / 3 + 4;
		bitmap.placement.height = (bitmap.placement.height) / 3 + 4;
		bitmap.placement.left = (bitmap.placement.left + 1) / 3 + 2;
		bitmap.placement.top = (bitmap.placement.top + 1) / 3 + 2;
		char_datas.insert(c, (AtlasLocation::default(), bitmap.placement, vec![]));
		char_textures.push((bitmap.placement.width, bitmap.placement.height, data));
	}

	let CreatedAtlasResult {
		tex,
		tex_data,
		placements,
		allocator,
	} = create_texture_atlas(
		"character_atlas",
		&char_textures,
		wgpu::TextureFormat::R8Unorm,
		wgpu::FilterMode::Linear,
		1,
		Some((atlas_size, atlas_size)),
		gpu_instance,
	);

	for (i, (_w, _h, data)) in char_textures.into_iter().enumerate() {
		let c = (i as u8 + b'!') as char;
		let char_data = char_datas
			.get_mut(&c)
			.expect("The character data should have been already inserted");
		char_data.0 = placements[i];
		char_data.2 = data;
	}

	Ok(TextRenderer {
		font_data,
		rasterize_size,
		atlas: tex,
		atlas_data: tex_data,
		atlas_allocator: allocator,
		char_datas,
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
	let output_width = input_width / 3 + 4;
	let output_height = input_height / 3 + 4;
	let mut output = vec![255; (output_width * output_height) as usize];

	let mut edge_points = vec![];
	for (i, v) in input.iter().copied().enumerate() {
		if v < 127 {
			continue;
		}
		let x = i as u32 % input_width;
		if x == 0
			|| x == input_width - 1
			|| i < input_width as usize
			|| i >= input.len() - input_width as usize
		{
			edge_points.push((i as i32 % input_width as i32, i as i32 / input_width as i32));
			continue;
		}
		if input[i - 1] < 127
			|| input[i + 1] < 127
			|| input[i - input_width as usize] < 127
			|| input[i + input_width as usize] < 127
		{
			edge_points.push((i as i32 % input_width as i32, i as i32 / input_width as i32));
		}
	}

	for x in 1..output_width - 1 {
		for y in 1..output_height - 1 {
			let (x_2, y_2) = (x as i32 * 3 - 5, y as i32 * 3 - 5);
			let mut closest_dist_squared = i32::MAX;
			for edge_point in &edge_points {
				let x_dist = edge_point.0 - x_2;
				let y_dist = edge_point.1 - y_2;
				let dist_squared = x_dist * x_dist + y_dist * y_dist;
				closest_dist_squared = closest_dist_squared.min(dist_squared);
			}
			let mut dist = (closest_dist_squared as f32).sqrt();
			if x != 1 && y != 1 && x != output_width - 2 && y != output_height - 2 {
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
