//! Physics for a scene: every mesh gets a body with a mass, and the scene follows the
//! bodies.

use std::collections::{HashMap, HashSet};

use super::camera::euler_from_matrix;
use super::graph::{ObjectId, ObjectKind, Scene};
use super::math::{Matrix4, Quaternion, Vector3};
use crate::physics::{Body, BodyId, BodyKind, Collider, PhysicsWorld, Shape};

struct Tracked {
    body: BodyId,
    /// The object's pose in the world as the scene gives it back after the last step:
    /// another one means the code moved it. (Read back, not as written: the scene keeps
    /// Euler angles, and the round trip changes the last digits.)
    pose: (Vector3, Quaternion),
    /// The body's pose written to the object last time.
    written: (Vector3, Quaternion),
    scale: Vector3,
    collider: Collider,
}

/// The physics of a [`Scene`]: every mesh is a body, with the mass of its volume (density 1,
/// change it with [`body_mut`](Self::body_mut)), pulled by gravity and colliding with the
/// others. Meshes with a volume fall; flat ones (`PlaneGeometry`) stay, as ground; children
/// follow their parent and push what they meet.
///
/// ```
/// use active::scene::*;
///
/// let mut scene = Scene::new();
/// let floor = scene.add(Mesh::new(PlaneGeometry::new(20.0, 20.0), MeshStandardMaterial::new()));
/// scene[floor].rotation.x = -std::f32::consts::FRAC_PI_2;
/// let ball = scene.add(Mesh::new(SphereGeometry::new(0.5, 32, 16), MeshStandardMaterial::new()));
/// scene[ball].position.y = 3.0;
///
/// let mut physics = Physics::new();
/// for _ in 0..180 {
///     physics.step(&mut scene, 1.0 / 60.0); // in the animation loop, before rendering
/// }
/// assert!((scene[ball].position.y - 0.5).abs() < 0.02);
/// // Push it: a force, an impulse, a velocity…
/// physics.body_mut(ball).unwrap().apply_impulse(Vector3::new(2.0, 0.0, 0.0));
/// ```
pub struct Physics {
    world: PhysicsWorld,
    tracked: HashMap<ObjectId, Tracked>,
    /// The longest step simulated at once (a slow frame slows the simulation down instead
    /// of letting objects go through each other): 1/30 s.
    pub max_step: f32,
}

impl Default for Physics {
    fn default() -> Physics {
        Physics::new()
    }
}

impl std::fmt::Debug for Physics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Physics")
            .field("bodies", &self.tracked.len())
            .finish()
    }
}

impl Physics {
    /// The physics of a scene, with Earth's gravity.
    pub fn new() -> Physics {
        Physics {
            world: PhysicsWorld::new(),
            tracked: HashMap::new(),
            max_step: 1.0 / 30.0,
        }
    }

    /// The physics world: gravity, attraction, substeps, collision events.
    pub fn world(&self) -> &PhysicsWorld {
        &self.world
    }

    /// The same, to change.
    pub fn world_mut(&mut self) -> &mut PhysicsWorld {
        &mut self.world
    }

    /// The body of an object (after its first step).
    pub fn body(&self, object: ObjectId) -> Option<&Body> {
        self.world.body(self.tracked.get(&object)?.body)
    }

    /// The body of an object, to push it, change its mass or make it static.
    pub fn body_mut(&mut self, object: ObjectId) -> Option<&mut Body> {
        let id = self.tracked.get(&object)?.body;
        self.world.body_mut(id)
    }

    /// The object of a body (for the collision events).
    pub fn object_of(&self, body: BodyId) -> Option<ObjectId> {
        self.tracked
            .iter()
            .find(|(_, t)| t.body == body)
            .map(|(o, _)| *o)
    }

    /// Moves the bodies by `dt` seconds and the meshes with them. Objects moved by your
    /// code are moved in the physics too.
    pub fn step(&mut self, scene: &mut Scene, dt: f32) {
        let dt = dt.min(self.max_step);
        let mut present = HashSet::new();
        let ids: Vec<ObjectId> = scene.object_ids().collect();
        for id in ids {
            let object = &scene[id];
            let ObjectKind::Mesh(mesh) = &object.kind else {
                continue;
            };
            present.insert(id);
            let (collider, flat) = match mesh.geometry.collider() {
                Some(c) => (*c, mesh.geometry.is_flat()),
                None => (bounding_box(&mesh.geometry), false),
            };
            let child = object.parent().is_some();
            let world = scene.world_matrix(id);
            let (position, rotation, scale) = world.decompose();
            match self.tracked.get_mut(&id) {
                None => {
                    let kind = if child {
                        BodyKind::Kinematic
                    } else if flat {
                        BodyKind::Static
                    } else {
                        BodyKind::Dynamic
                    };
                    let body = Body::new(kind, collider.scaled(scale))
                        .at(position)
                        .turned(rotation);
                    let body = self.world.add(body);
                    self.tracked.insert(
                        id,
                        Tracked {
                            body,
                            pose: (position, rotation),
                            written: (position, rotation),
                            scale,
                            collider,
                        },
                    );
                }
                Some(t) => {
                    let Some(body) = self.world.body_mut(t.body) else {
                        continue;
                    };
                    if t.collider != collider || !same_scale(t.scale, scale) {
                        body.set_collider(collider.scaled(scale));
                        t.collider = collider;
                        t.scale = scale;
                    }
                    let moved = (position, rotation) != t.pose;
                    match body.kind {
                        BodyKind::Kinematic if moved => {
                            // Reaches its new place during the step, pushing what is there.
                            let target = position + rotation * body.collider().center_of_mass();
                            body.linear_velocity = (target - body.center_of_mass()) / dt.max(1e-6);
                            body.angular_velocity = turn_rate(body.rotation, rotation, dt);
                        }
                        BodyKind::Kinematic => {
                            body.linear_velocity = Vector3::ZERO;
                            body.angular_velocity = Vector3::ZERO;
                        }
                        _ if moved => {
                            body.position = position;
                            body.rotation = rotation;
                            body.wake_up();
                        }
                        _ => {}
                    }
                }
            }
        }
        let gone: Vec<ObjectId> = self
            .tracked
            .keys()
            .filter(|id| !present.contains(id))
            .copied()
            .collect();
        for id in gone {
            if let Some(t) = self.tracked.remove(&id) {
                self.world.remove(t.body);
            }
        }

        self.world.step(dt);

        for (&id, t) in &mut self.tracked {
            let Some(body) = self.world.body(t.body) else {
                continue;
            };
            let pose = (body.position, body.rotation);
            if body.kind == BodyKind::Kinematic {
                // It went where the scene put it.
                t.pose = (
                    scene.world_matrix(id).decompose().0,
                    scene.world_matrix(id).decompose().1,
                );
                continue;
            }
            if body.kind != BodyKind::Dynamic || pose == t.written {
                continue;
            }
            t.written = pose;
            // Back into the parent's space, if any.
            let world = Matrix4::compose(pose.0, pose.1, t.scale);
            let local = match scene[id].parent() {
                Some(parent) => scene.world_matrix(parent).inverse() * world,
                None => world,
            };
            let (position, rotation, _) = local.decompose();
            let object = &mut scene[id];
            object.position = position;
            object.rotation =
                euler_from_matrix(&Matrix4::compose(Vector3::ZERO, rotation, Vector3::ONE));
            let (position, rotation, _) = scene.world_matrix(id).decompose();
            t.pose = (position, rotation);
        }
    }
}

/// Whether two scales are the same but for rounding: a scale read back from a world matrix
/// changes in its last digits as the object turns.
pub(crate) fn same_scale(a: Vector3, b: Vector3) -> bool {
    (a - b).length() <= 1e-5 * a.length().max(b.length())
}

/// How fast to turn from `from` to `to` in `dt` seconds.
pub(crate) fn turn_rate(from: Quaternion, to: Quaternion, dt: f32) -> Vector3 {
    let mut d = to * from.inverse();
    if d.w < 0.0 {
        d = Quaternion::from_xyzw(-d.x, -d.y, -d.z, -d.w);
    }
    let axis = Vector3::new(d.x, d.y, d.z);
    let s = axis.length();
    if s < 1e-7 {
        return Vector3::ZERO;
    }
    let angle = 2.0 * s.atan2(d.w);
    axis / s * (angle / dt.max(1e-6))
}

/// A geometry without a known shape collides as its bounding box.
pub(crate) fn bounding_box(geometry: &super::BufferGeometry) -> Collider {
    let (min, max) = geometry.bounding_box();
    let half = (max - min) * 0.5;
    Collider::new(Shape::Cuboid {
        half_extents: half.max(Vector3::splat(0.005)),
    })
    .with_offset((min + max) * 0.5)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{
        BoxGeometry, Group, Mesh, MeshStandardMaterial, PlaneGeometry, SphereGeometry,
    };

    #[test]
    fn meshes_fall_land_and_follow_their_parent() {
        let mut scene = Scene::new();
        let floor = scene.add(Mesh::new(
            PlaneGeometry::new(20.0, 20.0),
            MeshStandardMaterial::new(),
        ));
        scene[floor].rotation.x = -std::f32::consts::FRAC_PI_2;
        let cube = scene.add(Mesh::new(
            BoxGeometry::new(1.0, 1.0, 1.0),
            MeshStandardMaterial::new(),
        ));
        scene[cube].position.set(0.0, 2.0, 0.0);
        let arm = scene.add(Group::new());
        // A paddle from 1.75 m to 3.25 m from the arm's axis, its broad face forward.
        let paddle = scene.add_to(
            arm,
            Mesh::new(BoxGeometry::new(1.5, 1.0, 0.2), MeshStandardMaterial::new()),
        );
        scene[paddle].position.set(2.5, 0.5, 0.0);
        // A ball on its way (turning by +y goes towards -z).
        let start = Vector3::new(2.5 * 0.5f32.cos(), 0.3, -2.5 * 0.5f32.sin());
        let ball = scene.add(Mesh::new(
            SphereGeometry::new(0.3, 16, 8),
            MeshStandardMaterial::new(),
        ));
        scene[ball].position = start;
        let mut physics = Physics::new();
        for frame in 0..180 {
            // The arm turns: its paddle sweeps the ball away.
            scene[arm].rotation.y = frame as f32 * 0.02;
            physics.step(&mut scene, 1.0 / 60.0);
        }
        assert!(
            (scene[cube].position.y - 0.5).abs() < 0.02,
            "{:?}",
            scene[cube].position
        );
        assert_eq!(physics.body(floor).unwrap().kind, BodyKind::Static);
        assert_eq!(physics.body(paddle).unwrap().kind, BodyKind::Kinematic);
        let pushed = scene[ball].position;
        assert!(
            (pushed - start).length() > 0.5,
            "the paddle pushed the ball: {pushed:?}"
        );
        // Removed from the scene, removed from the physics.
        scene.remove(cube);
        physics.step(&mut scene, 1.0 / 60.0);
        assert!(physics.body(cube).is_none());
        assert_eq!(physics.world().len(), 3);
    }

    #[test]
    fn a_tumbling_box_lands_and_sleeps_and_the_scene_does_not_disturb_it() {
        // The scene keeps rotations as Euler angles: reading them back must not count as
        // a move by the code, which would wake the body and nudge it every frame.
        let mut scene = Scene::new();
        let floor = scene.add(Mesh::new(
            PlaneGeometry::new(20.0, 20.0),
            MeshStandardMaterial::new(),
        ));
        scene[floor].rotation.x = -std::f32::consts::FRAC_PI_2;
        let cube = scene.add(Mesh::new(
            BoxGeometry::new(1.0, 0.6, 0.8),
            MeshStandardMaterial::new(),
        ));
        scene[cube].position.set(0.0, 3.0, 0.0);
        scene[cube].rotation.set(0.7, 0.3, 1.1);
        let mut physics = Physics::new();
        for _ in 0..300 {
            physics.step(&mut scene, 1.0 / 60.0);
        }
        assert!(physics.body(cube).unwrap().is_sleeping());
        let rest = scene[cube].position;
        for _ in 0..60 {
            physics.step(&mut scene, 1.0 / 60.0);
        }
        assert!(
            physics.body(cube).unwrap().is_sleeping(),
            "woken by the scene"
        );
        assert_eq!(scene[cube].position, rest);
        // Moved by the code: it falls again.
        scene[cube].position.y += 2.0;
        physics.step(&mut scene, 1.0 / 60.0);
        assert!(!physics.body(cube).unwrap().is_sleeping());
    }
}
