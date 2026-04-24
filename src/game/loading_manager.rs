//! Loading manager stub module
//! TODO: Implement proper loading progress tracking

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LoadingStage {
    Initializing,
    LoadingAssets,
    BuildingWorld,
    PreparingScene,
    Complete,
}

#[derive(Debug, Clone, Copy)]
pub struct LoadingStateDetailed {
    pub stage: LoadingStage,
    pub progress: f32,
    pub current_resource: Option<String>,
    pub sub_progress: f32,
}

impl Default for LoadingStateDetailed {
    fn default() -> Self {
        Self {
            stage: LoadingStage::Initializing,
            progress: 0.0,
            current_resource: None,
            sub_progress: 0.0,
        }
    }
}

pub struct LoadingManager;

impl LoadingManager {
    pub fn new() -> Self {
        Self
    }

    pub fn set_stage(&mut self, stage: LoadingStage) {
        let _ = stage;
    }

    pub fn set_progress(&mut self, progress: f32) {
        let _ = progress;
    }

    pub fn get_state(&self) -> LoadingStateDetailed {
        LoadingStateDetailed::default()
    }
}

impl Default for LoadingManager {
    fn default() -> Self {
        Self::new()
    }
}