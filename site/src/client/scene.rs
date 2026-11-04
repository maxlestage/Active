//! The 3D demo of the home page, written like a Bevy game with `active::ecs`: shapes turn
//! on a disc under two lights; clicking the disc drops a new one, which falls, bounces,
//! rolls and piles up with the others (the physics of `DefaultPlugins`: every mesh has a
//! mass); the turning shapes push them aside; dragging turns the camera.

use std::f32::consts::{PI, TAU};

use active::ecs::prelude::*;
use active::ecs::{Running, palettes::css};

/// The shapes of the ring.
struct Spinning {
    speed: f32,
    phase: f32,
    base: f32,
}

/// A dropped shape.
struct Dropped;

active::component!(Spinning, Dropped);

/// The meshes and materials to drop, and a generator of numbers.
struct Palette {
    meshes: Vec<(Handle<Mesh>, f32)>,
    materials: Vec<Handle<StandardMaterial>>,
    seed: u64,
}

impl Palette {
    fn next(&mut self) -> u64 {
        // xorshift64*: enough to choose shapes and colors.
        self.seed ^= self.seed >> 12;
        self.seed ^= self.seed << 25;
        self.seed ^= self.seed >> 27;
        self.seed.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn pick<T: Copy>(&mut self, items: &[T]) -> T {
        items[(self.next() % items.len() as u64) as usize]
    }
}

/// The orbit of the camera, and the drag in progress.
#[derive(Default)]
struct Orbit {
    angle: f32,
    dragged: f32,
    still: bool,
}

/// Tells the island how many shapes there are.
struct Counter(active::State<usize>);

active::resource!(Palette, Orbit, Counter);

const RADIUS: f32 = 10.5;
const MAX_DROPPED: usize = 40;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // The floor.
    commands.spawn((
        Mesh3d(meshes.add(Circle::new(5.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb_u8(24, 24, 28),
            perceptual_roughness: 0.9,
            ..default()
        })),
        Transform::from_rotation(Quat::from_rotation_x(-PI / 2.0)),
    ));
    let shapes: Vec<(Handle<Mesh>, f32)> = vec![
        (meshes.add(Cuboid::new(0.9, 0.9, 0.9)), 0.45),
        (meshes.add(Sphere::new(0.55).mesh().uv(32, 18)), 0.55),
        (meshes.add(Torus::new(0.25, 0.6)), 0.6),
        (meshes.add(Cone::new(0.5, 1.0)), 0.5),
        (meshes.add(Cylinder::new(0.4, 0.9)), 0.45),
    ];
    let colors = [
        css::ORANGE_RED,
        Color::srgb_u8(255, 179, 138),
        css::GOLD,
        css::TURQUOISE,
        css::WHITE_SMOKE,
    ];
    let materials: Vec<Handle<StandardMaterial>> = colors
        .iter()
        .enumerate()
        .map(|(i, &base_color)| {
            materials.add(StandardMaterial {
                base_color,
                metallic: if i % 2 == 0 { 0.6 } else { 0.0 },
                perceptual_roughness: 0.35,
                ..default()
            })
        })
        .collect();
    // The ring: eight shapes turning and floating, children of one turning parent.
    commands
        .spawn((
            Transform::default(),
            Spinning {
                speed: 0.15,
                phase: 0.0,
                base: 0.0,
            },
        ))
        .with_children(|ring| {
            for i in 0..8 {
                let a = i as f32 / 8.0 * TAU;
                let (mesh, radius) = shapes[i % shapes.len()];
                ring.spawn((
                    Mesh3d(mesh),
                    MeshMaterial3d(materials[i % materials.len()]),
                    Transform::from_xyz(3.2 * a.cos(), 1.0 + radius, 3.2 * a.sin()),
                    Spinning {
                        speed: 0.8 + i as f32 * 0.1,
                        phase: a,
                        base: 1.0 + radius,
                    },
                ));
            }
        });
    commands.spawn((
        PointLight {
            color: Color::srgb_u8(255, 120, 60),
            intensity: 2_500_000.0,
            range: 30.0,
            ..default()
        },
        Transform::from_xyz(4.0, 6.0, 3.0),
    ));
    commands.spawn((
        PointLight {
            color: Color::srgb_u8(90, 140, 255),
            intensity: 1_200_000.0,
            range: 30.0,
            ..default()
        },
        Transform::from_xyz(-5.0, 3.0, -4.0),
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: 1_500.0,
            ..default()
        },
        Transform::from_xyz(1.0, 4.0, 2.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        Camera3d::default(),
        Camera {
            clear_color: ClearColorConfig::None,
            ..default()
        },
        Transform::from_xyz(0.0, 5.5, RADIUS).looking_at(Vec3::new(0.0, 0.8, 0.0), Vec3::Y),
    ));
    commands.insert_resource(Palette {
        meshes: shapes,
        materials,
        seed: 0x9E37_79B9_7F4A_7C15,
    });
}

fn spin(
    time: Res<Time>,
    orbit: Res<Orbit>,
    mut query: Query<(&mut Transform, &Spinning), Without<Dropped>>,
) {
    if orbit.still {
        return;
    }
    let t = time.elapsed_secs();
    for (mut transform, spinning) in &mut query {
        transform.rotate_y(spinning.speed * time.delta_secs());
        if spinning.base > 0.0 {
            transform.translation.y = spinning.base + 0.25 * (t * 1.6 + spinning.phase).sin();
        }
    }
}

/// Dragging turns the camera; a click without a drag drops a shape where it points.
// A Bevy system asks for what it uses: many parameters are usual.
#[allow(clippy::too_many_arguments)]
fn pointer(
    mut commands: Commands,
    buttons: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    time: Res<Time>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform, &mut Transform)>,
    mut orbit: ResMut<Orbit>,
    mut palette: ResMut<Palette>,
    dropped: Query<Entity, With<Dropped>>,
) {
    let (camera, at, mut transform) = camera.into_inner();
    if buttons.just_pressed(MouseButton::Left) {
        orbit.dragged = 0.0;
    }
    if buttons.pressed(MouseButton::Left) {
        orbit.dragged += motion.delta.length();
        orbit.angle -= motion.delta.x * 0.008;
    } else if !orbit.still {
        orbit.angle += 0.12 * time.delta_secs();
    }
    if buttons.just_released(MouseButton::Left) && orbit.dragged < 4.0 {
        let target = window
            .cursor_position()
            .and_then(|cursor| camera.viewport_to_world(&at, cursor).ok())
            .and_then(|ray| {
                Some(ray.get_point(ray.intersect_plane(Vec3::ZERO, InfinitePlane3d::new(Vec3::Y))?))
            })
            .filter(|p| p.length() < 5.0);
        if let Some(point) = target {
            // Too many: the oldest goes.
            let full = dropped.count() >= MAX_DROPPED;
            if let Some(oldest) = dropped.iter().next().filter(|_| full) {
                commands.entity(oldest).despawn();
            }
            let (meshes, materials) = (palette.meshes.clone(), palette.materials.clone());
            let (mesh, _) = palette.pick(&meshes);
            let material = palette.pick(&materials);
            let turn = (palette.next() % 628) as f32 / 100.0;
            let tilt = (palette.next() % 100) as f32 / 100.0 - 0.5;
            // A body like any mesh: its mass, gravity, collisions. A little spin, and bouncy.
            commands.spawn((
                Mesh3d(mesh),
                MeshMaterial3d(material),
                Transform::from_xyz(point.x, 5.0, point.z)
                    .with_rotation(Quat::from_rotation_y(turn) * Quat::from_rotation_x(tilt)),
                AngularVelocity(Vec3::new(tilt * 4.0, 0.0, -tilt * 3.0)),
                Restitution(0.35),
                Dropped,
            ));
        }
    }
    let eye = Vec3::new(RADIUS * orbit.angle.sin(), 5.5, RADIUS * orbit.angle.cos());
    *transform = Transform::from_translation(eye).looking_at(Vec3::new(0.0, 0.8, 0.0), Vec3::Y);
}

/// Shapes that fell off the disc go.
fn fallen(mut commands: Commands, dropped: Query<(Entity, &Transform), With<Dropped>>) {
    for (entity, transform) in &dropped {
        if transform.translation.y < -10.0 {
            commands.entity(entity).despawn();
        }
    }
}

fn count(counter: Res<Counter>, shapes: Query<Entity, With<Mesh3d>>) {
    let n = shapes.count().saturating_sub(1); // not the floor
    // Outside any effect: reading does not subscribe anything.
    if counter.0.is_alive() && counter.0.get() != n {
        counter.0.set(n);
    }
}

/// Starts the demo in the canvas; it stops when the returned handle is dropped.
pub fn start(canvas: &web_sys::Element, shapes: active::State<usize>) -> Option<Running> {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .insert_resource(Orbit {
            still: super::reduced_motion(),
            ..Orbit::default()
        })
        .insert_resource(Counter(shapes))
        .add_systems(Startup, setup)
        .add_systems(Update, (spin, pointer, fallen, count));
    match app.run_in(canvas) {
        Ok(running) => Some(running),
        Err(error) => {
            web_sys::console::warn_1(&error.into());
            None
        }
    }
}
