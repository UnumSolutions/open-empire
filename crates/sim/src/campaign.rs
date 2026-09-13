//! Bounded data-only objectives. No script execution or filesystem access.
use crate::{MapKind, PlayerId, World};
use empire_content::Kind;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Objective {
    Stockpile { resource: usize, amount: u32 },
    ReachAge(u8),
    Own { kind: Kind, count: usize },
    Survive(u64),
    DefeatEnemies,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mission {
    pub id: u8,
    pub title: String,
    pub briefing: String,
    pub objectives: Vec<Objective>,
}
impl Mission {
    pub fn original(id: u8) -> Result<Self, String> {
        let (title, briefing, objectives) = match id {
            1 => (
                "The Aster Frontier",
                "Gather food, establish a barracks, reach the second age, and hold the settlement.",
                vec![
                    Objective::Stockpile {
                        resource: 0,
                        amount: 200,
                    },
                    Objective::Own {
                        kind: Kind::Barracks,
                        count: 1,
                    },
                    Objective::ReachAge(2),
                    Objective::Survive(1200),
                ],
            ),
            2 => (
                "The Broken Gate",
                "Advance to the third age, train siege support, and defeat the rival settlement.",
                vec![
                    Objective::ReachAge(3),
                    Objective::Own {
                        kind: Kind::Ram,
                        count: 2,
                    },
                    Objective::DefeatEnemies,
                ],
            ),
            3 => (
                "The Veyran Crossing",
                "Establish a market and dock, assemble a fleet, and win the crossing.",
                vec![
                    Objective::Own {
                        kind: Kind::Market,
                        count: 1,
                    },
                    Objective::Own {
                        kind: Kind::Dock,
                        count: 1,
                    },
                    Objective::Own {
                        kind: Kind::Warship,
                        count: 3,
                    },
                    Objective::DefeatEnemies,
                ],
            ),
            _ => return Err("mission must be 1–3".into()),
        };
        Ok(Self {
            id,
            title: title.into(),
            briefing: briefing.into(),
            objectives,
        })
    }
    pub fn world(&self) -> Result<World, String> {
        let mut world = World::new(
            100 + self.id as u64,
            2,
            if self.id == 3 {
                MapKind::Mixed
            } else {
                MapKind::Land
            },
        )?;
        if self.id >= 2 {
            world.players[0].resources = [900, 1100, 800, 500];
            world.players[0].age = 2;
        }
        Ok(world)
    }
    pub fn progress(&self, world: &World, p: PlayerId) -> Result<Vec<bool>, String> {
        if self.objectives.len() > 64 {
            return Err("scenario objective budget exceeded".into());
        }
        let player = world
            .players
            .get(p as usize)
            .ok_or("invalid mission player")?;
        self.objectives
            .iter()
            .map(|o| {
                Ok(match o {
                    Objective::Stockpile { resource, amount } => {
                        *player.resources.get(*resource).ok_or("invalid resource")? >= *amount
                    }
                    Objective::ReachAge(age) => player.age >= *age,
                    Objective::Own { kind, count } => {
                        world
                            .entities
                            .values()
                            .filter(|e| e.owner == p && e.kind == *kind && e.construction == 0)
                            .count()
                            >= *count
                    }
                    Objective::Survive(tick) => world.tick >= *tick && !player.defeated,
                    Objective::DefeatEnemies => world.winner == Some(p),
                })
            })
            .collect()
    }
}
