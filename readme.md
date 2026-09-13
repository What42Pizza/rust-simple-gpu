# Simple Gpu

This is a layer over WGPU that is inspired by the simplicity of multimedia libraries like raylib and SDL. The main goal of this crate is to provide a convenient way of rendering custom data with custom shaders.

### Other similar crates:

- [simple-wgpu](https://crates.io/crates/simple-wgpu)
- [easy-gpu](https://crates.io/crates/easy-gpu)
- [easygpu](https://crates.io/crates/easygpu) (only meant for the [Kludgine](https://github.com/khonsulabs/kludgine) engine)

### What sets this apart:

- This focuses entirely on "vertex data -> vertex shader -> fragment shader -> textures" rendering.
- This gives a convenient rendering interface without trying to hide what's underneath.
- This is more of a toolbox, where you can choose which functions you will and won't use.
- This crate is extremely hackable because:
  - All struct fields are public, meaning you still have full control over the state and data.
  - This sticks very closely to wgpu's type and function calls, allowing you to easily work directly with wgpu wherever needed.
  - This is dedicated to the public domain (licensed under CC0), meaning you can copy and tweak this crate's code for your own needs.

### Workflow / full walkthrough:

## See the [example program](examples/basic.rs)

- First, you define:
  - The layout of uniforms (all shaders are given the same uniforms)
  - The layout of vertex types
  - The layout of instance types (optional)
- Then, you load:
  - A window (with any backend you want)
  - A `GpuInstance`
  - The shaders you will use (Glsl is suggested)
  - A buffer for uniforms
  - The textures you will render
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

### A few more features of this crate:

- Can load files directly into textures
- Can pack textures into atlases
- Can dynamically generate mipmaps
- Automatically creates pipeline layouts
- Vertex data buffers can be dereferenced to their `cpu_buffer: Vec<T>` field for convenience

See the full api [here](https://docs.rs/simple-gpu/latest/simple_gpu/all.html)

### What this crate decides for you:

- The bind groups are:
  - Bind group 0:
    - Binding 0: the uniforms buffer
  - All other bind groups:
    - Binding 0: a texture view
    - Binding 1: a texture sampler
- 3D rendering has:
  - Counter-clockwise triangles with back-face culling
  - A 24-bit float depth buffer
- Only 2D textures are used
- Samplers always clamp coordinates
- All triangles are made from triangle lists (this is the `wgpu::PrimitiveTopology`)
- Vertex index buffers use 16-bit indices instead of 32-bit
- Glsl is the default shader language (though there are utility functions for wgsl too)
- The window's surface texture must support an srgb output
- Vertex input buffers can only be updated by the cpu and read by vertex shaders
- A whole lot of other boring details (fragment shaders store their outputs, shader files can only have one entry point, etc)
