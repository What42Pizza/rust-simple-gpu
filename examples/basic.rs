#![allow(unused)]
#![warn(unused_must_use)]

// controls:
// - wasd
// - space: up
// - left shift: down
// - q: turn left
// - e: turn right
// - esc: quit



use anyhow::*;
use bytemuck::Zeroable;
use glam::{
	Mat4, Vec3,
	camera::rh::{proj, view},
	vec3,
};
use log::info;
use sdl3::{
	event::{Event, WindowEvent},
	keyboard::{KeyboardState, Keycode},
	libc::rand,
};
use simple_gpu::BufferItemRawData;
use std::{path::PathBuf, time::Instant};



#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
struct UniformsRawData {
	pub view_mat: Mat4,
	pub proj_mat: Mat4,
	pub proj_view_mat: Mat4,
	pub inv_view_mat: Mat4,
	pub inv_proj_mat: Mat4,
	pub inv_proj_view_mat: Mat4,
}

impl UniformsRawData {
	fn update(&mut self, program_data: &ProgramData) {
		let camera = &program_data.camera;
		let camera_target = glam::Vec3::new(
			camera.rot_xz.cos() * camera.rot_y.cos(),
			camera.rot_y.sin(),
			camera.rot_xz.sin() * camera.rot_y.cos(),
		);
		self.view_mat = view::look_to_mat4(camera.pos, camera_target, Vec3::new(0.0, 1.0, 0.0));
		self.proj_mat = proj::opengl::perspective(
			camera.fov_radians,
			program_data.aspect_ratio,
			camera.near_plane,
			camera.far_plane,
		);
		self.proj_view_mat = self.proj_mat * self.view_mat;
		self.inv_view_mat = self.view_mat.inverse();
		self.inv_proj_mat = self.proj_mat.inverse();
		self.inv_proj_view_mat = self.proj_view_mat.inverse();
	}
}



struct ProgramData {
	should_quit: bool,
	last_dt_instant: Instant,

	camera: CameraData,
	aspect_ratio: f32,

	vertex_buffer: simple_gpu::VertexBuffer<VertexData>,
	main_index_buffer: simple_gpu::IndexBuffer,
	instance_buffer: simple_gpu::VertexBuffer<InstanceData>,
	textures: Textures,

	uniforms_buffer: simple_gpu::UniformsBuffer<UniformsRawData>,
}

struct CameraData {
	pos: glam::Vec3,
	rot_xz: f32,
	rot_y: f32,
	fov_radians: f32,
	near_plane: f32,
	far_plane: f32,
}

struct Textures {
	wall_tex: simple_gpu::Texture,
}



simple_gpu::make_vertex_buffer_type!(Vertex, struct VertexData {
	pos:   [f32; 3] as location 0: Float32x3,
	uv:    [f32; 2] as location 1: Float32x2,
	color: [f32; 4] as location 2: Float32x4,
});

simple_gpu::make_vertex_buffer_type!(Instance, struct InstanceData {
	pos: [f32; 3] as location 3: Float32x3,
});




fn main() -> Result<()> {
	// enable logging
	if std::env::var("RUST_LOG").is_err() {
		unsafe {
			// safety: this seems to only be unsafe if other threads might be reading/writing env vars, and that should not be possible yet since this is the start of the program
			std::env::set_var("RUST_LOG", "warn");
		}
	}
	env_logger::init();

	let assets_path = get_assets_path()?;

	// sdl
	let sdl = sdl3::init()?;
	let video = sdl.video()?;
	let mut window = video
		.window("Simple Gpu Example", 1280, 720)
		.position_centered()
		.resizable()
		.hidden()
		.metal_view()
		.build()?;
	let window_size = window.size();
	let mut event_pump = sdl.event_pump()?;

	// basics
	let mut gpu_instance =
		simple_gpu::init(wgpu::Limits::defaults(), wgpu::MemoryHints::Performance)?;
	let (window, window_surface) = simple_gpu::get_window_surface_mut(
		&gpu_instance,
		&mut window,
		window_size,
		wgpu::PresentMode::AutoVsync,
	);
	let mut window_surface = window_surface?;
	let mut depth_tex = simple_gpu::create_depth_texture(
		"main depth tex",
		window_size,
		wgpu::FilterMode::Linear,
		&gpu_instance,
	);

	// shaders
	let shaders_path = assets_path.join("shaders");
	let main_vsh_shader =
		simple_gpu::load_glsl_vertex_shader(&shaders_path.join("main.vsh"), &gpu_instance, &[])?;
	let main_fsh_shader =
		simple_gpu::load_glsl_fragment_shader(&shaders_path.join("main.fsh"), &gpu_instance, &[])?;

	// uniforms
	let uniforms_buffer = simple_gpu::create_uniforms_buffer::<UniformsRawData>(&gpu_instance);
	let mut uniforms_raw_data = UniformsRawData::zeroed();

	// textures
	let textures_path = assets_path.join("textures");
	let mut wall_tex = simple_gpu::load_texture_from_path(
		&textures_path.join("wall.png"),
		wgpu::FilterMode::Linear,
		1,
		&gpu_instance,
	)?;

	// pipeline
	let main_pipeline = simple_gpu::create_3d_pipeline(
		"main pipeline",
		&[
			Some(VertexData::WGPU_LAYOUT),
			Some(InstanceData::WGPU_LAYOUT),
		],
		&main_vsh_shader,
		&main_fsh_shader,
		window_surface.wgpu_format,
		1,
		&mut gpu_instance,
	);

	// vertex data
	let mut main_vertex_buffer = simple_gpu::init_vertex_buffer(
		"main vertex buffer",
		&[
			VertexData {
				pos: [1.0, 1.0, 0.0],
				uv: [1.0, 0.0],
				color: [1.0; 4],
			},
			VertexData {
				pos: [-1.0, 1.0, 0.0],
				uv: [0.0, 0.0],
				color: [1.0; 4],
			},
			VertexData {
				pos: [1.0, -1.0, 0.0],
				uv: [1.0, 1.0],
				color: [1.0; 4],
			},
			VertexData {
				pos: [-1.0, -1.0, 0.0],
				uv: [0.0, 1.0],
				color: [1.0; 4],
			},
		],
		&gpu_instance,
	);

	// index data
	let mut main_index_buffer =
		simple_gpu::create_index_buffer("main index buffer", &[0, 1, 2, 2, 1, 3], &gpu_instance);

	// instance data
	let mut main_instance_buffer = simple_gpu::init_vertex_buffer(
		"main instance buffer",
		&[
			InstanceData {
				pos: [0.5, 0.5, -2.5],
			},
			InstanceData {
				pos: [0.0, 0.0, -3.0],
			},
		],
		&gpu_instance,
	);

	// assemble program's data
	let mut data = ProgramData {
		should_quit: false,
		last_dt_instant: Instant::now(),

		camera: CameraData {
			pos: vec3(0.0, 0.0, 0.0),
			rot_xz: -90.0f32.to_radians(),
			rot_y: 0.0,
			fov_radians: 70.0f32.to_radians(),
			near_plane: 0.1,
			far_plane: 500.0,
		},
		aspect_ratio: window_size.0 as f32 / window_size.1 as f32,

		vertex_buffer: main_vertex_buffer,
		main_index_buffer,
		instance_buffer: main_instance_buffer,
		textures: Textures { wall_tex },

		uniforms_buffer,
	};



	window.show();
	while !data.should_quit {
		// initial update
		let new_dt_instant = Instant::now();
		let dt = new_dt_instant
			.duration_since(data.last_dt_instant)
			.as_secs_f32();
		data.last_dt_instant = new_dt_instant;

		// handle events
		for event in event_pump.poll_iter() {
			match event {
				Event::Window {
					window_id,
					win_event:
						WindowEvent::PixelSizeChanged(new_width, new_height)
						| WindowEvent::Resized(new_width, new_height),
					..
				} if window_id == window.id() => {
					simple_gpu::reconfigure_window_surface(
						&mut window_surface,
						&gpu_instance,
						(new_width as u32, new_height as u32),
					);
					depth_tex = simple_gpu::create_depth_texture(
						"main depth texture",
						window.size(),
						wgpu::FilterMode::Linear,
						&gpu_instance,
					);
					data.aspect_ratio = new_width as f32 / new_height as f32;
				}
				Event::Quit { .. }
				| Event::Window {
					win_event: WindowEvent::CloseRequested,
					..
				}
				| Event::KeyDown {
					keycode: Some(Keycode::Escape),
					..
				} => {
					println!("closing");
					data.should_quit = true;
				}
				e => {
					info!("Unknown event: {e:?}");
				}
			}
		}

		// update
		let keyboard_state = KeyboardState::new(&event_pump);
		if keyboard_state.is_scancode_pressed(sdl3::keyboard::Scancode::A) {
			data.camera.pos.x -= 0.75 * dt;
		}
		if keyboard_state.is_scancode_pressed(sdl3::keyboard::Scancode::D) {
			data.camera.pos.x += 0.75 * dt;
		}
		if keyboard_state.is_scancode_pressed(sdl3::keyboard::Scancode::W) {
			data.camera.pos.z -= 0.75 * dt;
		}
		if keyboard_state.is_scancode_pressed(sdl3::keyboard::Scancode::S) {
			data.camera.pos.z += 0.75 * dt;
		}
		if keyboard_state.is_scancode_pressed(sdl3::keyboard::Scancode::Q) {
			data.camera.rot_xz -= 0.25 * dt;
		}
		if keyboard_state.is_scancode_pressed(sdl3::keyboard::Scancode::E) {
			data.camera.rot_xz += 0.25 * dt;
		}
		if keyboard_state.is_scancode_pressed(sdl3::keyboard::Scancode::Space) {
			data.camera.pos.y += 0.75 * dt;
		}
		if keyboard_state.is_scancode_pressed(sdl3::keyboard::Scancode::LShift) {
			data.camera.pos.y -= 0.75 * dt;
		}

		// render
		uniforms_raw_data.update(&data);
		simple_gpu::update_uniforms_buffer(
			&data.uniforms_buffer,
			&uniforms_raw_data,
			&gpu_instance,
		);

		let (surface_tex, surface_tex_view, mut command_encoder) =
			match simple_gpu::start_frame("render frame", &window_surface, &gpu_instance) {
				simple_gpu::StartFrameResult::Some(tex, view, commands) => (tex, view, commands),
				simple_gpu::StartFrameResult::None => continue,
				simple_gpu::StartFrameResult::Error => {
					simple_gpu::reconfigure_window_surface(
						&mut window_surface,
						&gpu_instance,
						window.size(),
					);
					depth_tex = simple_gpu::create_depth_texture(
						"main depth texture",
						window.size(),
						wgpu::FilterMode::Linear,
						&gpu_instance,
					);
					continue;
				}
			};

		let mut render_pass = simple_gpu::start_3d_render_pass(
			"main render pass",
			&surface_tex_view,
			&depth_tex.wgpu_view,
			Some(wgpu::Color::WHITE),
			true,
			&mut command_encoder,
		);

		simple_gpu::render(
			&mut render_pass,
			&main_pipeline,
			&[
				&data.vertex_buffer.wgpu_buffer,
				&data.instance_buffer.wgpu_buffer,
			],
			Some(&data.main_index_buffer),
			&[&data.textures.wall_tex],
			&data.uniforms_buffer.wgpu_bind_group,
			data.vertex_buffer.wgpu_buffer_len,
			data.instance_buffer.wgpu_buffer_len,
		);

		simple_gpu::finish_render_pass(render_pass);

		simple_gpu::finish_frame(command_encoder, surface_tex, &gpu_instance);
	}

	Ok(())
}



pub fn get_assets_path() -> Result<PathBuf> {
	let mut output = std::env::current_exe()?;
	loop {
		output.push("examples/assets");
		if output.exists() {
			return Ok(output);
		}
		output.pop();
		output.pop();
		if !output.pop() {
			bail!("Failed to find assets folder near executable or any parent folders.");
		}
	}
}
