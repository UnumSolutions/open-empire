//! Native Bevy client. The simulation remains independent of the renderer.
use bevy::{
    input::mouse::MouseWheel,
    prelude::*,
    render::{
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
    },
    window::PrimaryWindow,
};
use empire_content::{Kind, Resource as Goods};
use empire_sim::{
    Action, Command as SimCommand, EntityId, MapKind, Pos, Replay, Terrain, World as SimWorld,
    campaign::Mission,
};
use std::collections::BTreeMap;

const GOLD: Color = Color::srgb(0.88, 0.72, 0.39);
const INK: Color = Color::srgb(0.045, 0.066, 0.085);
const CREAM: Color = Color::srgb(0.91, 0.89, 0.80);
#[derive(Resource)]
struct Game {
    world: SimWorld,
    initial: SimWorld,
    history: Vec<SimCommand>,
    pending: Vec<SimCommand>,
    selected: Vec<EntityId>,
    build: Option<Kind>,
    paused: bool,
    message: String,
    mission: Option<Mission>,
    mission_done: Vec<bool>,
    groups: BTreeMap<u8, Vec<EntityId>>,
    drag: Option<Vec2>,
    last_touches: Vec<Vec2>,
}
impl Game {
    fn new(mission: Option<Mission>) -> Self {
        let world = mission
            .as_ref()
            .map_or_else(|| SimWorld::new(42, 2, MapKind::Land), |m| m.world())
            .unwrap();
        let initial = world.clone();
        Self {
            world,
            initial,
            history: vec![],
            pending: vec![],
            selected: vec![],
            build: None,
            paused: false,
            message: "Select your villagers, then right-click a resource to gather.".into(),
            mission,
            mission_done: vec![],
            groups: BTreeMap::new(),
            drag: None,
            last_touches: vec![],
        }
    }
    fn issue(&mut self, action: Action) {
        let sequence = self
            .pending
            .last()
            .map(|c| c.sequence + 1)
            .unwrap_or_else(|| self.world.last_sequences[0].map_or(0, |s| s + 1));
        self.pending.push(SimCommand {
            tick: self.world.tick,
            player: 0,
            sequence,
            action,
        });
    }
    fn selected_units(&self) -> Vec<EntityId> {
        self.selected
            .iter()
            .filter(|id| {
                self.world
                    .entities
                    .get(id)
                    .is_some_and(|e| e.owner == 0 && !e.kind.stats().building)
            })
            .copied()
            .collect()
    }
}
#[derive(Resource)]
struct Art {
    tile: Handle<Image>,
    unit: Handle<Image>,
    building: Handle<Image>,
    tree: Handle<Image>,
    stone: Handle<Image>,
}
#[derive(Component)]
struct Ground(Pos);
#[derive(Component)]
struct Piece(EntityId);
#[derive(Component)]
struct Decoration(Pos);
#[derive(Component)]
struct Hud;
#[derive(Component)]
struct Details;
#[derive(Component)]
struct Hint;
#[derive(Component, Clone)]
enum ButtonAction {
    Train(Kind),
    Build(Kind),
    Age,
    Pause,
    New,
    SelectVillagers,
    SelectArmy,
    Mission(u8),
    Save,
    Load,
    Stop,
}
fn iso(p: Pos) -> Vec2 {
    Vec2::new((p.x - p.y) as f32 * 32., -(p.x + p.y) as f32 * 16.)
}
fn uniso(v: Vec2) -> Pos {
    Pos::new(
        (v.x / 64. - v.y / 32.).round() as i16,
        (-v.x / 64. - v.y / 32.).round() as i16,
    )
}

pub fn run() {
    App::new()
        .insert_resource(ClearColor(INK))
        .insert_resource(Game::new(None))
        .insert_resource(Time::<Fixed>::from_hz(10.))
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Open Empire — Aster Frontier".into(),
                        resolution: (1440., 900.).into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .add_systems(Startup, setup)
        .add_systems(FixedUpdate, simulate)
        .add_systems(
            Update,
            (button_input, keyboard_input, pointer_input, touch_input).chain(),
        )
        .add_systems(Update, (draw_world, update_hud).after(pointer_input))
        .run();
}
fn texture(images: &mut Assets<Image>, shape: u8) -> Handle<Image> {
    let (w, h) = (64u32, 80u32);
    let mut bytes = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let dx = x as i32 - 32;
            let yy = y as i32;
            let inside = match shape {
                0 => dx.abs() * 2 + (yy - 40).abs() * 4 < 64,
                1 => {
                    (dx * dx + (yy - 20) * (yy - 20) < 49)
                        || (dx.abs() < 6 && (25..52).contains(&yy))
                        || ((dx.abs() - 5).abs() < 3 && (49..68).contains(&yy))
                }
                2 => {
                    (dx.abs() < 25 && (30..68).contains(&yy))
                        || (yy >= 8 && yy < 32 && dx.abs() < yy - 4)
                }
                3 => {
                    (dx.abs() < 4 && (48..74).contains(&yy))
                        || (yy > 4 && yy < 57 && dx.abs() < (yy - 4) / 2)
                }
                _ => dx * dx * 2 + (yy - 48) * (yy - 48) * 3 < 500,
            };
            if inside {
                let i = ((y * w + x) * 4) as usize;
                let shade = if shape == 2 && yy < 32 {
                    160
                } else if x < 32 {
                    235
                } else {
                    195
                };
                bytes[i..i + 4].copy_from_slice(&[shade, shade, shade, 255]);
            }
        }
    }
    images.add(Image::new(
        Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        bytes,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    ))
}
fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>, game: Res<Game>) {
    commands.spawn((Camera2d, Transform::from_xyz(0., -300., 0.)));
    let art = Art {
        tile: texture(&mut images, 0),
        unit: texture(&mut images, 1),
        building: texture(&mut images, 2),
        tree: texture(&mut images, 3),
        stone: texture(&mut images, 4),
    };
    for y in 0..game.world.map.height {
        for x in 0..game.world.map.width {
            let p = Pos::new(x, y);
            commands.spawn((
                Sprite::from_image(art.tile.clone()),
                Transform::from_translation(iso(p).extend(-100.)),
                Ground(p),
            ));
            let t = game.world.map.tile(p).unwrap();
            if let Some(r) = t.resource {
                let image = if r == Goods::Wood {
                    art.tree.clone()
                } else {
                    art.stone.clone()
                };
                commands.spawn((
                    Sprite::from_image(image),
                    Transform::from_translation(
                        (iso(p) + Vec2::Y * 18.).extend(-(x + y) as f32 / 100.),
                    ),
                    Decoration(p),
                ));
            }
        }
    }
    commands.insert_resource(art);
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.),
                right: Val::Px(0.),
                top: Val::Px(0.),
                height: Val::Px(80.),
                padding: UiRect::axes(Val::Px(28.), Val::Px(16.)),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(INK),
        ))
        .with_children(|p| {
            p.spawn((
                Text::new("OPEN EMPIRE"),
                TextFont {
                    font_size: 25.,
                    ..default()
                },
                TextColor(GOLD),
            ));
            p.spawn((
                Text::new(""),
                TextFont {
                    font_size: 19.,
                    ..default()
                },
                TextColor(CREAM),
                Hud,
            ));
        });
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(24.),
                top: Val::Px(96.),
                width: Val::Px(270.),
                padding: UiRect::all(Val::Px(18.)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.035, 0.05, 0.06, 0.93)),
        ))
        .with_children(|p| {
            p.spawn((
                Text::new(""),
                TextFont {
                    font_size: 16.,
                    ..default()
                },
                TextColor(CREAM),
                Details,
            ));
        });
    commands
        .spawn((Node {
            position_type: PositionType::Absolute,
            right: Val::Px(24.),
            top: Val::Px(96.),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(6.),
            ..default()
        },))
        .with_children(|p| {
            for (label, action) in [
                ("New skirmish", ButtonAction::New),
                ("1 · The Aster Frontier", ButtonAction::Mission(1)),
                ("2 · The Broken Gate", ButtonAction::Mission(2)),
                ("3 · The Veyran Crossing", ButtonAction::Mission(3)),
                ("Pause / Resume", ButtonAction::Pause),
                ("Save · F5", ButtonAction::Save),
                ("Load · F9", ButtonAction::Load),
            ] {
                button(p, label, action);
            }
        });
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(0.),
                left: Val::Px(0.),
                right: Val::Px(0.),
                min_height: Val::Px(185.),
                padding: UiRect::all(Val::Px(18.)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(10.),
                ..default()
            },
            BackgroundColor(INK),
        ))
        .with_children(|p| {
            p.spawn((
                Text::new(""),
                TextFont {
                    font_size: 16.,
                    ..default()
                },
                TextColor(GOLD),
                Hint,
            ));
            p.spawn(Node {
                flex_wrap: FlexWrap::Wrap,
                column_gap: Val::Px(6.),
                row_gap: Val::Px(5.),
                ..default()
            })
            .with_children(|p| {
                for (label, action) in [
                    ("Villagers", ButtonAction::SelectVillagers),
                    ("Army", ButtonAction::SelectArmy),
                    ("Train worker", ButtonAction::Train(Kind::Villager)),
                    ("Spearman", ButtonAction::Train(Kind::Spearman)),
                    ("Archer", ButtonAction::Train(Kind::Archer)),
                    ("Cavalry", ButtonAction::Train(Kind::Cavalry)),
                    ("Ram", ButtonAction::Train(Kind::Ram)),
                    ("Monk", ButtonAction::Train(Kind::Monk)),
                    ("Warship", ButtonAction::Train(Kind::Warship)),
                    ("Advance age", ButtonAction::Age),
                    ("Stop", ButtonAction::Stop),
                ] {
                    button(p, label, action);
                }
            });
            p.spawn(Node {
                flex_wrap: FlexWrap::Wrap,
                column_gap: Val::Px(6.),
                row_gap: Val::Px(5.),
                ..default()
            })
            .with_children(|p| {
                for kind in [
                    Kind::House,
                    Kind::Barracks,
                    Kind::ArcheryRange,
                    Kind::Stable,
                    Kind::Workshop,
                    Kind::Monastery,
                    Kind::Dock,
                    Kind::Market,
                    Kind::Tower,
                    Kind::TownCenter,
                    Kind::Wonder,
                ] {
                    button(p, &format!("{kind:?}"), ButtonAction::Build(kind));
                }
            });
        });
}
fn button(parent: &mut ChildSpawnerCommands, label: &str, action: ButtonAction) {
    parent
        .spawn((
            Button,
            action,
            Node {
                padding: UiRect::axes(Val::Px(12.), Val::Px(9.)),
                min_height: Val::Px(42.),
                border: UiRect::all(Val::Px(1.)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.095, 0.13, 0.15)),
            BorderColor(Color::srgb(0.22, 0.28, 0.28)),
        ))
        .with_children(|p| {
            p.spawn((
                Text::new(label),
                TextFont {
                    font_size: 14.,
                    ..default()
                },
                TextColor(CREAM),
            ));
        });
}
fn activate(game: &mut Game, action: &ButtonAction) {
    match action {
        ButtonAction::New => *game = Game::new(None),
        ButtonAction::Mission(id) => *game = Game::new(Some(Mission::original(*id).unwrap())),
        ButtonAction::Pause => game.paused = !game.paused,
        ButtonAction::Age => game.issue(Action::AdvanceAge),
        ButtonAction::Build(kind) => {
            game.build = Some(*kind);
            game.message = format!(
                "Place {kind:?}: select a villager, then click a visible free tile. Esc cancels."
            );
        }
        ButtonAction::Train(kind) => {
            let producer = kind.producer();
            let id = game
                .selected
                .iter()
                .filter_map(|id| game.world.entities.get(id))
                .find(|e| e.owner == 0 && Some(e.kind) == producer)
                .or_else(|| {
                    game.world
                        .entities
                        .values()
                        .find(|e| e.owner == 0 && Some(e.kind) == producer)
                })
                .map(|e| e.id);
            if let Some(building) = id {
                game.issue(Action::Train {
                    building,
                    kind: *kind,
                });
            } else {
                game.message = format!("Build a {:?} first.", producer.unwrap());
            }
        }
        ButtonAction::SelectVillagers => {
            game.selected = game
                .world
                .entities
                .values()
                .filter(|e| e.owner == 0 && e.kind == Kind::Villager)
                .map(|e| e.id)
                .collect()
        }
        ButtonAction::SelectArmy => {
            game.selected = game
                .world
                .entities
                .values()
                .filter(|e| e.owner == 0 && !e.kind.stats().building && e.kind != Kind::Villager)
                .map(|e| e.id)
                .collect()
        }
        ButtonAction::Stop => game.issue(Action::Stop {
            units: game.selected_units(),
        }),
        ButtonAction::Save => {
            game.message = save_game(game)
                .map_or_else(|e| e, |_| "Saved to saves/quicksave.empire-save".into());
        }
        ButtonAction::Load => {
            match std::fs::read("saves/quicksave.empire-save")
                .map_err(|e| e.to_string())
                .and_then(|b| SimWorld::load(&b))
            {
                Ok(w) => {
                    game.world = w.clone();
                    game.initial = w;
                    game.history.clear();
                    game.pending.clear();
                    game.selected.clear();
                    game.mission = None;
                    game.message = "Save restored.".into();
                }
                Err(e) => game.message = e,
            }
        }
    }
}
fn save_game(game: &Game) -> Result<(), String> {
    std::fs::create_dir_all("saves").map_err(|e| e.to_string())?;
    let bytes = game.world.save()?;
    std::fs::write("saves/quicksave.tmp", bytes).map_err(|e| e.to_string())?;
    std::fs::rename("saves/quicksave.tmp", "saves/quicksave.empire-save")
        .map_err(|e| e.to_string())?;
    let replay = Replay {
        format: 1,
        initial: game.initial.clone(),
        commands: game.history.clone(),
        end_tick: game.world.tick,
        final_hash: game.world.hash(),
    };
    std::fs::write(
        "saves/quicksave.empire-replay",
        serde_json::to_vec(&replay).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}
fn button_input(
    mut game: ResMut<Game>,
    mut buttons: Query<(&Interaction, &ButtonAction, &mut BackgroundColor), Changed<Interaction>>,
) {
    for (interaction, action, mut color) in &mut buttons {
        match interaction {
            Interaction::Pressed => {
                activate(&mut game, action);
                color.0 = Color::srgb(0.30, 0.26, 0.15);
            }
            Interaction::Hovered => color.0 = Color::srgb(0.18, 0.23, 0.24),
            Interaction::None => color.0 = Color::srgb(0.095, 0.13, 0.15),
        }
    }
}
fn keyboard_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut game: ResMut<Game>,
    time: Res<Time>,
    mut camera: Query<(&mut Transform, &mut Projection), With<Camera2d>>,
    mut wheel: EventReader<MouseWheel>,
) {
    if keys.just_pressed(KeyCode::Space) {
        game.paused = !game.paused;
    }
    if keys.just_pressed(KeyCode::Escape) {
        game.build = None;
        game.selected.clear();
    }
    if keys.just_pressed(KeyCode::F5) {
        activate(&mut game, &ButtonAction::Save);
    }
    if keys.just_pressed(KeyCode::F9) {
        activate(&mut game, &ButtonAction::Load);
    }
    for (key, kind) in [
        (KeyCode::KeyH, Kind::House),
        (KeyCode::KeyB, Kind::Barracks),
        (KeyCode::KeyR, Kind::ArcheryRange),
        (KeyCode::KeyS, Kind::Stable),
        (KeyCode::KeyD, Kind::Dock),
    ] {
        if keys.just_pressed(key) {
            activate(&mut game, &ButtonAction::Build(kind));
        }
    }
    for (i, key) in [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
        KeyCode::Digit7,
        KeyCode::Digit8,
        KeyCode::Digit9,
    ]
    .iter()
    .enumerate()
    {
        if keys.just_pressed(*key) {
            if keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::SuperLeft) {
                let selection = game.selected.clone();
                game.groups.insert(i as u8, selection);
            } else {
                game.selected = game.groups.get(&(i as u8)).cloned().unwrap_or_default();
            }
        }
    }
    if let Ok((mut t, mut projection)) = camera.single_mut() {
        let mut movement = Vec2::ZERO;
        if keys.pressed(KeyCode::ArrowLeft) {
            movement.x -= 1.;
        }
        if keys.pressed(KeyCode::ArrowRight) {
            movement.x += 1.;
        }
        if keys.pressed(KeyCode::ArrowUp) {
            movement.y += 1.;
        }
        if keys.pressed(KeyCode::ArrowDown) {
            movement.y -= 1.;
        }
        t.translation += (movement * time.delta_secs() * 650.).extend(0.);
        if let Projection::Orthographic(ref mut p) = *projection {
            for e in wheel.read() {
                p.scale = (p.scale * (1. - e.y * 0.08)).clamp(0.4, 2.5);
            }
        }
    }
}
fn in_field(p: Vec2, window: &Window) -> bool {
    p.y > 85.
        && p.y < window.height() - 200.
        && !(p.x < 300. && p.y < 390.)
        && !(p.x > window.width() - 245. && p.y < 460.)
}
fn context(game: &mut Game, p: Pos) {
    let units = game.selected_units();
    if units.is_empty() {
        return;
    }
    let target = game
        .world
        .entities
        .values()
        .find(|e| {
            e.pos.distance(p) <= 1 && e.owner != 0 && game.world.players[0].visible.contains(&e.pos)
        })
        .map(|e| e.id);
    if let Some(target) = target {
        game.issue(Action::Attack { units, target });
    } else if game
        .world
        .map
        .tile(p)
        .is_some_and(|t| t.resource.is_some() && t.amount > 0)
    {
        let workers = units
            .into_iter()
            .filter(|id| game.world.entities[id].kind == Kind::Villager)
            .collect();
        game.issue(Action::Gather {
            units: workers,
            target: p,
        });
    } else {
        game.issue(Action::Move { units, target: p });
    }
}
fn place(game: &mut Game, p: Pos) -> bool {
    if let Some(kind) = game.build {
        if let Some(worker) = game
            .selected
            .iter()
            .find(|id| {
                game.world
                    .entities
                    .get(id)
                    .is_some_and(|e| e.kind == Kind::Villager && e.owner == 0)
            })
            .copied()
        {
            game.issue(Action::Build {
                worker,
                kind,
                at: p,
            });
            game.build = None;
        } else {
            game.message = "Select a villager before placing a building.".into();
        }
        true
    } else {
        false
    }
}
fn pointer_input(
    window: Query<&Window, With<PrimaryWindow>>,
    camera: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut game: ResMut<Game>,
) {
    let (Ok(window), Ok((camera, transform))) = (window.single(), camera.single()) else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Ok(world_pos) = camera.viewport_to_world_2d(transform, cursor) else {
        return;
    };
    let p = uniso(world_pos);
    if mouse.just_pressed(MouseButton::Left) && in_field(cursor, window) {
        if !place(&mut game, p) {
            game.drag = Some(world_pos);
        }
    }
    if mouse.just_released(MouseButton::Left) {
        if let Some(start) = game.drag.take() {
            let mut found: Vec<_> = if start.distance(world_pos) > 12. {
                let min = start.min(world_pos);
                let max = start.max(world_pos);
                game.world
                    .entities
                    .values()
                    .filter(|e| {
                        e.owner == 0
                            && !e.kind.stats().building
                            && iso(e.pos).cmpge(min).all()
                            && iso(e.pos).cmple(max).all()
                    })
                    .map(|e| e.id)
                    .collect()
            } else {
                game.world
                    .entities
                    .values()
                    .filter(|e| e.owner == 0 && iso(e.pos).distance(world_pos) < 30.)
                    .min_by_key(|e| e.pos.distance(p))
                    .map(|e| vec![e.id])
                    .unwrap_or_default()
            };
            if keys.pressed(KeyCode::ShiftLeft) {
                found.extend(&game.selected);
                found.sort();
                found.dedup();
            }
            game.selected = found;
        }
    }
    if mouse.just_pressed(MouseButton::Right) && in_field(cursor, window) {
        context(&mut game, p);
    }
}
fn touch_input(
    touches: Res<Touches>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut cameras: Query<
        (&Camera, &GlobalTransform, &mut Transform, &mut Projection),
        With<Camera2d>,
    >,
    mut game: ResMut<Game>,
) {
    let Ok(window) = window.single() else {
        return;
    };
    let Ok((camera, global, mut t, mut projection)) = cameras.single_mut() else {
        return;
    };
    let current: Vec<_> = touches.iter().map(|t| t.position()).collect();
    if current.len() == 2 && game.last_touches.len() == 2 {
        let old_center = (game.last_touches[0] + game.last_touches[1]) / 2.;
        let new_center = (current[0] + current[1]) / 2.;
        let delta = new_center - old_center;
        if let Projection::Orthographic(ref mut p) = *projection {
            t.translation += Vec3::new(-delta.x * p.scale, delta.y * p.scale, 0.);
            let old = game.last_touches[0].distance(game.last_touches[1]);
            let new = current[0].distance(current[1]);
            if new > 10. {
                p.scale = (p.scale * old / new).clamp(0.4, 2.5);
            }
        }
    }
    if game.last_touches.len() < 2 {
        for touch in touches.iter_just_released() {
            if touch.distance().length() < 15. && in_field(touch.position(), window) {
                if let Ok(v) = camera.viewport_to_world_2d(global, touch.position()) {
                    let pos = uniso(v);
                    if !place(&mut game, pos) {
                        let own = game
                            .world
                            .entities
                            .values()
                            .filter(|e| e.owner == 0 && iso(e.pos).distance(v) < 30.)
                            .min_by_key(|e| e.pos.distance(pos))
                            .map(|e| e.id);
                        if let Some(id) = own {
                            game.selected = vec![id];
                        } else {
                            context(&mut game, pos);
                        }
                    }
                }
            }
        }
    }
    game.last_touches = current;
}
fn simulate(mut game: ResMut<Game>) {
    if game.paused || game.world.finished {
        return;
    }
    let pending = std::mem::take(&mut game.pending);
    let mut accepted = Vec::new();
    for cmd in pending {
        match game.world.apply(&cmd) {
            Ok(()) => accepted.push(cmd),
            Err(e) => game.message = e,
        }
    }
    let bots = empire_ai::commands(&game.world, 1, empire_ai::Difficulty::Normal);
    for cmd in bots {
        if game.world.apply(&cmd).is_ok() {
            accepted.push(cmd);
        }
    }
    game.history.extend(accepted);
    game.world.step(&[]);
    if let Some(m) = &game.mission {
        if let Ok(progress) = m.progress(&game.world, 0) {
            if game.mission_done.len() != progress.len() {
                game.mission_done = vec![false; progress.len()];
            }
            for (i, done) in progress.into_iter().enumerate() {
                game.mission_done[i] |= done;
            }
            if game.mission_done.iter().all(|b| *b) {
                game.message = "Mission complete! Choose the next chapter.".into();
            }
        }
    }
}
fn draw_world(
    mut commands: Commands,
    game: Res<Game>,
    art: Res<Art>,
    mut ground: Query<(&Ground, &mut Sprite), Without<Piece>>,
    mut decor: Query<
        (&Decoration, &mut Sprite, &mut Visibility),
        (Without<Ground>, Without<Piece>),
    >,
    mut pieces: Query<
        (Entity, &Piece, &mut Sprite, &mut Transform, &mut Visibility),
        Without<Ground>,
    >,
) {
    let player = &game.world.players[0];
    for (Ground(p), mut sprite) in &mut ground {
        let visible = player.visible.contains(p);
        let explored = player.explored.contains(p);
        let tile = game.world.map.tile(*p).unwrap();
        let variation = ((p.x * 13 + p.y * 7) % 5) as f32 * 0.014;
        let (r, g, b) = match tile.terrain {
            Terrain::Water => (0.10, 0.28, 0.36),
            _ => (0.30 + variation, 0.40 + variation, 0.23 + variation),
        };
        let light = if visible {
            1.
        } else if explored {
            0.35
        } else {
            0.08
        };
        sprite.color = Color::srgb(r * light, g * light, b * light);
    }
    for (Decoration(p), mut sprite, mut visibility) in &mut decor {
        let tile = game.world.map.tile(*p).unwrap();
        *visibility = if player.explored.contains(p) && tile.amount > 0 && tile.resource.is_some() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        sprite.color = match tile.resource {
            Some(Goods::Wood) => Color::srgb(0.18, 0.37, 0.19),
            Some(Goods::Food) => Color::srgb(0.65, 0.31, 0.19),
            Some(Goods::Gold) => GOLD,
            _ => Color::srgb(0.52, 0.55, 0.57),
        };
    }
    let mut existing = std::collections::BTreeSet::new();
    for (entity, Piece(id), mut sprite, mut transform, mut visibility) in &mut pieces {
        let Some(e) = game.world.entities.get(id) else {
            commands.entity(entity).despawn();
            continue;
        };
        existing.insert(*id);
        *visibility = if e.owner == 0 || player.visible.contains(&e.pos) {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        sprite.image = if e.kind.stats().building {
            art.building.clone()
        } else {
            art.unit.clone()
        };
        sprite.color = if game.selected.contains(id) {
            Color::srgb(1., 0.90, 0.47)
        } else if e.construction > 0 {
            Color::srgb(0.48, 0.41, 0.30)
        } else if e.owner == 0 {
            Color::srgb(0.48, 0.74, 0.92)
        } else {
            Color::srgb(0.90, 0.36, 0.27)
        };
        transform.translation =
            (iso(e.pos) + Vec2::Y * 20.).extend(10. - (e.pos.x + e.pos.y) as f32 / 100.);
        transform.scale = Vec3::splat(if e.kind.stats().building {
            1.0
        } else if e.kind == Kind::Cavalry || e.kind == Kind::Ram {
            0.9
        } else {
            0.65
        });
    }
    for e in game.world.entities.values() {
        if !existing.contains(&e.id) {
            commands.spawn((
                Sprite::from_image(if e.kind.stats().building {
                    art.building.clone()
                } else {
                    art.unit.clone()
                }),
                Transform::from_translation((iso(e.pos) + Vec2::Y * 20.).extend(10.)),
                Piece(e.id),
            ));
        }
    }
}
fn update_hud(
    game: Res<Game>,
    mut hud: Query<&mut Text, With<Hud>>,
    mut details: Query<&mut Text, (With<Details>, Without<Hud>)>,
    mut hint: Query<&mut Text, (With<Hint>, Without<Hud>, Without<Details>)>,
) {
    let p = &game.world.players[0];
    if let Ok(mut text) = hud.single_mut() {
        text.0 = format!(
            "FOOD {}     WOOD {}     GOLD {}     STONE {}     {}/{}     AGE {}",
            p.resources[0],
            p.resources[1],
            p.resources[2],
            p.resources[3],
            game.world.population(0),
            game.world.capacity(0),
            p.age
        );
    }
    if let Ok(mut text) = details.single_mut() {
        let title = game
            .mission
            .as_ref()
            .map_or("SKIRMISH", |m| m.title.as_str());
        let selection = game
            .selected
            .iter()
            .filter_map(|id| game.world.entities.get(id))
            .take(4)
            .map(|e| format!("{:?}   {}/{} HP", e.kind, e.hp, e.max_hp))
            .collect::<Vec<_>>()
            .join("\n");
        let mission = game
            .mission
            .as_ref()
            .map(|m| {
                format!(
                    "\n\n{}\nObjectives: {} / {}",
                    m.briefing,
                    game.mission_done.iter().filter(|b| **b).count(),
                    m.objectives.len()
                )
            })
            .unwrap_or_default();
        text.0 = format!(
            "{}\n{}\n\n{}:{:02}  •  {} selected\n{}{}\n\nArrows · Pan\nScroll · Zoom\nShift · Add selection\nCtrl + 1–9 · Save group\n1–9 · Recall group\nSpace · Pause",
            title,
            p.civilization.name(),
            game.world.tick / 600,
            game.world.tick / 10 % 60,
            game.selected.len(),
            selection,
            mission
        );
    }
    if let Ok(mut text) = hint.single_mut() {
        text.0 = if game.world.finished {
            format!(
                "MATCH COMPLETE · {}",
                if game.world.winner == Some(0) {
                    "Victory"
                } else {
                    "Defeat"
                }
            )
        } else if game.paused {
            "PAUSED · Press Space or Resume to continue".into()
        } else {
            game.message.clone()
        };
    }
}
