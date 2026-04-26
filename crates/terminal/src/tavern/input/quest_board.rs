//! Adventures → Quest Board sub-screen input.

use crossterm::event::KeyCode;

use super::super::state::{AdventureScreen, PartySetupFocus, TavernState};
use crate::game::GameData;

pub(super) fn handle_quest_board(state: &mut TavernState, data: &GameData, key: KeyCode) {
    if let KeyCode::Tab = key {
        state.adventure_view.screen = AdventureScreen::Roster;
        return;
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
