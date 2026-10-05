#[cfg(doc)]
use crate::sync_buffer;
use crate::{
	AtlasAllocator, CreatedAtlasResult, GpuBuffer, GpuInstance, Texture, USAGE_VERTEX_BUFFER,
	create_buffer, create_texture, create_texture_atlas, get_gpu_limits, place_textures_in_atlas,
	update_texture, vertex_buffer_item_type,
};
use anyhow::{Result, bail};
use std::collections::HashMap;
use swash::{
	CacheKey, FontRef, GlyphId,
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
	/// Flag for if `Self::atlas_tex_data` holds data that needs to be synced into `Self::atlas_tex`
	pub atlas_tex_is_dirty: bool,
	/// This is the allocator used for placing characters into the atlas
	pub atlas_allocator: AtlasAllocator,

	/// Stores the atlas location, glyph placement, and rasterized vdf (vector distance field) for glyphs
	pub glyph_render_datas: Vec<(GlyphId, Option<GlyphRenderData>)>,
	/// Holds the gpu buffer for per-string data (text color, flag that enables sub-pixel rendering, etc)
	pub string_datas_buffer: GpuBuffer<StringData>,
	/// Maps a [`StringData`] to its position in [`Self::string_datas_buffer`] (if it already exists there)
	pub string_data_locations: HashMap<StringData, u32>,

	/// The bind group for the text rendering pipeline
	pub wgpu_bind_group: wgpu::BindGroup,
}

/// Contains the data needed to render a character
pub struct GlyphRenderData {
	/// This is the location of the character's vdf texture within the text renderer's texture atlas (x1, y1, x2, y2)
	pub tex_coords: [u16; 4],
	/// Defines the placement of the glyph within the vdf texture
	pub glyph_offset: (u32, u32),
	/// This is the raw data of the character's vdf texture, used if the character atlas needs to be recreated
	pub vdf_tex_data: Vec<u8>,
	/// This is the width and height of `vdf_tex_data`
	pub vdf_tex_size: (u16, u16),
}

vertex_buffer_item_type!(Instance, struct GlyphInstanceData {
	pos: [i32; 2]        as location 0: Sint32x2,
	size: [u16; 2]       as location 1: Sint16x2,
	tex_coords: [u16; 4] as location 2: Uint16x4,
	string_id: u32       as location 3: Uint32,
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
	let mut glyph_render_datas = (b'!'..=b'~')
		.map(|c| {
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
			let glyph_data = GlyphRenderData {
				tex_coords: [0; 4],
				glyph_offset,
				vdf_tex_data: vec![],
				vdf_tex_size: (
					bitmap.placement.width as u16,
					bitmap.placement.height as u16,
				),
			};
			(glyph_id, Some(glyph_data))
		})
		.collect::<Vec<_>>();

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
		let glyph_data = glyph_render_datas[i].1.as_mut().unwrap();
		glyph_data.tex_coords = [
			loc.pos.0 as u16,
			loc.pos.1 as u16,
			(loc.pos.0 + loc.size.0) as u16,
			(loc.pos.1 + loc.size.1) as u16,
		];
		glyph_data.vdf_tex_data = data;
	}

	let string_datas_buffer = create_buffer(
		"string_datas_buffer",
		1024,
		wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
		gpu_instance,
	);

	let bind_group = gpu_instance
		.wgpu_device
		.create_bind_group(&wgpu::BindGroupDescriptor {
			label: Some("text_rendering_bind_group"),
			layout: &gpu_instance.wgpu_text_bind_group_layout,
			entries: &[
				wgpu::BindGroupEntry {
					binding: 0,
					resource: wgpu::BindingResource::TextureView(&atlas_tex.wgpu_view),
				},
				wgpu::BindGroupEntry {
					binding: 1,
					resource: wgpu::BindingResource::Sampler(&gpu_instance.wgpu_linear_sampler),
				},
				wgpu::BindGroupEntry {
					binding: 2,
					resource: wgpu::BindingResource::Buffer(
						string_datas_buffer.wgpu_buffer.as_entire_buffer_binding(),
					),
				},
			],
		});

	Ok(TextRenderer {
		font_data,
		rasterize_size,
		shaping_context: ShapeContext::new(),

		atlas_tex,
		atlas_tex_data,
		atlas_allocator,
		atlas_tex_is_dirty: false,

		glyph_render_datas,

		string_datas_buffer,
		string_data_locations: HashMap::new(),
		wgpu_bind_group: bind_group,
	})
}



/// Clears [`TextRenderer::string_datas_buffer`] and [`TextRenderer::string_data_locations`] if they contain more than `max_string_datas` items
///
/// Notes:
/// - This should never be called between [`place_text()`] and [`render_queued_text()`], this should only be called at the very start or (preferably) the very end of the frame.
/// - If this function is not used, that may be considered a memory leak. However, if you always render text with the same colors and depths, this function may not be needed because the number of stored string datas would not continuously increase.
#[inline]
pub fn trim_text_renderer(text_renderer: &mut TextRenderer, max_string_datas: u16) {
	if text_renderer.string_datas_buffer.len() > max_string_datas as usize {
		text_renderer.string_datas_buffer.clear();
		text_renderer.string_data_locations.clear();
	}
}

/// Returns the pipeline needed for rendering text with a given output texture format
#[inline]
#[must_use]
pub fn get_text_rendering_pipeline(
	output_format: wgpu::TextureFormat,
	gpu_instance: &mut GpuInstance,
) -> &wgpu::RenderPipeline {
	if let Some(pipeline) = gpu_instance.wgpu_text_pipelines.get(&output_format) {
		return pipeline;
	}
	let pipeline =
		gpu_instance
			.wgpu_device
			.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
				label: Some("text_rendering_pipeline"),
				layout: Some(&gpu_instance.wgpu_text_pipeline_layout),
				vertex: wgpu::VertexState {
					module: gpu_instance.wgpu_text_shaders.vertex(),
					entry_point: None,
					buffers: &[GlyphInstanceData::WGPU_LAYOUT],
					compilation_options: wgpu::PipelineCompilationOptions::default(),
				},
				fragment: Some(wgpu::FragmentState {
					module: gpu_instance.wgpu_text_shaders.fragment(),
					entry_point: None,
					targets: &[Some(wgpu::ColorTargetState {
						format: output_format,
						blend: Some(wgpu::BlendState::ALPHA_BLENDING),
						write_mask: wgpu::ColorWrites::ALL,
					})],
					compilation_options: wgpu::PipelineCompilationOptions::default(),
				}),
				primitive: wgpu::PrimitiveState {
					topology: wgpu::PrimitiveTopology::TriangleStrip,
					strip_index_format: None,
					front_face: wgpu::FrontFace::default(),
					cull_mode: None,
					polygon_mode: wgpu::PolygonMode::Fill,
					unclipped_depth: false,
					conservative: false,
				},
				depth_stencil: Some(wgpu::DepthStencilState {
					format: wgpu::TextureFormat::Depth24Plus,
					depth_write_enabled: Some(true),
					depth_compare: Some(wgpu::CompareFunction::Less),
					stencil: wgpu::StencilState::default(),
					bias: wgpu::DepthBiasState::default(),
				}),
				multisample: wgpu::MultisampleState::default(),
				multiview_mask: None,
				cache: None,
			});
	gpu_instance
		.wgpu_text_pipelines
		.insert(output_format, pipeline);
	&gpu_instance.wgpu_text_pipelines[&output_format]
}



/// Creates a buffer of character instance datas
#[inline]
#[must_use]
pub fn create_characters_buffer(
	name: impl Into<String>,
	gpu_instance: &mut GpuInstance,
) -> GpuBuffer<GlyphInstanceData> {
	create_buffer(name, 1024, USAGE_VERTEX_BUFFER, gpu_instance)
}



/// Processes and queues text to be rendered (reminder: you still have to call [`sync_buffer()`] on the characters buffer and on [`TextRenderer::string_datas_buffer`] and call [`render_queued_text()`] for the text to be rendered)
#[allow(unused)]
pub fn place_text(
	text: &str,
	pos: (i32, i32, u32),
	size: u32,
	color: wgpu::Color,
	characters_buffer: &mut GpuBuffer<GlyphInstanceData>,
	text_renderer: &mut TextRenderer,
	gpu_instance: &mut GpuInstance,
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

	let mut glyph_to_place = Vec::with_capacity(text.len());
	shaper.shape_with(|cluster| {
		for glyph in cluster.glyphs {
			glyph_to_place.push(*glyph);
		}
	});

	let mut total_advance = pos.0;
	for glyph in glyph_to_place {
		let glyph_render_data = get_glyph_render_data(glyph.id, text_renderer, gpu_instance);
		let Some(glyph_render_data) = glyph_render_data else {
			continue;
		};
		let char_instance = GlyphInstanceData {
			pos: [
				pos.0 + total_advance - glyph_render_data.glyph_offset.0 as i32,
				pos.1 - glyph_render_data.glyph_offset.1 as i32,
			],
			size: glyph_render_data.vdf_tex_size.into(),
			tex_coords: glyph_render_data.tex_coords,
			string_id,
		};
		characters_buffer.push(char_instance);
		total_advance += glyph.advance as i32;
	}
}



/// Get the [`GlyphRenderData`] for a given glyph, and if the data doesn't exist yet, it generates it and adds it to the [`TextRenderer`]
///
/// # Panics
///
/// This panics if it a glyph fails to render
pub fn get_glyph_render_data<'a>(
	glyph_id: GlyphId,
	text_renderer: &'a mut TextRenderer,
	gpu_instance: &mut GpuInstance,
) -> &'a Option<GlyphRenderData> {
	let insert_i = match text_renderer
		.glyph_render_datas
		.binary_search_by_key(&glyph_id, |(id, _data)| *id)
	{
		Ok(i) => return &text_renderer.glyph_render_datas[i].1,
		Err(i) => i,
	};

	let font_ref = FontRef {
		data: &text_renderer.font_data,
		offset: 0,
		key: CacheKey::new(),
	};
	let mut scale_context = ScaleContext::new();
	let mut font_scaler = scale_context
		.builder(font_ref)
		.size((text_renderer.rasterize_size * 3) as f32)
		.hint(true)
		.build();

	let mut bitmap = Render::new(&[Source::Outline, Source::Bitmap(StrikeWith::BestFit)])
		.format(Format::Alpha)
		.render(&mut font_scaler, glyph_id)
		.expect("Failed to render glyph for character");
	if bitmap.placement.width == 0 || bitmap.placement.height == 0 {
		text_renderer
			.glyph_render_datas
			.insert(insert_i, (glyph_id, None));
		return &text_renderer.glyph_render_datas[insert_i].1;
	}

	let data = generate_vdf(&bitmap.data, bitmap.placement.width);
	bitmap.placement.width = (bitmap.placement.width) / 3 + 4;
	bitmap.placement.height = (bitmap.placement.height) / 3 + 4;
	bitmap.placement.left = (bitmap.placement.left + 1) / 3 + 2;
	bitmap.placement.top = (bitmap.placement.top + 1) / 3 + 2;
	let glyph_offset = (bitmap.placement.left as u32, bitmap.placement.top as u32);

	let (placement, needs_resize, atlas_tex_size) = place_textures_in_atlas(
		&[&*data],
		&[(bitmap.placement.width, bitmap.placement.height)],
		2,
		&mut text_renderer.atlas_tex_data,
		&mut text_renderer.atlas_allocator,
		0,
	);
	let loc = placement[0];

	if needs_resize {
		text_renderer.atlas_tex = create_texture(
			&text_renderer.atlas_tex.settings.name,
			atlas_tex_size,
			wgpu::TextureFormat::Rg8Unorm,
			wgpu::FilterMode::Linear,
			1,
			gpu_instance,
		);
	}
	update_texture(
		&text_renderer.atlas_tex,
		&text_renderer.atlas_tex_data,
		gpu_instance,
	);

	let glyph_render_data = GlyphRenderData {
		tex_coords: [
			loc.pos.0 as u16,
			loc.pos.1 as u16,
			(loc.pos.0 + loc.size.0) as u16,
			(loc.pos.1 + loc.size.1) as u16,
		],
		glyph_offset,
		vdf_tex_data: data,
		vdf_tex_size: (
			bitmap.placement.width as u16,
			bitmap.placement.height as u16,
		),
	};

	text_renderer
		.glyph_render_datas
		.insert(insert_i, (glyph_id, Some(glyph_render_data)));
	&text_renderer.glyph_render_datas[insert_i].1
}



/// Renders queued text using an existing render pass
#[inline]
pub fn render_queued_text(
	characters_buffer: &GpuBuffer<GlyphInstanceData>,
	render_pass: &mut wgpu::RenderPass,
	output_format: wgpu::TextureFormat,
	text_renderer: &TextRenderer,
	gpu_instance: &mut GpuInstance,
) {
	let pipeline = get_text_rendering_pipeline(output_format, gpu_instance);

	render_pass.set_pipeline(pipeline);
	render_pass.set_bind_group(0, &text_renderer.wgpu_bind_group, &[]);
	render_pass.set_vertex_buffer(0, characters_buffer.wgpu_buffer.slice(..));
	render_pass.draw(0..4, 0..characters_buffer.wgpu_buffer_len);
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
