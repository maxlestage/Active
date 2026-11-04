//! The laws of mechanics, checked against their formulas: what the engine must get right
//! for its motion to look real, measured with tight tolerances.

use super::*;
use std::f32::consts::PI;

fn run(world: &mut PhysicsWorld, seconds: f32) {
    for _ in 0..(seconds * 60.0).round() as usize {
        world.step(1.0 / 60.0);
    }
}

/// A body with no damping: the formulas assume none.
fn ideal(mut body: Body) -> Body {
    body.linear_damping = 0.0;
    body.angular_damping = 0.0;
    body.rolling_friction = 0.0;
    body
}

fn floor(world: &mut PhysicsWorld) -> BodyId {
    world.add(Body::fixed(Collider::cuboid(200.0, 1.0, 200.0)).at(Vec3::new(0.0, -0.5, 0.0)))
}

/// A ramp turned by `angle` around z, its top face through the origin, going down to +x.
fn ramp(world: &mut PhysicsWorld, angle: f32, friction: f32) -> (BodyId, Vec3, Vec3) {
    let turn = Quat::from_rotation_z(-angle);
    let up = turn * Vec3::Y;
    let down = turn * Vec3::X;
    let mut body = Body::fixed(Collider::cuboid(100.0, 1.0, 20.0))
        .at(up * -0.5)
        .turned(turn);
    body.friction = friction;
    body.rolling_friction = 0.0;
    (world.add(body), up, down)
}

#[test]
fn a_thrown_ball_follows_a_parabola() {
    let mut world = PhysicsWorld::new();
    let v0 = Vec3::new(3.0, 8.0, -1.0);
    let ball = world.add(ideal(Body::dynamic(Collider::sphere(0.2))).moving(v0));
    run(&mut world, 1.0);
    let b = world.body(ball).unwrap();
    let expected = v0 + world.gravity * 0.5;
    assert!(
        (b.position - expected).length() < 0.001,
        "{:?} instead of {expected:?}",
        b.position
    );
    assert!((b.linear_velocity - (v0 + world.gravity)).length() < 1e-3);
}

#[test]
fn a_free_body_keeps_its_angular_momentum_and_energy() {
    // A brick spinning around a tilted axis, with no gravity: in space, its angular
    // momentum and its kinetic energy never change, while its rotation axis wobbles.
    let mut world = PhysicsWorld::new();
    world.gravity = Vec3::ZERO;
    let mut brick = ideal(Body::dynamic(Collider::cuboid(0.3, 1.0, 2.0)));
    brick.angular_velocity = Vec3::new(4.0, 1.0, 2.5);
    let id = world.add(brick);
    let start = world.body(id).unwrap();
    let (l0, e0) = (start.angular_momentum(), start.kinetic_energy());
    let mut worst = (0.0f32, 0.0f32);
    for _ in 0..600 {
        world.step(1.0 / 60.0);
        let b = world.body(id).unwrap();
        worst.0 = worst
            .0
            .max((b.angular_momentum() - l0).length() / l0.length());
        worst.1 = worst.1.max((b.kinetic_energy() - e0).abs() / e0);
    }
    assert!(
        worst.0 < 0.0001,
        "angular momentum drifted by {:.3}%",
        worst.0 * 100.0
    );
    assert!(
        worst.1 < 0.0005,
        "energy drifted by {:.3}%",
        worst.1 * 100.0
    );
}

#[test]
fn a_spin_around_the_middle_axis_flips() {
    // The tennis racket theorem (Dzhanibekov's nut): spinning around the axis of middle
    // inertia is unstable, and the body flips over and over.
    let mut world = PhysicsWorld::new();
    world.gravity = Vec3::ZERO;
    let mut brick = ideal(Body::dynamic(Collider::cuboid(0.2, 1.0, 2.0)));
    // x: the smallest inertia is along the longest side (z), the largest along x.
    // The middle one is along y.
    brick.angular_velocity = Vec3::new(0.01, 6.0, 0.01);
    let id = world.add(brick);
    let mut flipped = false;
    for _ in 0..900 {
        world.step(1.0 / 60.0);
        let b = world.body(id).unwrap();
        if (b.rotation * Vec3::Y).y < -0.9 {
            flipped = true;
        }
    }
    assert!(flipped, "it should turn upside down");
    // Around the largest or the smallest axis, it stays steady.
    for axis in [Vec3::X, Vec3::Z] {
        let mut world = PhysicsWorld::new();
        world.gravity = Vec3::ZERO;
        let mut brick = ideal(Body::dynamic(Collider::cuboid(0.2, 1.0, 2.0)));
        brick.angular_velocity = axis * 6.0 + Vec3::new(0.01, 0.01, 0.01);
        let id = world.add(brick);
        run(&mut world, 15.0);
        let b = world.body(id).unwrap();
        let w = b.angular_velocity.normalize();
        assert!(w.dot(axis).abs() > 0.99, "{axis:?}: {w:?}");
    }
}

#[test]
fn a_box_on_a_gentle_slope_stays_and_on_a_steep_one_slides() {
    // Friction 0.5: it holds up to atan(0.5) = 26.6°.
    let mu = 0.5;
    for degrees in [10.0f32, 20.0, 25.0] {
        let mut world = PhysicsWorld::new();
        let (_, up, _) = ramp(&mut world, degrees.to_radians(), mu);
        let mut b = ideal(Body::dynamic(Collider::cuboid(1.0, 1.0, 1.0)));
        b.friction = mu;
        let start = up * 0.5;
        let id = world.add(
            b.at(start)
                .turned(Quat::from_rotation_z(-degrees.to_radians())),
        );
        run(&mut world, 4.0);
        let moved = (world.body(id).unwrap().position - start).length();
        assert!(moved < 0.001, "{degrees}°: it crept {moved} m");
    }
    for degrees in [35.0f32, 45.0] {
        let angle = degrees.to_radians();
        let mut world = PhysicsWorld::new();
        let (_, up, down) = ramp(&mut world, angle, mu);
        let mut b = ideal(Body::dynamic(Collider::cuboid(1.0, 1.0, 1.0)));
        b.friction = mu;
        let start = up * 0.5;
        let id = world.add(b.at(start).turned(Quat::from_rotation_z(-angle)));
        let t = 1.5;
        run(&mut world, t);
        let along = (world.body(id).unwrap().position - start).dot(down);
        let a = 9.81 * (angle.sin() - mu * angle.cos());
        let expected = 0.5 * a * t * t;
        assert!(
            (along - expected).abs() < 0.01 * expected,
            "{degrees}°: {along} m instead of {expected} m"
        );
    }
}

#[test]
fn round_bodies_roll_down_a_slope_as_fast_as_their_inertia_allows() {
    // Rolling without slipping: a = g sin θ / (1 + I / (m r²)).
    let angle = 20f32.to_radians();
    let cases = [
        ("ball", Collider::sphere(0.5), 5.0 / 7.0),
        // A cylinder lying across the slope.
        (
            "cylinder",
            Collider::cylinder(0.5, 1.0).with_rotation(Quat::from_rotation_x(PI / 2.0)),
            2.0 / 3.0,
        ),
    ];
    for (name, collider, factor) in cases {
        let mut world = PhysicsWorld::new();
        let (_, up, down) = ramp(&mut world, angle, 1.0);
        let mut b = ideal(Body::dynamic(collider));
        b.friction = 1.0;
        let start = up * 0.5;
        let id = world.add(b.at(start));
        let t = 1.5;
        run(&mut world, t);
        let along = (world.body(id).unwrap().position - start).dot(down);
        let expected = 0.5 * factor * 9.81 * angle.sin() * t * t;
        assert!(
            (along - expected).abs() < 0.01 * expected,
            "{name}: {along} m instead of {expected} m"
        );
    }
}

#[test]
fn friction_is_the_same_in_every_direction() {
    // Coulomb's law: the friction force is μ N whatever the direction of sliding, so a
    // box stops after v² / (2 μ g) diagonally as well as straight.
    for direction in [
        Vec3::X,
        Vec3::new(1.0, 0.0, 1.0).normalize(),
        Vec3::new(-0.6, 0.0, 0.8),
    ] {
        let mut world = PhysicsWorld::new();
        let ground = floor(&mut world);
        world.body_mut(ground).unwrap().friction = 0.5;
        let mut b = ideal(Body::dynamic(Collider::cuboid(1.0, 1.0, 1.0)));
        b.friction = 0.5;
        let id = world.add(b.at(Vec3::new(0.0, 0.5, 0.0)).moving(direction * 3.0));
        run(&mut world, 2.0);
        let p = world.body(id).unwrap().position;
        let distance = Vec3::new(p.x, 0.0, p.z).length();
        let expected = 9.0 / (2.0 * 0.5 * 9.81);
        assert!(
            (distance - expected).abs() < 0.01 * expected,
            "{direction:?}: {distance} m instead of {expected} m"
        );
    }
}

#[test]
fn an_elastic_ball_bounces_back_to_where_it_started_again_and_again() {
    let mut world = PhysicsWorld::new();
    let ground = floor(&mut world);
    world.body_mut(ground).unwrap().restitution = 1.0;
    let mut ball = ideal(Body::dynamic(Collider::sphere(0.5)));
    ball.restitution = 1.0;
    let id = world.add(ball.at(Vec3::new(0.0, 3.5, 0.0)));
    let mut peaks = Vec::new();
    let mut last = (0.0f32, 3.5f32);
    for _ in 0..(8 * 60) {
        world.step(1.0 / 60.0);
        let b = world.body(id).unwrap();
        if last.0 > 0.0 && b.linear_velocity.y <= 0.0 {
            peaks.push(last.1);
        }
        last = (b.linear_velocity.y, b.position.y);
    }
    assert!(peaks.len() >= 4, "{peaks:?}");
    for peak in &peaks {
        assert!((peak - 3.5).abs() < 0.02, "peaks: {peaks:?}");
    }
}

#[test]
fn a_bounce_happens_at_the_surface_not_before() {
    // A ball fast enough to cross several centimeters per substep must still touch the
    // floor before it bounces.
    let mut world = PhysicsWorld::new();
    let ground = floor(&mut world);
    world.body_mut(ground).unwrap().restitution = 1.0;
    let mut ball = ideal(Body::dynamic(Collider::sphere(0.5)));
    ball.restitution = 1.0;
    let id = world.add(
        ball.at(Vec3::new(0.0, 1.5, 0.0))
            .moving(Vec3::new(0.0, -20.0, 0.0)),
    );
    let mut lowest = f32::MAX;
    for _ in 0..30 {
        world.step(1.0 / 60.0);
        lowest = lowest.min(world.body(id).unwrap().position.y);
    }
    assert!(
        lowest < 0.5 + 0.01,
        "it turned back {} m above the floor",
        lowest - 0.5
    );
    assert!(
        lowest > 0.5 - 0.05,
        "it sank {} m into the floor",
        0.5 - lowest
    );
}

#[test]
fn equal_balls_exchange_their_velocities() {
    // Newton's cradle: an elastic head-on hit between equal masses swaps the velocities.
    let mut world = PhysicsWorld::new();
    world.gravity = Vec3::ZERO;
    let mut a = ideal(Body::dynamic(Collider::sphere(0.5)));
    a.restitution = 1.0;
    let mut b = a.clone();
    let a = world.add(
        a.at(Vec3::new(-2.0, 0.0, 0.0))
            .moving(Vec3::new(3.0, 0.0, 0.0)),
    );
    b.restitution = 1.0;
    let b = world.add(b.at(Vec3::new(1.0, 0.0, 0.0)));
    run(&mut world, 1.5);
    let (va, vb) = (
        world.body(a).unwrap().linear_velocity,
        world.body(b).unwrap().linear_velocity,
    );
    assert!(va.length() < 0.05, "{va:?}");
    assert!((vb - Vec3::new(3.0, 0.0, 0.0)).length() < 0.05, "{vb:?}");
}

#[test]
fn a_slow_push_moves_a_sleeping_body() {
    // A box asleep on the floor is not a wall: a box sliding into it slowly, on ice,
    // passes its momentum on.
    let mut world = PhysicsWorld::new();
    let ground = floor(&mut world);
    world.body_mut(ground).unwrap().friction = 0.0;
    let mut resting = ideal(Body::dynamic(Collider::cuboid(1.0, 1.0, 1.0)));
    resting.friction = 0.0;
    let mut pusher = resting.clone();
    let resting = world.add(resting.at(Vec3::new(0.0, 0.5, 0.0)));
    run(&mut world, 2.0);
    assert!(world.body(resting).unwrap().is_sleeping());
    pusher.restitution = 0.0;
    let pusher = world.add(
        pusher
            .at(Vec3::new(-1.05, 0.5, 0.0))
            .moving(Vec3::new(0.12, 0.0, 0.0)),
    );
    run(&mut world, 6.0);
    let x = world.body(resting).unwrap().position.x;
    assert!(x > 0.2, "the sleeping box did not move: {x}");
    let total = world.body(resting).unwrap().linear_velocity.x
        + world.body(pusher).unwrap().linear_velocity.x;
    assert!((total - 0.12).abs() < 0.01, "momentum: {total}");
}

#[test]
fn changing_a_sleeping_body_wakes_it() {
    let mut world = PhysicsWorld::new();
    let ground = floor(&mut world);
    let id = world.add(Body::dynamic(Collider::cuboid(1.0, 1.0, 1.0)).at(Vec3::new(0.0, 0.5, 0.0)));
    run(&mut world, 2.0);
    assert!(world.body(id).unwrap().is_sleeping());
    // A velocity set directly.
    world.body_mut(id).unwrap().linear_velocity = Vec3::new(0.0, 4.0, 0.0);
    run(&mut world, 0.2);
    assert!(
        world.body(id).unwrap().position.y > 1.0,
        "a velocity set by hand is kept"
    );
    run(&mut world, 3.0);
    assert!(world.body(id).unwrap().is_sleeping());
    // A force set directly.
    world.body_mut(id).unwrap().force = Vec3::new(0.0, 100.0, 0.0);
    run(&mut world, 1.0 / 60.0);
    assert!(
        world.body(id).unwrap().linear_velocity.y > 0.5,
        "a force set by hand pushes"
    );
    run(&mut world, 3.0);
    // Moved by hand into the air: it falls.
    world.body_mut(id).unwrap().position.y = 3.0;
    run(&mut world, 0.3);
    assert!(
        world.body(id).unwrap().position.y < 2.9,
        "a body moved by hand falls"
    );
    run(&mut world, 3.0);
    // Its floor removed: it falls.
    assert!(world.body(id).unwrap().is_sleeping());
    world.remove(ground);
    run(&mut world, 0.5);
    assert!(
        world.body(id).unwrap().position.y < 0.0,
        "nothing holds it any more"
    );
}

#[test]
fn changing_gravity_wakes_everything() {
    let mut world = PhysicsWorld::new();
    floor(&mut world);
    let id = world.add(Body::dynamic(Collider::sphere(0.5)).at(Vec3::new(0.0, 0.5, 0.0)));
    run(&mut world, 3.0);
    assert!(world.body(id).unwrap().is_sleeping());
    world.gravity = Vec3::new(0.0, 9.81, 0.0);
    run(&mut world, 1.0);
    assert!(world.body(id).unwrap().position.y > 2.0, "it falls up");
}

#[test]
fn a_long_step_is_cut_into_short_ones() {
    // A frame of half a second (a tab put back to the front) gives the same motion.
    let mut world = PhysicsWorld::new();
    floor(&mut world);
    let ball = world.add(ideal(Body::dynamic(Collider::sphere(0.25))).at(Vec3::new(0.0, 4.0, 0.0)));
    world.step(0.5);
    let y = world.body(ball).unwrap().position.y;
    let expected = 4.0 - 0.5 * 9.81 * 0.25;
    assert!((y - expected).abs() < 0.002, "{y} instead of {expected}");
    world.step(2.0);
    let y = world.body(ball).unwrap().position.y;
    assert!(
        (y - 0.25).abs() < 0.03,
        "it lands and stays on the floor: {y}"
    );
}

#[test]
fn nothing_goes_through_a_thin_wall() {
    // Fast and small bodies against a 2 cm wall: they must hit it, whatever their shape.
    let shapes = [
        Collider::sphere(0.05),
        Collider::cuboid(0.1, 0.1, 0.1),
        Collider::cylinder(0.05, 0.1),
        Collider::capsule(0.05, 0.1),
        Collider::cone(0.05, 0.1),
    ];
    for (n, collider) in shapes.into_iter().enumerate() {
        for speed in [10.0f32, 40.0, 120.0] {
            let mut world = PhysicsWorld::new();
            world.gravity = Vec3::ZERO;
            world.add(Body::fixed(Collider::cuboid(0.02, 4.0, 4.0)));
            let id = world.add(
                ideal(Body::dynamic(collider))
                    .at(Vec3::new(-1.0, 0.0, 0.0))
                    .moving(Vec3::X * speed),
            );
            run(&mut world, 0.5);
            let x = world.body(id).unwrap().position.x;
            assert!(x < 0.0, "shape {n} at {speed} m/s went through: x = {x}");
        }
    }
}

#[test]
fn a_circular_orbit_stays_circular() {
    // Two bodies attracting each other turn around their common center of mass, at the
    // distance and the speed Kepler gives, keeping their energy.
    let mut world = PhysicsWorld::new();
    world.gravity = Vec3::ZERO;
    let g = 1.0;
    world.attraction = Some(Attraction {
        softening: 0.0,
        ..Attraction::new(g)
    });
    let (m, r) = (100.0f32, 5.0f32);
    // Equal masses: each on a circle of radius r/2, where m v² / (r/2) = G m² / r².
    let speed = (g * m / (2.0 * r)).sqrt();
    let body = |x: f32, v: f32| {
        ideal(Body::dynamic(Collider::sphere(0.1)))
            .with_mass(m)
            .at(Vec3::new(x, 0.0, 0.0))
            .moving(Vec3::new(0.0, 0.0, v))
    };
    let a = world.add(body(-r / 2.0, -speed));
    let b = world.add(body(r / 2.0, speed));
    let period = 2.0 * PI * (r / 2.0) / speed;
    let mut worst = 0.0f32;
    for _ in 0..(period * 60.0).round() as usize {
        world.step(1.0 / 60.0);
        let d = (world.body(a).unwrap().position - world.body(b).unwrap().position).length();
        worst = worst.max((d - r).abs() / r);
    }
    assert!(
        worst < 0.001,
        "the distance changed by {:.2}%",
        worst * 100.0
    );
    let back = world.body(b).unwrap().position;
    assert!(
        (back - Vec3::new(r / 2.0, 0.0, 0.0)).length() < 0.05 * r,
        "{back:?}"
    );
}

#[test]
fn rolling_resistance_stops_a_ball_where_the_formula_says() {
    // A ball rolling at v on a floor with rolling resistance C slows down by
    // C g / (1 + I / (m r²)) = 5/7 C g: it stops after v² / (2 · 5/7 · C g).
    let c = 0.05;
    let mut world = PhysicsWorld::new();
    let ground = floor(&mut world);
    world.body_mut(ground).unwrap().rolling_friction = 0.0;
    let mut ball = ideal(Body::dynamic(Collider::sphere(0.5)));
    ball.rolling_friction = c;
    ball.angular_velocity = Vec3::new(0.0, 0.0, -4.0);
    let id = world.add(
        ball.at(Vec3::new(0.0, 0.5, 0.0))
            .moving(Vec3::new(2.0, 0.0, 0.0)),
    );
    run(&mut world, 8.0);
    let b = world.body(id).unwrap();
    let expected = 4.0 / (2.0 * 5.0 / 7.0 * c * 9.81);
    assert!(
        (b.position.x - expected).abs() < 0.01 * expected,
        "{} m instead of {expected} m",
        b.position.x
    );
    assert!(b.linear_velocity.length() < 1e-3, "{:?}", b.linear_velocity);
}

#[test]
fn a_tall_stack_and_a_pyramid_stand() {
    let mut world = PhysicsWorld::new();
    floor(&mut world);
    let mut ids = Vec::new();
    for i in 0..12 {
        ids.push((
            world.add(Body::dynamic(Collider::cuboid(1.0, 1.0, 1.0)).at(Vec3::new(
                0.0,
                0.5 + i as f32,
                0.0,
            ))),
            Vec3::new(0.0, 0.5 + i as f32, 0.0),
        ));
    }
    for row in 0..6 {
        for k in 0..(6 - row) {
            let at = Vec3::new(
                5.0 + k as f32 * 1.02 + row as f32 * 0.51,
                0.5 + row as f32,
                0.0,
            );
            ids.push((
                world.add(Body::dynamic(Collider::cuboid(1.0, 1.0, 1.0)).at(at)),
                at,
            ));
        }
    }
    run(&mut world, 8.0);
    for (id, at) in ids {
        let b = world.body(id).unwrap();
        assert!(
            (b.position - at).length() < 0.05,
            "{at:?} moved to {:?}",
            b.position
        );
        assert!(b.is_sleeping(), "{at:?} still moves");
    }
}

#[test]
fn a_ring_has_a_hole() {
    // A ball smaller than the hole falls through a ring held in the air; one larger
    // rests in it.
    let ring = Collider::new(Shape::Torus {
        major_radius: 1.0,
        minor_radius: 0.2,
    });
    for (radius, through) in [(0.5f32, true), (1.0, false)] {
        let mut world = PhysicsWorld::new();
        world.add(Body::fixed(ring).at(Vec3::new(0.0, 2.0, 0.0)));
        let ball = world.add(Body::dynamic(Collider::sphere(radius)).at(Vec3::new(0.0, 4.0, 0.0)));
        run(&mut world, 1.5);
        let y = world.body(ball).unwrap().position.y;
        if through {
            assert!(y < 0.0, "the ball of {radius} m stayed at {y}");
        } else {
            // It sits on the tube: its center at sqrt((r + 0.2)² - 1) above the ring.
            let expected = 2.0 + ((radius + 0.2).powi(2) - 1.0).sqrt();
            assert!((y - expected).abs() < 0.02, "{y} instead of {expected}");
        }
    }
    // Lying on the floor, it rests on its tube, and it hangs on a peg.
    let mut world = PhysicsWorld::new();
    floor(&mut world);
    let lying = world.add(Body::dynamic(ring).at(Vec3::new(0.0, 1.0, 0.0)));
    world.add(
        Body::fixed(Collider::cylinder(0.1, 6.0))
            .at(Vec3::new(6.0, 3.0, 0.0))
            .turned(Quat::from_rotation_z(PI / 2.0)),
    );
    // Threaded on the peg (its hole around it), it falls until the peg holds it by
    // the inside of its tube: its center at 3 + 0.1 - (1 - 0.2) = 2.3.
    let hung = world.add(
        Body::dynamic(ring)
            .at(Vec3::new(5.0, 3.5, 0.0))
            .turned(Quat::from_rotation_z(PI / 2.0)),
    );
    run(&mut world, 3.0);
    let b = world.body(lying).unwrap();
    assert!((b.position.y - 0.2).abs() < 0.01, "{:?}", b.position);
    let h = world.body(hung).unwrap().position;
    assert!((h.y - 2.3).abs() < 0.01, "it hangs on the peg: {h:?}");
}
