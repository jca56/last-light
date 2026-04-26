//! Gathering → Grounds (location-select) sub-screen drawing.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::prelude::Frame;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::super::state::TavernState;
use super::super::util::{inset_rect, transition_color, wrap_text};
use super::collect_unique_drops;
use crate::game::{self, GameData, GameState};
use crate::ui;

pub(super) fn draw_gathering_grounds(
    frame: &mut Frame,
    state: &TavernState,
    data: &GameData,
    _game_state: &GameState,
    area: Rect,
    dim_level: u8,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(10),   // cards grid
            Constraint::Length(1), // hint bar
        ])
        .split(area);

    // 2x2 grid — always reserve 4 slots even if fewer locations exist
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
        match data.gather_locations.get(i) {
            Some(loc) => {
                let is_selected = i == state.gathering_view.selected_location;
                draw_location_card(frame, loc, padded, is_selected, dim_level, data);
            }
            None => {
                draw_empty_card_slot(frame, padded, dim_level);
            }
        }
    }

    let hint = Line::from(vec![
        Span::styled(
            "  ↑↓←→",
            Style::default()
                .fg(transition_color(ui::GOLD, dim_level))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " navigate  ",
            Style::default().fg(transition_color(ui::DIM, dim_level)),
        ),
        Span::styled(
            "Enter",
            Style::default()
                .fg(transition_color(ui::GOLD, dim_level))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " enter location",
            Style::default().fg(transition_color(ui::DIM, dim_level)),
        ),
    ]);
    frame.render_widget(Paragraph::new(hint), chunks[1]);
}

fn draw_empty_card_slot(frame: &mut Frame, area: Rect, dim_level: u8) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(transition_color(ui::DIM, dim_level)))
        .style(Style::default().bg(ui::SHADOW_BG));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let line = Line::from(Span::styled(
        "   ◌ — untouched —",
        Style::default().fg(transition_color(ui::DIM, dim_level)),
    ));
    frame.render_widget(Paragraph::new(vec![Line::from(""), line]), inner);
}

fn draw_location_card(
    frame: &mut Frame,
    loc: &game::GatherLocation,
    area: Rect,
    selected: bool,
    dim_level: u8,
    data: &GameData,
) {
    let border_color = if !loc.unlocked {
        transition_color(ui::DIM, dim_level)
    } else if selected {
        transition_color(ui::GOLD, dim_level)
    } else {
        transition_color(ui::BORDER, dim_level)
    };

    let title_marker = if selected { " ▸ " } else { "   " };
    let title_style = if !loc.unlocked {
        Style::default().fg(transition_color(ui::DIM, dim_level))
    } else if selected {
        Style::default()
            .fg(transition_color(ui::GOLD, dim_level))
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(transition_color(ui::WARM_WHITE, dim_level))
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(
            format!("{}{}", title_marker, loc.name),
            title_style,
        ))
        .style(Style::default().bg(ui::SHADOW_BG));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if !loc.unlocked {
        let locked = vec![
            Line::from(""),
            Line::from(Span::styled(
                "   ◆ Locked",
                Style::default().fg(transition_color(ui::DIM, dim_level)),
            )),
        ];
        frame.render_widget(Paragraph::new(locked), inner);
        return;
    }

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));

    let desc_lines = wrap_text(&loc.description, inner.width.saturating_sub(4) as usize);
    for dl in desc_lines {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                dl,
                Style::default().fg(transition_color(ui::WARM_WHITE, dim_level)),
            ),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Possible Finds",
        Style::default()
            .fg(transition_color(ui::DIM, dim_level))
            .add_modifier(Modifier::BOLD),
    )));

    let drops = collect_unique_drops(loc, data);
    for (name, rarity) in drops {
        let color = transition_color(ui::rarity_color(rarity), dim_level);
        let name_style = if rarity != game::Rarity::Common {
            Style::default().fg(color).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(color)
        };
        lines.push(Line::from(vec![
            Span::styled(
                "   • ",
                Style::default().fg(transition_color(ui::DIM, dim_level)),
            ),
            Span::styled(name, name_style),
        ]));
    }

    frame.render_widget(Paragraph::new(lines), inner);
}
