//! Transport-independent lockstep state machine. Socket service uses framed JSON.
use empire_sim::{Command, Identity, PlayerId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
pub const PROTOCOL_VERSION: u32 = 1;
pub const MAX_FRAME_BYTES: usize = 64 * 1024;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hello {
    pub protocol: u32,
    pub identity: Identity,
    pub player: PlayerId,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Turn {
    pub tick: u64,
    pub commands: Vec<Command>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    Hello(Hello),
    Submit(Turn),
    Hash { tick: u64, hash: String },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    Welcome {
        player: PlayerId,
        players: u8,
    },
    Ready,
    Turn(Turn),
    Waiting {
        tick: u64,
    },
    Error(String),
    Desync {
        tick: u64,
        hashes: BTreeMap<PlayerId, String>,
    },
}
#[derive(Debug)]
pub struct Lockstep {
    pub identity: Identity,
    pub tick: u64,
    players: u8,
    joined: BTreeSet<PlayerId>,
    pending: BTreeMap<PlayerId, Vec<Command>>,
    last_sequence: BTreeMap<PlayerId, u64>,
    hashes: BTreeMap<u64, BTreeMap<PlayerId, String>>,
}
impl Lockstep {
    pub fn new(identity: Identity, players: u8) -> Result<Self, String> {
        if !(1..=8).contains(&players) {
            return Err("players must be 1–8".into());
        }
        Ok(Self {
            identity,
            tick: 0,
            players,
            joined: BTreeSet::new(),
            pending: BTreeMap::new(),
            last_sequence: BTreeMap::new(),
            hashes: BTreeMap::new(),
        })
    }
    pub fn join(&mut self, hello: Hello) -> Result<(), String> {
        if hello.protocol != PROTOCOL_VERSION || hello.identity != self.identity {
            return Err("protocol, engine, ruleset or content mismatch".into());
        }
        if hello.player >= self.players || self.joined.contains(&hello.player) {
            return Err("invalid or occupied player slot".into());
        }
        self.joined.insert(hello.player);
        Ok(())
    }
    pub fn ready(&self) -> bool {
        self.joined.len() == self.players as usize
    }
    /// Sender identity comes from the connection; never trust a command's player field.
    pub fn submit(&mut self, sender: PlayerId, turn: Turn) -> Result<Option<Turn>, String> {
        if !self.ready() || !self.joined.contains(&sender) {
            return Err("lobby not ready".into());
        }
        if turn.tick != self.tick || self.pending.contains_key(&sender) || turn.commands.len() > 64
        {
            return Err("invalid, duplicate, or oversized turn".into());
        }
        let mut sequence = self.last_sequence.get(&sender).copied();
        for cmd in &turn.commands {
            if cmd.player != sender
                || cmd.tick != self.tick
                || sequence.is_some_and(|s| cmd.sequence <= s)
            {
                return Err("spoofed player, wrong tick, or stale command".into());
            }
            sequence = Some(cmd.sequence);
        }
        if let Some(s) = sequence {
            self.last_sequence.insert(sender, s);
        }
        self.pending.insert(sender, turn.commands);
        if self.pending.len() != self.players as usize {
            return Ok(None);
        }
        let mut commands: Vec<_> = std::mem::take(&mut self.pending)
            .into_values()
            .flatten()
            .collect();
        commands.sort_by_key(|c| (c.player, c.sequence));
        let turn = Turn {
            tick: self.tick,
            commands,
        };
        self.tick += 1;
        Ok(Some(turn))
    }
    pub fn report_hash(
        &mut self,
        sender: PlayerId,
        tick: u64,
        hash: String,
    ) -> Result<Option<ServerMessage>, String> {
        if !self.joined.contains(&sender)
            || tick > self.tick
            || self.tick.saturating_sub(tick) > 100
            || hash.len() != 64
            || !hash.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err("invalid state hash".into());
        }
        self.hashes
            .retain(|t, _| *t >= self.tick.saturating_sub(100));
        let hashes = self.hashes.entry(tick).or_default();
        if hashes.contains_key(&sender) {
            return Err("duplicate state hash".into());
        }
        hashes.insert(sender, hash);
        if hashes.len() == self.players as usize
            && hashes.values().collect::<BTreeSet<_>>().len() > 1
        {
            return Ok(Some(ServerMessage::Desync {
                tick,
                hashes: hashes.clone(),
            }));
        }
        Ok(None)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use empire_sim::Action;
    fn room() -> Lockstep {
        let mut r = Lockstep::new(Identity::default(), 2).unwrap();
        for player in 0..2 {
            r.join(Hello {
                protocol: 1,
                identity: Identity::default(),
                player,
            })
            .unwrap();
        }
        r
    }
    #[test]
    fn waits_for_every_player_and_orders() {
        let mut r = room();
        assert!(
            r.submit(
                1,
                Turn {
                    tick: 0,
                    commands: vec![]
                }
            )
            .unwrap()
            .is_none()
        );
        let t = r
            .submit(
                0,
                Turn {
                    tick: 0,
                    commands: vec![],
                },
            )
            .unwrap()
            .unwrap();
        assert_eq!(t.tick, 0);
        assert_eq!(r.tick, 1);
    }
    #[test]
    fn rejects_spoof_and_mismatch() {
        let mut r = room();
        assert!(
            r.submit(
                0,
                Turn {
                    tick: 0,
                    commands: vec![Command {
                        tick: 0,
                        player: 1,
                        sequence: 0,
                        action: Action::Resign
                    }]
                }
            )
            .is_err()
        );
        let mut h = Identity::default();
        h.content_hash = "bad".into();
        assert!(
            Lockstep::new(Identity::default(), 2)
                .unwrap()
                .join(Hello {
                    protocol: 1,
                    identity: h,
                    player: 0
                })
                .is_err()
        );
    }
    #[test]
    fn detects_desync() {
        let mut r = room();
        assert!(r.report_hash(0, 0, "a".repeat(64)).unwrap().is_none());
        assert!(matches!(
            r.report_hash(1, 0, "b".repeat(64)).unwrap(),
            Some(ServerMessage::Desync { .. })
        ));
    }
}
