//! Adventures → In-Adventure (map exploration) sub-screen drawing.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::prelude::Frame;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

use super::super::state::TavernState;
use super::super::tile_graphics::{tile_image_id, TileBackend};
use crate::game::{self, GameData, GameState, SquareKind};
use crate::ui;

const MAP_TILE_W: u16 = 8; // cells wide per tile
const MAP_TILE_H: u16 = 4; // cells tall per tile (square in display)

pub(super) fn draw_in_adventure(
    frame: &mut Frame,
    state: &mut TavernState,
    data: &GameData,
    game_state: &GameState,
    area: Rect,
) {
    let Some(adventure) = &game_state.active_adventure else {
        return;
    };

    // Use dungeon map if present, otherwise fall back to the static quest map
    let map: &game::QuestMap;
    let quest_opt = data.quest(&adventure.quest_id);
    let dungeon_map_ref;
    if let Some(dm) = adventure.active_map() {
        dungeon_map_ref = dm;
        map = dungeon_map_ref;
    } else if let Some(q) = &quest_opt {
        map = &q.map;
    } else {
        return;
    };

    let name = adventure
        .dungeon_id
        .as_ref()
        .and_then(|did| data.dungeons.iter().find(|d| d.id == *did))
        .map(|d| d.name.clone())
        .or_else(|| quest_opt.map(|q| q.name.clone()))
        .unwrap_or_else(|| adventure.quest_id.clone());

    // Floor indicator for dungeons
    let title = if adventure.dungeon_id.is_some() {
        format!(
            "{} — Floor {}/{}",
            name,
            adventure.current_floor + 1,
            adventure.total_floors
        )
    } else {
        name
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(17),   // map area
            Constraint::Length(6), // info + log
            Constraint::Length(1), // hint
        ])
        .split(area);

    draw_map_with(frame, state, adventure, map, &title, chunks[0]);
    draw_adventure_info_with(frame, adventure, map, chunks[1]);

    let hint = Line::from(vec![
        Span::raw("  "),
        Span::styled(
            "↑↓←→",
            Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" move  ", Style::default().fg(ui::DIM)),
        Span::styled(
            "Enter",
            Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" interact  ", Style::default().fg(ui::DIM)),
        Span::styled(
            "I",
            Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" use item  ", Style::default().fg(ui::DIM)),
        Span::styled(
            "Esc",
            Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" abandon", Style::default().fg(ui::DIM)),
    ]);
    frame.render_widget(Paragraph::new(hint), chunks[2]);
}

fn draw_map_with(
    frame: &mut Frame,
    state: &mut TavernState,
    adventure: &game::ActiveAdventure,
    map: &game::QuestMap,
    title: &str,
    area: Rect,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ui::FOREST_GREEN))
        .title(Span::styled(
            format!(" {} ", title),
            Style::default()
                .fg(ui::FOREST_GREEN)
                .add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(ui::SHADOW_BG));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let tile_w = MAP_TILE_W;
    let tile_h = MAP_TILE_H;

    // How many tiles fit in the viewport
    let view_cols = (inner.width / tile_w) as u32;
    let view_rows = (inner.height / tile_h) as u32;

    if view_cols == 0 || view_rows == 0 {
        return;
    }

    // Camera: center on player position, clamped to map edges
    let (px, py) = adventure.position;
    let cam_x = if map.width <= view_cols {
        0 // map fits entirely — no scrolling needed
    } else {
        let half = view_cols / 2;
        (px as i32 - half as i32)
            .max(0)
            .min((map.width - view_cols) as i32) as u32
    };
    let cam_y = if map.height <= view_rows {
        0
    } else {
        let half = view_rows / 2;
        (py as i32 - half as i32)
            .max(0)
            .min((map.height - view_rows) as i32) as u32
    };

    // Visible tile range
    let vis_cols = view_cols.min(map.width);
    let vis_rows = view_rows.min(map.height);

    // Center the visible region within the inner area
    let used_w = vis_cols as u16 * tile_w;
    let used_h = vis_rows as u16 * tile_h;
    let off_x = inner.x + (inner.width.saturating_sub(used_w)) / 2;
    let off_y = inner.y + (inner.height.saturating_sub(used_h)) / 2;

    for vy in 0..vis_rows {
        for vx in 0..vis_cols {
            let mx = cam_x + vx;
            let my = cam_y + vy;
            if mx >= map.width || my >= map.height {
                continue;
            }
            let tx = off_x + vx as u16 * tile_w;
            let ty = off_y + vy as u16 * tile_h;
            let tile_area = Rect {
                x: tx,
                y: ty,
                width: tile_w,
                height: tile_h,
            };
            draw_map_tile(frame, state, adventure, map, mx, my, tile_area);
        }
    }
}

fn draw_map_tile(
    frame: &mut Frame,
    state: &mut TavernState,
    adventure: &game::ActiveAdventure,
    map: &game::QuestMap,
    x: u32,
    y: u32,
    area: Rect,
) {
    // Determine which tile to draw
    let tile = if adventure.position == (x, y) {
        ui::MapTile::Party
    } else if !adventure.is_revealed(x, y) {
        ui::MapTile::Fog
    } else {
        match map.get(x, y) {
            Some(SquareKind::Empty) => ui::MapTile::Empty,
            Some(SquareKind::Treasure { .. }) => {
                if adventure.is_completed(x, y) {
                    ui::MapTile::Empty
                } else {
                    ui::MapTile::Treasure
                }
            }
            Some(SquareKind::Rest) => ui::MapTile::Rest,
            Some(SquareKind::Trap { .. }) => {
                if adventure.is_completed(x, y) {
                    ui::MapTile::Empty
                } else {
                    ui::MapTile::Trap
                }
            }
            Some(SquareKind::Combat { .. }) => {
                if adventure.is_completed(x, y) {
                    ui::MapTile::Empty
                } else {
                    ui::MapTile::Combat
                }
            }
            Some(SquareKind::Boss { .. }) => ui::MapTile::Boss,
            Some(SquareKind::LadderDown) => ui::MapTile::LadderDown,
            Some(SquareKind::LadderUp) => ui::MapTile::LadderUp,
            None => ui::MapTile::Fog,
        }
    };

    // Render based on the tile graphics backend
    match state.tile_graphics.backend {
        TileBackend::KittyInline => {
            // Queue this tile for direct stdout write after ratatui's draw
            // completes. Mark all the tile's cells as skip so ratatui doesn't
            // emit anything for this region (which would otherwise overwrite
            // our images on the next frame).
            let id = tile_image_id(x, y);
            state
                .pending_kitty_tiles
                .push((area.x, area.y, tile, id, area.width, area.height));
            let buf = frame.buffer_mut();
            for dy in 0..area.height {
                for dx in 0..area.width {
                    if let Some(cell) = buf.cell_mut((area.x + dx, area.y + dy)) {
                        cell.set_symbol(" ");
                        cell.set_skip(true);
                    }
                }
            }
        }
        TileBackend::Ratatui => {
            if let Some(protocol) = state.tile_graphics.ratatui_get_mut(tile) {
                let image = ratatui_image::StatefulImage::default()
                    .resize(ratatui_image::Resize::Crop(None));
                frame.render_stateful_widget(image, area, protocol);
            } else {
                let lines = ui::map_tile(tile, area.width);
                frame.render_widget(Paragraph::new(lines), area);
            }
        }
    }
}

fn draw_adventure_info_with(
    frame: &mut Frame,
    adventure: &game::ActiveAdventure,
    map: &game::QuestMap,
    area: Rect,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ui::BORDER))
        .title(Span::styled(
            " Quest Log ",
            Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(ui::SHADOW_BG));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Current square description
    let (cx, cy) = adventure.position;
    let square_desc = match map.get(cx, cy) {
        Some(SquareKind::Empty) => "An empty stretch of cellar floor.",
        Some(SquareKind::Treasure { .. }) => {
            if adventure.is_completed(cx, cy) {
                "An opened chest."
            } else {
                "A dusty chest sits here."
            }
        }
        Some(SquareKind::Rest) => "A safe spot to catch your breath.",
        Some(SquareKind::Trap { .. }) => {
            if adventure.is_completed(cx, cy) {
                "Sprung trap remnants."
            } else {
                "Something glints in the dust...!"
            }
        }
        Some(SquareKind::Combat { .. }) => {
            if adventure.is_completed(cx, cy) {
                "Carcasses on the floor."
            } else {
                "Skittering in the dark..."
            }
        }
        Some(SquareKind::Boss { .. }) => "A massive shape stirs in the gloom.",
        Some(SquareKind::LadderDown) => "A ladder descends into darkness below.",
        Some(SquareKind::LadderUp) => "A ladder leads back up to the previous floor.",
        None => "",
    };

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(vec![
        Span::raw(" "),
        Span::styled(
            format!("({},{})  ", cx, cy),
            Style::default().fg(ui::DIM),
        ),
        Span::styled(square_desc, Style::default().fg(ui::WARM_WHITE)),
    ]));
    lines.push(Line::from(""));

    // Recent log entries (last 3)
    let recent: Vec<&String> = adventure.log.iter().rev().take(3).collect();
    for entry in recent.iter().rev() {
        lines.push(Line::from(vec![
            Span::raw(" "),
            Span::styled(entry.as_str(), Style::default().fg(ui::DIM)),
        ]));
    }

    frame.render_widget(Paragraph::new(lines), inner);
}
