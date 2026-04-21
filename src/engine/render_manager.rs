//! Менеджер рендеринга - инкапсуляция графической подсистемы
//!
//! Этот модуль управляет рендерингом сцены, UI и пост-обработкой,
//! предоставляя контролируемый интерфейс для графических операций.
//!
//! Поддерживает несколько бэкендов:
//! - OpenGL (через glow)
//! - DirectX 11 (через RHI)
//! - RHI-абстракция для кроссплатформенности

use crate::game::MainMenu;
use crate::graphics::debug_renderer::DebugRenderer;
use crate::graphics::material::MaterialManager;
use crate::graphics::particles::ParticleSystem;
use crate::graphics::renderer::{MenuState, Renderer};
use crate::graphics::renderer_dx11::Dx11Renderer;
use crate::graphics::renderer_rhi::RendererRhi;
use crate::graphics::GraphicsContext;
use crate::ui::HudManager;
use nalgebra::Matrix4;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// Бэкенд рендерера
pub enum RenderBackend {
    /// OpenGL через glow
    OpenGL(Renderer),
    /// DirectX 11 нативный
    DX11(Dx11Renderer),
    /// RHI-абстракция (универсальный)
    Rhi(RendererRhi),
}

/// Менеджер рендеринга
pub struct RenderManager {
    /// Рендерер сцены (поддержка нескольких бэкендов)
    renderer: Option<RenderBackend>,
    /// Графический контекст
    graphics_context: GraphicsContext,
    /// Менеджер материалов
    material_manager: MaterialManager,
    /// Система частиц
    particle_system: ParticleSystem,
    /// Отладочный рендерер
    debug_renderer: DebugRenderer,
    /// HUD менеджер
    hud_manager: HudManager,
    /// Главное меню (через game модуль)
    main_menu: MainMenu,
    /// Позиция мыши X
    mouse_x: f32,
    /// Позиция мыши Y
    mouse_y: f32,
    /// Режим отладки
    debug_mode: bool,
    /// Предпочтение дискретной GPU
    prefer_discrete_gpu: bool,
}

impl RenderManager {
    /// Создаёт новый менеджер рендеринга
    pub fn new(
        mut graphics_context: GraphicsContext,
        material_manager: MaterialManager,
        particle_system: ParticleSystem,
        debug_renderer: DebugRenderer,
        hud_manager: HudManager,
    ) -> Self {
        // Определяем предпочтение дискретной GPU из контекста
        let prefer_discrete_gpu = match &graphics_context {
            GraphicsContext::DX11(ctx) => ctx.config.prefer_discrete_gpu,
            _ => true,
        };

        Self {
            renderer: None,
            graphics_context,
            material_manager,
            particle_system,
            debug_renderer,
            hud_manager,
            main_menu: MainMenu::new(),
            mouse_x: 0.0,
            mouse_y: 0.0,
            debug_mode: false,
            prefer_discrete_gpu,
        }
    }

    /// Инициализирует рендерер с автоматическим выбором бэкенда
    pub fn initialize_renderer(&mut self) -> Result<(), String> {
        info!(target: "render", "=== RenderManager::initialize_renderer START ===");
        
        match &self.graphics_context {
            GraphicsContext::OpenGL(ref ctx) => {
                info!(target: "render", "Initializing OpenGL backend");
                
                if let Some(ref gl) = ctx.gl {
                    match Renderer::new(gl.clone()) {
                        Ok(mut renderer) => {
                            renderer.width = ctx.width;
                            renderer.height = ctx.height;
                            renderer.menu_state = MenuState::MainMenu;
                            self.renderer = Some(RenderBackend::OpenGL(renderer));
                            info!(target: "render", "OpenGL renderer initialized successfully");
                            info!(target: "render", "=== RenderManager::initialize_renderer END (OpenGL) ===");
                            return Ok(());
                        }
                        Err(e) => {
                            let msg = format!("OpenGL renderer initialization failed: {}", e);
                            error!(target: "render", "{}", msg);
                            return Err(msg);
                        }
                    }
                } else {
                    let msg = "GL context is None".to_string();
                    error!(target: "render", "{}", msg);
                    return Err(msg);
                }
            }
            
            GraphicsContext::DX11(ref ctx) => {
                info!(target: "render", "Initializing DirectX 11 backend");
                
                // Получаем HWND из контекста
                let hwnd = ctx.get_hwnd() as isize;
                let width = ctx.width;
                let height = ctx.height;
                
                match Dx11Renderer::new(hwnd, width, height, self.prefer_discrete_gpu) {
                    Ok(mut renderer) => {
                        renderer.menu_state = MenuState::MainMenu;
                        self.renderer = Some(RenderBackend::DX11(renderer));
                        info!(target: "render", "DX11 renderer initialized successfully");
                        info!(target: "render", "=== RenderManager::initialize_renderer END (DX11) ===");
                        return Ok(());
                    }
                    Err(e) => {
                        let msg = format!("DX11 renderer initialization failed: {}", e);
                        error!(target: "render", "{}", msg);
                        warn!(target: "render", "Falling back to RHI abstraction");
                        
                        // Fallback на RHI-абстракцию
                        return self.initialize_rhi_backend();
                    }
                }
            }
        }
    }
    
    /// Инициализация через RHI-абстракцию (универсальный путь)
    fn initialize_rhi_backend(&mut self) -> Result<(), String> {
        info!(target: "render", "Initializing RHI backend");
        
        // В полной реализации здесь будет создание устройства через RhiFactory
        // Пока заглушка для будущего расширения
        warn!(target: "render", "RHI backend not fully implemented yet");
        Err("RHI backend not available".to_string())
    }

    /// Обновляет позицию мыши
    pub fn update_mouse_position(&mut self, x: f32, y: f32) {
        self.mouse_x = x;
        self.mouse_y = y;

        if let Some(ref mut backend) = self.renderer {
            match backend {
                RenderBackend::OpenGL(renderer) => {
                    renderer.mouse_x = x;
                    renderer.mouse_y = y;
                }
                RenderBackend::DX11(_) => {
                    // DX11 renderer handles mouse internally
                }
                RenderBackend::Rhi(_) => {
                    // RHI renderer handles mouse internally
                }
            }
        }
    }

    /// Забрать graphics_context обратно
    pub fn take_context(&mut self) -> GraphicsContext {
        std::mem::replace(
            &mut self.graphics_context,
            GraphicsContext::new_opengl(crate::graphics::GlContext::new_placeholder()),
        )
    }

    /// Обновляет камеру на основе позиции вертолёта
    pub fn update_camera_from_helicopter(&mut self, position: nalgebra::Vector3<f32>) {
        if let Some(ref mut backend) = self.renderer {
            match backend {
                RenderBackend::OpenGL(renderer) => {
                    renderer.camera.position = position;
                }
                RenderBackend::DX11(renderer) => {
                    renderer.camera.position = position;
                }
                RenderBackend::Rhi(renderer) => {
                    renderer.camera.position = position;
                }
            }
        }
    }

    /// Обновляет камеру на основе позиции транспорта (общая версия)
    pub fn update_camera_from_vehicle(
        &mut self,
        position: nalgebra::Vector3<f32>,
        rotation: nalgebra::Quaternion<f32>,
    ) {
        if let Some(ref mut backend) = self.renderer {
            let unit_rot = nalgebra::UnitQuaternion::new_unchecked(rotation);
            let offset = unit_rot * nalgebra::Vector3::new(0.0, 5.0, 10.0);
            
            match backend {
                RenderBackend::OpenGL(renderer) => {
                    renderer.camera.position = position + offset;
                    renderer.camera.target = position;
                }
                RenderBackend::DX11(renderer) => {
                    renderer.set_vehicle_transform(position, unit_rot);
                }
                RenderBackend::Rhi(renderer) => {
                    renderer.set_vehicle_transform(position, unit_rot);
                }
            }
        }
    }

    /// Устанавливает трансформацию транспорта для рендеринга
    pub fn set_vehicle_transform(
        &mut self,
        position: nalgebra::Vector3<f32>,
        rotation: nalgebra::UnitQuaternion<f32>,
    ) {
        if let Some(ref mut backend) = self.renderer {
            match backend {
                RenderBackend::OpenGL(renderer) => {
                    renderer.set_vehicle_transform(position, rotation);
                }
                RenderBackend::DX11(renderer) => {
                    renderer.set_vehicle_transform(position, rotation);
                }
                RenderBackend::Rhi(renderer) => {
                    renderer.set_vehicle_transform(position, rotation);
                }
            }
        }
    }

    /// Устанавливает цвета неба
    pub fn set_sky_colors(
        &mut self,
        top_color: nalgebra::Vector3<f32>,
        horizon_color: nalgebra::Vector3<f32>,
    ) {
        if let Some(ref mut backend) = self.renderer {
            match backend {
                RenderBackend::OpenGL(renderer) => {
                    renderer.set_sky_color(top_color, horizon_color);
                }
                RenderBackend::DX11(renderer) => {
                    renderer.set_sky_color(top_color, horizon_color);
                }
                RenderBackend::Rhi(renderer) => {
                    renderer.set_sky_color(top_color, horizon_color);
                }
            }
        }
    }

    /// Устанавливает направление солнца
    pub fn set_sun_direction(&mut self, sun_dir: nalgebra::Vector3<f32>) {
        if let Some(ref mut renderer) = self.renderer {
            renderer.sun_direction = sun_dir;
        }
    }

    /// Устанавливает состояние меню в рендерере
    pub fn set_menu_state(&mut self, menu_state: MenuState) {
        if let Some(ref mut renderer) = self.renderer {
            renderer.menu_state = menu_state;
        }
    }

    /// Устанавливает режим отладки
    pub fn set_debug_mode(&mut self, enabled: bool) {
        self.debug_mode = enabled;
        if let Some(ref mut renderer) = self.renderer {
            renderer.debug_mode = enabled;
        }
    }

    /// Начинает кадр (очистка буфера)
    pub fn begin_frame(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.graphics_context.begin_frame();
        Ok(())
    }

    /// Рендерит кадр
    pub fn render(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!(target: "render", "=== RenderManager: rendering frame ===");

        if let Some(ref mut renderer) = self.renderer {
            info!(target: "render", "=== RenderManager: rendering frame ===");
            renderer.render()?;

            info!(target: "render", "=== RenderManager: calling main_menu.render_ui() ===");
            self.main_menu.render_ui(renderer);
            info!(target: "render", "=== RenderManager: frame complete ===");
        } else {
            warn!(target: "render", "Renderer is None in RenderManager::render()");
            if let Some(ref gl) = self.graphics_context.get_glow() {
                let view_matrix = Matrix4::identity();
                let proj_matrix = self.graphics_context.get_projection_matrix(
                    std::f32::consts::PI / 4.0,
                    0.1,
                    1000.0,
                );
                let view_proj = proj_matrix * view_matrix;

                self.particle_system.render(gl, view_proj);

                if self.debug_mode {
                    self.debug_renderer.flush_to_gl(gl, view_proj);
                }
            }
        }
        Ok(())
    }

    /// Завершает кадр (swap buffers)
    pub fn end_frame(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!(target: "render", ">>> end_frame() called <<<");
        self.graphics_context.end_frame();
        Ok(())
    }

    /// Обрабатывает изменение размера окна
    pub fn on_resize(&mut self, width: u32, height: u32) -> Result<(), Box<dyn std::error::Error>> {
        let width = width.max(1);
        let height = height.max(1);

        self.graphics_context.resize(width, height);

        if let Some(ref mut renderer) = self.renderer {
            renderer.width = width;
            renderer.height = height;
        }

        Ok(())
    }

    /// Получает ссылку на рендерер
    pub fn get_renderer(&self) -> Option<&Renderer> {
        self.renderer.as_ref()
    }

    /// Получает мутабельную ссылку на рендерер
    pub fn get_renderer_mut(&mut self) -> Option<&mut Renderer> {
        self.renderer.as_mut()
    }

    /// Получает ссылку на HUD менеджер
    pub fn get_hud_manager(&self) -> &HudManager {
        &self.hud_manager
    }

    /// Получает мутабельную ссылку на HUD менеджер
    pub fn get_hud_manager_mut(&mut self) -> &mut HudManager {
        &mut self.hud_manager
    }

    /// Проверяет, инициализирован ли рендерер
    pub fn is_initialized(&self) -> bool {
        self.renderer.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_manager_creation() {
        // Тест требует создания GlContext, что сложно в unit тесте
        // Поэтому просто проверяем что структура компилируется
    }
}
