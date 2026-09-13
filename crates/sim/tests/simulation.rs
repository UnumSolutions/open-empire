use empire_content::{Kind, Resource};
use empire_sim::*;
fn cmd(w: &World, action: Action) -> Command {
    Command {
        tick: w.tick,
        player: 0,
        sequence: w.last_sequences[0].map_or(0, |s| s + 1),
        action,
    }
}
#[test]
fn gathering_conserves_resources_and_replays() {
    let mut w = World::new(42, 2, MapKind::Land).unwrap();
    let initial = w.clone();
    let c = cmd(
        &w,
        Action::Gather {
            units: vec![2],
            target: Pos::new(6, 13),
        },
    );
    let before = w.players[0].resources[Resource::Food.index()]
        + w.map.tile(Pos::new(6, 13)).unwrap().amount;
    assert!(w.step(&[c.clone()]).is_empty());
    for _ in 0..199 {
        w.step(&[]);
    }
    assert!(w.players[0].resources[0] > 300);
    assert_eq!(
        before,
        w.players[0].resources[0] + w.map.tile(Pos::new(6, 13)).unwrap().amount
    );
    let replay = Replay {
        format: 1,
        initial,
        commands: vec![c],
        end_tick: w.tick,
        final_hash: w.hash(),
    };
    assert_eq!(replay.play().unwrap().hash(), w.hash());
}
#[test]
fn invalid_commands_are_atomic() {
    let mut w = World::new(42, 2, MapKind::Land).unwrap();
    let before = w.hash();
    assert!(
        w.apply(&cmd(
            &w,
            Action::Build {
                worker: 2,
                kind: Kind::Wonder,
                at: Pos::new(10, 10)
            }
        ))
        .is_err()
    );
    assert_eq!(w.hash(), before);
    assert!(
        w.apply(&cmd(
            &w,
            Action::Move {
                units: vec![2, 6],
                target: Pos::new(12, 12)
            }
        ))
        .is_err()
    );
    assert_eq!(w.hash(), before);
}
#[test]
fn build_train_age_and_population() {
    let mut w = World::new(1, 2, MapKind::Land).unwrap();
    w.players[0].resources = [10000; 4];
    let c = cmd(
        &w,
        Action::Build {
            worker: 2,
            kind: Kind::House,
            at: Pos::new(7, 10),
        },
    );
    w.apply(&c).unwrap();
    for _ in 0..150 {
        w.step(&[]);
    }
    assert_eq!(w.capacity(0), 20);
    let c = cmd(&w, Action::AdvanceAge);
    w.apply(&c).unwrap();
    for _ in 0..100 {
        w.step(&[]);
    }
    assert_eq!(w.players[0].age, 2);
    let c = cmd(
        &w,
        Action::Train {
            building: 1,
            kind: Kind::Villager,
        },
    );
    w.apply(&c).unwrap();
    for _ in 0..40 {
        w.step(&[]);
    }
    assert_eq!(w.population(0), 4);
}
#[test]
fn save_roundtrip_and_corruption() {
    let mut w = World::new(99, 8, MapKind::Mixed).unwrap();
    for _ in 0..20 {
        w.step(&[]);
    }
    let save = w.save().unwrap();
    let mut loaded = World::load(&save).unwrap();
    for _ in 0..100 {
        w.step(&[]);
        loaded.step(&[]);
    }
    assert_eq!(w.hash(), loaded.hash());
    let mut damaged = String::from_utf8(save).unwrap();
    damaged = damaged.replacen("original-0.1", "unsupported", 1);
    assert!(World::load(damaged.as_bytes()).is_err());
}
#[test]
fn command_order_does_not_depend_on_arrival_order() {
    let mut a = World::new(3, 2, MapKind::Land).unwrap();
    let mut b = a.clone();
    let c0 = cmd(
        &a,
        Action::Move {
            units: vec![2],
            target: Pos::new(8, 8),
        },
    );
    let c1 = Command {
        tick: 0,
        player: 1,
        sequence: 0,
        action: Action::Move {
            units: vec![6],
            target: Pos::new(52, 52),
        },
    };
    a.step(&[c0.clone(), c1.clone()]);
    b.step(&[c1, c0]);
    assert_eq!(a.hash(), b.hash());
}
#[test]
fn no_moving_through_buildings() {
    let mut w = World::new(3, 2, MapKind::Land).unwrap();
    let c = cmd(
        &w,
        Action::Move {
            units: vec![2],
            target: Pos::new(9, 7),
        },
    );
    w.apply(&c).unwrap();
    for _ in 0..120 {
        w.step(&[]);
        assert_ne!(w.entities[&2].pos, Pos::new(9, 9));
    }
    assert_eq!(w.entities[&2].pos, Pos::new(9, 7));
}
#[test]
fn resign_finishes_match() {
    let mut w = World::new(1, 2, MapKind::Land).unwrap();
    let c = cmd(&w, Action::Resign);
    w.step(&[c]);
    assert!(w.finished);
    assert_eq!(w.winner, Some(1));
}
#[test]
fn campaign_objectives_are_bounded() {
    let m = campaign::Mission::original(1).unwrap();
    assert_eq!(m.progress(&m.world().unwrap(), 0).unwrap().len(), 4);
    assert!(campaign::Mission::original(4).is_err());
}
