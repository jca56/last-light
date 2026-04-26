//! Drawing for the Refining view: stations grounds (2x2 cards) and the
//! workshop dashboard (recipes + selected detail + active task).

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::prelude::Frame;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::gathering_view::draw_loot_popup;
use super::state::{PopupSource, RefiningScreen, TavernState};
use super::util::{format_duration, inset_rect, wrap_text};
use crate::game::{self, GameData, GameState, StationKind};
use crate::ui;

pub(super) fn draw_refining(
    frame: &mut Frame,
    state: &TavernState,
    data: &GameData,
    game_state: &GameState,
    area: Rect,
) {
    match state.refining_view.screen {
        RefiningScreen::Stations => draw_stations_grounds(frame, state, data, area),
        RefiningScreen::AtStation => draw_workshop(frame, state, data, game_state, area),
    }
}

// ── Stations Grounds (2x2 grid of station cards) ─────────────────────────

fn draw_stations_grounds(frame: &mut Frame, state: &TavernState, data: &GameData, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(10), Constraint::Length(1)])
        .split(area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
        .split(chunks[0]);

    let top_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
        .split(rows[0]);
    let bot_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
        .split(rows[1]);

    let slot_areas: [Rect; 4] = [top_cols[0], top_cols[1], bot_cols[0], bot_cols[1]];

    for (i, slot_area) in slot_areas.iter().enumerate() {
        let padded = inset_rect(*slot_area, 1, 1);
        match data.refining_stations.get(i) {
            Some(station) => {
                let is_selected = i == state.refining_view.selected_station;
                draw_station_card(frame, station, data, padded, is_selected);
            }
            None => {
                draw_empty_station_slot(frame, padded);
            }
        }
    }

    let hint = Line::from(vec![
        Span::styled(
            "  ↑↓←→",
            Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" navigate  ", Style::default().fg(ui::DIM)),
        Span::styled(
            "Enter",
            Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" enter station", Style::default().fg(ui::DIM)),
    ]);
    frame.render_widget(Paragraph::new(hint), chunks[1]);
}

fn draw_empty_station_slot(frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ui::DIM))
        .style(Style::default().bg(ui::SHADOW_BG));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let line = Line::from(Span::styled(
        "   ◌ — empty —",
        Style::default().fg(ui::DIM),
    ));
    frame.render_widget(Paragraph::new(vec![Line::from(""), line]), inner);
}

fn draw_station_card(
    frame: &mut Frame,
    station: &game::RefiningStation,
    data: &GameData,
    area: Rect,
    selected: bool,
) {
    let border_color = if !station.unlocked {
        ui::DIM
    } else if selected {
        ui::GOLD
    } else {
        ui::BORDER
    };

    let title_marker = if selected { " ▸ " } else { "   " };
    let title_style = if !station.unlocked {
        Style::default().fg(ui::DIM)
    } else if selected {
        Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(ui::WARM_WHITE)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(
            format!("{}{}", title_marker, station.name),
            title_style,
        ))
        .style(Style::default().bg(ui::SHADOW_BG));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if !station.unlocked {
        let locked = vec![
            Line::from(""),
            Line::from(Span::styled(
                "   ◆ Locked",
                Style::default().fg(ui::DIM),
            )),
            Line::from(""),
            Line::from(Span::styled(
                format!("   {}", station.description),
                Style::default().fg(ui::DIM),
            )),
        ];
        frame.render_widget(Paragraph::new(locked), inner);
        return;
    }

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));

    let desc_lines = wrap_text(&station.description, inner.width.saturating_sub(4) as usize);
    for dl in desc_lines {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(dl, Style::default().fg(ui::WARM_WHITE)),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Recipes",
        Style::default().fg(ui::DIM).add_modifier(Modifier::BOLD),
    )));

    let recipes = data.recipes_for_station(station.kind);
    for recipe in recipes.iter().take(6) {
        lines.push(Line::from(vec![
            Span::styled("   • ", Style::default().fg(ui::DIM)),
            Span::styled(recipe.name.clone(), Style::default().fg(ui::WARM_WHITE)),
        ]));
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

// ── Workshop dashboard (At Station) ──────────────────────────────────────

fn draw_workshop(
    frame: &mut Frame,
    state: &TavernState,
    data: &GameData,
    game_state: &GameState,
    area: Rect,
) {
    let station_idx = state.refining_view.current_station;
    let Some(station) = data.refining_stations.get(station_idx) else {
        return;
    };
    let kind = station.kind;
    let recipes = data.recipes_for_station(kind);

    // Layout: top half is recipes + details, bottom is active + hint
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // top spacer
            Constraint::Min(10),    // recipes + details (split horizontally)
            Constraint::Length(5),  // active panel
            Constraint::Length(1),  // hint
        ])
        .split(area);

    // Top split: recipes (left) + details (right)
    let top_split = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(chunks[1]);

    draw_recipes_panel(frame, state, &recipes, top_split[0]);
    draw_details_panel(frame, state, data, game_state, &recipes, top_split[1]);
    draw_workshop_active_panel(frame, state, data, game_state, kind, chunks[2]);

    let hint = Line::from(vec![
        Span::raw("  "),
        Span::styled(
            "↑↓",
            Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" recipe  ", Style::default().fg(ui::DIM)),
        Span::styled(
            "←→",
            Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" qty ±1  ", Style::default().fg(ui::DIM)),
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
        Span::styled(" start  ", Style::default().fg(ui::DIM)),
        Span::styled(
            "Esc",
            Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" back", Style::default().fg(ui::DIM)),
    ]);
    frame.render_widget(Paragraph::new(hint).alignment(Alignment::Center), chunks[3]);
}

fn draw_recipes_panel(
    frame: &mut Frame,
    state: &TavernState,
    recipes: &[&game::RefiningRecipe],
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

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));
    for (i, recipe) in recipes.iter().enumerate() {
        let is_selected = i == state.refining_view.selected_recipe;
        let (marker, name_style) = if is_selected {
            (
                Span::styled(" ▸ ", Style::default().fg(ui::FLAME)),
                Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
            )
        } else {
            (Span::raw("   "), Style::default().fg(ui::WARM_WHITE))
        };
        lines.push(Line::from(vec![
            marker,
            Span::styled(recipe.name.clone(), name_style),
        ]));
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

fn draw_details_panel(
    frame: &mut Frame,
    state: &TavernState,
    data: &GameData,
    game_state: &GameState,
    recipes: &[&game::RefiningRecipe],
    area: Rect,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ui::BORDER))
        .title(Span::styled(
            " Recipe Details ",
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

    let recipe = recipes[state.refining_view.selected_recipe.min(recipes.len() - 1)];

    let input_def = data.item_registry.get(&recipe.input_id);
    let input_name = input_def
        .map(|d| d.name.clone())
        .unwrap_or_else(|| recipe.input_id.0.clone());
    let have = game_state.inventory.count(&recipe.input_id);
    let need_per_unit = recipe.input_qty;

    let output_def = data.item_registry.get(&recipe.output_id);
    let output_name = output_def
        .map(|d| d.name.clone())
        .unwrap_or_else(|| recipe.output_id.0.clone());
    let output_rarity = output_def.map(|d| d.rarity).unwrap_or(game::Rarity::Common);
    let output_color = ui::rarity_color(output_rarity);

    let max_qty = if need_per_unit == 0 {
        0
    } else {
        have / need_per_unit
    };
    let qty = state.refining_view.quantity.min(max_qty.max(1));
    let total_input = qty * need_per_unit;
    let total_output = qty * recipe.output_qty;
    let total_time = recipe.duration_per_unit * qty as u64;

    let have_color = if have >= need_per_unit {
        ui::WARM_WHITE
    } else {
        ui::EMBER
    };

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(
            recipe.name.clone(),
            Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
        ),
    ]));
    lines.push(Line::from(""));

    // Input
    lines.push(Line::from(vec![
        Span::styled("  Input:  ", Style::default().fg(ui::DIM)),
        Span::styled(
            format!("{}× {}", need_per_unit, input_name),
            Style::default().fg(ui::WARM_WHITE),
        ),
        Span::styled("  (have ", Style::default().fg(ui::DIM)),
        Span::styled(format!("{}", have), Style::default().fg(have_color)),
        Span::styled(")", Style::default().fg(ui::DIM)),
    ]));

    // Output
    lines.push(Line::from(vec![
        Span::styled("  Output: ", Style::default().fg(ui::DIM)),
        Span::styled(
            format!("{}× {}", recipe.output_qty, output_name),
            Style::default().fg(output_color).add_modifier(Modifier::BOLD),
        ),
    ]));

    // Time per unit
    lines.push(Line::from(vec![
        Span::styled("  Time:   ", Style::default().fg(ui::DIM)),
        Span::styled(
            format!("{}/unit", format_duration(recipe.duration_per_unit)),
            Style::default().fg(ui::WARM_WHITE),
        ),
    ]));
    lines.push(Line::from(""));

    // Quantity selector
    let qty_color = if max_qty > 0 { ui::FLAME } else { ui::DIM };
    lines.push(Line::from(vec![
        Span::styled("  Quantity:  ", Style::default().fg(ui::DIM)),
        Span::styled("◂ ", Style::default().fg(ui::DIM)),
        Span::styled(
            format!("{}", qty),
            Style::default().fg(qty_color).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" ▸", Style::default().fg(ui::DIM)),
        Span::styled(
            format!("   (max {})", max_qty),
            Style::default().fg(ui::DIM),
        ),
    ]));
    lines.push(Line::from(""));

    // Totals
    lines.push(Line::from(vec![
        Span::styled("  Batch:   ", Style::default().fg(ui::DIM)),
        Span::styled(
            format!("{}× {} → {}× {}", total_input, input_name, total_output, output_name),
            Style::default().fg(ui::WARM_WHITE),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  Total:   ", Style::default().fg(ui::DIM)),
        Span::styled(
            format_duration(total_time),
            Style::default().fg(ui::WARM_WHITE),
        ),
    ]));

    frame.render_widget(Paragraph::new(lines), inner);
}

fn draw_workshop_active_panel(
    frame: &mut Frame,
    state: &TavernState,
    data: &GameData,
    game_state: &GameState,
    kind: StationKind,
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

    match game_state.refining.slot(kind) {
        Some(task) => {
            let recipe = data.recipe(&task.recipe_id);
            let recipe_name = recipe
                .map(|r| r.name.clone())
                .unwrap_or_else(|| task.recipe_id.clone());
            let progress = task.current_unit_progress();
            let filled = (progress * bar_width as f64) as usize;
            let bar: String =
                "█".repeat(filled) + &"░".repeat(bar_width.saturating_sub(filled));
            let bar_color = ui::FOREST_GREEN;
            let next_in = format_duration(task.next_unit_remaining_ms());

            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled(" ", Style::default()),
                Span::styled(
                    format!("{:<20}", recipe_name),
                    Style::default().fg(ui::WARM_WHITE).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(" {}/{}  ", task.completed_units, task.total_units),
                    Style::default().fg(ui::DIM),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::raw(" "),
                Span::styled(bar, Style::default().fg(bar_color)),
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

    // ── Loot popups overlay for refining ───────────────────────────────
    // Anchor to the bar row of the active panel
    for popup in state.loot_popups.iter() {
        let PopupSource::Refine { station } = &popup.source else {
            continue;
        };
        if *station != kind {
            continue;
        }
        // The bar is at row inner.y + 2 (spacer + name row + bar row)
        let anchor_y = inner.y + 2;
        draw_loot_popup(frame, popup, inner, anchor_y, 22);
    }
}
