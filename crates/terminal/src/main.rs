pub use last_light_game as game;
mod halfblock;
mod menu;
mod tavern;
mod ui;

use std::io;

use crossterm::{
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::prelude::*;

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    // Show main menu
    let start = menu::run(&mut terminal)?;

    if start {
        // Load game definitions and saved state
        let data = game::GameData::new();
        let mut game_state = game::load_game().unwrap_or_else(game::GameState::new);
        game_state.migrate_adventurers();

        // Run the tavern dashboard
        tavern::run(&mut terminal, &data, &mut game_state)?;

        // Auto-save on quit
        if let Err(e) = game::save_game(&game_state) {
            eprintln!("Save failed: {e}");
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}
