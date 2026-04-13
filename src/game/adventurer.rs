#![allow(dead_code)]

use serde::{Deserialize, Serialize};

use super::item::{ItemId, ItemRegistry};
use super::time::Timestamp;

// ── Constants ────────────────────────────────────────────────────────────

pub const MAX_LEVEL: u32 = 20;
pub const CONSUMABLE_SLOTS: usize = 4;

// ── Class ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AdventurerClass {
    Warrior,
    Rogue,
    Mage,
    Paladin,
    Druid,
}

impl AdventurerClass {
    pub fn label(self) -> &'static str {
        match self {
            AdventurerClass::Warrior => "Warrior",
            AdventurerClass::Rogue => "Rogue",
            AdventurerClass::Mage => "Mage",
            AdventurerClass::Paladin => "Paladin",
            AdventurerClass::Druid => "Druid",
        }
    }

    /// Stat growth per level: (hp, str, dex, int).
    /// These are added to base_stats for each level above 1.
    pub fn growth(self) -> (i32, i32, i32, i32) {
        match self {
            //                    HP  STR DEX INT
            AdventurerClass::Warrior => (4, 2, 1, 0),
            AdventurerClass::Rogue => (2, 1, 3, 1),
            AdventurerClass::Mage => (1, 0, 1, 3),
            AdventurerClass::Paladin => (3, 2, 0, 2),
            AdventurerClass::Druid => (2, 1, 1, 2),
        }
    }
}

// ── XP table ─────────────────────────────────────────────────────────────

/// XP required to reach a given level (from level 1).
/// Index 0 = level 1 (0 XP), index 1 = level 2 (100 XP), etc.
/// Curve: base 100, scaling ~1.35× per level.
const XP_TABLE: [u32; 20] = [
    0,      // Level 1
    100,    // Level 2
    135,    // Level 3
    182,    // Level 4
    246,    // Level 5
    332,    // Level 6
    448,    // Level 7
    605,    // Level 8
    817,    // Level 9
    1_103,  // Level 10
    1_489,  // Level 11
    2_010,  // Level 12
    2_714,  // Level 13
    3_664,  // Level 14
    4_946,  // Level 15
    6_677,  // Level 16
    9_014,  // Level 17
    12_169, // Level 18
    16_428, // Level 19
    22_178, // Level 20
];

/// Cumulative XP needed to reach a given level (the "floor" for that level).
/// Level 1 = 0 XP, Level 2 = XP_TABLE[0]+XP_TABLE[1] = 100, etc.
pub fn xp_for_level(level: u32) -> u32 {
    if level <= 1 {
        return 0;
    }
    // Sum XP_TABLE entries for levels 1 through (level-1).
    // Index in XP_TABLE is (lvl - 1), so we sum indices 0 through (level-2).
    let end = (level as usize).saturating_sub(1).min(MAX_LEVEL as usize);
    XP_TABLE[..end].iter().sum()
}

/// XP needed to go from current level to the next.
pub fn xp_to_next_level(level: u32) -> Option<u32> {
    if level >= MAX_LEVEL {
        return None; // already max
    }
    Some(XP_TABLE[level as usize])
}

/// Given total XP, compute the level (1-based).
pub fn level_from_xp(xp: u32) -> u32 {
    let mut cumulative = 0u32;
    for lvl in 1..=MAX_LEVEL {
        let needed = XP_TABLE[(lvl - 1) as usize];
        cumulative += needed;
        if xp < cumulative {
            return lvl;
        }
    }
    MAX_LEVEL
}

/// XP progress within the current level: (current_in_level, needed_for_level).
/// Returns None if at max level.
pub fn xp_progress(xp: u32, level: u32) -> Option<(u32, u32)> {
    let needed = xp_to_next_level(level)?;
    let floor = xp_for_level(level);
    let progress = xp.saturating_sub(floor);
    Some((progress, needed))
}

// ── Status ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdventurerStatus {
    /// Available for a new quest.
    Ready,
    /// Currently in an active adventure.
    OnQuest,
    /// Recovering from injuries until the given timestamp.
    Recovering(Timestamp),
    /// Out of action — needs revival.
    Downed,
}

// ── Stats ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stats {
    pub max_hp: i32,
    pub strength: i32,
    pub dexterity: i32,
    pub intellect: i32,
}

// ── Equipment ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EquipSlot {
    Weapon,
    Armor,
    Accessory,
}

impl EquipSlot {
    pub fn label(self) -> &'static str {
        match self {
            EquipSlot::Weapon => "Weapon",
            EquipSlot::Armor => "Armor",
            EquipSlot::Accessory => "Accessory",
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Equipment {
    pub weapon: Option<ItemId>,
    pub armor: Option<ItemId>,
    pub accessory: Option<ItemId>,
}

#[allow(dead_code)]
impl Equipment {
    pub fn slot(&self, slot: EquipSlot) -> &Option<ItemId> {
        match slot {
            EquipSlot::Weapon => &self.weapon,
            EquipSlot::Armor => &self.armor,
            EquipSlot::Accessory => &self.accessory,
        }
    }

    pub fn slot_mut(&mut self, slot: EquipSlot) -> &mut Option<ItemId> {
        match slot {
            EquipSlot::Weapon => &mut self.weapon,
            EquipSlot::Armor => &mut self.armor,
            EquipSlot::Accessory => &mut self.accessory,
        }
    }

    pub fn equipped_ids(&self) -> Vec<&ItemId> {
        let mut out = Vec::new();
        if let Some(id) = &self.weapon {
            out.push(id);
        }
        if let Some(id) = &self.armor {
            out.push(id);
        }
        if let Some(id) = &self.accessory {
            out.push(id);
        }
        out
    }
}

// ── Adventurer ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Adventurer {
    pub id: String,
    pub name: String,
    pub class: AdventurerClass,
    pub level: u32,
    pub xp: u32,
    pub base_stats: Stats,
    pub equipment: Equipment,
    #[serde(default)]
    pub consumables: Vec<Option<ItemId>>,
    pub status: AdventurerStatus,
}

#[allow(dead_code)]
impl Adventurer {
    /// Combine base stats with level growth and equipped gear bonuses.
    pub fn effective_stats(&self, registry: &ItemRegistry) -> Stats {
        let mut s = self.base_stats.clone();

        // Level-based growth
        let levels_gained = self.level.saturating_sub(1) as i32;
        let (hp_g, str_g, dex_g, int_g) = self.class.growth();
        s.max_hp += hp_g * levels_gained;
        s.strength += str_g * levels_gained;
        s.dexterity += dex_g * levels_gained;
        s.intellect += int_g * levels_gained;

        // Gear bonuses
        for id in self.equipment.equipped_ids() {
            if let Some(def) = registry.get(id) {
                if let Some(g) = &def.properties.gear_stats {
                    s.max_hp += g.hp;
                    s.strength += g.strength;
                    s.dexterity += g.dexterity;
                    s.intellect += g.intellect;
                }
            }
        }
        s
    }

    /// Try to level up. Returns the new level if a level-up occurred.
    pub fn try_level_up(&mut self) -> Option<u32> {
        if self.level >= MAX_LEVEL {
            return None;
        }
        let new_level = level_from_xp(self.xp);
        if new_level > self.level {
            self.level = new_level;
            Some(new_level)
        } else {
            None
        }
    }

    /// Ensure consumable slots are initialised to the correct length.
    pub fn init_consumable_slots(&mut self) {
        if self.consumables.len() < CONSUMABLE_SLOTS {
            self.consumables.resize(CONSUMABLE_SLOTS, None);
        }
    }

    /// Equipped consumable item IDs (non-None slots).
    pub fn consumable_ids(&self) -> Vec<&ItemId> {
        self.consumables.iter().filter_map(|s| s.as_ref()).collect()
    }
}

// ── Phase 3a starter roster ───────────────────────────────────────────────

fn new_adventurer(
    id: &str,
    name: &str,
    class: AdventurerClass,
    base_stats: Stats,
) -> Adventurer {
    Adventurer {
        id: id.into(),
        name: name.into(),
        class,
        level: 1,
        xp: 0,
        base_stats,
        equipment: Equipment::default(),
        consumables: vec![None; CONSUMABLE_SLOTS],
        status: AdventurerStatus::Ready,
    }
}

pub fn register_adventurers() -> Vec<Adventurer> {
    vec![
        new_adventurer("torvald", "Torvald", AdventurerClass::Warrior, Stats {
            max_hp: 25, strength: 10, dexterity: 4, intellect: 3,
        }),
        new_adventurer("sylvara", "Sylvara", AdventurerClass::Rogue, Stats {
            max_hp: 15, strength: 5, dexterity: 12, intellect: 6,
        }),
        new_adventurer("ember", "Ember", AdventurerClass::Mage, Stats {
            max_hp: 12, strength: 3, dexterity: 5, intellect: 14,
        }),
        new_adventurer("aldric", "Aldric", AdventurerClass::Paladin, Stats {
            max_hp: 22, strength: 8, dexterity: 4, intellect: 7,
        }),
        new_adventurer("briar", "Briar", AdventurerClass::Druid, Stats {
            max_hp: 18, strength: 5, dexterity: 7, intellect: 11,
        }),
    ]
}
