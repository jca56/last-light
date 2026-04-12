//! Drawing for the left column: Lanty box and the unified Active tasks panel.

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::prelude::Frame;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

use super::gathering_view::draw_loot_popup;
use super::state::{BottomTab, PopupSource, TavernState};
use super::util::format_duration;
use crate::game::{self, GameData, GameState};
use crate::ui;

pub(super) fn draw_left(
    frame: &mut Frame,
    state: &TavernState,
    area: Rect,
    data: &GameData,
    game_state: &GameState,
) {
    // Lanty box (or Party box during adventure), Active panel (bottom)
    let active_height: u16 = 12;

    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(8),                    // top: Lanty or Party
            Constraint::Length(active_height),     // active tasks panel
        ])
        .split(area);

    // Show party box when an adventure is active
    if game_state.active_adventure.is_some() {
        draw_party_box(frame, left_chunks[0], data, game_state);
    } else {
        draw_lanty_box(frame, left_chunks[0], game_state);
    }
    draw_active_panel(frame, state, data, game_state, left_chunks[1]);
}

fn draw_party_box(frame: &mut Frame, area: Rect, data: &GameData, game_state: &GameState) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ui::FOREST_GREEN))
        .title(Span::styled(
            " Party ",
            Style::default()
                .fg(ui::FOREST_GREEN)
                .add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(ui::SHADOW_BG));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(adventure) = &game_state.active_adventure else {
        return;
    };

    let mut lines: Vec<Line> = Vec::new();
    let bar_width: usize = (inner.width.saturating_sub(8) as usize).clamp(8, 18);

    for (i, member) in adventure.party.iter().enumerate() {
        let Some(adv) = game_state.adventurers.get(member.roster_idx) else {
            continue;
        };
        let hp_ratio = if member.max_hp > 0 {
            member.current_hp.max(0) as f64 / member.max_hp as f64
        } else {
            0.0
        };
        let filled = (hp_ratio * bar_width as f64) as usize;
        let bar: String = "█".repeat(filled) + &"░".repeat(bar_width.saturating_sub(filled));
        let bar_color = if member.downed {
            ui::DIM
        } else if hp_ratio < 0.34 {
            ui::EMBER
        } else if hp_ratio < 0.67 {
            ui::FLAME
        } else {
            ui::FOREST_GREEN
        };

        let name_style = if member.downed {
            Style::default().fg(ui::DIM)
        } else {
            Style::default()
                .fg(ui::WARM_WHITE)
                .add_modifier(Modifier::BOLD)
        };

        // Separator between party members
        if i > 0 {
            lines.push(Line::from(Span::styled(
                "─".repeat(inner.width as usize),
                Style::default().fg(ui::BORDER),
            )));
        }

        // Name + class
        lines.push(Line::from(vec![
            Span::raw(" "),
            Span::styled(adv.name.clone(), name_style),
            Span::styled(
                format!("  {}", adv.class.label()),
                Style::default().fg(ui::DIM),
            ),
        ]));

        // HP bar
        lines.push(Line::from(vec![
            Span::raw(" "),
            Span::styled(bar, Style::default().fg(bar_color)),
            Span::styled(
                format!(" {}/{}", member.current_hp.max(0), member.max_hp),
                Style::default().fg(ui::DIM),
            ),
        ]));

        // Stats line
        lines.push(Line::from(vec![
            Span::styled(" STR ", Style::default().fg(ui::DIM)),
            Span::styled(
                format!("{}", member.strength),
                Style::default().fg(ui::WARM_WHITE),
            ),
            Span::styled("  DEX ", Style::default().fg(ui::DIM)),
            Span::styled(
                format!("{}", member.dexterity),
                Style::default().fg(ui::WARM_WHITE),
            ),
            Span::styled("  INT ", Style::default().fg(ui::DIM)),
            Span::styled(
                format!("{}", member.intellect),
                Style::default().fg(ui::WARM_WHITE),
            ),
        ]));

        // Equipment slots
        let equip_slots = [
            ("Wpn", &adv.equipment.weapon),
            ("Arm", &adv.equipment.armor),
            ("Acc", &adv.equipment.accessory),
        ];
        for (label, equipped) in equip_slots {
            match equipped {
                Some(id) => {
                    let name = data
                        .item_registry
                        .get(id)
                        .map(|d| d.name.clone())
                        .unwrap_or_else(|| id.0.clone());
                    let rarity = data
                        .item_registry
                        .get(id)
                        .map(|d| d.rarity)
                        .unwrap_or(game::Rarity::Common);
                    let color = ui::rarity_color(rarity);
                    lines.push(Line::from(vec![
                        Span::styled(format!(" {} ", label), Style::default().fg(ui::DIM)),
                        Span::styled(name, Style::default().fg(color)),
                    ]));
                }
                None => {
                    lines.push(Line::from(vec![
                        Span::styled(format!(" {} ", label), Style::default().fg(ui::DIM)),
                        Span::styled("— empty —", Style::default().fg(ui::DIM)),
                    ]));
                }
            }
        }
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

fn draw_lanty_box(frame: &mut Frame, area: Rect, game_state: &GameState) {
    let lanty_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ui::BORDER))
        .title(Span::styled(" Lanty ", Style::default().fg(ui::GOLD)))
        .style(Style::default().bg(ui::SHADOW_BG));

    let lanty_inner = lanty_block.inner(area);
    frame.render_widget(lanty_block, area);

    // Reserve the bottom 2 rows for day/time
    let info_h: u16 = 2;
    let portrait_h = lanty_inner.height.saturating_sub(info_h);

    let portrait_area = Rect {
        x: lanty_inner.x,
        y: lanty_inner.y,
        width: lanty_inner.width,
        height: portrait_h,
    };
    let info_area = Rect {
        x: lanty_inner.x,
        y: lanty_inner.y + portrait_h,
        width: lanty_inner.width,
        height: info_h.min(lanty_inner.height),
    };

    if portrait_area.height > 0 {
        let lanty_w = portrait_area.width.min(20);
        let mush = ui::lanty_portrait(lanty_w);
        frame.render_widget(
            Paragraph::new(mush).alignment(Alignment::Center),
            portrait_area,
        );
    }

    if info_area.height > 0 {
        let info = Line::from(vec![
            Span::styled(
                format!("Day {}", game_state.day),
                Style::default()
                    .fg(ui::WARM_WHITE)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  ·  ", Style::default().fg(ui::DIM)),
            Span::styled("Evening", Style::default().fg(ui::GOLD)),
        ]);
        frame.render_widget(
            Paragraph::new(info).alignment(Alignment::Center),
            info_area,
        );
    }
}

fn draw_active_panel(
    frame: &mut Frame,
    state: &TavernState,
    data: &GameData,
    game_state: &GameState,
    area: Rect,
) {
    // Title with right-aligned gold counter
    let gold_text = format!(" {}g ", game_state.gold);
    let gold_span = Span::styled(
        gold_text,
        Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
    );

    let title = Line::from(vec![Span::styled(
        " Active ",
        Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
    )]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ui::BORDER))
        .title(title)
        .title_top(Line::from(gold_span).right_aligned())
        .style(Style::default().bg(ui::SHADOW_BG));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width < 4 || inner.height < 2 {
        return;
    }

    // Reserve top row for the tab strip
    let tabs_area = Rect {
        x: inner.x,
        y: inner.y,
        width: inner.width,
        height: 1,
    };
    let content_area = Rect {
        x: inner.x,
        y: inner.y + 1,
        width: inner.width,
        height: inner.height.saturating_sub(1),
    };

    draw_tab_strip(frame, state.bottom_tab, tabs_area);

    match state.bottom_tab {
        BottomTab::Expeditions => draw_expeditions_tab(frame, state, data, game_state, content_area),
        BottomTab::Refining => draw_refining_tab(frame, state, data, game_state, content_area),
        BottomTab::Crafting => draw_crafting_tab(frame, state, data, game_state, content_area),
        BottomTab::Adventures => draw_adventures_tab(frame, content_area, data, game_state),
    }
}

fn draw_tab_strip(frame: &mut Frame, active: BottomTab, area: Rect) {
    let mut spans: Vec<Span<'static>> = Vec::new();
    spans.push(Span::raw(" "));
    for (i, tab) in BottomTab::all().iter().enumerate() {
        let is_active = *tab == active;
        if is_active {
            spans.push(Span::styled(
                "▸",
                Style::default().fg(ui::FLAME).add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::styled(
                format!(" {} ", tab.label()),
                Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::raw(" "));
            spans.push(Span::styled(
                format!(" {} ", tab.label()),
                Style::default().fg(ui::DIM),
            ));
        }
        if i + 1 < BottomTab::all().len() {
            spans.push(Span::styled("·", Style::default().fg(ui::DIM)));
        }
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

// ── Expeditions tab ────────────────────────────────────────────────────────

fn draw_expeditions_tab(
    frame: &mut Frame,
    state: &TavernState,
    data: &GameData,
    game_state: &GameState,
    area: Rect,
) {
    let bar_width = area.width.saturating_sub(2) as usize;
    let mut lines: Vec<Line> = Vec::new();

    for (i, slot) in game_state.gathering.slots.iter().enumerate() {
        let slot_label = format!("{}", i + 1);
        match slot {
            Some(task) => {
                let location_name = data
                    .location(&task.location_id)
                    .map(|l| l.name.clone())
                    .unwrap_or_else(|| task.location_id.clone());
                let progress = task.progress();
                let filled = (progress * bar_width as f64) as usize;
                let bar: String =
                    "█".repeat(filled) + &"░".repeat(bar_width.saturating_sub(filled));
                let bar_color = if task.is_complete() {
                    ui::FLAME
                } else {
                    ui::FOREST_GREEN
                };
                let time_str = format_duration(task.remaining_ms());

                lines.push(Line::from(vec![
                    Span::styled(format!(" {}  ", slot_label), Style::default().fg(ui::DIM)),
                    Span::styled(
                        location_name,
                        Style::default().fg(ui::WARM_WHITE).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(format!("  {}", time_str), Style::default().fg(ui::DIM)),
                ]));
                lines.push(Line::from(vec![
                    Span::raw(" "),
                    Span::styled(bar, Style::default().fg(bar_color)),
                ]));
            }
            None => {
                let empty_bar: String = "░".repeat(bar_width);
                lines.push(Line::from(vec![
                    Span::styled(format!(" {}  ", slot_label), Style::default().fg(ui::DIM)),
                    Span::styled("— idle —", Style::default().fg(ui::DIM)),
                ]));
                lines.push(Line::from(vec![
                    Span::raw(" "),
                    Span::styled(empty_bar, Style::default().fg(ui::DIM)),
                ]));
            }
        }
        lines.push(Line::from(""));
    }

    frame.render_widget(Paragraph::new(lines), area);

    // Loot popups: gather only on this tab
    for popup in state.loot_popups.iter() {
        let PopupSource::Gather { slot_index, .. } = &popup.source else {
            continue;
        };
        if *slot_index >= game_state.gathering.slots.len() {
            continue;
        }
        let slot_anchor_y = area.y + *slot_index as u16 * 3 + 1;
        draw_loot_popup(frame, popup, area, slot_anchor_y, 3);
    }
}

// ── Refining tab ───────────────────────────────────────────────────────────

fn draw_refining_tab(
    frame: &mut Frame,
    state: &TavernState,
    data: &GameData,
    game_state: &GameState,
    area: Rect,
) {
    let bar_width = area.width.saturating_sub(2) as usize;
    let mut lines: Vec<Line> = Vec::new();

    let stations = [
        game::StationKind::Workbench,
        game::StationKind::Furnace,
        game::StationKind::Alchemy,
        game::StationKind::Loom,
    ];

    for kind in stations {
        let station_unlocked = data
            .refining_stations
            .iter()
            .find(|s| s.kind == kind)
            .map(|s| s.unlocked)
            .unwrap_or(false);

        let label_short = short_station(kind);
        let slot = game_state.refining.slot(kind);

        match slot {
            Some(task) => {
                let recipe = data.recipe(&task.recipe_id);
                let recipe_name = recipe
                    .map(|r| r.name.clone())
                    .unwrap_or_else(|| task.recipe_id.clone());
                let progress = task.current_unit_progress();
                let filled = (progress * bar_width as f64) as usize;
                let bar: String =
                    "█".repeat(filled) + &"░".repeat(bar_width.saturating_sub(filled));
                let next_in = format_duration(task.next_unit_remaining_ms());

                lines.push(Line::from(vec![
                    Span::styled(format!(" {}  ", label_short), Style::default().fg(ui::DIM)),
                    Span::styled(
                        recipe_name,
                        Style::default().fg(ui::WARM_WHITE).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("  {}/{}  ", task.completed_units, task.total_units),
                        Style::default().fg(ui::DIM),
                    ),
                    Span::styled(next_in, Style::default().fg(ui::DIM)),
                ]));
                lines.push(Line::from(vec![
                    Span::raw(" "),
                    Span::styled(bar, Style::default().fg(ui::FOREST_GREEN)),
                ]));
            }
            None => {
                let empty_bar: String = "░".repeat(bar_width);
                let status = if station_unlocked { "— idle —" } else { "locked" };
                lines.push(Line::from(vec![
                    Span::styled(format!(" {}  ", label_short), Style::default().fg(ui::DIM)),
                    Span::styled(status, Style::default().fg(ui::DIM)),
                ]));
                lines.push(Line::from(vec![
                    Span::raw(" "),
                    Span::styled(empty_bar, Style::default().fg(ui::DIM)),
                ]));
            }
        }
        lines.push(Line::from(""));
    }

    frame.render_widget(Paragraph::new(lines), area);

    // Loot popups: refining only on this tab
    for popup in state.loot_popups.iter() {
        let PopupSource::Refine { station } = &popup.source else {
            continue;
        };
        let idx = stations.iter().position(|k| k == station);
        if let Some(idx) = idx {
            let anchor_y = area.y + idx as u16 * 3 + 1;
            draw_loot_popup(frame, popup, area, anchor_y, 3);
        }
    }
}

fn short_station(kind: game::StationKind) -> &'static str {
    match kind {
        game::StationKind::Workbench => "Bench",
        game::StationKind::Furnace => "Furn ",
        game::StationKind::Alchemy => "Alch ",
        game::StationKind::Loom => "Loom ",
    }
}

// ── Crafting tab ───────────────────────────────────────────────────────────

fn draw_crafting_tab(
    frame: &mut Frame,
    state: &TavernState,
    data: &GameData,
    game_state: &GameState,
    area: Rect,
) {
    let bar_width = area.width.saturating_sub(2) as usize;
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

            lines.push(Line::from(vec![
                Span::styled(" Bench  ", Style::default().fg(ui::DIM)),
                Span::styled(
                    recipe_name,
                    Style::default().fg(ui::WARM_WHITE).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("  {}/{}  ", task.completed_units, task.total_units),
                    Style::default().fg(ui::DIM),
                ),
                Span::styled(next_in, Style::default().fg(ui::DIM)),
            ]));
            lines.push(Line::from(vec![
                Span::raw(" "),
                Span::styled(bar, Style::default().fg(ui::FOREST_GREEN)),
            ]));
        }
        None => {
            let empty_bar: String = "░".repeat(bar_width);
            lines.push(Line::from(vec![
                Span::styled(" Bench  ", Style::default().fg(ui::DIM)),
                Span::styled("— idle —", Style::default().fg(ui::DIM)),
            ]));
            lines.push(Line::from(vec![
                Span::raw(" "),
                Span::styled(empty_bar, Style::default().fg(ui::DIM)),
            ]));
        }
    }

    frame.render_widget(Paragraph::new(lines), area);

    // Loot popups: crafting only on this tab
    for popup in state.loot_popups.iter() {
        if !matches!(popup.source, PopupSource::Craft) {
            continue;
        }
        let anchor_y = area.y + 1;
        draw_loot_popup(frame, popup, area, anchor_y, 3);
    }
}

// ── Adventures tab ─────────────────────────────────────────────────────────

fn draw_adventures_tab(frame: &mut Frame, area: Rect, data: &GameData, game_state: &GameState) {
    let mut lines: Vec<Line> = Vec::new();
    match &game_state.active_adventure {
        Some(adventure) => {
            let quest_name = data
                .quest(&adventure.quest_id)
                .map(|q| q.name.clone())
                .unwrap_or_else(|| adventure.quest_id.clone());
            let pos = adventure.position;
            let alive = adventure.party.iter().filter(|p| !p.downed).count();
            let total = adventure.party.len();
            let state_label = match &adventure.state {
                game::AdventureState::Exploring => "Exploring",
                game::AdventureState::InCombat(_) => "In Combat",
                game::AdventureState::Complete { success: true } => "Complete",
                game::AdventureState::Complete { success: false } => "Failed",
            };
            lines.push(Line::from(vec![
                Span::raw(" "),
                Span::styled(
                    quest_name,
                    Style::default()
                        .fg(ui::WARM_WHITE)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::raw(" "),
                Span::styled(state_label, Style::default().fg(ui::FOREST_GREEN)),
                Span::styled(
                    format!("   ({},{})", pos.0, pos.1),
                    Style::default().fg(ui::DIM),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::raw(" "),
                Span::styled(
                    format!("Party: {}/{} alive", alive, total),
                    Style::default().fg(ui::DIM),
                ),
            ]));
        }
        None => {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  No adventurers are out.",
                Style::default().fg(ui::DIM),
            )));
        }
    }
    frame.render_widget(Paragraph::new(lines), area);
}
