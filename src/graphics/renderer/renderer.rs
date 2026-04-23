//! Главный модуль Renderer - координирует рендеринг сцены, UI и отладки
//! 
//! Использует RHI для абстракции над графическим бэкендом

use crate::graphics::rhi::{IDevice, ICommandQueue, ISwapChain, ResourceHandle};
use crate::graphics::renderer::{
    SceneRenderer, UIRenderer, DebugRenderer, PipelineCache,
    MainRenderPass, ShadowRenderPass, PostProcessRenderPass,
    RenderCommand, UiCommand, RendererConfig,
};
use crate::graphics::camera::Camera;
use std::sync::Arc;

/// Основной Renderer
pub struct Renderer {
    device: Arc<dyn IDevice>,
    command_queue: Arc<dyn ICommandQueue>,
    swap_chain: Arc<dyn ISwapChain>,
    
    // Под-рендереры
    scene_renderer: SceneRenderer,
    ui_renderer: UIRenderer,
    debug_renderer: DebugRenderer,
    
    // Кэш пайплайнов
    pipeline_cache: PipelineCache,
    
    // Render passes
    main_pass: Option<MainRenderPass>,
    shadow_pass: Option<ShadowRenderPass>,
    post_process_pass: Option<PostProcessRenderPass>,
    
    // Камера
    camera: Camera,
    
    // Размеры экрана
    width: u32,
    height: u32,
    
    // Состояние
    debug_mode: bool,
    vsync: bool,
}

impl Renderer {
    /// Создаёт новый рендерер
    pub fn new(
        device: Arc<dyn IDevice>,
        command_queue: Arc<dyn ICommandQueue>,
        swap_chain: Arc<dyn ISwapChain>,
        config: &RendererConfig,
    ) -> Result<Self, String> {
        let width = config.width;
        let height = config.height;
        
        // Создаём под-рендереры
        let mut scene_renderer = SceneRenderer::new(device.clone());
        let mut ui_renderer = UIRenderer::new(device.clone(), width, height);
        let debug_renderer = DebugRenderer::new(device.clone());
        
        // Инициализируем UI рендерер (шейдеры, PSO, буферы)
        ui_renderer.initialize()
            .map_err(|e| format!("Failed to initialize UI renderer: {}", e))?;
        
        // Инициализируем Scene рендерер
        scene_renderer.initialize()
            .map_err(|e| format!("Failed to initialize Scene renderer: {}", e))?;
        
        let mut renderer = Self {
            device: device.clone(),
            command_queue,
            swap_chain: swap_chain.clone(),
            scene_renderer,
            ui_renderer,
            debug_renderer,
            pipeline_cache: PipelineCache::new(),
            main_pass: None,
            shadow_pass: None,
            post_process_pass: None,
            camera: Camera::default(),
            width,
            height,
            debug_mode: config.debug_mode,
            vsync: config.vsync,
        };
        
        // Инициализируем под-рендереры
        renderer.scene_renderer.initialize()?;
        renderer.ui_renderer.initialize()?;
        renderer.debug_renderer.initialize()?;
        
        // Обновляем орто-матрицу UI
        renderer.ui_renderer.update_ortho_matrix(width, height);
        
        // Создаём render passes с размерами экрана
        renderer.create_render_passes()?;
        
        Ok(renderer)
    }
    
    /// Создаёт все render passes
    fn create_render_passes(&mut self) -> Result<(), String> {
        // Для начала создадим простой тестовый pass с очисткой экрана
        // В полной реализации здесь будут созданы framebuffer'ы для color/depth
        
        // Получаем backbuffer texture из swapchain для основного прохода
        let backbuffer_texture = self.swap_chain.get_back_buffer_texture();
        
        // Создаём depth texture (пока заглушка - в полной реализации создать через device.create_texture)
        let depth_texture = ResourceHandle::default();
        
        self.main_pass = Some(MainRenderPass::new(
            backbuffer_texture,
            depth_texture,
            self.width,
            self.height,
        ));
        
        Ok(())
    }
    
    /// Начинает кадр
    pub fn begin_frame(&mut self) -> Result<(), String> {
        // Очищаем накопленные команды в под-рендерерах
        self.debug_renderer.clear();
        self.ui_renderer.clear();
        
        Ok(())
    }
    
    /// Рендер кадра (основной метод)
    pub fn render_frame(&mut self) -> Result<(), String> {
        // Создаём command list для текущего кадра
        let mut cmd_list = self.device.create_command_list(crate::graphics::rhi::CommandListType::Graphics)
            .map_err(|e| format!("Failed to create command list: {:?}", e))?;
        
        // Начинаем render pass с очисткой экрана
        if let Some(ref main_pass) = self.main_pass {
            cmd_list.begin_render_pass(&main_pass.description());
            
            // Здесь будет рендеринг сцены, UI и отладки
            // Пока просто очищаем экран цветом из main_pass
            
            cmd_list.end_render_pass();
        }
        
        // Завершаем command list и отправляем на выполнение
        cmd_list.close();
        self.command_queue.submit(&[&cmd_list])
            .map_err(|e| format!("Failed to submit command list: {:?}", e))?;
        
        // Present
        self.end_frame()
    }
    
    /// Рендерит сцену
    pub fn render_scene(&mut self, commands: &[RenderCommand], cmd_list: &mut dyn ICommandList) -> Result<(), String> {
        self.scene_renderer.render(&self.camera, commands, cmd_list)?;
        Ok(())
    }
    
    /// Рендерит UI
    pub fn render_ui(&mut self, commands: &[UiCommand], cmd_list: &mut dyn ICommandList) -> Result<(), String> {
        self.ui_renderer.render(commands, cmd_list)?;
        Ok(())
    }
    
    /// Рендерит отладочную информацию
    pub fn render_debug(&mut self) -> Result<(), String> {
        // Debug rendering будет вызван внутри render_scene или отдельно
        Ok(())
    }
    
    /// Завершает кадр
    pub fn end_frame(&mut self) -> Result<(), String> {
        // Present через swap chain
        self.swap_chain.present()
            .map_err(|e| format!("Failed to present: {:?}", e))?;
        
        Ok(())
    }
    
    /// Обрабатывает изменение размера окна
    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width.max(1);
        self.height = height.max(1);
        
        // Обновляем камеру
        self.camera.update_aspect(self.width as f32, self.height as f32);
        
        // Обновляем UI орто-матрицу
        self.ui_renderer.update_ortho_matrix(self.width, self.height);
        
        // Пересоздаём swap chain и render passes при необходимости
        // TODO: resize swap chain
    }
    
    /// Устанавливает камеру
    pub fn set_camera(&mut self, camera: Camera) {
        self.camera = camera;
    }
    
    /// Получает ссылку на камеру
    pub fn camera(&self) -> &Camera {
        &self.camera
    }
    
    /// Получает мутабельную ссылку на камеру
    pub fn camera_mut(&mut self) -> &mut Camera {
        &mut self.camera
    }
    
    /// Устанавливает режим отладки
    pub fn set_debug_mode(&mut self, enabled: bool) {
        self.debug_mode = enabled;
    }
    
    /// Проверяет режим отладки
    pub fn is_debug_mode(&self) -> bool {
        self.debug_mode
    }
    
    /// Получает ширину экрана
    pub fn width(&self) -> u32 {
        self.width
    }
    
    /// Получает высоту экрана
    pub fn height(&self) -> u32 {
        self.height
    }
    
    /// Получает статистику pipeline cache
    pub fn pipeline_cache_stats(&self) -> crate::graphics::renderer::PipelineCacheStats {
        // Пока заглушка - нужно добавить поле для статистики
        crate::graphics::renderer::PipelineCacheStats::default()
    }
}
