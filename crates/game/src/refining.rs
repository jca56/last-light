use serde::{Deserialize, Serialize};

use super::item::ItemId;
use super::time::{self, DurationMs, Timestamp, SECOND};

// ── Stations ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StationKind {
    Workbench,
    Furnace,
    Alchemy,
    Loom,
}

impl StationKind {
    pub fn label(self) -> &'static str {
        match self {
            StationKind::Workbench => "Workbench",
            StationKind::Furnace => "Furnace",
            StationKind::Alchemy => "Alchemy Table",
            StationKind::Loom => "Loom",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefiningStation {
    pub kind: StationKind,
    pub name: String,
    pub description: String,
    pub unlocked: bool,
}

// ── Recipes ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefiningRecipe {
    pub id: String,
    pub name: String,
    pub station: StationKind,
    pub input_id: ItemId,
    pub input_qty: u32,
    pub output_id: ItemId,
    pub output_qty: u32,
    pub duration_per_unit: DurationMs,
}

// ── Active task ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefiningTask {
    pub recipe_id: String,
    pub station: StationKind,
    pub total_units: u32,
    pub completed_units: u32,
    pub started_at: Timestamp,
    pub duration_per_unit: DurationMs,
}

#[allow(dead_code)]
impl RefiningTask {
    /// How many units the timer says should be done so far (capped at total).
    pub fn expected_units_done(&self) -> u32 {
        let elapsed = time::now().saturating_sub(self.started_at);
        let units = (elapsed / self.duration_per_unit) as u32;
        units.min(self.total_units)
    }

    pub fn is_batch_done(&self) -> bool {
        self.completed_units >= self.total_units
    }

    /// 0.0 to 1.0 progress within the currently-cooking unit.
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

    /// Time remaining until the *next* unit completes (ms). 0 if batch done.
    pub fn next_unit_remaining_ms(&self) -> DurationMs {
        if self.is_batch_done() {
            return 0;
        }
        let elapsed = time::now().saturating_sub(self.started_at);
        let next_completion = (self.completed_units as u64 + 1) * self.duration_per_unit;
        next_completion.saturating_sub(elapsed)
    }
}

// ── Refining state ────────────────────────────────────────────────────────

/// One slot per station kind. None = idle.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RefiningState {
    pub workbench: Option<RefiningTask>,
    pub furnace: Option<RefiningTask>,
    pub alchemy: Option<RefiningTask>,
    pub loom: Option<RefiningTask>,
}

#[allow(dead_code)]
impl RefiningState {
    pub fn slot(&self, kind: StationKind) -> &Option<RefiningTask> {
        match kind {
            StationKind::Workbench => &self.workbench,
            StationKind::Furnace => &self.furnace,
            StationKind::Alchemy => &self.alchemy,
            StationKind::Loom => &self.loom,
        }
    }

    pub fn slot_mut(&mut self, kind: StationKind) -> &mut Option<RefiningTask> {
        match kind {
            StationKind::Workbench => &mut self.workbench,
            StationKind::Furnace => &mut self.furnace,
            StationKind::Alchemy => &mut self.alchemy,
            StationKind::Loom => &mut self.loom,
        }
    }

    pub fn is_busy(&self, kind: StationKind) -> bool {
        self.slot(kind).is_some()
    }

    pub fn start(
        &mut self,
        kind: StationKind,
        recipe_id: String,
        total_units: u32,
        duration_per_unit: DurationMs,
    ) -> bool {
        if self.is_busy(kind) {
            return false;
        }
        *self.slot_mut(kind) = Some(RefiningTask {
            recipe_id,
            station: kind,
            total_units,
            completed_units: 0,
            started_at: time::now(),
            duration_per_unit,
        });
        true
    }

    pub fn dev_skip(&mut self) {
        let now = time::now();
        for kind in [
            StationKind::Workbench,
            StationKind::Furnace,
            StationKind::Alchemy,
            StationKind::Loom,
        ] {
            if let Some(task) = self.slot_mut(kind) {
                let needed = task.duration_per_unit * task.total_units as u64 + 1;
                task.started_at = now.saturating_sub(needed);
            }
        }
    }
}

// ── Phase 1 station + recipe data ─────────────────────────────────────────

pub fn register_stations() -> Vec<RefiningStation> {
    vec![
        RefiningStation {
            kind: StationKind::Workbench,
            name: "Workbench".into(),
            description:
                "A sturdy oak bench worn smooth by years of work. Boards, bundles, and bottles take shape here."
                    .into(),
            unlocked: true,
        },
        RefiningStation {
            kind: StationKind::Furnace,
            name: "Furnace".into(),
            description: "A cold hearth waiting for ore and coals. Locked until the smith's stones are laid."
                .into(),
            unlocked: false,
        },
        RefiningStation {
            kind: StationKind::Alchemy,
            name: "Alchemy Table".into(),
            description: "Glassware and reagent racks, all empty for now. The herbalist hasn't arrived."
                .into(),
            unlocked: false,
        },
        RefiningStation {
            kind: StationKind::Loom,
            name: "Loom".into(),
            description: "A heavy weaving frame in the corner. Awaits fibers and a steady hand.".into(),
            unlocked: false,
        },
    ]
}

pub fn register_recipes() -> Vec<RefiningRecipe> {
    vec![
        // ── Wood ────────────────────────────────────────────────
        RefiningRecipe {
            id: "wood_to_planks".into(),
            name: "Wood → Planks".into(),
            station: StationKind::Workbench,
            input_id: "wood".into(),
            input_qty: 1,
            output_id: "planks".into(),
            output_qty: 1,
            duration_per_unit: 30 * SECOND,
        },
        RefiningRecipe {
            id: "wood_to_kindling".into(),
            name: "Wood → Kindling".into(),
            station: StationKind::Workbench,
            input_id: "wood".into(),
            input_qty: 1,
            output_id: "kindling".into(),
            output_qty: 2,
            duration_per_unit: 10 * SECOND,
        },
        // ── Herbs ───────────────────────────────────────────────
        RefiningRecipe {
            id: "herbs_to_dried".into(),
            name: "Herbs → Dried Herbs".into(),
            station: StationKind::Workbench,
            input_id: "herbs".into(),
            input_qty: 1,
            output_id: "dried_herbs".into(),
            output_qty: 1,
            duration_per_unit: 15 * SECOND,
        },
        RefiningRecipe {
            id: "herbs_to_crushed".into(),
            name: "Herbs → Crushed Herbs".into(),
            station: StationKind::Workbench,
            input_id: "herbs".into(),
            input_qty: 1,
            output_id: "crushed_herbs".into(),
            output_qty: 1,
            duration_per_unit: 20 * SECOND,
        },
        // ── Berries ─────────────────────────────────────────────
        RefiningRecipe {
            id: "berries_to_dried".into(),
            name: "Berries → Dried Berries".into(),
            station: StationKind::Workbench,
            input_id: "berries".into(),
            input_qty: 1,
            output_id: "dried_berries".into(),
            output_qty: 1,
            duration_per_unit: 15 * SECOND,
        },
        RefiningRecipe {
            id: "berries_to_juice".into(),
            name: "Berries → Berry Juice".into(),
            station: StationKind::Workbench,
            input_id: "berries".into(),
            input_qty: 2,
            output_id: "berry_juice".into(),
            output_qty: 1,
            duration_per_unit: 20 * SECOND,
        },
    ]
}
