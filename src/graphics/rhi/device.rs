// RHI Core Traits - Device, Command Context, and Resource interfaces
// Abstracts GPU operations across Vulkan, DirectX 12, and OpenGL

use super::types::*;
use std::sync::Arc;

/// GPU Device - represents physical GPU and creates all resources
pub trait IDevice: Send + Sync {
    /// Get device name/description
    fn get_device_name(&self) -> &str;
    
    /// Get supported features
    fn get_features(&self) -> DeviceFeatures;
    
    /// Get device limits
    fn get_limits(&self) -> DeviceLimits;
    
    // ==================== Resource Creation ====================
    
    /// Create a buffer resource
    fn create_buffer(&self, desc: &BufferDescription) -> RhiResult<ResourceHandle>;
    
    /// Create a texture resource
    fn create_texture(&self, desc: &TextureDescription) -> RhiResult<ResourceHandle>;
    
    /// Create a texture view (SRV/RTV/DSV/UAV)
    fn create_texture_view(
        &self,
        texture: ResourceHandle,
        desc: &TextureViewDescription,
    ) -> RhiResult<ResourceHandle>;
    
    /// Create a sampler
    fn create_sampler(&self, desc: &SamplerDescription) -> RhiResult<ResourceHandle>;
    
    /// Create a shader from SPIR-V or other bytecode
    fn create_shader(&self, desc: &ShaderDescription) -> RhiResult<ResourceHandle>;
    
    /// Create a Pipeline State Object (PSO)
    fn create_pipeline_state(&self, desc: &PipelineStateObject) -> RhiResult<ResourceHandle>;
    
    /// Create a descriptor heap/set for binding resources
    fn create_descriptor_heap(&self, desc: &DescriptorHeapDescription) -> RhiResult<ResourceHandle>;
    
    // ==================== Command List Creation ====================
    
    /// Create a command list for recording GPU commands
    fn create_command_list(&self, cmd_type: CommandListType) -> RhiResult<Arc<dyn ICommandList>>;
    
    /// Create a command queue for submitting command lists
    fn create_command_queue(&self, cmd_type: CommandListType) -> RhiResult<Arc<dyn ICommandQueue>>;
    
    // ==================== Synchronization ====================
    
    /// Create a fence for CPU-GPU synchronization
    fn create_fence(&self, initial_value: u64) -> RhiResult<Arc<dyn IFence>>;
    
    /// Create a semaphore for GPU-GPU synchronization
    fn create_semaphore(&self) -> RhiResult<Arc<dyn ISemaphore>>;
    
    // ==================== Swap Chain ====================
    
    /// Create a swap chain for presenting to a window
    fn create_swap_chain(
        &self,
        window_handle: *mut std::ffi::c_void,
        width: u32,
        height: u32,
        format: TextureFormat,
        vsync: bool,
    ) -> RhiResult<Arc<dyn ISwapChain>>;
    
    // ==================== Resource Management ====================
    
    /// Update buffer data
    fn update_buffer(
        &self,
        buffer: ResourceHandle,
        offset: u64,
        data: &[u8],
    ) -> RhiResult<()>;
    
    /// Map a buffer for CPU write access (returns pointer)
    fn map_buffer(&self, buffer: ResourceHandle) -> RhiResult<*mut u8>;
    
    /// Unmap a buffer after CPU write
    fn unmap_buffer(&self, buffer: ResourceHandle);
    
    /// Read back texture data to CPU
    fn read_back_texture(&self, texture: ResourceHandle) -> RhiResult<Vec<u8>>;
    
    /// Destroy a resource
    fn destroy_resource(&self, handle: ResourceHandle);
    
    /// Wait for GPU to finish all work
    fn wait_idle(&self) -> RhiResult<()>;
    
    /// Get memory statistics
    fn get_memory_stats(&self) -> MemoryStats;
}

/// Command List - records GPU commands for submission
pub trait ICommandList: Send + Sync {
    /// Reset the command list for re-recording
    fn reset(&mut self) -> RhiResult<()>;
    
    /// Close the command list for submission
    fn close(&mut self) -> RhiResult<()>;
    
    // ==================== Render Pass ====================
    
    /// Begin a render pass
    fn begin_render_pass(&mut self, desc: &RenderPassDescription);
    
    /// End the current render pass
    fn end_render_pass(&mut self);
    
    // ==================== Pipeline & State ====================
    
    /// Set the pipeline state object
    fn set_pipeline_state(&mut self, pso: ResourceHandle);
    
    /// Set primitive topology
    fn set_primitive_topology(&mut self, topology: PrimitiveTopology);
    
    /// Set viewport
    fn set_viewport(&mut self, viewport: &Viewport);
    
    /// Set scissor rect
    fn set_scissor_rect(&mut self, scissor: &ScissorRect);
    
    /// Set blend constants
    fn set_blend_constants(&mut self, constants: [f32; 4]);
    
    /// Set stencil reference value
    fn set_stencil_reference(&mut self, reference: u8);
    
    // ==================== Resource Binding ====================
    
    /// Bind vertex buffers
    fn bind_vertex_buffers(&mut self, start_slot: u32, buffers: &[(ResourceHandle, u64)]);
    
    /// Bind index buffer
    fn bind_index_buffer(&mut self, buffer: ResourceHandle, offset: u64, index_format: IndexFormat);
    
    /// Bind constant buffer to a shader stage
    fn bind_constant_buffer(
        &mut self,
        stage: ShaderStage,
        slot: u32,
        buffer: ResourceHandle,
    );
    
    /// Bind shader resource view
    fn bind_shader_resource(
        &mut self,
        stage: ShaderStage,
        slot: u32,
        view: ResourceHandle,
    );
    
    /// Bind sampler
    fn bind_sampler(&mut self, stage: ShaderStage, slot: u32, sampler: ResourceHandle);
    
    // ==================== Draw Commands ====================
    
    /// Draw vertices without indices
    fn draw(&mut self, vertex_count: u32, instance_count: u32, start_vertex: u32, start_instance: u32);
    
    /// Draw indexed vertices
    fn draw_indexed(
        &mut self,
        index_count: u32,
        instance_count: u32,
        start_index: u32,
        base_vertex: i32,
        start_instance: u32,
    );
    
    /// Indirect draw
    fn draw_indirect(&mut self, buffer: ResourceHandle, offset: u64, draw_count: u32);
    
    /// Indirect indexed draw
    fn draw_indexed_indirect(&mut self, buffer: ResourceHandle, offset: u64, draw_count: u32);
    
    // ==================== Compute ====================
    
    /// Dispatch compute work groups
    fn dispatch(&mut self, group_count_x: u32, group_count_y: u32, group_count_z: u32);
    
    /// Indirect dispatch
    fn dispatch_indirect(&mut self, buffer: ResourceHandle, offset: u64);
    
    // ==================== Resource Barriers ====================
    
    /// Transition resource states
    fn resource_barrier(&mut self, barriers: &[ResourceBarrier]);
    
    // ==================== Clear Operations ====================
    
    /// Clear render target
    fn clear_render_target(&mut self, view: ResourceHandle, color: [f32; 4]);
    
    /// Clear depth/stencil
    fn clear_depth_stencil(
        &mut self,
        view: ResourceHandle,
        clear_depth: Option<f32>,
        clear_stencil: Option<u8>,
    );
    
    // ==================== Debug ====================
    
    /// Insert debug marker
    fn insert_debug_marker(&mut self, name: &str);
    
    /// Begin debug group
    fn begin_debug_group(&mut self, name: &str);
    
    /// End debug group
    fn end_debug_group(&mut self);
}

/// Command Queue - submits command lists to GPU
pub trait ICommandQueue: Send + Sync {
    /// Submit command lists for execution
    fn submit(&self, command_lists: &[&dyn ICommandList], wait_semaphores: &[Arc<dyn ISemaphore>], signal_semaphores: &[Arc<dyn ISemaphore>]) -> RhiResult<()>;
    
    /// Present a swap chain image
    fn present(&self, swap_chain: &dyn ISwapChain) -> RhiResult<()>;
    
    /// Signal a fence after all submitted work completes
    fn signal(&self, fence: &dyn IFence, value: u64) -> RhiResult<()>;
    
    /// Wait for a fence to reach a value
    fn wait(&self, fence: &dyn IFence, value: u64, timeout_ms: u32) -> RhiResult<bool>;
}

/// Fence for CPU-GPU synchronization
pub trait IFence: Send + Sync {
    /// Get current fence value
    fn get_value(&self) -> u64;

    /// Set fence value (called by command queue when signal is submitted)
    fn set_value(&self, value: u64);

    /// Set event to be signaled when fence reaches value
    fn set_event_on_completion(&self, value: u64) -> RhiResult<Arc<dyn std::any::Any + Send + Sync>>;
}

/// Semaphore for GPU-GPU synchronization
pub trait ISemaphore: Send + Sync {}

/// Swap chain for presenting to window
pub trait ISwapChain: Send + Sync {
    /// Get the current back buffer index
    fn get_current_back_buffer_index(&self) -> u32;
    
    /// Get the back buffer texture
    fn get_back_buffer(&self) -> ResourceHandle;
    
    /// Resize the swap chain
    fn resize(&mut self, width: u32, height: u32) -> RhiResult<()>;
    
    /// Present the current frame
    fn present(&self) -> RhiResult<()>;
}

/// Texture view description
#[derive(Debug, Clone)]
pub struct TextureViewDescription {
    pub view_type: TextureViewType,
    pub format: TextureFormat,
    pub most_detailed_mip: u32,
    pub mip_level_count: u32,
    pub first_array_slice: u32,
    pub array_slice_count: u32,
}

/// Texture view type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureViewType {
    ShaderResource,
    RenderTarget,
    DepthStencil,
    UnorderedAccess,
}

/// Descriptor heap description
#[derive(Debug, Clone)]
pub struct DescriptorHeapDescription {
    pub heap_type: DescriptorHeapType,
    pub capacity: u32,
    pub flags: DescriptorHeapFlags,
}

/// Descriptor heap type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DescriptorHeapType {
    ConstantBufferView,
    ShaderResourceView,
    UnorderedAccessView,
    Sampler,
    RenderTargetView,
    DepthStencilView,
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct DescriptorHeapFlags: u32 {
        const SHADER_VISIBLE = 1 << 0;
    }
}

/// Render pass description
#[derive(Debug, Clone)]
pub struct RenderPassDescription {
    pub color_attachments: Vec<RenderAttachment>,
    pub depth_stencil_attachment: Option<DepthStencilAttachment>,
}

/// Render attachment description
#[derive(Debug, Clone)]
pub struct RenderAttachment {
    pub view: ResourceHandle,
    pub load_op: LoadOp,
    pub store_op: StoreOp,
    pub clear_value: Option<ClearValue>,
}

/// Depth/stencil attachment description
#[derive(Debug, Clone)]
pub struct DepthStencilAttachment {
    pub view: ResourceHandle,
    pub depth_load_op: LoadOp,
    pub depth_store_op: StoreOp,
    pub stencil_load_op: LoadOp,
    pub stencil_store_op: StoreOp,
    pub depth_clear_value: Option<f32>,
    pub stencil_clear_value: Option<u8>,
}

/// Load operation for attachments
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadOp {
    Load,
    Clear,
    Discard,
}

/// Store operation for attachments
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreOp {
    Store,
    Discard,
}

/// Index format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexFormat {
    Uint16,
    Uint32,
}

/// Resource barrier for state transitions
#[derive(Debug, Clone)]
pub struct ResourceBarrier {
    pub resource: ResourceHandle,
    pub state_before: ResourceState,
    pub state_after: ResourceState,
    pub subresource: u32, // Use u32::MAX for all subresources
}

impl ResourceBarrier {
    pub fn transition(resource: ResourceHandle, before: ResourceState, after: ResourceState) -> Self {
        Self {
            resource,
            state_before: before,
            state_after: after,
            subresource: u32::MAX,
        }
    }
}

/// Device features
#[derive(Debug, Clone, Default)]
pub struct DeviceFeatures {
    pub anisotropic_filtering: bool,
    pub bc_compression: bool,
    pub compute_shaders: bool,
    pub geometry_shaders: bool,
    pub tessellation: bool,
    pub conservative_rasterization: bool,
    pub multi_draw_indirect: bool,
    pub draw_indirect_first_instance: bool,
    pub dual_source_blending: bool,
    pub depth_bounds_test: bool,
    pub sample_rate_shading: bool,
    pub texture_cube_map_array: bool,
    pub texture_3d_as_2d_array: bool,
    pub independent_blend: bool,
    pub logic_op: bool,
    pub occlusion_query: bool,
    pub timestamp_query: bool,
    pub pipeline_statistics_query: bool,
    pub stream_output: bool,
    pub variable_rate_shading: bool,
    pub mesh_shaders: bool,
    pub ray_tracing: bool,
    pub sampler_lod_bias: bool,
    pub border_color_clamp: bool,
}

/// Device limits
#[derive(Debug, Clone, Default)]
pub struct DeviceLimits {
    pub max_texture_dimension_1d: u32,
    pub max_texture_dimension_2d: u32,
    pub max_texture_dimension_3d: u32,
    pub max_texture_array_layers: u32,
    pub max_buffer_size: u64,
    pub max_vertex_input_attributes: u32,
    pub max_vertex_input_bindings: u32,
    pub max_vertex_input_attribute_offset: u32,
    pub max_vertex_input_binding_stride: u32,
    pub max_vertex_output_components: u32,
    pub max_fragment_input_components: u32,
    pub max_fragment_output_attachments: u32,
    pub max_compute_work_group_count: [u32; 3],
    pub max_compute_work_group_invocations: u32,
    pub max_compute_shared_memory_size: u32,
    pub max_uniform_buffer_range: u32,
    pub max_storage_buffer_range: u32,
    pub max_sampler_anisotropy: f32,
    pub min_texel_buffer_offset_alignment: u32,
    pub min_uniform_buffer_offset_alignment: u32,
    pub min_storage_buffer_offset_alignment: u32,
    pub max_descriptor_set_samplers: u32,
    pub max_descriptor_set_uniform_buffers: u32,
    pub max_descriptor_set_storage_buffers: u32,
    pub max_descriptor_set_textures: u32,
    pub max_descriptor_set_storage_images: u32,
    pub max_per_stage_descriptor_samplers: u32,
    pub max_per_stage_descriptor_uniform_buffers: u32,
    pub max_per_stage_descriptor_storage_buffers: u32,
    pub max_per_stage_descriptor_textures: u32,
    pub max_per_stage_descriptor_storage_images: u32,
}

/// Memory statistics
#[derive(Debug, Clone, Default)]
pub struct MemoryStats {
    pub total_gpu_memory: u64,
    pub used_gpu_memory: u64,
    pub total_upload_memory: u64,
    pub used_upload_memory: u64,
    pub total_download_memory: u64,
    pub used_download_memory: u64,
}
