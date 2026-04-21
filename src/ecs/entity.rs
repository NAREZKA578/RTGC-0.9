//! Entity component system - Entity type definition

use std::fmt;

/// Unique identifier for an entity in the ECS world
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Entity {
    /// The index of the entity in the archetype storage
    pub index: u32,
    /// Generation counter to detect stale entity references
    pub generation: u32,
}

impl Entity {
    /// Creates a new entity with the given index and generation
    pub const fn new(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }

    /// Returns a null entity (invalid)
    pub const fn null() -> Self {
        Self { index: u32::MAX, generation: u32::MAX }
    }

    /// Checks if this entity is null/invalid
    pub const fn is_null(&self) -> bool {
        self.index == u32::MAX && self.generation == u32::MAX
    }
}

impl Default for Entity {
    fn default() -> Self {
        Self::null()
    }
}

impl fmt::Display for Entity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Entity({}, {})", self.index, self.generation)
    }
}

/// Allocator for generating unique entity IDs
pub struct EntityAllocator {
    next_index: u32,
    generations: Vec<u32>,
    free_list: Vec<u32>,
}

impl EntityAllocator {
    /// Creates a new entity allocator
    pub fn new() -> Self {
        Self {
            next_index: 0,
            generations: Vec::new(),
            free_list: Vec::new(),
        }
    }

    /// Allocates a new entity ID
    pub fn allocate(&mut self) -> Entity {
        if let Some(index) = self.free_list.pop() {
            let generation = self.generations[index as usize];
            self.generations[index as usize] = generation.wrapping_add(1);
            Entity::new(index, self.generations[index as usize])
        } else {
            let index = self.next_index;
            self.next_index += 1;
            self.generations.push(1);
            Entity::new(index, 1)
        }
    }

    /// Frees an entity ID, allowing it to be reused
    pub fn free(&mut self, entity: Entity) {
        if (entity.index as usize) < self.generations.len() {
            self.free_list.push(entity.index);
        }
    }

    /// Checks if an entity ID is valid
    pub fn is_valid(&self, entity: Entity) -> bool {
        if entity.is_null() {
            return false;
        }
        if (entity.index as usize) >= self.generations.len() {
            return false;
        }
        self.generations[entity.index as usize] == entity.generation
    }

    /// Returns the number of allocated entities
    pub fn len(&self) -> usize {
        self.next_index as usize - self.free_list.len()
    }

    /// Returns true if no entities are allocated
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for EntityAllocator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_creation() {
        let entity = Entity::new(0, 1);
        assert!(!entity.is_null());
        assert_eq!(entity.index, 0);
        assert_eq!(entity.generation, 1);
    }

    #[test]
    fn test_null_entity() {
        let entity = Entity::null();
        assert!(entity.is_null());
    }

    #[test]
    fn test_allocator() {
        let mut allocator = EntityAllocator::new();
        let e1 = allocator.allocate();
        let e2 = allocator.allocate();
        
        assert!(allocator.is_valid(e1));
        assert!(allocator.is_valid(e2));
        assert_ne!(e1, e2);

        allocator.free(e1);
        assert!(!allocator.is_valid(e1));
        
        let e3 = allocator.allocate();
        assert!(allocator.is_valid(e3));
        assert_eq!(e3.index, e1.index);
        assert_ne!(e3.generation, e1.generation);
    }
}
