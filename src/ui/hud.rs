// ЧАСТЬ 1 — HUD: ЕДИНЫЙ ЦЕНТР ИНФОРМАЦИИ
// Весь HUD хранится в одном месте, управляется единым HudManager.

use crate::game::{InventoryItem, ItemType};
use nalgebra::{UnitQuaternion, Vector3};

/// Все данные для HUD — заполняются движком, рисуются HudManager
#[derive(Debug, Clone, Default)]
pub struct VehicleHudData {
    // === Блок ДВИЖЕНИЯ ===
    pub speed_kmh: f32,       // Текущая скорость км/ч
    pub speed_max_kmh: f32,   // Максимальная скорость (для шкалы)
    pub gear: GearState,      // Передача: Park, Rev, N, 1..8
    pub engine_rpm: f32,      // Текущие обороты
    pub engine_rpm_max: f32,  // Красная зона начинается отсюда
    pub engine_running: bool, // Двигатель запущен?

    // === Блок РЕСУРСОВ ===
    pub fuel_level: f32,          // 0.0..1.0
    pub fuel_reserve: bool,       // Резервный уровень (мигать)
    pub engine_temp: f32,         // °C, 0..120
    pub engine_overheating: bool, // Перегрев (мигать)

    // === Блок ТРАНСМИССИИ ===
    pub diff_front_locked: bool,  // Блокировка переднего диффа
    pub diff_rear_locked: bool,   // Блокировка заднего диффа
    pub diff_center_locked: bool, // Межосевая блокировка
    pub awd_active: bool,         // Полный привод активен
    pub low_range: bool,          // Понижающий ряд включён

    // === Блок ПОДВЕСКИ ===
    pub wheel_contact: [bool; 4],  // Какие колёса в контакте с землёй
    pub wheel_slip: [f32; 4],      // Проскальзывание 0..1 каждого колеса
    pub suspension_load: [f32; 4], // Нагрузка подвески 0..1

    // === Блок ГРУЗА ===
    pub cargo_attached: bool, // Груз прицеплен
    pub cargo_weight_kg: f32, // Масса груза
    pub cargo_damage: f32,    // Повреждение груза 0..1
    pub winch_active: bool,   // Лебёдка активна
    pub winch_length_m: f32,  // Длина троса лебёдки

    // === Блок ОКРУЖЕНИЯ ===
    pub altitude_m: f32,        // Высота над уровнем моря
    pub terrain_angle_deg: f32, // Угол наклона поверхности
    pub vehicle_roll_deg: f32,  // Крен машины (бок)
    pub vehicle_pitch_deg: f32, // Тангаж (нос/корма)
    pub is_tipped_over: bool,   // Машина перевёрнута?

    // === Блок ПОВРЕЖДЕНИЙ ===
    pub vehicle_health: f32, // 0.0..1.0

    // === Ф1.5: Компас ===
    pub heading_degrees: f32, // 0-360°, направление игрока/машины
    pub active_waypoint: Option<CompassWaypoint>, // Активная цель миссии
}

/// Waypoint для компаса — цель миссии
#[derive(Debug, Clone)]
pub struct CompassWaypoint {
    pub name: String,         // Название цели (например, "Бердск")
    pub heading_degrees: f32, // Направление к цели (0-360°)
    pub distance_meters: f32, // Дистанция до цели в метрах
}

impl Default for CompassWaypoint {
    fn default() -> Self {
        CompassWaypoint {
            name: String::new(),
            heading_degrees: 0.0,
            distance_meters: 0.0,
        }
    }
}

/// Waypoint для карты/мини-карты
pub type Waypoint = crate::game::ui::MapWaypoint;

#[derive(Debug, Clone, PartialEq)]
pub enum GearState {
    Park,
    Reverse,
    Neutral,
    Drive(u8), // 1..8
}

impl Default for GearState {
    fn default() -> Self {
        GearState::Neutral
    }
}

/// Конфигурация отображения HUD
#[derive(Debug, Clone)]
pub struct HudLayout {
    pub show_speed: bool,
    pub show_gear: bool,
    pub show_fuel: bool,
    pub show_diff_status: bool,
    pub show_wheel_status: bool,
    pub show_cargo: bool,
    pub show_terrain_angle: bool,
    pub compact_mode: bool, // Мини-версия для слабых экранов
    pub show_minimap: bool, // Правый блок с картой
    pub show_compass: bool, // Ф1.5: Компас вверху экрана
}

impl Default for HudLayout {
    fn default() -> Self {
        Self {
            show_speed: true,
            show_gear: true,
            show_fuel: true,
            show_diff_status: true,
            show_wheel_status: true,
            show_cargo: true,
            show_terrain_angle: true,
            compact_mode: false,
            show_minimap: true,
            show_compass: true, // Ф1.5: включен по умолчанию
        }
    }
}

/// Единый менеджер HUD — единственное место где рисуется интерфейс
#[derive(Clone)]
pub struct HudManager {
    visible: bool,
    opacity: f32,
    layout: HudLayout,
    last_data: Option<VehicleHudData>,
    // Анимационные состояния
    flash_timer: f32,
    flash_element: Option<HudFlashElement>,
    // Ф1.6: Инвентарь
    inventory_open: bool,
    // Настройки из SettingsManager
    settings_enabled: bool,
    settings_open: bool,
    // Карта и метки
    map_system: crate::game::MapSystem,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HudFlashElement {
    FuelReserve,
    EngineOverheat,
    WheelSlip(usize), // index 0..3
}

impl HudManager {
    pub fn new() -> Self {
        Self {
            visible: true,
            opacity: 1.0,
            layout: HudLayout::default(),
            last_data: None,
            flash_timer: 0.0,
            flash_element: None,
            inventory_open: false,  // Ф1.6: Инвентарь закрыт по умолчанию
            settings_enabled: true, // Настройки включены
            settings_open: false,   // Настройки закрыты по умолчанию
            map_system: crate::game::MapSystem::new(), // Инициализация карты
        }
    }

    /// Ф1.6: Переключить состояние инвентаря
    pub fn toggle_inventory(&mut self) {
        self.inventory_open = !self.inventory_open;
    }

    /// Ф1.6: Проверить, открыт ли инвентарь
    pub fn is_inventory_open(&self) -> bool {
        self.inventory_open
    }

    /// Ф1.6: Установить состояние инвентаря
    pub fn set_inventory_open(&mut self, open: bool) {
        self.inventory_open = open;
    }

    /// Настройки: Переключить состояние меню настроек
    pub fn toggle_settings(&mut self) {
        self.settings_open = !self.settings_open;
    }

    /// Настройки: Проверить, открыто ли меню настроек
    pub fn is_settings_open(&self) -> bool {
        self.settings_open
    }

    /// Настройки: Установить состояние меню настроек
    pub fn set_settings_open(&mut self, open: bool) {
        self.settings_open = open;
    }

    /// Настройки: Применить настройки к HUD
    pub fn apply_settings(&mut self, hud_settings: &crate::game::HudSettings) {
        self.layout.show_speed = hud_settings.show_speed;
        self.layout.show_gear = hud_settings.show_gear;
        self.layout.show_fuel = hud_settings.show_fuel;
        self.layout.show_diff_status = hud_settings.show_diff_status;
        self.layout.show_wheel_status = hud_settings.show_wheel_status;
        self.layout.show_cargo = hud_settings.show_cargo;
        self.layout.show_terrain_angle = hud_settings.show_terrain_angle;
        self.layout.show_minimap = hud_settings.show_minimap;
        self.layout.show_compass = hud_settings.show_compass;
        self.layout.compact_mode = hud_settings.compact_mode;
        self.opacity = hud_settings.hud_opacity;
        self.visible = hud_settings.hud_enabled;
    }

    /// Обновить данные HUD
    pub fn update(&mut self, data: VehicleHudData, dt: f32) {
        // Проверка на мигающие элементы
        if data.fuel_reserve {
            self.flash_element = Some(HudFlashElement::FuelReserve);
            self.flash_timer = 0.5; // мигать каждые 0.5 сек
        } else if data.engine_overheating {
            self.flash_element = Some(HudFlashElement::EngineOverheat);
            self.flash_timer = 0.3; // быстрее мигать для перегрева
        } else {
            // Проверка проскальзывания колёс
            let mut slipping_wheel = None;
            for (i, &slip) in data.wheel_slip.iter().enumerate() {
                if slip > 0.5 {
                    slipping_wheel = Some(i);
                    break;
                }
            }

            if let Some(idx) = slipping_wheel {
                self.flash_element = Some(HudFlashElement::WheelSlip(idx));
                self.flash_timer = 0.2;
            } else {
                self.flash_element = None;
            }
        }

        // Обновление таймера мигания
        if self.flash_timer > 0.0 {
            self.flash_timer -= dt;
            if self.flash_timer <= 0.0 {
                self.flash_timer = 0.0;
                self.flash_element = None;
            }
        }

        self.last_data = Some(data);
    }

    /// Показать/скрыть HUD
    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Установить прозрачность (0.0..1.0)
    pub fn set_opacity(&mut self, opacity: f32) {
        self.opacity = opacity.clamp(0.0, 1.0);
    }

    /// Получить текущие данные
    pub fn get_data(&self) -> Option<&VehicleHudData> {
        self.last_data.as_ref()
    }

    /// Получить конфигурацию отображения
    pub fn get_layout(&self) -> &HudLayout {
        &self.layout
    }

    /// Изменить конфигурацию отображения
    pub fn set_layout(&mut self, layout: HudLayout) {
        self.layout = layout;
    }

    /// Проверить, должен ли элемент мигать сейчас
    pub fn is_flashing(&self, element: &HudFlashElement) -> bool {
        if let Some(ref flash) = self.flash_element {
            if flash == element {
                // Мигать: включено половину времени
                return self.flash_timer > 0.25;
            }
        }
        false
    }

    /// Сгенерировать VehicleHudData из параметров автомобиля (helper)
    pub fn create_vehicle_data(
        speed_kmh: f32,
        rpm: f32,
        rpm_max: f32,
        gear: GearState,
        engine_running: bool,
        fuel: f32,
        temp: f32,
    ) -> VehicleHudData {
        VehicleHudData {
            speed_kmh,
            speed_max_kmh: 120.0, // default for trucks
            gear,
            engine_rpm: rpm,
            engine_rpm_max: rpm_max,
            engine_running,
            fuel_level: fuel,
            fuel_reserve: fuel < 0.15,
            engine_temp: temp,
            engine_overheating: temp > 100.0,
            ..Default::default()
        }
    }

    /// Рендеринг HUD через OpenGL
    /// Вызывается из renderer.rs после отрисовки игрового мира
    pub fn render(&self, renderer: &mut crate::graphics::renderer::Renderer) {
        // Ф1.6: Если инвентарь открыт, рисуем только его
        if self.inventory_open {
            self.render_inventory(renderer);
            return;
        }

        // Настройки: Если настройки открыты, рисуем меню настроек
        if self.settings_open {
            self.render_settings(renderer);
            return;
        }

        // Карта: Если карта открыта, рисуем её
        if self.map_system.map_open {
            self.render_map(renderer);
            return;
        }

        // Контекстное меню карты (поверх всего)
        if self.map_system.context_menu_open {
            self.render_map_context_menu(renderer);
        }

        if !self.visible || self.last_data.is_none() {
            return;
        }

        let data = match &self.last_data {
            Some(d) => d,
            None => return,
        };
        let layout = &self.layout;

        let screen_width = renderer.get_width() as f32;
        let screen_height = renderer.get_height() as f32;

        // === СТИЛЬ: МИНИМАЛИЗМ / ГРЯЗЬ / ХАРДКОР ===
        // Никаких спидометров и тахометров. Только критическая информация.

        // 1. ИНДИКАТОРЫ ДИФФЕРЕНЦИАЛОВ (Левый верхний угол)
        // Крупные буквы, горят ярко, когда включены
        let diff_font_size = 28.0;
        let start_x = 25.0;
        let start_y = 25.0;
        let gap = 45.0;

        // Передний дифф
        let f_color = if data.diff_front_locked {
            [1.0, 0.2, 0.2, 1.0]
        } else {
            [0.3, 0.3, 0.3, 0.4]
        };
        unsafe {
            renderer.draw_text("F", start_x, start_y, diff_font_size, f_color);
        }

        // Центральный дифф
        let c_color = if data.diff_center_locked {
            [1.0, 0.8, 0.0, 1.0]
        } else {
            [0.3, 0.3, 0.3, 0.4]
        };
        unsafe {
            renderer.draw_text("C", start_x + gap, start_y, diff_font_size, c_color);
        }

        // Задний дифф
        let r_color = if data.diff_rear_locked {
            [1.0, 0.2, 0.2, 1.0]
        } else {
            [0.3, 0.3, 0.3, 0.4]
        };
        unsafe {
            renderer.draw_text("R", start_x + gap * 2.0, start_y, diff_font_size, r_color);
        }

        // Пониженная передача
        if data.low_range {
            let low_color = [1.0, 0.6, 0.0, 1.0];
            unsafe {
                renderer.draw_text("LOW", start_x + gap * 0.5, start_y + 35.0, 18.0, low_color);
            }
        }

        // 2. СТАТУС ЛЕБЁДКИ (Правый верхний угол)
        if data.winch_active {
            let winch_x = screen_width - 180.0;
            let winch_y = 25.0;

            // Длина троса
            let rope_len = format!("{:.1}m", data.winch_length_m);
            unsafe {
                renderer.draw_text(&rope_len, winch_x, winch_y, 22.0, [1.0, 0.9, 0.5, 1.0]);
            }

            // Статус натяжения
            let tension_status = if data.winch_length_m > 0.5 {
                "TIGHT"
            } else {
                "LOOSE"
            };
            let tension_color = if data.winch_length_m > 0.5 {
                [1.0, 0.3, 0.3, 1.0]
            } else {
                [0.5, 0.5, 0.5, 0.8]
            };
            unsafe {
                renderer.draw_text(tension_status, winch_x, winch_y + 28.0, 16.0, tension_color);
            }

            // Рамка вокруг лебедки если активна
            unsafe {
                renderer.draw_rect(
                    winch_x - 8.0,
                    winch_y - 8.0,
                    130.0,
                    60.0,
                    [0.0, 0.0, 0.0, 0.4],
                );
            }
            unsafe {
                renderer.draw_rect_border(
                    winch_x - 8.0,
                    winch_y - 8.0,
                    130.0,
                    60.0,
                    2.0,
                    [0.6, 0.6, 0.6, 0.6],
                );
            }
        }

        // 3. КОЛЕСА И КОНТАКТ (Нижняя часть экрана, по центру)
        // 4 точки, показывающие загрузку колес.
        // Зеленая = контакт с землей, Красная = в воздухе (вывешено)
        if layout.show_wheel_status {
            let wheel_y = screen_height - 70.0;
            let wheel_spacing = 50.0;
            let total_w = wheel_spacing * 3.0;
            let start_wheel_x = (screen_width - total_w) / 2.0;

            for (i, &contact) in data.wheel_contact.iter().enumerate() {
                let x = start_wheel_x + (i as f32 * wheel_spacing);
                let color = if contact {
                    [0.0, 1.0, 0.3, 1.0]
                } else {
                    [1.0, 0.0, 0.0, 0.7]
                };

                // Основной индикатор контакта
                let size = 10.0;
                unsafe {
                    renderer.draw_rect(x, wheel_y, size, size, color);
                }

                // Если колесо в воздухе, добавляем вторую точку ниже (индикатор хода подвески)
                if !contact {
                    unsafe {
                        renderer.draw_rect(x, wheel_y + 16.0, size, size, [0.4, 0.4, 0.4, 0.6]);
                    }
                }

                // Мигание при сильном проскальзывании
                if data.wheel_slip.get(i).copied().unwrap_or(0.0) > 0.4 {
                    let slip_color = [1.0, 1.0, 0.0, 0.8];
                    unsafe {
                        renderer.draw_rect(
                            x + 2.0,
                            wheel_y + 2.0,
                            size - 4.0,
                            size - 4.0,
                            slip_color,
                        );
                    }
                }
            }
        }

        // 4. ПОДСКАЗКИ УПРАВЛЕНИЯ (Внизу по центру, полупрозрачные)
        let hints_y = screen_height - 40.0;
        let hint_color = [0.5, 0.5, 0.5, 0.7];
        let font_size = 13.0;

        let hint_text = "[WASD] Drive  [SHIFT] Winch  [B] Diff Locks  [ESC] Menu";
        let text_w = hint_text.len() as f32 * font_size * 0.55;
        let hint_x = (screen_width - text_w) / 2.0;

        unsafe {
            renderer.draw_text(hint_text, hint_x, hints_y, font_size, hint_color);
        }

        // 5. ИНДИКАТОР ПОВРЕЖДЕНИЙ (Оверлей по краям экрана)
        // Если здоровье машины < 100%, рисуем красную виньетку по краям
        if data.vehicle_health < 1.0 {
            let damage_factor = 1.0 - data.vehicle_health;
            let alpha = (damage_factor * 0.6).min(0.75);

            let border_size = 35.0 * (1.0 + damage_factor * 1.5);

            // Top
            unsafe {
                renderer.draw_rect(0.0, 0.0, screen_width, border_size, [1.0, 0.0, 0.0, alpha]);
            }
            // Bottom
            unsafe {
                renderer.draw_rect(
                    0.0,
                    screen_height - border_size,
                    screen_width,
                    border_size,
                    [1.0, 0.0, 0.0, alpha],
                );
            }
            // Left
            unsafe {
                renderer.draw_rect(0.0, 0.0, border_size, screen_height, [1.0, 0.0, 0.0, alpha]);
            }
            // Right
            unsafe {
                renderer.draw_rect(
                    screen_width - border_size,
                    0.0,
                    border_size,
                    screen_height,
                    [1.0, 0.0, 0.0, alpha],
                );
            }

            // Текст предупреждения если критично
            if data.vehicle_health < 0.25 {
                let warn_text = "CRITICAL DAMAGE";
                let warn_x = (screen_width - 180.0) / 2.0;
                let warn_y = screen_height / 2.0 - 80.0;
                unsafe {
                    renderer.draw_text(warn_text, warn_x, warn_y, 32.0, [1.0, 0.0, 0.0, 1.0]);
                }
            }
        }

        // 6. СТАТУС ГРУЗА (Левая сторона, ниже диффов)
        if layout.show_cargo && data.cargo_attached {
            let cargo_x = 25.0;
            let cargo_y = 120.0;

            let weight_text = format!("{:.0} kg", data.cargo_weight_kg);
            unsafe {
                renderer.draw_text(&weight_text, cargo_x, cargo_y, 20.0, [0.9, 0.9, 0.9, 0.9]);
            }

            // Повреждение груза
            if data.cargo_damage > 0.3 {
                let damage_color = if data.cargo_damage > 0.7 {
                    [1.0, 0.0, 0.0, 1.0]
                } else {
                    [1.0, 0.5, 0.0, 1.0]
                };
                unsafe {
                    renderer.draw_text("DAMAGED", cargo_x, cargo_y + 25.0, 16.0, damage_color);
                }
            }
        }

        // 7. Ф1.5 — КОМПАС В HUD (Верхняя часть экрана, по центру)
        // Полоска 400×24px, вращается по heading, показывает N/S/E/W + цифры текущего направления
        if layout.show_compass {
            let compass_width = 400.0;
            let compass_height = 24.0;
            let compass_x = (screen_width - compass_width) / 2.0;
            let compass_y = 15.0; // Чуть ниже самого верха

            // Фон компаса (полупрозрачный чёрный)
            unsafe {
                renderer.draw_rect(
                    compass_x,
                    compass_y,
                    compass_width,
                    compass_height,
                    [0.0, 0.0, 0.0, 0.5],
                );
            }
            unsafe {
                renderer.draw_rect_border(
                    compass_x,
                    compass_y,
                    compass_width,
                    compass_height,
                    2.0,
                    [0.6, 0.6, 0.6, 0.8],
                );
            }

            // Центральный маркер (треугольник вверх) - КРАСНЫЙ по запросу
            let center_x = screen_width / 2.0;
            let triangle_size = 8.0;
            let triangle_color = [1.0, 0.0, 0.0, 1.0]; // КРАСНЫЙ вместо жёлтого

            // Рисуем треугольник (центр сверху)
            unsafe {
                // Левая половина треугольника
                renderer.draw_line(
                    center_x - triangle_size / 2.0,
                    compass_y + compass_height,
                    center_x,
                    compass_y + compass_height - triangle_size,
                    2.0,
                    triangle_color,
                );
                // Правая половина треугольника
                renderer.draw_line(
                    center_x,
                    compass_y + compass_height - triangle_size,
                    center_x + triangle_size / 2.0,
                    compass_y + compass_height,
                    2.0,
                    triangle_color,
                );
            }

            // Вычисляем смещение шкалы компаса на основе heading
            // heading_degrees: 0=N, 90=E, 180=S, 270=W
            let heading = data.heading_degrees;
            let scale_pixels_per_degree = compass_width / 180.0; // 180° видимой области

            // Основные направления
            let directions = [
                (0.0, "N", [1.0, 1.0, 1.0, 1.0]),    // N - белый
                (45.0, "NE", [0.7, 0.7, 0.7, 0.8]),  // NE - серый
                (90.0, "E", [1.0, 1.0, 1.0, 1.0]),   // E - белый
                (135.0, "SE", [0.7, 0.7, 0.7, 0.8]), // SE - серый
                (180.0, "S", [1.0, 1.0, 1.0, 1.0]),  // S - белый
                (225.0, "SW", [0.7, 0.7, 0.7, 0.8]), // SW - серый
                (270.0, "W", [1.0, 1.0, 1.0, 1.0]),  // W - белый
                (315.0, "NW", [0.7, 0.7, 0.7, 0.8]), // NW - серый
            ];

            // Рисуем деления и буквы
            for (angle, label, color) in directions.iter() {
                // Вычисляем относительное положение относительно текущего heading
                let mut rel_angle = *angle - heading;

                // Нормализуем угол (-180 до +180)
                while rel_angle > 180.0 {
                    rel_angle -= 360.0;
                }
                while rel_angle < -180.0 {
                    rel_angle += 360.0;
                }

                // Если в пределах видимой области (±90° от центра)
                if rel_angle.abs() <= 90.0 {
                    let x = center_x + rel_angle * scale_pixels_per_degree;

                    // Делаем цвет тусклее если далеко от центра
                    let draw_color = if rel_angle.abs() > 60.0 {
                        [0.5, 0.5, 0.5, 0.5]
                    } else {
                        *color
                    };

                    let font_size = if rel_angle.abs() < 15.0 { 16.0 } else { 12.0 };
                    let text_y = compass_y + 4.0;
                    let text_x = x - (label.len() as f32 * font_size * 0.3);

                    unsafe {
                        renderer.draw_text(label, text_x, text_y, font_size, draw_color);
                    }
                }
            }

            // Цифры текущего направления (под компасом, маленькие)
            let current_heading = data.heading_degrees;
            let heading_text = format!("{:.0}°", current_heading);
            let heading_font_size = 12.0;
            let heading_text_width = heading_text.len() as f32 * heading_font_size * 0.6;
            let heading_x = center_x - heading_text_width / 2.0;
            let heading_y = compass_y + compass_height + 4.0;

            unsafe {
                renderer.draw_text(
                    &heading_text,
                    heading_x,
                    heading_y,
                    heading_font_size,
                    [1.0, 1.0, 0.0, 1.0],
                );
            }

            // Отображение меток на компасе (до 4 штук)
            // Метки показываются как маленькие треугольники над компасом
            for marker in self.map_system.get_compass_markers() {
                // Вычисляем направление к метке относительно игрока
                // Для простоты используем заглушку - в реальной игре нужно вычислять угол
                // между позицией игрока и позицией метки
                let marker_rel_angle = 30.0; // Заглушка: метка справа от игрока

                // Нормализуем угол относительно текущего heading
                let mut display_angle = marker_rel_angle - heading;
                while display_angle > 180.0 {
                    display_angle -= 360.0;
                }
                while display_angle < -180.0 {
                    display_angle += 360.0;
                }

                // Если метка в пределах видимой области компаса (±90°)
                if display_angle.abs() <= 90.0 {
                    let marker_x = center_x + display_angle * scale_pixels_per_degree;
                    let marker_y = compass_y - 8.0; // Над компасом

                    // Рисуем маленький треугольник цвета метки
                    let marker_color = marker.marker_type.color();
                    unsafe {
                        renderer.draw_triangle(
                            marker_x,
                            marker_y - 6.0,
                            marker_x - 5.0,
                            marker_y + 4.0,
                            marker_x + 5.0,
                            marker_y + 4.0,
                            marker_color,
                        );
                    }
                }
            }
        }
    }
    /// Ф1.6: Рендеринг инвентаря (Grid-based, Tarkov-style)
    /// Отображает сетку инвентаря с предметами, вес, свободные слоты
    pub fn render_inventory(&self, renderer: &mut crate::graphics::renderer::Renderer) {
        let screen_width = renderer.get_width() as f32;
        let screen_height = renderer.get_height() as f32;

        // Параметры сетки инвентаря
        let inv_width = 500.0;
        let inv_height = 400.0;
        let inv_x = (screen_width - inv_width) / 2.0;
        let inv_y = (screen_height - inv_height) / 2.0;

        // Фон инвентаря (тёмный полупрозрачный)
        unsafe {
            renderer.draw_rect(
                inv_x,
                inv_y,
                inv_width,
                inv_height,
                [0.05, 0.05, 0.08, 0.95],
            );
            renderer.draw_rect_border(
                inv_x,
                inv_y,
                inv_width,
                inv_height,
                3.0,
                [0.5, 0.5, 0.5, 0.8],
            );
        }

        // Заголовок
        let title = "INVENTORY";
        unsafe {
            renderer.draw_text(
                title,
                inv_x + 20.0,
                inv_y + 15.0,
                24.0,
                [0.9, 0.9, 0.9, 1.0],
            );
        }

        // Сетка инвентаря (10 колонок × 8 рядов = 80 слотов)
        let grid_start_x = inv_x + 20.0;
        let grid_start_y = inv_y + 80.0;
        let slot_size = 45.0;
        let gap = 2.0;
        let cols = 10;
        let rows = 8;

        // Рисуем сетку
        for row in 0..rows {
            for col in 0..cols {
                let x = grid_start_x + (col as f32 * (slot_size + gap));
                let y = grid_start_y + (row as f32 * (slot_size + gap));

                // Пустой слот - тёмная ячейка с рамкой
                unsafe {
                    renderer.draw_rect(x, y, slot_size, slot_size, [0.15, 0.15, 0.18, 1.0]);
                    renderer.draw_rect_border(
                        x,
                        y,
                        slot_size,
                        slot_size,
                        1.0,
                        [0.3, 0.3, 0.3, 0.8],
                    );
                }
            }
        }

        // Подсказка закрытия
        let close_hint = "[TAB] Close Inventory";
        let hint_x = (screen_width - close_hint.len() as f32 * 14.0) / 2.0;
        unsafe {
            renderer.draw_text(
                close_hint,
                hint_x,
                inv_y + inv_height + 20.0,
                14.0,
                [0.5, 0.5, 0.5, 0.8],
            );
        }
    }

    /// Настройки: Рендеринг меню настроек
    /// Отображает все категории настроек с возможностью навигации
    pub fn render_settings(&self, renderer: &mut crate::graphics::renderer::Renderer) {
        let screen_width = renderer.get_width() as f32;
        let screen_height = renderer.get_height() as f32;

        // Параметры окна настроек
        let settings_width = 700.0;
        let settings_height = 550.0;
        let settings_x = (screen_width - settings_width) / 2.0;
        let settings_y = (screen_height - settings_height) / 2.0;

        // Фон настроек (тёмный полупрозрачный)
        unsafe {
            renderer.draw_rect(
                settings_x,
                settings_y,
                settings_width,
                settings_height,
                [0.05, 0.05, 0.08, 0.98],
            );
            renderer.draw_rect_border(
                settings_x,
                settings_y,
                settings_width,
                settings_height,
                3.0,
                [0.6, 0.6, 0.6, 0.9],
            );
        }

        // Заголовок
        let title = "SETTINGS";
        unsafe {
            renderer.draw_text(
                title,
                settings_x + 30.0,
                settings_y + 20.0,
                28.0,
                [0.95, 0.95, 0.95, 1.0],
            );
        }

        // Категории настроек (левая панель)
        let categories = [
            "Display",
            "Graphics",
            "Audio",
            "Controls",
            "Gameplay",
            "HUD",
            "Network",
            "Performance",
        ];

        let panel_x = settings_x + 30.0;
        let panel_y = settings_y + 80.0;
        let panel_width = 180.0;
        let panel_height = 400.0;

        // Фон левой панели
        unsafe {
            renderer.draw_rect(
                panel_x,
                panel_y,
                panel_width,
                panel_height,
                [0.1, 0.1, 0.15, 0.8],
            );
            renderer.draw_rect_border(
                panel_x,
                panel_y,
                panel_width,
                panel_height,
                2.0,
                [0.4, 0.4, 0.4, 0.7],
            );
        }

        // Список категорий
        let mut y_offset = panel_y + 20.0;
        for (i, category) in categories.iter().enumerate() {
            let color = if i == 0 {
                [0.3, 0.8, 0.3, 1.0]
            } else {
                [0.7, 0.7, 0.7, 0.9]
            };
            unsafe {
                renderer.draw_text(category, panel_x + 15.0, y_offset, 16.0, color);
            }
            y_offset += 35.0;
        }

        // Правая панель (контент настроек)
        let content_x = settings_x + 230.0;
        let content_y = settings_y + 80.0;
        let content_width = settings_width - 250.0;
        let content_height = 400.0;

        // Фон правой панели
        unsafe {
            renderer.draw_rect(
                content_x,
                content_y,
                content_width,
                content_height,
                [0.12, 0.12, 0.16, 0.7],
            );
            renderer.draw_rect_border(
                content_x,
                content_y,
                content_width,
                content_height,
                2.0,
                [0.4, 0.4, 0.4, 0.7],
            );
        }

        // Пример контента (заглушка для демонстрации)
        unsafe {
            renderer.draw_text(
                "Display Settings",
                content_x + 20.0,
                content_y + 30.0,
                20.0,
                [0.9, 0.9, 0.9, 1.0],
            );

            // Пример опции: Fullscreen
            renderer.draw_text(
                "Fullscreen:",
                content_x + 20.0,
                content_y + 80.0,
                16.0,
                [0.7, 0.7, 0.7, 1.0],
            );
            renderer.draw_text(
                "[OFF]",
                content_x + 150.0,
                content_y + 80.0,
                16.0,
                [0.8, 0.8, 0.3, 1.0],
            );

            // Пример опции: VSync
            renderer.draw_text(
                "VSync:",
                content_x + 20.0,
                content_y + 120.0,
                16.0,
                [0.7, 0.7, 0.7, 1.0],
            );
            renderer.draw_text(
                "[ON]",
                content_x + 150.0,
                content_y + 120.0,
                16.0,
                [0.3, 0.8, 0.3, 1.0],
            );

            // Пример опции: FPS Limit
            renderer.draw_text(
                "FPS Limit:",
                content_x + 20.0,
                content_y + 160.0,
                16.0,
                [0.7, 0.7, 0.7, 1.0],
            );
            renderer.draw_text(
                "60",
                content_x + 150.0,
                content_y + 160.0,
                16.0,
                [0.9, 0.9, 0.9, 1.0],
            );

            // Пример опции: Brightness
            renderer.draw_text(
                "Brightness:",
                content_x + 20.0,
                content_y + 200.0,
                16.0,
                [0.7, 0.7, 0.7, 1.0],
            );
            renderer.draw_text(
                "100%",
                content_x + 150.0,
                content_y + 200.0,
                16.0,
                [0.9, 0.9, 0.9, 1.0],
            );

            // Разделитель
            renderer.draw_rect(
                content_x + 20.0,
                content_y + 240.0,
                content_width - 40.0,
                1.0,
                [0.3, 0.3, 0.3, 1.0],
            );

            // Подсказка
            renderer.draw_text(
                "Use mouse to navigate • Click to change values",
                content_x + 20.0,
                content_y + 270.0,
                14.0,
                [0.5, 0.5, 0.5, 0.8],
            );
        }

        // Нижняя панель с кнопками
        let bottom_y = settings_y + settings_height - 60.0;

        // Кнопка "Save & Close"
        let save_btn_x = settings_x + settings_width - 280.0;
        let save_btn_y = bottom_y + 10.0;
        let save_btn_w = 130.0;
        let save_btn_h = 35.0;

        unsafe {
            renderer.draw_rect(
                save_btn_x,
                save_btn_y,
                save_btn_w,
                save_btn_h,
                [0.2, 0.6, 0.2, 0.9],
            );
            renderer.draw_rect_border(
                save_btn_x,
                save_btn_y,
                save_btn_w,
                save_btn_h,
                2.0,
                [0.4, 0.8, 0.4, 1.0],
            );
            renderer.draw_text(
                "Save & Close",
                save_btn_x + 15.0,
                save_btn_y + 10.0,
                16.0,
                [1.0, 1.0, 1.0, 1.0],
            );
        }

        // Кнопка "Cancel"
        let cancel_btn_x = save_btn_x + save_btn_w + 15.0;
        unsafe {
            renderer.draw_rect(
                cancel_btn_x,
                save_btn_y,
                save_btn_w,
                save_btn_h,
                [0.6, 0.2, 0.2, 0.9],
            );
            renderer.draw_rect_border(
                cancel_btn_x,
                save_btn_y,
                save_btn_w,
                save_btn_h,
                2.0,
                [0.8, 0.4, 0.4, 1.0],
            );
            renderer.draw_text(
                "Cancel",
                cancel_btn_x + 35.0,
                save_btn_y + 10.0,
                16.0,
                [1.0, 1.0, 1.0, 1.0],
            );
        }

        // Кнопка "Defaults"
        let defaults_btn_x = settings_x + 30.0;
        unsafe {
            renderer.draw_rect(
                defaults_btn_x,
                save_btn_y,
                save_btn_w,
                save_btn_h,
                [0.2, 0.4, 0.6, 0.9],
            );
            renderer.draw_rect_border(
                defaults_btn_x,
                save_btn_y,
                save_btn_w,
                save_btn_h,
                2.0,
                [0.4, 0.6, 0.8, 1.0],
            );
            renderer.draw_text(
                "Defaults",
                defaults_btn_x + 30.0,
                save_btn_y + 10.0,
                16.0,
                [1.0, 1.0, 1.0, 1.0],
            );
        }

        // Подсказка закрытия
        let close_hint = "[ESC] Close Settings";
        let hint_x = (screen_width - close_hint.len() as f32 * 14.0) / 2.0;
        unsafe {
            renderer.draw_text(
                close_hint,
                hint_x,
                settings_y + settings_height + 20.0,
                14.0,
                [0.5, 0.5, 0.5, 0.8],
            );
        }
    }
}

impl Default for HudManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hud_manager_creation() {
        let hud = HudManager::new();
        assert!(hud.is_visible());
        assert_eq!(hud.get_data(), None);
    }

    #[test]
    fn test_hud_update() {
        let mut hud = HudManager::new();
        let data = VehicleHudData {
            speed_kmh: 60.0,
            engine_rpm: 2000.0,
            gear: GearState::Drive(3),
            engine_running: true,
            fuel_level: 0.5,
            ..Default::default()
        };

        hud.update(data.clone(), 0.016);

        let hud_data = hud.get_data().expect("HUD data should exist");
        assert_eq!(hud_data.speed_kmh, 60.0);
        assert_eq!(hud_data.gear, GearState::Drive(3));
    }

    #[test]
    fn test_fuel_reserve_flash() {
        let mut hud = HudManager::new();
        let data = VehicleHudData {
            fuel_level: 0.1, // ниже 15%
            ..Default::default()
        };

        hud.update(data, 0.016);
        assert!(hud.flash_element.is_some());
        assert_eq!(
            hud.flash_element.expect("Flash element should exist"),
            HudFlashElement::FuelReserve
        );
    }

    #[test]
    fn test_map_system() {
        let hud = HudManager::new();
        let map = hud.get_map_system();
        assert!(!map.map_open);
        assert_eq!(map.markers.len(), 0);
    }
}

// ============================================================================
// КАРТА: Методы рендеринга (вынесены отдельно для читаемости)
// ============================================================================

impl HudManager {
    /// Карта: Рендеринг полноэкранной карты
    /// Вид сверху с местоположением игрока и метками
    pub fn render_map(&self, renderer: &mut crate::graphics::renderer::Renderer) {
        let screen_width = renderer.get_width() as f32;
        let screen_height = renderer.get_height() as f32;

        // Фон карты (тёмный, на весь экран)
        unsafe {
            renderer.draw_rect(
                0.0,
                0.0,
                screen_width,
                screen_height,
                [0.08, 0.08, 0.12, 1.0],
            );
        }

        // Заголовок карты
        let title = "MAP";
        unsafe {
            renderer.draw_text(title, 20.0, 20.0, 32.0, [0.9, 0.9, 0.9, 1.0]);
        }

        // Легенда (слева сверху) - расшифровка маркеров
        let legend_x = 20.0;
        let legend_y = 70.0;
        let legend_title = "ЛЕГЕНДА:";
        unsafe {
            renderer.draw_text(legend_title, legend_x, legend_y, 20.0, [0.7, 0.7, 0.7, 1.0]);
        }

        use crate::game::MarkerType;
        let markers = [
            (MarkerType::Player, "Игрок"),
            (MarkerType::Objective, "Цель"),
            (MarkerType::Friend, "Друг"),
            (MarkerType::Danger, "Опасность"),
            (MarkerType::Custom1, "Метка 1"),
            (MarkerType::Custom2, "Метка 2"),
        ];

        for (i, (marker_type, label)) in markers.iter().enumerate() {
            let y = legend_y + 30.0 + (i as f32 * 25.0);
            let color = marker_type.color();

            // Рисуем цветной квадрат
            unsafe {
                renderer.draw_rect(legend_x, y, 20.0, 20.0, color);
                renderer.draw_text(label, legend_x + 30.0, y + 2.0, 16.0, [0.9, 0.9, 0.9, 1.0]);
            }
        }

        // Область карты (центр экрана)
        let map_size = screen_height.min(screen_width) * 0.7;
        let map_x = (screen_width - map_size) / 2.0;
        let map_y = (screen_height - map_size) / 2.0 + 30.0;

        // Граница карты
        unsafe {
            renderer.draw_rect_border(map_x, map_y, map_size, map_size, 3.0, [0.5, 0.5, 0.5, 0.8]);
            // Сетка карты (условная)
            for i in 1..10 {
                let line_x = map_x + (map_size / 10.0) * i as f32;
                let line_y = map_y + (map_size / 10.0) * i as f32;
                renderer.draw_line(
                    line_x,
                    map_y,
                    line_x,
                    map_y + map_size,
                    1.0,
                    [0.2, 0.2, 0.25, 0.5],
                );
                renderer.draw_line(
                    map_x,
                    line_y,
                    map_x + map_size,
                    line_y,
                    1.0,
                    [0.2, 0.2, 0.25, 0.5],
                );
            }
        }

        // Игрок в центре карты (синий треугольник)
        let player_x = map_x + map_size / 2.0;
        let player_y = map_y + map_size / 2.0;
        unsafe {
            // Треугольник игрока
            renderer.draw_triangle(
                player_x,
                player_y - 12.0,
                player_x - 10.0,
                player_y + 8.0,
                player_x + 10.0,
                player_y + 8.0,
                MarkerType::Player.color(),
            );
        }

        // Метки игроков (до 4 штук)
        for marker in &self.map_system.markers {
            // Преобразуем мировые координаты в экранные (упрощённо)
            let scale = map_size / 10000.0; // 10км = размер карты
            let marker_x = map_x + map_size / 2.0 + marker.position.0 * scale;
            let marker_y = map_y + map_size / 2.0 - marker.position.1 * scale;

            // Ограничиваем пределами карты
            let marker_x = marker_x.clamp(map_x, map_x + map_size);
            let marker_y = marker_y.clamp(map_y, map_y + map_size);

            unsafe {
                // Рисуем маркер как круг с цветом
                renderer.draw_circle(marker_x, marker_y, 8.0, marker.marker_type.color());

                // Подпись маркера
                if !marker.label.is_empty() {
                    renderer.draw_text(
                        &marker.label,
                        marker_x + 12.0,
                        marker_y - 6.0,
                        12.0,
                        [0.9, 0.9, 0.9, 0.9],
                    );
                }
            }
        }

        // Управление зумом (подсказка)
        let zoom_hint = format!(
            "Zoom: {:.1}x | [+/-] Zoom | [ESC] Close",
            self.map_system.zoom
        );
        let hint_x = (screen_width - zoom_hint.len() as f32 * 14.0) / 2.0;
        unsafe {
            renderer.draw_text(
                &zoom_hint,
                hint_x,
                screen_height - 40.0,
                14.0,
                [0.6, 0.6, 0.6, 0.9],
            );
        }

        // Контекстное меню (если открыто)
        if self.map_system.context_menu_open {
            self.render_map_context_menu(renderer);
        }
    }

    /// Карта: Рендеринг контекстного меню для бумажной карты
    /// Появляется при правом клике на карту в инвентаре
    pub fn render_map_context_menu(&self, renderer: &mut crate::graphics::renderer::Renderer) {
        let menu_width = 180.0;
        let menu_height = 80.0;

        // Позиция меню (рядом с курсором или по центру)
        let menu_x = self.map_system.context_menu_pos.0;
        let menu_y = self.map_system.context_menu_pos.1;

        // Фон меню
        unsafe {
            renderer.draw_rect(
                menu_x,
                menu_y,
                menu_width,
                menu_height,
                [0.1, 0.1, 0.15, 0.95],
            );
            renderer.draw_rect_border(
                menu_x,
                menu_y,
                menu_width,
                menu_height,
                2.0,
                [0.6, 0.6, 0.6, 0.9],
            );
        }

        // Кнопка "Открыть"
        let open_btn_y = menu_y + 20.0;
        let open_btn = "[ОТКРЫТЬ]";
        unsafe {
            renderer.draw_text(
                open_btn,
                menu_x + 10.0,
                open_btn_y,
                18.0,
                [0.2, 0.9, 0.2, 1.0],
            );
        }

        // Подсказка
        let hint = "ЛКМ - Открыть карту";
        unsafe {
            renderer.draw_text(
                hint,
                menu_x + 10.0,
                menu_y + 50.0,
                12.0,
                [0.7, 0.7, 0.7, 0.8],
            );
        }
    }

    /// Получить доступ к системе карт
    pub fn get_map_system(&self) -> &crate::game::MapSystem {
        &self.map_system
    }

    /// Получить мутабельный доступ к системе карт
    pub fn get_map_system_mut(&mut self) -> &mut crate::game::MapSystem {
        &mut self.map_system
    }

    /// Добавить метку на карту
    pub fn add_map_marker(
        &mut self,
        marker_type: crate::game::MarkerType,
        x: f32,
        z: f32,
        label: String,
    ) -> Option<u32> {
        self.map_system.add_marker(marker_type, x, z, label)
    }

    /// Удалить метку с карты
    pub fn remove_map_marker(&mut self, id: u32) {
        self.map_system.remove_marker(id);
    }

    /// Получить все метки для компаса
    pub fn get_compass_markers(&self) -> Vec<&crate::game::MapMarker> {
        self.map_system.get_compass_markers()
    }
}
