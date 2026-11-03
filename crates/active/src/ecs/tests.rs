//! The ECS, system by system.

// Written like Bevy code: `Query<&T>`, not `Query<'_, '_, &T>`.
#![allow(elided_lifetimes_in_paths)]

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use super::prelude::*;

#[derive(Debug, PartialEq)]
struct Position(f32);
#[derive(Debug, PartialEq)]
struct Velocity(f32);
struct Player;
struct Enemy;
crate::component!(Position, Velocity, Player, Enemy);

#[derive(Default)]
struct Log(Vec<String>);
crate::resource!(Log);

fn log(app: &App) -> Vec<String> {
    app.world().resource::<Log>().0.clone()
}

#[test]
fn queries_filter_and_change_components() {
    let mut app = App::new();
    app.add_systems(
        Update,
        |mut q: Query<(&mut Position, &Velocity), Without<Enemy>>| {
            for (mut p, v) in &mut q {
                p.0 += v.0;
            }
        },
    );
    let world = app.world_mut();
    let a = world.spawn((Position(0.0), Velocity(1.0), Player)).id();
    let b = world.spawn((Position(0.0), Velocity(5.0), Enemy)).id();
    let c = world.spawn(Position(10.0)).id();
    app.update();
    app.update();
    let world = app.world();
    assert_eq!(*world.get::<Position>(a).unwrap(), Position(2.0));
    assert_eq!(*world.get::<Position>(b).unwrap(), Position(0.0));
    assert_eq!(*world.get::<Position>(c).unwrap(), Position(10.0));
    // Option, Has, Entity, With, Or.
    let rows: Vec<(Entity, bool, bool)> = world
        .query::<(Entity, Has<Player>, Option<&Velocity>)>()
        .iter(world)
        .map(|(e, player, v)| (e, player, v.is_some()))
        .collect();
    assert_eq!(rows, [(a, true, true), (b, false, true), (c, false, false)]);
    let either = world
        .query_filtered::<Entity, Or<(With<Player>, With<Enemy>)>>()
        .iter(world)
        .count();
    assert_eq!(either, 2);
}

#[test]
fn changed_and_added_see_what_happened_since_the_last_run() {
    let mut app = App::new();
    app.init_resource::<Log>();
    app.add_systems(
        Update,
        |added: Query<&Position, Added<Position>>,
         changed: Query<&Position, Changed<Position>>,
         mut log: ResMut<Log>| {
            let added: Vec<f32> = added.iter().map(|p| p.0).collect();
            let changed: Vec<f32> = changed.iter().map(|p| p.0).collect();
            log.0.push(format!("{added:?} {changed:?}"));
        },
    );
    let e = app.world_mut().spawn(Position(1.0)).id();
    app.update();
    app.update();
    app.world().get_mut::<Position>(e).unwrap().0 = 2.0;
    app.update();
    assert_eq!(log(&app), ["[1.0] [1.0]", "[] []", "[] [2.0]"]);
}

#[test]
fn mut_marks_changed_only_when_written() {
    let mut app = App::new();
    app.init_resource::<Log>();
    app.add_systems(
        Update,
        (
            |mut q: Query<&mut Position>| {
                for p in &mut q {
                    // Read only: not changed.
                    let _ = p.0;
                }
            },
            |q: Query<Entity, Changed<Position>>, mut log: ResMut<Log>| {
                log.0.push(q.count().to_string())
            },
        ),
    );
    app.world_mut().spawn(Position(1.0));
    app.update();
    app.update();
    assert_eq!(log(&app), ["1", "0"]);
}

#[test]
fn commands_spawn_insert_remove_and_despawn_after_the_system() {
    let mut app = App::new();
    app.add_systems(Startup, |mut commands: Commands| {
        let e = commands.spawn(Position(1.0)).id();
        commands.entity(e).insert(Velocity(2.0));
        commands.spawn((Position(5.0), Enemy));
    });
    app.add_systems(
        Update,
        |mut commands: Commands, q: Query<(Entity, &Position)>| {
            for (e, p) in &q {
                if p.0 > 4.0 {
                    commands.entity(e).despawn();
                } else {
                    commands.entity(e).remove::<Velocity>();
                }
            }
        },
    );
    app.update();
    let world = app.world();
    assert_eq!(world.entity_count(), 1);
    let (p, v) = world
        .query::<(&Position, Option<&Velocity>)>()
        .single(world)
        .unwrap();
    assert_eq!((p.0, v.is_some()), (1.0, false));
}

#[test]
fn despawned_entities_are_gone_and_their_index_comes_back_newer() {
    let mut world = World::new();
    let a = world.spawn(Position(1.0)).id();
    assert!(world.despawn(a));
    assert!(!world.contains(a) && world.get::<Position>(a).is_none());
    let b = world.spawn(Position(2.0)).id();
    assert_eq!(b.index(), a.index());
    assert_ne!(a, b);
    assert!(
        !world.despawn(a),
        "the old id does not despawn the new entity"
    );
    assert_eq!(world.get::<Position>(b).unwrap().0, 2.0);
}

#[test]
fn children_follow_their_parent_and_go_with_it() {
    let mut app = App::new();
    app.add_plugins(TransformPlugin);
    let world = app.world_mut();
    let parent = world
        .spawn(
            Transform::from_xyz(1.0, 0.0, 0.0)
                .with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
        )
        .id();
    let child = world
        .spawn((Transform::from_xyz(0.0, 0.0, 2.0), ChildOf(parent)))
        .id();
    let grandchild = world
        .spawn((Transform::from_xyz(0.0, 1.0, 0.0), ChildOf(child)))
        .id();
    app.update();
    let world = app.world();
    assert_eq!(world.get::<Children>(parent).unwrap().to_vec(), [child]);
    let at = world
        .get::<GlobalTransform>(grandchild)
        .unwrap()
        .translation();
    // Turned a quarter around y: +z becomes +x.
    assert!((at - Vec3::new(3.0, 1.0, 0.0)).length() < 1e-5, "{at:?}");
    // Reparenting updates both lists.
    let other = app.world_mut().spawn(Transform::IDENTITY).id();
    app.world_mut().insert(child, ChildOf(other));
    assert!(app.world().get::<Children>(parent).is_none());
    assert_eq!(
        app.world().get::<Children>(other).unwrap().to_vec(),
        [child]
    );
    assert!(app.world_mut().despawn(other));
    assert!(!app.world().contains(child) && !app.world().contains(grandchild));
}

#[test]
fn with_children_spawns_children() {
    let mut app = App::new();
    app.add_plugins(TransformPlugin);
    app.add_systems(Startup, |mut commands: Commands| {
        commands
            .spawn((Transform::from_xyz(0.0, 2.0, 0.0), Player))
            .with_children(|parent| {
                parent.spawn(Transform::from_xyz(1.0, 0.0, 0.0));
                parent.spawn(Transform::from_xyz(-1.0, 0.0, 0.0));
            });
    });
    app.update();
    let world = app.world();
    let parent = world
        .query_filtered::<Entity, With<Player>>()
        .single(world)
        .unwrap();
    let children = world.get::<Children>(parent).unwrap().to_vec();
    assert_eq!(children.len(), 2);
    let y = world
        .get::<GlobalTransform>(children[0])
        .unwrap()
        .translation()
        .y;
    assert_eq!(y, 2.0);
}

#[test]
fn required_components_come_with_their_owner() {
    let mut world = World::new();
    let e = world.spawn(Transform::IDENTITY).id();
    assert!(
        world.has::<GlobalTransform>(e)
            && world.has::<Visibility>(e)
            && world.has::<InheritedVisibility>(e)
    );
    let camera = world.spawn(Camera3d::default()).id();
    assert!(
        world.has::<Camera>(camera)
            && world.has::<Projection>(camera)
            && world.has::<Transform>(camera)
    );
    // Given in the same bundle, a required component keeps its value.
    let hidden = world.spawn((Transform::IDENTITY, Visibility::Hidden)).id();
    assert_eq!(
        *world.get::<Visibility>(hidden).unwrap(),
        Visibility::Hidden
    );
}

#[test]
fn hidden_parents_hide_their_children() {
    let mut app = App::new();
    app.add_plugins(TransformPlugin);
    let world = app.world_mut();
    let parent = world.spawn((Transform::IDENTITY, Visibility::Hidden)).id();
    let child = world.spawn((Transform::IDENTITY, ChildOf(parent))).id();
    let shown = world
        .spawn((Transform::IDENTITY, Visibility::Visible, ChildOf(parent)))
        .id();
    app.update();
    let world = app.world();
    assert!(!world.get::<InheritedVisibility>(child).unwrap().get());
    assert!(world.get::<InheritedVisibility>(shown).unwrap().get());
}

#[test]
fn locals_resources_and_run_conditions() {
    #[derive(Default)]
    struct Paused;
    crate::resource!(Paused);
    let mut app = App::new();
    app.init_resource::<Log>();
    app.add_systems(
        Update,
        (
            (|mut n: Local<u32>, mut log: ResMut<Log>| {
                *n += 1;
                log.0.push(format!("tick {}", *n));
            })
            .run_if(not(resource_exists::<Paused>)),
            (|mut log: ResMut<Log>| log.0.push("once".into())).run_if(run_once),
        ),
    );
    app.update();
    app.update();
    app.insert_resource(Paused);
    app.update();
    assert_eq!(log(&app), ["tick 1", "once", "tick 2"]);
}

#[test]
fn a_missing_resource_panics_with_the_system_s_name() {
    struct Missing;
    crate::resource!(Missing);
    fn needs_it(_: Res<Missing>) {}
    let mut app = App::new();
    app.add_systems(Update, needs_it);
    let message = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| app.update()))
        .err()
        .and_then(|e| e.downcast::<String>().ok())
        .unwrap();
    assert!(
        message.contains("needs_it") && message.contains("Missing"),
        "{message}"
    );
}

#[test]
fn single_skips_the_system_unless_one_entity_matches() {
    let mut app = App::new();
    app.init_resource::<Log>();
    app.add_systems(
        Update,
        |player: Single<&Position, With<Player>>, mut log: ResMut<Log>| {
            log.0.push(player.0.to_string());
        },
    );
    app.update();
    let first = app.world_mut().spawn((Position(1.0), Player)).id();
    app.update();
    app.world_mut().spawn((Position(2.0), Player));
    app.update();
    app.world_mut().despawn(first);
    app.update();
    assert_eq!(log(&app), ["1", "2"]);
}

#[test]
fn two_queries_on_the_same_component_work_when_kept_apart() {
    let mut app = App::new();
    app.add_systems(
        Update,
        |players: Query<&Position, With<Player>>,
         mut enemies: Query<&mut Position, Without<Player>>| {
            let target = players.single().map(|p| p.0).unwrap_or(0.0);
            for mut enemy in &mut enemies {
                enemy.0 += (target - enemy.0) / 2.0;
            }
        },
    );
    app.world_mut().spawn((Position(10.0), Player));
    let enemy = app.world_mut().spawn((Position(0.0), Enemy)).id();
    app.update();
    assert_eq!(app.world().get::<Position>(enemy).unwrap().0, 5.0);
}

#[derive(Debug, Clone, PartialEq)]
struct Hit(u32);
crate::message!(Hit);

#[test]
fn messages_reach_each_reader_once() {
    let mut app = App::new();
    app.add_message::<Hit>().init_resource::<Log>();
    app.add_systems(
        Update,
        (
            |mut writer: MessageWriter<Hit>, mut n: Local<u32>| {
                *n += 1;
                if *n <= 2 {
                    writer.write(Hit(*n));
                }
            },
            |mut reader: MessageReader<Hit>, mut log: ResMut<Log>| {
                for hit in reader.read() {
                    log.0.push(format!("a{}", hit.0));
                }
            },
        ),
    );
    // A reader added later still sees the messages of the last two frames.
    app.add_systems(
        PostUpdate,
        |mut reader: MessageReader<Hit>, mut log: ResMut<Log>| {
            for hit in reader.read() {
                log.0.push(format!("b{}", hit.0));
            }
        },
    );
    for _ in 0..4 {
        app.update();
    }
    assert_eq!(log(&app), ["a1", "b1", "a2", "b2"]);
    // Old messages go after two frames.
    assert!(app.world().resource::<Messages<Hit>>().is_empty());
}

struct Explosion {
    at: f32,
}
crate::event!(Explosion);

#[test]
fn observers_run_when_their_event_is_triggered() {
    let mut app = App::new();
    app.init_resource::<Log>();
    app.add_observer(
        |explosion: On<Explosion>, mut commands: Commands, mut log: ResMut<Log>| {
            log.0.push(format!("boom at {}", explosion.at));
            commands.spawn(Position(explosion.at));
        },
    );
    app.add_systems(Update, |mut commands: Commands, mut done: Local<bool>| {
        if !std::mem::replace(&mut *done, true) {
            commands.trigger(Explosion { at: 3.0 });
        }
    });
    app.update();
    app.update();
    assert_eq!(log(&app), ["boom at 3"]);
    let world = app.world();
    assert_eq!(world.query::<&Position>().single(world).unwrap().0, 3.0);
    // Directly on the world too.
    app.world_mut().trigger(Explosion { at: 4.0 });
    assert_eq!(log(&app).len(), 2);
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
enum Screen {
    #[default]
    Title,
    Game,
}
crate::states!(Screen);

#[test]
fn states_run_their_enter_and_exit_schedules() {
    let mut app = App::new();
    app.init_resource::<Log>().init_state::<Screen>();
    for screen in [Screen::Title, Screen::Game] {
        app.add_systems(OnEnter(screen), move |mut log: ResMut<Log>| {
            log.0.push(format!("enter {screen:?}"))
        });
        app.add_systems(OnExit(screen), move |mut log: ResMut<Log>| {
            log.0.push(format!("exit {screen:?}"))
        });
    }
    app.add_systems(
        Update,
        (|mut next: ResMut<NextState<Screen>>| next.set(Screen::Game))
            .run_if(in_state(Screen::Title)),
    );
    app.add_systems(
        Update,
        (|mut log: ResMut<Log>| log.0.push("playing".into())).run_if(in_state(Screen::Game)),
    );
    app.update();
    app.update();
    app.update();
    assert_eq!(
        log(&app),
        [
            "enter Title",
            "exit Title",
            "enter Game",
            "playing",
            "playing"
        ]
    );
}

#[test]
fn time_moves_and_fixed_update_runs_at_its_rate() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        100,
    )));
    let steps = Rc::new(Cell::new(0));
    let counted = steps.clone();
    app.add_systems(FixedUpdate, move |time: Res<Time>| {
        assert_eq!(time.delta(), Duration::from_micros(15_625));
        counted.set(counted.get() + 1);
    });
    app.add_systems(Update, |time: Res<Time>| {
        assert_eq!(time.delta(), Duration::from_millis(100))
    });
    for _ in 0..10 {
        app.update();
    }
    // One second at 64 steps a second.
    assert_eq!(steps.get(), 64);
    assert!((app.world().resource::<Time>().elapsed_secs() - 1.0).abs() < 1e-6);
}

#[test]
fn timers_finish_and_repeat() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins).init_resource::<Log>();
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        400,
    )));
    app.add_systems(
        Update,
        (|mut log: ResMut<Log>| log.0.push("second".into()))
            .run_if(on_timer(Duration::from_secs(1))),
    );
    for _ in 0..5 {
        app.update();
    }
    // 0.4, 0.8, 1.2 (once), 1.6, 2.0 (twice).
    assert_eq!(log(&app), ["second", "second"]);
    let mut once = Timer::from_seconds(1.0, TimerMode::Once);
    once.tick(Duration::from_secs(2));
    assert!(once.just_finished() && once.is_finished());
    once.tick(Duration::from_secs(1));
    assert!(!once.just_finished() && once.is_finished());
}

#[test]
fn input_comes_from_the_page_before_update() {
    let mut app = App::new();
    app.add_plugins(InputPlugin).init_resource::<Log>();
    app.add_systems(
        Update,
        (|mut log: ResMut<Log>| log.0.push("jump".into()))
            .run_if(input_just_pressed(KeyCode::Space)),
    );
    app.add_systems(
        Update,
        |keys: Res<ButtonInput<KeyCode>>, mut log: ResMut<Log>| {
            if keys.pressed(KeyCode::KeyW) {
                log.0.push("forward".into());
            }
        },
    );
    let press = |app: &mut App, key, down| {
        app.world_mut()
            .resource_mut::<super::input::PendingInput>()
            .keys
            .push((key, down))
    };
    press(&mut app, KeyCode::Space, true);
    press(&mut app, KeyCode::KeyW, true);
    app.update();
    app.update();
    press(&mut app, KeyCode::KeyW, false);
    app.update();
    assert_eq!(log(&app), ["jump", "forward", "forward"]);
    assert_eq!(KeyCode::from_code("ArrowLeft"), Some(KeyCode::ArrowLeft));
}

#[test]
fn closures_exclusive_systems_and_plugins() {
    struct Hello;
    impl Plugin for Hello {
        fn build(&self, app: &mut App) {
            app.init_resource::<Log>()
                .add_systems(Startup, |world: &mut World| {
                    world.resource_mut::<Log>().0.push("hello".into());
                    world.spawn(Position(0.0));
                });
        }
    }
    let mut app = App::new();
    app.add_plugins((Hello, |app: &mut App| {
        app.add_systems(Update, |q: Query<&Position>, mut log: ResMut<Log>| {
            log.0.push(q.count().to_string())
        });
    }));
    assert!(app.is_plugin_added::<Hello>());
    app.update();
    assert_eq!(log(&app), ["hello", "1"]);
}

#[test]
fn an_app_runs_until_a_system_asks_it_to_exit() {
    let mut app = App::new();
    app.add_systems(
        Update,
        |mut n: Local<u32>, mut exit: MessageWriter<AppExit>| {
            *n += 1;
            if *n == 3 {
                exit.write(AppExit::from_code(2));
            }
        },
    );
    assert_eq!(app.run(), AppExit::from_code(2));
}

#[test]
fn the_frame_has_the_meshes_lights_and_camera_in_physical_units() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    app.add_systems(
        Startup,
        |mut commands: Commands,
         mut meshes: ResMut<Assets<Mesh>>,
         mut materials: ResMut<Assets<StandardMaterial>>| {
            commands.spawn((
                Mesh3d(meshes.add(Circle::new(4.0))),
                MeshMaterial3d(materials.add(Color::WHITE)),
                Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
            ));
            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgba(1.0, 0.0, 0.0, 0.5),
                    ..default()
                })),
                Visibility::Hidden,
            ));
            commands.spawn((
                PointLight {
                    intensity: 4.0 * std::f32::consts::PI,
                    ..default()
                },
                Transform::from_xyz(4.0, 8.0, 4.0),
            ));
            commands.spawn((
                DirectionalLight::default(),
                Transform::default().looking_to(Vec3::NEG_Y, Vec3::Z),
            ));
            commands.spawn((
                Camera3d::default(),
                Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
            ));
        },
    );
    app.update();
    let render = RenderFrame::extract(app.world(), Vec2::new(800.0, 600.0)).unwrap();
    let frame = render.frame();
    assert_eq!(frame.meshes.len(), 1, "the hidden cube is not drawn");
    // 4π lumens is one candela.
    assert!((frame.lights.point[0].color.r - 1.0).abs() < 1e-5);
    assert_eq!(frame.lights.point[0].position, Vec3::new(4.0, 8.0, 4.0));
    // The sun shines down: the direction towards it is up.
    assert!((frame.lights.directional[0].0 - Vec3::Y).length() < 1e-5);
    assert!((frame.exposure - 1.0 / (2f32.powf(9.7) * 1.2)).abs() < 1e-9);
    assert_eq!(frame.tone_mapping, crate::three::ToneMapping::AcesFilmic);
    assert!(frame.background.is_some());
}

#[test]
fn the_pointer_casts_a_ray_from_the_camera() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    let camera = app
        .world_mut()
        .spawn((
            Camera3d::default(),
            Transform::from_xyz(0.0, 10.0, 0.0).looking_at(Vec3::ZERO, Vec3::Z),
        ))
        .id();
    app.update();
    let world = app.world();
    let (camera, at) = (
        world.get::<Camera>(camera).unwrap(),
        world.get::<GlobalTransform>(camera).unwrap(),
    );
    // The middle of the window (1280 × 720 by default) looks straight down.
    let ray = camera
        .viewport_to_world(&at, Vec2::new(640.0, 360.0))
        .unwrap();
    let distance = ray
        .intersect_plane(Vec3::ZERO, InfinitePlane3d::new(Vec3::Y))
        .unwrap();
    assert!(
        ray.get_point(distance).length() < 1e-3,
        "{:?}",
        ray.get_point(distance)
    );
    let back = camera.world_to_viewport(&at, Vec3::ZERO).unwrap();
    assert!((back - Vec2::new(640.0, 360.0)).length() < 1e-2, "{back:?}");
}

#[test]
fn colors_convert_like_bevy() {
    let c = Color::srgb_u8(255, 128, 0);
    let linear = c.to_linear();
    assert!((linear.green - 0.2158605).abs() < 1e-4, "{linear:?}");
    let back = Color::from(linear);
    assert!((back.green - c.green).abs() < 1e-5);
    let red = Color::hsl(0.0, 1.0, 0.5);
    assert_eq!((red.red, red.green, red.blue), (1.0, 0.0, 0.0));
    assert_eq!(palettes::css::TOMATO, Color::srgb_u8(255, 99, 71));
}
