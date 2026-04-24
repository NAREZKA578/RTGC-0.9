//! Renderer module - координирует рендеринг сцены, UI и отладки

pub mod commands;
pub mod renderer;
pub mod scene;
pub mod ui;
pub mod debug;
pub mod passes;
pub mod pipeline_cache;

// Re-export основные типы
pub use commands::{RenderCommand, UiCommand, RenderQueue};
pub use commands::{UI_DEPTH_BACKGROUND, UI_DEPTH_HUD, UI_DEPTH_PROMPT, UI_DEPTH_NOTIFICATIONS, UI_DEPTH_CURSOR};
pub use renderer::{Renderer, RendererConfig};
pub use scene::SceneRenderer;
pub use ui::UIRenderer;
pub use debug::DebugRenderer;
pub use passes::{MainRenderPass, ShadowRenderPass, PostProcessRenderPass};
pub use pipeline_cache::PipelineCache;
pub use scene::SceneRendererStats;

/// GraphicsContext - enum для выбора бэкенда рендеринга
pub enum GraphicsContext {
    Gl(crate::graphics::gl_context::GlContext),
}

impl GraphicsContext {
    pub fn new_gl() -> Self {
        GraphicsContext::Gl(crate::graphics::gl_context::GlContext::new())
    }
}
