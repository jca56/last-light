//! Drawing for the Adventures view: Roster, Quest Board, Party Setup,
//! In-Adventure map, Combat, and Results sub-screens.
//!
//! Each sub-screen lives in its own sibling module. `mod.rs` owns the
//! top-level dispatcher and the sub-tab strip shown above the Roster /
//! Quest Board screens.

mod combat;
mod in_adventure;
mod party_setup;
mod quest_board;
mod results;
mod roster;

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::prelude::Frame;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::state::{AdventureScreen, TavernState};
use crate::game::{GameData, GameState};
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
                    roster::draw_roster(frame, state, data, game_state, chunks[1])
                }
                _ => quest_board::draw_quest_board(frame, state, data, chunks[1]),
            }
        }
        // Sub-screens don't show the tab strip (full screen)
        AdventureScreen::PartySetup => {
            party_setup::draw_party_setup(frame, state, data, game_state, area)
        }
        AdventureScreen::InAdventure => {
            in_adventure::draw_in_adventure(frame, state, data, game_state, area)
        }
        AdventureScreen::Combat => combat::draw_combat(frame, state, data, game_state, area),
        AdventureScreen::Results => results::draw_results(frame, state, data, game_state, area),
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
