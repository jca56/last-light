//! Shared equip / unequip / cycle helpers used by the roster and party-setup
//! screens. Lives in its own module to break the bidirectional dependency
//! that would otherwise exist between `roster` and `party_setup`.

use crate::game::{self, GameData, GameState};

/// Get all items from inventory that are compatible with the given equipment
/// or consumable slot. Returns (ItemId, quantity) pairs sorted by name.
pub(crate) fn compatible_items_for_slot(
    game_state: &GameState,
    slot: usize,
) -> Vec<(game::ItemId, u32)> {
    let registry = game::ItemRegistry::new();

    if slot <= 2 {
        // Equipment slot: filter by Weapon/Armor/Accessory
        let target_category = match slot {
            0 => game::ItemCategory::Weapon,
            1 => game::ItemCategory::Armor,
            2 => game::ItemCategory::Accessory,
            _ => return Vec::new(),
        };
        let mut items: Vec<(game::ItemId, u32)> = game_state
            .inventory
            .items()
            .iter()
            .filter_map(|(id, qty)| {
                if *qty == 0 {
                    return None;
                }
                let def = registry.get(id)?;
                if def.category == target_category {
                    Some((id.clone(), *qty))
                } else {
                    None
                }
            })
            .collect();
        items.sort_by(|(a, _), (b, _)| {
            let a_name = registry.get(a).map(|d| d.name.clone()).unwrap_or_default();
            let b_name = registry.get(b).map(|d| d.name.clone()).unwrap_or_default();
            a_name.cmp(&b_name)
        });
        items
    } else {
        // Consumable slot: show consumable items
        let mut items: Vec<(game::ItemId, u32)> = game_state
            .inventory
            .items()
            .iter()
            .filter_map(|(id, qty)| {
                if *qty == 0 {
                    return None;
                }
                let def = registry.get(id)?;
                if def.tags.iter().any(|t| t == "consumable") {
                    Some((id.clone(), *qty))
                } else {
                    None
                }
            })
            .collect();
        items.sort_by(|(a, _), (b, _)| {
            let a_name = registry.get(a).map(|d| d.name.clone()).unwrap_or_default();
            let b_name = registry.get(b).map(|d| d.name.clone()).unwrap_or_default();
            a_name.cmp(&b_name)
        });
        items
    }
}

/// Equip a specific gear item from the picker. Unequips current item first.
pub(super) fn equip_item_from_picker(
    game_state: &mut GameState,
    adv_idx: usize,
    equip_slot: usize,
    item_id: &game::ItemId,
) {
    unequip_slot(game_state, adv_idx, equip_slot);
    if !game_state.inventory.remove(item_id, 1) {
        return;
    }
    let Some(adv) = game_state.adventurers.get_mut(adv_idx) else {
        game_state.inventory.add(item_id, 1);
        return;
    };
    match equip_slot {
        0 => adv.equipment.weapon = Some(item_id.clone()),
        1 => adv.equipment.armor = Some(item_id.clone()),
        2 => adv.equipment.accessory = Some(item_id.clone()),
        _ => {
            game_state.inventory.add(item_id, 1);
        }
    }
}

/// Equip a consumable item from the picker into a consumable slot.
pub(super) fn equip_consumable_from_picker(
    game_state: &mut GameState,
    adv_idx: usize,
    consumable_slot: usize,
    item_id: &game::ItemId,
) {
    unequip_consumable(game_state, adv_idx, consumable_slot);
    if !game_state.inventory.remove(item_id, 1) {
        return;
    }
    let Some(adv) = game_state.adventurers.get_mut(adv_idx) else {
        game_state.inventory.add(item_id, 1);
        return;
    };
    adv.init_consumable_slots();
    if consumable_slot < adv.consumables.len() {
        adv.consumables[consumable_slot] = Some(item_id.clone());
    } else {
        game_state.inventory.add(item_id, 1);
    }
}

/// Cycle through equippable items in inventory for the given equip slot.
/// Each press equips the next compatible item (returning the previous to inventory).
pub(super) fn cycle_equipment(
    game_state: &mut GameState,
    data: &GameData,
    adv_idx: usize,
    equip_slot: usize,
) {
    let target_category = match equip_slot {
        0 => game::ItemCategory::Weapon,
        1 => game::ItemCategory::Armor,
        2 => game::ItemCategory::Accessory,
        _ => return,
    };

    // First: safely unequip current item (returns it to inventory via take())
    unequip_slot(game_state, adv_idx, equip_slot);

    // Now find compatible items from inventory (which now includes the returned item)
    let compatible: Vec<game::ItemId> = game_state
        .inventory
        .items()
        .iter()
        .filter_map(|(id, qty)| {
            if *qty == 0 {
                return None;
            }
            let def = data.item_registry.get(id)?;
            if def.category == target_category {
                Some(id.clone())
            } else {
                None
            }
        })
        .collect();

    if compatible.is_empty() {
        return;
    }

    // Pick the first compatible item and equip it
    let next_id = compatible[0].clone();
    equip_item_from_picker(game_state, adv_idx, equip_slot, &next_id);
}

pub(super) fn unequip_slot(game_state: &mut GameState, adv_idx: usize, equip_slot: usize) {
    let Some(adv) = game_state.adventurers.get_mut(adv_idx) else {
        return;
    };
    let removed = match equip_slot {
        0 => adv.equipment.weapon.take(),
        1 => adv.equipment.armor.take(),
        2 => adv.equipment.accessory.take(),
        _ => None,
    };
    if let Some(id) = removed {
        game_state.inventory.add(&id, 1);
    }
}

pub(super) fn cycle_consumable(
    game_state: &mut GameState,
    data: &GameData,
    adv_idx: usize,
    slot: usize,
) {
    let compatible: Vec<game::ItemId> = game_state
        .inventory
        .items()
        .iter()
        .filter_map(|(id, qty)| {
            if *qty == 0 {
                return None;
            }
            let def = data.item_registry.get(id)?;
            if def.category == game::ItemCategory::Consumable {
                Some(id.clone())
            } else {
                None
            }
        })
        .collect();

    if compatible.is_empty() {
        return;
    }

    let Some(adv) = game_state.adventurers.get_mut(adv_idx) else {
        return;
    };
    adv.init_consumable_slots();

    let current_id = adv.consumables.get(slot).cloned().flatten();

    let next_id = match &current_id {
        None => compatible[0].clone(),
        Some(curr) => {
            let pos = compatible.iter().position(|id| id == curr);
            match pos {
                Some(i) => compatible[(i + 1) % compatible.len()].clone(),
                None => compatible[0].clone(),
            }
        }
    };

    if let Some(prev) = current_id {
        game_state.inventory.add(&prev, 1);
    }

    if !game_state.inventory.remove(&next_id, 1) {
        return;
    }

    let Some(adv) = game_state.adventurers.get_mut(adv_idx) else {
        return;
    };
    if slot < adv.consumables.len() {
        adv.consumables[slot] = Some(next_id);
    }
}

pub(super) fn unequip_consumable(game_state: &mut GameState, adv_idx: usize, slot: usize) {
    let Some(adv) = game_state.adventurers.get_mut(adv_idx) else {
        return;
    };
    adv.init_consumable_slots();
    let removed = if slot < adv.consumables.len() {
        adv.consumables[slot].take()
    } else {
        None
    };
    if let Some(id) = removed {
        game_state.inventory.add(&id, 1);
    }
}
