//! Render Module - новый рендерер на основе RHI
//! 
//! Модульная структура:
//! - commands: определения команд рендеринга
//! - debug: отладочная визуализация
//! - passes: render passes (main, shadow, post)
//! - pipeline_cache: кэш PSO
//! - scene: SceneRenderer для 3D сцены
//! - ui: UIRenderer для интерфейса
//! - renderer: главный Renderer

pub mod commands;
pub mod debug;
pub mod passes;
pub mod pipeline_cache;
pub mod renderer;
pub mod scene;
pub mod ui;

pub use commands::{RenderCommand, UiCommand, RendererConfig};
pub use debug::DebugRenderer;
pub use passes::{MainRenderPass, ShadowRenderPass, PostProcessRenderPass};
pub use pipeline_cache::{PipelineCache, PipelineKey, PipelineCacheStats};
pub use renderer::Renderer;
pub use scene::SceneRenderer;
pub use ui::UIRenderer;

