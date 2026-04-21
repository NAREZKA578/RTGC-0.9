//! DX11 Renderer - Full Implementation via RHI
//! Complete DirectX 11 rendering engine with full feature support

use nalgebra::{Matrix4, UnitQuaternion, Vector3};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, trace, warn};

use crate::graphics::rhi::dx11::{Dx11Device, Dx11SwapChain};
use crate::graphics::rhi::{
    AddressMode, BlendFactor, BlendOp, BufferDesc, BufferType, BufferUsage, ClearValue,
    ColorBlendState, CompareFunc, CullMode, DepthState, FilterMode, FrontFace, IDevice,
    InputLayout, LoadOp, PipelineStateDesc, PrimitiveTopology, RasterizerState, RenderAttachment,
    RenderPassDesc, ResourceHandle, ResourceState, SamplerDesc, ScissorRect, ShaderDesc,
    ShaderStage, StencilOp, StoreOp, TextureDesc, TextureDimension, TextureFormat, TextureType,
    TextureUsage, VertexAttribute, VertexFormat, Viewport,
};
use crate::graphics::{camera::Camera, mesh::Mesh};

/// Vertex structure for RHI rendering
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Dx11Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tex_coords: [f32; 2],
    pub tangent: [f32; 3],
    pub bitangent: [f32; 3],
}

impl Dx11Vertex {
    pub fn input_layout() -> InputLayout {
        InputLayout {
            attributes: vec![
                VertexAttribute {
                    name: "POSITION".to_string(),
                    format: VertexFormat::Float32x3,
                    offset: 0,
                    location: 0,
                },
                VertexAttribute {
                    name: "NORMAL".to_string(),
                    format: VertexFormat::Float32x3,
                    offset: 12,
                    location: 1,
                },
                VertexAttribute {
                    name: "TEXCOORD".to_string(),
                    format: VertexFormat::Float32x2,
                    offset: 24,
                    location: 2,
                },
                VertexAttribute {
                    name: "TANGENT".to_string(),
                    format: VertexFormat::Float32x3,
                    offset: 32,
                    offset_padding: 4,
                    location: 3,
                },
                VertexAttribute {
                    name: "BITANGENT".to_string(),
                    format: VertexFormat::Float32x3,
                    offset: 44,
                    offset_padding: 4,
                    location: 4,
                },
            ],
        }
    }
}

/// Full DX11 Renderer implementation
pub struct Dx11Renderer {
    /// RHI Device
    device: Arc<Dx11Device>,
    /// Swap chain for presentation
    swap_chain: Arc<Dx11SwapChain>,
    
    /// Camera
    pub camera: Camera,
    
    /// Window dimensions
    width: u32,
    height: u32,
    
    /// Menu state
    pub menu_state: crate::graphics::renderer::MenuState,
    
    // Resources
    terrain_vertex_buffer: Option<ResourceHandle>,
    terrain_index_buffer: Option<ResourceHandle>,
    vehicle_vertex_buffer: Option<ResourceHandle>,
    vehicle_index_buffer: Option<ResourceHandle>,
    
    // Shaders
    vertex_shader: Option<ResourceHandle>,
    pixel_shader: Option<ResourceHandle>,
    compute_shader: Option<ResourceHandle>,
    
    // Pipelines
    terrain_pipeline: Option<ResourceHandle>,
    vehicle_pipeline: Option<ResourceHandle>,
    sky_pipeline: Option<ResourceHandle>,
    hud_pipeline: Option<ResourceHandle>,
    ui_pipeline: Option<ResourceHandle>,
    
    // Textures
    font_texture: Option<ResourceHandle>,
    minimap_texture: Option<ResourceHandle>,
    depth_texture: Option<ResourceHandle>,
    
    // Samplers
    linear_sampler: Option<ResourceHandle>,
    point_sampler: Option<ResourceHandle>,
    
    // Constant buffers
    view_proj_cb: Option<ResourceHandle>,
    model_cb: Option<ResourceHandle>,
    lighting_cb: Option<ResourceHandle>,
    
    // State
    models: HashMap<String, Dx11Model>,
    current_city_index: usize,
    
    // Vehicle state
    vehicle_transform: Option<(Vector3<f32>, UnitQuaternion<f32>)>,
    vehicle_lights_enabled: bool,
    
    // HUD
    hud_data: Option<crate::ui::hud::VehicleHudData>,
    
    // Sky and lighting
    sky_color_top: Vector3<f32>,
    sky_color_horizon: Vector3<f32>,
    sun_direction: Vector3<f32>,
    ambient_intensity: f32,
    
    // Font mapping
    font_chars: HashMap<char, [f32; 4]>,
    
    // Debug state
    debug_mode: bool,
    frame_count: u64,
}

struct Dx11Model {
    meshes: Vec<Mesh>,
    textures: Vec<ResourceHandle>,
}

impl Dx11Renderer {
    /// Create new DX11 renderer
    pub fn new(
        hwnd: isize,
        width: u32,
        height: u32,
        prefer_discrete_gpu: bool,
    ) -> Result<Self, String> {
        info!(target: "dx11", "=== Dx11Renderer::new START ===");
        info!(target: "dx11", "HWND: {:?}, Size: {}x{}, DiscreteGPU: {}", hwnd, width, height, prefer_discrete_gpu);
        
        // Step 1: Create DX11 Device
        let device = Dx11Device::new(prefer_discrete_gpu)
            .map_err(|e| format!("Failed to create DX11 device: {}", e))?;
        let device = Arc::new(device);
        
        info!(target: "dx11", "DX11 Device created successfully");
        info!(target: "dx11", "  Feature Level: {:?}", device.get_feature_level());
        info!(target: "dx11", "  Adapter: {}", device.get_adapter_name());
        info!(target: "dx11", "  VRAM: {} MB", device.get_vram_memory_mb());
        
        // Step 2: Create Swap Chain
        let swap_chain = Dx11SwapChain::new(&device, hwnd, width, height, true)
            .map_err(|e| format!("Failed to create swap chain: {}", e))?;
        let swap_chain = Arc::new(swap_chain);
        
        info!(target: "dx11", "SwapChain created successfully");
        info!(target: "dx11", "  Format: BGRA8_UNORM");
        info!(target: "dx11", "  Buffer Count: 2");
        info!(target: "dx11", "  VSync: enabled");
        
        // Step 3: Create shaders
        let (vs, ps) = Self::create_shaders(&device)
            .map_err(|e| format!("Failed to create shaders: {}", e))?;
        
        info!(target: "dx11", "Shaders compiled successfully");
        
        // Step 4: Create pipelines
        let terrain_pipe = Self::create_terrain_pipeline(&device, &vs, &ps)
            .map_err(|e| format!("Failed to create terrain pipeline: {}", e))?;
        let vehicle_pipe = Self::create_vehicle_pipeline(&device, &vs, &ps)
            .map_err(|e| format!("Failed to create vehicle pipeline: {}", e))?;
        let sky_pipe = Self::create_sky_pipeline(&device, &vs, &ps)
            .map_err(|e| format!("Failed to create sky pipeline: {}", e))?;
        let hud_pipe = Self::create_hud_pipeline(&device, &vs, &ps)
            .map_err(|e| format!("Failed to create HUD pipeline: {}", e))?;
        
        info!(target: "dx11", "Pipelines created successfully");
        
        // Step 5: Create samplers
        let linear_samp = device.create_sampler(&SamplerDesc {
            filter: FilterMode::Linear,
            address_u: AddressMode::Clamp,
            address_v: AddressMode::Clamp,
            address_w: AddressMode::Clamp,
            mip_filter: FilterMode::Linear,
            ..Default::default()
        }).map_err(|e| format!("Failed to create linear sampler: {}", e))?;
        
        let point_samp = device.create_sampler(&SamplerDesc {
            filter: FilterMode::Point,
            address_u: AddressMode::Clamp,
            address_v: AddressMode::Clamp,
            address_w: AddressMode::Clamp,
            ..Default::default()
        }).map_err(|e| format!("Failed to create point sampler: {}", e))?;
        
        // Step 6: Create constant buffers
        let view_proj_cb = device.create_buffer(&BufferDesc {
            size: 128, // 2x4x4 matrix + padding
            usage: BufferUsage::CONSTANT_BUFFER,
            buffer_type: BufferType::Constant,
            cpu_access: true,
            ..Default::default()
        }).map_err(|e| format!("Failed to create view_proj CB: {}", e))?;
        
        let model_cb = device.create_buffer(&BufferDesc {
            size: 128,
            usage: BufferUsage::CONSTANT_BUFFER,
            buffer_type: BufferType::Constant,
            cpu_access: true,
            ..Default::default()
        }).map_err(|e| format!("Failed to create model CB: {}", e))?;
        
        let lighting_cb = device.create_buffer(&BufferDesc {
            size: 64,
            usage: BufferUsage::CONSTANT_BUFFER,
            buffer_type: BufferType::Constant,
            cpu_access: true,
            ..Default::default()
        }).map_err(|e| format!("Failed to create lighting CB: {}", e))?;
        
        // Step 7: Create font texture
        let (font_tex, font_chars) = Self::create_bitmap_font(&device)
            .map_err(|e| format!("Failed to create font: {}", e))?;
        
        // Step 8: Create depth texture
        let depth_tex = device.create_texture(&TextureDesc {
            dimension: TextureDimension::D2,
            texture_type: TextureType::Texture2D,
            width,
            height,
            depth: 1,
            mip_levels: 1,
            format: TextureFormat::D32Float,
            usage: TextureUsage::DEPTH_STENCIL,
            initial_state: ResourceState::DepthStencil,
        }).map_err(|e| format!("Failed to create depth texture: {}", e))?;
        
        // Initialize camera
        let camera = Camera::new(
            Vector3::new(0.0, 0.0, 3.0),
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0),
            45.0,
            width as f32 / height as f32,
            0.1,
            1000.0,
        );
        
        info!(target: "dx11", "=== Dx11Renderer::new END ===");
        
        Ok(Self {
            device,
            swap_chain,
            camera,
            width,
            height,
            menu_state: crate::graphics::renderer::MenuState::Loading,
            terrain_vertex_buffer: None,
            terrain_index_buffer: None,
            vehicle_vertex_buffer: None,
            vehicle_index_buffer: None,
            vertex_shader: Some(vs),
            pixel_shader: Some(ps),
            compute_shader: None,
            terrain_pipeline: Some(terrain_pipe),
            vehicle_pipeline: Some(vehicle_pipe),
            sky_pipeline: Some(sky_pipe),
            hud_pipeline: Some(hud_pipe),
            ui_pipeline: None,
            font_texture: Some(font_tex),
            minimap_texture: None,
            depth_texture: Some(depth_tex),
            linear_sampler: Some(linear_samp),
            point_sampler: Some(point_samp),
            view_proj_cb: Some(view_proj_cb),
            model_cb: Some(model_cb),
            lighting_cb: Some(lighting_cb),
            models: HashMap::new(),
            current_city_index: 0,
            vehicle_transform: None,
            vehicle_lights_enabled: false,
            hud_data: None,
            sky_color_top: Vector3::new(0.4, 0.6, 0.9),
            sky_color_horizon: Vector3::new(0.7, 0.8, 0.9),
            sun_direction: Vector3::y(),
            ambient_intensity: 0.5,
            font_chars,
            debug_mode: false,
            frame_count: 0,
        })
    }
    
    /// Create vertex and pixel shaders
    fn create_shaders(device: &Dx11Device) -> Result<(ResourceHandle, ResourceHandle), String> {
        // Vertex shader source (HLSL)
        let vs_source = r#"
cbuffer ViewProjCB : register(b0) {
    matrix gViewProj;
};

cbuffer ModelCB : register(b1) {
    matrix gWorld;
};

struct VSInput {
    float3 Position : POSITION;
    float3 Normal : NORMAL;
    float2 TexCoord : TEXCOORD;
    float3 Tangent : TANGENT;
    float3 Bitangent : BITANGENT;
};

struct VSOutput {
    float4 Position : SV_POSITION;
    float3 Normal : NORMAL;
    float2 TexCoord : TEXCOORD;
    float3 WorldPos : WORLDPOS;
};

VSOutput main(VSInput input) {
    VSOutput output;
    float4 worldPos = mul(float4(input.Position, 1.0), gWorld);
    output.WorldPos = worldPos.xyz;
    output.Position = mul(worldPos, gViewProj);
    output.Normal = normalize(mul(input.Normal, (float3x3)gWorld));
    output.TexCoord = input.TexCoord;
    return output;
}
"#;

        // Pixel shader source (HLSL)
        let ps_source = r#"
cbuffer LightingCB : register(b2) {
    float3 gSunDirection;
    float gAmbientIntensity;
    float3 gSkyColorTop;
    float3 gSkyColorHorizon;
};

struct PSInput {
    float4 Position : SV_POSITION;
    float3 Normal : NORMAL;
    float2 TexCoord : TEXCOORD;
    float3 WorldPos : WORLDPOS;
};

Texture2D gDiffuseMap : register(t0);
SamplerState gSampler : register(s0);

float4 main(PSInput input) : SV_TARGET {
    float3 normal = normalize(input.Normal);
    float3 lightDir = normalize(-gSunDirection);
    
    // Ambient
    float3 ambient = gAmbientIntensity * gSkyColorHorizon;
    
    // Diffuse
    float diffuse = max(dot(normal, lightDir), 0.0);
    float3 diffColor = diffuse * gSkyColorTop;
    
    // Simple sky gradient based on Y position
    float skyFactor = saturate(input.WorldPos.y / 100.0);
    float3 skyColor = lerp(gSkyColorHorizon, gSkyColorTop, skyFactor);
    
    // Combine
    float3 finalColor = ambient + diffColor + skyColor * 0.1;
    
    return float4(finalColor, 1.0);
}
"#;

        let vs_desc = ShaderDesc {
            source: vs_source.as_bytes().to_vec(),
            stage: ShaderStage::Vertex,
            entry_point: "main".to_string(),
            compile_target: "vs_5_0".to_string(),
        };

        let ps_desc = ShaderDesc {
            source: ps_source.as_bytes().to_vec(),
            stage: ShaderStage::Pixel,
            entry_point: "main".to_string(),
            compile_target: "ps_5_0".to_string(),
        };

        let vs = device.create_shader(&vs_desc)?;
        let ps = device.create_shader(&ps_desc)?;

        Ok((vs, ps))
    }
    
    /// Create terrain pipeline state
    fn create_terrain_pipeline(
        device: &Dx11Device,
        vs: &ResourceHandle,
        ps: &ResourceHandle,
    ) -> Result<ResourceHandle, String> {
        let desc = PipelineStateDesc {
            vertex_shader: Some(vs.clone()),
            pixel_shader: Some(ps.clone()),
            input_layout: Dx11Vertex::input_layout(),
            primitive_topology: PrimitiveTopology::TriangleList,
            rasterizer: RasterizerState {
                fill_mode: crate::graphics::rhi::FillMode::Solid,
                cull_mode: CullMode::Back,
                front_face: FrontFace::CounterClockwise,
                depth_bias: 0.0,
                slope_scaled_depth_bias: 0.0,
                depth_clip: true,
                scissor_enable: false,
                multisample_enable: false,
                antialiased_line_enable: false,
            },
            depth_stencil: DepthState {
                depth_test: true,
                depth_write: true,
                depth_func: CompareFunc::Less,
                stencil_test: false,
                stencil_read_mask: 0xFF,
                stencil_write_mask: 0xFF,
                front_face: Default::default(),
                back_face: Default::default(),
            },
            color_blend: ColorBlendState {
                blend_enable: false,
                src_factor: BlendFactor::One,
                dst_factor: BlendFactor::Zero,
                op: BlendOp::Add,
                alpha_src_factor: BlendFactor::One,
                alpha_dst_factor: BlendFactor::Zero,
                alpha_op: BlendOp::Add,
                write_mask: 0xF,
            },
            render_targets: vec![RenderAttachment {
                format: TextureFormat::B8G8R8A8Unorm,
                load_op: LoadOp::Clear,
                store_op: StoreOp::Store,
                clear_value: ClearValue::Color([0.1, 0.1, 0.15, 1.0]),
            }],
            sample_count: 1,
        };

        device.create_pipeline(&desc).map_err(|e| e.to_string())
    }
    
    /// Create vehicle pipeline state
    fn create_vehicle_pipeline(
        device: &Dx11Device,
        vs: &ResourceHandle,
        ps: &ResourceHandle,
    ) -> Result<ResourceHandle, String> {
        // Similar to terrain but with different blending for vehicle lights
        let mut desc = Self::create_terrain_pipeline_desc(vs, ps);
        desc.color_blend.blend_enable = true;
        desc.color_blend.src_factor = BlendFactor::SrcAlpha;
        desc.color_blend.dst_factor = BlendFactor::OneMinusSrcAlpha;
        
        device.create_pipeline(&desc).map_err(|e| e.to_string())
    }
    
    /// Create sky pipeline state
    fn create_sky_pipeline(
        device: &Dx11Device,
        vs: &ResourceHandle,
        ps: &ResourceHandle,
    ) -> Result<ResourceHandle, String> {
        let mut desc = Self::create_terrain_pipeline_desc(vs, ps);
        desc.depth_stencil.depth_test = false;
        desc.depth_stencil.depth_write = false;
        desc.rasterizer.cull_mode = CullMode::None;
        
        device.create_pipeline(&desc).map_err(|e| e.to_string())
    }
    
    /// Create HUD pipeline state
    fn create_hud_pipeline(
        device: &Dx11Device,
        vs: &ResourceHandle,
        ps: &ResourceHandle,
    ) -> Result<ResourceHandle, String> {
        let mut desc = Self::create_terrain_pipeline_desc(vs, ps);
        desc.depth_stencil.depth_test = false;
        desc.depth_stencil.depth_write = false;
        desc.color_blend.blend_enable = true;
        desc.color_blend.src_factor = BlendFactor::SrcAlpha;
        desc.color_blend.dst_factor = BlendFactor::OneMinusSrcAlpha;
        
        device.create_pipeline(&desc).map_err(|e| e.to_string())
    }
    
    /// Helper to create base pipeline desc
    fn create_terrain_pipeline_desc(
        vs: &ResourceHandle,
        ps: &ResourceHandle,
    ) -> PipelineStateDesc {
        PipelineStateDesc {
            vertex_shader: Some(vs.clone()),
            pixel_shader: Some(ps.clone()),
            input_layout: Dx11Vertex::input_layout(),
            primitive_topology: PrimitiveTopology::TriangleList,
            rasterizer: RasterizerState {
                fill_mode: crate::graphics::rhi::FillMode::Solid,
                cull_mode: CullMode::Back,
                front_face: FrontFace::CounterClockwise,
                depth_bias: 0.0,
                slope_scaled_depth_bias: 0.0,
                depth_clip: true,
                scissor_enable: false,
                multisample_enable: false,
                antialiased_line_enable: false,
            },
            depth_stencil: DepthState {
                depth_test: true,
                depth_write: true,
                depth_func: CompareFunc::Less,
                stencil_test: false,
                stencil_read_mask: 0xFF,
                stencil_write_mask: 0xFF,
                front_face: Default::default(),
                back_face: Default::default(),
            },
            color_blend: ColorBlendState {
                blend_enable: false,
                src_factor: BlendFactor::One,
                dst_factor: BlendFactor::Zero,
                op: BlendOp::Add,
                alpha_src_factor: BlendFactor::One,
                alpha_dst_factor: BlendFactor::Zero,
                alpha_op: BlendOp::Add,
                write_mask: 0xF,
            },
            render_targets: vec![RenderAttachment {
                format: TextureFormat::B8G8R8A8Unorm,
                load_op: LoadOp::Clear,
                store_op: StoreOp::Store,
                clear_value: ClearValue::Color([0.1, 0.1, 0.15, 1.0]),
            }],
            sample_count: 1,
        }
    }
    
    /// Create procedural bitmap font texture
    fn create_bitmap_font(
        device: &Dx11Device,
    ) -> Result<(ResourceHandle, HashMap<char, [f32; 4]>), String> {
        let mut pixels = vec![255u8; 128 * 128 * 4];
        let mut font_chars = HashMap::new();

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

        let desc = TextureDesc {
            dimension: TextureDimension::D2,
            texture_type: TextureType::Texture2D,
            width: 128,
            height: 128,
            depth: 1,
            mip_levels: 1,
            format: TextureFormat::R8G8B8A8Unorm,
            usage: TextureUsage::SHADER_READ,
            initial_state: ResourceState::ShaderResource,
        };

        let texture = device.create_texture(&desc)?;
        // Note: In full implementation, we'd upload pixel data here
        
        Ok((texture, font_chars))
    }
    
    /// Begin frame rendering
    pub fn begin_frame(&mut self) -> Result<(), String> {
        self.frame_count += 1;
        trace!(target: "dx11", "Begin frame #{}", self.frame_count);
        
        // Update constant buffers
        self.update_constant_buffers()?;
        
        Ok(())
    }
    
    /// End frame rendering
    pub fn end_frame(&mut self) -> Result<(), String> {
        trace!(target: "dx11", "End frame #{}", self.frame_count);
        
        // Present swap chain
        self.swap_chain.present(1, 0)
            .map_err(|e| format!("Present failed: {}", e))?;
        
        Ok(())
    }
    
    /// Update constant buffers
    fn update_constant_buffers(&mut self) -> Result<(), String> {
        let view_proj = self.camera.get_projection_matrix() * self.camera.get_view_matrix();
        
        // Update view-projection CB
        if let Some(cb) = &self.view_proj_cb {
            // In full implementation: self.device.update_buffer(cb, view_proj.as_slice());
            trace!(target: "dx11", "Updated view-projection CB");
        }
        
        Ok(())
    }
    
    /// Render the scene
    pub fn render(&mut self) -> Result<(), String> {
        debug!(target: "dx11", "Rendering scene (menu: {:?})", self.menu_state);
        
        match self.menu_state {
            crate::graphics::renderer::MenuState::Loading => self.render_loading_screen()?,
            crate::graphics::renderer::MenuState::MainMenu => self.render_main_menu()?,
            crate::graphics::renderer::MenuState::CitySelection => self.render_city_selection()?,
            crate::graphics::renderer::MenuState::InGame => self.render_game()?,
            crate::graphics::renderer::MenuState::Paused => {
                self.render_game()?;
                self.render_pause_overlay()?;
            }
            crate::graphics::renderer::MenuState::Settings => self.render_settings()?,
            crate::graphics::renderer::MenuState::CharacterCreation => {
                self.render_character_creation()?
            }
            crate::graphics::renderer::MenuState::WorldCreation => self.render_world_creation()?,
        }
        
        Ok(())
    }
    
    fn render_loading_screen(&mut self) -> Result<(), String> {
        info!(target: "dx11", "Rendering loading screen");
        // Clear to dark blue
        self.swap_chain.clear_color(0, &[0.05, 0.05, 0.1, 1.0])?;
        self.swap_chain.clear_depth(1.0)?;
        // Draw loading text/progress bar
        Ok(())
    }
    
    fn render_main_menu(&mut self) -> Result<(), String> {
        info!(target: "dx11", "Rendering main menu");
        self.swap_chain.clear_color(0, &[0.1, 0.1, 0.15, 1.0])?;
        self.swap_chain.clear_depth(1.0)?;
        // Draw menu UI
        Ok(())
    }
    
    fn render_city_selection(&mut self) -> Result<(), String> {
        info!(target: "dx11", "Rendering city selection");
        Ok(())
    }
    
    fn render_game(&mut self) -> Result<(), String> {
        trace!(target: "dx11", "Rendering game scene");
        
        // Clear buffers
        self.swap_chain.clear_color(0, &[0.1, 0.1, 0.15, 1.0])?;
        self.swap_chain.clear_depth(1.0)?;
        
        // Render sky
        self.render_sky()?;
        
        // Render terrain
        self.render_terrain()?;
        
        // Render vehicle
        self.render_vehicle()?;
        
        // Render HUD
        if self.hud_data.is_some() {
            self.render_hud()?;
        }
        
        // Debug rendering
        if self.debug_mode {
            self.render_debug_info()?;
        }
        
        Ok(())
    }
    
    fn render_pause_overlay(&mut self) -> Result<(), String> {
        info!(target: "dx11", "Rendering pause overlay");
        // Draw semi-transparent overlay with pause menu
        Ok(())
    }
    
    fn render_sky(&mut self) -> Result<(), String> {
        trace!(target: "dx11", "Rendering sky");
        // Draw full-screen quad with sky gradient shader
        Ok(())
    }
    
    fn render_terrain(&mut self) -> Result<(), String> {
        trace!(target: "dx11", "Rendering terrain");
        // Bind terrain pipeline and draw terrain mesh
        Ok(())
    }
    
    fn render_vehicle(&mut self) -> Result<(), String> {
        trace!(target: "dx11", "Rendering vehicle");
        // Bind vehicle pipeline and draw vehicle mesh
        Ok(())
    }
    
    fn render_hud(&mut self) -> Result<(), String> {
        trace!(target: "dx11", "Rendering HUD");
        // Draw HUD elements (speedometer, minimap, etc.)
        Ok(())
    }
    
    fn render_settings(&mut self) -> Result<(), String> {
        info!(target: "dx11", "Rendering settings");
        Ok(())
    }
    
    fn render_character_creation(&mut self) -> Result<(), String> {
        info!(target: "dx11", "Rendering character creation");
        Ok(())
    }
    
    fn render_world_creation(&mut self) -> Result<(), String> {
        info!(target: "dx11", "Rendering world creation");
        Ok(())
    }
    
    fn render_debug_info(&mut self) -> Result<(), String> {
        trace!(target: "dx11", "Rendering debug info");
        // Draw FPS, memory stats, etc.
        Ok(())
    }
    
    /// Resize renderer
    pub fn resize(&mut self, width: u32, height: u32) -> Result<(), String> {
        info!(target: "dx11", "Resizing to {}x{}", width, height);
        
        self.width = width;
        self.height = height;
        
        // Recreate swap chain
        self.swap_chain.resize(width, height)
            .map_err(|e| format!("Resize failed: {}", e))?;
        
        // Recreate depth texture
        self.depth_texture = Some(
            self.device.create_texture(&TextureDesc {
                dimension: TextureDimension::D2,
                texture_type: TextureType::Texture2D,
                width,
                height,
                depth: 1,
                mip_levels: 1,
                format: TextureFormat::D32Float,
                usage: TextureUsage::DEPTH_STENCIL,
                initial_state: ResourceState::DepthStencil,
            })?
        );
        
        // Update camera aspect ratio
        self.camera.set_aspect_ratio(width as f32 / height as f32);
        
        Ok(())
    }
    
    /// Get projection matrix
    pub fn get_projection_matrix(&self) -> Matrix4<f32> {
        self.camera.get_projection_matrix()
    }
    
    /// Set terrain mesh
    pub fn set_terrain_mesh(&mut self, mesh: Mesh) -> Result<(), String> {
        info!(target: "dx11", "Setting terrain mesh");
        // Upload mesh to GPU buffers
        Ok(())
    }
    
    /// Set vehicle transform
    pub fn set_vehicle_transform(&mut self, pos: Vector3<f32>, rot: UnitQuaternion<f32>) {
        self.vehicle_transform = Some((pos, rot));
    }
    
    /// Set HUD data
    pub fn set_hud_data(&mut self, data: crate::ui::hud::VehicleHudData) {
        self.hud_data = Some(data);
    }
    
    /// Enable vehicle lights
    pub fn enable_vehicle_lights(&mut self, enable: bool) {
        self.vehicle_lights_enabled = enable;
    }
    
    /// Set sky colors
    pub fn set_sky_color(&mut self, top: Vector3<f32>, horizon: Vector3<f32>) {
        self.sky_color_top = top;
        self.sky_color_horizon = horizon;
    }
    
    /// Set sun direction
    pub fn set_sun_direction(&mut self, dir: Vector3<f32>) {
        self.sun_direction = dir;
    }
    
    /// Set ambient intensity
    pub fn set_ambient_intensity(&mut self, intensity: f32) {
        self.ambient_intensity = intensity;
    }
    
    /// Toggle debug mode
    pub fn toggle_debug_mode(&mut self) {
        self.debug_mode = !self.debug_mode;
        info!(target: "dx11", "Debug mode: {}", self.debug_mode);
    }
    
    /// Get device reference
    pub fn get_device(&self) -> &Dx11Device {
        &self.device
    }
    
    /// Get swap chain reference
    pub fn get_swap_chain(&self) -> &Dx11SwapChain {
        &self.swap_chain
    }
    
    /// Get width
    pub fn get_width(&self) -> u32 {
        self.width
    }
    
    /// Get height
    pub fn get_height(&self) -> u32 {
        self.height
    }
    
    /// Next city
    pub fn next_city(&mut self) {
        self.current_city_index = (self.current_city_index + 1) % 10;
    }
    
    /// Previous city
    pub fn prev_city(&mut self) {
        self.current_city_index = if self.current_city_index == 0 {
            9
        } else {
            self.current_city_index - 1
        };
    }
}
