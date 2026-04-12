//! High-resolution tile graphics for the adventure map.
//!
//! Two render paths:
//!
//! 1. **Lantern direct-placement Kitty** (used when running in Lantern Terminal):
//!    Sends raw Kitty graphics escape sequences in direct placement mode
//!    (`a=T,f=100,c=W,r=H`). Lantern's image manager decodes the PNG and
//!    renders it at the cursor position with the requested cell dimensions.
//!    This is much simpler than ratatui-image's Unicode placeholder approach
//!    and matches what Lantern actually implements.
//!
//! 2. **ratatui-image fallback** (other terminals):
//!    Wraps ratatui-image which auto-detects Kitty/Sixel/iTerm2/halfblock
//!    and uses whichever protocol the terminal supports.

use std::collections::HashMap;

use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;

use crate::ui::MapTile;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TileBackend {
    /// Inline raw Kitty escape sequences (direct placement). Used in Lantern.
    KittyInline,
    /// ratatui-image with whatever protocol it auto-detected.
    Ratatui,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum TileKey {
    Empty,
    Treasure,
    Rest,
    Trap,
    Combat,
    Boss,
    Party,
    Fog,
}

impl TileKey {
    pub(super) fn from_map(t: MapTile) -> Self {
        match t {
            MapTile::Empty => TileKey::Empty,
            MapTile::Treasure => TileKey::Treasure,
            MapTile::Rest => TileKey::Rest,
            MapTile::Trap => TileKey::Trap,
            MapTile::Combat => TileKey::Combat,
            MapTile::Boss => TileKey::Boss,
            MapTile::Party => TileKey::Party,
            MapTile::Fog => TileKey::Fog,
        }
    }

    fn bytes(self) -> &'static [u8] {
        match self {
            TileKey::Empty => crate::ui::map_tile_bytes(MapTile::Empty),
            TileKey::Treasure => crate::ui::map_tile_bytes(MapTile::Treasure),
            TileKey::Rest => crate::ui::map_tile_bytes(MapTile::Rest),
            TileKey::Trap => crate::ui::map_tile_bytes(MapTile::Trap),
            TileKey::Combat => crate::ui::map_tile_bytes(MapTile::Combat),
            TileKey::Boss => crate::ui::map_tile_bytes(MapTile::Boss),
            TileKey::Party => crate::ui::map_tile_bytes(MapTile::Party),
            TileKey::Fog => crate::ui::map_tile_bytes(MapTile::Fog),
        }
    }
}

pub(super) struct TileGraphics {
    pub(super) backend: TileBackend,
    /// ratatui-image stateful protocols (used for Ratatui backend)
    pub(super) ratatui_tiles: HashMap<TileKey, StatefulProtocol>,
    /// Pre-built base64-encoded Kitty escape sequence base for each tile
    /// (used for KittyInline backend). Format: full PNG base64 string ready
    /// to splice into a `\x1b_Ga=T,f=100,c=W,r=H;<b64>\x1b\\` command.
    pub(super) kitty_b64: HashMap<TileKey, String>,
}

impl TileGraphics {
    pub(super) fn new() -> Self {
        // Detect Lantern terminal — if so, use the inline Kitty path
        let is_lantern =
            std::env::var("TERM_PROGRAM").is_ok_and(|p| p == "Lantern");

        if is_lantern {
            // Pre-encode each tile PNG as base64 for fast escape-sequence assembly
            let mut kitty_b64 = HashMap::new();
            for key in [
                TileKey::Empty,
                TileKey::Treasure,
                TileKey::Rest,
                TileKey::Trap,
                TileKey::Combat,
                TileKey::Boss,
                TileKey::Party,
                TileKey::Fog,
            ] {
                kitty_b64.insert(key, base64_encode(key.bytes()));
            }
            return TileGraphics {
                backend: TileBackend::KittyInline,
                ratatui_tiles: HashMap::new(),
                kitty_b64,
            };
        }

        // Other terminals — use ratatui-image with auto-detected protocol
        let picker = Picker::from_query_stdio()
            .unwrap_or_else(|_| Picker::from_fontsize((8, 16)));

        let mut ratatui_tiles = HashMap::new();
        for key in [
            TileKey::Empty,
            TileKey::Treasure,
            TileKey::Rest,
            TileKey::Trap,
            TileKey::Combat,
            TileKey::Boss,
            TileKey::Party,
            TileKey::Fog,
        ] {
            if let Ok(img) = image::load_from_memory(key.bytes()) {
                ratatui_tiles.insert(key, picker.new_resize_protocol(img));
            }
        }

        TileGraphics {
            backend: TileBackend::Ratatui,
            ratatui_tiles,
            kitty_b64: HashMap::new(),
        }
    }

    pub(super) fn ratatui_get_mut(&mut self, t: MapTile) -> Option<&mut StatefulProtocol> {
        self.ratatui_tiles.get_mut(&TileKey::from_map(t))
    }

    /// Flush a list of queued tile draws directly to stdout, bypassing
    /// ratatui's buffer entirely. Each draw:
    ///  1. Deletes any existing image at the same ID
    ///  2. Positions the cursor with CUP
    ///  3. Transmits and displays the new image
    ///
    /// Also sends delete commands for any IDs from the previous frame that
    /// aren't in the current draw list, ensuring stale images get cleared.
    pub(super) fn flush_pending(
        &self,
        tiles: &[(u16, u16, MapTile, u32, u16, u16)],
        prev_ids: &[u32],
    ) {
        if self.backend != TileBackend::KittyInline {
            return;
        }
        use std::io::Write;
        let mut out = String::new();

        // Delete IDs that were in the previous frame but not in this one
        let current_ids: Vec<u32> = tiles.iter().map(|t| t.3).collect();
        for prev in prev_ids {
            if !current_ids.contains(prev) {
                out.push_str(&format!("\x1b_Ga=d,d=I,i={},q=2\x1b\\", prev));
            }
        }

        // Save cursor position so we can restore it after our writes (so the
        // text cursor doesn't end up jumping to the last image we drew)
        out.push_str("\x1b[s");

        for (x, y, tile, id, cols, rows) in tiles {
            let Some(b64) = self.kitty_b64.get(&TileKey::from_map(*tile)) else {
                continue;
            };
            // Delete old image at this ID
            out.push_str(&format!("\x1b_Ga=d,d=I,i={},q=2\x1b\\", id));
            // Move cursor to the tile's position (1-indexed in CUP)
            out.push_str(&format!("\x1b[{};{}H", y + 1, x + 1));
            // Transmit and display the new tile
            out.push_str(&format!(
                "\x1b_Ga=T,i={id},q=2,f=100,c={cols},r={rows};{b64}\x1b\\"
            ));
        }

        // Restore cursor
        out.push_str("\x1b[u");

        let mut stdout = std::io::stdout();
        let _ = stdout.write_all(out.as_bytes());
        let _ = stdout.flush();
    }

    /// Write delete commands for all map-tile image IDs to stdout. Call this
    /// when leaving the adventure view or quitting the game so Lantern's
    /// image manager doesn't keep stale images on screen.
    pub(super) fn cleanup_all(&self) {
        if self.backend != TileBackend::KittyInline {
            return;
        }
        use std::io::Write;
        let mut out = String::new();
        // Cover the full possible ID range we use (5x5 map with id = row*100+col+1)
        for y in 0..10u32 {
            for x in 0..10u32 {
                let id = y * 100 + x + 1;
                out.push_str(&format!("\x1b_Ga=d,d=I,i={},q=2\x1b\\", id));
            }
        }
        let mut stdout = std::io::stdout();
        let _ = stdout.write_all(out.as_bytes());
        let _ = stdout.flush();
    }
}

/// Compute a unique image ID for a tile at the given map position.
pub(super) fn tile_image_id(x: u32, y: u32) -> u32 {
    // Plenty of room for maps up to 100 wide
    y * 100 + x + 1
}

/// Standard base64 encoding (no line breaks).
fn base64_encode(input: &[u8]) -> String {
    const CHARSET: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((input.len() + 2) / 3 * 4);
    let mut chunks = input.chunks_exact(3);
    for chunk in chunks.by_ref() {
        let n = ((chunk[0] as u32) << 16) | ((chunk[1] as u32) << 8) | (chunk[2] as u32);
        out.push(CHARSET[((n >> 18) & 0x3F) as usize] as char);
        out.push(CHARSET[((n >> 12) & 0x3F) as usize] as char);
        out.push(CHARSET[((n >> 6) & 0x3F) as usize] as char);
        out.push(CHARSET[(n & 0x3F) as usize] as char);
    }
    let rem = chunks.remainder();
    match rem.len() {
        1 => {
            let n = (rem[0] as u32) << 16;
            out.push(CHARSET[((n >> 18) & 0x3F) as usize] as char);
            out.push(CHARSET[((n >> 12) & 0x3F) as usize] as char);
            out.push('=');
            out.push('=');
        }
        2 => {
            let n = ((rem[0] as u32) << 16) | ((rem[1] as u32) << 8);
            out.push(CHARSET[((n >> 18) & 0x3F) as usize] as char);
            out.push(CHARSET[((n >> 12) & 0x3F) as usize] as char);
            out.push(CHARSET[((n >> 6) & 0x3F) as usize] as char);
            out.push('=');
        }
        _ => {}
    }
    out
}
