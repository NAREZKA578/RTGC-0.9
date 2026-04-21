use nalgebra::{Vector3, Matrix4, UnitQuaternion, Quaternion, Point3};

/// Validates camera state to prevent NaN/Inf propagation in rendering
fn validate_camera_state(position: &Vector3<f32>, target: &Vector3<f32>, up: &Vector3<f32>) -> bool {
    position.x.is_finite() && position.y.is_finite() && position.z.is_finite()
        && target.x.is_finite() && target.y.is_finite() && target.z.is_finite()
        && up.x.is_finite() && up.y.is_finite() && up.z.is_finite()
}

#[derive(Debug, Clone)]
pub enum CameraType {
    FirstPerson,
    ThirdPerson,
}

#[derive(Debug, Clone)]
pub struct Camera {
    pub position: Vector3<f32>,
    pub target: Vector3<f32>,
    pub up: Vector3<f32>,
    pub fov: f32,
    pub aspect_ratio: f32,
    pub near: f32,
    pub far: f32,
    pub camera_type: CameraType,
    pub offset: Vector3<f32>, // Offset for third person view
    pub rotation: UnitQuaternion<f32>,
}

impl Camera {
    /// Creates a new camera with validation of input parameters
    pub fn new(
        position: Vector3<f32>,
        target: Vector3<f32>,
        up: Vector3<f32>,
        fov: f32,
        aspect_ratio: f32,
        near: f32,
        far: f32,
    ) -> Self {
        // Validate inputs to prevent NaN/Inf propagation
        if !validate_camera_state(&position, &target, &up) {
            tracing::warn!(target: "graphics", "Invalid camera state detected during creation, using defaults");
            return Self::default_safe();
        }
        
        // Validate numeric parameters
        let fov = if fov.is_finite() && fov > 0.0 && fov < 180.0 { fov } else { 60.0 };
        let aspect_ratio = if aspect_ratio.is_finite() && aspect_ratio > 0.0 { aspect_ratio } else { 16.0 / 9.0 };
        let near = if near.is_finite() && near > 0.0 { near } else { 0.1 };
        let far = if far.is_finite() && far > near { far } else { 1000.0 };
        
        Self {
            position,
            target,
            up,
            fov,
            aspect_ratio,
            near,
            far,
            camera_type: CameraType::ThirdPerson,
            offset: Vector3::new(0.0, 2.0, -5.0), // Default offset for third person
            rotation: UnitQuaternion::identity(),
        }
    }
    
    /// Creates a safe default camera for recovery from invalid states
    fn default_safe() -> Self {
        Self {
            position: Vector3::new(0.0, 2.0, 5.0),
            target: Vector3::new(0.0, 0.0, 0.0),
            up: Vector3::new(0.0, 1.0, 0.0),
            fov: 60.0,
            aspect_ratio: 16.0 / 9.0,
            near: 0.1,
            far: 1000.0,
            camera_type: CameraType::ThirdPerson,
            offset: Vector3::new(0.0, 2.0, -5.0),
            rotation: UnitQuaternion::identity(),
        }
    }
    
    /// Validates current camera state and resets to safe defaults if invalid
    pub fn validate_and_fix(&mut self) -> bool {
        if !validate_camera_state(&self.position, &self.target, &self.up) {
            tracing::warn!(target: "graphics", "Camera state invalid, resetting to safe defaults");
            *self = Self::default_safe();
            return false;
        }
        
        // Validate numeric parameters
        if !self.fov.is_finite() || self.fov <= 0.0 || self.fov >= 180.0 {
            tracing::warn!(target: "graphics", "Invalid FOV {}, resetting to 60.0", self.fov);
            self.fov = 60.0;
        }
        
        if !self.aspect_ratio.is_finite() || self.aspect_ratio <= 0.0 {
            tracing::warn!(target: "graphics", "Invalid aspect ratio {}, resetting to 16:9", self.aspect_ratio);
            self.aspect_ratio = 16.0 / 9.0;
        }
        
        if !self.near.is_finite() || self.near <= 0.0 {
            tracing::warn!(target: "graphics", "Invalid near plane {}, resetting to 0.1", self.near);
            self.near = 0.1;
        }
        
        if !self.far.is_finite() || self.far <= self.near {
            tracing::warn!(target: "graphics", "Invalid far plane {}, resetting to 1000.0", self.far);
            self.far = 1000.0;
        }
        
        true
    }

    pub fn new_with_rotation(
        position: Vector3<f32>,
        rotation: UnitQuaternion<f32>,
        fov: f32,
        aspect_ratio: f32,
        near: f32,
        far: f32,
    ) -> Self {
        // Validate position to prevent NaN/Inf propagation
        if !position.x.is_finite() || !position.y.is_finite() || !position.z.is_finite() {
            tracing::warn!(target: "graphics", "Invalid position in camera::new_with_rotation, using safe default");
            return Self::default_safe();
        }
        
        let forward = rotation.transform_vector(&Vector3::new(0.0, 0.0, 1.0));
        let target = position + forward;

        Self {
            position,
            target,
            up: Vector3::new(0.0, 1.0, 0.0),
            fov: if fov.is_finite() && fov > 0.0 && fov < 180.0 { fov } else { 60.0 },
            aspect_ratio: if aspect_ratio.is_finite() && aspect_ratio > 0.0 { aspect_ratio } else { 16.0 / 9.0 },
            near: if near.is_finite() && near > 0.0 { near } else { 0.1 },
            far: if far.is_finite() && far > near { far } else { 1000.0 },
            camera_type: CameraType::FirstPerson,
            offset: Vector3::new(0.0, 0.0, 0.0),
            rotation,
        }
    }

    pub fn switch_to_first_person(&mut self, truck_position: Vector3<f32>, truck_rotation: UnitQuaternion<f32>) {
        self.camera_type = CameraType::FirstPerson;
        // Validate truck position to prevent NaN/Inf propagation
        if !truck_position.x.is_finite() || !truck_position.y.is_finite() || !truck_position.z.is_finite() {
            tracing::warn!(target: "graphics", "Invalid truck position in switch_to_first_person, skipping update");
            return;
        }
        // Position camera at truck's position with slight height adjustment
        self.position = truck_position + Vector3::new(0.0, 1.5, 0.0);
        // Set camera to look in the same direction as the truck
        self.rotation = truck_rotation;
        let forward = self.rotation.transform_vector(&Vector3::new(0.0, 0.0, 1.0));
        self.target = self.position + forward;
    }

    pub fn switch_to_third_person(&mut self, truck_position: Vector3<f32>, truck_rotation: UnitQuaternion<f32>) {
        self.camera_type = CameraType::ThirdPerson;
        // Validate truck position to prevent NaN/Inf propagation
        if !truck_position.x.is_finite() || !truck_position.y.is_finite() || !truck_position.z.is_finite() {
            tracing::warn!(target: "graphics", "Invalid truck position in switch_to_third_person, skipping update");
            return;
        }
        // Position camera behind and above the truck
        let backward = truck_rotation.transform_vector(&Vector3::new(0.0, 0.0, -1.0));
        let offset = Vector3::new(0.0, 2.0, -5.0); // Standard offset
        self.position = truck_position + backward * offset.z + Vector3::new(0.0, offset.y, 0.0);
        self.target = truck_position;
        self.rotation = truck_rotation;
    }

    pub fn update_for_truck(&mut self, truck_position: Vector3<f32>, truck_rotation: UnitQuaternion<f32>) {
        // Validate truck position to prevent NaN/Inf propagation
        if !truck_position.x.is_finite() || !truck_position.y.is_finite() || !truck_position.z.is_finite() {
            tracing::warn!(target: "graphics", "Invalid truck position in update_for_truck, skipping update");
            return;
        }
        match self.camera_type {
            CameraType::FirstPerson => {
                self.position = truck_position + Vector3::new(0.0, 1.5, 0.0);
                self.rotation = truck_rotation;
                let forward = self.rotation.transform_vector(&Vector3::new(0.0, 0.0, 1.0));
                self.target = self.position + forward;
            }
            CameraType::ThirdPerson => {
                let backward = truck_rotation.transform_vector(&Vector3::new(0.0, 0.0, -1.0));
                let offset = Vector3::new(0.0, 2.0, -5.0);
                self.position = truck_position + backward * offset.z + Vector3::new(0.0, offset.y, 0.0);
                self.target = truck_position;
            }
        }
    }

    pub fn view_matrix(&self) -> Matrix4<f32> {
        // Validate state before computing view matrix
        let mut camera = self.clone();
        camera.validate_and_fix();
        let pos = Point3::from(camera.position);
        let tgt = Point3::from(camera.target);
        Matrix4::look_at_rh(&pos, &tgt, &camera.up)
    }

    pub fn projection_matrix(&self) -> Matrix4<f32> {
        // Validate parameters before computing projection matrix
        let fov = if self.fov.is_finite() && self.fov > 0.0 && self.fov < 180.0 { self.fov } else { 60.0 };
        let aspect_ratio = if self.aspect_ratio.is_finite() && self.aspect_ratio > 0.0 { self.aspect_ratio } else { 16.0 / 9.0 };
        let near = if self.near.is_finite() && self.near > 0.0 { self.near } else { 0.1 };
        let far = if self.far.is_finite() && self.far > near { self.far } else { 1000.0 };
        Matrix4::new_perspective(aspect_ratio, fov, near, far)
    }

    pub fn view_projection_matrix(&self) -> Matrix4<f32> {
        self.projection_matrix() * self.view_matrix()
    }

    pub fn get_direction(&self) -> Vector3<f32> {
        let dir = self.target - self.position;
        // Prevent division by zero or NaN propagation in normalize
        if dir.x.is_finite() && dir.y.is_finite() && dir.z.is_finite() && dir.norm_squared() > 0.0 {
            dir.normalize()
        } else {
            tracing::warn!(target: "graphics", "Invalid camera direction, returning default forward vector");
            Vector3::new(0.0, 0.0, 1.0)
        }
    }

    pub fn update_aspect_ratio(&mut self, aspect_ratio: f32) {
        if aspect_ratio.is_finite() && aspect_ratio > 0.0 {
            self.aspect_ratio = aspect_ratio;
        } else {
            tracing::warn!(target: "graphics", "Invalid aspect ratio {}, ignoring update", aspect_ratio);
        }
    }
}