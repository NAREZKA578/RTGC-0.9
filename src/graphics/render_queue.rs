//! Render Queue - Manages and sorts render commands for efficient rendering

use crate::graphics::material::Material;
use crate::graphics::render_command::RenderCommand;
use std::collections::HashMap;

/// Render queue for batching and sorting draw calls
#[derive(Clone)]
pub struct RenderQueue {
    /// All pending render commands
    commands: Vec<RenderCommand>,
    /// Commands sorted by material/shader for batch rendering
    sorted_commands: Vec<RenderCommand>,
    /// Statistics for the current frame
    stats: RenderQueueStats,
}

/// Statistics about the render queue
#[derive(Debug, Clone, Default)]
pub struct RenderQueueStats {
    pub total_commands: usize,
    pub mesh_commands: usize,
    pub particle_commands: usize,
    pub ui_commands: usize,
    pub debug_commands: usize,
    pub skybox_commands: usize,
    pub terrain_commands: usize,
    pub material_batches: usize,
    pub shader_batches: usize,
}

impl RenderQueue {
    pub fn new() -> Self {
        Self {
            commands: Vec::with_capacity(1024),
            sorted_commands: Vec::with_capacity(1024),
            stats: RenderQueueStats::default(),
        }
    }

    /// Submit a render command to the queue
    pub fn submit(&mut self, command: RenderCommand) {
        // Count command types for stats
        match &command {
            RenderCommand::Mesh { .. } => self.stats.mesh_commands += 1,
            RenderCommand::ParticleSystem { .. } => self.stats.particle_commands += 1,
            RenderCommand::UIDraw { .. } | RenderCommand::UIElement { .. } => {
                self.stats.ui_commands += 1
            }
            RenderCommand::DebugLine { .. } | RenderCommand::DebugLines { .. } => {
                self.stats.debug_commands += 1
            }
            RenderCommand::Skybox { .. } => self.stats.skybox_commands += 1,
            RenderCommand::TerrainChunk { .. } => self.stats.terrain_commands += 1,
            RenderCommand::Vehicle { .. } => self.stats.mesh_commands += 1, // Count vehicle as mesh
            RenderCommand::Clear { .. } | RenderCommand::Viewport { .. } => {} // Control commands not counted
        }

        self.commands.push(command);
        self.stats.total_commands += 1;
    }

    /// Sort commands by material/shader for batched rendering
    pub fn sort(&mut self) {
        // Clear previous sorted list
        self.sorted_commands.clear();
        self.sorted_commands.reserve(self.commands.len());

        // Copy commands
        self.sorted_commands.extend_from_slice(&self.commands);

        // Sort by material handle first, then by other criteria
        self.sorted_commands.sort_by(|a, b| {
            let key_a = Self::compute_sort_key(a);
            let key_b = Self::compute_sort_key(b);
            key_a.cmp(&key_b)
        });

        // Count material batches
        self.stats.material_batches = self.count_material_batches();
        self.stats.shader_batches = self.count_shader_batches();
    }

    /// Compute a sort key for a render command
    fn compute_sort_key(command: &RenderCommand) -> u64 {
        // Priority order: Clear/Viewport -> Skybox -> Terrain -> Vehicle -> Mesh -> Particles -> UI -> Debug
        let priority = match command {
            RenderCommand::Clear { .. } | RenderCommand::Viewport { .. } => 0u64,
            RenderCommand::Skybox { .. } => 1,
            RenderCommand::TerrainChunk { .. } => 2,
            RenderCommand::Vehicle { .. } => 3,
            RenderCommand::Mesh { .. } => 4,
            RenderCommand::ParticleSystem { .. } => 5,
            RenderCommand::UIDraw { .. } | RenderCommand::UIElement { .. } => 6,
            RenderCommand::DebugLine { .. } | RenderCommand::DebugLines { .. } => 7,
        };

        // Get material ID for batching (commands without materials get 0)
        let material_id = command.material_handle().map(|h| h.id()).unwrap_or(0);

        // Combine priority and material ID into a single sort key
        // Priority in high bits, material ID in low bits
        (priority << 32) | (material_id & 0xFFFFFFFF)
    }

    /// Count the number of material batches after sorting
    fn count_material_batches(&self) -> usize {
        if self.sorted_commands.is_empty() {
            return 0;
        }

        let mut batches = 1;
        let mut last_material = self.sorted_commands[0].material_handle();

        for command in &self.sorted_commands[1..] {
            let current_material = command.material_handle();
            if current_material != last_material {
                batches += 1;
                last_material = current_material;
            }
        }

        batches
    }

    /// Count the number of shader batches (simplified - assumes 1 shader per material)
    fn count_shader_batches(&self) -> usize {
        // For now, assume same as material batches
        // In a real implementation, this would track shader IDs separately
        self.stats.material_batches
    }

    /// Get all sorted commands for rendering
    pub fn commands(&self) -> &[RenderCommand] {
        &self.sorted_commands
    }

    /// Get mutable access to commands
    pub fn commands_mut(&mut self) -> &mut [RenderCommand] {
        &mut self.sorted_commands
    }

    /// Clear the render queue
    pub fn clear(&mut self) {
        self.commands.clear();
        self.sorted_commands.clear();
        self.stats = RenderQueueStats::default();
    }

    /// Get statistics for the current frame
    pub fn stats(&self) -> &RenderQueueStats {
        &self.stats
    }

    /// Check if the queue is empty
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// Get the number of commands
    pub fn len(&self) -> usize {
        self.commands.len()
    }
}

impl Default for RenderQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nalgebra::{Matrix4, Vector3};

    #[test]
    fn test_render_queue_submit() {
        let mut queue = RenderQueue::new();

        let cmd = RenderCommand::DebugLine {
            start: Vector3::new(0.0, 0.0, 0.0),
            end: Vector3::new(1.0, 1.0, 1.0),
            color: [1.0, 0.0, 0.0, 1.0],
            sort_key: 0,
        };

        queue.submit(cmd);
        assert_eq!(queue.len(), 1);
        assert_eq!(queue.stats().total_commands, 1);
        assert_eq!(queue.stats().debug_commands, 1);
    }

    #[test]
    fn test_render_queue_sort() {
        let mut queue = RenderQueue::new();

        // Add commands in random order
        for i in 0..5 {
            let cmd = RenderCommand::DebugLine {
                start: Vector3::new(i as f32, 0.0, 0.0),
                end: Vector3::new(i as f32 + 1.0, 0.0, 0.0),
                color: [1.0, 0.0, 0.0, 1.0],
                sort_key: i,
            };
            queue.submit(cmd);
        }

        queue.sort();
        assert_eq!(queue.commands().len(), 5);
    }

    #[test]
    fn test_render_queue_clear() {
        let mut queue = RenderQueue::new();

        let cmd = RenderCommand::DebugLine {
            start: Vector3::new(0.0, 0.0, 0.0),
            end: Vector3::new(1.0, 1.0, 1.0),
            color: [1.0, 0.0, 0.0, 1.0],
            sort_key: 0,
        };

        queue.submit(cmd);
        queue.clear();

        assert!(queue.is_empty());
        assert_eq!(queue.stats().total_commands, 0);
    }
}
