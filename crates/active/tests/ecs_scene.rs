//! The game example of the documentation (site/docs/*/3d.md), run without a browser.

#![allow(elided_lifetimes_in_paths)]

use active::ecs::prelude::*;

struct Spinning;
active::component!(Spinning);

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Circle::new(4.0))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
        Transform::from_xyz(0.0, 0.5, 0.0),
        Spinning,
    ));
    commands.spawn((PointLight::default(), Transform::from_xyz(4.0, 8.0, 4.0)));
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn spin(time: Res<Time>, mut cubes: Query<&mut Transform, With<Spinning>>) {
    for mut transform in &mut cubes {
        transform.rotate_y(time.delta_secs());
    }
}

fn game() -> App {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, spin);
    app
}

#[test]
fn the_documented_game_turns_its_cube_and_draws_two_meshes() {
    let mut app = game();
    app.insert_resource(TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_millis(500),
    ));
    app.update();
    app.update();
    let world = app.world();
    let turned = world
        .query_filtered::<&Transform, With<Spinning>>()
        .single(world)
        .unwrap();
    assert!(turned.rotation != Quat::IDENTITY);
    let frame = RenderFrame::extract(world, Vec2::new(800.0, 600.0)).unwrap();
    assert_eq!(frame.mesh_count(), 2);
    // The cube turned by one radian in two half-second frames.
    let forward = turned.forward();
    assert!(
        (forward.x + 1f32.sin()).abs() < 1e-4 && (forward.z + 1f32.cos()).abs() < 1e-4,
        "{forward:?}"
    );
}
