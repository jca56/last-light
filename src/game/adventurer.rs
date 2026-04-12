#![allow(dead_code)]

use serde::{Deserialize, Serialize};

use super::item::{ItemId, ItemRegistry};
use super::time::Timestamp;

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
    pub status: AdventurerStatus,
}

#[allow(dead_code)]
impl Adventurer {
    /// Combine base stats with equipped gear bonuses.
    pub fn effective_stats(&self, registry: &ItemRegistry) -> Stats {
        let mut s = self.base_stats.clone();
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
}

// ── Phase 3a starter roster ───────────────────────────────────────────────

pub fn register_adventurers() -> Vec<Adventurer> {
    vec![
        Adventurer {
            id: "torvald".into(),
            name: "Torvald".into(),
            class: AdventurerClass::Warrior,
            level: 1,
            xp: 0,
            base_stats: Stats {
                max_hp: 25,
                strength: 10,
                dexterity: 4,
                intellect: 3,
            },
            equipment: Equipment::default(),
            status: AdventurerStatus::Ready,
        },
        Adventurer {
            id: "sylvara".into(),
            name: "Sylvara".into(),
            class: AdventurerClass::Rogue,
            level: 1,
            xp: 0,
            base_stats: Stats {
                max_hp: 15,
                strength: 5,
                dexterity: 12,
                intellect: 6,
            },
            equipment: Equipment::default(),
            status: AdventurerStatus::Ready,
        },
        Adventurer {
            id: "ember".into(),
            name: "Ember".into(),
            class: AdventurerClass::Mage,
            level: 1,
            xp: 0,
            base_stats: Stats {
                max_hp: 12,
                strength: 3,
                dexterity: 5,
                intellect: 14,
            },
            equipment: Equipment::default(),
            status: AdventurerStatus::Ready,
        },
        Adventurer {
            id: "aldric".into(),
            name: "Aldric".into(),
            class: AdventurerClass::Paladin,
            level: 1,
            xp: 0,
            base_stats: Stats {
                max_hp: 22,
                strength: 8,
                dexterity: 4,
                intellect: 7,
            },
            equipment: Equipment::default(),
            status: AdventurerStatus::Ready,
        },
        Adventurer {
            id: "briar".into(),
            name: "Briar".into(),
            class: AdventurerClass::Druid,
            level: 1,
            xp: 0,
            base_stats: Stats {
                max_hp: 18,
                strength: 5,
                dexterity: 7,
                intellect: 11,
            },
            equipment: Equipment::default(),
            status: AdventurerStatus::Ready,
        },
    ]
}
