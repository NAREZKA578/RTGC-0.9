//! Graphics Module for RTGC-0.8
//! Provides rendering, camera, shaders, meshes, textures, and RHI abstraction

pub mod camera;
pub mod material;
pub mod particles;
pub mod renderer;
pub mod lighting;
pub mod rhi;
pub mod gl_context;
pub mod shader;
pub mod mesh;

pub use camera::Camera;
pub use material::{MaterialManager, TextureQuality};
pub use particles::ParticleSystem;
pub use renderer::{Renderer, RenderCommand, UiCommand, RendererConfig};
pub use gl_context::GlContext;
pub use shader::{load_shader_from_file, load_vertex_shader, load_fragment_shader};
pub use mesh::{Mesh, SimpleVertex};

/// Универсальный графический контекст
pub enum GraphicsContext {
    OpenGL(GlContext),
    // DX11(dx11_context::Dx11GraphicsContext), // Закомментировано до реализации
}

impl GraphicsContext {
    /// Создать OpenGL контекст
    pub fn new_opengl(ctx: GlContext) -> Self {
        Self::OpenGL(ctx)
    }
    
    /// Получить GL контекст если это OpenGL
    pub fn as_gl(&self) -> Option<&GlContext> {
        match self {
            Self::OpenGL(ctx) => Some(ctx),
            _ => None,
        }
    }
    
    /// Получить GL контекст если это OpenGL (mutable)
    pub fn as_gl_mut(&mut self) -> Option<&mut GlContext> {
        match self {
            Self::OpenGL(ctx) => Some(ctx),
            _ => None,
        }
    }
}
