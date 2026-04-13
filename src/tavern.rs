//! Tavern dashboard — the main game UI.
//!
//! Submodules:
//! - `state`           — TavernState, sub-view state, ticks
//! - `input`           — keyboard handlers
//! - `util`            — drawing helpers (color/rect/text utilities)
//! - `left`            — left column (Menu, Lanty, Expeditions panel)
//! - `right`           — right column (content panel + input bar)
//! - `gathering_view`  — Gathering view drawing & loot popups
//! - `inventory_view`  — Inventory grid + detail panel

mod adventure_view;
mod crafting_view;
mod gathering_view;
mod input;
mod inventory_view;
mod left;
mod refining_view;
mod right;
mod state;
mod tile_graphics;
mod util;

use std::io;

use crossterm::event::{self, Event, KeyEventKind};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::prelude::{CrosstermBackend, Frame};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Terminal;

use crate::game::{self, GameData, GameState};
use crate::ui;

use input::handle_input;
use left::draw_left;
use right::draw_right;
use state::{
    tick_loot_popups, tick_transition, LootPopup, PopupSource, TavernState, POPUP_DURATION_FRAMES,
};

pub fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    data: &GameData,
    game_state: &mut GameState,
) -> io::Result<()> {
    let mut state = TavernState::new();

    loop {
        // Tick animations
        state.frame_count = state.frame_count.wrapping_add(1);
        tick_transition(&mut state);
        tick_loot_popups(&mut state);

        // Tick game logic — check for completed tasks
        let events = game_state.update(data);
        for event in events {
            match event {
                game::GameEvent::GatherComplete {
                    slot_index,
                    location_id,
                    location,
                    items,
                } => {
                    // Resolve item names + rarity for the popup
                    let popup_items: Vec<(String, u32, game::Rarity)> = items
                        .iter()
                        .map(|(id, qty)| {
                            let def = data.item_registry.get(id);
                            let name = def
                                .map(|d| d.name.clone())
                                .unwrap_or_else(|| id.0.clone());
                            let rarity = def.map(|d| d.rarity).unwrap_or(game::Rarity::Common);
                            (name, *qty, rarity)
                        })
                        .collect();

                    state.loot_popups.push(LootPopup {
                        source: PopupSource::Gather {
                            slot_index,
                            location_id: location_id.clone(),
                        },
                        items: popup_items,
                        frames_remaining: POPUP_DURATION_FRAMES,
                    });

                    // Log message
                    let item_list: Vec<String> = items
                        .iter()
                        .map(|(id, qty)| {
                            let name = data
                                .item_registry
                                .get(id)
                                .map(|d| d.name.as_str())
                                .unwrap_or(id.0.as_str());
                            format!("{}x {}", qty, name)
                        })
                        .collect();
                    state.log_messages.push((
                        format!(
                            "Gathering at {} complete! Got: {}",
                            location,
                            item_list.join(", ")
                        ),
                        Style::default().fg(ui::GOLD),
                    ));
                    state.auto_scroll(20);
                }
                game::GameEvent::RefiningBatchDone {
                    station,
                    recipe_name,
                    output_id,
                    output_qty,
                    halted_for_lack_of_input,
                } => {
                    let def = data.item_registry.get(&output_id);
                    let output_name = def
                        .map(|d| d.name.clone())
                        .unwrap_or_else(|| output_id.0.clone());
                    let rarity = def.map(|d| d.rarity).unwrap_or(game::Rarity::Common);

                    if output_qty > 0 {
                        state.loot_popups.push(LootPopup {
                            source: PopupSource::Refine { station },
                            items: vec![(output_name.clone(), output_qty, rarity)],
                            frames_remaining: POPUP_DURATION_FRAMES,
                        });
                    }

                    let msg = if halted_for_lack_of_input {
                        format!(
                            "{} halted at the {} — ran out of materials. Produced {}× {}.",
                            recipe_name,
                            station.label(),
                            output_qty,
                            output_name
                        )
                    } else {
                        format!(
                            "{} complete at the {}! Produced {}× {}.",
                            recipe_name,
                            station.label(),
                            output_qty,
                            output_name
                        )
                    };
                    state
                        .log_messages
                        .push((msg, Style::default().fg(ui::GOLD)));
                    state.auto_scroll(20);
                }
                game::GameEvent::CraftingBatchDone {
                    recipe_name,
                    output_id,
                    output_qty,
                    halted_for_lack_of_input,
                } => {
                    let def = data.item_registry.get(&output_id);
                    let output_name = def
                        .map(|d| d.name.clone())
                        .unwrap_or_else(|| output_id.0.clone());
                    let rarity = def.map(|d| d.rarity).unwrap_or(game::Rarity::Common);

                    if output_qty > 0 {
                        state.loot_popups.push(LootPopup {
                            source: PopupSource::Craft,
                            items: vec![(output_name.clone(), output_qty, rarity)],
                            frames_remaining: POPUP_DURATION_FRAMES,
                        });
                    }

                    let msg = if halted_for_lack_of_input {
                        format!(
                            "{} halted at the bench — ran out of materials. Produced {}× {}.",
                            recipe_name, output_qty, output_name
                        )
                    } else {
                        format!(
                            "{} complete! Produced {}× {}.",
                            recipe_name, output_qty, output_name
                        )
                    };
                    state
                        .log_messages
                        .push((msg, Style::default().fg(ui::GOLD)));
                    state.auto_scroll(20);
                }
                game::GameEvent::LevelUp {
                    adventurer_name,
                    new_level,
                } => {
                    state.log_messages.push((
                        format!("{} reached level {}!", adventurer_name, new_level),
                        Style::default().fg(ui::GOLD),
                    ));
                    state.auto_scroll(20);
                }
            }
        }

        // Reset the pending Kitty queues for this frame
        let had_adv_portraits = !state.pending_adv_portraits.is_empty();
        state.pending_kitty_tiles.clear();
        state.pending_enemy_portrait = None;
        state.pending_adv_portraits.clear();

        terminal.draw(|frame| draw(frame, &mut state, data, game_state))?;

        // After ratatui finishes drawing, flush any queued Kitty graphics
        // directly to stdout.
        if !state.pending_kitty_tiles.is_empty()
            || !state.prev_frame_kitty_ids.is_empty()
        {
            let pending = std::mem::take(&mut state.pending_kitty_tiles);
            let prev_ids = std::mem::take(&mut state.prev_frame_kitty_ids);
            state.tile_graphics.flush_pending(&pending, &prev_ids);
            state.prev_frame_kitty_ids = pending.iter().map(|t| t.3).collect();
            state.pending_kitty_tiles = pending;
            state.pending_kitty_tiles.clear();
        }

        // Flush enemy portrait (if any)
        if let Some((x, y, ref name, cols, rows)) = state.pending_enemy_portrait {
            state
                .tile_graphics
                .flush_enemy_portrait(x, y, name, cols, rows);
        }

        // Flush adventurer portraits (if any), or clean up if we had some last
        // frame but don't this frame (view switched away).
        if !state.pending_adv_portraits.is_empty() {
            // Clean old ones first then draw new
            state.tile_graphics.cleanup_adv_portraits();
            let portraits = std::mem::take(&mut state.pending_adv_portraits);
            state.tile_graphics.flush_adv_portraits(&portraits);
        } else if had_adv_portraits {
            // We had portraits last frame but not this frame — clean up
            state.tile_graphics.cleanup_adv_portraits();
        }

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                if handle_input(&mut state, game_state, data, key.code, key.modifiers) {
                    // Clean up any leftover Kitty graphics images so they
                    // don't persist in the terminal after we exit.
                    state.tile_graphics.cleanup_all();
                    return Ok(());
                }
            }
        }
    }
}

fn draw(frame: &mut Frame, state: &mut TavernState, data: &GameData, game_state: &GameState) {
    let area = frame.area();

    frame.render_widget(
        Block::default().style(Style::default().bg(ui::SHADOW_BG)),
        area,
    );

    let main_split = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(area);

    draw_left(frame, state, main_split[0], data, game_state);
    draw_right(frame, state, main_split[1], data, game_state);

    // Modal overlays drawn last so they sit above everything
    if state.quit_prompt_open {
        draw_quit_prompt(frame, area);
    }
}

fn draw_quit_prompt(frame: &mut Frame, screen: Rect) {
    let modal_w: u16 = 38;
    let modal_h: u16 = 7;
    let x = screen.x + (screen.width.saturating_sub(modal_w)) / 2;
    let y = screen.y + (screen.height.saturating_sub(modal_h)) / 2;
    let area = Rect {
        x,
        y,
        width: modal_w.min(screen.width),
        height: modal_h.min(screen.height),
    };

    // Clear the area first so background bleed doesn't interfere
    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ui::FLAME).add_modifier(Modifier::BOLD))
        .title(Line::from(Span::styled(
            " Quit? ",
            Style::default().fg(ui::FLAME).add_modifier(Modifier::BOLD),
        )))
        .style(Style::default().bg(ui::SHADOW_BG));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Leave The Last Light?",
            Style::default()
                .fg(ui::WARM_WHITE)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "[Y] ",
                Style::default().fg(ui::FLAME).add_modifier(Modifier::BOLD),
            ),
            Span::styled("Yes", Style::default().fg(ui::WARM_WHITE)),
            Span::styled("    ", Style::default()),
            Span::styled(
                "[N] ",
                Style::default().fg(ui::GOLD).add_modifier(Modifier::BOLD),
            ),
            Span::styled("Stay", Style::default().fg(ui::WARM_WHITE)),
        ]),
    ];

    frame.render_widget(
        Paragraph::new(lines).alignment(Alignment::Center),
        inner,
    );
}
