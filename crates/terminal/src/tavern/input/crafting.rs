//! Input for the Crafting view (category tabs + recipe + batch picker).

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::style::Style;

use super::super::state::{BottomTab, TavernState};
use crate::game::{self, GameData, GameState};
use crate::ui;

pub(super) fn handle_crafting_input(
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
