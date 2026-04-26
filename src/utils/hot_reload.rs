//! Hot reload functionality stub module
//! TODO: Implement hot reload for dev_tools feature

use std::time::Duration;

pub struct HotReloadConfig {
    pub enabled: bool,
    pub poll_interval: Duration,
}

impl Default for HotReloadConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            poll_interval: Duration::from_millis(500),
        }
    }
}

pub struct HotReloadManager {
    config: HotReloadConfig,
}

impl HotReloadManager {
    pub fn new(config: HotReloadConfig) -> Self {
        Self { config }
    }

    pub fn poll(&mut self) -> bool {
        false
    }
}

impl Default for HotReloadManager {
    fn default() -> Self {
        Self::new(HotReloadConfig::default())
    }
}