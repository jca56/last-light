//! Adventures → Roster sub-screen input.

use crossterm::event::KeyCode;

use super::super::state::{AdventureScreen, RosterFocus, TavernState};
use super::gear::{
    compatible_items_for_slot, equip_consumable_from_picker, equip_item_from_picker,
    unequip_consumable, unequip_slot,
};
use crate::game::{self, GameState};

pub(super) fn handle_roster(state: &mut TavernState, game_state: &mut GameState, key: KeyCode) {
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
