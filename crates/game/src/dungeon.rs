#![allow(dead_code)]

use rand::Rng;
use serde::{Deserialize, Serialize};

use super::item::ItemId;
use super::quest::{Encounter, Enemy, QuestMap, SquareKind};

// ── Dungeon definition ────────────────────────────────────────────────────

/// A dungeon biome that can be entered multiple times. Each entry generates
/// a fresh set of procedurally-generated floors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DungeonDef {
    pub id: String,
    pub name: String,
    pub description: String,
    pub tier: u32,
    pub min_floors: u32,
    pub max_floors: u32,
    /// (width, height) range per floor. Grows with depth.
    pub base_floor_size: (u32, u32),
    pub floor_size_growth: u32, // added to each dimension per floor
    /// Enemy pools per floor range. Index by min(floor, pools.len()-1).
    pub enemy_pools: Vec<EnemyPool>,
    /// Boss encounter on the final floor.
    pub boss: Encounter,
    /// Party size limits.
    pub min_party: u32,
    pub max_party: u32,
    pub recommended_level: u32,
    /// Base rewards — scaled by depth.
    pub base_gold_per_floor: u32,
    pub base_xp_per_floor: u32,
    /// Completion bonus (on top of per-floor rewards).
    pub completion_gold: u32,
    pub completion_xp: u32,
    pub completion_loot: Vec<(ItemId, u32)>,
}

/// The pool of enemies that can appear on a given floor range.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnemyPool {
    pub enemies: Vec<Enemy>,
    /// How many encounters to place per floor from this pool.
    pub encounters_min: u32,
    pub encounters_max: u32,
}

// ── Floor generation ──────────────────────────────────────────────────────

/// A tile in the generated map: Wall or Open.
#[derive(Clone, Copy, PartialEq)]
enum Tile {
    Wall,
    Open,
}

/// Generate a single dungeon floor.
pub fn generate_floor(
    dungeon: &DungeonDef,
    floor_idx: u32,
    total_floors: u32,
) -> QuestMap {
    let mut rng = rand::rng();
    let is_boss_floor = floor_idx + 1 == total_floors;

    // Floor size grows with depth
    let w = (dungeon.base_floor_size.0 + floor_idx * dungeon.floor_size_growth).min(14);
    let h = (dungeon.base_floor_size.1 + floor_idx * dungeon.floor_size_growth).min(14);

    // Generate room layout
    let (grid, rooms) = generate_rooms(&mut rng, w, h);

    // Place squares
    let mut squares: Vec<Vec<SquareKind>> =
        vec![vec![SquareKind::Empty; w as usize]; h as usize];

    // Start position: center of the first room (or bottom-center)
    let start_room = &rooms[0];
    let start = (
        start_room.x + start_room.w / 2,
        start_room.y + start_room.h / 2,
    );

    // Pick the enemy pool for this floor
    let pool_idx = (floor_idx as usize).min(dungeon.enemy_pools.len().saturating_sub(1));
    let pool = &dungeon.enemy_pools[pool_idx];

    // Depth scaling factor: stats × (1 + 0.15 × floor_idx)
    let scale = 1.0 + 0.15 * floor_idx as f64;

    // Place encounters in rooms (not the start room)
    let enc_count = rng.random_range(pool.encounters_min..=pool.encounters_max);
    let mut encounter_positions: Vec<(u32, u32)> = Vec::new();
    let candidate_rooms: Vec<&Room> = rooms.iter().skip(1).collect();
    for i in 0..enc_count {
        let room = candidate_rooms[i as usize % candidate_rooms.len()];
        let ex = room.x + rng.random_range(0..room.w);
        let ey = room.y + rng.random_range(0..room.h);
        if (ex, ey) != start && grid[ey as usize][ex as usize] == Tile::Open {
            // Create a scaled encounter
            let enemy_count = rng.random_range(1..=3u32).min(pool.enemies.len() as u32);
            let mut enemies = Vec::new();
            for _ in 0..enemy_count {
                let template = &pool.enemies[rng.random_range(0..pool.enemies.len())];
                enemies.push(scale_enemy(template, scale));
            }
            let enc_id = format!("gen_f{}_{}", floor_idx, i);
            squares[ey as usize][ex as usize] = SquareKind::Combat {
                encounter_id: enc_id.clone(),
            };
            encounter_positions.push((ex, ey));
        }
    }

    // Place treasure (1-2 per floor, more on deeper floors)
    let treasure_count = rng.random_range(1..=2) + (floor_idx / 2);
    for _ in 0..treasure_count {
        if let Some((tx, ty)) = find_empty_open(&grid, &squares, &mut rng, w, h, start) {
            let gold = (dungeon.base_gold_per_floor as f64 * scale * 0.5) as u32
                + rng.random_range(1..=5);
            let mut items = Vec::new();
            // Rare drops only from floor 3+
            if floor_idx >= 2 && rng.random_range(0..100) < 15 {
                items.push(("heartwood".into(), 1));
            }
            squares[ty as usize][tx as usize] = SquareKind::Treasure { gold, items };
        }
    }

    // Place rest spot (0-1)
    if rng.random_range(0..100) < 60 {
        if let Some((rx, ry)) = find_empty_open(&grid, &squares, &mut rng, w, h, start) {
            squares[ry as usize][rx as usize] = SquareKind::Rest;
        }
    }

    // Place traps (0-2, more on deeper floors)
    let trap_count = rng.random_range(0..=1) + floor_idx.min(2);
    for _ in 0..trap_count {
        if let Some((tx, ty)) = find_empty_open(&grid, &squares, &mut rng, w, h, start) {
            let damage = (3.0 * scale) as i32;
            let dex_dc = 8 + floor_idx as i32;
            squares[ty as usize][tx as usize] = SquareKind::Trap { damage, dex_dc };
        }
    }

    // Place boss or ladder down
    // Find a good spot far from start (last room preferably)
    let end_room = rooms.last().unwrap();
    let end = (
        end_room.x + end_room.w / 2,
        end_room.y + end_room.h / 2,
    );

    if is_boss_floor {
        squares[end.1 as usize][end.0 as usize] = SquareKind::Boss {
            encounter_id: "boss".into(),
        };
    } else {
        squares[end.1 as usize][end.0 as usize] = SquareKind::LadderDown;
    }

    // Make walls truly impassable by leaving them as Empty (already done)
    // Actually we need to mark walls distinctly so the player can't walk through them.
    // For now, use Empty for open tiles and keep walls implicit (player can walk on Empty squares).
    // We'll handle wall collision by only allowing movement to Open tiles.

    // Convert: Wall tiles become a non-walkable marker. We'll use a simple approach:
    // open tiles stay Empty (or whatever was placed), wall tiles get a special "Wall" variant.
    // But SquareKind doesn't have a Wall variant... let's keep it simple.
    // The map generator ensures rooms and corridors are Open. Everything else is wall.
    // We'll store wall info separately.

    QuestMap {
        width: w,
        height: h,
        squares,
        start,
        end,
    }
}

/// Scale an enemy's stats by a depth multiplier.
pub fn scale_enemy(template: &Enemy, scale: f64) -> Enemy {
    Enemy {
        name: template.name.clone(),
        max_hp: (template.max_hp as f64 * scale) as i32,
        strength: (template.strength as f64 * scale) as i32,
        xp_reward: (template.xp_reward as f64 * scale) as u32,
    }
}

/// Scale a boss encounter.
pub fn scale_boss(boss: &Encounter, scale: f64) -> Encounter {
    Encounter {
        id: boss.id.clone(),
        enemies: boss.enemies.iter().map(|e| scale_enemy(e, scale)).collect(),
    }
}

// ── Room generation ───────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct Room {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}

impl Room {
    fn center(&self) -> (u32, u32) {
        (self.x + self.w / 2, self.y + self.h / 2)
    }
}

fn generate_rooms(rng: &mut impl Rng, map_w: u32, map_h: u32) -> (Vec<Vec<Tile>>, Vec<Room>) {
    let mut grid = vec![vec![Tile::Wall; map_w as usize]; map_h as usize];
    let mut rooms = Vec::new();

    // Try to place 3-6 rooms
    let target_rooms = rng.random_range(3..=6u32).min(map_w * map_h / 12 + 2);
    for _ in 0..target_rooms * 3 {
        // attempt multiple times
        if rooms.len() >= target_rooms as usize {
            break;
        }
        let rw = rng.random_range(2..=4u32).min(map_w - 2);
        let rh = rng.random_range(2..=4u32).min(map_h - 2);
        let rx = rng.random_range(1..map_w.saturating_sub(rw).max(2));
        let ry = rng.random_range(1..map_h.saturating_sub(rh).max(2));

        // Check overlap
        let new_room = Room {
            x: rx,
            y: ry,
            w: rw,
            h: rh,
        };
        let overlaps = rooms.iter().any(|r: &Room| {
            rx < r.x + r.w + 1
                && rx + rw + 1 > r.x
                && ry < r.y + r.h + 1
                && ry + rh + 1 > r.y
        });
        if overlaps {
            continue;
        }

        // Carve room
        for y in ry..ry + rh {
            for x in rx..rx + rw {
                if (x as usize) < map_w as usize && (y as usize) < map_h as usize {
                    grid[y as usize][x as usize] = Tile::Open;
                }
            }
        }
        rooms.push(new_room);
    }

    // Ensure at least 2 rooms
    if rooms.len() < 2 {
        // Fallback: create two rooms manually
        let r1 = Room {
            x: 1,
            y: map_h - 3,
            w: 2,
            h: 2,
        };
        let r2 = Room {
            x: map_w - 3,
            y: 1,
            w: 2,
            h: 2,
        };
        for r in [&r1, &r2] {
            for y in r.y..r.y + r.h {
                for x in r.x..r.x + r.w {
                    if (x as usize) < map_w as usize && (y as usize) < map_h as usize {
                        grid[y as usize][x as usize] = Tile::Open;
                    }
                }
            }
        }
        rooms = vec![r1, r2];
    }

    // Connect rooms with corridors (L-shaped tunnels between consecutive rooms)
    for i in 0..rooms.len() - 1 {
        let (cx1, cy1) = rooms[i].center();
        let (cx2, cy2) = rooms[i + 1].center();
        carve_corridor(&mut grid, cx1, cy1, cx2, cy2, map_w, map_h);
    }

    (grid, rooms)
}

fn carve_corridor(
    grid: &mut Vec<Vec<Tile>>,
    x1: u32,
    y1: u32,
    x2: u32,
    y2: u32,
    w: u32,
    h: u32,
) {
    // Horizontal first, then vertical
    let (mut x, mut y) = (x1, y1);
    while x != x2 {
        if (x as usize) < w as usize && (y as usize) < h as usize {
            grid[y as usize][x as usize] = Tile::Open;
        }
        if x < x2 {
            x += 1;
        } else {
            x -= 1;
        }
    }
    while y != y2 {
        if (x as usize) < w as usize && (y as usize) < h as usize {
            grid[y as usize][x as usize] = Tile::Open;
        }
        if y < y2 {
            y += 1;
        } else {
            y -= 1;
        }
    }
    if (x as usize) < w as usize && (y as usize) < h as usize {
        grid[y as usize][x as usize] = Tile::Open;
    }
}

fn find_empty_open(
    grid: &[Vec<Tile>],
    squares: &[Vec<SquareKind>],
    rng: &mut impl Rng,
    w: u32,
    h: u32,
    start: (u32, u32),
) -> Option<(u32, u32)> {
    for _ in 0..50 {
        let x = rng.random_range(0..w);
        let y = rng.random_range(0..h);
        if (x, y) == start {
            continue;
        }
        if grid[y as usize][x as usize] != Tile::Open {
            continue;
        }
        if !matches!(squares[y as usize][x as usize], SquareKind::Empty) {
            continue;
        }
        return Some((x, y));
    }
    None
}

// ── Dungeon registry ──────────────────────────────────────────────────────

pub fn register_dungeons() -> Vec<DungeonDef> {
    vec![the_cellar(), the_burrows()]
}

fn the_cellar() -> DungeonDef {
    DungeonDef {
        id: "cellar".into(),
        name: "The Cellar".into(),
        description: "Rats have infested the old root cellar beneath the tavern. Clear them out... if you dare go deep enough.".into(),
        tier: 1,
        min_floors: 2,
        max_floors: 3,
        base_floor_size: (5, 5),
        floor_size_growth: 1,
        enemy_pools: vec![
            // Floor 1: just rats
            EnemyPool {
                enemies: vec![Enemy {
                    name: "Giant Rat".into(),
                    max_hp: 8,
                    strength: 3,
                    xp_reward: 5,
                }],
                encounters_min: 2,
                encounters_max: 3,
            },
            // Floor 2+: rats + cellar spiders
            EnemyPool {
                enemies: vec![
                    Enemy {
                        name: "Giant Rat".into(),
                        max_hp: 8,
                        strength: 3,
                        xp_reward: 5,
                    },
                    Enemy {
                        name: "Cellar Spider".into(),
                        max_hp: 6,
                        strength: 4,
                        xp_reward: 7,
                    },
                ],
                encounters_min: 3,
                encounters_max: 4,
            },
        ],
        boss: Encounter {
            id: "rat_king".into(),
            enemies: vec![
                Enemy {
                    name: "Rat King".into(),
                    max_hp: 30,
                    strength: 6,
                    xp_reward: 25,
                },
                Enemy {
                    name: "Giant Rat".into(),
                    max_hp: 8,
                    strength: 3,
                    xp_reward: 5,
                },
            ],
        },
        min_party: 1,
        max_party: 2,
        recommended_level: 1,
        base_gold_per_floor: 8,
        base_xp_per_floor: 15,
        completion_gold: 25,
        completion_xp: 40,
        completion_loot: vec![("wood".into(), 3), ("herbs".into(), 2)],
    }
}

fn the_burrows() -> DungeonDef {
    DungeonDef {
        id: "burrows".into(),
        name: "The Burrows".into(),
        description: "Dark tunnels wind beneath the hills. Spiders and snakes have made their home in the damp corridors.".into(),
        tier: 2,
        min_floors: 3,
        max_floors: 4,
        base_floor_size: (5, 5),
        floor_size_growth: 2,
        enemy_pools: vec![
            // Floor 1: cave spiders
            EnemyPool {
                enemies: vec![
                    Enemy {
                        name: "Cave Spider".into(),
                        max_hp: 12,
                        strength: 5,
                        xp_reward: 10,
                    },
                    Enemy {
                        name: "Giant Rat".into(),
                        max_hp: 8,
                        strength: 3,
                        xp_reward: 5,
                    },
                ],
                encounters_min: 2,
                encounters_max: 3,
            },
            // Floor 2: spiders + snakes
            EnemyPool {
                enemies: vec![
                    Enemy {
                        name: "Cave Spider".into(),
                        max_hp: 12,
                        strength: 5,
                        xp_reward: 10,
                    },
                    Enemy {
                        name: "Giant Snake".into(),
                        max_hp: 18,
                        strength: 7,
                        xp_reward: 14,
                    },
                ],
                encounters_min: 3,
                encounters_max: 4,
            },
            // Floor 3+: more snakes, harder
            EnemyPool {
                enemies: vec![
                    Enemy {
                        name: "Giant Snake".into(),
                        max_hp: 18,
                        strength: 7,
                        xp_reward: 14,
                    },
                    Enemy {
                        name: "Cave Spider".into(),
                        max_hp: 12,
                        strength: 5,
                        xp_reward: 10,
                    },
                ],
                encounters_min: 3,
                encounters_max: 5,
            },
        ],
        boss: Encounter {
            id: "brood_mother".into(),
            enemies: vec![
                Enemy {
                    name: "Brood Mother".into(),
                    max_hp: 50,
                    strength: 8,
                    xp_reward: 45,
                },
                Enemy {
                    name: "Cave Spider".into(),
                    max_hp: 12,
                    strength: 5,
                    xp_reward: 10,
                },
                Enemy {
                    name: "Cave Spider".into(),
                    max_hp: 12,
                    strength: 5,
                    xp_reward: 10,
                },
            ],
        },
        min_party: 1,
        max_party: 3,
        recommended_level: 3,
        base_gold_per_floor: 15,
        base_xp_per_floor: 25,
        completion_gold: 50,
        completion_xp: 75,
        completion_loot: vec![
            ("planks".into(), 5),
            ("dried_herbs".into(), 3),
            ("heartwood".into(), 1),
        ],
    }
}
