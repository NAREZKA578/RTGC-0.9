//! Entity component system - System trait and scheduler

use crate::ecs::world::EcsWorld;
use std::time::Duration;

/// Trait for all systems that operate on entities
pub trait System: Send + Sync {
    /// Returns the name of this system for debugging
    fn system_name(&self) -> &'static str;

    /// Called once when the system is first added
    fn init(&mut self, _world: &mut EcsWorld) {}

    /// Called every frame with delta time
    fn update(&mut self, world: &mut EcsWorld, delta_time: Duration);

    /// Called after all updates, before rendering
    fn post_update(&mut self, _world: &mut EcsWorld) {}

    /// Called when the system is removed
    fn shutdown(&mut self, _world: &mut EcsWorld) {}

    /// Returns the execution order priority (lower = earlier)
    fn priority(&self) -> i32 {
        0
    }
}

/// Scheduler for managing and executing systems
pub struct SystemScheduler {
    systems: Vec<Box<dyn System>>,
    enabled: Vec<bool>,
}

impl SystemScheduler {
    /// Creates a new system scheduler
    pub fn new() -> Self {
        Self {
            systems: Vec::new(),
            enabled: Vec::new(),
        }
    }

    /// Adds a system to the scheduler
    pub fn add_system<S: System + 'static>(&mut self, system: S) {
        let priority = system.priority();
        let mut index = 0;
        
        // Insert in priority order
        for (i, existing) in self.systems.iter().enumerate() {
            if existing.priority() > priority {
                break;
            }
            index = i + 1;
        }

        self.systems.insert(index, Box::new(system));
        self.enabled.insert(index, true);
    }

    /// Removes a system by name
    pub fn remove_system(&mut self, name: &str) -> Option<Box<dyn System>> {
        for (i, system) in self.systems.iter().enumerate() {
            if system.system_name() == name {
                self.enabled.remove(i);
                return Some(self.systems.remove(i));
            }
        }
        None
    }

    /// Enables or disables a system by name
    pub fn set_system_enabled(&mut self, name: &str, enabled: bool) {
        for (i, system) in self.systems.iter().enumerate() {
            if system.system_name() == name {
                self.enabled[i] = enabled;
                break;
            }
        }
    }

    /// Checks if a system is enabled
    pub fn is_system_enabled(&self, name: &str) -> bool {
        for (i, system) in self.systems.iter().enumerate() {
            if system.system_name() == name {
                return self.enabled[i];
            }
        }
        false
    }

    /// Initializes all systems
    pub fn init_all(&mut self, world: &mut EcsWorld) {
        for system in &mut self.systems {
            system.init(world);
        }
    }

    /// Updates all enabled systems
    pub fn update_all(&mut self, world: &mut EcsWorld, delta_time: Duration) {
        for (i, system) in self.systems.iter_mut().enumerate() {
            if self.enabled[i] {
                system.update(world, delta_time);
            }
        }
    }

    /// Calls post_update on all enabled systems
    pub fn post_update_all(&mut self, world: &mut EcsWorld) {
        for (i, system) in self.systems.iter_mut().enumerate() {
            if self.enabled[i] {
                system.post_update(world);
            }
        }
    }

    /// Shuts down all systems
    pub fn shutdown_all(&mut self, world: &mut EcsWorld) {
        for system in &mut self.systems {
            system.shutdown(world);
        }
        self.systems.clear();
        self.enabled.clear();
    }

    /// Returns the number of registered systems
    pub fn len(&self) -> usize {
        self.systems.len()
    }

    /// Returns true if no systems are registered
    pub fn is_empty(&self) -> bool {
        self.systems.is_empty()
    }

    /// Returns a list of system names
    pub fn system_names(&self) -> Vec<&str> {
        self.systems.iter().map(|s| s.system_name()).collect()
    }
}

impl Default for SystemScheduler {
    fn default() -> Self {
        Self::new()
    }
}

/// Example system for testing
#[cfg(test)]
pub struct TestSystem {
    update_count: u32,
}

#[cfg(test)]
impl TestSystem {
    pub fn new() -> Self {
        Self { update_count: 0 }
    }
}

#[cfg(test)]
impl System for TestSystem {
    fn system_name(&self) -> &'static str {
        "TestSystem"
    }

    fn update(&mut self, _world: &mut EcsWorld, _delta_time: Duration) {
        self.update_count += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scheduler() {
        let mut scheduler = SystemScheduler::new();
        let mut world = EcsWorld::new();
        
        scheduler.add_system(TestSystem::new());
        scheduler.init_all(&mut world);
        
        assert_eq!(scheduler.len(), 1);
        assert!(scheduler.is_system_enabled("TestSystem"));
        
        scheduler.update_all(&mut world, Duration::from_millis(16));
        scheduler.update_all(&mut world, Duration::from_millis(16));
        
        assert!(scheduler.is_system_enabled("TestSystem"));
    }
}
