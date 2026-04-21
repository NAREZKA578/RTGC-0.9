// Asset Loader - JSON-based serialization for game objects and vehicles
// Uses serde for flexible and efficient asset loading

use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use anyhow::{Result, Context};
use crate::error::AssetError;

/// Base trait for all loadable assets
pub trait Asset: Sized + Serialize + for<'de> Deserialize<'de> {
    const ASSET_TYPE: &'static str;

    fn load_from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path.as_ref())
            .with_context(|| format!("Failed to open asset file: {:?}", path.as_ref()))?;
        let reader = BufReader::new(file);
        serde_json::from_reader(reader)
            .with_context(|| format!("Failed to deserialize asset: {:?}", path.as_ref()))
    }

    fn save_to_path<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let file = File::create(path.as_ref())
            .with_context(|| format!("Failed to create asset file: {:?}", path.as_ref()))?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, self)
            .with_context(|| format!("Failed to serialize asset: {:?}", path.as_ref()))
    }
}

// ==================== Vehicle Assets ====================

/// Vehicle physics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VehiclePhysicsConfig {
    /// Mass in kilograms
    pub mass: f32,
    /// Engine power in horsepower
    pub engine_power: f32,
    /// Maximum torque in Nm
    pub max_torque: f32,
    /// Transmission ratios (forward gears)
    pub gear_ratios: Vec<f32>,
    /// Final drive ratio
    pub final_drive_ratio: f32,
    /// Wheel radius in meters
    pub wheel_radius: f32,
    /// Wheel width in meters
    pub wheel_width: f32,
    /// Suspension stiffness
    pub suspension_stiffness: f32,
    /// Suspension damping
    pub suspension_damping: f32,
    /// Maximum steering angle in radians
    pub max_steering_angle: f32,
    /// Brake force in Newtons
    pub brake_force: f32,
    /// Center of mass offset (x, y, z)
    pub center_of_mass: [f32; 3],
    /// Drag coefficient
    pub drag_coefficient: f32,
    /// Downforce coefficient
    pub downforce_coefficient: f32,
}

impl Default for VehiclePhysicsConfig {
    fn default() -> Self {
        Self {
            mass: 2000.0,
            engine_power: 300.0,
            max_torque: 500.0,
            gear_ratios: vec![3.5, 2.5, 1.8, 1.4, 1.0, 0.8],
            final_drive_ratio: 3.7,
            wheel_radius: 0.4,
            wheel_width: 0.25,
            suspension_stiffness: 35000.0,
            suspension_damping: 5000.0,
            max_steering_angle: 0.6,
            brake_force: 15000.0,
            center_of_mass: [0.0, 0.5, 0.0],
            drag_coefficient: 0.35,
            downforce_coefficient: 0.1,
        }
    }
}

/// Wheel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WheelConfig {
    /// Position relative to vehicle center (x, y, z)
    pub position: [f32; 3],
    /// Is this wheel steerable?
    pub steerable: bool,
    /// Is this wheel driven?
    pub driven: bool,
    /// Is this wheel braked?
    pub braked: bool,
    /// Suspension travel in meters
    pub suspension_travel: f32,
    /// Spring rest length in meters
    pub spring_rest_length: f32,
}

/// Vehicle definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VehicleAsset {
    /// Unique identifier
    pub id: String,
    /// Display name
    pub name: String,
    /// Vehicle type (truck, car, bus, etc.)
    pub vehicle_type: String,
    /// Manufacturer
    pub manufacturer: String,
    /// Model year
    pub model_year: u32,
    /// Physics configuration
    pub physics: VehiclePhysicsConfig,
    /// Wheel configurations
    pub wheels: Vec<WheelConfig>,
    /// Mesh file path
    pub mesh_path: String,
    /// Material/texture paths
    pub material_paths: Vec<String>,
    /// Interior mesh path (optional)
    pub interior_mesh_path: Option<String>,
    /// Collision mesh path (optional)
    pub collision_mesh_path: Option<String>,
    /// Engine sound path
    pub engine_sound_path: Option<String>,
    /// Max fuel capacity in liters
    pub fuel_capacity: f32,
    /// Fuel consumption rate (L/100km)
    pub fuel_consumption: f32,
    /// Maximum speed in km/h
    pub max_speed: f32,
    /// Cargo capacity in kg
    pub cargo_capacity: f32,
}

impl Asset for VehicleAsset {
    const ASSET_TYPE: &'static str = "vehicle";
}

impl VehicleAsset {
    /// Create a new vehicle asset with default values
    pub fn new(id: &str, name: &str, vehicle_type: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            vehicle_type: vehicle_type.to_string(),
            manufacturer: String::new(),
            model_year: 2024,
            physics: VehiclePhysicsConfig::default(),
            wheels: Vec::new(),
            mesh_path: String::new(),
            material_paths: Vec::new(),
            interior_mesh_path: None,
            collision_mesh_path: None,
            engine_sound_path: None,
            fuel_capacity: 100.0,
            fuel_consumption: 10.0,
            max_speed: 120.0,
            cargo_capacity: 1000.0,
        }
    }
    
    /// Load a preset vehicle configuration
    pub fn load_preset(preset: VehiclePreset) -> Self {
        match preset {
            VehiclePreset::KamazTruck => Self {
                id: "kamaz_truck".to_string(),
                name: "KAMAZ 6520".to_string(),
                vehicle_type: "truck".to_string(),
                manufacturer: "KAMAZ".to_string(),
                model_year: 2023,
                physics: VehiclePhysicsConfig {
                    mass: 15000.0,
                    engine_power: 400.0,
                    max_torque: 1800.0,
                    gear_ratios: vec![5.0, 3.5, 2.5, 1.8, 1.3, 1.0, 0.75],
                    final_drive_ratio: 5.0,
                    wheel_radius: 0.55,
                    wheel_width: 0.4,
                    suspension_stiffness: 80000.0,
                    suspension_damping: 12000.0,
                    max_steering_angle: 0.5,
                    brake_force: 40000.0,
                    center_of_mass: [0.0, 1.2, 0.5],
                    drag_coefficient: 0.6,
                    downforce_coefficient: 0.0,
                },
                wheels: vec![
                    WheelConfig { position: [-1.2, 0.5, 2.5], steerable: true, driven: false, braked: true, suspension_travel: 0.15, spring_rest_length: 0.5 },
                    WheelConfig { position: [1.2, 0.5, 2.5], steerable: true, driven: false, braked: true, suspension_travel: 0.15, spring_rest_length: 0.5 },
                    WheelConfig { position: [-1.2, 0.5, -1.0], steerable: false, driven: true, braked: true, suspension_travel: 0.15, spring_rest_length: 0.5 },
                    WheelConfig { position: [1.2, 0.5, -1.0], steerable: false, driven: true, braked: true, suspension_travel: 0.15, spring_rest_length: 0.5 },
                    WheelConfig { position: [-1.2, 0.5, -2.5], steerable: false, driven: true, braked: true, suspension_travel: 0.15, spring_rest_length: 0.5 },
                    WheelConfig { position: [1.2, 0.5, -2.5], steerable: false, driven: true, braked: true, suspension_travel: 0.15, spring_rest_length: 0.5 },
                ],
                fuel_capacity: 350.0,
                fuel_consumption: 35.0,
                max_speed: 90.0,
                cargo_capacity: 20000.0,
                ..Self::default()
            },
            VehiclePreset::PassengerCar => Self {
                id: "passenger_car".to_string(),
                name: "Lada Vesta".to_string(),
                vehicle_type: "car".to_string(),
                manufacturer: "AvtoVAZ".to_string(),
                model_year: 2023,
                physics: VehiclePhysicsConfig {
                    mass: 1300.0,
                    engine_power: 106.0,
                    max_torque: 148.0,
                    gear_ratios: vec![3.8, 2.2, 1.5, 1.1, 0.9],
                    final_drive_ratio: 3.9,
                    wheel_radius: 0.32,
                    wheel_width: 0.2,
                    suspension_stiffness: 25000.0,
                    suspension_damping: 3500.0,
                    max_steering_angle: 0.6,
                    brake_force: 8000.0,
                    center_of_mass: [0.0, 0.5, 0.0],
                    drag_coefficient: 0.32,
                    downforce_coefficient: 0.05,
                },
                wheels: vec![
                    WheelConfig { position: [-0.75, 0.3, 1.5], steerable: true, driven: true, braked: true, suspension_travel: 0.12, spring_rest_length: 0.35 },
                    WheelConfig { position: [0.75, 0.3, 1.5], steerable: true, driven: true, braked: true, suspension_travel: 0.12, spring_rest_length: 0.35 },
                    WheelConfig { position: [-0.75, 0.3, -1.5], steerable: false, driven: false, braked: true, suspension_travel: 0.12, spring_rest_length: 0.35 },
                    WheelConfig { position: [0.75, 0.3, -1.5], steerable: false, driven: false, braked: true, suspension_travel: 0.12, spring_rest_length: 0.35 },
                ],
                fuel_capacity: 55.0,
                fuel_consumption: 7.5,
                max_speed: 180.0,
                cargo_capacity: 480.0,
                ..Self::default()
            },
            VehiclePreset::Bus => Self {
                id: "city_bus".to_string(),
                name: "LiAZ 5292".to_string(),
                vehicle_type: "bus".to_string(),
                manufacturer: "LiAZ".to_string(),
                model_year: 2022,
                physics: VehiclePhysicsConfig {
                    mass: 18000.0,
                    engine_power: 340.0,
                    max_torque: 1400.0,
                    gear_ratios: vec![4.5, 3.0, 2.0, 1.5, 1.0],
                    final_drive_ratio: 4.5,
                    wheel_radius: 0.5,
                    wheel_width: 0.35,
                    suspension_stiffness: 90000.0,
                    suspension_damping: 15000.0,
                    max_steering_angle: 0.55,
                    brake_force: 50000.0,
                    center_of_mass: [0.0, 1.5, 0.0],
                    drag_coefficient: 0.7,
                    downforce_coefficient: 0.0,
                },
                wheels: vec![
                    WheelConfig { position: [-1.1, 0.5, 3.0], steerable: true, driven: false, braked: true, suspension_travel: 0.15, spring_rest_length: 0.5 },
                    WheelConfig { position: [1.1, 0.5, 3.0], steerable: true, driven: false, braked: true, suspension_travel: 0.15, spring_rest_length: 0.5 },
                    WheelConfig { position: [-1.1, 0.5, -2.0], steerable: false, driven: true, braked: true, suspension_travel: 0.15, spring_rest_length: 0.5 },
                    WheelConfig { position: [1.1, 0.5, -2.0], steerable: false, driven: true, braked: true, suspension_travel: 0.15, spring_rest_length: 0.5 },
                ],
                fuel_capacity: 280.0,
                fuel_consumption: 40.0,
                max_speed: 70.0,
                cargo_capacity: 10000.0,
                ..Self::default()
            },
        }
    }
}

impl Default for VehicleAsset {
    fn default() -> Self {
        Self::new("default", "Default Vehicle", "generic")
    }
}

/// Preset vehicle types
#[derive(Debug, Clone, Copy)]
pub enum VehiclePreset {
    KamazTruck,
    PassengerCar,
    Bus,
}

// ==================== Game Object Assets ====================

/// Transform component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transform {
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub scale: [f32; 3],
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
        }
    }
}

/// Collider types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ColliderType {
    Box { size: [f32; 3] },
    Sphere { radius: f32 },
    Capsule { radius: f32, height: f32 },
    Cylinder { radius: f32, height: f32 },
    Mesh { mesh_path: String },
}

/// Collider component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collider {
    pub collider_type: ColliderType,
    pub is_trigger: bool,
    pub friction: f32,
    pub restitution: f32,
}

impl Default for Collider {
    fn default() -> Self {
        Self {
            collider_type: ColliderType::Box { size: [1.0, 1.0, 1.0] },
            is_trigger: false,
            friction: 0.5,
            restitution: 0.1,
        }
    }
}

/// Rigid body dynamics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rigidbody {
    pub mass: f32,
    pub linear_damping: f32,
    pub angular_damping: f32,
    pub is_kinematic: bool,
    pub use_gravity: bool,
}

impl Default for Rigidbody {
    fn default() -> Self {
        Self {
            mass: 1.0,
            linear_damping: 0.05,
            angular_damping: 0.05,
            is_kinematic: false,
            use_gravity: true,
        }
    }
}

/// Light types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LightType {
    Directional,
    Point,
    Spot { inner_angle: f32, outer_angle: f32 },
    Area { width: f32, height: f32 },
}

/// Light component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Light {
    pub light_type: LightType,
    pub color: [f32; 3],
    pub intensity: f32,
    pub range: f32,
    pub cast_shadows: bool,
    pub shadow_bias: f32,
}

impl Default for Light {
    fn default() -> Self {
        Self {
            light_type: LightType::Directional,
            color: [1.0, 0.98, 0.95],
            intensity: 1.0,
            range: 100.0,
            cast_shadows: true,
            shadow_bias: 0.005,
        }
    }
}

/// Game object definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameObjectAsset {
    pub id: String,
    pub name: String,
    pub object_type: String,
    pub transform: Transform,
    pub mesh_path: Option<String>,
    pub material_paths: Vec<String>,
    pub collider: Option<Collider>,
    pub rigidbody: Option<Rigidbody>,
    pub light: Option<Light>,
    pub lod_distances: Vec<f32>,
    pub tags: Vec<String>,
    pub properties: serde_json::Value,
}

impl Asset for GameObjectAsset {
    const ASSET_TYPE: &'static str = "game_object";
}

impl GameObjectAsset {
    pub fn new(id: &str, name: &str, object_type: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            object_type: object_type.to_string(),
            transform: Transform::default(),
            mesh_path: None,
            material_paths: Vec::new(),
            collider: None,
            rigidbody: None,
            light: None,
            lod_distances: vec![0.0, 20.0, 50.0, 100.0],
            tags: Vec::new(),
            properties: serde_json::Value::Null,
        }
    }
}

// ==================== Asset Manager ====================

pub struct AssetManager {
    vehicles: std::collections::HashMap<String, VehicleAsset>,
    game_objects: std::collections::HashMap<String, GameObjectAsset>,
    search_paths: Vec<PathBuf>,
}

impl AssetManager {
    pub fn new() -> Self {
        Self {
            vehicles: std::collections::HashMap::new(),
            game_objects: std::collections::HashMap::new(),
            search_paths: vec![
                PathBuf::from("assets/vehicles"),
                PathBuf::from("assets/objects"),
                PathBuf::from("assets"),
            ],
        }
    }
    
    pub fn add_search_path<P: Into<PathBuf>>(&mut self, path: P) {
        self.search_paths.push(path.into());
    }
    
    fn find_asset_file(&self, asset_type: &str, name: &str) -> Option<PathBuf> {
        for search_path in &self.search_paths {
            let path = search_path.join(format!("{}.json", name));
            if path.exists() {
                return Some(path);
            }
        }
        None
    }
    
    pub fn load_vehicle(&mut self, name: &str) -> Result<&VehicleAsset> {
        if let Some(path) = self.find_asset_file("vehicle", name) {
            let vehicle = VehicleAsset::load_from_path(&path)?;
            self.vehicles.insert(vehicle.id.clone(), vehicle);
        } else {
            let preset = match name {
                "kamaz" | "kamaz_truck" => VehiclePreset::KamazTruck,
                "car" | "passenger_car" => VehiclePreset::PassengerCar,
                "bus" => VehiclePreset::Bus,
                _ => anyhow::bail!("Unknown vehicle preset: {}", name),
            };
            let vehicle = VehicleAsset::load_preset(preset);
            self.vehicles.insert(vehicle.id.clone(), vehicle);
        }
        
        self.vehicles.get(name)
            .ok_or_else(|| AssetError::NotFound(format!("Vehicle '{}' not found", name)).into())
    }
    
    pub fn load_game_object(&mut self, name: &str) -> Result<&GameObjectAsset> {
        if let Some(path) = self.find_asset_file("game_object", name) {
            let obj = GameObjectAsset::load_from_path(&path)?;
            self.game_objects.insert(obj.id.clone(), obj);
        }
        
        self.game_objects.get(name)
            .ok_or_else(|| AssetError::NotFound(format!("Game object '{}' not found", name)).into())
    }
    
    pub fn get_vehicle(&self, id: &str) -> Option<&VehicleAsset> {
        self.vehicles.get(id)
    }
    
    pub fn get_game_object(&self, id: &str) -> Option<&GameObjectAsset> {
        self.game_objects.get(id)
    }
    
    pub fn preload_directory<P: AsRef<Path>>(&mut self, dir: P) -> Result<usize> {
        let mut count = 0;
        let dir = dir.as_ref();
        
        if !dir.exists() {
            return Ok(0);
        }
        
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(asset_type) = value.get("asset_type").and_then(|v| v.as_str()) {
                            match asset_type {
                                "vehicle" => {
                                    if let Ok(vehicle) = serde_json::from_value::<VehicleAsset>(value) {
                                        self.vehicles.insert(vehicle.id.clone(), vehicle);
                                        count += 1;
                                    }
                                }
                                "game_object" => {
                                    if let Ok(obj) = serde_json::from_value::<GameObjectAsset>(value) {
                                        self.game_objects.insert(obj.id.clone(), obj);
                                        count += 1;
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
        
        Ok(count)
    }
}

impl Default for AssetManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_vehicle_serialization() {
        let vehicle = VehicleAsset::load_preset(VehiclePreset::KamazTruck);
        let json = serde_json::to_string_pretty(&vehicle)
            .map_err(|e| RhiError::InternalError(format!("Serialization failed: {}", e)))?; // Тест: паника допустима при ошибке сериализации
        let loaded: Result<VehicleAsset, _> = serde_json::from_str(&json);
        assert!(loaded.is_ok());
        let loaded = loaded.map_err(|e| RhiError::InternalError(format!("Deserialization failed: {}", e)))?; // Тест: явная ошибка вместо unwrap
        assert_eq!(vehicle.id, loaded.id);
        assert_eq!(vehicle.name, loaded.name);
    }
    
    #[test]
    fn test_game_object_serialization() {
        let mut obj = GameObjectAsset::new("test_obj", "Test Object", "prop");
        obj.transform.position = [1.0, 2.0, 3.0];
        obj.collider = Some(Collider::default());
        
        let json = serde_json::to_string_pretty(&obj)
            .map_err(|e| RhiError::InternalError(format!("Serialization failed: {}", e)))?; // Тест: паника допустима при ошибке сериализации
        let loaded: Result<GameObjectAsset, _> = serde_json::from_str(&json);
        assert!(loaded.is_ok());
        let loaded = loaded.map_err(|e| RhiError::InternalError(format!("Deserialization failed: {}", e)))?; // Тест: явная ошибка вместо unwrap
        assert_eq!(obj.id, loaded.id);
        assert_eq!(obj.transform.position, loaded.transform.position);
    }
}
