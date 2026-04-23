//! Scene Renderer - рендеринг 3D сцены через RHI
//! 
//! Владеет постоянными буферами (view_proj, light), сортирует команды по материалам

use crate::graphics::rhi::{IDevice, ICommandList, ResourceHandle, BufferDescription, BufferType, BufferUsage, ResourceState};
use crate::graphics::renderer::commands::RenderCommand;
use crate::graphics::camera::Camera;
use nalgebra::Matrix4;
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

/// Scene Renderer
pub struct SceneRenderer {
    device: Arc<dyn IDevice>,
    camera_buffer: Option<ResourceHandle>,
    light_buffer: Option<ResourceHandle>,
    pipeline_cache: std::collections::HashMap<String, ResourceHandle>,
    camera_data: CameraBuffer,
    light_data: LightBuffer,
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
        };
        
        self.light_buffer = Some(
            self.device.create_buffer(&lb_desc)
                .map_err(|e| format!("Failed to create light buffer: {:?}", e))?
        );
        
        Ok(())
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
    
    /// Рендерит сцену
    pub fn render(&mut self, camera: &Camera, commands: &[RenderCommand]) -> Result<(), String> {
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
        
        // Сортируем команды по материалам для минимизации смены состояний
        // В полной реализации здесь будет batching и sorting
        
        // Рендерим каждую команду
        for command in commands {
            match command {
                RenderCommand::Mesh { mesh, material, transform } => {
                    // TODO: установить константный буфер с трансформацией
                    // TODO: забиндить материал и меш
                    // TODO: вызвать draw
                    tracing::debug!("Rendering mesh with material {:?}", material);
                }
                RenderCommand::MeshInstanced { mesh, material, transforms } => {
                    // TODO: инстансированный рендеринг
                    tracing::debug!("Rendering {} instances with material {:?}", transforms.len(), material);
                }
                RenderCommand::LineList { vertices, colors } => {
                    // TODO: отрисовка линий (можно использовать DebugRenderer)
                    tracing::debug!("Rendering {} line vertices", vertices.len());
                }
            }
        }
        
        Ok(())
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
