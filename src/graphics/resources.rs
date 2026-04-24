//! Resource management stub module
//! TODO: Implement proper resource management

use std::collections::HashMap;
use std::sync::RwLock;

pub struct ResourceManager;

impl ResourceManager {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ResourceManager {
    fn default() -> Self {
        Self::new()
    }
}