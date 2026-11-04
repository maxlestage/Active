//! The physics example of the documentation (site/docs/*/3d.md), run without a browser.

#![allow(elided_lifetimes_in_paths)]

use active::ecs::prelude::*;

struct Crate;
active::component!(Crate);

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // The ground: flat, so it stays where it is.
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(20.0, 20.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.5, 0.3))),
    ));
    // A crate of 20 kg, thrown sideways, and a bouncy ball.
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
        Transform::from_xyz(0.0, 4.0, 0.0),
        Mass(20.0),
        LinearVelocity(Vec3::new(2.0, 0.0, 0.0)),
        Crate,
    ));
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.5))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_xyz(3.0, 6.0, 0.0),
        Restitution(0.9),
    ));
    commands.spawn((PointLight::default(), Transform::from_xyz(4.0, 8.0, 4.0)));
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-4.0, 5.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// Space makes the crates jump.
fn jump(keys: Res<ButtonInput<KeyCode>>, mut crates: Query<&mut LinearVelocity, With<Crate>>) {
    if keys.just_pressed(KeyCode::Space) {
        for mut velocity in &mut crates {
            velocity.y = 6.0;
        }
    }
}

#[derive(Default)]
struct Landings(u32);
active::resource!(Landings);

/// Each landing is a message: a sound, a score, a spark…
fn landings(mut collisions: MessageReader<CollisionStarted>, mut count: ResMut<Landings>) {
    count.0 += collisions.read().count() as u32;
}

fn game() -> App {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .init_resource::<Landings>()
        .add_systems(Startup, setup)
        .add_systems(Update, (jump, landings));
    app
}

#[test]
fn the_documented_physics_falls_lands_and_jumps() {
    let mut app = game();
    app.insert_resource(TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f64(1.0 / 60.0),
    ));
    for _ in 0..180 {
        app.update();
    }
    let world = app.world();
    let (crate_at, mass) = {
        let q = world.query_filtered::<(&Transform, &Mass), With<Crate>>();
        let (t, m) = q.single(world).unwrap();
        (t.translation, m.0)
    };
    assert_eq!(mass, 20.0);
    assert!(
        (crate_at.y - 0.5).abs() < 0.03 && crate_at.x > 0.5,
        "landed, after sliding: {crate_at:?}"
    );
    assert!(world.resource::<Landings>().0 >= 2, "two landings at least");
    // What `jump` does: the velocity set by the code, used by the next step.
    for mut velocity in app
        .world()
        .query_filtered::<&mut LinearVelocity, With<Crate>>()
        .iter(app.world())
    {
        velocity.y = 6.0;
    }
    app.update();
    app.update();
    let world = app.world();
    let y = world
        .query_filtered::<&Transform, With<Crate>>()
        .single(world)
        .unwrap()
        .translation
        .y;
    assert!(y > 0.55, "it left the ground: {y}");
}
