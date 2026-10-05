use crate::{GpuInstance, file_name};
use anyhow::{Context, Ok, Result};
use std::{fs, path::Path};



/// Represents a vertex shader and a fragment shader, which can be either one or two [`wgpu::ShaderModule`] values
#[derive(Debug, Clone)]
pub enum Shaders {
	/// Holds two separate shaders
	Separate {
		/// Vertex shader
		vertex: wgpu::ShaderModule,
		/// Fragment shader
		fragment: wgpu::ShaderModule,
	},
	/// Holds both the shaders (vertex and fragment) in one [`wgpu::ShaderModule`]
	Combined {
		/// Holds both the shaders (vertex and fragment) in one [`wgpu::ShaderModule`]
		both: wgpu::ShaderModule,
	},
}

impl Shaders {
	/// Gets the [`wgpu::ShaderModule`] that holds the vertex shader
	#[inline]
	#[must_use]
	pub const fn vertex(&self) -> &wgpu::ShaderModule {
		match self {
			Self::Separate {
				vertex,
				fragment: _,
			} => vertex,
			Self::Combined { both } => both,
		}
	}
	/// Gets the [`wgpu::ShaderModule`] that holds the fragment shader
	#[inline]
	#[must_use]
	pub const fn fragment(&self) -> &wgpu::ShaderModule {
		match self {
			Self::Separate {
				vertex: _,
				fragment,
			} => fragment,
			Self::Combined { both } => both,
		}
	}
}



/// Loads a two glsl shaders (one vertex shader and one fragment shader) from files
///
/// # Errors
///
/// This only errors if the files cannot be read.
#[inline]
#[cfg(feature = "glsl")]
pub fn load_glsl_shaders(
	vsh_path: impl AsRef<Path>,
	fsh_path: impl AsRef<Path>,
	gpu_instance: &GpuInstance,
	defines: &[(&str, &str)],
) -> Result<Shaders> {
	let (vsh_path, fsh_path) = (vsh_path.as_ref(), fsh_path.as_ref());
	let vertex_shader = fs::read_to_string(vsh_path)
		.with_context(|| format!("Failed to read file {}", vsh_path.display()))?;
	let fragment_shader = fs::read_to_string(fsh_path)
		.with_context(|| format!("Failed to read file {}", fsh_path.display()))?;
	let vertex_shader =
		gpu_instance
			.wgpu_device
			.create_shader_module(wgpu::ShaderModuleDescriptor {
				label: file_name(vsh_path).as_deref(),
				source: wgpu::ShaderSource::Glsl {
					shader: vertex_shader.into(),
					stage: wgpu::naga::ShaderStage::Vertex,
					defines,
				},
			});
	let fragment_shader =
		gpu_instance
			.wgpu_device
			.create_shader_module(wgpu::ShaderModuleDescriptor {
				label: file_name(fsh_path).as_deref(),
				source: wgpu::ShaderSource::Glsl {
					shader: fragment_shader.into(),
					stage: wgpu::naga::ShaderStage::Fragment,
					defines,
				},
			});
	Ok(Shaders::Separate {
		vertex: vertex_shader,
		fragment: fragment_shader,
	})
}



/// Loads a wgsl shader from a file, this function is used for both vertex and fragment shaders
///
/// # Errors
///
/// This only errors if it fails to read the file.
#[inline]
#[cfg(feature = "wgsl")]
pub fn load_wgsl_shader(path: impl AsRef<Path>, gpu_instance: &GpuInstance) -> Result<Shaders> {
	let path = path.as_ref();
	let shader = fs::read_to_string(path)
		.with_context(|| format!("Failed to read file {}", path.display()))?;
	let shader = gpu_instance
		.wgpu_device
		.create_shader_module(wgpu::ShaderModuleDescriptor {
			label: file_name(path).as_deref(),
			source: wgpu::ShaderSource::Wgsl(shader.into()),
		});
	Ok(Shaders::Combined { both: shader })
}
