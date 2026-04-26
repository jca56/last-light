//! Drawing for the Inventory view: tile grid, detail panel, header stats.

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::prelude::Frame;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::state::{sorted_inventory_items, TavernState, ICON_WIDTH};
use super::util::{format_quantity, wrap_text};
use crate::game::{self, GameData, GameState};
use crate::ui;

pub(super) const TILE_W: u16 = 10;
pub(super) const TILE_H: u16 = 7;

pub(super) struct InventoryStats {
    pub unique: usize,
    pub total: u32,
    pub value: u32,
}

pub(super) fn inventory_stats(game_state: &GameState, data: &GameData) -> InventoryStats {
    let items = game_state.inventory.items();
    let unique = items.len();
    let total: u32 = items.values().sum();
    let value: u32 = items
        .iter()
        .map(|(id, qty)| {
            data.item_registry
                .get(id)
                .map(|d| d.gold_value * qty)
                .unwrap_or(0)
        })
        .sum();
    InventoryStats {
        unique,
        total,
        value,
    }
}

pub(super) fn draw_inventory(
    frame: &mut Frame,
    state: &mut TavernState,
    data: &GameData,
    game_state: &GameState,
    area: Rect,
) {
    const DETAIL_H: u16 = 6;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(TILE_H + 2),
            Constraint::Length(DETAIL_H),
            Constraint::Length(1), // hint
        ])
        .split(area);

    let grid_area = chunks[0];
    let detail_area = chunks[1];
    let hint_area = chunks[2];

    let items = sorted_inventory_items(game_state, data);

    if items.is_empty() {
        draw_inventory_empty(frame, grid_area);
        draw_inventory_detail_empty(frame, detail_area);
        draw_inventory_hint(frame, hint_area);
        return;
    }

    if state.inventory_view.selected >= items.len() {
        state.inventory_view.selected = items.len() - 1;
    }

    let inner_w = grid_area.width;
    let cols = ((inner_w / TILE_W) as usize).max(1);
    state.inventory_view.last_grid_cols = cols;

    draw_inventory_grid(frame, state, data, &items, grid_area, cols);

    let selected_idx = state.inventory_view.selected.min(items.len() - 1);
    let (sel_id, sel_qty) = &items[selected_idx];
    draw_inventory_detail(frame, data, sel_id, *sel_qty, detail_area);

    draw_inventory_hint(frame, hint_area);
}

fn draw_inventory_empty(frame: &mut Frame, area: Rect) {
    let lines = vec![
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled(
            "   Your pack is empty.",
            Style::default().fg(ui::DIM),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "   The Wilds await.",
            Style::default().fg(ui::DIM),
        )),
    ];
    frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), area);
}

fn draw_inventory_detail_empty(frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ui::BORDER))
        .style(Style::default().bg(ui::SHADOW_BG));
    frame.render_widget(block, area);
}

fn draw_inventory_hint(frame: &mut Frame, area: Rect) {
    let hint = Line::from(vec![
        Span::raw("  "),
        Span::styled(
            "↑↓←→",
            Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" navigate  ", Style::default().fg(ui::DIM)),
        Span::styled(
            "Home/End",
            Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" jump", Style::default().fg(ui::DIM)),
    ]);
    frame.render_widget(Paragraph::new(hint), area);
}

fn draw_inventory_grid(
    frame: &mut Frame,
    state: &mut TavernState,
    data: &GameData,
    items: &[(game::ItemId, u32)],
    area: Rect,
    cols: usize,
) {
    let selected = state.inventory_view.selected;
    let start_x = area.x + 1;
    let start_y = area.y + 1;

    for (i, (id, qty)) in items.iter().enumerate() {
        let row = i / cols;
        let col = i % cols;
        let tile_x = start_x + col as u16 * TILE_W;
        let tile_y = start_y + row as u16 * TILE_H;

        if tile_y + TILE_H > area.y + area.height {
            break;
        }

        let tile_area = Rect {
            x: tile_x,
            y: tile_y,
            width: TILE_W.min(area.x + area.width - tile_x),
            height: TILE_H,
        };

        draw_inventory_tile(frame, state, data, id, *qty, tile_area, i == selected);
    }
}

fn draw_inventory_tile(
    frame: &mut Frame,
    state: &mut TavernState,
    data: &GameData,
    id: &game::ItemId,
    qty: u32,
    area: Rect,
    selected: bool,
) {
    let def = data.item_registry.get(id);
    let rarity = def.map(|d| d.rarity).unwrap_or(game::Rarity::Common);
    let rarity_col = ui::rarity_color(rarity);

    let border_col = if selected {
        ui::GOLD
    } else if rarity == game::Rarity::Common {
        ui::BORDER
    } else {
        rarity_col
    };

    let border_style = if selected {
        Style::default()
            .fg(border_col)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(border_col)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .style(Style::default().bg(ui::SHADOW_BG));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let icon_lines: Vec<Line<'static>> = state
        .icon_for(&id.0)
        .cloned()
        .unwrap_or_else(|| fallback_icon(def, ICON_WIDTH));

    let icon_h = icon_lines.len() as u16;
    let icon_area = Rect {
        x: inner.x,
        y: inner.y,
        width: inner.width,
        height: icon_h.min(inner.height.saturating_sub(1)),
    };
    if icon_area.height > 0 {
        frame.render_widget(Paragraph::new(icon_lines), icon_area);
    }

    if inner.height > 0 {
        let qty_str = format_quantity(qty);
        let qty_line = Line::from(Span::styled(
            qty_str,
            Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
        ));
        let qty_area = Rect {
            x: inner.x,
            y: inner.y + inner.height - 1,
            width: inner.width,
            height: 1,
        };
        frame.render_widget(
            Paragraph::new(qty_line).alignment(Alignment::Right),
            qty_area,
        );
    }
}

/// Fallback icon: a solid colored square with the first letter of the item name.
fn fallback_icon(def: Option<&game::ItemDef>, width: u16) -> Vec<Line<'static>> {
    let color = def.map(|d| ui::rarity_color(d.rarity)).unwrap_or(ui::DIM);
    let letter = def
        .and_then(|d| d.name.chars().next())
        .unwrap_or('?')
        .to_ascii_uppercase();
    let height = (width / 2).max(2);
    let mut lines: Vec<Line<'static>> = Vec::new();
    for y in 0..height {
        let mut spans = Vec::new();
        for x in 0..width {
            let ch = if y == height / 2 && x == width / 2 {
                letter.to_string()
            } else {
                "█".to_string()
            };
            spans.push(Span::styled(ch, Style::default().fg(color)));
        }
        lines.push(Line::from(spans));
    }
    lines
}

fn draw_inventory_detail(
    frame: &mut Frame,
    data: &GameData,
    id: &game::ItemId,
    qty: u32,
    area: Rect,
) {
    let def = data.item_registry.get(id);
    let name = def.map(|d| d.name.clone()).unwrap_or_else(|| id.0.clone());
    let rarity = def.map(|d| d.rarity).unwrap_or(game::Rarity::Common);
    let rarity_col = ui::rarity_color(rarity);

    let border_col = if rarity == game::Rarity::Common {
        ui::BORDER
    } else {
        rarity_col
    };

    let rarity_label = match rarity {
        game::Rarity::Common => "common",
        game::Rarity::Uncommon => "uncommon",
        game::Rarity::Rare => "rare",
        game::Rarity::VeryRare => "very rare",
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_col))
        .title(Line::from(vec![
            Span::styled(
                format!(" {} ", name),
                Style::default()
                    .fg(rarity_col)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("· {} ", rarity_label),
                Style::default().fg(ui::DIM),
            ),
        ]))
        .style(Style::default().bg(ui::SHADOW_BG));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let desc = def
        .map(|d| d.description.clone())
        .unwrap_or_else(|| "An unknown item.".into());
    let gold_value = def.map(|d| d.gold_value).unwrap_or(0);
    let total_value = gold_value * qty;

    let desc_width = inner.width.saturating_sub(4) as usize;
    let desc_lines = wrap_text(&desc, desc_width);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));
    for dl in desc_lines {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(dl, Style::default().fg(ui::WARM_WHITE)),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(
            format!("Qty: {}", qty),
            Style::default().fg(ui::WARM_WHITE),
        ),
        Span::styled("   ·   ", Style::default().fg(ui::DIM)),
        Span::styled(
            format!("{}g each", gold_value),
            Style::default().fg(ui::DIM),
        ),
        Span::styled("   ·   ", Style::default().fg(ui::DIM)),
        Span::styled(
            format!("{}g total", total_value),
            Style::default().fg(ui::GOLD),
        ),
    ]));

    frame.render_widget(Paragraph::new(lines), inner);
}
