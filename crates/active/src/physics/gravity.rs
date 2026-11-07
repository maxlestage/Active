//! Bodies that attract each other (Newton's gravitation), computed with a Barnes–Hut
//! octree: a far group of bodies counts as one body at its center of mass, which takes the
//! cost from O(n²) to O(n log n) (the 2D simulation this engine learned from used a
//! quadtree).

use crate::scene::math::Vec3;

/// Mutual attraction between the bodies: every dynamic body pulls the others towards it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Attraction {
    /// The gravitational constant (6.674e-11 in SI units; larger to see it at human scale).
    pub constant: f32,
    /// How coarse the far field may be: 0 sums every pair exactly, 0.5 is accurate,
    /// larger is faster.
    pub theta: f32,
    /// How much close pulls are softened, so that close bodies do not get huge
    /// accelerations: a separation `d` counts as `sqrt(d² + softening²)` (Plummer's
    /// softening).
    pub softening: f32,
}

impl Default for Attraction {
    fn default() -> Attraction {
        Attraction {
            constant: 6.674e-11,
            theta: 0.5,
            softening: 0.05,
        }
    }
}

impl Attraction {
    /// An attraction of this strength.
    pub fn new(constant: f32) -> Attraction {
        Attraction {
            constant,
            ..Attraction::default()
        }
    }
}

#[derive(Clone, Copy)]
struct Node {
    center: Vec3,
    half: f32,
    mass: f32,
    weighted: Vec3,
    /// The first of eight children (0: a leaf).
    children: u32,
    /// The body of a leaf holding one.
    body: Option<u32>,
}

struct Octree {
    nodes: Vec<Node>,
}

impl Octree {
    fn build(positions: &[Vec3], masses: &[f32]) -> Octree {
        let (mut lo, mut hi) = (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY));
        for &p in positions {
            lo = lo.min(p);
            hi = hi.max(p);
        }
        let size = hi - lo;
        let half = size.x.max(size.y).max(size.z) * 0.5 + 1e-3;
        let root = Node {
            center: (lo + hi) * 0.5,
            half,
            mass: 0.0,
            weighted: Vec3::ZERO,
            children: 0,
            body: None,
        };
        let mut tree = Octree { nodes: vec![root] };
        for (i, (&p, &m)) in positions.iter().zip(masses).enumerate() {
            if m > 0.0 {
                tree.insert(i as u32, p, m, positions);
            }
        }
        tree
    }

    fn octant(center: Vec3, p: Vec3) -> usize {
        usize::from(p.x >= center.x)
            | usize::from(p.y >= center.y) << 1
            | usize::from(p.z >= center.z) << 2
    }

    fn split(&mut self, node: usize) {
        let Node { center, half, .. } = self.nodes[node];
        let first = self.nodes.len() as u32;
        let q = half * 0.5;
        for o in 0..8 {
            let offset = Vec3::new(
                if o & 1 != 0 { q } else { -q },
                if o & 2 != 0 { q } else { -q },
                if o & 4 != 0 { q } else { -q },
            );
            self.nodes.push(Node {
                center: center + offset,
                half: q,
                mass: 0.0,
                weighted: Vec3::ZERO,
                children: 0,
                body: None,
            });
        }
        self.nodes[node].children = first;
    }

    fn insert(&mut self, body: u32, p: Vec3, m: f32, positions: &[Vec3]) {
        let mut node = 0;
        // Bodies at the same place share a leaf instead of splitting forever.
        for _ in 0..48 {
            self.nodes[node].mass += m;
            self.nodes[node].weighted += p * m;
            let n = self.nodes[node];
            if n.children == 0 {
                if n.body.is_none() && n.mass == m {
                    self.nodes[node].body = Some(body);
                    return;
                }
                let Some(previous) = n.body else { return };
                self.split(node);
                self.nodes[node].body = None;
                let q = positions[previous as usize];
                let pm = n.mass - m;
                let child = self.nodes[node].children as usize + Self::octant(n.center, q);
                self.nodes[child].mass += pm;
                self.nodes[child].weighted += q * pm;
                self.nodes[child].body = Some(previous);
            }
            node = self.nodes[node].children as usize + Self::octant(self.nodes[node].center, p);
        }
    }

    fn acceleration(&self, i: usize, p: Vec3, cfg: &Attraction) -> Vec3 {
        let soft = cfg.softening * cfg.softening;
        let mut a = Vec3::ZERO;
        let mut stack = vec![0usize];
        while let Some(index) = stack.pop() {
            let n = &self.nodes[index];
            if n.mass <= 0.0 || n.body == Some(i as u32) {
                continue;
            }
            let com = n.weighted / n.mass;
            let d = com - p;
            let dist2 = d.length_squared() + soft;
            // A node holding p is always opened, and the others are judged by their true
            // distance: softening weakens a pull, it does not move its source away.
            let holds = (p.x - n.center.x).abs() <= n.half
                && (p.y - n.center.y).abs() <= n.half
                && (p.z - n.center.z).abs() <= n.half;
            let far = !holds
                && (2.0 * n.half) * (2.0 * n.half) < cfg.theta * cfg.theta * d.length_squared();
            if n.children == 0 || far {
                // (Two bodies at the same point, with no softening: no pull, not NaN.)
                if dist2 > 0.0 {
                    a += d * (cfg.constant * n.mass / (dist2 * dist2.sqrt()));
                }
            } else {
                stack.extend((0..8).map(|o| n.children as usize + o));
            }
        }
        a
    }
}

/// The acceleration of each body due to all the others.
pub(crate) fn accelerations(positions: &[Vec3], masses: &[f32], cfg: &Attraction) -> Vec<Vec3> {
    let soft = cfg.softening * cfg.softening;
    if positions.len() <= 16 || cfg.theta <= 0.0 {
        return (0..positions.len())
            .map(|i| {
                let mut a = Vec3::ZERO;
                for j in 0..positions.len() {
                    if j != i && masses[j] > 0.0 {
                        let d = positions[j] - positions[i];
                        let dist2 = d.length_squared() + soft;
                        if dist2 > 0.0 {
                            a += d * (cfg.constant * masses[j] / (dist2 * dist2.sqrt()));
                        }
                    }
                }
                a
            })
            .collect();
    }
    let tree = Octree::build(positions, masses);
    positions
        .iter()
        .enumerate()
        .map(|(i, &p)| tree.acceleration(i, p, cfg))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn barnes_hut_is_close_to_the_exact_sum() {
        let positions: Vec<Vec3> = (0..200)
            .map(|i| {
                let f = i as f32;
                Vec3::new(
                    (f * 1.37).sin() * 10.0,
                    (f * 0.71).cos() * 10.0,
                    (f * 2.3).sin() * 10.0,
                )
            })
            .collect();
        let masses: Vec<f32> = (0..200).map(|i| 1.0 + (i % 7) as f32).collect();
        let exact = accelerations(
            &positions,
            &masses,
            &Attraction {
                constant: 1.0,
                theta: 0.0,
                softening: 0.1,
            },
        );
        let approx = accelerations(
            &positions,
            &masses,
            &Attraction {
                constant: 1.0,
                theta: 0.4,
                softening: 0.1,
            },
        );
        let errors: Vec<f32> = exact
            .iter()
            .zip(&approx)
            .map(|(e, a)| (*e - *a).length() / e.length().max(1e-6))
            .collect();
        let mean = errors.iter().sum::<f32>() / errors.len() as f32;
        let worst = errors.iter().copied().fold(0.0, f32::max);
        assert!(mean < 0.01 && worst < 0.08, "mean {mean}, worst {worst}");
    }

    #[test]
    fn bodies_at_the_same_point_do_not_pull_with_no_softening() {
        let cfg = Attraction {
            constant: 1.0,
            theta: 0.5,
            softening: 0.0,
        };
        let positions = vec![Vec3::ZERO; 40];
        let masses = vec![1.0; 40];
        for a in accelerations(&positions, &masses, &cfg) {
            assert_eq!(a, Vec3::ZERO);
        }
        let pair = accelerations(&[Vec3::ZERO, Vec3::ZERO], &[1.0, 1.0], &cfg);
        assert!(pair.iter().all(|a| *a == Vec3::ZERO), "{pair:?}");
    }
}
