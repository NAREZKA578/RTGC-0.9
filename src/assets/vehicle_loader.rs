// ЧАСТЬ 3 — ТРАНСПОРТ: ХРАНЕНИЕ И ЗАГРУЗКА
// Загрузчик транспорта из .vehicle.toml файлов

use crate::assets::AssetLoader;
use crate::graphics::renderer::Model;
use crate::physics::vehicle::{VehicleConfig, WheelState};
use crate::physics::{PhysicsWorld, RigidBody};
use nalgebra::{UnitQuaternion, Vector3};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Cursor;
use std::path::Path;

/// Метаданные транспортного средства
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VehicleMetadata {
    pub id: String,
    pub name: String,
    pub category: String,
    #[serde(default)]
    pub unlock_condition: Option<String>,
}

/// Конфигурация двигателя
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    #[serde(default = "default_engine_type")]
    pub r#type: String,
    pub max_power_kw: f32,
    pub max_torque_nm: f32,
    pub idle_rpm: f32,
    pub max_rpm: f32,
    pub fuel_capacity_l: f32,
    #[serde(default = "default_fuel_consumption")]
    pub fuel_consumption_l_per_100km: f32,
}

fn default_engine_type() -> String {
    "diesel".to_string()
}
fn default_fuel_consumption() -> f32 {
    30.0
}

/// Конфигурация трансмиссии
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransmissionConfig {
    #[serde(default = "default_transmission_type")]
    pub r#type: String,
    pub gears: u8,
    #[serde(default = "default_reverse_gears")]
    pub reverse_gears: u8,
    pub gear_ratios: Vec<f32>,
    pub reverse_ratio: f32,
    pub final_drive: f32,
}

fn default_transmission_type() -> String {
    "manual".to_string()
}
fn default_reverse_gears() -> u8 {
    1
}

/// Конфигурация трансмиссии
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DrivetrainConfig {
    #[serde(default = "default_drivetrain_type")]
    pub r#type: String,
    #[serde(default)]
    pub has_front_diff_lock: bool,
    #[serde(default)]
    pub has_rear_diff_lock: bool,
    #[serde(default)]
    pub has_center_diff_lock: bool,
    #[serde(default)]
    pub has_low_range: bool,
    #[serde(default = "default_low_range_ratio")]
    pub low_range_ratio: f32,
}

fn default_drivetrain_type() -> String {
    "4x4".to_string()
}
fn default_low_range_ratio() -> f32 {
    2.0
}

/// Определение колеса
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WheelDefinition {
    pub id: String,
    pub position: [f32; 3],
    pub radius_m: f32,
    pub width_m: f32,
    #[serde(default)]
    pub mesh: Option<String>,
    #[serde(default)]
    pub is_steerable: bool,
    #[serde(default = "default_true")]
    pub is_driven: bool,
    #[serde(default = "default_steer_angle")]
    pub max_steer_angle_deg: f32,
    #[serde(default = "default_suspension_stiffness")]
    pub suspension_stiffness: f32,
    #[serde(default = "default_suspension_damping")]
    pub suspension_damping: f32,
    #[serde(default = "default_suspension_rest_length")]
    pub suspension_rest_length: f32,
}

fn default_true() -> bool {
    true
}
fn default_steer_angle() -> f32 {
    35.0
}
fn default_suspension_stiffness() -> f32 {
    75000.0
}
fn default_suspension_damping() -> f32 {
    5000.0
}
fn default_suspension_rest_length() -> f32 {
    0.45
}

/// Аудио ресурсы транспорта
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VehicleAudioHandles {
    #[serde(default)]
    pub engine_idle: Option<String>,
    #[serde(default)]
    pub engine_rev: Option<String>,
    #[serde(default)]
    pub tire_dirt: Option<String>,
    #[serde(default)]
    pub tire_mud: Option<String>,
}

/// Параметры повреждений
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DamageConfig {
    pub max_health: f32,
    pub engine_damage_threshold: f32,
    pub immobilized_threshold: f32,
}

/// Полное определение транспорта
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VehicleDefinition {
    pub metadata: VehicleMetadata,
    #[serde(rename = "body")]
    pub body_config: BodyConfig,
    pub engine: EngineConfig,
    pub transmission: TransmissionConfig,
    #[serde(default)]
    pub drivetrain: DrivetrainConfig,
    pub wheels: Vec<WheelDefinition>,
    #[serde(default)]
    pub audio: VehicleAudioHandles,
    #[serde(default)]
    pub damage: Option<DamageConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BodyConfig {
    pub mass_kg: f32,
    #[serde(default)]
    pub center_of_mass: [f32; 3],
    pub dimensions: [f32; 3],
    #[serde(default = "default_drag_coefficient")]
    pub drag_coefficient: f32,
    #[serde(default)]
    pub mesh: Option<String>,
}

fn default_drag_coefficient() -> f32 {
    0.7
}

/// Ошибки загрузки транспорта
#[derive(Debug, Clone)]
pub enum VehicleLoadError {
    FileNotFound(String),
    ParseError(String),
    InvalidConfig(String),
    AssetLoadError(String),
}

impl std::fmt::Display for VehicleLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            VehicleLoadError::FileNotFound(path) => write!(f, "File not found: {}", path),
            VehicleLoadError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            VehicleLoadError::InvalidConfig(msg) => write!(f, "Invalid config: {}", msg),
            VehicleLoadError::AssetLoadError(msg) => write!(f, "Asset load error: {}", msg),
        }
    }
}

/// Загрузчик транспорта
pub struct VehicleLoader;

impl VehicleLoader {
    /// Загрузить транспорт по ID из папки assets/vehicles/
    pub fn load(
        id: &str,
        _loader: &mut AssetLoader,
    ) -> Result<VehicleDefinition, VehicleLoadError> {
        let vehicle_path = format!("assets/vehicles/{}/{}.vehicle.toml", id, id);

        // Для альфы используем дефолтные значения если файл не найден
        let content = if Path::new(&vehicle_path).exists() {
            fs::read_to_string(&vehicle_path)
                .map_err(|e| VehicleLoadError::FileNotFound(format!("{}: {}", vehicle_path, e)))?
        } else {
            // Возвращаем дефолтный конфиг для тестирования
            return Ok(Self::create_default_vehicle(id));
        };

        let def: VehicleDefinition =
            toml::from_str(&content).map_err(|e| VehicleLoadError::ParseError(e.to_string()))?;

        Ok(def)
    }

    /// Загрузить GLTF/GLB модель транспорта
    pub fn load_gltf(path: &str) -> Result<Model, String> {
        use crate::graphics::mesh::Vertex;
        use std::path::PathBuf;

        let full_path = PathBuf::from(path);

        // Проверка существования файла
        if !full_path.exists() {
            // Если файл не найден, пробуем альтернативные пути
            let alt_paths = [
                format!("assets/models/{}", path),
                format!("assets/vehicles/{}", path),
                path.to_string(),
            ];

            let found_path = alt_paths
                .iter()
                .find(|p| PathBuf::from(p).exists())
                .ok_or_else(|| {
                    format!(
                        "GLTF file not found: {} (tried: {})",
                        path,
                        alt_paths.join(", ")
                    )
                })?;

            return Self::load_gltf_from_path(found_path);
        }

        Self::load_gltf_from_path(path)
    }

    /// Загрузить GLTF/GLB из указанного пути
    fn load_gltf_from_path(path: &str) -> Result<Model, String> {
        use crate::graphics::mesh::Vertex;

        let is_glb = path.ends_with(".glb") || path.ends_with(".GLB");

        let (document, buffers, _images) = match gltf::import(path) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("GLTF import failed (non-critical): {}", e);
                return Err(format!("GLTF import failed: {}", e));
            }
        };

        let mut vertices: Vec<Vertex> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();
        let mut index_offset: u32 = 0;

        for mesh in document.meshes() {
            for primitive in mesh.primitives() {
                let reader =
                    primitive.reader(|buffer| buffers.get(buffer.index()).map(|x| x.0.as_slice()));

                // Читаем позиции вершин
                if let Some(positions) = reader.read_positions() {
                    for pos in positions {
                        vertices.push(Vertex {
                            position: pos,
                            normal: [0.0, 1.0, 0.0],
                            tex_coords: [0.0, 0.0],
                        });
                    }
                }

                // Читаем нормали если есть
                if let Some(normals) = reader.read_normals() {
                    let normals: Vec<[f32; 3]> = normals.collect();
                    for (i, normal) in normals.iter().enumerate() {
                        if i < vertices.len() {
                            vertices[i].normal = *normal;
                        }
                    }
                }

                // Читаем индексы
                if let Some(indices_reader) = reader.read_indices() {
                    match indices_reader {
                        gltf::mesh::util::ReadIndices::U16(iter) => {
                            for idx in iter {
                                indices.push(index_offset + idx as u32);
                            }
                        }
                        gltf::mesh::util::ReadIndices::U32(iter) => {
                            for idx in iter {
                                indices.push(index_offset + idx);
                            }
                        }
                        gltf::mesh::util::ReadIndices::U8(iter) => {
                            for idx in iter {
                                indices.push(index_offset + idx as u32);
                            }
                        }
                    }
                }

                index_offset = vertices.len() as u32;
            }
        }

        // Если вершины не найдены, создаём простую коробку как заглушку
        if vertices.is_empty() {
            tracing::warn!("No vertices found in GLTF {}, using box placeholder", path);
            let size = Vector3::new(1.0, 1.0, 2.0);
            vertices = Self::create_box_mesh(size);
            indices = vec![
                0, 1, 2, 0, 2, 3, // Front
                4, 5, 6, 4, 6, 7, // Back
                8, 9, 10, 8, 10, 11, // Top
                12, 13, 14, 12, 14, 15, // Bottom
                16, 17, 18, 16, 18, 19, // Left
                20, 21, 22, 20, 22, 23, // Right
            ];
        }

        // Создаём модель с пустыми мешами и текстурами
        // Примечание: реальная загрузка требует создание Mesh через renderer
        Ok(Model {
            meshes: Vec::new(),
            textures: Vec::new(),
        })
    }

    /// Создать простую коробку-меш
    fn create_box_mesh(size: Vector3<f32>) -> Vec<crate::graphics::mesh::Vertex> {
        let hx = size.x / 2.0;
        let hy = size.y / 2.0;
        let hz = size.z / 2.0;

        vec![
            // Front face
            crate::graphics::mesh::Vertex {
                position: [-hx, -hy, hz],
                normal: [0.0, 0.0, 1.0],
                tex_coords: [0.0, 0.0],
            },
            crate::graphics::mesh::Vertex {
                position: [hx, -hy, hz],
                normal: [0.0, 0.0, 1.0],
                tex_coords: [1.0, 0.0],
            },
            crate::graphics::mesh::Vertex {
                position: [hx, hy, hz],
                normal: [0.0, 0.0, 1.0],
                tex_coords: [1.0, 1.0],
            },
            crate::graphics::mesh::Vertex {
                position: [-hx, hy, hz],
                normal: [0.0, 0.0, 1.0],
                tex_coords: [0.0, 1.0],
            },
            // Back face
            crate::graphics::mesh::Vertex {
                position: [hx, -hy, -hz],
                normal: [0.0, 0.0, -1.0],
                tex_coords: [0.0, 0.0],
            },
            crate::graphics::mesh::Vertex {
                position: [-hx, -hy, -hz],
                normal: [0.0, 0.0, -1.0],
                tex_coords: [1.0, 0.0],
            },
            crate::graphics::mesh::Vertex {
                position: [-hx, hy, -hz],
                normal: [0.0, 0.0, -1.0],
                tex_coords: [1.0, 1.0],
            },
            crate::graphics::mesh::Vertex {
                position: [hx, hy, -hz],
                normal: [0.0, 0.0, -1.0],
                tex_coords: [0.0, 1.0],
            },
            // Top face
            crate::graphics::mesh::Vertex {
                position: [-hx, hy, hz],
                normal: [0.0, 1.0, 0.0],
                tex_coords: [0.0, 0.0],
            },
            crate::graphics::mesh::Vertex {
                position: [hx, hy, hz],
                normal: [0.0, 1.0, 0.0],
                tex_coords: [1.0, 0.0],
            },
            crate::graphics::mesh::Vertex {
                position: [hx, hy, -hz],
                normal: [0.0, 1.0, 0.0],
                tex_coords: [1.0, 1.0],
            },
            crate::graphics::mesh::Vertex {
                position: [-hx, hy, -hz],
                normal: [0.0, 1.0, 0.0],
                tex_coords: [0.0, 1.0],
            },
            // Bottom face
            crate::graphics::mesh::Vertex {
                position: [hx, -hy, hz],
                normal: [0.0, -1.0, 0.0],
                tex_coords: [0.0, 0.0],
            },
            crate::graphics::mesh::Vertex {
                position: [-hx, -hy, hz],
                normal: [0.0, -1.0, 0.0],
                tex_coords: [1.0, 0.0],
            },
            crate::graphics::mesh::Vertex {
                position: [-hx, -hy, -hz],
                normal: [0.0, -1.0, 0.0],
                tex_coords: [1.0, 1.0],
            },
            crate::graphics::mesh::Vertex {
                position: [hx, -hy, -hz],
                normal: [0.0, -1.0, 0.0],
                tex_coords: [0.0, 1.0],
            },
            // Left face
            crate::graphics::mesh::Vertex {
                position: [-hx, -hy, hz],
                normal: [-1.0, 0.0, 0.0],
                tex_coords: [0.0, 0.0],
            },
            crate::graphics::mesh::Vertex {
                position: [-hx, hy, hz],
                normal: [-1.0, 0.0, 0.0],
                tex_coords: [1.0, 0.0],
            },
            crate::graphics::mesh::Vertex {
                position: [-hx, hy, -hz],
                normal: [-1.0, 0.0, 0.0],
                tex_coords: [1.0, 1.0],
            },
            crate::graphics::mesh::Vertex {
                position: [-hx, -hy, -hz],
                normal: [-1.0, 0.0, 0.0],
                tex_coords: [0.0, 1.0],
            },
            // Right face
            crate::graphics::mesh::Vertex {
                position: [hx, -hy, -hz],
                normal: [1.0, 0.0, 0.0],
                tex_coords: [0.0, 0.0],
            },
            crate::graphics::mesh::Vertex {
                position: [hx, hy, -hz],
                normal: [1.0, 0.0, 0.0],
                tex_coords: [1.0, 0.0],
            },
            crate::graphics::mesh::Vertex {
                position: [hx, hy, hz],
                normal: [1.0, 0.0, 0.0],
                tex_coords: [1.0, 1.0],
            },
            crate::graphics::mesh::Vertex {
                position: [hx, -hy, hz],
                normal: [1.0, 0.0, 0.0],
                tex_coords: [0.0, 1.0],
            },
        ]
    }

    /// Список всех доступных транспортных средств
    pub fn list_available() -> Vec<VehicleMetadata> {
        let mut vehicles = Vec::new();

        if let Ok(entries) = fs::read_dir("assets/vehicles") {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(id) = path.file_name().and_then(|n| n.to_str()) {
                        vehicles.push(VehicleMetadata {
                            id: id.to_string(),
                            name: id.replace("_", " ").to_uppercase(),
                            category: "truck".to_string(),
                            unlock_condition: None,
                        });
                    }
                }
            }
        }

        if vehicles.is_empty() {
            // Добавить дефолтный автомобиль
            vehicles.push(VehicleMetadata {
                id: "default_truck".to_string(),
                name: "Default Truck".to_string(),
                category: "truck".to_string(),
                unlock_condition: None,
            });
        }

        vehicles
    }

    /// Создать физический объект в PhysicsWorld из определения
    pub fn spawn(
        def: &VehicleDefinition,
        position: Vector3<f32>,
        rotation: UnitQuaternion<f32>,
        world: &mut PhysicsWorld,
    ) -> usize {
        // Создать шасси как rigid body
        let half_extents = Vector3::new(
            def.body_config.dimensions[0] / 2.0,
            def.body_config.dimensions[1] / 2.0,
            def.body_config.dimensions[2] / 2.0,
        );

        let mut chassis = RigidBody::new_box(position, def.body_config.mass_kg, half_extents);
        chassis.rotation = rotation;
        chassis.collision_layer = crate::physics::LAYER_VEHICLE;
        chassis.collision_mask = crate::physics::LAYER_WORLD | crate::physics::LAYER_CARGO;
        chassis.enable_ccd = true;

        let chassis_id = world.add_body(chassis);

        // Настроить колёса (для простой модели пока не создаём отдельные тела)
        // В полной версии здесь создавались бы SpringConstraint для подвески

        chassis_id
    }

    /// Конвертировать VehicleDefinition в VehicleConfig
    pub fn to_vehicle_config(def: &VehicleDefinition) -> VehicleConfig {
        VehicleConfig {
            mass: def.body_config.mass_kg,
            wheel_count: def.wheels.len() as u8,
            wheel_radius: def.wheels.first().map(|w| w.radius_m).unwrap_or(0.5),
            suspension_stiffness: def
                .wheels
                .first()
                .map(|w| w.suspension_stiffness)
                .unwrap_or(75000.0),
            suspension_damping: def
                .wheels
                .first()
                .map(|w| w.suspension_damping)
                .unwrap_or(5000.0),
            suspension_rest_length: def
                .wheels
                .first()
                .map(|w| w.suspension_rest_length)
                .unwrap_or(0.4),
            max_suspension_travel: 0.2,
            engine_force: def.engine.max_power_kw * 1000.0, // kW -> W
            brake_force: 10000.0,
            max_steering_angle: def
                .wheels
                .iter()
                .find(|w| w.is_steerable)
                .map(|w| w.max_steer_angle_deg.to_radians())
                .unwrap_or(0.6), // ~35 градусов
            lateral_friction: 1.0,
            longitudinal_friction: 1.0,
            drag_coefficient: def.body_config.drag_coefficient,
            downforce_coefficient: 0.0,
            diff_front_locked: def.drivetrain.has_front_diff_lock,
            diff_rear_locked: def.drivetrain.has_rear_diff_lock,
            low_range_enabled: def.drivetrain.has_low_range,
            low_range_ratio: def.drivetrain.low_range_ratio,
        }
    }

    /// Создать дефолтный автомобиль для тестирования
    fn create_default_vehicle(id: &str) -> VehicleDefinition {
        VehicleDefinition {
            metadata: VehicleMetadata {
                id: id.to_string(),
                name: "Default Truck".to_string(),
                category: "truck".to_string(),
                unlock_condition: None,
            },
            body_config: BodyConfig {
                mass_kg: 8200.0,
                center_of_mass: [0.0, -0.3, 0.2],
                dimensions: [2.7, 2.9, 8.1],
                drag_coefficient: 0.7,
                mesh: None,
            },
            engine: EngineConfig {
                r#type: "diesel".to_string(),
                max_power_kw: 176.0,
                max_torque_nm: 833.0,
                idle_rpm: 600.0,
                max_rpm: 2100.0,
                fuel_capacity_l: 250.0,
                fuel_consumption_l_per_100km: 38.0,
            },
            transmission: TransmissionConfig {
                r#type: "manual".to_string(),
                gears: 5,
                reverse_gears: 1,
                gear_ratios: vec![6.17, 3.40, 1.79, 1.00, 0.78],
                reverse_ratio: 6.69,
                final_drive: 8.21,
            },
            drivetrain: DrivetrainConfig {
                r#type: "6x6".to_string(),
                has_front_diff_lock: true,
                has_rear_diff_lock: true,
                has_center_diff_lock: true,
                has_low_range: true,
                low_range_ratio: 2.15,
            },
            wheels: vec![
                // Передние колёса
                WheelDefinition {
                    id: "front_left".to_string(),
                    position: [-0.95, -0.6, 2.1],
                    radius_m: 0.53,
                    width_m: 0.34,
                    mesh: None,
                    is_steerable: true,
                    is_driven: true,
                    max_steer_angle_deg: 35.0,
                    suspension_stiffness: 75000.0,
                    suspension_damping: 5000.0,
                    suspension_rest_length: 0.45,
                },
                WheelDefinition {
                    id: "front_right".to_string(),
                    position: [0.95, -0.6, 2.1],
                    radius_m: 0.53,
                    width_m: 0.34,
                    mesh: None,
                    is_steerable: true,
                    is_driven: true,
                    max_steer_angle_deg: 35.0,
                    suspension_stiffness: 75000.0,
                    suspension_damping: 5000.0,
                    suspension_rest_length: 0.45,
                },
                // Задние колёса (два моста)
                WheelDefinition {
                    id: "rear_left_1".to_string(),
                    position: [-0.95, -0.6, -0.5],
                    radius_m: 0.53,
                    width_m: 0.34,
                    mesh: None,
                    is_steerable: false,
                    is_driven: true,
                    max_steer_angle_deg: 0.0,
                    suspension_stiffness: 85000.0,
                    suspension_damping: 6000.0,
                    suspension_rest_length: 0.45,
                },
                WheelDefinition {
                    id: "rear_right_1".to_string(),
                    position: [0.95, -0.6, -0.5],
                    radius_m: 0.53,
                    width_m: 0.34,
                    mesh: None,
                    is_steerable: false,
                    is_driven: true,
                    max_steer_angle_deg: 0.0,
                    suspension_stiffness: 85000.0,
                    suspension_damping: 6000.0,
                    suspension_rest_length: 0.45,
                },
                WheelDefinition {
                    id: "rear_left_2".to_string(),
                    position: [-0.95, -0.6, -1.5],
                    radius_m: 0.53,
                    width_m: 0.34,
                    mesh: None,
                    is_steerable: false,
                    is_driven: true,
                    max_steer_angle_deg: 0.0,
                    suspension_stiffness: 85000.0,
                    suspension_damping: 6000.0,
                    suspension_rest_length: 0.45,
                },
                WheelDefinition {
                    id: "rear_right_2".to_string(),
                    position: [0.95, -0.6, -1.5],
                    radius_m: 0.53,
                    width_m: 0.34,
                    mesh: None,
                    is_steerable: false,
                    is_driven: true,
                    max_steer_angle_deg: 0.0,
                    suspension_stiffness: 85000.0,
                    suspension_damping: 6000.0,
                    suspension_rest_length: 0.45,
                },
            ],
            audio: VehicleAudioHandles::default(),
            damage: Some(DamageConfig {
                max_health: 1000.0,
                engine_damage_threshold: 600.0,
                immobilized_threshold: 200.0,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_default_vehicle() {
        let def = VehicleLoader::create_default_vehicle("test_truck");
        assert_eq!(def.metadata.id, "test_truck");
        assert_eq!(def.wheels.len(), 6);
        assert!(def.body_config.mass_kg > 0.0);
    }

    #[test]
    fn test_to_vehicle_config() {
        let def = VehicleLoader::create_default_vehicle("test");
        let config = VehicleLoader::to_vehicle_config(&def);
        assert!(config.engine_power > 0.0);
        assert_eq!(config.wheel_count, 6);
    }
}
