// Rapier 0.36.1 (https://rapier.rs), © Dimforge, under the Apache License 2.0: copied
// into Active and changed (features resolved, paths moved). See README.md and
// LICENSE in crates/active/src/physics/rapier.
//! The intersection (sensor) pair update: filters and updates every
//! intersection pair involving a moved or user-modified collider.

use super::{NarrowPhase, collect_pairs_to_update};
use crate::physics::rapier::alloc_prelude::*;
use crate::physics::rapier::dynamics::{IslandManager, RigidBodySet, RigidBodyType};
use crate::physics::rapier::geometry::{ColliderHandle, ColliderSet, IntersectionPair};
use crate::physics::rapier::pipeline::{
    ActiveEvents, ActiveHooks, EventHandler, PairFilterContext, PhysicsHooks,
};

impl NarrowPhase {
    #[profiling::function]
    pub(crate) fn compute_intersections(
        &mut self,
        islands: &IslandManager,
        bodies: &RigidBodySet,
        colliders: &ColliderSet,
        modified_colliders: &[ColliderHandle],
        hooks: &dyn PhysicsHooks,
        events: &dyn EventHandler,
    ) {
        self.update_awake_body_mask(islands);
        let awake_body_mask = core::mem::take(&mut self.awake_body_mask);

        // Only iterate on pairs involving at least one changed collider instead of
        // the whole graph, which can be very large when most pairs are asleep.
        let mut update_candidates = core::mem::take(&mut self.update_candidates);
        collect_pairs_to_update(
            &mut update_candidates,
            &self.graph_indices,
            &self.intersection_graph.graph,
            islands,
            bodies,
            colliders,
            modified_colliders,
            |gid| gid.intersection_graph_index,
        );

        let nodes = &self.intersection_graph.graph.nodes;
        let query_dispatcher = &*self.query_dispatcher;

        // Takes the hooks as an argument rather than capturing them, so that under
        // `unsync-callbacks` the workers can be handed a `&()` (a no-op `PhysicsHooks` that
        // is `Sync`) while only this thread passes the real `hooks`.
        let process_pair =
            |edge: &mut crate::physics::rapier::data::graph::Edge<IntersectionPair>,
             hooks: &dyn PhysicsHooks| {
                let handle1 = nodes[edge.source().index()].weight;
                let handle2 = nodes[edge.target().index()].weight;
                let had_intersection = edge.weight.intersecting;
                let co1 = &colliders[handle1];
                let co2 = &colliders[handle2];
                let rb_handle1 = co1.parent.map(|p| p.handle);
                let rb_handle2 = co2.parent.map(|p| p.handle);

                'emit_events: {
                    let body_awake = |co: &crate::physics::rapier::geometry::Collider| {
                        co.parent.as_ref().is_some_and(|p| {
                            awake_body_mask
                                .get(p.handle.into_raw_parts().0 as usize)
                                .copied()
                                .unwrap_or(false)
                        })
                    };
                    if !co1.changes.needs_narrow_phase_update()
                        && !co2.changes.needs_narrow_phase_update()
                        && !body_awake(co1)
                        && !body_awake(co2)
                    {
                        // Neither collider was changed by the user nor possibly moved by
                        // the simulation (its parent body is asleep or fixed).
                        return None;
                    }

                    if rb_handle1 == rb_handle2 && co1.parent.is_some() {
                        // Same parents. Ignore collisions.
                        edge.weight.intersecting = false;
                        break 'emit_events;
                    }
                    // TODO: avoid lookup into bodies.
                    let mut rb_type1 = RigidBodyType::Fixed;
                    let mut rb_type2 = RigidBodyType::Fixed;

                    if let Some(co_parent1) = &co1.parent {
                        rb_type1 = bodies[co_parent1.handle].body_type;
                    }

                    if let Some(co_parent2) = &co2.parent {
                        rb_type2 = bodies[co_parent2.handle].body_type;
                    }

                    // Filter based on the rigid-body types.
                    if !co1.flags.active_collision_types.test(rb_type1, rb_type2)
                        && !co2.flags.active_collision_types.test(rb_type1, rb_type2)
                    {
                        edge.weight.intersecting = false;
                        break 'emit_events;
                    }

                    // Filter based on collision groups.
                    if !co1.flags.collision_groups.test(co2.flags.collision_groups) {
                        edge.weight.intersecting = false;
                        break 'emit_events;
                    }

                    let active_hooks = co1.flags.active_hooks | co2.flags.active_hooks;

                    if active_hooks.contains(ActiveHooks::FILTER_INTERSECTION_PAIR) {
                        let context = PairFilterContext {
                            bodies,
                            colliders,
                            rigid_body1: rb_handle1,
                            rigid_body2: rb_handle2,
                            collider1: handle1,
                            collider2: handle2,
                        };

                        if !hooks.filter_intersection_pair(&context) {
                            // No intersection allowed.
                            edge.weight.intersecting = false;
                            break 'emit_events;
                        }
                    }

                    let pos12 = co1.pos.inv_mul(&co2.pos);
                    edge.weight.intersecting = query_dispatcher
                        .intersection_test(&pos12, &*co1.shape, &*co2.shape)
                        .is_ok_and(|hit| hit.intersecting);
                }

                let active_events = co1.flags.active_events | co2.flags.active_events;

                // Event emission is deferred so the parallel path emits in candidate
                // order (not scheduling order), identical to the serial path.
                if active_events.contains(ActiveEvents::COLLISION_EVENTS)
                    && had_intersection != edge.weight.intersecting
                {
                    return Some(edge.weight.intersecting);
                }
                None
            };

        let deferred_events: Vec<(u32, bool)> = update_candidates
            .iter()
            .filter_map(|id| {
                process_pair(
                    &mut self.intersection_graph.graph.edges[*id as usize],
                    hooks,
                )
                .map(|started| (*id, started))
            })
            .collect();

        for (id, started) in deferred_events {
            let edge = &mut self.intersection_graph.graph.edges[id as usize];
            let handle1 = nodes[edge.source().index()].weight;
            let handle2 = nodes[edge.target().index()].weight;
            if started {
                edge.weight
                    .emit_start_event(bodies, colliders, handle1, handle2, events);
            } else {
                edge.weight
                    .emit_stop_event(bodies, colliders, handle1, handle2, events);
            }
        }

        self.update_candidates = update_candidates;
        self.awake_body_mask = awake_body_mask;
    }
}
