use crate::{GpuInstance, file_name};
use anyhow::{Context, Ok, Result};
use std::{fs, path::Path};



/// Loads a glsl vertex shader from a file
///
/// # Errors
///
/// This only errors if it fails to read the file
#[cfg(feature = "glsl")]
pub fn load_glsl_vertex_shader(
	path: &Path,
	gpu_instance: &GpuInstance,
	defines: &[(&str, &str)],
) -> Result<wgpu::ShaderModule> {
	let shader = fs::read_to_string(path)
		.with_context(|| format!("Failed to read file {}", path.display()))?;
	let shader = gpu_instance
		.wgpu_device
		.create_shader_module(wgpu::ShaderModuleDescriptor {
			label: file_name(path).as_deref(),
			source: wgpu::ShaderSource::Glsl {
				shader: shader.into(),
				stage: wgpu::naga::ShaderStage::Vertex,
				defines,
			},
		});
	Ok(shader)
}

/// Loads a glsl vertex shader from a file
///
/// # Errors
///
/// This only errors if it fails to read the file
#[cfg(feature = "glsl")]
pub fn load_glsl_fragment_shader(
	path: &Path,
	gpu_instance: &GpuInstance,
	defines: &[(&str, &str)],
) -> Result<wgpu::ShaderModule> {
	let shader = fs::read_to_string(path)
		.with_context(|| format!("Failed to read file {}", path.display()))?;
	let shader = gpu_instance
		.wgpu_device
		.create_shader_module(wgpu::ShaderModuleDescriptor {
			label: file_name(path).as_deref(),
			source: wgpu::ShaderSource::Glsl {
				shader: shader.into(),
				stage: wgpu::naga::ShaderStage::Fragment,
				defines,
			},
		});
	Ok(shader)
}



/// Loads a wgsl shader from a file, this function is used for both vertex and fragment shaders
///
/// # Errors
///
/// This only errors if it fails to read the file
#[cfg(feature = "wgsl")]
pub fn load_wgsl_shader(path: &Path, gpu_instance: &GpuInstance) -> Result<wgpu::ShaderModule> {
	let shader = fs::read_to_string(path)
		.with_context(|| format!("Failed to read file {}", path.display()))?;
	let shader = gpu_instance
		.wgpu_device
		.create_shader_module(wgpu::ShaderModuleDescriptor {
			label: file_name(path).as_deref(),
			source: wgpu::ShaderSource::Wgsl(shader.into()),
		});
	Ok(shader)
}
