// Render Hardware Interface (RHI) - Universal Abstraction Layer
// Provides unified interface for Vulkan, DirectX 12, and OpenGL backends
// Designed for multi-threaded command recording and PSO-based rendering

use std::sync::Arc;
use std::fmt;

/// Resource handle for GPU resources
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResourceHandle(pub u64);

impl ResourceHandle {
    pub const INVALID: Self = ResourceHandle(u64::MAX);
    
    pub fn is_valid(&self) -> bool {
        self.0 != u64::MAX
    }
}

/// Vertex attribute format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VertexFormat {
    Float32x2,
    Float32x3,
    Float32x4,
    Float32x2x2, // mat2
    Float32x3x3, // mat3
    Float32x4x4, // mat4
    Uint8x4Norm,
    Uint16x2Norm,
    Uint16x4Norm,
}

impl VertexFormat {
    pub fn size_bytes(&self) -> usize {
        match self {
            VertexFormat::Float32x2 => 8,
            VertexFormat::Float32x3 => 12,
            VertexFormat::Float32x4 => 16,
            VertexFormat::Float32x2x2 => 16,
            VertexFormat::Float32x3x3 => 36,
            VertexFormat::Float32x4x4 => 64,
            VertexFormat::Uint8x4Norm => 4,
            VertexFormat::Uint16x2Norm => 4,
            VertexFormat::Uint16x4Norm => 8,
        }
    }
}

/// Vertex attribute description
#[derive(Debug, Clone)]
pub struct VertexAttribute {
    pub name: String,
    pub format: VertexFormat,
    pub offset: u32,
}

/// Input layout for vertex shader
#[derive(Debug, Clone)]
pub struct InputLayout {
    pub attributes: Vec<VertexAttribute>,
    pub stride: u32,
}

impl InputLayout {
    pub fn new(attributes: Vec<VertexAttribute>) -> Self {
        let stride = attributes.iter().map(|a| a.format.size_bytes() as u32).sum();
        Self { attributes, stride }
    }
}

/// Shader stage type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderStage {
    Vertex,
    Fragment,
    Compute,
    Geometry,
    TessellationControl,
    TessellationEvaluation,
}

/// Shader description
#[derive(Debug, Clone)]
pub struct ShaderDescription {
    pub stage: ShaderStage,
    pub source: Vec<u8>, // SPIR-V bytecode or HLSL source
    pub entry_point: String,
}

impl ShaderDescription {
    /// Alias for source field for backwards compatibility
    pub fn bytecode(&self) -> &[u8] {
        &self.source
    }
}

/// Blend mode for color blending
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    Zero,
    One,
    SrcColor,
    OneMinusSrcColor,
    DstColor,
    OneMinusDstColor,
    SrcAlpha,
    OneMinusSrcAlpha,
    DstAlpha,
    OneMinusDstAlpha,
}

/// Blend operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendOp {
    Add,
    Subtract,
    ReverseSubtract,
    Min,
    Max,
}

/// Color blend state for render target
#[derive(Debug, Clone)]
pub struct ColorBlendState {
    pub enabled: bool,
    pub src_color_blend: BlendMode,
    pub dst_color_blend: BlendMode,
    pub color_blend_op: BlendOp,
    pub src_alpha_blend: BlendMode,
    pub dst_alpha_blend: BlendMode,
    pub alpha_blend_op: BlendOp,
    pub write_mask: u8, // RGBA bitmask
}

impl Default for ColorBlendState {
    fn default() -> Self {
        Self {
            enabled: false,
            src_color_blend: BlendMode::One,
            dst_color_blend: BlendMode::Zero,
            color_blend_op: BlendOp::Add,
            src_alpha_blend: BlendMode::One,
            dst_alpha_blend: BlendMode::Zero,
            alpha_blend_op: BlendOp::Add,
            write_mask: 0xF, // Enable all channels
        }
    }
}

/// Depth/stencil test function
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CompareFunc {
    Never,
    Less,
    Equal,
    LessEqual,
    Greater,
    NotEqual,
    GreaterEqual,
    #[default]
    Always,
}

/// Depth state
#[derive(Debug, Clone)]
pub struct DepthState {
    pub enabled: bool,
    pub write_enabled: bool,
    pub compare_func: CompareFunc,
}

impl Default for DepthState {
    fn default() -> Self {
        Self {
            enabled: true,
            write_enabled: true,
            compare_func: CompareFunc::Less,
        }
    }
}

/// Stencil operation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StencilOp {
    #[default]
    Keep,
    Zero,
    Replace,
    IncrementClamp,
    DecrementClamp,
    Invert,
    IncrementWrap,
    DecrementWrap,
}

/// Stencil face state
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StencilFaceState {
    pub fail_op: StencilOp,
    pub depth_fail_op: StencilOp,
    pub pass_op: StencilOp,
    pub compare_func: CompareFunc,
}

/// Stencil state
#[derive(Debug, Clone)]
pub struct StencilState {
    pub enabled: bool,
    pub front_face: StencilFaceState,
    pub back_face: StencilFaceState,
    pub read_mask: u8,
    pub write_mask: u8,
    pub reference: u8,
}

impl Default for StencilState {
    fn default() -> Self {
        Self {
            enabled: false,
            front_face: StencilFaceState {
                fail_op: StencilOp::Keep,
                depth_fail_op: StencilOp::Keep,
                pass_op: StencilOp::Keep,
                compare_func: CompareFunc::Always,
            },
            back_face: StencilFaceState {
                fail_op: StencilOp::Keep,
                depth_fail_op: StencilOp::Keep,
                pass_op: StencilOp::Keep,
                compare_func: CompareFunc::Always,
            },
            read_mask: 0xFF,
            write_mask: 0xFF,
            reference: 0,
        }
    }
}

/// Cull mode for face culling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CullMode {
    None,
    Front,
    Back,
}

/// Front face winding order
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrontFace {
    CounterClockwise,
    Clockwise,
}

/// Rasterizer state
#[derive(Debug, Clone)]
pub struct RasterizerState {
    pub cull_mode: CullMode,
    pub front_face: FrontFace,
    pub fill_mode: FillMode,
    pub polygon_offset_factor: f32,
    pub polygon_offset_units: f32,
}

impl Default for RasterizerState {
    fn default() -> Self {
        Self {
            cull_mode: CullMode::Back,
            front_face: FrontFace::CounterClockwise,
            fill_mode: FillMode::Solid,
            polygon_offset_factor: 0.0,
            polygon_offset_units: 0.0,
        }
    }
}

/// Fill mode for polygons
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillMode {
    Solid,
    Wireframe,
    Point,
}

/// Pipeline State Object (PSO) - immutable render state
#[derive(Debug, Clone)]
pub struct PipelineStateObject {
    pub vertex_shader: ResourceHandle,
    pub fragment_shader: Option<ResourceHandle>,
    pub compute_shader: Option<ResourceHandle>,
    pub input_layout: InputLayout,
    pub color_blend_states: Vec<ColorBlendState>, // One per render target
    pub depth_state: DepthState,
    pub stencil_state: StencilState,
    pub rasterizer_state: RasterizerState,
    pub primitive_topology: PrimitiveTopology,
    pub sample_count: u32,
}

impl PipelineStateObject {
    /// Get all shaders as a vector for backwards compatibility
    pub fn shaders(&self) -> Vec<ResourceHandle> {
        let mut shaders = Vec::new();
        shaders.push(self.vertex_shader);
        if let Some(fs) = self.fragment_shader {
            shaders.push(fs);
        }
        if let Some(cs) = self.compute_shader {
            shaders.push(cs);
        }
        shaders
    }
}

/// Primitive topology for drawing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveTopology {
    PointList,
    LineList,
    LineStrip,
    TriangleList,
    TriangleStrip,
}

/// Texture dimension
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureDimension {
    D1,
    D2,
    D3,
    Cube,
}

/// Texture type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureType {
    Texture1D,
    Texture2D,
    Texture3D,
    TextureCube,
    Texture1DArray,
    Texture2DArray,
    TextureCubeArray,
}

/// Texture format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureFormat {
    R8Unorm,
    R8G8Unorm,
    R8G8B8Unorm,
    R8G8B8A8Unorm,
    R8G8B8A8Srgb,
    R8Uint,
    R16Float,
    R16G16Float,
    R16G16B16A16Float,
    R32Float,
    R32G32Float,
    R32G32B32A32Float,
    Rg8Unorm,
    Rg16Float,
    Rg32Float,
    Rgba8Unorm,
    Rgba8Uint,
    Rgba8Snorm,
    Rgba16Float,
    Rgba32Float,
    Bgra8Unorm,
    D16Unorm,
    D24UnormS8Uint,
    D32Float,
    D32FloatS8UintX24,
    Depth16Unorm,
    Depth24Plus,
    Depth32Float,
    Stencil8,
    Depth24PlusStencil8,
    Depth32FloatStencil8,
    BC1RgbUnorm,
    BC1RgbaUnorm,
    BC2Unorm,
    BC3Unorm,
    BC3RgbaUnorm,
    BC4Unorm,
    BC5Unorm,
    BC6HUfloat,
    BC7Unorm,
    BC7RgbaUnorm,
    // Дополнительные форматы для совместимости с gl.rs
    R8G8B8UnormSrgb,
    R8G8B8A8UnormSrgb,
    R16Uint,
    R16Sint,
    R16G16Uint,
    R16G16Sint,
    R16G16B16A16Uint,
    R16G16B16A16Sint,
    R32Uint,
    R32Sint,
    R32G32Uint,
    R32G32Sint,
    R32G32B32A32Uint,
    R32G32B32A32Sint,
    Rg8Uint,
    Rg8Sint,
    Rg16Uint,
    Rg16Sint,
    Rg32Uint,
    Rg32Sint,
    Rgba8Sint,
    Rgba16Uint,
    Rgba16Sint,
    Rgba32Uint,
    Rgba32Sint,
    Bgra8UnormSrgb,
    Bgr8Unorm,
    Bgr8UnormSrgb,
    Rgb10A2Unorm,
    R11G11B10Float,
    R9G9B9E5Float,
    Depth16,
    Depth24,
    Depth32,
    Depth24Stencil8,
    Depth32Stencil8,
    BC1RgbaUnormSrgb,
    BC2UnormSrgb,
    BC3UnormSrgb,
    BC4Snorm,
    BC5Snorm,
    BC6HFloat,
    BC7UnormSrgb,
}

impl TextureFormat {
    pub fn is_depth_format(&self) -> bool {
        matches!(
            self,
            TextureFormat::D16Unorm
                | TextureFormat::D24UnormS8Uint
                | TextureFormat::D32Float
                | TextureFormat::D32FloatS8UintX24
                | TextureFormat::Depth16Unorm
                | TextureFormat::Depth24Plus
                | TextureFormat::Depth32Float
                | TextureFormat::Stencil8
                | TextureFormat::Depth24PlusStencil8
                | TextureFormat::Depth32FloatStencil8
                | TextureFormat::Depth16
                | TextureFormat::Depth24
                | TextureFormat::Depth32
                | TextureFormat::Depth24Stencil8
                | TextureFormat::Depth32Stencil8
        )
    }

    pub fn is_compressed(&self) -> bool {
        matches!(
            self,
            TextureFormat::BC1RgbUnorm
                | TextureFormat::BC1RgbaUnorm
                | TextureFormat::BC1RgbaUnormSrgb
                | TextureFormat::BC2Unorm
                | TextureFormat::BC2UnormSrgb
                | TextureFormat::BC3Unorm
                | TextureFormat::BC3UnormSrgb
                | TextureFormat::BC3RgbaUnorm
                | TextureFormat::BC4Unorm
                | TextureFormat::BC4Snorm
                | TextureFormat::BC5Unorm
                | TextureFormat::BC5Snorm
                | TextureFormat::BC6HUfloat
                | TextureFormat::BC6HFloat
                | TextureFormat::BC7Unorm
                | TextureFormat::BC7UnormSrgb
                | TextureFormat::BC7RgbaUnorm
        )
    }
}

/// Texture description
#[derive(Debug, Clone)]
pub struct TextureDescription {
    pub dimension: TextureDimension,
    pub texture_type: TextureType,
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub depth_or_array_layers: u32,
    pub mip_levels: u32,
    pub format: TextureFormat,
    pub usage: TextureUsage,
    pub initial_state: ResourceState,
}

impl TextureDescription {
    /// Get depth value (alias for depth_or_array_layers)
    pub fn depth(&self) -> u32 {
        self.depth_or_array_layers
    }
}

/// Texture usage flags
bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct TextureUsage: u32 {
        const SHADER_READ = 1 << 0;
        const SHADER_WRITE = 1 << 1;
        const RENDER_TARGET = 1 << 2;
        const DEPTH_STENCIL = 1 << 3;
        const TRANSFER_SRC = 1 << 4;
        const TRANSFER_DST = 1 << 5;
        const STORAGE = 1 << 6;
        const PRESENT = 1 << 7;
    }
}

/// Resource state for barrier synchronization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceState {
    Undefined,
    Common,
    VertexBuffer,
    IndexBuffer,
    ConstantBuffer,
    ShaderResource,
    UnorderedAccess,
    RenderTarget,
    DepthWrite,
    DepthRead,
    Present,
    TransferSource,
    TransferDestination,
}

/// Buffer type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferType {
    Vertex,
    Index,
    Constant,
    Storage,
    Indirect,
    Uniform,
}

/// Buffer description
#[derive(Debug, Clone)]
pub struct BufferDescription {
    pub buffer_type: BufferType,
    pub size: u64,
    pub usage: BufferUsage,
    pub initial_state: ResourceState,
}

/// Alias for BufferDescription
pub type BufferDesc = BufferDescription;

/// Buffer usage flags
bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct BufferUsage: u32 {
        const VERTEX_BUFFER = 1 << 0;
        const INDEX_BUFFER = 1 << 1;
        const CONSTANT_BUFFER = 1 << 2;
        const SHADER_RESOURCE = 1 << 3;
        const UNORDERED_ACCESS = 1 << 4;
        const TRANSFER_SRC = 1 << 5;
        const TRANSFER_DST = 1 << 6;
        const STORAGE_BUFFER = 1 << 7;
        const INDIRECT_BUFFER = 1 << 8;
        const IMMUTABLE = 1 << 9;
        const DYNAMIC = 1 << 10;
        const TRANSIENT = 1 << 11;
        const UPLOAD = 1 << 12;
        const READBACK = 1 << 13;
    }
}

impl BufferUsage {
    /// Alias for IMMUTABLE for backwards compatibility
    pub const Immutable: BufferUsage = BufferUsage::IMMUTABLE;
    /// Alias for DYNAMIC for backwards compatibility
    pub const Dynamic: BufferUsage = BufferUsage::DYNAMIC;
    /// Alias for TRANSIENT for backwards compatibility
    pub const Transient: BufferUsage = BufferUsage::TRANSIENT;
    /// Alias for UPLOAD for backwards compatibility
    pub const Upload: BufferUsage = BufferUsage::UPLOAD;
    /// Alias for READBACK for backwards compatibility
    pub const Readback: BufferUsage = BufferUsage::READBACK;
}

/// Sampler filter mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterMode {
    Point,
    Bilinear,
    Trilinear,
    Anisotropic,
}

/// Sampler address mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressMode {
    ClampToEdge,
    Wrap,
    Mirror,
    Border,
    MirrorOnce,
}

/// Sampler description
#[derive(Debug, Clone)]
pub struct SamplerDescription {
    pub min_filter: FilterMode,
    pub mag_filter: FilterMode,
    pub mip_filter: FilterMode,
    pub address_u: AddressMode,
    pub address_v: AddressMode,
    pub address_w: AddressMode,
    pub mip_lod_bias: f32,
    pub max_anisotropy: u32,
    pub compare_func: Option<CompareFunc>,
    pub min_lod: f32,
    pub max_lod: f32,
    pub border_color: [f32; 4],
}

impl Default for SamplerDescription {
    fn default() -> Self {
        Self {
            min_filter: FilterMode::Bilinear,
            mag_filter: FilterMode::Bilinear,
            mip_filter: FilterMode::Bilinear,
            address_u: AddressMode::Wrap,
            address_v: AddressMode::Wrap,
            address_w: AddressMode::Wrap,
            mip_lod_bias: 0.0,
            max_anisotropy: 1,
            compare_func: None,
            min_lod: 0.0,
            max_lod: f32::MAX,
            border_color: [0.0; 4],
        }
    }
}

/// Command list type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandListType {
    Direct,     // Graphics + compute + transfer
    Compute,    // Compute only
    Copy,       // Transfer only
}

/// Clear value for render targets / depth buffers
#[derive(Debug, Clone, Copy)]
pub enum ClearValue {
    Color([f32; 4]),
    Depth(f32),
    DepthStencil(f32, u8),
}

/// Viewport
#[derive(Debug, Clone, Copy)]
pub struct Viewport {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub min_depth: f32,
    pub max_depth: f32,
}

impl Viewport {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width,
            height,
            min_depth: 0.0,
            max_depth: 1.0,
        }
    }
    
    pub fn full_screen(width: u32, height: u32) -> Self {
        Self::new(width as f32, height as f32)
    }
}

/// Scissor rect
#[derive(Debug, Clone, Copy)]
pub struct ScissorRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl ScissorRect {
    /// Get x position (alias for left)
    pub fn x(&self) -> i32 {
        self.left
    }
    
    /// Get y position (alias for top)
    pub fn y(&self) -> i32 {
        self.top
    }
    
    /// Get width (right - left)
    pub fn width(&self) -> i32 {
        self.right - self.left
    }
    
    /// Get height (bottom - top)
    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }
    
    pub fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self { left, top, right, bottom }
    }
    
    pub fn full_screen(width: u32, height: u32) -> Self {
        Self {
            left: 0,
            top: 0,
            right: width as i32,
            bottom: height as i32,
        }
    }
}

/// Draw indexed indirect structure
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DrawIndexedIndirectArgs {
    pub index_count: u32,
    pub instance_count: u32,
    pub start_index: u32,
    pub base_vertex: i32,
    pub start_instance: u32,
}

/// Draw indirect structure
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DrawIndirectArgs {
    pub vertex_count: u32,
    pub instance_count: u32,
    pub start_vertex: u32,
    pub start_instance: u32,
}

/// Dispatch indirect structure
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DispatchIndirectArgs {
    pub group_count_x: u32,
    pub group_count_y: u32,
    pub group_count_z: u32,
}

/// Resource error types
#[derive(Debug, Clone)]
pub enum RhiError {
    InitializationFailed(String),
    OutOfMemory,
    DeviceLost,
    InvalidParameter(String),
    ShaderCompilationFailed(String),
    CompilationFailed(String),
    InvalidResourceHandle(String),
    ResourceCreationFailed(String),
    QueueFull,
    Timeout,
    Unsupported(String),
}

impl fmt::Display for RhiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RhiError::InitializationFailed(msg) => write!(f, "RHI initialization failed: {}", msg),
            RhiError::OutOfMemory => write!(f, "Out of memory"),
            RhiError::DeviceLost => write!(f, "Device lost"),
            RhiError::InvalidParameter(msg) => write!(f, "Invalid parameter: {}", msg),
            RhiError::ShaderCompilationFailed(msg) => write!(f, "Shader compilation failed: {}", msg),
            RhiError::CompilationFailed(msg) => write!(f, "Compilation failed: {}", msg),
            RhiError::InvalidResourceHandle(msg) => write!(f, "Invalid resource handle: {}", msg),
            RhiError::ResourceCreationFailed(msg) => write!(f, "Resource creation failed: {}", msg),
            RhiError::QueueFull => write!(f, "Command queue full"),
            RhiError::Timeout => write!(f, "Operation timeout"),
            RhiError::Unsupported(msg) => write!(f, "Unsupported: {}", msg),
        }
    }
}

impl std::error::Error for RhiError {}

impl From<String> for RhiError {
    fn from(msg: String) -> Self {
        RhiError::InitializationFailed(msg)
    }
}

pub type RhiResult<T> = Result<T, RhiError>;

// Type aliases for backwards compatibility
pub type TextureDesc = TextureDescription;
pub type SamplerDesc = SamplerDescription;
pub type PipelineDesc = PipelineStateObject;

/// 4-component color (RGBA)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color4f {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color4f {
    pub const WHITE: Self = Self { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
    pub const BLACK: Self = Self { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };

    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn black() -> Self {
        Self { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }
    }

    pub fn white() -> Self {
        Self { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }
    }
}

impl Default for Color4f {
    fn default() -> Self {
        Self::WHITE
    }
}

/// 2D rectangle
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect2D {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Rect2D {
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self { x, y, width, height }
    }
}

/// Index format for indexed rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexType {
    Uint16,
    Uint32,
}

/// Blend state for render pipeline
pub type BlendState = ColorBlendState;

/// Depth stencil state
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DepthStencilState {
    pub depth_test_enabled: bool,
    pub depth_write_enabled: bool,
    pub depth_compare_func: CompareFunc,
    pub stencil_test_enabled: bool,
    pub stencil_front: StencilFaceState,
    pub stencil_back: StencilFaceState,
    pub stencil_read_mask: u8,
    pub stencil_write_mask: u8,
}

impl Default for DepthStencilState {
    fn default() -> Self {
        Self {
            depth_test_enabled: true,
            depth_write_enabled: true,
            depth_compare_func: CompareFunc::Less,
            stencil_test_enabled: false,
            stencil_front: StencilFaceState::default(),
            stencil_back: StencilFaceState::default(),
            stencil_read_mask: 0xFF,
            stencil_write_mask: 0xFF,
        }
    }
}

// Re-export IDevice and ICommandList as RhiDevice and RhiCommandList for backwards compatibility
pub use super::device::IDevice as RhiDevice;
pub use super::device::ICommandList as RhiCommandList;

/// RHI Buffer trait alias
pub trait RhiBuffer: Send + Sync {}

/// RHI Texture trait alias
pub trait RhiTexture: Send + Sync {}

/// RHI Sampler trait alias
pub trait RhiSampler: Send + Sync {}

/// RHI Pipeline trait alias
pub trait RhiPipeline: Send + Sync {}
