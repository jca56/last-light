//! Drawing for the Adventures view: Quest Board, Party Setup, In-Adventure
//! map, Combat, and Results sub-screens.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::prelude::Frame;
use ratatui::style::{Color, Modifier, Style};
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
    state: &mut TavernState,
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
    state: &mut TavernState,
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

    // Split: portrait (left) + info (right)
    let portrait_w: u16 = 12;
    let detail_split = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(portrait_w.min(inner.width / 3)),
            Constraint::Min(20),
        ])
        .split(inner);

    // Portrait
    let p_area = detail_split[0];
    use super::tile_graphics::TileBackend;
    if state.tile_graphics.backend == TileBackend::KittyInline {
        state.pending_adv_portraits.push((
            p_area.x,
            p_area.y,
            adv.id.clone(),
            10 + idx, // use slot 10+ for roster to avoid colliding with combat slots
            p_area.width,
            p_area.height.min(6),
        ));
        let buf = frame.buffer_mut();
        for dy in 0..p_area.height.min(6) {
            for dx in 0..p_area.width {
                if let Some(cell) = buf.cell_mut((p_area.x + dx, p_area.y + dy)) {
                    cell.set_symbol(" ");
                    cell.set_skip(true);
                }
            }
        }
    } else if let Some(bytes) = ui::adventurer_portrait_bytes(&adv.id) {
        let portrait_area = Rect {
            x: p_area.x,
            y: p_area.y,
            width: p_area.width,
            height: p_area.height.min(6),
        };
        let lines_p = crate::halfblock::png_to_halfblock(bytes, portrait_area.width);
        frame.render_widget(Paragraph::new(lines_p), portrait_area);
    }

    let info_area = detail_split[1];
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

// ── Quest Board ───────────────────────────────────────────────────────────

fn draw_quest_board(frame: &mut Frame, state: &TavernState, data: &GameData, area: Rect) {
    let total = data.quests.len() + data.dungeons.len();
    if total == 0 {
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

    // Combined list: story quests first, then dungeons
    let n = total as u32;
    let constraints: Vec<Constraint> = (0..n).map(|_| Constraint::Ratio(1, n)).collect();
    let card_areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(chunks[0]);

    let mut card_idx = 0;
    // Story quests
    for quest in &data.quests {
        if let Some(slot) = card_areas.get(card_idx) {
            let padded = inset_rect(*slot, 2, 0);
            draw_quest_card(frame, quest, padded, card_idx == state.adventure_view.selected_quest);
        }
        card_idx += 1;
    }
    // Dungeons
    for dungeon in &data.dungeons {
        if let Some(slot) = card_areas.get(card_idx) {
            let padded = inset_rect(*slot, 2, 0);
            draw_dungeon_card(
                frame,
                dungeon,
                padded,
                card_idx == state.adventure_view.selected_quest,
            );
        }
        card_idx += 1;
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

fn draw_dungeon_card(
    frame: &mut Frame,
    dungeon: &game::DungeonDef,
    area: Rect,
    selected: bool,
) {
    let border_color = if selected { ui::GOLD } else { ui::BORDER };
    let title_marker = if selected { " ▸ " } else { "   " };
    let title_style = if selected {
        Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(ui::WARM_WHITE)
    };

    let tier_label = format!("T{}", dungeon.tier);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(Line::from(vec![
            Span::styled(
                format!("{}{}", title_marker, dungeon.name),
                title_style,
            ),
            Span::styled(
                format!("  [{}]", tier_label),
                Style::default().fg(ui::FLAME),
            ),
            Span::styled(
                "  ⟳ Randomized",
                Style::default().fg(ui::FOREST_GREEN),
            ),
        ]))
        .style(Style::default().bg(ui::SHADOW_BG));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();
    let desc_lines = wrap_text(&dungeon.description, inner.width.saturating_sub(4) as usize);
    for dl in desc_lines {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(dl, Style::default().fg(ui::WARM_WHITE)),
        ]));
    }
    lines.push(Line::from(vec![
        Span::styled("  Party: ", Style::default().fg(ui::DIM)),
        Span::styled(
            format!("{}-{}", dungeon.min_party, dungeon.max_party),
            Style::default().fg(ui::WARM_WHITE),
        ),
        Span::styled("    Lv: ", Style::default().fg(ui::DIM)),
        Span::styled(
            format!("{}", dungeon.recommended_level),
            Style::default().fg(ui::WARM_WHITE),
        ),
        Span::styled("    Floors: ", Style::default().fg(ui::DIM)),
        Span::styled(
            format!("{}-{}", dungeon.min_floors, dungeon.max_floors),
            Style::default().fg(ui::WARM_WHITE),
        ),
    ]));

    frame.render_widget(Paragraph::new(lines), inner);
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
            Constraint::Min(17), // map area
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

// ── Combat ────────────────────────────────────────────────────────────────

fn draw_combat(
    frame: &mut Frame,
    state: &mut TavernState,
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

    // Layout: enemy portrait+list, party portraits, actions, log
    let action_h: u16 = if state.adventure_view.combat_picking_consumable { 14 } else { 10 };
    let party_row_h: u16 = 5; // small portrait row for party
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(8),              // portrait + enemies
            Constraint::Length(party_row_h), // party portraits
            Constraint::Length(action_h),    // action menu
            Constraint::Length(6),           // log
            Constraint::Length(1),           // hint
        ])
        .split(area);

    // Top row: enemy portrait on left, enemy list on right
    let portrait_w: u16 = 20;
    let top_split = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(portrait_w), Constraint::Min(20)])
        .split(chunks[0]);

    draw_combat_portrait(frame, state, combat, top_split[0]);
    draw_combat_enemies(frame, combat, state, top_split[1]);

    // Party portraits row
    draw_combat_party_row(frame, state, adventure, combat, game_state, chunks[1]);

    // Action menu with damage previews
    draw_combat_actions(frame, state, data, adventure, combat, chunks[2]);

    // Combat log
    draw_combat_log(frame, combat, chunks[3]);

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
    frame.render_widget(Paragraph::new(hint), chunks[4]);
}

fn draw_combat_portrait(
    frame: &mut Frame,
    state: &mut TavernState,
    combat: &game::CombatState,
    area: Rect,
) {
    // Show portrait of the targeted enemy (if picking target) or the first alive enemy
    let enemy_name = if state.adventure_view.combat_picking_target {
        combat
            .enemies
            .get(state.adventure_view.combat_target_idx)
            .map(|e| e.name.clone())
            .unwrap_or_default()
    } else {
        combat
            .enemies
            .iter()
            .find(|e| e.current_hp > 0)
            .map(|e| e.name.clone())
            .unwrap_or_default()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ui::EMBER))
        .style(Style::default().bg(ui::SHADOW_BG));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    use super::tile_graphics::TileBackend;
    if state.tile_graphics.backend == TileBackend::KittyInline && !enemy_name.is_empty() {
        // Queue the enemy portrait for Kitty rendering after draw
        state.pending_enemy_portrait = Some((
            inner.x,
            inner.y,
            enemy_name,
            inner.width,
            inner.height,
        ));
        // Mark cells as skip
        let buf = frame.buffer_mut();
        for dy in 0..inner.height {
            for dx in 0..inner.width {
                if let Some(cell) = buf.cell_mut((inner.x + dx, inner.y + dy)) {
                    cell.set_symbol(" ");
                    cell.set_skip(true);
                }
            }
        }
    } else {
        // Halfblock fallback
        if let Some(bytes) = ui::enemy_portrait_bytes(&enemy_name) {
            let lines = crate::halfblock::png_to_halfblock(bytes, inner.width);
            frame.render_widget(Paragraph::new(lines), inner);
        }
    }
}

fn draw_combat_party_row(
    frame: &mut Frame,
    state: &mut TavernState,
    adventure: &game::ActiveAdventure,
    combat: &game::CombatState,
    game_state: &GameState,
    area: Rect,
) {
    // Split horizontally: one column per party member
    let n = adventure.party.len();
    if n == 0 {
        return;
    }
    let constraints: Vec<Constraint> = (0..n)
        .map(|_| Constraint::Ratio(1, n as u32))
        .collect();
    let slots = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(area);

    let is_party_turn = matches!(combat.current_actor(), Some(game::CombatActor::Party(_)));
    let active_idx = match combat.current_actor() {
        Some(game::CombatActor::Party(i)) => Some(i),
        _ => None,
    };

    for (i, member) in adventure.party.iter().enumerate() {
        let Some(adv) = game_state.adventurers.get(member.roster_idx) else {
            continue;
        };
        let is_active = is_party_turn && active_idx == Some(i);
        let _border_color = if member.downed {
            ui::DIM
        } else if is_active {
            ui::GOLD
        } else {
            ui::BORDER
        };

        let slot_area = slots[i];

        // Split each slot: portrait (left 8 cols) + info (right)
        let portrait_w: u16 = 8;
        let inner_split = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(portrait_w.min(slot_area.width / 2)),
                Constraint::Min(8),
            ])
            .split(slot_area);

        // Portrait area (Kitty or halfblock)
        let p_area = inner_split[0];
        use super::tile_graphics::TileBackend;
        if state.tile_graphics.backend == TileBackend::KittyInline {
            state.pending_adv_portraits.push((
                p_area.x,
                p_area.y,
                adv.id.clone(),
                i,
                p_area.width,
                p_area.height,
            ));
            let buf = frame.buffer_mut();
            for dy in 0..p_area.height {
                for dx in 0..p_area.width {
                    if let Some(cell) = buf.cell_mut((p_area.x + dx, p_area.y + dy)) {
                        cell.set_symbol(" ");
                        cell.set_skip(true);
                    }
                }
            }
        } else if let Some(bytes) = ui::adventurer_portrait_bytes(&adv.id) {
            let lines = crate::halfblock::png_to_halfblock(bytes, p_area.width);
            frame.render_widget(Paragraph::new(lines), p_area);
        }

        // Info area: name + HP bar
        let info = inner_split[1];
        let hp_ratio = if member.max_hp > 0 {
            member.current_hp.max(0) as f64 / member.max_hp as f64
        } else {
            0.0
        };
        let bar_w = info.width.saturating_sub(2) as usize;
        let filled = (hp_ratio * bar_w as f64) as usize;
        let bar: String = "█".repeat(filled) + &"░".repeat(bar_w.saturating_sub(filled));
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
        } else if is_active {
            Style::default()
                .fg(ui::GOLD)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(ui::WARM_WHITE)
        };

        let active_marker = if is_active { "⚔ " } else { "  " };
        let mut lines: Vec<Line> = Vec::new();
        lines.push(Line::from(vec![
            Span::styled(
                active_marker,
                Style::default().fg(ui::FLAME),
            ),
            Span::styled(adv.name.clone(), name_style),
        ]));
        lines.push(Line::from(vec![
            Span::raw(" "),
            Span::styled(bar, Style::default().fg(bar_color)),
        ]));
        lines.push(Line::from(vec![
            Span::raw(" "),
            Span::styled(
                format!("{}/{}", member.current_hp.max(0), member.max_hp),
                Style::default().fg(ui::DIM),
            ),
        ]));

        frame.render_widget(Paragraph::new(lines), info);
    }
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
    data: &GameData,
    adventure: &game::ActiveAdventure,
    combat: &game::CombatState,
    area: Rect,
) {
    // Show whose turn it is — prominently in the title bar
    let (actor_label, title_color) = match combat.current_actor() {
        Some(game::CombatActor::Party(i)) => {
            let name = adventure
                .party
                .get(i)
                .map(|m| m.name.clone())
                .unwrap_or_else(|| "???".into());
            (format!(" ⚔ {}'s Turn ", name), ui::GOLD)
        }
        Some(game::CombatActor::Enemy(_)) => (" Enemy Turn... ".into(), ui::EMBER),
        None => (" ... ".into(), ui::DIM),
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(title_color))
        .title(Span::styled(
            actor_label,
            Style::default()
                .fg(title_color)
                .add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(ui::SHADOW_BG));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));

    // Check if the active member has consumables
    let has_items = match combat.current_actor() {
        Some(game::CombatActor::Party(i)) => {
            adventure.party.get(i).map(|m| m.has_consumables()).unwrap_or(false)
        }
        _ => false,
    };

    // Get current party member's stats for damage preview
    let actor_stats = match combat.current_actor() {
        Some(game::CombatActor::Party(i)) => adventure.party.get(i),
        _ => None,
    };
    let atk_dmg = actor_stats
        .map(|m| m.strength.max(m.dexterity).max(1))
        .unwrap_or(0);
    let avg_dex = {
        let alive: Vec<&game::PartyMember> =
            adventure.party.iter().filter(|p| !p.downed).collect();
        if alive.is_empty() {
            0
        } else {
            alive.iter().map(|p| p.dexterity).sum::<i32>() / alive.len() as i32
        }
    };

    let action_info: Vec<(&str, String)> = {
        let mut v = vec![
            ("Attack", format!("~{} damage", atk_dmg)),
            ("Defend", "halves incoming damage".into()),
            ("Flee", format!("DEX {} vs DC 10", avg_dex)),
        ];
        if has_items {
            v.push(("Use Item", "use a consumable".into()));
        }
        v
    };

    let is_party_turn = matches!(combat.current_actor(), Some(game::CombatActor::Party(_)))
        && !state.adventure_view.combat_picking_target
        && !state.adventure_view.combat_picking_consumable;
    for (i, (label, preview)) in action_info.iter().enumerate() {
        let is_selected = is_party_turn && i == state.adventure_view.combat_action_idx;
        let marker = if is_selected { " ▸ " } else { "   " };
        let style = if is_selected {
            Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD)
        } else if is_party_turn {
            Style::default().fg(ui::WARM_WHITE)
        } else {
            Style::default().fg(ui::DIM)
        };
        let preview_style = if is_selected {
            Style::default().fg(ui::DIM)
        } else {
            Style::default().fg(Color::Rgb(60, 55, 50))
        };
        lines.push(Line::from(vec![
            Span::styled(marker, Style::default().fg(ui::FLAME)),
            Span::styled(format!("{:<12}", label), style),
            Span::styled(format!("({})", preview), preview_style),
        ]));
    }

    if state.adventure_view.combat_picking_target {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  ↑↓ pick target  Enter confirm",
            Style::default().fg(ui::FLAME),
        )));
    }

    if state.adventure_view.combat_picking_consumable {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Items:",
            Style::default().fg(ui::DIM).add_modifier(Modifier::BOLD),
        )));
        if let Some(game::CombatActor::Party(pi)) = combat.current_actor() {
            if let Some(member) = adventure.party.get(pi) {
                for (ci, slot) in member.consumables.iter().enumerate() {
                    let is_sel = ci == state.adventure_view.combat_consumable_idx;
                    let marker = if is_sel { " ▸ " } else { "   " };
                    match slot {
                        Some(id) => {
                            let name = data
                                .item_registry
                                .get(id)
                                .map(|d| d.name.clone())
                                .unwrap_or_else(|| id.0.clone());
                            let style = if is_sel {
                                Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD)
                            } else {
                                Style::default().fg(ui::WARM_WHITE)
                            };
                            lines.push(Line::from(vec![
                                Span::styled(marker, Style::default().fg(ui::FLAME)),
                                Span::styled(name, style),
                            ]));
                        }
                        None => {
                            lines.push(Line::from(vec![
                                Span::styled(marker, Style::default().fg(ui::FLAME)),
                                Span::styled("— empty —", Style::default().fg(ui::DIM)),
                            ]));
                        }
                    }
                }
            }
        }
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
        // Preview level-ups
        for member in &adventure.party {
            if let Some(adv) = game_state.adventurers.get(member.roster_idx) {
                let future_xp = adv.xp.saturating_add(adventure.pending_xp);
                let future_level = game::level_from_xp(future_xp);
                if future_level > adv.level {
                    lines.push(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(
                            format!("  {} levels up to {}!", adv.name, future_level),
                            Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
                        ),
                    ]));
                }
            }
        }
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
