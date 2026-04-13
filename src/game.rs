mod adventure;
mod adventurer;
mod crafting;
pub mod dungeon;
mod gathering;
mod inventory;
mod item;
mod quest;
mod refining;
mod save;
mod time;

pub use adventure::{
    resolve_enemy_turn, resolve_party_action, ActiveAdventure, AdventureState, CombatAction,
    CombatActor, CombatState, PartyMember,
};
#[allow(unused_imports)]
pub use adventurer::{
    Adventurer, AdventurerClass, AdventurerStatus, CONSUMABLE_SLOTS, MAX_LEVEL,
    level_from_xp, xp_for_level, xp_progress, xp_to_next_level,
};
pub use crafting::{CraftingCategory, CraftingRecipe, CraftingState};
pub use gathering::{GatherLocation, GatheringState};
pub use inventory::Inventory;
#[allow(unused_imports)]
pub use item::{ConsumableEffect, GearStats, ItemCategory, ItemDef, ItemId, ItemRegistry, Rarity};
pub use dungeon::DungeonDef;
pub use quest::{Encounter, Quest, QuestMap, SquareKind};
pub use refining::{RefiningRecipe, RefiningState, RefiningStation, StationKind};
pub use save::{load_game, save_game};
pub use time::{now, DurationMs, Timestamp};

use serde::{Deserialize, Serialize};

// ── Game events (ephemeral, not saved) ────────────────────────────────────

#[derive(Debug, Clone)]
pub enum GameEvent {
    GatherComplete {
        slot_index: usize,
        location_id: String,
        location: String,
        items: Vec<(ItemId, u32)>,
    },
    RefiningBatchDone {
        station: StationKind,
        recipe_name: String,
        output_id: ItemId,
        output_qty: u32,
        halted_for_lack_of_input: bool,
    },
    CraftingBatchDone {
        recipe_name: String,
        output_id: ItemId,
        output_qty: u32,
        halted_for_lack_of_input: bool,
    },
    LevelUp {
        adventurer_name: String,
        new_level: u32,
    },
}

// ── Immutable game definitions ────────────────────────────────────────────

pub struct GameData {
    pub item_registry: ItemRegistry,
    pub gather_locations: Vec<GatherLocation>,
    pub refining_stations: Vec<RefiningStation>,
    pub refining_recipes: Vec<RefiningRecipe>,
    pub crafting_recipes: Vec<CraftingRecipe>,
    pub quests: Vec<Quest>,
    pub encounters: Vec<Encounter>,
    pub dungeons: Vec<DungeonDef>,
}

impl GameData {
    pub fn new() -> Self {
        GameData {
            item_registry: ItemRegistry::new(),
            gather_locations: gathering::register_locations(),
            refining_stations: refining::register_stations(),
            refining_recipes: refining::register_recipes(),
            crafting_recipes: crafting::register_recipes(),
            quests: quest::register_quests(),
            encounters: quest::register_encounters(),
            dungeons: dungeon::register_dungeons(),
        }
    }

    pub fn location(&self, id: &str) -> Option<&GatherLocation> {
        self.gather_locations.iter().find(|loc| loc.id == id)
    }

    pub fn recipes_for_station(&self, kind: StationKind) -> Vec<&RefiningRecipe> {
        self.refining_recipes
            .iter()
            .filter(|r| r.station == kind)
            .collect()
    }

    pub fn recipe(&self, id: &str) -> Option<&RefiningRecipe> {
        self.refining_recipes.iter().find(|r| r.id == id)
    }

    pub fn crafting_recipes_in(&self, cat: CraftingCategory) -> Vec<&CraftingRecipe> {
        self.crafting_recipes
            .iter()
            .filter(|r| r.category == cat)
            .collect()
    }

    pub fn crafting_recipe(&self, id: &str) -> Option<&CraftingRecipe> {
        self.crafting_recipes.iter().find(|r| r.id == id)
    }

    pub fn quest(&self, id: &str) -> Option<&Quest> {
        self.quests.iter().find(|q| q.id == id)
    }

    pub fn encounter(&self, id: &str) -> Option<&Encounter> {
        self.encounters.iter().find(|e| e.id == id)
    }
}

// ── Persistent game state ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub gold: u32,
    pub day: u32,
    pub inventory: Inventory,
    pub gathering: GatheringState,
    #[serde(default)]
    pub refining: RefiningState,
    #[serde(default)]
    pub crafting: CraftingState,
    #[serde(default = "default_adventurers")]
    pub adventurers: Vec<Adventurer>,
    #[serde(default)]
    pub active_adventure: Option<ActiveAdventure>,
    pub last_updated: Timestamp,
}

fn default_adventurers() -> Vec<Adventurer> {
    adventurer::register_adventurers()
}

impl GameState {
    pub fn new() -> Self {
        GameState {
            gold: 50,
            day: 1,
            inventory: Inventory::default(),
            gathering: GatheringState::new(),
            refining: RefiningState::default(),
            crafting: CraftingState::default(),
            adventurers: adventurer::register_adventurers(),
            active_adventure: None,
            last_updated: now(),
        }
    }

    /// Ensure adventurer data is up-to-date after loading a save.
    pub fn migrate_adventurers(&mut self) {
        for adv in &mut self.adventurers {
            adv.init_consumable_slots();
            // Re-derive level from XP in case the XP table changed
            let derived = adventurer::level_from_xp(adv.xp);
            if derived > adv.level {
                adv.level = derived;
            }
        }
    }

    /// Apply XP to party members and check for level-ups. Returns level-up events.
    pub fn apply_adventure_xp(&mut self, party: &[PartyMember], xp: u32) -> Vec<GameEvent> {
        let mut events = Vec::new();
        for member in party {
            if let Some(adv) = self.adventurers.get_mut(member.roster_idx) {
                adv.xp = adv.xp.saturating_add(xp);
                if let Some(new_level) = adv.try_level_up() {
                    events.push(GameEvent::LevelUp {
                        adventurer_name: adv.name.clone(),
                        new_level,
                    });
                }
            }
        }
        events
    }

    /// Tick the game forward. Returns events that occurred (for the UI log).
    pub fn update(&mut self, data: &GameData) -> Vec<GameEvent> {
        let mut events = Vec::new();

        // ── Gathering ──────────────────────────────────────
        for slot_idx in 0..self.gathering.slots.len() {
            let is_complete = self.gathering.slots[slot_idx]
                .as_ref()
                .is_some_and(|t| t.is_complete());

            if is_complete {
                let task = self.gathering.slots[slot_idx].take().unwrap();
                if let Some(location) = data.location(&task.location_id) {
                    let drop_table = &location.durations[task.duration_index].drop_table;
                    let drops = drop_table.roll();
                    for (item_id, qty) in &drops {
                        self.inventory.add(item_id, *qty);
                    }
                    events.push(GameEvent::GatherComplete {
                        slot_index: slot_idx,
                        location_id: location.id.clone(),
                        location: location.name.clone(),
                        items: drops,
                    });
                }
            }
        }

        // ── Refining ───────────────────────────────────────
        for kind in [
            StationKind::Workbench,
            StationKind::Furnace,
            StationKind::Alchemy,
            StationKind::Loom,
        ] {
            self.tick_refining_station(kind, data, &mut events);
        }

        // ── Crafting ───────────────────────────────────────
        self.tick_crafting(data, &mut events);

        self.last_updated = now();
        events
    }

    fn tick_crafting(&mut self, data: &GameData, events: &mut Vec<GameEvent>) {
        if self.crafting.bench.is_none() {
            return;
        }

        let (recipe_id, expected, completed_so_far, total_units) = {
            let task = self.crafting.bench.as_ref().unwrap();
            (
                task.recipe_id.clone(),
                task.expected_units_done(),
                task.completed_units,
                task.total_units,
            )
        };

        let Some(recipe) = data.crafting_recipe(&recipe_id) else {
            self.crafting.bench = None;
            return;
        };

        let mut newly_done: u32 = 0;
        let mut halted = false;

        for _ in completed_so_far..expected {
            // Check we have all inputs
            let can_afford = recipe
                .inputs
                .iter()
                .all(|(id, qty)| self.inventory.has(id, *qty));
            if !can_afford {
                halted = true;
                break;
            }
            // Deduct all inputs
            for (id, qty) in &recipe.inputs {
                self.inventory.remove(id, *qty);
            }
            // Deposit output
            self.inventory.add(&recipe.output_id, recipe.output_qty);
            newly_done += 1;
        }

        if newly_done > 0 {
            if let Some(task) = &mut self.crafting.bench {
                task.completed_units += newly_done;
            }
        }

        let task_finished = self.crafting.bench.as_ref().unwrap().completed_units >= total_units;

        if task_finished || halted {
            let final_completed = self.crafting.bench.as_ref().unwrap().completed_units;
            let total_output = final_completed * recipe.output_qty;
            self.crafting.bench = None;
            if total_output > 0 || halted {
                events.push(GameEvent::CraftingBatchDone {
                    recipe_name: recipe.name.clone(),
                    output_id: recipe.output_id.clone(),
                    output_qty: total_output,
                    halted_for_lack_of_input: halted,
                });
            }
        }
    }

    fn tick_refining_station(
        &mut self,
        kind: StationKind,
        data: &GameData,
        events: &mut Vec<GameEvent>,
    ) {
        // Take the slot to avoid borrowing self mutably twice
        let slot_was_some = self.refining.slot(kind).is_some();
        if !slot_was_some {
            return;
        }

        // Snapshot needed values
        let (recipe_id, expected, completed_so_far, total_units) = {
            let task = self.refining.slot(kind).as_ref().unwrap();
            (
                task.recipe_id.clone(),
                task.expected_units_done(),
                task.completed_units,
                task.total_units,
            )
        };

        let Some(recipe) = data.recipe(&recipe_id) else {
            *self.refining.slot_mut(kind) = None;
            return;
        };

        let mut newly_done: u32 = 0;
        let mut halted = false;

        for _ in completed_so_far..expected {
            if self.inventory.remove(&recipe.input_id, recipe.input_qty) {
                self.inventory.add(&recipe.output_id, recipe.output_qty);
                newly_done += 1;
            } else {
                halted = true;
                break;
            }
        }

        if newly_done > 0 {
            if let Some(task) = self.refining.slot_mut(kind) {
                task.completed_units += newly_done;
            }
        }

        // Check whether the batch is done (or halted)
        let task_finished = {
            let task = self.refining.slot(kind).as_ref().unwrap();
            task.completed_units >= total_units
        };

        if task_finished || halted {
            let final_completed = self.refining.slot(kind).as_ref().unwrap().completed_units;
            // Only emit if at least one unit was produced overall, OR if it was halted with zero
            // (in either case we want to clear the slot)
            let total_output = final_completed * recipe.output_qty;
            *self.refining.slot_mut(kind) = None;
            if total_output > 0 || halted {
                events.push(GameEvent::RefiningBatchDone {
                    station: kind,
                    recipe_name: recipe.name.clone(),
                    output_id: recipe.output_id.clone(),
                    output_qty: total_output,
                    halted_for_lack_of_input: halted,
                });
            }
        }
    }

    /// Dev tool: instantly complete all active timers.
    pub fn dev_skip_all(&mut self) {
        let current = now();
        for slot in &mut self.gathering.slots {
            if let Some(task) = slot {
                task.started_at = current.saturating_sub(task.duration_ms + 1);
            }
        }
        self.refining.dev_skip();
        self.crafting.dev_skip();
    }
}
