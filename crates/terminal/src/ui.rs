/// Shared UI constants, colors, and embedded image art for The Last Light.
use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};

use crate::halfblock;

// ── Color palette ──────────────────────────────────────────────────────────
pub const GOLD: Color = Color::Rgb(212, 160, 32);
pub const WARM_WHITE: Color = Color::Rgb(240, 230, 210);
pub const DIM: Color = Color::Rgb(100, 85, 65);
pub const FLAME: Color = Color::Rgb(255, 140, 40);
pub const EMBER: Color = Color::Rgb(200, 80, 20);
pub const SHADOW_BG: Color = Color::Rgb(30, 25, 20);
pub const BORDER: Color = Color::Rgb(80, 65, 45);

// ── Forest palette ─────────────────────────────────────────────────────────
pub const FOREST_GREEN: Color = Color::Rgb(75, 130, 70);

// ── Rarity palette ─────────────────────────────────────────────────────────
pub const RARITY_COMMON: Color = Color::Rgb(240, 230, 210);  // WARM_WHITE
pub const RARITY_UNCOMMON: Color = Color::Rgb(90, 180, 90);  // green
pub const RARITY_RARE: Color = Color::Rgb(80, 150, 230);     // blue
pub const RARITY_VERY_RARE: Color = Color::Rgb(170, 100, 220); // purple

pub fn rarity_color(rarity: crate::game::Rarity) -> Color {
    match rarity {
        crate::game::Rarity::Common => RARITY_COMMON,
        crate::game::Rarity::Uncommon => RARITY_UNCOMMON,
        crate::game::Rarity::Rare => RARITY_RARE,
        crate::game::Rarity::VeryRare => RARITY_VERY_RARE,
    }
}

// ── Embedded assets (baked into the binary) ────────────────────────────────
const LANTY_SVG: &[u8] = include_bytes!("../../../assets/lanty.svg");
const LNTRN_PNG: &[u8] = include_bytes!("../../../assets/lntrn.png");
const TREE_PINE_PNG: &[u8] = include_bytes!("../../../assets/tree_pine.png");
const TREE_OAK_PNG: &[u8] = include_bytes!("../../../assets/tree_oak.png");

// ── Map tile icons (for adventures) ────────────────────────────────────────
const TILE_EMPTY: &[u8] = include_bytes!("../../../assets/tiles/empty.png");
const TILE_TREASURE: &[u8] = include_bytes!("../../../assets/tiles/treasure.png");
const TILE_REST: &[u8] = include_bytes!("../../../assets/tiles/rest.png");
const TILE_TRAP: &[u8] = include_bytes!("../../../assets/tiles/trap.png");
const TILE_COMBAT: &[u8] = include_bytes!("../../../assets/tiles/combat.png");
const TILE_BOSS: &[u8] = include_bytes!("../../../assets/tiles/boss.png");
const TILE_PARTY: &[u8] = include_bytes!("../../../assets/tiles/party.png");
const TILE_FOG: &[u8] = include_bytes!("../../../assets/tiles/fog.png");
const TILE_LADDER_DOWN: &[u8] = include_bytes!("../../../assets/tiles/ladder_down.png");
const TILE_LADDER_UP: &[u8] = include_bytes!("../../../assets/tiles/ladder_up.png");

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum MapTile {
    Empty,
    Treasure,
    Rest,
    Trap,
    Combat,
    Boss,
    Party,
    Fog,
    LadderDown,
    LadderUp,
}

pub fn map_tile_bytes(tile: MapTile) -> &'static [u8] {
    match tile {
        MapTile::Empty => TILE_EMPTY,
        MapTile::Treasure => TILE_TREASURE,
        MapTile::Rest => TILE_REST,
        MapTile::Trap => TILE_TRAP,
        MapTile::Combat => TILE_COMBAT,
        MapTile::Boss => TILE_BOSS,
        MapTile::Party => TILE_PARTY,
        MapTile::Fog => TILE_FOG,
        MapTile::LadderDown => TILE_LADDER_DOWN,
        MapTile::LadderUp => TILE_LADDER_UP,
    }
}

pub fn map_tile(tile: MapTile, width: u16) -> Vec<Line<'static>> {
    halfblock::png_to_halfblock(map_tile_bytes(tile), width)
}

// ── Adventurer portraits ───────────────────────────────────────────────────
#[allow(dead_code)]
const PORTRAIT_TORVALD: &[u8] = include_bytes!("../../../assets/portraits/torvald.png");
#[allow(dead_code)]
const PORTRAIT_SYLVARA: &[u8] = include_bytes!("../../../assets/portraits/sylvara.png");
#[allow(dead_code)]
const PORTRAIT_EMBER: &[u8] = include_bytes!("../../../assets/portraits/ember.png");
#[allow(dead_code)]
const PORTRAIT_ALDRIC: &[u8] = include_bytes!("../../../assets/portraits/aldric.png");
#[allow(dead_code)]
const PORTRAIT_BRIAR: &[u8] = include_bytes!("../../../assets/portraits/briar.png");

#[allow(dead_code)]
pub fn adventurer_portrait_bytes(id: &str) -> Option<&'static [u8]> {
    match id {
        "torvald" => Some(PORTRAIT_TORVALD),
        "sylvara" => Some(PORTRAIT_SYLVARA),
        "ember" => Some(PORTRAIT_EMBER),
        "aldric" => Some(PORTRAIT_ALDRIC),
        "briar" => Some(PORTRAIT_BRIAR),
        _ => None,
    }
}

#[allow(dead_code)]
pub fn adventurer_portrait(id: &str, width: u16) -> Option<Vec<Line<'static>>> {
    adventurer_portrait_bytes(id).map(|bytes| halfblock::png_to_halfblock(bytes, width))
}

// ── Enemy portraits ────────────────────────────────────────────────────────
const ENEMY_GIANT_RAT: &[u8] = include_bytes!("../../../assets/enemies/giant_rat.png");
const ENEMY_RAT_KING: &[u8] = include_bytes!("../../../assets/enemies/rat_king.png");
const ENEMY_CELLAR_SPIDER: &[u8] = include_bytes!("../../../assets/enemies/cellar_spider.png");
const ENEMY_CAVE_SPIDER: &[u8] = include_bytes!("../../../assets/enemies/cave_spider.png");
const ENEMY_GIANT_SNAKE: &[u8] = include_bytes!("../../../assets/enemies/giant_snake.png");
const ENEMY_BROOD_MOTHER: &[u8] = include_bytes!("../../../assets/enemies/brood_mother.png");

pub fn enemy_portrait_bytes(name: &str) -> Option<&'static [u8]> {
    match name {
        "Giant Rat" => Some(ENEMY_GIANT_RAT),
        "Rat King" => Some(ENEMY_RAT_KING),
        "Cellar Spider" => Some(ENEMY_CELLAR_SPIDER),
        "Cave Spider" => Some(ENEMY_CAVE_SPIDER),
        "Giant Snake" => Some(ENEMY_GIANT_SNAKE),
        "Brood Mother" => Some(ENEMY_BROOD_MOTHER),
        _ => None,
    }
}

// ── Item icons (compile-time registry) ─────────────────────────────────────
// To add a new item icon, add a PNG to assets/items/ and a line to ITEM_ICONS.
const ITEM_ICONS: &[(&str, &[u8])] = &[
    ("wood", include_bytes!("../../../assets/items/wood.png")),
    ("herbs", include_bytes!("../../../assets/items/herbs.png")),
    ("berries", include_bytes!("../../../assets/items/berries.png")),
    ("heartwood", include_bytes!("../../../assets/items/heartwood.png")),
    ("planks", include_bytes!("../../../assets/items/planks.png")),
    ("kindling", include_bytes!("../../../assets/items/kindling.png")),
    ("dried_herbs", include_bytes!("../../../assets/items/dried_herbs.png")),
    ("crushed_herbs", include_bytes!("../../../assets/items/crushed_herbs.png")),
    ("dried_berries", include_bytes!("../../../assets/items/dried_berries.png")),
    ("berry_juice", include_bytes!("../../../assets/items/berry_juice.png")),
    // Crafted gear
    ("wooden_club", include_bytes!("../../../assets/items/wooden_club.png")),
    ("hunters_bow", include_bytes!("../../../assets/items/hunters_bow.png")),
    ("herbalists_staff", include_bytes!("../../../assets/items/herbalists_staff.png")),
    ("bark_vest", include_bytes!("../../../assets/items/bark_vest.png")),
    ("herb_cloak", include_bytes!("../../../assets/items/herb_cloak.png")),
    ("berry_pendant", include_bytes!("../../../assets/items/berry_pendant.png")),
    ("heartwood_charm", include_bytes!("../../../assets/items/heartwood_charm.png")),
    // Tavern goods
    ("hearty_stew", include_bytes!("../../../assets/items/hearty_stew.png")),
    ("berry_tart", include_bytes!("../../../assets/items/berry_tart.png")),
    ("herb_bread", include_bytes!("../../../assets/items/herb_bread.png")),
    ("berry_cordial", include_bytes!("../../../assets/items/berry_cordial.png")),
    ("herbal_tea", include_bytes!("../../../assets/items/herbal_tea.png")),
];

/// Look up raw PNG bytes for an item's icon. Returns None if no icon is registered.
pub fn item_icon_bytes(item_id: &str) -> Option<&'static [u8]> {
    ITEM_ICONS
        .iter()
        .find(|(id, _)| *id == item_id)
        .map(|(_, bytes)| *bytes)
}

/// Render an item icon at the given width. Returns None if no icon is registered.
pub fn item_icon(item_id: &str, width: u16) -> Option<Vec<Line<'static>>> {
    item_icon_bytes(item_id).map(|bytes| halfblock::png_to_halfblock(bytes, width))
}

// ── Lanty the mushroom (rendered from SVG) ─────────────────────────────────

/// Render Lanty at the given width in terminal columns.
pub fn lanty_portrait(width: u16) -> Vec<Line<'static>> {
    halfblock::svg_to_halfblock(LANTY_SVG, width)
}

// ── Lantern icon (rendered from PNG) ───────────────────────────────────────

/// Render the lntrn lantern at the given width in terminal columns.
pub fn lantern_art(width: u16) -> Vec<Line<'static>> {
    halfblock::png_to_halfblock(LNTRN_PNG, width)
}

// ── Forest trees (rendered from PNG) ───────────────────────────────────────

/// Render a tall pine tree sprite at the given width in terminal columns.
pub fn tree_pine(width: u16) -> Vec<Line<'static>> {
    halfblock::png_to_halfblock(TREE_PINE_PNG, width)
}

/// Render a bushy oak tree sprite at the given width in terminal columns.
pub fn tree_oak(width: u16) -> Vec<Line<'static>> {
    halfblock::png_to_halfblock(TREE_OAK_PNG, width)
}

// ── Ember particles ────────────────────────────────────────────────────────
pub fn ember_line(frame_count: u64, width: u16) -> Line<'static> {
    let mut spans = Vec::new();
    for i in 0..width {
        let hash = ((i as u64).wrapping_mul(2654435761) ^ frame_count.wrapping_mul(40503)) % 100;
        if hash < 2 {
            let c = if hash == 0 { FLAME } else { EMBER };
            spans.push(Span::styled("·", Style::default().fg(c)));
        } else {
            spans.push(Span::styled(" ", Style::default()));
        }
    }
    Line::from(spans)
}

// ── Forest ambiance particles ──────────────────────────────────────────────
pub const LEAF_GREEN: Color = Color::Rgb(90, 130, 60);
pub const LEAF_AMBER: Color = Color::Rgb(150, 110, 50);

/// Sparse drifting leaf/seed particles for forest scenes.
pub fn leaf_line(frame_count: u64, width: u16, row_seed: u16) -> Line<'static> {
    let mut spans = Vec::new();
    for i in 0..width {
        let drift = frame_count / 4;
        let hash = (((i as u64 + drift) ^ (row_seed as u64).wrapping_mul(2246822519))
            .wrapping_mul(2654435761))
            % 400;
        if hash < 2 {
            let c = if hash == 0 { LEAF_GREEN } else { LEAF_AMBER };
            spans.push(Span::styled("·", Style::default().fg(c)));
        } else {
            spans.push(Span::styled(" ", Style::default()));
        }
    }
    Line::from(spans)
}
