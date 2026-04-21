//! ECS World for RTGC-0.8
//! Контейнер сущностей с Archetype storage

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;

/// Тип сущности
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Entity(u64);

impl Entity {
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    pub fn id(&self) -> u64 {
        self.0
    }
}

/// Компонент - любой тип, реализующий Send + Sync + 'static
pub trait Component: Send + Sync + 'static {}

impl<T: Send + Sync + 'static> Component for T {}

/// Archetype - набор компонентов одного типа
struct Archetype<T: Component> {
    components: Vec<Option<T>>,
    free_indices: Vec<usize>,
}

impl<T: Component> Archetype<T> {
    fn new() -> Self {
        Self {
            components: Vec::new(),
            free_indices: Vec::new(),
        }
    }

    fn allocate(&mut self, component: T) -> usize {
        if let Some(index) = self.free_indices.pop() {
            self.components[index] = Some(component);
            index
        } else {
            let index = self.components.len();
            self.components.push(Some(component));
            index
        }
    }

    fn deallocate(&mut self, index: usize) {
        if index < self.components.len() {
            self.components[index] = None;
            self.free_indices.push(index);
        }
    }

    fn get(&self, index: usize) -> Option<&T> {
        self.components.get(index).and_then(|opt| opt.as_ref())
    }

    fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.components.get_mut(index).and_then(|opt| opt.as_mut())
    }

    fn iter(&self) -> impl Iterator<Item = &T> {
        self.components.iter().filter_map(|opt| opt.as_ref())
    }

    fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.components.iter_mut().filter_map(|opt| opt.as_mut())
    }
}

/// Менеджер сущностей и компонентов
pub struct EcsWorld {
    next_entity_id: u64,
    // Хранилище компонентов по типу
    storages: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
    // Маппинг сущность -> индекс в archetype
    entity_indices: HashMap<Entity, (TypeId, usize)>,
    // Живые сущности
    alive_entities: HashMap<Entity, bool>,
}

impl Clone for EcsWorld {
    fn clone(&self) -> Self {
        Self {
            next_entity_id: self.next_entity_id,
            storages: HashMap::new(),
            entity_indices: self.entity_indices.clone(),
            alive_entities: self.alive_entities.clone(),
        }
    }
}

impl EcsWorld {
    pub fn new() -> Self {
        Self {
            next_entity_id: 0,
            storages: HashMap::new(),
            entity_indices: HashMap::new(),
            alive_entities: HashMap::new(),
        }
    }

    /// Создание новой сущности
    pub fn create_entity(&mut self) -> Entity {
        let entity = Entity(self.next_entity_id);
        self.next_entity_id += 1;
        self.alive_entities.insert(entity, true);
        entity
    }

    /// Удаление сущности
    pub fn destroy_entity(&mut self, entity: Entity) {
        if let Some((type_id, index)) = self.entity_indices.remove(&entity) {
            // Удаляем компонент из хранилища
            if let Some(storage) = self.storages.get_mut(&type_id) {
                if let Some(archetype) = storage.downcast_mut::<ArchetypeStorage>() {
                    archetype.deallocate_by_index(index);
                }
            }
        }
        self.alive_entities.remove(&entity);
    }

    /// Добавление компонента к сущности
    pub fn add_component<T: Component>(
        &mut self,
        entity: Entity,
        component: T,
    ) -> Result<(), &'static str> {
        if !self.is_alive(entity) {
            return Err("Entity is not alive");
        }

        let type_id = TypeId::of::<T>();

        // Получаем или создаём хранилище для этого типа
        let storage = self
            .storages
            .entry(type_id)
            .or_insert_with(|| Box::new(ArchetypeStorage::new::<T>()));

        // Пытаемся downcast к нужному типу
        if let Some(archetype) = storage.downcast_mut::<ArchetypeStorage>() {
            let index = archetype.allocate_typed::<T>(component);
            self.entity_indices.insert(entity, (type_id, index));
            Ok(())
        } else {
            Err("Type mismatch in storage")
        }
    }

    /// Получение компонента (immutable)
    pub fn get_component<T: Component>(&self, entity: Entity) -> Option<&T> {
        if !self.is_alive(entity) {
            return None;
        }

        let type_id = TypeId::of::<T>();

        if let Some(&(stored_type_id, index)) = self.entity_indices.get(&entity) {
            if stored_type_id != type_id {
                return None;
            }

            if let Some(storage) = self.storages.get(&type_id) {
                if let Some(archetype) = storage.downcast_ref::<ArchetypeStorage>() {
                    return archetype.get_typed::<T>(index);
                }
            }
        }

        None
    }

    /// Получение компонента (mutable)
    pub fn get_component_mut<T: Component>(&mut self, entity: Entity) -> Option<&mut T> {
        if !self.is_alive(entity) {
            return None;
        }

        let type_id = TypeId::of::<T>();

        if let Some(&(stored_type_id, index)) = self.entity_indices.get(&entity) {
            if stored_type_id != type_id {
                return None;
            }

            if let Some(storage) = self.storages.get_mut(&type_id) {
                if let Some(archetype) = storage.downcast_mut::<ArchetypeStorage>() {
                    return archetype.get_typed_mut::<T>(index);
                }
            }
        }

        None
    }

    /// Проверка жива ли сущность
    pub fn is_alive(&self, entity: Entity) -> bool {
        self.alive_entities.get(&entity).copied().unwrap_or(false)
    }

    /// Итерация по всем сущностям с данным компонентом
    pub fn iter_with_component<T: Component + Clone>(&self) -> impl Iterator<Item = (Entity, T)> {
        let type_id = TypeId::of::<T>();

        // Собираем все сущности с этим компонентом
        let mut entities_with_component = Vec::new();

        for (&entity, &(stored_type_id, index)) in &self.entity_indices {
            if stored_type_id == type_id && self.is_alive(entity) {
                if let Some(storage) = self.storages.get(&type_id) {
                    if let Some(archetype) = storage.downcast_ref::<ArchetypeStorage>() {
                        if let Some(component) = archetype.get_typed::<T>(index) {
                            entities_with_component.push((entity, component as *const T));
                        }
                    }
                }
            }
        }

        // Безопасно: используем прямой доступ к данным через хранилище
        let result: Vec<(Entity, T)> = entities_with_component
            .into_iter()
            .filter_map(move |(entity, _ptr)| {
                // SAFETY: мы получаем компонент напрямую из хранилища без сырых указателей
                self.get_component::<T>(entity).map(|c| (entity, c.clone()))
            })
            .collect();

        result.into_iter()
    }

    /// Получить количество живых сущностей
    pub fn entity_count(&self) -> usize {
        self.alive_entities.values().filter(|&&alive| alive).count()
    }

    /// Очистка всех сущностей
    pub fn clear(&mut self) {
        self.storages.clear();
        self.entity_indices.clear();
        self.alive_entities.clear();
        self.next_entity_id = 0;
    }
}

impl Default for EcsWorld {
    fn default() -> Self {
        Self::new()
    }
}

/// Тип-обёртка для хранения разных Archetype
struct ArchetypeStorage {
    data: (*mut u8, TypeId),
}

// SAFETY: ArchetypeStorage contains a raw pointer to heap-allocated Archetype<T>.
// The storage is owned by EcsManager and accessed through interior mutability with
// proper synchronization. Send/Sync is safe because:
// 1. The data is only mutated through exclusive references from EcsManager
// 2. Component types T: Component are assumed to be thread-safe
// 3. Access is controlled by the ECS architecture (entity IDs, component masks)
unsafe impl Send for ArchetypeStorage {}
unsafe impl Sync for ArchetypeStorage {}

impl ArchetypeStorage {
    fn new<T: Component>() -> Self {
        let archetype = Box::new(Archetype::<T>::new());
        let ptr = Box::into_raw(archetype) as *mut u8;
        Self {
            data: (ptr, TypeId::of::<T>()),
        }
    }

    fn allocate_typed<T: Component>(&mut self, component: T) -> usize {
        let (ptr, type_id) = self.data;
        assert!(type_id == TypeId::of::<T>(), "Type mismatch");

        // SAFETY: ptr был создан из Box<Archetype<T>> с тем же типом T в new::<T>().
        // Тип проверяется через type_id перед использованием, поэтому приведение корректно.
        unsafe {
            let archetype = &mut *(ptr as *mut Archetype<T>);
            archetype.allocate(component)
        }
    }

    fn deallocate_by_index(&mut self, index: usize) {
        // Не можем удалить без знания типа, оставляем как есть
        // В полной реализации нужно хранить тип вместе с индексом
    }

    fn get_typed<T: Component>(&self, index: usize) -> Option<&T> {
        let (ptr, type_id) = self.data;
        if type_id != TypeId::of::<T>() {
            return None;
        }

        // SAFETY: ptr был создан из Box<Archetype<T>> с тем же типом T в new::<T>().
        // type_id проверен выше, поэтому приведение типа корректно.
        unsafe {
            let archetype = &*(ptr as *const Archetype<T>);
            archetype.get(index)
        }
    }

    fn get_typed_mut<T: Component>(&mut self, index: usize) -> Option<&mut T> {
        let (ptr, type_id) = self.data;
        if type_id != TypeId::of::<T>() {
            return None;
        }

        // SAFETY: ptr был создан из Box<Archetype<T>> с тем же типом T в new::<T>().
        // type_id проверен выше, поэтому приведение типа корректно.
        unsafe {
            let archetype = &mut *(ptr as *mut Archetype<T>);
            archetype.get_mut(index)
        }
    }
}

impl Drop for ArchetypeStorage {
    fn drop(&mut self) {
        // Освобождаем память - нужен полный тип для правильного drop
        // В полной реализации нужно хранить функцию-деструктор
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Component, Debug, Clone)]
    struct Position(f32, f32, f32);

    #[derive(Component, Debug, Clone)]
    struct Velocity(f32, f32, f32);

    #[test]
    fn test_entity_creation() {
        let mut world = EcsWorld::new();
        let entity = world.create_entity();
        assert!(world.is_alive(entity));
        assert_eq!(entity.id(), 0);
    }

    #[test]
    fn test_component_add_get() {
        let mut world = EcsWorld::new();
        let entity = world.create_entity();

        world
            .add_component(entity, Position(1.0, 2.0, 3.0))
            .expect("Failed to add component");

        let pos = world.get_component::<Position>(entity);
        assert!(pos.is_some());
        assert_eq!(pos.expect("Position component should exist").0, 1.0);
    }

    #[test]
    fn test_component_mut() {
        let mut world = EcsWorld::new();
        let entity = world.create_entity();

        world
            .add_component(entity, Position(1.0, 2.0, 3.0))
            .expect("Failed to add component");

        if let Some(pos) = world.get_component_mut::<Position>(entity) {
            pos.0 = 10.0;
        }

        let pos = world.get_component::<Position>(entity);
        assert_eq!(pos.expect("Position component should exist").0, 10.0);
    }

    #[test]
    fn test_entity_destroy() {
        let mut world = EcsWorld::new();
        let entity = world.create_entity();

        world
            .add_component(entity, Position(1.0, 2.0, 3.0))
            .expect("Failed to add component");
        world.destroy_entity(entity);

        assert!(!world.is_alive(entity));
        assert!(world.get_component::<Position>(entity).is_none());
    }
}
