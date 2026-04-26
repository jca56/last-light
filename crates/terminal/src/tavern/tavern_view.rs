//! Drawing for the Tavern view: illustrated scene + info panels.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::prelude::Frame;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

use super::state::TavernState;
use crate::game::{self, GameData, GameState};
use crate::ui;

pub(super) fn draw_tavern(
    frame: &mut Frame,
    _state: &TavernState,
    data: &GameData,
    game_state: &GameState,
    area: Rect,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(10),    // scene
            Constraint::Length(10), // info panels
        ])
        .split(area);

    draw_tavern_scene(frame, game_state, chunks[0]);
    draw_tavern_panels(frame, data, game_state, chunks[1]);
}

fn draw_tavern_scene(frame: &mut Frame, game_state: &GameState, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ui::GOLD))
        .title(Span::styled(
            " The Last Light ",
            Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(ui::SHADOW_BG));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let tavern = &game_state.tavern;
    let visitor_count = tavern.occupancy();
    let capacity = tavern.capacity();

    // Static illustrated tavern scene using box-drawing + text art
    let mut lines: Vec<Line> = Vec::new();

    // Roof line
    lines.push(Line::from(""));

    // Top wall with window
    let wall = Style::default().fg(ui::BORDER);
    let warm = Style::default().fg(ui::WARM_WHITE);
    let gold = Style::default().fg(ui::GOLD);
    let flame = Style::default().fg(ui::FLAME);
    let dim = Style::default().fg(ui::DIM);

    lines.push(Line::from(vec![
        Span::styled("  ╔", wall),
        Span::styled("═══════════════════════════════════════════════", wall),
        Span::styled("╗", wall),
    ]));

    // Bar area
    lines.push(Line::from(vec![
        Span::styled("  ║", wall),
        Span::styled("  ┌─BAR─┐", gold),
        Span::styled("              ", dim),
        Span::styled("  🔥 ", flame),
        Span::styled("              ", dim),
        Span::styled("  ║", wall),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  ║", wall),
        Span::styled("  │", gold),
        Span::styled(" ◉◉◉ ", Style::default().fg(ui::FOREST_GREEN)),
        Span::styled("│", gold),
        Span::styled("          ", dim),
        Span::styled(" /|\\  ", flame),
        Span::styled("             ", dim),
        Span::styled("║", wall),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  ║", wall),
        Span::styled("  └──────┘", gold),
        Span::styled("         hearth", dim),
        Span::styled("              ║", wall),
    ]));

    // Tables area — show visitors as dots
    lines.push(Line::from(vec![
        Span::styled("  ║", wall),
        Span::styled("                                               ║", dim),
    ]));

    // Draw tables with visitor occupancy
    let tables = tavern.upgrades.tables;
    let tables_per_row = 4u32;
    let mut visitors_placed = 0u32;

    for row in 0..((tables + tables_per_row - 1) / tables_per_row) {
        let mut spans: Vec<Span> = vec![Span::styled("  ║  ", wall)];
        let cols_this_row = (tables - row * tables_per_row).min(tables_per_row);
        for _ in 0..cols_this_row {
            let seats = game::tavern::SEATS_PER_TABLE;
            let visitors_at_table = if visitors_placed < visitor_count {
                let v = seats.min(visitor_count - visitors_placed);
                visitors_placed += v;
                v
            } else {
                0
            };

            // Table with visitors
            let visitor_str = match visitors_at_table {
                0 => "·  · ",
                1 => "☺  · ",
                _ => "☺  ☺ ",
            };
            let table_style = if visitors_at_table > 0 { warm } else { dim };
            spans.push(Span::styled("┌──┐ ", dim));
            spans.push(Span::styled(visitor_str, table_style));
        }
        // Pad to fill the line
        let remaining = inner.width as i32 - spans.iter().map(|s| s.content.chars().count() as i32).sum::<i32>() - 1;
        if remaining > 0 {
            spans.push(Span::styled(" ".repeat(remaining as usize), dim));
        }
        spans.push(Span::styled("║", wall));
        lines.push(Line::from(spans));
    }

    // Bottom of tavern
    lines.push(Line::from(vec![
        Span::styled("  ║", wall),
        Span::styled("                                               ║", dim),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  ╚", wall),
        Span::styled("══════════════════", wall),
        Span::styled("╡ DOOR ╞", Style::default().fg(ui::WARM_WHITE)),
        Span::styled("═════════════════════", wall),
        Span::styled("╝", wall),
    ]));

    // Status line below scene
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("    Visitors: ", dim),
        Span::styled(
            format!("{}/{}", visitor_count, capacity),
            if visitor_count > 0 { gold } else { dim },
        ),
        Span::styled("    Reputation: ", dim),
        Span::styled(format!("{}", tavern.reputation), warm),
        Span::styled("    Served: ", dim),
        Span::styled(format!("{}", tavern.total_served), warm),
        Span::styled("    Earned: ", dim),
        Span::styled(format!("{}g", tavern.total_gold_earned), gold),
    ]));

    frame.render_widget(Paragraph::new(lines), inner);
}

fn draw_tavern_panels(
    frame: &mut Frame,
    data: &GameData,
    game_state: &GameState,
    area: Rect,
) {
    let split = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
        ])
        .split(area);

    draw_stock_panel(frame, data, game_state, split[0]);
    draw_visitors_panel(frame, game_state, split[1]);
    draw_upgrades_panel(frame, game_state, split[2]);
}

fn draw_stock_panel(frame: &mut Frame, data: &GameData, game_state: &GameState, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ui::BORDER))
        .title(Span::styled(
            " Stock ",
            Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(ui::SHADOW_BG));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Show food and drink items that visitors can order
    let servable = [
        "hearty_stew",
        "berry_tart",
        "herb_bread",
        "berry_cordial",
        "herbal_tea",
    ];

    let mut lines: Vec<Line> = Vec::new();
    for id_str in servable {
        let id: game::ItemId = id_str.into();
        let count = game_state.inventory.count(&id);
        let name = data
            .item_registry
            .get(&id)
            .map(|d| d.name.clone())
            .unwrap_or_else(|| id_str.to_string());
        let count_style = if count > 0 {
            Style::default().fg(ui::WARM_WHITE)
        } else {
            Style::default().fg(ui::EMBER)
        };
        lines.push(Line::from(vec![
            Span::styled(format!(" {:<16}", name), Style::default().fg(ui::DIM)),
            Span::styled(format!("×{}", count), count_style),
        ]));
    }

    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            " No servable items.",
            Style::default().fg(ui::DIM),
        )));
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

fn draw_visitors_panel(frame: &mut Frame, game_state: &GameState, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ui::BORDER))
        .title(Span::styled(
            " Visitors ",
            Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(ui::SHADOW_BG));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let tavern = &game_state.tavern;
    let mut lines: Vec<Line> = Vec::new();

    if tavern.visitors.is_empty() {
        lines.push(Line::from(Span::styled(
            " No visitors yet.",
            Style::default().fg(ui::DIM),
        )));
    } else {
        for visitor in &tavern.visitors {
            let status = if visitor.is_fully_served() {
                ("✓", ui::FOREST_GREEN)
            } else {
                let remaining = visitor.time_remaining() / 1000;
                if remaining < 30 {
                    ("!", ui::EMBER)
                } else {
                    ("·", ui::WARM_WHITE)
                }
            };

            let wants: Vec<&str> = [
                visitor
                    .food_order
                    .as_ref()
                    .map(|_| if visitor.food_served { "🍽" } else { "🍽?" }),
                visitor
                    .drink_order
                    .as_ref()
                    .map(|_| if visitor.drink_served { "🍺" } else { "🍺?" }),
            ]
            .into_iter()
            .flatten()
            .collect();

            lines.push(Line::from(vec![
                Span::styled(
                    format!(" {} ", status.0),
                    Style::default().fg(status.1),
                ),
                Span::styled(
                    format!("{:<14}", &visitor.name[..visitor.name.len().min(14)]),
                    Style::default().fg(ui::WARM_WHITE),
                ),
                Span::styled(
                    wants.join(""),
                    Style::default().fg(ui::DIM),
                ),
            ]));
        }
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

fn draw_upgrades_panel(frame: &mut Frame, game_state: &GameState, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ui::BORDER))
        .title(Span::styled(
            " Tavern ",
            Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(ui::SHADOW_BG));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let upgrades = &game_state.tavern.upgrades;
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(vec![
        Span::styled(" Tables    ", Style::default().fg(ui::DIM)),
        Span::styled(
            format!("{}", upgrades.tables),
            Style::default().fg(ui::WARM_WHITE),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled(" Kitchen   ", Style::default().fg(ui::DIM)),
        Span::styled(
            format!("Lv {}", upgrades.kitchen_level),
            Style::default().fg(ui::WARM_WHITE),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled(" Cellar    ", Style::default().fg(ui::DIM)),
        Span::styled(
            format!("Lv {}", upgrades.cellar_level),
            Style::default().fg(ui::WARM_WHITE),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled(" Rooms     ", Style::default().fg(ui::DIM)),
        Span::styled(
            format!("{}", upgrades.rooms),
            Style::default().fg(ui::WARM_WHITE),
        ),
    ]));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " Auto-stock: ON",
        Style::default().fg(ui::FOREST_GREEN),
    )));

    frame.render_widget(Paragraph::new(lines), inner);
}
