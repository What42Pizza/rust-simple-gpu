use crate::{
	AtlasAllocator, AtlasLocation, CreatedAtlasResult, GpuInstance, Texture, VertexBuffer,
	create_texture_atlas, create_vertex_buffer, get_gpu_limits, make_vertex_buffer_type,
};
use anyhow::{Result, bail};
use std::{array::from_fn, collections::HashMap};
use swash::{
	CacheKey, FontRef,
	scale::{Render, ScaleContext, Source, StrikeWith},
	zeno::Format,
};



/// Holds all the data needed for rendering text
pub struct TextRenderer {
	/// Holds the raw font data
	pub font_data: Vec<u8>,
	/// This is the size at which font is rendered for storage within the character atlas
	pub rasterize_size: u32,

	/// The atlas for character textures
	pub atlas_tex: Texture,
	/// The raw data for the atlas texture
	pub atlas_tex_data: Vec<u8>,
	/// This is the allocator used for placing characters into the atlas
	pub atlas_allocator: AtlasAllocator,

	/// Stores the atlas location, glyph placement, and rasterized sdf (signed distance field) for ascii characters
	pub ascii_chars: [CharRenderData; (b'~' - b'!' + 1) as usize],
	/// Stores the atlas location, glyph placement, and rasterized sdf (signed distance field) for non-ascii characters
	pub non_asci_chars: HashMap<char, CharRenderData>,

	/// Holds all the characters that will be rendered
	pub instances_buffer: VertexBuffer<CharInstanceData>,
	/// Holds the wgpu buffer for per-string data
	pub string_datas_buffer: wgpu::Buffer,
	/// The current capacity of `Self::string_datas_buffer`
	pub string_datas_buffer_cap: u32,
	/// The current cpu-side copy of `Self::string_datas_buffer`
	pub string_datas_buffer_vec: Vec<StringData>,
	/// Holds the locations of each value of [`StringData`] so that strings that use the same rendering settings can share the same string data instance
	pub string_data_locations: HashMap<StringData, u32>,
}

/// Contains the data needed to render a character
pub struct CharRenderData {
	/// This is the location of the character's sdf texture within the text renderer's texture atlas
	pub atlas_location: AtlasLocation,
	/// Defines the placement of the glyph within the sdf texture
	pub glyph_offset: (u32, u32),
	/// This is the raw data of the character's sdf texture, used if the character atlas needs to be recreated
	pub sdf_tex_data: Vec<u8>,
}

make_vertex_buffer_type!(Instance, struct CharInstanceData {
	screen_coords: [u16; 4] as location 0: Uint16x4,
	tex_coords: [u16; 4]    as location 1: Uint16x4,
	string_id: u32          as location 2: Uint32,
});

/// Holds the per-string data to render
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct StringData {
	/// Holds the color
	pub color: [u8; 4],
	/// Holds the background color, used for subpixel rendering. Note: the fourth component is a bool that indicates if this uses subpixel rendering
	pub background_color: [u8; 4],
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

	let mut char_textures = vec![];
	let mut ascii_chars = from_fn(|i| {
		let c = (i as u8 + b'!') as char;
		let glyph_id = font_ref.charmap().map(c);
		let mut bitmap = Render::new(&[Source::Outline, Source::Bitmap(StrikeWith::BestFit)])
			.format(Format::Alpha)
			.render(&mut font_scaler, glyph_id)
			.expect("Failed to render glyph for character");
		let data = generate_sdf(&bitmap.data, bitmap.placement.width, 0.03);
		bitmap.placement.width = (bitmap.placement.width) / 3 + 4;
		bitmap.placement.height = (bitmap.placement.height) / 3 + 4;
		bitmap.placement.left = (bitmap.placement.left + 1) / 3 + 2;
		bitmap.placement.top = (bitmap.placement.top + 1) / 3 + 2;
		char_textures.push((bitmap.placement.width, bitmap.placement.height, data));
		let glyph_offset = (bitmap.placement.left as u32, bitmap.placement.top as u32);
		CharRenderData {
			atlas_location: AtlasLocation::default(),
			glyph_offset,
			sdf_tex_data: vec![],
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
		wgpu::TextureFormat::R8Unorm,
		wgpu::FilterMode::Linear,
		3,
		255,
		Some((atlas_size, atlas_size)),
		gpu_instance,
	);
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
		ascii_chars[i].atlas_location = placements[i];
		ascii_chars[i].sdf_tex_data = data;
	}

	let string_datas_buffer_cap = 256;
	let string_datas_buffer = gpu_instance
		.wgpu_device
		.create_buffer(&wgpu::BufferDescriptor {
			label: Some("string_datas_buffer"),
			size: u64::from(string_datas_buffer_cap) * std::mem::size_of::<StringData>() as u64,
			usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
			mapped_at_creation: false,
		});

	Ok(TextRenderer {
		font_data,
		rasterize_size,

		atlas_tex,
		atlas_tex_data,
		atlas_allocator,

		ascii_chars,
		non_asci_chars: HashMap::new(),

		instances_buffer: create_vertex_buffer("text_instances_buffer", 1024, gpu_instance),
		string_datas_buffer,
		string_datas_buffer_cap,
		string_datas_buffer_vec: vec![],
		string_data_locations: HashMap::new(),
	})
}



/// Processes text to be rendered and queues it for rendering
#[allow(unused)]
pub fn render_text(
	text: &str,
	pos: (u32, u32),
	size: u32,
	color: wgpu::Color,
	text_renderer: &mut TextRenderer,
) {
	let string_data = StringData {
		color: [
			(color.r * 255.0) as u8,
			(color.g * 255.0) as u8,
			(color.b * 255.0) as u8,
			(color.a * 255.0) as u8,
		],
		background_color: [0; 4],
	};
	let string_data_id = text_renderer.string_data_locations.entry(string_data);
	let string_data_id = *string_data_id.or_insert_with(|| {
		let id = text_renderer.string_datas_buffer_vec.len();
		text_renderer.string_datas_buffer_vec.push(string_data);
		id as u32
	});
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
