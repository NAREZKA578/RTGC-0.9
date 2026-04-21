//! DirectX 11 Context - graphics context for DX11 backend
//! Provides swapchain, render target, and basic rendering operations

use tracing::info;

/// DirectX 11 Graphics Context
pub struct Dx11GraphicsContext {
    pub width: u32,
    pub height: u32,
    pub hwnd: isize,
    /// Swap chain pointer (raw handle for FFI)
    swap_chain: *mut std::ffi::c_void,
    /// Render target view
    render_target_view: *mut std::ffi::c_void,
    /// Device context
    device_context: *mut std::ffi::c_void,
    /// Depth stencil view
    depth_stencil_view: *mut std::ffi::c_void,
    /// Viewport set flag
    viewport_set: bool,
    /// Framebuffer ready flag
    framebuffer_ready: bool,
}

// SAFETY: Dx11GraphicsContext contains raw COM pointers which are reference-counted.
// The struct owns the COM references and manages them safely. Send/Sync is allowed
// because COM objects in DirectX 11 are thread-safe for read operations, but state
// mutations (like resize) must be synchronized externally. This context is designed
// to be used on the main render thread only.
//
// NOTE: The underlying Windows COM interfaces require careful thread management.
// This implementation assumes Single-Threaded Apartment (STA) model on the main thread.
unsafe impl Send for Dx11GraphicsContext {}
unsafe impl Sync for Dx11GraphicsContext {}

impl Dx11GraphicsContext {
    /// Create new DX11 graphics context
    pub fn new(hwnd: isize, width: u32, height: u32) -> Result<Self, String> {
        info!(target: "dx11", "=== Dx11GraphicsContext ===");
        info!(target: "dx11", "HWND: {:?}, Size: {}x{}", hwnd, width, height);
        
        // In a full implementation, this would:
        // 1. Create D3D11 device and device context
        // 2. Create swap chain description
        // 3. Create swap chain
        // 4. Create render target view from back buffer
        // 5. Create depth stencil texture and view
        // 6. Set up viewport
        
        // For now, initialize with null pointers as stubs
        // The actual DX11 initialization requires Windows-specific FFI
        
        info!(target: "dx11", "DX11 context created (stub - requires Windows FFI)");
        Ok(Self {
            width,
            height,
            hwnd,
            swap_chain: std::ptr::null_mut(),
            render_target_view: std::ptr::null_mut(),
            device_context: std::ptr::null_mut(),
            depth_stencil_view: std::ptr::null_mut(),
            viewport_set: false,
            framebuffer_ready: false,
        })
    }

    /// Set viewport to current dimensions
    pub fn set_viewport(&self) {
        // In full implementation:
        // D3D11_VIEWPORT vp = { 0, 0, width, height, 0.0f, 1.0f };
        // device_context->RSSetViewports(1, &vp);
        tracing::trace!("DX11 viewport set to {}x{}", self.width, self.height);
    }

    /// Clear render target with optional color
    pub fn clear(&self, color: Option<[f32; 4]>) {
        // In full implementation:
        // device_context->ClearRenderTargetView(rtv, color.data);
        let c = color.unwrap_or([0.0, 0.0, 0.0, 1.0]);
        tracing::trace!("DX11 clear with color: {:?}", c);
    }

    /// Begin frame - prepare for rendering
    pub fn begin_frame(&self) {
        // In full implementation:
        // - Clear render target
        // - Clear depth stencil
        // - Set viewport
        tracing::trace!("DX11 begin frame");
    }

    /// End frame - present swap chain
    pub fn end_frame(&self) {
        // In full implementation:
        // swap_chain->Present(1, 0);
        tracing::trace!("DX11 end frame (present)");
    }

    /// Resize swap chain and render targets
    pub fn resize(&mut self, width: u32, height: u32) -> Result<(), String> {
        info!(target: "dx11", "DX11 resize to {}x{}", width, height);
        
        // In full implementation:
        // 1. Release old render target view
        // 2. Resize swap chain buffers
        // 3. Recreate render target view
        // 4. Recreate depth stencil if needed
        // 5. Update viewport
        
        self.width = width;
        self.height = height;
        self.viewport_set = false;
        Ok(())
    }

    /// Get device name for display
    pub fn get_device_name(&self) -> &str {
        "DirectX 11"
    }

    /// Render a simple quad (debug/test)
    pub fn render_simple_quad(&self) {
        // In full implementation:
        // - Set up vertex/index buffers for a full-screen quad
        // - Set pixel shader for solid color or gradient
        // - Issue draw call
        tracing::trace!("DX11 render simple quad (stub)");
    }

    /// Check if context is initialized and ready
    pub fn is_initialized(&self) -> bool {
        // For stub, consider initialized if width/height > 0
        self.width > 0 && self.height > 0
    }

    /// Get raw swap chain pointer (for FFI)
    pub fn get_swap_chain(&self) -> *mut std::ffi::c_void {
        self.swap_chain
    }

    /// Get raw device context pointer (for FFI)
    pub fn get_device_context(&self) -> *mut std::ffi::c_void {
        self.device_context
    }
}
