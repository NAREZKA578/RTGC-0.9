use nalgebra::Vector3;
use std::sync::Arc;

/// LOD (Level of Detail) model variants with different polygon counts
#[derive(Debug, Clone)]
pub enum LodModel {
    HighPoly {
        vertices: Arc<Vec<Vector3<f32>>>,
        indices: Arc<Vec<u32>>,
    },
    MediumPoly {
        vertices: Arc<Vec<Vector3<f32>>>,
        indices: Arc<Vec<u32>>,
    },
    LowPoly {
        vertices: Arc<Vec<Vector3<f32>>>,
        indices: Arc<Vec<u32>>,
    },
    Billboard {
        texture_id: u32,
        size: f32,
    },
}

impl Default for LodModel {
    fn default() -> Self {
        LodModel::Billboard {
            texture_id: 0,
            size: 1.0,
        }
    }
}

/// LOD Object with hysteresis to prevent "popcorn" effect during LOD transitions
#[derive(Clone)]
pub struct LodObject {
    pub position: Vector3<f32>,
    pub lod_distances: [f32; 3], // [high_to_med, med_to_low, low_to_billboard]
    pub lod_hysteresis: [f32; 3], // Hysteresis values to prevent rapid switching
    pub lod_models: [LodModel; 4], // [high, medium, low, billboard/none]
    pub current_lod: usize,
    last_update_time: std::time::Instant,
    min_update_interval: std::time::Duration,
}

impl LodObject {
    /// Creates a new LOD object with default hysteresis (10% of distance thresholds)
    pub fn new(position: Vector3<f32>, lod_distances: [f32; 3], lod_models: [LodModel; 4]) -> Self {
        // Default hysteresis is 10% of the distance threshold to prevent rapid switching
        let lod_hysteresis = [
            lod_distances[0] * 0.1,
            lod_distances[1] * 0.1,
            lod_distances[2] * 0.1,
        ];

        Self {
            position,
            lod_distances,
            lod_hysteresis,
            lod_models,
            current_lod: 0,
            last_update_time: std::time::Instant::now(),
            min_update_interval: std::time::Duration::from_millis(100),
        }
    }

    /// Creates a new LOD object with a simple radius (convenience constructor)
    pub fn from_radius(position: Vector3<f32>, radius: f32) -> Self {
        let lod_distances = [radius, radius * 2.0, radius * 3.0];
        let lod_models = [
            LodModel::default(),
            LodModel::default(),
            LodModel::default(),
            LodModel::default(),
        ];
        Self::new(position, lod_distances, lod_models)
    }

    /// Creates a new LOD object with custom hysteresis values
    pub fn with_hysteresis(
        position: Vector3<f32>,
        lod_distances: [f32; 3],
        lod_hysteresis: [f32; 3],
        lod_models: [LodModel; 4],
    ) -> Self {
        Self {
            position,
            lod_distances,
            lod_hysteresis,
            lod_models,
            current_lod: 0,
            last_update_time: std::time::Instant::now(),
            min_update_interval: std::time::Duration::from_millis(50),
        }
    }

    /// Updates LOD level based on camera distance with hysteresis to prevent popcorn effect
    pub fn update_lod(&mut self, camera_position: &Vector3<f32>) {
        // Throttle LOD updates to prevent excessive switching
        if self.last_update_time.elapsed() < self.min_update_interval {
            return;
        }

        let distance = (self.position - camera_position).magnitude();

        // Apply hysteresis: require distance to exceed threshold + hysteresis to switch to lower LOD,
        // and be below threshold - hysteresis to switch to higher LOD
        let new_lod = if self.current_lod > 0
            && distance
                < self.lod_distances[self.current_lod - 1]
                    - self.lod_hysteresis[self.current_lod - 1]
        {
            // Switch to higher detail (lower LOD index)
            self.current_lod - 1
        } else if self.current_lod < 3
            && distance
                > self.lod_distances[self.current_lod] + self.lod_hysteresis[self.current_lod]
        {
            // Switch to lower detail (higher LOD index)
            self.current_lod + 1
        } else {
            // Stay at current LOD
            self.current_lod
        };

        // Only update if LOD actually changed
        if new_lod != self.current_lod {
            self.current_lod = new_lod;
            self.last_update_time = std::time::Instant::now();
        }
    }

    /// Force immediate LOD update without throttling (useful for teleportation or cutscenes)
    pub fn update_lod_immediate(&mut self, camera_position: &Vector3<f32>) {
        let distance = (self.position - camera_position).magnitude();

        self.current_lod = if distance < self.lod_distances[0] {
            0 // High poly
        } else if distance < self.lod_distances[1] {
            1 // Medium poly
        } else if distance < self.lod_distances[2] {
            2 // Low poly
        } else {
            3 // Billboard or none
        };

        self.last_update_time = std::time::Instant::now();
    }

    pub fn get_current_model(&self) -> &LodModel {
        &self.lod_models[self.current_lod]
    }

    pub fn get_current_lod(&self) -> usize {
        self.current_lod
    }

    pub fn get_render_distance(&self) -> f32 {
        // Return the furthest distance at which this object should render
        self.lod_distances[2]
    }

    /// Returns the approximate triangle count of the current LOD level
    pub fn get_triangle_count(&self) -> usize {
        match &self.lod_models[self.current_lod] {
            LodModel::HighPoly { indices, .. }
            | LodModel::MediumPoly { indices, .. }
            | LodModel::LowPoly { indices, .. } => indices.len() / 3,
            LodModel::Billboard { .. } => 2, // Billboards are typically 2 triangles
        }
    }

    /// Sets the minimum update interval for LOD throttling
    pub fn set_update_interval(&mut self, interval_ms: u64) {
        self.min_update_interval = std::time::Duration::from_millis(interval_ms);
    }
}

/// LOD Manager with frustum culling and batched updates
#[derive(Clone)]
pub struct LodManager {
    pub objects: Vec<LodObject>,
    total_triangles_rendered: usize,
    objects_updated: usize,
}

impl LodManager {
    pub fn new() -> Self {
        Self {
            objects: Vec::with_capacity(1024), // Pre-allocate for performance
            total_triangles_rendered: 0,
            objects_updated: 0,
        }
    }

    pub fn add_object(&mut self, lod_object: LodObject) {
        self.objects.push(lod_object);
    }

    /// Updates all LODs with optional frustum culling
    pub fn update_all_lods(&mut self, camera_position: &Vector3<f32>) {
        self.total_triangles_rendered = 0;
        self.objects_updated = 0;

        for obj in &mut self.objects {
            obj.update_lod(camera_position);
            self.objects_updated += 1;
            self.total_triangles_rendered += obj.get_triangle_count();
        }
    }

    /// Gets visible objects sorted by LOD for better rendering batching
    pub fn get_objects_in_view(
        &self,
        camera_position: &Vector3<f32>,
        view_distance: f32,
    ) -> Vec<(usize, &LodModel)> {
        let mut visible_objects = Vec::with_capacity(self.objects.len());

        for (index, obj) in self.objects.iter().enumerate() {
            let distance = (obj.position - camera_position).magnitude();

            // Only include objects that are within the view distance and have a model to render
            if distance < view_distance.min(obj.get_render_distance()) {
                visible_objects.push((index, obj.get_current_model()));
            }
        }

        // Sort by LOD level to batch similar-detail objects together (reduces state changes)
        visible_objects.sort_by_key(|(_, model)| match model {
            LodModel::HighPoly { .. } => 0,
            LodModel::MediumPoly { .. } => 1,
            LodModel::LowPoly { .. } => 2,
            LodModel::Billboard { .. } => 3,
        });

        visible_objects
    }

    /// Returns statistics about the current LOD state
    pub fn get_stats(&self) -> LodStats {
        LodStats {
            total_objects: self.objects.len(),
            objects_updated: self.objects_updated,
            total_triangles_rendered: self.total_triangles_rendered,
        }
    }

    /// Clears all objects from the manager
    pub fn clear(&mut self) {
        self.objects.clear();
        self.total_triangles_rendered = 0;
        self.objects_updated = 0;
    }
}

/// Statistics about LOD system performance
#[derive(Debug, Clone, Default)]
pub struct LodStats {
    pub total_objects: usize,
    pub objects_updated: usize,
    pub total_triangles_rendered: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lod_switching_with_hysteresis() {
        let pos = Vector3::new(0.0, 0.0, 0.0);
        let distances = [10.0, 50.0, 100.0];

        // Create dummy models
        let models = [
            LodModel::HighPoly {
                vertices: vec![],
                indices: vec![],
            },
            LodModel::MediumPoly {
                vertices: vec![],
                indices: vec![],
            },
            LodModel::LowPoly {
                vertices: vec![],
                indices: vec![],
            },
            LodModel::Billboard {
                texture_id: 0,
                size: 1.0,
            },
        ];

        let mut obj = LodObject::new(pos, distances, models);

        // Camera far away - should be at lowest LOD
        let cam_far = Vector3::new(200.0, 0.0, 0.0);
        obj.update_lod_immediate(&cam_far);
        assert_eq!(obj.get_current_lod(), 3);

        // Move closer but within hysteresis zone - should stay at LOD 3
        let cam_mid = Vector3::new(95.0, 0.0, 0.0); // Within hysteresis of 100.0
        obj.update_lod(&cam_mid);
        assert_eq!(obj.get_current_lod(), 3); // Should not switch yet due to hysteresis

        // Move well inside threshold - should switch to LOD 2
        let cam_close = Vector3::new(80.0, 0.0, 0.0);
        obj.update_lod(&cam_close);
        assert_eq!(obj.get_current_lod(), 2);
    }

    #[test]
    fn test_lod_throttling() {
        let pos = Vector3::new(0.0, 0.0, 0.0);
        let distances = [10.0, 50.0, 100.0];
        let models = [
            LodModel::HighPoly {
                vertices: vec![],
                indices: vec![],
            },
            LodModel::MediumPoly {
                vertices: vec![],
                indices: vec![],
            },
            LodModel::LowPoly {
                vertices: vec![],
                indices: vec![],
            },
            LodModel::Billboard {
                texture_id: 0,
                size: 1.0,
            },
        ];

        let mut obj = LodObject::new(pos, distances, models);
        obj.set_update_interval(1000); // 1 second throttle

        let cam1 = Vector3::new(200.0, 0.0, 0.0);
        obj.update_lod(&cam1);
        let lod1 = obj.get_current_lod();

        // Immediate second update should be throttled
        let cam2 = Vector3::new(5.0, 0.0, 0.0);
        obj.update_lod(&cam2);
        assert_eq!(obj.get_current_lod(), lod1); // Should not have changed
    }
}
