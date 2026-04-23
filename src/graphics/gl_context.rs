//! OpenGL Context Management
//! 
//! This module provides GlContext which creates an OpenGL 4.5+ context using glutin
//! and integrates with the OpenGL RHI backend.

use glow::Context;
use std::sync::Arc;
use std::num::NonZeroU32;
use winit::window::Window;
use glutin::surface::{SurfaceAttributesBuilder, WindowSurface};
use glutin::context::{ContextApi, ContextAttributesBuilder, Version};
use glutin::config::GlConfig;
use glutin::prelude::*;
use glutin_winit::DisplayBuilder;
use crate::graphics::rhi::gl::{GlDevice, GlCommandQueue, GlSwapChainInternal};
use crate::graphics::rhi::device::{IDevice, ICommandQueue, ISwapChain};
use crate::graphics::rhi::types::{TextureFormat, RhiResult};

/// OpenGL Graphics Context
/// Manages window, GL context, and RHI device
pub struct GlContext {
    window: Arc<Window>,
    surface: glutin::surface::Surface<WindowSurface>,
    gl_context: Arc<Context>,
    device: Arc<GlDevice>,
    command_queue: Arc<GlCommandQueue>,
    swapchain: Option<Arc<GlSwapChainInternal>>,
    width: u32,
    height: u32,
    initialized: bool,
}

impl GlContext {
    /// Create a new OpenGL context with existing window
    pub fn new(window: Window) -> RhiResult<Self> {
        let (width, height) = window.inner_size().into();
        let raw_window_handle = window.window_handle()
            .map_err(|e| crate::graphics::rhi::types::RhiError::InitializationFailed(
                format!("Failed to get window handle: {:?}", e)
            ))?
            .as_raw();
        
        // Initialize glutin display with the existing window
        let display_builder = DisplayBuilder::new()
            .with_window_attributes(None); // We already have a window
        
        // Create event loop for display initialization
        let event_loop = winit::event_loop::EventLoop::new()
            .map_err(|e| crate::graphics::rhi::types::RhiError::InitializationFailed(
                format!("Failed to create event loop: {:?}", e)
            ))?;
        
        let (_, gl_config) = display_builder
            .build(&event_loop, |configs| {
                configs.find(|c| c.supports_glsl_version(&Version::new(4, 5)))
                    .unwrap_or_else(|| configs.next().unwrap())
            })
            .map_err(|e| crate::graphics::rhi::types::RhiError::InitializationFailed(
                format!("Failed to build GL display: {:?}", e)
            ))?;
        
        let window = Arc::new(window);
        
        // Create GL context
        let context_attributes = ContextAttributesBuilder::new()
            .with_profile(glutin::context::GlProfile::Core)
            .with_context_api(ContextApi::OpenGl(Some(Version::new(4, 5))))
            .build(Some(raw_window_handle));
        
        let not_current_context = unsafe {
            gl_config.display().create_context(&gl_config, &context_attributes)
                .map_err(|e| crate::graphics::rhi::types::RhiError::InitializationFailed(
                    format!("Failed to create GL context: {:?}", e)
                ))?
        };
        
        // Create surface
        let attrs = SurfaceAttributesBuilder::<WindowSurface>::new()
            .with_srgb(Some(true))
            .build(
                raw_window_handle,
                NonZeroU32::new(width).unwrap(),
                NonZeroU32::new(height).unwrap(),
            );
        
        let surface = unsafe {
            gl_config.display().create_window_surface(&gl_config, &attrs)
                .map_err(|e| crate::graphics::rhi::types::RhiError::InitializationFailed(
                    format!("Failed to create surface: {:?}", e)
                ))?
        };
        
        // Make context current
        let gl_context = not_current_context.make_current(&surface)
            .map_err(|e| crate::graphics::rhi::types::RhiError::InitializationFailed(
                format!("Failed to make context current: {:?}", e)
            ))?;
        
        // Create glow context
        let glow_context = unsafe {
            Context::from_loader_function(|s| {
                let s = std::ffi::CString::new(s).unwrap();
                gl_config.display().get_proc_address(&s) as *const _
            })
        };
        
        let gl_context = Arc::new(glow_context);
        
        // Create GL device
        let device = Arc::new(GlDevice::new(gl_context.clone()));
        
        // Create command queue
        let command_queue = Arc::new(GlCommandQueue::new(
            gl_context.clone(),
            crate::graphics::rhi::types::CommandListType::Direct,
        ));
        
        Ok(Self {
            window,
            surface,
            gl_context,
            device,
            command_queue,
            swapchain: None,
            width,
            height,
            initialized: false,
        })
    }
    
    /// Get reference to the window
    pub fn window(&self) -> &Window {
        &self.window
    }
    
    /// Get GL device for resource creation
    pub fn device(&self) -> Arc<dyn IDevice> {
        self.device.clone()
    }
    
    /// Get command queue for submitting commands
    pub fn command_queue(&self) -> Arc<dyn ICommandQueue> {
        self.command_queue.clone()
    }
    
    /// Create or recreate swapchain for the window
    pub fn create_swapchain(&mut self, vsync: bool) -> RhiResult<()> {
        let swapchain = self.device.create_swap_chain(
            self.window.window_handle().unwrap().as_raw().as_ptr(),
            self.width,
            self.height,
            TextureFormat::Bgra8Unorm,
            vsync,
        )?;
        
        // Cast to GlSwapChainInternal
        let gl_swapchain = swapchain
            .as_any()
            .downcast_ref::<GlSwapChainInternal>()
            .ok_or_else(|| crate::graphics::rhi::types::RhiError::InitializationFailed(
                "Failed to downcast swapchain to GlSwapChainInternal".to_string()
            ))?
            .clone();
        
        self.swapchain = Some(Arc::new(gl_swapchain));
        self.initialized = true;
        
        Ok(())
    }
    
    /// Get swapchain
    pub fn swapchain(&self) -> Option<Arc<dyn ISwapChain>> {
        self.swapchain.clone().map(|s| s as Arc<dyn ISwapChain>)
    }
    
    /// Get GL-specific swapchain for direct access to GL resources
    pub fn gl_swapchain(&self) -> Option<Arc<GlSwapChainInternal>> {
        self.swapchain.clone()
    }
    
    /// Check if context is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
    
    /// Get window size
    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }
    
    /// Handle window resize
    pub fn on_resize(&mut self, width: u32, height: u32) -> RhiResult<()> {
        self.width = width;
        self.height = height;
        
        if let Some(ref mut swapchain) = self.swapchain {
            swapchain.resize(width, height)?;
        }
        
        Ok(())
    }
    
    /// Present the current frame
    pub fn present(&self) -> RhiResult<()> {
        // В OpenGL презентация происходит через swap_buffers в glutin/winit
        // Вызываем surface.swap_buffers для обновления экрана
        use glutin::surface::SurfaceTypeTrait;
        
        // Сначала flush для гарантии выполнения всех GL команд
        unsafe { self.gl_context.flush(); }
        
        // Затем swap buffers
        self.surface.swap_buffers(&self.gl_context.make_current(&self.surface).unwrap())
            .map_err(|e| crate::graphics::rhi::types::RhiError::InitializationFailed(
                format!("Failed to swap buffers: {:?}", e)
            ))?;
        Ok(())
    }
    
    /// Swap buffers (alias for present)
    pub fn swap_buffers(&self) -> RhiResult<()> {
        self.present()
    }
}
