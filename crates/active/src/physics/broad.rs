//! The broad phase: the pairs of bodies whose boxes overlap, found with a uniform grid
//! (the spatial hash of the 2D simulation this engine learned from, in 3D and for bodies
//! of different sizes).

use super::shape::Aabb;
use crate::three::math::Vec3;

/// A body covering more cells than this is tested against every other one instead (a
/// floor, a wall).
const MAX_CELLS: i64 = 512;

type Cell = (i32, i32, i32);

fn cell_of(p: Vec3, inv: f32) -> Cell {
    (
        (p.x * inv).floor() as i32,
        (p.y * inv).floor() as i32,
        (p.z * inv).floor() as i32,
    )
}

/// The pairs `(i, j)`, `i < j`, of `boxes` that overlap, each once.
pub(crate) fn overlapping_pairs(boxes: &[Aabb]) -> Vec<(usize, usize)> {
    let mut pairs = Vec::new();
    if boxes.len() < 2 {
        return pairs;
    }
    // Cells the size of a typical body: a body then sits in a few cells.
    let mut sizes: Vec<f32> = boxes
        .iter()
        .map(|b| {
            let s = b.max - b.min;
            s.x.max(s.y).max(s.z)
        })
        .collect();
    sizes.sort_unstable_by(f32::total_cmp);
    let cell = sizes[sizes.len() / 2].clamp(0.05, 1000.0);
    let inv = 1.0 / cell;

    let mut entries: Vec<(Cell, usize)> = Vec::new();
    let mut oversize = Vec::new();
    for (index, b) in boxes.iter().enumerate() {
        let (lo, hi) = (cell_of(b.min, inv), cell_of(b.max, inv));
        let count =
            i64::from(hi.0 - lo.0 + 1) * i64::from(hi.1 - lo.1 + 1) * i64::from(hi.2 - lo.2 + 1);
        if count > MAX_CELLS {
            oversize.push(index);
            continue;
        }
        for x in lo.0..=hi.0 {
            for y in lo.1..=hi.1 {
                for z in lo.2..=hi.2 {
                    entries.push(((x, y, z), index));
                }
            }
        }
    }
    entries.sort_unstable();
    let mut start = 0;
    while start < entries.len() {
        let key = entries[start].0;
        let mut end = start + 1;
        while end < entries.len() && entries[end].0 == key {
            end += 1;
        }
        for x in start..end {
            for y in x + 1..end {
                let (i, j) = (entries[x].1, entries[y].1);
                let (a, b) = (&boxes[i], &boxes[j]);
                // Each pair once: in the cell holding the low corner of their overlap.
                if a.overlaps(b) && cell_of(a.min.max(b.min), inv) == key {
                    pairs.push((i.min(j), i.max(j)));
                }
            }
        }
        start = end;
    }
    for (n, &i) in oversize.iter().enumerate() {
        for j in 0..boxes.len() {
            // Two large ones once; a large one with every other.
            if j == i || (oversize.contains(&j) && oversize[..n].contains(&j)) {
                continue;
            }
            if boxes[i].overlaps(&boxes[j]) {
                pairs.push((i.min(j), i.max(j)));
            }
        }
    }
    pairs.sort_unstable();
    pairs.dedup();
    pairs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn aabb(x: f32, y: f32, z: f32, half: f32) -> Aabb {
        Aabb {
            min: Vec3::new(x - half, y - half, z - half),
            max: Vec3::new(x + half, y + half, z + half),
        }
    }

    #[test]
    fn finds_every_overlapping_pair_once() {
        let mut boxes: Vec<Aabb> = (0..60)
            .map(|i| aabb((i % 10) as f32 * 0.9, (i / 10) as f32 * 0.9, 0.0, 0.5))
            .collect();
        boxes.push(Aabb {
            min: Vec3::new(-50.0, -2.0, -50.0),
            max: Vec3::new(50.0, -0.4, 50.0),
        });
        boxes.push(Aabb {
            min: Vec3::new(-50.0, -3.0, -50.0),
            max: Vec3::new(50.0, -1.0, 50.0),
        });
        let found = overlapping_pairs(&boxes);
        let mut expected = Vec::new();
        for i in 0..boxes.len() {
            for j in i + 1..boxes.len() {
                if boxes[i].overlaps(&boxes[j]) {
                    expected.push((i, j));
                }
            }
        }
        assert_eq!(found, expected);
    }
}
