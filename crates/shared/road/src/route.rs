//! A preliminary route over a terrain (plan/road.md Y12.1, docs/adr/0910 §1; ROAD-ALIGN-13): the least-cost path
//! between two places on a grid of the terrain's heights, as a corridor analysis gives it before an alignment is
//! designed. A step between grid nodes costs its plan length plus `climb` times the height it rises or falls; a step
//! steeper than `grade` (rise over run) is not taken; a node off the terrain or in an area to avoid is not entered. Nodes are
//! joined to their 16 neighbours (the knight's moves too, so a path is not held to 45° turns). Equal costs keep the
//! path found first, the neighbours tried in a fixed order; the nodes' heights are rounded to the micrometre so the
//! same terrain gives the same path wherever it is computed. The result is a line to fit, not an alignment: its fit,
//! its checks and its approval are the alignment's own (`road.alignments.fit`). The limits are the user's; nothing here
//! is a standard's number.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

/// The most nodes a grid holds.
pub const MOST_NODES: usize = 2_000_000;

/// The 16 neighbours' offsets in the order they are tried.
const STEPS: [(i32, i32); 16] = [
    (1, 0),
    (0, 1),
    (-1, 0),
    (0, -1),
    (1, 1),
    (-1, 1),
    (-1, -1),
    (1, -1),
    (2, 1),
    (1, 2),
    (-1, 2),
    (-2, 1),
    (-2, -1),
    (-1, -2),
    (1, -2),
    (2, -1),
];

/// The grid over the two places: its origin (lower left node), its cell and its size in nodes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grid {
    pub origin: [f64; 2],
    pub cell: f64,
    pub cols: usize,
    pub rows: usize,
}

impl Grid {
    /// The grid spanning `a` and `b` and `margin` around them; none when it would hold more than [`MOST_NODES`].
    pub fn over(a: [f64; 2], b: [f64; 2], cell: f64, margin: f64) -> Option<Grid> {
        let origin = [a[0].min(b[0]) - margin, a[1].min(b[1]) - margin];
        let span = [(a[0] - b[0]).abs() + 2.0 * margin, (a[1] - b[1]).abs() + 2.0 * margin];
        let cols = (span[0] / cell).ceil() + 1.0;
        let rows = (span[1] / cell).ceil() + 1.0;
        if !(cols.is_finite() && rows.is_finite()) || cols * rows > MOST_NODES as f64 {
            return None;
        }
        Some(Grid { origin, cell, cols: cols as usize, rows: rows as usize })
    }

    /// A node's place.
    pub fn place(&self, n: usize) -> [f64; 2] {
        [self.origin[0] + (n % self.cols) as f64 * self.cell, self.origin[1] + (n / self.cols) as f64 * self.cell]
    }

    /// The node nearest a place.
    pub fn nearest(&self, p: [f64; 2]) -> usize {
        let i = ((p[0] - self.origin[0]) / self.cell + 0.5).floor().clamp(0.0, (self.cols - 1) as f64) as usize;
        let j = ((p[1] - self.origin[1]) / self.cell + 0.5).floor().clamp(0.0, (self.rows - 1) as f64) as usize;
        j * self.cols + i
    }
}

/// A height rounded to the micrometre.
pub fn rounded(z: f64) -> f64 {
    (z * 1e6 + 0.5).floor() / 1e6
}

/// Why no route was found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoRoute {
    /// The start's or the end's node is off the terrain or in an area to avoid.
    Blocked { end: bool },
    /// No path joins them within the limits.
    Unreached,
}

/// A route found: its nodes from start to end, its cost and how many nodes were settled.
#[derive(Clone, Debug, PartialEq)]
pub struct Found {
    pub nodes: Vec<usize>,
    pub cost: f64,
    pub settled: usize,
}

#[derive(PartialEq)]
struct Open(f64, usize);

impl Eq for Open {}

impl PartialOrd for Open {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Open {
    // The least cost first, then the least node.
    fn cmp(&self, other: &Self) -> Ordering {
        other.0.total_cmp(&self.0).then_with(|| other.1.cmp(&self.1))
    }
}

/// The least-cost path from node `a` to node `b`: `heights[n]` the node's height (none: not entered), `grade` the
/// steepest step (rise over run), `climb` the cost of a metre risen or fallen.
pub fn least_cost(grid: &Grid, heights: &[Option<f64>], a: usize, b: usize, grade: f64, climb: f64) -> Result<Found, NoRoute> {
    if heights[a].is_none() {
        return Err(NoRoute::Blocked { end: false });
    }
    if heights[b].is_none() {
        return Err(NoRoute::Blocked { end: true });
    }
    let n = grid.cols * grid.rows;
    let mut dist = vec![f64::INFINITY; n];
    let mut from = vec![usize::MAX; n];
    let mut done = vec![false; n];
    let steepest = grade;
    let lengths: Vec<f64> = STEPS
        .iter()
        .map(|&(di, dj)| {
            let (dx, dy) = (f64::from(di) * grid.cell, f64::from(dj) * grid.cell);
            (dx * dx + dy * dy).sqrt()
        })
        .collect();
    let mut open = BinaryHeap::new();
    dist[a] = 0.0;
    open.push(Open(0.0, a));
    let mut settled = 0;
    while let Some(Open(d, u)) = open.pop() {
        if done[u] || d > dist[u] {
            continue;
        }
        done[u] = true;
        settled += 1;
        if u == b {
            break;
        }
        let (ui, uj) = ((u % grid.cols) as i64, (u / grid.cols) as i64);
        let Some(zu) = heights[u] else { continue };
        for (k, &(di, dj)) in STEPS.iter().enumerate() {
            let (vi, vj) = (ui + i64::from(di), uj + i64::from(dj));
            if vi < 0 || vj < 0 || vi >= grid.cols as i64 || vj >= grid.rows as i64 {
                continue;
            }
            let v = vj as usize * grid.cols + vi as usize;
            let Some(zv) = heights[v] else { continue };
            if done[v] {
                continue;
            }
            let rise = (zv - zu).abs();
            if rise > steepest * lengths[k] + 1e-9 {
                continue;
            }
            let next = d + (lengths[k] + climb * rise);
            if next < dist[v] {
                dist[v] = next;
                from[v] = u;
                open.push(Open(next, v));
            }
        }
    }
    if !dist[b].is_finite() {
        return Err(NoRoute::Unreached);
    }
    let mut nodes = vec![b];
    while let Some(&last) = nodes.last() {
        if last == a {
            break;
        }
        nodes.push(from[last]);
    }
    nodes.reverse();
    Ok(Found { nodes, cost: dist[b], settled })
}

/// The points of `line` Douglas–Peucker keeps within `tolerance` (in plan): the ends, and recursively the farthest
/// point from the chord when farther than `tolerance` (the first of equal ones).
pub fn simplified(line: &[[f64; 3]], tolerance: f64) -> Vec<[f64; 3]> {
    if line.len() < 3 || tolerance <= 0.0 {
        return line.to_vec();
    }
    let mut keep = vec![false; line.len()];
    keep[0] = true;
    keep[line.len() - 1] = true;
    let mut stack = vec![(0, line.len() - 1)];
    while let Some((i, j)) = stack.pop() {
        let (a, b) = (line[i], line[j]);
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let span = (dx * dx + dy * dy).sqrt();
        let (mut far, mut at) = (0.0, 0);
        for (k, p) in line.iter().enumerate().take(j).skip(i + 1) {
            let d = if span > 0.0 {
                ((p[0] - a[0]) * dy - (p[1] - a[1]) * dx).abs() / span
            } else {
                ((p[0] - a[0]).powi(2) + (p[1] - a[1]).powi(2)).sqrt()
            };
            if d > far {
                far = d;
                at = k;
            }
        }
        if far > tolerance {
            keep[at] = true;
            stack.push((at, j));
            stack.push((i, at));
        }
    }
    line.iter().zip(&keep).filter(|(_, k)| **k).map(|(p, _)| *p).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_flat_grid_goes_straight_and_a_ridge_is_avoided() {
        let grid = Grid::over([0.0, 0.0], [10.0, 0.0], 1.0, 2.0).expect("a grid");
        assert_eq!((grid.cols, grid.rows), (15, 5));
        let flat = vec![Some(100.0); grid.cols * grid.rows];
        let (a, b) = (grid.nearest([0.0, 0.0]), grid.nearest([10.0, 0.0]));
        let f = least_cost(&grid, &flat, a, b, 10.0, 5.0).expect("found");
        assert!((f.cost - 10.0).abs() < 1e-12);
        assert_eq!(f.nodes.len(), 11);
        // A wall across the middle, open only at its top row: the path goes around.
        let mut walled = flat.clone();
        for j in 0..grid.rows - 1 {
            walled[j * grid.cols + 7] = None;
        }
        let g = least_cost(&grid, &walled, a, b, 10.0, 5.0).expect("found");
        assert!(g.cost > 10.0);
        assert!(g.nodes.iter().any(|&n| n / grid.cols == grid.rows - 1 || n % grid.cols != 7));
        walled[(grid.rows - 1) * grid.cols + 7] = None;
        // The knight's step jumps a one-node wall; a two-node wall stops it.
        for j in 0..grid.rows {
            walled[j * grid.cols + 8] = None;
        }
        assert_eq!(least_cost(&grid, &walled, a, b, 10.0, 5.0), Err(NoRoute::Unreached));
    }

    #[test]
    fn simplifying_keeps_the_corners() {
        let line = [[0.0, 0.0, 0.0], [1.0, 0.01, 0.0], [2.0, 0.0, 0.0], [2.0, 2.0, 0.0]];
        assert_eq!(simplified(&line, 0.1), vec![[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [2.0, 2.0, 0.0]]);
    }
}
