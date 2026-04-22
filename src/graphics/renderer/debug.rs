//! Debug Renderer - отладочная визуализация через RHI
//! 
//! Поддерживает рисование линий, AABB, точек, стрелок

use crate::graphics::rhi::{IDevice, ICommandList, ResourceHandle, BufferDescription, BufferType, BufferUsage, ResourceState, VertexFormat, VertexAttribute, InputLayout, PipelineStateObject, PrimitiveTopology, RasterizerState, CullMode, FrontFace, FillMode, DepthState, ColorBlendState, BlendMode, BlendOp};
use nalgebra::{Matrix4, Vector3};
use std::sync::Arc;

/// Вершина для отладочного рендеринга
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DebugVertex {
    pub position: [f32; 3],
    pub color: [f32; 4],
}

impl DebugVertex {
    pub fn new(position: [f32; 3], color: [f32; 4]) -> Self {
        Self { position, color }
    }
    
    pub fn layout() -> InputLayout {
        let attributes = vec![
            VertexAttribute {
                name: "position".to_string(),
                format: VertexFormat::Float32x3,
                offset: 0,
            },
            VertexAttribute {
                name: "color".to_string(),
                format: VertexFormat::Float32x4,
                offset: 12,
            },
        ];
        InputLayout::new(attributes)
    }
}

/// Отладочный рендерер
pub struct DebugRenderer {
    device: Arc<dyn IDevice>,
    vertex_buffer: Option<ResourceHandle>,
    pipeline: Option<ResourceHandle>,
    vertices: Vec<DebugVertex>,
    max_vertices: usize,
}

impl DebugRenderer {
    pub fn new(device: Arc<dyn IDevice>) -> Self {
        const MAX_VERTICES: usize = 65536;
        
        Self {
            device,
            vertex_buffer: None,
            pipeline: None,
            vertices: Vec::with_capacity(MAX_VERTICES),
            max_vertices: MAX_VERTICES,
        }
    }
    
    /// Инициализирует ресурсы (буферы, пайплайны)
    pub fn initialize(&mut self) -> Result<(), String> {
        // Создаём вершинный буфер
        let buffer_desc = BufferDescription {
            buffer_type: BufferType::Vertex,
            size: (self.max_vertices * std::mem::size_of::<DebugVertex>()) as u64,
            usage: BufferUsage::VERTEX_BUFFER | BufferUsage::DYNAMIC,
            initial_state: ResourceState::VertexBuffer,
        };
        
        self.vertex_buffer = Some(
            self.device.create_buffer(&buffer_desc)
                .map_err(|e| format!("Failed to create debug vertex buffer: {:?}", e))?
        );
        
        // Создаём простой шейдер для отладки (будет загружен из файлов позже)
        // Пока заглушка - в реальной реализации нужно загрузить шейдеры
        
        Ok(())
    }
    
    /// Добавляет линию
    pub fn add_line(&mut self, start: Vector3<f32>, end: Vector3<f32>, color: [f32; 4]) {
        if self.vertices.len() + 2 > self.max_vertices {
            tracing::warn!("DebugRenderer: vertex buffer full");
            return;
        }
        
        self.vertices.push(DebugVertex::new(start.into(), color));
        self.vertices.push(DebugVertex::new(end.into(), color));
    }
    
    /// Добавляет AABB (bounding box)
    pub fn add_aabb(&mut self, min: Vector3<f32>, max: Vector3<f32>, color: [f32; 4]) {
        // 12 edges of the box
        let corners = [
            Vector3::new(min.x, min.y, min.z),
            Vector3::new(max.x, min.y, min.z),
            Vector3::new(max.x, max.y, min.z),
            Vector3::new(min.x, max.y, min.z),
            Vector3::new(min.x, min.y, max.z),
            Vector3::new(max.x, min.y, max.z),
            Vector3::new(max.x, max.y, max.z),
            Vector3::new(min.x, max.y, max.z),
        ];
        
        // Bottom face
        self.add_line(corners[0], corners[1], color);
        self.add_line(corners[1], corners[2], color);
        self.add_line(corners[2], corners[3], color);
        self.add_line(corners[3], corners[0], color);
        
        // Top face
        self.add_line(corners[4], corners[5], color);
        self.add_line(corners[5], corners[6], color);
        self.add_line(corners[6], corners[7], color);
        self.add_line(corners[7], corners[4], color);
        
        // Vertical edges
        self.add_line(corners[0], corners[4], color);
        self.add_line(corners[1], corners[5], color);
        self.add_line(corners[2], corners[6], color);
        self.add_line(corners[3], corners[7], color);
    }
    
    /// Добавляет точку
    pub fn add_point(&mut self, position: Vector3<f32>, color: [f32; 4], size: f32) {
        // Рисуем небольшой крестик
        let half = size / 2.0;
        self.add_line(position - Vector3::x() * half, position + Vector3::x() * half, color);
        self.add_line(position - Vector3::y() * half, position + Vector3::y() * half, color);
        self.add_line(position - Vector3::z() * half, position + Vector3::z() * half, color);
    }
    
    /// Добавляет стрелку (направление)
    pub fn add_arrow(&mut self, from: Vector3<f32>, to: Vector3<f32>, color: [f32; 4]) {
        self.add_line(from, to, color);
        
        // Arrow head
        let dir = (to - from).normalize();
        let arrow_size = 0.2;
        
        // Найти перпендикулярные векторы для основания стрелки
        let perp1 = if dir.abs().x < 0.9 {
            dir.cross(&Vector3::x()).normalize()
        } else {
            dir.cross(&Vector3::y()).normalize()
        };
        
        let perp2 = dir.cross(&perp1).normalize();
        
        let tip = to;
        let base = to - dir * arrow_size;
        
        self.add_line(tip, base + perp1 * arrow_size, color);
        self.add_line(tip, base - perp1 * arrow_size, color);
        self.add_line(tip, base + perp2 * arrow_size, color);
        self.add_line(tip, base - perp2 * arrow_size, color);
    }
    
    /// Рендерит все накопленные отладочные примитивы
    pub fn render(&mut self, command_list: &mut dyn ICommandList, view_proj: &Matrix4<f32>) -> Result<(), String> {
        if self.vertices.is_empty() {
            return Ok(());
        }
        
        // Обновляем вершинный буфер
        if let Some(buffer) = self.vertex_buffer {
            let vertex_data: &[u8] = bytemuck::cast_slice(&self.vertices);
            self.device.update_buffer(buffer, 0, vertex_data)
                .map_err(|e| format!("Failed to update debug buffer: {:?}", e))?;
        }
        
        // Устанавливаем пайплайн
        if let Some(pipeline) = self.pipeline {
            command_list.set_pipeline_state(pipeline);
        }
        
        // Биндим вершинный буфер
        if let Some(buffer) = self.vertex_buffer {
            command_list.bind_vertex_buffers(0, &[(buffer, 0)]);
        }
        
        // Рисуем
        let vertex_count = self.vertices.len() as u32;
        command_list.draw(vertex_count, 1, 0, 0);
        
        // Очищаем буфер
        self.vertices.clear();
        
        Ok(())
    }
    
    /// Очищает накопленные примитивы
    pub fn clear(&mut self) {
        self.vertices.clear();
    }
}
