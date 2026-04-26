//! Gathering → At-location scene: tree columns + center panels.

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::prelude::Frame;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::super::state::TavernState;
use super::super::util::{dim_lines, transition_color};
use super::panels::{draw_active_panel, draw_finds_panel, draw_forage_panel};
use crate::game::{self, GameData, GameState};
use crate::ui;

pub(super) fn draw_gathering_location(
    frame: &mut Frame,
    state: &TavernState,
    data: &GameData,
    game_state: &GameState,
    area: Rect,
    dim_level: u8,
) {
    let loc_idx = state.gathering_view.current_location;
    let Some(location) = data.gather_locations.get(loc_idx) else {
        return;
    };

    let tree_width: u16 = 12;
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(tree_width),
            Constraint::Min(30),
            Constraint::Length(tree_width),
        ])
        .split(area);

    draw_tree_column(frame, cols[0], state.frame_count, true, dim_level);
    draw_tree_column(frame, cols[2], state.frame_count, false, dim_level);

    draw_location_center(frame, state, location, game_state, data, cols[1], dim_level);
}

fn draw_tree_column(
    frame: &mut Frame,
    area: Rect,
    frame_count: u64,
    left_side: bool,
    dim_level: u8,
) {
    if area.width < 4 || area.height < 4 {
        return;
    }

    // Gentle sway — shift between 0 and 1 columns over time
    let sway_phase = (frame_count / 12) % 4;
    let sway_offset: u16 = match sway_phase {
        1 | 3 => {
            if left_side {
                0
            } else {
                1
            }
        }
        2 => 1,
        _ => 0,
    };

    let tree_w = (area.width.saturating_sub(3)).max(4);
    let pine = ui::tree_pine(tree_w);
    let oak = ui::tree_oak(tree_w);

    let (first, second) = if left_side {
        (pine.clone(), oak.clone())
    } else {
        (oak.clone(), pine.clone())
    };
    let first_h = first.len() as u16;
    let second_h = second.len() as u16;

    let gap: u16 = 4;
    let pad_top: u16 = if left_side { 2 } else { 4 };

    let first_area = Rect {
        x: area.x + sway_offset,
        y: area.y + pad_top,
        width: area.width.saturating_sub(sway_offset),
        height: first_h.min(area.height.saturating_sub(pad_top)),
    };
    if first_area.height > 0 {
        let dimmed = dim_lines(&first, dim_level);
        frame.render_widget(Paragraph::new(dimmed), first_area);
    }

    let second_y = area.y + pad_top + first_h + gap;
    if second_y < area.y + area.height {
        let avail = area.y + area.height - second_y;
        let second_area = Rect {
            x: area.x + (1 - sway_offset),
            y: second_y,
            width: area.width.saturating_sub(1 - sway_offset),
            height: second_h.min(avail),
        };
        if second_area.height > 0 {
            let dimmed = dim_lines(&second, dim_level);
            frame.render_widget(Paragraph::new(dimmed), second_area);
        }
    }

    let leaf_seed = if left_side { 17 } else { 53 };
    let leaf_y = area.y + pad_top + first_h + (gap / 2);
    if leaf_y < area.y + area.height && dim_level == 0 {
        let leaf_line = ui::leaf_line(frame_count, area.width, leaf_seed);
        let leaf_area = Rect {
            x: area.x,
            y: leaf_y,
            width: area.width,
            height: 1,
        };
        frame.render_widget(Paragraph::new(leaf_line), leaf_area);
    }
}

fn draw_location_center(
    frame: &mut Frame,
    state: &TavernState,
    location: &game::GatherLocation,
    game_state: &GameState,
    data: &GameData,
    area: Rect,
    dim_level: u8,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // spacer
            Constraint::Length(2 + location.durations.len() as u16 * 2 + 2), // forage panel
            Constraint::Length(4), // possible finds
            Constraint::Length(2 + game_state.gathering.slots.len() as u16 + 2), // active
            Constraint::Min(1),    // hint / spacer
        ])
        .split(area);

    draw_forage_panel(frame, state, location, chunks[1], dim_level);
    draw_finds_panel(frame, location, data, chunks[2], dim_level);
    draw_active_panel(frame, state, game_state, location, chunks[3], dim_level);

    let hint = Line::from(vec![
        Span::raw("  "),
        Span::styled(
            "↑↓",
            Style::default()
                .fg(transition_color(ui::GOLD, dim_level))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " select  ",
            Style::default().fg(transition_color(ui::DIM, dim_level)),
        ),
        Span::styled(
            "Enter",
            Style::default()
                .fg(transition_color(ui::GOLD, dim_level))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " start  ",
            Style::default().fg(transition_color(ui::DIM, dim_level)),
        ),
        Span::styled(
            "Esc",
            Style::default()
                .fg(transition_color(ui::GOLD, dim_level))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " back to grounds",
            Style::default().fg(transition_color(ui::DIM, dim_level)),
        ),
    ]);
    frame.render_widget(Paragraph::new(hint).alignment(Alignment::Center), chunks[4]);
}
