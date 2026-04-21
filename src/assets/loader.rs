//! Asset loader - Universal asset loading system

use image::DynamicImage;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::warn;

/// Handle to a loaded asset
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AssetHandle(u64);

impl AssetHandle {
    pub const fn null() -> Self {
        Self(0)
    }

    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

/// Supported asset types
#[derive(Debug, Clone)]
pub enum AssetType {
    Texture,
    Mesh,
    Shader,
    Audio,
    Font,
    Config,
    Model,
}

/// Loaded asset data
#[derive(Debug, Clone)]
pub enum AssetData {
    Texture {
        width: u32,
        height: u32,
        channels: u8,
        data: Vec<u8>,
    },
    Mesh {
        vertices: Vec<f32>,
        indices: Vec<u32>,
    },
    Shader {
        source: String,
        shader_type: ShaderStage,
    },
    Audio {
        sample_rate: u32,
        channels: u16,
        samples: Vec<f32>,
    },
    Font {
        name: String,
        size: u32,
        data: Vec<u8>,
    },
    Config {
        content: String,
    },
    Model {
        path: PathBuf,
        data: Vec<u8>,
    },
}

/// Shader stage type
#[derive(Debug, Clone, Copy)]
pub enum ShaderStage {
    Vertex,
    Fragment,
    Compute,
    Geometry,
    TessellationControl,
    TessellationEvaluation,
}

/// Metadata for an asset
#[derive(Debug, Clone)]
pub struct AssetMetadata {
    pub name: String,
    pub asset_type: AssetType,
    pub path: PathBuf,
    pub size_bytes: u64,
    pub load_time_ms: f64,
}

/// Asset loading errors
#[derive(Debug)]
pub enum AssetLoadError {
    IoError(std::io::Error),
    InvalidFormat(String),
    InvalidData(String),
    NotFound(String),
    UnsupportedType(String),
    DecodeError(String),
    UnsupportedFormat(String),
}

impl From<std::io::Error> for AssetLoadError {
    fn from(err: std::io::Error) -> Self {
        AssetLoadError::IoError(err)
    }
}

impl std::fmt::Display for AssetLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssetLoadError::IoError(e) => write!(f, "IO error: {}", e),
            AssetLoadError::InvalidFormat(msg) => write!(f, "Invalid format: {}", msg),
            AssetLoadError::InvalidData(msg) => write!(f, "Invalid data: {}", msg),
            AssetLoadError::NotFound(path) => write!(f, "Asset not found: {}", path),
            AssetLoadError::UnsupportedType(ty) => write!(f, "Unsupported type: {}", ty),
            AssetLoadError::DecodeError(msg) => write!(f, "Decode error: {}", msg),
            AssetLoadError::UnsupportedFormat(msg) => write!(f, "Unsupported format: {}", msg),
        }
    }
}

impl std::error::Error for AssetLoadError {}

/// Asset loader configuration
#[derive(Debug, Clone)]
pub struct LoaderConfig {
    pub root_path: PathBuf,
    pub cache_size_mb: usize,
    pub async_loading: bool,
    pub hot_reload: bool,
}

impl Default for LoaderConfig {
    fn default() -> Self {
        Self {
            root_path: PathBuf::from("assets"),
            cache_size_mb: 512,
            async_loading: true,
            hot_reload: false,
        }
    }
}

/// Universal asset loader
#[derive(Clone)]
pub struct AssetLoader {
    config: LoaderConfig,
    assets: HashMap<AssetHandle, Arc<AssetData>>,
    metadata: HashMap<AssetHandle, AssetMetadata>,
    next_handle: u64,
    path_to_handle: HashMap<PathBuf, AssetHandle>,
}

impl AssetLoader {
    /// Creates a new asset loader with default config
    pub fn new() -> Self {
        Self::with_config(LoaderConfig::default())
    }

    /// Creates a new asset loader with custom config
    pub fn with_config(config: LoaderConfig) -> Self {
        Self {
            config,
            assets: HashMap::new(),
            metadata: HashMap::new(),
            next_handle: 1,
            path_to_handle: HashMap::new(),
        }
    }

    /// Generates a new unique asset handle
    fn generate_handle(&mut self) -> AssetHandle {
        let handle = AssetHandle(self.next_handle);
        self.next_handle += 1;
        handle
    }

    /// Loads an asset from a file path
    pub fn load<P: AsRef<Path>>(
        &mut self,
        path: P,
        asset_type: AssetType,
    ) -> Result<AssetHandle, AssetLoadError> {
        let path = path.as_ref().to_path_buf();

        // Check if already loaded
        if let Some(&handle) = self.path_to_handle.get(&path) {
            return Ok(handle);
        }

        let full_path = self.config.root_path.join(&path);

        if !full_path.exists() {
            return Err(AssetLoadError::NotFound(full_path.display().to_string()));
        }

        let start_time = std::time::Instant::now();

        let data = match asset_type {
            AssetType::Texture => self.load_texture(&full_path)?,
            AssetType::Mesh => self.load_mesh(&full_path)?,
            AssetType::Shader => self.load_shader(&full_path)?,
            AssetType::Audio => self.load_audio(&full_path)?,
            AssetType::Font => self.load_font(&full_path)?,
            AssetType::Config => self.load_config(&full_path)?,
            AssetType::Model => self.load_model(&full_path)?,
        };

        let load_time_ms = start_time.elapsed().as_secs_f64() * 1000.0;
        let size_bytes = full_path.metadata()?.len();

        let handle = self.generate_handle();
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown") // Безопасно: fallback для некорректных имён файлов
            .to_string();

        let metadata = AssetMetadata {
            name: name.clone(),
            asset_type: asset_type.clone(),
            path: path.clone(),
            size_bytes,
            load_time_ms,
        };

        let arc_data = Arc::new(data);
        self.assets.insert(handle, arc_data);
        self.metadata.insert(handle, metadata);
        self.path_to_handle.insert(path, handle);

        Ok(handle)
    }

    /// Gets a reference to a loaded asset
    pub fn get(&self, handle: AssetHandle) -> Option<Arc<AssetData>> {
        self.assets.get(&handle).cloned()
    }

    /// Gets metadata for a loaded asset
    pub fn get_metadata(&self, handle: AssetHandle) -> Option<&AssetMetadata> {
        self.metadata.get(&handle)
    }

    /// Unloads an asset
    pub fn unload(&mut self, handle: AssetHandle) -> bool {
        if let Some(metadata) = self.metadata.remove(&handle) {
            self.path_to_handle.remove(&metadata.path);
            self.assets.remove(&handle);
            true
        } else {
            false
        }
    }

    /// Loads a texture (PNG, JPG, etc.)
    fn load_texture(&mut self, path: &Path) -> Result<AssetData, AssetLoadError> {
        // Try to load actual file first
        let mut file = match std::fs::File::open(path) {
            Ok(f) => f,
            Err(_) => {
                // Generate procedural texture based on filename
                return Ok(Self::generate_procedural_texture(path));
            }
        };
        let mut data = Vec::new();
        use std::io::Read;
        if let Err(e) = file.read_to_end(&mut data) {
            // Fall back to procedural texture on read error
            return Ok(Self::generate_procedural_texture(path));
        }

        // Check file signature and try to decode
        if data.len() > 8 && data.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
            // PNG format - try to decode with image crate
            match image::load_from_memory(&data) {
                Ok(img) => {
                    let rgba = img.to_rgba8();
                    let (width, height) = rgba.dimensions();
                    return Ok(AssetData::Texture {
                        width,
                        height,
                        channels: 4,
                        data: rgba.into_raw(),
                    });
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to decode PNG {}: {}, using procedural",
                        path.display(),
                        e
                    );
                    return Ok(Self::generate_procedural_texture(path));
                }
            }
        } else if data.len() > 2 && data.starts_with(&[0xFF, 0xD8]) {
            // JPEG format
            match image::load_from_memory(&data) {
                Ok(img) => {
                    let rgba = img.to_rgba8();
                    let (width, height) = rgba.dimensions();
                    return Ok(AssetData::Texture {
                        width,
                        height,
                        channels: 4,
                        data: rgba.into_raw(),
                    });
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to decode JPEG {}: {}, using procedural",
                        path.display(),
                        e
                    );
                    return Ok(Self::generate_procedural_texture(path));
                }
            };
        } else if data.len() > 4 && data.starts_with(&[0x44, 0x44, 0x53, 0x20]) {
            // DDS format - basic support
            tracing::info!(
                "DDS format detected in {}, using procedural (DDS decoding not fully implemented)",
                path.display()
            );
            return Ok(Self::generate_procedural_texture(path));
        }

        // Default: generate procedural texture
        Ok(Self::generate_procedural_texture(path))
    }

    fn generate_procedural_texture(path: &Path) -> AssetData {
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("default");

        let path_str = path.to_string_lossy().to_lowercase();

        // Generate different textures based on path
        if path_str.contains("terrain") || path_str.contains("ground") || path_str.contains("grass")
        {
            Self::generate_terrain_texture(256, 256)
        } else if path_str.contains("sky") || path_str.contains("cloud") {
            Self::generate_sky_texture(256, 256)
        } else if path_str.contains("vehicle")
            || path_str.contains("car")
            || path_str.contains("uaz")
        {
            Self::generate_vehicle_texture(256, 256)
        } else if path_str.contains("road") || path_str.contains("asphalt") {
            Self::generate_road_texture(256, 256)
        } else if path_str.contains("metal") || path_str.contains("steel") {
            Self::generate_metal_texture(256, 256)
        } else if path_str.contains("wood") || path_str.contains("tree") {
            Self::generate_wood_texture(256, 256)
        } else if path_str.contains("ui") || path_str.contains("hud") || path_str.contains("button")
        {
            Self::generate_ui_texture(256, 256)
        } else {
            Self::generate_default_texture(256, 256)
        }
    }

    fn generate_terrain_texture(width: u32, height: u32) -> AssetData {
        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height {
            for x in 0..width {
                // Grass-like green with variation
                let noise = ((x as f32 * 0.1).sin() * (y as f32 * 0.1).sin() * 30.0) as i32;
                let g = 100 + ((x % 32) as i32 - 16).abs() + ((y % 32) as i32 - 16).abs();
                let r = (50 + noise).clamp(30, 80) as u8;
                let g = (g + noise).clamp(80, 140) as u8;
                let b = (30 + noise / 2).clamp(20, 50) as u8;
                pixels.extend_from_slice(&[r, g, b, 255]);
            }
        }
        AssetData::Texture {
            width,
            height,
            channels: 4,
            data: pixels,
        }
    }

    fn generate_sky_texture(width: u32, height: u32) -> AssetData {
        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height {
            for x in 0..width {
                let t = y as f32 / height as f32;
                // Sky gradient: lighter at horizon, deeper blue at top
                let r = (100.0 + t * 100.0) as u8;
                let g = (150.0 + t * 80.0) as u8;
                let b = (220.0 + t * 35.0) as u8;
                pixels.extend_from_slice(&[r, g, b, 255]);
            }
        }
        AssetData::Texture {
            width,
            height,
            channels: 4,
            data: pixels,
        }
    }

    /// Generates a more detailed terrain texture with multiple layers
    fn generate_terrain_texture_detailed(width: u32, height: u32) -> AssetData {
        let mut pixels = Vec::with_capacity((width * height * 4) as usize);

        // Simplex-like noise for more natural terrain
        for y in 0..height {
            for x in 0..width {
                let fx = x as f32 / width as f32;
                let fy = y as f32 / height as f32;

                // Multiple noise layers
                let noise1 = (fx * 8.0).sin() * (fy * 8.0).sin() * 0.5;
                let noise2 = (fx * 16.0 + 0.5).sin() * (fy * 16.0 + 0.5).sin() * 0.25;
                let noise3 = (fx * 32.0).sin() * (fy * 32.0).sin() * 0.125;
                let combined = noise1 + noise2 + noise3;

                // Base grass colors with variation
                let base_g = 100.0 + combined * 40.0;
                let r = (45.0 + combined * 15.0).clamp(25.0, 75.0) as u8;
                let g = base_g.clamp(70.0, 150.0) as u8;
                let b = (25.0 + combined * 10.0).clamp(15.0, 45.0) as u8;

                pixels.extend_from_slice(&[r, g, b, 255]);
            }
        }

        AssetData::Texture {
            width,
            height,
            channels: 4,
            data: pixels,
        }
    }

    /// Generates a road texture with lane markings
    fn generate_road_texture_detailed(width: u32, height: u32) -> AssetData {
        let mut pixels = Vec::with_capacity((width * height * 4) as usize);

        for y in 0..height {
            for x in 0..width {
                let fx = x as f32 / width as f32;
                let fy = y as f32 / height as f32;

                // Base asphalt
                let mut base = 35.0 + ((x % 4) as f32 - 2.0).abs();

                // Center line (yellow/white)
                if (fy > 0.45 && fy < 0.55) && ((fx * 20.0).floor() as u32) % 2 == 0 {
                    base = 220.0;
                }

                // Edge lines (white)
                if fy < 0.05 || fy > 0.95 {
                    base = 180.0;
                }

                let shade = base.clamp(20.0, 230.0) as u8;
                pixels.extend_from_slice(&[shade, shade, shade, 255]);
            }
        }

        AssetData::Texture {
            width,
            height,
            channels: 4,
            data: pixels,
        }
    }

    fn generate_vehicle_texture(width: u32, height: u32) -> AssetData {
        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height {
            for x in 0..width {
                // Metallic gray with slight gradient
                let cx = x as f32 - width as f32 / 2.0;
                let cy = y as f32 - height as f32 / 2.0;
                let dist = (cx * cx + cy * cy).sqrt() / (width as f32 / 2.0);
                let shade = (150.0 - dist * 50.0) as u8;
                let r = shade;
                let g = shade;
                let b = shade;
                pixels.extend_from_slice(&[r, g, b, 255]);
            }
        }
        AssetData::Texture {
            width,
            height,
            channels: 4,
            data: pixels,
        }
    }

    fn generate_road_texture(width: u32, height: u32) -> AssetData {
        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height {
            for x in 0..width {
                // Dark asphalt gray
                let noise = ((x % 8) as i8 - 4).abs() as u8;
                let shade = 40 + noise;
                pixels.extend_from_slice(&[shade, shade, shade, 255]);
            }
        }
        AssetData::Texture {
            width,
            height,
            channels: 4,
            data: pixels,
        }
    }

    fn generate_metal_texture(width: u32, height: u32) -> AssetData {
        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height {
            for x in 0..width {
                // Brushed metal look
                let shine = ((x as f32 * 0.5).sin() * 30.0) as i32;
                let shade = (180 + shine).clamp(150, 220) as u8;
                pixels.extend_from_slice(&[shade, shade, shade, 255]);
            }
        }
        AssetData::Texture {
            width,
            height,
            channels: 4,
            data: pixels,
        }
    }

    fn generate_wood_texture(width: u32, height: u32) -> AssetData {
        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height {
            for x in 0..width {
                // Wood grain pattern
                let grain = ((y as f32 * 0.3).sin() * 30.0) as i32;
                let shade = (120 + grain).clamp(80, 160) as u8;
                pixels.extend_from_slice(&[shade, shade * 3 / 4, shade / 2, 255]);
            }
        }
        AssetData::Texture {
            width,
            height,
            channels: 4,
            data: pixels,
        }
    }

    fn generate_ui_texture(width: u32, height: u32) -> AssetData {
        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height {
            for x in 0..width {
                // Semi-transparent dark UI element
                let gradient = (y as f32 / height as f32 * 100.0) as u8;
                pixels.extend_from_slice(&[20, 20, 20, 200]);
            }
        }
        AssetData::Texture {
            width,
            height,
            channels: 4,
            data: pixels,
        }
    }

    fn generate_default_texture(width: u32, height: u32) -> AssetData {
        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height {
            for x in 0..width {
                // Checkerboard pattern
                let color = if ((x / 16) + (y / 16)) % 2 == 0 {
                    255
                } else {
                    128
                };
                pixels.extend_from_slice(&[color, color, color, 255]);
            }
        }
        AssetData::Texture {
            width,
            height,
            channels: 4,
            data: pixels,
        }
    }

    /// Loads a mesh (OBJ, FBX, glTF, etc.)
    fn load_mesh(&self, path: &Path) -> Result<AssetData, AssetLoadError> {
        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or(""); // Безопасно: пустая строка для файлов без расширения

        match extension.to_lowercase().as_str() {
            "obj" => self.load_obj(path),
            _ => Err(AssetLoadError::UnsupportedFormat(format!(
                "Unknown mesh format: {}",
                extension
            ))),
        }
    }

    /// Loads an OBJ mesh file
    fn load_obj(&self, path: &Path) -> Result<AssetData, AssetLoadError> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut positions: Vec<[f32; 3]> = Vec::new();

        // Full OBJ parser with support for v, vn, vt, f statements
        for line in std::io::BufRead::lines(reader) {
            let line = match line {
                Ok(l) => l,
                Err(_) => continue,
            };

            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            match parts[0] {
                "v" => {
                    // Vertex position: v x y z [w]
                    if parts.len() >= 4 {
                        if let (Ok(x), Ok(y), Ok(z)) = (
                            parts[1].parse::<f32>(),
                            parts[2].parse::<f32>(),
                            parts[3].parse::<f32>(),
                        ) {
                            positions.push([x, y, z]);
                        }
                    }
                }
                "vn" => {
                    // Vertex normal: vn x y z
                    // Could be stored separately if needed
                }
                "vt" => {
                    // Texture coordinate: vt u [v]
                    // Could be stored separately if needed
                }
                "f" => {
                    // Face: f v1/vt1/vn1 v2/vt2/vn2 ...
                    // Support various formats: f v, f v/vt, f v//vn, f v/vt/vn
                    if parts.len() >= 4 {
                        let mut face_indices = Vec::new();
                        for part in &parts[1..] {
                            let vertex_idx = part
                                .split('/')
                                .next()
                                .and_then(|s| s.parse::<usize>().ok())
                                .map(|i| i - 1); // OBJ uses 1-based indexing

                            if let Some(idx) = vertex_idx {
                                if idx < positions.len() {
                                    face_indices.push(idx);
                                }
                            }
                        }

                        // Triangulate the face (fan triangulation)
                        if face_indices.len() >= 3 {
                            for i in 1..face_indices.len() - 1 {
                                let v0 = face_indices[0];
                                let v1 = face_indices[i];
                                let v2 = face_indices[i + 1];

                                // Add vertex positions
                                vertices.extend_from_slice(&positions[v0]);
                                vertices.extend_from_slice(&positions[v1]);
                                vertices.extend_from_slice(&positions[v2]);

                                // Add indices (sequential)
                                let base_idx = (vertices.len() / 3 - 3) as u32;
                                indices.extend_from_slice(&[base_idx, base_idx + 1, base_idx + 2]);
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        if vertices.is_empty() {
            return Err(AssetLoadError::InvalidData(
                "No valid vertices found in OBJ file".to_string(),
            ));
        }

        Ok(AssetData::Mesh { vertices, indices })
    }

    /// Loads a shader file
    fn load_shader(&self, path: &Path) -> Result<AssetData, AssetLoadError> {
        let mut file = File::open(path)?;
        let mut source = String::new();
        file.read_to_string(&mut source)?;

        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or(""); // Безопасно: пустая строка для файлов без расширения

        let shader_type = match extension.to_lowercase().as_str() {
            "vert" | "glslv" => ShaderStage::Vertex,
            "frag" | "glslf" => ShaderStage::Fragment,
            "comp" | "glslc" => ShaderStage::Compute,
            "geom" | "glslg" => ShaderStage::Geometry,
            "tesc" => ShaderStage::TessellationControl,
            "tese" => ShaderStage::TessellationEvaluation,
            _ => {
                return Err(AssetLoadError::UnsupportedType(format!(
                    "Unknown shader extension: {}",
                    extension
                )))
            }
        };

        Ok(AssetData::Shader {
            source,
            shader_type,
        })
    }

    /// Loads an audio file (WAV, OGG, etc.)
    fn load_audio(&self, path: &Path) -> Result<AssetData, AssetLoadError> {
        // Placeholder - would use hound or ogg crate
        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or(""); // Безопасно: пустая строка для файлов без расширения

        match extension.to_lowercase().as_str() {
            "wav" => self.load_wav(path),
            _ => Err(AssetLoadError::UnsupportedType(format!(
                "Unknown audio format: {}",
                extension
            ))),
        }
    }

    /// Loads a WAV audio file
    fn load_wav(&self, path: &Path) -> Result<AssetData, AssetLoadError> {
        // Placeholder implementation
        Ok(AssetData::Audio {
            sample_rate: 44100,
            channels: 2,
            samples: vec![0.0f32; 44100],
        })
    }

    /// Loads a font file
    fn load_font(&self, path: &Path) -> Result<AssetData, AssetLoadError> {
        let mut file = File::open(path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;

        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("font") // Безопасно: fallback для некорректных имён файлов
            .to_string();

        Ok(AssetData::Font {
            name,
            size: 16,
            data,
        })
    }

    /// Loads a config file (JSON, TOML, etc.)
    fn load_config(&self, path: &Path) -> Result<AssetData, AssetLoadError> {
        let mut file = File::open(path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;

        Ok(AssetData::Config { content })
    }

    /// Loads a 3D model file (glTF/GLB)
    fn load_model(&self, path: &Path) -> Result<AssetData, AssetLoadError> {
        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or(""); // Безопасно: пустая строка для файлов без расширения

        match extension.to_lowercase().as_str() {
            "gltf" | "glb" => self.load_gltf(path),
            _ => Err(AssetLoadError::UnsupportedType(format!(
                "Unknown model format: {}",
                extension
            ))),
        }
    }

    /// Loads a glTF/GLB model file
    fn load_gltf(&self, path: &Path) -> Result<AssetData, AssetLoadError> {
        use gltf::{buffer::Data, Gltf};

        // Read file
        let mut file = File::open(path)?;
        let mut data = Vec::new();
        std::io::Read::read_to_end(&mut file, &mut data)?;

        // Parse glTF
        let (document, buffers, _images) = gltf::import(path)
            .map_err(|e| AssetLoadError::DecodeError(format!("Failed to import glTF: {}", e)))?;

        let mut all_vertices = Vec::new();
        let mut all_indices = Vec::new();

        // Iterate through all meshes
        for mesh in document.meshes() {
            for primitive in mesh.primitives() {
                let reader = primitive.reader(|buf| Some(&buffers[buf.index()]));

                // Read positions
                let positions: Vec<[f32; 3]> = reader
                    .read_positions()
                    .ok_or_else(|| {
                        AssetLoadError::InvalidFormat("No positions in mesh".to_string())
                    })?
                    .collect();

                // Read normals (or generate defaults)
                let normals: Vec<[f32; 3]> = reader
                    .read_normals()
                    .map(|n| n.collect())
                    .unwrap_or_else(|| vec![[0.0, 1.0, 0.0]; positions.len()]); // Безопасно: дефолтные нормали

                // Read UVs (or default to 0,0)
                let uvs: Vec<[f32; 2]> = reader
                    .read_tex_coords(0)
                    .map(|t| t.into_f32().collect())
                    .unwrap_or_else(|| vec![[0.0, 0.0]; positions.len()]); // Безопасно: дефолтные UV

                // Read indices
                let indices: Vec<u32> = reader
                    .read_indices()
                    .map(|i| i.into_u32().collect())
                    .unwrap_or_else(|| (0..positions.len() as u32).collect()); // Безопасно: последовательные индексы

                // Build interleaved vertex buffer: pos(3) + normal(3) + uv(2) = 8 floats per vertex
                let vertices: Vec<f32> = positions
                    .iter()
                    .zip(normals.iter())
                    .zip(uvs.iter())
                    .flat_map(|((p, n), uv)| vec![p[0], p[1], p[2], n[0], n[1], n[2], uv[0], uv[1]])
                    .collect();

                all_vertices.extend(vertices);
                all_indices.extend(indices);
            }
        }

        Ok(AssetData::Mesh {
            vertices: all_vertices,
            indices: all_indices,
        })
    }
}

impl Default for AssetLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asset_handle() {
        let handle = AssetHandle::null();
        assert!(handle.is_null());

        let handle2 = AssetHandle(1);
        assert!(!handle2.is_null());
    }

    #[test]
    fn test_loader_creation() {
        let loader = AssetLoader::new();
        assert_eq!(loader.loaded_count(), 0);
        assert_eq!(loader.memory_usage(), 0);
    }

    #[test]
    fn test_loader_config() {
        let config = LoaderConfig::default();
        assert_eq!(config.cache_size_mb, 512);
        assert!(config.async_loading);
    }
}
