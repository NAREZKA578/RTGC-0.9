//! UI Renderer - рендеринг интерфейса через RHI
//! 
//! Использует ортографическую проекцию, батчинг спрайтов

use crate::graphics::rhi::{IDevice, ICommandList, ResourceHandle, BufferDescription, BufferType, BufferUsage, ResourceState};
use crate::graphics::renderer::commands::UiCommand;
use nalgebra::Matrix4;
use std::sync::Arc;

/// Вершина для UI рендеринга
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct UiVertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

impl UiVertex {
    pub fn new(position: [f32; 2], uv: [f32; 2], color: [f32; 4]) -> Self {
        Self { position, uv, color }
    }
}

/// UI Renderer
pub struct UIRenderer {
    device: Arc<dyn IDevice>,
    vertex_buffer: Option<ResourceHandle>,
    index_buffer: Option<ResourceHandle>,
    pipeline: Option<ResourceHandle>,
    vertices: Vec<UiVertex>,
    indices: Vec<u16>,
    max_vertices: usize,
    max_indices: usize,
    ortho_matrix: Matrix4<f32>,
}

impl UIRenderer {
    pub fn new(device: Arc<dyn IDevice>) -> Self {
        const MAX_VERTICES: usize = 65536;
        const MAX_INDICES: usize = 65536;
        
        Self {
            device,
            vertex_buffer: None,
            index_buffer: None,
            pipeline: None,
            vertices: Vec::with_capacity(MAX_VERTICES),
            indices: Vec::with_capacity(MAX_INDICES),
            max_vertices: MAX_VERTICES,
            max_indices: MAX_INDICES,
            ortho_matrix: Matrix4::identity(),
        }
    }
    
    /// Инициализирует ресурсы
    pub fn initialize(&mut self) -> Result<(), String> {
        // Создаём вершинный буфер
        let vb_desc = BufferDescription {
            buffer_type: BufferType::Vertex,
            size: (self.max_vertices * std::mem::size_of::<UiVertex>()) as u64,
            usage: BufferUsage::VERTEX_BUFFER | BufferUsage::DYNAMIC,
            initial_state: ResourceState::VertexBuffer,
        };
        
        self.vertex_buffer = Some(
            self.device.create_buffer(&vb_desc)
                .map_err(|e| format!("Failed to create UI vertex buffer: {:?}", e))?
        );
        
        // Создаём индексный буфер
        let ib_desc = BufferDescription {
            buffer_type: BufferType::Index,
            size: (self.max_indices * std::mem::size_of::<u16>()) as u64,
            usage: BufferUsage::INDEX_BUFFER | BufferUsage::DYNAMIC,
            initial_state: ResourceState::IndexBuffer,
        };
        
        self.index_buffer = Some(
            self.device.create_buffer(&ib_desc)
                .map_err(|e| format!("Failed to create UI index buffer: {:?}", e))?
        );
        
        Ok(())
    }
    
    /// Обновляет ортографическую матрицу при изменении размера окна
    pub fn update_ortho_matrix(&mut self, width: u32, height: u32) {
        self.ortho_matrix = Matrix4::new_orthographic(
            0.0, width as f32,
            height as f32, 0.0, // Y inverted for UI
            -1.0, 1.0,
        );
    }
    
    /// Рендерит UI команды
    pub fn render(&mut self, commands: &[UiCommand], screen_size: (u32, u32)) -> Result<(), String> {
        self.vertices.clear();
        self.indices.clear();
        
        // Генерируем геометрию из команд
        for command in commands {
            match command {
                UiCommand::Rect { position, size, color } => {
                    self.add_rect(*position, *size, *color, None);
                }
                UiCommand::TexturedRect { position, size, texture, uv_rect, color } => {
                    let uv = uv_rect.unwrap_or([0.0, 0.0, 1.0, 1.0]);
                    self.add_rect(*position, *size, *color, Some((*texture, uv)));
                }
                UiCommand::Text { text, position, font_size, color } => {
                    // TODO: реализовать рендеринг текста через font.rs
                    tracing::debug!("UI Text: '{}' at {:?}", text, position);
                }
                UiCommand::Sprite { position, size, texture, color, flip_x, flip_y } => {
                    let mut uv = [0.0, 0.0, 1.0, 1.0];
                    if *flip_x {
                        uv[0] = 1.0;
                        uv[2] = 0.0;
                    }
                    if *flip_y {
                        uv[1] = 1.0;
                        uv[3] = 0.0;
                    }
                    self.add_rect(*position, *size, *color, Some((*texture, uv)));
                }
            }
        }
        
        if self.vertices.is_empty() {
            return Ok(());
        }
        
        // TODO: записать команды в command list и отправить на рендер
        // Пока заглушка
        
        Ok(())
    }
    
    /// Добавляет прямоугольник в батч
    fn add_rect(&mut self, pos: [f32; 2], size: [f32; 2], color: [f32; 4], texture: Option<(ResourceHandle, [f32; 4])>) {
        if self.vertices.len() + 4 > self.max_vertices || self.indices.len() + 6 > self.max_indices {
            tracing::warn!("UIRenderer: buffer full, flushing...");
            // В полной реализации здесь был бы flush
            return;
        }
        
        let x0 = pos[0];
        let y0 = pos[1];
        let x1 = pos[0] + size[0];
        let y1 = pos[1] + size[1];
        
        let (uv0, uv1) = if let Some((_, uv)) = texture {
            ([uv[0], uv[1]], [uv[2], uv[3]])
        } else {
            ([0.0, 0.0], [1.0, 1.0])
        };
        
        let start_index = self.vertices.len() as u16;
        
        // 4 vertices
        self.vertices.push(UiVertex::new([x0, y0], uv0, color)); // top-left
        self.vertices.push(UiVertex::new([x1, y0], [uv1[0], uv0[1]], color)); // top-right
        self.vertices.push(UiVertex::new([x1, y1], uv1, color)); // bottom-right
        self.vertices.push(UiVertex::new([x0, y1], [uv0[0], uv1[1]], color)); // bottom-left
        
        // 2 triangles (6 indices)
        self.indices.extend_from_slice(&[
            start_index, start_index + 1, start_index + 2,
            start_index, start_index + 2, start_index + 3,
        ]);
    }
    
    /// Очищает накопленные данные
    pub fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
    }
}
