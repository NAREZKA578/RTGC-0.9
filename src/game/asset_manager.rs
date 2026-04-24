//! Asset manager stub module
//! TODO: Implement proper asset management

use std::collections::HashMap;
use std::sync::RwLock;

pub struct AssetManager;

impl AssetManager {
    pub fn new() -> Self {
        Self
    }

    pub fn load_texture(&mut self, path: &str) -> Result<(), String> {
        let _ = path;
        Ok(())
    }

    pub fn load_model(&mut self, path: &str) -> Result<(), String> {
        let _ = path;
        Ok(())
    }
}

impl Default for AssetManager {
    fn default() -> Self {
        Self::new()
    }
}