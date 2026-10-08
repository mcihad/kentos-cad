//! Sınır çıkar's rings (docs/adr/0207 §7): the filled cells of a grid
//! (`raster::Frame`, row 0 at the top) taken together as areas with exact
//! right-angled edges: the edges between a filled cell and an empty one (or
//! the outside), the filled side on the left, joined into rings; where two
//! filled cells touch only at a corner the rings turn left, so they stay
//! apart; points on a straight run are dropped. Rings running
//! counter-clockwise are outer rings, the others holes, each hole in the
//! smallest outer ring around it. The parts in the order of their first
//! cell (top row first, then left to right).

use std::collections::HashMap;

use super::raster::Frame;

/// An area: its outer ring and holes, as world points (not closed: the last
/// point is not the first again).
#[derive(Clone, Debug, PartialEq)]
pub struct Part {
    pub outer: Vec<[f64; 2]>,
    pub holes: Vec<Vec<[f64; 2]>>,
}

/// A grid corner: column and row of the corner lines (0 … cols, 0 … rows).
type Corner = (i64, i64);

/// The rings of the filled cells, as grid corners.
fn rings(filled: &[bool], cols: usize, rows: usize) -> Vec<Vec<Corner>> {
    let at = |c: i64, r: i64| -> bool {
        c >= 0
            && r >= 0
            && (c as usize) < cols
            && (r as usize) < rows
            && filled[r as usize * cols + c as usize]
    };
    // Directed edges, the filled cell on the left. Rows grow downwards: in
    // corner coordinates (c, r) a step "up" on the drawing is r − 1.
    let mut out_of: HashMap<Corner, Vec<Corner>> = HashMap::new();
    let mut edges = 0usize;
    for r in 0..rows as i64 {
        for c in 0..cols as i64 {
            if !at(c, r) {
                continue;
            }
            // Corners: top-left (c, r), top-right (c+1, r), bottom-right (c+1, r+1), bottom-left (c, r+1).
            // Counter-clockwise on the drawing (y up): bottom-left → bottom-right → top-right → top-left.
            if !at(c, r + 1) {
                out_of.entry((c, r + 1)).or_default().push((c + 1, r + 1));
                edges += 1;
            }
            if !at(c + 1, r) {
                out_of.entry((c + 1, r + 1)).or_default().push((c + 1, r));
                edges += 1;
            }
            if !at(c, r - 1) {
                out_of.entry((c + 1, r)).or_default().push((c, r));
                edges += 1;
            }
            if !at(c - 1, r) {
                out_of.entry((c, r)).or_default().push((c, r + 1));
                edges += 1;
            }
        }
    }
    let mut starts: Vec<Corner> = out_of.keys().copied().collect();
    // The rings start in a fixed order: top row first, then left to right.
    starts.sort_unstable_by_key(|&(c, r)| (r, c));
    let mut used = 0usize;
    let mut out = Vec::new();
    for s in starts {
        while let Some(first) = out_of
            .get_mut(&s)
            .and_then(|v| (!v.is_empty()).then(|| v.remove(0)))
        {
            let mut ring = vec![s];
            let (mut prev, mut cur) = (s, first);
            used += 1;
            while cur != s {
                ring.push(cur);
                let dir = (cur.0 - prev.0, cur.1 - prev.1);
                let Some(next) = out_of.get_mut(&cur) else {
                    break;
                };
                // Where two leave a corner, the left turn: the ring keeps round the
                // cell it came along. On the drawing (y up, rows down) the left of a
                // step (dc, dr) is (dr, −dc) in rows and columns.
                let pick = if next.len() > 1 {
                    let left = (dir.1, -dir.0);
                    next.iter()
                        .position(|n| (n.0 - cur.0, n.1 - cur.1) == left)
                        .unwrap_or(0)
                } else {
                    0
                };
                let n = next.remove(pick);
                used += 1;
                prev = cur;
                cur = n;
            }
            out.push(ring);
        }
    }
    debug_assert_eq!(used, edges);
    out
}

/// A ring's corners with the points on a straight run dropped.
fn simplify(ring: &[Corner]) -> Vec<Corner> {
    let n = ring.len();
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let a = ring[(i + n - 1) % n];
        let b = ring[i];
        let c = ring[(i + 1) % n];
        let (d1, d2) = ((b.0 - a.0, b.1 - a.1), (c.0 - b.0, c.1 - b.1));
        if d1.0 * d2.1 - d1.1 * d2.0 != 0 {
            out.push(b);
        }
    }
    out
}

/// Twice a ring's signed area in grid units, counter-clockwise on the drawing positive.
fn area2(ring: &[Corner]) -> i64 {
    let n = ring.len();
    let mut s = 0i64;
    for i in 0..n {
        let (a, b) = (ring[i], ring[(i + 1) % n]);
        // Rows grow downwards: the drawing's y is −r.
        s += a.0 * (-b.1) - b.0 * (-a.1);
    }
    s
}

/// Whether the grid point (`x`, `y`) (half-cell coordinates, never on an edge) is inside the ring.
fn inside(ring: &[Corner], x: f64, y: f64) -> bool {
    let n = ring.len();
    let mut odd = false;
    for i in 0..n {
        let (a, b) = (ring[i], ring[(i + 1) % n]);
        let (ax, ay, bx, by) = (a.0 as f64, a.1 as f64, b.0 as f64, b.1 as f64);
        if (ay > y) != (by > y) && x < ax + (y - ay) / (by - ay) * (bx - ax) {
            odd = !odd;
        }
    }
    odd
}

/// The areas of the filled cells of `frame` (`filled` by cell, row by row from the top).
pub fn parts(frame: &Frame, filled: &[bool]) -> Vec<Part> {
    let all = rings(filled, frame.cols, frame.rows);
    let mut outers: Vec<(Vec<Corner>, i64)> = Vec::new();
    // A hole and the centre of an empty cell inside it: right of its first
    // edge (the filled side is on the left), on half cells, never on an edge.
    let mut holes: Vec<(Vec<Corner>, (f64, f64))> = Vec::new();
    for raw in all {
        let (a, b) = (raw[0], raw[1 % raw.len()]);
        let r = simplify(&raw);
        if r.len() < 4 {
            continue;
        }
        let area = area2(&r);
        if area > 0 {
            outers.push((r, area));
        } else {
            let (dc, dr) = ((b.0 - a.0) as f64, (b.1 - a.1) as f64);
            let mid = ((a.0 + b.0) as f64 / 2.0, (a.1 + b.1) as f64 / 2.0);
            // Right of a step (dc, dr) on the drawing is (−dr, dc) in columns and rows.
            holes.push((r, (mid.0 - 0.5 * dr, mid.1 + 0.5 * dc)));
        }
    }
    let mut parts: Vec<(Vec<Corner>, Vec<Vec<Corner>>)> = outers
        .iter()
        .map(|(r, _)| (r.clone(), Vec::new()))
        .collect();
    for (h, probe) in holes {
        let best = outers
            .iter()
            .enumerate()
            .filter(|(_, (r, _))| inside(r, probe.0, probe.1))
            .min_by_key(|(_, (_, a))| *a)
            .map(|(i, _)| i);
        if let Some(i) = best {
            parts[i].1.push(h);
        }
    }
    let world = |r: &[Corner]| -> Vec<[f64; 2]> {
        r.iter()
            .map(|&(c, rr)| {
                [
                    frame.x0 + c as f64 * frame.cell,
                    frame.top - rr as f64 * frame.cell,
                ]
            })
            .collect()
    };
    parts
        .into_iter()
        .map(|(o, hs)| Part {
            outer: world(&o),
            holes: hs.iter().map(|h| world(h)).collect(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(cols: usize, rows: usize) -> Frame {
        Frame {
            x0: 0.0,
            top: rows as f64,
            cell: 1.0,
            cols,
            rows,
        }
    }

    #[test]
    fn a_ring_with_a_hole_and_a_corner_touch() {
        // 4 × 4: a ring of cells around an empty middle, and a cell touching it at a corner.
        let f = frame(5, 5);
        let mut filled = vec![false; 25];
        for (c, r) in [
            (0, 0),
            (1, 0),
            (2, 0),
            (0, 1),
            (2, 1),
            (0, 2),
            (1, 2),
            (2, 2),
            (3, 3),
        ] {
            filled[r * 5 + c] = true;
        }
        let p = parts(&f, &filled);
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].outer.len(), 4);
        assert_eq!(p[0].holes.len(), 1);
        assert_eq!(p[0].holes[0].len(), 4);
        // The ring's outer: x 0…3, y 2…5 (rows 0…2 from the top of 5).
        let xs: Vec<f64> = p[0].outer.iter().map(|q| q[0]).collect();
        assert!(xs.contains(&0.0) && xs.contains(&3.0));
        assert_eq!(p[1].outer.len(), 4);
        assert!(p[1].holes.is_empty());
    }
}
