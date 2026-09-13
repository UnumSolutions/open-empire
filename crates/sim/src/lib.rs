//! Deterministic integer-grid RTS. No renderer, wall clock, OS RNG, or unordered maps.
use empire_content::{Civilization, Kind, RULESET_VERSION, Resource, original_hash};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
pub type EntityId = u32;
pub type PlayerId = u8;
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const TICKS_PER_SECOND: u64 = 10;
pub const SAVE_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Pos {
    pub x: i16,
    pub y: i16,
}
impl Pos {
    pub fn new(x: i16, y: i16) -> Self {
        Self { x, y }
    }
    pub fn distance(self, other: Self) -> u16 {
        self.x.abs_diff(other.x) + self.y.abs_diff(other.y)
    }
    fn neighbors(self) -> [Self; 4] {
        [
            Self::new(self.x, self.y - 1),
            Self::new(self.x - 1, self.y),
            Self::new(self.x + 1, self.y),
            Self::new(self.x, self.y + 1),
        ]
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Terrain {
    Grass,
    Water,
    Forest,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MapKind {
    Land,
    Islands,
    Mixed,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tile {
    pub terrain: Terrain,
    pub resource: Option<Resource>,
    pub amount: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Map {
    pub width: i16,
    pub height: i16,
    pub tiles: Vec<Tile>,
}
impl Map {
    pub fn index(&self, p: Pos) -> Option<usize> {
        (p.x >= 0 && p.y >= 0 && p.x < self.width && p.y < self.height)
            .then(|| p.y as usize * self.width as usize + p.x as usize)
    }
    pub fn tile(&self, p: Pos) -> Option<&Tile> {
        self.index(p).and_then(|i| self.tiles.get(i))
    }
    pub fn tile_mut(&mut self, p: Pos) -> Option<&mut Tile> {
        self.index(p).and_then(|i| self.tiles.get_mut(i))
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Order {
    Idle,
    Move(Pos),
    Gather(Pos),
    Attack(EntityId),
    Construct(EntityId),
    Heal(EntityId),
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Production {
    pub kind: Kind,
    pub remaining: u16,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: EntityId,
    pub owner: PlayerId,
    pub kind: Kind,
    pub pos: Pos,
    pub hp: i32,
    pub max_hp: i32,
    pub construction: u16,
    pub order: Order,
    pub queue: VecDeque<Production>,
    pub cooldown: u16,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub civilization: Civilization,
    pub resources: [u32; 4],
    pub age: u8,
    pub age_work: u16,
    pub defeated: bool,
    pub explored: BTreeSet<Pos>,
    pub visible: BTreeSet<Pos>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Action {
    Move {
        units: Vec<EntityId>,
        target: Pos,
    },
    Gather {
        units: Vec<EntityId>,
        target: Pos,
    },
    Attack {
        units: Vec<EntityId>,
        target: EntityId,
    },
    Heal {
        units: Vec<EntityId>,
        target: EntityId,
    },
    Build {
        worker: EntityId,
        kind: Kind,
        at: Pos,
    },
    Train {
        building: EntityId,
        kind: Kind,
    },
    AdvanceAge,
    Trade {
        sell: Resource,
    },
    Stop {
        units: Vec<EntityId>,
    },
    Resign,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Command {
    pub tick: u64,
    pub player: PlayerId,
    pub sequence: u64,
    pub action: Action,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Identity {
    pub engine: String,
    pub ruleset: String,
    pub content_hash: String,
}
impl Default for Identity {
    fn default() -> Self {
        Self {
            engine: ENGINE_VERSION.into(),
            ruleset: RULESET_VERSION.into(),
            content_hash: original_hash(),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct World {
    pub identity: Identity,
    pub tick: u64,
    pub seed: u64,
    pub map: Map,
    pub players: Vec<Player>,
    pub entities: BTreeMap<EntityId, Entity>,
    pub next_id: EntityId,
    pub winner: Option<PlayerId>,
    pub finished: bool,
    pub last_sequences: Vec<Option<u64>>,
    pub wonder_since: BTreeMap<PlayerId, u64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Replay {
    pub format: u32,
    pub initial: World,
    pub commands: Vec<Command>,
    pub end_tick: u64,
    pub final_hash: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Save {
    format: u32,
    hash: String,
    world: World,
}

impl World {
    pub fn new(seed: u64, count: u8, map_kind: MapKind) -> Result<Self, String> {
        if !(1..=8).contains(&count) {
            return Err("player count must be 1–8".into());
        }
        let size = 64i16;
        let mut rng = seed.max(1);
        let mut tiles = Vec::new();
        for y in 0..size {
            for x in 0..size {
                rng ^= rng << 13;
                rng ^= rng >> 7;
                rng ^= rng << 17;
                let water = match map_kind {
                    MapKind::Land => false,
                    MapKind::Islands => x == 31 || x == 32 || y == 31 || y == 32,
                    MapKind::Mixed => (29..=33).contains(&x) && !(29..=34).contains(&y),
                };
                let resource = match rng % 90 {
                    0..=8 => Some(Resource::Wood),
                    9..=11 => Some(Resource::Food),
                    12 => Some(Resource::Gold),
                    13 => Some(Resource::Stone),
                    _ => None,
                };
                tiles.push(Tile {
                    terrain: if water {
                        Terrain::Water
                    } else if resource == Some(Resource::Wood) {
                        Terrain::Forest
                    } else {
                        Terrain::Grass
                    },
                    resource: if water { None } else { resource },
                    amount: 1000,
                });
            }
        }
        let mut world = Self {
            identity: Identity::default(),
            tick: 0,
            seed,
            map: Map {
                width: size,
                height: size,
                tiles,
            },
            players: (0..count)
                .map(|p| Player {
                    civilization: if p % 2 == 0 {
                        Civilization::Aster
                    } else {
                        Civilization::Veyran
                    },
                    resources: [300, 400, 150, 200],
                    age: 1,
                    age_work: 0,
                    defeated: false,
                    explored: BTreeSet::new(),
                    visible: BTreeSet::new(),
                })
                .collect(),
            entities: BTreeMap::new(),
            next_id: 1,
            winner: None,
            finished: false,
            last_sequences: vec![None; count as usize],
            wonder_since: BTreeMap::new(),
        };
        let spawns = [
            Pos::new(9, 9),
            Pos::new(54, 54),
            Pos::new(54, 9),
            Pos::new(9, 54),
            Pos::new(30, 9),
            Pos::new(30, 54),
            Pos::new(9, 30),
            Pos::new(54, 30),
        ];
        for p in 0..count {
            let at = spawns[p as usize];
            for y in at.y - 4..=at.y + 4 {
                for x in at.x - 4..=at.x + 4 {
                    *world.map.tile_mut(Pos::new(x, y)).unwrap() = Tile {
                        terrain: Terrain::Grass,
                        resource: None,
                        amount: 0,
                    };
                }
            }
            for (i, res) in [
                Resource::Food,
                Resource::Wood,
                Resource::Gold,
                Resource::Stone,
            ]
            .iter()
            .enumerate()
            {
                let t = world
                    .map
                    .tile_mut(Pos::new(at.x - 3 + i as i16 * 2, at.y + 4))
                    .unwrap();
                t.resource = Some(*res);
                t.amount = 2500;
                if *res == Resource::Wood {
                    t.terrain = Terrain::Forest;
                }
            }
            world.spawn(p, Kind::TownCenter, at, 0);
            for x in -1..=1 {
                world.spawn(p, Kind::Villager, Pos::new(at.x + x, at.y + 2), 0);
            }
        }
        world.update_visibility();
        Ok(world)
    }
    fn spawn(&mut self, owner: PlayerId, kind: Kind, pos: Pos, construction: u16) -> EntityId {
        let id = self.next_id;
        self.next_id += 1;
        let mut hp = kind.stats().hp;
        let civ = self.players[owner as usize].civilization;
        if civ == Civilization::Aster && kind.stats().building {
            hp = hp * 120 / 100;
        }
        if civ == Civilization::Veyran && kind == Kind::Cavalry {
            hp = hp * 115 / 100;
        }
        self.entities.insert(
            id,
            Entity {
                id,
                owner,
                kind,
                pos,
                hp,
                max_hp: hp,
                construction,
                order: Order::Idle,
                queue: VecDeque::new(),
                cooldown: 0,
            },
        );
        id
    }
    pub fn population(&self, p: PlayerId) -> usize {
        self.entities
            .values()
            .filter(|e| e.owner == p && !e.kind.stats().building)
            .count()
    }
    pub fn capacity(&self, p: PlayerId) -> usize {
        self.entities
            .values()
            .filter(|e| e.owner == p && e.construction == 0)
            .map(|e| match e.kind {
                Kind::TownCenter => 15,
                Kind::House => 5,
                _ => 0,
            })
            .sum::<usize>()
            .min(200)
    }
    pub fn pending_population(&self, p: PlayerId) -> usize {
        self.entities
            .values()
            .filter(|e| e.owner == p)
            .map(|e| e.queue.len())
            .sum()
    }
    pub fn owned(&self, id: EntityId, p: PlayerId) -> Result<&Entity, String> {
        self.entities
            .get(&id)
            .filter(|e| e.owner == p)
            .ok_or("entity not owned".into())
    }
    pub fn passable(&self, p: Pos, naval: bool) -> bool {
        self.map.tile(p).is_some_and(|t| {
            if naval {
                t.terrain == Terrain::Water
            } else {
                t.terrain != Terrain::Water && !(t.resource == Some(Resource::Wood) && t.amount > 0)
            }
        }) && !self
            .entities
            .values()
            .any(|e| e.pos == p && e.kind.stats().building)
    }
    fn free(&self, p: Pos, naval: bool) -> bool {
        self.passable(p, naval) && !self.entities.values().any(|e| e.pos == p)
    }
    fn pay(&mut self, p: PlayerId, cost: [u32; 4]) -> Result<(), String> {
        let r = &mut self.players[p as usize].resources;
        if r.iter().zip(cost).any(|(have, need)| *have < need) {
            return Err("not enough resources".into());
        }
        for i in 0..4 {
            r[i] -= cost[i];
        }
        Ok(())
    }
    fn units(&self, p: PlayerId, ids: &[EntityId]) -> Result<(), String> {
        if ids.is_empty() || ids.len() > 200 {
            return Err("select 1–200 units".into());
        }
        let mut seen = BTreeSet::new();
        for id in ids {
            let e = self.owned(*id, p)?;
            if !seen.insert(id) || e.kind.stats().building {
                return Err("invalid unit selection".into());
            }
        }
        Ok(())
    }
    pub fn apply(&mut self, cmd: &Command) -> Result<(), String> {
        let p = cmd.player;
        if self.finished {
            return Err("match finished".into());
        }
        if cmd.tick != self.tick {
            return Err("command tick mismatch".into());
        }
        if p as usize >= self.players.len() || self.players[p as usize].defeated {
            return Err("invalid player".into());
        }
        if self.last_sequences[p as usize].is_some_and(|s| cmd.sequence <= s) {
            return Err("duplicate or stale sequence".into());
        }
        match &cmd.action {
            Action::Move { units, target } | Action::Gather { units, target } => {
                self.units(p, units)?;
                if self.map.index(*target).is_none() {
                    return Err("target outside map".into());
                }
                let gather = matches!(cmd.action, Action::Gather { .. });
                if gather {
                    if !self.players[p as usize].explored.contains(target) {
                        return Err("resource unexplored".into());
                    }
                    if !self
                        .map
                        .tile(*target)
                        .is_some_and(|t| t.resource.is_some() && t.amount > 0)
                    {
                        return Err("no resource here".into());
                    }
                    if units
                        .iter()
                        .any(|id| self.entities[id].kind != Kind::Villager)
                    {
                        return Err("only villagers gather".into());
                    }
                }
                for id in units {
                    self.entities.get_mut(id).unwrap().order = if gather {
                        Order::Gather(*target)
                    } else {
                        Order::Move(*target)
                    };
                }
            }
            Action::Attack { units, target } | Action::Heal { units, target } => {
                self.units(p, units)?;
                let t = self.entities.get(target).ok_or("target missing")?;
                let heal = matches!(cmd.action, Action::Heal { .. });
                if !self.players[p as usize].visible.contains(&t.pos) {
                    return Err("target not visible".into());
                }
                if heal {
                    if t.owner != p || units.iter().any(|id| self.entities[id].kind != Kind::Monk) {
                        return Err("monks heal friendly units".into());
                    }
                } else if t.owner == p {
                    return Err("cannot attack own units".into());
                }
                for id in units {
                    self.entities.get_mut(id).unwrap().order = if heal {
                        Order::Heal(*target)
                    } else {
                        Order::Attack(*target)
                    };
                }
            }
            Action::Build { worker, kind, at } => {
                let e = self.owned(*worker, p)?;
                if e.kind != Kind::Villager || !kind.stats().building {
                    return Err("invalid builder or building".into());
                }
                if self.players[p as usize].age < kind.stats().age {
                    return Err("advance age first".into());
                }
                if !self.players[p as usize].visible.contains(at) {
                    return Err("building site not visible".into());
                }
                let dock = *kind == Kind::Dock;
                if !self.free(*at, dock)
                    || self
                        .map
                        .tile(*at)
                        .is_some_and(|t| t.resource.is_some() && t.amount > 0)
                {
                    return Err("building site blocked".into());
                }
                if dock && !at.neighbors().iter().any(|p| self.passable(*p, false)) {
                    return Err("dock must touch land".into());
                }
                self.pay(p, kind.stats().cost)?;
                let id = self.spawn(p, *kind, *at, kind.stats().work);
                self.entities.get_mut(worker).unwrap().order = Order::Construct(id);
            }
            Action::Train { building, kind } => {
                let e = self.owned(*building, p)?;
                if kind.producer() != Some(e.kind) || e.construction > 0 || e.queue.len() >= 5 {
                    return Err("invalid producer or full queue".into());
                }
                if self.players[p as usize].age < kind.stats().age {
                    return Err("advance age first".into());
                }
                if self.population(p) + self.pending_population(p) >= self.capacity(p) {
                    return Err("population limit; build houses".into());
                }
                self.pay(p, kind.stats().cost)?;
                self.entities
                    .get_mut(building)
                    .unwrap()
                    .queue
                    .push_back(Production {
                        kind: *kind,
                        remaining: kind.stats().work,
                    });
            }
            Action::AdvanceAge => {
                let pl = &self.players[p as usize];
                if pl.age >= 4 || pl.age_work > 0 {
                    return Err("age advancement unavailable".into());
                }
                if !self
                    .entities
                    .values()
                    .any(|e| e.owner == p && e.kind == Kind::TownCenter && e.construction == 0)
                {
                    return Err("town center required".into());
                }
                let age = pl.age;
                self.pay(
                    p,
                    match age {
                        1 => [400, 0, 0, 0],
                        2 => [600, 0, 200, 0],
                        _ => [800, 0, 600, 0],
                    },
                )?;
                self.players[p as usize].age_work = 100 * age as u16;
            }
            Action::Trade { sell } => {
                if *sell == Resource::Gold {
                    return Err("sell food, wood or stone for gold".into());
                }
                if !self
                    .entities
                    .values()
                    .any(|e| e.owner == p && e.kind == Kind::Market && e.construction == 0)
                {
                    return Err("market required".into());
                }
                let mut cost = [0; 4];
                cost[sell.index()] = 100;
                self.pay(p, cost)?;
                let pl = &mut self.players[p as usize];
                pl.resources[2] =
                    pl.resources[2].saturating_add(if pl.civilization == Civilization::Veyran {
                        85
                    } else {
                        70
                    });
            }
            Action::Stop { units } => {
                self.units(p, units)?;
                for id in units {
                    self.entities.get_mut(id).unwrap().order = Order::Idle;
                }
            }
            Action::Resign => {
                self.players[p as usize].defeated = true;
                self.entities.retain(|_, e| e.owner != p);
            }
        }
        self.last_sequences[p as usize] = Some(cmd.sequence);
        Ok(())
    }
    /// Commands are ordered canonically; rejected commands never partially mutate state.
    pub fn step(&mut self, commands: &[Command]) -> Vec<String> {
        if self.finished {
            return vec![];
        }
        let mut ordered = commands.to_vec();
        ordered.sort_by_key(|c| (c.player, c.sequence));
        let mut errors = Vec::new();
        for c in ordered {
            if let Err(e) = self.apply(&c) {
                errors.push(format!("player {} sequence {}: {e}", c.player, c.sequence));
            }
        }
        for p in &mut self.players {
            if p.age_work > 0 {
                p.age_work -= 1;
                if p.age_work == 0 {
                    p.age += 1;
                }
            }
        }
        let ids: Vec<_> = self.entities.keys().copied().collect();
        for id in ids {
            self.update_entity(id);
        }
        self.entities.retain(|_, e| e.hp > 0);
        self.tick += 1;
        self.update_visibility();
        self.check_victory();
        errors
    }
    fn update_entity(&mut self, id: EntityId) {
        let Some(e) = self.entities.get(&id).cloned() else {
            return;
        };
        if e.hp <= 0 || e.construction > 0 {
            return;
        }
        if e.cooldown > 0 {
            self.entities.get_mut(&id).unwrap().cooldown -= 1;
        }
        if let Some(job) = e.queue.front() {
            if job.remaining > 0 {
                self.entities
                    .get_mut(&id)
                    .unwrap()
                    .queue
                    .front_mut()
                    .unwrap()
                    .remaining -= 1;
            } else if self.population(e.owner) < self.capacity(e.owner) {
                if let Some(pos) = e
                    .pos
                    .neighbors()
                    .into_iter()
                    .find(|p| self.free(*p, job.kind == Kind::Warship))
                {
                    self.spawn(e.owner, job.kind, pos, 0);
                    self.entities.get_mut(&id).unwrap().queue.pop_front();
                }
            }
        }
        match e.order {
            Order::Move(to) => {
                if e.pos == to {
                    self.entities.get_mut(&id).unwrap().order = Order::Idle;
                } else {
                    self.walk(id, to, 0);
                }
            }
            Order::Gather(to) => {
                let resource = self
                    .map
                    .tile(to)
                    .and_then(|t| if t.amount > 0 { t.resource } else { None });
                if let Some(r) = resource {
                    if e.pos.distance(to) > 1 {
                        self.walk(id, to, 1);
                    } else if self.tick % 5 == 0 {
                        self.map.tile_mut(to).unwrap().amount -= 1;
                        self.players[e.owner as usize].resources[r.index()] =
                            self.players[e.owner as usize].resources[r.index()].saturating_add(1);
                    }
                } else {
                    self.entities.get_mut(&id).unwrap().order = Order::Idle;
                }
            }
            Order::Construct(target) => {
                if let Some(t) = self.entities.get(&target).cloned() {
                    if e.pos.distance(t.pos) > 1 {
                        self.walk(id, t.pos, 1);
                    } else if t.construction > 0 {
                        self.entities.get_mut(&target).unwrap().construction -= 1;
                    } else {
                        self.entities.get_mut(&id).unwrap().order = Order::Idle;
                    }
                } else {
                    self.entities.get_mut(&id).unwrap().order = Order::Idle;
                }
            }
            Order::Attack(target) => self.engage(id, target, false),
            Order::Heal(target) => self.engage(id, target, true),
            Order::Idle => {
                if e.kind.stats().damage > 0 && e.kind != Kind::Villager {
                    if let Some(t) = self
                        .entities
                        .values()
                        .filter(|t| {
                            t.owner != e.owner
                                && t.hp > 0
                                && self.players[e.owner as usize].visible.contains(&t.pos)
                                && e.pos.distance(t.pos) <= 6
                        })
                        .min_by_key(|t| (e.pos.distance(t.pos), t.id))
                        .map(|t| t.id)
                    {
                        if e.kind.stats().building {
                            self.engage(id, t, false);
                        } else {
                            self.entities.get_mut(&id).unwrap().order = Order::Attack(t);
                        }
                    }
                }
            }
        }
    }
    fn engage(&mut self, id: EntityId, target: EntityId, heal: bool) {
        let e = self.entities[&id].clone();
        let Some(t) = self.entities.get(&target).cloned() else {
            self.entities.get_mut(&id).unwrap().order = Order::Idle;
            return;
        };
        if !self.players[e.owner as usize].visible.contains(&t.pos) {
            self.entities.get_mut(&id).unwrap().order = Order::Idle;
            return;
        }
        if e.pos.distance(t.pos) > e.kind.stats().range {
            if !e.kind.stats().building {
                self.walk(id, t.pos, e.kind.stats().range);
            }
            return;
        }
        if e.cooldown > 0 {
            return;
        }
        if heal {
            self.entities.get_mut(&target).unwrap().hp = (t.hp + 5).min(t.max_hp);
        } else {
            let mut damage = e.kind.stats().damage;
            if e.kind == Kind::Spearman && t.kind == Kind::Cavalry {
                damage += 20;
            }
            if e.kind == Kind::Ram && t.kind.stats().building {
                damage += 60;
            }
            if e.kind == Kind::Archer && t.kind == Kind::Spearman {
                damage += 4;
            }
            self.entities.get_mut(&target).unwrap().hp -= damage;
        }
        self.entities.get_mut(&id).unwrap().cooldown = 10;
    }
    /// Stable breadth-first routing with occupied-cell avoidance. Hierarchical routing is a later optimization.
    fn walk(&mut self, id: EntityId, target: Pos, range: u16) {
        let e = &self.entities[&id];
        if self.tick % if e.kind == Kind::Cavalry { 2 } else { 3 } != 0 {
            return;
        }
        let start = e.pos;
        let naval = e.kind == Kind::Warship;
        let occupied: BTreeSet<_> = self
            .entities
            .values()
            .filter(|e| e.id != id)
            .map(|e| e.pos)
            .collect();
        let blocked: BTreeSet<_> = self
            .entities
            .values()
            .filter(|e| e.kind.stats().building)
            .map(|e| e.pos)
            .collect();
        let passable = |p: Pos| {
            self.map.tile(p).is_some_and(|t| {
                if naval {
                    t.terrain == Terrain::Water
                } else {
                    t.terrain != Terrain::Water
                        && !(t.resource == Some(Resource::Wood) && t.amount > 0)
                }
            }) && !blocked.contains(&p)
                && !occupied.contains(&p)
        };
        let mut q = VecDeque::from([start]);
        let mut prev = BTreeMap::from([(start, start)]);
        let mut found = None;
        while let Some(p) = q.pop_front() {
            if p.distance(target) <= range {
                found = Some(p);
                break;
            }
            for n in p.neighbors() {
                if !prev.contains_key(&n) && passable(n) {
                    prev.insert(n, p);
                    q.push_back(n);
                }
            }
        }
        if let Some(mut p) = found {
            if p == start {
                return;
            }
            while prev[&p] != start {
                p = prev[&p];
            }
            self.entities.get_mut(&id).unwrap().pos = p;
        }
    }
    fn update_visibility(&mut self) {
        for p in &mut self.players {
            p.visible.clear();
        }
        for e in self.entities.values() {
            if e.hp <= 0 {
                continue;
            }
            for y in -7i16..=7 {
                for x in -7i16..=7 {
                    if x.abs() + y.abs() > 7 {
                        continue;
                    }
                    let p = Pos::new(e.pos.x + x, e.pos.y + y);
                    if self.map.index(p).is_some() {
                        self.players[e.owner as usize].visible.insert(p);
                    }
                }
            }
        }
        for p in &mut self.players {
            p.explored.extend(&p.visible);
        }
    }
    fn check_victory(&mut self) {
        for p in 0..self.players.len() {
            if !self.entities.values().any(|e| e.owner as usize == p) {
                self.players[p].defeated = true;
            }
        }
        for p in 0..self.players.len() as u8 {
            if self
                .entities
                .values()
                .any(|e| e.owner == p && e.kind == Kind::Wonder && e.construction == 0)
            {
                let since = *self.wonder_since.entry(p).or_insert(self.tick);
                if self.tick - since >= 2000 {
                    self.winner = Some(p);
                    self.finished = true;
                }
            } else {
                self.wonder_since.remove(&p);
            }
        }
        let alive: Vec<_> = self
            .players
            .iter()
            .enumerate()
            .filter(|(_, p)| !p.defeated)
            .map(|(i, _)| i as u8)
            .collect();
        if self.players.len() > 1 && alive.len() <= 1 {
            self.winner = alive.first().copied();
            self.finished = true;
        }
    }
    pub fn hash(&self) -> String {
        blake3::hash(&serde_json::to_vec(self).expect("serializable state"))
            .to_hex()
            .to_string()
    }
    pub fn save(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec(&Save {
            format: SAVE_VERSION,
            hash: self.hash(),
            world: self.clone(),
        })
        .map_err(|e| e.to_string())
    }
    pub fn load(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > 32 * 1024 * 1024 {
            return Err("save too large".into());
        }
        let save: Save = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        if save.format != SAVE_VERSION || save.world.identity != Identity::default() {
            return Err("save version/content mismatch".into());
        }
        if save.world.hash() != save.hash {
            return Err("save checksum mismatch".into());
        }
        save.world.validate()?;
        Ok(save.world)
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.identity != Identity::default() {
            return Err("unsupported identity".into());
        }
        if !(1..=8).contains(&self.players.len()) || self.last_sequences.len() != self.players.len()
        {
            return Err("invalid player state".into());
        }
        if !(8..=256).contains(&self.map.width)
            || !(8..=256).contains(&self.map.height)
            || self.map.tiles.len() != self.map.width as usize * self.map.height as usize
        {
            return Err("invalid map".into());
        }
        if self.entities.len() > 10000 {
            return Err("too many entities".into());
        }
        for (id, e) in &self.entities {
            if *id != e.id
                || *id >= self.next_id
                || e.owner as usize >= self.players.len()
                || self.map.index(e.pos).is_none()
                || e.queue.len() > 5
                || e.hp <= 0
                || e.max_hp <= 0
                || e.hp > e.max_hp
            {
                return Err("invalid entity state".into());
            }
        }
        if self.players.iter().any(|p| {
            !(1..=4).contains(&p.age)
                || p.explored
                    .iter()
                    .chain(&p.visible)
                    .any(|pos| self.map.index(*pos).is_none())
        }) {
            return Err("invalid player data".into());
        }
        Ok(())
    }
}
impl Replay {
    pub fn play(&self) -> Result<World, String> {
        self.initial.validate()?;
        if self.format != SAVE_VERSION
            || self.end_tick < self.initial.tick
            || self.end_tick - self.initial.tick > 1_000_000
            || self.commands.len() > 1_000_000
        {
            return Err("unsupported replay or limits exceeded".into());
        }
        if self
            .commands
            .iter()
            .any(|c| c.tick < self.initial.tick || c.tick >= self.end_tick)
        {
            return Err("command outside replay".into());
        }
        let mut world = self.initial.clone();
        let mut by_tick: BTreeMap<u64, Vec<Command>> = BTreeMap::new();
        for cmd in &self.commands {
            by_tick.entry(cmd.tick).or_default().push(cmd.clone());
        }
        while world.tick < self.end_tick {
            if world.finished {
                return Err("replay continues after victory".into());
            }
            let errors = world.step(by_tick.get(&world.tick).map_or(&[], |v| v.as_slice()));
            if !errors.is_empty() {
                return Err(errors.join("; "));
            }
        }
        if world.hash() != self.final_hash {
            return Err("replay hash mismatch".into());
        }
        Ok(world)
    }
}

pub mod campaign;
