//! Graphics Context - unified abstraction for OpenGL and DirectX 11
//! Switch via config.json: "backend": "opengl" or "backend": "dx11"

use std::fmt::Debug;
use std::sync::Arc;

/// Graphics API type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphicsApi {
    OpenGL,
    DirectX11,
}

impl Default for GraphicsApi {
    fn default() -> Self {
        GraphicsApi::OpenGL
    }
}

impl GraphicsApi {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "dx11" | "directx11" => GraphicsApi::DirectX11,
            _ => GraphicsApi::OpenGL,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            GraphicsApi::OpenGL => "OpenGL",
            GraphicsApi::DirectX11 => "DirectX 11",
        }
    }
}

/// Unified graphics context
pub enum GraphicsContext {
    OpenGL(crate::graphics::GlContext),
    DX11(crate::graphics::dx11_context::Dx11GraphicsContext),
}

impl GraphicsContext {
    /// Create based on config
    pub fn new(
        backend: &str,
        hwnd: Option<isize>,
        width: u32,
        height: u32,
    ) -> Result<Self, String> {
        match GraphicsApi::from_str(backend) {
            GraphicsApi::DirectX11 => {
                if let Some(hwnd) = hwnd {
                    Ok(GraphicsContext::DX11(
                        crate::graphics::dx11_context::Dx11GraphicsContext::new(
                            hwnd, width, height,
                        )?,
                    ))
                } else {
                    Err("DX11 requires hwnd".to_string())
                }
            }
            GraphicsApi::OpenGL => Err("OpenGL needs event_loop - use Engine".to_string()),
        }
    }

    /// Create OpenGL context
    pub fn new_opengl(gl: crate::graphics::GlContext) -> Self {
        GraphicsContext::OpenGL(gl)
    }

    /// Get API type
    pub fn api_type(&self) -> GraphicsApi {
        match self {
            GraphicsContext::OpenGL(_) => GraphicsApi::OpenGL,
            GraphicsContext::DX11(_) => GraphicsApi::DirectX11,
        }
    }

    /// Get size
    pub fn get_size(&self) -> (u32, u32) {
        match self {
            GraphicsContext::OpenGL(c) => (c.width, c.height),
            GraphicsContext::DX11(c) => (c.width, c.height),
        }
    }

    /// Check if initialized
    pub fn is_initialized(&self) -> bool {
        match self {
            GraphicsContext::OpenGL(c) => c.is_initialized(),
            GraphicsContext::DX11(c) => c.width > 0,
        }
    }

    /// Swap buffers / present
    pub fn swap_buffers(&self) -> Result<(), String> {
        match self {
            GraphicsContext::OpenGL(c) => c.swap_buffers().map_err(|e| e.to_string()),
            GraphicsContext::DX11(c) => {
                c.end_frame();
                Ok(())
            }
        }
    }

    /// Get glow context (for OpenGL rendering)
    pub fn get_glow(&self) -> Option<Arc<glow::Context>> {
        match self {
            GraphicsContext::OpenGL(c) => c.gl.clone(),
            GraphicsContext::DX11(_) => None,
        }
    }

    /// Begin frame
    pub fn begin_frame(&self) {
        match self {
            GraphicsContext::OpenGL(_) => {}
            GraphicsContext::DX11(c) => c.begin_frame(),
        }
    }

    /// End frame  
    pub fn end_frame(&self) {
        match self {
            GraphicsContext::OpenGL(c) => {
                let _ = c.swap_buffers();
            }
            GraphicsContext::DX11(c) => c.end_frame(),
        }
    }

    /// Resize
    pub fn resize(&mut self, width: u32, height: u32) {
        match self {
            GraphicsContext::OpenGL(c) => {
                c.width = width;
                c.height = height;
            }
            GraphicsContext::DX11(c) => {
                let _ = c.resize(width, height);
            }
        }
    }

    /// Get projection matrix
    pub fn get_projection_matrix(&self, fov: f32, near: f32, far: f32) -> nalgebra::Matrix4<f32> {
        match self {
            GraphicsContext::OpenGL(c) => {
                let aspect = c.width as f32 / c.height as f32;
                *nalgebra::Perspective3::new(aspect, fov, near, far).as_matrix()
            }
            GraphicsContext::DX11(c) => {
                let aspect = c.width as f32 / c.height as f32;
                *nalgebra::Perspective3::new(aspect, fov, near, far).as_matrix()
            }
        }
    }
}
