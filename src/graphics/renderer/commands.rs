//! Render Commands - определения команд рендеринга
//! 
//! Этот модуль содержит команды для SceneRenderer и UIRenderer

use crate::graphics::rhi::ResourceHandle;
use nalgebra::Matrix4;

/// UI Depth constants for sorting
pub const UI_DEPTH_BACKGROUND: u32 = 0;
pub const UI_DEPTH_HUD: u32 = 100;
pub const UI_DEPTH_PROMPT: u32 = 200;
pub const UI_DEPTH_NOTIFICATIONS: u32 = 300;
pub const UI_DEPTH_CURSOR: u32 = 1000;

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
    /// Отрисовка чанка террейна
    TerrainChunk {
        chunk_id: (i32, i32),
        mesh: ResourceHandle,
        material: ResourceHandle,
        transform: Matrix4<f32>,
        lod: u32,
    },
    /// Небо (скайбокс или процедурное)
    Skybox {
        texture: Option<ResourceHandle>,
        sun_direction: [f32; 3],
    },
    /// Солнце (визуальное представление)
    Sun {
        direction: [f32; 3],
        angular_radius: f32,
        color: [f32; 3],
    },
    /// Отрисовка линий (для отладки)
    LineList {
        vertices: Vec<[f32; 3]>,
        colors: Vec<[f32; 4]>,
    },
    /// UI элемент (прямоугольник)
    UIElement {
        rect: [f32; 4], // x, y, width, height
        texture: Option<ResourceHandle>,
        color: [f32; 4],
        depth: u32,
        sort_key: u32,
    },
    /// UI текст
    UIText {
        text: String,
        position: [f32; 2],
        font_size: f32,
        color: [f32; 4],
        depth: u32,
        sort_key: u32,
    },
}

/// RenderQueue - очередь команд рендеринга
#[derive(Debug, Default)]
pub struct RenderQueue {
    commands: Vec<RenderCommand>,
}

impl RenderQueue {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }
    
    pub fn submit(&mut self, command: RenderCommand) {
        self.commands.push(command);
    }
    
    pub fn drain(&mut self) -> Vec<RenderCommand> {
        std::mem::take(&mut self.commands)
    }
    
    pub fn clear(&mut self) {
        self.commands.clear();
    }
    
    pub fn len(&self) -> usize {
        self.commands.len()
    }
    
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
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
