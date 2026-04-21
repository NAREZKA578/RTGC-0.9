//! Weather re-exports - используем единые реализации из world и weather модулей

pub use crate::weather::dynamic_weather::WeatherSystem;
pub use crate::world::DayNightCycle;

/// Упрощённое состояние погоды для совместимости
#[derive(Clone, Debug)]
pub struct WeatherState {
    pub precipitation_type: PrecipitationType,
    pub intensity: f32,
    pub cloud_coverage: f32,
    pub wind_speed: f32,
    pub temperature: f32,
}

impl Default for WeatherState {
    fn default() -> Self {
        Self {
            precipitation_type: PrecipitationType::None,
            intensity: 0.0,
            cloud_coverage: 0.3,
            wind_speed: 2.0,
            temperature: 20.0,
        }
    }
}

/// Упрощённые типы осадков для совместимости
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PrecipitationType {
    None,
    Rain,
    Snow,
}
