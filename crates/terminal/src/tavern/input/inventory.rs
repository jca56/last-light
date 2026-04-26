//! Input for the Inventory grid.

use crossterm::event::KeyCode;

use super::super::state::{sorted_inventory_items, TavernState};
use crate::game::{GameData, GameState};

pub(super) fn handle_inventory_input(
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
