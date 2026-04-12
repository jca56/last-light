//! Drawing for the Adventures view: Quest Board, Party Setup, In-Adventure
//! map, Combat, and Results sub-screens.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::prelude::Frame;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

use super::state::{AdventureScreen, PartySetupFocus, TavernState};
use super::util::{inset_rect, wrap_text};
use crate::game::{
    self, AdventureState, GameData, GameState, Quest, SquareKind,
};
use crate::ui;

pub(super) fn draw_adventure(
    frame: &mut Frame,
    state: &mut TavernState,
    data: &GameData,
    game_state: &GameState,
    area: Rect,
) {
    match state.adventure_view.screen {
        // Top-level screens get a sub-tab strip
        AdventureScreen::Roster | AdventureScreen::QuestBoard => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(1), Constraint::Min(5)])
                .split(area);
            draw_adventure_tabs(frame, state, chunks[0]);
            match state.adventure_view.screen {
                AdventureScreen::Roster => {
                    draw_roster(frame, state, data, game_state, chunks[1])
                }
                _ => draw_quest_board(frame, state, data, chunks[1]),
            }
        }
        // Sub-screens don't show the tab strip (full screen)
        AdventureScreen::PartySetup => draw_party_setup(frame, state, data, game_state, area),
        AdventureScreen::InAdventure => draw_in_adventure(frame, state, data, game_state, area),
        AdventureScreen::Combat => draw_combat(frame, state, data, game_state, area),
        AdventureScreen::Results => draw_results(frame, state, data, game_state, area),
    }
}

fn draw_adventure_tabs(frame: &mut Frame, state: &TavernState, area: Rect) {
    let on_roster = state.adventure_view.screen == AdventureScreen::Roster;
    let tabs = [
        ("Adventurers", on_roster),
        ("Quest Board", !on_roster),
    ];
    let mut spans: Vec<Span<'static>> = vec![Span::raw("  ")];
    for (i, (label, active)) in tabs.iter().enumerate() {
        if *active {
            spans.push(Span::styled(
                "▸ ",
                Style::default().fg(ui::FLAME).add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::styled(
                *label,
                Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled("  ", Style::default()));
            spans.push(Span::styled(*label, Style::default().fg(ui::DIM)));
        }
        if i == 0 {
            spans.push(Span::styled("    ", Style::default()));
        }
    }
    spans.push(Span::styled(
        "    (Tab to switch)",
        Style::default().fg(ui::DIM),
    ));
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

// ── Roster ────────────────────────────────────────────────────────────────

fn draw_roster(
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
    // Right: selected adventurer detail
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

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));

    // Level and XP
    lines.push(Line::from(vec![
        Span::styled("  Level ", Style::default().fg(ui::DIM)),
        Span::styled(
            format!("{}", adv.level),
            Style::default().fg(ui::WARM_WHITE).add_modifier(Modifier::BOLD),
        ),
        Span::styled("    XP ", Style::default().fg(ui::DIM)),
        Span::styled(
            format!("{}", adv.xp),
            Style::default().fg(ui::WARM_WHITE),
        ),
    ]));
    lines.push(Line::from(""));

    // Stats (base + effective)
    lines.push(Line::from(Span::styled(
        "  Stats",
        Style::default().fg(ui::DIM).add_modifier(Modifier::BOLD),
    )));
    let stat_rows = [
        ("  HP ", stats.max_hp, adv.base_stats.max_hp),
        ("  STR", stats.strength, adv.base_stats.strength),
        ("  DEX", stats.dexterity, adv.base_stats.dexterity),
        ("  INT", stats.intellect, adv.base_stats.intellect),
    ];
    for (label, effective, base) in stat_rows {
        let bonus = effective - base;
        let mut spans = vec![
            Span::styled(format!("  {} ", label), Style::default().fg(ui::DIM)),
            Span::styled(
                format!("{}", effective),
                Style::default().fg(ui::WARM_WHITE).add_modifier(Modifier::BOLD),
            ),
        ];
        if bonus != 0 {
            spans.push(Span::styled(
                format!("  ({} {:+})", base, bonus),
                Style::default().fg(ui::FOREST_GREEN),
            ));
        }
        lines.push(Line::from(spans));
    }
    lines.push(Line::from(""));

    // Equipment
    use super::state::RosterFocus;
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

    // Hint bar
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

    frame.render_widget(Paragraph::new(lines), inner);
}

// ── Quest Board ───────────────────────────────────────────────────────────

fn draw_quest_board(frame: &mut Frame, state: &TavernState, data: &GameData, area: Rect) {
    if data.quests.is_empty() {
        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No quests available yet.",
                Style::default().fg(ui::DIM),
            )),
        ];
        frame.render_widget(Paragraph::new(lines), area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(10), Constraint::Length(1)])
        .split(area);

    // Cards in a single column for now (only 1-3 quests in MVP)
    let n = data.quests.len() as u32;
    let constraints: Vec<Constraint> = (0..n).map(|_| Constraint::Ratio(1, n)).collect();
    let card_areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(chunks[0]);

    for (i, quest) in data.quests.iter().enumerate() {
        if let Some(slot) = card_areas.get(i) {
            let padded = inset_rect(*slot, 2, 1);
            draw_quest_card(frame, quest, padded, i == state.adventure_view.selected_quest);
        }
    }

    let hint = Line::from(vec![
        Span::raw("  "),
        Span::styled(
            "↑↓",
            Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" select  ", Style::default().fg(ui::DIM)),
        Span::styled(
            "Enter",
            Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" set up party", Style::default().fg(ui::DIM)),
    ]);
    frame.render_widget(Paragraph::new(hint), chunks[1]);
}

fn draw_quest_card(frame: &mut Frame, quest: &Quest, area: Rect, selected: bool) {
    let border_color = if selected { ui::GOLD } else { ui::BORDER };
    let title_marker = if selected { " ▸ " } else { "   " };
    let title_style = if selected {
        Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(ui::WARM_WHITE)
    };

    let stars = "★".repeat(quest.difficulty as usize);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(Line::from(vec![
            Span::styled(format!("{}{}", title_marker, quest.name), title_style),
            Span::styled(format!("  {}", stars), Style::default().fg(ui::FLAME)),
        ]))
        .style(Style::default().bg(ui::SHADOW_BG));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));
    let desc_lines = wrap_text(&quest.description, inner.width.saturating_sub(4) as usize);
    for dl in desc_lines {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(dl, Style::default().fg(ui::WARM_WHITE)),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  Party: ", Style::default().fg(ui::DIM)),
        Span::styled(
            format!("{}-{}", quest.min_party, quest.max_party),
            Style::default().fg(ui::WARM_WHITE),
        ),
        Span::styled("    Lv: ", Style::default().fg(ui::DIM)),
        Span::styled(
            format!("{}", quest.recommended_level),
            Style::default().fg(ui::WARM_WHITE),
        ),
        Span::styled("    Time: ", Style::default().fg(ui::DIM)),
        Span::styled(
            format!("~{}m", quest.estimated_minutes),
            Style::default().fg(ui::WARM_WHITE),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  Reward: ", Style::default().fg(ui::DIM)),
        Span::styled(
            format!("{}g", quest.completion_gold),
            Style::default().fg(ui::GOLD),
        ),
        Span::styled("    XP: ", Style::default().fg(ui::DIM)),
        Span::styled(
            format!("{}", quest.xp_reward),
            Style::default().fg(ui::FLAME),
        ),
    ]));

    frame.render_widget(Paragraph::new(lines), inner);
}

// ── Party Setup ───────────────────────────────────────────────────────────

fn draw_party_setup(
    frame: &mut Frame,
    state: &TavernState,
    data: &GameData,
    game_state: &GameState,
    area: Rect,
) {
    let Some(quest) = data.quests.get(state.adventure_view.selected_quest) else {
        return;
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),  // quest summary
            Constraint::Min(10),    // party slots + equipment
            Constraint::Length(1),  // hint
        ])
        .split(area);

    // Quest summary
    let mut summary: Vec<Line> = Vec::new();
    summary.push(Line::from(""));
    summary.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(
            quest.name.clone(),
            Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("    Party: {}-{}    ", quest.min_party, quest.max_party),
            Style::default().fg(ui::DIM),
        ),
        Span::styled(
            format!("Reward: {}g + {} XP", quest.completion_gold, quest.xp_reward),
            Style::default().fg(ui::DIM),
        ),
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
                    let stats = adv.effective_stats(&data_registry_passthrough());
                    // We don't have data here — show base stats since we can't get registry
                    let _ = stats;
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

// We can't compute effective stats here without a registry. Use base stats only.
// This helper exists so the function compiles; we ignore its result.
fn data_registry_passthrough() -> game::ItemRegistry {
    game::ItemRegistry::new()
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

    let slots = [
        (0, "Weapon", &adv.equipment.weapon),
        (1, "Armor", &adv.equipment.armor),
        (2, "Accessory", &adv.equipment.accessory),
    ];
    for (i, label, equipped) in slots {
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

// ── In-Adventure (map) ────────────────────────────────────────────────────

const MAP_TILE_W: u16 = 8;  // cells wide per tile
const MAP_TILE_H: u16 = 4;  // cells tall per tile (square in display)

fn draw_in_adventure(
    frame: &mut Frame,
    state: &mut TavernState,
    data: &GameData,
    game_state: &GameState,
    area: Rect,
) {
    let Some(adventure) = &game_state.active_adventure else {
        return;
    };
    let Some(quest) = data.quest(&adventure.quest_id) else {
        return;
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(17),    // map area (large enough for 5x5 grid of tiles)
            Constraint::Length(6),  // info + log
            Constraint::Length(1),  // hint
        ])
        .split(area);

    draw_map(frame, state, adventure, quest, chunks[0]);
    draw_adventure_info(frame, adventure, quest, chunks[1]);

    let hint = Line::from(vec![
        Span::raw("  "),
        Span::styled(
            "↑↓←→",
            Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" move  ", Style::default().fg(ui::DIM)),
        Span::styled(
            "Esc",
            Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" abandon quest", Style::default().fg(ui::DIM)),
    ]);
    frame.render_widget(Paragraph::new(hint), chunks[2]);
}

fn draw_map(
    frame: &mut Frame,
    state: &mut TavernState,
    adventure: &game::ActiveAdventure,
    quest: &Quest,
    area: Rect,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ui::FOREST_GREEN))
        .title(Span::styled(
            format!(" {} ", quest.name),
            Style::default()
                .fg(ui::FOREST_GREEN)
                .add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(ui::SHADOW_BG));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let map = &quest.map;
    let tile_w = MAP_TILE_W;
    let tile_h = MAP_TILE_H;

    let total_w = map.width as u16 * tile_w;
    let total_h = map.height as u16 * tile_h;

    if total_w > inner.width || total_h > inner.height {
        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "  Map too large for terminal.",
                Style::default().fg(ui::DIM),
            )),
        ];
        frame.render_widget(Paragraph::new(lines), inner);
        return;
    }

    let start_x = inner.x + (inner.width - total_w) / 2;
    let start_y = inner.y + (inner.height - total_h) / 2;

    for y in 0..map.height {
        for x in 0..map.width {
            let tx = start_x + x as u16 * tile_w;
            let ty = start_y + y as u16 * tile_h;
            let tile_area = Rect {
                x: tx,
                y: ty,
                width: tile_w,
                height: tile_h,
            };
            draw_map_tile(frame, state, adventure, map, x, y, tile_area);
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
            None => ui::MapTile::Fog,
        }
    };

    // Render based on the tile graphics backend
    use super::tile_graphics::{tile_image_id, TileBackend};
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

fn draw_adventure_info(
    frame: &mut Frame,
    adventure: &game::ActiveAdventure,
    quest: &Quest,
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
    let square_desc = match quest.map.get(cx, cy) {
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

// ── Combat ────────────────────────────────────────────────────────────────

fn draw_combat(
    frame: &mut Frame,
    state: &TavernState,
    data: &GameData,
    game_state: &GameState,
    area: Rect,
) {
    let Some(adventure) = &game_state.active_adventure else {
        return;
    };
    let AdventureState::InCombat(combat) = &adventure.state else {
        return;
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7),  // enemies
            Constraint::Length(6),  // action menu
            Constraint::Min(4),     // log
            Constraint::Length(1),  // hint
        ])
        .split(area);

    draw_combat_enemies(frame, combat, state, chunks[0]);
    draw_combat_actions(frame, state, adventure, combat, chunks[1]);
    draw_combat_log(frame, combat, chunks[2]);

    let hint = Line::from(vec![
        Span::raw("  "),
        Span::styled(
            "↑↓",
            Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" select  ", Style::default().fg(ui::DIM)),
        Span::styled(
            "Enter",
            Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" confirm", Style::default().fg(ui::DIM)),
    ]);
    let _ = data;
    frame.render_widget(Paragraph::new(hint), chunks[3]);
}

fn draw_combat_enemies(
    frame: &mut Frame,
    combat: &game::CombatState,
    state: &TavernState,
    area: Rect,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ui::EMBER))
        .title(Span::styled(
            " Enemies ",
            Style::default().fg(ui::EMBER).add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(ui::SHADOW_BG));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));
    for (i, enemy) in combat.enemies.iter().enumerate() {
        let is_target = state.adventure_view.combat_picking_target
            && i == state.adventure_view.combat_target_idx;
        let alive = enemy.current_hp > 0;
        let marker = if is_target {
            " ▸ "
        } else if alive {
            "   "
        } else {
            " ✗ "
        };

        let name_style = if !alive {
            Style::default().fg(ui::DIM)
        } else if is_target {
            Style::default().fg(ui::FLAME).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(ui::WARM_WHITE).add_modifier(Modifier::BOLD)
        };

        let bar_width: usize = 14;
        let hp_ratio = if enemy.max_hp > 0 {
            enemy.current_hp.max(0) as f64 / enemy.max_hp as f64
        } else {
            0.0
        };
        let filled = (hp_ratio * bar_width as f64) as usize;
        let bar: String = "█".repeat(filled) + &"░".repeat(bar_width.saturating_sub(filled));
        let bar_color = if !alive {
            ui::DIM
        } else if hp_ratio < 0.34 {
            ui::EMBER
        } else if hp_ratio < 0.67 {
            ui::FLAME
        } else {
            ui::FOREST_GREEN
        };

        lines.push(Line::from(vec![
            Span::styled(marker, Style::default().fg(ui::FLAME)),
            Span::styled(format!("{:<14}", enemy.name), name_style),
            Span::styled(bar, Style::default().fg(bar_color)),
            Span::styled(
                format!("  {}/{}", enemy.current_hp.max(0), enemy.max_hp),
                Style::default().fg(ui::DIM),
            ),
        ]));
    }
    frame.render_widget(Paragraph::new(lines), inner);
}

fn draw_combat_actions(
    frame: &mut Frame,
    state: &TavernState,
    adventure: &game::ActiveAdventure,
    combat: &game::CombatState,
    area: Rect,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ui::GOLD))
        .title(Span::styled(
            " Action ",
            Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(ui::SHADOW_BG));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Show whose turn it is
    let actor_label = match combat.current_actor() {
        Some(game::CombatActor::Party(i)) => {
            adventure
                .party
                .get(i)
                .map(|m| format!("{}'s turn", m.name))
                .unwrap_or_else(|| "Party turn".into())
        }
        Some(game::CombatActor::Enemy(_)) => "Enemy turn...".into(),
        None => "...".into(),
    };

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(
            actor_label,
            Style::default().fg(ui::WARM_WHITE).add_modifier(Modifier::BOLD),
        ),
    ]));
    lines.push(Line::from(""));

    let actions = ["Attack", "Defend", "Flee"];
    let is_party_turn = matches!(combat.current_actor(), Some(game::CombatActor::Party(_)))
        && !state.adventure_view.combat_picking_target;
    for (i, label) in actions.iter().enumerate() {
        let is_selected = is_party_turn && i == state.adventure_view.combat_action_idx;
        let marker = if is_selected { " ▸ " } else { "   " };
        let style = if is_selected {
            Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD)
        } else if is_party_turn {
            Style::default().fg(ui::WARM_WHITE)
        } else {
            Style::default().fg(ui::DIM)
        };
        lines.push(Line::from(vec![
            Span::styled(marker, Style::default().fg(ui::FLAME)),
            Span::styled(*label, style),
        ]));
    }

    if state.adventure_view.combat_picking_target {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  ↑↓ pick target  Enter confirm",
            Style::default().fg(ui::FLAME),
        )));
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

fn draw_combat_log(frame: &mut Frame, combat: &game::CombatState, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ui::BORDER))
        .title(Span::styled(
            " Combat Log ",
            Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(ui::SHADOW_BG));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let visible = inner.height as usize;
    let recent: Vec<&String> = combat.log.iter().rev().take(visible).collect();
    let lines: Vec<Line> = recent
        .into_iter()
        .rev()
        .map(|s| {
            Line::from(vec![
                Span::raw(" "),
                Span::styled(s.clone(), Style::default().fg(ui::WARM_WHITE)),
            ])
        })
        .collect();
    frame.render_widget(Paragraph::new(lines), inner);
}

// ── Results ───────────────────────────────────────────────────────────────

fn draw_results(
    frame: &mut Frame,
    _state: &TavernState,
    data: &GameData,
    game_state: &GameState,
    area: Rect,
) {
    let Some(adventure) = &game_state.active_adventure else {
        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No active adventure.",
                Style::default().fg(ui::DIM),
            )),
        ];
        frame.render_widget(Paragraph::new(lines), area);
        return;
    };

    let success = matches!(adventure.state, AdventureState::Complete { success: true });
    let title_color = if success { ui::FOREST_GREEN } else { ui::EMBER };
    let title_text = if success { " Quest Complete " } else { " Quest Failed " };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(title_color))
        .title(Span::styled(
            title_text,
            Style::default().fg(title_color).add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(ui::SHADOW_BG));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));

    if success {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("You return triumphant!", Style::default().fg(ui::GOLD)),
        ]));
    } else {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("The party retreats wounded.", Style::default().fg(ui::EMBER)),
        ]));
    }
    lines.push(Line::from(""));

    if adventure.pending_gold > 0 {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                format!("+{} gold", adventure.pending_gold),
                Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
            ),
        ]));
    }
    if adventure.pending_xp > 0 {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                format!("+{} XP per adventurer", adventure.pending_xp),
                Style::default().fg(ui::FLAME).add_modifier(Modifier::BOLD),
            ),
        ]));
    }
    if !adventure.pending_loot.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Loot:",
            Style::default().fg(ui::DIM).add_modifier(Modifier::BOLD),
        )));
        for (id, qty) in &adventure.pending_loot {
            let name = data
                .item_registry
                .get(id)
                .map(|d| d.name.clone())
                .unwrap_or_else(|| id.0.clone());
            lines.push(Line::from(vec![
                Span::styled("    + ", Style::default().fg(ui::DIM)),
                Span::styled(
                    format!("{}× {}", qty, name),
                    Style::default().fg(ui::WARM_WHITE),
                ),
            ]));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Press Enter to return to the Quest Board",
        Style::default().fg(ui::DIM),
    )));

    frame.render_widget(Paragraph::new(lines), inner);
}
