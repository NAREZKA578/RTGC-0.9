//! Render Command - Encapsulates rendering operations for the render queue
//! DEBUG: Handle использует только Clone (без Copy) для избежания конфликтов

use crate::graphics::mesh::Mesh;
use crate::graphics::texture::Texture;
use crate::graphics::material::Material;
use crate::graphics::particles::ParticleSystem;
use nalgebra::{Matrix4, Vector3};

/// Проблема 12: Z-depth константы для UI элементов
/// Используются для сортировки UI при отрисовке (0.0 = ближний, 1.0 = дальний)
pub const UI_DEPTH_BACKGROUND: f32 = 0.7;
pub const UI_DEPTH_HUD: f32 = 0.8;
pub const UI_DEPTH_PROMPT: f32 = 0.85;
pub const UI_DEPTH_NOTIFICATIONS: f32 = 0.9;
pub const UI_DEPTH_TOOLTIP: f32 = 0.95;
pub const UI_DEPTH_CURSOR: f32 = 1.0;

/// Unique handle for resources
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Handle<T>(u64, std::marker::PhantomData<T>);

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        Self(self.0, std::marker::PhantomData)
    }
}

impl<T> Handle<T> {
    pub fn new(id: u64) -> Self {
        Self(id, std::marker::PhantomData)
    }

    pub fn id(&self) -> u64 {
        self.0
    }

    pub fn null() -> Self {
        Self(0, std::marker::PhantomData)
    }

    pub fn is_null(&self) -> bool {
        self.0 == 0
    }
}

/// Render command types for the render queue
#[derive(Debug, Clone)]
pub enum RenderCommand {
    /// Render a mesh with material
    Mesh {
        mesh: Handle<Mesh>,
        material: Handle<Material>,
        transform: Matrix4<f32>,
        sort_key: u64,
    },
    /// Render particle system
    ParticleSystem {
        system: Handle<ParticleSystem>,
        transform: Matrix4<f32>,
        sort_key: u64,
    },
    /// UI drawing command
    UIDraw {
        texture: Handle<Texture>,
        position: Vector3<f32>,
        size: Vector3<f32>,
        color: [f32; 4],
        sort_key: u64,
    },
    /// UI element with rect (for legacy compatibility)
    UIElement {
        rect: [f32; 4],
        texture: Option<Handle<Texture>>,
        color: [f32; 4],
        depth: f32,
        sort_key: u64,
    },
    /// UI text rendering
    UIText {
        text: String,
        position: [f32; 2],
        font_size: f32,
        color: [f32; 4],
        depth: f32,
        sort_key: u64,
    },
    /// Debug line drawing
    DebugLine {
        start: Vector3<f32>,
        end: Vector3<f32>,
        color: [f32; 4],
        sort_key: u64,
    },
    /// Debug lines batch (legacy compatibility)
    DebugLines {
        lines: Vec<([f32; 3], [f32; 3], [f32; 4])>,
        sort_key: u64,
    },
    /// Skybox rendering
    Skybox {
        texture: Handle<Texture>,
        rotation: Matrix4<f32>,
        sort_key: u64,
    },
    /// Terrain chunk rendering
    TerrainChunk {
        chunk_id: u64,
        mesh: Handle<Mesh>,
        material: Handle<Material>,
        transform: Matrix4<f32>,
        lod_level: u32,
        sort_key: u64,
    },
    /// Vehicle rendering
    Vehicle {
        position: Vector3<f32>,
        rotation: Matrix4<f32>,
        color: [f32; 4],
        sort_key: u64,
    },
    /// Clear screen
    Clear {
        color: Option<[f32; 4]>,
        depth: bool,
        stencil: bool,
        sort_key: u64,
    },
    /// Set viewport
    Viewport {
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        sort_key: u64,
    },
    /// Mesh deformation for damage visualization
    MeshDeformation {
        mesh: Handle<Mesh>,
        vertex_displacements: Vec<(usize, Vector3<f32>)>,
        sort_key: u64,
    },
}

impl RenderCommand {
    /// Get the sort key for this command
    pub fn sort_key(&self) -> u64 {
        match self {
            RenderCommand::Mesh { sort_key, .. } => *sort_key,
            RenderCommand::ParticleSystem { sort_key, .. } => *sort_key,
            RenderCommand::UIDraw { sort_key, .. } => *sort_key,
            RenderCommand::UIElement { sort_key, .. } => *sort_key,
            RenderCommand::UIText { sort_key, .. } => *sort_key,
            RenderCommand::DebugLine { sort_key, .. } => *sort_key,
            RenderCommand::DebugLines { sort_key, .. } => *sort_key,
            RenderCommand::Skybox { sort_key, .. } => *sort_key,
            RenderCommand::TerrainChunk { sort_key, .. } => *sort_key,
            RenderCommand::Vehicle { sort_key, .. } => *sort_key,
            RenderCommand::Clear { sort_key, .. } => *sort_key,
            RenderCommand::Viewport { sort_key, .. } => *sort_key,
            RenderCommand::MeshDeformation { sort_key, .. } => *sort_key,
        }
    }

    /// Set the sort key for this command
    pub fn set_sort_key(&mut self, key: u64) {
        match self {
            RenderCommand::Mesh { sort_key, .. } => *sort_key = key,
            RenderCommand::ParticleSystem { sort_key, .. } => *sort_key = key,
            RenderCommand::UIDraw { sort_key, .. } => *sort_key = key,
            RenderCommand::UIElement { sort_key, .. } => *sort_key = key,
            RenderCommand::UIText { sort_key, .. } => *sort_key = key,
            RenderCommand::DebugLine { sort_key, .. } => *sort_key = key,
            RenderCommand::DebugLines { sort_key, .. } => *sort_key = key,
            RenderCommand::Skybox { sort_key, .. } => *sort_key = key,
            RenderCommand::TerrainChunk { sort_key, .. } => *sort_key = key,
            RenderCommand::Vehicle { sort_key, .. } => *sort_key = key,
            RenderCommand::Clear { sort_key, .. } => *sort_key = key,
            RenderCommand::Viewport { sort_key, .. } => *sort_key = key,
            RenderCommand::MeshDeformation { sort_key, .. } => *sort_key = key,
        }
    }

    /// Get the material handle if applicable
    pub fn material_handle(&self) -> Option<Handle<Material>> {
        match self {
            RenderCommand::Mesh { material, .. } => Some(material.clone()),
            RenderCommand::TerrainChunk { material, .. } => Some(material.clone()),
            _ => None,
        }
    }

    /// Get the transform matrix if applicable
    pub fn transform(&self) -> Option<&Matrix4<f32>> {
        match self {
            RenderCommand::Mesh { transform, .. } => Some(transform),
            RenderCommand::ParticleSystem { transform, .. } => Some(transform),
            RenderCommand::Skybox { rotation, .. } => Some(rotation),
            RenderCommand::TerrainChunk { transform, .. } => Some(transform),
            RenderCommand::Vehicle { rotation, .. } => Some(rotation),
            _ => None,
        }
    }
}

/// Builder for creating render commands
pub struct RenderCommandBuilder {
    command_type: CommandType,
}

#[derive(Debug, Clone)]
enum CommandType {
    Mesh,
    ParticleSystem,
    UIDraw,
    DebugLine,
    Skybox,
    TerrainChunk,
    Vehicle,
}

impl RenderCommandBuilder {
    pub fn mesh() -> Self {
        Self {
            command_type: CommandType::Mesh,
        }
    }

    pub fn particle_system() -> Self {
        Self {
            command_type: CommandType::ParticleSystem,
        }
    }

    pub fn ui_draw() -> Self {
        Self {
            command_type: CommandType::UIDraw,
        }
    }

    pub fn debug_line() -> Self {
        Self {
            command_type: CommandType::DebugLine,
        }
    }

    pub fn skybox() -> Self {
        Self {
            command_type: CommandType::Skybox,
        }
    }

    pub fn terrain_chunk() -> Self {
        Self {
            command_type: CommandType::TerrainChunk,
        }
    }

    pub fn vehicle() -> Self {
        Self {
            command_type: CommandType::Vehicle,
        }
    }

    /// Build a mesh render command
    pub fn build_mesh(
        self,
        mesh: Handle<Mesh>,
        material: Handle<Material>,
        transform: Matrix4<f32>,
        sort_key: u64,
    ) -> RenderCommand {
        RenderCommand::Mesh {
            mesh,
            material,
            transform,
            sort_key,
        }
    }

    /// Build a particle system render command
    pub fn build_particle_system(
        self,
        system: Handle<ParticleSystem>,
        transform: Matrix4<f32>,
        sort_key: u64,
    ) -> RenderCommand {
        RenderCommand::ParticleSystem {
            system,
            transform,
            sort_key,
        }
    }

    /// Build a UI draw command
    pub fn build_ui_draw(
        self,
        texture: Handle<Texture>,
        position: Vector3<f32>,
        size: Vector3<f32>,
        color: [f32; 4],
        sort_key: u64,
    ) -> RenderCommand {
        RenderCommand::UIDraw {
            texture,
            position,
            size,
            color,
            sort_key,
        }
    }

    /// Build a debug line command
    pub fn build_debug_line(
        self,
        start: Vector3<f32>,
        end: Vector3<f32>,
        color: [f32; 4],
        sort_key: u64,
    ) -> RenderCommand {
        RenderCommand::DebugLine {
            start,
            end,
            color,
            sort_key,
        }
    }

    /// Build a skybox command
    pub fn build_skybox(
        self,
        texture: Handle<Texture>,
        rotation: Matrix4<f32>,
        sort_key: u64,
    ) -> RenderCommand {
        RenderCommand::Skybox {
            texture,
            rotation,
            sort_key,
        }
    }

    /// Build a terrain chunk command
    pub fn build_terrain_chunk(
        self,
        chunk_id: u64,
        mesh: Handle<Mesh>,
        material: Handle<Material>,
        transform: Matrix4<f32>,
        lod_level: u32,
        sort_key: u64,
    ) -> RenderCommand {
        RenderCommand::TerrainChunk {
            chunk_id,
            mesh,
            material,
            transform,
            lod_level,
            sort_key,
        }
    }

    /// Build a vehicle render command
    pub fn build_vehicle(
        self,
        position: Vector3<f32>,
        rotation: Matrix4<f32>,
        color: [f32; 4],
        sort_key: u64,
    ) -> RenderCommand {
        RenderCommand::Vehicle {
            position,
            rotation,
            color,
            sort_key,
        }
    }
}
