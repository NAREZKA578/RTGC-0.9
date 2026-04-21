//! Asset Cache - High-performance asset caching system

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::RwLock;
use tracing::{info, warn, debug};

/// Handle to a cached asset
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CacheHandle(u64);

impl CacheHandle {
    pub const fn null() -> Self {
        Self(0)
    }
    
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
    
    fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Cached asset entry
#[derive(Debug)]
pub struct CachedAsset<T> {
    pub data: Arc<T>,
    pub path: PathBuf,
    pub load_time: Instant,
    pub last_accessed: Instant,
    pub access_count: u32,
    pub size_bytes: u64,
}

impl<T> CachedAsset<T> {
    pub fn new(data: T, path: PathBuf, size_bytes: u64) -> Self {
        let now = Instant::now();
        Self {
            data: Arc::new(data),
            path,
            load_time: now,
            last_accessed: now,
            access_count: 1,
            size_bytes,
        }
    }
    
    pub fn mark_accessed(&mut self) {
        self.access_count += 1;
        self.last_accessed = Instant::now();
    }
}

/// Asset cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub max_memory_mb: usize,
    pub min_ttl_seconds: u64,
    pub check_interval_seconds: u64,
    pub auto_unload_unused: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_memory_mb: 512,
            min_ttl_seconds: 60,
            check_interval_seconds: 10,
            auto_unload_unused: true,
        }
    }
}

/// Thread-safe asset cache
pub struct AssetCache<T> {
    config: CacheConfig,
    cache: RwLock<HashMap<String, CachedAsset<T>>>,
    path_to_key: RwLock<HashMap<PathBuf, String>>,
    next_id: RwLock<u64>,
    total_memory: RwLock<u64>,
}

impl<T: Clone> AssetCache<T> {
    pub fn new(config: CacheConfig) -> Self {
        Self {
            config,
            cache: RwLock::new(HashMap::with_capacity(256)),
            path_to_key: RwLock::new(HashMap::with_capacity(256)),
            next_id: RwLock::new(1),
            total_memory: RwLock::new(0),
        }
    }
    
    /// Get an asset from cache by key
    pub fn get(&self, key: &str) -> Option<Arc<T>> {
        let mut cache = self.cache.write();
        if let Some(asset) = cache.get_mut(key) {
            asset.mark_accessed();
            return Some(asset.data.clone());
        }
        None
    }
    
    /// Get an asset from cache by path
    pub fn get_by_path(&self, path: &Path) -> Option<Arc<T>> {
        let key = {
            let path_to_key = self.path_to_key.read();
            path_to_key.get(path).cloned()
        };
        if let Some(key) = key {
            return self.get(&key);
        }
        None
    }
    
    /// Insert an asset into cache
    pub fn insert(&self, key: String, data: T, path: PathBuf, size_bytes: u64) -> CacheHandle {
        let asset = CachedAsset::new(data, path.clone(), size_bytes);
        
        let handle = CacheHandle::new(*self.next_id.read());
        *self.next_id.write() += 1;
        
        let mut cache = self.cache.write();
        cache.insert(key.clone(), asset);
        
        let mut path_to_key = self.path_to_key.write();
        path_to_key.insert(path, key);
        
        *self.total_memory.write() += size_bytes;
        
        // Check memory limit
        self.enforce_memory_limit();
        
        handle
    }
    
    /// Remove an asset from cache
    pub fn remove(&self, key: &str) -> bool {
        let asset = {
            let mut cache = self.cache.write();
            cache.remove(key)
        };
        
        if let Some(asset) = asset {
            *self.total_memory.write() -= asset.size_bytes;

            let mut path_to_key = self.path_to_key.write();
            path_to_key.remove(&asset.path);

            return true;
        }
        false
    }
    
    /// Check if an asset is in cache
    pub fn contains(&self, key: &str) -> bool {
        self.cache.read().contains_key(key)
    }
    
    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let cache = self.cache.read();
        let total_memory = *self.total_memory.read();
        
        CacheStats {
            count: cache.len(),
            total_memory_bytes: total_memory,
            max_memory_bytes: self.config.max_memory_mb as u64 * 1024 * 1024,
        }
    }
    
    /// Clear unused assets (not accessed recently)
    pub fn clear_unused(&self) -> usize {
        let now = Instant::now();
        let ttl = Duration::from_secs(self.config.min_ttl_seconds);
        let mut removed = 0;
        
        let mut cache = self.cache.write();
        let mut to_remove = Vec::new();

        for (key, asset) in cache.iter() {
            if now.duration_since(asset.last_accessed) > ttl && asset.access_count == 1 {
                to_remove.push((key.clone(), asset.size_bytes));
            }
        }

        for (key, size) in to_remove {
            let asset = cache.remove(&key);
            if let Some(asset) = asset {
                *self.total_memory.write() -= size;

                let mut path_to_key = self.path_to_key.write();
                path_to_key.remove(&asset.path);

                removed += 1;
                debug!("Unloaded unused asset: {}", key);
            }
        }
        
        removed
    }
    
    /// Enforce memory limit by removing least recently used assets
    fn enforce_memory_limit(&self) {
        let max_memory = self.config.max_memory_mb as u64 * 1024 * 1024;
        let mut current_memory = *self.total_memory.read();
        
        if current_memory <= max_memory {
            return;
        }

        let mut cache = self.cache.write();

        // Collect keys to remove first to avoid borrow conflicts
        let mut to_remove = Vec::new();
        for (key, asset) in cache.iter() {
            if asset.access_count == 1 {
                to_remove.push((key.clone(), asset.size_bytes));
            }
        }
        drop(cache);

        // Now remove the collected keys
        let mut cache = self.cache.write();
        for (key, size) in to_remove {
            if current_memory <= max_memory {
                break;
            }

            let asset = cache.remove(&key);
            if let Some(asset) = asset {
                current_memory -= size;

                let mut path_to_key = self.path_to_key.write();
                path_to_key.remove(&asset.path);

                debug!("Unloaded asset due to memory pressure: {}", key);
            }
        }
        
        *self.total_memory.write() = current_memory;
    }
    
    /// Clear all assets
    pub fn clear(&self) {
        self.cache.write().clear();
        self.path_to_key.write().clear();
        *self.total_memory.write() = 0;
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub count: usize,
    pub total_memory_bytes: u64,
    pub max_memory_bytes: u64,
}

impl CacheStats {
    pub fn memory_usage_mb(&self) -> f64 {
        self.total_memory_bytes as f64 / (1024.0 * 1024.0)
    }
    
    pub fn max_memory_mb(&self) -> f64 {
        self.max_memory_bytes as f64 / (1024.0 * 1024.0)
    }
    
    pub fn usage_percent(&self) -> f64 {
        if self.max_memory_bytes == 0 {
            return 0.0;
        }
        (self.total_memory_bytes as f64 / self.max_memory_bytes as f64) * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cache_basic() {
        let cache: AssetCache<String> = AssetCache::new(CacheConfig::default());
        
        let handle = cache.insert(
            "test".to_string(),
            "data".to_string(),
            PathBuf::from("test.txt"),
            100,
        );
        
        assert!(!handle.is_null());
        assert!(cache.contains("test"));
        
        let data = cache.get("test");
        assert!(data.is_some());
        assert_eq!(*data.ok_or("Test assertion failed")?, "data");
    }
    
    #[test]
    fn test_cache_stats() {
        let cache: AssetCache<String> = AssetCache::new(CacheConfig::default());
        
        cache.insert("a".to_string(), "data".to_string(), PathBuf::from("a.txt"), 100);
        cache.insert("b".to_string(), "data".to_string(), PathBuf::from("b.txt"), 200);
        
        let stats = cache.stats();
        assert_eq!(stats.count, 2);
        assert_eq!(stats.total_memory_bytes, 300);
    }
}
