//! Assets module for RTGC-0.8

pub mod loader;
pub mod asset_loader;
pub mod vehicle_loader;
pub mod cache;
pub mod hot_reload;
pub mod preloader;

pub use loader::{AssetLoader, AssetHandle, AssetData, AssetType, AssetMetadata, LoaderConfig, AssetLoadError};
pub use asset_loader::{Asset, AssetManager, VehicleAsset, VehiclePreset, GameObjectAsset};
pub use vehicle_loader::{VehicleLoader, VehicleDefinition, VehicleMetadata, VehicleLoadError};
pub use cache::{AssetCache, CacheConfig, CacheHandle, CacheStats, CachedAsset};
pub use hot_reload::{HotReloadManager, HotReloadConfig, FileMetadata, WatchedAsset};
pub use preloader::{AssetPreloader, LoadJob, LoadResult, preload_scene_assets};
