//! Drawing for the Gathering view: location grounds (select), at-location
//! scene with trees + center panels, and the loot popup overlay.

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::prelude::Frame;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::state::{
    GatheringScreen, LootPopup, PopupSource, TavernState, Transition, POPUP_DURATION_FRAMES,
    TRANSITION_FRAMES,
};
use super::util::{
    dim_lines, fade_color, format_duration, inset_rect, transition_color, wrap_text,
};
use crate::game::{self, GameData, GameState};
use crate::ui;

pub(super) fn effective_gathering_screen(state: &TavernState) -> GatheringScreen {
    match state.gathering_view.transition {
        Transition::None => state.gathering_view.screen,
        Transition::EnteringLocation(_) => GatheringScreen::Grounds,
        Transition::LeavingLocation(_) => GatheringScreen::AtLocation,
    }
}

pub(super) fn draw_gathering(
    frame: &mut Frame,
    state: &TavernState,
    data: &GameData,
    game_state: &GameState,
    area: Rect,
) {
    // During a transition, dim the content based on how far in we are.
    let (screen, dim_level) = match state.gathering_view.transition {
        Transition::None => (state.gathering_view.screen, 0),
        Transition::EnteringLocation(n) => (GatheringScreen::Grounds, TRANSITION_FRAMES - n),
        Transition::LeavingLocation(n) => (GatheringScreen::AtLocation, TRANSITION_FRAMES - n),
    };

    match screen {
        GatheringScreen::Grounds => {
            draw_gathering_grounds(frame, state, data, game_state, area, dim_level)
        }
        GatheringScreen::AtLocation => {
            draw_gathering_location(frame, state, data, game_state, area, dim_level)
        }
    }
}

// ── Location Select (Grounds) ─────────────────────────────────────────────

fn draw_gathering_grounds(
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

// ── At Location ──────────────────────────────────────────────────────────

fn draw_gathering_location(
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

fn draw_forage_panel(
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

fn draw_active_panel(
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

/// Render a loot popup floating up from a given anchor row.
/// `anchor_y`: the Y coordinate of the slot row inside the panel.
/// `x_offset`: horizontal offset from the panel's inner left edge for popup x.
pub(super) fn draw_loot_popup(
    frame: &mut Frame,
    popup: &LootPopup,
    panel_inner: Rect,
    anchor_y: u16,
    x_offset: u16,
) {
    let elapsed = (POPUP_DURATION_FRAMES - popup.frames_remaining) as f32;
    let total = POPUP_DURATION_FRAMES as f32;
    let progress = (elapsed / total).clamp(0.0, 1.0);

    let float_offset = (progress * 4.0) as u16;

    let item_count = popup.items.len() as u16;
    if item_count == 0 {
        return;
    }
    let popup_top_y = anchor_y.saturating_sub(float_offset);

    // Format: "▎ +N  ITEM" with thick left bar, uppercase name for visual weight
    let lines: Vec<Line<'static>> = popup
        .items
        .iter()
        .map(|(name, qty, rarity)| {
            let base_color = ui::rarity_color(*rarity);
            let faded = fade_color(base_color, progress);
            let plus_color = fade_color(ui::FLAME, progress);
            let bar_color = fade_color(base_color, progress);
            Line::from(vec![
                Span::styled(
                    "▎ ",
                    Style::default().fg(bar_color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("+{}  ", qty),
                    Style::default()
                        .fg(plus_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    name.to_uppercase(),
                    Style::default().fg(faded).add_modifier(Modifier::BOLD),
                ),
            ])
        })
        .collect();

    let popup_height = item_count;
    if popup_top_y < panel_inner.y.saturating_sub(4) {
        return;
    }

    let x_off = x_offset.min(panel_inner.width.saturating_sub(4));
    let render_area = Rect {
        x: panel_inner.x + x_off,
        y: popup_top_y,
        width: panel_inner.width.saturating_sub(x_off),
        height: popup_height,
    };

    if render_area.height == 0 || render_area.width == 0 {
        return;
    }

    if render_area.y + render_area.height > panel_inner.y + panel_inner.height + 4 {
        return;
    }

    frame.render_widget(Paragraph::new(lines), render_area);
}

fn draw_finds_panel(
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

/// Returns unique (name, rarity) pairs from a location's drop tables, sorted by rarity ascending.
fn collect_unique_drops(
    location: &game::GatherLocation,
    data: &GameData,
) -> Vec<(String, game::Rarity)> {
    let mut seen: Vec<(String, game::Rarity)> = Vec::new();
    let push = |id: &game::ItemId, seen: &mut Vec<(String, game::Rarity)>| {
        let Some(def) = data.item_registry.get(id) else {
            return;
        };
        if seen.iter().any(|(n, _)| *n == def.name) {
            return;
        }
        seen.push((def.name.clone(), def.rarity));
    };
    for dur in &location.durations {
        for entry in &dur.drop_table.guaranteed {
            push(&entry.item_id, &mut seen);
        }
        for entry in &dur.drop_table.random_pool {
            push(&entry.item_id, &mut seen);
        }
    }
    seen.sort_by_key(|(_, r)| *r);
    seen
}
