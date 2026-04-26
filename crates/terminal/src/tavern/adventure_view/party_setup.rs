//! Adventures → Party Setup sub-screen drawing.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::prelude::Frame;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

use super::super::state::{PartySetupFocus, TavernState};
use crate::game::{self, GameData, GameState};
use crate::ui;

pub(super) fn draw_party_setup(
    frame: &mut Frame,
    state: &TavernState,
    data: &GameData,
    game_state: &GameState,
    area: Rect,
) {
    let idx = state.adventure_view.selected_quest;
    let quest_count = data.quests.len();

    // Extract name, party limits, and reward info from either quest or dungeon
    let (name, min_p, max_p, reward_text) = if idx < quest_count {
        let Some(q) = data.quests.get(idx) else {
            return;
        };
        (
            q.name.clone(),
            q.min_party,
            q.max_party,
            format!("Reward: {}g + {} XP", q.completion_gold, q.xp_reward),
        )
    } else {
        let di = idx - quest_count;
        let Some(d) = data.dungeons.get(di) else {
            return;
        };
        (
            d.name.clone(),
            d.min_party,
            d.max_party,
            format!(
                "Reward: {}g + {} XP  ·  {}-{} floors",
                d.completion_gold, d.completion_xp, d.min_floors, d.max_floors
            ),
        )
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),  // quest summary
            Constraint::Min(10),    // party slots + equipment
            Constraint::Length(1),  // hint
        ])
        .split(area);

    // Quest/dungeon summary
    let mut summary: Vec<Line> = Vec::new();
    summary.push(Line::from(""));
    summary.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(
            name,
            Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("    Party: {}-{}    ", min_p, max_p),
            Style::default().fg(ui::DIM),
        ),
        Span::styled(reward_text, Style::default().fg(ui::DIM)),
    ]));
    frame.render_widget(Paragraph::new(summary), chunks[0]);

    // Party slot picker on left, equipment editor on right
    let split = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    draw_party_slots(frame, state, game_state, split[0]);
    draw_equipment_editor(frame, state, data, game_state, split[1]);

    // Hint
    let hint_text = if state.adventure_view.picking_adventurer {
        vec![
            Span::raw("  "),
            Span::styled(
                "↑↓",
                Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" choose  ", Style::default().fg(ui::DIM)),
            Span::styled(
                "Enter",
                Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" assign  ", Style::default().fg(ui::DIM)),
            Span::styled(
                "Esc",
                Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" cancel", Style::default().fg(ui::DIM)),
        ]
    } else {
        vec![
            Span::raw("  "),
            Span::styled(
                "Tab",
                Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" focus  ", Style::default().fg(ui::DIM)),
            Span::styled(
                "↑↓",
                Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" navigate  ", Style::default().fg(ui::DIM)),
            Span::styled(
                "Enter",
                Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" assign/equip  ", Style::default().fg(ui::DIM)),
            Span::styled(
                "X",
                Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" clear  ", Style::default().fg(ui::DIM)),
            Span::styled(
                "S",
                Style::default().fg(ui::FLAME).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" start  ", Style::default().fg(ui::DIM)),
            Span::styled(
                "Esc",
                Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" back", Style::default().fg(ui::DIM)),
        ]
    };
    frame.render_widget(Paragraph::new(Line::from(hint_text)), chunks[2]);
}

fn draw_party_slots(
    frame: &mut Frame,
    state: &TavernState,
    game_state: &GameState,
    area: Rect,
) {
    let focused = state.adventure_view.setup_focus == PartySetupFocus::PartySlots;
    let border = if focused { ui::GOLD } else { ui::BORDER };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border))
        .title(Span::styled(
            " Party ",
            Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(ui::SHADOW_BG));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));
    for i in 0..3 {
        let is_selected = focused && !state.adventure_view.picking_adventurer
            && i == state.adventure_view.setup_slot;
        let marker = if is_selected { " ▸ " } else { "   " };

        let slot_label = format!("Slot {}", i + 1);
        match state.adventure_view.party_slots[i] {
            Some(adv_idx) => {
                if let Some(adv) = game_state.adventurers.get(adv_idx) {
                    let stats = &adv.base_stats;
                    lines.push(Line::from(vec![
                        Span::styled(marker, Style::default().fg(ui::FLAME)),
                        Span::styled(
                            slot_label,
                            Style::default().fg(ui::DIM),
                        ),
                        Span::raw("  "),
                        Span::styled(
                            adv.name.clone(),
                            Style::default()
                                .fg(ui::GOLD)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!("  {}", adv.class.label()),
                            Style::default().fg(ui::DIM),
                        ),
                    ]));
                    lines.push(Line::from(vec![
                        Span::raw("       "),
                        Span::styled(
                            format!(
                                "HP {}  STR {}  DEX {}  INT {}",
                                stats.max_hp, stats.strength, stats.dexterity, stats.intellect
                            ),
                            Style::default().fg(ui::WARM_WHITE),
                        ),
                    ]));
                }
            }
            None => {
                lines.push(Line::from(vec![
                    Span::styled(marker, Style::default().fg(ui::FLAME)),
                    Span::styled(slot_label, Style::default().fg(ui::DIM)),
                    Span::raw("  "),
                    Span::styled("— empty —", Style::default().fg(ui::DIM)),
                ]));
                lines.push(Line::from(""));
            }
        }
        lines.push(Line::from(""));
    }

    // If picking an adventurer, show the picker
    if state.adventure_view.picking_adventurer && focused {
        lines.push(Line::from(Span::styled(
            "  Choose:",
            Style::default().fg(ui::DIM).add_modifier(Modifier::BOLD),
        )));
        for (i, adv) in game_state.adventurers.iter().enumerate() {
            let is_selected = i == state.adventure_view.picker_idx;
            let marker = if is_selected { " ▸ " } else { "   " };
            let style = if is_selected {
                Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(ui::WARM_WHITE)
            };
            let in_party = state
                .adventure_view
                .party_slots
                .iter()
                .any(|s| s == &Some(i));
            let suffix = if in_party { " (in party)" } else { "" };
            lines.push(Line::from(vec![
                Span::styled(marker, Style::default().fg(ui::FLAME)),
                Span::styled(format!("{} ({})", adv.name, adv.class.label()), style),
                Span::styled(suffix, Style::default().fg(ui::DIM)),
            ]));
        }
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

fn draw_equipment_editor(
    frame: &mut Frame,
    state: &TavernState,
    data: &GameData,
    game_state: &GameState,
    area: Rect,
) {
    let focused = state.adventure_view.setup_focus == PartySetupFocus::EquipmentSlots;
    let border = if focused { ui::GOLD } else { ui::BORDER };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border))
        .title(Span::styled(
            " Equipment ",
            Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(ui::SHADOW_BG));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let slot_idx = state.adventure_view.setup_slot;
    let Some(adv_idx) = state.adventure_view.party_slots.get(slot_idx).copied().flatten() else {
        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "  Pick a party member first.",
                Style::default().fg(ui::DIM),
            )),
        ];
        frame.render_widget(Paragraph::new(lines), inner);
        return;
    };
    let Some(adv) = game_state.adventurers.get(adv_idx) else {
        return;
    };

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(
            adv.name.clone(),
            Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("  ({})", adv.class.label()),
            Style::default().fg(ui::DIM),
        ),
    ]));
    lines.push(Line::from(""));

    let gear_slots: [(usize, &str, &Option<game::ItemId>); 3] = [
        (0, "Weapon", &adv.equipment.weapon),
        (1, "Armor", &adv.equipment.armor),
        (2, "Accessory", &adv.equipment.accessory),
    ];
    for (i, label, equipped) in gear_slots {
        let is_selected = focused && i == state.adventure_view.setup_equip_slot;
        let marker = if is_selected { " ▸ " } else { "   " };
        let name_style = if is_selected {
            Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(ui::WARM_WHITE)
        };
        let equipped_text = match equipped {
            Some(id) => data
                .item_registry
                .get(id)
                .map(|d| d.name.clone())
                .unwrap_or_else(|| id.0.clone()),
            None => "— empty —".into(),
        };
        let equipped_style = if equipped.is_some() {
            Style::default().fg(ui::WARM_WHITE)
        } else {
            Style::default().fg(ui::DIM)
        };
        lines.push(Line::from(vec![
            Span::styled(marker, Style::default().fg(ui::FLAME)),
            Span::styled(format!("{:<10}", label), name_style),
            Span::styled(equipped_text, equipped_style),
        ]));
    }

    // Consumable slots
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "   Items",
        Style::default().fg(ui::DIM).add_modifier(Modifier::BOLD),
    )));
    for slot_i in 0..game::CONSUMABLE_SLOTS {
        let combined = 3 + slot_i;
        let is_selected = focused && combined == state.adventure_view.setup_equip_slot;
        let marker = if is_selected { " ▸ " } else { "   " };
        let name_style = if is_selected {
            Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(ui::WARM_WHITE)
        };
        let item = adv.consumables.get(slot_i).and_then(|s| s.as_ref());
        let (text, style) = match item {
            Some(id) => {
                let name = data
                    .item_registry
                    .get(id)
                    .map(|d| d.name.clone())
                    .unwrap_or_else(|| id.0.clone());
                (name, Style::default().fg(ui::WARM_WHITE))
            }
            None => ("— empty —".into(), Style::default().fg(ui::DIM)),
        };
        lines.push(Line::from(vec![
            Span::styled(marker, Style::default().fg(ui::FLAME)),
            Span::styled(format!("Slot {}    ", slot_i + 1), name_style),
            Span::styled(text, style),
        ]));
    }

    // Effective stats
    let stats = adv.effective_stats(&data.item_registry);
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  Stats:  ", Style::default().fg(ui::DIM)),
        Span::styled(
            format!(
                "HP {}  STR {}  DEX {}  INT {}",
                stats.max_hp, stats.strength, stats.dexterity, stats.intellect
            ),
            Style::default().fg(ui::WARM_WHITE),
        ),
    ]));

    frame.render_widget(Paragraph::new(lines), inner);
}
