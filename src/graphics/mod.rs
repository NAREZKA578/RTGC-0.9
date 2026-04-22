//! Graphics Module for RTGC-0.8
//! Provides rendering, camera, shaders, meshes, textures, and RHI abstraction

pub mod camera;
pub mod material;
pub mod particles;
pub mod renderer;
pub mod lighting;
pub mod rhi;

pub use camera::Camera;
pub use material::{MaterialManager, TextureQuality};
pub use particles::ParticleSystem;
pub use renderer::{Renderer, RenderCommand, UiCommand, RendererConfig};
