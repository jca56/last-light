use std::fs;
use std::path::PathBuf;

use super::GameState;

fn save_dir() -> PathBuf {
    let mut dir = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    dir.push("last-light");
    dir
}

fn save_path() -> PathBuf {
    save_dir().join("save.json")
}

pub fn save_game(state: &GameState) -> Result<(), String> {
    let dir = save_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create save dir: {e}"))?;
    let json = serde_json::to_string_pretty(state).map_err(|e| format!("Serialize error: {e}"))?;
    fs::write(save_path(), json).map_err(|e| format!("Write error: {e}"))?;
    Ok(())
}

pub fn load_game() -> Option<GameState> {
    let path = save_path();
    if !path.exists() {
        return None;
    }
    let data = fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}
