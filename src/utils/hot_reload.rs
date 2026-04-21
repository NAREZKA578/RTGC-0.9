//! Hot Reload System - Monitor and reload assets/configs at runtime

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use std::fs;
use tracing::{info, warn, debug, error};

/// Configuration for hot reload system
#[derive(Debug, Clone)]
pub struct HotReloadConfig {
    /// Enable hot reload
    pub enabled: bool,
    /// Polling interval in milliseconds
    pub poll_interval_ms: u64,
    /// Watch these file extensions
    pub watch_extensions: Vec<String>,
    /// Watch these directories
    pub watch_directories: Vec<PathBuf>,
}

impl Default for HotReloadConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            poll_interval_ms: 1000,
            watch_extensions: vec![
                ".toml".to_string(),
                ".json".to_string(),
                ".vert".to_string(),
                ".frag".to_string(),
                ".glsl".to_string(),
            ],
            watch_directories: vec![
                PathBuf::from("assets"),
                PathBuf::from("config"),
            ],
        }
    }
}

/// File metadata for tracking changes
#[derive(Debug, Clone)]
struct FileMetadata {
    path: PathBuf,
    last_modified: u64,
    size: u64,
}

/// Callback type for reload notifications
pub type ReloadCallback = Box<dyn Fn(&Path) + Send + Sync>;

/// Manages hot reloading of files
pub struct HotReloadManager {
    config: HotReloadConfig,
    /// Tracked files and their metadata
    tracked_files: HashMap<PathBuf, FileMetadata>,
    /// Callbacks for different file types
    callbacks: HashMap<String, Vec<ReloadCallback>>,
    /// Files pending reload
    pending_reloads: Vec<PathBuf>,
    /// Last poll time
    last_poll_time: u64,
}

impl HotReloadManager {
    pub fn new(config: HotReloadConfig) -> Self {
        Self {
            config,
            tracked_files: HashMap::new(),
            callbacks: HashMap::new(),
            pending_reloads: Vec::new(),
            last_poll_time: 0,
        }
    }

    /// Register a callback for a specific file extension
    pub fn register_callback<F>(&mut self, extension: &str, callback: F)
    where
        F: Fn(&Path) + Send + Sync + 'static,
    {
        let ext = extension.to_lowercase();
        self.callbacks
            .entry(ext)
            .or_insert_with(Vec::new)
            .push(Box::new(callback));
        info!("Registered hot reload callback for extension: {}", extension);
    }

    /// Add a file to be watched
    pub fn watch_file(&mut self, path: &Path) -> Result<(), String> {
        if !path.exists() {
            return Err(format!("File does not exist: {:?}", path));
        }

        let metadata = fs::metadata(path).map_err(|e| e.to_string())?;
        let last_modified = metadata
            .modified()
            .map_err(|e| e.to_string())?
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_secs();

        let file_meta = FileMetadata {
            path: path.to_path_buf(),
            last_modified,
            size: metadata.len(),
        };

        self.tracked_files.insert(path.to_path_buf(), file_meta);
        debug!("Watching file: {:?}", path);
        Ok(())
    }

    /// Add a directory to be watched (recursively watches all matching files)
    pub fn watch_directory(&mut self, dir: &Path) -> Result<usize, String> {
        if !dir.is_dir() {
            return Err(format!("Not a directory: {:?}", dir));
        }

        let mut count = 0;
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    // Recursively watch subdirectories
                    count += self.watch_directory(&path).unwrap_or(0);
                } else if self.should_watch(&path) {
                    if self.watch_file(&path).is_ok() {
                        count += 1;
                    }
                }
            }
        }

        info!("Watched {} files in directory: {:?}", count, dir);
        Ok(count)
    }

    /// Check if a file should be watched based on extension
    fn should_watch(&self, path: &Path) -> bool {
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            let ext = format!(".{}", ext.to_lowercase());
            self.config.watch_extensions.contains(&ext)
        } else {
            false
        }
    }

    /// Poll for file changes (call this every frame or at poll_interval)
    pub fn poll(&mut self) -> Vec<PathBuf> {
        if !self.config.enabled {
            return Vec::new();
        }

        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_else(|e| {
                tracing::warn!("SystemTime before UNIX_EPOCH in hot_reload poll: {}", e);
                std::time::Duration::ZERO
            })
            .as_secs();

        // Check if enough time has passed since last poll
        if current_time - self.last_poll_time < self.config.poll_interval_ms / 1000 {
            return Vec::new();
        }

        self.last_poll_time = current_time;
        self.pending_reloads.clear();

        // Check all tracked files for changes
        for (path, meta) in &mut self.tracked_files {
            if let Ok(new_metadata) = fs::metadata(path) {
                let new_modified = new_metadata
                    .modified()
                    .map(|t| t.duration_since(UNIX_EPOCH).unwrap_or_else(|e| {
                        tracing::warn!("File modified time before UNIX_EPOCH for {:?}: {}", path, e);
                        std::time::Duration::ZERO
                    }).as_secs())
                    .unwrap_or(0);

                if new_modified > meta.last_modified {
                    info!("File changed: {:?}", path);
                    meta.last_modified = new_modified;
                    meta.size = new_metadata.len();
                    self.pending_reloads.push(path.clone());
                }
            }
        }

        // Also scan watched directories for new files
        // Collect directories first to avoid borrow issues
        let watch_dirs: Vec<_> = self.config.watch_directories.clone();
        
        for dir in &watch_dirs {
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if self.should_watch(&path) && !self.tracked_files.contains_key(&path) {
                        if self.watch_file(&path).is_ok() {
                            info!("New file detected: {:?}", path);
                            self.pending_reloads.push(path);
                        }
                    }
                }
            }
        }

        // Trigger callbacks for changed files
        for path in &self.pending_reloads.clone() {
            self.trigger_callbacks(path);
        }

        self.pending_reloads.clone()
    }

    /// Trigger callbacks for a changed file
    fn trigger_callbacks(&self, path: &Path) {
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            let ext = format!(".{}", ext.to_lowercase());
            if let Some(callbacks) = self.callbacks.get(&ext) {
                for callback in callbacks {
                    callback(path);
                }
            }
        }
    }

    /// Get all pending reloads and clear the list
    pub fn take_pending_reloads(&mut self) -> Vec<PathBuf> {
        std::mem::take(&mut self.pending_reloads)
    }

    /// Check if a specific file has pending reload
    pub fn is_pending_reload(&self, path: &Path) -> bool {
        self.pending_reloads.contains(&path.to_path_buf())
    }

    /// Remove a file from watching
    pub fn unwatch_file(&mut self, path: &Path) -> bool {
        self.tracked_files.remove(path).is_some()
    }

    /// Clear all watched files
    pub fn clear(&mut self) {
        self.tracked_files.clear();
        self.pending_reloads.clear();
    }

    /// Enable/disable hot reload
    pub fn set_enabled(&mut self, enabled: bool) {
        self.config.enabled = enabled;
        if !enabled {
            info!("Hot reload disabled");
        } else {
            info!("Hot reload enabled");
        }
    }

    /// Check if hot reload is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Get statistics about watched files
    pub fn get_stats(&self) -> HotReloadStats {
        HotReloadStats {
            total_watched: self.tracked_files.len(),
            pending_reloads: self.pending_reloads.len(),
            callbacks_registered: self.callbacks.values().map(|v| v.len()).sum(),
        }
    }
}

/// Statistics about hot reload system
#[derive(Debug, Clone)]
pub struct HotReloadStats {
    pub total_watched: usize,
    pub pending_reloads: usize,
    pub callbacks_registered: usize,
}

impl Default for HotReloadManager {
    fn default() -> Self {
        Self::new(HotReloadConfig::default())
    }
}

/// Helper function to reload shader files
pub fn create_shader_reload_callback<F>(reload_fn: F) -> ReloadCallback
where
    F: Fn(&Path) + Send + Sync + 'static,
{
    Box::new(reload_fn)
}

/// Helper function to reload config files
pub fn create_config_reload_callback<F>(reload_fn: F) -> ReloadCallback
where
    F: Fn(&Path) + Send + Sync + 'static,
{
    Box::new(reload_fn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_hot_reload_manager_creation() {
        let manager = HotReloadManager::new(HotReloadConfig::default());
        assert!(manager.is_enabled());
        assert_eq!(manager.get_stats().total_watched, 0);
    }

    #[test]
    fn test_watch_file() {
        let mut manager = HotReloadManager::new(HotReloadConfig::default());
        
        // Create a temp file
        let mut temp_file = NamedTempFile::new().ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "Operation failed"))?;
        writeln!(temp_file, "test content").ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "Operation failed"))?;
        
        let result = manager.watch_file(temp_file.path());
        assert!(result.is_ok());
        assert_eq!(manager.get_stats().total_watched, 1);
    }

    #[test]
    fn test_callback_registration() {
        let mut manager = HotReloadManager::new(HotReloadConfig::default());
        
        let called = std::sync::Arc::new(std::sync::Mutex::new(false));
        let called_clone = called.clone();
        
        manager.register_callback(".toml", move |_path| {
            *called_clone.lock().ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "Operation failed"))? = true;
        });
        
        assert_eq!(manager.get_stats().callbacks_registered, 1);
    }
}
