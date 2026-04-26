//! Adventures → Results sub-screen drawing.

use ratatui::layout::Rect;
use ratatui::prelude::Frame;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

use super::super::state::TavernState;
use crate::game::{self, AdventureState, GameData, GameState};
use crate::ui;

pub(super) fn draw_results(
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
