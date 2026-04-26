//! Keyboard input handling — top-level dispatch and per-view handlers.
//!
//! Each domain lives in its own sibling module. `mod.rs` owns the
//! highest-level routing (modals, focus, view-level dispatch) and the small
//! handlers for the terminal log + text input bar.

mod adventure;
mod combat;
mod crafting;
mod gathering;
mod gear;
mod in_adventure;
mod inventory;
mod party_setup;
mod quest_board;
mod refining;
mod results;
mod roster;

pub(super) use gear::compatible_items_for_slot;

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::style::Style;

use super::state::{
    Focus, GatheringScreen, RefiningScreen, TavernState, Transition, View, NAV_ITEMS,
    TRANSITION_FRAMES,
};
use crate::game::{GameData, GameState};
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
            if adventure::adventure_esc(state, game_state) {
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
        View::Gathering => gathering::handle_gathering_input(state, game_state, data, key),
        View::Refining => refining::handle_refining_input(state, game_state, data, key),
        View::Crafting => crafting::handle_crafting_input(state, game_state, data, key, mods),
        View::Adventuring => adventure::handle_adventure_input(state, game_state, data, key),
        View::Inventory => inventory::handle_inventory_input(state, game_state, data, key),
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
