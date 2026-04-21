//! Entity component system - Component trait and standard components

use nalgebra::{Vector3, Quaternion, UnitQuaternion};
use crate::physics::RigidBody;
use crate::graphics::MeshHandle;

/// Trait for all components that can be attached to entities
pub trait Component: Send + Sync + 'static {
    /// Returns the name of this component type for debugging
    fn component_name() -> &'static str;
}

/// Transform component - position, rotation, scale in world space
#[derive(Debug, Clone)]
pub struct Transform {
    pub position: Vector3<f32>,
    pub rotation: Quaternion<f32>,
    pub scale: Vector3<f32>,
}

impl Transform {
    pub fn new(
        position: Vector3<f32>,
        rotation: Quaternion<f32>,
        scale: Vector3<f32>,
    ) -> Self {
        Self { position, rotation, scale }
    }

    pub fn identity() -> Self {
        Self {
            position: Vector3::zeros(),
            rotation: Quaternion::identity(),
            scale: Vector3::repeat(1.0),
        }
    }

    /// Returns the forward direction vector
    pub fn forward(&self) -> Vector3<f32> {
        let unit_rot = UnitQuaternion::from_quaternion(self.rotation);
        unit_rot * Vector3::new(0.0f32, 0.0, -1.0)
    }

    /// Returns the right direction vector
    pub fn right(&self) -> Vector3<f32> {
        let unit_rot = UnitQuaternion::from_quaternion(self.rotation);
        unit_rot * Vector3::new(1.0f32, 0.0, 0.0)
    }

    /// Returns the up direction vector
    pub fn up(&self) -> Vector3<f32> {
        let unit_rot = UnitQuaternion::from_quaternion(self.rotation);
        unit_rot * Vector3::new(0.0f32, 1.0, 0.0)
    }

    /// Transforms a point from local space to world space
    pub fn transform_point(&self, point: Vector3<f32>) -> Vector3<f32> {
        let unit_rot = UnitQuaternion::from_quaternion(self.rotation);
        self.position + unit_rot * (point.component_mul(&self.scale))
    }

    /// Transforms a direction from local space to world space (no translation)
    pub fn transform_direction(&self, direction: Vector3<f32>) -> Vector3<f32> {
        let unit_rot = UnitQuaternion::from_quaternion(self.rotation);
        unit_rot * direction
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::identity()
    }
}

impl Component for Transform {
    fn component_name() -> &'static str {
        "Transform"
    }
}

/// Velocity component - linear and angular velocity
#[derive(Debug, Clone, Copy)]
pub struct Velocity {
    pub linear: Vector3<f32>,
    pub angular: Vector3<f32>,
}

impl Velocity {
    pub fn new(linear: Vector3<f32>, angular: Vector3<f32>) -> Self {
        Self { linear, angular }
    }

    pub fn zero() -> Self {
        Self {
            linear: Vector3::zeros(),
            angular: Vector3::zeros(),
        }
    }
}

impl Default for Velocity {
    fn default() -> Self {
        Self::zero()
    }
}

impl Component for Velocity {
    fn component_name() -> &'static str {
        "Velocity"
    }
}

/// Mesh component - reference to a GPU mesh
#[derive(Debug, Clone)]
pub struct MeshComponent {
    pub mesh_handle: MeshHandle,
    pub material_index: u32,
    pub cast_shadow: bool,
    pub receive_shadow: bool,
}

impl MeshComponent {
    pub fn new(mesh_handle: MeshHandle) -> Self {
        Self {
            mesh_handle,
            material_index: 0,
            cast_shadow: true,
            receive_shadow: true,
        }
    }
}

impl Component for MeshComponent {
    fn component_name() -> &'static str {
        "Mesh"
    }
}

/// RigidBody component - physics simulation
#[derive(Debug, Clone)]
pub struct RigidBodyComponent {
    pub body: RigidBody,
    pub mass: f32,
    pub is_kinematic: bool,
    pub is_trigger: bool,
}

impl RigidBodyComponent {
    pub fn new(body: RigidBody, mass: f32) -> Self {
        Self {
            body,
            mass,
            is_kinematic: false,
            is_trigger: false,
        }
    }

    pub fn kinematic(body: RigidBody) -> Self {
        Self {
            body,
            mass: 0.0,
            is_kinematic: true,
            is_trigger: false,
        }
    }

    pub fn trigger(body: RigidBody) -> Self {
        Self {
            body,
            mass: 0.0,
            is_kinematic: false,
            is_trigger: true,
        }
    }
}

impl Component for RigidBodyComponent {
    fn component_name() -> &'static str {
        "RigidBody"
    }
}

/// Camera component - marks entity as a camera
#[derive(Debug, Clone)]
pub struct Camera {
    pub fov_y: f32,
    pub aspect_ratio: f32,
    pub near_plane: f32,
    pub far_plane: f32,
    pub is_active: bool,
}

impl Camera {
    pub fn new(fov_y: f32, aspect_ratio: f32) -> Self {
        Self {
            fov_y,
            aspect_ratio,
            near_plane: 0.1,
            far_plane: 1000.0,
            is_active: true,
        }
    }

    pub fn perspective(fov_y_degrees: f32, aspect_ratio: f32) -> Self {
        Self {
            fov_y: fov_y_degrees.to_radians(),
            aspect_ratio,
            near_plane: 0.1,
            far_plane: 1000.0,
            is_active: true,
        }
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self::perspective(60.0, 16.0 / 9.0)
    }
}

impl Component for Camera {
    fn component_name() -> &'static str {
        "Camera"
    }
}

/// Light component - light source types
#[derive(Debug, Clone)]
pub enum LightType {
    Directional {
        intensity: f32,
        color: Vector3<f32>,
    },
    Point {
        intensity: f32,
        color: Vector3<f32>,
        radius: f32,
    },
    Spot {
        intensity: f32,
        color: Vector3<f32>,
        inner_angle: f32,
        outer_angle: f32,
        radius: f32,
    },
}

#[derive(Debug, Clone)]
pub struct Light {
    pub light_type: LightType,
    pub cast_shadows: bool,
    pub shadow_map_resolution: u32,
}

impl Light {
    pub fn directional(intensity: f32, color: Vector3<f32>) -> Self {
        Self {
            light_type: LightType::Directional { intensity, color },
            cast_shadows: true,
            shadow_map_resolution: 2048,
        }
    }

    pub fn point(intensity: f32, color: Vector3<f32>, radius: f32) -> Self {
        Self {
            light_type: LightType::Point { intensity, color, radius },
            cast_shadows: true,
            shadow_map_resolution: 1024,
        }
    }

    pub fn spot(
        intensity: f32,
        color: Vector3<f32>,
        inner_angle: f32,
        outer_angle: f32,
        radius: f32,
    ) -> Self {
        Self {
            light_type: LightType::Spot {
                intensity,
                color,
                inner_angle,
                outer_angle,
                radius,
            },
            cast_shadows: true,
            shadow_map_resolution: 1024,
        }
    }
}

impl Component for Light {
    fn component_name() -> &'static str {
        "Light"
    }
}

/// Tag component for simple entity categorization
#[derive(Debug, Clone)]
pub struct Tag(pub String);

impl Tag {
    pub fn new(name: &str) -> Self {
        Self(name.to_string())
    }
}

impl Component for Tag {
    fn component_name() -> &'static str {
        "Tag"
    }
}

/// Name component for entity identification
#[derive(Debug, Clone)]
pub struct Name(pub String);

impl Name {
    pub fn new(name: &str) -> Self {
        Self(name.to_string())
    }
}

impl Component for Name {
    fn component_name() -> &'static str {
        "Name"
    }
}
