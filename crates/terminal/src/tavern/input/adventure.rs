//! Adventures view — top-level Esc handler and sub-screen dispatch.

use crossterm::event::KeyCode;

use super::super::state::{AdventureScreen, RosterFocus, TavernState};
use super::{combat, in_adventure, party_setup, quest_board, results, roster};
use crate::game::{GameData, GameState};

/// Handle Esc in the Adventures view based on current sub-screen.
/// Returns true if the key was consumed.
pub(super) fn adventure_esc(state: &mut TavernState, game_state: &mut GameState) -> bool {
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
            results::abandon_active_adventure(game_state);
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
            let events = results::finish_active_adventure(game_state);
            results::log_level_ups(state, &events);
            state.tile_graphics.cleanup_all();
            state.adventure_view.screen = AdventureScreen::QuestBoard;
            true
        }
    }
}

pub(super) fn handle_adventure_input(
    state: &mut TavernState,
    game_state: &mut GameState,
    data: &GameData,
    key: KeyCode,
) {
    match state.adventure_view.screen {
        AdventureScreen::Roster => roster::handle_roster(state, game_state, key),
        AdventureScreen::QuestBoard => quest_board::handle_quest_board(state, data, key),
        AdventureScreen::PartySetup => {
            party_setup::handle_party_setup(state, game_state, data, key)
        }
        AdventureScreen::InAdventure => {
            in_adventure::handle_in_adventure(state, game_state, data, key)
        }
        AdventureScreen::Combat => combat::handle_combat(state, game_state, data, key),
        AdventureScreen::Results => results::handle_results(state, game_state, key),
    }
}
