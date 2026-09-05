# Simple Gpu

This is a tiny abstraction over WGPU that is inspired by the simplicity and directness of multimedia libraries like raylib and SDL. The main goal of this crate is to provide a simple and convenient way of rendering custom data with custom shaders.

## See the [example program](examples/basic.rs)

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
  - Start a new frame with `simple_gpu::start_frame()` (or call `::get_surface_texture()` then `::start_command_encoder()`)
  - Start a render pass with `simple_gpu::start_3d_render_pass()` (or 2d version)
  - Render a pipeline with `simple_gpu::render()`
  - Finish a render pass with `simple_gpu::finish_render_pass()`
  - Finish the frame with `simple_gpu::finish_frame()` (or call `::submit_gpu_commands()` then `::present_frame()`)

### What this crate decides for you:

- Every shader has 4 bindings:
  - Bind group 0 binding 0: the uniforms buffer
  - Bind group 1 binding 0: the texture being rendered (which can / should be a texture atlas)
  - Bind group 1 binding 1: a bilinear sampler
  - Bind group 1 binding 2: a nearest sampler
- 3D rendering has:
  - Counter-clockwise triangles with back-face culling
  - A 24-bit float depth buffer
- Only 2D textures will be used
- Samplers always clamp coordinates
- All triangles are made from triangle lists (this is the `wgpu::PrimitiveTopology`)
- Vertex index buffers use 16-bit indices instead of 32-bit
- Glsl is the default shader language (though there are utility functions for wgsl too)
- The window's surface texture must support an srgb output
- Textures can either be updated by the cpu or rendered to by the gpu, but not both
- Vertex input buffers can only be updated by the cpu and read by vertex shaders
- A whole lot of other boring details (color blending is enabled, fragment shaders store their outputs, etc)
