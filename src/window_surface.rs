use crate::GpuInstance;
use raw_window_handle::{DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, WindowHandle};
use anyhow::{Result, Ok};



/// Everything related to a window's surface
pub struct WindowSurface<'a> {
	/// This is just a handle to the window's surface
	pub wgpu_surface: wgpu::Surface<'a>,
	/// This holds the configuration of the current surface
	pub wgpu_config: wgpu::SurfaceConfiguration,
	/// This is the format that was chosen for the surface
	pub wgpu_format: wgpu::TextureFormat,
	/// This is the list of capabilities that the surface has
	pub wgpu_capabilities: wgpu::SurfaceCapabilities,
}

/// Updates a surface's size to match the window
#[inline]
pub fn reconfigure_window_surface(surface: &mut WindowSurface, gpu_instance: &GpuInstance, new_window_size: (u32, u32)) {
	surface.wgpu_config.width = new_window_size.0;
	surface.wgpu_config.height = new_window_size.1;
	surface.wgpu_surface.configure(&gpu_instance.wgpu_device, &surface.wgpu_config);
}

/// Represents whether 1: a texture was retrieved, 2: no texture was given, or 3: there was an error requiring reconfiguring
pub enum SurfaceTextureResult {
	/// Surface texture was successfully acquired
	Some(wgpu::SurfaceTexture, wgpu::TextureView),
	/// Surface timed out or is occluded (no rendering is needed)
	None,
	/// Surface textured errored, requires `simple_gpu::reconfigure_window_surface()`
	Error,
}

/// Returns the a output texture that can be used to render to the window
pub fn get_surface_texture(surface: &WindowSurface) -> SurfaceTextureResult {
	match surface.wgpu_surface.get_current_texture() {
		wgpu::CurrentSurfaceTexture::Success (frame) => {
			let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
			SurfaceTextureResult::Some(frame, view)
		}
		wgpu::CurrentSurfaceTexture::Timeout
			| wgpu::CurrentSurfaceTexture::Occluded => SurfaceTextureResult::None,
		wgpu::CurrentSurfaceTexture::Outdated
			| wgpu::CurrentSurfaceTexture::Lost
			| wgpu::CurrentSurfaceTexture::Suboptimal (_)
			| wgpu::CurrentSurfaceTexture::Validation => SurfaceTextureResult::Error,
	}
}



/// Similar to `get_window_surface()`, but allows you to mutate the window after creating the surface
#[inline]
pub fn get_window_surface_mut<'a, T: HasDisplayHandle + HasWindowHandle>(
	gpu_instance: &GpuInstance,
	window: &'a mut T,
	window_size: (u32, u32),
	present_mode: wgpu::PresentMode,
) -> (&'a mut T, Result<WindowSurface<'a>>) {
	let surface = (|| { // closure makes error handling easier
		let window: &'a T = unsafe {
			// Safety: this removes the lifetime and immediately adds it back, which allows the `simple_gpu::Surface` to reference the window while still having a mutable window reference. This is safe because the Surface does not actually contain any references to the window, it only has the lifetime to ensure that the window lives long enough
			&*std::ptr::from_ref::<T>(window)
		};
		surface_from_raw_data(gpu_instance, window.window_handle()?, Some(window.display_handle()?), window_size, present_mode)
	})();
	(window, surface)
}

/// Creates a `wgpu::Surface` (and other related structs) using the window.
/// 
/// It should be noted that you cannot mutate the window after calling this, so if that is needed then you should use `get_window_surface_mut()` instead
/// 
/// # Errors
/// 
/// This returns an error if it fails to retrieve the window handles
#[inline]
pub fn get_window_surface<'a, T: HasDisplayHandle + HasWindowHandle>(
	gpu_instance: &GpuInstance,
	window: &'a T,
	window_size: (u32, u32),
	present_mode: wgpu::PresentMode,
) -> Result<WindowSurface<'a>> {
	surface_from_raw_data(gpu_instance, window.window_handle()?, Some(window.display_handle()?), window_size, present_mode)
}



/// Allows you to start rendering a frame by preparing instructions for the gpu
#[must_use]
pub fn start_command_encoder(name: &str, gpu_instance: &GpuInstance) -> wgpu::CommandEncoder {
	gpu_instance.wgpu_device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
		label: Some(name),
	})
}

/// Submits a command encoder to the gpu, which starts the actual rendering of the next frame
pub fn finish_command_encoder(command_encoder: wgpu::CommandEncoder, gpu_instance: &GpuInstance) {
	gpu_instance.wgpu_queue.submit(std::iter::once(command_encoder.finish()));
}

/// Tells the gpu to present the rendered frame when it is ready
pub fn present_frame(surface_tex: wgpu::SurfaceTexture, gpu_instance: &GpuInstance) {
	gpu_instance.wgpu_queue.present(surface_tex);
}



/// Creates a `wgpu::Surface` (and other related structs) using a window's raw handles
/// 
/// # Errors
/// 
/// This returns an error only if `wgpu::Instance::create_surface()` errors
/// 
/// # Panics
/// 
/// This panics if the surface does not support any srgb texture formats
pub fn surface_from_raw_data<'a>(
	gpu_instance: &GpuInstance,
	window_handle: WindowHandle<'a>,
	display_handle: Option<DisplayHandle<'a>>,
	window_size: (u32, u32),
	present_mode: wgpu::PresentMode,
) -> Result<WindowSurface<'a>> {
	
	let surface_target = if let Some(display_handle) = display_handle {
		let sync_handles = SyncWindowDisplayHandle { window_handle, display_handle };
		wgpu::SurfaceTarget::DisplayAndWindow(Box::new(sync_handles))
	} else {
		let sync_handle = SyncWindowHandle { window_handle };
		wgpu::SurfaceTarget::Window(Box::new(sync_handle))
	};
	let surface = gpu_instance.wgpu_instance.create_surface(surface_target)?;
	
	let capabilities = surface.get_capabilities(&gpu_instance.wgpu_adapter);
	let format = *capabilities.formats
		.iter()
		.find(|format| format.is_srgb())
		.expect("srgb-compatible window surface is required");
	
	let config = wgpu::SurfaceConfiguration {
		usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
		format,
		color_space: wgpu::SurfaceColorSpace::Srgb,
		width: window_size.0,
		height: window_size.1,
		present_mode,
		alpha_mode: wgpu::CompositeAlphaMode::Auto,
		desired_maximum_frame_latency: 0,
		view_formats: vec![],
	};
	
	surface.configure(&gpu_instance.wgpu_device, &config);
	
	Ok(WindowSurface {
		wgpu_surface: surface,
		wgpu_config: config,
		wgpu_format: format,
		wgpu_capabilities: capabilities,
	})
}



// implementation stuff:

#[derive(Debug, PartialEq)]
struct SyncWindowDisplayHandle<'a> {
	window_handle: WindowHandle<'a>,
	display_handle: DisplayHandle<'a>,
}

// Note: this is necessary because Wgpu needs the given window handle struct to be Send/Sync
// Safety: SyncWindowDisplayHandle is not send/sync safe in the same way that Arc<*mut T> is not send/sync unless T is send/sync. Whether or not the underlying window and display handles are send/sync is an extremely difficult question to answer, so these unsafe impls are likely not sound. It should be noted that SyncWindowDisplayHandle itself can be sent, referenced, and accessed across threads safely, but send/sync also requires the returned values to be safely usable across threads (which they likely aren't).
unsafe impl Send for SyncWindowDisplayHandle<'_> {}
unsafe impl Sync for SyncWindowDisplayHandle<'_> {}

impl<'a> HasWindowHandle for SyncWindowDisplayHandle<'a> {
	fn window_handle(&self) -> Result<WindowHandle<'a>, HandleError> {
		Result::Ok(self.window_handle)
	}
}

impl<'a> HasDisplayHandle for SyncWindowDisplayHandle<'a> {
	fn display_handle(&self) -> Result<DisplayHandle<'a>, HandleError> {
		Result::Ok(self.display_handle)
	}
}

#[derive(Debug, PartialEq)]
struct SyncWindowHandle<'a> {
	window_handle: WindowHandle<'a>,
}

// Safety: same as with `SyncWindowDisplayHandle`
unsafe impl Send for SyncWindowHandle<'_> {}
unsafe impl Sync for SyncWindowHandle<'_> {}

impl<'a> HasWindowHandle for SyncWindowHandle<'a> {
	fn window_handle(&self) -> Result<WindowHandle<'a>, HandleError> {
		Result::Ok(self.window_handle)
	}
}
