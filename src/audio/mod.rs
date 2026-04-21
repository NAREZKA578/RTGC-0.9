//! Audio module for RTGC-0.8

pub mod engine;
pub mod audio_module;

pub use engine::{AudioEngine, AudioConfig, AudioSource, SoundHandle, create_audio_engine};
pub use audio_module::AudioSystem;
