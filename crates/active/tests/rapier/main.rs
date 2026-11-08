// Rapier 0.36.1 (https://rapier.rs), © Dimforge, under the Apache License 2.0: copied
// into Active and changed (features resolved, paths moved). See README.md and
// LICENSE in crates/active/src/physics/rapier.
//! The tests of Rapier, run on the copy inside Active.

#![cfg(feature = "physics")]
#![allow(clippy::all, unused)]

mod additional_solver_iterations;
mod broad_phase_pair_filter;
mod ccd_default_vs_fixed;
mod ccd_split_step;
mod collision_pipeline_moving_bodies;
mod contact_clustering;
mod contact_force_event_clustering;
mod contact_force_event_first_tick;
mod gyroscopic;
mod heightfield_solver_graph;
mod issue_182_heightfield_seam;
mod issue_217_ccd_large_dt_hitch;
mod issue_287_kinematic_wakes_jointed_dynamic;
mod issue_309_locked_rotations_offset_com;
mod issue_372_trimesh_sleep;
mod issue_485_kcc_depenetration;
mod issue_499_angular_limits;
mod issue_524_thin_slab_trimesh_tunnel;
mod issue_666_additional_mass_inertia;
mod issue_692_joint_get_mut_wakes_bodies;
mod issue_717_trimesh_result;
mod issue_730_many_separate_colliders_perf;
mod issue_734_collider_reparenting;
mod issue_746_prismatic_axis_frames;
mod issue_749_query_after_collider_removal;
mod issue_78_additional_mass_rest;
mod issue_792_coupled_angular_spring;
mod issue_810_cubes_thin_cylinder_tunnel;
mod issue_836_physics_world;
mod issue_856_motor_position_rotating_base;
mod issue_862_kcc_impulse_point;
mod issue_868_same_machine_determinism;
mod issue_925_wheel_rotation;
mod issue_932_low_ccd_substeps_stutter;
mod issue_946_vehicle_braking_friction;
mod issue_952_simd_joint_offset_com;
mod issue_968_multibody_weld_settle;
mod issue_970_multi_collider_body_perf;
mod issue_974_restitution;
mod joint_assembly_persistence;
mod joint_contact_solve_order;
mod joint_stability;
mod loop_closure_com_repro;
mod miri_scenes;
mod multibody_armature;
mod multibody_coupling;
mod multibody_coupling_topology;
mod multibody_dof_inertia;
mod multibody_spring;
mod persistent_islands;
mod pipeline_counters;
mod query_pipeline_subshapes;
mod simd_backend_parity;
mod sleep_wake;
mod sleep_wide_bodies;
mod soft_bodies;
mod soft_body_clusters;
mod soft_body_grab;
mod soft_body_joints;
mod soft_body_meshes;
mod soft_recovery3;
mod soft_wires;
mod solver_graph_stale_refs;
mod speed_cap;
mod substep_chain_high_mass_ratio;
mod total_contact_impulse;
mod whole_island_sleep;
mod zero_dt_step;
