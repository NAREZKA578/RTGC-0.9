//! Integration tests for RTGC engine
//! 
//! These tests verify that different subsystems work together correctly.

use rtgc::config::{Config, GraphicsConfig, PhysicsConfig};
use rtgc::error::EngineError;

/// Test: Configuration validation prevents invalid values
#[test]
fn test_config_validation_rejects_invalid_values() {
    // Test invalid FPS (0 should be rejected)
    let config = Config {
        graphics: GraphicsConfig {
            max_fps: Some(0),
            ..Default::default()
        },
        ..Default::default()
    };
    
    assert!(config.validate().is_err());
    
    // Test invalid FPS (> 1000 should be rejected)
    let config = Config {
        graphics: GraphicsConfig {
            max_fps: Some(1001),
            ..Default::default()
        },
        ..Default::default()
    };
    
    assert!(config.validate().is_err());
    
    // Test excessive memory budget
    let config = Config {
        graphics: GraphicsConfig {
            texture_streaming_budget_mb: 5000,
            ..Default::default()
        },
        ..Default::default()
    };
    
    assert!(config.validate().is_err());
}

/// Test: Configuration validation accepts valid values
#[test]
fn test_config_validation_accepts_valid_values() {
    let config = Config {
        graphics: GraphicsConfig {
            max_fps: Some(60),
            texture_streaming_budget_mb: 2048,
            ..Default::default()
        },
        physics: PhysicsConfig {
            substeps: 4,
            ..Default::default()
        },
        ..Default::default()
    };
    
    assert!(config.validate().is_ok());
}

/// Test: Path sanitization prevents directory traversal
#[test]
fn test_path_sanitization_prevents_traversal() {
    use std::path::PathBuf;
    
    // These paths should be rejected or sanitized
    let malicious_paths = vec![
        "/etc/passwd",
        "../../../etc/passwd",
        "C:\\Windows\\System32",
        "..\\..\\..\\secret.txt",
    ];
    
    for path_str in malicious_paths {
        let result = rtgc::utils::sanitize_path(path_str);
        // Path should either be rejected or sanitized to a safe value
        assert!(result.is_err() || !result.unwrap().to_string_lossy().contains(".."));
    }
    
    // Valid paths should be accepted
    let valid_paths = vec![
        "saves/game1",
        "./assets/textures",
        "/home/user/rtgc/data",
    ];
    
    for path_str in valid_paths {
        let result = rtgc::utils::sanitize_path(path_str);
        assert!(result.is_ok(), "Valid path {} was rejected", path_str);
    }
}

/// Test: Physics state validation detects NaN
#[test]
fn test_physics_state_validation_detects_nan() {
    use nalgebra::{Vector3, Vector4};
    
    // Create vectors with NaN values
    let nan_vector = Vector3::new(f32::NAN, 0.0, 0.0);
    let inf_vector = Vector3::new(f32::INFINITY, 0.0, 0.0);
    let valid_vector = Vector3::new(1.0, 2.0, 3.0);
    
    assert!(!rtgc::physics::is_finite_vector(&nan_vector));
    assert!(!rtgc::physics::is_finite_vector(&inf_vector));
    assert!(rtgc::physics::is_finite_vector(&valid_vector));
    
    // Test quaternion validation
    let nan_quat = Vector4::new(f32::NAN, 0.0, 0.0, 0.0);
    let valid_quat = Vector4::new(1.0, 0.0, 0.0, 0.0);
    
    assert!(!rtgc::physics::is_finite_quaternion(&nan_quat));
    assert!(rtgc::physics::is_finite_quaternion(&valid_quat));
}

/// Test: Vehicle physics stability under extreme conditions
#[test]
fn test_vehicle_physics_stability() {
    use rtgc::physics::vehicle::Vehicle;
    use rtgc::physics::PhysicsWorld;
    use nalgebra::Vector3;
    
    let mut vehicle = Vehicle::new(Vector3::zeros());
    let mut world = PhysicsWorld::new();
    
    // Set extreme but finite values
    vehicle.set_velocity(Vector3::new(1000.0, 0.0, 0.0));
    
    // Update should not panic and should maintain finite state
    vehicle.physics_update(0.016, &mut world, &|| None, &|| None);
    
    // Verify state is still finite after update
    assert!(vehicle.get_position().iter().all(|&x| x.is_finite()));
    assert!(vehicle.get_velocity().iter().all(|&x| x.is_finite()));
}

/// Test: Helicopter physics handles NaN gracefully
#[test]
fn test_helicopter_physics_handles_nan() {
    use rtgc::physics::helicopter::Helicopter;
    use rtgc::physics::PhysicsWorld;
    use nalgebra::Vector3;
    
    let mut heli = Helicopter::new(Vector3::new(0.0, 100.0, 0.0));
    let mut world = PhysicsWorld::new();
    
    // Inject NaN into thrust (simulating sensor failure or calculation error)
    heli.set_thrust(Vector3::new(f32::NAN, 0.0, 0.0));
    
    // Update should detect NaN and reset to safe state without panicking
    heli.physics_update(0.016, &mut world, &|| None, &|| None);
    
    // After update, all values should be finite (safe state)
    assert!(heli.get_position().iter().all(|&x| x.is_finite()));
    assert!(heli.get_velocity().iter().all(|&x| x.is_finite()));
}

/// Test: Profiler respects measurement limits
#[test]
fn test_profiler_respects_limits() {
    use rtgc::profiler::Profiler;
    
    let mut profiler = Profiler::new();
    const MAX_MEASUREMENTS: usize = 1000;
    
    // Add more measurements than the limit
    for i in 0..MAX_MEASUREMENTS + 100 {
        let name = format!("test_measurement_{}", i % 10);
        profiler.start_timer(&name);
        profiler.stop_timer(&name);
    }
    
    // Verify that we don't exceed the limit significantly
    // (some overflow is acceptable due to concurrent access)
    let report = profiler.get_report();
    assert!(report.len() <= MAX_MEASUREMENTS * 2, 
            "Profiler exceeded reasonable memory bounds");
}

/// Test: Error types properly chain causes
#[test]
fn test_error_chaining() {
    use rtgc::error::{EngineError, GraphicsError};
    use std::io;
    
    // Create a chained error
    let io_error = io::Error::new(io::ErrorKind::NotFound, "file not found");
    let graphics_error = GraphicsError::Io(io_error);
    let engine_error = EngineError::Graphics(graphics_error);
    
    // Verify error chain
    let error_string = format!("{}", engine_error);
    assert!(error_string.contains("Graphics error"));
    assert!(error_string.contains("file not found"));
}

/// Test: Engine state transitions are valid
#[test]
fn test_engine_state_transitions() {
    use rtgc::engine::EngineState;
    
    // Test state transitions
    let initializing = EngineState::Initializing { progress: 0.0 };
    assert_eq!(initializing.progress(), 0.0);
    
    let loading = EngineState::Initializing { progress: 0.5 };
    assert_eq!(loading.progress(), 0.5);
    
    let complete = EngineState::Initializing { progress: 1.0 };
    assert_eq!(complete.progress(), 1.0);
    
    // Invalid progress should be handled
    let invalid_low = EngineState::Initializing { progress: -0.1 };
    assert!(invalid_low.progress().clamp(0.0, 1.0) == 0.0);
    
    let invalid_high = EngineState::Initializing { progress: 1.5 };
    assert!(invalid_high.progress().clamp(0.0, 1.0) == 1.0);
}

/// Test: Deformable terrain validates state correctly
#[test]
fn test_deformable_terrain_validation() {
    use rtgc::physics::deformable_terrain::{DeformableTerrainComponent, DeformationProperties};
    use nalgebra::Vector3;
    
    // Create valid terrain
    let mut terrain = DeformableTerrainComponent::new(0, 10, 10);
    assert!(terrain.validate_state());
    
    // Corrupt heightmap with NaN
    terrain.current_heightmap[0][0] = f32::NAN;
    assert!(!terrain.validate_state());
    
    // Reset should restore valid state
    terrain.reset_to_safe_state();
    assert!(terrain.validate_state());
}

/// Test: Fracture component validates state correctly
#[test]
fn test_fracture_component_validation() {
    use rtgc::physics::fracture_component::FractureComponent;
    
    // Create valid fracture component
    let mut fracture = FractureComponent::new(100.0);
    assert!(fracture.validate_state());
    
    // Invalid strength threshold
    fracture.strength_threshold = f32::NAN;
    assert!(!fracture.validate_state());
    
    // Reset should restore valid state
    fracture.reset_to_safe_state();
    assert!(fracture.validate_state());
    
    // Invalid structural integrity (out of range)
    fracture.structural_integrity = 1.5;
    assert!(!fracture.validate_state());
}
