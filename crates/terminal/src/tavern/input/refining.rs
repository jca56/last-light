//! Input for the Refining view (station grid + at-station recipe + batch picker).

use crossterm::event::KeyCode;
use ratatui::style::Style;

use super::super::state::{BottomTab, RefiningScreen, TavernState};
use crate::game::{self, GameData, GameState};
use crate::ui;

pub(super) fn handle_refining_input(
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
