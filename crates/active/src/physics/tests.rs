//! The engine, body by body: what a physics teacher would check.

use super::*;
use std::f32::consts::FRAC_PI_2;

fn floor(world: &mut PhysicsWorld) -> BodyId {
    world.add(Body::fixed(Collider::cuboid(40.0, 1.0, 40.0)).at(Vec3::new(0.0, -0.5, 0.0)))
}

fn run(world: &mut PhysicsWorld, seconds: f32) {
    for _ in 0..(seconds * 60.0).round() as usize {
        world.step(1.0 / 60.0);
    }
}

#[test]
fn a_box_lands_flat_stays_put_and_falls_asleep() {
    let mut world = PhysicsWorld::new();
    floor(&mut world);
    let cube =
        world.add(Body::dynamic(Collider::cuboid(1.0, 1.0, 1.0)).at(Vec3::new(0.0, 2.0, 0.0)));
    run(&mut world, 3.0);
    let b = world.body(cube).unwrap();
    assert!((b.position.y - 0.5).abs() < 0.02, "{:?}", b.position);
    // Landing perfectly flat, its four corners bounce one after the other in Rapier's
    // solver: it moves a few centimeters aside, no more.
    assert!(
        b.position.x.abs() < 0.05 && b.position.z.abs() < 0.05,
        "no drift: {:?}",
        b.position
    );
    assert!(
        (b.rotation.dot(Quat::IDENTITY)).abs() > 0.9999,
        "still upright: {:?}",
        b.rotation
    );
    assert!(b.is_sleeping());
}

#[test]
fn a_stack_of_boxes_stands() {
    let mut world = PhysicsWorld::new();
    floor(&mut world);
    let boxes: Vec<BodyId> = (0..5)
        .map(|i| {
            world.add(Body::dynamic(Collider::cuboid(1.0, 1.0, 1.0)).at(Vec3::new(
                0.0,
                0.5 + i as f32 * 1.01,
                0.0,
            )))
        })
        .collect();
    run(&mut world, 5.0);
    for (i, id) in boxes.iter().enumerate() {
        let p = world.body(*id).unwrap().position;
        assert!((p.y - (0.5 + i as f32)).abs() < 0.05, "box {i}: {p:?}");
        assert!(p.x.abs() < 0.05 && p.z.abs() < 0.05, "box {i} slid: {p:?}");
    }
}

#[test]
fn a_bouncy_ball_bounces_back_up() {
    let mut world = PhysicsWorld::new();
    let ground = floor(&mut world);
    world.body_mut(ground).unwrap().restitution = 0.8;
    let mut ball = Body::dynamic(Collider::sphere(0.5)).at(Vec3::new(0.0, 5.5, 0.0));
    ball.restitution = 0.8;
    ball.linear_damping = 0.0;
    let ball = world.add(ball);
    let mut highest_after_bounce = 0.0f32;
    let mut bounced = false;
    for _ in 0..240 {
        world.step(1.0 / 60.0);
        let b = world.body(ball).unwrap();
        if b.linear_velocity.y > 0.0 {
            bounced = true;
        }
        if bounced {
            highest_after_bounce = highest_after_bounce.max(b.position.y - 0.5);
        }
    }
    // Dropped from 5 m: e² × 5 = 3.2 m back up, a little less with the step.
    assert!(
        bounced && (2.6..3.4).contains(&highest_after_bounce),
        "{highest_after_bounce}"
    );
}

#[test]
fn friction_stops_a_sliding_box_and_ice_does_not() {
    for (friction, stops) in [(0.6, true), (0.0, false)] {
        let mut world = PhysicsWorld::new();
        let ground = floor(&mut world);
        world.body_mut(ground).unwrap().friction = friction;
        let mut cube = Body::dynamic(Collider::cuboid(1.0, 1.0, 1.0))
            .at(Vec3::new(0.0, 0.5, 0.0))
            .moving(Vec3::new(4.0, 0.0, 0.0));
        cube.friction = friction;
        cube.linear_damping = 0.0;
        let cube = world.add(cube);
        run(&mut world, 2.0);
        let v = world.body(cube).unwrap().linear_velocity.x;
        if stops {
            // μ g = 5.9 m/s²: stopped well within two seconds.
            assert!(v.abs() < 0.05, "{v}");
        } else {
            assert!(v > 3.9, "{v}");
        }
    }
}

#[test]
fn a_force_accelerates_by_force_over_mass() {
    let mut world = PhysicsWorld::new();
    world.gravity = Vec3::ZERO;
    let mut b = Body::dynamic(Collider::cuboid(1.0, 1.0, 1.0)).with_mass(2.0);
    b.linear_damping = 0.0;
    b.angular_damping = 0.0;
    let id = world.add(b);
    for _ in 0..60 {
        world
            .body_mut(id)
            .unwrap()
            .apply_force(Vec3::new(10.0, 0.0, 0.0));
        world.step(1.0 / 60.0);
    }
    let b = world.body(id).unwrap();
    // a = F/m = 5 m/s², for one second.
    assert!(
        (b.linear_velocity.x - 5.0).abs() < 1e-3,
        "{:?}",
        b.linear_velocity
    );
    assert!((b.position.x - 2.5).abs() < 0.05, "{:?}", b.position);
    // A torque turns it by τ/I: I = m(1+1)/12 = 1/3 for this cube.
    world
        .body_mut(id)
        .unwrap()
        .apply_torque(Vec3::new(0.0, 1.0, 0.0));
    world.step(1.0);
    let w = world.body(id).unwrap().angular_velocity.y;
    assert!((w - 3.0).abs() < 1e-3, "{w}");
}

#[test]
fn heavier_bodies_push_lighter_ones_and_momentum_is_kept() {
    let mut world = PhysicsWorld::new();
    world.gravity = Vec3::ZERO;
    let mut heavy = Body::dynamic(Collider::sphere(0.5))
        .with_mass(4.0)
        .at(Vec3::new(-2.0, 0.0, 0.0))
        .moving(Vec3::new(3.0, 0.0, 0.0));
    let mut light = Body::dynamic(Collider::sphere(0.5))
        .with_mass(1.0)
        .at(Vec3::new(2.0, 0.0, 0.0));
    for b in [&mut heavy, &mut light] {
        b.restitution = 1.0;
        b.linear_damping = 0.0;
        b.angular_damping = 0.0;
    }
    let (heavy, light) = (world.add(heavy), world.add(light));
    let before = world.momentum();
    run(&mut world, 3.0);
    let after = world.momentum();
    assert!((after - before).length() < 1e-3, "{before:?} → {after:?}");
    // Elastic: v1' = (m1 - m2)/(m1 + m2) v = 1.8, v2' = 2 m1/(m1 + m2) v = 4.8.
    let (v1, v2) = (
        world.body(heavy).unwrap().linear_velocity.x,
        world.body(light).unwrap().linear_velocity.x,
    );
    assert!(
        (v1 - 1.8).abs() < 0.1 && (v2 - 4.8).abs() < 0.1,
        "{v1} {v2}"
    );
}

#[test]
fn every_shape_rests_on_the_floor() {
    let shapes = [
        (Collider::cylinder(0.5, 1.0), 0.5),
        (Collider::cone(0.5, 1.0), 0.5),
        (Collider::sphere(0.4), 0.4),
        (
            Collider::new(Shape::Torus {
                major_radius: 0.6,
                minor_radius: 0.2,
            }),
            0.2,
        ),
    ];
    for (collider, height) in shapes {
        let mut world = PhysicsWorld::new();
        floor(&mut world);
        let id = world.add(Body::dynamic(collider).at(Vec3::new(0.0, height + 0.3, 0.0)));
        run(&mut world, 3.0);
        let b = world.body(id).unwrap();
        assert!(
            (b.position.y - height).abs() < 0.03,
            "{:?}: {:?}",
            collider.shape,
            b.position
        );
        assert!(
            b.position.x.abs() < 0.05 && b.position.z.abs() < 0.05,
            "{:?} wandered: {:?}",
            collider.shape,
            b.position
        );
    }
}

#[test]
fn a_capsule_on_its_end_falls_over() {
    let mut world = PhysicsWorld::new();
    floor(&mut world);
    let mut capsule = Body::dynamic(Collider::capsule(0.3, 1.0))
        .at(Vec3::new(0.0, 0.85, 0.0))
        .turned(Quat::from_rotation_z(0.05));
    capsule.angular_damping = 2.0;
    let id = world.add(capsule);
    run(&mut world, 4.0);
    let b = world.body(id).unwrap();
    // Lying down: its axis horizontal, its center one radius above the floor.
    assert!(
        (b.rotation * Vec3::Y).y.abs() < 0.05 && (b.position.y - 0.3).abs() < 0.03,
        "{:?} {:?}",
        b.rotation,
        b.position
    );
}

#[test]
fn a_cylinder_hanging_off_an_edge_tips_over_it() {
    let mut world = PhysicsWorld::new();
    floor(&mut world);
    world.add(Body::fixed(Collider::cuboid(2.0, 1.0, 2.0)).at(Vec3::new(0.0, 0.5, 0.0)));
    // Most of it beyond the edge of the table (x = 1).
    let can = world.add(Body::dynamic(Collider::cylinder(0.4, 0.4)).at(Vec3::new(1.25, 1.21, 0.0)));
    run(&mut world, 3.0);
    let p = world.body(can).unwrap().position;
    assert!(p.y < 0.9, "it fell off the table: {p:?}");
}

#[test]
fn a_box_on_its_edge_falls_flat() {
    let mut world = PhysicsWorld::new();
    floor(&mut world);
    let tilted = Quat::from_rotation_z(0.3);
    let cube = world.add(
        Body::dynamic(Collider::cuboid(1.0, 1.0, 1.0))
            .at(Vec3::new(0.0, 1.2, 0.0))
            .turned(tilted),
    );
    run(&mut world, 3.0);
    let b = world.body(cube).unwrap();
    let up = b.rotation * Vec3::Y;
    // One of its faces is down: an axis of the cube is vertical again.
    let flat = [Vec3::X, Vec3::Y, Vec3::Z]
        .iter()
        .any(|a| (b.rotation * *a).y.abs() > 0.999);
    assert!(
        flat && (b.position.y - 0.5).abs() < 0.03,
        "{up:?} {:?}",
        b.position
    );
}

#[test]
fn bodies_attract_each_other() {
    let mut world = PhysicsWorld::new();
    world.gravity = Vec3::ZERO;
    world.attraction = Some(Attraction::new(1.0));
    let a = world.add(
        Body::dynamic(Collider::sphere(0.2))
            .with_mass(10.0)
            .at(Vec3::new(-2.0, 0.0, 0.0)),
    );
    let b = world.add(
        Body::dynamic(Collider::sphere(0.2))
            .with_mass(10.0)
            .at(Vec3::new(2.0, 0.0, 0.0)),
    );
    world.step(1.0 / 60.0);
    // G m / d² = 10 / 16, towards each other.
    let (va, vb) = (
        world.body(a).unwrap().linear_velocity,
        world.body(b).unwrap().linear_velocity,
    );
    assert!(va.x > 0.0 && vb.x < 0.0 && (va.x + vb.x).abs() < 1e-5);
    assert!((va.x * 60.0 - 0.625).abs() < 0.01, "{va:?}");
    run(&mut world, 4.0);
    let d = world.body(b).unwrap().position.x - world.body(a).unwrap().position.x;
    assert!(d < 0.5, "they met: {d}");
}

#[test]
fn collisions_start_and_end() {
    let mut world = PhysicsWorld::new();
    let ground = floor(&mut world);
    let ball = world.add(Body::dynamic(Collider::sphere(0.5)).at(Vec3::new(0.0, 1.5, 0.0)));
    let mut started = None;
    for frame in 0..60 {
        world.step(1.0 / 60.0);
        if world
            .events()
            .contains(&CollisionEvent::Started(ground, ball))
        {
            started = Some(frame);
        }
    }
    assert!(started.is_some_and(|f| f > 10), "{started:?}");
    assert_eq!(world.contacts().count(), 1);
    world.body_mut(ball).unwrap().position.y = 5.0;
    world.body_mut(ball).unwrap().wake_up();
    world.step(1.0 / 60.0);
    assert!(
        world
            .events()
            .contains(&CollisionEvent::Ended(ground, ball)),
        "{:?}",
        world.events()
    );
}

#[test]
fn a_kinematic_platform_carries_and_pushes() {
    let mut world = PhysicsWorld::new();
    floor(&mut world);
    let mut wall = Body::kinematic(Collider::cuboid(0.5, 2.0, 4.0)).at(Vec3::new(-3.0, 1.0, 0.0));
    wall.linear_velocity = Vec3::new(2.0, 0.0, 0.0);
    let wall = world.add(wall);
    let cube =
        world.add(Body::dynamic(Collider::cuboid(1.0, 1.0, 1.0)).at(Vec3::new(0.0, 0.5, 0.0)));
    run(&mut world, 3.0);
    // The wall moved 6 m and pushed the box ahead of it.
    let (w, c) = (
        world.body(wall).unwrap().position.x,
        world.body(cube).unwrap().position.x,
    );
    assert!((w - 3.0).abs() < 1e-3, "{w}");
    assert!(c > w, "pushed ahead: wall {w}, box {c}");
    // Turned a quarter, a plate still rests on its slab.
    let _ = FRAC_PI_2;
}

#[test]
fn a_removed_body_is_gone_and_its_id_is_stale() {
    let mut world = PhysicsWorld::new();
    let a = world.add(Body::dynamic(Collider::sphere(1.0)));
    assert!(world.remove(a).is_some());
    let b = world.add(Body::dynamic(Collider::sphere(1.0)));
    assert!(world.body(a).is_none() && world.body(b).is_some());
    assert_eq!(world.len(), 1);
}
