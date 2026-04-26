//! Input for the Gathering view (location grid + at-location duration picker).

use crossterm::event::KeyCode;
use ratatui::style::Style;

use super::super::state::{BottomTab, GatheringScreen, TavernState, Transition, TRANSITION_FRAMES};
use crate::game::{GameData, GameState};
use crate::ui;

pub(super) fn handle_gathering_input(
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
