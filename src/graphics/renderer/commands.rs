//! Render Commands - определения команд рендеринга
//! 
//! Этот модуль содержит команды для SceneRenderer и UIRenderer

use crate::graphics::rhi::ResourceHandle;
use nalgebra::Matrix4;

/// Команды рендеринга сцены
#[derive(Debug, Clone)]
pub enum RenderCommand {
    /// Отрисовка меша с материалом
    Mesh {
        mesh: ResourceHandle,
        material: ResourceHandle,
        transform: Matrix4<f32>,
    },
    /// Отрисовка меша с инстансингом
    MeshInstanced {
        mesh: ResourceHandle,
        material: ResourceHandle,
        transforms: Vec<Matrix4<f32>>,
    },
    /// Отрисовка линий (для отладки)
    LineList {
        vertices: Vec<[f32; 3]>,
        colors: Vec<[f32; 4]>,
    },
}

/// Команды рендеринга UI
#[derive(Debug, Clone)]
pub enum UiCommand {
    /// Прямоугольник
    Rect {
        position: [f32; 2],
        size: [f32; 2],
        color: [f32; 4],
    },
    /// Прямоугольник с текстурой
    TexturedRect {
        position: [f32; 2],
        size: [f32; 2],
        texture: ResourceHandle,
        uv_rect: Option<[f32; 4]>, // [u0, v0, u1, v1]
        color: [f32; 4],
    },
    /// Текст
    Text {
        text: String,
        position: [f32; 2],
        font_size: f32,
        color: [f32; 4],
    },
    /// Спрайт
    Sprite {
        position: [f32; 2],
        size: [f32; 2],
        texture: ResourceHandle,
        color: [f32; 4],
        flip_x: bool,
        flip_y: bool,
    },
}

/// Конфигурация рендерера
#[derive(Debug, Clone)]
pub struct RendererConfig {
    pub width: u32,
    pub height: u32,
    pub vsync: bool,
    pub debug_mode: bool,
}

impl Default for RendererConfig {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            vsync: true,
            debug_mode: false,
        }
    }
}
