use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::item::{ItemCategory, ItemId, ItemRegistry};

/// Stores item quantities. Serialized as part of the save file.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Inventory {
    items: HashMap<ItemId, u32>,
}

#[allow(dead_code)]
impl Inventory {
    pub fn add(&mut self, id: &ItemId, amount: u32) {
        *self.items.entry(id.clone()).or_insert(0) += amount;
    }

    /// Removes `amount` of an item. Returns false (and removes nothing) if insufficient.
    pub fn remove(&mut self, id: &ItemId, amount: u32) -> bool {
        let Some(current) = self.items.get(id) else {
            return amount == 0;
        };
        if *current < amount {
            return false;
        }
        let new = current - amount;
        if new == 0 {
            self.items.remove(id);
        } else {
            self.items.insert(id.clone(), new);
        }
        true
    }

    pub fn count(&self, id: &ItemId) -> u32 {
        self.items.get(id).copied().unwrap_or(0)
    }

    pub fn has(&self, id: &ItemId, amount: u32) -> bool {
        self.count(id) >= amount
    }

    pub fn has_all(&self, requirements: &[(ItemId, u32)]) -> bool {
        requirements.iter().all(|(id, qty)| self.has(id, *qty))
    }

    pub fn items(&self) -> &HashMap<ItemId, u32> {
        &self.items
    }

    pub fn items_in_category<'a>(
        &'a self,
        registry: &'a ItemRegistry,
        cat: &ItemCategory,
    ) -> Vec<(&'a ItemId, u32)> {
        self.items
            .iter()
            .filter(|(id, _)| {
                registry
                    .get(id)
                    .map(|def| &def.category == cat)
                    .unwrap_or(false)
            })
            .map(|(id, qty)| (id, *qty))
            .collect()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}
