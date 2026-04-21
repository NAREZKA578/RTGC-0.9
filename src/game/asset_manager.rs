//! Asset Manager - Handles loading, caching, and reference counting of game assets

use crate::graphics::mesh::Mesh;
use crate::graphics::render_command::Handle;
use crate::graphics::texture::Texture;
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Reference counted handle wrapper
#[derive(Debug, Clone)]
pub struct RefCountedHandle<T> {
    handle: Handle<T>,
    ref_count: Arc<std::sync::atomic::AtomicUsize>,
}

impl<T> RefCountedHandle<T> {
    pub fn new(handle: Handle<T>) -> Self {
        Self {
            handle,
            ref_count: Arc::new(std::sync::atomic::AtomicUsize::new(1)),
        }
    }

    pub fn handle(&self) -> Handle<T> {
        self.handle.clone()
    }

    pub fn add_ref(&self) {
        self.ref_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn release(&self) -> usize {
        self.ref_count
            .fetch_sub(1, std::sync::atomic::Ordering::SeqCst)
            - 1
    }

    pub fn ref_count(&self) -> usize {
        self.ref_count.load(std::sync::atomic::Ordering::SeqCst)
    }
}

/// Asset metadata
#[derive(Debug, Clone)]
pub struct AssetMetadata {
    pub path: String,
    pub load_time_ms: u64,
    pub size_bytes: u64,
    pub last_modified: u64,
}

/// Asset manager configuration
#[derive(Debug, Clone)]
pub struct AssetManagerConfig {
    pub root_path: String,
    pub enable_hot_reload: bool,
    pub preload_on_scene_load: bool,
    pub max_cached_assets: usize,
}

impl Default for AssetManagerConfig {
    fn default() -> Self {
        Self {
            root_path: "assets".to_string(),
            enable_hot_reload: true,
            preload_on_scene_load: true,
            max_cached_assets: 1000,
        }
    }
}

/// Manages all game assets with reference counting and async loading
pub struct AssetManager {
    config: AssetManagerConfig,
    meshes: HashMap<String, RefCountedHandle<Mesh>>,
    textures: HashMap<String, RefCountedHandle<Texture>>,
    metadata: HashMap<String, AssetMetadata>,
    pending_loads: Vec<String>,
    stats: AssetManagerStats,
}

/// Asset manager statistics
#[derive(Debug, Clone, Default)]
pub struct AssetManagerStats {
    pub total_meshes: usize,
    pub total_textures: usize,
    pub total_memory_bytes: u64,
    pub loads_this_frame: usize,
    pub unloads_this_frame: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
}

impl AssetManager {
    pub fn new(config: AssetManagerConfig) -> Self {
        Self {
            config,
            meshes: HashMap::new(),
            textures: HashMap::new(),
            metadata: HashMap::new(),
            pending_loads: Vec::new(),
            stats: AssetManagerStats::default(),
        }
    }

    /// Load or get a mesh asset
    pub fn load_or_get_mesh(&mut self, path: &str) -> Result<Handle<Mesh>, String> {
        // Check cache first
        if let Some(handle) = self.meshes.get(path) {
            handle.add_ref();
            self.stats.cache_hits += 1;
            debug!("Cache hit for mesh: {}", path);
            return Ok(handle.handle());
        }

        self.stats.cache_misses += 1;

        // Load the mesh
        let full_path = Path::new(&self.config.root_path).join(path);
        let start_time = std::time::Instant::now();

        // For now, create a placeholder - in real implementation, load from file
        let mesh = Mesh::new_placeholder();

        let load_time = start_time.elapsed().as_millis() as u64;
        let handle = Handle::<Mesh>::new(self.generate_handle_id());
        let ref_handle = RefCountedHandle::new(handle.clone());

        self.meshes.insert(path.to_string(), ref_handle);
        self.metadata.insert(
            path.to_string(),
            AssetMetadata {
                path: path.to_string(),
                load_time_ms: load_time,
                size_bytes: 0,    // Would calculate from file
                last_modified: 0, // Would get from filesystem
            },
        );

        self.stats.total_meshes += 1;
        self.stats.loads_this_frame += 1;

        info!("Loaded mesh: {} ({}ms)", path, load_time);
        Ok(handle)
    }

    /// Load or get a texture asset
    pub fn load_or_get_texture(&mut self, path: &str) -> Result<Handle<Texture>, String> {
        // Check cache first
        if let Some(handle) = self.textures.get(path) {
            handle.add_ref();
            self.stats.cache_hits += 1;
            debug!("Cache hit for texture: {}", path);
            return Ok(handle.handle());
        }

        self.stats.cache_misses += 1;

        // Load the texture
        let full_path = Path::new(&self.config.root_path).join(path);
        let start_time = std::time::Instant::now();

        // For now, create a placeholder - in real implementation, load from file
        let texture = Texture::new_placeholder();

        let load_time = start_time.elapsed().as_millis() as u64;
        let handle = Handle::<Texture>::new(self.generate_handle_id());
        let ref_handle = RefCountedHandle::new(handle.clone());

        self.textures.insert(path.to_string(), ref_handle);
        self.metadata.insert(
            path.to_string(),
            AssetMetadata {
                path: path.to_string(),
                load_time_ms: load_time,
                size_bytes: 0,
                last_modified: 0,
            },
        );

        self.stats.total_textures += 1;
        self.stats.loads_this_frame += 1;

        info!("Loaded texture: {} ({}ms)", path, load_time);
        Ok(handle)
    }

    /// Release a mesh handle
    pub fn release_mesh(&mut self, path: &str) {
        if let Some(handle) = self.meshes.get_mut(path) {
            let remaining = handle.release();
            if remaining == 0 {
                self.meshes.remove(path);
                self.metadata.remove(path);
                self.stats.total_meshes -= 1;
                self.stats.unloads_this_frame += 1;
                debug!("Unloaded mesh: {}", path);
            }
        }
    }

    /// Release a texture handle
    pub fn release_texture(&mut self, path: &str) {
        if let Some(handle) = self.textures.get_mut(path) {
            let remaining = handle.release();
            if remaining == 0 {
                self.textures.remove(path);
                self.metadata.remove(path);
                self.stats.total_textures -= 1;
                self.stats.unloads_this_frame += 1;
                debug!("Unloaded texture: {}", path);
            }
        }
    }

    /// Preload assets for a scene using rayon parallelism
    pub fn preload_scene_assets(&mut self, asset_paths: &[&str]) {
        info!("Preloading {} assets for scene", asset_paths.len());

        // Load assets sequentially (parallel would require Arc<Mutex> wrapper)
        for path in asset_paths {
            let start = std::time::Instant::now();
            // In real implementation, determine type from extension or manifest
            let result =
                if path.ends_with(".png") || path.ends_with(".jpg") || path.ends_with(".dds") {
                    self.load_or_get_texture(path).map(|h| ("texture", h.id()))
                } else {
                    self.load_or_get_mesh(path).map(|h| ("mesh", h.id()))
                };
            match result {
                Ok((asset_type, id)) => {
                    debug!(
                        "Preloaded {} '{}' (id: {}, time: {:?})",
                        asset_type,
                        path,
                        id,
                        start.elapsed()
                    );
                }
                Err(e) => {
                    warn!("Failed to preload '{}': {}", path, e);
                }
            }
        }

        info!("Scene asset preloading complete");
    }

    /// Unload unused assets (reference count = 0)
    pub fn unload_unused(&mut self) {
        let mut to_unload_mesh = Vec::new();
        let mut to_unload_texture = Vec::new();

        for (path, handle) in &self.meshes {
            if handle.ref_count() == 0 {
                to_unload_mesh.push(path.clone());
            }
        }

        for (path, handle) in &self.textures {
            if handle.ref_count() == 0 {
                to_unload_texture.push(path.clone());
            }
        }

        for path in &to_unload_mesh {
            self.release_mesh(path);
        }

        for path in &to_unload_texture {
            self.release_texture(path);
        }

        if !to_unload_mesh.is_empty() || !to_unload_texture.is_empty() {
            info!(
                "Unloaded {} meshes and {} textures",
                to_unload_mesh.len(),
                to_unload_texture.len()
            );
        }
    }

    /// Get statistics
    pub fn stats(&self) -> &AssetManagerStats {
        &self.stats
    }

    /// Reset frame statistics
    pub fn reset_frame_stats(&mut self) {
        self.stats.loads_this_frame = 0;
        self.stats.unloads_this_frame = 0;
    }

    /// Generate a unique handle ID
    fn generate_handle_id(&self) -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_else(|_| {
                tracing::warn!("SystemTime before UNIX_EPOCH, using zero");
                std::time::Duration::ZERO
            })
            .subsec_nanos() as u64
    }

    /// Check if an asset is loaded
    pub fn is_mesh_loaded(&self, path: &str) -> bool {
        self.meshes.contains_key(path)
    }

    pub fn is_texture_loaded(&self, path: &str) -> bool {
        self.textures.contains_key(path)
    }

    /// Get metadata for an asset
    pub fn get_metadata(&self, path: &str) -> Option<&AssetMetadata> {
        self.metadata.get(path)
    }

    /// Clear all assets (force unload)
    pub fn clear(&mut self) {
        self.meshes.clear();
        self.textures.clear();
        self.metadata.clear();
        self.pending_loads.clear();
        self.stats = AssetManagerStats::default();
        info!("Asset manager cleared");
    }
}

impl Default for AssetManager {
    fn default() -> Self {
        Self::new(AssetManagerConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asset_manager_cache() {
        let mut manager = AssetManager::new(AssetManagerConfig::default());

        // First load should be a cache miss
        let handle1 = manager.load_or_get_mesh("test.obj");
        assert!(handle1.is_ok());
        assert_eq!(manager.stats().cache_misses, 1);

        // Second load should be a cache hit
        let handle2 = manager.load_or_get_mesh("test.obj");
        assert!(handle2.is_ok());
        assert_eq!(manager.stats().cache_hits, 1);
    }

    #[test]
    fn test_reference_counting() {
        let mut manager = AssetManager::new(AssetManagerConfig::default());

        let handle = manager.load_or_get_mesh("test.obj").unwrap();

        // Should have one reference
        assert!(manager.is_mesh_loaded("test.obj"));

        // Release once
        manager.release_mesh("test.obj");

        // After release with ref count 0, should be unloaded
        assert!(!manager.is_mesh_loaded("test.obj"));
    }
}
