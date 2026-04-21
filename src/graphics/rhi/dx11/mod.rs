//! DirectX 11 Backend - Stubs (DX11 реально не работает без переписывания RHI)

pub mod buffer_dx11;
pub mod context_dx11;
pub mod device_dx11;
pub mod pipeline_dx11;
pub mod shader_dx11;
pub mod swapchain_dx11;
pub mod texture_dx11;

pub use device_dx11::Dx11Device;
