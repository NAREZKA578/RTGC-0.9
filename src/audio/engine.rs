//! Audio engine based on cpal and symphonia for decoding

use std::collections::HashMap;
use std::io::BufReader;
use nalgebra::Vector3;
use tracing;

/// Audio configuration
#[derive(Debug, Clone)]
pub struct AudioConfig {
    pub master_volume: f32,
    pub doppler_factor: f32,
    pub listener_position: Vector3<f32>,
    pub listener_velocity: Vector3<f32>,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            master_volume: 1.0,
            doppler_factor: 1.0,
            listener_position: Vector3::zeros(),
            listener_velocity: Vector3::zeros(),
        }
    }
}

/// Sound source handle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SoundHandle(u32);

/// Loaded sound data (decoded samples)
#[derive(Clone)]
pub struct LoadedSound {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u16,
}

/// Sound source parameters
#[derive(Debug, Clone)]
pub struct AudioSource {
    pub position: Vector3<f32>,
    pub velocity: Vector3<f32>,
    pub volume: f32,
    pub pitch: f32,
    pub is_looping: bool,
    pub max_distance: f32,
    pub rolloff_factor: f32,
    pub sound_handle: Option<SoundHandle>,
    is_playing: bool,
}

impl AudioSource {
    /// Update audio source state
    pub fn update(&mut self) {
        // Placeholder update logic
    }
    
    /// Check if sound is finished playing
    pub fn is_finished(&self) -> bool {
        !self.is_playing && !self.is_looping
    }
    
    /// Update 3D position based on listener
    pub fn update_3d_position(&mut self, listener_pos: &Vector3<f32>, listener_dir: &Vector3<f32>) {
        // Placeholder for 3D audio positioning
        let _distance = (self.position - *listener_pos).norm();
        let _direction = listener_dir;
    }
}

impl Default for AudioSource {
    fn default() -> Self {
        Self {
            position: Vector3::zeros(),
            velocity: Vector3::zeros(),
            volume: 1.0,
            pitch: 1.0,
            is_looping: false,
            max_distance: 100.0,
            rolloff_factor: 1.0,
            sound_handle: None,
            is_playing: false,
        }
    }
}

/// Audio engine - упрощенная заглушка для компиляции
pub struct AudioEngine {
    config: AudioConfig,
    sources: Vec<(SoundHandle, AudioSource)>,
    loaded_sounds: HashMap<SoundHandle, LoadedSound>,
    next_handle_id: u32,
    listener_position: Vector3<f32>,
    listener_orientation: Vector3<f32>,
}

impl AudioEngine {
    /// Creates a new audio engine
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            config: AudioConfig::default(),
            sources: Vec::new(),
            loaded_sounds: HashMap::new(),
            next_handle_id: 0,
            listener_position: Vector3::zeros(),
            listener_orientation: Vector3::new(0.0, 0.0, -1.0),
        })
    }

    /// Creates a new audio engine with custom config
    pub fn with_config(config: AudioConfig) -> Result<Self, String> {
        Ok(Self {
            config,
            sources: Vec::new(),
            loaded_sounds: HashMap::new(),
            next_handle_id: 0,
            listener_position: Vector3::zeros(),
            listener_orientation: Vector3::new(0.0, 0.0, -1.0),
        })
    }

    /// Loads a sound from file
    pub fn load_sound(&mut self, path: &str) -> Result<SoundHandle, String> {
        let handle = SoundHandle(self.next_handle_id);
        self.next_handle_id += 1;

        // Attempt to load the actual sound file
        match self.load_sound_file(path) {
            Ok(loaded_sound) => {
                self.loaded_sounds.insert(handle, loaded_sound);
                Ok(handle)
            }
            Err(e) => {
                tracing::warn!("Failed to load sound '{}': {}. Creating silent placeholder.", path, e);
                // If loading fails, create a silent placeholder
                self.loaded_sounds.insert(handle, LoadedSound {
                    samples: vec![0.0f32; 1024],
                    sample_rate: 44100,
                    channels: 2,
                });
                Ok(handle)
            }
        }
    }

    /// Internal method to load sound file
    fn load_sound_file(&self, path: &str) -> Result<LoadedSound, String> {
        use std::fs::File;
        use std::io::BufReader;

        // Try to detect format from extension
        let ext = std::path::Path::new(path).extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        match ext.as_str() {
            "wav" => self.load_wav_file(path),
            "ogg" => self.load_ogg_stub(path),
            "mp3" => self.load_mp3_stub(path),
            _ => Err(format!("Unsupported audio format: {}", ext)),
        }
    }

    /// Load WAV file (simple PCM decoder)
    fn load_wav_file(&self, path: &str) -> Result<LoadedSound, String> {
        use std::fs::File;
        use std::io::{Read, Seek, SeekFrom};

        let file = File::open(path).map_err(|e| format!("Cannot open file: {}", e))?;
        let mut reader = BufReader::new(file);

        // Read RIFF header
        let mut riff = [0u8; 4];
        reader.read_exact(&mut riff).map_err(|e| format!("Read error: {}", e))?;
        if &riff != b"RIFF" {
            return Err("Not a valid WAV file: missing RIFF header".to_string());
        }

        // Skip file size
        let mut size_buf = [0u8; 4];
        reader.read_exact(&mut size_buf).map_err(|e| format!("Read error: {}", e))?;

        // Read WAVE header
        let mut wave = [0u8; 4];
        reader.read_exact(&mut wave).map_err(|e| format!("Read error: {}", e))?;
        if &wave != b"WAVE" {
            return Err("Not a valid WAV file: missing WAVE header".to_string());
        }

        // Find fmt and data chunks
        let mut fmt_chunk = None;
        let mut data_offset = None;
        let mut data_size = None;

        loop {
            let mut chunk_id = [0u8; 4];
            if reader.read_exact(&mut chunk_id).is_err() {
                break;
            }

            let mut chunk_size_buf = [0u8; 4];
            reader.read_exact(&mut chunk_size_buf).map_err(|e| format!("Read error: {}", e))?;
            let chunk_size = u32::from_le_bytes(chunk_size_buf);

            let chunk_id_str = String::from_utf8_lossy(&chunk_id);
            match chunk_id_str.as_ref() {
                "fmt " => {
                    let mut fmt_data = vec![0u8; chunk_size as usize];
                    reader.read_exact(&mut fmt_data).map_err(|e| format!("Read error: {}", e))?;
                    
                    if fmt_data.len() >= 16 {
                        let audio_format = u16::from_le_bytes([fmt_data[0], fmt_data[1]]);
                        let num_channels = u16::from_le_bytes([fmt_data[2], fmt_data[3]]);
                        let sample_rate = u32::from_le_bytes([fmt_data[4], fmt_data[5], fmt_data[6], fmt_data[7]]);
                        let bits_per_sample = u16::from_le_bytes([fmt_data[14], fmt_data[15]]);
                        
                        if audio_format != 1 {
                            return Err(format!("Unsupported WAV format: {} (only PCM supported)", audio_format));
                        }
                        
                        fmt_chunk = Some((num_channels, sample_rate, bits_per_sample));
                    }
                }
                "data" => {
                    data_offset = Some(reader.stream_position().map_err(|e| format!("Seek error: {}", e))?);
                    data_size = Some(chunk_size);
                    break;
                }
                _ => {
                    // Skip unknown chunk
                    reader.seek(SeekFrom::Current(chunk_size as i64)).map_err(|e| format!("Seek error: {}", e))?;
                }
            }
        }

        let (num_channels, sample_rate, bits_per_sample) = fmt_chunk.ok_or("Missing fmt chunk in WAV file")?;
        let data_offset = data_offset.ok_or("Missing data chunk in WAV file")?;
        let data_size = data_size.ok_or("Missing data size in WAV file")?;

        // Read audio data
        reader.seek(SeekFrom::Start(data_offset)).map_err(|e| format!("Seek error: {}", e))?;
        
        let num_samples = (data_size as usize) / (bits_per_sample as usize / 8);
        let mut samples = Vec::with_capacity(num_samples);

        if bits_per_sample == 16 {
            for _ in 0..(num_samples / num_channels as usize) {
                let mut sample_buf = vec![0u8; 2 * num_channels as usize];
                if reader.read_exact(&mut sample_buf).is_ok() {
                    for ch in 0..num_channels as usize {
                        let sample = i16::from_le_bytes([sample_buf[ch * 2], sample_buf[ch * 2 + 1]]);
                        samples.push(sample as f32 / 32768.0);
                    }
                }
            }
        } else if bits_per_sample == 8 {
            for _ in 0..(num_samples / num_channels as usize) {
                let mut sample_buf = vec![0u8; num_channels as usize];
                if reader.read_exact(&mut sample_buf).is_ok() {
                    for ch in 0..num_channels as usize {
                        let sample = sample_buf[ch] as f32 / 128.0 - 1.0;
                        samples.push(sample);
                    }
                }
            }
        } else {
            return Err(format!("Unsupported bits per sample: {}", bits_per_sample));
        }

        Ok(LoadedSound {
            samples,
            sample_rate,
            channels: num_channels as u16,
        })
    }

    /// Stub for OGG loading (returns error with helpful message)
    fn load_ogg_stub(&self, _path: &str) -> Result<LoadedSound, String> {
        Err("OGG decoding requires ogg/vorbis crate. Install with: cargo add ogg vorbis".to_string())
    }

    /// Stub for MP3 loading (returns error with helpful message)
    fn load_mp3_stub(&self, _path: &str) -> Result<LoadedSound, String> {
        Err("MP3 decoding requires mp3decode or symphonia-mp3 crate".to_string())
    }

    /// Plays a loaded sound and returns a source handle
    pub fn play_loaded_sound(&mut self, sound_handle: SoundHandle, source: AudioSource) -> SoundHandle {
        if !self.loaded_sounds.contains_key(&sound_handle) {
            return sound_handle;
        }

        let handle = SoundHandle(self.next_handle_id);
        self.next_handle_id += 1;

        let mut source = source;
        source.sound_handle = Some(sound_handle);
        self.sources.push((handle, source));
        handle
    }

    /// Sets the pitch of a sound source (for engine RPM effect)
    pub fn set_pitch(&mut self, handle: SoundHandle, pitch: f32) {
        if let Some((_, source)) = self.sources.iter_mut().find(|(h, _)| *h == handle) {
            source.pitch = pitch.clamp(0.5, 2.0);
        }
    }

    /// Updates engine sound based on RPM
    pub fn update_engine_sound(
        &mut self,
        handle: SoundHandle,
        rpm: f32,
        max_rpm: f32
    ) {
        let pitch = 0.5 + (rpm / max_rpm) * 1.5; // pitch 0.5..2.0
        self.set_pitch(handle, pitch);
    }

    /// Plays a sound and returns a handle
    pub fn play_sound(&mut self, source: AudioSource) -> SoundHandle {
        let handle = SoundHandle(self.next_handle_id);
        self.next_handle_id += 1;
        self.sources.push((handle, source));
        handle
    }

    /// Stops a sound by handle
    pub fn stop_sound(&mut self, handle: SoundHandle) {
        if let Some(pos) = self.sources.iter().position(|(h, _)| *h == handle) {
            self.sources.remove(pos);
        }
    }

    /// Updates the position of a sound source
    pub fn set_source_position(&mut self, handle: SoundHandle, position: Vector3<f32>) {
        if let Some((_, source)) = self.sources.iter_mut().find(|(h, _)| *h == handle) {
            source.position = position;
        }
    }

    /// Updates the velocity of a sound source
    pub fn set_source_velocity(&mut self, handle: SoundHandle, velocity: Vector3<f32>) {
        if let Some((_, source)) = self.sources.iter_mut().find(|(h, _)| *h == handle) {
            source.velocity = velocity;
        }
    }

    /// Sets the volume of a sound source
    pub fn set_source_volume(&mut self, handle: SoundHandle, volume: f32) {
        if let Some((_, source)) = self.sources.iter_mut().find(|(h, _)| *h == handle) {
            source.volume = volume.clamp(0.0, 1.0);
        }
    }

    /// Sets whether a sound should loop
    pub fn set_source_looping(&mut self, handle: SoundHandle, looping: bool) {
        if let Some((_, source)) = self.sources.iter_mut().find(|(h, _)| *h == handle) {
            source.is_looping = looping;
        }
    }

    /// Updates the listener position
    pub fn set_listener_position(&mut self, position: Vector3<f32>) {
        self.config.listener_position = position;
    }

    /// Updates the listener velocity
    pub fn set_listener_velocity(&mut self, velocity: Vector3<f32>) {
        self.config.listener_velocity = velocity;
    }

    /// Sets the master volume
    pub fn set_master_volume(&mut self, volume: f32) {
        self.config.master_volume = volume.clamp(0.0, 1.0);
    }

    /// Updates all audio sources
    pub fn update(&mut self) {
        // Update all active sound sources
        self.sources.retain_mut(|(_, source)| {
            source.update();
            !source.is_finished()
        });

        // Update 3D audio positions based on listener position
        for (_, source) in &mut self.sources {
            source.update_3d_position(&self.listener_position, &self.listener_orientation);
        }
    }

    /// Returns the number of active sound sources
    pub fn active_source_count(&self) -> usize {
        self.sources.len()
    }
}

impl Drop for AudioEngine {
    fn drop(&mut self) {
        // Stop all sounds
        self.sources.clear();
    }
}

/// Creates a default audio engine, returning None if unavailable
pub fn create_audio_engine() -> Option<AudioEngine> {
    AudioEngine::new().ok()
}
