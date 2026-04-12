use serde::{Deserialize, Serialize};

use super::item::ItemId;
use super::time::{self, DurationMs, Timestamp, SECOND};

// ── Categories ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CraftingCategory {
    Weapons,
    Armor,
    Accessories,
    Consumables,
    Food,
    Drinks,
}

impl CraftingCategory {
    pub fn label(self) -> &'static str {
        match self {
            CraftingCategory::Weapons => "Weapons",
            CraftingCategory::Armor => "Armor",
            CraftingCategory::Accessories => "Accessories",
            CraftingCategory::Consumables => "Consumables",
            CraftingCategory::Food => "Food",
            CraftingCategory::Drinks => "Drinks",
        }
    }

    pub fn all() -> [CraftingCategory; 6] {
        [
            CraftingCategory::Weapons,
            CraftingCategory::Armor,
            CraftingCategory::Accessories,
            CraftingCategory::Consumables,
            CraftingCategory::Food,
            CraftingCategory::Drinks,
        ]
    }
}

// ── Recipes ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CraftingRecipe {
    pub id: String,
    pub name: String,
    pub category: CraftingCategory,
    /// One or more ingredients with quantity per crafted unit.
    pub inputs: Vec<(ItemId, u32)>,
    pub output_id: ItemId,
    pub output_qty: u32,
    pub duration_per_unit: DurationMs,
}

// ── Active task ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CraftingTask {
    pub recipe_id: String,
    pub total_units: u32,
    pub completed_units: u32,
    pub started_at: Timestamp,
    pub duration_per_unit: DurationMs,
}

#[allow(dead_code)]
impl CraftingTask {
    pub fn expected_units_done(&self) -> u32 {
        let elapsed = time::now().saturating_sub(self.started_at);
        let units = (elapsed / self.duration_per_unit) as u32;
        units.min(self.total_units)
    }

    pub fn is_batch_done(&self) -> bool {
        self.completed_units >= self.total_units
    }

    pub fn current_unit_progress(&self) -> f64 {
        if self.is_batch_done() {
            return 1.0;
        }
        let elapsed = time::now().saturating_sub(self.started_at);
        let unit_start = self.completed_units as u64 * self.duration_per_unit;
        if elapsed < unit_start {
            return 0.0;
        }
        let into_unit = elapsed - unit_start;
        (into_unit as f64 / self.duration_per_unit as f64).min(1.0)
    }

    pub fn next_unit_remaining_ms(&self) -> DurationMs {
        if self.is_batch_done() {
            return 0;
        }
        let elapsed = time::now().saturating_sub(self.started_at);
        let next_completion = (self.completed_units as u64 + 1) * self.duration_per_unit;
        next_completion.saturating_sub(elapsed)
    }
}

// ── Crafting state — single shared slot ──────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CraftingState {
    pub bench: Option<CraftingTask>,
}

#[allow(dead_code)]
impl CraftingState {
    pub fn is_busy(&self) -> bool {
        self.bench.is_some()
    }

    pub fn start(
        &mut self,
        recipe_id: String,
        total_units: u32,
        duration_per_unit: DurationMs,
    ) -> bool {
        if self.is_busy() {
            return false;
        }
        self.bench = Some(CraftingTask {
            recipe_id,
            total_units,
            completed_units: 0,
            started_at: time::now(),
            duration_per_unit,
        });
        true
    }

    pub fn dev_skip(&mut self) {
        if let Some(task) = &mut self.bench {
            let needed = task.duration_per_unit * task.total_units as u64 + 1;
            task.started_at = time::now().saturating_sub(needed);
        }
    }
}

// ── Phase 1 recipe data ───────────────────────────────────────────────────

pub fn register_recipes() -> Vec<CraftingRecipe> {
    vec![
        // ── Weapons ──────────────────────────────────────────────
        CraftingRecipe {
            id: "wooden_club".into(),
            name: "Wooden Club".into(),
            category: CraftingCategory::Weapons,
            inputs: vec![("planks".into(), 3)],
            output_id: "wooden_club".into(),
            output_qty: 1,
            duration_per_unit: 45 * SECOND,
        },
        CraftingRecipe {
            id: "hunters_bow".into(),
            name: "Hunter's Bow".into(),
            category: CraftingCategory::Weapons,
            inputs: vec![("planks".into(), 2), ("kindling".into(), 1)],
            output_id: "hunters_bow".into(),
            output_qty: 1,
            duration_per_unit: 60 * SECOND,
        },
        CraftingRecipe {
            id: "herbalists_staff".into(),
            name: "Herbalist's Staff".into(),
            category: CraftingCategory::Weapons,
            inputs: vec![("planks".into(), 2), ("crushed_herbs".into(), 2)],
            output_id: "herbalists_staff".into(),
            output_qty: 1,
            duration_per_unit: 60 * SECOND,
        },
        // ── Armor ────────────────────────────────────────────────
        CraftingRecipe {
            id: "bark_vest".into(),
            name: "Bark Vest".into(),
            category: CraftingCategory::Armor,
            inputs: vec![("planks".into(), 4)],
            output_id: "bark_vest".into(),
            output_qty: 1,
            duration_per_unit: 60 * SECOND,
        },
        CraftingRecipe {
            id: "herb_cloak".into(),
            name: "Herb Cloak".into(),
            category: CraftingCategory::Armor,
            inputs: vec![("dried_herbs".into(), 3), ("planks".into(), 1)],
            output_id: "herb_cloak".into(),
            output_qty: 1,
            duration_per_unit: 90 * SECOND,
        },
        // ── Accessories ──────────────────────────────────────────
        CraftingRecipe {
            id: "berry_pendant".into(),
            name: "Berry Pendant".into(),
            category: CraftingCategory::Accessories,
            inputs: vec![("dried_berries".into(), 5), ("kindling".into(), 1)],
            output_id: "berry_pendant".into(),
            output_qty: 1,
            duration_per_unit: 45 * SECOND,
        },
        CraftingRecipe {
            id: "heartwood_charm".into(),
            name: "Heartwood Charm".into(),
            category: CraftingCategory::Accessories,
            inputs: vec![("heartwood".into(), 1), ("planks".into(), 2)],
            output_id: "heartwood_charm".into(),
            output_qty: 1,
            duration_per_unit: 120 * SECOND,
        },
        // ── Food ────────────────────────────────────────────────
        CraftingRecipe {
            id: "hearty_stew".into(),
            name: "Hearty Stew".into(),
            category: CraftingCategory::Food,
            inputs: vec![
                ("dried_herbs".into(), 2),
                ("dried_berries".into(), 1),
                ("kindling".into(), 1),
            ],
            output_id: "hearty_stew".into(),
            output_qty: 1,
            duration_per_unit: 60 * SECOND,
        },
        CraftingRecipe {
            id: "berry_tart".into(),
            name: "Berry Tart".into(),
            category: CraftingCategory::Food,
            inputs: vec![("dried_berries".into(), 3), ("planks".into(), 1)],
            output_id: "berry_tart".into(),
            output_qty: 1,
            duration_per_unit: 45 * SECOND,
        },
        CraftingRecipe {
            id: "herb_bread".into(),
            name: "Herb Bread".into(),
            category: CraftingCategory::Food,
            inputs: vec![("dried_herbs".into(), 2), ("kindling".into(), 2)],
            output_id: "herb_bread".into(),
            output_qty: 1,
            duration_per_unit: 45 * SECOND,
        },
        // ── Drinks ──────────────────────────────────────────────
        CraftingRecipe {
            id: "berry_cordial".into(),
            name: "Berry Cordial".into(),
            category: CraftingCategory::Drinks,
            inputs: vec![("berry_juice".into(), 2), ("dried_berries".into(), 1)],
            output_id: "berry_cordial".into(),
            output_qty: 1,
            duration_per_unit: 45 * SECOND,
        },
        CraftingRecipe {
            id: "herbal_tea".into(),
            name: "Herbal Tea".into(),
            category: CraftingCategory::Drinks,
            inputs: vec![("crushed_herbs".into(), 1), ("kindling".into(), 1)],
            output_id: "herbal_tea".into(),
            output_qty: 1,
            duration_per_unit: 30 * SECOND,
        },
        // ── Consumables ─────────────────────────────────────────
        CraftingRecipe {
            id: "minor_healing_potion".into(),
            name: "Minor Healing Potion".into(),
            category: CraftingCategory::Consumables,
            inputs: vec![("crushed_herbs".into(), 2), ("berry_juice".into(), 1)],
            output_id: "minor_healing_potion".into(),
            output_qty: 1,
            duration_per_unit: 30 * SECOND,
        },
        CraftingRecipe {
            id: "healing_potion".into(),
            name: "Healing Potion".into(),
            category: CraftingCategory::Consumables,
            inputs: vec![("crushed_herbs".into(), 4), ("berry_juice".into(), 2)],
            output_id: "healing_potion".into(),
            output_qty: 1,
            duration_per_unit: 60 * SECOND,
        },
        CraftingRecipe {
            id: "strength_tonic".into(),
            name: "Strength Tonic".into(),
            category: CraftingCategory::Consumables,
            inputs: vec![("crushed_herbs".into(), 2), ("kindling".into(), 1)],
            output_id: "strength_tonic".into(),
            output_qty: 1,
            duration_per_unit: 45 * SECOND,
        },
        CraftingRecipe {
            id: "swiftfoot_elixir".into(),
            name: "Swiftfoot Elixir".into(),
            category: CraftingCategory::Consumables,
            inputs: vec![("crushed_herbs".into(), 2), ("berry_juice".into(), 1)],
            output_id: "swiftfoot_elixir".into(),
            output_qty: 1,
            duration_per_unit: 45 * SECOND,
        },
        CraftingRecipe {
            id: "magespark_draught".into(),
            name: "Magespark Draught".into(),
            category: CraftingCategory::Consumables,
            inputs: vec![("crushed_herbs".into(), 3), ("dried_herbs".into(), 1)],
            output_id: "magespark_draught".into(),
            output_qty: 1,
            duration_per_unit: 45 * SECOND,
        },
    ]
}
