//! Adventures → Results sub-screen input + adventure cleanup helpers.

use crossterm::event::KeyCode;
use ratatui::style::Style;

use super::super::state::{AdventureScreen, TavernState};
use crate::game::{self, AdventureState, GameState};
use crate::ui;

pub(super) fn handle_results(
    state: &mut TavernState,
    game_state: &mut GameState,
    key: KeyCode,
) {
    if key == KeyCode::Enter {
        let events = finish_active_adventure(game_state);
        log_level_ups(state, &events);
        state.tile_graphics.cleanup_all();
        state.adventure_view.screen = AdventureScreen::QuestBoard;
    }
}

pub(super) fn log_level_ups(state: &mut TavernState, events: &[game::GameEvent]) {
    for event in events {
        if let game::GameEvent::LevelUp { adventurer_name, new_level } = event {
            state.log_messages.push((
                format!("{} reached level {}!", adventurer_name, new_level),
                Style::default().fg(ui::GOLD),
            ));
        }
    }
    if !events.is_empty() {
        state.auto_scroll(20);
    }
}

/// Apply pending rewards from the active adventure and clear it.
/// Returns level-up events (if any) for the UI log.
pub(super) fn finish_active_adventure(game_state: &mut GameState) -> Vec<game::GameEvent> {
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

pub(super) fn abandon_active_adventure(game_state: &mut GameState) {
    let Some(adventure) = game_state.active_adventure.take() else {
        return;
    };
    for member in &adventure.party {
        if let Some(adv) = game_state.adventurers.get_mut(member.roster_idx) {
            adv.status = game::AdventurerStatus::Ready;
        }
    }
}
