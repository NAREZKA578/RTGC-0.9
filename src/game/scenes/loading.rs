//! Loading Scene

use super::super::scene::{Scene, SceneType, TransitionEffect};
use std::any::Any;

pub struct LoadingScene {
    name: String,
    progress: f32,
    loading_complete: bool,
}

impl LoadingScene {
    pub fn new() -> Self {
        Self {
            name: "Loading".to_string(),
            progress: 0.0,
            loading_complete: false,
        }
    }

    pub fn set_progress(&mut self, progress: f32) {
        self.progress = progress.clamp(0.0, 1.0);
        if self.progress >= 1.0 {
            self.loading_complete = true;
        }
    }

    pub fn is_loading_complete(&self) -> bool {
        self.loading_complete
    }
}

impl Default for LoadingScene {
    fn default() -> Self {
        Self::new()
    }
}

impl Scene for LoadingScene {
    fn scene_type(&self) -> SceneType {
        SceneType::Loading
    }

    fn on_enter(&mut self) {
        tracing::info!("Entering Loading Screen");
        self.progress = 0.0;
        self.loading_complete = false;
    }

    fn on_exit(&mut self) {
        tracing::info!("Exiting Loading Screen");
    }

    fn update(&mut self, delta_time: f32) {
        // Auto-increment progress if loading tasks are running
        // This ensures the loading screen progresses even without external set_progress() calls
        if !self.loading_complete && self.progress < 1.0 {
            // Simulate loading progress: ~2 seconds to full load at 60fps
            let progress_increment = delta_time * 0.5;
            self.progress = (self.progress + progress_increment).min(1.0);
            
            if self.progress >= 1.0 {
                self.loading_complete = true;
                tracing::info!("Loading complete");
            }
        }
    }

    fn render(
        &mut self,
        renderer: &mut crate::graphics::renderer::Renderer,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Render loading screen with progress bar
        let w = renderer.width as f32;
        let h = renderer.height as f32;

        unsafe {
            // Background
            renderer.draw_rect(0.0, 0.0, w, h, [0.0, 0.0, 0.0, 1.0]);

            // Progress bar background
            let bar_width = 400.0;
            let bar_height = 30.0;
            let bar_x = w / 2.0 - bar_width / 2.0;
            let bar_y = h / 2.0;
            renderer.draw_rect(bar_x, bar_y, bar_width, bar_height, [0.2, 0.2, 0.2, 1.0]);

            // Progress bar fill
            let fill_width = bar_width * self.progress;
            if fill_width > 0.0 {
                renderer.draw_rect(
                    bar_x + 2.0,
                    bar_y + 2.0,
                    fill_width - 4.0,
                    bar_height - 4.0,
                    [0.0, 0.8, 0.0, 1.0],
                );
            }

            // Progress text
            let progress_pct = (self.progress * 100.0) as i32;
            let text = format!("ЗАГРУЗКА... {}%", progress_pct);
            renderer.draw_text(
                &text,
                w / 2.0 - 80.0,
                bar_y - 40.0,
                1.2,
                [1.0, 1.0, 1.0, 1.0],
            );
        }

        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
