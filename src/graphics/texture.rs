//! Texture management stub module
//! TODO: Implement proper texture management

use crate::graphics::rhi::types::{TextureDescription, TextureFormat};

pub struct TextureManager;

impl TextureManager {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TextureManager {
    fn default() -> Self {
        Self::new()
    }
}