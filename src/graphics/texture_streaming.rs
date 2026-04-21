use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

// Represents a texture tile with position in the world
#[derive(Debug, Clone)]
pub struct TextureTile {
    pub id: u32,
    pub x: i32,
    pub y: i32,
    pub zoom_level: u8,
    pub file_path: String,
    pub loaded: bool,
}

// Manages texture loading/unloading based on camera position
#[derive(Clone)]
pub struct TextureStreamingSystem {
    // Cache of currently loaded textures
    texture_cache: Arc<RwLock<HashMap<String, TextureTile>>>,

    // Maximum number of textures to keep in memory
    max_cache_size: usize,

    // Current camera position
    camera_position: nalgebra::Vector2<f32>,

    // Size of texture tiles in world units
    tile_size: f32,

    // Radius of tiles to keep loaded around camera (in tiles)
    load_radius: u32,

    // Background thread handle - wrapped in Arc for Clone
    worker_thread: Arc<Option<thread::JoinHandle<()>>>,

    // Channel for sending commands to the worker thread
    command_sender: Option<std::sync::mpsc::Sender<TextureCommand>>,

    // Flag to stop the worker thread
    stop_signal: Arc<std::sync::atomic::AtomicBool>,
}

#[derive(Debug)]
enum TextureCommand {
    UpdateCamera(nalgebra::Vector2<f32>),
    LoadTile(String, TextureTile),
    UnloadTile(String),
    Shutdown,
}

impl TextureStreamingSystem {
    pub fn new(max_cache_size: usize, tile_size: f32, load_radius: u32) -> Self {
        let texture_cache = Arc::new(RwLock::new(HashMap::new()));
        let stop_signal = Arc::new(std::sync::atomic::AtomicBool::new(false));

        let cache_clone = Arc::clone(&texture_cache);
        let stop_clone = Arc::clone(&stop_signal);

        let (sender, receiver) = std::sync::mpsc::channel::<TextureCommand>();

        let worker_thread = thread::spawn(move || {
            while !stop_clone.load(std::sync::atomic::Ordering::Relaxed) {
                // Process all pending commands
                while let Ok(cmd) = receiver.try_recv() {
                    match cmd {
                        TextureCommand::UpdateCamera(pos) => {
                            // Update internal camera position
                            // Actual loading/unloading happens in the main logic
                        }
                        TextureCommand::LoadTile(key, tile) => {
                            let mut cache = cache_clone.write();
                            cache.insert(key, tile);
                        }
                        TextureCommand::UnloadTile(key) => {
                            let mut cache = cache_clone.write();
                            cache.remove(&key);

                            // In a real implementation, we would free GPU memory here
                        }
                        TextureCommand::Shutdown => {
                            return;
                        }
                    }
                }

                // Small delay to prevent busy waiting
                thread::sleep(Duration::from_millis(16)); // ~60 FPS
            }
        });

        Self {
            texture_cache,
            max_cache_size,
            camera_position: nalgebra::Vector2::new(0.0, 0.0),
            tile_size,
            load_radius,
            worker_thread: Arc::new(Some(worker_thread)),
            command_sender: Some(sender),
            stop_signal,
        }
    }

    pub fn update_camera_position(&mut self, position: nalgebra::Vector2<f32>) {
        // Update our camera position
        self.camera_position = position;

        // Send command to worker thread
        if let Some(ref sender) = self.command_sender {
            let _ = sender.send(TextureCommand::UpdateCamera(position));
        }

        // Calculate which tiles should be loaded based on new camera position
        self.manage_tiles();
    }

    fn manage_tiles(&mut self) {
        let center_x = (self.camera_position.x / self.tile_size) as i32;
        let center_y = (self.camera_position.y / self.tile_size) as i32;

        let mut needed_tiles = Vec::new();

        // Generate list of tiles that should be loaded
        let radius = self.load_radius as i32;
        for dx in -radius..=radius {
            for dy in -radius..=radius {
                // Simple distance check to make it roughly circular instead of square
                let distance_sq = (dx * dx + dy * dy) as f32;
                if distance_sq <= (self.load_radius as f32).powi(2) {
                    let tile_x = center_x + dx;
                    let tile_y = center_y + dy;

                    let key = format!("tile_{}_{}_{}", tile_x, tile_y, 0); // Assuming zoom level 0
                    let tile = TextureTile {
                        id: 0, // Will be assigned when loaded
                        x: tile_x,
                        y: tile_y,
                        zoom_level: 0,
                        file_path: format!("assets/textures/tile_{}_{}.png", tile_x, tile_y),
                        loaded: false,
                    };

                    needed_tiles.push((key, tile));
                }
            }
        }

        // Check which tiles we need to load
        {
            let cache = self.texture_cache.read();
            for (key, tile) in &needed_tiles {
                if !cache.contains_key(key) {
                    // Need to load this tile
                    if let Some(ref sender) = self.command_sender {
                        let _ = sender.send(TextureCommand::LoadTile(key.clone(), tile.clone()));
                    }
                }
            }
        }

        // Check which tiles we need to unload (outside the extended radius)
        let extended_load_radius = self.load_radius + 2; // Unload slightly outside the load radius

        {
            let cache = self.texture_cache.read();
            let keys_to_unload: Vec<String> = cache
                .keys()
                .filter(|key| {
                    // Parse the key to get tile coordinates
                    if let Some(coords) = parse_tile_key(key) {
                        let tile_center_x = (coords.0 as f32) * self.tile_size;
                        let tile_center_y = (coords.1 as f32) * self.tile_size;
                        let distance = ((tile_center_x - self.camera_position.x).powi(2)
                            + (tile_center_y - self.camera_position.y).powi(2))
                        .sqrt();

                        distance > (extended_load_radius as f32) * self.tile_size
                    } else {
                        false
                    }
                })
                .cloned()
                .collect();

            for key in keys_to_unload {
                if let Some(ref sender) = self.command_sender {
                    let _ = sender.send(TextureCommand::UnloadTile(key));
                }
            }
        }
    }

    pub fn is_tile_loaded(&self, x: i32, y: i32) -> bool {
        let key = format!("tile_{}_{}_{}", x, y, 0);
        self.texture_cache.read().contains_key(&key)
    }

    pub fn get_loaded_texture_count(&self) -> usize {
        self.texture_cache.read().len()
    }

    pub fn get_texture_cache_size(&self) -> usize {
        self.texture_cache.read().len()
    }
}

fn parse_tile_key(key: &str) -> Option<(i32, i32, u8)> {
    // Parse "tile_x_y_zoom" format
    let parts: Vec<&str> = key.split('_').collect();
    if parts.len() >= 4 && parts[0] == "tile" {
        if let (Ok(x), Ok(y), Ok(zoom)) = (parts[1].parse(), parts[2].parse(), parts[3].parse()) {
            Some((x, y, zoom))
        } else {
            None
        }
    } else {
        None
    }
}

impl Drop for TextureStreamingSystem {
    fn drop(&mut self) {
        // Signal the worker thread to stop
        self.stop_signal
            .store(true, std::sync::atomic::Ordering::Relaxed);

        // Send shutdown command
        if let Some(ref sender) = self.command_sender {
            let _ = sender.send(TextureCommand::Shutdown);
        }

        // Note: We cannot join the thread here because JoinHandle is not Clone.
        // The thread will naturally stop when it checks the stop_signal.
        // For proper shutdown, call shutdown() explicitly before dropping.
    }
}
