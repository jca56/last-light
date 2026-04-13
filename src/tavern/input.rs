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
            if state.adventure_view.roster_focus == RosterFocus::ItemPicker {
                state.adventure_view.roster_focus = RosterFocus::Equipment;
                return true;
            }
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
            let events = finish_active_adventure(game_state);
            log_level_ups(state, &events);
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
        RosterFocus::Equipment => {
            // Slots 0-2 = equipment, 3-6 = consumables
            let max_slot = 2 + game::CONSUMABLE_SLOTS;
            match key {
                KeyCode::Up => {
                    if state.adventure_view.roster_equip_slot > 0 {
                        state.adventure_view.roster_equip_slot -= 1;
                    }
                }
                KeyCode::Down => {
                    if state.adventure_view.roster_equip_slot < max_slot {
                        state.adventure_view.roster_equip_slot += 1;
                    }
                }
                KeyCode::Enter => {
                    // Open item picker for this slot
                    state.adventure_view.roster_focus = RosterFocus::ItemPicker;
                    state.adventure_view.roster_picker_idx = 0;
                }
                KeyCode::Char('x') | KeyCode::Char('X') => {
                    let adv_idx = state.adventure_view.selected_adventurer;
                    let slot = state.adventure_view.roster_equip_slot;
                    if slot <= 2 {
                        unequip_slot(game_state, adv_idx, slot);
                    } else {
                        unequip_consumable(game_state, adv_idx, slot - 3);
                    }
                }
                KeyCode::Esc | KeyCode::Left => {
                    state.adventure_view.roster_focus = RosterFocus::List;
                }
                KeyCode::Tab => {
                    state.adventure_view.roster_focus = RosterFocus::List;
                    state.adventure_view.screen = AdventureScreen::QuestBoard;
                }
                _ => {}
            }
        }
        RosterFocus::ItemPicker => {
            let adv_idx = state.adventure_view.selected_adventurer;
            let slot = state.adventure_view.roster_equip_slot;
            let compatible = compatible_items_for_slot(game_state, slot);

            match key {
                KeyCode::Up => {
                    if state.adventure_view.roster_picker_idx > 0 {
                        state.adventure_view.roster_picker_idx -= 1;
                    }
                }
                KeyCode::Down => {
                    if !compatible.is_empty()
                        && state.adventure_view.roster_picker_idx + 1 < compatible.len()
                    {
                        state.adventure_view.roster_picker_idx += 1;
                    }
                }
                KeyCode::Enter => {
                    if let Some((item_id, _)) =
                        compatible.get(state.adventure_view.roster_picker_idx)
                    {
                        let item_id = item_id.clone();
                        if slot <= 2 {
                            equip_item_from_picker(game_state, adv_idx, slot, &item_id);
                        } else {
                            equip_consumable_from_picker(
                                game_state,
                                adv_idx,
                                slot - 3,
                                &item_id,
                            );
                        }
                    }
                    state.adventure_view.roster_focus = RosterFocus::Equipment;
                }
                KeyCode::Esc => {
                    state.adventure_view.roster_focus = RosterFocus::Equipment;
                }
                _ => {}
            }
        }
    }
}

/// Get all items from inventory that are compatible with the given equipment
/// or consumable slot. Returns (ItemId, quantity) pairs sorted by name.
pub(super) fn compatible_items_for_slot(
    game_state: &GameState,
    slot: usize,
) -> Vec<(game::ItemId, u32)> {
    let registry = game::ItemRegistry::new();

    if slot <= 2 {
        // Equipment slot: filter by Weapon/Armor/Accessory
        let target_category = match slot {
            0 => game::ItemCategory::Weapon,
            1 => game::ItemCategory::Armor,
            2 => game::ItemCategory::Accessory,
            _ => return Vec::new(),
        };
        let mut items: Vec<(game::ItemId, u32)> = game_state
            .inventory
            .items()
            .iter()
            .filter_map(|(id, qty)| {
                if *qty == 0 {
                    return None;
                }
                let def = registry.get(id)?;
                if def.category == target_category {
                    Some((id.clone(), *qty))
                } else {
                    None
                }
            })
            .collect();
        items.sort_by(|(a, _), (b, _)| {
            let a_name = registry.get(a).map(|d| d.name.clone()).unwrap_or_default();
            let b_name = registry.get(b).map(|d| d.name.clone()).unwrap_or_default();
            a_name.cmp(&b_name)
        });
        items
    } else {
        // Consumable slot: show consumable items
        let mut items: Vec<(game::ItemId, u32)> = game_state
            .inventory
            .items()
            .iter()
            .filter_map(|(id, qty)| {
                if *qty == 0 {
                    return None;
                }
                let def = registry.get(id)?;
                if def.tags.iter().any(|t| t == "consumable") {
                    Some((id.clone(), *qty))
                } else {
                    None
                }
            })
            .collect();
        items.sort_by(|(a, _), (b, _)| {
            let a_name = registry.get(a).map(|d| d.name.clone()).unwrap_or_default();
            let b_name = registry.get(b).map(|d| d.name.clone()).unwrap_or_default();
            a_name.cmp(&b_name)
        });
        items
    }
}

/// Equip a specific gear item from the picker. Unequips current item first.
fn equip_item_from_picker(
    game_state: &mut GameState,
    adv_idx: usize,
    equip_slot: usize,
    item_id: &game::ItemId,
) {
    unequip_slot(game_state, adv_idx, equip_slot);
    if !game_state.inventory.remove(item_id, 1) {
        return;
    }
    let Some(adv) = game_state.adventurers.get_mut(adv_idx) else {
        game_state.inventory.add(item_id, 1);
        return;
    };
    match equip_slot {
        0 => adv.equipment.weapon = Some(item_id.clone()),
        1 => adv.equipment.armor = Some(item_id.clone()),
        2 => adv.equipment.accessory = Some(item_id.clone()),
        _ => {
            game_state.inventory.add(item_id, 1);
        }
    }
}

/// Equip a consumable item from the picker into a consumable slot.
fn equip_consumable_from_picker(
    game_state: &mut GameState,
    adv_idx: usize,
    consumable_slot: usize,
    item_id: &game::ItemId,
) {
    unequip_consumable(game_state, adv_idx, consumable_slot);
    if !game_state.inventory.remove(item_id, 1) {
        return;
    }
    let Some(adv) = game_state.adventurers.get_mut(adv_idx) else {
        game_state.inventory.add(item_id, 1);
        return;
    };
    adv.init_consumable_slots();
    if consumable_slot < adv.consumables.len() {
        adv.consumables[consumable_slot] = Some(item_id.clone());
    } else {
        game_state.inventory.add(item_id, 1);
    }
}

/// Cycle equipment for the roster view. Same logic as the party setup
/// cycle_equipment but doesn't need GameData for filtering — we just match
/// by item category directly.
fn handle_quest_board(state: &mut TavernState, data: &GameData, key: KeyCode) {
    match key {
        KeyCode::Tab => {
            state.adventure_view.screen = AdventureScreen::Roster;
            return;
        }
        _ => {}
    }
    let total = data.quests.len() + data.dungeons.len();
    state.adventure_view.quest_board_count = total;
    if total == 0 {
        return;
    }
    match key {
        KeyCode::Up => {
            if state.adventure_view.selected_quest > 0 {
                state.adventure_view.selected_quest -= 1;
            }
        }
        KeyCode::Down => {
            if state.adventure_view.selected_quest + 1 < total {
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
        KeyCode::Down => {
            let max_equip = 2 + game::CONSUMABLE_SLOTS;
            match state.adventure_view.setup_focus {
                PartySetupFocus::PartySlots => {
                    if state.adventure_view.setup_slot < 2 {
                        state.adventure_view.setup_slot += 1;
                    }
                }
                PartySetupFocus::EquipmentSlots => {
                    if state.adventure_view.setup_equip_slot < max_equip {
                        state.adventure_view.setup_equip_slot += 1;
                    }
                }
            }
        }
        KeyCode::Enter => match state.adventure_view.setup_focus {
            PartySetupFocus::PartySlots => {
                // Open adventurer picker
                state.adventure_view.picking_adventurer = true;
                state.adventure_view.picker_idx = 0;
            }
            PartySetupFocus::EquipmentSlots => {
                let slot_idx = state.adventure_view.setup_slot;
                let equip_slot = state.adventure_view.setup_equip_slot;
                if let Some(adv_idx) = state.adventure_view.party_slots[slot_idx] {
                    if equip_slot <= 2 {
                        cycle_equipment(game_state, data, adv_idx, equip_slot);
                    } else {
                        cycle_consumable(game_state, data, adv_idx, equip_slot - 3);
                    }
                }
            }
        },
        KeyCode::Char('x') | KeyCode::Char('X') => {
            // Clear selected slot or unequip selected gear/consumable
            match state.adventure_view.setup_focus {
                PartySetupFocus::PartySlots => {
                    state.adventure_view.party_slots[state.adventure_view.setup_slot] = None;
                }
                PartySetupFocus::EquipmentSlots => {
                    let slot_idx = state.adventure_view.setup_slot;
                    let equip_slot = state.adventure_view.setup_equip_slot;
                    if let Some(adv_idx) = state.adventure_view.party_slots[slot_idx] {
                        if equip_slot <= 2 {
                            unequip_slot(game_state, adv_idx, equip_slot);
                        } else {
                            unequip_consumable(game_state, adv_idx, equip_slot - 3);
                        }
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

    // First: safely unequip current item (returns it to inventory via take())
    unequip_slot(game_state, adv_idx, equip_slot);

    // Now find compatible items from inventory (which now includes the returned item)
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

    // Pick the first compatible item and equip it
    let next_id = compatible[0].clone();
    equip_item_from_picker(game_state, adv_idx, equip_slot, &next_id);
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

fn cycle_consumable(
    game_state: &mut GameState,
    data: &GameData,
    adv_idx: usize,
    slot: usize,
) {
    let compatible: Vec<game::ItemId> = game_state
        .inventory
        .items()
        .iter()
        .filter_map(|(id, qty)| {
            if *qty == 0 {
                return None;
            }
            let def = data.item_registry.get(id)?;
            if def.category == game::ItemCategory::Consumable {
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
    adv.init_consumable_slots();

    let current_id = adv.consumables.get(slot).cloned().flatten();

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

    if let Some(prev) = current_id {
        game_state.inventory.add(&prev, 1);
    }

    if !game_state.inventory.remove(&next_id, 1) {
        return;
    }

    let Some(adv) = game_state.adventurers.get_mut(adv_idx) else {
        return;
    };
    if slot < adv.consumables.len() {
        adv.consumables[slot] = Some(next_id);
    }
}

fn unequip_consumable(game_state: &mut GameState, adv_idx: usize, slot: usize) {
    let Some(adv) = game_state.adventurers.get_mut(adv_idx) else {
        return;
    };
    adv.init_consumable_slots();
    let removed = if slot < adv.consumables.len() {
        adv.consumables[slot].take()
    } else {
        None
    };
    if let Some(id) = removed {
        game_state.inventory.add(&id, 1);
    }
}

fn try_start_adventure(state: &mut TavernState, game_state: &mut GameState, data: &GameData) {
    let idx = state.adventure_view.selected_quest;
    let quest_count = data.quests.len();

    // Determine if this is a story quest or a dungeon
    let (min_party, max_party, name) = if idx < quest_count {
        let q = &data.quests[idx];
        (q.min_party, q.max_party, q.name.clone())
    } else {
        let di = idx - quest_count;
        let Some(d) = data.dungeons.get(di) else {
            return;
        };
        (d.min_party, d.max_party, d.name.clone())
    };

    let party_count: u32 = state
        .adventure_view
        .party_slots
        .iter()
        .filter(|s| s.is_some())
        .count() as u32;
    if party_count < min_party {
        state.log_messages.push((
            format!("Need at least {} adventurer(s).", min_party),
            ratatui::style::Style::default().fg(ui::DIM),
        ));
        state.auto_scroll(20);
        return;
    }
    if party_count > max_party {
        state.log_messages.push((
            format!("Maximum {} adventurer(s).", max_party),
            ratatui::style::Style::default().fg(ui::DIM),
        ));
        state.auto_scroll(20);
        return;
    }

    // Build party members
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

    // Create the adventure (story quest or dungeon)
    let active = if idx < quest_count {
        game::ActiveAdventure::new(&data.quests[idx], party)
    } else {
        let di = idx - quest_count;
        game::ActiveAdventure::new_dungeon(&data.dungeons[di], party)
    };

    game_state.active_adventure = Some(active);
    state.adventure_view.screen = AdventureScreen::InAdventure;
    state.bottom_tab = BottomTab::Adventures;

    state.log_messages.push((
        format!("The party departs for {}.", name),
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
    // Get map dimensions (dungeon floor or static quest)
    let (w, h) = {
        let Some(adventure) = game_state.active_adventure.as_ref() else {
            return;
        };
        if let Some(dm) = adventure.active_map() {
            (dm.width, dm.height)
        } else if let Some(q) = data.quest(&adventure.quest_id) {
            (q.map.width, q.map.height)
        } else {
            return;
        }
    };

    match key {
        KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right => {
            let (dx, dy) = match key {
                KeyCode::Up => (0i32, -1i32),
                KeyCode::Down => (0, 1),
                KeyCode::Left => (-1, 0),
                KeyCode::Right => (1, 0),
                _ => unreachable!(),
            };
            let Some(adventure) = game_state.active_adventure.as_mut() else {
                return;
            };
            if !adventure.try_move(dx, dy, w, h) {
                return;
            }
            // Auto-trigger dangerous squares (traps, combat, boss)
            trigger_auto_square(state, game_state, data);
        }
        KeyCode::Enter => {
            // Manually interact with the current square (chests, rest, ladders)
            trigger_manual_square(state, game_state, data);
        }
        KeyCode::Char('i') | KeyCode::Char('I') => {
            // Use a consumable item out of combat
            use_consumable_out_of_combat(state, game_state, data);
        }
        _ => {}
    }
}

/// Use a consumable from the first party member who has one.
/// Shows the consumable picker (reuses combat consumable picking state).
fn use_consumable_out_of_combat(
    state: &mut TavernState,
    game_state: &mut GameState,
    data: &GameData,
) {
    let Some(adventure) = game_state.active_adventure.as_mut() else {
        return;
    };

    // Find any party member with consumables
    let has_any = adventure
        .party
        .iter()
        .any(|m| !m.downed && m.has_consumables());
    if !has_any {
        state.log_messages.push((
            "No consumables available.".into(),
            ratatui::style::Style::default().fg(ui::DIM),
        ));
        state.auto_scroll(20);
        return;
    }

    // Find first non-downed member with consumables
    let member_idx = adventure
        .party
        .iter()
        .position(|m| !m.downed && m.has_consumables());
    let Some(mi) = member_idx else {
        return;
    };

    // Use the first available consumable
    let consumable_idx = adventure.party[mi]
        .consumables
        .iter()
        .position(|s| s.is_some());
    let Some(ci) = consumable_idx else {
        return;
    };
    let item_id = adventure.party[mi].consumables[ci].take();
    let Some(id) = item_id else {
        return;
    };

    // Apply the consumable effect
    let item_name = data
        .item_registry
        .get(&id)
        .map(|d| d.name.clone())
        .unwrap_or_else(|| id.0.clone());
    let member_name = adventure.party[mi].name.clone();

    // Check tags for effect
    let def = data.item_registry.get(&id);
    let is_healing = def
        .map(|d| d.tags.iter().any(|t| t == "healing"))
        .unwrap_or(false);

    if is_healing {
        // Heal the most damaged non-downed party member
        let target_idx = adventure
            .party
            .iter()
            .enumerate()
            .filter(|(_, m)| !m.downed && m.current_hp < m.max_hp)
            .min_by_key(|(_, m)| m.current_hp)
            .map(|(i, _)| i);
        if let Some(ti) = target_idx {
            let heal_amount = if id.0.contains("minor") { 8 } else { 15 };
            let target = &mut adventure.party[ti];
            let old_hp = target.current_hp;
            target.current_hp = (target.current_hp + heal_amount).min(target.max_hp);
            let healed = target.current_hp - old_hp;
            let target_name = target.name.clone();
            adventure.add_log(format!(
                "{} uses {} — {} heals {} HP!",
                member_name, item_name, target_name, healed
            ));
        }
    } else {
        adventure.add_log(format!("{} uses {}.", member_name, item_name));
    }
}

/// Helper: get the current square from the active map.
fn get_current_square(
    adventure: &game::ActiveAdventure,
    data: &GameData,
) -> Option<game::SquareKind> {
    let map = if let Some(dm) = adventure.active_map() {
        dm
    } else if let Some(q) = data.quest(&adventure.quest_id) {
        &q.map
    } else {
        return None;
    };
    let (x, y) = adventure.position;
    if adventure.is_completed(x, y) {
        return None;
    }
    map.get(x, y).cloned()
}

/// Auto-triggered when stepping onto a tile: traps and combat only.
fn trigger_auto_square(state: &mut TavernState, game_state: &mut GameState, data: &GameData) {
    let Some(adventure) = game_state.active_adventure.as_mut() else {
        return;
    };
    let Some(square) = get_current_square(adventure, data) else {
        return;
    };
    let (x, y) = adventure.position;

    match square {
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
        game::SquareKind::Combat { ref encounter_id }
        | game::SquareKind::Boss { ref encounter_id } => {
            let is_boss = matches!(square, game::SquareKind::Boss { .. });
            let encounter_id = encounter_id.clone();
            let encounter = if adventure.dungeon_id.is_some() {
                let dungeon = adventure
                    .dungeon_id
                    .as_ref()
                    .and_then(|did| data.dungeons.iter().find(|d| d.id == *did));
                if is_boss {
                    dungeon.map(|d| {
                        let scale = 1.0 + 0.15 * adventure.current_floor as f64;
                        game::dungeon::scale_boss(&d.boss, scale)
                    })
                } else {
                    dungeon.map(|d| {
                        let pool_idx = (adventure.current_floor as usize)
                            .min(d.enemy_pools.len().saturating_sub(1));
                        let pool = &d.enemy_pools[pool_idx];
                        let scale = 1.0 + 0.15 * adventure.current_floor as f64;
                        use rand::Rng;
                        let mut rng = rand::rng();
                        let count = rng.random_range(1..=3u32).min(pool.enemies.len() as u32);
                        let mut enemies = Vec::new();
                        for _ in 0..count {
                            let template =
                                &pool.enemies[rng.random_range(0..pool.enemies.len())];
                            enemies.push(game::dungeon::scale_enemy(template, scale));
                        }
                        game::Encounter {
                            id: encounter_id.clone(),
                            enemies,
                        }
                    })
                }
            } else {
                data.encounter(&encounter_id).cloned()
            };
            if let Some(enc) = encounter {
                let combat = game::CombatState::new(&enc, adventure.party.len(), is_boss);
                adventure.state = AdventureState::InCombat(combat);
                state.adventure_view.screen = AdventureScreen::Combat;
                state.adventure_view.combat_action_idx = 0;
                state.adventure_view.combat_target_idx = 0;
                state.adventure_view.combat_picking_target = false;
            }
        }
        _ => {} // Everything else requires Enter
    }
}

/// Manually triggered with Enter: chests, rest spots, ladders.
fn trigger_manual_square(state: &mut TavernState, game_state: &mut GameState, data: &GameData) {
    let Some(adventure) = game_state.active_adventure.as_mut() else {
        return;
    };
    let Some(square) = get_current_square(adventure, data) else {
        return;
    };
    let (x, y) = adventure.position;

    match square {
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
        game::SquareKind::LadderDown => {
            state.tile_graphics.cleanup_all();
            if let Some(did) = adventure.dungeon_id.clone() {
                if let Some(dungeon) = data.dungeons.iter().find(|d| d.id == did) {
                    adventure.descend_floor(dungeon);
                }
            }
        }
        game::SquareKind::LadderUp => {
            adventure.pending_xp = adventure.pending_xp / 2;
            adventure.add_log("The party retreats up the ladder.");
            adventure.state = AdventureState::Complete { success: true };
            state.adventure_view.screen = AdventureScreen::Results;
            state.tile_graphics.cleanup_all();
        }
        _ => {} // Traps/combat are auto, empty does nothing
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

    if state.adventure_view.combat_picking_consumable {
        // Consumable picker
        let member = &adventure.party[actor_idx];
        let slot_count = member.consumables.len();
        match key {
            KeyCode::Up => {
                if state.adventure_view.combat_consumable_idx > 0 {
                    state.adventure_view.combat_consumable_idx -= 1;
                }
            }
            KeyCode::Down => {
                if state.adventure_view.combat_consumable_idx + 1 < slot_count {
                    state.adventure_view.combat_consumable_idx += 1;
                }
            }
            KeyCode::Enter => {
                let ci = state.adventure_view.combat_consumable_idx;
                let item_id = member.consumables.get(ci).cloned().flatten();
                if let Some(id) = item_id {
                    resolve_consumable_use(adventure, actor_idx, ci, &id, data);
                    state.adventure_view.combat_picking_consumable = false;
                    state.adventure_view.combat_action_idx = 0;
                    advance_combat_after_action(state, game_state, data);
                }
            }
            KeyCode::Esc => {
                state.adventure_view.combat_picking_consumable = false;
            }
            _ => {}
        }
        return;
    }

    // Check if current member has any consumables
    let has_items = adventure.party.get(actor_idx).map(|m| m.has_consumables()).unwrap_or(false);
    let max_action = if has_items { 3 } else { 2 };

    // Action menu
    match key {
        KeyCode::Up => {
            if state.adventure_view.combat_action_idx > 0 {
                state.adventure_view.combat_action_idx -= 1;
            }
        }
        KeyCode::Down => {
            if state.adventure_view.combat_action_idx < max_action {
                state.adventure_view.combat_action_idx += 1;
            }
        }
        KeyCode::Enter => {
            match state.adventure_view.combat_action_idx {
                0 => {
                    // Attack — pick target
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
                3 => {
                    // Use Item — open consumable picker
                    state.adventure_view.combat_consumable_idx = 0;
                    state.adventure_view.combat_picking_consumable = true;
                }
                _ => {}
            }
        }
        _ => {}
    }
}

/// Apply a consumable effect to the acting party member, consume the item,
/// and log the result.
fn resolve_consumable_use(
    adventure: &mut game::ActiveAdventure,
    actor_idx: usize,
    consumable_slot: usize,
    item_id: &game::ItemId,
    data: &GameData,
) {
    let Some(def) = data.item_registry.get(item_id) else {
        return;
    };
    let Some(effect) = &def.properties.consumable_effect else {
        return;
    };
    let member = &mut adventure.party[actor_idx];
    let member_name = member.name.clone();
    let item_name = def.name.clone();

    match effect {
        game::ConsumableEffect::Heal(amount) => {
            let old_hp = member.current_hp;
            member.current_hp = (member.current_hp + amount).min(member.max_hp);
            let healed = member.current_hp - old_hp;
            if let game::AdventureState::InCombat(combat) = &mut adventure.state {
                combat.log.push(format!(
                    "{} uses {} — heals {} HP",
                    member_name, item_name, healed
                ));
            }
        }
        game::ConsumableEffect::BoostStrength(amount) => {
            member.strength += amount;
            if let game::AdventureState::InCombat(combat) = &mut adventure.state {
                combat.log.push(format!(
                    "{} uses {} — STR +{}",
                    member_name, item_name, amount
                ));
            }
        }
        game::ConsumableEffect::BoostDexterity(amount) => {
            member.dexterity += amount;
            if let game::AdventureState::InCombat(combat) = &mut adventure.state {
                combat.log.push(format!(
                    "{} uses {} — DEX +{}",
                    member_name, item_name, amount
                ));
            }
        }
        game::ConsumableEffect::BoostIntellect(amount) => {
            member.intellect += amount;
            if let game::AdventureState::InCombat(combat) = &mut adventure.state {
                combat.log.push(format!(
                    "{} uses {} — INT +{}",
                    member_name, item_name, amount
                ));
            }
        }
    }

    // Remove from member's consumable slots
    if consumable_slot < member.consumables.len() {
        member.consumables[consumable_slot] = None;
    }

    // Also remove from the adventurer's permanent consumable list
    if let Some(adv) = adventure.party.get(actor_idx) {
        // We can't access game_state here, but we already consumed it from
        // the snapshot — the adventurer's consumables are separate.
        // The item was already taken from inventory when equipping, so nothing
        // to return. The adventurer's consumables will be synced back if needed.
        let _ = adv;
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

fn cleanup_combat_portraits(state: &TavernState) {
    state.tile_graphics.cleanup_enemy_portraits();
    state.tile_graphics.cleanup_adv_portraits();
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

    // Clean up enemy portrait before leaving combat screen
    cleanup_combat_portraits(state);

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
    cleanup_combat_portraits(state);
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
        let events = finish_active_adventure(game_state);
        log_level_ups(state, &events);
        state.tile_graphics.cleanup_all();
        state.adventure_view.screen = AdventureScreen::QuestBoard;
    }
}

fn log_level_ups(state: &mut TavernState, events: &[game::GameEvent]) {
    for event in events {
        if let game::GameEvent::LevelUp { adventurer_name, new_level } = event {
            state.log_messages.push((
                format!("{} reached level {}!", adventurer_name, new_level),
                ratatui::style::Style::default().fg(ui::GOLD),
            ));
        }
    }
    if !events.is_empty() {
        state.auto_scroll(20);
    }
}

/// Apply pending rewards from the active adventure and clear it.
/// Returns level-up events (if any) for the UI log.
fn finish_active_adventure(game_state: &mut GameState) -> Vec<game::GameEvent> {
    let Some(adventure) = game_state.active_adventure.take() else {
        return Vec::new();
    };
    let success = matches!(adventure.state, AdventureState::Complete { success: true });
    let mut events = Vec::new();

    // Apply quest completion rewards if successful
    if success {
        // Pending loot from squares
        for (id, qty) in &adventure.pending_loot {
            game_state.inventory.add(id, *qty);
        }
        game_state.gold = game_state.gold.saturating_add(adventure.pending_gold);

        // XP + level-ups
        events = game_state.apply_adventure_xp(&adventure.party, adventure.pending_xp);
    }

    // Restore party adventurers to Ready (downed → 1 HP / Ready in MVP)
    for member in &adventure.party {
        if let Some(adv) = game_state.adventurers.get_mut(member.roster_idx) {
            adv.status = game::AdventurerStatus::Ready;
        }
    }

    events
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
