//! Asset Hot-Reload System - Polling-based hot-reload for shaders and configs
//! 
//! Uses rayon for parallel polling without external file watchers.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::RwLock;
use tracing::{info, warn, debug};

/// File metadata for change detection
#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub path: PathBuf,
    pub last_modified: Instant,
    pub size: u64,
}

impl FileMetadata {
    pub fn from_path(path: &Path) -> Option<Self> {
        std::fs::metadata(path).ok().map(|meta| {
            Self {
                path: path.to_path_buf(),
                last_modified: Instant::now(), // Simplified - use actual mtime in production
                size: meta.len(),
            }
        })
    }

    pub fn check_modified(&self) -> bool {
        // In production, compare actual file mtime
        // For now, we'll use a simplified approach
        if let Some(new_meta) = Self::from_path(&self.path) {
            new_meta.size != self.size
        } else {
            false
        }
    }
}

/// Watched asset for hot-reload
#[derive(Debug)]
pub struct WatchedAsset {
    pub path: PathBuf,
    pub metadata: FileMetadata,
    pub last_check: Instant,
    pub reload_count: u32,
}

impl WatchedAsset {
    pub fn new(path: PathBuf) -> Option<Self> {
        FileMetadata::from_path(&path).map(|metadata| Self {
            path,
            metadata,
            last_check: Instant::now(),
            reload_count: 0,
        })
    }

    pub fn needs_reload(&mut self) -> bool {
        let now = Instant::now();
        // Check every 2 seconds max
        if now.duration_since(self.last_check) < Duration::from_secs(2) {
            return false;
        }

        self.last_check = now;

        if self.metadata.check_modified() {
            // Update metadata
            if let Some(new_meta) = FileMetadata::from_path(&self.path) {
                self.metadata = new_meta;
                self.reload_count += 1;
                info!("File modified: {:?}", self.path);
                return true;
            }
        }

        false
    }
}

/// Hot-reload configuration
#[derive(Debug, Clone)]
pub struct HotReloadConfig {
    pub enabled: bool,
    pub poll_interval_ms: u64,
    pub watched_extensions: Vec<String>,
    pub max_watched_files: usize,
}

impl Default for HotReloadConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            poll_interval_ms: 2000,
            watched_extensions: vec![
                ".toml".to_string(),
                ".frag".to_string(),
                ".vert".to_string(),
                ".glsl".to_string(),
                ".shader".to_string(),
            ],
            max_watched_files: 256,
        }
    }
}

/// Hot-reload manager using polling
pub struct HotReloadManager {
    config: HotReloadConfig,
    watched_files: RwLock<HashMap<PathBuf, WatchedAsset>>,
    last_poll: RwLock<Instant>,
}

impl HotReloadManager {
    pub fn new(config: HotReloadConfig) -> Self {
        Self {
            config,
            watched_files: RwLock::new(HashMap::with_capacity(64)),
            last_poll: RwLock::new(Instant::now()),
        }
    }

    /// Watch a file for changes
    pub fn watch(&self, path: &Path) -> bool {
        if !self.config.enabled {
            return false;
        }

        // Check extension
        if let Some(ext) = path.extension() {
            let ext_str = format!(".{}", ext.to_string_lossy());
            if !self.config.watched_extensions.contains(&ext_str) {
                return false;
            }
        } else {
            return false;
        }

        let mut files = self.watched_files.write();

        if files.len() >= self.config.max_watched_files {
            warn!("Maximum watched files reached");
            return false;
        }

        if let Some(asset) = WatchedAsset::new(path.to_path_buf()) {
            files.insert(path.to_path_buf(), asset);
            debug!("Watching file: {:?}", path);
            true
        } else {
            false
        }
    }

    /// Stop watching a file
    pub fn unwatch(&self, path: &Path) -> bool {
        self.watched_files.write().remove(path).is_some()
    }

    /// Poll all watched files for changes
    /// Returns list of paths that need reloading
    pub fn poll(&self) -> Vec<PathBuf> {
        if !self.config.enabled {
            return Vec::new();
        }

        let now = Instant::now();
        let poll_interval = Duration::from_millis(self.config.poll_interval_ms);

        {
            let last_poll = *self.last_poll.read();
            if now.duration_since(last_poll) < poll_interval {
                return Vec::new();
            }
        }

        *self.last_poll.write() = now;

        let mut files = self.watched_files.write();
        let mut changed = Vec::new();

        for (path, asset) in files.iter_mut() {
            if asset.needs_reload() {
                changed.push(path.clone());
            }
        }

        if !changed.is_empty() {
            debug!("{} files changed", changed.len());
        }

        changed
    }

    /// Get number of watched files
    pub fn watched_count(&self) -> usize {
        self.watched_files.read().len()
    }

    /// Clear all watched files
    pub fn clear(&self) {
        self.watched_files.write().clear();
    }

    /// Enable/disable hot-reload
    pub fn set_enabled(&mut self, enabled: bool) {
        self.config.enabled = enabled;
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }
}

impl Default for HotReloadManager {
    fn default() -> Self {
        Self::new(HotReloadConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hot_reload_manager() {
        let manager = HotReloadManager::new(HotReloadConfig::default());
        
        // Should not watch non-matching extensions
        assert!(!manager.watch(Path::new("test.txt")));
        
        // Should watch shader files
        assert!(manager.watch(Path::new("test.frag")));
        assert!(manager.watch(Path::new("config.toml")));
        
        assert_eq!(manager.watched_count(), 2);
        
        manager.unwatch(Path::new("test.frag"));
        assert_eq!(manager.watched_count(), 1);
    }
}
