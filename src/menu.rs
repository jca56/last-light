use std::io;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    prelude::*,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Terminal,
};

use crate::ui;

/// "LAST LIGHT" in halfblock pixel font style.
/// Each letter is 5 wide x 6 pixel rows, packed into 3 terminal rows.
fn title_lines() -> Vec<Line<'static>> {
    #[rustfmt::skip]
    let letters: Vec<[[u8; 5]; 6]> = vec![
        // L
        [[1,0,0,0,0],[1,0,0,0,0],[1,0,0,0,0],[1,0,0,0,0],[1,0,0,0,0],[1,1,1,1,0]],
        // A
        [[0,1,1,0,0],[1,0,0,1,0],[1,0,0,1,0],[1,1,1,1,0],[1,0,0,1,0],[1,0,0,1,0]],
        // S
        [[0,1,1,1,0],[1,0,0,0,0],[0,1,1,0,0],[0,0,0,1,0],[0,0,0,1,0],[1,1,1,0,0]],
        // T
        [[1,1,1,1,1],[0,0,1,0,0],[0,0,1,0,0],[0,0,1,0,0],[0,0,1,0,0],[0,0,1,0,0]],
        // (space)
        [[0,0,0,0,0],[0,0,0,0,0],[0,0,0,0,0],[0,0,0,0,0],[0,0,0,0,0],[0,0,0,0,0]],
        // L
        [[1,0,0,0,0],[1,0,0,0,0],[1,0,0,0,0],[1,0,0,0,0],[1,0,0,0,0],[1,1,1,1,0]],
        // I
        [[0,1,1,1,0],[0,0,1,0,0],[0,0,1,0,0],[0,0,1,0,0],[0,0,1,0,0],[0,1,1,1,0]],
        // G
        [[0,1,1,1,0],[1,0,0,0,0],[1,0,0,0,0],[1,0,1,1,0],[1,0,0,1,0],[0,1,1,1,0]],
        // H
        [[1,0,0,1,0],[1,0,0,1,0],[1,1,1,1,0],[1,0,0,1,0],[1,0,0,1,0],[1,0,0,1,0]],
        // T
        [[1,1,1,1,1],[0,0,1,0,0],[0,0,1,0,0],[0,0,1,0,0],[0,0,1,0,0],[0,0,1,0,0]],
    ];

    let total_width = letters.len() * 6 - 1;
    let mut rows: Vec<Vec<u8>> = vec![vec![0; total_width]; 6];

    for (li, letter) in letters.iter().enumerate() {
        let x_off = li * 6;
        for y in 0..6 {
            for x in 0..5 {
                if x_off + x < total_width {
                    rows[y][x_off + x] = letter[y][x];
                }
            }
        }
    }

    let mut lines = Vec::new();
    for row_pair in 0..3 {
        let top_row = &rows[row_pair * 2];
        let bot_row = &rows[row_pair * 2 + 1];

        let mut spans = Vec::new();
        for col in 0..total_width {
            let top = top_row[col] == 1;
            let bot = bot_row[col] == 1;
            match (top, bot) {
                (true, true) => spans.push(Span::styled("█", Style::default().fg(ui::GOLD))),
                (true, false) => spans.push(Span::styled("▀", Style::default().fg(ui::GOLD))),
                (false, true) => spans.push(Span::styled("▄", Style::default().fg(ui::GOLD))),
                (false, false) => spans.push(Span::styled(" ", Style::default())),
            }
        }
        lines.push(Line::from(spans));
    }
    lines
}

pub fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<bool> {
    let mut selected = 0;
    let menu_items = vec!["Start"];
    let mut frame_count: u64 = 0;

    loop {
        terminal.draw(|frame| {
            let area = frame.area();

            // Dark background
            frame.render_widget(
                Block::default().style(Style::default().bg(ui::SHADOW_BG)),
                area,
            );

            // Vertical layout
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(2),        // top spacer
                    Constraint::Length(1),      // ember particles
                    Constraint::Length(1),      // spacing
                    Constraint::Length(9),      // lantern
                    Constraint::Length(1),      // spacing
                    Constraint::Length(3),      // title
                    Constraint::Length(1),      // subtitle
                    Constraint::Length(2),      // spacing
                    Constraint::Length(1),      // menu item
                    Constraint::Min(1),        // bottom (mushroom)
                    Constraint::Length(1),      // hint bar
                ])
                .split(area);

            // Ember particles
            frame.render_widget(
                Paragraph::new(ui::ember_line(frame_count, area.width)),
                chunks[1],
            );

            // Lantern centered above title (rendered from PNG)
            let lantern_w = chunks[3].width.min(16);
            let lantern = ui::lantern_art(lantern_w);
            frame.render_widget(
                Paragraph::new(lantern).alignment(Alignment::Center),
                chunks[3],
            );

            // Title "LAST LIGHT"
            frame.render_widget(
                Paragraph::new(title_lines())
                    .alignment(Alignment::Center)
                    .style(Style::default().bg(ui::SHADOW_BG)),
                chunks[5],
            );

            // Subtitle
            let subtitle = Line::from(vec![
                Span::styled("━━━ ", Style::default().fg(ui::DIM)),
                Span::styled("An Inn at the Edge of the Wilds", Style::default().fg(ui::WARM_WHITE)),
                Span::styled(" ━━━", Style::default().fg(ui::DIM)),
            ]);
            frame.render_widget(
                Paragraph::new(subtitle).alignment(Alignment::Center),
                chunks[6],
            );

            // Menu
            let mut menu_lines = Vec::new();
            for (i, item) in menu_items.iter().enumerate() {
                if i == selected {
                    menu_lines.push(Line::from(vec![
                        Span::styled("  ▸ ", Style::default().fg(ui::FLAME)),
                        Span::styled(*item, Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD)),
                        Span::styled(" ◂  ", Style::default().fg(ui::FLAME)),
                    ]));
                } else {
                    menu_lines.push(Line::from(vec![
                        Span::styled("    ", Style::default()),
                        Span::styled(*item, Style::default().fg(ui::DIM)),
                        Span::styled("    ", Style::default()),
                    ]));
                }
            }
            frame.render_widget(
                Paragraph::new(menu_lines).alignment(Alignment::Center),
                chunks[8],
            );

            // Lanty in bottom-right (rendered from SVG)
            let bottom = chunks[9];
            let mush_w = 10u16.min(bottom.width.saturating_sub(2));
            if bottom.height >= 8 && mush_w >= 6 {
                let mush = ui::lanty_portrait(mush_w);
                let mush_h = mush.len() as u16;
                let rect = Rect {
                    x: bottom.right().saturating_sub(mush_w + 2),
                    y: bottom.bottom().saturating_sub(mush_h),
                    width: mush_w,
                    height: mush_h,
                };
                frame.render_widget(Paragraph::new(mush), rect);
            }

            // Controls hint
            let hint = Line::from(vec![
                Span::styled("↑↓", Style::default().fg(ui::DIM).add_modifier(Modifier::BOLD)),
                Span::styled(" navigate  ", Style::default().fg(ui::DIM)),
                Span::styled("Enter", Style::default().fg(ui::DIM).add_modifier(Modifier::BOLD)),
                Span::styled(" select  ", Style::default().fg(ui::DIM)),
                Span::styled("q", Style::default().fg(ui::DIM).add_modifier(Modifier::BOLD)),
                Span::styled(" quit", Style::default().fg(ui::DIM)),
            ]);
            frame.render_widget(
                Paragraph::new(hint).alignment(Alignment::Center),
                chunks[10],
            );
        })?;

        frame_count = frame_count.wrapping_add(1);

        if event::poll(std::time::Duration::from_millis(150))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(false),
                    KeyCode::Up => {
                        if selected > 0 { selected -= 1; }
                    }
                    KeyCode::Down => {
                        if selected < menu_items.len() - 1 { selected += 1; }
                    }
                    KeyCode::Enter => {
                        if selected == 0 {
                            return Ok(true); // Start game
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
