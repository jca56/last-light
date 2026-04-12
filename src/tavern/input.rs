//! Keyboard input handling — top-level dispatch and per-view handlers.

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::style::Style;

use super::state::{
    sorted_inventory_items, AdventureScreen, BottomTab, Focus, GatheringScreen, PartySetupFocus,
    RefiningScreen, RosterFocus, TavernState, Transition, View, NAV_ITEMS, TRANSITION_FRAMES,
};
use crate::game::{self, AdventureState, CombatAction, CombatActor, GameData, GameState};
use crate::ui;

/// Top-level input handler. Returns `true` if the game should quit.
pub(super) fn handle_input(
    state: &mut TavernState,
    game_state: &mut GameState,
    data: &GameData,
    key: KeyCode,
    mods: KeyModifiers,
) -> bool {
    // ── Highest priority: Ctrl+C always quits immediately ─────────────
    if key == KeyCode::Char('c') && mods.contains(KeyModifiers::CONTROL) {
        return true;
    }

    // ── Quit prompt modal — intercepts everything ─────────────────────
    if state.quit_prompt_open {
        match key {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => return true,
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                state.quit_prompt_open = false;
                return false;
            }
            _ => return false,
        }
    }

    // ── If input is focused, route everything to the text input ───────
    // The only escape hatches are Esc (unfocus) and Ctrl+C (quit, handled above).
    if state.focus == Focus::Input {
        if key == KeyCode::Esc {
            state.focus = Focus::Terminal;
            return false;
        }
        handle_text_input(state, key);
        return false;
    }

    // ── Esc: context-aware ────────────────────────────────────────────
    if key == KeyCode::Esc {
        if state.current_view == View::Gathering
            && state.gathering_view.screen == GatheringScreen::AtLocation
            && state.gathering_view.transition == Transition::None
        {
            state.gathering_view.transition = Transition::LeavingLocation(TRANSITION_FRAMES);
            return false;
        }
        if state.current_view == View::Refining
            && state.refining_view.screen == RefiningScreen::AtStation
        {
            state.refining_view.screen = RefiningScreen::Stations;
            return false;
        }
        if state.current_view == View::Adventuring {
            // Adventures has its own back-out logic per sub-screen
            if adventure_esc(state, game_state) {
                return false;
            }
        }
        // No sub-state to back out of — open the quit confirmation prompt
        state.quit_prompt_open = true;
        return false;
    }

    // ── Dev skip: F12 ────────────────────────────────────────────────
    if key == KeyCode::F(12) {
        game_state.dev_skip_all();
        state.log_messages.push((
            "[DEV] Skipped all timers.".into(),
            Style::default().fg(ui::FLAME),
        ));
        state.auto_scroll(20);
        return false;
    }

    // ── Numbered hotkeys 1-8: switch view ────────────────────────────
    // Special case: 1 on Terminal view focuses the input bar instead of "switching" to Terminal.
    if let KeyCode::Char(c) = key {
        if let Some(idx) = char_to_view_index(c) {
            if idx < NAV_ITEMS.len() {
                let target_view = NAV_ITEMS[idx].0;
                if target_view == View::Terminal && state.current_view == View::Terminal {
                    state.focus = Focus::Input;
                } else {
                    state.current_view = target_view;
                }
                return false;
            }
        }
    }

    // ── Bottom-left active panel tab cycling: [ and ] ────────────────
    if key == KeyCode::Char('[') {
        state.bottom_tab = state.bottom_tab.prev();
        return false;
    }
    if key == KeyCode::Char(']') {
        state.bottom_tab = state.bottom_tab.next();
        return false;
    }

    // ── Per-view input ───────────────────────────────────────────────
    match state.current_view {
        View::Gathering => handle_gathering_input(state, game_state, data, key),
        View::Refining => handle_refining_input(state, game_state, data, key),
        View::Crafting => handle_crafting_input(state, game_state, data, key, mods),
        View::Adventuring => handle_adventure_input(state, game_state, data, key),
        View::Inventory => handle_inventory_input(state, game_state, data, key),
        _ => handle_terminal_input(state, key),
    }

    false
}

/// Convert a digit char ('1'..='8') to a NAV_ITEMS index.
fn char_to_view_index(c: char) -> Option<usize> {
    match c {
        '1' => Some(0),
        '2' => Some(1),
        '3' => Some(2),
        '4' => Some(3),
        '5' => Some(4),
        '6' => Some(5),
        '7' => Some(6),
        '8' => Some(7),
        _ => None,
    }
}

fn handle_terminal_input(state: &mut TavernState, key: KeyCode) {
    match key {
        KeyCode::Up => {
            if state.log_scroll > 0 {
                state.log_scroll -= 1;
            }
        }
        KeyCode::Down => {
            state.log_scroll += 1;
        }
        _ => {}
    }
}

fn handle_text_input(state: &mut TavernState, key: KeyCode) {
    match key {
        KeyCode::Char(c) => {
            state.input.insert(state.cursor, c);
            state.cursor += 1;
        }
        KeyCode::Backspace => {
            if state.cursor > 0 {
                state.cursor -= 1;
                state.input.remove(state.cursor);
            }
        }
        KeyCode::Delete => {
            if state.cursor < state.input.len() {
                state.input.remove(state.cursor);
            }
        }
        KeyCode::Left => {
            if state.cursor > 0 {
                state.cursor -= 1;
            }
        }
        KeyCode::Right => {
            if state.cursor < state.input.len() {
                state.cursor += 1;
            }
        }
        KeyCode::Home => state.cursor = 0,
        KeyCode::End => state.cursor = state.input.len(),
        KeyCode::Enter => {
            if !state.input.is_empty() {
                let text = state.input.clone();
                state
                    .log_messages
                    .push((format!("> {}", text), Style::default().fg(ui::GOLD)));
                state.input.clear();
                state.cursor = 0;
                state.auto_scroll(20);
            }
        }
        _ => {}
    }
}

fn handle_gathering_input(
    state: &mut TavernState,
    game_state: &mut GameState,
    data: &GameData,
    key: KeyCode,
) {
    // Block input during transitions
    if state.gathering_view.transition != Transition::None {
        return;
    }

    match state.gathering_view.screen {
        GatheringScreen::Grounds => match key {
            // 2x2 grid navigation — always 4 slots (indices 0-3)
            KeyCode::Left => {
                let cur = state.gathering_view.selected_location;
                if cur % 2 == 1 {
                    state.gathering_view.selected_location = cur - 1;
                }
            }
            KeyCode::Right => {
                let cur = state.gathering_view.selected_location;
                if cur % 2 == 0 && cur + 1 < 4 {
                    state.gathering_view.selected_location = cur + 1;
                }
            }
            KeyCode::Up => {
                let cur = state.gathering_view.selected_location;
                if cur >= 2 {
                    state.gathering_view.selected_location = cur - 2;
                }
            }
            KeyCode::Down => {
                let cur = state.gathering_view.selected_location;
                if cur + 2 < 4 {
                    state.gathering_view.selected_location = cur + 2;
                }
            }
            KeyCode::Enter => {
                let idx = state.gathering_view.selected_location;
                if let Some(loc) = data.gather_locations.get(idx) {
                    if loc.unlocked {
                        state.gathering_view.current_location = idx;
                        state.gathering_view.selected_duration = 0;
                        state.gathering_view.transition =
                            Transition::EnteringLocation(TRANSITION_FRAMES);
                    }
                }
            }
            _ => {}
        },
        GatheringScreen::AtLocation => {
            let loc_idx = state.gathering_view.current_location;
            let Some(location) = data.gather_locations.get(loc_idx) else {
                return;
            };
            match key {
                KeyCode::Up => {
                    if state.gathering_view.selected_duration > 0 {
                        state.gathering_view.selected_duration -= 1;
                    }
                }
                KeyCode::Down => {
                    let max = location.durations.len().saturating_sub(1);
                    if state.gathering_view.selected_duration < max {
                        state.gathering_view.selected_duration += 1;
                    }
                }
                KeyCode::Enter => {
                    if let Some(slot) = game_state.gathering.first_empty_slot() {
                        let dur_idx = state.gathering_view.selected_duration;
                        if let Some(dur) = location.durations.get(dur_idx) {
                            let started = game_state.gathering.start(
                                slot,
                                location.id.clone(),
                                dur_idx,
                                dur.duration_ms,
                            );
                            if started {
                                state.bottom_tab = BottomTab::Expeditions;
                                state.log_messages.push((
                                    format!("Started {} at {}.", dur.label, location.name),
                                    Style::default().fg(ui::WARM_WHITE),
                                ));
                                state.auto_scroll(20);
                            }
                        }
                    } else {
                        state.log_messages.push((
                            "All gathering slots are full.".into(),
                            Style::default().fg(ui::DIM),
                        ));
                        state.auto_scroll(20);
                    }
                }
                _ => {}
            }
        }
    }
}

fn handle_refining_input(
    state: &mut TavernState,
    game_state: &mut GameState,
    data: &GameData,
    key: KeyCode,
) {
    match state.refining_view.screen {
        RefiningScreen::Stations => match key {
            // 2x2 grid navigation — always 4 stations (indices 0-3)
            KeyCode::Left => {
                let cur = state.refining_view.selected_station;
                if cur % 2 == 1 {
                    state.refining_view.selected_station = cur - 1;
                }
            }
            KeyCode::Right => {
                let cur = state.refining_view.selected_station;
                if cur % 2 == 0 && cur + 1 < 4 {
                    state.refining_view.selected_station = cur + 1;
                }
            }
            KeyCode::Up => {
                let cur = state.refining_view.selected_station;
                if cur >= 2 {
                    state.refining_view.selected_station = cur - 2;
                }
            }
            KeyCode::Down => {
                let cur = state.refining_view.selected_station;
                if cur + 2 < 4 {
                    state.refining_view.selected_station = cur + 2;
                }
            }
            KeyCode::Enter => {
                let idx = state.refining_view.selected_station;
                if let Some(station) = data.refining_stations.get(idx) {
                    if station.unlocked {
                        state.refining_view.current_station = idx;
                        state.refining_view.selected_recipe = 0;
                        state.refining_view.quantity = 1;
                        state.refining_view.screen = RefiningScreen::AtStation;
                    }
                }
            }
            _ => {}
        },
        RefiningScreen::AtStation => {
            let station_idx = state.refining_view.current_station;
            let Some(station) = data.refining_stations.get(station_idx) else {
                return;
            };
            let kind = station.kind;
            let recipes = data.recipes_for_station(kind);
            if recipes.is_empty() {
                return;
            }

            match key {
                KeyCode::Up => {
                    if state.refining_view.selected_recipe > 0 {
                        state.refining_view.selected_recipe -= 1;
                        state.refining_view.quantity = 1;
                    }
                }
                KeyCode::Down => {
                    if state.refining_view.selected_recipe + 1 < recipes.len() {
                        state.refining_view.selected_recipe += 1;
                        state.refining_view.quantity = 1;
                    }
                }
                KeyCode::Left => {
                    if state.refining_view.quantity > 1 {
                        state.refining_view.quantity -= 1;
                    }
                }
                KeyCode::Right => {
                    let recipe = recipes[state.refining_view.selected_recipe];
                    let max = max_quantity_for_recipe(game_state, recipe);
                    if state.refining_view.quantity < max {
                        state.refining_view.quantity += 1;
                    }
                }
                KeyCode::PageUp => {
                    if state.refining_view.quantity > 5 {
                        state.refining_view.quantity -= 5;
                    } else {
                        state.refining_view.quantity = 1;
                    }
                }
                KeyCode::PageDown => {
                    let recipe = recipes[state.refining_view.selected_recipe];
                    let max = max_quantity_for_recipe(game_state, recipe);
                    state.refining_view.quantity = (state.refining_view.quantity + 5).min(max);
                }
                KeyCode::Home => {
                    state.refining_view.quantity = 1;
                }
                KeyCode::End => {
                    let recipe = recipes[state.refining_view.selected_recipe];
                    state.refining_view.quantity =
                        max_quantity_for_recipe(game_state, recipe).max(1);
                }
                KeyCode::Enter => {
                    let recipe = recipes[state.refining_view.selected_recipe];
                    let qty = state.refining_view.quantity;
                    let max = max_quantity_for_recipe(game_state, recipe);
                    if game_state.refining.is_busy(kind) {
                        state.log_messages.push((
                            format!("The {} is already in use.", kind.label()),
                            Style::default().fg(ui::DIM),
                        ));
                        state.auto_scroll(20);
                    } else if qty == 0 || max == 0 {
                        state.log_messages.push((
                            format!(
                                "Not enough {} to start.",
                                data.item_registry
                                    .get(&recipe.input_id)
                                    .map(|d| d.name.as_str())
                                    .unwrap_or(recipe.input_id.0.as_str())
                            ),
                            Style::default().fg(ui::DIM),
                        ));
                        state.auto_scroll(20);
                    } else {
                        let started = game_state.refining.start(
                            kind,
                            recipe.id.clone(),
                            qty,
                            recipe.duration_per_unit,
                        );
                        if started {
                            state.bottom_tab = BottomTab::Refining;
                            state.log_messages.push((
                                format!(
                                    "Started {} × {} at the {}.",
                                    recipe.name,
                                    qty,
                                    kind.label()
                                ),
                                Style::default().fg(ui::WARM_WHITE),
                            ));
                            state.auto_scroll(20);
                            state.refining_view.quantity = 1;
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

/// Maximum number of units the player can afford to refine for a recipe.
fn max_quantity_for_recipe(game_state: &GameState, recipe: &game::RefiningRecipe) -> u32 {
    if recipe.input_qty == 0 {
        return 0;
    }
    game_state.inventory.count(&recipe.input_id) / recipe.input_qty
}

fn handle_crafting_input(
    state: &mut TavernState,
    game_state: &mut GameState,
    data: &GameData,
    key: KeyCode,
    mods: KeyModifiers,
) {
    let categories = game::CraftingCategory::all();
    let cur_cat_idx = state.crafting_view.selected_category.min(categories.len() - 1);
    let cur_cat = categories[cur_cat_idx];
    let recipes = data.crafting_recipes_in(cur_cat);

    match key {
        // Tab cycles to next category, Shift+Tab to previous
        KeyCode::Tab => {
            if mods.contains(KeyModifiers::SHIFT) {
                state.crafting_view.selected_category =
                    (cur_cat_idx + categories.len() - 1) % categories.len();
            } else {
                state.crafting_view.selected_category = (cur_cat_idx + 1) % categories.len();
            }
            state.crafting_view.selected_recipe = 0;
            state.crafting_view.quantity = 1;
        }
        KeyCode::BackTab => {
            state.crafting_view.selected_category =
                (cur_cat_idx + categories.len() - 1) % categories.len();
            state.crafting_view.selected_recipe = 0;
            state.crafting_view.quantity = 1;
        }
        KeyCode::Up => {
            if state.crafting_view.selected_recipe > 0 {
                state.crafting_view.selected_recipe -= 1;
                state.crafting_view.quantity = 1;
            }
        }
        KeyCode::Down => {
            if !recipes.is_empty() && state.crafting_view.selected_recipe + 1 < recipes.len() {
                state.crafting_view.selected_recipe += 1;
                state.crafting_view.quantity = 1;
            }
        }
        KeyCode::Left => {
            if state.crafting_view.quantity > 1 {
                state.crafting_view.quantity -= 1;
            }
        }
        KeyCode::Right => {
            if let Some(recipe) = recipes.get(state.crafting_view.selected_recipe) {
                let max = max_craft_quantity(game_state, recipe);
                if state.crafting_view.quantity < max {
                    state.crafting_view.quantity += 1;
                }
            }
        }
        KeyCode::PageUp => {
            if state.crafting_view.quantity > 5 {
                state.crafting_view.quantity -= 5;
            } else {
                state.crafting_view.quantity = 1;
            }
        }
        KeyCode::PageDown => {
            if let Some(recipe) = recipes.get(state.crafting_view.selected_recipe) {
                let max = max_craft_quantity(game_state, recipe);
                state.crafting_view.quantity = (state.crafting_view.quantity + 5).min(max);
            }
        }
        KeyCode::Home => state.crafting_view.quantity = 1,
        KeyCode::End => {
            if let Some(recipe) = recipes.get(state.crafting_view.selected_recipe) {
                let max = max_craft_quantity(game_state, recipe);
                state.crafting_view.quantity = max.max(1);
            }
        }
        KeyCode::Enter => {
            let Some(recipe) = recipes.get(state.crafting_view.selected_recipe).copied() else {
                return;
            };
            let qty = state.crafting_view.quantity;
            let max = max_craft_quantity(game_state, recipe);
            if game_state.crafting.is_busy() {
                state.log_messages.push((
                    "The crafting bench is in use.".into(),
                    Style::default().fg(ui::DIM),
                ));
                state.auto_scroll(20);
            } else if qty == 0 || max == 0 {
                state.log_messages.push((
                    format!("Not enough materials to craft {}.", recipe.name),
                    Style::default().fg(ui::DIM),
                ));
                state.auto_scroll(20);
            } else {
                let started = game_state.crafting.start(
                    recipe.id.clone(),
                    qty,
                    recipe.duration_per_unit,
                );
                if started {
                    state.bottom_tab = BottomTab::Crafting;
                    state.log_messages.push((
                        format!("Started crafting {} × {}.", recipe.name, qty),
                        Style::default().fg(ui::WARM_WHITE),
                    ));
                    state.auto_scroll(20);
                    state.crafting_view.quantity = 1;
                }
            }
        }
        _ => {}
    }
}

/// Maximum number of units the player can afford to craft for a recipe.
fn max_craft_quantity(game_state: &GameState, recipe: &game::CraftingRecipe) -> u32 {
    let mut min_max = u32::MAX;
    for (id, qty) in &recipe.inputs {
        if *qty == 0 {
            continue;
        }
        let have = game_state.inventory.count(id);
        let possible = have / qty;
        if possible < min_max {
            min_max = possible;
        }
    }
    if min_max == u32::MAX {
        0
    } else {
        min_max
    }
}

fn handle_inventory_input(
    state: &mut TavernState,
    game_state: &mut GameState,
    data: &GameData,
    key: KeyCode,
) {
    let items = sorted_inventory_items(game_state, data);
    if items.is_empty() {
        return;
    }
    let count = items.len();
    let cols = state.inventory_view.last_grid_cols.max(1);
    let current = state.inventory_view.selected.min(count - 1);

    match key {
        KeyCode::Left => {
            if current > 0 {
                state.inventory_view.selected = current - 1;
            }
        }
        KeyCode::Right => {
            if current + 1 < count {
                state.inventory_view.selected = current + 1;
            }
        }
        KeyCode::Up => {
            if current >= cols {
                state.inventory_view.selected = current - cols;
            }
        }
        KeyCode::Down => {
            if current + cols < count {
                state.inventory_view.selected = current + cols;
            }
        }
        KeyCode::Home => {
            state.inventory_view.selected = 0;
        }
        KeyCode::End => {
            state.inventory_view.selected = count - 1;
        }
        _ => {}
    }
}

// ── Adventure input ───────────────────────────────────────────────────────

/// Handle Esc in the Adventures view based on current sub-screen.
/// Returns true if the key was consumed.
fn adventure_esc(state: &mut TavernState, game_state: &mut GameState) -> bool {
    match state.adventure_view.screen {
        AdventureScreen::Roster => {
            if state.adventure_view.roster_focus == RosterFocus::Equipment {
                state.adventure_view.roster_focus = RosterFocus::List;
                return true;
            }
            false // let it quit
        }
        AdventureScreen::QuestBoard => false, // let it quit
        AdventureScreen::PartySetup => {
            if state.adventure_view.picking_adventurer {
                state.adventure_view.picking_adventurer = false;
            } else {
                // Return any equipped items from currently-selected party back to inventory
                // (we don't, since equipping moves items between inventory and adventurer)
                // Just go back to quest board
                state.adventure_view.screen = AdventureScreen::QuestBoard;
            }
            true
        }
        AdventureScreen::InAdventure => {
            // Abandon quest — return to quest board, restore party
            abandon_active_adventure(game_state);
            state.tile_graphics.cleanup_all();
            state.adventure_view.screen = AdventureScreen::QuestBoard;
            true
        }
        AdventureScreen::Combat => {
            // Can't escape combat with esc — must use Flee action
            true
        }
        AdventureScreen::Results => {
            // Treat as "press Enter" — apply rewards and return to quest board
            finish_active_adventure(game_state);
            state.tile_graphics.cleanup_all();
            state.adventure_view.screen = AdventureScreen::QuestBoard;
            true
        }
    }
}

fn handle_adventure_input(
    state: &mut TavernState,
    game_state: &mut GameState,
    data: &GameData,
    key: KeyCode,
) {
    match state.adventure_view.screen {
        AdventureScreen::Roster => handle_roster(state, game_state, key),
        AdventureScreen::QuestBoard => handle_quest_board(state, data, key),
        AdventureScreen::PartySetup => handle_party_setup(state, game_state, data, key),
        AdventureScreen::InAdventure => handle_in_adventure(state, game_state, data, key),
        AdventureScreen::Combat => handle_combat(state, game_state, data, key),
        AdventureScreen::Results => handle_results(state, game_state, key),
    }
}

fn handle_roster(state: &mut TavernState, game_state: &mut GameState, key: KeyCode) {
    let count = game_state.adventurers.len();
    if count == 0 {
        return;
    }
    match state.adventure_view.roster_focus {
        RosterFocus::List => match key {
            KeyCode::Up => {
                if state.adventure_view.selected_adventurer > 0 {
                    state.adventure_view.selected_adventurer -= 1;
                }
            }
            KeyCode::Down => {
                if state.adventure_view.selected_adventurer + 1 < count {
                    state.adventure_view.selected_adventurer += 1;
                }
            }
            KeyCode::Enter | KeyCode::Right => {
                // Enter the equipment panel for the selected adventurer
                state.adventure_view.roster_focus = RosterFocus::Equipment;
                state.adventure_view.roster_equip_slot = 0;
            }
            KeyCode::Tab => {
                state.adventure_view.screen = AdventureScreen::QuestBoard;
            }
            _ => {}
        },
        RosterFocus::Equipment => match key {
            KeyCode::Up => {
                if state.adventure_view.roster_equip_slot > 0 {
                    state.adventure_view.roster_equip_slot -= 1;
                }
            }
            KeyCode::Down => {
                if state.adventure_view.roster_equip_slot < 2 {
                    state.adventure_view.roster_equip_slot += 1;
                }
            }
            KeyCode::Enter => {
                // Cycle to the next compatible item from inventory
                let adv_idx = state.adventure_view.selected_adventurer;
                let equip_slot = state.adventure_view.roster_equip_slot;
                // Reuse the same cycle_equipment function from party setup
                cycle_equipment_for_roster(game_state, adv_idx, equip_slot);
            }
            KeyCode::Char('x') | KeyCode::Char('X') => {
                let adv_idx = state.adventure_view.selected_adventurer;
                let equip_slot = state.adventure_view.roster_equip_slot;
                unequip_slot(game_state, adv_idx, equip_slot);
            }
            KeyCode::Esc | KeyCode::Left => {
                state.adventure_view.roster_focus = RosterFocus::List;
            }
            KeyCode::Tab => {
                state.adventure_view.roster_focus = RosterFocus::List;
                state.adventure_view.screen = AdventureScreen::QuestBoard;
            }
            _ => {}
        },
    }
}

/// Cycle equipment for the roster view. Same logic as the party setup
/// cycle_equipment but doesn't need GameData for filtering — we just match
/// by item category directly.
fn cycle_equipment_for_roster(
    game_state: &mut GameState,
    adv_idx: usize,
    equip_slot: usize,
) {
    let target_category = match equip_slot {
        0 => game::ItemCategory::Weapon,
        1 => game::ItemCategory::Armor,
        2 => game::ItemCategory::Accessory,
        _ => return,
    };

    // Find compatible items in inventory
    // (We need the registry to check categories, but we can access it via a fresh one)
    let registry = game::ItemRegistry::new();
    let compatible: Vec<game::ItemId> = game_state
        .inventory
        .items()
        .iter()
        .filter_map(|(id, qty)| {
            if *qty == 0 {
                return None;
            }
            let def = registry.get(id)?;
            if def.category == target_category {
                Some(id.clone())
            } else {
                None
            }
        })
        .collect();

    if compatible.is_empty() {
        return;
    }

    let Some(adv) = game_state.adventurers.get_mut(adv_idx) else {
        return;
    };

    let current_id = match equip_slot {
        0 => adv.equipment.weapon.clone(),
        1 => adv.equipment.armor.clone(),
        2 => adv.equipment.accessory.clone(),
        _ => return,
    };

    let next_id = match &current_id {
        None => compatible[0].clone(),
        Some(curr) => {
            let pos = compatible.iter().position(|id| id == curr);
            match pos {
                Some(i) => compatible[(i + 1) % compatible.len()].clone(),
                None => compatible[0].clone(),
            }
        }
    };

    // Return old item
    if let Some(prev) = current_id {
        game_state.inventory.add(&prev, 1);
    }

    // Take new item
    if !game_state.inventory.remove(&next_id, 1) {
        return;
    }

    let Some(adv) = game_state.adventurers.get_mut(adv_idx) else {
        return;
    };
    match equip_slot {
        0 => adv.equipment.weapon = Some(next_id),
        1 => adv.equipment.armor = Some(next_id),
        2 => adv.equipment.accessory = Some(next_id),
        _ => {}
    }
}

fn handle_quest_board(state: &mut TavernState, data: &GameData, key: KeyCode) {
    match key {
        KeyCode::Tab => {
            state.adventure_view.screen = AdventureScreen::Roster;
            return;
        }
        _ => {}
    }
    if data.quests.is_empty() {
        return;
    }
    match key {
        KeyCode::Up => {
            if state.adventure_view.selected_quest > 0 {
                state.adventure_view.selected_quest -= 1;
            }
        }
        KeyCode::Down => {
            if state.adventure_view.selected_quest + 1 < data.quests.len() {
                state.adventure_view.selected_quest += 1;
            }
        }
        KeyCode::Enter => {
            // Move to Party Setup, reset selections
            state.adventure_view.screen = AdventureScreen::PartySetup;
            state.adventure_view.party_slots = [None, None, None];
            state.adventure_view.setup_slot = 0;
            state.adventure_view.setup_focus = PartySetupFocus::PartySlots;
        }
        _ => {}
    }
}

fn handle_party_setup(
    state: &mut TavernState,
    game_state: &mut GameState,
    data: &GameData,
    key: KeyCode,
) {
    // Picker mode: navigating the adventurer roster to assign to a slot
    if state.adventure_view.picking_adventurer {
        match key {
            KeyCode::Up => {
                if state.adventure_view.picker_idx > 0 {
                    state.adventure_view.picker_idx -= 1;
                }
            }
            KeyCode::Down => {
                if state.adventure_view.picker_idx + 1 < game_state.adventurers.len() {
                    state.adventure_view.picker_idx += 1;
                }
            }
            KeyCode::Enter => {
                let adv_idx = state.adventure_view.picker_idx;
                // Don't allow assigning the same adventurer to two slots
                let already_in_party = state
                    .adventure_view
                    .party_slots
                    .iter()
                    .any(|s| *s == Some(adv_idx));
                if !already_in_party {
                    state.adventure_view.party_slots[state.adventure_view.setup_slot] =
                        Some(adv_idx);
                }
                state.adventure_view.picking_adventurer = false;
            }
            _ => {}
        }
        return;
    }

    match key {
        KeyCode::Tab => {
            state.adventure_view.setup_focus = match state.adventure_view.setup_focus {
                PartySetupFocus::PartySlots => PartySetupFocus::EquipmentSlots,
                PartySetupFocus::EquipmentSlots => PartySetupFocus::PartySlots,
            };
        }
        KeyCode::Up => match state.adventure_view.setup_focus {
            PartySetupFocus::PartySlots => {
                if state.adventure_view.setup_slot > 0 {
                    state.adventure_view.setup_slot -= 1;
                }
            }
            PartySetupFocus::EquipmentSlots => {
                if state.adventure_view.setup_equip_slot > 0 {
                    state.adventure_view.setup_equip_slot -= 1;
                }
            }
        },
        KeyCode::Down => match state.adventure_view.setup_focus {
            PartySetupFocus::PartySlots => {
                if state.adventure_view.setup_slot < 2 {
                    state.adventure_view.setup_slot += 1;
                }
            }
            PartySetupFocus::EquipmentSlots => {
                if state.adventure_view.setup_equip_slot < 2 {
                    state.adventure_view.setup_equip_slot += 1;
                }
            }
        },
        KeyCode::Enter => match state.adventure_view.setup_focus {
            PartySetupFocus::PartySlots => {
                // Open adventurer picker
                state.adventure_view.picking_adventurer = true;
                state.adventure_view.picker_idx = 0;
            }
            PartySetupFocus::EquipmentSlots => {
                // Cycle to next equippable item from inventory for current slot
                let slot_idx = state.adventure_view.setup_slot;
                let equip_slot = state.adventure_view.setup_equip_slot;
                if let Some(adv_idx) = state.adventure_view.party_slots[slot_idx] {
                    cycle_equipment(game_state, data, adv_idx, equip_slot);
                }
            }
        },
        KeyCode::Char('x') | KeyCode::Char('X') => {
            // Clear selected slot or unequip selected gear
            match state.adventure_view.setup_focus {
                PartySetupFocus::PartySlots => {
                    state.adventure_view.party_slots[state.adventure_view.setup_slot] = None;
                }
                PartySetupFocus::EquipmentSlots => {
                    let slot_idx = state.adventure_view.setup_slot;
                    let equip_slot = state.adventure_view.setup_equip_slot;
                    if let Some(adv_idx) = state.adventure_view.party_slots[slot_idx] {
                        unequip_slot(game_state, adv_idx, equip_slot);
                    }
                }
            }
        }
        KeyCode::Char('s') | KeyCode::Char('S') => {
            // Start the adventure
            try_start_adventure(state, game_state, data);
        }
        _ => {}
    }
}

/// Cycle through equippable items in inventory for the given equip slot.
/// Each press equips the next compatible item (returning the previous to inventory).
fn cycle_equipment(
    game_state: &mut GameState,
    data: &GameData,
    adv_idx: usize,
    equip_slot: usize,
) {
    let target_category = match equip_slot {
        0 => game::ItemCategory::Weapon,
        1 => game::ItemCategory::Armor,
        2 => game::ItemCategory::Accessory,
        _ => return,
    };

    // Find compatible items in inventory
    let compatible: Vec<game::ItemId> = game_state
        .inventory
        .items()
        .iter()
        .filter_map(|(id, qty)| {
            if *qty == 0 {
                return None;
            }
            let def = data.item_registry.get(id)?;
            if def.category == target_category {
                Some(id.clone())
            } else {
                None
            }
        })
        .collect();

    if compatible.is_empty() {
        return;
    }

    let Some(adv) = game_state.adventurers.get_mut(adv_idx) else {
        return;
    };

    // Get currently equipped item id (if any)
    let current_id = match equip_slot {
        0 => adv.equipment.weapon.clone(),
        1 => adv.equipment.armor.clone(),
        2 => adv.equipment.accessory.clone(),
        _ => return,
    };

    // Pick the next item (or first if currently None)
    let next_id: game::ItemId = match &current_id {
        None => compatible[0].clone(),
        Some(curr) => {
            let pos = compatible.iter().position(|id| id == curr);
            match pos {
                Some(i) => compatible[(i + 1) % compatible.len()].clone(),
                None => compatible[0].clone(),
            }
        }
    };

    // Return the previous item to inventory
    if let Some(prev) = current_id {
        game_state.inventory.add(&prev, 1);
    }

    // Take the new item from inventory
    if !game_state.inventory.remove(&next_id, 1) {
        return;
    }

    // Re-borrow adv (was borrowed during compatible iteration above)
    let Some(adv) = game_state.adventurers.get_mut(adv_idx) else {
        return;
    };
    match equip_slot {
        0 => adv.equipment.weapon = Some(next_id),
        1 => adv.equipment.armor = Some(next_id),
        2 => adv.equipment.accessory = Some(next_id),
        _ => {}
    }
}

fn unequip_slot(game_state: &mut GameState, adv_idx: usize, equip_slot: usize) {
    let Some(adv) = game_state.adventurers.get_mut(adv_idx) else {
        return;
    };
    let removed = match equip_slot {
        0 => adv.equipment.weapon.take(),
        1 => adv.equipment.armor.take(),
        2 => adv.equipment.accessory.take(),
        _ => None,
    };
    if let Some(id) = removed {
        game_state.inventory.add(&id, 1);
    }
}

fn try_start_adventure(state: &mut TavernState, game_state: &mut GameState, data: &GameData) {
    let Some(quest) = data.quests.get(state.adventure_view.selected_quest) else {
        return;
    };
    let party_count: u32 = state
        .adventure_view
        .party_slots
        .iter()
        .filter(|s| s.is_some())
        .count() as u32;
    if party_count < quest.min_party {
        state.log_messages.push((
            format!("Need at least {} adventurer(s) for this quest.", quest.min_party),
            ratatui::style::Style::default().fg(ui::DIM),
        ));
        state.auto_scroll(20);
        return;
    }
    if party_count > quest.max_party {
        state.log_messages.push((
            format!("Maximum {} adventurer(s) for this quest.", quest.max_party),
            ratatui::style::Style::default().fg(ui::DIM),
        ));
        state.auto_scroll(20);
        return;
    }

    // Build party members from selected adventurers
    let mut party = Vec::new();
    for slot in state.adventure_view.party_slots.iter() {
        if let Some(adv_idx) = slot {
            if let Some(adv) = game_state.adventurers.get(*adv_idx) {
                party.push(game::PartyMember::from_adventurer(
                    *adv_idx,
                    adv,
                    &data.item_registry,
                ));
            }
        }
    }

    // Mark adventurers as OnQuest
    for slot in state.adventure_view.party_slots.iter() {
        if let Some(adv_idx) = slot {
            if let Some(adv) = game_state.adventurers.get_mut(*adv_idx) {
                adv.status = game::AdventurerStatus::OnQuest;
            }
        }
    }

    let active = game::ActiveAdventure::new(quest, party);
    game_state.active_adventure = Some(active);
    state.adventure_view.screen = AdventureScreen::InAdventure;
    state.bottom_tab = BottomTab::Adventures;

    state.log_messages.push((
        format!("The party departs for {}.", quest.name),
        ratatui::style::Style::default().fg(ui::GOLD),
    ));
    state.auto_scroll(20);
}

fn handle_in_adventure(
    state: &mut TavernState,
    game_state: &mut GameState,
    data: &GameData,
    key: KeyCode,
) {
    let Some(quest) = game_state
        .active_adventure
        .as_ref()
        .and_then(|a| data.quest(&a.quest_id))
    else {
        return;
    };
    let w = quest.map.width;
    let h = quest.map.height;

    let (dx, dy) = match key {
        KeyCode::Up => (0i32, -1i32),
        KeyCode::Down => (0, 1),
        KeyCode::Left => (-1, 0),
        KeyCode::Right => (1, 0),
        _ => return,
    };

    let Some(adventure) = game_state.active_adventure.as_mut() else {
        return;
    };
    if !adventure.try_move(dx, dy, w, h) {
        return;
    }

    // Trigger square effect
    trigger_current_square(state, game_state, data);
}

fn trigger_current_square(state: &mut TavernState, game_state: &mut GameState, data: &GameData) {
    let Some(adventure) = game_state.active_adventure.as_mut() else {
        return;
    };
    let Some(quest) = data.quest(&adventure.quest_id) else {
        return;
    };
    let (x, y) = adventure.position;
    if adventure.is_completed(x, y) {
        return;
    }
    let Some(square) = quest.map.get(x, y).cloned() else {
        return;
    };

    match square {
        game::SquareKind::Empty => {}
        game::SquareKind::Treasure { gold, items } => {
            adventure.pending_gold += gold;
            for (id, qty) in &items {
                adventure.pending_loot.push((id.clone(), *qty));
            }
            adventure.add_log(format!("Found {} gold and {} item(s).", gold, items.len()));
            adventure.mark_completed(x, y);
        }
        game::SquareKind::Rest => {
            for member in adventure.party.iter_mut() {
                if !member.downed {
                    member.current_hp = member.max_hp;
                }
            }
            adventure.add_log("The party rests and recovers.");
            adventure.mark_completed(x, y);
        }
        game::SquareKind::Trap { damage, dex_dc } => {
            use rand::Rng;
            let mut rng = rand::rng();
            let mut hits = 0;
            for member in adventure.party.iter_mut() {
                if member.downed {
                    continue;
                }
                let roll = rng.random_range(1..=20);
                let total = roll + member.dexterity;
                if total < dex_dc {
                    member.current_hp = (member.current_hp - damage).max(0);
                    if member.current_hp == 0 {
                        member.downed = true;
                    }
                    hits += 1;
                }
            }
            adventure.add_log(format!("Trap! {} party member(s) hit.", hits));
            adventure.mark_completed(x, y);
            // Check if party is wiped
            if adventure.party.iter().all(|m| m.downed) {
                adventure.state = AdventureState::Complete { success: false };
                state.adventure_view.screen = AdventureScreen::Results;
                state.log_messages.push((
                    "The party fell to a trap.".into(),
                    ratatui::style::Style::default().fg(ui::EMBER),
                ));
                state.auto_scroll(20);
            }
        }
        game::SquareKind::Combat { encounter_id }
        | game::SquareKind::Boss { encounter_id } => {
            let is_boss = matches!(quest.map.get(x, y), Some(game::SquareKind::Boss { .. }));
            if let Some(encounter) = data.encounter(&encounter_id) {
                let combat = game::CombatState::new(encounter, adventure.party.len(), is_boss);
                adventure.state = AdventureState::InCombat(combat);
                state.adventure_view.screen = AdventureScreen::Combat;
                state.adventure_view.combat_action_idx = 0;
                state.adventure_view.combat_target_idx = 0;
                state.adventure_view.combat_picking_target = false;
            }
        }
    }
}

fn handle_combat(
    state: &mut TavernState,
    game_state: &mut GameState,
    data: &GameData,
    key: KeyCode,
) {
    let Some(adventure) = game_state.active_adventure.as_mut() else {
        return;
    };
    let AdventureState::InCombat(combat) = &mut adventure.state else {
        return;
    };

    // Only allow input on a party turn
    let current = combat.current_actor();
    let CombatActor::Party(actor_idx) = (match current {
        Some(a) => a,
        None => return,
    }) else {
        return;
    };

    if state.adventure_view.combat_picking_target {
        // Target picker
        match key {
            KeyCode::Up => {
                if state.adventure_view.combat_target_idx > 0 {
                    state.adventure_view.combat_target_idx -= 1;
                }
            }
            KeyCode::Down => {
                if state.adventure_view.combat_target_idx + 1 < combat.enemies.len() {
                    state.adventure_view.combat_target_idx += 1;
                }
            }
            KeyCode::Enter => {
                // Find a valid target (skip dead enemies)
                let target = state.adventure_view.combat_target_idx;
                if combat
                    .enemies
                    .get(target)
                    .map(|e| e.current_hp > 0)
                    .unwrap_or(false)
                {
                    game::resolve_party_action(
                        &mut adventure.party,
                        combat,
                        actor_idx,
                        CombatAction::Attack,
                        Some(target),
                    );
                    state.adventure_view.combat_picking_target = false;
                    state.adventure_view.combat_action_idx = 0;
                    advance_combat_after_action(state, game_state, data);
                }
            }
            KeyCode::Esc => {
                state.adventure_view.combat_picking_target = false;
            }
            _ => {}
        }
        return;
    }

    // Action menu
    match key {
        KeyCode::Up => {
            if state.adventure_view.combat_action_idx > 0 {
                state.adventure_view.combat_action_idx -= 1;
            }
        }
        KeyCode::Down => {
            if state.adventure_view.combat_action_idx < 2 {
                state.adventure_view.combat_action_idx += 1;
            }
        }
        KeyCode::Enter => {
            match state.adventure_view.combat_action_idx {
                0 => {
                    // Attack — pick target
                    // Find first alive enemy as default
                    let first_alive = combat.enemies.iter().position(|e| e.current_hp > 0);
                    if let Some(idx) = first_alive {
                        state.adventure_view.combat_target_idx = idx;
                        state.adventure_view.combat_picking_target = true;
                    }
                }
                1 => {
                    game::resolve_party_action(
                        &mut adventure.party,
                        combat,
                        actor_idx,
                        CombatAction::Defend,
                        None,
                    );
                    advance_combat_after_action(state, game_state, data);
                }
                2 => {
                    game::resolve_party_action(
                        &mut adventure.party,
                        combat,
                        actor_idx,
                        CombatAction::Flee,
                        None,
                    );
                    advance_combat_after_action(state, game_state, data);
                }
                _ => {}
            }
        }
        _ => {}
    }
}

/// After a party action resolves, advance the turn order. If all party members
/// have acted this round, run the enemy turn. Check win/loss conditions.
fn advance_combat_after_action(
    state: &mut TavernState,
    game_state: &mut GameState,
    data: &GameData,
) {
    let Some(adventure) = game_state.active_adventure.as_mut() else {
        return;
    };
    let AdventureState::InCombat(combat) = &mut adventure.state else {
        return;
    };

    // Check immediate win
    if combat.party_won() {
        end_combat_victory(state, game_state, data);
        return;
    }
    if combat.party_lost(&adventure.party) {
        end_combat_loss(state, game_state);
        return;
    }

    combat.advance_turn(&adventure.party);

    // If next actor is enemy, run all remaining enemy turns until party turn or end
    loop {
        let cur = combat.current_actor();
        match cur {
            Some(CombatActor::Enemy(_)) => {
                // Run a single enemy turn (the resolve_enemy_turn does all enemies but
                // we're using turn order so we just resolve this one)
                // For simplicity: resolve_enemy_turn handles all enemies at once. Then
                // we skip past remaining enemy turns.
                game::resolve_enemy_turn(&mut adventure.party, combat);
                // Skip to next party turn
                while matches!(combat.current_actor(), Some(CombatActor::Enemy(_))) {
                    combat.advance_turn(&adventure.party);
                    if combat.party_lost(&adventure.party) || combat.party_won() {
                        break;
                    }
                }
                if combat.party_won() {
                    end_combat_victory(state, game_state, data);
                    return;
                }
                if combat.party_lost(&adventure.party) {
                    end_combat_loss(state, game_state);
                    return;
                }
                // Skip downed party members
                while let Some(CombatActor::Party(i)) = combat.current_actor() {
                    if adventure
                        .party
                        .get(i)
                        .map(|p| p.downed)
                        .unwrap_or(true)
                    {
                        combat.advance_turn(&adventure.party);
                    } else {
                        break;
                    }
                }
                break;
            }
            Some(CombatActor::Party(i)) => {
                // Skip downed party members
                if adventure
                    .party
                    .get(i)
                    .map(|p| p.downed)
                    .unwrap_or(true)
                {
                    combat.advance_turn(&adventure.party);
                    continue;
                }
                break;
            }
            None => break,
        }
    }

    state.adventure_view.combat_action_idx = 0;
}

fn end_combat_victory(state: &mut TavernState, game_state: &mut GameState, data: &GameData) {
    let Some(adventure) = game_state.active_adventure.as_mut() else {
        return;
    };
    let AdventureState::InCombat(combat) = &adventure.state else {
        return;
    };

    let was_boss = combat.is_boss;
    let xp_gained: u32 = combat.enemies.iter().map(|e| e.xp_reward).sum();
    adventure.pending_xp += xp_gained;
    adventure.add_log(format!("Victory! +{} XP", xp_gained));

    // Mark current square as completed
    let pos = adventure.position;
    adventure.mark_completed(pos.0, pos.1);

    // Return to exploring
    adventure.state = AdventureState::Exploring;
    state.adventure_view.screen = AdventureScreen::InAdventure;

    if was_boss {
        // Apply quest completion rewards into pending pool
        if let Some(quest) = data.quest(&adventure.quest_id) {
            adventure.pending_gold += quest.completion_gold;
            adventure.pending_xp += quest.xp_reward;
            for (id, qty) in &quest.completion_loot {
                adventure.pending_loot.push((id.clone(), *qty));
            }
        }
        adventure.state = AdventureState::Complete { success: true };
        state.adventure_view.screen = AdventureScreen::Results;
        state.log_messages.push((
            "The party has triumphed!".into(),
            ratatui::style::Style::default().fg(ui::GOLD),
        ));
        state.auto_scroll(20);
    }
}

fn end_combat_loss(state: &mut TavernState, game_state: &mut GameState) {
    let Some(adventure) = game_state.active_adventure.as_mut() else {
        return;
    };
    adventure.state = AdventureState::Complete { success: false };
    state.adventure_view.screen = AdventureScreen::Results;
    state.log_messages.push((
        "The party falls in battle.".into(),
        ratatui::style::Style::default().fg(ui::EMBER),
    ));
    state.auto_scroll(20);
}

fn handle_results(state: &mut TavernState, game_state: &mut GameState, key: KeyCode) {
    if key == KeyCode::Enter {
        finish_active_adventure(game_state);
        state.tile_graphics.cleanup_all();
        state.adventure_view.screen = AdventureScreen::QuestBoard;
    }
}

/// Apply pending rewards from the active adventure and clear it.
fn finish_active_adventure(game_state: &mut GameState) {
    let Some(adventure) = game_state.active_adventure.take() else {
        return;
    };
    let success = matches!(adventure.state, AdventureState::Complete { success: true });

    // Apply quest completion rewards if successful
    if success {
        // Pending loot from squares
        for (id, qty) in &adventure.pending_loot {
            game_state.inventory.add(id, *qty);
        }
        game_state.gold = game_state.gold.saturating_add(adventure.pending_gold);

        // Apply quest completion bonus rewards (need to look up quest from data — but no data here)
        // We'll instead pre-merge them into pending_* when starting the adventure. For now, the
        // quest's completion_loot/gold are baked into pending in the success path here:
        // ... but we don't have data. We'll just use what's already in pending.

        // XP
        for member in &adventure.party {
            if let Some(adv) = game_state.adventurers.get_mut(member.roster_idx) {
                adv.xp = adv.xp.saturating_add(adventure.pending_xp);
            }
        }
    }

    // Restore party adventurers to Ready (downed → 1 HP / Ready in MVP)
    for member in &adventure.party {
        if let Some(adv) = game_state.adventurers.get_mut(member.roster_idx) {
            adv.status = game::AdventurerStatus::Ready;
        }
    }
}

fn abandon_active_adventure(game_state: &mut GameState) {
    let Some(adventure) = game_state.active_adventure.take() else {
        return;
    };
    for member in &adventure.party {
        if let Some(adv) = game_state.adventurers.get_mut(member.roster_idx) {
            adv.status = game::AdventurerStatus::Ready;
        }
    }
}
