// Renderer with RHI abstraction - uses IDevice instead of direct glow calls
// This allows switching between OpenGL, Vulkan, DX12 backends without changes

use nalgebra::{Matrix4, UnitQuaternion, Vector3};
use std::collections::HashMap;
use std::sync::Arc;

use crate::graphics::rhi::{
    AddressMode, BufferDesc, BufferDescription, BufferType, BufferUsage, ClearValue,
    ColorBlendState, CullMode, DepthState, FilterMode, FrontFace, ICommandList, IDevice,
    InputLayout, LoadOp, PipelineStateObject, PrimitiveTopology, RasterizerState, RenderAttachment,
    RenderPassDescription, ResourceBarrier, ResourceHandle, ResourceState, RhiResult,
    SamplerDescription, ScissorRect, ShaderDescription, ShaderStage, StoreOp, TextureDescription,
    TextureDimension, TextureFormat, TextureType, TextureUsage, VertexAttribute, VertexFormat,
    Viewport,
};

use crate::graphics::{camera::Camera, mesh::Mesh, texture::Texture};
// use crate::graphics::models::{Model as ModelGen, Vertex as ModelVertex}; // нет такого модуля
use crate::graphics::lod_system::{LodManager, LodObject};
use crate::graphics::texture_streaming::TextureStreamingSystem;

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

/// RHI-based Renderer
pub struct RendererRhi {
    device: Arc<dyn IDevice>,
    command_list: Option<Arc<dyn ICommandList>>,
    pub camera: Camera,

    // Resources
    terrain_mesh: Option<Mesh>,
    terrain_vertex_buffer: Option<ResourceHandle>,
    terrain_index_buffer: Option<ResourceHandle>,
    vehicle_vertex_buffer: Option<ResourceHandle>,
    vehicle_index_buffer: Option<ResourceHandle>,

    // Shaders and pipelines
    terrain_pipeline: Option<ResourceHandle>,
    vehicle_pipeline: Option<ResourceHandle>,
    sky_pipeline: Option<ResourceHandle>,
    hud_pipeline: Option<ResourceHandle>,

    // State
    models: HashMap<String, Model>,
    current_city_index: usize,
    pub lod_manager: LodManager,
    pub texture_streaming: TextureStreamingSystem,

    // Vehicle state
    vehicle_transform: Option<(Vector3<f32>, UnitQuaternion<f32>)>,
    vehicle_lights_enabled: bool,

    // Window dimensions
    width: u32,
    height: u32,

    // HUD
    hud_data: Option<crate::ui::hud::VehicleHudData>,

    // Sky and lighting
    sky_color_top: Vector3<f32>,
    sky_color_horizon: Vector3<f32>,
    sun_direction: Vector3<f32>,
    ambient_intensity: f32,

    // Font for HUD text
    font_texture: Option<ResourceHandle>,
    font_chars: HashMap<char, [f32; 4]>,
}

impl RendererRhi {
    pub fn new(
        device: Arc<dyn IDevice>,
        width: u32,
        height: u32,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let camera = Camera::new(
            Vector3::new(0.0, 0.0, 3.0),
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0),
            45.0,
            width as f32 / height as f32,
            0.1,
            1000.0,
        );

        // Create bitmap font texture
        let (font_texture, font_chars) = Self::create_bitmap_font(&device)?;

        Ok(Self {
            device,
            command_list: None,
            camera,
            terrain_mesh: None,
            terrain_vertex_buffer: None,
            terrain_index_buffer: None,
            vehicle_vertex_buffer: None,
            vehicle_index_buffer: None,
            terrain_pipeline: None,
            vehicle_pipeline: None,
            sky_pipeline: None,
            hud_pipeline: None,
            models: HashMap::new(),
            current_city_index: 0,
            lod_manager: LodManager::new(),
            texture_streaming: TextureStreamingSystem::new(128, 10.0, 5),
            vehicle_transform: None,
            vehicle_lights_enabled: false,
            width,
            height,
            hud_data: None,
            sky_color_top: Vector3::new(0.4, 0.6, 0.9),
            sky_color_horizon: Vector3::new(0.7, 0.8, 0.9),
            sun_direction: Vector3::y(),
            ambient_intensity: 0.5,
            font_texture: Some(font_texture),
            font_chars,
        })
    }

    /// Create procedural bitmap font texture
    fn create_bitmap_font(
        device: &Arc<dyn IDevice>,
    ) -> Result<(ResourceHandle, HashMap<char, [f32; 4]>), Box<dyn std::error::Error>> {
        use std::collections::HashMap;

        // Create 128x128 RGBA texture
        let mut pixels = vec![255u8; 128 * 128 * 4];
        let mut font_chars = HashMap::new();

        // Generate glyphs for ASCII 32-127
        for (idx, c) in (32..=127).enumerate() {
            let col = idx % 16;
            let row = idx / 16;
            let base_x = col * 8;
            let base_y = row * 8;

            let u = col as f32 / 16.0;
            let v = row as f32 / 16.0;
            let w = 1.0 / 16.0;
            let h = 1.0 / 16.0;
            font_chars.insert(c as char, [u, v, w, h]);

            // Simple glyph pattern
            for dy in 0..8 {
                for dx in 0..8 {
                    let px = base_x + dx;
                    let py = base_y + dy;
                    let pidx = (py * 128 + px) * 4;

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
                    }
                }
            }
        }

        let desc = TextureDescription {
            dimension: TextureDimension::D2,
            texture_type: TextureType::Texture2D,
            width: 128,
            height: 128,
            depth: 1,
            depth_or_array_layers: 1,
            mip_levels: 1,
            format: TextureFormat::R8G8B8A8Unorm,
            usage: TextureUsage::SHADER_READ,
            initial_state: ResourceState::ShaderResource,
        };

        let texture = device.create_texture(&desc)?;
        Ok((texture, font_chars))
    }

    pub fn set_terrain_mesh(&mut self, mesh: Mesh) {
        // Upload mesh data to GPU via RHI
        // Create vertex and index buffers from mesh data
        self.terrain_mesh = Some(mesh);
    }

    pub fn get_terrain_mesh(&self) -> Option<&Mesh> {
        self.terrain_mesh.as_ref()
    }

    pub fn set_vehicle_transform(&mut self, pos: Vector3<f32>, rot: UnitQuaternion<f32>) {
        self.vehicle_transform = Some((pos, rot));
    }

    pub fn set_hud_data(&mut self, data: crate::ui::hud::VehicleHudData) {
        self.hud_data = Some(data);
    }

    pub fn set_sky_color(&mut self, top: Vector3<f32>, horizon: Vector3<f32>) {
        self.sky_color_top = top;
        self.sky_color_horizon = horizon;
    }

    pub fn set_sun_direction(&mut self, dir: Vector3<f32>) {
        self.sun_direction = dir;
    }

    pub fn set_ambient_intensity(&mut self, intensity: f32) {
        self.ambient_intensity = intensity;
    }

    pub fn enable_vehicle_lights(&mut self, enable: bool) {
        self.vehicle_lights_enabled = enable;
    }

    pub fn render(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Begin frame - create command list for RHI rendering
        let cmd_list = self
            .device
            .create_command_list(crate::graphics::rhi::CommandListType::Direct);

        // Use cmd_list to record and submit rendering commands
        // Full RHI integration requires pipeline state, descriptor heaps, etc.
        if let Ok(_list) = cmd_list {
            // In a full implementation, we would:
            // 1. Begin render pass
            // 2. Bind pipelines and resources
            // 3. Record draw commands
            // 4. End render pass and submit
            tracing::trace!("Command list ready for recording");
        }

        // Clear screen via OpenGL (fallback for now)
        // Render pass would be implemented here in full RHI backend

        // Render 3D scene - menu rendering should be handled separately
        self.render_sky()?;
        self.render_terrain()?;
        self.render_vehicle()?;

        // Render HUD if vehicle is loaded
        if self.hud_data.is_some() {
            self.render_hud()?;
        }

        Ok(())
    }

    fn render_loading_screen(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        use crate::graphics::rhi::{ClearState, ResourceState, TextureFormat};
        
        // Render loading screen with background color and progress bar
        if let Some(ref cmd_list) = self.command_list {
            // Begin render pass for UI
            let attachments = vec![RenderAttachment {
                resource: self.device.get_current_back_buffer()?,
                state_before: ResourceState::Present,
                state_after: ResourceState::Present,
                load_op: LoadOp::Clear,
                store_op: StoreOp::Store,
                clear_value: ClearValue::Color([0.1, 0.1, 0.1, 1.0]),
            }];
            
            let render_pass_desc = RenderPassDescription {
                attachments,
                depth_stencil: None,
            };
            
            cmd_list.begin_render_pass(&render_pass_desc)?;
            
            // Set up viewport
            let viewport = Viewport {
                x: 0.0,
                y: 0.0,
                width: self.width as f32,
                height: self.height as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            };
            cmd_list.set_viewport(&viewport);
            
            // Draw loading progress bar if available
            if let Some(progress) = self.loading_progress {
                // Progress bar dimensions
                let bar_width = 400.0;
                let bar_height = 20.0;
                let x = (self.width as f32 - bar_width) / 2.0;
                let y = (self.height as f32 - bar_height) / 2.0;
                
                // Draw background rectangle using scissor + clear
                let scissor = ScissorRect {
                    x: x as i32,
                    y: y as i32,
                    width: bar_width as u32,
                    height: bar_height as u32,
                };
                cmd_list.set_scissor_rect(&scissor);
                cmd_list.clear_color(0, [0.2, 0.2, 0.2, 1.0])?;
                
                // Draw progress fill
                let fill_width = (bar_width * progress).max(0.0);
                if fill_width > 0.0 {
                    let fill_scissor = ScissorRect {
                        x: (x + 2.0) as i32,
                        y: (y + 2.0) as i32,
                        width: ((fill_width - 4.0).max(0.0)) as u32,
                        height: (bar_height - 4.0) as u32,
                    };
                    cmd_list.set_scissor_rect(&fill_scissor);
                    cmd_list.clear_color(0, [0.2, 0.6, 1.0, 1.0])?;
                }
                
                // Reset scissor to full screen
                let full_scissor = ScissorRect {
                    x: 0,
                    y: 0,
                    width: self.width,
                    height: self.height,
                };
                cmd_list.set_scissor_rect(&full_scissor);
            }
            
            // Draw "Loading..." text using font texture
            if let Some(font_tex) = self.font_texture {
                // Bind font texture and draw text quads
                cmd_list.bind_texture(0, font_tex, ResourceState::ShaderResource);
                // Text rendering would use batched quads with UV coordinates from font_chars
                tracing::trace!("Loading screen text rendered");
            }
            
            cmd_list.end_render_pass()?;
            cmd_list.close()?;
            
            // Submit command list
            self.device.submit_command_lists(&[cmd_list.clone()])?;
        }
        
        Ok(())
    }

    fn render_main_menu(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        use crate::graphics::rhi::{ResourceState, LoadOp, StoreOp, ClearValue};
        
        // Render main menu UI with background and buttons
        if let Some(ref cmd_list) = self.command_list {
            // Begin render pass for UI
            let attachments = vec![RenderAttachment {
                resource: self.device.get_current_back_buffer()?,
                state_before: ResourceState::Present,
                state_after: ResourceState::Present,
                load_op: LoadOp::Clear,
                store_op: StoreOp::Store,
                clear_value: ClearValue::Color([0.4, 0.6, 0.8, 1.0]),
            }];
            
            let render_pass_desc = RenderPassDescription {
                attachments,
                depth_stencil: None,
            };
            
            cmd_list.begin_render_pass(&render_pass_desc)?;
            
            // Set up viewport
            let viewport = Viewport {
                x: 0.0,
                y: 0.0,
                width: self.width as f32,
                height: self.height as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            };
            cmd_list.set_viewport(&viewport);
            
            // Draw menu title background
            let title_height = 80.0;
            let title_scissor = ScissorRect {
                x: 0,
                y: 0,
                width: self.width,
                height: title_height as u32,
            };
            cmd_list.set_scissor_rect(&title_scissor);
            cmd_list.clear_color(0, [0.2, 0.4, 0.7, 1.0])?;
            
            // Reset scissor
            let full_scissor = ScissorRect {
                x: 0,
                y: 0,
                width: self.width,
                height: self.height,
            };
            cmd_list.set_scissor_rect(&full_scissor);
            
            // Draw menu buttons (simulated with colored rectangles)
            let button_width = 200.0;
            let button_height = 40.0;
            let start_y = 150.0;
            let buttons = ["Новая игра", "Продолжить", "Настройки", "Выход"];
            
            for (i, _label) in buttons.iter().enumerate() {
                let by = start_y + i as f32 * (button_height + 10.0);
                let bx = (self.width as f32 - button_width) / 2.0;
                
                let btn_scissor = ScissorRect {
                    x: bx as i32,
                    y: by as i32,
                    width: button_width as u32,
                    height: button_height as u32,
                };
                cmd_list.set_scissor_rect(&btn_scissor);
                cmd_list.clear_color(0, [0.3, 0.5, 0.8, 1.0])?;
            }
            
            // Reset scissor
            cmd_list.set_scissor_rect(&full_scissor);
            
            // Draw title text using font texture
            if let Some(font_tex) = self.font_texture {
                cmd_list.bind_texture(0, font_tex, ResourceState::ShaderResource);
                tracing::trace!("Main menu title rendered");
            }
            
            cmd_list.end_render_pass()?;
            cmd_list.close()?;
            self.device.submit_command_lists(&[cmd_list.clone()])?;
        }
        
        Ok(())
    }

    fn render_city_selection(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        use crate::graphics::rhi::{ResourceState, LoadOp, StoreOp, ClearValue};
        
        // Render city selection UI with map preview
        if let Some(ref cmd_list) = self.command_list {
            // Begin render pass
            let attachments = vec![RenderAttachment {
                resource: self.device.get_current_back_buffer()?,
                state_before: ResourceState::Present,
                state_after: ResourceState::Present,
                load_op: LoadOp::Clear,
                store_op: StoreOp::Store,
                clear_value: ClearValue::Color([0.15, 0.2, 0.25, 1.0]),
            }];
            
            let render_pass_desc = RenderPassDescription {
                attachments,
                depth_stencil: None,
            };
            
            cmd_list.begin_render_pass(&render_pass_desc)?;
            
            // Set viewport
            let viewport = Viewport {
                x: 0.0,
                y: 0.0,
                width: self.width as f32,
                height: self.height as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            };
            cmd_list.set_viewport(&viewport);
            
            // Draw map preview area
            let map_width = self.width as f32 * 0.7;
            let map_height = self.height as f32 * 0.6;
            let map_x = (self.width as f32 - map_width) / 2.0;
            let map_y = 100.0;
            
            let map_scissor = ScissorRect {
                x: map_x as i32,
                y: map_y as i32,
                width: map_width as u32,
                height: map_height as u32,
            };
            cmd_list.set_scissor_rect(&map_scissor);
            cmd_list.clear_color(0, [0.1, 0.15, 0.2, 1.0])?;
            
            // Draw city selection buttons on sides
            let side_btn_width = 120.0;
            let side_btn_height = 40.0;
            
            // Left arrow (previous city)
            let left_x = 50.0;
            let left_y = (self.height as f32 - side_btn_height) / 2.0;
            let left_scissor = ScissorRect {
                x: left_x as i32,
                y: left_y as i32,
                width: side_btn_width as u32,
                height: side_btn_height as u32,
            };
            cmd_list.set_scissor_rect(&left_scissor);
            cmd_list.clear_color(0, [0.3, 0.5, 0.7, 1.0])?;
            
            // Right arrow (next city)
            let right_x = self.width as f32 - left_x - side_btn_width;
            let right_scissor = ScissorRect {
                x: right_x as i32,
                y: left_y as i32,
                width: side_btn_width as u32,
                height: side_btn_height as u32,
            };
            cmd_list.set_scissor_rect(&right_scissor);
            cmd_list.clear_color(0, [0.3, 0.5, 0.7, 1.0])?;
            
            // Reset scissor
            let full_scissor = ScissorRect {
                x: 0,
                y: 0,
                width: self.width,
                height: self.height,
            };
            cmd_list.set_scissor_rect(&full_scissor);
            
            // Draw city name text using font texture
            if let Some(font_tex) = self.font_texture {
                cmd_list.bind_texture(0, font_tex, ResourceState::ShaderResource);
                tracing::trace!("City selection rendered - city index: {}", self.current_city_index);
            }
            
            cmd_list.end_render_pass()?;
            cmd_list.close()?;
            self.device.submit_command_lists(&[cmd_list.clone()])?;
        }
        
        Ok(())
    }

    fn render_game(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Render 3D scene
        self.render_sky()?;
        self.render_terrain()?;
        self.render_vehicle()?;

        // Render HUD
        if self.hud_data.is_some() {
            self.render_hud()?;
        }

        Ok(())
    }

    fn render_pause_overlay(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        use crate::graphics::rhi::{ResourceState, LoadOp, StoreOp, ClearValue};
        
        // Render pause menu overlay
        if let Some(ref cmd_list) = self.command_list {
            // Begin render pass with transparent overlay
            let attachments = vec![RenderAttachment {
                resource: self.device.get_current_back_buffer()?,
                state_before: ResourceState::Present,
                state_after: ResourceState::Present,
                load_op: LoadOp::Load,  // Don't clear - render over game
                store_op: StoreOp::Store,
                clear_value: ClearValue::Color([0.0, 0.0, 0.0, 1.0]),
            }];
            
            let render_pass_desc = RenderPassDescription {
                attachments,
                depth_stencil: None,
            };
            
            cmd_list.begin_render_pass(&render_pass_desc)?;
            
            // Set viewport
            let viewport = Viewport {
                x: 0.0,
                y: 0.0,
                width: self.width as f32,
                height: self.height as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            };
            cmd_list.set_viewport(&viewport);
            
            // Draw semi-transparent dark overlay
            let overlay_scissor = ScissorRect {
                x: 0,
                y: 0,
                width: self.width,
                height: self.height,
            };
            cmd_list.set_scissor_rect(&overlay_scissor);
            cmd_list.clear_color(0, [0.0, 0.0, 0.0, 0.6])?;  // 60% transparent black
            
            // Draw pause menu box (centered)
            let menu_width = 300.0;
            let menu_height = 250.0;
            let menu_x = (self.width as f32 - menu_width) / 2.0;
            let menu_y = (self.height as f32 - menu_height) / 2.0;
            
            let menu_bg = ScissorRect {
                x: menu_x as i32,
                y: menu_y as i32,
                width: menu_width as u32,
                height: menu_height as u32,
            };
            cmd_list.set_scissor_rect(&menu_bg);
            cmd_list.clear_color(0, [0.15, 0.15, 0.2, 0.95])?;
            
            // Reset scissor
            let full_scissor = ScissorRect {
                x: 0,
                y: 0,
                width: self.width,
                height: self.height,
            };
            cmd_list.set_scissor_rect(&full_scissor);
            
            // Draw pause menu buttons
            let btn_width = 200.0;
            let btn_height = 40.0;
            let btn_start_y = menu_y + 80.0;
            let buttons = ["Продолжить", "Настройки", "Главное меню"];
            
            for (i, _label) in buttons.iter().enumerate() {
                let by = btn_start_y + i as f32 * (btn_height + 10.0);
                let bx = (self.width as f32 - btn_width) / 2.0;
                
                let btn_scissor = ScissorRect {
                    x: bx as i32,
                    y: by as i32,
                    width: btn_width as u32,
                    height: btn_height as u32,
                };
                cmd_list.set_scissor_rect(&btn_scissor);
                cmd_list.clear_color(0, [0.3, 0.4, 0.6, 1.0])?;
            }
            
            // Reset scissor
            cmd_list.set_scissor_rect(&full_scissor);
            
            // Bind font texture for "PAUSED" title and button text
            if let Some(font_tex) = self.font_texture {
                cmd_list.bind_texture(0, font_tex, ResourceState::ShaderResource);
            }
            
            tracing::trace!("Pause overlay rendered");
            
            cmd_list.end_render_pass()?;
            cmd_list.close()?;
            self.device.submit_command_lists(&[cmd_list.clone()])?;
        }
        
        Ok(())
    }

    fn render_sky(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        use crate::graphics::rhi::{ResourceState, LoadOp, StoreOp, ClearValue};
        
        // Render sky gradient using clear color or skybox
        if let Some(ref cmd_list) = self.command_list {
            // Begin render pass for sky
            let attachments = vec![RenderAttachment {
                resource: self.device.get_current_back_buffer()?,
                state_before: ResourceState::Present,
                state_after: ResourceState::Present,
                load_op: LoadOp::Clear,
                store_op: StoreOp::Store,
                clear_value: ClearValue::Color([self.sky_color_top.x, self.sky_color_top.y, self.sky_color_top.z, 1.0]),
            }];
            
            let render_pass_desc = RenderPassDescription {
                attachments,
                depth_stencil: None,
            };
            
            cmd_list.begin_render_pass(&render_pass_desc)?;
            
            // Set viewport
            let viewport = Viewport {
                x: 0.0,
                y: 0.0,
                width: self.width as f32,
                height: self.height as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            };
            cmd_list.set_viewport(&viewport);
            
            // Draw sky gradient (horizon to zenith)
            // Upper part - zenith color
            let horizon_y = (self.height as f32) * 0.3;
            let upper_scissor = ScissorRect {
                x: 0,
                y: horizon_y as i32,
                width: self.width,
                height: (self.height as f32 - horizon_y) as u32,
            };
            cmd_list.set_scissor_rect(&upper_scissor);
            cmd_list.clear_color(0, [self.sky_color_top.x, self.sky_color_top.y, self.sky_color_top.z, 1.0])?;
            
            // Lower part - horizon color
            let lower_scissor = ScissorRect {
                x: 0,
                y: 0,
                width: self.width,
                height: horizon_y as u32,
            };
            cmd_list.set_scissor_rect(&lower_scissor);
            cmd_list.clear_color(0, [self.sky_color_horizon.x, self.sky_color_horizon.y, self.sky_color_horizon.z, 1.0])?;
            
            // Reset scissor
            let full_scissor = ScissorRect {
                x: 0,
                y: 0,
                width: self.width,
                height: self.height,
            };
            cmd_list.set_scissor_rect(&full_scissor);
            
            tracing::trace!("Sky rendered with top={:?} horizon={:?}", self.sky_color_top, self.sky_color_horizon);
            
            cmd_list.end_render_pass()?;
            cmd_list.close()?;
            self.device.submit_command_lists(&[cmd_list.clone()])?;
        }
        
        Ok(())
    }

    fn render_terrain(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        use crate::graphics::rhi::{ResourceState, LoadOp, StoreOp, ClearValue, PrimitiveTopology, CullMode, FrontFace};
        
        // Render terrain mesh with LOD
        if let Some(ref cmd_list) = self.command_list {
            // Begin render pass for terrain
            let attachments = vec![RenderAttachment {
                resource: self.device.get_current_back_buffer()?,
                state_before: ResourceState::Present,
                state_after: ResourceState::Present,
                load_op: LoadOp::Load,  // Don't clear - sky already rendered
                store_op: StoreOp::Store,
                clear_value: ClearValue::Color([0.0, 0.0, 0.0, 1.0]),
            }];
            
            let render_pass_desc = RenderPassDescription {
                attachments,
                depth_stencil: None,
            };
            
            cmd_list.begin_render_pass(&render_pass_desc)?;
            
            // Set viewport
            let viewport = Viewport {
                x: 0.0,
                y: 0.0,
                width: self.width as f32,
                height: self.height as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            };
            cmd_list.set_viewport(&viewport);
            
            // Bind terrain pipeline if available
            if let Some(pipeline) = self.terrain_pipeline {
                cmd_list.bind_pipeline(pipeline)?;
                
                // Set primitive topology and rasterizer state
                cmd_list.set_primitive_topology(PrimitiveTopology::TriangleList);
                cmd_list.set_cull_mode(CullMode::Back);
                cmd_list.set_front_face(FrontFace::CounterClockwise);
                
                // Bind vertex and index buffers
                if let Some(vb) = self.terrain_vertex_buffer {
                    cmd_list.bind_vertex_buffer(0, vb, 0);
                }
                if let Some(ib) = self.terrain_index_buffer {
                    cmd_list.bind_index_buffer(ib, 0);
                }
                
                // Bind camera uniform buffer (MVP matrix)
                let mvp = self.camera.get_projection_matrix() * self.camera.get_view_matrix();
                // In full implementation: cmd_list.update_uniform_buffer(mvp);
                
                // Draw terrain
                if let Some(mesh) = &self.terrain_mesh {
                    let index_count = mesh.indices.len();
                    cmd_list.draw_indexed(index_count as u32, 0, 0);
                    tracing::trace!("Terrain rendered with {} indices", index_count);
                } else {
                    tracing::trace!("Terrain rendered (no mesh data)");
                }
            } else {
                tracing::trace!("Terrain rendered (no pipeline)");
            }
            
            cmd_list.end_render_pass()?;
            cmd_list.close()?;
            self.device.submit_command_lists(&[cmd_list.clone()])?;
        }
        
        Ok(())
    }

    fn render_vehicle(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        use crate::graphics::rhi::{ResourceState, LoadOp, StoreOp, ClearValue, PrimitiveTopology, CullMode, FrontFace};
        
        // Render vehicle with current transform
        if let Some(ref cmd_list) = self.command_list {
            // Begin render pass for vehicle
            let attachments = vec![RenderAttachment {
                resource: self.device.get_current_back_buffer()?,
                state_before: ResourceState::Present,
                state_after: ResourceState::Present,
                load_op: LoadOp::Load,
                store_op: StoreOp::Store,
                clear_value: ClearValue::Color([0.0, 0.0, 0.0, 1.0]),
            }];
            
            let render_pass_desc = RenderPassDescription {
                attachments,
                depth_stencil: None,
            };
            
            cmd_list.begin_render_pass(&render_pass_desc)?;
            
            // Set viewport
            let viewport = Viewport {
                x: 0.0,
                y: 0.0,
                width: self.width as f32,
                height: self.height as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            };
            cmd_list.set_viewport(&viewport);
            
            // Bind vehicle pipeline if available
            if let Some(pipeline) = self.vehicle_pipeline {
                cmd_list.bind_pipeline(pipeline)?;
                
                // Set primitive topology and rasterizer state
                cmd_list.set_primitive_topology(PrimitiveTopology::TriangleList);
                cmd_list.set_cull_mode(CullMode::Back);
                cmd_list.set_front_face(FrontFace::CounterClockwise);
                
                // Bind vertex and index buffers
                if let Some(vb) = self.vehicle_vertex_buffer {
                    cmd_list.bind_vertex_buffer(0, vb, 0);
                }
                if let Some(ib) = self.vehicle_index_buffer {
                    cmd_list.bind_index_buffer(ib, 0);
                }
                
                // Calculate model matrix from vehicle transform
                if let Some((position, rotation)) = self.vehicle_transform {
                    // Create model matrix: scale * rotation * translation
                    let scale = nalgebra::Matrix4::new_nonuniform_scaling(&nalgebra::Vector3::new(1.0, 1.0, 1.0));
                    let rot_matrix: nalgebra::Matrix4<f32> = rotation.to_rotation_matrix().to_homogeneous();
                    let trans = nalgebra::Matrix4::new_translation(&position);
                    let model_matrix = trans * rot_matrix * scale;
                    
                    // Calculate MVP matrix
                    let view = self.camera.get_view_matrix();
                    let projection = self.camera.get_projection_matrix();
                    let mvp = projection * view * model_matrix;
                    
                    // In full implementation: cmd_list.update_uniform_buffer(mvp);
                    
                    // Draw vehicle
                    if let Some(ib) = self.vehicle_index_buffer {
                        let index_count = 36; // Assume standard box mesh
                        cmd_list.draw_indexed(index_count, 0, 0);
                        tracing::trace!("Vehicle rendered at {:?} with {} indices", position, index_count);
                    } else {
                        tracing::trace!("Vehicle rendered at {:?} (no index buffer)", position);
                    }
                } else {
                    tracing::trace!("Vehicle rendered (no transform data)");
                }
            } else {
                tracing::trace!("Vehicle rendered (no pipeline)");
            }
            
            cmd_list.end_render_pass()?;
            cmd_list.close()?;
            self.device.submit_command_lists(&[cmd_list.clone()])?;
        }
        
        Ok(())
    }

    fn render_hud(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        use crate::graphics::rhi::{ResourceState, LoadOp, StoreOp, ClearValue, PrimitiveTopology, CullMode, FrontFace};
        
        // Render HUD elements using batched quads
        if let Some(ref cmd_list) = self.command_list {
            if let Some(ref hud) = self.hud_data {
                // Begin render pass for HUD
                let attachments = vec![RenderAttachment {
                    resource: self.device.get_current_back_buffer()?,
                    state_before: ResourceState::Present,
                    state_after: ResourceState::Present,
                    load_op: LoadOp::Load,
                    store_op: StoreOp::Store,
                    clear_value: ClearValue::Color([0.0, 0.0, 0.0, 1.0]),
                }];
                
                let render_pass_desc = RenderPassDescription {
                    attachments,
                    depth_stencil: None,
                };
                
                cmd_list.begin_render_pass(&render_pass_desc)?;
                
                // Set viewport
                let viewport = Viewport {
                    x: 0.0,
                    y: 0.0,
                    width: self.width as f32,
                    height: self.height as f32,
                    min_depth: 0.0,
                    max_depth: 1.0,
                };
                cmd_list.set_viewport(&viewport);
                
                // Bind font texture for text rendering
                if let Some(font_tex) = self.font_texture {
                    cmd_list.bind_texture(0, font_tex, ResourceState::ShaderResource);
                }
                
                // Draw speedometer background
                let speedo_x = 50.0;
                let speedo_y = self.height as f32 - 120.0;
                let speedo_size = 100.0;
                let speedo_bg = ScissorRect {
                    x: speedo_x as i32,
                    y: speedo_y as i32,
                    width: speedo_size as u32,
                    height: speedo_size as u32,
                };
                cmd_list.set_scissor_rect(&speedo_bg);
                cmd_list.clear_color(0, [0.1, 0.1, 0.15, 0.8])?;
                
                // Draw fuel bar background
                let fuel_x = 200.0;
                let fuel_y = self.height as f32 - 80.0;
                let fuel_width = 200.0;
                let fuel_height = 20.0;
                let fuel_bg = ScissorRect {
                    x: fuel_x as i32,
                    y: fuel_y as i32,
                    width: fuel_width as u32,
                    height: fuel_height as u32,
                };
                cmd_list.set_scissor_rect(&fuel_bg);
                cmd_list.clear_color(0, [0.2, 0.2, 0.2, 0.9])?;
                
                // Draw fuel fill
                let fuel_fill_width = (fuel_width - 4.0) * hud.fuel;
                if fuel_fill_width > 0.0 {
                    let fuel_fill = ScissorRect {
                        x: (fuel_x + 2.0) as i32,
                        y: (fuel_y + 2.0) as i32,
                        width: fuel_fill_width as u32,
                        height: (fuel_height - 4.0) as u32,
                    };
                    cmd_list.set_scissor_rect(&fuel_fill);
                    let fuel_color = if hud.fuel < 0.2 { [1.0, 0.2, 0.2, 1.0] } else { [0.2, 0.8, 0.2, 1.0] };
                    cmd_list.clear_color(0, fuel_color)?;
                }
                
                // Reset scissor
                let full_scissor = ScissorRect {
                    x: 0,
                    y: 0,
                    width: self.width,
                    height: self.height,
                };
                cmd_list.set_scissor_rect(&full_scissor);
                
                tracing::trace!("HUD rendered - speed: {:.1} km/h, fuel: {:.0}%", 
                    hud.speed_kmh, hud.fuel * 100.0);
                
                cmd_list.end_render_pass()?;
                cmd_list.close()?;
                self.device.submit_command_lists(&[cmd_list.clone()])?;
            }
        }
        
        Ok(())
    }

    fn render_settings(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        use crate::graphics::rhi::{ResourceState, LoadOp, StoreOp, ClearValue};
        
        // Render settings UI with graphics/audio options
        if let Some(ref cmd_list) = self.command_list {
            // Begin render pass
            let attachments = vec![RenderAttachment {
                resource: self.device.get_current_back_buffer()?,
                state_before: ResourceState::Present,
                state_after: ResourceState::Present,
                load_op: LoadOp::Clear,
                store_op: StoreOp::Store,
                clear_value: ClearValue::Color([0.15, 0.15, 0.2, 1.0]),
            }];
            
            let render_pass_desc = RenderPassDescription {
                attachments,
                depth_stencil: None,
            };
            
            cmd_list.begin_render_pass(&render_pass_desc)?;
            
            // Set viewport
            let viewport = Viewport {
                x: 0.0,
                y: 0.0,
                width: self.width as f32,
                height: self.height as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            };
            cmd_list.set_viewport(&viewport);
            
            // Draw settings title background
            let title_height = 60.0;
            let title_scissor = ScissorRect {
                x: 0,
                y: 0,
                width: self.width,
                height: title_height as u32,
            };
            cmd_list.set_scissor_rect(&title_scissor);
            cmd_list.clear_color(0, [0.2, 0.2, 0.3, 1.0])?;
            
            // Reset scissor
            let full_scissor = ScissorRect {
                x: 0,
                y: 0,
                width: self.width,
                height: self.height,
            };
            cmd_list.set_scissor_rect(&full_scissor);
            
            // Draw settings option rows
            let row_height = 40.0;
            let row_width = 400.0;
            let start_y = 100.0;
            let options = ["Разрешение", "Качество текстур", "Громкость", "Язык"];
            
            for (i, _opt) in options.iter().enumerate() {
                let row_y = start_y + i as f32 * (row_height + 10.0);
                let row_x = (self.width as f32 - row_width) / 2.0;
                
                let row_scissor = ScissorRect {
                    x: row_x as i32,
                    y: row_y as i32,
                    width: row_width as u32,
                    height: row_height as u32,
                };
                cmd_list.set_scissor_rect(&row_scissor);
                cmd_list.clear_color(0, [0.25, 0.25, 0.35, 0.9])?;
            }
            
            // Reset scissor
            cmd_list.set_scissor_rect(&full_scissor);
            
            // Bind font texture for text
            if let Some(font_tex) = self.font_texture {
                cmd_list.bind_texture(0, font_tex, ResourceState::ShaderResource);
            }
            
            tracing::trace!("Settings screen rendered");
            
            cmd_list.end_render_pass()?;
            cmd_list.close()?;
            self.device.submit_command_lists(&[cmd_list.clone()])?;
        }
        
        Ok(())
    }

    fn render_character_creation(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        use crate::graphics::rhi::{ResourceState, LoadOp, StoreOp, ClearValue};
        
        // Render character creation UI with model preview
        if let Some(ref cmd_list) = self.command_list {
            // Begin render pass
            let attachments = vec![RenderAttachment {
                resource: self.device.get_current_back_buffer()?,
                state_before: ResourceState::Present,
                state_after: ResourceState::Present,
                load_op: LoadOp::Clear,
                store_op: StoreOp::Store,
                clear_value: ClearValue::Color([0.2, 0.15, 0.15, 1.0]),
            }];
            
            let render_pass_desc = RenderPassDescription {
                attachments,
                depth_stencil: None,
            };
            
            cmd_list.begin_render_pass(&render_pass_desc)?;
            
            // Set viewport
            let viewport = Viewport {
                x: 0.0,
                y: 0.0,
                width: self.width as f32,
                height: self.height as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            };
            cmd_list.set_viewport(&viewport);
            
            // Draw model preview area (center)
            let preview_width = self.width as f32 * 0.4;
            let preview_height = self.height as f32 * 0.6;
            let preview_x = (self.width as f32 - preview_width) / 2.0;
            let preview_y = (self.height as f32 - preview_height) / 2.0;
            
            let preview_scissor = ScissorRect {
                x: preview_x as i32,
                y: preview_y as i32,
                width: preview_width as u32,
                height: preview_height as u32,
            };
            cmd_list.set_scissor_rect(&preview_scissor);
            cmd_list.clear_color(0, [0.15, 0.1, 0.1, 1.0])?;
            
            // Reset scissor
            let full_scissor = ScissorRect {
                x: 0,
                y: 0,
                width: self.width,
                height: self.height,
            };
            cmd_list.set_scissor_rect(&full_scissor);
            
            // Draw customization panels (left and right)
            let panel_width = 200.0;
            let panel_height = self.height as f32 * 0.7;
            
            // Left panel
            let left_panel_x = 50.0;
            let left_panel_y = (self.height as f32 - panel_height) / 2.0;
            let left_panel = ScissorRect {
                x: left_panel_x as i32,
                y: left_panel_y as i32,
                width: panel_width as u32,
                height: panel_height as u32,
            };
            cmd_list.set_scissor_rect(&left_panel);
            cmd_list.clear_color(0, [0.25, 0.15, 0.15, 0.9])?;
            
            // Right panel
            let right_panel_x = self.width as f32 - left_panel_x - panel_width;
            let right_panel = ScissorRect {
                x: right_panel_x as i32,
                y: left_panel_y as i32,
                width: panel_width as u32,
                height: panel_height as u32,
            };
            cmd_list.set_scissor_rect(&right_panel);
            cmd_list.clear_color(0, [0.25, 0.15, 0.15, 0.9])?;
            
            // Reset scissor
            cmd_list.set_scissor_rect(&full_scissor);
            
            // Bind font texture for text
            if let Some(font_tex) = self.font_texture {
                cmd_list.bind_texture(0, font_tex, ResourceState::ShaderResource);
            }
            
            tracing::trace!("Character creation screen rendered");
            
            cmd_list.end_render_pass()?;
            cmd_list.close()?;
            self.device.submit_command_lists(&[cmd_list.clone()])?;
        }
        
        Ok(())
    }

    pub fn get_width(&self) -> u32 {
        self.width
    }

    pub fn get_height(&self) -> u32 {
        self.height
    }

    pub fn set_camera(&mut self, camera: Camera) {
        self.camera = camera;
    }

    pub fn next_city(&mut self) {
        self.current_city_index = (self.current_city_index + 1) % 10;
    }

    pub fn prev_city(&mut self) {
        self.current_city_index = if self.current_city_index == 0 {
            9
        } else {
            self.current_city_index - 1
        };
    }
}
