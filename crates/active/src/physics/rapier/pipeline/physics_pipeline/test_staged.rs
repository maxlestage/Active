// Rapier 0.36.1 (https://rapier.rs), © Dimforge, under the Apache License 2.0: copied
// into Active and changed (features resolved, paths moved). See README.md and
// LICENSE in crates/active/src/physics/rapier.
//! Staged-solver parity and substep-group partition regression tests.

use crate::physics::rapier::dynamics::RigidBodyBuilder;
use crate::physics::rapier::geometry::ColliderBuilder;
use crate::physics::rapier::math::Vector;
use crate::physics::rapier::prelude::RevoluteJointBuilder;

/// Substep-group partition: the awake body list must reorder into contiguous groups by
/// component max `additional_solver_iterations` (descending), re-stamp the
/// `active_set_id == index` invariant, and not churn the epoch once stable.
#[test]
fn substep_groups_partition() {
    use crate::physics::rapier::alloc_prelude::*;
    use crate::physics::rapier::pipeline::PhysicsWorld;

    let mut world = PhysicsWorld::new();

    let cuboid = || ColliderBuilder::cuboid(0.5, 0.5, 0.5);
    let dynamic = |x: crate::physics::rapier::math::Real, y: crate::physics::rapier::math::Real| {
        RigidBodyBuilder::dynamic()
            .translation(Vector::X * x + Vector::Y * y)
            .can_sleep(false)
    };

    // Default-cadence stack: one box resting on the ground.
    let _ = world.insert(RigidBodyBuilder::fixed(), cuboid());
    let (plain, _) = world.insert(dynamic(0.0, 1.001), cuboid());

    // A two-body jointed pair where only ONE body is elevated: the joint
    // edge must pull the partner into the same (elevated) group.
    let (chain_a, _) = world.insert(dynamic(10.0, 5.0), cuboid());
    let (chain_b, _) = world.insert(dynamic(10.0, 3.0).additional_solver_iterations(4), cuboid());
    let joint = RevoluteJointBuilder::new(Vector::Z);
    world.insert_impulse_joint(chain_a, chain_b, joint);

    // An isolated component at a higher elevation still.
    let (lone, _) = world.insert(
        dynamic(-10.0, 5.0).additional_solver_iterations(8),
        cuboid(),
    );

    for _ in 0..3 {
        world.step();
    }

    // Expect three groups, descending: {lone}=8, {chain_a, chain_b}=4,
    // {plain}=0. The fixed ground is not part of the awake set.
    let groups = &world.islands.solve_groups;
    assert_eq!(
        groups.iter().map(|g| g.extra_iters).collect::<Vec<_>>(),
        vec![8, 4, 0],
    );
    assert_eq!(
        groups
            .iter()
            .map(|g| g.body_range.len())
            .collect::<Vec<_>>(),
        vec![1, 2, 1],
    );
    assert_eq!(groups[0].body_range.start, 0);
    assert_eq!(groups.last().unwrap().body_range.end, 4);

    // Each body sits in its expected group range, and the
    // `active_set_id == index in the awake island's bodies` invariant
    // holds after the reorder.
    let awake_bodies: Vec<_> = world.islands.active_bodies().collect();
    for (i, handle) in awake_bodies.iter().enumerate() {
        assert_eq!(world.bodies[*handle].ids.active_set_id as usize, i);
    }
    let group_of = |h: crate::physics::rapier::dynamics::RigidBodyHandle| {
        let id = world.bodies[h].ids.active_set_id as usize;
        groups.iter().position(|g| g.body_range.contains(&id))
    };
    assert_eq!(group_of(lone), Some(0));
    assert_eq!(group_of(chain_a), Some(1), "joint partner must be lifted");
    assert_eq!(group_of(chain_b), Some(1));
    assert_eq!(group_of(plain), Some(2));

    // Steady state: the grouping is contiguous, so further steps must not bump the
    // active-set epoch (a bump forces a full solver-graph rebuild — re-partitioning
    // every step would tank elevated scenes).
    let epoch = world.islands.active_set_epoch;
    for _ in 0..5 {
        world.step();
    }
    assert_eq!(
        world.islands.active_set_epoch, epoch,
        "stable grouping must not churn the active-set epoch"
    );

    // Un-elevating every body empties the groups (single implicit group).
    world
        .bodies
        .get_mut(lone)
        .unwrap()
        .set_additional_solver_iterations(0);
    world
        .bodies
        .get_mut(chain_b)
        .unwrap()
        .set_additional_solver_iterations(0);
    world.step();
    assert!(world.islands.solve_groups.is_empty());
}
