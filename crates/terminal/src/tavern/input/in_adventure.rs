//! Adventures → In-Adventure (map exploration) sub-screen input.

use crossterm::event::KeyCode;
use ratatui::style::Style;

use super::super::state::{AdventureScreen, TavernState};
use crate::game::{self, AdventureState, GameData, GameState};
use crate::ui;

pub(super) fn handle_in_adventure(
    state: &mut TavernState,
    game_state: &mut GameState,
    data: &GameData,
    key: KeyCode,
) {
    // Get map dimensions (dungeon floor or static quest)
    let (w, h) = {
        let Some(adventure) = game_state.active_adventure.as_ref() else {
            return;
        };
        if let Some(dm) = adventure.active_map() {
            (dm.width, dm.height)
        } else if let Some(q) = data.quest(&adventure.quest_id) {
            (q.map.width, q.map.height)
        } else {
            return;
        }
    };

    match key {
        KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right => {
            let (dx, dy) = match key {
                KeyCode::Up => (0i32, -1i32),
                KeyCode::Down => (0, 1),
                KeyCode::Left => (-1, 0),
                KeyCode::Right => (1, 0),
                _ => unreachable!(),
            };
            let Some(adventure) = game_state.active_adventure.as_mut() else {
                return;
            };
            if !adventure.try_move(dx, dy, w, h) {
                return;
            }
            // Auto-trigger dangerous squares (traps, combat, boss)
            trigger_auto_square(state, game_state, data);
        }
        KeyCode::Enter => {
            // Manually interact with the current square (chests, rest, ladders)
            trigger_manual_square(state, game_state, data);
        }
        KeyCode::Char('i') | KeyCode::Char('I') => {
            // Use a consumable item out of combat
            use_consumable_out_of_combat(state, game_state, data);
        }
        _ => {}
    }
}

/// Use a consumable from the first party member who has one.
/// Shows the consumable picker (reuses combat consumable picking state).
fn use_consumable_out_of_combat(
    state: &mut TavernState,
    game_state: &mut GameState,
    data: &GameData,
) {
    let Some(adventure) = game_state.active_adventure.as_mut() else {
        return;
    };

    // Find any party member with consumables
    let has_any = adventure
        .party
        .iter()
        .any(|m| !m.downed && m.has_consumables());
    if !has_any {
        state.log_messages.push((
            "No consumables available.".into(),
            Style::default().fg(ui::DIM),
        ));
        state.auto_scroll(20);
        return;
    }

    // Find first non-downed member with consumables
    let member_idx = adventure
        .party
        .iter()
        .position(|m| !m.downed && m.has_consumables());
    let Some(mi) = member_idx else {
        return;
    };

    // Use the first available consumable
    let consumable_idx = adventure.party[mi]
        .consumables
        .iter()
        .position(|s| s.is_some());
    let Some(ci) = consumable_idx else {
        return;
    };
    let item_id = adventure.party[mi].consumables[ci].take();
    let Some(id) = item_id else {
        return;
    };

    // Apply the consumable effect
    let item_name = data
        .item_registry
        .get(&id)
        .map(|d| d.name.clone())
        .unwrap_or_else(|| id.0.clone());
    let member_name = adventure.party[mi].name.clone();

    // Check tags for effect
    let def = data.item_registry.get(&id);
    let is_healing = def
        .map(|d| d.tags.iter().any(|t| t == "healing"))
        .unwrap_or(false);

    if is_healing {
        // Heal the most damaged non-downed party member
        let target_idx = adventure
            .party
            .iter()
            .enumerate()
            .filter(|(_, m)| !m.downed && m.current_hp < m.max_hp)
            .min_by_key(|(_, m)| m.current_hp)
            .map(|(i, _)| i);
        if let Some(ti) = target_idx {
            let heal_amount = if id.0.contains("minor") { 8 } else { 15 };
            let target = &mut adventure.party[ti];
            let old_hp = target.current_hp;
            target.current_hp = (target.current_hp + heal_amount).min(target.max_hp);
            let healed = target.current_hp - old_hp;
            let target_name = target.name.clone();
            adventure.add_log(format!(
                "{} uses {} — {} heals {} HP!",
                member_name, item_name, target_name, healed
            ));
        }
    } else {
        adventure.add_log(format!("{} uses {}.", member_name, item_name));
    }
}

/// Helper: get the current square from the active map.
fn get_current_square(
    adventure: &game::ActiveAdventure,
    data: &GameData,
) -> Option<game::SquareKind> {
    let map = if let Some(dm) = adventure.active_map() {
        dm
    } else if let Some(q) = data.quest(&adventure.quest_id) {
        &q.map
    } else {
        return None;
    };
    let (x, y) = adventure.position;
    if adventure.is_completed(x, y) {
        return None;
    }
    map.get(x, y).cloned()
}

/// Auto-triggered when stepping onto a tile: traps and combat only.
fn trigger_auto_square(state: &mut TavernState, game_state: &mut GameState, data: &GameData) {
    let Some(adventure) = game_state.active_adventure.as_mut() else {
        return;
    };
    let Some(square) = get_current_square(adventure, data) else {
        return;
    };
    let (x, y) = adventure.position;

    match square {
        game::SquareKind::Trap { damage, dex_dc } => {
            use rand::Rng;
            let mut rng = rand::rng();
            let mut hits = 0;
            for member in adventure.party.iter_mut() {
                if member.downed {
                    continue;
                }
                let roll = rng.random_range(1..=20);
                let total = roll + member.dexterity;
                if total < dex_dc {
                    member.current_hp = (member.current_hp - damage).max(0);
                    if member.current_hp == 0 {
                        member.downed = true;
                    }
                    hits += 1;
                }
            }
            adventure.add_log(format!("Trap! {} party member(s) hit.", hits));
            adventure.mark_completed(x, y);
            if adventure.party.iter().all(|m| m.downed) {
                adventure.state = AdventureState::Complete { success: false };
                state.adventure_view.screen = AdventureScreen::Results;
                state.log_messages.push((
                    "The party fell to a trap.".into(),
                    Style::default().fg(ui::EMBER),
                ));
                state.auto_scroll(20);
            }
        }
        game::SquareKind::Combat { ref encounter_id }
        | game::SquareKind::Boss { ref encounter_id } => {
            let is_boss = matches!(square, game::SquareKind::Boss { .. });
            let encounter_id = encounter_id.clone();
            let encounter = if adventure.dungeon_id.is_some() {
                let dungeon = adventure
                    .dungeon_id
                    .as_ref()
                    .and_then(|did| data.dungeons.iter().find(|d| d.id == *did));
                if is_boss {
                    dungeon.map(|d| {
                        let scale = 1.0 + 0.15 * adventure.current_floor as f64;
                        game::dungeon::scale_boss(&d.boss, scale)
                    })
                } else {
                    dungeon.map(|d| {
                        let pool_idx = (adventure.current_floor as usize)
                            .min(d.enemy_pools.len().saturating_sub(1));
                        let pool = &d.enemy_pools[pool_idx];
                        let scale = 1.0 + 0.15 * adventure.current_floor as f64;
                        use rand::Rng;
                        let mut rng = rand::rng();
                        let count = rng.random_range(1..=3u32).min(pool.enemies.len() as u32);
                        let mut enemies = Vec::new();
                        for _ in 0..count {
                            let template =
                                &pool.enemies[rng.random_range(0..pool.enemies.len())];
                            enemies.push(game::dungeon::scale_enemy(template, scale));
                        }
                        game::Encounter {
                            id: encounter_id.clone(),
                            enemies,
                        }
                    })
                }
            } else {
                data.encounter(&encounter_id).cloned()
            };
            if let Some(enc) = encounter {
                let combat = game::CombatState::new(&enc, adventure.party.len(), is_boss);
                adventure.state = AdventureState::InCombat(combat);
                state.adventure_view.screen = AdventureScreen::Combat;
                state.adventure_view.combat_action_idx = 0;
                state.adventure_view.combat_target_idx = 0;
                state.adventure_view.combat_picking_target = false;
            }
        }
        _ => {} // Everything else requires Enter
    }
}

/// Manually triggered with Enter: chests, rest spots, ladders.
fn trigger_manual_square(state: &mut TavernState, game_state: &mut GameState, data: &GameData) {
    let Some(adventure) = game_state.active_adventure.as_mut() else {
        return;
    };
    let Some(square) = get_current_square(adventure, data) else {
        return;
    };
    let (x, y) = adventure.position;

    match square {
        game::SquareKind::Treasure { gold, items } => {
            adventure.pending_gold += gold;
            for (id, qty) in &items {
                adventure.pending_loot.push((id.clone(), *qty));
            }
            adventure.add_log(format!("Found {} gold and {} item(s).", gold, items.len()));
            adventure.mark_completed(x, y);
        }
        game::SquareKind::Rest => {
            for member in adventure.party.iter_mut() {
                if !member.downed {
                    member.current_hp = member.max_hp;
                }
            }
            adventure.add_log("The party rests and recovers.");
            adventure.mark_completed(x, y);
        }
        game::SquareKind::LadderDown => {
            state.tile_graphics.cleanup_all();
            if let Some(did) = adventure.dungeon_id.clone() {
                if let Some(dungeon) = data.dungeons.iter().find(|d| d.id == did) {
                    adventure.descend_floor(dungeon);
                }
            }
        }
        game::SquareKind::LadderUp => {
            adventure.pending_xp = adventure.pending_xp / 2;
            adventure.add_log("The party retreats up the ladder.");
            adventure.state = AdventureState::Complete { success: true };
            state.adventure_view.screen = AdventureScreen::Results;
            state.tile_graphics.cleanup_all();
        }
        _ => {} // Traps/combat are auto, empty does nothing
    }
}
