#![allow(dead_code)]

use serde::{Deserialize, Serialize};

use super::item::ItemId;
use super::time::{self, DurationMs, Timestamp, SECOND};

// ── Constants ─────────────────────────────────────────────────────────────

pub const STARTING_TABLES: u32 = 4;
pub const SEATS_PER_TABLE: u32 = 2;
/// How many visitors can be in the tavern at once (tables × seats).
pub fn max_visitors(tables: u32) -> u32 {
    tables * SEATS_PER_TABLE
}

// ── Visitor ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Visitor {
    pub name: String,
    /// What they want to eat (item ID, or None if they don't want food).
    pub food_order: Option<ItemId>,
    /// What they want to drink.
    pub drink_order: Option<ItemId>,
    /// Gold they'll pay if served.
    pub gold_reward: u32,
    /// When they arrived.
    pub arrived_at: Timestamp,
    /// How long they'll wait before leaving unhappy (ms).
    pub patience: DurationMs,
    /// Whether they've been served food.
    pub food_served: bool,
    /// Whether they've been served drink.
    pub drink_served: bool,
}

impl Visitor {
    pub fn is_fully_served(&self) -> bool {
        (self.food_order.is_none() || self.food_served)
            && (self.drink_order.is_none() || self.drink_served)
    }

    pub fn is_patience_expired(&self) -> bool {
        time::now() >= self.arrived_at + self.patience
    }

    /// Time remaining before they leave.
    pub fn time_remaining(&self) -> DurationMs {
        let deadline = self.arrived_at + self.patience;
        let now = time::now();
        if now >= deadline {
            0
        } else {
            deadline - now
        }
    }
}

// ── Visitor types ─────────────────────────────────────────────────────────

/// Generate a random visitor with food/drink preferences.
pub fn generate_visitor(reputation: u32) -> Visitor {
    use rand::Rng;
    let mut rng = rand::rng();

    let names = [
        "A weary traveler",
        "A hooded stranger",
        "A local farmer",
        "A wandering bard",
        "A merchant",
        "A retired guard",
        "A curious scholar",
        "A young apprentice",
        "A grizzled hunter",
        "A cheerful dwarf",
    ];
    let name = names[rng.random_range(0..names.len())].to_string();

    // Food options (items the tavern can serve)
    let food_options: Vec<ItemId> = vec![
        "hearty_stew".into(),
        "berry_tart".into(),
        "herb_bread".into(),
    ];
    let drink_options: Vec<ItemId> = vec![
        "berry_cordial".into(),
        "herbal_tea".into(),
    ];

    // Most visitors want both food and drink, some want just one
    let wants_food = rng.random_range(0..100) < 80;
    let wants_drink = rng.random_range(0..100) < 70;

    let food_order = if wants_food {
        Some(food_options[rng.random_range(0..food_options.len())].clone())
    } else {
        None
    };
    let drink_order = if wants_drink || !wants_food {
        // At least one order
        Some(drink_options[rng.random_range(0..drink_options.len())].clone())
    } else {
        None
    };

    // Base gold: 3-8, scales slightly with reputation
    let base_gold = rng.random_range(3..=8);
    let rep_bonus = (reputation / 10).min(5);
    let gold_reward = base_gold + rep_bonus;

    // Patience: 60-120 seconds, slightly more with reputation
    let patience = rng.random_range(60..=120) * SECOND + (reputation as u64 * SECOND / 2);

    Visitor {
        name,
        food_order,
        drink_order,
        gold_reward,
        arrived_at: time::now(),
        patience,
        food_served: false,
        drink_served: false,
    }
}

// ── Upgrades ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TavernUpgrades {
    /// Number of tables (each seats SEATS_PER_TABLE visitors).
    pub tables: u32,
    /// Kitchen level: multiplies food gold value.
    pub kitchen_level: u32,
    /// Cellar level: multiplies drink gold value.
    pub cellar_level: u32,
    /// Rooms: passive gold per day cycle.
    pub rooms: u32,
    /// Noticeboard: unlocks higher quest tiers.
    pub noticeboard_level: u32,
}

impl Default for TavernUpgrades {
    fn default() -> Self {
        TavernUpgrades {
            tables: STARTING_TABLES,
            kitchen_level: 1,
            cellar_level: 1,
            rooms: 0,
            noticeboard_level: 0,
        }
    }
}

/// An upgrade the player can purchase.
#[derive(Debug, Clone)]
pub struct UpgradeDef {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub gold_cost: u32,
    pub material_cost: Vec<(ItemId, u32)>,
}

pub fn available_upgrades(upgrades: &TavernUpgrades) -> Vec<UpgradeDef> {
    let mut list = Vec::new();

    if upgrades.tables < 8 {
        let cost = (upgrades.tables + 1) * 30;
        list.push(UpgradeDef {
            id: "table",
            name: "Add Table",
            description: "Another table for visitors to sit at.",
            gold_cost: cost,
            material_cost: vec![("planks".into(), 4)],
        });
    }

    if upgrades.kitchen_level < 3 {
        let cost = upgrades.kitchen_level * 50;
        list.push(UpgradeDef {
            id: "kitchen",
            name: "Upgrade Kitchen",
            description: "Better kitchen — food earns more gold.",
            gold_cost: cost,
            material_cost: vec![("planks".into(), 6), ("kindling".into(), 3)],
        });
    }

    if upgrades.cellar_level < 3 {
        let cost = upgrades.cellar_level * 50;
        list.push(UpgradeDef {
            id: "cellar",
            name: "Upgrade Cellar",
            description: "Better cellar — drinks earn more gold.",
            gold_cost: cost,
            material_cost: vec![("planks".into(), 6)],
        });
    }

    if upgrades.rooms < 4 {
        let cost = (upgrades.rooms + 1) * 75;
        list.push(UpgradeDef {
            id: "room",
            name: "Add Room",
            description: "A room for rent — passive gold over time.",
            gold_cost: cost,
            material_cost: vec![("planks".into(), 8), ("dried_herbs".into(), 2)],
        });
    }

    list
}

// ── Tavern state ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TavernData {
    pub upgrades: TavernUpgrades,
    pub visitors: Vec<Visitor>,
    pub reputation: u32,
    /// Total gold earned from the tavern (lifetime stat).
    pub total_gold_earned: u32,
    /// Total visitors served (lifetime stat).
    pub total_served: u32,
    /// When the next visitor might arrive (ms timestamp).
    pub next_visitor_at: Timestamp,
    /// Auto-stock: the tavern automatically pulls from inventory.
    pub auto_stock: bool,
}

impl TavernData {
    pub fn new() -> Self {
        TavernData {
            upgrades: TavernUpgrades::default(),
            visitors: Vec::new(),
            reputation: 0,
            total_gold_earned: 0,
            total_served: 0,
            next_visitor_at: time::now() + 30 * SECOND,
            auto_stock: true,
        }
    }

    /// How many visitors can currently fit.
    pub fn capacity(&self) -> u32 {
        max_visitors(self.upgrades.tables)
    }

    /// How many visitors are currently seated.
    pub fn occupancy(&self) -> u32 {
        self.visitors.len() as u32
    }

    /// Schedule the next visitor arrival.
    pub fn schedule_next_visitor(&mut self) {
        use rand::Rng;
        let mut rng = rand::rng();
        // 30-120 seconds, faster with more reputation
        let base = rng.random_range(30..=120) as u64;
        let rep_speedup = (self.reputation as u64 / 5).min(40);
        let interval = (base.saturating_sub(rep_speedup)).max(15) * SECOND;
        self.next_visitor_at = time::now() + interval;
    }

    /// Gold multiplier from kitchen upgrades.
    pub fn food_multiplier(&self) -> f64 {
        1.0 + (self.upgrades.kitchen_level - 1) as f64 * 0.25
    }

    /// Gold multiplier from cellar upgrades.
    pub fn drink_multiplier(&self) -> f64 {
        1.0 + (self.upgrades.cellar_level - 1) as f64 * 0.25
    }

    /// Passive gold from rooms (per tick — called each game update).
    pub fn room_passive_gold(&self) -> u32 {
        // Rooms generate gold slowly — 1 gold per room per ~60 seconds
        // This is handled by the tick system, not here
        self.upgrades.rooms
    }
}
