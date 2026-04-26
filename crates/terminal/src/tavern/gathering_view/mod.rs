//! Drawing for the Gathering view: location grounds (select), at-location
//! scene with trees + center panels, and the loot popup overlay.
//!
//! Sub-screens live in sibling modules. `mod.rs` owns the screen dispatcher,
//! the loot-popup primitive (also reused by other views), and the
//! drop-collection helper shared between grounds and panels.

mod grounds;
mod location;
mod panels;

use ratatui::layout::Rect;
use ratatui::prelude::Frame;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::state::{
    GatheringScreen, LootPopup, TavernState, Transition, POPUP_DURATION_FRAMES, TRANSITION_FRAMES,
};
use super::util::fade_color;
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
            grounds::draw_gathering_grounds(frame, state, data, game_state, area, dim_level)
        }
        GatheringScreen::AtLocation => {
            location::draw_gathering_location(frame, state, data, game_state, area, dim_level)
        }
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

/// Returns unique (name, rarity) pairs from a location's drop tables, sorted
/// by rarity ascending. Shared between grounds (location card) and the
/// at-location finds panel.
pub(super) fn collect_unique_drops(
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
