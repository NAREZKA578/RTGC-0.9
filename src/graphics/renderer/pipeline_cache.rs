//! Pipeline Cache - кэш PSO для избежания повторного создания

use crate::graphics::rhi::{ResourceHandle, PipelineStateObject};
use std::collections::HashMap;

/// Ключ для кэширования пайплайнов
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PipelineKey {
    pub vertex_shader: u64,
    pub fragment_shader: Option<u64>,
    pub input_layout_hash: u64,
    pub blend_state_hash: u64,
    pub depth_state_hash: u64,
    pub rasterizer_state_hash: u64,
    pub primitive_topology: u32,
}

impl PipelineKey {
    pub fn from_pso(pso: &PipelineStateObject) -> Self {
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;
        
        let mut hasher = DefaultHasher::new();
        pso.input_layout.hash(&mut hasher);
        let input_layout_hash = hasher.finish();
        
        let mut hasher = DefaultHasher::new();
        for blend in &pso.color_blend_states {
            blend.hash(&mut hasher);
        }
        let blend_state_hash = hasher.finish();
        
        let mut hasher = DefaultHasher::new();
        pso.depth_state.hash(&mut hasher);
        let depth_state_hash = hasher.finish();
        
        let mut hasher = DefaultHasher::new();
        pso.rasterizer_state.hash(&mut hasher);
        let rasterizer_state_hash = hasher.finish();
        
        Self {
            vertex_shader: pso.vertex_shader.0,
            fragment_shader: pso.fragment_shader.map(|h| h.0),
            input_layout_hash,
            blend_state_hash,
            depth_state_hash,
            rasterizer_state_hash,
            primitive_topology: pso.primitive_topology as u32,
        }
    }
}

/// Кэш Pipeline State Objects
pub struct PipelineCache {
    cache: HashMap<PipelineKey, ResourceHandle>,
    hit_count: usize,
    miss_count: usize,
}

impl PipelineCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            hit_count: 0,
            miss_count: 0,
        }
    }
    
    /// Получает пайплайн из кэша или создаёт новый
    pub fn get_or_insert<F>(&mut self, key: PipelineKey, create_fn: F) -> ResourceHandle
    where
        F: FnOnce() -> ResourceHandle,
    {
        match self.cache.get(&key) {
            Some(handle) => {
                self.hit_count += 1;
                *handle
            }
            None => {
                self.miss_count += 1;
                let pipeline = create_fn();
                self.cache.insert(key, pipeline);
                pipeline
            }
        }
    }
    
    /// Проверяет наличие пайплайна в кэше
    pub fn contains(&self, key: &PipelineKey) -> bool {
        self.cache.contains_key(key)
    }
    
    /// Статистика кэша
    pub fn stats(&self) -> PipelineCacheStats {
        PipelineCacheStats {
            hit_count: self.hit_count,
            miss_count: self.miss_count,
            cached_pipelines: self.cache.len(),
        }
    }
    
    /// Очищает кэш
    pub fn clear(&mut self) {
        self.cache.clear();
        self.hit_count = 0;
        self.miss_count = 0;
    }
}

impl Default for PipelineCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Статистика кэша пайплайнов
#[derive(Debug, Clone, Default)]
pub struct PipelineCacheStats {
    pub hit_count: usize,
    pub miss_count: usize,
    pub cached_pipelines: usize,
}

impl PipelineCacheStats {
    pub fn hit_rate(&self) -> f32 {
        let total = self.hit_count + self.miss_count;
        if total == 0 {
            0.0
        } else {
            self.hit_count as f32 / total as f32
        }
    }
}
