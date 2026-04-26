//! Adventures → Quest Board sub-screen drawing.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::prelude::Frame;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

use super::super::state::TavernState;
use super::super::util::{inset_rect, wrap_text};
use crate::game::{self, GameData, Quest};
use crate::ui;

pub(super) fn draw_quest_board(
    frame: &mut Frame,
    state: &TavernState,
    data: &GameData,
    area: Rect,
) {
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
