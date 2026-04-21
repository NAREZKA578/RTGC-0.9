//! Graphics Module for RTGC-0.8
//! Provides rendering, camera, shaders, meshes, textures, and RHI abstraction

pub mod renderer;
pub mod camera;
pub mod shader;
pub mod mesh;
pub mod texture;
pub mod lod_system;
pub mod texture_streaming;
pub mod lighting;
pub mod rhi;
pub mod material;
pub mod particles;
pub mod debug_renderer;
pub mod gl_context;
pub mod dx11_context;
pub mod graphics_context;
pub mod render_command;
pub mod render_queue;
pub mod renderer_rhi;
pub mod renderer_dx11;

pub use renderer::Renderer;
pub use camera::Camera;
pub use shader::Shader;
pub use mesh::{Mesh, MeshHandle};
pub use texture::Texture;
pub use gl_context::GlContext;
pub use dx11_context::Dx11GraphicsContext;
pub use graphics_context::GraphicsContext;
pub use render_command::{RenderCommand, Handle};
pub use render_queue::{RenderQueue, RenderQueueStats};
pub use material::{Material, MaterialManager, MaterialLayers, MaterialParams, TextureQuality, MaterialStats};
pub use renderer_rhi::RendererRhi;
pub use renderer_dx11::Dx11Renderer;
