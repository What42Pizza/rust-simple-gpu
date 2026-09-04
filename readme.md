# Simple Gpu

This is a tiny abstraction over WGPU that aims for a multimedia-like api for rendering custom vertex data with custom shaders.

<br>

Workflow:

- First, you define:
  - The layout of uniforms (all shaders are given the same uniforms)
  - The layout of vertex types
  - The layout of instance types (optional)
- Then, you load:
  - A window (with any backend you want)
  - A `GpuInstance` (with `simple_gpu::init()?`)
  - The shaders you will use (Glsl is suggested)
    - `simple_gpu::load_glsl_vertex_shader()`
    - `simple_gpu::load_glsl_fragment_shader()`
  - A buffer for uniforms (with `simple_gpu::create_uniforms_buffer()`)
  - The textures you will render
    - Each render pass expects one texture input, so atlases are useful here
	- `simple_gpu::load_texture_from_path()`
	- `simple_gpu::create_texture()` and `simple_gpu::update_texture()`
  - A depth texture (optional, with `simple_gpu::create_depth_texture()`)
  
