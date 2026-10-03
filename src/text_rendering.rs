#[cfg(doc)]
use crate::sync_buffer;
use crate::{
	AtlasAllocator, CreatedAtlasResult, GpuBuffer, GpuInstance, Texture, USAGE_VERTEX_BUFFER,
	create_buffer, create_texture_atlas, get_gpu_limits, vertex_buffer_item_type,
};
use anyhow::{Result, bail};
use std::{array::from_fn, collections::HashMap};
use swash::{
	CacheKey, FontRef,
	scale::{Render, ScaleContext, Source, StrikeWith},
	shape::ShapeContext,
	zeno::Format,
};



/// Holds all the data needed for rendering text
///
/// Note: you might want to clear both [`string_datas_buffer`](Self::string_datas_buffer) and [`string_data_locations`](Self::string_data_locations) if `Self::string_datas_buffer.len()` gets too high. Especially if you're rendering strings that have lots of different depths
pub struct TextRenderer {
	/// Holds the raw font data
	pub font_data: Vec<u8>,
	/// This is the size at which font is rendered for storage within the character atlas
	pub rasterize_size: u32,
	/// Weird implementation detail, this value is needed for text layout creation
	pub shaping_context: ShapeContext,

	/// The atlas for character textures
	pub atlas_tex: Texture,
	/// The raw data for the atlas texture
	pub atlas_tex_data: Vec<u8>,
	/// This is the allocator used for placing characters into the atlas
	pub atlas_allocator: AtlasAllocator,

	/// Stores the atlas location, glyph placement, and rasterized vdf (vector distance field) for ascii characters
	pub ascii_chars: [CharRenderData; (b'~' - b'!' + 1) as usize],
	/// Stores the atlas location, glyph placement, and rasterized vdf (vector distance field) for non-ascii characters
	pub non_asci_chars: HashMap<char, CharRenderData>,

	/// Holds the gpu buffer for per-string data (text color, flag that enables sub-pixel rendering, etc)
	pub string_datas_buffer: GpuBuffer<StringData>,
	/// Holds the locations of each value of [`StringData`] so that strings that use the same rendering settings can share the same string data instance
	pub string_data_locations: HashMap<StringData, u32>,
}

/// Contains the data needed to render a character
pub struct CharRenderData {
	/// This is the location of the character's vdf texture within the text renderer's texture atlas (x1, y1, x2, y2)
	pub tex_coords: [u16; 4],
	/// Defines the placement of the glyph within the vdf texture
	pub glyph_offset: (u32, u32),
	/// This is the raw data of the character's vdf texture, used if the character atlas needs to be recreated
	pub vdf_tex_data: Vec<u8>,
}

vertex_buffer_item_type!(Instance, struct CharInstanceData {
	screen_pos: [i32; 2]  as location 0: Sint32x2,
	screen_size: [u16; 2] as location 0: Sint16x2,
	tex_coords: [u16; 4]  as location 1: Uint16x4,
	string_id: u32        as location 2: Uint32,
});

/// Holds the per-string data to render
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct StringData {
	/// Holds the color
	pub color: [u8; 4],
	/// Holds the background color, used for subpixel rendering. Note: the fourth component is a bool that indicates if this uses subpixel rendering (0 = no subpixel rendering, >0 = subpixel rendering)
	pub background_color: [u8; 4],
	/// Holds the depth that the string will be rendered with (assuming a depth tex is provided)
	pub depth: u32,
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
	font_data: impl Into<Vec<u8>>,
	rasterize_size: u32,
	max_atlas_size: Option<u32>,
	gpu_instance: &mut GpuInstance,
) -> Result<TextRenderer> {
	let font_data = font_data.into();
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

	let mut char_textures = vec![];
	let mut ascii_chars = from_fn(|i| {
		let c = (i as u8 + b'!') as char;
		let glyph_id = font_ref.charmap().map(c);
		let mut bitmap = Render::new(&[Source::Outline, Source::Bitmap(StrikeWith::BestFit)])
			.format(Format::Alpha)
			.render(&mut font_scaler, glyph_id)
			.expect("Failed to render glyph for character");
		let data = generate_vdf(&bitmap.data, bitmap.placement.width);
		bitmap.placement.width = (bitmap.placement.width) / 3 + 4;
		bitmap.placement.height = (bitmap.placement.height) / 3 + 4;
		bitmap.placement.left = (bitmap.placement.left + 1) / 3 + 2;
		bitmap.placement.top = (bitmap.placement.top + 1) / 3 + 2;
		char_textures.push((bitmap.placement.width, bitmap.placement.height, data));
		let glyph_offset = (bitmap.placement.left as u32, bitmap.placement.top as u32);
		CharRenderData {
			tex_coords: [0; 4],
			glyph_offset,
			vdf_tex_data: vec![],
		}
	});

	let CreatedAtlasResult {
		placements,
		atlas_tex,
		atlas_tex_data,
		atlas_allocator,
	} = create_texture_atlas(
		"character_atlas",
		&char_textures,
		wgpu::TextureFormat::Rg8Unorm,
		wgpu::FilterMode::Linear,
		3,
		0,
		Some((atlas_size, atlas_size)),
		gpu_instance,
	);

	// this is needed due to `CharInstanceData::tex_coords` using u16
	if atlas_tex.wgpu_texture.width() > 65535 {
		bail!(
			"The character atlas cannot be any larger than 65535 (in width or height) but the width is {}, please use a smaller rasterize size.",
			atlas_tex.wgpu_texture.width()
		);
	}
	if atlas_tex.wgpu_texture.height() > 65535 {
		bail!(
			"The character atlas cannot be any larger than 65535 (in width or height) but the height is {}, please use a smaller rasterize size.",
			atlas_tex.wgpu_texture.height()
		);
	}

	for (i, (_w, _h, data)) in char_textures.into_iter().enumerate() {
		let loc = placements[i];
		ascii_chars[i].tex_coords = [
			loc.pos.0 as u16,
			loc.pos.1 as u16,
			(loc.pos.0 + loc.size.0) as u16,
			(loc.pos.1 + loc.size.1) as u16,
		];
		ascii_chars[i].vdf_tex_data = data;
	}

	Ok(TextRenderer {
		font_data,
		rasterize_size,
		shaping_context: ShapeContext::new(),

		atlas_tex,
		atlas_tex_data,
		atlas_allocator,

		ascii_chars,
		non_asci_chars: HashMap::new(),

		string_datas_buffer: create_buffer(
			"string_datas_buffer",
			1024,
			wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
			gpu_instance,
		),
		string_data_locations: HashMap::new(),
	})
}



/// Clears [`TextRenderer::string_datas_buffer`] and [`TextRenderer::string_data_locations`] if they contain more than `max_string_datas` items
///
/// Notes:
/// - This should never be called between [`place_text()`] and [`render_queued_text()`], this should only be called at the very start or (preferably) the very end of the frame.
/// - If this function is not used, that may be considered a memory leak. However, if you always render text with the same colors and depths, this function may not be needed because the number of stored string datas would not continuously increase.
pub fn trim_text_renderer(text_renderer: &mut TextRenderer, max_string_datas: u16) {
	if text_renderer.string_datas_buffer.len() > max_string_datas as usize {
		text_renderer.string_datas_buffer.clear();
		text_renderer.string_data_locations.clear();
	}
}



/// Creates a buffer of character instance datas
#[inline]
#[must_use]
pub fn create_characters_buffer(
	name: impl Into<String>,
	gpu_instance: &mut GpuInstance,
) -> GpuBuffer<CharInstanceData> {
	create_buffer(name, 1024, USAGE_VERTEX_BUFFER, gpu_instance)
}



/// Processes and queues text to be rendered (reminder: you still have to call [`sync_buffer()`] on the characters buffer and on [`TextRenderer::string_datas_buffer`] and call [`render_queued_text()`] for the text to be rendered)
#[allow(unused)]
pub fn place_text(
	text: &str,
	pos: (i32, i32, u32),
	size: u32,
	color: wgpu::Color,
	characters_buffer: &mut GpuBuffer<CharInstanceData>,
	text_renderer: &mut TextRenderer,
) {
	let font_ref = FontRef {
		data: &text_renderer.font_data,
		offset: 0,
		key: CacheKey::new(),
	};

	let string_data = StringData {
		color: [
			(color.r * 255.0) as u8,
			(color.g * 255.0) as u8,
			(color.b * 255.0) as u8,
			(color.a * 255.0) as u8,
		],
		background_color: [0; 4],
		depth: pos.2,
	};
	let string_id = text_renderer.string_data_locations.entry(string_data);
	let string_id = *string_id.or_insert_with(|| {
		let id = text_renderer.string_datas_buffer.len();
		text_renderer.string_datas_buffer.push(string_data);
		id as u32
	});

	let mut shaper = text_renderer
		.shaping_context
		.builder(font_ref)
		.size(size as f32)
		.build();
	shaper.add_str(text);
	//let mut glyph_to_char_mappings = HashMap::new();
	//for c in text.chars() {
	//	let glyph_id = font_ref.charmap().map(c);
	//	glyph_to_char_mappings.insert(glyph_id, c);
	//}
	let mut x = 0;
	shaper.shape_with(|cluster| {
		for glyph in cluster.glyphs {
			x += glyph.advance as i32;
		}
	});
}



/// Renders queued text using an existing render pass
#[allow(unused)]
pub fn render_queued_text(
	characters_buffer: &GpuBuffer<CharInstanceData>,
	render_pass: &wgpu::RenderPass,
	text_renderer: &TextRenderer,
	gpu_instance: &mut GpuInstance,
) {
	todo!();
}



/// Generates an vdf (vector distance field) texture from an alpha texture. More:
///
/// - The input is expected to be 3 times larger than the output in both dimensions
/// - The input is [`R8Unorm`](wgpu::TextureFormat::R8Unorm), where 0 is fully transparent and 1 is fully opaque
/// - The output is [`Rg8Unorm`](wgpu::TextureFormat::Rg8Unorm), which stores the vector from the pixel to the nearest glyph edge
///   -
///   - is (127, 127) if the pixel is inside the glyph
#[must_use]
pub fn generate_vdf(input: &[u8], input_width: u32) -> Vec<u8> {
	let input_height = input.len() as u32 / input_width;
	let output_width = input_width / 3 + 4;
	let output_height = input_height / 3 + 4;
	let mut output = vec![0; (output_width * output_height) as usize * 2];

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

	for x in 0..output_width {
		for y in 0..output_height {
			let (x_2, y_2) = ((x as i32 - 2) * 3 + 2, (y as i32 - 2) * 3 + 2);
			let mut closest_dist_squared = i32::MAX;
			let mut x_closest_diff = 0;
			let mut y_closest_diff = 0;
			for edge_point in &edge_points {
				let x_diff = edge_point.0 - x_2;
				let y_diff = edge_point.1 - y_2;
				let dist_squared = x_diff * x_diff + y_diff * y_diff;
				if dist_squared < closest_dist_squared {
					closest_dist_squared = dist_squared;
					x_closest_diff = x_diff;
					y_closest_diff = y_diff;
				}
			}
			if x > 1 && y > 1 && x < output_width - 2 && y < output_height - 2 {
				let is_in_glyph = input[x_2 as usize + y_2 as usize * input_width as usize] >= 127;
				if is_in_glyph {
					x_closest_diff = 0;
					y_closest_diff = 0;
				}
			}
			let x_diff_unorm = (x_closest_diff + 128) as u8;
			output[(x as usize + y as usize * output_width as usize) * 2] = x_diff_unorm;
			let y_diff_unorm = (y_closest_diff + 128) as u8;
			output[(x as usize + y as usize * output_width as usize) * 2 + 1] = y_diff_unorm;
		}
	}

	output
}
