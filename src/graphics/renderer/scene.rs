//! Scene Renderer - рендеринг 3D сцены через RHI
//! 
//! Владеет постоянными буферами (view_proj, light), сортирует команды по материалам

use crate::graphics::rhi::{IDevice, ResourceHandle, BufferDescription, BufferType, BufferUsage, ResourceState, CommandListGuard};
use crate::graphics::renderer::commands::RenderCommand;
use crate::graphics::camera::Camera;
use nalgebra::{Matrix4, Vector3};
use std::sync::Arc;
use bytemuck;
use tracing;

/// Константный буфер для камеры
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CameraBuffer {
    pub view_proj: Matrix4<f32>,
    pub view: Matrix4<f32>,
    pub proj: Matrix4<f32>,
    pub camera_position: [f32; 4],
}

unsafe impl bytemuck::NoUninit for CameraBuffer {}

impl Default for CameraBuffer {
    fn default() -> Self {
        Self {
            view_proj: Matrix4::identity(),
            view: Matrix4::identity(),
            proj: Matrix4::identity(),
            camera_position: [0.0, 0.0, 0.0, 1.0],
        }
    }
}

/// Константный буфер для освещения
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LightBuffer {
    pub sun_direction: [f32; 4],
    pub sun_color: [f32; 4],
    pub ambient_color: [f32; 4],
    pub num_lights: u32,
    pub _padding: [u32; 3],
}

unsafe impl bytemuck::NoUninit for LightBuffer {}

impl Default for LightBuffer {
    fn default() -> Self {
        Self {
            sun_direction: [0.0, -1.0, 0.0, 0.0],
            sun_color: [1.0, 1.0, 0.9, 1.0],
            ambient_color: [0.1, 0.1, 0.15, 1.0],
            num_lights: 0,
            _padding: [0; 3],
        }
    }
}

/// Константный буфер для модели (трансформация объекта)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ModelBuffer {
    pub model: Matrix4<f32>,
    pub normal_matrix: Matrix4<f32>,
    pub material_params: [f32; 4], // roughness, metallic, padding, padding
}

impl Default for ModelBuffer {
    fn default() -> Self {
        Self {
            model: Matrix4::identity(),
            normal_matrix: Matrix4::identity(),
            material_params: [0.5, 0.0, 0.0, 0.0],
        }
    }
}

/// Данные для сортировки команд рендеринга
struct RenderSortKey {
    material: ResourceHandle,
    distance_sq: f32,
    command_index: usize,
}

/// Scene Renderer
pub struct SceneRenderer {
    device: Arc<dyn IDevice>,
    camera_buffer: Option<ResourceHandle>,
    light_buffer: Option<ResourceHandle>,
    pipeline_cache: std::collections::HashMap<String, ResourceHandle>,
    camera_data: CameraBuffer,
    light_data: LightBuffer,
    
    // Буфер для команд рендеринга
    pub(crate) command_buffer: Vec<RenderCommand>,
    
    // Статистика
    stats: SceneRendererStats,
}

/// Статистика рендерера сцены
#[derive(Debug, Clone, Default)]
pub struct SceneRendererStats {
    pub draw_calls: u32,
    pub triangle_count: u32,
    pub terrain_chunks_rendered: u32,
    pub skybox_rendered: bool,
}

impl SceneRenderer {
    pub fn new(device: Arc<dyn IDevice>) -> Self {
        Self {
            device,
            camera_buffer: None,
            light_buffer: None,
            pipeline_cache: std::collections::HashMap::new(),
            camera_data: CameraBuffer::default(),
            light_data: LightBuffer::default(),
            command_buffer: Vec::with_capacity(256),
            stats: SceneRendererStats::default(),
        }
    }
    
    /// Инициализирует константные буферы
    pub fn initialize(&mut self) -> Result<(), String> {
        // Создаём буфер камеры
        let cb_desc = BufferDescription {
            buffer_type: BufferType::Constant,
            size: std::mem::size_of::<CameraBuffer>() as u64,
            usage: BufferUsage::CONSTANT_BUFFER | BufferUsage::DYNAMIC,
            initial_state: ResourceState::ConstantBuffer,
            initial_data: None,
        };
        
        self.camera_buffer = Some(
            self.device.create_buffer(&cb_desc)
                .map_err(|e| format!("Failed to create camera buffer: {:?}", e))?
        );
        
        // Создаём буфер освещения
        let lb_desc = BufferDescription {
            buffer_type: BufferType::Constant,
            size: std::mem::size_of::<LightBuffer>() as u64,
            usage: BufferUsage::CONSTANT_BUFFER | BufferUsage::DYNAMIC,
            initial_state: ResourceState::ConstantBuffer,
            initial_data: None,
        };
        
        self.light_buffer = Some(
            self.device.create_buffer(&lb_desc)
                .map_err(|e| format!("Failed to create light buffer: {:?}", e))?
        );
        
        Ok(())
    }
    
    /// Очищает буфер команд перед новым кадром
    pub fn clear_commands(&mut self) {
        self.command_buffer.clear();
        self.stats = SceneRendererStats::default();
    }
    
    /// Добавляет команду в буфер
    pub fn add_command(&mut self, command: RenderCommand) {
        self.command_buffer.push(command);
    }
    
    /// Обновляет данные камеры
    pub fn update_camera(&mut self, camera: &Camera) {
        self.camera_data.view = camera.view_matrix();
        self.camera_data.proj = camera.proj_matrix();
        self.camera_data.view_proj = camera.view_proj_matrix();
        self.camera_data.camera_position = [
            camera.position.x,
            camera.position.y,
            camera.position.z,
            1.0,
        ];
    }
    
    /// Устанавливает параметры освещения
    pub fn set_sun_direction(&mut self, direction: nalgebra::Vector3<f32>) {
        self.light_data.sun_direction = [direction.x, direction.y, direction.z, 0.0];
    }
    
    /// Устанавливает параметры солнца
    pub fn set_sun_params(&mut self, color: [f32; 3], ambient: [f32; 3]) {
        self.light_data.sun_color = [color[0], color[1], color[2], 1.0];
        self.light_data.ambient_color = [ambient[0], ambient[1], ambient[2], 1.0];
    }
    
    /// Вычисляет плоскости фрустума из матрицы view_proj
    pub fn compute_frustum_planes(view_proj: &Matrix4<f32>) -> [[f32; 4]; 6] {
        let m = view_proj.as_slice();
        
        fn add_rows(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
            [a[0] + b[0], a[1] + b[1], a[2] + b[2], a[3] + b[3]]
        }
        fn sub_rows(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
            [a[0] - b[0], a[1] - b[1], a[2] - b[2], a[3] - b[3]]
        }
        
        let row = |i: usize| -> [f32; 4] { 
            let idx = i * 4;
            [m[idx], m[idx + 1], m[idx + 2], m[idx + 3]]
        };
        
        [
            // Left plane
            add_rows(row(3), row(0)),
            // Right plane
            sub_rows(row(3), row(0)),
            // Bottom plane
            add_rows(row(3), row(1)),
            // Top plane
            sub_rows(row(3), row(1)),
            // Near plane
            add_rows(row(3), row(2)),
            // Far plane
            sub_rows(row(3), row(2)),
        ]
    }
    
    /// Проверяет пересечение сферы с фрустумом
    pub fn sphere_in_frustum(center: &Vector3<f32>, radius: f32, planes: &[[f32; 4]; 6]) -> bool {
        for plane in planes.iter() {
            let distance = plane[0] * center.x + plane[1] * center.y + plane[2] * center.z + plane[3];
            if distance < -radius {
                return false;
            }
        }
        true
    }
    
    /// Сортирует команды для оптимального рендеринга
    fn sort_commands(&mut self, camera_pos: &Vector3<f32>) {
        // Разделяем на непрозрачные и прозрачные
        // Сортируем непрозрачные по материалу и расстоянию (front-to-back)
        // Сортируем прозрачные по расстоянию (back-to-front)
        
        // Пока простая реализация - сортировка по материалу
        self.command_buffer.sort_by(|a, b| {
            let mat_a = match a {
            RenderCommand::Mesh { material: ma, .. } => ma.0,
            RenderCommand::TerrainChunk { material: ma, .. } => ma.0,
            _ => u64::MAX,
        };
        let mat_b = match b {
            RenderCommand::Mesh { material: mb, .. } => mb.0,
            RenderCommand::TerrainChunk { material: mb, .. } => mb.0,
            _ => u64::MAX,
        };
        
        mat_a.cmp(&mat_b)
        });
    }
    
    /// Рендерит сцену через command list
    pub fn render(&mut self, camera: &Camera, commands: &[RenderCommand], cmd_list: &CommandListGuard) -> Result<(), String> {
        // Обновляем камеру
        self.update_camera(camera);
        
        // Обновляем буфер камеры
        if let Some(buffer) = self.camera_buffer {
            let data: &[u8] = bytemuck::bytes_of(&self.camera_data);
            self.device.update_buffer(buffer, 0, data)
                .map_err(|e| format!("Failed to update camera buffer: {:?}", e))?;
        }
        
        // Обновляем буфер освещения
        if let Some(buffer) = self.light_buffer {
            let data: &[u8] = bytemuck::bytes_of(&self.light_data);
            self.device.update_buffer(buffer, 0, data)
                .map_err(|e| format!("Failed to update light buffer: {:?}", e))?;
        }
        
        // Вычисляем плоскости фрустума для culling
        let frustum_planes = Self::compute_frustum_planes(&self.camera_data.view_proj);
        let camera_pos = camera.position;
        
        // Собираем команды с frustum culling
        let mut render_commands = Vec::with_capacity(commands.len());
        
        for command in commands {
            match command {
                RenderCommand::TerrainChunk { chunk_id, mesh, material, transform, lod } => {
                    // Простая проверка видимости чанка
                    let chunk_world_pos = Vector3::new(
                        chunk_id.0 as f32 * 64.0 + 32.0,
                        0.0,
                        chunk_id.1 as f32 * 64.0 + 32.0,
                    );
                    let bounding_radius = 64.0; // Примерный радиус чанка
                    
                    if Self::sphere_in_frustum(&chunk_world_pos, bounding_radius, &frustum_planes) {
                        render_commands.push(command.clone());
                        self.stats.terrain_chunks_rendered += 1;
                    }
                }
                RenderCommand::Skybox { .. } => {
                    // Скайбокс всегда рендерится
                    render_commands.push(command.clone());
                    self.stats.skybox_rendered = true;
                }
                RenderCommand::Sun { .. } => {
                    // Солнце всегда рендерится
                    render_commands.push(command.clone());
                }
                RenderCommand::Mesh { mesh, material, transform } => {
                    // TODO: frustum culling для мешей
                    render_commands.push(command.clone());
                }
                RenderCommand::MeshInstanced { mesh, material, transforms } => {
                    render_commands.push(command.clone());
                }
                RenderCommand::LineList { vertices, colors } => {
                    render_commands.push(command.clone());
                }
                RenderCommand::MeshDeform { .. } => {
                    // Деформации меша обрабатываются на этапе подготовки данных
                    render_commands.push(command.clone());
                }
            }
        }
        
        // Сортируем команды
        self.sort_commands(&camera_pos);
        
        // Рендерим каждую команду
        for command in &render_commands {
            match command {
                RenderCommand::Mesh { mesh, material, transform } => {
                    // TODO: установить константный буфер с трансформацией
                    // TODO: забиндить материал и меш
                    // TODO: вызвать draw
                    tracing::debug!("Rendering mesh with material {:?}", material);
                    self.stats.draw_calls += 1;
                }
                RenderCommand::MeshInstanced { mesh, material, transforms } => {
                    // TODO: инстансированный рендеринг
                    tracing::debug!("Rendering {} instances with material {:?}", transforms.len(), material);
                    self.stats.draw_calls += 1;
                }
                RenderCommand::TerrainChunk { chunk_id, mesh, material, transform, lod } => {
                    // Рендеринг чанка террейна
                    tracing::debug!("Rendering terrain chunk {:?} at LOD {}", chunk_id, lod);
                    self.stats.draw_calls += 1;
                }
                RenderCommand::Skybox { texture, sun_direction } => {
                    // Рендеринг неба
                    tracing::debug!("Rendering skybox with sun direction {:?}", sun_direction);
                    self.stats.draw_calls += 1;
                }
                RenderCommand::Sun { direction, angular_radius, color } => {
                    // Рендеринг солнца
                    tracing::debug!("Rendering sun at direction {:?}", direction);
                    self.stats.draw_calls += 1;
                }
                RenderCommand::LineList { vertices, colors } => {
                    // TODO: отрисовка линий (можно использовать DebugRenderer)
                    tracing::debug!("Rendering {} line vertices", vertices.len());
                    self.stats.draw_calls += 1;
                }
                RenderCommand::MeshDeform { mesh, deformations } => {
                    tracing::debug!("Deforming mesh {:?} with {} deformations", mesh, deformations.len());
                }
            }
        }
        
        Ok(())
    }
    
    /// Получает статистику рендеринга
    pub fn get_stats(&self) -> SceneRendererStats {
        self.stats.clone()
    }
    
    /// Получает или создаёт пайплайн из кэша
    pub fn get_or_create_pipeline(&mut self, key: String, create_fn: impl FnOnce() -> ResourceHandle) -> ResourceHandle {
        use std::collections::hash_map::Entry;
        
        match self.pipeline_cache.entry(key) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => {
                let pipeline = create_fn();
                *entry.insert(pipeline)
            }
        }
    }
}
