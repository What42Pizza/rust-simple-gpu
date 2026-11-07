use crate::{GpuInstance, file_name};
use anyhow::{Context, Ok, Result, bail};
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



/// Loads two glsl shaders (one vertex shader and one fragment shader) from files
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
	let Some(vsh_name) = file_name(vsh_path) else {
		bail!(
			"Failed to get the file name of the path \"{}\"",
			vsh_path.display()
		);
	};
	let Some(fsh_name) = file_name(fsh_path) else {
		bail!(
			"Failed to get the file name of the path \"{}\"",
			fsh_path.display()
		);
	};
	let shaders = make_glsl_shaders(
		vsh_name,
		vertex_shader,
		fsh_name,
		fragment_shader,
		gpu_instance,
		defines,
	);
	Ok(shaders)
}

/// Creates two glsl shaders (one vertex shader and one fragment shader) from source strings
#[inline]
#[cfg(feature = "glsl")]
pub fn make_glsl_shaders(
	vsh_name: impl AsRef<str>,
	vsh_shader: impl AsRef<str>,
	fsh_name: impl AsRef<str>,
	fsh_shader: impl AsRef<str>,
	gpu_instance: &GpuInstance,
	defines: &[(&str, &str)],
) -> Shaders {
	let (vsh_shader, fsh_shader) = (vsh_shader.as_ref(), fsh_shader.as_ref());
	let vertex_shader =
		gpu_instance
			.wgpu_device
			.create_shader_module(wgpu::ShaderModuleDescriptor {
				label: Some(vsh_name.as_ref()),
				source: wgpu::ShaderSource::Glsl {
					shader: vsh_shader.into(),
					stage: wgpu::naga::ShaderStage::Vertex,
					defines,
				},
			});
	let fragment_shader =
		gpu_instance
			.wgpu_device
			.create_shader_module(wgpu::ShaderModuleDescriptor {
				label: Some(fsh_name.as_ref()),
				source: wgpu::ShaderSource::Glsl {
					shader: fsh_shader.into(),
					stage: wgpu::naga::ShaderStage::Fragment,
					defines,
				},
			});
	Shaders::Separate {
		vertex: vertex_shader,
		fragment: fragment_shader,
	}
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
	let Some(name) = file_name(path) else {
		bail!(
			"Failed to get the file name of the path \"{}\"",
			path.display()
		);
	};
	let shaders = make_wgsl_shader(name, shader, gpu_instance);
	Ok(shaders)
}

/// Creates a wgsl shader from a source string, this function is used for both vertex and fragment shaders
#[inline]
#[cfg(feature = "wgsl")]
pub fn make_wgsl_shader(
	name: impl AsRef<str>,
	shader: impl AsRef<str>,
	gpu_instance: &GpuInstance,
) -> Shaders {
	let shader = shader.as_ref();
	let shader = gpu_instance
		.wgpu_device
		.create_shader_module(wgpu::ShaderModuleDescriptor {
			label: Some(name.as_ref()),
			source: wgpu::ShaderSource::Wgsl(shader.into()),
		});
	Shaders::Combined { both: shader }
}
