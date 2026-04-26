//! Adventures → Combat sub-screen drawing.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::prelude::Frame;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

use super::super::state::TavernState;
use super::super::tile_graphics::TileBackend;
use crate::game::{self, AdventureState, GameData, GameState};
use crate::ui;

pub(super) fn draw_combat(
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
    draw_combat_party_row(frame, adventure, combat, game_state, chunks[1]);

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

        let info = slots[i];
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
