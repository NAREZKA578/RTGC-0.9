//! UI Renderer - рендеринг интерфейса через RHI
//! 
//! Использует ортографическую проекцию, батчинг спрайтов

use crate::graphics::rhi::{
    IDevice, ICommandList, ResourceHandle, BufferDesc, BufferType, BufferUsage,
    ShaderDescription, ShaderStage, InputLayout, VertexAttribute, VertexFormat,
    PrimitiveTopology, RasterizerState, DepthState, ColorBlendState, BlendOp,
    BlendMode, PipelineStateObject, CompareFunc, CullMode, FrontFace, FillMode,
    StencilState,
};
use crate::graphics::renderer::commands::UiCommand;
use crate::graphics::font::FontAtlas;
use nalgebra::Matrix4;
use std::sync::Arc;
use tracing;

/// Вершина для UI рендеринга: позиция (x, y), UV, цвет (r, g, b, a)
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
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
    uniform_buffer: Option<ResourceHandle>,
    vertices: Vec<UiVertex>,
    indices: Vec<u16>,
    max_vertices: usize,
    max_indices: usize,
    ortho_matrix: Matrix4<f32>,
    width: u32,
    height: u32,
    /// Шрифт для рендеринга текста (опционально)
    font: Option<Arc<FontAtlas>>,
}

impl UIRenderer {
    pub fn new(device: Arc<dyn IDevice>, width: u32, height: u32) -> Self {
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
            width,
            height,
            font: None,
        }
    }
    
    /// Установить шрифт для рендеринга текста
    pub fn set_font(&mut self, font: Arc<FontAtlas>) {
        // Если шрифт ещё не имеет текстуры, нужно создать её
        // Это должно быть сделано до вызова set_font
        self.font = Some(font);
    }
    
    /// Инициализирует ресурсы (шейдеры, PSO, буферы)
    pub fn initialize(&mut self) -> Result<(), String> {
        // 1. Загрузка шейдеров
        let vs_source = std::fs::read_to_string("shaders/ui.vert")
            .unwrap_or_else(|_| include_str!("../../../shaders/ui.vert").to_string());
        
        let fs_source = std::fs::read_to_string("shaders/ui.frag")
            .unwrap_or_else(|_| include_str!("../../../shaders/ui.frag").to_string());

        let vs_desc = ShaderDescription {
            stage: ShaderStage::Vertex,
            source: vs_source.into_bytes(),
            entry_point: "main".to_string(),
        };
        
        let fs_desc = ShaderDescription {
            stage: ShaderStage::Fragment,
            source: fs_source.into_bytes(),
            entry_point: "main".to_string(),
        };

        let vertex_shader = self.device.create_shader(&vs_desc)
            .map_err(|e| format!("Failed to create UI vertex shader: {:?}", e))?;
        
        let fragment_shader = self.device.create_shader(&fs_desc)
            .map_err(|e| format!("Failed to create UI fragment shader: {:?}", e))?;

        // 2. Создание Input Layout
        // Структура UiVertex: position(2 floats) + uv(2 floats) + color(4 floats) = 8 floats = 32 bytes
        let input_layout = InputLayout {
            attributes: vec![
                VertexAttribute { name: "aPos".to_string(), format: VertexFormat::Float32x2, offset: 0, location: 0 },
                VertexAttribute { name: "aUV".to_string(), format: VertexFormat::Float32x2, offset: 8, location: 1 },
                VertexAttribute { name: "aColor".to_string(), format: VertexFormat::Float32x4, offset: 16, location: 2 },
            ],
            stride: 32,
        };

        // 3. Настройка блендинга для прозрачности UI
        let blend_state = ColorBlendState {
            enabled: true,
            src_color_blend: BlendMode::SrcAlpha,
            dst_color_blend: BlendMode::OneMinusSrcAlpha,
            color_blend_op: BlendOp::Add,
            src_alpha_blend: BlendMode::One,
            dst_alpha_blend: BlendMode::Zero,
            alpha_blend_op: BlendOp::Add,
            write_mask: 0xF, // Enable all channels (RGBA)
        };

        // 4. Создание PSO
        let pso_desc = PipelineStateObject {
            vertex_shader,
            fragment_shader: Some(fragment_shader),
            compute_shader: None,
            input_layout,
            color_blend_states: vec![blend_state],
            depth_state: DepthState {
                enabled: false,
                write_enabled: false,
                compare_func: CompareFunc::Always,
            },
            stencil_state: StencilState::default(),
            rasterizer_state: RasterizerState {
                cull_mode: CullMode::None,
                front_face: FrontFace::CounterClockwise,
                fill_mode: FillMode::Solid,
                polygon_offset_factor: 0.0,
                polygon_offset_units: 0.0,
            },
            primitive_topology: PrimitiveTopology::TriangleList,
            sample_count: 1,
        };

        self.pipeline = Some(self.device.create_pipeline_state(&pso_desc)
            .map_err(|e| format!("Failed to create UI pipeline: {:?}", e))?);

        // 5. Создание uniform буфера для матрицы проекции (64 байта для Mat4)
        let ub_desc = BufferDesc {
            buffer_type: BufferType::Uniform,
            size: 64, // 4x4 matrix = 16 floats = 64 bytes
            usage: BufferUsage::CONSTANT_BUFFER,
            initial_data: None,
        };
        
        self.uniform_buffer = Some(
            self.device.create_buffer(&ub_desc)
                .map_err(|e| format!("Failed to create UI uniform buffer: {:?}", e))?
        );

        // 6. Создание вершинного буфера
        let vb_desc = BufferDesc {
            buffer_type: BufferType::Vertex,
            size: (self.max_vertices * std::mem::size_of::<UiVertex>()) as u64,
            usage: BufferUsage::VERTEX_BUFFER,
            initial_data: None,
        };
        
        self.vertex_buffer = Some(
            self.device.create_buffer(&vb_desc)
                .map_err(|e| format!("Failed to create UI vertex buffer: {:?}", e))?
        );
        
        // 7. Создание индексного буфера
        let ib_desc = BufferDesc {
            buffer_type: BufferType::Index,
            size: (self.max_indices * std::mem::size_of::<u16>()) as u64,
            usage: BufferUsage::INDEX_BUFFER,
            initial_data: None,
        };
        
        self.index_buffer = Some(
            self.device.create_buffer(&ib_desc)
                .map_err(|e| format!("Failed to create UI index buffer: {:?}", e))?
        );
        
        // 8. Обновление ортографической матрицы и запись в uniform буфер
        self.update_ortho_matrix(self.width, self.height);
        
        Ok(())
    }
    
    /// Обновляет ортографическую матрицу при изменении размера окна и записывает в uniform буфер
    pub fn update_ortho_matrix(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        // Ортографическая проекция: Left=0, Right=width, Bottom=height, Top=0, Near=-1, Far=1
        // Y инвертирован, чтобы (0,0) был в левом верхнем углу
        self.ortho_matrix = Matrix4::new_orthographic(
            0.0, width as f32,
            height as f32, 0.0,
            -1.0, 1.0,
        );
        
        // Записываем матрицу в uniform буфер
        if let Some(ub) = self.uniform_buffer {
            let matrix_data = bytemuck::bytes_of(&self.ortho_matrix);
            if let Err(e) = self.device.update_buffer(ub, 0, matrix_data) {
                tracing::error!("Failed to update UI uniform buffer: {:?}", e);
            }
        }
    }
    
    /// Рендерит UI команды через command list
    pub fn render(&mut self, commands: &[UiCommand], cmd_list: &mut dyn ICommandList) -> Result<(), String> {
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
                    if let Some(ref font) = self.font {
                        self.add_text(text, *position, *font_size, *color, font);
                    } else {
                        tracing::warn!("UI Text requested but no font set: '{}'", text);
                    }
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
        
        // Обновляем вершинный буфер
        let vb = self.vertex_buffer.ok_or("UI vertex buffer not initialized")?;
        let vertex_data = bytemuck::cast_slice(&self.vertices);
        self.device.update_buffer(vb, 0, vertex_data)
            .map_err(|e| format!("Failed to update UI vertex buffer: {:?}", e))?;
        
        // Обновляем индексный буфер
        let ib = self.index_buffer.ok_or("UI index buffer not initialized")?;
        let index_data = bytemuck::cast_slice(&self.indices);
        self.device.update_buffer(ib, 0, index_data)
            .map_err(|e| format!("Failed to update UI index buffer: {:?}", e))?;
        
        // Устанавливаем pipeline
        let pipeline = self.pipeline.ok_or("UI pipeline not initialized")?;
        cmd_list.set_pipeline_state(pipeline);
        
        // Привязываем буферы
        cmd_list.bind_vertex_buffers(0, &[(vb, 0)]);
        cmd_list.bind_index_buffer(ib, 0);
        
        // Привязываем uniform буфер с матрицей проекции (binding 0 для вершинного шейдера)
        if let Some(ub) = self.uniform_buffer {
            cmd_list.bind_constant_buffer(crate::graphics::rhi::ShaderStage::Vertex, 0, ub);
        }
        
        // Выполняем draw call
        cmd_list.draw_indexed(self.indices.len() as u32, 1, 0, 0, 0);
        
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
    
    /// Добавляет текст в батч (генерирует прямоугольники для каждого глифа)
    fn add_text(&mut self, text: &str, pos: [f32; 2], font_size: f32, color: [f32; 4], font: &FontAtlas) {
        let mut cursor_x = pos[0];
        let cursor_y = pos[1];
        
        // Масштабирование относительно размера шрифта в атласе
        let scale = font_size / font.pixel_height;
        
        for ch in text.chars() {
            if ch == '\n' {
                cursor_x = pos[0];
                cursor_y += font_size;
                continue;
            }
            
            if ch == ' ' {
                // Пропускаем пробел, только двигаем курсор
                if let Some(glyph_data) = font.get_glyph(ch) {
                    cursor_x += glyph_data.advance * scale;
                }
                continue;
            }
            
            if let Some(glyph_data) = font.get_glyph(ch) {
                let width = glyph_data.uv_rect[2] * font.atlas_width as f32 * scale;
                let height = glyph_data.uv_rect[3] * font.atlas_height as f32 * scale;
                
                // Позиция глифа с учётом смещения
                let glyph_x = cursor_x + glyph_data.offset[0] * scale;
                let glyph_y = cursor_y + glyph_data.offset[1] * scale;
                
                // UV координаты из атласа
                let uv = glyph_data.uv_rect;
                
                // Добавляем прямоугольник для глифа
                self.add_rect(
                    [glyph_x, glyph_y - height], // Инвертируем Y для правильного отображения
                    [width, height],
                    color,
                    Some((font.texture.unwrap(), uv)),
                );
                
                cursor_x += glyph_data.advance * scale;
            }
        }
    }
    
    /// Очищает накопленные данные
    pub fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
    }
}
