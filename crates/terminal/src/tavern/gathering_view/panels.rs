//! Gathering → at-location panels: Forage (duration picker), Possible Finds,
//! and Active Expeditions (with loot popup overlay).

use ratatui::layout::Rect;
use ratatui::prelude::Frame;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::super::state::{PopupSource, TavernState};
use super::super::util::{format_duration, transition_color};
use super::{collect_unique_drops, draw_loot_popup};
use crate::game::{self, GameData, GameState};
use crate::ui;

pub(super) fn draw_forage_panel(
    frame: &mut Frame,
    state: &TavernState,
    location: &game::GatherLocation,
    area: Rect,
    dim_level: u8,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(transition_color(ui::BORDER, dim_level)))
        .title(Span::styled(
            " Forage ",
            Style::default()
                .fg(transition_color(ui::GOLD, dim_level))
                .add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(ui::SHADOW_BG));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();
    for (i, dur) in location.durations.iter().enumerate() {
        let is_selected = i == state.gathering_view.selected_duration;
        let (marker, name_style) = if is_selected {
            (
                Span::styled(
                    " ▸ ",
                    Style::default().fg(transition_color(ui::FLAME, dim_level)),
                ),
                Style::default()
                    .fg(transition_color(ui::GOLD, dim_level))
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            (
                Span::raw("   "),
                Style::default().fg(transition_color(ui::WARM_WHITE, dim_level)),
            )
        };

        let yield_hint = duration_tier_hint(i, location.durations.len());
        lines.push(Line::from(vec![
            marker,
            Span::styled(dur.label.clone(), name_style),
            Span::raw("  "),
            Span::styled(
                format_duration(dur.duration_ms),
                Style::default().fg(transition_color(ui::DIM, dim_level)),
            ),
            Span::styled(
                "  ·  ",
                Style::default().fg(transition_color(ui::DIM, dim_level)),
            ),
            Span::styled(
                yield_hint,
                Style::default().fg(transition_color(ui::DIM, dim_level)),
            ),
        ]));
        lines.push(Line::from(""));
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

fn duration_tier_hint(idx: usize, total: usize) -> &'static str {
    if total == 1 {
        return "yield";
    }
    let fraction = idx as f32 / (total - 1) as f32;
    if fraction < 0.34 {
        "low yield"
    } else if fraction < 0.67 {
        "moderate yield"
    } else {
        "high yield"
    }
}

pub(super) fn draw_active_panel(
    frame: &mut Frame,
    state: &TavernState,
    game_state: &GameState,
    location: &game::GatherLocation,
    area: Rect,
    dim_level: u8,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(transition_color(ui::BORDER, dim_level)))
        .title(Span::styled(
            " Active Expeditions ",
            Style::default()
                .fg(transition_color(ui::GOLD, dim_level))
                .add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(ui::SHADOW_BG));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let bar_width = inner.width.saturating_sub(22) as usize;
    let mut lines: Vec<Line> = Vec::new();
    for (i, slot) in game_state.gathering.slots.iter().enumerate() {
        let label = format!(" Slot {}  ", i + 1);
        match slot {
            Some(task) => {
                let dur_label = location
                    .durations
                    .get(task.duration_index)
                    .map(|d| d.label.as_str())
                    .unwrap_or("Expedition");
                let progress = task.progress();
                let filled = (progress * bar_width as f64) as usize;
                let bar: String =
                    "█".repeat(filled) + &"░".repeat(bar_width.saturating_sub(filled));
                let remaining_ms = task.remaining_ms();
                let time_str = format_duration(remaining_ms);

                let bar_color = if task.is_complete() {
                    transition_color(ui::FLAME, dim_level)
                } else {
                    transition_color(ui::GOLD, dim_level)
                };

                lines.push(Line::from(vec![
                    Span::styled(
                        label,
                        Style::default().fg(transition_color(ui::DIM, dim_level)),
                    ),
                    Span::styled(
                        format!("{:<18}", dur_label),
                        Style::default().fg(transition_color(ui::WARM_WHITE, dim_level)),
                    ),
                    Span::styled(bar, Style::default().fg(bar_color)),
                    Span::raw(" "),
                    Span::styled(
                        time_str,
                        Style::default().fg(transition_color(ui::DIM, dim_level)),
                    ),
                ]));
            }
            None => {
                let empty_bar: String = "░".repeat(bar_width);
                lines.push(Line::from(vec![
                    Span::styled(
                        label,
                        Style::default().fg(transition_color(ui::DIM, dim_level)),
                    ),
                    Span::styled(
                        format!("{:<18}", "— idle —"),
                        Style::default().fg(transition_color(ui::DIM, dim_level)),
                    ),
                    Span::styled(
                        empty_bar,
                        Style::default().fg(transition_color(ui::DIM, dim_level)),
                    ),
                    Span::raw(" "),
                    Span::styled(
                        "─",
                        Style::default().fg(transition_color(ui::DIM, dim_level)),
                    ),
                ]));
            }
        }
    }

    frame.render_widget(Paragraph::new(lines), inner);

    // ── Loot popups overlay ────────────────────────────────────────────
    for popup in state.loot_popups.iter() {
        // Only gather popups for this location appear here
        let PopupSource::Gather {
            slot_index,
            location_id,
        } = &popup.source
        else {
            continue;
        };
        if location_id != &location.id {
            continue;
        }
        if *slot_index >= game_state.gathering.slots.len() {
            continue;
        }
        let slot_row_y = inner.y + *slot_index as u16;
        draw_loot_popup(frame, popup, inner, slot_row_y, 30);
    }
}

pub(super) fn draw_finds_panel(
    frame: &mut Frame,
    location: &game::GatherLocation,
    data: &GameData,
    area: Rect,
    dim_level: u8,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(transition_color(ui::BORDER, dim_level)))
        .title(Span::styled(
            " Possible Finds ",
            Style::default()
                .fg(transition_color(ui::GOLD, dim_level))
                .add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(ui::SHADOW_BG));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let drops = collect_unique_drops(location, data);

    let mut spans: Vec<Span> = vec![Span::raw(" ")];
    for (i, (name, rarity)) in drops.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(
                "  ·  ",
                Style::default().fg(transition_color(ui::DIM, dim_level)),
            ));
        }
        let color = transition_color(ui::rarity_color(*rarity), dim_level);
        let style = if *rarity != game::Rarity::Common {
            Style::default().fg(color).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(color)
        };
        spans.push(Span::styled(name.clone(), style));
    }
    let lines = vec![Line::from(""), Line::from(spans)];

    frame.render_widget(Paragraph::new(lines), inner);
}
