//! Ядро движка - координация всех подсистем
//!
//! Этот модуль предоставляет основной класс `Engine`, который координирует работу всех менеджеров.
//! Вся специализированная логика вынесена в отдельные менеджеры.

use crate::config::Config;
use crate::engine::game_loop_manager::GameLoopManager;
use crate::engine::input_manager::InputManagerWrapper;
use crate::engine::physics_manager::PhysicsManager;
use crate::engine::render_manager::RenderManager;
use crate::engine::state::{EngineState, MenuState};
use crate::engine::subsystems::EngineSubsystems;
use crate::engine::vehicle_manager::VehicleManager;
use crate::engine::world_manager::WorldManager;
use crate::game::debug_menu::DebugMenu;
use crate::game::interaction::InteractionSystem;
use crate::game::loading_manager::{LoadingManager, LoadingStage, LoadingStateDetailed};
use crate::game::MainMenu;
use crate::graphics::debug_renderer::DebugRenderer;
use crate::graphics::material::MaterialManager;
use crate::graphics::particles::ParticleSystem;
use crate::graphics::GlContext;
use crate::graphics::GraphicsContext;
use crate::physics::set_global_physics_world;
use crate::ui::HudManager;
use nalgebra::{UnitQuaternion, Vector3};
use raw_window_handle::HasWindowHandle;
use std::sync::Arc;
use std::time::Instant;
use tracing::{error, info, warn};
use winit::application::ApplicationHandler;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

/// Основной класс движка
pub struct Engine {
    /// Графический контекст (универсальный) - хранится в RenderManager
    pub graphics_context: Option<GraphicsContext>,

    /// Конфигурация движка
    pub config: Config,

    /// Контейнер всех подсистем
    pub subsystems: EngineSubsystems,

    /// Менеджер физики
    pub physics_manager: PhysicsManager,

    /// Менеджер мира
    pub world_manager: WorldManager,

    /// Менеджер транспортных средств
    pub vehicle_manager: VehicleManager,

    /// Менеджер ввода
    pub input_manager: InputManagerWrapper,

    /// Менеджер рендеринга (будет инициализирован позже)
    pub render_manager: Option<RenderManager>,

    /// Менеджер игрового цикла
    pub game_loop_manager: GameLoopManager,

    /// Главное меню
    main_menu: MainMenu,

    /// Состояние игры
    game_state: EngineState,

    /// Последнее время кадра
    last_frame_time: Instant,

    /// Аккумулятор физического времени
    physics_accumulator: f32,

    /// Шаг физического времени
    physics_timestep: f32,

    /// Флаг выхода
    should_quit: bool,
}

impl Engine {
    /// Создаёт новый движок
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Try to load config - check multiple possible locations
        let config_paths = [
            std::path::Path::new("config.json"),
            std::path::Path::new("..\\config.json"),
            std::path::Path::new("..\\..\\config.json"),
            std::path::Path::new("..\\..\\..\\config.json"),
        ];

        // Find first existing config file
        let config_file = config_paths.iter().find(|p| p.exists());

        let config = if let Some(p) = config_file {
            tracing::info!(target: "engine", "Found config at: {:?}", p);
            match Config::load(p) {
                Ok(c) => {
                    tracing::info!(target: "engine", "Config loaded! Backend: {}", c.graphics.backend);
                    c
                }
                Err(e) => {
                    tracing::warn!("Config parse error: {}, using default", e);
                    Config::default()
                }
            }
        } else {
            tracing::warn!("config.json not found in any location, using default");
            Config::default()
        };

        // Графический контекст будет создан в resumed()
        let graphics_context: Option<GraphicsContext> = None;

        // Создание подсистем
        let physics_world = crate::physics::PhysicsWorld::new();
        let subsystems = EngineSubsystems::new(
            crate::engine::subsystems::GraphicsSubsystem::new(
                None,
                MaterialManager::new(crate::graphics::material::TextureQuality::Medium),
                ParticleSystem::new(1000),
                DebugRenderer::new(),
            ),
            crate::engine::subsystems::PhysicsSubsystem::new(physics_world),
            crate::input::InputManager::new(),
            crate::audio::AudioSystem::new()?,
            crate::ecs::EcsManager::new(),
            crate::engine::subsystems::UISubsystem::new(
                HudManager::new(),
                crate::game::ui::UIManager::new(),
                DebugMenu::new(),
            ),
            crate::engine::subsystems::WorldSubsystem::new(crate::world::DayNightCycle::new(
                55.0, 82.9,
            )),
            LoadingManager::new("assets"),
            crate::game::save::SaveSystem::default(),
        );

        // Создание менеджеров
        let physics_manager = PhysicsManager::new(subsystems.physics.physics_world.clone());

        // Обновляем глобальный physics_world для raycast
        set_global_physics_world(&subsystems.physics.physics_world);

        let world_manager = WorldManager::new(42);
        let vehicle_manager = VehicleManager::new(Vector3::zeros());
        let input_manager = InputManagerWrapper::new();

        // Создание менеджера игрового цикла - используем те же экземпляры из subsystems для избежания дублирования
        let game_loop_manager = GameLoopManager::new(
            InteractionSystem::new(),
            subsystems.ui.debug_menu.clone(),
            subsystems.ui.hud_manager.clone(),
            subsystems.graphics.particle_system.clone(),
            subsystems.graphics.debug_renderer.clone(),
        );

        Ok(Self {
            graphics_context,
            config,
            subsystems,
            physics_manager,
            world_manager,
            vehicle_manager,
            input_manager,
            render_manager: None,
            game_loop_manager,
            main_menu: MainMenu::new(),
            game_state: EngineState::main_menu(),
            last_frame_time: Instant::now(),
            physics_accumulator: 0.0,
            physics_timestep: 1.0 / 60.0,
            should_quit: false,
        })
    }

    /// Запускает движок
    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!(target: "engine", "=== RTGC-0.8 ENGINE STARTING ===");
        info!(target: "engine", "Version: 0.8.0 | Build: Release");
        info!(target: "engine", "Starting engine...");

        let event_loop = EventLoop::new()?;
        event_loop.set_control_flow(ControlFlow::Poll); // Poll - постоянный цикл событий

        let mut app = GameApp {
            window: None,
            last_frame_time: Instant::now(),
            initialized: false,
            engine: self,
        };

        info!(target: "engine", "Event loop created, entering main loop...");
        event_loop.run_app(&mut app)?;

        info!(target: "engine", "=== RTGC-0.8 ENGINE SHUTDOWN ===");
        Ok(())
    }
}

/// Приложение winit
struct GameApp<'a> {
    window: Option<Arc<Window>>,
    last_frame_time: Instant,
    initialized: bool,
    engine: &'a mut Engine,
}

impl ApplicationHandler for GameApp<'_> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.initialized {
            return;
        }

        info!(target: "engine", "=== RESUMED CALLED ===");

        let backend = &self.engine.config.graphics.backend;
        info!(target: "engine", "Requested graphics backend: {}", backend);

        // For DX11/DX12: create our own window
        // For OpenGL: let GlContext create its own window internally
        let window_attrs = WindowAttributes::default()
            .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 720.0))
            .with_title("RTGC-0.8");

        // Create window only for DX backends
        let window_arc = if backend.to_lowercase().as_str() == "dx11"
            || backend.to_lowercase().as_str() == "dx12"
        {
            match event_loop.create_window(window_attrs) {
                Ok(w) => Some(Arc::new(w)),
                Err(e) => {
                    error!(target: "engine", "Failed to create window: {:?}", e);
                    event_loop.exit();
                    return;
                }
            }
        } else {
            None
        };

        match backend.to_lowercase().as_str() {
            "dx11" | "dx12" => {
                // DX11/DX12: Use our window for DX context
                use raw_window_handle::RawWindowHandle;
                let hwnd = match window_arc.as_ref().unwrap().window_handle() {
                    Ok(handle) => {
                        let raw = handle.as_raw();
                        match raw {
                            RawWindowHandle::Win32(win32) => win32.hwnd.get() as isize,
                            _ => {
                                error!(target: "engine", "Unsupported window handle type");
                                event_loop.exit();
                                return;
                            }
                        }
                    }
                    Err(e) => {
                        error!(target: "engine", "Failed to get window handle: {:?}", e);
                        event_loop.exit();
                        return;
                    }
                };

                info!(target: "engine", "Creating DX{} context...", &backend[2..4]);
                let width = window_arc.as_ref().unwrap().inner_size().width;
                let height = window_arc.as_ref().unwrap().inner_size().height;
                match crate::graphics::dx11_context::Dx11GraphicsContext::new(hwnd, width, height) {
                    Ok(dx_ctx) => {
                        self.engine.graphics_context = Some(GraphicsContext::DX11(dx_ctx));
                        info!(target: "engine", "DX{} context created successfully", &backend[2..4]);
                    }
                    Err(e) => {
                        error!(target: "engine", "Failed to create DX{} context: {}", &backend[2..4], e);
                        event_loop.exit();
                        return;
                    }
                }
            }
            _ => {
                // OpenGL: Let GlContext create its own window with full GL setup
                info!(target: "engine", "Creating GL context...");
                let window_attrs = WindowAttributes::default()
                    .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 720.0))
                    .with_title("RTGC-0.8");
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    crate::graphics::GlContext::new(event_loop, window_attrs)
                }));

                let mut gl_context = match result {
                    Ok(Ok(ctx)) => ctx,
                    Ok(Err(e)) => {
                        error!(target: "engine", "GL context creation failed: {:?}", e);
                        event_loop.exit();
                        return;
                    }
                    Err(panic_info) => {
                        error!(target: "engine", "PANIC during GL context creation: {:?}", panic_info);
                        event_loop.exit();
                        return;
                    }
                };

                if !gl_context.is_initialized() {
                    error!(target: "engine", "Graphics context not initialized after creation");
                    event_loop.exit();
                    return;
                }

                info!(target: "engine", "GL context is initialized!");
                self.engine.graphics_context = Some(GraphicsContext::new_opengl(gl_context));
            }
        };

        info!(target: "engine", "Creating render manager...");

        let material_manager = self.engine.subsystems.graphics.material_manager.clone();
        let particle_system = self.engine.subsystems.graphics.particle_system.clone();
        let debug_renderer = self.engine.subsystems.graphics.debug_renderer.clone();
        let hud_manager = self.engine.subsystems.ui.hud_manager.clone();

        // Забираем graphics_context из Engine
        let gc = self
            .engine
            .graphics_context
            .take()
            .expect("graphics_context not initialized");

        let mut render_manager = RenderManager::new(
            gc,
            material_manager,
            particle_system,
            debug_renderer,
            hud_manager,
        );

        if let Err(e) = render_manager.initialize_renderer() {
            error!(target: "engine", "Renderer init failed: {:?}", e);
            event_loop.exit();
            return;
        }

        // Сохраняем контекст обратно в Engine
        self.engine.graphics_context = Some(render_manager.take_context());

        self.engine.render_manager = Some(render_manager);

        let world_init = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.engine.world_manager.initialize_world()
        }));

        match world_init {
            Ok(Ok(())) => {
                info!(target: "engine", "World initialized successfully");
            }
            Ok(Err(e)) => {
                error!(target: "engine", "World init error: {:?}", e);
            }
            Err(panic_err) => {
                error!(target: "engine", "World initialization PANIC: {:?}", panic_err);
            }
        }

        // For storing window - only for DX backends
        let window_for_storing = window_arc.clone();

        self.last_frame_time = Instant::now();
        self.initialized = true;
        self.window = window_for_storing;

        if let Some(ref w) = self.window {
            w.request_redraw();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: WindowId,
        event: winit::event::WindowEvent,
    ) {
        use winit::event::WindowEvent;
        use winit::keyboard::{KeyCode, PhysicalKey};

        // Логируем ВСЕ события для диагностики
        info!(target: "engine", ">>> WindowEvent: {:?}", event);

        match event {
            WindowEvent::CloseRequested => {
                self.engine.should_quit = true;
                event_loop.exit();
            }

            WindowEvent::Resized(new_size) => {
                if let Some(ref mut render_manager) = self.engine.render_manager {
                    let _ = render_manager.on_resize(new_size.width, new_size.height);
                }
            }

            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(key_code) = event.physical_key {
                    self.engine.input_manager.set_key_state(
                        PhysicalKey::Code(key_code),
                        event.state == winit::event::ElementState::Pressed,
                    );

                    // Обработка специальных клавиш
                    if event.state == winit::event::ElementState::Pressed {
                        match key_code {
                            KeyCode::Escape => {
                                // Переключение паузы
                                if self.engine.game_state.is_playing() {
                                    self.engine.game_state = EngineState::paused(
                                        crate::engine::state::PauseReason::UserRequested,
                                    );
                                } else if self.engine.game_state.is_paused() {
                                    // Снятие с паузы - возврат в игру
                                    self.engine.game_state = EngineState::playing(0);
                                } else if self.engine.game_state.is_in_menu() {
                                    self.engine.should_quit = true;
                                    event_loop.exit();
                                }
                            }
                            KeyCode::F3 => {
                                // Toggle debug mode
                                let current = self.engine.game_loop_manager.is_debug_mode();
                                self.engine.game_loop_manager.set_debug_mode(!current);
                                if let Some(ref mut rm) = self.engine.render_manager {
                                    rm.set_debug_mode(!current);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }

            WindowEvent::MouseInput { button, state, .. } => {
                self.engine
                    .input_manager
                    .set_mouse_button_state(button, state == winit::event::ElementState::Pressed);
            }

            WindowEvent::CursorMoved { position, .. } => {
                if let Some(ref mut render_manager) = self.engine.render_manager {
                    render_manager.update_mouse_position(position.x as f32, position.y as f32);
                }
            }

            WindowEvent::RedrawRequested => {
                if !self.initialized {
                    error!(target: "engine", "Not initialized yet!");
                    return;
                }

                info!(target: "engine", ">>> REDRAW REQUESTED <<<");

                let current_time = Instant::now();
                let dt = current_time
                    .duration_since(self.last_frame_time)
                    .as_secs_f32();
                self.last_frame_time = current_time;
                let dt = dt.min(0.1);

                // Обновление
                if let Err(e) = self.engine.update(dt) {
                    error!(target: "engine", "Update error: {:?}", e);
                }

                // Рендеринг
                if let Some(ref mut render_manager) = self.engine.render_manager {
                    // Wrap in try-catch to prevent crashes
                    let render_result =
                        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            if let Err(e) = render_manager.begin_frame() {
                                error!(target: "engine", "begin_frame error: {:?}", e);
                            }
                            if let Err(e) = render_manager.render() {
                                error!(target: "engine", "Render error: {:?}", e);
                            }
                            if let Err(e) = render_manager.end_frame() {
                                error!(target: "engine", "end_frame error: {:?}", e);
                            }
                        }));

                    if let Err(panic_err) = render_result {
                        error!(target: "engine", "Render panic: {:?}", panic_err);
                        // Don't crash on render errors - just log them
                    }
                }

                if let Some(ref window) = self.window {
                    window.request_redraw();
                }
            }

            _ => {}
        }
    }
}

impl Engine {
    /// Обновляет все системы движка
    fn update(&mut self, dt: f32) -> Result<(), Box<dyn std::error::Error>> {
        // Проверка на NaN/Inf
        if !dt.is_finite() || dt <= 0.0 {
            warn!(target: "engine", "Invalid dt value: {}, skipping update", dt);
            return Ok(());
        }

        // Обновление ввода
        self.input_manager.update();

        // Передача ввода от игрока к физике транспорта
        let throttle = if self
            .input_manager
            .state()
            .is_action_held(crate::input::mapping::InputAction::ThrottleUp)
        {
            1.0
        } else if self
            .input_manager
            .state()
            .is_action_held(crate::input::mapping::InputAction::ThrottleDown)
        {
            -1.0
        } else {
            0.0
        };
        let brake = if self
            .input_manager
            .state()
            .is_action_held(crate::input::mapping::InputAction::Brake)
        {
            1.0
        } else {
            0.0
        };
        let steering = self
            .input_manager
            .state()
            .get_action_state(crate::input::mapping::InputAction::YawLeft)
            .map(|s| match s {
                crate::input::input_module::ActionState::Held => -1.0,
                _ => 0.0,
            })
            .unwrap_or(0.0)
            + self
                .input_manager
                .state()
                .get_action_state(crate::input::mapping::InputAction::YawRight)
                .map(|s| match s {
                    crate::input::input_module::ActionState::Held => 1.0,
                    _ => 0.0,
                })
                .unwrap_or(0.0);

        self.physics_manager
            .set_vehicle_inputs(throttle, steering, brake);

        // Физический шаг с фиксированным timestep
        self.physics_accumulator += dt;
        while self.physics_accumulator >= self.physics_timestep {
            if let Err(e) = self.physics_manager.step(self.physics_timestep) {
                error!(target: "physics", "Physics step error: {:?}", e);
            }
            self.physics_accumulator -= self.physics_timestep;
        }

        // Синхронизация физики с рендером: передача позиции транспорта в камеру
        if let Some(vehicle) = self.physics_manager.get_vehicle() {
            let pos = vehicle.position();
            let rot = vehicle.rotation();
            if let Some(ref mut rm) = self.render_manager {
                rm.set_vehicle_transform(pos, rot);
                rm.update_camera_from_vehicle(pos, *rot);
            }
        }

        // Синхронизация мира с рендером: освещение, небо, погода
        let hour = self.world_manager.get_current_hour();
        let sun_dir = self.world_manager.get_day_night_cycle().get_sun_direction();
        let sky_top = self.world_manager.get_day_night_cycle().get_sky_color_top();
        if let Some(ref mut rm) = self.render_manager {
            rm.set_sky_colors(sky_top, sky_top);
            rm.set_sun_direction(sun_dir);
        }

        // Обновление мира
        if let Err(e) = self.world_manager.update(dt) {
            error!(target: "world", "World update error: {:?}", e);
        }

        // Обновление игрового цикла
        let player_position = Some(self.vehicle_manager.get_player_position());
        let player_forward = self.vehicle_manager.get_player_forward();
        if let Err(e) =
            self.game_loop_manager
                .update(dt, &self.game_state, player_position, player_forward)
        {
            error!(target: "gameloop", "Game loop update error: {:?}", e);
        }

        // Обновление подсистем
        self.subsystems.update(dt);

        Ok(())
    }
}
