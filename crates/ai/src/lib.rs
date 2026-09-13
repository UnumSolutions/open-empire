//! AI uses the same validated commands as humans and networking.
use empire_content::{Kind, Resource};
use empire_sim::{Action, Command, Order, PlayerId, Pos, World};
#[derive(Debug, Clone, Copy)]
pub enum Difficulty {
    Easy,
    Normal,
    Hard,
}
pub fn commands(world: &World, p: PlayerId, difficulty: Difficulty) -> Vec<Command> {
    if world.finished || p as usize >= world.players.len() || world.players[p as usize].defeated {
        return vec![];
    }
    let interval = match difficulty {
        Difficulty::Easy => 30,
        Difficulty::Normal => 15,
        Difficulty::Hard => 5,
    };
    if world.tick % interval != 0 {
        return vec![];
    }
    let mut scratch = world.clone();
    let mut result = Vec::new();
    let mut submit = |action: Action| {
        let sequence = scratch.last_sequences[p as usize].map_or(0, |s| s + 1);
        let cmd = Command {
            tick: world.tick,
            player: p,
            sequence,
            action,
        };
        if scratch.apply(&cmd).is_ok() {
            result.push(cmd);
        }
    };
    let own: Vec<_> = world.entities.values().filter(|e| e.owner == p).collect();
    for e in &own {
        if e.kind == Kind::Villager && e.order == Order::Idle {
            let resource = match e.id % 5 {
                0 | 1 => Resource::Food,
                2 | 3 => Resource::Wood,
                _ => Resource::Gold,
            };
            let target = world.players[p as usize]
                .explored
                .iter()
                .filter(|pos| {
                    world
                        .map
                        .tile(**pos)
                        .is_some_and(|t| t.resource == Some(resource) && t.amount > 0)
                })
                .min_by_key(|pos| (e.pos.distance(**pos), **pos));
            if let Some(target) = target {
                submit(Action::Gather {
                    units: vec![e.id],
                    target: *target,
                });
            }
        }
        if !e.kind.stats().building
            && e.kind != Kind::Villager
            && e.kind != Kind::Monk
            && matches!(e.order, Order::Idle | Order::Move(_))
        {
            if let Some(t) = world
                .entities
                .values()
                .filter(|t| t.owner != p && world.players[p as usize].visible.contains(&t.pos))
                .min_by_key(|t| (e.pos.distance(t.pos), t.id))
            {
                submit(Action::Attack {
                    units: vec![e.id],
                    target: t.id,
                });
            } else if world.tick > 600 {
                let side = if p % 2 == 0 { 54 } else { 9 };
                submit(Action::Move {
                    units: vec![e.id],
                    target: Pos::new(side, side),
                });
            }
        }
    }
    if let Some(tc) = own.iter().find(|e| e.kind == Kind::TownCenter) {
        if own.iter().filter(|e| e.kind == Kind::Villager).count() < 14 && tc.queue.is_empty() {
            submit(Action::Train {
                building: tc.id,
                kind: Kind::Villager,
            });
        }
    }
    if let Some(worker) = own
        .iter()
        .find(|e| e.kind == Kind::Villager && !matches!(e.order, Order::Construct(_)))
    {
        let needed = if world.population(p) + world.pending_population(p) + 2 >= world.capacity(p) {
            Some(Kind::House)
        } else if !own.iter().any(|e| e.kind == Kind::Barracks) {
            Some(Kind::Barracks)
        } else if world.players[p as usize].age >= 2
            && !own.iter().any(|e| e.kind == Kind::ArcheryRange)
        {
            Some(Kind::ArcheryRange)
        } else {
            None
        };
        if let Some(kind) = needed {
            let site = world.players[p as usize]
                .visible
                .iter()
                .filter(|pos| {
                    world.passable(**pos, false)
                        && !world.entities.values().any(|e| e.pos == **pos)
                        && world.map.tile(**pos).is_some_and(|t| t.resource.is_none())
                })
                .min_by_key(|pos| (worker.pos.distance(**pos), **pos));
            if let Some(at) = site {
                submit(Action::Build {
                    worker: worker.id,
                    kind,
                    at: *at,
                });
            }
        }
    }
    submit(Action::AdvanceAge);
    for e in own {
        let kind = match e.kind {
            Kind::Barracks => Some(Kind::Spearman),
            Kind::ArcheryRange => Some(Kind::Archer),
            Kind::Stable => Some(Kind::Cavalry),
            _ => None,
        };
        if let Some(kind) = kind {
            if e.queue.len() < 2 {
                submit(Action::Train {
                    building: e.id,
                    kind,
                });
            }
        }
    }
    result
}
