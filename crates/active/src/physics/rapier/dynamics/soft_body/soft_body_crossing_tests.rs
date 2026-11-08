// Rapier 0.36.1 (https://rapier.rs), © Dimforge, under the Apache License 2.0: copied
// into Active and changed (features resolved, paths moved). See README.md and
// LICENSE in crates/active/src/physics/rapier.
//! Exact crossing predicates shared by the tangle detection and the volume constraints.

use crate::physics::rapier::math::Vector;
use crate::physics::rapier::utils::DotProduct;

/// Whether two triangles cross: any edge of one pierces the other. For a self pair, an
/// edge touching the other triangle's vertices is skipped (the fan around a shared vertex
/// is contact, not crossing), matching the recovery's self-crossing predicate.
pub(crate) fn tri_pair_crosses(
    ids_a: &[u32],
    pa: &[Vector; 3],
    ids_b: &[u32],
    qb: &[Vector; 3],
    is_self: bool,
) -> bool {
    let edge_hits = |ids_e: &[u32], pe: &[Vector; 3], ids_t: &[u32], pt: &[Vector; 3]| {
        for k in 0..3 {
            let (i, j) = (k, (k + 1) % 3);
            if is_self && (ids_t.contains(&ids_e[i]) || ids_t.contains(&ids_e[j])) {
                continue;
            }
            if segment_crosses_triangle(pe[i], pe[j], pt[0], pt[1], pt[2]) {
                return true;
            }
        }
        false
    };
    edge_hits(ids_a, pa, ids_b, qb) || edge_hits(ids_b, qb, ids_a, pa)
}

/// Whether the open segment strictly pierces the triangle (sign tests; touching or
/// degenerate configurations never cross).
pub(crate) fn segment_crosses_triangle(
    p: Vector,
    q: Vector,
    a: Vector,
    b: Vector,
    c: Vector,
) -> bool {
    let n = (b - a).cross(c - a);
    let dp = (p - a).gdot(n);
    let dq = (q - a).gdot(n);
    if dp == 0.0 || dq == 0.0 || dp.signum() == dq.signum() {
        return false;
    }
    let d = q - p;
    let s1 = (a - p).cross(b - p).gdot(d);
    let s2 = (b - p).cross(c - p).gdot(d);
    let s3 = (c - p).cross(a - p).gdot(d);
    (s1 > 0.0 && s2 > 0.0 && s3 > 0.0) || (s1 < 0.0 && s2 < 0.0 && s3 < 0.0)
}
