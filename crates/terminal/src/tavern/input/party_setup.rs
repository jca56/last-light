//! Adventures → Party Setup sub-screen input + adventure launch.

use crossterm::event::KeyCode;
use ratatui::style::Style;

use super::super::state::{AdventureScreen, BottomTab, PartySetupFocus, TavernState};
use super::gear::{cycle_consumable, cycle_equipment, unequip_consumable, unequip_slot};
use crate::game::{self, GameData, GameState};
use crate::ui;

pub(super) fn handle_party_setup(
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
            Style::default().fg(ui::DIM),
        ));
        state.auto_scroll(20);
        return;
    }
    if party_count > max_party {
        state.log_messages.push((
            format!("Maximum {} adventurer(s).", max_party),
            Style::default().fg(ui::DIM),
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
        Style::default().fg(ui::GOLD),
    ));
    state.auto_scroll(20);
}
