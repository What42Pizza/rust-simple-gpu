# Simple Gpu

This is a tiny abstraction over WGPU that is inspired by the simplicity of multimedia libraries like raylib and SDL. The main goal of this crate is to provide a simple and convenient way of rendering custom data with custom shaders.

### Other similar crates:

- [simple-wgpu](https://crates.io/crates/simple-wgpu)
- [easy-gpu](https://crates.io/crates/easy-gpu)
- [easygpu](https://crates.io/crates/easygpu) (only meant for the [Kludgine](https://github.com/khonsulabs/kludgine) engine)

### What sets this apart:

- This focuses entirely on "vertex data -> vertex shader -> fragment shader -> texture" rendering.
- The main function of this crate is to give a good set of defaults, and this sticks very closely to wgpu's types and function calls. In fact, you could easily transition from using this crate to using wgpu directly wherever needed.
- This crate is extremely hackable, meaning you can easily edit the crate's code to fit your own needs. This is because:
  - All type fields are public, meaning you have unrestricted access to use and/or replace the underlying wgpu types.
  - This sticks very closely to wgpu's type and function calls (as stated earlier).
  - This is licensed using CC0, meaning this is dedicated to the public domain and you can copy the crate's code into your own codebase without any restrictions or requirements.

### What this crate decides for you:

- Every shader has 4 bindings:
  - Bind group 0 binding 0: the uniforms buffer
  - Bind group 1 binding 0: the texture being rendered (which can / should be a texture atlas)
  - Bind group 1 binding 1: a bilinear sampler
  - Bind group 1 binding 2: a nearest sampler

### Workflow / full walkthrough:

- First, you define:
  - The layout of uniforms (all shaders are given the same uniforms)
  - The layout of vertex types
  - The layout of instance types (optional)
- Then, you load:
  - A window (with any backend you want)
  - A `GpuInstance`
  - The shaders you will use (Glsl is suggested)
  - A buffer for uniforms
  - The textures you will render (each render pass expects one texture input, so atlases are useful here)
  - A depth texture (optional)
  - A pipeline (specifies the shaders, vertex input format, and output texture format that it will use)
  - Vertex data
  - Index data (optional)
  - Instance data (optional)
- Finally, you render:
  - Start a new frame with `simple_gpu::start_frame()` (or use `::get_surface_texture()` and `::start_command_encoder()`)
  - Start a render pass with `simple_gpu::start_3d_render_pass()` (or 2d version)
  - Render a pipeline with `simple_gpu::render()`
  - Finish a render pass with `simple_gpu::finish_render_pass()`
  - Finish the frame with `simple_gpu::finish_frame()` (or use `::submit_gpu_commands()` and `::present_frame()`)
