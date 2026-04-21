//! Inventory System for RTGC-0.8
//! Manages player inventory, item stacking, weight limits

use serde::{Deserialize, Serialize};

/// Maximum inventory weight (kg)
pub const MAX_INVENTORY_WEIGHT: f32 = 60.0;

/// Maximum inventory slots
pub const MAX_INVENTORY_SLOTS: usize = 40;

/// Item types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ItemType {
    // Tools
    Wrench,
    Screwdriver,
    Hammer,
    Pliers,
    Drill,
    WeldingTorch,
    Jack,

    // Vehicle parts
    SparkPlug,
    OilFilter,
    AirFilter,
    BrakePad,
    Tire,
    Battery,
    Alternator,
    Starter,
    FuelPump,
    Radiator,

    // Construction
    Nail,
    Screw,
    Bolt,
    WoodenPlank,
    SteelBeam,
    ConcreteBag,
    Brick,

    // Resources
    Wood,
    Steel,
    Aluminum,
    Copper,
    Plastic,
    Rubber,
    Glass,

    // Consumables
    WaterBottle,
    FoodCan,
    FirstAidKit,
    FuelCanister,

    // Miscellaneous
    Rope,
    Flashlight,
    Map,
    Compass,
    Radio,
    Phone,

    // Documents
    License,
    Permit,
    Contract,
    Blueprint,
}

impl ItemType {
    /// Get base weight for item type (kg)
    pub fn base_weight(&self) -> f32 {
        match self {
            ItemType::Wrench => 0.5,
            ItemType::Screwdriver => 0.2,
            ItemType::Hammer => 0.8,
            ItemType::Pliers => 0.3,
            ItemType::Drill => 1.5,
            ItemType::WeldingTorch => 3.0,
            ItemType::Jack => 5.0,

            ItemType::SparkPlug => 0.1,
            ItemType::OilFilter => 0.3,
            ItemType::AirFilter => 0.2,
            ItemType::BrakePad => 0.5,
            ItemType::Tire => 10.0,
            ItemType::Battery => 15.0,
            ItemType::Alternator => 8.0,
            ItemType::Starter => 6.0,
            ItemType::FuelPump => 2.0,
            ItemType::Radiator => 4.0,

            ItemType::Nail => 0.01,
            ItemType::Screw => 0.02,
            ItemType::Bolt => 0.05,
            ItemType::WoodenPlank => 2.0,
            ItemType::SteelBeam => 25.0,
            ItemType::ConcreteBag => 25.0,
            ItemType::Brick => 2.5,

            ItemType::Wood => 5.0,
            ItemType::Steel => 10.0,
            ItemType::Aluminum => 5.0,
            ItemType::Copper => 8.0,
            ItemType::Plastic => 2.0,
            ItemType::Rubber => 3.0,
            ItemType::Glass => 5.0,

            ItemType::WaterBottle => 0.5,
            ItemType::FoodCan => 0.4,
            ItemType::FirstAidKit => 0.5,
            ItemType::FuelCanister => 5.0,

            ItemType::Rope => 1.0,
            ItemType::Flashlight => 0.3,
            ItemType::Map => 0.1,
            ItemType::Compass => 0.1,
            ItemType::Radio => 0.5,
            ItemType::Phone => 0.2,

            ItemType::License => 0.01,
            ItemType::Permit => 0.01,
            ItemType::Contract => 0.01,
            ItemType::Blueprint => 0.05,
        }
    }

    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            ItemType::Wrench => "Wrench",
            ItemType::Screwdriver => "Screwdriver",
            ItemType::Hammer => "Hammer",
            ItemType::Pliers => "Pliers",
            ItemType::Drill => "Power Drill",
            ItemType::WeldingTorch => "Welding Torch",
            ItemType::Jack => "Car Jack",

            ItemType::SparkPlug => "Spark Plug",
            ItemType::OilFilter => "Oil Filter",
            ItemType::AirFilter => "Air Filter",
            ItemType::BrakePad => "Brake Pad",
            ItemType::Tire => "Tire",
            ItemType::Battery => "Car Battery",
            ItemType::Alternator => "Alternator",
            ItemType::Starter => "Starter Motor",
            ItemType::FuelPump => "Fuel Pump",
            ItemType::Radiator => "Radiator",

            ItemType::Nail => "Nail",
            ItemType::Screw => "Screw",
            ItemType::Bolt => "Bolt",
            ItemType::WoodenPlank => "Wooden Plank",
            ItemType::SteelBeam => "Steel Beam",
            ItemType::ConcreteBag => "Concrete Bag (25kg)",
            ItemType::Brick => "Brick",

            ItemType::Wood => "Wood Bundle",
            ItemType::Steel => "Steel Ingot",
            ItemType::Aluminum => "Aluminum Sheet",
            ItemType::Copper => "Copper Wire",
            ItemType::Plastic => "Plastic Sheets",
            ItemType::Rubber => "Rubber Sheets",
            ItemType::Glass => "Glass Pane",

            ItemType::WaterBottle => "Water Bottle",
            ItemType::FoodCan => "Canned Food",
            ItemType::FirstAidKit => "First Aid Kit",
            ItemType::FuelCanister => "Fuel Canister (5L)",

            ItemType::Rope => "Rope (10m)",
            ItemType::Flashlight => "Flashlight",
            ItemType::Map => "Map",
            ItemType::Compass => "Compass",
            ItemType::Radio => "Radio",
            ItemType::Phone => "Mobile Phone",

            ItemType::License => "Driver's License",
            ItemType::Permit => "Work Permit",
            ItemType::Contract => "Contract",
            ItemType::Blueprint => "Blueprint",
        }
    }

    /// Get stack size limit
    pub fn max_stack_size(&self) -> u32 {
        match self {
            ItemType::Nail | ItemType::Screw | ItemType::Bolt => 100,
            ItemType::Brick | ItemType::WoodenPlank => 20,
            ItemType::ConcreteBag => 10,
            ItemType::WaterBottle | ItemType::FoodCan => 10,
            ItemType::SparkPlug => 8,
            ItemType::Tire => 4,
            _ => 1,
        }
    }

    /// Get base value in rubles
    pub fn base_value(&self) -> u32 {
        match self {
            ItemType::Wrench => 500,
            ItemType::Screwdriver => 300,
            ItemType::Hammer => 400,
            ItemType::Pliers => 350,
            ItemType::Drill => 3000,
            ItemType::WeldingTorch => 8000,
            ItemType::Jack => 2000,

            ItemType::SparkPlug => 200,
            ItemType::OilFilter => 400,
            ItemType::AirFilter => 300,
            ItemType::BrakePad => 800,
            ItemType::Tire => 3000,
            ItemType::Battery => 5000,
            ItemType::Alternator => 8000,
            ItemType::Starter => 6000,
            ItemType::FuelPump => 4000,
            ItemType::Radiator => 5000,

            ItemType::Nail => 5,
            ItemType::Screw => 8,
            ItemType::Bolt => 15,
            ItemType::WoodenPlank => 200,
            ItemType::SteelBeam => 3000,
            ItemType::ConcreteBag => 300,
            ItemType::Brick => 50,

            ItemType::Wood => 1000,
            ItemType::Steel => 2000,
            ItemType::Aluminum => 1500,
            ItemType::Copper => 2500,
            ItemType::Plastic => 800,
            ItemType::Rubber => 1000,
            ItemType::Glass => 1200,

            ItemType::WaterBottle => 50,
            ItemType::FoodCan => 100,
            ItemType::FirstAidKit => 500,
            ItemType::FuelCanister => 400,

            ItemType::Rope => 300,
            ItemType::Flashlight => 400,
            ItemType::Map => 200,
            ItemType::Compass => 300,
            ItemType::Radio => 1500,
            ItemType::Phone => 5000,

            ItemType::License => 0,
            ItemType::Permit => 0,
            ItemType::Contract => 0,
            ItemType::Blueprint => 0,
        }
    }
}

/// Single inventory item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryItem {
    pub item_type: ItemType,
    pub quantity: u32,
    pub condition: f32, // 0.0 - 1.0 (1.0 = new)
    pub custom_name: Option<String>,
}

impl InventoryItem {
    pub fn new(item_type: ItemType, quantity: u32) -> Self {
        Self {
            item_type,
            quantity,
            condition: 1.0,
            custom_name: None,
        }
    }

    /// Get total weight (kg)
    pub fn total_weight(&self) -> f32 {
        self.item_type.base_weight() * self.quantity as f32
    }

    /// Get total value (rubles)
    pub fn total_value(&self) -> u32 {
        self.item_type.base_value() * self.quantity
    }

    /// Check if item can be stacked with another
    pub fn can_stack_with(&self, other: &InventoryItem) -> bool {
        self.item_type == other.item_type
            && self.condition == other.condition
            && self.custom_name == other.custom_name
    }
}

/// Inventory slot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventorySlot {
    pub item: Option<InventoryItem>,
}

impl Default for InventorySlot {
    fn default() -> Self {
        Self { item: None }
    }
}

/// Player inventory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inventory {
    pub slots: Vec<InventorySlot>,
    pub money: u32,
}

impl Inventory {
    pub fn new() -> Self {
        let mut slots = Vec::with_capacity(MAX_INVENTORY_SLOTS);
        for _ in 0..MAX_INVENTORY_SLOTS {
            slots.push(InventorySlot::default());
        }

        Self {
            slots,
            money: 50000, // Starting money
        }
    }

    /// Get current total weight (kg)
    pub fn total_weight(&self) -> f32 {
        self.slots
            .iter()
            .filter_map(|slot| slot.item.as_ref())
            .map(|item| item.total_weight())
            .sum()
    }

    /// Get remaining capacity (kg)
    pub fn remaining_capacity(&self) -> f32 {
        MAX_INVENTORY_WEIGHT - self.total_weight()
    }

    /// Check if inventory is full
    pub fn is_full(&self) -> bool {
        self.slots.iter().all(|slot| slot.item.is_some())
    }

    /// Add item to inventory
    pub fn add_item(&mut self, item_type: ItemType, quantity: u32) -> Result<u32, String> {
        let item_weight = item_type.base_weight() * quantity as f32;

        if item_weight > self.remaining_capacity() {
            return Err(format!(
                "Not enough capacity! Need {:.1} kg, have {:.1} kg",
                item_weight,
                self.remaining_capacity()
            ));
        }

        // Try to stack with existing items first
        let mut remaining = quantity;

        for slot in &mut self.slots {
            if let Some(ref mut existing) = slot.item {
                if existing.item_type == item_type {
                    let max_stack = item_type.max_stack_size();
                    let space = max_stack - existing.quantity;

                    if space > 0 {
                        let add = remaining.min(space);
                        existing.quantity += add;
                        remaining -= add;

                        if remaining == 0 {
                            return Ok(0);
                        }
                    }
                }
            }
        }

        // Find empty slot for remaining items
        for slot in &mut self.slots {
            if slot.item.is_none() && remaining > 0 {
                let add = remaining.min(item_type.max_stack_size());
                slot.item = Some(InventoryItem::new(item_type, add));
                remaining -= add;

                if remaining == 0 {
                    return Ok(0);
                }
            }
        }

        // Return leftover if couldn't fit everything
        Ok(remaining)
    }

    /// Remove item from inventory
    pub fn remove_item(&mut self, item_type: ItemType, quantity: u32) -> Result<(), String> {
        let mut remaining = quantity;

        for slot in &mut self.slots {
            if let Some(ref mut item) = slot.item {
                if item.item_type == item_type {
                    if item.quantity >= remaining {
                        item.quantity -= remaining;

                        if item.quantity == 0 {
                            slot.item = None;
                        }

                        return Ok(());
                    } else {
                        remaining -= item.quantity;
                        slot.item = None;
                    }
                }
            }
        }

        Err(format!("Not enough {:?} in inventory", item_type))
    }

    /// Check if item exists in inventory
    pub fn has_item(&self, item_type: ItemType, quantity: u32) -> bool {
        let mut total = 0u32;

        for slot in &self.slots {
            if let Some(ref item) = slot.item {
                if item.item_type == item_type {
                    total += item.quantity;
                }
            }
        }

        total >= quantity
    }

    /// Get quantity of specific item
    pub fn get_quantity(&self, item_type: ItemType) -> u32 {
        self.slots
            .iter()
            .filter_map(|slot| slot.item.as_ref())
            .filter(|item| item.item_type == item_type)
            .map(|item| item.quantity)
            .sum()
    }

    /// Get all items
    pub fn get_all_items(&self) -> Vec<&InventoryItem> {
        self.slots
            .iter()
            .filter_map(|slot| slot.item.as_ref())
            .collect()
    }

    /// Clear inventory
    pub fn clear(&mut self) {
        for slot in &mut self.slots {
            slot.item = None;
        }
    }

    /// Get total value of all items
    pub fn total_value(&self) -> u32 {
        self.slots
            .iter()
            .filter_map(|slot| slot.item.as_ref())
            .map(|item| item.total_value())
            .sum()
    }
}

impl Default for Inventory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inventory_creation() {
        let inv = Inventory::new();
        assert_eq!(inv.slots.len(), MAX_INVENTORY_SLOTS);
        assert_eq!(inv.total_weight(), 0.0);
        assert_eq!(inv.money, 50000);
    }

    #[test]
    fn test_add_item() {
        let mut inv = Inventory::new();

        // Add wrench
        assert!(inv.add_item(ItemType::Wrench, 1).is_ok());
        assert_eq!(inv.get_quantity(ItemType::Wrench), 1);
        assert_eq!(inv.total_weight(), ItemType::Wrench.base_weight());
    }

    #[test]
    fn test_stacking() {
        let mut inv = Inventory::new();

        // Add multiple nails (should stack)
        inv.add_item(ItemType::Nail, 50).unwrap();
        inv.add_item(ItemType::Nail, 30).unwrap();

        assert_eq!(inv.get_quantity(ItemType::Nail), 80);

        // Check that it's in one or few slots
        let nail_slots = inv
            .slots
            .iter()
            .filter(|slot| {
                slot.item
                    .as_ref()
                    .map_or(false, |i| i.item_type == ItemType::Nail)
            })
            .count();
        assert_eq!(nail_slots, 1);
    }

    #[test]
    fn test_weight_limit() {
        let mut inv = Inventory::new();

        // Add steel beams until full
        let beam_weight = ItemType::SteelBeam.base_weight();
        let max_beams = (MAX_INVENTORY_WEIGHT / beam_weight) as u32;

        for _ in 0..max_beams {
            inv.add_item(ItemType::SteelBeam, 1).unwrap();
        }

        // Next should fail
        assert!(inv.add_item(ItemType::SteelBeam, 1).is_err());
    }

    #[test]
    fn test_remove_item() {
        let mut inv = Inventory::new();

        inv.add_item(ItemType::Wrench, 3).unwrap();
        assert_eq!(inv.get_quantity(ItemType::Wrench), 3);

        inv.remove_item(ItemType::Wrench, 2).unwrap();
        assert_eq!(inv.get_quantity(ItemType::Wrench), 1);

        inv.remove_item(ItemType::Wrench, 1).unwrap();
        assert_eq!(inv.get_quantity(ItemType::Wrench), 0);
    }
}
