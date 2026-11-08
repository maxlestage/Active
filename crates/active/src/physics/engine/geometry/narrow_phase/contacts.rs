// Copyright 2020 Sébastien Crozet, under the Apache License 2.0 (see LICENSE in
// crates/active/src/physics/engine). Changed for Active.
//! `NarrowPhase::compute_contacts`: the per-step contact-update drivers
//! (single-threaded loop and broadcast parallel-for over the update
//! candidates), plus the deferred solver-graph coloring pass.

use super::pair_update::{
    self, HintsPtr, OUTCOME_CLEARED_IN_GRAPH, OUTCOME_FULL, OUTCOME_FULL_COMPOSITE,
    OUTCOME_RECYCLED_REQUALIFIED, PairTransition,
};
use super::soft_contacts::SoftDetectionCtx;
use super::{
    NarrowPhase, assign_pair_solver_color, clear_pair_solver_color, collect_pairs_to_update,
    pack_color_body_info, strong_wake_sleeping_side, unpack_color_body_info,
};
use crate::physics::engine::alloc_prelude::*;
use crate::physics::engine::dynamics::{
    ImpulseJointSet, IntegrationParameters, IslandManager, MultibodyJointSet, RigidBodySet,
    SoftBodySet,
};
use crate::physics::engine::geometry::{ColliderHandle, ColliderSet, ContactPair};
use crate::physics::engine::math::Real;
use crate::physics::engine::pipeline::{ActiveEvents, EventHandler, PhysicsHooks};

impl NarrowPhase {
    pub(crate) fn compute_contacts(
        &mut self,
        prediction_distance: Real,
        dt: Real,
        contact_clustering: bool,
        // Contact-recycling drift threshold; `0.0` disables recycling.
        contact_recycle_distance: Real,
        islands: &mut IslandManager,
        bodies: &mut RigidBodySet,
        colliders: &ColliderSet,
        impulse_joints: &ImpulseJointSet,
        multibody_joints: &MultibodyJointSet,
        modified_colliders: &[ColliderHandle],
        hooks: &dyn PhysicsHooks,
        events: &dyn EventHandler,
        // The soft bodies and the step's parameters: the soft contact detection runs on
        // the soft-surface pairs and, after the transitions, on the awake self-colliding
        // meshes (`None`: no soft contact detection, as in the collision pipeline).
        soft_bodies: Option<(&SoftBodySet, &IntegrationParameters)>,
    ) {
        self.update_awake_body_mask(islands);
        let awake_body_mask = core::mem::take(&mut self.awake_body_mask);
        let soft_ctx = soft_bodies.map(|(soft_bodies, params)| SoftDetectionCtx {
            soft_bodies,
            bodies: &*bodies,
            colliders,
            params,
            dt,
        });
        let soft_ctx = soft_ctx.as_ref();

        // The solver hints follow the contact-graph edge indexing; removals are
        // mirrored eagerly, so this resize only matters for appended pairs (their
        // hint is written by the pair-update below) and after a deserialization.
        self.pair_solver_hints
            .resize(self.contact_graph.graph.edges.len(), 0);
        let hints_ptr = HintsPtr(self.pair_solver_hints.as_mut_ptr());
        let hints_ptr = &hints_ptr;

        let query_dispatcher = &*self.query_dispatcher;

        // Edges fully updated this step (bucket membership may have changed) for the
        // incremental maintenance of `solver_contact_graph`; the parallel path
        // collects them through its fold/reduce below.
        let mut solver_graph_dirty = {
            let mut d = core::mem::take(&mut self.solver_graph_dirty);
            d.clear();
            d
        };
        // Set when any composite pair (unstable manifold ordinals) was fully
        // updated this step, forcing a from-scratch solver-graph rebuild.
        let mut force_full_rebuild = false;

        // Only iterate on pairs involving at least one changed collider instead of
        // the whole graph, which can be very large when most pairs are asleep.
        let mut update_candidates = core::mem::take(&mut self.update_candidates);
        collect_pairs_to_update(
            &mut update_candidates,
            &self.graph_indices,
            &self.contact_graph.graph,
            islands,
            bodies,
            colliders,
            modified_colliders,
            |gid| gid.contact_graph_index,
        );

        // Begin/end-touch transitions detected during the update; applied by the
        // sorted post-loop pass (see `PairTransition`).
        let mut transitions: Vec<PairTransition> = Vec::new();
        let process_pair = |edge: &mut crate::physics::engine::data::graph::Edge<ContactPair>,
                            edge_id: u32| {
            pair_update::process_pair(
                edge,
                edge_id,
                prediction_distance,
                dt,
                contact_clustering,
                contact_recycle_distance,
                bodies,
                colliders,
                impulse_joints,
                multibody_joints,
                hooks,
                query_dispatcher,
                &awake_body_mask,
                hints_ptr,
                soft_ctx,
                &mut transitions,
            )
        };
        // Takes the hooks as an argument rather than capturing them: under
        // `unsync-callbacks` the workers are handed a `&()` (a no-op `PhysicsHooks` that is
        // `Sync`) and only this thread ever passes the real `hooks`. Then the user hooks
        // are actually applied in another `Sync`-friendly pass.

        // The pairs are accessed through an index array in an order the hardware
        // prefetcher can't predict; prefetch a few iterations ahead.
        const PREFETCH_AHEAD: usize = 4;

        {
            let mut process_pair = process_pair;
            let edges = &mut self.contact_graph.graph.edges;
            for (i, id) in update_candidates.iter().enumerate() {
                if let Some(next_id) = update_candidates.get(i + PREFETCH_AHEAD) {
                    crate::physics::engine::utils::prefetch_read::<2, _>(&edges[*next_id as usize]);
                }
                match process_pair(&mut edges[*id as usize], *id) {
                    OUTCOME_RECYCLED_REQUALIFIED | OUTCOME_FULL => {
                        solver_graph_dirty.push(*id);
                    }
                    OUTCOME_FULL_COMPOSITE | OUTCOME_CLEARED_IN_GRAPH => {
                        solver_graph_dirty.push(*id);
                        force_full_rebuild = true;
                    }
                    _ => {}
                }
            }

            // Canonical order, matching what the parallel path's fold + sort produces.
            // The candidates are gathered per-collider adjacency in the sparse-awake
            // regime, so pushing in visit order is NOT ascending edge id — and this list
            // drives `reconcile_pair` (bucket membership, hence solve order) and the
            // force-event reconciliation, both of which are order-sensitive.
            solver_graph_dirty.sort_unstable();
        }

        self.apply_pair_transitions(&mut transitions, islands, bodies, colliders, events);

        // The self contacts, once the transitions woke the bodies a new pair touches (the context
        // is rebuilt: the transitions borrowed the bodies mutably).
        if let Some((soft_bodies, params)) = soft_bodies {
            let ctx = SoftDetectionCtx {
                soft_bodies,
                bodies: &*bodies,
                colliders,
                params,
                dt,
            };
            self.update_soft_self_contacts(&ctx, bodies);
        }

        self.update_candidates = update_candidates;
        self.awake_body_mask = awake_body_mask;
        {
            self.solver_graph_dirty = solver_graph_dirty;
            if force_full_rebuild {
                // A composite pair rebuilt its manifolds: invalidate the graph so
                // the next maintenance pass rebuilds it from scratch (see
                // `OUTCOME_FULL_COMPOSITE`).
                self.solver_graph_valid = false;
            }
        }
    }

    /// Applies the begin/end-touch transitions recorded by the pair update, in
    /// sorted edge-id order: event emission, sleeping-side wake-up, solver
    /// coloring, and persistent-island link/unlink. The sort makes the resulting
    /// island and coloring state independent of the update schedule (parallel
    /// completion order or serial iteration order).
    fn apply_pair_transitions(
        &mut self,
        transitions: &mut [PairTransition],
        islands: &mut IslandManager,
        bodies: &mut RigidBodySet,
        colliders: &ColliderSet,
        events: &dyn EventHandler,
    ) {
        transitions.sort_unstable_by_key(|&(edge_id, ..)| edge_id);

        let mut color_todo = core::mem::take(&mut self.solver_color_todo);
        color_todo.clear();
        for &(edge_id, parent1, parent2, any_active_contact) in transitions.iter() {
            {
                let pair = &mut self.contact_graph.graph.edges[edge_id as usize].weight;
                let co1 = &colliders[pair.collider1];
                let co2 = &colliders[pair.collider2];
                let active_events = co1.flags.active_events | co2.flags.active_events;
                if active_events.contains(ActiveEvents::COLLISION_EVENTS) {
                    if any_active_contact {
                        pair.emit_start_event(bodies, colliders, events);
                    } else {
                        pair.emit_stop_event(bodies, colliders, events);
                    }
                }
                // Persistent solver-graph color: begin-touch coloring is deferred to
                // the sorted coloring pass (order-insensitive greedy); end-touch frees
                // its color first so that pass sees it.
                if !any_active_contact {
                    clear_pair_solver_color(&mut self.body_solver_color_masks, pair);
                }
            }

            // Wake rule (whole-island sleep): starts wake unconditionally; stops
            // never wake (touching implies same island, so a moving support means the
            // island is already awake — support loss can't strand a sleeping body).
            if any_active_contact {
                strong_wake_sleeping_side(islands, bodies, parent1, parent2);
                let body_info = |h: Option<crate::physics::engine::dynamics::RigidBodyHandle>| {
                    h.map(|h| {
                        let rb = &bodies[h];
                        (h.into_raw_parts().0, rb.is_fixed())
                    })
                };
                color_todo.push((
                    edge_id,
                    pack_color_body_info(body_info(parent1)),
                    pack_color_body_info(body_info(parent2)),
                ));
            }

            islands.interaction_changed(bodies, parent1, parent2, any_active_contact);
            // Persistent islands: a touching transition links/unlinks the
            // pair's contact edge (link merges the two islands).
            if any_active_contact {
                islands
                    .persistent
                    .link_contact(bodies, edge_id, parent1, parent2);
            } else {
                islands.persistent.unlink_contact(edge_id);
            }
        }
        self.apply_deferred_solver_coloring(&mut color_todo);
        self.solver_color_todo = color_todo;
    }

    /// Greedily colors `color_todo` entries (`(edge id, packed body infos)`, see
    /// [`pack_color_body_info`]) after sorting into canonical `(min, max body id)` order,
    /// making the coloring discovery-order independent (first-fit packs ≈ Δ colors, not ≈ 2Δ).
    fn apply_deferred_solver_coloring(&mut self, color_todo: &mut [(u32, u32, u32)]) {
        let color_id = |i: u32| if i == u32::MAX { u32::MAX } else { i >> 1 };
        color_todo.sort_unstable_by_key(|&(edge, i1, i2)| {
            let (a, b) = (color_id(i1), color_id(i2));
            (((a.min(b) as u64) << 32) | a.max(b) as u64, edge)
        });
        let edges = &mut self.contact_graph.graph.edges;
        let masks = &mut self.body_solver_color_masks;
        for &(edge, i1, i2) in color_todo.iter() {
            assign_pair_solver_color(
                masks,
                &mut edges[edge as usize].weight,
                unpack_color_body_info(i1),
                unpack_color_body_info(i2),
            );
        }
    }
}
