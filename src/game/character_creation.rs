//! Character Creation System - Full character customization screen
//! Implements gender, height, skin color, face, hair, education, vehicle color, and starting location

use crate::game::economy::PlayerWallet;
use crate::game::player::Player;
use crate::game::save::PlayerSkillsData;
use crate::game::skills::SkillType;
use crate::game::InventoryItem;
use nalgebra::Vector3;

/// Character creation state machine
#[derive(Debug, Clone, PartialEq)]
pub enum CreationStep {
    /// Step 1: Gender selection
    Gender,
    /// Step 2: Height adjustment (1.50 - 2.10 m)
    Height,
    /// Step 3: Skin color selection
    SkinColor,
    /// Step 4: Face variant selection
    Face,
    /// Step 5: Hair style selection
    HairStyle,
    /// Step 6: Hair color selection
    HairColor,
    /// Step 7: Education selection (university/college)
    Education,
    /// Step 8: Vehicle color selection (UAZ Patriot)
    VehicleColor,
    /// Step 9: Starting location in Novosibirsk
    StartingLocation,
    /// Step 10: Summary screen
    Summary,
    /// Character creation complete
    Complete,
}

impl Default for CreationStep {
    fn default() -> Self {
        CreationStep::Gender
    }
}

/// Gender enum (only 2 options as per plan)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Gender {
    Male,
    Female,
}

impl Default for Gender {
    fn default() -> Self {
        Gender::Male
    }
}

impl std::fmt::Display for Gender {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Gender::Male => write!(f, "Мужской"),
            Gender::Female => write!(f, "Женский"),
        }
    }
}

/// Skin tone preset (realistic palette)
#[derive(Debug, Clone, Copy)]
pub struct SkinTone {
    pub name: &'static str,
    pub rgb: [f32; 3],
}

pub const SKIN_TONES: &[SkinTone] = &[
    SkinTone {
        name: "Очень светлый",
        rgb: [0.95, 0.85, 0.80],
    },
    SkinTone {
        name: "Светлый",
        rgb: [0.88, 0.75, 0.68],
    },
    SkinTone {
        name: "Средний светлый",
        rgb: [0.80, 0.65, 0.55],
    },
    SkinTone {
        name: "Средний",
        rgb: [0.72, 0.58, 0.48],
    },
    SkinTone {
        name: "Средний тёмный",
        rgb: [0.62, 0.48, 0.38],
    },
    SkinTone {
        name: "Тёмный",
        rgb: [0.50, 0.38, 0.28],
    },
    SkinTone {
        name: "Очень тёмный",
        rgb: [0.35, 0.25, 0.18],
    },
    SkinTone {
        name: "Глубокий тёмный",
        rgb: [0.22, 0.15, 0.10],
    },
];

/// Hair color preset
#[derive(Debug, Clone, Copy)]
pub struct HairColor {
    pub name: &'static str,
    pub rgb: [f32; 3],
}

pub const HAIR_COLORS: &[HairColor] = &[
    HairColor {
        name: "Чёрный",
        rgb: [0.08, 0.06, 0.05],
    },
    HairColor {
        name: "Тёмно-коричневый",
        rgb: [0.25, 0.18, 0.12],
    },
    HairColor {
        name: "Коричневый",
        rgb: [0.38, 0.28, 0.18],
    },
    HairColor {
        name: "Светло-коричневый",
        rgb: [0.52, 0.40, 0.28],
    },
    HairColor {
        name: "Блонд",
        rgb: [0.75, 0.65, 0.45],
    },
    HairColor {
        name: "Золотистый блонд",
        rgb: [0.85, 0.72, 0.45],
    },
    HairColor {
        name: "Рыжий",
        rgb: [0.70, 0.35, 0.18],
    },
    HairColor {
        name: "Тёмно-рыжий",
        rgb: [0.55, 0.25, 0.12],
    },
    HairColor {
        name: "Красный",
        rgb: [0.65, 0.20, 0.15],
    },
    HairColor {
        name: "Седой",
        rgb: [0.75, 0.75, 0.75],
    },
    HairColor {
        name: "Белый",
        rgb: [0.92, 0.92, 0.92],
    },
];

/// Hair style indices (0-7 for now)
pub const HAIR_STYLES: usize = 8;

/// Face variant indices (0-5 for now)
pub const FACE_VARIANTS: usize = 6;

/// Education option from universities.toml
#[derive(Debug, Clone)]
pub struct EducationOption {
    pub university_id: String,
    pub university_name: String,
    pub specialty_id: String,
    pub specialty_name: String,
    pub skills: Vec<(SkillType, u8, f32)>, // (type, rank, mastery)
    pub starting_capital_rub: f64,
    pub contacts: Vec<String>,
}

/// UAZ Patriot color option
#[derive(Debug, Clone)]
pub struct VehicleColorOption {
    pub name: &'static str,
    pub rgb: [f32; 3],
}

pub const UAZ_COLORS: &[VehicleColorOption] = &[
    VehicleColorOption {
        name: "Arctic White",
        rgb: [0.95, 0.95, 0.95],
    },
    VehicleColorOption {
        name: "Silver Metallic",
        rgb: [0.75, 0.75, 0.78],
    },
    VehicleColorOption {
        name: "Dark Gray",
        rgb: [0.35, 0.35, 0.38],
    },
    VehicleColorOption {
        name: "Black",
        rgb: [0.12, 0.12, 0.15],
    },
    VehicleColorOption {
        name: "Red",
        rgb: [0.65, 0.15, 0.15],
    },
    VehicleColorOption {
        name: "Blue",
        rgb: [0.15, 0.25, 0.55],
    },
    VehicleColorOption {
        name: "Green",
        rgb: [0.20, 0.35, 0.20],
    },
    VehicleColorOption {
        name: "Beige",
        rgb: [0.75, 0.68, 0.55],
    },
    VehicleColorOption {
        name: "Brown",
        rgb: [0.40, 0.28, 0.18],
    },
    VehicleColorOption {
        name: "Orange",
        rgb: [0.80, 0.45, 0.15],
    },
    VehicleColorOption {
        name: "Yellow",
        rgb: [0.90, 0.80, 0.20],
    },
    VehicleColorOption {
        name: "Khaki",
        rgb: [0.55, 0.52, 0.40],
    },
];

/// Starting location in Novosibirsk
#[derive(Debug, Clone)]
pub struct StartLocation {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub position: Vector3<f32>,
}

pub const START_LOCATIONS: &[StartLocation] = &[
    StartLocation {
        id: "ploskad_lenina",
        name: "Площадь Ленина",
        description: "Центр города, рядом с метро",
        position: Vector3::new(0.0, 0.0, 0.0),
    },
    StartLocation {
        id: "ngtu",
        name: "НГТУ",
        description: "Новосибирский государственный технический университет",
        position: Vector3::new(2500.0, 0.0, 1500.0),
    },
    StartLocation {
        id: "nsu",
        name: "НГУ (Академгородок)",
        description: "Новосибирский государственный университет",
        position: Vector3::new(-8000.0, 0.0, -5000.0),
    },
    StartLocation {
        id: "railway_station",
        name: "Ж/Д Вокзал Новосибирск-Главный",
        description: "Крупный транспортный узел",
        position: Vector3::new(-500.0, 0.0, 800.0),
    },
    StartLocation {
        id: "airport_tolmachevo",
        name: "Аэропорт Толмачёво",
        description: "Международный аэропорт",
        position: Vector3::new(-15000.0, 0.0, 8000.0),
    },
    StartLocation {
        id: "left_bank",
        name: "Левый берег (Жилмассив)",
        description: "Спальный район на левом берегу Оби",
        position: Vector3::new(3000.0, 0.0, -2000.0),
    },
];

/// Character creation data (temporary, before finalizing)
pub struct CharacterCreationData {
    /// Player name
    pub name: String,
    /// Gender
    pub gender: Gender,
    /// Height in meters (1.50 - 2.10)
    pub height: f32,
    /// Skin color [r, g, b]
    pub skin_color: [f32; 3],
    /// Skin tone index
    pub skin_tone_index: usize,
    /// Face variant (0-5)
    pub face_variant: u8,
    /// Hair style (0-7)
    pub hair_style: u8,
    /// Hair color [r, g, b]
    pub hair_color: [f32; 3],
    /// Hair color index
    pub hair_color_index: usize,
    /// Selected education
    pub education: Option<EducationOption>,
    /// Selected vehicle color
    pub vehicle_color: VehicleColorOption,
    /// Selected starting location
    pub start_location: StartLocation,
}

impl Default for CharacterCreationData {
    fn default() -> Self {
        Self {
            name: String::from("Игрок"),
            gender: Gender::default(),
            height: 1.75,
            skin_color: SKIN_TONES[2].rgb,
            skin_tone_index: 2,
            face_variant: 0,
            hair_style: 0,
            hair_color: HAIR_COLORS[1].rgb,
            hair_color_index: 1,
            education: None,
            vehicle_color: UAZ_COLORS[0].clone(),
            start_location: START_LOCATIONS[0].clone(),
        }
    }
}

impl CharacterCreationData {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set gender
    pub fn set_gender(&mut self, gender: Gender) {
        self.gender = gender;
    }

    /// Set height (clamped to 1.50 - 2.10)
    pub fn set_height(&mut self, height: f32) {
        self.height = height.clamp(1.50, 2.10);
    }

    /// Adjust height by delta
    pub fn adjust_height(&mut self, delta: f32) {
        self.set_height(self.height + delta);
    }

    /// Set skin tone by index
    pub fn set_skin_tone(&mut self, index: usize) {
        let idx = index.min(SKIN_TONES.len() - 1);
        self.skin_tone_index = idx;
        self.skin_color = SKIN_TONES[idx].rgb;
    }

    /// Cycle skin tone
    pub fn cycle_skin_tone(&mut self, direction: i32) {
        let len = SKIN_TONES.len() as i32;
        let new_idx = ((self.skin_tone_index as i32) + direction).rem_euclid(len) as usize;
        self.set_skin_tone(new_idx);
    }

    /// Set face variant
    pub fn set_face(&mut self, variant: u8) {
        self.face_variant = variant % FACE_VARIANTS as u8;
    }

    /// Cycle face variant
    pub fn cycle_face(&mut self, direction: i32) {
        let new_val = ((self.face_variant as i32) + direction).rem_euclid(FACE_VARIANTS as i32);
        self.face_variant = new_val as u8;
    }

    /// Set hair style
    pub fn set_hair_style(&mut self, style: u8) {
        self.hair_style = style % HAIR_STYLES as u8;
    }

    /// Cycle hair style
    pub fn cycle_hair_style(&mut self, direction: i32) {
        let new_val = ((self.hair_style as i32) + direction).rem_euclid(HAIR_STYLES as i32);
        self.hair_style = new_val as u8;
    }

    /// Set hair color by index
    pub fn set_hair_color(&mut self, index: usize) {
        let idx = index.min(HAIR_COLORS.len() - 1);
        self.hair_color_index = idx;
        self.hair_color = HAIR_COLORS[idx].rgb;
    }

    /// Cycle hair color
    pub fn cycle_hair_color(&mut self, direction: i32) {
        let len = HAIR_COLORS.len() as i32;
        let new_idx = ((self.hair_color_index as i32) + direction).rem_euclid(len) as usize;
        self.set_hair_color(new_idx);
    }

    /// Set education
    pub fn set_education(&mut self, edu: EducationOption) {
        self.education = Some(edu);
    }

    /// Set vehicle color
    pub fn set_vehicle_color(&mut self, color: VehicleColorOption) {
        self.vehicle_color = color;
    }

    /// Set starting location
    pub fn set_start_location(&mut self, location: StartLocation) {
        self.start_location = location;
    }

    /// Build final Player from creation data
    pub fn build_player(&self) -> Player {
        let mut player = Player::new(self.name.clone());

        player.is_male = match self.gender {
            Gender::Male => true,
            Gender::Female => false,
        };

        player.height = self.height;
        player.skin_color = self.skin_color;
        player.face_variant = self.face_variant;
        player.hair_style = self.hair_style;
        player.hair_color = self.hair_color;

        // Apply education skills and starting capital
        if let Some(ref edu) = self.education {
            player.skills = PlayerSkillsData::from_education(&edu.specialty_id);
            player.money.rub = edu.starting_capital_rub;
        } else {
            // Default: basic driving and fitness, small capital
            player.skills.driving.rank = 2;
            player.skills.driving.mastery = 0.3;
            player.skills.fitness.rank = 2;
            player.skills.fitness.mastery = 0.3;
            player.money.rub = 15000.0;
        }

        player
    }

    /// Get summary string for final screen
    pub fn get_summary(&self) -> Vec<String> {
        let mut lines = Vec::new();

        lines.push(format!("Имя: {}", self.name));
        lines.push(format!("Пол: {}", self.gender));
        lines.push(format!("Рост: {:.2} м", self.height));
        lines.push(format!(
            "Цвет кожи: {}",
            SKIN_TONES[self.skin_tone_index].name
        ));
        lines.push(format!("Лицо: вариант #{}", self.face_variant + 1));
        lines.push(format!("Причёска: стиль #{}", self.hair_style + 1));
        lines.push(format!(
            "Цвет волос: {}",
            HAIR_COLORS[self.hair_color_index].name
        ));

        if let Some(ref edu) = self.education {
            lines.push(format!(
                "Образование: {} - {}",
                edu.university_name, edu.specialty_name
            ));
            lines.push(format!(
                "Стартовый капитал: {:.0} RUB",
                edu.starting_capital_rub
            ));
            lines.push(format!("Контакты: {}", edu.contacts.join(", ")));
        } else {
            lines.push("Образование: Среднее".to_string());
            lines.push("Стартовый капитал: 15000 RUB".to_string());
        }

        lines.push(format!("Цвет UAZ Patriot: {}", self.vehicle_color.name));
        lines.push(format!("Точка старта: {}", self.start_location.name));
        lines.push(format!("Описание: {}", self.start_location.description));

        lines
    }
}

/// Character creation manager
pub struct CharacterCreationManager {
    /// Current step
    pub current_step: CreationStep,
    /// Temporary character data
    pub data: CharacterCreationData,
    /// Available education options (loaded from toml)
    pub education_options: Vec<EducationOption>,
    /// Is character creation active
    pub is_active: bool,
}

impl CharacterCreationManager {
    pub fn new() -> Self {
        Self {
            current_step: CreationStep::Gender,
            data: CharacterCreationData::new(),
            education_options: Vec::new(),
            is_active: true,
        }
    }

    /// Load education options from toml file
    pub fn load_education_from_toml(&mut self, toml_content: &str) {
        self.education_options.clear();

        // Simple TOML parser for specialties
        // In production, use proper toml crate
        for line in toml_content.lines() {
            if line.contains("[[specialties]]") {
                // Parse specialty block
                // This is simplified - real implementation would use toml::from_str
            }
        }

        // Add default education options if parsing failed
        if self.education_options.is_empty() {
            self.education_options = vec![
                EducationOption {
                    university_id: "ngtu".to_string(),
                    university_name: "НГТУ".to_string(),
                    specialty_id: "automotive_engineering".to_string(),
                    specialty_name: "Автомобилестроение".to_string(),
                    skills: vec![
                        (SkillType::Mechanics, 4, 0.3),
                        (SkillType::Driving, 3, 0.5),
                        (SkillType::Electrics, 3, 0.2),
                    ],
                    starting_capital_rub: 50000.0,
                    contacts: vec!["auto_mechanic".to_string(), "car_dealer".to_string()],
                },
                EducationOption {
                    university_id: "nsu".to_string(),
                    university_name: "НГУ".to_string(),
                    specialty_id: "geology".to_string(),
                    specialty_name: "Геология".to_string(),
                    skills: vec![
                        (SkillType::Geology, 4, 0.5),
                        (SkillType::Drilling, 3, 0.3),
                        (SkillType::Mining, 2, 0.4),
                    ],
                    starting_capital_rub: 55000.0,
                    contacts: vec!["geologist".to_string(), "mining_company".to_string()],
                },
                EducationOption {
                    university_id: "ngueu".to_string(),
                    university_name: "НГУЭУ".to_string(),
                    specialty_id: "business".to_string(),
                    specialty_name: "Бизнес и менеджмент".to_string(),
                    skills: vec![
                        (SkillType::Business, 4, 0.3),
                        (SkillType::Logistics, 3, 0.4),
                        (SkillType::Trading, 3, 0.2),
                    ],
                    starting_capital_rub: 80000.0,
                    contacts: vec!["businessman".to_string(), "bank_manager".to_string()],
                },
                EducationOption {
                    university_id: "ngmu".to_string(),
                    university_name: "НГМУ".to_string(),
                    specialty_id: "medicine".to_string(),
                    specialty_name: "Лечебное дело".to_string(),
                    skills: vec![(SkillType::Medicine, 4, 0.5), (SkillType::Fitness, 2, 0.3)],
                    starting_capital_rub: 40000.0,
                    contacts: vec!["doctor".to_string(), "pharmacist".to_string()],
                },
                EducationOption {
                    university_id: "sibadi".to_string(),
                    university_name: "СибАДИ".to_string(),
                    specialty_id: "road_building".to_string(),
                    specialty_name: "Дорожное строительство".to_string(),
                    skills: vec![
                        (SkillType::RoadBuilding, 4, 0.4),
                        (SkillType::Construction, 3, 0.3),
                        (SkillType::Driving, 2, 0.5),
                    ],
                    starting_capital_rub: 40000.0,
                    contacts: vec![
                        "road_worker".to_string(),
                        "construction_company".to_string(),
                    ],
                },
                EducationOption {
                    university_id: "mstu_ga".to_string(),
                    university_name: "МГТУ ГА".to_string(),
                    specialty_id: "aviation".to_string(),
                    specialty_name: "Лётная эксплуатация".to_string(),
                    skills: vec![
                        (SkillType::Piloting, 4, 0.2),
                        (SkillType::Navigation, 3, 0.4),
                        (SkillType::Mechanics, 2, 0.3),
                    ],
                    starting_capital_rub: 70000.0,
                    contacts: vec!["pilot".to_string(), "airport".to_string()],
                },
            ];
        }
    }

    /// Go to next step
    pub fn next_step(&mut self) {
        use CreationStep::*;

        self.current_step = match self.current_step {
            Gender => Height,
            Height => SkinColor,
            SkinColor => Face,
            Face => HairStyle,
            HairStyle => HairColor,
            HairColor => Education,
            Education => VehicleColor,
            VehicleColor => StartingLocation,
            StartingLocation => Summary,
            Summary => Complete,
            Complete => Complete,
        };
    }

    /// Go to previous step
    pub fn prev_step(&mut self) {
        use CreationStep::*;

        self.current_step = match self.current_step {
            Gender => Gender,
            Height => Gender,
            SkinColor => Height,
            Face => SkinColor,
            HairStyle => Face,
            HairColor => HairStyle,
            Education => HairColor,
            VehicleColor => Education,
            StartingLocation => VehicleColor,
            Summary => StartingLocation,
            Complete => Summary,
        };
    }

    /// Check if can go back
    pub fn can_go_back(&self) -> bool {
        self.current_step != CreationStep::Gender
    }

    /// Check if can go forward
    pub fn can_go_forward(&self) -> bool {
        self.current_step != CreationStep::Complete
    }

    /// Check if character creation is complete
    pub fn is_complete(&self) -> bool {
        self.current_step == CreationStep::Complete
    }

    /// Finalize and create player
    pub fn finalize(&mut self) -> Option<Player> {
        if self.current_step == CreationStep::Summary {
            self.next_step(); // Move to Complete
            Some(self.data.build_player())
        } else {
            None
        }
    }

    /// Get current step name
    pub fn get_step_name(&self) -> &'static str {
        match self.current_step {
            CreationStep::Gender => "Выбор пола",
            CreationStep::Height => "Настройка роста",
            CreationStep::SkinColor => "Цвет кожи",
            CreationStep::Face => "Выбор лица",
            CreationStep::HairStyle => "Причёска",
            CreationStep::HairColor => "Цвет волос",
            CreationStep::Education => "Образование",
            CreationStep::VehicleColor => "Цвет автомобиля",
            CreationStep::StartingLocation => "Точка старта",
            CreationStep::Summary => "Итоговый экран",
            CreationStep::Complete => "Готово",
        }
    }

    /// Get step number (1-10)
    pub fn get_step_number(&self) -> usize {
        match self.current_step {
            CreationStep::Gender => 1,
            CreationStep::Height => 2,
            CreationStep::SkinColor => 3,
            CreationStep::Face => 4,
            CreationStep::HairStyle => 5,
            CreationStep::HairColor => 6,
            CreationStep::Education => 7,
            CreationStep::VehicleColor => 8,
            CreationStep::StartingLocation => 9,
            CreationStep::Summary => 10,
            CreationStep::Complete => 11,
        }
    }

    /// Get total steps
    pub fn get_total_steps(&self) -> usize {
        10
    }

    /// Get final character data for player creation
    pub fn get_final_data(&self) -> Option<&CharacterCreationData> {
        Some(&self.data)
    }

    /// Update character creation (placeholder for future async loading)
    pub fn update(&mut self, _dt: f32) {
        // Placeholder for future updates
    }

    /// Render character creation UI
    pub fn render_ui(&self, renderer: &mut crate::graphics::renderer::Renderer) {
        let w = renderer.width as f32;
        let h = renderer.height as f32;

        unsafe {
            // Background
            renderer.draw_rect(0.0, 0.0, w, h, [0.0, 0.0, 0.0, 0.8]);

            // Title
            let step_num = self.get_step_number();
            let total_steps = self.get_total_steps();
            let title = format!("СОЗДАНИЕ ПЕРСОНАЖА - ШАГ {}/{}", step_num, total_steps);
            renderer.draw_text(&title, w / 2.0 - 200.0, 50.0, 1.2, [1.0, 1.0, 1.0, 1.0]);

            // Current step info
            let step_info = match self.current_step {
                CreationStep::Gender => "Выберите пол: 1 - Мужской, 2 - Женский",
                CreationStep::Height => "Рост: стрелки вверх/вниз (1.50 - 2.10 м)",
                CreationStep::SkinColor => "Оттенок кожи: стрелки влево/вправо",
                CreationStep::Face => "Лицо: стрелки влево/вправо",
                CreationStep::HairStyle => "Причёска: стрелки влево/вправо",
                CreationStep::HairColor => "Цвет волос: стрелки влево/вправо",
                CreationStep::Education => "Выберите образование",
                CreationStep::VehicleColor => "Цвет автомобиля: стрелки влево/вправо",
                CreationStep::StartingLocation => "Место старта: стрелки влево/вправо",
                CreationStep::Summary => "Проверьте характеристики и нажмите Enter",
                CreationStep::Complete => "Создание завершено",
            };
            renderer.draw_text(
                step_info,
                w / 2.0 - 200.0,
                h / 2.0 - 50.0,
                1.0,
                [0.8, 0.8, 0.8, 1.0],
            );

            // Character preview info
            let info = format!(
                "Имя: {}\nПол: {:?}\nРост: {:.2} м\nКожа: {}\nВолосы: {}",
                self.data.name,
                self.data.gender,
                self.data.height,
                self.data.skin_tone_index,
                self.data.hair_color_index
            );
            renderer.draw_text(
                &info,
                w / 2.0 - 200.0,
                h / 2.0 + 20.0,
                0.9,
                [0.7, 0.7, 0.7, 1.0],
            );

            // Navigation hint
            renderer.draw_text(
                "Стрелки - выбор, Enter - далее, Esc - назад",
                w / 2.0 - 180.0,
                h - 80.0,
                0.8,
                [0.5, 0.5, 0.5, 1.0],
            );
        }
    }
}

impl Default for CharacterCreationManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_character_creation_flow() {
        let mut manager = CharacterCreationManager::new();

        assert_eq!(manager.get_step_number(), 1);
        assert_eq!(manager.current_step, CreationStep::Gender);

        // Test height clamping
        manager.data.adjust_height(1.0);
        assert_eq!(manager.data.height, 2.10);

        manager.data.adjust_height(-1.0);
        assert_eq!(manager.data.height, 1.50);

        // Test cycling
        manager.data.cycle_skin_tone(1);
        assert_eq!(manager.data.skin_tone_index, 3);

        manager.data.cycle_skin_tone(-10);
        assert_eq!(manager.data.skin_tone_index, 1);
    }

    #[test]
    fn test_build_player() {
        let mut manager = CharacterCreationManager::new();
        manager.data.name = "Test Player".to_string();
        manager.data.height = 1.85;
        manager
            .data
            .set_education(manager.education_options.first().unwrap().clone());

        let player = manager.data.build_player();

        assert_eq!(player.name, "Test Player");
        assert_eq!(player.height, 1.85);
        assert!(player.money.rub > 0.0);
    }
}
