//! Drawing for the right column: content panel (with custom titles per view),
//! terminal log, placeholder views, and the input bar.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::prelude::Frame;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

use super::adventure_view::draw_adventure;
use super::crafting_view::draw_crafting;
use super::gathering_view::{draw_gathering, effective_gathering_screen};
use super::inventory_view::{draw_inventory, inventory_stats};
use super::refining_view::draw_refining;
use super::state::{
    AdventureScreen, Focus, GatheringScreen, RefiningScreen, TavernState, View, NAV_ITEMS,
};
use crate::game::{GameData, GameState};
use crate::ui;

pub(super) fn draw_right(
    frame: &mut Frame,
    state: &mut TavernState,
    area: Rect,
    data: &GameData,
    game_state: &GameState,
) {
    // Tab strip (1 row), content panel, optional input bar
    let on_terminal = state.current_view == View::Terminal;
    let constraints: Vec<Constraint> = if on_terminal {
        vec![
            Constraint::Length(1), // tab strip
            Constraint::Min(5),    // content
            Constraint::Length(3), // input bar
        ]
    } else {
        vec![
            Constraint::Length(1), // tab strip
            Constraint::Min(5),    // content
        ]
    };
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    // ── Tab strip ──────────────────────────────────────
    draw_tab_strip(frame, state, right_chunks[0]);

    let view_name = match state.current_view {
        View::Terminal => "Terminal",
        View::Tavern => "Tavern",
        View::Inventory => "Inventory",
        View::Gathering => "Gathering",
        View::Refining => "Refining",
        View::Crafting => "Crafting",
        View::Adventuring => "Adventuring",
        View::Shop => "Shop",
    };

    // The "content" area is always treated as focused for highlight purposes
    // (since there's no Menu focus state anymore)
    let term_focused = !matches!(state.focus, Focus::Input);

    // Custom title and border for the Gathering / Inventory view sub-states
    let (title_spans, border_color): (Vec<Span>, Color) = if state.current_view == View::Gathering
    {
        let screen = effective_gathering_screen(state);
        match screen {
            GatheringScreen::Grounds => (
                vec![
                    Span::styled(
                        " Gathering Grounds ",
                        Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        "· Choose where to gather today ",
                        Style::default().fg(ui::DIM),
                    ),
                ],
                if term_focused { ui::GOLD } else { ui::BORDER },
            ),
            GatheringScreen::AtLocation => {
                let loc = data
                    .gather_locations
                    .get(state.gathering_view.current_location);
                let name = loc.map(|l| l.name.clone()).unwrap_or_default();
                (
                    vec![Span::styled(
                        format!(" {} ", name),
                        Style::default()
                            .fg(ui::FOREST_GREEN)
                            .add_modifier(Modifier::BOLD),
                    )],
                    ui::FOREST_GREEN,
                )
            }
        }
    } else if state.current_view == View::Refining {
        match state.refining_view.screen {
            RefiningScreen::Stations => (
                vec![
                    Span::styled(
                        " Refining Stations ",
                        Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        "· Pick a workshop ",
                        Style::default().fg(ui::DIM),
                    ),
                ],
                if term_focused { ui::GOLD } else { ui::BORDER },
            ),
            RefiningScreen::AtStation => {
                let station = data
                    .refining_stations
                    .get(state.refining_view.current_station);
                let name = station.map(|s| s.name.clone()).unwrap_or_default();
                (
                    vec![Span::styled(
                        format!(" {} ", name),
                        Style::default()
                            .fg(ui::GOLD)
                            .add_modifier(Modifier::BOLD),
                    )],
                    if term_focused { ui::GOLD } else { ui::BORDER },
                )
            }
        }
    } else if state.current_view == View::Adventuring {
        let title = match state.adventure_view.screen {
            AdventureScreen::Roster => " Adventurers ",
            AdventureScreen::QuestBoard => " Quest Board ",
            AdventureScreen::PartySetup => " Party Setup ",
            AdventureScreen::InAdventure => " On the Path ",
            AdventureScreen::Combat => " Combat! ",
            AdventureScreen::Results => " Results ",
        };
        let color = match state.adventure_view.screen {
            AdventureScreen::Combat => ui::EMBER,
            AdventureScreen::InAdventure => ui::FOREST_GREEN,
            _ => ui::GOLD,
        };
        (
            vec![Span::styled(
                title.to_string(),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            )],
            color,
        )
    } else if state.current_view == View::Crafting {
        (
            vec![
                Span::styled(
                    " Crafting Bench ",
                    Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
                ),
                Span::styled("· Make something ", Style::default().fg(ui::DIM)),
            ],
            if term_focused { ui::GOLD } else { ui::BORDER },
        )
    } else if state.current_view == View::Inventory {
        let stats = inventory_stats(game_state, data);
        (
            vec![
                Span::styled(
                    " Inventory ",
                    Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(
                        "· {} items · {} total · {}g ",
                        stats.unique, stats.total, stats.value
                    ),
                    Style::default().fg(ui::DIM),
                ),
            ],
            if term_focused { ui::GOLD } else { ui::BORDER },
        )
    } else {
        (
            vec![Span::styled(
                format!(" {} ", view_name),
                Style::default().fg(ui::WARM_WHITE).add_modifier(Modifier::BOLD),
            )],
            if term_focused { ui::GOLD } else { ui::BORDER },
        )
    };

    let content_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(Line::from(title_spans))
        .style(Style::default().bg(ui::SHADOW_BG));

    let content_inner = content_block.inner(right_chunks[1]);
    frame.render_widget(content_block, right_chunks[1]);

    match state.current_view {
        View::Terminal => draw_terminal_log(frame, state, content_inner),
        View::Gathering => draw_gathering(frame, state, data, game_state, content_inner),
        View::Refining => draw_refining(frame, state, data, game_state, content_inner),
        View::Crafting => draw_crafting(frame, state, data, game_state, content_inner),
        View::Adventuring => draw_adventure(frame, state, data, game_state, content_inner),
        View::Inventory => draw_inventory(frame, state, data, game_state, content_inner),
        _ => draw_placeholder(frame, view_name, content_inner),
    }

    if on_terminal {
        draw_input_bar(frame, state, right_chunks[2]);
    }
}

/// Draws a single-row tab strip showing all 8 views with numbered hotkeys.
fn draw_tab_strip(frame: &mut Frame, state: &TavernState, area: Rect) {
    let mut spans: Vec<Span<'static>> = Vec::new();
    spans.push(Span::raw(" "));
    for (i, (view, _)) in NAV_ITEMS.iter().enumerate() {
        let is_active = *view == state.current_view;
        let label = view_short_label(*view);
        let key = (b'1' + i as u8) as char;

        if is_active {
            spans.push(Span::styled(
                format!("[{}]", key),
                Style::default().fg(ui::FLAME).add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::styled(
                format!(" {}", label),
                Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled(
                format!("[{}]", key),
                Style::default().fg(ui::DIM).add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::styled(
                format!(" {}", label),
                Style::default().fg(ui::DIM),
            ));
        }
        if i + 1 < NAV_ITEMS.len() {
            spans.push(Span::styled("  ", Style::default()));
        }
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn view_short_label(view: View) -> &'static str {
    match view {
        View::Terminal => "Term",
        View::Tavern => "Tav",
        View::Inventory => "Inv",
        View::Gathering => "Gath",
        View::Refining => "Ref",
        View::Crafting => "Craft",
        View::Adventuring => "Adv",
        View::Shop => "Shop",
    }
}

fn draw_terminal_log(frame: &mut Frame, state: &TavernState, area: Rect) {
    let visible_height = area.height as usize;
    let total = state.log_messages.len();
    let max_scroll = total.saturating_sub(visible_height);
    let scroll = (state.log_scroll as usize).min(max_scroll);

    let lines: Vec<Line> = state.log_messages[scroll..]
        .iter()
        .take(visible_height)
        .map(|(text, style)| Line::from(Span::styled(format!(" {}", text), *style)))
        .collect();

    frame.render_widget(Paragraph::new(lines), area);
}

fn draw_placeholder(frame: &mut Frame, name: &str, area: Rect) {
    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  {} — Coming soon.", name),
            Style::default().fg(ui::DIM),
        )),
    ];
    frame.render_widget(Paragraph::new(lines), area);
}

fn draw_input_bar(frame: &mut Frame, state: &TavernState, area: Rect) {
    let input_focused = state.focus == Focus::Input;
    let input_border = if input_focused {
        Style::default().fg(ui::GOLD)
    } else {
        Style::default().fg(ui::BORDER)
    };

    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_style(input_border)
        .style(Style::default().bg(ui::SHADOW_BG));

    let inner = input_block.inner(area);
    frame.render_widget(input_block, area);

    if state.input.is_empty() && !input_focused {
        let placeholder = Line::from(Span::styled(
            " Type here...",
            Style::default().fg(Color::Rgb(60, 50, 40)),
        ));
        frame.render_widget(Paragraph::new(placeholder), inner);
    } else {
        let before: String = state.input[..state.cursor].to_string();
        let cursor_char = if state.cursor < state.input.len() {
            state.input[state.cursor..state.cursor + 1].to_string()
        } else {
            " ".to_string()
        };
        let after: String = if state.cursor < state.input.len() {
            state.input[state.cursor + 1..].to_string()
        } else {
            String::new()
        };

        let cursor_style = if input_focused {
            Style::default().fg(ui::SHADOW_BG).bg(ui::GOLD)
        } else {
            Style::default().fg(ui::WARM_WHITE)
        };

        let line = Line::from(vec![
            Span::styled(format!(" {}", before), Style::default().fg(ui::WARM_WHITE)),
            Span::styled(cursor_char, cursor_style),
            Span::styled(after, Style::default().fg(ui::WARM_WHITE)),
        ]);
        frame.render_widget(Paragraph::new(line), inner);
    }
}
