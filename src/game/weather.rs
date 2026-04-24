//! Weather system stub module
//! TODO: Implement proper weather system

use nalgebra::Vector3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PrecipitationType {
    None,
    Rain,
    Snow,
    Sleet,
}

#[derive(Debug, Clone, Copy)]
pub struct WeatherState {
    pub precipitation: PrecipitationType,
    pub intensity: f32,
    pub visibility: f32,
    pub wind_direction: Vector3<f32>,
    pub wind_speed: f32,
}

impl Default for WeatherState {
    fn default() -> Self {
        Self {
            precipitation: PrecipitationType::None,
            intensity: 0.0,
            visibility: 1.0,
            wind_direction: Vector3::new(0.0, 0.0, 1.0),
            wind_speed: 0.0,
        }
    }
}