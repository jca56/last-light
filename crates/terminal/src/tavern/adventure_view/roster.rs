//! Adventures → Roster sub-screen drawing.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::prelude::Frame;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

use super::super::state::{RosterFocus, TavernState};
use crate::game::{self, GameData, GameState};
use crate::ui;

pub(super) fn draw_roster(
    frame: &mut Frame,
    state: &TavernState,
    data: &GameData,
    game_state: &GameState,
    area: Rect,
) {
    if game_state.adventurers.is_empty() {
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(22), Constraint::Min(30)])
        .split(area);

    // Left: adventurer list
    draw_roster_list(frame, state, game_state, chunks[0]);
    // Right: selected adventurer detail (with portrait)
    draw_roster_detail(frame, state, data, game_state, chunks[1]);
}

fn draw_roster_list(
    frame: &mut Frame,
    state: &TavernState,
    game_state: &GameState,
    area: Rect,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ui::BORDER))
        .title(Span::styled(
            " Roster ",
            Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(ui::SHADOW_BG));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));
    for (i, adv) in game_state.adventurers.iter().enumerate() {
        let is_selected = i == state.adventure_view.selected_adventurer;
        let marker = if is_selected { " ▸ " } else { "   " };
        let name_style = if is_selected {
            Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(ui::WARM_WHITE)
        };
        let status_str = match &adv.status {
            game::AdventurerStatus::Ready => "",
            game::AdventurerStatus::OnQuest => " (quest)",
            game::AdventurerStatus::Recovering(_) => " (rest)",
            game::AdventurerStatus::Downed => " (down)",
        };
        lines.push(Line::from(vec![
            Span::styled(marker, Style::default().fg(ui::FLAME)),
            Span::styled(adv.name.clone(), name_style),
            Span::styled(status_str, Style::default().fg(ui::DIM)),
        ]));
        lines.push(Line::from(vec![
            Span::raw("    "),
            Span::styled(adv.class.label(), Style::default().fg(ui::DIM)),
            Span::styled(
                format!("  Lv {}", adv.level),
                Style::default().fg(ui::DIM),
            ),
        ]));
        lines.push(Line::from(""));
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

fn draw_roster_detail(
    frame: &mut Frame,
    state: &TavernState,
    data: &GameData,
    game_state: &GameState,
    area: Rect,
) {
    let idx = state.adventure_view.selected_adventurer;
    let Some(adv) = game_state.adventurers.get(idx) else {
        return;
    };
    let stats = adv.effective_stats(&data.item_registry);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ui::GOLD))
        .title(Line::from(vec![
            Span::styled(
                format!(" {} ", adv.name),
                Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("· {} ", adv.class.label()),
                Style::default().fg(ui::DIM),
            ),
        ]))
        .style(Style::default().bg(ui::SHADOW_BG));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let info_area = inner;
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));

    // Level and XP bar
    lines.push(Line::from(vec![
        Span::styled("  Level ", Style::default().fg(ui::DIM)),
        Span::styled(
            format!("{}", adv.level),
            Style::default().fg(ui::WARM_WHITE).add_modifier(Modifier::BOLD),
        ),
    ]));
    // XP progress bar
    if let Some((progress, needed)) = game::xp_progress(adv.xp, adv.level) {
        let bar_width = 20usize;
        let filled = if needed > 0 {
            ((progress as f64 / needed as f64) * bar_width as f64) as usize
        } else {
            bar_width
        };
        let empty = bar_width - filled;
        lines.push(Line::from(vec![
            Span::styled("  XP ", Style::default().fg(ui::DIM)),
            Span::styled("█".repeat(filled), Style::default().fg(ui::FLAME)),
            Span::styled("░".repeat(empty), Style::default().fg(ui::BORDER)),
            Span::styled(
                format!("  {}/{}", progress, needed),
                Style::default().fg(ui::DIM),
            ),
        ]));
    } else {
        lines.push(Line::from(vec![
            Span::styled("  XP ", Style::default().fg(ui::DIM)),
            Span::styled("MAX", Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD)),
        ]));
    }
    lines.push(Line::from(""));

    // Stats: effective = base + level growth + gear
    // Show effective total, with gear bonus in parentheses
    let levels_gained = adv.level.saturating_sub(1) as i32;
    let (hp_g, str_g, dex_g, int_g) = adv.class.growth();
    lines.push(Line::from(Span::styled(
        "  Stats",
        Style::default().fg(ui::DIM).add_modifier(Modifier::BOLD),
    )));
    let stat_rows = [
        ("  HP ", stats.max_hp, adv.base_stats.max_hp + hp_g * levels_gained),
        ("  STR", stats.strength, adv.base_stats.strength + str_g * levels_gained),
        ("  DEX", stats.dexterity, adv.base_stats.dexterity + dex_g * levels_gained),
        ("  INT", stats.intellect, adv.base_stats.intellect + int_g * levels_gained),
    ];
    for (label, effective, base_plus_level) in stat_rows {
        let gear_bonus = effective - base_plus_level;
        let mut spans = vec![
            Span::styled(format!("  {} ", label), Style::default().fg(ui::DIM)),
            Span::styled(
                format!("{}", effective),
                Style::default().fg(ui::WARM_WHITE).add_modifier(Modifier::BOLD),
            ),
        ];
        if gear_bonus != 0 {
            spans.push(Span::styled(
                format!("  ({} {:+})", base_plus_level, gear_bonus),
                Style::default().fg(ui::FOREST_GREEN),
            ));
        }
        lines.push(Line::from(spans));
    }
    lines.push(Line::from(""));

    // Equipment
    let equip_focused = state.adventure_view.roster_focus == RosterFocus::Equipment;
    let equip_title_style = if equip_focused {
        Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(ui::DIM).add_modifier(Modifier::BOLD)
    };
    lines.push(Line::from(Span::styled("  Equipment", equip_title_style)));
    let equip_slots = [
        (0, "  Weapon   ", &adv.equipment.weapon),
        (1, "  Armor    ", &adv.equipment.armor),
        (2, "  Accessory", &adv.equipment.accessory),
    ];
    for (slot_i, label, equipped) in equip_slots {
        let is_selected = equip_focused && slot_i == state.adventure_view.roster_equip_slot;
        let marker = if is_selected { "▸ " } else { "  " };
        let label_style = if is_selected {
            Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(ui::DIM)
        };
        match equipped {
            Some(id) => {
                let def = data.item_registry.get(id);
                let name = def
                    .map(|d| d.name.clone())
                    .unwrap_or_else(|| id.0.clone());
                let rarity = def.map(|d| d.rarity).unwrap_or(game::Rarity::Common);
                let color = ui::rarity_color(rarity);
                let mut stat_hint = String::new();
                if let Some(gs) = def.and_then(|d| d.properties.gear_stats.as_ref()) {
                    let parts: Vec<String> = [
                        (gs.hp, "HP"),
                        (gs.strength, "STR"),
                        (gs.dexterity, "DEX"),
                        (gs.intellect, "INT"),
                    ]
                    .iter()
                    .filter(|(v, _)| *v != 0)
                    .map(|(v, l)| format!("{:+}{}", v, l))
                    .collect();
                    if !parts.is_empty() {
                        stat_hint = format!("  ({})", parts.join(" "));
                    }
                }
                lines.push(Line::from(vec![
                    Span::styled(marker, Style::default().fg(ui::FLAME)),
                    Span::styled(label, label_style),
                    Span::styled(name, Style::default().fg(color)),
                    Span::styled(stat_hint, Style::default().fg(ui::DIM)),
                ]));
            }
            None => {
                lines.push(Line::from(vec![
                    Span::styled(marker, Style::default().fg(ui::FLAME)),
                    Span::styled(label, label_style),
                    Span::styled("— empty —", Style::default().fg(ui::DIM)),
                ]));
            }
        }
    }
    lines.push(Line::from(""));

    // Consumables
    lines.push(Line::from(Span::styled(
        "  Consumables",
        Style::default().fg(ui::DIM).add_modifier(Modifier::BOLD),
    )));
    for slot_i in 0..game::CONSUMABLE_SLOTS {
        let combined_slot = 3 + slot_i; // 0-2 are equipment, 3+ are consumables
        let is_selected = equip_focused && combined_slot == state.adventure_view.roster_equip_slot;
        let marker = if is_selected { "▸ " } else { "  " };
        let label_style = if is_selected {
            Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(ui::DIM)
        };
        let slot_label = format!("  {}. ", slot_i + 1);
        let item = adv.consumables.get(slot_i).and_then(|s| s.as_ref());
        match item {
            Some(id) => {
                let def = data.item_registry.get(id);
                let name = def
                    .map(|d| d.name.clone())
                    .unwrap_or_else(|| id.0.clone());
                let rarity = def.map(|d| d.rarity).unwrap_or(game::Rarity::Common);
                let color = ui::rarity_color(rarity);
                lines.push(Line::from(vec![
                    Span::styled(marker, Style::default().fg(ui::FLAME)),
                    Span::styled(slot_label, label_style),
                    Span::styled(name, Style::default().fg(color)),
                ]));
            }
            None => {
                lines.push(Line::from(vec![
                    Span::styled(marker, Style::default().fg(ui::FLAME)),
                    Span::styled(slot_label, label_style),
                    Span::styled("— empty —", Style::default().fg(ui::BORDER)),
                ]));
            }
        }
    }
    lines.push(Line::from(""));

    // Status
    let status = match &adv.status {
        game::AdventurerStatus::Ready => ("Ready", ui::FOREST_GREEN),
        game::AdventurerStatus::OnQuest => ("On Quest", ui::GOLD),
        game::AdventurerStatus::Recovering(_) => ("Recovering", ui::FLAME),
        game::AdventurerStatus::Downed => ("Downed", ui::EMBER),
    };
    lines.push(Line::from(vec![
        Span::styled("  Status  ", Style::default().fg(ui::DIM)),
        Span::styled(
            status.0,
            Style::default().fg(status.1).add_modifier(Modifier::BOLD),
        ),
    ]));

    // Item picker or hint bar
    let picker_active = state.adventure_view.roster_focus == RosterFocus::ItemPicker;
    if picker_active {
        // Show the item picker list
        let slot = state.adventure_view.roster_equip_slot;
        let registry = game::ItemRegistry::new();
        let compatible = crate::tavern::input::compatible_items_for_slot(game_state, slot);

        lines.push(Line::from(""));
        let slot_label = if slot <= 2 {
            match slot {
                0 => "Weapon",
                1 => "Armor",
                _ => "Accessory",
            }
        } else {
            "Consumable"
        };
        lines.push(Line::from(Span::styled(
            format!("  Choose {} ─────────────────", slot_label),
            Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
        )));

        if compatible.is_empty() {
            lines.push(Line::from(Span::styled(
                "  No compatible items in inventory.",
                Style::default().fg(ui::DIM),
            )));
        } else {
            for (i, (item_id, qty)) in compatible.iter().enumerate() {
                let is_selected = i == state.adventure_view.roster_picker_idx;
                let marker = if is_selected { " ▸ " } else { "   " };
                let def = registry.get(item_id);
                let name = def
                    .map(|d| d.name.clone())
                    .unwrap_or_else(|| item_id.0.clone());
                let rarity = def.map(|d| d.rarity).unwrap_or(game::Rarity::Common);
                let color = ui::rarity_color(rarity);
                let name_style = if is_selected {
                    Style::default().fg(color).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(color)
                };

                // Build stat hint
                let mut stat_str = String::new();
                if let Some(gs) = def.and_then(|d| d.properties.gear_stats.as_ref()) {
                    let parts: Vec<String> = [
                        (gs.hp, "HP"),
                        (gs.strength, "STR"),
                        (gs.dexterity, "DEX"),
                        (gs.intellect, "INT"),
                    ]
                    .iter()
                    .filter(|(v, _)| *v != 0)
                    .map(|(v, l)| format!("{:+}{}", v, l))
                    .collect();
                    if !parts.is_empty() {
                        stat_str = format!("  {}", parts.join(" "));
                    }
                }

                lines.push(Line::from(vec![
                    Span::styled(marker, Style::default().fg(ui::FLAME)),
                    Span::styled(name, name_style),
                    Span::styled(stat_str, Style::default().fg(ui::DIM)),
                    Span::styled(format!("  x{}", qty), Style::default().fg(ui::DIM)),
                ]));
            }
        }

        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("  ↑↓", Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD)),
            Span::styled(" select  ", Style::default().fg(ui::DIM)),
            Span::styled("Enter", Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD)),
            Span::styled(" equip  ", Style::default().fg(ui::DIM)),
            Span::styled("Esc", Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD)),
            Span::styled(" cancel", Style::default().fg(ui::DIM)),
        ]));
    } else {
        // Standard hint bar
        lines.push(Line::from(""));
        if equip_focused {
            lines.push(Line::from(vec![
                Span::styled("  ↑↓", Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD)),
                Span::styled(" slot  ", Style::default().fg(ui::DIM)),
                Span::styled("Enter", Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD)),
                Span::styled(" equip  ", Style::default().fg(ui::DIM)),
                Span::styled("X", Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD)),
                Span::styled(" unequip  ", Style::default().fg(ui::DIM)),
                Span::styled("Esc", Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD)),
                Span::styled(" back", Style::default().fg(ui::DIM)),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::styled("  ↑↓", Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD)),
                Span::styled(" browse  ", Style::default().fg(ui::DIM)),
                Span::styled("Enter", Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD)),
                Span::styled(" equip  ", Style::default().fg(ui::DIM)),
                Span::styled("Tab", Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD)),
                Span::styled(" quest board", Style::default().fg(ui::DIM)),
            ]));
        }
    }

    frame.render_widget(Paragraph::new(lines), info_area);
}
