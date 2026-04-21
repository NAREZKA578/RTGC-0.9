//! Pause Scene

use super::super::scene::{Scene, SceneType};
use std::any::Any;

pub struct PauseScene {
    name: String,
}

impl PauseScene {
    pub fn new() -> Self {
        Self {
            name: "Pause Menu".to_string(),
        }
    }
}

impl Default for PauseScene {
    fn default() -> Self {
        Self::new()
    }
}

impl Scene for PauseScene {
    fn scene_type(&self) -> SceneType {
        SceneType::Pause
    }

    fn on_enter(&mut self) {
        tracing::info!("Entering Pause Menu");
    }

    fn on_exit(&mut self) {
        tracing::info!("Exiting Pause Menu");
    }

    fn update(&mut self, _delta_time: f32) {
        // Handle pause menu input
    }

    fn render(
        &mut self,
        renderer: &mut crate::graphics::renderer::Renderer,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Render pause menu overlay
        let w = renderer.width as f32;
        let h = renderer.height as f32;

        unsafe {
            // Semi-transparent overlay
            renderer.draw_rect(0.0, 0.0, w, h, [0.0, 0.0, 0.0, 0.6]);

            // Pause menu panel
            let panel_width = 300.0;
            let panel_height = 200.0;
            let panel_x = w / 2.0 - panel_width / 2.0;
            let panel_y = h / 2.0 - panel_height / 2.0;
            renderer.draw_rect(
                panel_x,
                panel_y,
                panel_width,
                panel_height,
                [0.1, 0.1, 0.15, 0.95],
            );

            // Title
            renderer.draw_text(
                "ПАУЗА",
                w / 2.0 - 40.0,
                panel_y + 40.0,
                1.5,
                [1.0, 1.0, 1.0, 1.0],
            );

            // Menu options
            renderer.draw_text(
                "Нажмите ESC для продолжения",
                w / 2.0 - 130.0,
                panel_y + 100.0,
                0.9,
                [0.8, 0.8, 0.8, 1.0],
            );
            renderer.draw_text(
                "M - Главное меню",
                w / 2.0 - 80.0,
                panel_y + 130.0,
                0.9,
                [0.7, 0.7, 0.7, 1.0],
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
