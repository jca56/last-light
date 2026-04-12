#![allow(dead_code)]

use rand::Rng;
use serde::{Deserialize, Serialize};

use super::adventurer::Adventurer;
use super::item::{ItemId, ItemRegistry};
use super::quest::{Encounter, Enemy, Quest};

// ── Party member (snapshot during adventure) ──────────────────────────────

/// Snapshot of an adventurer for the duration of an adventure. Persistent
/// changes (XP, downed status) are written back to the roster on completion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartyMember {
    /// Index into game_state.adventurers
    pub roster_idx: usize,
    pub name: String,
    pub current_hp: i32,
    pub max_hp: i32,
    pub strength: i32,
    pub dexterity: i32,
    pub intellect: i32,
    pub xp_earned: u32,
    pub defending: bool,
    pub downed: bool,
    /// Consumables carried into the adventure. Slots become None when used.
    #[serde(default)]
    pub consumables: Vec<Option<ItemId>>,
}

impl PartyMember {
    pub fn from_adventurer(roster_idx: usize, adv: &Adventurer, registry: &ItemRegistry) -> Self {
        let stats = adv.effective_stats(registry);
        PartyMember {
            roster_idx,
            name: adv.name.clone(),
            current_hp: stats.max_hp,
            max_hp: stats.max_hp,
            strength: stats.strength,
            dexterity: stats.dexterity,
            intellect: stats.intellect,
            xp_earned: 0,
            defending: false,
            downed: false,
            consumables: adv.consumables.clone(),
        }
    }

    /// Check if this party member has any usable consumables.
    pub fn has_consumables(&self) -> bool {
        self.consumables.iter().any(|s| s.is_some())
    }
}

// ── Combat state ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnemyInstance {
    pub name: String,
    pub max_hp: i32,
    pub current_hp: i32,
    pub strength: i32,
    pub xp_reward: u32,
}

impl EnemyInstance {
    pub fn from_template(t: &Enemy) -> Self {
        EnemyInstance {
            name: t.name.clone(),
            max_hp: t.max_hp,
            current_hp: t.max_hp,
            strength: t.strength,
            xp_reward: t.xp_reward,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CombatActor {
    /// Index into ActiveAdventure.party
    Party(usize),
    /// Index into CombatState.enemies
    Enemy(usize),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatState {
    pub enemies: Vec<EnemyInstance>,
    pub turn_order: Vec<CombatActor>,
    pub turn_idx: usize,
    pub round: u32,
    pub log: Vec<String>,
    /// Tracks whether this came from a Boss square (for marking quest end).
    pub is_boss: bool,
    /// Action selection state for current party member's turn (target selection).
    pub pending_target_for: Option<usize>,
}

impl CombatState {
    pub fn new(encounter: &Encounter, party_size: usize, is_boss: bool) -> Self {
        let enemies: Vec<EnemyInstance> =
            encounter.enemies.iter().map(EnemyInstance::from_template).collect();

        // Turn order: party first, then enemies
        let mut turn_order = Vec::new();
        for i in 0..party_size {
            turn_order.push(CombatActor::Party(i));
        }
        for i in 0..enemies.len() {
            turn_order.push(CombatActor::Enemy(i));
        }

        CombatState {
            enemies,
            turn_order,
            turn_idx: 0,
            round: 1,
            log: vec![format!("Round 1")],
            is_boss,
            pending_target_for: None,
        }
    }

    pub fn current_actor(&self) -> Option<CombatActor> {
        self.turn_order.get(self.turn_idx).copied()
    }

    /// Advance to the next actor whose entity is still alive. Loops to the
    /// next round when reaching the end of the order.
    pub fn advance_turn(&mut self, party: &[PartyMember]) {
        loop {
            self.turn_idx += 1;
            if self.turn_idx >= self.turn_order.len() {
                self.turn_idx = 0;
                self.round += 1;
                self.log.push(format!("Round {}", self.round));
                // Reset defending flag at start of new round
                // (handled by caller — we'd need &mut party here)
            }
            let actor = self.turn_order[self.turn_idx];
            if self.is_actor_alive(actor, party) {
                return;
            }
            // Safety: if all actors dead we still need to break out somehow
            if !self.any_alive(party) {
                return;
            }
        }
    }

    fn is_actor_alive(&self, actor: CombatActor, party: &[PartyMember]) -> bool {
        match actor {
            CombatActor::Party(i) => party.get(i).map(|p| !p.downed).unwrap_or(false),
            CombatActor::Enemy(i) => {
                self.enemies.get(i).map(|e| e.current_hp > 0).unwrap_or(false)
            }
        }
    }

    fn any_alive(&self, party: &[PartyMember]) -> bool {
        party.iter().any(|p| !p.downed)
            || self.enemies.iter().any(|e| e.current_hp > 0)
    }

    pub fn party_won(&self) -> bool {
        self.enemies.iter().all(|e| e.current_hp <= 0)
    }

    pub fn party_lost(&self, party: &[PartyMember]) -> bool {
        party.iter().all(|p| p.downed)
    }
}

// ── Adventure state ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AdventureState {
    /// Walking the map.
    Exploring,
    /// In the middle of a combat encounter.
    InCombat(CombatState),
    /// Adventure has ended; ready to be cleared by the caller.
    Complete { success: bool },
}

// ── Active adventure ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveAdventure {
    pub quest_id: String,
    pub party: Vec<PartyMember>,
    pub position: (u32, u32),
    /// Squares the party has revealed via fog of war (includes current pos and adjacents).
    pub revealed: Vec<(u32, u32)>,
    /// Squares whose effect has already been triggered (so we don't re-loot).
    pub completed_squares: Vec<(u32, u32)>,
    pub state: AdventureState,
    pub log: Vec<String>,
    pub pending_loot: Vec<(ItemId, u32)>,
    pub pending_gold: u32,
    pub pending_xp: u32,
}

#[allow(dead_code)]
impl ActiveAdventure {
    pub fn new(quest: &Quest, party: Vec<PartyMember>) -> Self {
        let mut adv = ActiveAdventure {
            quest_id: quest.id.clone(),
            party,
            position: quest.map.start,
            revealed: Vec::new(),
            completed_squares: Vec::new(),
            state: AdventureState::Exploring,
            log: vec![format!("Entered {}", quest.name)],
            pending_loot: Vec::new(),
            pending_gold: 0,
            pending_xp: 0,
        };
        adv.reveal_around(quest.map.start.0, quest.map.start.1, quest.map.width, quest.map.height);
        adv
    }

    pub fn reveal_around(&mut self, x: u32, y: u32, w: u32, h: u32) {
        let xi = x as i32;
        let yi = y as i32;
        for (dx, dy) in [(0, 0), (-1, 0), (1, 0), (0, -1), (0, 1)] {
            let nx = xi + dx;
            let ny = yi + dy;
            if nx >= 0 && ny >= 0 && (nx as u32) < w && (ny as u32) < h {
                let p = (nx as u32, ny as u32);
                if !self.revealed.contains(&p) {
                    self.revealed.push(p);
                }
            }
        }
    }

    pub fn is_revealed(&self, x: u32, y: u32) -> bool {
        self.revealed.contains(&(x, y))
    }

    pub fn is_completed(&self, x: u32, y: u32) -> bool {
        self.completed_squares.contains(&(x, y))
    }

    pub fn mark_completed(&mut self, x: u32, y: u32) {
        if !self.completed_squares.contains(&(x, y)) {
            self.completed_squares.push((x, y));
        }
    }

    /// Try to move the party in the given direction. Returns true if moved.
    pub fn try_move(&mut self, dx: i32, dy: i32, w: u32, h: u32) -> bool {
        if !matches!(self.state, AdventureState::Exploring) {
            return false;
        }
        let nx = self.position.0 as i32 + dx;
        let ny = self.position.1 as i32 + dy;
        if nx < 0 || ny < 0 || (nx as u32) >= w || (ny as u32) >= h {
            return false;
        }
        self.position = (nx as u32, ny as u32);
        self.reveal_around(self.position.0, self.position.1, w, h);
        true
    }

    pub fn add_log(&mut self, msg: impl Into<String>) {
        self.log.push(msg.into());
    }
}

// ── Combat actions ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombatAction {
    Attack,
    Defend,
    Flee,
}

/// Resolve a single party member's action. Mutates the combat state and party.
/// `target` is required for Attack (index into combat.enemies).
pub fn resolve_party_action(
    party: &mut [PartyMember],
    combat: &mut CombatState,
    actor_idx: usize,
    action: CombatAction,
    target: Option<usize>,
) -> bool {
    let Some(actor) = party.get_mut(actor_idx) else {
        return false;
    };
    if actor.downed {
        return false;
    }
    let actor_name = actor.name.clone();

    match action {
        CombatAction::Attack => {
            let Some(t_idx) = target else {
                return false;
            };
            let Some(enemy) = combat.enemies.get_mut(t_idx) else {
                return false;
            };
            if enemy.current_hp <= 0 {
                return false;
            }
            // Damage = strength (Phase 3a — gear is already in stats; armor reduction later)
            let damage = actor.strength.max(1);
            enemy.current_hp = (enemy.current_hp - damage).max(0);
            combat
                .log
                .push(format!("{} hits {} for {} damage", actor_name, enemy.name, damage));
            if enemy.current_hp == 0 {
                combat.log.push(format!("{} is defeated!", enemy.name));
            }
            actor.defending = false;
        }
        CombatAction::Defend => {
            actor.defending = true;
            combat.log.push(format!("{} braces", actor_name));
        }
        CombatAction::Flee => {
            // Party-wide DEX check: average DEX vs DC 10
            let avg_dex: i32 = party
                .iter()
                .filter(|p| !p.downed)
                .map(|p| p.dexterity)
                .sum::<i32>()
                / (party.iter().filter(|p| !p.downed).count().max(1) as i32);
            let mut rng = rand::rng();
            let roll = rng.random_range(1..=20);
            let total = roll + avg_dex;
            combat.log.push(format!("Flee check: {}+{}={} vs DC 10", roll, avg_dex, total));
            if total >= 10 {
                combat.log.push("The party escapes!".into());
                // Mark combat as ended without victory — set all enemies to "fled" by zeroing HP
                for e in combat.enemies.iter_mut() {
                    e.current_hp = 0;
                }
            } else {
                combat.log.push("The escape fails!".into());
            }
        }
    }
    true
}

/// Resolve all enemy turns (all enemies act, party damaged accordingly).
pub fn resolve_enemy_turn(party: &mut [PartyMember], combat: &mut CombatState) {
    let mut rng = rand::rng();
    for enemy in combat.enemies.iter() {
        if enemy.current_hp <= 0 {
            continue;
        }
        // Pick a random non-downed party member
        let alive_indices: Vec<usize> = party
            .iter()
            .enumerate()
            .filter(|(_, p)| !p.downed)
            .map(|(i, _)| i)
            .collect();
        if alive_indices.is_empty() {
            return;
        }
        let target_idx = alive_indices[rng.random_range(0..alive_indices.len())];
        let target = &mut party[target_idx];
        let mut damage = enemy.strength.max(1);
        if target.defending {
            damage = (damage / 2).max(1);
        }
        target.current_hp = (target.current_hp - damage).max(0);
        let target_name = target.name.clone();
        combat
            .log
            .push(format!("{} hits {} for {} damage", enemy.name, target_name, damage));
        if target.current_hp == 0 {
            target.downed = true;
            combat.log.push(format!("{} falls!", target_name));
        }
    }
    // Reset defending flags after enemy phase
    for p in party.iter_mut() {
        p.defending = false;
    }
}
