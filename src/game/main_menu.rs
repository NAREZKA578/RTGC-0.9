//! Main Menu System for RTGC-0.8
//! Handles main menu, new game, continue, options, exit

use crate::game::character_creation::CharacterCreationManager;
use crate::game::save::{SaveMetadata, SaveSystem};
use chrono::Local;
use serde_json;
use std::path::PathBuf;

/// Main menu states
#[derive(Debug, Clone, PartialEq)]
pub enum MenuState {
    /// Main menu (New Game, Continue, Options, Exit)
    MainMenu,
    /// Character creation in progress
    CharacterCreation,
    /// Loading screen
    Loading,
    /// In-game menu (paused)
    Paused,
}

/// Main menu manager
pub struct MainMenu {
    state: MenuState,
    hovered_button: Option<MenuButton>,
    character_creation: Option<CharacterCreationManager>,
    saves: Vec<SaveMetadata>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MenuButton {
    NewGame,
    Continue,
    Options,
    Exit,
    Resume,
    SaveGame,
    LoadGame,
    Settings,
    Back,
}

impl MainMenu {
    pub fn new() -> Self {
        let mut menu = Self {
            state: MenuState::MainMenu,
            hovered_button: None,
            character_creation: None,
            saves: Vec::new(),
        };
        menu.load_saves();
        menu
    }

    /// Load save metadata for "Continue" button
    fn load_saves(&mut self) {
        self.saves.clear();
        
        let save_dir = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("saves");

        if save_dir.exists() {
            // Read metadata files (.meta) instead of save files
            if let Ok(entries) = std::fs::read_dir(&save_dir) {
                let mut meta_files: Vec<_> = entries
                    .flatten()
                    .filter(|e| e.path().extension().map_or(false, |ext| ext == "meta"))
                    .collect();
                
                // Sort by filename to get consistent slot ordering
                meta_files.sort_by_key(|e| e.path());
                
                for entry in meta_files {
                    if let Ok(content) = std::fs::read_to_string(entry.path()) {
                        if let Ok(meta) = serde_json::from_str::<SaveMetadata>(&content) {
                            self.saves.push(meta);
                        }
                    }
                }
            }
        }
    }

    /// Get current menu state
    pub fn state(&self) -> &MenuState {
        &self.state
    }

    /// Start new game - initialize character creation
    pub fn start_new_game(&mut self) {
        self.state = MenuState::CharacterCreation;
        self.character_creation = Some(CharacterCreationManager::new());
    }

    /// Get mutable reference to character creation manager
    pub fn character_creation_mut(&mut self) -> Option<&mut CharacterCreationManager> {
        self.character_creation.as_mut()
    }

    /// Check if character creation is complete
    pub fn is_character_creation_complete(&self) -> bool {
        self.character_creation
            .as_ref()
            .map_or(false, |cc| cc.is_complete())
    }

    /// Get character creation data if complete
    pub fn get_character_data(
        &self,
    ) -> Option<&crate::game::character_creation::CharacterCreationData> {
        self.character_creation
            .as_ref()
            .and_then(|cc| cc.get_final_data())
    }

    /// Continue game - load most recent save
    pub fn continue_game(&mut self) -> Option<PathBuf> {
        if self.saves.is_empty() {
            return None;
        }

        // Load most recent save
        let save_dir = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("saves");

        // Find most recent save file
        let mut latest_save: Option<(PathBuf, std::time::SystemTime)> = None;

        if save_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&save_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map_or(false, |ext| ext == "json") {
                        if let Ok(meta) = entry.metadata() {
                            if let Ok(modified) = meta.modified() {
                                match &latest_save {
                                    None => latest_save = Some((path, modified)),
                                    Some((_, latest_time)) => {
                                        if modified > *latest_time {
                                            latest_save = Some((path, modified));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        latest_save.map(|(path, _)| path)
    }

    /// Handle button hover
    pub fn hover_button(&mut self, button: MenuButton) {
        self.hovered_button = Some(button);
    }

    /// Handle button click
    pub fn click_button(&mut self, button: MenuButton) -> MenuAction {
        match button {
            MenuButton::NewGame => {
                self.start_new_game();
                MenuAction::None
            }
            MenuButton::Continue => {
                if let Some(path) = self.continue_game() {
                    MenuAction::LoadGame(path)
                } else {
                    MenuAction::None
                }
            }
            MenuButton::Exit => MenuAction::Exit,
            MenuButton::Resume => MenuAction::Resume,
            MenuButton::SaveGame => MenuAction::SaveGame,
            MenuButton::LoadGame => MenuAction::OpenLoadMenu,
            MenuButton::Options | MenuButton::Settings => MenuAction::OpenSettings,
            MenuButton::Back => {
                self.state = MenuState::MainMenu;
                MenuAction::None
            }
        }
    }

    /// Update menu (handle character creation progress)
    pub fn update(&mut self, dt: f32) {
        if let Some(cc) = &mut self.character_creation {
            cc.update(dt);

            // If character creation is complete, transition to loading
            if cc.is_complete() {
                self.state = MenuState::Loading;
            }
        }
    }

    /// Render menu UI (placeholder - actual rendering in renderer)
    pub fn render(&self) -> MenuRenderData {
        match self.state {
            MenuState::MainMenu => MenuRenderData::MainMenu {
                hovered: self.hovered_button,
                has_saves: !self.saves.is_empty(),
            },
            MenuState::CharacterCreation => MenuRenderData::CharacterCreation,
            MenuState::Loading => MenuRenderData::Loading,
            MenuState::Paused => MenuRenderData::Paused {
                hovered: self.hovered_button,
            },
        }
    }

    /// Render UI elements directly through renderer
    pub fn render_ui(&self, renderer: &mut crate::graphics::renderer::Renderer) {
        let w = renderer.width as f32;
        let h = renderer.height as f32;

        // ТЕСТ: Рисуем красный прямоугольник на весь экран для проверки
        tracing::info!("[MainMenu] render_ui called: {}x{}", w, h);

        unsafe {
            // Полный экран красный - тест
            renderer.draw_rect(0.0, 0.0, w, h, [1.0, 0.0, 0.0, 1.0]);
            tracing::info!("[MainMenu] Drew RED fullscreen rect");
        }

        match self.state {
            MenuState::MainMenu => {
                tracing::info!("[MainMenu] Rendering main menu");

                // Центральная панель
                unsafe {
                    renderer.draw_rect(
                        w / 2.0 - 150.0,
                        h / 2.0 - 120.0,
                        300.0,
                        240.0,
                        [0.1, 0.1, 0.15, 0.9],
                    );
                }

                let button_width = 240.0;
                let button_height = 40.0;
                let center_x = w / 2.0;
                let mouse_x = renderer.mouse_x;
                let mouse_y = renderer.mouse_y;

                // Функция для проверки hover
                let is_hovered = |mouse_x: f32, mouse_y: f32, y: f32| -> bool {
                    mouse_x >= center_x - button_width / 2.0
                        && mouse_x <= center_x + button_width / 2.0
                        && mouse_y >= y
                        && mouse_y <= y + button_height
                };

                // Пункты меню с hover-эффектами
                let buttons = [
                    (
                        MenuButton::NewGame,
                        "НОВАЯ ИГРА",
                        h / 2.0 - 80.0,
                        [0.0, 0.8, 0.0, 1.0],
                        [0.0, 1.0, 0.0, 1.0],
                    ),
                    (
                        MenuButton::Continue,
                        "ПРОДОЛЖИТЬ",
                        h / 2.0 - 30.0,
                        [0.0, 0.3, 0.8, 1.0],
                        [0.0, 0.5, 1.0, 1.0],
                    ),
                    (
                        MenuButton::Options,
                        "НАСТРОЙКИ",
                        h / 2.0 + 20.0,
                        [0.5, 0.5, 0.5, 1.0],
                        [0.7, 0.7, 0.7, 1.0],
                    ),
                    (
                        MenuButton::Exit,
                        "ВЫХОД",
                        h / 2.0 + 70.0,
                        [0.8, 0.0, 0.0, 1.0],
                        [1.0, 0.0, 0.0, 1.0],
                    ),
                ];

                for (btn, text, y, color_normal, color_hover) in buttons.iter() {
                    let hovered = is_hovered(mouse_x, mouse_y, *y);
                    let color = if hovered { *color_hover } else { *color_normal };

                    unsafe {
                        renderer.draw_rect(
                            w / 2.0 - button_width / 2.0,
                            *y,
                            button_width,
                            button_height,
                            color,
                        );
                        renderer.draw_text(
                            text,
                            w / 2.0 - 60.0,
                            *y + 12.0,
                            1.0,
                            [1.0, 1.0, 1.0, 1.0],
                        );
                    }
                }
                tracing::info!("[MainMenu] Menu rendered, {} buttons", buttons.len());
            }
            MenuState::CharacterCreation => {
                tracing::info!("[MainMenu] Rendering character creation");
                // Character creation UI handled separately
                if let Some(cc) = &self.character_creation {
                    cc.render_ui(renderer);
                }
            }
            MenuState::Loading => {
                tracing::info!("[MainMenu] Rendering loading screen");
                // Loading screen
                unsafe {
                    renderer.draw_rect(0.0, 0.0, w, h, [0.0, 0.0, 0.0, 1.0]);
                    renderer.draw_text(
                        "ЗАГРУЗКА...",
                        w / 2.0 - 80.0,
                        h / 2.0,
                        1.5,
                        [1.0, 1.0, 1.0, 1.0],
                    );
                }
            }
            MenuState::Paused => {
                tracing::info!("[MainMenu] Rendering pause menu");
                // Pause menu
                unsafe {
                    renderer.draw_rect(
                        w / 2.0 - 150.0,
                        h / 2.0 - 100.0,
                        300.0,
                        200.0,
                        [0.1, 0.1, 0.15, 0.95],
                    );
                    renderer.draw_text(
                        "ПАУЗА",
                        w / 2.0 - 40.0,
                        h / 2.0 - 60.0,
                        1.2,
                        [1.0, 1.0, 1.0, 1.0],
                    );
                }
            }
        }
    }
}

/// Actions to perform based on menu interaction
#[derive(Debug)]
pub enum MenuAction {
    None,
    Exit,
    Resume,
    LoadGame(PathBuf),
    SaveGame,
    OpenLoadMenu,
    OpenSettings,
    StartGame,
}

/// Data for rendering the menu
#[derive(Debug)]
pub enum MenuRenderData {
    MainMenu {
        hovered: Option<MenuButton>,
        has_saves: bool,
    },
    CharacterCreation,
    Loading,
    Paused {
        hovered: Option<MenuButton>,
    },
}

impl Default for MainMenu {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_main_menu_creation() {
        let menu = MainMenu::new();
        assert_eq!(menu.state(), &MenuState::MainMenu);
    }

    #[test]
    fn test_start_new_game() {
        let mut menu = MainMenu::new();
        menu.start_new_game();
        assert_eq!(menu.state(), &MenuState::CharacterCreation);
        assert!(menu.character_creation.is_some());
    }
}
