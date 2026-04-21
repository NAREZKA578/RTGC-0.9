//! Async Asset Preloader - Uses rayon job system for parallel asset loading

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::RwLock;
use rayon::prelude::*;
use tracing::{info, debug, warn};

/// Asset load job
#[derive(Debug, Clone)]
pub struct LoadJob {
    pub path: PathBuf,
    pub priority: u32,
    pub group: String,
}

impl LoadJob {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            priority: 0,
            group: "default".to_string(),
        }
    }

    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_group(mut self, group: &str) -> Self {
        self.group = group.to_string();
        self
    }
}

/// Job result
#[derive(Debug)]
pub enum LoadResult<T> {
    Success(T),
    Failed(String),
}

/// Preload queue manager
pub struct AssetPreloader<T> {
    pending_jobs: RwLock<VecDeque<LoadJob>>,
    processing_jobs: RwLock<Vec<LoadJob>>,
    completed_count: Arc<RwLock<usize>>,
    failed_count: Arc<RwLock<usize>>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Send + Sync + 'static> AssetPreloader<T> {
    pub fn new() -> Self {
        Self {
            pending_jobs: RwLock::new(VecDeque::with_capacity(256)),
            processing_jobs: RwLock::new(Vec::new()),
            completed_count: Arc::new(RwLock::new(0)),
            failed_count: Arc::new(RwLock::new(0)),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Queue a single asset for preloading
    pub fn queue(&self, job: LoadJob) {
        self.pending_jobs.write().push_back(job);
    }

    /// Queue multiple assets for preloading
    pub fn queue_batch(&self, jobs: Vec<LoadJob>) {
        let mut pending = self.pending_jobs.write();
        for job in jobs {
            pending.push_back(job);
        }
    }

    /// Process queued jobs using rayon parallelism
    /// Returns completed job paths
    pub fn process<F>(&self, load_fn: F, max_concurrent: usize) -> Vec<PathBuf>
    where
        F: Fn(PathBuf) -> LoadResult<T> + Send + Sync + 'static,
    {
        let mut pending = self.pending_jobs.write();
        
        if pending.is_empty() {
            return Vec::new();
        }

        // Take up to max_concurrent jobs
        let batch_size = max_concurrent.min(pending.len());
        let mut batch: Vec<LoadJob> = Vec::with_capacity(batch_size);
        
        for _ in 0..batch_size {
            if let Some(job) = pending.pop_front() {
                batch.push(job);
            }
        }

        drop(pending);

        if batch.is_empty() {
            return Vec::new();
        }

        // Move to processing
        {
            let mut processing = self.processing_jobs.write();
            processing.extend(batch.iter().cloned());
        }

        let load_fn = Arc::new(load_fn);
        let completed_count = self.completed_count.clone();
        let failed_count = self.failed_count.clone();

        // Process in parallel using rayon
        let results: Vec<_> = batch
            .into_par_iter()
            .map({
                let load_fn = load_fn.clone();
                move |job| {
                    let path = job.path.clone();
                    match load_fn(path.clone()) {
                        LoadResult::Success(_) => {
                            *completed_count.write() += 1;
                            tracing::debug!("Loaded: {:?}", path);
                            Some(path)
                        }
                        LoadResult::Failed(err) => {
                            *failed_count.write() += 1;
                            tracing::warn!("Failed to load {:?}: {}", path, err);
                            None
                        }
                    }
                }
            })
            .collect();

        // Remove from processing
        {
            let mut processing = self.processing_jobs.write();
            processing.retain(|job| {
                !results.iter().any(|p| p.as_ref() == Some(&job.path))
            });
        }

        results.into_iter().flatten().collect()
    }

    /// Get number of pending jobs
    pub fn pending_count(&self) -> usize {
        self.pending_jobs.read().len()
    }

    /// Get number of processing jobs
    pub fn processing_count(&self) -> usize {
        self.processing_jobs.read().len()
    }

    /// Get total completed count
    pub fn completed_count(&self) -> usize {
        *self.completed_count.read()
    }

    /// Get total failed count
    pub fn failed_count(&self) -> usize {
        *self.failed_count.read()
    }

    /// Clear all pending jobs
    pub fn clear_pending(&self) {
        self.pending_jobs.write().clear();
    }

    /// Reset counters
    pub fn reset_counters(&self) {
        *self.completed_count.write() = 0;
        *self.failed_count.write() = 0;
    }

    /// Check if all jobs are complete
    pub fn is_idle(&self) -> bool {
        self.pending_jobs.read().is_empty() 
            && self.processing_jobs.read().is_empty()
    }

    /// Get progress (0.0 to 1.0)
    pub fn progress(&self) -> f32 {
        let completed = *self.completed_count.read();
        let failed = *self.failed_count.read();
        let pending = self.pending_jobs.read().len();
        let processing = self.processing_jobs.read().len();

        let total = completed + failed + pending + processing;
        if total == 0 {
            return 1.0;
        }

        (completed + failed) as f32 / total as f32
    }
}

impl<T: Send + Sync + 'static> Default for AssetPreloader<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Bulk preload helper for level/scene assets
pub fn preload_scene_assets<T, F>(
    preloader: &AssetPreloader<T>,
    asset_paths: Vec<PathBuf>,
    load_fn: F,
) where
    T: Send + Sync + 'static,
    F: Fn(PathBuf) -> LoadResult<T> + Send + Sync + 'static,
{
    let jobs: Vec<LoadJob> = asset_paths
        .into_iter()
        .map(LoadJob::new)
        .collect();

    preloader.queue_batch(jobs);

    // Process with rayon thread pool size
    let max_concurrent = rayon::current_num_threads();
    preloader.process(load_fn, max_concurrent);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preloader_basic() {
        let preloader: AssetPreloader<String> = AssetPreloader::new();
        
        preloader.queue(LoadJob::new(PathBuf::from("test.txt")));
        preloader.queue(LoadJob::new(PathBuf::from("test2.txt")).with_priority(10));
        
        assert_eq!(preloader.pending_count(), 2);
        
        let success_fn = |path: PathBuf| -> LoadResult<String> {
            LoadResult::Success(format!("{:?}", path))
        };
        
        let completed = preloader.process(success_fn, 2);
        assert_eq!(completed.len(), 2);
        assert_eq!(preloader.completed_count(), 2);
        assert!(preloader.is_idle());
    }

    #[test]
    fn test_preloader_with_failures() {
        let preloader: AssetPreloader<String> = AssetPreloader::new();
        
        preloader.queue(LoadJob::new(PathBuf::from("good.txt")));
        preloader.queue(LoadJob::new(PathBuf::from("bad.txt")));
        
        let mixed_fn = |path: PathBuf| -> LoadResult<String> {
            if path.to_string_lossy().contains("good") {
                LoadResult::Success("ok".to_string())
            } else {
                LoadResult::Failed("not found".to_string())
            }
        };
        
        let completed = preloader.process(mixed_fn, 2);
        assert_eq!(completed.len(), 1);
        assert_eq!(preloader.completed_count(), 1);
        assert_eq!(preloader.failed_count(), 1);
    }
}
