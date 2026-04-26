//! Adventures → Combat sub-screen input.

use crossterm::event::KeyCode;
use ratatui::style::Style;

use super::super::state::{AdventureScreen, TavernState};
use crate::game::{self, AdventureState, CombatAction, CombatActor, GameData, GameState};
use crate::ui;

pub(super) fn handle_combat(
    state: &mut TavernState,
    game_state: &mut GameState,
    data: &GameData,
    key: KeyCode,
) {
    let Some(adventure) = game_state.active_adventure.as_mut() else {
        return;
    };
    let AdventureState::InCombat(combat) = &mut adventure.state else {
        return;
    };

    // Only allow input on a party turn
    let current = combat.current_actor();
    let CombatActor::Party(actor_idx) = (match current {
        Some(a) => a,
        None => return,
    }) else {
        return;
    };

    if state.adventure_view.combat_picking_target {
        // Target picker
        match key {
            KeyCode::Up => {
                if state.adventure_view.combat_target_idx > 0 {
                    state.adventure_view.combat_target_idx -= 1;
                }
            }
            KeyCode::Down => {
                if state.adventure_view.combat_target_idx + 1 < combat.enemies.len() {
                    state.adventure_view.combat_target_idx += 1;
                }
            }
            KeyCode::Enter => {
                // Find a valid target (skip dead enemies)
                let target = state.adventure_view.combat_target_idx;
                if combat
                    .enemies
                    .get(target)
                    .map(|e| e.current_hp > 0)
                    .unwrap_or(false)
                {
                    game::resolve_party_action(
                        &mut adventure.party,
                        combat,
                        actor_idx,
                        CombatAction::Attack,
                        Some(target),
                    );
                    state.adventure_view.combat_picking_target = false;
                    state.adventure_view.combat_action_idx = 0;
                    advance_combat_after_action(state, game_state, data);
                }
            }
            KeyCode::Esc => {
                state.adventure_view.combat_picking_target = false;
            }
            _ => {}
        }
        return;
    }

    if state.adventure_view.combat_picking_consumable {
        // Consumable picker
        let member = &adventure.party[actor_idx];
        let slot_count = member.consumables.len();
        match key {
            KeyCode::Up => {
                if state.adventure_view.combat_consumable_idx > 0 {
                    state.adventure_view.combat_consumable_idx -= 1;
                }
            }
            KeyCode::Down => {
                if state.adventure_view.combat_consumable_idx + 1 < slot_count {
                    state.adventure_view.combat_consumable_idx += 1;
                }
            }
            KeyCode::Enter => {
                let ci = state.adventure_view.combat_consumable_idx;
                let item_id = member.consumables.get(ci).cloned().flatten();
                if let Some(id) = item_id {
                    resolve_consumable_use(adventure, actor_idx, ci, &id, data);
                    state.adventure_view.combat_picking_consumable = false;
                    state.adventure_view.combat_action_idx = 0;
                    advance_combat_after_action(state, game_state, data);
                }
            }
            KeyCode::Esc => {
                state.adventure_view.combat_picking_consumable = false;
            }
            _ => {}
        }
        return;
    }

    // Check if current member has any consumables
    let has_items = adventure.party.get(actor_idx).map(|m| m.has_consumables()).unwrap_or(false);
    let max_action = if has_items { 3 } else { 2 };

    // Action menu
    match key {
        KeyCode::Up => {
            if state.adventure_view.combat_action_idx > 0 {
                state.adventure_view.combat_action_idx -= 1;
            }
        }
        KeyCode::Down => {
            if state.adventure_view.combat_action_idx < max_action {
                state.adventure_view.combat_action_idx += 1;
            }
        }
        KeyCode::Enter => {
            match state.adventure_view.combat_action_idx {
                0 => {
                    // Attack — pick target
                    let first_alive = combat.enemies.iter().position(|e| e.current_hp > 0);
                    if let Some(idx) = first_alive {
                        state.adventure_view.combat_target_idx = idx;
                        state.adventure_view.combat_picking_target = true;
                    }
                }
                1 => {
                    game::resolve_party_action(
                        &mut adventure.party,
                        combat,
                        actor_idx,
                        CombatAction::Defend,
                        None,
                    );
                    advance_combat_after_action(state, game_state, data);
                }
                2 => {
                    game::resolve_party_action(
                        &mut adventure.party,
                        combat,
                        actor_idx,
                        CombatAction::Flee,
                        None,
                    );
                    advance_combat_after_action(state, game_state, data);
                }
                3 => {
                    // Use Item — open consumable picker
                    state.adventure_view.combat_consumable_idx = 0;
                    state.adventure_view.combat_picking_consumable = true;
                }
                _ => {}
            }
        }
        _ => {}
    }
}

/// Apply a consumable effect to the acting party member, consume the item,
/// and log the result.
fn resolve_consumable_use(
    adventure: &mut game::ActiveAdventure,
    actor_idx: usize,
    consumable_slot: usize,
    item_id: &game::ItemId,
    data: &GameData,
) {
    let Some(def) = data.item_registry.get(item_id) else {
        return;
    };
    let Some(effect) = &def.properties.consumable_effect else {
        return;
    };
    let member = &mut adventure.party[actor_idx];
    let member_name = member.name.clone();
    let item_name = def.name.clone();

    match effect {
        game::ConsumableEffect::Heal(amount) => {
            let old_hp = member.current_hp;
            member.current_hp = (member.current_hp + amount).min(member.max_hp);
            let healed = member.current_hp - old_hp;
            if let game::AdventureState::InCombat(combat) = &mut adventure.state {
                combat.log.push(format!(
                    "{} uses {} — heals {} HP",
                    member_name, item_name, healed
                ));
            }
        }
        game::ConsumableEffect::BoostStrength(amount) => {
            member.strength += amount;
            if let game::AdventureState::InCombat(combat) = &mut adventure.state {
                combat.log.push(format!(
                    "{} uses {} — STR +{}",
                    member_name, item_name, amount
                ));
            }
        }
        game::ConsumableEffect::BoostDexterity(amount) => {
            member.dexterity += amount;
            if let game::AdventureState::InCombat(combat) = &mut adventure.state {
                combat.log.push(format!(
                    "{} uses {} — DEX +{}",
                    member_name, item_name, amount
                ));
            }
        }
        game::ConsumableEffect::BoostIntellect(amount) => {
            member.intellect += amount;
            if let game::AdventureState::InCombat(combat) = &mut adventure.state {
                combat.log.push(format!(
                    "{} uses {} — INT +{}",
                    member_name, item_name, amount
                ));
            }
        }
    }

    // Remove from member's consumable slots
    if consumable_slot < member.consumables.len() {
        member.consumables[consumable_slot] = None;
    }
}

/// After a party action resolves, advance the turn order. If all party members
/// have acted this round, run the enemy turn. Check win/loss conditions.
fn advance_combat_after_action(
    state: &mut TavernState,
    game_state: &mut GameState,
    data: &GameData,
) {
    let Some(adventure) = game_state.active_adventure.as_mut() else {
        return;
    };
    let AdventureState::InCombat(combat) = &mut adventure.state else {
        return;
    };

    // Check immediate win
    if combat.party_won() {
        end_combat_victory(state, game_state, data);
        return;
    }
    if combat.party_lost(&adventure.party) {
        end_combat_loss(state, game_state);
        return;
    }

    combat.advance_turn(&adventure.party);

    // If next actor is enemy, run all remaining enemy turns until party turn or end
    loop {
        let cur = combat.current_actor();
        match cur {
            Some(CombatActor::Enemy(_)) => {
                game::resolve_enemy_turn(&mut adventure.party, combat);
                // Skip to next party turn
                while matches!(combat.current_actor(), Some(CombatActor::Enemy(_))) {
                    combat.advance_turn(&adventure.party);
                    if combat.party_lost(&adventure.party) || combat.party_won() {
                        break;
                    }
                }
                if combat.party_won() {
                    end_combat_victory(state, game_state, data);
                    return;
                }
                if combat.party_lost(&adventure.party) {
                    end_combat_loss(state, game_state);
                    return;
                }
                // Skip downed party members
                while let Some(CombatActor::Party(i)) = combat.current_actor() {
                    if adventure
                        .party
                        .get(i)
                        .map(|p| p.downed)
                        .unwrap_or(true)
                    {
                        combat.advance_turn(&adventure.party);
                    } else {
                        break;
                    }
                }
                break;
            }
            Some(CombatActor::Party(i)) => {
                // Skip downed party members
                if adventure
                    .party
                    .get(i)
                    .map(|p| p.downed)
                    .unwrap_or(true)
                {
                    combat.advance_turn(&adventure.party);
                    continue;
                }
                break;
            }
            None => break,
        }
    }

    state.adventure_view.combat_action_idx = 0;
}

fn cleanup_combat_portraits(state: &TavernState) {
    state.tile_graphics.cleanup_enemy_portraits();
}

fn end_combat_victory(state: &mut TavernState, game_state: &mut GameState, data: &GameData) {
    let Some(adventure) = game_state.active_adventure.as_mut() else {
        return;
    };
    let AdventureState::InCombat(combat) = &adventure.state else {
        return;
    };

    let was_boss = combat.is_boss;
    let xp_gained: u32 = combat.enemies.iter().map(|e| e.xp_reward).sum();
    adventure.pending_xp += xp_gained;
    adventure.add_log(format!("Victory! +{} XP", xp_gained));

    // Mark current square as completed
    let pos = adventure.position;
    adventure.mark_completed(pos.0, pos.1);

    // Clean up enemy portrait before leaving combat screen
    cleanup_combat_portraits(state);

    // Return to exploring
    adventure.state = AdventureState::Exploring;
    state.adventure_view.screen = AdventureScreen::InAdventure;

    if was_boss {
        // Apply quest completion rewards into pending pool
        if let Some(quest) = data.quest(&adventure.quest_id) {
            adventure.pending_gold += quest.completion_gold;
            adventure.pending_xp += quest.xp_reward;
            for (id, qty) in &quest.completion_loot {
                adventure.pending_loot.push((id.clone(), *qty));
            }
        }
        adventure.state = AdventureState::Complete { success: true };
        state.adventure_view.screen = AdventureScreen::Results;
        state.log_messages.push((
            "The party has triumphed!".into(),
            Style::default().fg(ui::GOLD),
        ));
        state.auto_scroll(20);
    }
}

fn end_combat_loss(state: &mut TavernState, game_state: &mut GameState) {
    let Some(adventure) = game_state.active_adventure.as_mut() else {
        return;
    };
    cleanup_combat_portraits(state);
    adventure.state = AdventureState::Complete { success: false };
    state.adventure_view.screen = AdventureScreen::Results;
    state.log_messages.push((
        "The party falls in battle.".into(),
        Style::default().fg(ui::EMBER),
    ));
    state.auto_scroll(20);
}
