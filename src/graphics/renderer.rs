//! Renderer module - Command queue based rendering system with trait abstraction
//!
//! ARCHITECTURE: This module uses RenderCommand from render_command.rs for all
//! rendering operations. Local types (Handle, Material, RenderCommand, RenderQueue)
//! have been removed to avoid duplication.

use crate::graphics::debug_renderer::DebugRenderer;
use crate::graphics::lod_system::{LodManager, LodObject};
use crate::graphics::material::Material;
use crate::graphics::particles::ParticleSystem;
use crate::graphics::render_command::{Handle, RenderCommand};
use crate::graphics::render_queue::RenderQueue;
use crate::graphics::texture_streaming::TextureStreamingSystem;
use crate::graphics::{camera::Camera, mesh::Mesh, shader::Shader, texture::Texture};
use glow::{Context, HasContext};
use nalgebra::{Matrix4, UnitQuaternion, Vector3};
use std::collections::HashMap;
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, warn};

/// Renderer trait for backend abstraction
/// Примечание: не требуем Send так как glow::Context не реализует Send/Sync
pub trait RendererTrait {
    /// Submit a render command to the queue
    fn submit(&mut self, command: RenderCommand);

    /// Flush the render queue - execute all commands
    fn flush_render(&mut self) -> Result<(), Box<dyn std::error::Error>>;

    /// Set viewport dimensions
    fn set_viewport(&mut self, x: i32, y: i32, width: u32, height: u32);

    /// Clear the screen
    fn clear(&mut self, color: Option<[f32; 4]>, depth: bool, stencil: bool);

    /// Get camera reference
    fn camera(&self) -> &Camera;

    /// Get mutable camera reference
    fn camera_mut(&mut self) -> &mut Camera;
}

#[derive(Debug, Clone)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tex_coords: [f32; 2],
}

pub struct Model {
    pub meshes: Vec<Mesh>,
    pub textures: Vec<Texture>,
}

/// Main Renderer struct - high-performance command-based rendering system
///
/// Architecture:
/// - Uses render_queue::RenderQueue for efficient batching and sorting
/// - Supports multiple render backends through RHI abstraction
/// - Features: LOD, texture streaming, particle systems, debug rendering
/// - Optimized for minimal draw calls through material/shader batching
pub struct Renderer {
    gl: Arc<Context>,
    pub shader: Shader,
    pub camera: Camera,
    models: HashMap<String, Model>,
    current_city_index: usize,
    pub menu_state: MenuState,
    pub lod_manager: LodManager,
    pub texture_streaming: TextureStreamingSystem,
    // Render queue for command-based rendering (uses dedicated render_queue module)
    render_queue: crate::graphics::render_queue::RenderQueue,
    // Asset loader for loading meshes and textures
    pub asset_loader: crate::assets::loader::AssetLoader,
    // Terrain & Vehicle rendering
    terrain_mesh: Option<Mesh>,
    vehicle_box_mesh: Option<Mesh>,
    // Позиция и вращение транспорта для рендеринга (используем единое представление)
    pub vehicle_position: Option<Vector3<f32>>,
    pub vehicle_rotation: Option<UnitQuaternion<f32>>,
    // Window dimensions for HUD rendering
    pub width: u32,
    pub height: u32,
    // Mouse position for UI interaction
    pub mouse_x: f32,
    pub mouse_y: f32,
    // HUD Manager reference for rendering
    hud_data: Option<crate::ui::hud::VehicleHudData>,
    // Weather and Day/Night cycle support
    sky_color_top: Vector3<f32>,
    sky_color_horizon: Vector3<f32>,
    // Цвета неба для внешнего доступа
    pub sky_top_color: Vector3<f32>,
    pub sky_horizon_color: Vector3<f32>,
    pub sun_direction: Vector3<f32>,
    ambient_intensity: f32,
    vehicle_lights_enabled: bool,
    // Задача 2: Vehicle shader
    vehicle_shader: Option<Shader>,
    // UI шейдер для отрисовки интерфейса
    ui_shader: Option<Shader>,
    // Исп-2: Sky shader (separate from terrain shader)
    sky_shader: Option<Shader>,
    // Задача 3: Sky VAO
    sky_vao: Option<glow::VertexArray>,
    sky_vbo: Option<glow::Buffer>,
    // Граф-1: Bitmap font texture
    font_texture: Option<Texture>,
    font_chars: HashMap<char, [f32; 4]>,
    // Граф-2: Batched HUD VAO/VBO for optimization
    hud_vao: Option<glow::VertexArray>,
    hud_vbo: Option<glow::Buffer>,
    hud_vertices: Vec<f32>,
    // Граф-3: Minimap texture
    minimap_texture: Option<Texture>,
    minimap_size: u32,
    // Debug renderer for debug lines
    debug_renderer: DebugRenderer,
    // Particle system
    particle_system: ParticleSystem,
    // Debug mode flag
    pub debug_mode: bool,
    // Кэш для LOD мешей чтобы не создавать их каждый кадр
    lod_mesh_cache: HashMap<u64, Mesh>,
    // Переиспользуемые буферы для примитивов (линии, круги, треугольники)
    primitive_vao: Option<glow::VertexArray>,
    primitive_vbo: Option<glow::Buffer>,
    primitive_ibo: Option<glow::Buffer>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MenuState {
    Loading,
    MainMenu,
    CitySelection,
    InGame,
    WorldCreation,
    Settings,
    Paused,
    CharacterCreation,
}

/// Получить директорию исполняемого файла
fn get_exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Получить путь к директории assets
fn get_assets_dir() -> PathBuf {
    // Сначала пробуем найти assets рядом с exe
    let exe_dir = get_exe_dir();
    let exe_assets = exe_dir.join("assets");
    if exe_assets.join("shaders").exists() {
        return exe_assets;
    }

    // Потом пробуем текущую директорию
    let cwd = std::env::current_dir().unwrap_or_else(|_| exe_dir.clone());
    let current_assets = cwd.join("assets");
    if current_assets.join("shaders").exists() {
        return current_assets;
    }

    // Фолбэк: ищем в родительских директориях (для release сборки в target/release)
    let parent = exe_dir.parent().map(|p| p.to_path_buf());
    if let Some(parent) = parent {
        let target_assets = parent.join("assets");
        if target_assets.join("shaders").exists() {
            return target_assets;
        }
        // Также пробуем parent/../assets (симметричная структура)
        let project_assets = parent.parent().map(|p| p.join("assets"));
        if let Some(project_assets) = project_assets {
            if project_assets.join("shaders").exists() {
                return project_assets;
            }
        }
    }

    // Фолбэк: assets рядом с exe
    exe_assets
}

impl Renderer {
    pub fn new(gl: Arc<Context>) -> Result<Self, Box<dyn std::error::Error>> {
        unsafe {
            gl.enable(glow::DEPTH_TEST);
            gl.depth_func(glow::LESS);
            gl.enable(glow::CULL_FACE);
            gl.cull_face(glow::BACK);
        }

        // Исп-4: Загружать шейдер из файла относительно exe-файла или текущей директории
        let assets_dir = get_assets_dir();
        let shader_path = assets_dir.join("shaders");

        // Проверяем что assets директория существует
        if !shader_path.exists() {
            return Err(format!("Shader directory not found: {:?}", shader_path).into());
        }

        // Загрузка terrain shader с обработкой ошибок
        let vertex_src = match std::fs::read_to_string(shader_path.join("terrain.vert")) {
            Ok(s) => s,
            Err(e) => return Err(format!("Failed to load terrain.vert: {}", e).into()),
        };
        let fragment_src = match std::fs::read_to_string(shader_path.join("terrain.frag")) {
            Ok(s) => s,
            Err(e) => return Err(format!("Failed to load terrain.frag: {}", e).into()),
        };
        let shader = match Shader::new(&gl, &vertex_src, &fragment_src) {
            Ok(s) => s,
            Err(e) => return Err(format!("Failed to create terrain shader: {}", e).into()),
        };

        // Задача 2: Загрузить vehicle shader
        let vehicle_shader = match std::fs::read_to_string(shader_path.join("vehicle.vert")) {
            Ok(vs) => match std::fs::read_to_string(shader_path.join("vehicle.frag")) {
                Ok(fs) => Shader::new(&gl, &vs, &fs).ok(),
                Err(e) => {
                    warn!("Failed to load vehicle.frag: {}", e);
                    None
                }
            },
            Err(e) => {
                warn!("Failed to load vehicle.vert: {}", e);
                None
            }
        };

        // UI шейдер для отрисовки интерфейса
        let ui_shader = match std::fs::read_to_string(shader_path.join("ui.vert")) {
            Ok(vs) => match std::fs::read_to_string(shader_path.join("ui.frag")) {
                Ok(fs) => Shader::new(&gl, &vs, &fs).ok(),
                Err(e) => {
                    warn!("UI frag shader failed: {}", e);
                    None
                }
            },
            Err(e) => {
                warn!("UI vertex shader failed: {}", e);
                None
            }
        };

        // Исп-2: Создать простой шейдер для неба
        let sky_shader = Shader::new(&gl,
            "#version 330 core\nlayout(location=0) in vec2 pos;\nlayout(location=1) in vec3 col;\nout vec3 v_col;\nvoid main() { gl_Position = vec4(pos, 0.0, 1.0); v_col = col; }",
            "#version 330 core\nin vec3 v_col; out vec4 FragColor;\nvoid main() { FragColor = vec4(v_col, 1.0); }"
        ).ok();

        let camera = Camera::new(
            Vector3::new(0.0, 0.0, 3.0),
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0),
            45.0,
            800.0 / 600.0,
            0.1,
            1000.0,
        );

        // Задача 3: Создать VAO для неба
        let (sky_vao, sky_vbo) = unsafe {
            let vao = gl.create_vertex_array().ok();
            let vbo = gl.create_buffer().ok();
            if let Some(v) = vao {
                gl.bind_vertex_array(Some(v));
                // Вершины для 2 треугольников на весь экран [x, y, r, g, b]
                let verts: [f32; 30] = [
                    -1.0, -1.0, 0.7, 0.8, 0.9, // bottom-left horizon
                    1.0, -1.0, 0.7, 0.8, 0.9, // bottom-right horizon
                    1.0, 1.0, 0.4, 0.6, 0.9, // top-right top
                    -1.0, -1.0, 0.7, 0.8, 0.9, 1.0, 1.0, 0.4, 0.6, 0.9, -1.0, 1.0, 0.4, 0.6,
                    0.9, // top-left top
                ];
                if let Some(b) = vbo {
                    gl.bind_buffer(glow::ARRAY_BUFFER, Some(b));
                    gl.buffer_data_u8_slice(
                        glow::ARRAY_BUFFER,
                        bytemuck::cast_slice(&verts),
                        glow::STATIC_DRAW,
                    );
                    gl.enable_vertex_attrib_array(0);
                    gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 20, 0);
                    gl.enable_vertex_attrib_array(1);
                    gl.vertex_attrib_pointer_f32(1, 3, glow::FLOAT, false, 20, 8);
                }
                gl.bind_vertex_array(None);
                gl.bind_buffer(glow::ARRAY_BUFFER, None);
            }
            (vao, vbo)
        };

        // Граф-1: Создать bitmap font texture (процедурно, 128x128, 16x16 сетка символов)
        let (font_texture, font_chars) = Self::create_bitmap_font(&gl)
            .map_err(|e| format!("Failed to create bitmap font: {}", e))?;

        // Граф-2: Создать VAO/VBO для батчинга HUD
        let (hud_vao, hud_vbo) = unsafe {
            let vao = gl.create_vertex_array().ok();
            let vbo = gl.create_buffer().ok();
            if let Some(vao) = vao {
                gl.bind_vertex_array(Some(vao));
            }
            if let Some(vbo) = vbo {
                gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
                // Пустой буфер, будем обновлять каждый кадр
                gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, &[], glow::DYNAMIC_DRAW);
                gl.enable_vertex_attrib_array(0); // position: vec2
                gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 24, 0);
                gl.enable_vertex_attrib_array(1); // color: vec4
                gl.vertex_attrib_pointer_f32(1, 4, glow::FLOAT, false, 24, 8);
                gl.enable_vertex_attrib_array(2); // uv: vec2
                gl.vertex_attrib_pointer_f32(2, 2, glow::FLOAT, false, 24, 16);
            }
            gl.bind_vertex_array(None);
            gl.bind_buffer(glow::ARRAY_BUFFER, None);
            (vao, vbo)
        };

        Ok(Self {
            gl,
            shader,
            camera,
            models: HashMap::new(),
            current_city_index: 0,
            menu_state: MenuState::Loading,
            lod_manager: LodManager::new(),
            texture_streaming: TextureStreamingSystem::new(128, 10.0, 5),
            // Asset loader for loading meshes and textures
            asset_loader: crate::assets::loader::AssetLoader::new(),
            // Terrain & vehicle mesh placeholders (initialized on demand)
            terrain_mesh: None,
            vehicle_box_mesh: None,
            // Позиция и вращение транспорта для рендеринга
            vehicle_position: None,
            vehicle_rotation: None,
            hud_data: None,
            // Weather and Day/Night defaults
            sky_color_top: Vector3::new(0.4, 0.6, 0.9),
            sky_color_horizon: Vector3::new(0.7, 0.8, 0.9),
            // Цвета неба для внешнего доступа
            sky_top_color: Vector3::new(0.4, 0.6, 0.9),
            sky_horizon_color: Vector3::new(0.7, 0.8, 0.9),
            sun_direction: Vector3::y(),
            ambient_intensity: 0.5,
            vehicle_lights_enabled: false,
            // Задача 2: Vehicle shader
            vehicle_shader,
            // UI шейдер для отрисовки интерфейса
            ui_shader,
            // Исп-2: Sky shader
            sky_shader,
            // Задача 3: Sky VAO
            sky_vao,
            sky_vbo,
            // Граф-1: Bitmap font
            font_texture: Some(font_texture),
            font_chars,
            // Граф-2: Batched HUD
            hud_vao,
            hud_vbo,
            hud_vertices: Vec::with_capacity(1024),
            // Граф-3: Minimap
            minimap_texture: None,
            minimap_size: 128,
            width: 800,
            height: 600,
            // Mouse position for UI interaction
            mouse_x: 0.0,
            mouse_y: 0.0,
            // Render queue (using dedicated render_queue module for optimal batching)
            render_queue: crate::graphics::render_queue::RenderQueue::new(),
            // Debug renderer and particle system
            debug_renderer: DebugRenderer::new(),
            particle_system: ParticleSystem::new(1000),
            debug_mode: false,
            lod_mesh_cache: HashMap::new(),
            // Инициализация переиспользуемых буферов для примитивов
            primitive_vao: None,
            primitive_vbo: None,
            primitive_ibo: None,
        })
    }

    /// Инициализация переиспользуемых буферов для примитивов
    fn init_primitive_buffers(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.primitive_vao.is_some() {
            return Ok(()); // Уже инициализировано
        }

        let gl = &self.gl;
        let (vao, vbo, ibo) = unsafe {
            let vao = gl.create_vertex_array().ok();
            let vbo = gl.create_buffer().ok();
            let ibo = gl.create_buffer().ok();

            if let (Some(vao), Some(vbo)) = (vao, vbo) {
                gl.bind_vertex_array(Some(vao));
                gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
                
                // Выделяем буфер достаточного размера для примитивов
                gl.buffer_data::<glow::NativeVertex>(
                    glow::ARRAY_BUFFER,
                    (std::mem::size_of::<f32>() * 64) as isize,
                    glow::DYNAMIC_DRAW,
                );

                // Описание вершины (позиция + цвет)
                let stride = 7 * 4; // 7 f32 = 28 байт
                gl.enable_vertex_attrib_array(0);
                gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, stride as i32, 0);
                gl.enable_vertex_attrib_array(1);
                gl.vertex_attrib_pointer_f32(1, 4, glow::FLOAT, false, stride as i32, 12);

                gl.bind_vertex_array(None);
                gl.bind_buffer(glow::ARRAY_BUFFER, None);
            }

            (vao, vbo, ibo)
        };

        self.primitive_vao = vao;
        self.primitive_vbo = vbo;
        self.primitive_ibo = ibo;

        Ok(())
    }

    /// Submit a render command to the queue
    pub fn submit(&mut self, command: RenderCommand) {
        self.render_queue.submit(command);
    }

    /// Flush the render queue - execute all commands
    pub fn flush_render(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.render_queue.sort();

        let commands: Vec<_> = self.render_queue.commands().iter().cloned().collect();
        for command in commands {
            self.execute_command(&command)?;
        }

        self.render_queue.clear();
        Ok(())
    }

    /// Flush the render queue - execute all commands (alias for backward compatibility)
    pub fn flush(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.flush_render()
    }

    /// Проблема 11: Get orthographic projection matrix for UI rendering
    /// Используется для draw_text/draw_rect/UI элементов
    pub fn get_ortho_matrix(&self) -> Matrix4<f32> {
        Matrix4::new_orthographic(
            0.0,
            self.width as f32,
            self.height as f32,
            0.0, // Y=0 at top (screen coordinates)
            -1.0,
            1.0,
        )
    }

    /// Execute a single render command
    fn execute_command(
        &mut self,
        command: &RenderCommand,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match command {
            RenderCommand::Clear {
                color,
                depth,
                stencil,
                ..
            } => unsafe {
                let mut clear_bits = 0;
                if let Some([r, g, b, a]) = color {
                    self.gl.clear_color(*r, *g, *b, *a);
                    clear_bits |= glow::COLOR_BUFFER_BIT;
                }
                if *depth {
                    clear_bits |= glow::DEPTH_BUFFER_BIT;
                }
                if *stencil {
                    clear_bits |= glow::STENCIL_BUFFER_BIT;
                }
                if clear_bits != 0 {
                    self.gl.clear(clear_bits);
                }
            },
            RenderCommand::Viewport {
                x,
                y,
                width,
                height,
                ..
            } => unsafe {
                self.gl.viewport(*x, *y, *width as i32, *height as i32);
            },
            RenderCommand::DebugLine {
                start, end, color, ..
            } => {
                self.draw_debug_line([start.x, start.y, start.z], [end.x, end.y, end.z], *color);
            }
            RenderCommand::DebugLines { lines, .. } => {
                for (start, end, color) in lines {
                    self.draw_debug_line(*start, *end, *color);
                }
            }
            RenderCommand::UIElement {
                rect,
                texture,
                color,
                depth,
                ..
            } => {
                self.draw_ui_element(*rect, texture.clone(), *color, *depth);
            }
            // Execute Mesh command with proper rendering
            RenderCommand::Mesh {
                mesh,
                material,
                transform,
                ..
            } => {
                self.render_mesh_command(mesh, material, transform);
            }
            // Execute TerrainChunk command
            RenderCommand::TerrainChunk {
                chunk_id: _,
                mesh,
                material,
                transform,
                lod_level: _,
                ..
            } => {
                self.render_terrain_command(mesh, material, transform);
            }
            // Execute Skybox command
            RenderCommand::Skybox {
                texture: _,
                rotation,
                ..
            } => {
                self.render_skybox_command(rotation);
            }
            // Execute ParticleSystem command
            RenderCommand::ParticleSystem {
                system, transform, ..
            } => {
                self.render_particles_command(system, transform);
            }
            // Execute Vehicle command
            RenderCommand::Vehicle {
                position, rotation, ..
            } => {
                let rot =
                    UnitQuaternion::from_matrix(&rotation.fixed_view::<3, 3>(0, 0).into_owned());
                self.render_vehicle_command(position, &rot);
            }
            // UIDraw is handled in render_hud()
            RenderCommand::UIDraw { .. } => {
                // Handled separately in render_hud() method
            }
        }
        Ok(())
    }

    /// Render vehicle box from command
    fn render_vehicle_command(&mut self, position: &Vector3<f32>, rotation: &UnitQuaternion<f32>) {
        let model_matrix = rotation.to_homogeneous().prepend_translation(position);

        // Use vehicle_shader if available
        if let Some(ref vs) = self.vehicle_shader {
            vs.bind(&self.gl);
            unsafe {
                if let Some(u_model) = self.gl.get_uniform_location(vs.program(), "u_model") {
                    self.gl.uniform_matrix_4_f32_slice(
                        Some(&u_model),
                        false,
                        model_matrix.as_slice(),
                    );
                }
                if let Some(u_color) = self.gl.get_uniform_location(vs.program(), "u_color") {
                    // Rusty metal color
                    self.gl.uniform_4_f32(Some(&u_color), 0.8, 0.3, 0.1, 1.0);
                }
            }
        } else {
            self.shader.bind(&self.gl);
            unsafe {
                if let Some(u_model) = self
                    .gl
                    .get_uniform_location(self.shader.program(), "u_model")
                {
                    self.gl.uniform_matrix_4_f32_slice(
                        Some(&u_model),
                        false,
                        model_matrix.as_slice(),
                    );
                }
            }
        }

        if let Some(ref box_mesh) = self.vehicle_box_mesh {
            box_mesh.draw(&self.gl);
        }
    }

    /// Render a mesh from command
    fn render_mesh_command(
        &mut self,
        mesh_handle: &Handle<Mesh>,
        _material_handle: &Handle<Material>,
        transform: &Matrix4<f32>,
    ) {
        // Set model transform uniform
        unsafe {
            self.shader.bind(&self.gl);
            if let Some(u_model) = self
                .gl
                .get_uniform_location(self.shader.program(), "u_model")
            {
                self.gl
                    .uniform_matrix_4_f32_slice(Some(&u_model), false, transform.as_slice());
            }
        }
        // Fetch mesh from resource manager using mesh_handle
        // For now, use terrain_mesh as fallback for demonstration
        if let Some(ref m) = self.terrain_mesh {
            m.draw(&self.gl);
        }
    }

    /// Render a terrain chunk from command
    fn render_terrain_command(
        &mut self,
        _mesh_handle: &Handle<Mesh>,
        _material_handle: &Handle<Material>,
        transform: &Matrix4<f32>,
    ) {
        // Set model transform uniform
        unsafe {
            self.shader.bind(&self.gl);
            if let Some(u_model) = self
                .gl
                .get_uniform_location(self.shader.program(), "u_model")
            {
                self.gl
                    .uniform_matrix_4_f32_slice(Some(&u_model), false, transform.as_slice());
            }
        }
        // Fetch terrain mesh from resource manager using mesh_handle
        // For now, use terrain_mesh as fallback for demonstration
        if let Some(ref m) = self.terrain_mesh {
            m.draw(&self.gl);
        }
    }

    /// Render skybox from command
    fn render_skybox_command(&mut self, rotation: &Matrix4<f32>) {
        // Apply rotation to skybox rendering by passing rotation matrix to sky_shader
        unsafe {
            if let Some(ref ss) = self.sky_shader {
                ss.bind(&self.gl);
                if let Some(u_rotation) = self.gl.get_uniform_location(ss.program(), "u_rotation") {
                    self.gl.uniform_matrix_4_f32_slice(
                        Some(&u_rotation),
                        false,
                        rotation.as_slice(),
                    );
                }
            }
        }
        let _ = self.render_sky();
    }

    /// Render particle system from command
    fn render_particles_command(
        &mut self,
        _system_handle: &Handle<ParticleSystem>,
        transform: &Matrix4<f32>,
    ) {
        // Use built-in particle_system with transform applied
        // Full implementation would fetch particle system from resource manager using handle
        let view_proj = self.camera.projection_matrix() * self.camera.view_matrix();

        // Apply transform to particle system (model matrix for particle emission point)
        // ParticleSystem::render expects view_proj, we set model matrix in shader
        unsafe {
            self.shader.bind(&self.gl);
            if let Some(u_model) = self
                .gl
                .get_uniform_location(self.shader.program(), "u_model")
            {
                self.gl
                    .uniform_matrix_4_f32_slice(Some(&u_model), false, transform.as_slice());
            }
        }
        self.particle_system.render(&self.gl, view_proj);
    }

    /// Draw a debug line
    fn draw_debug_line(&mut self, start: [f32; 3], end: [f32; 3], color: [f32; 4]) {
        let from = Vector3::new(start[0], start[1], start[2]);
        let to = Vector3::new(end[0], end[1], end[2]);
        let col = [color[0], color[1], color[2]];
        self.debug_renderer.draw_line(from, to, col);
    }

    /// Draw a UI element
    fn draw_ui_element(
        &mut self,
        rect: [f32; 4],
        texture: Option<Handle<Texture>>,
        color: [f32; 4],
        depth: f32,
    ) {
        // Render UI quad using the HUD batch system
        // Uses hud_vao/hud_vbo for batched rendering
        self.hud_vertices.clear();

        let [x, y, w, h] = rect;
        // Add quad vertices for batched rendering (position + color + uv)
        // Vertex format: x, y, r, g, b, a, u, v (8 floats = 32 bytes per vertex)
        let vertices: [f32; 32] = [
            // Top-left
            x,
            y,
            color[0],
            color[1],
            color[2],
            color[3],
            0.0,
            0.0,
            // Top-right
            x + w,
            y,
            color[0],
            color[1],
            color[2],
            color[3],
            1.0,
            0.0,
            // Bottom-right
            x + w,
            y + h,
            color[0],
            color[1],
            color[2],
            color[3],
            1.0,
            1.0,
            // Bottom-left
            x,
            y + h,
            color[0],
            color[1],
            color[2],
            color[3],
            0.0,
            1.0,
        ];
        self.hud_vertices.extend_from_slice(&vertices);

        // Upload and draw if we have a VAO
        if let (Some(vao), Some(vbo)) = (self.hud_vao, self.hud_vbo) {
            unsafe {
                self.gl.bind_vertex_array(Some(vao));
                self.gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
                self.gl.buffer_data_u8_slice(
                    glow::ARRAY_BUFFER,
                    bytemuck::cast_slice(&self.hud_vertices),
                    glow::DYNAMIC_DRAW,
                );
                self.gl.draw_arrays(glow::TRIANGLE_FAN, 0, 4);
                self.gl.bind_vertex_array(None);
                self.gl.bind_buffer(glow::ARRAY_BUFFER, None);
            }
        }
    }

    // ========================================================================
    // Заглушки методов для 2D рендеринга (E0599 fix)
    // Эти методы используются HUD системой для отрисовки интерфейса
    // ========================================================================

    /// Draw a 2D line (for HUD elements)
    pub unsafe fn draw_line(
        &mut self,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        thickness: f32,
        color: [f32; 4],
    ) {
        // Инициализируем переиспользуемые буферы если еще не созданы
        if let Err(e) = self.init_primitive_buffers() {
            tracing::warn!("Failed to init primitive buffers: {}", e);
            return;
        }

        // Рисуем линию как тонкий прямоугольник
        let dx = x2 - x1;
        let dy = y2 - y1;
        let length = (dx * dx + dy * dy).sqrt();
        if length < 0.001 {
            return;
        }
        let angle = (dy / length).atan2(dx);
        let cos = angle.cos();
        let sin = angle.sin();

        // Рисуем прямоугольник повёрнутый вдоль линии
        let half_thickness = thickness / 2.0;
        let vertices: [f32; 8] = [
            x1 - sin * half_thickness,
            y1 - cos * half_thickness,
            x2 - sin * half_thickness,
            y2 - cos * half_thickness,
            x2 + sin * half_thickness,
            y2 + cos * half_thickness,
            x1 + sin * half_thickness,
            y1 + cos * half_thickness,
        ];
        let indices: [u32; 6] = [0, 1, 2, 0, 2, 3];

        // Используем переиспользуемые буферы вместо создания новых каждый кадр
        let vao = match self.primitive_vao {
            Some(v) => v,
            None => return,
        };
        let vbo = match self.primitive_vbo {
            Some(v) => v,
            None => return,
        };
        let _ibo = match self.primitive_ibo {
            Some(v) => v,
            None => return,
        };

        self.gl.bind_vertex_array(Some(vao));
        self.gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
        self.gl.buffer_data_u8_slice(
            glow::ARRAY_BUFFER,
            bytemuck::cast_slice(&vertices),
            glow::STREAM_DRAW,
        );
        self.gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(_ibo));
        self.gl.buffer_data_u8_slice(
            glow::ELEMENT_ARRAY_BUFFER,
            bytemuck::cast_slice(&indices),
            glow::STREAM_DRAW,
        );

        self.gl.enable_vertex_attrib_array(0);
        self.gl
            .vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 8, 0);

        if let Some(u) = self
            .gl
            .get_uniform_location(self.shader.program(), "u_use_solid_color")
        {
            self.gl.uniform_1_i32(Some(&u), 1);
        }
        if let Some(u) = self
            .gl
            .get_uniform_location(self.shader.program(), "u_color")
        {
            self.gl
                .uniform_4_f32(Some(&u), color[0], color[1], color[2], color[3]);
        }

        self.gl
            .draw_elements(glow::TRIANGLES, 6, glow::UNSIGNED_INT, 0);

        if let Some(u) = self
            .gl
            .get_uniform_location(self.shader.program(), "u_use_solid_color")
        {
            self.gl.uniform_1_i32(Some(&u), 0);
        }

        self.gl.delete_vertex_array(vao);
        self.gl.delete_buffer(vbo);
        self.gl.delete_buffer(ebo);
    }

    /// Draw a 2D circle (for map markers)
    pub unsafe fn draw_circle(
        &mut self,
        center_x: f32,
        center_y: f32,
        radius: f32,
        color: [f32; 4],
    ) {
        // Инициализируем переиспользуемые буферы если еще не созданы
        if let Err(e) = self.init_primitive_buffers() {
            tracing::warn!("Failed to init primitive buffers: {}", e);
            return;
        }

        const SEGMENTS: u32 = 32;
        let mut vertices: Vec<f32> = Vec::with_capacity(((SEGMENTS + 2) * 2) as usize);
        let mut indices: Vec<u32> = Vec::with_capacity((SEGMENTS * 3) as usize);

        // Центр круга
        vertices.push(center_x);
        vertices.push(center_y);

        // Вершины по окружности
        for i in 0..=SEGMENTS {
            let angle = 2.0 * std::f32::consts::PI * (i as f32) / (SEGMENTS as f32);
            let x = center_x + radius * angle.cos();
            let y = center_y + radius * angle.sin();
            vertices.push(x);
            vertices.push(y);
        }

        // Индексы для треугольного веера
        for i in 0..SEGMENTS {
            indices.push(0);
            indices.push(i + 1);
            indices.push(i + 2);
        }

        // Используем переиспользуемые буферы вместо создания новых каждый кадр
        let vao = match self.primitive_vao {
            Some(v) => v,
            None => return,
        };
        let vbo = match self.primitive_vbo {
            Some(v) => v,
            None => return,
        };
        let ibo = match self.primitive_ibo {
            Some(v) => v,
            None => return,
        };

        self.gl.bind_vertex_array(Some(vao));
        self.gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
        self.gl.buffer_data_u8_slice(
            glow::ARRAY_BUFFER,
            bytemuck::cast_slice(&vertices),
            glow::STREAM_DRAW,
        );
        self.gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(ibo));
        self.gl.buffer_data_u8_slice(
            glow::ELEMENT_ARRAY_BUFFER,
            bytemuck::cast_slice(&indices),
            glow::STREAM_DRAW,
        );

        self.gl.enable_vertex_attrib_array(0);
        self.gl
            .vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 8, 0);

        if let Some(u) = self
            .gl
            .get_uniform_location(self.shader.program(), "u_use_solid_color")
        {
            self.gl.uniform_1_i32(Some(&u), 1);
        }
        if let Some(u) = self
            .gl
            .get_uniform_location(self.shader.program(), "u_color")
        {
            self.gl
                .uniform_4_f32(Some(&u), color[0], color[1], color[2], color[3]);
        }

        self.gl
            .draw_elements(glow::TRIANGLES, indices.len() as i32, glow::UNSIGNED_INT, 0);

        if let Some(u) = self
            .gl
            .get_uniform_location(self.shader.program(), "u_use_solid_color")
        {
            self.gl.uniform_1_i32(Some(&u), 0);
        }

        // Не удаляем буферы - они переиспользуются
    }

    /// Draw a 2D triangle - публичная версия для HUD
    pub unsafe fn draw_triangle(
        &mut self,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        x3: f32,
        y3: f32,
        color: [f32; 4],
    ) {
        // Инициализируем переиспользуемые буферы если еще не созданы
        if let Err(e) = self.init_primitive_buffers() {
            tracing::warn!("Failed to init primitive buffers: {}", e);
            return;
        }

        let ortho =
            Matrix4::new_orthographic(0.0, self.width as f32, 0.0, self.height as f32, -1.0, 1.0);

        let vertices: [f32; 6] = [x1, y1, x2, y2, x3, y3];

        // Используем переиспользуемые буферы вместо создания новых каждый кадр
        let vao = match self.primitive_vao {
            Some(v) => v,
            None => return,
        };
        let vbo = match self.primitive_vbo {
            Some(v) => v,
            None => return,
        };

        self.gl.bind_vertex_array(Some(vao));
        self.gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
        self.gl.buffer_data_u8_slice(
            glow::ARRAY_BUFFER,
            bytemuck::cast_slice(&vertices),
            glow::STREAM_DRAW,
        );
        self.gl.enable_vertex_attrib_array(0);
        self.gl
            .vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 8, 0);

        if let Some(u) = self
            .gl
            .get_uniform_location(self.shader.program(), "u_use_solid_color")
        {
            self.gl.uniform_1_i32(Some(&u), 1);
        }
        if let Some(u) = self
            .gl
            .get_uniform_location(self.shader.program(), "u_color")
        {
            self.gl
                .uniform_4_f32(Some(&u), color[0], color[1], color[2], color[3]);
        }
        if let Some(u) = self
            .gl
            .get_uniform_location(self.shader.program(), "u_projection")
        {
            self.gl
                .uniform_matrix_4_f32_slice(Some(&u), false, ortho.as_slice());
        }

        self.gl.draw_arrays(glow::TRIANGLES, 0, 3);

        if let Some(u) = self
            .gl
            .get_uniform_location(self.shader.program(), "u_use_solid_color")
        {
            self.gl.uniform_1_i32(Some(&u), 0);
        }

        // Не удаляем буферы - они переиспользуются
    }

    // Старое приватное определение draw_triangle_internal удалено

    /// Граф-1: Создать процедурную bitmap font текстуру 128x128
    fn create_bitmap_font(gl: &Arc<Context>) -> Result<(Texture, HashMap<char, [f32; 4]>), String> {
        use std::collections::HashMap;
        // Создаём текстуру 128x128 с символами 8x8 в сетке 16x16
        let mut pixels = vec![255u8; 128 * 128 * 4]; // RGBA
        let mut font_chars = HashMap::new();

        // Простые глифы для ASCII 32-127 (96 символов)
        // Каждый символ 8x8 пикселей, сетка 16 колонок × 6 рядов = 96 мест
        for (idx, c) in (32..=127).enumerate() {
            let col = idx % 16;
            let row = idx / 16;
            let base_x = col * 8;
            let base_y = row * 8;

            // UV координаты для этого символа
            let u = col as f32 / 16.0;
            let v = row as f32 / 16.0;
            let w = 1.0 / 16.0;
            let h = 1.0 / 16.0;
            font_chars.insert(c as char, [u, v, w, h]);

            // Рисуем простой глиф (паттерн на основе кода символа)
            for dy in 0..8 {
                for dx in 0..8 {
                    let px = base_x + dx;
                    let py = base_y + dy;
                    let pidx = (py * 128 + px) * 4;

                    // Простой паттерн: некоторые пиксели чёрные, некоторые белые
                    let pattern = match c {
                        b'0'..=b'9' => (dx + dy) % 3 == 0,
                        b'A'..=b'Z' | b'a'..=b'z' => (dx * dy) % 2 == 0,
                        b' ' => false,
                        _ => (dx + dy) % 2 == 0,
                    };

                    if pattern {
                        pixels[pidx] = 0;
                        pixels[pidx + 1] = 0;
                        pixels[pidx + 2] = 0;
                        pixels[pidx + 3] = 255;
                    } else {
                        pixels[pidx] = 255;
                        pixels[pidx + 1] = 255;
                        pixels[pidx + 2] = 255;
                        pixels[pidx + 3] = 0;
                    }
                }
            }
        }

        // Попытка создать основную текстуру
        match Texture::from_rgba8(gl, 128, 128, &pixels) {
            Ok(texture) => Ok((texture, font_chars)),
            Err(e) => {
                warn!("Failed to create font texture, using fallback: {}", e);
                // Fallback: создать пустую текстуру 1x1
                match Texture::from_rgba8(gl, 1, 1, &[255, 255, 255, 255]) {
                    Ok(texture) => {
                        // Пустой font_chars для fallback
                        Ok((texture, HashMap::new()))
                    }
                    Err(e) => Err(format!("Failed to create fallback font texture: {}", e)),
                }
            }
        }
    }

    /// Set the terrain mesh for rendering
    pub fn set_terrain_mesh(&mut self, mesh: Mesh) {
        self.terrain_mesh = Some(mesh);
    }

    /// Set vehicle transform and HUD data
    pub fn set_vehicle_transform(&mut self, pos: Vector3<f32>, rot: UnitQuaternion<f32>) {
        self.vehicle_position = Some(pos);
        self.vehicle_rotation = Some(rot);
    }

    /// Set HUD data for rendering
    pub fn set_hud_data(&mut self, data: crate::ui::hud::VehicleHudData) {
        self.hud_data = Some(data);
    }

    // Weather and Day/Night cycle methods
    pub fn set_sky_color(&mut self, top: Vector3<f32>, horizon: Vector3<f32>) {
        self.sky_color_top = top;
        self.sky_color_horizon = horizon;
        self.update_sky_colors(top, horizon);
    }

    /// Граф-4: Обновить цвета неба в VAO
    fn update_sky_colors(&self, top: Vector3<f32>, horizon: Vector3<f32>) {
        unsafe {
            if let Some(vao) = self.sky_vao {
                self.gl.bind_vertex_array(Some(vao));
                // Bind the buffer before updating
                if let Some(vbo) = self.sky_vbo {
                    self.gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
                    // Обновить вершины с новыми цветами через buffer_sub_data
                    let verts: [f32; 30] = [
                        -1.0, -1.0, horizon.x, horizon.y, horizon.z, // bottom-left horizon
                        1.0, -1.0, horizon.x, horizon.y, horizon.z, // bottom-right horizon
                        1.0, 1.0, top.x, top.y, top.z, // top-right top
                        -1.0, -1.0, horizon.x, horizon.y, horizon.z, 1.0, 1.0, top.x, top.y, top.z,
                        -1.0, 1.0, top.x, top.y, top.z, // top-left top
                    ];
                    // Используем buffer_sub_data для обновления без пересоздания
                    self.gl.buffer_sub_data_u8_slice(
                        glow::ARRAY_BUFFER,
                        0,
                        bytemuck::cast_slice(&verts),
                    );
                    self.gl.bind_buffer(glow::ARRAY_BUFFER, None);
                }
            }
        }
    }

    pub fn set_sun_direction(&mut self, dir: Vector3<f32>) {
        self.sun_direction = dir;
    }

    pub fn set_ambient_intensity(&mut self, intensity: f32) {
        self.ambient_intensity = intensity.clamp(0.0, 1.0);
    }

    pub fn enable_vehicle_lights(&mut self, enable: bool) {
        self.vehicle_lights_enabled = enable;
    }

    /// Create a simple box mesh for the vehicle (temporary until GLTF loading works)
    pub fn create_vehicle_box_mesh(
        &mut self,
        half_extents: Vector3<f32>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Create a unit cube centered at origin, scaled by half_extents
        let hx = half_extents.x;
        let hy = half_extents.y;
        let hz = half_extents.z;

        // Cube vertices: 8 corners with normals
        let vertices: Vec<f32> = vec![
            // Front face (z = +hz)
            -hx, -hy, hz, 0.0, 0.0, 1.0, 0.0, 0.0, hx, -hy, hz, 0.0, 0.0, 1.0, 1.0, 0.0, hx, hy, hz,
            0.0, 0.0, 1.0, 1.0, 1.0, -hx, hy, hz, 0.0, 0.0, 1.0, 0.0, 1.0,
            // Back face (z = -hz)
            hx, -hy, -hz, 0.0, 0.0, -1.0, 0.0, 0.0, -hx, -hy, -hz, 0.0, 0.0, -1.0, 1.0, 0.0, -hx,
            hy, -hz, 0.0, 0.0, -1.0, 1.0, 1.0, hx, hy, -hz, 0.0, 0.0, -1.0, 0.0, 1.0,
            // Top face (y = +hy)
            -hx, hy, -hz, 0.0, 1.0, 0.0, 0.0, 0.0, hx, hy, -hz, 0.0, 1.0, 0.0, 1.0, 0.0, hx, hy, hz,
            0.0, 1.0, 0.0, 1.0, 1.0, -hx, hy, hz, 0.0, 1.0, 0.0, 0.0, 1.0,
            // Bottom face (y = -hy)
            -hx, -hy, hz, 0.0, -1.0, 0.0, 0.0, 0.0, hx, -hy, hz, 0.0, -1.0, 0.0, 1.0, 0.0, hx, -hy,
            -hz, 0.0, -1.0, 0.0, 1.0, 1.0, -hx, -hy, -hz, 0.0, -1.0, 0.0, 0.0, 1.0,
            // Right face (x = +hx)
            hx, -hy, -hz, 1.0, 0.0, 0.0, 0.0, 0.0, hx, hy, -hz, 1.0, 0.0, 0.0, 1.0, 0.0, hx, hy, hz,
            1.0, 0.0, 0.0, 1.0, 1.0, hx, -hy, hz, 1.0, 0.0, 0.0, 0.0, 1.0,
            // Left face (x = -hx)
            -hx, -hy, hz, -1.0, 0.0, 0.0, 0.0, 0.0, -hx, hy, hz, -1.0, 0.0, 0.0, 1.0, 0.0, -hx, hy,
            -hz, -1.0, 0.0, 0.0, 1.0, 1.0, -hx, -hy, -hz, -1.0, 0.0, 0.0, 0.0, 1.0,
        ];

        let indices: Vec<u32> = vec![
            0, 1, 2, 0, 2, 3, // Front
            4, 5, 6, 4, 6, 7, // Back
            8, 9, 10, 8, 10, 11, // Top
            12, 13, 14, 12, 14, 15, // Bottom
            16, 17, 18, 16, 18, 19, // Right
            20, 21, 22, 20, 22, 23, // Left
        ];

        match Mesh::new_raw(&self.gl, &vertices, &indices) {
            Ok(mesh) => self.vehicle_box_mesh = Some(mesh),
            Err(e) => tracing::warn!("Failed to create vehicle box mesh: {}", e),
        }
        Ok(())
    }

    pub fn render(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Очистка экрана голубым цветом (один раз за кадр)
        unsafe {
            self.gl.clear_color(0.4, 0.6, 0.9, 1.0);
            self.gl
                .clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
        }

        info!(target: "render", ">>> Renderer::render() called, menu_state={:?}", self.menu_state);

        // Update LOD system based on camera position
        self.lod_manager.update_all_lods(&self.camera.position);

        // Update texture streaming based on camera position
        self.texture_streaming
            .update_camera_position(nalgebra::Vector2::new(
                self.camera.position.x,
                self.camera.position.z,
            ));

        match self.menu_state {
            MenuState::Loading => self.render_loading_screen()?,
            MenuState::MainMenu => {
                info!(target: "render", "Rendering MainMenu state - calling render_main_menu()");
                self.render_main_menu()?;
            }
            MenuState::CitySelection => self.render_city_selection()?,
            MenuState::InGame => {
                self.render_game()?;
                self.render_sky()?; // Рендерим небо только в игровом режиме
            }
            MenuState::Paused => {
                self.render_game()?;
                self.render_sky()?;
                self.render_pause_overlay()?;
            }
            MenuState::WorldCreation => self.render_world_creation()?,
            MenuState::Settings => self.render_settings()?,
            MenuState::CharacterCreation => self.render_character_creation()?,
        }

        Ok(())
    }

    /// Обработка ввода мыши для меню
    pub fn handle_mouse_input(
        &mut self,
        state: winit::event::ElementState,
        button: winit::event::MouseButton,
    ) {
        use winit::event::{ElementState, MouseButton};

        if state != ElementState::Pressed {
            return;
        }

        let mouse_x = self.mouse_x;
        let mouse_y = self.height as f32 - self.mouse_y; // Инвертируем Y

        match self.menu_state {
            MenuState::MainMenu => {
                // Проверяем клики по кнопкам меню
                let button_width = 240.0;
                let button_height = 40.0;
                let center_x = self.width as f32 / 2.0;

                let is_hovered = |mx: f32, my: f32, y: f32| -> bool {
                    mx >= center_x - button_width / 2.0
                        && mx <= center_x + button_width / 2.0
                        && my >= y
                        && my <= y + button_height
                };

                if button == MouseButton::Left {
                    let new_game_y = self.height as f32 / 2.0 - 80.0;
                    if is_hovered(mouse_x, mouse_y, new_game_y) {
                        info!(target: "ui", "New Game clicked");
                        self.menu_state = MenuState::CharacterCreation;
                    }

                    let continue_y = self.height as f32 / 2.0 - 30.0;
                    if is_hovered(mouse_x, mouse_y, continue_y) {
                        info!(target: "ui", "Continue clicked");
                    }

                    let exit_y = self.height as f32 / 2.0 + 20.0;
                    if is_hovered(mouse_x, mouse_y, exit_y) {
                        info!(target: "ui", "Exit clicked");
                        std::process::exit(0);
                    }
                }
            }
            MenuState::CharacterCreation => {
                // Обработка ввода при создании персонажа
            }
            _ => {}
        }
    }

    pub fn update_camera_for_frame(
        &mut self,
        truck_position: Vector3<f32>,
        truck_rotation: UnitQuaternion<f32>,
    ) {
        self.camera.update_for_truck(truck_position, truck_rotation);
    }

    fn render_character_creation(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            self.gl.disable(glow::DEPTH_TEST);
            self.gl.clear_color(0.1, 0.1, 0.15, 1.0);
            self.gl.clear(glow::COLOR_BUFFER_BIT);
            let w = self.width as f32;
            let h = self.height as f32;
            self.draw_text(
                "СОЗДАНИЕ ПЕРСОНАЖА",
                w / 2.0 - 80.0,
                h / 2.0,
                1.5,
                [1.0, 1.0, 1.0, 1.0],
            );
        }
        Ok(())
    }

    fn render_loading_screen(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            self.gl.disable(glow::DEPTH_TEST);
            self.gl.clear_color(0.05, 0.05, 0.1, 1.0);
            self.gl.clear(glow::COLOR_BUFFER_BIT);
            // Центральная надпись (пока просто прямоугольник)
            let w = self.width as f32;
            let h = self.height as f32;
            self.draw_rect(
                w / 2.0 - 100.0,
                h / 2.0 - 30.0,
                200.0,
                60.0,
                [0.2, 0.4, 0.6, 0.9],
            );
            self.gl.enable(glow::DEPTH_TEST);
        }
        Ok(())
    }

    fn render_main_menu(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            self.gl.disable(glow::DEPTH_TEST);
            self.gl.enable(glow::BLEND);
            self.gl
                .blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);

            let w = self.width as f32;
            let h = self.height as f32;

            // Центральная панель
            self.draw_rect(
                w / 2.0 - 150.0,
                h / 2.0 - 120.0,
                300.0,
                240.0,
                [0.1, 0.1, 0.15, 0.9],
            );

            // Получаем позицию мыши для hover-эффектов
            // Y=0 вверху в обоих winit и ортографической проекции - инверсия не нужна
            let mouse_x = self.mouse_x;
            let mouse_y = self.mouse_y;

            let button_width = 240.0;
            let button_height = 40.0;
            let center_x = w / 2.0;

            // Функция для проверки hover
            let is_hovered = |mouse_x: f32, mouse_y: f32, y: f32| -> bool {
                mouse_x >= center_x - button_width / 2.0
                    && mouse_x <= center_x + button_width / 2.0
                    && mouse_y >= y
                    && mouse_y <= y + button_height
            };

            // Пункты меню с текстом и hover-эффектами
            // "Новая игра" — ЗЕЛЁНАЯ ЯРКАЯ (тест)
            let new_game_y = h / 2.0 - 80.0;
            let new_game_hover = is_hovered(mouse_x, mouse_y, new_game_y);
            let new_game_color = if new_game_hover {
                [0.0, 1.0, 0.0, 1.0] // ЯРКО-ЗЕЛЁНЫЙ при наведении
            } else {
                [0.0, 0.8, 0.0, 1.0] // ЯРКО-ЗЕЛЁНЫЙ без прозрачности
            };
            self.draw_rect(w / 2.0 - 120.0, new_game_y, 240.0, 40.0, new_game_color);
            self.draw_text(
                "НОВАЯ ИГРА",
                w / 2.0 - 60.0,
                new_game_y + 12.0,
                1.0,
                [1.0, 1.0, 1.0, 1.0],
            );

            // "Продолжить" — СИНЯЯ ЯРКАЯ (тест)
            let continue_y = h / 2.0 - 30.0;
            let continue_hover = is_hovered(mouse_x, mouse_y, continue_y);
            let continue_color = if continue_hover {
                [0.0, 0.5, 1.0, 1.0] // ЯРКО-СИНИЙ при наведении
            } else {
                [0.0, 0.3, 0.8, 1.0] // ЯРКО-СИНИЙ без прозрачности
            };
            self.draw_rect(w / 2.0 - 120.0, continue_y, 240.0, 40.0, continue_color);
            self.draw_text(
                "ПРОДОЛЖИТЬ",
                w / 2.0 - 60.0,
                continue_y + 12.0,
                1.0,
                [1.0, 1.0, 1.0, 1.0],
            );

            // "Настройки" — СЕРЫЙ ЯРКИЙ (тест)
            let settings_y = h / 2.0 + 20.0;
            let settings_hover = is_hovered(mouse_x, mouse_y, settings_y);
            let settings_color = if settings_hover {
                [0.7, 0.7, 0.7, 1.0] // СВЕТЛО-СЕРЫЙ при наведении
            } else {
                [0.5, 0.5, 0.5, 1.0] // СЕРЫЙ без прозрачности
            };
            self.draw_rect(w / 2.0 - 120.0, settings_y, 240.0, 40.0, settings_color);
            self.draw_text(
                "НАСТРОЙКИ",
                w / 2.0 - 55.0,
                settings_y + 12.0,
                1.0,
                [1.0, 1.0, 1.0, 1.0],
            );

            // "Выход" — КРАСНАЯ ЯРКАЯ (тест)
            let exit_y = h / 2.0 + 70.0;
            let exit_hover = is_hovered(mouse_x, mouse_y, exit_y);
            let exit_color = if exit_hover {
                [1.0, 0.0, 0.0, 1.0] // ЯРКО-КРАСНЫЙ при наведении
            } else {
                [0.8, 0.0, 0.0, 1.0] // КРАСНЫЙ без прозрачности
            };
            self.draw_rect(w / 2.0 - 120.0, exit_y, 240.0, 40.0, exit_color);
            self.draw_text(
                "ВЫХОД",
                w / 2.0 - 35.0,
                exit_y + 12.0,
                1.0,
                [1.0, 1.0, 1.0, 1.0],
            );

            // Отключаем blending после рендеринга UI
            self.gl.disable(glow::BLEND);
            self.gl.enable(glow::DEPTH_TEST);
        }
        Ok(())
    }

    fn render_city_selection(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            self.gl.disable(glow::DEPTH_TEST);
            self.gl.clear_color(0.05, 0.05, 0.1, 1.0);
            self.gl.clear(glow::COLOR_BUFFER_BIT);
            let w = self.width as f32;
            let h = self.height as f32;
            // Панель выбора города
            self.draw_rect(
                w / 2.0 - 200.0,
                h / 2.0 - 150.0,
                400.0,
                300.0,
                [0.1, 0.1, 0.15, 0.9],
            );
            self.gl.enable(glow::DEPTH_TEST);
        }
        Ok(())
    }

    /// Render a billboard (textured quad facing camera)
    fn render_billboard(&mut self, texture_id: u32, size: f32) {
        unsafe {
            // Bind billboard texture
            self.gl.active_texture(glow::TEXTURE0);
            if let Some(tid) = NonZeroU32::new(texture_id) {
                self.gl
                    .bind_texture(glow::TEXTURE_2D, Some(glow::NativeTexture(tid)));
            }

            // Simple billboard shader setup
            self.shader.bind(&self.gl);
            if let Some(loc) = self
                .gl
                .get_uniform_location(self.shader.program(), "u_texture")
            {
                self.gl.uniform_1_i32(Some(&loc), 0);
            }

            // Create billboard quad facing camera
            let cam_pos = self.camera.position;
            let up = Vector3::y();
            let forward = (cam_pos - self.camera.target).normalize();
            let right = up.cross(&forward).normalize();
            let up_billboard = forward.cross(&right).normalize();

            let half_size = size / 2.0;
            let corners = [
                // Position, normal, texcoord
                cam_pos - right * half_size - up_billboard * half_size, // bottom-left
                cam_pos + right * half_size - up_billboard * half_size, // bottom-right
                cam_pos + right * half_size + up_billboard * half_size, // top-right
                cam_pos - right * half_size + up_billboard * half_size, // top-left
            ];

            // Simple quad vertices
            let vertices: Vec<f32> = vec![
                corners[0].x,
                corners[0].y,
                corners[0].z,
                0.0,
                0.0,
                1.0,
                0.0,
                0.0,
                corners[1].x,
                corners[1].y,
                corners[1].z,
                0.0,
                0.0,
                1.0,
                1.0,
                0.0,
                corners[2].x,
                corners[2].y,
                corners[2].z,
                0.0,
                0.0,
                1.0,
                1.0,
                1.0,
                corners[0].x,
                corners[0].y,
                corners[0].z,
                0.0,
                0.0,
                1.0,
                0.0,
                0.0,
                corners[2].x,
                corners[2].y,
                corners[2].z,
                0.0,
                0.0,
                1.0,
                1.0,
                1.0,
                corners[3].x,
                corners[3].y,
                corners[3].z,
                0.0,
                0.0,
                1.0,
                0.0,
                1.0,
            ];

            // Draw using immediate mode (simple fallback)
            // In production, use VBO/VAO
            let vao = self.gl.create_vertex_array().ok();
            let vbo = self.gl.create_buffer().ok();

            if let (Some(vao), Some(vbo)) = (vao, vbo) {
                self.gl.bind_vertex_array(Some(vao));
                self.gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
                self.gl.buffer_data_u8_slice(
                    glow::ARRAY_BUFFER,
                    bytemuck::cast_slice(&vertices),
                    glow::STATIC_DRAW,
                );
                self.gl.enable_vertex_attrib_array(0);
                self.gl
                    .vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, 32, 0);
                self.gl.enable_vertex_attrib_array(1);
                self.gl
                    .vertex_attrib_pointer_f32(1, 3, glow::FLOAT, false, 32, 12);
                self.gl.enable_vertex_attrib_array(2);
                self.gl
                    .vertex_attrib_pointer_f32(2, 2, glow::FLOAT, false, 32, 24);

                self.gl.draw_arrays(glow::TRIANGLES, 0, 6);

                self.gl.bind_vertex_array(None);
                self.gl.bind_buffer(glow::ARRAY_BUFFER, None);
                self.gl.delete_vertex_array(vao);
                self.gl.delete_buffer(vbo);
            }

            self.gl.bind_texture(glow::TEXTURE_2D, None);
        }
    }

    fn render_game(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Render the actual game scene with proper OpenGL rendering

        // Обновляем цвета неба после рендера
        self.update_sky_colors(self.sky_color_top, self.sky_color_horizon);

        // === SPRINT 5: Texture streaming update ===
        self.texture_streaming
            .update_camera_position(nalgebra::Vector2::new(
                self.camera.position.x,
                self.camera.position.z,
            ));

        // Get visible objects from LOD system
        let camera_pos = self.camera.position;
        let visible_objects = self.lod_manager.get_objects_in_view(&camera_pos, 100.0);
        let visible_vec: Vec<_> = visible_objects.into_iter().collect();

        // Collect billboards to render after the loop to avoid borrow conflicts
        let mut billboards_to_render = Vec::new();

        // Use visible_vec for LOD-based culling instead of rendering all objects
        // Render each visible object using appropriate LOD model
        for (_index, lod_model) in visible_vec {
            match lod_model {
                crate::graphics::lod_system::LodModel::HighPoly { vertices, indices } => {
                    if !vertices.is_empty() && !indices.is_empty() {
                        // Generate a hash key for caching
                        let mesh_key = Mesh::generate_mesh_key_from_arc(vertices, indices);

                        // Get or create mesh from cache
                        let mesh = self.lod_mesh_cache.entry(mesh_key).or_insert_with(|| {
                            let vert_data: Vec<f32> = vertices
                                .iter()
                                .flat_map(|v| [v[0], v[1], v[2], 0.0, 0.0, 1.0, 0.0, 0.0])
                                .collect();
                            Mesh::new_with_normals(&self.gl, &vert_data, indices).unwrap_or_else(
                                |e| {
                                    warn!("Failed to create HighPoly mesh: {}", e);
                                    Mesh::empty(&self.gl)
                                },
                            )
                        });
                        mesh.draw(&self.gl);
                    }
                }
                crate::graphics::lod_system::LodModel::MediumPoly { vertices, indices } => {
                    if !vertices.is_empty() && !indices.is_empty() {
                        let mesh_key = Mesh::generate_mesh_key_from_arc(vertices, indices);

                        let mesh = self.lod_mesh_cache.entry(mesh_key).or_insert_with(|| {
                            let vert_data: Vec<f32> = vertices
                                .iter()
                                .flat_map(|v| [v[0], v[1], v[2], 0.0, 0.0, 1.0, 0.0, 0.0])
                                .collect();
                            Mesh::new_with_normals(&self.gl, &vert_data, indices).unwrap_or_else(
                                |e| {
                                    warn!("Failed to create MediumPoly mesh: {}", e);
                                    Mesh::empty(&self.gl)
                                },
                            )
                        });
                        mesh.draw(&self.gl);
                    }
                }
                crate::graphics::lod_system::LodModel::LowPoly { vertices, indices } => {
                    if !vertices.is_empty() && !indices.is_empty() {
                        let mesh_key = Mesh::generate_mesh_key_from_arc(vertices, indices);

                        let mesh = self.lod_mesh_cache.entry(mesh_key).or_insert_with(|| {
                            let vert_data: Vec<f32> = vertices
                                .iter()
                                .flat_map(|v| [v[0], v[1], v[2], 0.0, 0.0, 1.0, 0.0, 0.0])
                                .collect();
                            Mesh::new_with_normals(&self.gl, &vert_data, indices).unwrap_or_else(
                                |e| {
                                    warn!("Failed to create LowPoly mesh: {}", e);
                                    Mesh::empty(&self.gl)
                                },
                            )
                        });
                        mesh.draw(&self.gl);
                    }
                }
                crate::graphics::lod_system::LodModel::Billboard { texture_id, size } => {
                    // Collect billboards to render after the loop
                    billboards_to_render.push((*texture_id, *size));
                }
            }
        }

        // Render collected billboards - use into_iter() to take ownership
        for (texture_id, size) in billboards_to_render.into_iter() {
            self.render_billboard(texture_id, size);
        }

        // Use the shader
        self.shader.bind(&self.gl);

        // Set up view and projection matrices
        let projection = self.camera.projection_matrix();
        let view = self.camera.view_matrix();

        unsafe {
            // Set uniforms with safe handling - skip if uniform not found
            if let Some(u_projection) = self
                .gl
                .get_uniform_location(self.shader.program(), "u_projection")
            {
                self.gl.uniform_matrix_4_f32_slice(
                    Some(&u_projection),
                    false,
                    projection.as_slice(),
                );
            }
            if let Some(u_view) = self
                .gl
                .get_uniform_location(self.shader.program(), "u_view")
            {
                self.gl
                    .uniform_matrix_4_f32_slice(Some(&u_view), false, view.as_slice());
            }
            // SPRINT 5: Light direction from sun direction (shader expects u_light_dir)
            if let Some(u_light_dir) = self
                .gl
                .get_uniform_location(self.shader.program(), "u_light_dir")
            {
                self.gl.uniform_3_f32(
                    Some(&u_light_dir),
                    self.sun_direction.x,
                    self.sun_direction.y,
                    self.sun_direction.z,
                );
            }
            if let Some(u_view_pos) = self
                .gl
                .get_uniform_location(self.shader.program(), "u_view_pos")
            {
                self.gl.uniform_3_f32(
                    Some(&u_view_pos),
                    self.camera.position.x,
                    self.camera.position.y,
                    self.camera.position.z,
                );
            }
            // SPRINT 5: Light color affected by ambient intensity and weather
            if let Some(u_light_color) = self
                .gl
                .get_uniform_location(self.shader.program(), "u_light_color")
            {
                let light_intensity = self.ambient_intensity;
                self.gl.uniform_3_f32(
                    Some(&u_light_color),
                    light_intensity,
                    light_intensity,
                    light_intensity * 1.1,
                );
            }
            // Исп-7: Ambient intensity uniform
            if let Some(u) = self
                .gl
                .get_uniform_location(self.shader.program(), "u_ambient_intensity")
            {
                self.gl.uniform_1_f32(Some(&u), self.ambient_intensity);
            }
            // Terrain shader material uniforms
            if let Some(u) = self
                .gl
                .get_uniform_location(self.shader.program(), "u_ambient")
            {
                self.gl.uniform_3_f32(Some(&u), 0.2, 0.2, 0.2);
            }
            if let Some(u) = self
                .gl
                .get_uniform_location(self.shader.program(), "u_diffuse")
            {
                self.gl.uniform_3_f32(Some(&u), 0.7, 0.7, 0.7);
            }
            if let Some(u) = self
                .gl
                .get_uniform_location(self.shader.program(), "u_specular")
            {
                self.gl.uniform_3_f32(Some(&u), 0.1, 0.1, 0.1);
            }
            if let Some(u) = self
                .gl
                .get_uniform_location(self.shader.program(), "u_shininess")
            {
                self.gl.uniform_1_f32(Some(&u), 32.0);
            }
            // Задача 10: Fog uniforms
            if let Some(u) = self
                .gl
                .get_uniform_location(self.shader.program(), "u_fog_start")
            {
                self.gl.uniform_1_f32(Some(&u), 200.0);
            }
            if let Some(u) = self
                .gl
                .get_uniform_location(self.shader.program(), "u_fog_end")
            {
                self.gl.uniform_1_f32(Some(&u), 500.0);
            }
            if let Some(u) = self
                .gl
                .get_uniform_location(self.shader.program(), "u_fog_color")
            {
                self.gl.uniform_3_f32(
                    Some(&u),
                    self.sky_color_horizon.x,
                    self.sky_color_horizon.y,
                    self.sky_color_horizon.z,
                );
            }
            // Исп-7: Solid color disabled for terrain rendering
            if let Some(u) = self
                .gl
                .get_uniform_location(self.shader.program(), "u_use_solid_color")
            {
                self.gl.uniform_1_i32(Some(&u), 0);
            }
        }

        // === SPRINT 1: Render terrain mesh ===
        if let Some(ref terrain_mesh) = self.terrain_mesh {
            unsafe {
                // Set model matrix to identity for terrain
                if let Some(u_model) = self
                    .gl
                    .get_uniform_location(self.shader.program(), "u_model")
                {
                    let identity = Matrix4::identity();
                    self.gl
                        .uniform_matrix_4_f32_slice(Some(&u_model), false, identity.as_slice());
                }
            }
            terrain_mesh.draw(&self.gl);
        }

        // === SPRINT 1: Render vehicle as box ===
        if let (Some(pos), Some(rot)) = (self.vehicle_position, self.vehicle_rotation) {
            let model_matrix = rot.to_homogeneous().prepend_translation(&pos);

            // Задача 2: Использовать vehicle_shader если доступен
            if let Some(ref vs) = self.vehicle_shader {
                vs.bind(&self.gl);
                unsafe {
                    if let Some(u_model) = self.gl.get_uniform_location(vs.program(), "u_model") {
                        self.gl.uniform_matrix_4_f32_slice(
                            Some(&u_model),
                            false,
                            model_matrix.as_slice(),
                        );
                    }
                    if let Some(u_color) = self.gl.get_uniform_location(vs.program(), "u_color") {
                        // Ржавый металл цвет
                        self.gl.uniform_4_f32(Some(&u_color), 0.8, 0.3, 0.1, 1.0);
                    }
                }
            } else {
                self.shader.bind(&self.gl);
                unsafe {
                    if let Some(u_model) = self
                        .gl
                        .get_uniform_location(self.shader.program(), "u_model")
                    {
                        self.gl.uniform_matrix_4_f32_slice(
                            Some(&u_model),
                            false,
                            model_matrix.as_slice(),
                        );
                    }
                }
            }

            if let Some(ref box_mesh) = self.vehicle_box_mesh {
                box_mesh.draw(&self.gl);
            }
        }

        // Also render models from the traditional model system
        for (_, model) in &self.models {
            for mesh in &model.meshes {
                mesh.draw(&self.gl);
            }
        }

        // === SPRINT 5: Render debug lines and particles ===
        if self.debug_mode {
            let view_proj = self.camera.projection_matrix() * self.camera.view_matrix();
            self.debug_renderer.flush_to_gl(&self.gl, view_proj);
        }

        // Render particles
        let view_proj = self.camera.projection_matrix() * self.camera.view_matrix();
        self.particle_system.render(&self.gl, view_proj);

        // === SPRINT 2: Render HUD ===
        // HUD рисуется после основной сцены, без depth test
        self.render_hud()?;

        // Примечание: оверлей паузы теперь рендерится в render() для MenuState::Paused

        Ok(())
    }

    /// Ввод-2: Оверлей паузы
    fn render_pause_overlay(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            self.gl.disable(glow::DEPTH_TEST);

            let w = self.width as f32;
            let h = self.height as f32;

            // Полупрозрачный фон
            self.draw_rect(0.0, 0.0, w, h, [0.0, 0.0, 0.0, 0.5]);

            // Центральная панель
            self.draw_rect(
                w / 2.0 - 150.0,
                h / 2.0 - 100.0,
                300.0,
                200.0,
                [0.1, 0.1, 0.15, 0.95],
            );

            // Кнопка "Продолжить" (зелёная)
            let resume_y = h / 2.0 - 40.0;
            self.draw_rect(w / 2.0 - 120.0, resume_y, 240.0, 40.0, [0.2, 0.6, 0.2, 0.8]);
            self.draw_text(
                "ПРОДОЛЖИТЬ",
                w / 2.0 - 60.0,
                resume_y + 12.0,
                1.0,
                [1.0, 1.0, 1.0, 1.0],
            );

            // Кнопка "Настройки" (серая)
            let settings_y = h / 2.0 + 10.0;
            self.draw_rect(
                w / 2.0 - 120.0,
                settings_y,
                240.0,
                40.0,
                [0.3, 0.3, 0.3, 0.8],
            );
            self.draw_text(
                "НАСТРОЙКИ",
                w / 2.0 - 55.0,
                settings_y + 12.0,
                1.0,
                [1.0, 1.0, 1.0, 1.0],
            );

            // Кнопка "Выход в меню" (красная)
            let menu_y = h / 2.0 + 60.0;
            self.draw_rect(w / 2.0 - 120.0, menu_y, 240.0, 40.0, [0.6, 0.2, 0.2, 0.8]);
            self.draw_text(
                "В МЕНЮ",
                w / 2.0 - 40.0,
                menu_y + 12.0,
                1.0,
                [1.0, 1.0, 1.0, 1.0],
            );

            self.gl.enable(glow::DEPTH_TEST);
        }
        Ok(())
    }

    /// Задача 3: Рендерить небо (gradient quad)
    pub fn render_sky(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            self.gl.disable(glow::DEPTH_TEST);

            // Исп-2: Использовать sky_shader для рендеринга неба
            if let Some(ref ss) = self.sky_shader {
                ss.bind(&self.gl);
            } else {
                self.gl.enable(glow::DEPTH_TEST);
                return Ok(());
            }

            if let Some(vao) = self.sky_vao {
                self.gl.bind_vertex_array(Some(vao));
                self.gl.draw_arrays(glow::TRIANGLES, 0, 6);
            }

            // Вернуть основной шейдер
            self.shader.bind(&self.gl);

            self.gl.enable(glow::DEPTH_TEST);
        }
        Ok(())
    }

    /// Render HUD overlay (2D UI without depth test)
    pub fn render_hud(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        use crate::ui::hud::HudFlashElement;

        // Get HUD data from renderer's stored data (set by engine via set_hud_data)
        let hud_data = self
            .hud_data
            .clone()
            .unwrap_or_else(|| crate::ui::hud::VehicleHudData {
                speed_kmh: self
                    .vehicle_position
                    .map(|_| 65.0) // placeholder if no data
                    .unwrap_or(0.0),
                engine_rpm: 2200.0,
                engine_rpm_max: 3200.0,
                gear: crate::ui::hud::GearState::Drive(4),
                engine_running: true,
                fuel_level: 0.75,
                ..Default::default()
            });

        unsafe {
            // Disable depth test for 2D UI
            self.gl.disable(glow::DEPTH_TEST);

            // Use simple color for now (will use shader later)
            self.gl.use_program(Some(self.shader.program()));

            // Draw speed panel (bottom left rectangle)
            self.draw_rect(
                10.0,
                self.height as f32 - 60.0,
                200.0,
                50.0,
                [0.1, 0.1, 0.1, 0.8],
            );

            // Draw speed value (simple representation)
            let speed_text = format!("{:.0} km/h", hud_data.speed_kmh);
            // Text rendering will be added later with bitmap font
            self.draw_text(
                &speed_text,
                15.0,
                self.height as f32 - 55.0,
                1.5,
                [1.0, 1.0, 1.0, 1.0],
            );

            // Draw RPM bar
            let rpm_ratio = (hud_data.engine_rpm / hud_data.engine_rpm_max).min(1.0);
            let bar_width = 150.0 * rpm_ratio;
            self.draw_rect(
                20.0,
                self.height as f32 - 40.0,
                bar_width,
                10.0,
                [0.2, 0.8, 0.2, 1.0],
            );

            // Draw fuel bar
            let fuel_width = 100.0 * hud_data.fuel_level;
            self.draw_rect(
                20.0,
                self.height as f32 - 25.0,
                fuel_width,
                8.0,
                [0.8, 0.8, 0.2, 1.0],
            );

            // Draw wheel contact indicators (4 dots)
            for (i, &contact) in hud_data.wheel_contact.iter().enumerate() {
                let x = 250.0 + (i as f32 * 20.0);
                let y = self.height as f32 - 40.0;
                let color = if contact {
                    [0.0, 1.0, 0.0, 1.0]
                } else {
                    [1.0, 0.0, 0.0, 1.0]
                };
                // Using small rect instead of circle for simplicity
                self.draw_rect(x - 6.0, y - 6.0, 12.0, 12.0, color);
            }

            // Flash warning for low fuel
            if hud_data.fuel_reserve {
                self.draw_rect(
                    150.0,
                    self.height as f32 - 25.0,
                    100.0,
                    8.0,
                    [1.0, 0.0, 0.0, 1.0],
                );
            }

            // Граф-3: Мини-карта в правом верхнем углу
            self.render_minimap(&hud_data);

            // Ф1.5: Компас вверху экрана (400×24px)
            self.render_compass(&hud_data);

            // Re-enable depth test
            self.gl.enable(glow::DEPTH_TEST);
        }

        Ok(())
    }

    /// Ф1.5: Рендер компаса вверху экрана
    /// Полоска 400×24px, маркеры N/E/S/NE/SE/SW/NW + промежуточные
    /// Вращается по heading (yaw), стрелка к цели миссии
    fn render_compass(&mut self, hud_data: &crate::ui::hud::VehicleHudData) {
        unsafe {
            let compass_width = 400.0;
            let compass_height = 24.0;
            let compass_x = (self.width as f32 - compass_width) / 2.0;
            let compass_y = self.height as f32 - compass_height - 10.0;

            // Фон компаса (полупрозрачный чёрный)
            self.draw_rect(
                compass_x,
                compass_y,
                compass_width,
                compass_height,
                [0.15, 0.15, 0.15, 0.85],
            );

            // Рамка компаса
            self.draw_rect_border(
                compass_x,
                compass_y,
                compass_width,
                compass_height,
                1.5,
                [0.6, 0.6, 0.6, 1.0],
            );

            // Получаем heading игрока (из HUD данных или используем заглушку)
            let player_heading = hud_data.heading_degrees; // 0-360 градусов

            // Центр компаса
            let center_x = compass_x + compass_width / 2.0;
            let center_y = compass_y + compass_height / 2.0;

            // Рисуем основные направления (N, E, S, W)
            let mut directions = [
                (0.0, "N", [1.0, 0.2, 0.2, 1.0]),   // Север - красный
                (90.0, "E", [1.0, 1.0, 1.0, 0.7]),  // Восток
                (180.0, "S", [1.0, 1.0, 1.0, 0.7]), // Юг
                (270.0, "W", [1.0, 1.0, 1.0, 0.7]), // Запад
            ];

            for (angle_deg, label, color) in directions.iter() {
                // Вычисляем относительный угол с учётом поворота игрока
                let rel_angle = (angle_deg - player_heading + 720.0) % 360.0; // нормализуем 0-360

                // Преобразуем в позицию на полоске компаса (0-360 -> 0-width)
                let x_offset = ((rel_angle / 360.0) - 0.5) * compass_width;
                let x = center_x + x_offset;

                // Рисуем только если маркер в пределах видимости
                if x > compass_x + 10.0 && x < compass_x + compass_width - 10.0 {
                    // Увеличиваем яркость для ближайших направлений
                    let draw_color = if rel_angle < 30.0 || rel_angle > 330.0 {
                        [1.0, 1.0, 1.0, 1.0]
                    } else {
                        *color
                    };

                    // Рисуем метку направления
                    self.draw_text(label, x - 6.0, center_y + 4.0, 0.7, draw_color);

                    // Рисуем деление
                    let tick_height = if rel_angle < 30.0 || rel_angle > 330.0 {
                        10.0
                    } else {
                        5.0
                    };
                    let tick_color = if rel_angle < 30.0 || rel_angle > 330.0 {
                        [1.0, 0.5, 0.0, 1.0]
                    } else {
                        [0.7, 0.7, 0.7, 0.8]
                    };
                    self.draw_rect(
                        x - 1.0,
                        compass_y + compass_height - tick_height,
                        2.0,
                        tick_height,
                        tick_color,
                    );
                }
            }

            // Промежуточные направления (NE, SE, SW, NW)
            let intercardinal = [(45.0, "NE"), (135.0, "SE"), (225.0, "SW"), (315.0, "NW")];

            for (angle_deg, label) in intercardinal.iter() {
                let rel_angle = (angle_deg - player_heading + 720.0) % 360.0;
                let x_offset = ((rel_angle / 360.0) - 0.5) * compass_width;
                let x = center_x + x_offset;

                if x > compass_x + 15.0 && x < compass_x + compass_width - 15.0 {
                    self.draw_text(label, x - 10.0, center_y + 5.0, 0.5, [0.8, 0.8, 0.8, 0.6]);
                }
            }

            // Центральная метка (текущее направление)
            self.draw_rect(
                center_x - 2.0,
                compass_y + 2.0,
                4.0,
                6.0,
                [1.0, 1.0, 0.0, 1.0],
            );

            // Стрелка к цели миссии (если есть waypoint)
            if let Some(waypoint) = &hud_data.active_waypoint {
                let target_heading = waypoint.heading_degrees;
                let rel_angle = (target_heading - player_heading + 720.0) % 360.0;
                let x_offset = ((rel_angle / 360.0) - 0.5) * compass_width;
                let target_x = center_x + x_offset;

                if target_x > compass_x + 5.0 && target_x < compass_x + compass_width - 5.0 {
                    // Рисуем стрелку цели (треугольник вниз)
                    let arrow_color = [0.2, 1.0, 0.2, 1.0]; // зелёный

                    // Маленький треугольник
                    let arrow_size = 8.0;
                    self.draw_triangle(
                        target_x,
                        compass_y + compass_height - 2.0,
                        target_x - arrow_size / 2.0,
                        compass_y + compass_height - 2.0 - arrow_size,
                        target_x + arrow_size / 2.0,
                        compass_y + compass_height - 2.0 - arrow_size,
                        arrow_color,
                    );

                    // Дистанция до цели
                    let distance_km = waypoint.distance_meters / 1000.0;
                    let dist_text = if distance_km >= 1.0 {
                        format!("{:.1} км", distance_km)
                    } else {
                        format!("{:.0} м", waypoint.distance_meters)
                    };
                    self.draw_text(
                        &dist_text,
                        target_x - 20.0,
                        compass_y - 12.0,
                        0.5,
                        arrow_color,
                    );
                }
            }
        }
    }

    /// Draw a 2D rectangle (simple quad) with proper VAO/VBO implementation
    pub unsafe fn draw_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) {
        // Disable depth test and enable blending for UI
        self.gl.disable(glow::DEPTH_TEST);
        self.gl.enable(glow::BLEND);
        self.gl
            .blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);

        // Use UI shader if available, otherwise fall back to main shader
        let shader = self.ui_shader.as_ref().unwrap_or(&self.shader);

        // Create orthographic projection for UI with Y=0 at top (screen coordinates)
        let ortho = Matrix4::new_orthographic(
            0.0,
            self.width as f32, // left, right
            self.height as f32,
            0.0, // bottom, top (Y=0 at top for screen coords)
            -1.0,
            1.0,
        );

        // Set up vertices for a quad (2 triangles) with position, color, and uv
        // Format: pos (2) + color (4) + uv (2) = 8 floats per vertex
        let vertices: [f32; 32] = [
            // Position          // Color                 // UV
            x,
            y,
            color[0],
            color[1],
            color[2],
            color[3],
            0.0,
            0.0, // bottom-left
            x + width,
            y,
            color[0],
            color[1],
            color[2],
            color[3],
            1.0,
            0.0, // bottom-right
            x + width,
            y + height,
            color[0],
            color[1],
            color[2],
            color[3],
            1.0,
            1.0, // top-right
            x,
            y + height,
            color[0],
            color[1],
            color[2],
            color[3],
            0.0,
            1.0, // top-left
        ];

        let indices: [u32; 6] = [0, 1, 2, 0, 2, 3];

        // Create temporary VAO/VBO for the rect
        let vao = match self.gl.create_vertex_array() {
            Ok(v) => v,
            Err(_) => return,
        };
        let vbo = match self.gl.create_buffer() {
            Ok(v) => v,
            Err(_) => {
                self.gl.delete_vertex_array(vao);
                return;
            }
        };
        let ebo = match self.gl.create_buffer() {
            Ok(v) => v,
            Err(_) => {
                self.gl.delete_vertex_array(vao);
                self.gl.delete_buffer(vbo);
                return;
            }
        };

        self.gl.bind_vertex_array(Some(vao));

        self.gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
        self.gl.buffer_data_u8_slice(
            glow::ARRAY_BUFFER,
            bytemuck::cast_slice(&vertices),
            glow::STREAM_DRAW,
        );

        self.gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(ebo));
        self.gl.buffer_data_u8_slice(
            glow::ELEMENT_ARRAY_BUFFER,
            bytemuck::cast_slice(&indices),
            glow::STREAM_DRAW,
        );

        // Position attribute (location 0) - 2 floats per vertex
        self.gl.enable_vertex_attrib_array(0);
        self.gl
            .vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 32, 0);

        // Color attribute (location 1) - 4 floats per vertex
        self.gl.enable_vertex_attrib_array(1);
        self.gl
            .vertex_attrib_pointer_f32(1, 4, glow::FLOAT, false, 32, 8);

        // UV attribute (location 2) - 2 floats per vertex
        self.gl.enable_vertex_attrib_array(2);
        self.gl
            .vertex_attrib_pointer_f32(2, 2, glow::FLOAT, false, 32, 24);

        // Bind shader and set uniforms
        shader.bind(&self.gl);

        if let Some(u) = self.gl.get_uniform_location(shader.program(), "u_color") {
            self.gl
                .uniform_4_f32(Some(&u), color[0], color[1], color[2], color[3]);
        }
        if let Some(u) = self
            .gl
            .get_uniform_location(shader.program(), "u_projection")
        {
            self.gl
                .uniform_matrix_4_f32_slice(Some(&u), false, ortho.as_slice());
        }
        if let Some(u) = self
            .gl
            .get_uniform_location(shader.program(), "u_use_texture")
        {
            self.gl.uniform_1_i32(Some(&u), 0); // No texture for rect
        }

        // Draw the quad
        self.gl
            .draw_elements(glow::TRIANGLES, 6, glow::UNSIGNED_INT, 0);

        // Flush OpenGL commands immediately
        self.gl.flush();

        // Restore OpenGL state
        self.gl.disable(glow::BLEND);
        self.gl.enable(glow::DEPTH_TEST);

        // Cleanup
        self.gl.delete_vertex_array(vao);
        self.gl.delete_buffer(vbo);
        self.gl.delete_buffer(ebo);
    }

    /// Draw a 2D rectangle border
    pub unsafe fn draw_rect_border(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        thickness: f32,
        color: [f32; 4],
    ) {
        // Top
        self.draw_rect(x, y, width, thickness, color);
        // Bottom
        self.draw_rect(x, y + height - thickness, width, thickness, color);
        // Left
        self.draw_rect(x, y, thickness, height, color);
        // Right
        self.draw_rect(x + width - thickness, y, thickness, height, color);
    }

    // Старое приватное определение draw_triangle удалено - теперь используется публичная версия выше

    /// Граф-3: Рендеринг мини-карты
    fn render_minimap(&mut self, hud_data: &crate::ui::hud::VehicleHudData) {
        let map_size = 128.0;
        let margin = 10.0;
        let x = self.width as f32 - map_size - margin;
        let y = self.height as f32 - map_size - margin;

        unsafe {
            // Рамка мини-карты
            self.draw_rect(
                x - 2.0,
                y - 2.0,
                map_size + 4.0,
                map_size + 4.0,
                [0.0, 0.0, 0.0, 0.8],
            );

            // Фон (условная земля)
            self.draw_rect(x, y, map_size, map_size, [0.2, 0.3, 0.2, 1.0]);

            // Иконка игрока (треугольник по центру)
            let cx = x + map_size / 2.0;
            let cy = y + map_size / 2.0;
            let icon_size = 8.0;
            self.draw_triangle(
                cx,
                cy - icon_size,
                cx - icon_size / 2.0,
                cy + icon_size / 2.0,
                cx + icon_size / 2.0,
                cy + icon_size / 2.0,
                [1.0, 1.0, 0.0, 1.0],
            );

            // Если есть данные о грузе - показать маркер
            if hud_data.cargo_attached {
                self.draw_rect(
                    x + map_size - 20.0,
                    y + 5.0,
                    10.0,
                    10.0,
                    [0.0, 1.0, 1.0, 1.0],
                );
            }
        }
    }

    /// Граф-1: Draw text using bitmap font
    pub unsafe fn draw_text(&mut self, text: &str, x: f32, y: f32, size: f32, color: [f32; 4]) {
        let char_size = size; // 8x8 scaled
        let mut cursor_x = x;

        // Use UI shader if available, otherwise fall back to main shader
        let shader = self.ui_shader.as_ref().unwrap_or(&self.shader);

        // Create orthographic projection for UI with Y=0 at top (screen coordinates)
        let ortho = Matrix4::new_orthographic(
            0.0,
            self.width as f32,
            self.height as f32,
            0.0, // Y=0 at top
            -1.0,
            1.0,
        );

        // Bind font texture
        if let Some(ref tex) = self.font_texture {
            tex.bind(&self.gl);
        }

        // Bind shader and set uniforms
        shader.bind(&self.gl);

        if let Some(u) = self
            .gl
            .get_uniform_location(shader.program(), "u_projection")
        {
            self.gl
                .uniform_matrix_4_f32_slice(Some(&u), false, ortho.as_slice());
        }
        if let Some(u) = self.gl.get_uniform_location(shader.program(), "u_color") {
            self.gl
                .uniform_4_f32(Some(&u), color[0], color[1], color[2], color[3]);
        }
        if let Some(u) = self
            .gl
            .get_uniform_location(shader.program(), "u_use_texture")
        {
            self.gl.uniform_1_i32(Some(&u), 1); // use texture mode for font
        }

        for ch in text.chars() {
            if let Some(uv) = self.font_chars.get(&ch) {
                let [u, v, w, h] = *uv;

                // Draw textured quad for this character
                let vertices: [f32; 32] = [
                    // pos (2) + color (4) + uv (2) = 8 floats per vertex, 4 vertices
                    cursor_x,
                    y + char_size,
                    color[0],
                    color[1],
                    color[2],
                    color[3],
                    u,
                    v + h,
                    cursor_x + char_size,
                    y + char_size,
                    color[0],
                    color[1],
                    color[2],
                    color[3],
                    u + w,
                    v + h,
                    cursor_x + char_size,
                    y,
                    color[0],
                    color[1],
                    color[2],
                    color[3],
                    u + w,
                    v,
                    cursor_x,
                    y,
                    color[0],
                    color[1],
                    color[2],
                    color[3],
                    u,
                    v,
                ];

                let indices: [u32; 6] = [0, 1, 2, 0, 2, 3];

                let vao = self.gl.create_vertex_array().ok();
                let vbo = self.gl.create_buffer().ok();
                let ebo = self.gl.create_buffer().ok();

                if let (Some(vao), Some(vbo), Some(ebo)) = (vao, vbo, ebo) {
                    self.gl.bind_vertex_array(Some(vao));
                    self.gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
                    self.gl.buffer_data_u8_slice(
                        glow::ARRAY_BUFFER,
                        bytemuck::cast_slice(&vertices),
                        glow::STREAM_DRAW,
                    );
                    self.gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(ebo));
                    self.gl.buffer_data_u8_slice(
                        glow::ELEMENT_ARRAY_BUFFER,
                        bytemuck::cast_slice(&indices),
                        glow::STREAM_DRAW,
                    );

                    // pos: loc 0, 2 floats
                    self.gl.enable_vertex_attrib_array(0);
                    self.gl
                        .vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 32, 0);
                    // color: loc 1, 4 floats
                    self.gl.enable_vertex_attrib_array(1);
                    self.gl
                        .vertex_attrib_pointer_f32(1, 4, glow::FLOAT, false, 32, 8);
                    // uv: loc 2, 2 floats
                    self.gl.enable_vertex_attrib_array(2);
                    self.gl
                        .vertex_attrib_pointer_f32(2, 2, glow::FLOAT, false, 32, 24);

                    self.gl
                        .draw_elements(glow::TRIANGLES, 6, glow::UNSIGNED_INT, 0);

                    self.gl.delete_vertex_array(vao);
                    self.gl.delete_buffer(vbo);
                    self.gl.delete_buffer(ebo);
                }

                cursor_x += char_size;
            } else if ch == ' ' {
                cursor_x += char_size;
            }
        }

        // Reset shader state
        if let Some(u) = self
            .gl
            .get_uniform_location(shader.program(), "u_use_texture")
        {
            self.gl.uniform_1_i32(Some(&u), 0);
        }
    }

    /// Get renderer width
    pub fn get_width(&self) -> u32 {
        self.width
    }

    /// Get renderer height
    pub fn get_height(&self) -> u32 {
        self.height
    }

    fn render_world_creation(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            self.gl.disable(glow::DEPTH_TEST);
            self.gl.clear_color(0.05, 0.05, 0.1, 1.0);
            self.gl.clear(glow::COLOR_BUFFER_BIT);
            let w = self.width as f32;
            let h = self.height as f32;
            // Панель создания мира
            self.draw_rect(
                w / 2.0 - 200.0,
                h / 2.0 - 150.0,
                400.0,
                300.0,
                [0.1, 0.1, 0.15, 0.9],
            );
            self.gl.enable(glow::DEPTH_TEST);
        }
        Ok(())
    }

    fn render_settings(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            self.gl.disable(glow::DEPTH_TEST);
            self.gl.clear_color(0.05, 0.05, 0.1, 1.0);
            self.gl.clear(glow::COLOR_BUFFER_BIT);
            let w = self.width as f32;
            let h = self.height as f32;
            // Панель настроек
            self.draw_rect(
                w / 2.0 - 200.0,
                h / 2.0 - 150.0,
                400.0,
                300.0,
                [0.1, 0.1, 0.15, 0.9],
            );
            self.gl.enable(glow::DEPTH_TEST);
        }
        Ok(())
    }

    pub fn load_model(&mut self, name: String, model: Model) {
        self.models.insert(name, model);
    }

    /// Load a model from a Mesh (for OBJ files loaded via AssetLoader)
    pub fn load_mesh_as_model(&mut self, name: String, mesh: Mesh) {
        let model = Model {
            meshes: vec![mesh],
            textures: vec![],
        };
        self.models.insert(name, model);
    }

    pub fn render_model(&self, model_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(model) = self.models.get(model_name) {
            self.shader.bind(&self.gl);

            let projection = self.camera.projection_matrix();
            let view = self.camera.view_matrix();

            unsafe {
                // Set uniforms with safe handling - skip if uniform not found
                if let Some(u_projection) = self
                    .gl
                    .get_uniform_location(self.shader.program(), "u_projection")
                {
                    self.gl.uniform_matrix_4_f32_slice(
                        Some(&u_projection),
                        false,
                        projection.as_slice(),
                    );
                }
                if let Some(u_view) = self
                    .gl
                    .get_uniform_location(self.shader.program(), "u_view")
                {
                    self.gl
                        .uniform_matrix_4_f32_slice(Some(&u_view), false, view.as_slice());
                }
            }

            for mesh in &model.meshes {
                mesh.draw(&self.gl);
            }
        }

        Ok(())
    }

    pub fn set_camera(&mut self, camera: Camera) {
        self.camera = camera;
    }

    pub fn next_city(&mut self) {
        self.current_city_index = (self.current_city_index + 1) % 14; // 14 Siberian cities
    }

    pub fn prev_city(&mut self) {
        if self.current_city_index == 0 {
            self.current_city_index = 13;
        } else {
            self.current_city_index -= 1;
        }
    }
}

// Implement RendererTrait for Renderer
impl RendererTrait for Renderer {
    fn submit(&mut self, command: RenderCommand) {
        self.render_queue.submit(command);
    }

    fn flush_render(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.flush()
    }

    fn set_viewport(&mut self, x: i32, y: i32, width: u32, height: u32) {
        unsafe {
            self.gl.viewport(x, y, width as i32, height as i32);
        }
        self.width = width;
        self.height = height;
    }

    fn clear(&mut self, color: Option<[f32; 4]>, depth: bool, stencil: bool) {
        unsafe {
            let mut clear_bits = 0;
            if let Some([r, g, b, a]) = color {
                self.gl.clear_color(r, g, b, a);
                clear_bits |= glow::COLOR_BUFFER_BIT;
            }
            if depth {
                clear_bits |= glow::DEPTH_BUFFER_BIT;
            }
            if stencil {
                clear_bits |= glow::STENCIL_BUFFER_BIT;
            }
            if clear_bits != 0 {
                self.gl.clear(clear_bits);
            }
        }
    }

    fn camera(&self) -> &Camera {
        &self.camera
    }

    fn camera_mut(&mut self) -> &mut Camera {
        &mut self.camera
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        // Освобождаем переиспользуемые буферы для примитивов
        if let Some(vao) = self.primitive_vao {
            unsafe {
                self.gl.delete_vertex_array(vao);
            }
        }
        if let Some(vbo) = self.primitive_vbo {
            unsafe {
                self.gl.delete_buffer(vbo);
            }
        }
        if let Some(ibo) = self.primitive_ibo {
            unsafe {
                self.gl.delete_buffer(ibo);
            }
        }

        // Примечание: остальные ресурсы (шейдеры, текстуры, меши)
        // освобождаются через их собственные реализации Drop
    }
}
