//! Render queue for batching and sorting render commands
//! TODO: Implement proper sorting and batching logic

use crate::graphics::render_command::RenderCommand;
use crate::graphics::renderer::commands::UiCommand;

/// Queue for render commands
pub struct RenderQueue {
    pub commands: Vec<RenderCommand>,
    pub ui_commands: Vec<UiCommand>,
}

impl RenderQueue {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            ui_commands: Vec::new(),
        }
    }

    pub fn add_command(&mut self, command: RenderCommand) {
        self.commands.push(command);
    }

    pub fn add_ui_command(&mut self, command: UiCommand) {
        self.ui_commands.push(command);
    }

    pub fn clear(&mut self) {
        self.commands.clear();
        self.ui_commands.clear();
    }

    pub fn get_commands(&self) -> &[RenderCommand] {
        &self.commands
    }

    pub fn get_ui_commands(&self) -> &[UiCommand] {
        &self.ui_commands
    }
}

impl Default for RenderQueue {
    fn default() -> Self {
        Self::new()
    }
}