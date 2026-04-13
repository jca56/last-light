# The Last Light - Roadmap

## Vision

A terminal-based innkeeper idle game. You run The Last Light tavern — gathering resources, refining materials, crafting goods and gear, and sending adventurers on quests. Designed for the tab-in, queue actions, tab-out workflow of someone who lives in the terminal.

---

## Core Loop

```
Gathering --> Refining --> Crafting --> Equipping / Stocking
    ^                                        |
    <---- Adventures (loot + unlocks) <------+
                    |
              Tavern Income (gold from visitors)
                    |
              Upgrades (better gathering, new recipes, etc.)
```

---

## The Five Pillars

### 1. Gathering

Assign gathering tasks to locations. Pick a duration, start the task, collect when done.

**Locations** (unlocked via reputation):
- Whispering Woods — wood, herbs, berries (starter)
- Stonefang Quarry — ore, stone, gems (early unlock)
- Silver Creek — fish, river clay, freshwater pearls (early unlock)
- Ashenmoor Bog — mushrooms, peat, rare reagents (mid-game)
- Frosthollow Peaks — rare ore, ice crystals, alpine herbs (late-game)

**Duration tiers:**
- Quick forage: 5-10 seconds (small yield, active play)
- Standard expedition: 30s-1 min (moderate yield)
- Deep expedition: 3-10 min (large yield, rare drop chance)

**Gathering slots:** Start with 1, upgradeable to 2, then 3.

### 2. Refining

Turn raw materials into usable materials at refining stations.

**Stations** (built/upgraded via crafting):
- Workbench (starter) — wood -> planks, herbs -> dried herbs
- Furnace (built early) — ore -> ingots, sand -> glass
- Alchemy Table (mid-game) — herbs + reagents -> potions, extracts
- Loom (mid-game) — fibers -> cloth, spider silk -> fine thread

**Time scales:**
- Simple: 5-10 seconds (planks, dried herbs)
- Standard: 30-45 seconds (ingots, extracts)
- Complex: 1-3 minutes (alloys, fine materials)

**Batch processing:** Queue multiple units. Time is per unit, they process sequentially.

### 3. Crafting

Create gear for adventurers and goods for the tavern from refined materials.

**Gear branch:**
- Weapons: swords, daggers, staves, maces, bows
- Armor: light / medium / heavy
- Accessories: rings, amulets, cloaks
- Tier progression: wooden -> iron -> steel -> mithril
- Each piece provides stat bonuses (HP, STR, DEX, INT)

**Tavern goods branch:**
- Food: stew, roast, pie, etc.
- Drinks: ale, wine, mead, specialties
- Room upgrades: better beds, decorations (permanent bonuses)
- Quality tiers affect gold earned per serving

**Recipe discovery:**
- Basic recipes known from the start
- Others found as adventure loot or reputation unlocks

**Time scales:**
- Simple items: 10-30 seconds
- Standard gear: 1-3 minutes
- High-tier gear: 5-10 minutes

### 4. Adventures

The game's main course. Adventures are designed to be **interactive when you want them to be, idle when you don't**. The player chooses between **manual play** (engaging tactical mode) and **auto-resolve** (idle queue-and-wait), both available for every quest. Manual play is the engaging way and gives slightly better results; auto-resolve is the idle escape hatch.

#### The Five Adventurers

| Name     | Class   | STR  | DEX  | INT  | HP   | Flavor                          |
|----------|---------|------|------|------|------|---------------------------------|
| Torvald  | Warrior | High | Low  | Low  | High | Tank, loyal, straightforward    |
| Sylvara  | Rogue   | Low  | High | Med  | Low  | Quick, cunning, treasure-hunter |
| Ember    | Mage    | Low  | Low  | High | Low  | Bookish, powerful, glass cannon |
| Aldric   | Paladin | Med  | Low  | Med  | High | Steady, protective, healer      |
| Briar    | Druid   | Low  | Med  | High | Med  | Nature-attuned, versatile, quiet|

**Stats:** HP, Strength, Dexterity, Intellect. Base stats grow on level-up. Gear adds bonuses.

**Equipment slots:** Weapon, Armor, Accessory.

**Status states:** `Ready`, `OnQuest`, `Recovering` (cooldown after hard quests), `Downed` (1 HP at quest end — no permadeath).

**Leveling:** XP from completed quests. Stat growth on level-up.

#### Party Composition

Send 1–3 adventurers per quest. Solo is hard mode. Some quests require a minimum or recommend a specific party size. Only **one active adventure at a time** for MVP; multiple parallel adventures come later.

#### Quest System

Quests live on a **Quest Board** sub-screen. Each quest has:
- Name + flavor description
- Difficulty rating (1–5 stars)
- Recommended party size (solo / pair / trio)
- Recommended level
- Map (grid of squares)
- Estimated time (for auto-resolve)
- Reward preview

**Quest types** (variety in mechanics):
- **Combat Run** — fight-heavy, multiple encounters, boss at end (Phase 3a)
- **Exploration** — sparse combat, lots of treasure/events (Phase 3a)
- **Escort** — protect an NPC, fail if they die (later)
- **Hunt** — track and kill a specific named creature (later)
- **Mystery** — puzzle/event-driven, INT checks (later)
- **Gathering Run** — high-risk gathering at dangerous location (later)

#### The Map

Each quest has a **grid of squares** (5×5 to 7×7 typical, variable per quest). Rendered as halfblock pixel art tiles for a rich visual.

**Square types:**
- **Empty** — path squares, nothing happens
- **Combat** — enemy encounter
- **Treasure** — find loot, no fight
- **Rest** — heal up the party
- **Trap** — DEX check, fail = damage
- **Event** — narrative encounter with stat-checked choices
- **Boss** — quest's final challenge (one per map)

**Movement:** Arrow keys, one square at a time, Manhattan only (no diagonal).
**Fog of war:** Squares hidden until party gets adjacent.

#### Combat

Turn-based, party-first. Each adventurer takes a turn in initiative order, then enemies, repeat.

**Enemy counts:** 1–3 enemies per encounter. Boss encounters: a single tougher enemy, optionally with 1–2 minions for harder quests.

**Adventurer actions:**
- **Attack** — basic damage from STR or DEX (whichever fits the weapon)
- **Skill** — class-specific ability (Phase 3f):
  - Torvald: Cleave (hits 2 enemies)
  - Sylvara: Backstab (high damage on DEX check)
  - Ember: Firebolt (INT-based ranged)
  - Aldric: Heal (restore HP to one ally)
  - Briar: Entangle (skip enemy turn)
- **Item** — use a food/potion from the party's pack
- **Defend** — halve incoming damage next round
- **Flee** — party-wide DEX check to escape

**Damage formula:** `damage = (STR or DEX) + weapon_bonus - target_armor`. Crits, misses, status effects come later.

**Loss conditions:**
- All adventurers downed → quest fails, party returned with injuries
- Downed adventurer in MVP: restored to 1 HP at quest end (no permadeath)

#### Auto-Resolve

The escape hatch for idle play:
1. Simulates each square in sequence using the same combat math
2. Takes the quest's full estimated duration in real time (idle-friendly)
3. Returns slightly worse rewards than manual (no tactical optimization)
4. Active panel's Adventures tab shows progress like other timers

#### Items in Combat

Crafted food/drinks become combat-usable consumables, giving them a real purpose beyond the tavern. Hearty Stew restores party HP mid-fight, etc.

#### Loot

- Quest completion rewards (gold, materials, recipe scrolls)
- Treasure squares
- Enemy drops (combat as bonus gathering)
- Boss drops (rare/unique items)

#### Adventures View Layout

Sub-screens within the Adventures view:
1. **Quest Board** (default) — list of available quests as cards
2. **Party Setup** — choose party, equip gear, "Start Manual" / "Start Auto"
3. **In-Adventure** — map view with party position, fog of war
4. **Combat** — turn-based combat UI with enemies, party actions, log
5. **Event** — narrative text + choice buttons
6. **Results** — summary of XP, loot, casualties

**Left column adapts:** When in an active adventure (sub-screens 3–5), the Lanty box is replaced with **Party portraits** showing each adventurer's HP, status, current action. Returns to Lanty when the adventure ends.

#### Future Enhancements (Phase 3d–3f and beyond)
- Items usable in combat (crafted food = combat consumables)
- Loot drops from defeated enemies
- Stories/narrative log entries from adventures
- Adventurer XP and leveling
- Recovery time between adventures
- Reputation gain from quest completion
- Mood/morale system
- Adventurer personalities affecting event outcomes
- Boss encounters with phases
- Bestiary tracking
- Adventurer dialog and party banter
- Procedurally generated maps
- Optional permadeath toggle
- Multi-day expeditions with wayfaring inns

### 5. The Tavern

The hub. Visitors arrive, order food and drinks, pay gold. Keep it stocked.

**Visitors:**
- Arrive passively on a timer (every 30s-2min)
- Order food + drink; if stocked, you earn gold; if not, reputation hit
- Higher reputation = more visitors, wealthier visitors
- Returning adventurers always visit (bonus gold + story in the log)

**Stocking:** Place crafted food/drinks into tavern stock. Consumed by visitors over time.

**Upgrades** (permanent, cost gold + materials):
- Better tables — more visitor capacity
- Better kitchen — food gives more gold
- Better cellar — drinks give more gold
- Rooms for rent — passive overnight gold
- Noticeboard — unlocks higher-tier quests

**Reputation:** Increases from serving visitors, completing quests, upgrading. Unlocks gathering locations, recipes, quest tiers, adventurer recruitment.

**The Log:** All events play out narratively. "A weary traveler orders your mushroom stew. +3 gold." "Torvald returns from the Bandit Camp, triumphant! He orders three ales to celebrate."

---

## Adventurer Stats & Combat Resolution

Stats determine quest outcomes:
- **HP** — survivability, buffer against failure
- **Strength** — physical quests, combat encounters
- **Dexterity** — stealth quests, traps, speed challenges
- **Intellect** — puzzle quests, magical encounters, lore discovery

Quest difficulty defined as stat thresholds. Adventurer's effective stats (base + gear) compared against difficulty. Margin determines outcome tier (triumph/success/narrow escape/failure).

---

## Time & Persistence

- **Real-time based.** Tasks store timestamps. On resume, calculate what completed while away.
- **Save system.** Serialize full game state. Auto-save periodically + on quit. Load on startup.
- **Dev skip button.** Keybind to instantly complete all active timers. Dev mode only.

---

## Lanty (Mascot & Emotional Barometer)

Pixel-art PNG sprites, cached as halfblock frames at startup. Expression swaps are instant.

**Expressions:**
- Idle/neutral — default, gentle bob animation
- Happy — adventurer returned successfully, good tavern night
- Excited — new recipe discovered, level up
- Worried — stock running low, adventurer on hard quest
- Sleepy — nothing happening, long idle
- Celebrating — big milestone, triumph result

---

## Items & Resources

### Raw Materials (from Gathering)
Wood, ore, stone, herbs, berries, fish, river clay, freshwater pearls, mushrooms, peat, reagents, rare ore, ice crystals, alpine herbs, gems, fibers, spider silk, sand

### Refined Materials (from Refining)
Planks, ingots, glass, dried herbs, cloth, fine thread, potions, extracts, alloys

### Crafted Goods (from Crafting)
Weapons, armor, accessories (with stat bonuses and tiers), food, drinks, room upgrades (with gold value and tavern bonuses)

---

## Implementation Phases

### Phase 1: Foundation
- Game state data model (all structs, enums, items)
- Time/tick system with real timestamps
- Save/load (JSON serialization)
- Gathering system (locations, tasks, collection)
- Basic inventory system
- Dev skip-wait keybind
- *UI: Alva's department*

### Phase 2: Processing Pipeline
- Refining system (stations, recipes, queues, batch processing)
- Crafting system (recipes, gear stats, tavern goods)
- Recipe data (starter set for each station/category)
- Item stat system for gear
- *UI: Alva's department*

### Phase 3: Adventures (broken into sub-phases)

#### Phase 3a: Foundation & First Adventure (MVP)
- Adventurer data layer (5 starters, stats, status)
- Equipment system (slots, gear stat bonuses)
- Quest data layer (1 hand-designed quest with 5×5 map)
- Party View (replaces Lanty during adventures)
- Quest Board sub-screen (quest cards)
- Party Setup sub-screen (pick party, equip gear, start)
- In-Adventure sub-screen (halfblock map, fog of war, movement)
- Square types: Empty, Treasure, Rest, Trap
- One Combat encounter on the map (basic turn-based)
- Quest completion → loot return
- Adventures tab in the active panel (shows current quest)
- *UI: Alva's department for visual treatment*

#### Phase 3b: Manual Combat Expansion
- Full combat sub-screen with enemies, party actions, log
- Action selection: Attack, Defend, Flee, Item
- Items usable in combat (crafted consumables)
- Damage calc, HP tracking, downed adventurers
- Multiple enemies per encounter (1–3)
- Win/loss conditions

#### Phase 3c: Auto-Resolve
- Auto-resolve simulation (uses real combat math)
- Time-based progression for auto adventures
- Slightly worse rewards than manual play
- Active panel shows progress

#### Phase 3d: Variety
- Event squares with narrative + choices + stat checks
- Boss encounters (single tough enemy + optional minions)
- More quests with different layouts and themes
- Quest types: Escort, Hunt, Mystery, Gathering Run

#### Phase 3e: Progression & Recovery
- XP and leveling system
- Recovery timers between adventures
- Reputation gain from quest completion
- Reputation unlocks (new quests, locations)

#### Phase 3f: Skills & Class Identity
- Class-specific combat skills (Cleave, Backstab, Firebolt, Heal, Entangle)
- Skill cooldowns/resource limits
- Tactical depth

#### Phase 3g: Randomized Dungeons
- Procedurally generated multi-floor dungeons replace static maps for replayability
- **DungeonDef**: biome, enemy pool per floor, floor count, floor sizes, boss
- **Map generation**: rooms + corridors, random placement of encounters/treasure/traps/ladders
- **Multi-floor**: ladders connect floors; deeper = harder + better loot
- **Retreat mechanic**: go back up to floor 1 then exit. Keep loot, get 50% XP. Wipe = lose some loot + recovery time
- **Enemy scaling**: base stats × (1 + 0.15 × floor_depth)
- **Loot scaling**: more/better treasure on deeper floors, rare drops only from floor 3+
- **Tier 1 — The Cellar** (2-3 floors, 5×5 to 7×7): Giant Rats, Cellar Spiders. Boss: Rat King
- **Tier 2 — The Burrows** (3-4 floors, 5×5 to 8×8): Cave Spiders, Giant Snakes. Boss: Brood Mother
- **Tier 3 — The Wilds** (3-5 floors, 6×6 to 10×10): Goblin Scouts/Warriors/Shamans. Boss: Goblin Chieftain
- **Tier 4 — The Depths** (4-5 floors, 7×7 to 12×12): harder versions + new enemy types
- Original "Sunken Cellar" kept as a story quest alongside randomized dungeons
- Each dungeon regenerates on re-entry (new layout, same enemy pool)
- Ladder must be found (hidden on map behind fog of war)
- Enemy portraits for all new enemy types
- Floor indicator in UI showing current depth

#### Phase 3h: Depth & Polish (the "more ideas" list)
- Adventurer personalities affecting events
- Adventurer dialog and party banter
- Mood/morale system
- Boss encounters with phases
- Loot drops from defeated enemies (combat = bonus gathering)
- Stories/narrative log entries
- Bestiary tracking
- Optional permadeath toggle
- Multi-day expeditions with wayfaring inns
- Multiple parallel adventures (split party)

### Phase 4: The Tavern Economy
- Visitor system (arrival timer, orders, gold)
- Tavern stocking
- Reputation system
- Tavern upgrades
- Narrative log events for tavern activity
- Gold economy balancing
- *UI: Alva's department*

### Phase 5: Polish & Progression
- Lanty expression system (PNG sprites, cached frames, state-driven swaps)
- Lanty idle animations (bob, sway, blink)
- Reputation unlock gates (locations, recipes, quests, adventurers)
- Recipe discovery via adventure loot
- Full item/recipe content pass (all tiers, all categories)
- Balance pass on timers, yields, costs, difficulty
- Particle effects, transitions, color animations

### Future Considerations
- Party system (multi-adventurer quests)
- Randomly generated adventurers (mercenary board)
- Adventurer relationship webs
- Skills and abilities
- Personality traits affecting quest outcomes
- More gathering locations
- Seasonal events
- Achievement system
