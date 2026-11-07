//! How long the physics of the home page's demo takes per frame (20 shapes on a disc).
use active::physics::*;
use active::scene::math::{Quat, Vec3};

fn main() {
    let mut world = PhysicsWorld::new();
    world.add(Body::fixed(Collider::cylinder(5.0, 0.5)).at(Vec3::new(0.0, -0.25, 0.0)));
    let shapes = [
        Collider::cuboid(0.9, 0.9, 0.9),
        Collider::sphere(0.55),
        Collider::new(Shape::Torus {
            major_radius: 0.42,
            minor_radius: 0.18,
        }),
        Collider::cone(0.5, 1.0),
        Collider::cylinder(0.4, 0.9),
    ];
    for i in 0..8 {
        let a = i as f32 / 8.0 * std::f32::consts::TAU;
        let mut b = Body::kinematic(shapes[i % 5]).at(Vec3::new(3.2 * a.cos(), 1.5, 3.2 * a.sin()));
        b.angular_velocity = Vec3::new(0.0, 1.0, 0.0);
        world.add(b);
    }
    for i in 0..40 {
        let p = Vec3::new(
            (i % 5) as f32 * 0.4 - 0.8,
            2.0 + (i / 5) as f32 * 1.1,
            ((i * 7) % 5) as f32 * 0.3 - 0.6,
        );
        world.add(
            Body::dynamic(shapes[i % 5])
                .at(p)
                .turned(Quat::from_rotation_x(i as f32 * 0.3)),
        );
    }
    let start = std::time::Instant::now();
    let frames = 600;
    for _ in 0..frames {
        world.step(1.0 / 60.0);
    }
    let per = start.elapsed().as_secs_f64() * 1000.0 / frames as f64;
    let asleep = world.bodies().filter(|(_, b)| b.is_sleeping()).count();
    println!(
        "{} bodies: {per:.3} ms per frame, {asleep} asleep at the end",
        world.len()
    );
}
