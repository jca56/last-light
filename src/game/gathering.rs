use rand::Rng;
use serde::{Deserialize, Serialize};

use super::item::ItemId;
use super::time::{self, DurationMs, Timestamp, MINUTE, SECOND};

// ── Drop tables ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DropEntry {
    pub item_id: ItemId,
    pub min_qty: u32,
    pub max_qty: u32,
    /// Relative probability weight (higher = more likely).
    pub weight: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DropTable {
    /// Always dropped on completion.
    pub guaranteed: Vec<DropEntry>,
    /// Weighted random selection pool.
    pub random_pool: Vec<DropEntry>,
    /// Number of random picks from the pool.
    pub random_picks: u32,
}

impl DropTable {
    pub fn roll(&self) -> Vec<(ItemId, u32)> {
        let mut rng = rand::rng();
        let mut drops: Vec<(ItemId, u32)> = Vec::new();

        // Guaranteed drops
        for entry in &self.guaranteed {
            let qty = rng.random_range(entry.min_qty..=entry.max_qty);
            if qty > 0 {
                drops.push((entry.item_id.clone(), qty));
            }
        }

        // Random picks
        if !self.random_pool.is_empty() {
            let total_weight: u32 = self.random_pool.iter().map(|e| e.weight).sum();
            if total_weight > 0 {
                for _ in 0..self.random_picks {
                    let roll = rng.random_range(0..total_weight);
                    let mut cumulative = 0;
                    for entry in &self.random_pool {
                        cumulative += entry.weight;
                        if roll < cumulative {
                            let qty = rng.random_range(entry.min_qty..=entry.max_qty);
                            if qty > 0 {
                                drops.push((entry.item_id.clone(), qty));
                            }
                            break;
                        }
                    }
                }
            }
        }

        // Merge duplicates
        let mut merged: Vec<(ItemId, u32)> = Vec::new();
        for (id, qty) in drops {
            if let Some(existing) = merged.iter_mut().find(|(eid, _)| *eid == id) {
                existing.1 += qty;
            } else {
                merged.push((id, qty));
            }
        }

        merged
    }
}

// ── Locations ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatherDuration {
    pub label: String,
    pub duration_ms: DurationMs,
    pub drop_table: DropTable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatherLocation {
    pub id: String,
    pub name: String,
    pub description: String,
    pub unlocked: bool,
    pub durations: Vec<GatherDuration>,
}

// ── Active tasks ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatherTask {
    pub location_id: String,
    pub duration_index: usize,
    pub started_at: Timestamp,
    pub duration_ms: DurationMs,
}

impl GatherTask {
    pub fn is_complete(&self) -> bool {
        time::is_complete(self.started_at, self.duration_ms)
    }

    pub fn remaining_ms(&self) -> DurationMs {
        time::remaining(self.started_at, self.duration_ms)
    }

    pub fn progress(&self) -> f64 {
        time::progress_fraction(self.started_at, self.duration_ms)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatheringState {
    pub slots: Vec<Option<GatherTask>>,
    pub max_slots: usize,
}

impl Default for GatheringState {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl GatheringState {
    pub fn new() -> Self {
        GatheringState {
            slots: vec![None],
            max_slots: 1,
        }
    }

    pub fn start(
        &mut self,
        slot: usize,
        location_id: String,
        duration_index: usize,
        duration_ms: DurationMs,
    ) -> bool {
        if slot >= self.slots.len() || self.slots[slot].is_some() {
            return false;
        }
        self.slots[slot] = Some(GatherTask {
            location_id,
            duration_index,
            started_at: time::now(),
            duration_ms,
        });
        true
    }

    pub fn complete(&mut self, slot: usize) -> Option<GatherTask> {
        if slot >= self.slots.len() {
            return None;
        }
        self.slots[slot].take()
    }

    pub fn has_empty_slot(&self) -> bool {
        self.slots.iter().any(|s| s.is_none())
    }

    pub fn first_empty_slot(&self) -> Option<usize> {
        self.slots.iter().position(|s| s.is_none())
    }

    pub fn add_slot(&mut self) {
        if self.slots.len() < self.max_slots {
            self.slots.push(None);
        }
    }
}

// ── Phase 1 location data ─────────────────────────────────────────────────

pub fn register_locations() -> Vec<GatherLocation> {
    vec![GatherLocation {
        id: "whispering_woods".into(),
        name: "Whispering Woods".into(),
        description: "Ancient trees murmur secrets to those who listen. Rich in wood, herbs, and wild berries.".into(),
        unlocked: true,
        durations: vec![
            GatherDuration {
                label: "Quick Forage".into(),
                duration_ms: 10 * SECOND,
                drop_table: DropTable {
                    guaranteed: vec![DropEntry {
                        item_id: "wood".into(),
                        min_qty: 1,
                        max_qty: 2,
                        weight: 0,
                    }],
                    random_pool: vec![DropEntry {
                        item_id: "herbs".into(),
                        min_qty: 1,
                        max_qty: 1,
                        weight: 20,
                    }],
                    random_picks: 1,
                },
            },
            GatherDuration {
                label: "Standard Expedition".into(),
                duration_ms: 45 * SECOND,
                drop_table: DropTable {
                    guaranteed: vec![
                        DropEntry {
                            item_id: "wood".into(),
                            min_qty: 3,
                            max_qty: 5,
                            weight: 0,
                        },
                        DropEntry {
                            item_id: "herbs".into(),
                            min_qty: 1,
                            max_qty: 1,
                            weight: 0,
                        },
                    ],
                    random_pool: vec![DropEntry {
                        item_id: "berries".into(),
                        min_qty: 1,
                        max_qty: 2,
                        weight: 70,
                    }],
                    random_picks: 1,
                },
            },
            GatherDuration {
                label: "Deep Expedition".into(),
                duration_ms: 3 * MINUTE,
                drop_table: DropTable {
                    guaranteed: vec![
                        DropEntry {
                            item_id: "wood".into(),
                            min_qty: 5,
                            max_qty: 8,
                            weight: 0,
                        },
                        DropEntry {
                            item_id: "herbs".into(),
                            min_qty: 2,
                            max_qty: 3,
                            weight: 0,
                        },
                        DropEntry {
                            item_id: "berries".into(),
                            min_qty: 1,
                            max_qty: 2,
                            weight: 0,
                        },
                    ],
                    random_pool: vec![DropEntry {
                        item_id: "heartwood".into(),
                        min_qty: 1,
                        max_qty: 1,
                        weight: 5,
                    }],
                    random_picks: 1,
                },
            },
        ],
    }]
}
