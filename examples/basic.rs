#![allow(unused)]
#![warn(unused_must_use)]



use std::{fs, path::PathBuf, time::{Duration, Instant}};
use anyhow::*;
use bytemuck::Zeroable;
use glam::{Mat4, Quat, Vec3, camera::rh::{proj, view}, vec3};
use log::{info, warn};
use sdl3::{event::{Event, WindowEvent}, keyboard::{self, Keycode}};
use simple_gpu::BufferItemRawData;



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
		let camera_target = camera.pos + glam::Vec3::new(
			camera.rot_xz.cos() * camera.rot_y.cos(),
			camera.rot_y.sin(),
			camera.rot_xz.sin() * camera.rot_y.cos(),
		);
		self.view_mat = view::look_to_mat4(camera.pos, camera_target, Vec3::new(0.0, 1.0, 0.0));
		self.proj_mat = proj::opengl::perspective(camera.fov_radians, program_data.aspect_ratio, camera.near_plane, camera.far_plane);
		self.proj_view_mat = self.proj_mat * self.view_mat;
		self.inv_view_mat = self.view_mat.inverse();
		self.inv_proj_mat = self.proj_mat.inverse();
		self.inv_proj_view_mat = self.proj_view_mat.inverse();
		
	}
}



struct ProgramData {
	camera: CameraData,
	aspect_ratio: f32,
}

struct CameraData {
	pos: glam::Vec3,
	rot_xz: f32,
	rot_y: f32,
	fov_radians: f32,
	near_plane: f32,
	far_plane: f32,
}



#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
struct BasicVertexRawData {
	pub pos: [f32; 3],
	pub uv: [f32; 2],
	pub color: [f32; 4],
}

impl simple_gpu::BufferItemRawData for BasicVertexRawData {
	const FIELDS: &[wgpu::VertexAttribute] = &wgpu::vertex_attr_array![
		0 => Float32x3,
		1 => Float32x2,
		2 => Float32x4,
	];
	const STEP_MODE: wgpu::VertexStepMode = wgpu::VertexStepMode::Vertex;
}



fn main() -> Result<()> {
	if std::env::var("RUST_LOG").is_err() {
		unsafe {
			// safety: this seems to only be unsafe if other threads might be reading/writing env vars, and that should not be possible yet since this is the start of the program
			std::env::set_var("RUST_LOG", "info");
		}
	}
	env_logger::init();
	
	let sdl = sdl3::init()?;
	let video = sdl.video()?;
	let mut window =
		video.window("Simple Gpu Example", 1280, 720)
		.position_centered()
		.resizable()
		.metal_view()
		.build()?;
	let mut event_pump = sdl.event_pump()?;
	
	
	
	let gpu_instance = simple_gpu::init()?;
	let window_size = window.size();
	let (window, window_surface) = simple_gpu::get_window_surface_mut(&gpu_instance, &mut window, window_size, wgpu::PresentMode::AutoVsync);
	let mut window_surface = window_surface?;
	window.show();
	
	
	
	let assets_path = get_assets_path()?;
	
	let shaders_path = assets_path.join("shaders");
	let main_vsh_shader = simple_gpu::load_glsl_vertex_shader(&shaders_path.join("main.vsh"), &gpu_instance, &[])?;
	let main_fsh_shader = simple_gpu::load_glsl_fragment_shader(&shaders_path.join("main.fsh"), &gpu_instance, &[])?;
	
	
	
	let uniforms_buffer = simple_gpu::create_uniforms_buffer::<UniformsRawData>(&gpu_instance);
	let mut uniforms_raw_data = UniformsRawData::zeroed();
	
	
	
	let textures_path = assets_path.join("textures");
	let sampler = simple_gpu::make_sampler(wgpu::AddressMode::Repeat, wgpu::FilterMode::Nearest, &gpu_instance);
	let wall_tex = simple_gpu::load_texture_from_path(&textures_path.join("wall.png"), &sampler, &gpu_instance)?;
	
	let mut depth_tex = simple_gpu::create_depth_texture("main depth tex", window_size, &gpu_instance);
	
	
	
	let main_pipeline = simple_gpu::create_3d_pipeline("main pipeline", &[Some(BasicVertexRawData::BUFFER_LAYOUT)], &main_vsh_shader, &main_fsh_shader, &window_surface.wgpu_format, &gpu_instance);
	
	
	
	let mut vertex_buffer = simple_gpu::create_vertex_buffer("main vertex buffer", 6, &gpu_instance);
	simple_gpu::update_vertex_buffer(&mut vertex_buffer, &[
		BasicVertexRawData {
			pos: [1.0, 1.0, -3.0],
			uv: [1.0, 0.0],
			color: [1.0; 4],
		},
		BasicVertexRawData {
			pos: [-1.0, 1.0, -3.0],
			uv: [0.0, 0.0],
			color: [1.0; 4],
		},
		BasicVertexRawData {
			pos: [1.0, -1.0, -3.0],
			uv: [1.0, 1.0],
			color: [1.0; 4],
		},
		BasicVertexRawData {
			pos: [1.0, -1.0, -3.0],
			uv: [1.0, 1.0],
			color: [1.0; 4],
		},
		BasicVertexRawData {
			pos: [-1.0, 1.0, -3.0],
			uv: [0.0, 0.0],
			color: [1.0; 4],
		},
		BasicVertexRawData {
			pos: [-1.0, -1.0, -3.0],
			uv: [0.0, 1.0],
			color: [1.0; 4],
		},
	], &gpu_instance);
	
	
	
	let mut program_data = ProgramData {
		camera: CameraData {
			pos: vec3(0.0, 0.0, 0.0),
			rot_xz: -90.0f32.to_radians(),
			rot_y: 0.0,
			fov_radians: 70.0f32.to_radians(),
			near_plane: 0.1,
			far_plane: 500.0,
		},
		aspect_ratio: window_size.0 as f32 / window_size.1 as f32,
	};
	
	
	
	'running: loop {
		
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
					simple_gpu::reconfigure_window_surface(&mut window_surface, &gpu_instance, (new_width as u32, new_height as u32));
					depth_tex = simple_gpu::create_depth_texture("main depth texture", window.size(), &gpu_instance);
				}
				Event::Quit { .. }
				| Event::Window { win_event: WindowEvent::CloseRequested, .. }
				| Event::KeyDown {
					keycode: Some(Keycode::Escape),
					..
				} => {
					println!("closing");
					break 'running;
				}
				e => {
					info!("Unknown event: {e:?}");
				}
			}
		}
		
		// update
		
		// render
		uniforms_raw_data.update(&program_data);
		simple_gpu::update_uniforms_buffer(&uniforms_buffer, &uniforms_raw_data, &gpu_instance);
		
		let (surface_tex, surface_tex_view) = match simple_gpu::get_surface_texture(&window_surface) {
			simple_gpu::SurfaceTextureResult::Some(tex, view) => (tex, view),
			simple_gpu::SurfaceTextureResult::None => continue,
			simple_gpu::SurfaceTextureResult::Error => {
				simple_gpu::reconfigure_window_surface(&mut window_surface, &gpu_instance, window.size());
				depth_tex = simple_gpu::create_depth_texture("main depth texture", window.size(), &gpu_instance);
				continue;
			}
		};
		
		let mut command_encoder = simple_gpu::start_command_encoder("render frame", &gpu_instance);
		
		let mut render_pass = simple_gpu::start_3d_render_pass("main render pass", &surface_tex_view, &depth_tex.wgpu_view, Some(wgpu::Color::WHITE), true, &mut command_encoder);
		
		simple_gpu::render(&mut render_pass, &main_pipeline, &[&vertex_buffer.wgpu_buffer], None, &wall_tex, &uniforms_buffer.wgpu_bind_group, vertex_buffer.count as u32, 1);
		
		simple_gpu::finish_render_pass(render_pass);
		
		simple_gpu::finish_command_encoder(command_encoder, &gpu_instance);
		simple_gpu::present_frame(surface_tex, &gpu_instance);
		
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
