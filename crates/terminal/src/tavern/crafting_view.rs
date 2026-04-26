//! Drawing for the Crafting view: category tabs, recipes list, details panel,
//! active task, and loot popup overlay.

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::prelude::Frame;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::gathering_view::draw_loot_popup;
use super::state::{PopupSource, TavernState};
use super::util::{format_duration, wrap_text};
use crate::game::{self, CraftingCategory, GameData, GameState};
use crate::ui;

pub(super) fn draw_crafting(
    frame: &mut Frame,
    state: &TavernState,
    data: &GameData,
    game_state: &GameState,
    area: Rect,
) {
    // Layout: tabs row, recipes/details split, active panel, hint
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),  // category tabs
            Constraint::Min(10),    // recipes + details split
            Constraint::Length(5),  // active panel
            Constraint::Length(1),  // hint
        ])
        .split(area);

    let categories = CraftingCategory::all();
    let cur_cat_idx = state
        .crafting_view
        .selected_category
        .min(categories.len() - 1);
    let cur_cat = categories[cur_cat_idx];

    draw_category_tabs(frame, &categories, cur_cat_idx, chunks[0]);

    let recipes = data.crafting_recipes_in(cur_cat);

    let split = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(chunks[1]);

    draw_recipes_panel(frame, state, &recipes, game_state, split[0]);
    draw_details_panel(frame, state, data, game_state, &recipes, split[1]);
    draw_active_panel(frame, state, data, game_state, chunks[2]);
    draw_hint(frame, chunks[3]);
}

fn draw_category_tabs(
    frame: &mut Frame,
    categories: &[CraftingCategory],
    active: usize,
    area: Rect,
) {
    let mut spans: Vec<Span<'static>> = Vec::new();
    spans.push(Span::raw("  "));
    for (i, cat) in categories.iter().enumerate() {
        let is_active = i == active;
        if is_active {
            spans.push(Span::styled(
                "▸ ",
                Style::default().fg(ui::FLAME).add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::styled(
                cat.label(),
                Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled("  ", Style::default()));
            spans.push(Span::styled(cat.label(), Style::default().fg(ui::DIM)));
        }
        if i + 1 < categories.len() {
            spans.push(Span::styled("    ", Style::default()));
        }
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn draw_recipes_panel(
    frame: &mut Frame,
    state: &TavernState,
    recipes: &[&game::CraftingRecipe],
    game_state: &GameState,
    area: Rect,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ui::BORDER))
        .title(Span::styled(
            " Recipes ",
            Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(ui::SHADOW_BG));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if recipes.is_empty() {
        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No recipes here yet.",
                Style::default().fg(ui::DIM),
            )),
        ];
        frame.render_widget(Paragraph::new(lines), inner);
        return;
    }

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));
    for (i, recipe) in recipes.iter().enumerate() {
        let is_selected = i == state.crafting_view.selected_recipe;
        let can_craft = recipe_is_craftable(recipe, game_state);

        let (marker, name_style) = if is_selected {
            (
                Span::styled(" ▸ ", Style::default().fg(ui::FLAME)),
                if can_craft {
                    Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(ui::DIM).add_modifier(Modifier::BOLD)
                },
            )
        } else {
            (
                Span::raw("   "),
                if can_craft {
                    Style::default().fg(ui::WARM_WHITE)
                } else {
                    Style::default().fg(ui::DIM)
                },
            )
        };

        let check = if can_craft {
            Span::styled(" ✓", Style::default().fg(ui::FOREST_GREEN))
        } else {
            Span::styled(" ✗", Style::default().fg(ui::EMBER))
        };

        lines.push(Line::from(vec![
            marker,
            Span::styled(recipe.name.clone(), name_style),
            check,
        ]));
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

fn recipe_is_craftable(recipe: &game::CraftingRecipe, game_state: &GameState) -> bool {
    recipe
        .inputs
        .iter()
        .all(|(id, qty)| game_state.inventory.has(id, *qty))
}

fn draw_details_panel(
    frame: &mut Frame,
    state: &TavernState,
    data: &GameData,
    game_state: &GameState,
    recipes: &[&game::CraftingRecipe],
    area: Rect,
) {
    if recipes.is_empty() {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(ui::BORDER))
            .title(Span::styled(
                " Details ",
                Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(ui::SHADOW_BG));
        frame.render_widget(block, area);
        return;
    }

    let recipe = recipes[state.crafting_view.selected_recipe.min(recipes.len() - 1)];

    let output_def = data.item_registry.get(&recipe.output_id);
    let output_name = output_def
        .map(|d| d.name.clone())
        .unwrap_or_else(|| recipe.output_id.0.clone());
    let rarity = output_def.map(|d| d.rarity).unwrap_or(game::Rarity::Common);
    let rarity_col = ui::rarity_color(rarity);
    let rarity_label = match rarity {
        game::Rarity::Common => "common",
        game::Rarity::Uncommon => "uncommon",
        game::Rarity::Rare => "rare",
        game::Rarity::VeryRare => "very rare",
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ui::BORDER))
        .title(Line::from(vec![
            Span::styled(
                format!(" {} ", output_name),
                Style::default().fg(rarity_col).add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!("· {} ", rarity_label), Style::default().fg(ui::DIM)),
        ]))
        .style(Style::default().bg(ui::SHADOW_BG));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));

    // Description
    if let Some(def) = output_def {
        let desc_lines = wrap_text(&def.description, inner.width.saturating_sub(4) as usize);
        for dl in desc_lines {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(dl, Style::default().fg(ui::WARM_WHITE)),
            ]));
        }
        lines.push(Line::from(""));
    }

    // Inputs
    lines.push(Line::from(Span::styled(
        "  Inputs:",
        Style::default().fg(ui::DIM).add_modifier(Modifier::BOLD),
    )));
    for (id, need_per_unit) in &recipe.inputs {
        let name = data
            .item_registry
            .get(id)
            .map(|d| d.name.clone())
            .unwrap_or_else(|| id.0.clone());
        let have = game_state.inventory.count(id);
        let have_color = if have >= *need_per_unit {
            ui::WARM_WHITE
        } else {
            ui::EMBER
        };
        lines.push(Line::from(vec![
            Span::raw("    "),
            Span::styled(
                format!("{}× {}", need_per_unit, name),
                Style::default().fg(ui::WARM_WHITE),
            ),
            Span::styled("   (have ", Style::default().fg(ui::DIM)),
            Span::styled(format!("{}", have), Style::default().fg(have_color)),
            Span::styled(")", Style::default().fg(ui::DIM)),
        ]));
    }
    lines.push(Line::from(""));

    // Output + bonus
    lines.push(Line::from(vec![
        Span::styled("  Output:  ", Style::default().fg(ui::DIM)),
        Span::styled(
            format!("{}× {}", recipe.output_qty, output_name),
            Style::default().fg(rarity_col).add_modifier(Modifier::BOLD),
        ),
    ]));

    if let Some(def) = output_def {
        if let Some(stats) = &def.properties.gear_stats {
            let bonus_str = format_gear_stats(stats);
            if !bonus_str.is_empty() {
                lines.push(Line::from(vec![
                    Span::styled("  Bonus:   ", Style::default().fg(ui::DIM)),
                    Span::styled(bonus_str, Style::default().fg(ui::FLAME)),
                ]));
            }
        } else if def.gold_value > 0 && matches!(
            def.category,
            game::ItemCategory::Food | game::ItemCategory::Drink
        ) {
            lines.push(Line::from(vec![
                Span::styled("  Value:   ", Style::default().fg(ui::DIM)),
                Span::styled(
                    format!("{}g per serving", def.gold_value),
                    Style::default().fg(ui::GOLD),
                ),
            ]));
        }
    }

    lines.push(Line::from(vec![
        Span::styled("  Time:    ", Style::default().fg(ui::DIM)),
        Span::styled(
            format!("{}/unit", format_duration(recipe.duration_per_unit)),
            Style::default().fg(ui::WARM_WHITE),
        ),
    ]));
    lines.push(Line::from(""));

    // Quantity
    let max_qty = max_craft_quantity_for_recipe(game_state, recipe);
    let qty = state.crafting_view.quantity.min(max_qty.max(1));
    let qty_color = if max_qty > 0 { ui::FLAME } else { ui::DIM };
    lines.push(Line::from(vec![
        Span::styled("  Quantity: ", Style::default().fg(ui::DIM)),
        Span::styled("◂ ", Style::default().fg(ui::DIM)),
        Span::styled(
            format!("{}", qty),
            Style::default().fg(qty_color).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" ▸", Style::default().fg(ui::DIM)),
        Span::styled(format!("   (max {})", max_qty), Style::default().fg(ui::DIM)),
    ]));

    frame.render_widget(Paragraph::new(lines), inner);
}

fn format_gear_stats(stats: &game::GearStats) -> String {
    let mut parts: Vec<String> = Vec::new();
    if stats.hp != 0 {
        parts.push(format!("HP {:+}", stats.hp));
    }
    if stats.strength != 0 {
        parts.push(format!("STR {:+}", stats.strength));
    }
    if stats.dexterity != 0 {
        parts.push(format!("DEX {:+}", stats.dexterity));
    }
    if stats.intellect != 0 {
        parts.push(format!("INT {:+}", stats.intellect));
    }
    parts.join("   ")
}

fn max_craft_quantity_for_recipe(game_state: &GameState, recipe: &game::CraftingRecipe) -> u32 {
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

fn draw_active_panel(
    frame: &mut Frame,
    state: &TavernState,
    data: &GameData,
    game_state: &GameState,
    area: Rect,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ui::BORDER))
        .title(Span::styled(
            " Active ",
            Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(ui::SHADOW_BG));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width < 6 || inner.height < 1 {
        return;
    }

    let bar_width = inner.width.saturating_sub(28) as usize;
    let mut lines: Vec<Line> = Vec::new();

    match &game_state.crafting.bench {
        Some(task) => {
            let recipe = data.crafting_recipe(&task.recipe_id);
            let recipe_name = recipe
                .map(|r| r.name.clone())
                .unwrap_or_else(|| task.recipe_id.clone());
            let progress = task.current_unit_progress();
            let filled = (progress * bar_width as f64) as usize;
            let bar: String =
                "█".repeat(filled) + &"░".repeat(bar_width.saturating_sub(filled));
            let next_in = format_duration(task.next_unit_remaining_ms());

            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::raw(" "),
                Span::styled(
                    format!("{:<24}", recipe_name),
                    Style::default()
                        .fg(ui::WARM_WHITE)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(" {}/{}", task.completed_units, task.total_units),
                    Style::default().fg(ui::DIM),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::raw(" "),
                Span::styled(bar, Style::default().fg(ui::FOREST_GREEN)),
                Span::styled(format!("  {}", next_in), Style::default().fg(ui::DIM)),
            ]));
        }
        None => {
            let empty_bar: String = "░".repeat(bar_width);
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::raw(" "),
                Span::styled("— idle —", Style::default().fg(ui::DIM)),
            ]));
            lines.push(Line::from(vec![
                Span::raw(" "),
                Span::styled(empty_bar, Style::default().fg(ui::DIM)),
                Span::styled("  ─", Style::default().fg(ui::DIM)),
            ]));
        }
    }

    frame.render_widget(Paragraph::new(lines), inner);

    // ── Loot popup overlay ─────────────────────────────────────────────
    for popup in state.loot_popups.iter() {
        if !matches!(popup.source, PopupSource::Craft) {
            continue;
        }
        // Anchor to the bar row of the active panel
        let anchor_y = inner.y + 2;
        draw_loot_popup(frame, popup, inner, anchor_y, 22);
    }
}

fn draw_hint(frame: &mut Frame, area: Rect) {
    let hint = Line::from(vec![
        Span::raw("  "),
        Span::styled(
            "Tab",
            Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" category  ", Style::default().fg(ui::DIM)),
        Span::styled(
            "↑↓",
            Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" recipe  ", Style::default().fg(ui::DIM)),
        Span::styled(
            "←→",
            Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" qty  ", Style::default().fg(ui::DIM)),
        Span::styled(
            "PgUp/Dn",
            Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" ±5  ", Style::default().fg(ui::DIM)),
        Span::styled(
            "End",
            Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" max  ", Style::default().fg(ui::DIM)),
        Span::styled(
            "Enter",
            Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" craft", Style::default().fg(ui::DIM)),
    ]);
    frame.render_widget(Paragraph::new(hint).alignment(Alignment::Center), area);
}
