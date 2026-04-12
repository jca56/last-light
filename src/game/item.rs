use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize};

// ── ItemId ────────────────────────────────────────────────────────────────

/// A unique string identifier for an item type (e.g. "wood", "iron_ore").
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ItemId(pub String);

impl From<&str> for ItemId {
    fn from(s: &str) -> Self {
        ItemId(s.to_string())
    }
}

impl fmt::Display for ItemId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ── Category ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemCategory {
    RawMaterial,
    RefinedMaterial,
    Food,
    Drink,
    Weapon,
    Armor,
    Accessory,
    Reagent,
    Special,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
#[repr(u8)]
pub enum Rarity {
    #[default]
    Common = 0,
    Uncommon = 1,
    Rare = 2,
    VeryRare = 3,
}

// ── Properties ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GearStats {
    pub hp: i32,
    pub strength: i32,
    pub dexterity: i32,
    pub intellect: i32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ItemProperties {
    pub gear_stats: Option<GearStats>,
    pub food_servings: Option<u32>,
}

// ── ItemDef ───────────────────────────────────────────────────────────────

/// The immutable definition of an item type. Not serialized into saves.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemDef {
    pub id: ItemId,
    pub name: String,
    pub description: String,
    pub category: ItemCategory,
    #[serde(default)]
    pub rarity: Rarity,
    pub stack_limit: u32,
    pub gold_value: u32,
    pub tags: Vec<String>,
    pub properties: ItemProperties,
}

// ── Registry ──────────────────────────────────────────────────────────────

/// Catalog of all known item definitions. Rebuilt at startup, not saved.
pub struct ItemRegistry {
    items: HashMap<ItemId, ItemDef>,
}

#[allow(dead_code)]
impl ItemRegistry {
    pub fn new() -> Self {
        let mut reg = ItemRegistry {
            items: HashMap::new(),
        };
        register_items(&mut reg);
        reg
    }

    pub fn get(&self, id: &ItemId) -> Option<&ItemDef> {
        self.items.get(id)
    }

    pub fn register(&mut self, def: ItemDef) {
        self.items.insert(def.id.clone(), def);
    }

    pub fn items_by_category(&self, cat: &ItemCategory) -> Vec<&ItemDef> {
        self.items.values().filter(|d| &d.category == cat).collect()
    }

    pub fn contains(&self, id: &ItemId) -> bool {
        self.items.contains_key(id)
    }

    pub fn all(&self) -> &HashMap<ItemId, ItemDef> {
        &self.items
    }
}

// ── Phase 1 item data ─────────────────────────────────────────────────────

fn register_items(reg: &mut ItemRegistry) {
    reg.register(ItemDef {
        id: "wood".into(),
        name: "Wood".into(),
        description: "A rough log from the Whispering Woods.".into(),
        category: ItemCategory::RawMaterial,
        rarity: Rarity::Common,
        stack_limit: 999,
        gold_value: 1,
        tags: vec!["wood".into(), "tier_1".into()],
        properties: ItemProperties::default(),
    });

    reg.register(ItemDef {
        id: "herbs".into(),
        name: "Herbs".into(),
        description: "Fragrant wild herbs gathered from the forest floor.".into(),
        category: ItemCategory::RawMaterial,
        rarity: Rarity::Common,
        stack_limit: 999,
        gold_value: 2,
        tags: vec!["herb".into(), "tier_1".into()],
        properties: ItemProperties::default(),
    });

    reg.register(ItemDef {
        id: "berries".into(),
        name: "Berries".into(),
        description: "A handful of tart woodland berries.".into(),
        category: ItemCategory::RawMaterial,
        rarity: Rarity::Uncommon,
        stack_limit: 999,
        gold_value: 1,
        tags: vec!["food_raw".into(), "tier_1".into()],
        properties: ItemProperties::default(),
    });

    reg.register(ItemDef {
        id: "heartwood".into(),
        name: "Heartwood".into(),
        description: "A rare, dense core from an ancient whispering tree. Warm to the touch.".into(),
        category: ItemCategory::RawMaterial,
        rarity: Rarity::Rare,
        stack_limit: 99,
        gold_value: 15,
        tags: vec!["wood".into(), "tier_1".into()],
        properties: ItemProperties::default(),
    });

    // ── Refined materials ────────────────────────────────────────────────

    reg.register(ItemDef {
        id: "planks".into(),
        name: "Planks".into(),
        description: "Smooth wooden boards, ready to be shaped into something useful.".into(),
        category: ItemCategory::RefinedMaterial,
        rarity: Rarity::Common,
        stack_limit: 999,
        gold_value: 3,
        tags: vec!["wood".into(), "structural".into(), "tier_1".into()],
        properties: ItemProperties::default(),
    });

    reg.register(ItemDef {
        id: "kindling".into(),
        name: "Kindling".into(),
        description: "Bundled twigs and shavings. Catches a spark in a heartbeat.".into(),
        category: ItemCategory::RefinedMaterial,
        rarity: Rarity::Common,
        stack_limit: 999,
        gold_value: 2,
        tags: vec!["wood".into(), "fuel".into(), "tier_1".into()],
        properties: ItemProperties::default(),
    });

    reg.register(ItemDef {
        id: "dried_herbs".into(),
        name: "Dried Herbs".into(),
        description: "Hung and dried until fragrant. The backbone of any good stew.".into(),
        category: ItemCategory::RefinedMaterial,
        rarity: Rarity::Common,
        stack_limit: 999,
        gold_value: 4,
        tags: vec!["herb".into(), "food_ingredient".into(), "tier_1".into()],
        properties: ItemProperties::default(),
    });

    reg.register(ItemDef {
        id: "crushed_herbs".into(),
        name: "Crushed Herbs".into(),
        description: "Ground fine in a mortar. Releases its essence under heat.".into(),
        category: ItemCategory::RefinedMaterial,
        rarity: Rarity::Common,
        stack_limit: 999,
        gold_value: 5,
        tags: vec!["herb".into(), "alchemy_ingredient".into(), "tier_1".into()],
        properties: ItemProperties::default(),
    });

    reg.register(ItemDef {
        id: "dried_berries".into(),
        name: "Dried Berries".into(),
        description: "Sweet and chewy. Keeps for ages and travels well.".into(),
        category: ItemCategory::RefinedMaterial,
        rarity: Rarity::Common,
        stack_limit: 999,
        gold_value: 3,
        tags: vec!["food_ingredient".into(), "tier_1".into()],
        properties: ItemProperties::default(),
    });

    reg.register(ItemDef {
        id: "berry_juice".into(),
        name: "Berry Juice".into(),
        description: "Pressed and strained. Tart, vivid, and full of summer.".into(),
        category: ItemCategory::RefinedMaterial,
        rarity: Rarity::Common,
        stack_limit: 999,
        gold_value: 4,
        tags: vec!["drink_ingredient".into(), "tier_1".into()],
        properties: ItemProperties::default(),
    });

    // ── Crafted weapons ──────────────────────────────────────────────────

    reg.register(ItemDef {
        id: "wooden_club".into(),
        name: "Wooden Club".into(),
        description: "A simple cudgel of dense oak. Heavy enough to leave a mark.".into(),
        category: ItemCategory::Weapon,
        rarity: Rarity::Common,
        stack_limit: 99,
        gold_value: 12,
        tags: vec!["weapon".into(), "wood".into(), "tier_1".into()],
        properties: ItemProperties {
            gear_stats: Some(GearStats {
                strength: 2,
                ..GearStats::default()
            }),
            ..ItemProperties::default()
        },
    });

    reg.register(ItemDef {
        id: "hunters_bow".into(),
        name: "Hunter's Bow".into(),
        description: "A flexible bow strung with sinew. Quick in skilled hands.".into(),
        category: ItemCategory::Weapon,
        rarity: Rarity::Uncommon,
        stack_limit: 99,
        gold_value: 24,
        tags: vec!["weapon".into(), "ranged".into(), "tier_1".into()],
        properties: ItemProperties {
            gear_stats: Some(GearStats {
                dexterity: 2,
                ..GearStats::default()
            }),
            ..ItemProperties::default()
        },
    });

    reg.register(ItemDef {
        id: "herbalists_staff".into(),
        name: "Herbalist's Staff".into(),
        description: "A walking staff infused with crushed herbs. Faintly hums with old knowledge."
            .into(),
        category: ItemCategory::Weapon,
        rarity: Rarity::Uncommon,
        stack_limit: 99,
        gold_value: 28,
        tags: vec!["weapon".into(), "magic".into(), "tier_1".into()],
        properties: ItemProperties {
            gear_stats: Some(GearStats {
                intellect: 2,
                ..GearStats::default()
            }),
            ..ItemProperties::default()
        },
    });

    // ── Crafted armor ────────────────────────────────────────────────────

    reg.register(ItemDef {
        id: "bark_vest".into(),
        name: "Bark Vest".into(),
        description: "Hardened bark stitched into a protective tunic. Crude but sturdy.".into(),
        category: ItemCategory::Armor,
        rarity: Rarity::Common,
        stack_limit: 99,
        gold_value: 18,
        tags: vec!["armor".into(), "wood".into(), "tier_1".into()],
        properties: ItemProperties {
            gear_stats: Some(GearStats {
                hp: 5,
                ..GearStats::default()
            }),
            ..ItemProperties::default()
        },
    });

    reg.register(ItemDef {
        id: "herb_cloak".into(),
        name: "Herb Cloak".into(),
        description: "Dried herbs woven through coarse cloth. Wards off the cold and the curious."
            .into(),
        category: ItemCategory::Armor,
        rarity: Rarity::Uncommon,
        stack_limit: 99,
        gold_value: 26,
        tags: vec!["armor".into(), "magic".into(), "tier_1".into()],
        properties: ItemProperties {
            gear_stats: Some(GearStats {
                hp: 3,
                intellect: 1,
                ..GearStats::default()
            }),
            ..ItemProperties::default()
        },
    });

    // ── Crafted accessories ──────────────────────────────────────────────

    reg.register(ItemDef {
        id: "berry_pendant".into(),
        name: "Berry Pendant".into(),
        description: "A string of dried berries on twine. A traveler's lucky charm.".into(),
        category: ItemCategory::Accessory,
        rarity: Rarity::Common,
        stack_limit: 99,
        gold_value: 14,
        tags: vec!["accessory".into(), "tier_1".into()],
        properties: ItemProperties {
            gear_stats: Some(GearStats {
                hp: 1,
                dexterity: 1,
                ..GearStats::default()
            }),
            ..ItemProperties::default()
        },
    });

    reg.register(ItemDef {
        id: "heartwood_charm".into(),
        name: "Heartwood Charm".into(),
        description: "A polished sliver of heartwood, warm and pulsing with quiet power.".into(),
        category: ItemCategory::Accessory,
        rarity: Rarity::Rare,
        stack_limit: 99,
        gold_value: 60,
        tags: vec!["accessory".into(), "rare".into(), "tier_1".into()],
        properties: ItemProperties {
            gear_stats: Some(GearStats {
                hp: 2,
                intellect: 3,
                ..GearStats::default()
            }),
            ..ItemProperties::default()
        },
    });

    // ── Tavern food ──────────────────────────────────────────────────────

    reg.register(ItemDef {
        id: "hearty_stew".into(),
        name: "Hearty Stew".into(),
        description: "Thick, savory, and warming. The cure for a long day's road.".into(),
        category: ItemCategory::Food,
        rarity: Rarity::Common,
        stack_limit: 99,
        gold_value: 8,
        tags: vec!["food".into(), "tier_1".into()],
        properties: ItemProperties {
            food_servings: Some(1),
            ..ItemProperties::default()
        },
    });

    reg.register(ItemDef {
        id: "berry_tart".into(),
        name: "Berry Tart".into(),
        description: "A flaky pastry brimming with sweet berries. Disappears in three bites.".into(),
        category: ItemCategory::Food,
        rarity: Rarity::Common,
        stack_limit: 99,
        gold_value: 6,
        tags: vec!["food".into(), "dessert".into(), "tier_1".into()],
        properties: ItemProperties {
            food_servings: Some(1),
            ..ItemProperties::default()
        },
    });

    reg.register(ItemDef {
        id: "herb_bread".into(),
        name: "Herb Bread".into(),
        description: "A crusty loaf studded with dried herbs. Simple, honest fare.".into(),
        category: ItemCategory::Food,
        rarity: Rarity::Common,
        stack_limit: 99,
        gold_value: 5,
        tags: vec!["food".into(), "tier_1".into()],
        properties: ItemProperties {
            food_servings: Some(1),
            ..ItemProperties::default()
        },
    });

    // ── Tavern drinks ────────────────────────────────────────────────────

    reg.register(ItemDef {
        id: "berry_cordial".into(),
        name: "Berry Cordial".into(),
        description: "A sweet, dark drink that warms from the throat down. Mildly fortifying."
            .into(),
        category: ItemCategory::Drink,
        rarity: Rarity::Uncommon,
        stack_limit: 99,
        gold_value: 7,
        tags: vec!["drink".into(), "tier_1".into()],
        properties: ItemProperties::default(),
    });

    reg.register(ItemDef {
        id: "herbal_tea".into(),
        name: "Herbal Tea".into(),
        description: "Steaming and fragrant. Settles the nerves of even the weariest traveler."
            .into(),
        category: ItemCategory::Drink,
        rarity: Rarity::Common,
        stack_limit: 99,
        gold_value: 4,
        tags: vec!["drink".into(), "tier_1".into()],
        properties: ItemProperties::default(),
    });
}
