#![allow(dead_code)]

use serde::{Deserialize, Serialize};

use super::item::ItemId;

// ── Map squares ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SquareKind {
    /// Empty path square — nothing happens here.
    Empty,
    /// Treasure: gold + items granted on entry.
    Treasure {
        gold: u32,
        items: Vec<(ItemId, u32)>,
    },
    /// Rest: heals each party member to max HP.
    Rest,
    /// Trap: party members roll DEX checks; failures take damage.
    Trap { damage: i32, dex_dc: i32 },
    /// Combat encounter (standard).
    Combat { encounter_id: String },
    /// Boss encounter (final challenge of the quest).
    Boss { encounter_id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestMap {
    pub width: u32,
    pub height: u32,
    /// squares[y][x] — row-major
    pub squares: Vec<Vec<SquareKind>>,
    pub start: (u32, u32),
    pub end: (u32, u32),
}

#[allow(dead_code)]
impl QuestMap {
    pub fn get(&self, x: u32, y: u32) -> Option<&SquareKind> {
        self.squares.get(y as usize)?.get(x as usize)
    }
}

// ── Enemies & encounters ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Enemy {
    pub name: String,
    pub max_hp: i32,
    pub strength: i32,
    pub xp_reward: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Encounter {
    pub id: String,
    pub enemies: Vec<Enemy>,
}

// ── Quest ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quest {
    pub id: String,
    pub name: String,
    pub description: String,
    pub difficulty: u32,        // 1..5
    pub min_party: u32,
    pub max_party: u32,
    pub recommended_level: u32,
    pub map: QuestMap,
    /// Loot granted on completion (in addition to treasure squares).
    pub completion_loot: Vec<(ItemId, u32)>,
    pub completion_gold: u32,
    pub xp_reward: u32,
    pub estimated_minutes: u32,
}

// ── Phase 3a quest data ───────────────────────────────────────────────────

pub fn register_quests() -> Vec<Quest> {
    vec![sunken_cellar()]
}

pub fn register_encounters() -> Vec<Encounter> {
    vec![
        Encounter {
            id: "cellar_rats".into(),
            enemies: vec![
                Enemy {
                    name: "Giant Rat".into(),
                    max_hp: 8,
                    strength: 3,
                    xp_reward: 5,
                },
                Enemy {
                    name: "Giant Rat".into(),
                    max_hp: 8,
                    strength: 3,
                    xp_reward: 5,
                },
            ],
        },
        Encounter {
            id: "rat_king".into(),
            enemies: vec![Enemy {
                name: "Rat King".into(),
                max_hp: 30,
                strength: 6,
                xp_reward: 25,
            }],
        },
    ]
}

fn sunken_cellar() -> Quest {
    use SquareKind::*;
    // 5x5 grid. Layout (y=0 at top, y=4 at bottom; player starts at bottom):
    //
    //   x=0  1  2  3  4
    // y=0  .  .  B  .  .   <- top, boss
    // y=1  .  C  .  T  .
    // y=2  .  .  R  .  .
    // y=3  T  .  .  .  C
    // y=4  .  .  S  .  .   <- bottom, start
    //
    // Note: end == boss square here. Reaching it triggers the boss fight.
    let combat_id = "cellar_rats".to_string();
    let boss_id = "rat_king".to_string();

    let row = |squares: Vec<SquareKind>| squares;

    let map = QuestMap {
        width: 5,
        height: 5,
        squares: vec![
            // y=0
            row(vec![
                Empty,
                Empty,
                Boss {
                    encounter_id: boss_id,
                },
                Empty,
                Empty,
            ]),
            // y=1
            row(vec![
                Empty,
                Combat {
                    encounter_id: combat_id.clone(),
                },
                Empty,
                Treasure {
                    gold: 8,
                    items: vec![("planks".into(), 2)],
                },
                Empty,
            ]),
            // y=2
            row(vec![Empty, Empty, Rest, Empty, Empty]),
            // y=3
            row(vec![
                Treasure {
                    gold: 5,
                    items: vec![("dried_herbs".into(), 2)],
                },
                Empty,
                Empty,
                Empty,
                Combat {
                    encounter_id: combat_id,
                },
            ]),
            // y=4
            row(vec![Empty, Empty, Empty, Empty, Empty]),
        ],
        start: (2, 4),
        end: (2, 0),
    };

    Quest {
        id: "sunken_cellar".into(),
        name: "The Sunken Cellar".into(),
        description: "A musty root cellar beneath an abandoned farmhouse. Something has been gnawing at the rafters... and the bones."
            .into(),
        difficulty: 1,
        min_party: 1,
        max_party: 2,
        recommended_level: 1,
        map,
        completion_loot: vec![("wood".into(), 5), ("herbs".into(), 3)],
        completion_gold: 20,
        xp_reward: 30,
        estimated_minutes: 5,
    }
}
