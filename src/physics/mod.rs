//! Physics Module for RTGC-0.8
//! Provides rigid body dynamics, collision detection, constraints, and vehicle physics

pub mod advanced_vehicle;
pub mod arena_allocator;
pub mod async_physics;
pub mod constraints;
pub mod crane_arm;
pub mod deformable_terrain;
pub mod fracture_component;
pub mod helicopter;
pub mod physics_module;
pub mod spatial_hash;
pub mod thread_pool;
pub mod tracked_vehicle;
pub mod vehicle;

// Re-export collision layer constants
pub use advanced_vehicle::AdvancedVehicle;
pub use arena_allocator::ArenaAllocator;
pub use async_physics::AsyncPhysicsEngine;
pub use constraints::{RaycastSuspension, SpringConstraint};
pub use crane_arm::{CraneArm, CraneConfig, CraneState};
pub use deformable_terrain::{DeformableTerrainComponent, DeformationType};
pub use fracture_component::FractureComponent;
pub use helicopter::{Helicopter, HelicopterConfig, HelicopterControls, HelicopterState};
pub use physics_module::{Aabb, PhysicsStats, PhysicsWorld, Ray, RaycastHit, RigidBody, Shape};
pub use physics_module::{LAYER_CARGO, LAYER_PLAYER, LAYER_TRIGGER, LAYER_VEHICLE, LAYER_WORLD};
pub use physics_module::{LAYER_INTERACTABLE_DOOR, LAYER_INTERACTABLE_OBJECT, LAYER_INTERACTABLE_VEHICLE};
pub use physics_module::set_global_physics_world;
pub use spatial_hash::SpatialHash;
pub use thread_pool::ThreadPool;
pub use tracked_vehicle::{
    TrackedControls, TrackedVehicle, TrackedVehicleState, TrackedVehicleType,
};
pub use vehicle::{Vehicle, VehicleConfig, VehicleControls};
// Re-export SurfaceType from world module for backward compatibility
// Note: This creates a dependency on world module, but is necessary for engine.rs
pub use crate::world::SurfaceType;
