//! TavernState and all sub-view state types.
//!
//! `TavernState` is the UI state for the tavern dashboard. It is purely
//! presentation — game logic state lives in `GameState`. Fields are exposed
//! to the rest of the `tavern` module via `pub(super)` so siblings can read
//! and mutate them.

use std::collections::HashMap;

use ratatui::style::Style;
use ratatui::text::Line;

use super::tile_graphics::TileGraphics;
use crate::game::{self, GameData};
use crate::ui;

// ── Constants ─────────────────────────────────────────────────────────────

pub(super) const ICON_WIDTH: u16 = 8;
pub(super) const TRANSITION_FRAMES: u8 = 6;
pub(super) const POPUP_DURATION_FRAMES: u8 = 12;

// ── Focus & view ──────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
pub(super) enum Focus {
    Terminal,
    Input,
}

#[derive(Clone, Copy, PartialEq)]
pub(super) enum View {
    Terminal,
    Tavern,
    Inventory,
    Gathering,
    Refining,
    Crafting,
    Adventuring,
    Shop,
}

pub(super) const NAV_ITEMS: &[(View, &str)] = &[
    (View::Terminal, "Terminal"),
    (View::Tavern, "Tavern"),
    (View::Inventory, "Inventory"),
    (View::Gathering, "Gathering"),
    (View::Refining, "Refining"),
    (View::Crafting, "Crafting"),
    (View::Adventuring, "Adventuring"),
    (View::Shop, "Shop"),
];

// ── Gathering sub-view ────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
pub(super) enum GatheringScreen {
    Grounds,    // Location select grid
    AtLocation, // Inside a location
}

#[derive(Clone, Copy, PartialEq)]
pub(super) enum Transition {
    None,
    EnteringLocation(u8),
    LeavingLocation(u8),
}

pub(super) struct GatheringView {
    pub(super) screen: GatheringScreen,
    pub(super) selected_location: usize,
    pub(super) current_location: usize,
    pub(super) selected_duration: usize,
    pub(super) transition: Transition,
}

impl Default for GatheringView {
    fn default() -> Self {
        GatheringView {
            screen: GatheringScreen::Grounds,
            selected_location: 0,
            current_location: 0,
            selected_duration: 0,
            transition: Transition::None,
        }
    }
}

// ── Loot popup ────────────────────────────────────────────────────────────

/// Where a loot popup originates from. Determines where it gets anchored on screen.
#[derive(Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub(super) enum PopupSource {
    Gather {
        slot_index: usize,
        location_id: String,
    },
    Refine {
        station: game::StationKind,
    },
    Craft,
    Adventure,
}

pub(super) struct LootPopup {
    pub(super) source: PopupSource,
    pub(super) items: Vec<(String, u32, game::Rarity)>,
    pub(super) frames_remaining: u8,
}

// ── Adventure sub-view ────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
pub(super) enum RosterFocus {
    List,      // Browsing the adventurer list
    Equipment, // Managing equipment slots for the selected adventurer
}

#[derive(Clone, Copy, PartialEq)]
pub(super) enum AdventureScreen {
    Roster,     // View all adventurers, stats, equipment
    QuestBoard,
    PartySetup,
    InAdventure,
    Combat,
    Results,
}

#[derive(Clone, Copy, PartialEq)]
pub(super) enum PartySetupFocus {
    PartySlots,
    EquipmentSlots,
}

pub(super) struct AdventureView {
    pub(super) screen: AdventureScreen,
    pub(super) selected_adventurer: usize, // index into game_state.adventurers (for Roster)
    pub(super) roster_focus: RosterFocus,
    pub(super) roster_equip_slot: usize,   // 0=Weapon, 1=Armor, 2=Accessory
    pub(super) selected_quest: usize,
    /// 3 party slots, each is an index into game_state.adventurers (or None)
    pub(super) party_slots: [Option<usize>; 3],
    /// Which party slot is selected on Party Setup screen
    pub(super) setup_slot: usize,
    /// Which equipment slot we're managing for the selected party member
    pub(super) setup_equip_slot: usize, // 0=Weapon 1=Armor 2=Accessory
    pub(super) setup_focus: PartySetupFocus,
    /// Adventurer roster picker mode (when assigning to a slot)
    pub(super) picking_adventurer: bool,
    pub(super) picker_idx: usize,
    /// Combat: which party member is acting, what action they're on
    pub(super) combat_action_idx: usize, // 0=Attack 1=Defend 2=Flee
    pub(super) combat_target_idx: usize,
    pub(super) combat_picking_target: bool,
}

impl Default for AdventureView {
    fn default() -> Self {
        AdventureView {
            screen: AdventureScreen::Roster,
            selected_adventurer: 0,
            roster_focus: RosterFocus::List,
            roster_equip_slot: 0,
            selected_quest: 0,
            party_slots: [None, None, None],
            setup_slot: 0,
            setup_equip_slot: 0,
            setup_focus: PartySetupFocus::PartySlots,
            picking_adventurer: false,
            picker_idx: 0,
            combat_action_idx: 0,
            combat_target_idx: 0,
            combat_picking_target: false,
        }
    }
}

// ── Crafting sub-view ─────────────────────────────────────────────────────

pub(super) struct CraftingView {
    pub(super) selected_category: usize, // 0..5
    pub(super) selected_recipe: usize,   // index into recipes_in(category)
    pub(super) quantity: u32,
}

impl Default for CraftingView {
    fn default() -> Self {
        CraftingView {
            selected_category: 0,
            selected_recipe: 0,
            quantity: 1,
        }
    }
}

// ── Refining sub-view ─────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
pub(super) enum RefiningScreen {
    Stations,   // Station select grid
    AtStation,  // Workshop dashboard
}

pub(super) struct RefiningView {
    pub(super) screen: RefiningScreen,
    pub(super) selected_station: usize,  // index into data.refining_stations (0..4)
    pub(super) current_station: usize,   // the one you're "inside"
    pub(super) selected_recipe: usize,   // index into recipes_for_station(current)
    pub(super) quantity: u32,            // batch size
}

impl Default for RefiningView {
    fn default() -> Self {
        RefiningView {
            screen: RefiningScreen::Stations,
            selected_station: 0,
            current_station: 0,
            selected_recipe: 0,
            quantity: 1,
        }
    }
}

// ── Bottom-left active tasks panel ────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum BottomTab {
    Expeditions,
    Refining,
    Crafting,
    Adventures,
}

impl BottomTab {
    pub(super) fn label(self) -> &'static str {
        match self {
            BottomTab::Expeditions => "Exp",
            BottomTab::Refining => "Ref",
            BottomTab::Crafting => "Craft",
            BottomTab::Adventures => "Adv",
        }
    }

    pub(super) fn all() -> [BottomTab; 4] {
        [
            BottomTab::Expeditions,
            BottomTab::Refining,
            BottomTab::Crafting,
            BottomTab::Adventures,
        ]
    }

    pub(super) fn next(self) -> BottomTab {
        let all = Self::all();
        let i = all.iter().position(|t| *t == self).unwrap_or(0);
        all[(i + 1) % all.len()]
    }

    pub(super) fn prev(self) -> BottomTab {
        let all = Self::all();
        let i = all.iter().position(|t| *t == self).unwrap_or(0);
        all[(i + all.len() - 1) % all.len()]
    }
}

// ── Inventory sub-view ────────────────────────────────────────────────────

pub(super) struct InventoryView {
    pub(super) selected: usize,
    pub(super) last_grid_cols: usize,
}

impl Default for InventoryView {
    fn default() -> Self {
        InventoryView {
            selected: 0,
            last_grid_cols: 1,
        }
    }
}

// ── TavernState ───────────────────────────────────────────────────────────

pub struct TavernState {
    pub(super) focus: Focus,
    pub(super) current_view: View,
    pub(super) log_messages: Vec<(String, Style)>,
    pub(super) log_scroll: u16,
    pub(super) input: String,
    pub(super) cursor: usize,
    pub(super) frame_count: u64,
    pub(super) gathering_view: GatheringView,
    pub(super) refining_view: RefiningView,
    pub(super) crafting_view: CraftingView,
    pub(super) adventure_view: AdventureView,
    pub(super) inventory_view: InventoryView,
    pub(super) icon_cache: HashMap<String, Vec<Line<'static>>>,
    pub(super) loot_popups: Vec<LootPopup>,
    pub(super) quit_prompt_open: bool,
    pub(super) bottom_tab: BottomTab,
    pub(super) tile_graphics: TileGraphics,
    /// Queue of Kitty graphics tiles to draw after terminal.draw() completes.
    /// Each entry: (terminal_x, terminal_y, tile_kind, image_id, cols, rows).
    pub(super) pending_kitty_tiles: Vec<(u16, u16, crate::ui::MapTile, u32, u16, u16)>,
    /// Image IDs we've previously drawn this frame, used to delete leftovers
    /// (so when the active tile area shrinks/moves, stale images get cleared).
    pub(super) prev_frame_kitty_ids: Vec<u32>,
}

impl TavernState {
    pub(super) fn new() -> Self {
        let dim = Style::default().fg(ui::DIM);
        let warm = Style::default().fg(ui::WARM_WHITE);
        let gold = Style::default().fg(ui::GOLD);

        TavernState {
            focus: Focus::Terminal,
            current_view: View::Terminal,
            log_messages: vec![
                ("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".into(), dim),
                ("  Welcome to The Last Light.".into(), gold),
                ("  The Lantern burns steady above.".into(), warm),
                ("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".into(), dim),
                ("".into(), dim),
                (
                    "The hearth crackles softly. Outside, the Wilds press".into(),
                    warm,
                ),
                ("against the edges of the Lantern's reach.".into(), warm),
                ("".into(), dim),
                (
                    "A few travelers sit at the long table, nursing drinks".into(),
                    warm,
                ),
                ("and speaking in low voices about the roads.".into(), warm),
            ],
            log_scroll: 0,
            input: String::new(),
            cursor: 0,
            frame_count: 0,
            gathering_view: GatheringView::default(),
            refining_view: RefiningView::default(),
            crafting_view: CraftingView::default(),
            adventure_view: AdventureView::default(),
            inventory_view: InventoryView::default(),
            icon_cache: HashMap::new(),
            loot_popups: Vec::new(),
            quit_prompt_open: false,
            bottom_tab: BottomTab::Expeditions,
            tile_graphics: TileGraphics::new(),
            pending_kitty_tiles: Vec::new(),
            prev_frame_kitty_ids: Vec::new(),
        }
    }

    /// Look up a cached halfblock icon, rendering and caching on first use.
    pub(super) fn icon_for(&mut self, item_id: &str) -> Option<&Vec<Line<'static>>> {
        if !self.icon_cache.contains_key(item_id) {
            let rendered = ui::item_icon(item_id, ICON_WIDTH)?;
            self.icon_cache.insert(item_id.to_string(), rendered);
        }
        self.icon_cache.get(item_id)
    }

    pub(super) fn auto_scroll(&mut self, visible_height: usize) {
        let total = self.log_messages.len();
        if total > visible_height {
            self.log_scroll = (total - visible_height) as u16;
        }
    }
}

// ── Tick helpers ──────────────────────────────────────────────────────────

pub(super) fn tick_loot_popups(state: &mut TavernState) {
    for popup in state.loot_popups.iter_mut() {
        if popup.frames_remaining > 0 {
            popup.frames_remaining -= 1;
        }
    }
    state.loot_popups.retain(|p| p.frames_remaining > 0);
}

pub(super) fn tick_transition(state: &mut TavernState) {
    state.gathering_view.transition = match state.gathering_view.transition {
        Transition::EnteringLocation(n) if n > 0 => Transition::EnteringLocation(n - 1),
        Transition::EnteringLocation(_) => {
            state.gathering_view.screen = GatheringScreen::AtLocation;
            Transition::None
        }
        Transition::LeavingLocation(n) if n > 0 => Transition::LeavingLocation(n - 1),
        Transition::LeavingLocation(_) => {
            state.gathering_view.screen = GatheringScreen::Grounds;
            Transition::None
        }
        Transition::None => Transition::None,
    };
}

/// Returns the inventory contents sorted by category, then rarity, then name.
pub(super) fn sorted_inventory_items(
    game_state: &crate::game::GameState,
    data: &GameData,
) -> Vec<(game::ItemId, u32)> {
    let mut items: Vec<(game::ItemId, u32)> = game_state
        .inventory
        .items()
        .iter()
        .map(|(id, qty)| (id.clone(), *qty))
        .collect();

    items.sort_by(|(a_id, _), (b_id, _)| {
        let a_def = data.item_registry.get(a_id);
        let b_def = data.item_registry.get(b_id);
        let a_cat = a_def.map(|d| category_order(&d.category)).unwrap_or(99);
        let b_cat = b_def.map(|d| category_order(&d.category)).unwrap_or(99);
        a_cat
            .cmp(&b_cat)
            .then_with(|| {
                let a_r = a_def.map(|d| d.rarity).unwrap_or(game::Rarity::Common);
                let b_r = b_def.map(|d| d.rarity).unwrap_or(game::Rarity::Common);
                a_r.cmp(&b_r)
            })
            .then_with(|| {
                let a_n = a_def.map(|d| d.name.clone()).unwrap_or(a_id.0.clone());
                let b_n = b_def.map(|d| d.name.clone()).unwrap_or(b_id.0.clone());
                a_n.cmp(&b_n)
            })
    });
    items
}

fn category_order(cat: &game::ItemCategory) -> u8 {
    match cat {
        game::ItemCategory::RawMaterial => 0,
        game::ItemCategory::RefinedMaterial => 1,
        game::ItemCategory::Food => 2,
        game::ItemCategory::Drink => 3,
        game::ItemCategory::Weapon => 4,
        game::ItemCategory::Armor => 5,
        game::ItemCategory::Accessory => 6,
        game::ItemCategory::Reagent => 7,
        game::ItemCategory::Special => 8,
    }
}
