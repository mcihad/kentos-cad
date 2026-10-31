//! The walls a slab's outline round a seed can read (docs/adr/0782 §2): the drawing's walls on a level fall into
//! pieces, walls whose reach boxes come within `GAP` of each other in one piece, and only the pieces whose box holds
//! the seed (with the pieces their box meets, transitively) can bound, hold or hole the face round it. The first wall's
//! piece goes too: the overlay moves its work to the first face line's start (`overlay::origin_of`), and the same
//! origin keeps the answer the whole level's to the last bit. A wall of another piece is at least `GAP` from every wall
//! taken, so no join, crossing or snap reaches across. `wall_pieces_tests` compares the two on random drawings.

use kentos_contracts::Entity;

use crate::document::Document;
use crate::space_follow::{CoreWall, core_wall, takes_level};

/// A box: min x, min y, max x, max y.
type Box4 = [f64; 4];

/// How far apart two walls' reach boxes must be for them to be in different pieces, metres.
pub const GAP: f64 = 1.0;

fn grow(b: Box4, by: f64) -> Box4 {
    [b[0] - by, b[1] - by, b[2] + by, b[3] + by]
}

fn meets(a: &Box4, b: &Box4) -> bool {
    a[0] <= b[2] && b[0] <= a[2] && a[1] <= b[3] && b[1] <= a[3]
}

fn join(a: Box4, b: Box4) -> Box4 {
    [a[0].min(b[0]), a[1].min(b[1]), a[2].max(b[2]), a[3].max(b[3])]
}

/// A wall's reach: its points, each arc's bulge out of its chord, and its width and offset either side.
pub fn reach(e: &Entity) -> Option<Box4> {
    let Entity::Wall(w) = e else { return None };
    let w = &w.wall;
    let mut b = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
    let n = w.pts.len();
    for (i, p) in w.pts.iter().enumerate() {
        b = join(b, [p.x, p.y, p.x, p.y]);
        let bulge = w.bulges.as_ref().and_then(|bs| bs.get(i)).copied().unwrap_or(0.0);
        if bulge != 0.0 && (i + 1 < n || w.closed) {
            let q = &w.pts[(i + 1) % n];
            // The arc lies within its chord's box grown by its sagitta (|bulge| half chords), and that by the radius
            // when it bulges past a half circle.
            let half = ((q.x - p.x).hypot(q.y - p.y)) / 2.0;
            let sagitta = bulge.abs() * half;
            let radius = half * (1.0 + bulge * bulge) / (2.0 * bulge.abs());
            let out = sagitta.max(radius) + 1e-9;
            b = join(b, grow([p.x.min(q.x), p.y.min(q.y), p.x.max(q.x), p.y.max(q.y)], out));
        }
    }
    let r = w.width.abs() + w.offset.abs();
    b[0].is_finite().then(|| grow(b, r + 1e-9))
}

fn find(parent: &mut [usize], mut i: usize) -> usize {
    while parent[i] != i {
        parent[i] = parent[parent[i]];
        i = parent[i];
    }
    i
}

/// The level's walls (as `takes_level` reads it) the face round `seed` can read, in the drawing's order; none when
/// every wall is needed (then the caller reads them all).
pub fn walls_round(doc: &Document, seed: kentos_contracts::Vec2, level: Option<&str>) -> Option<Vec<CoreWall>> {
    let walls: Vec<(&Entity, Box4)> = doc
        .entities()
        .filter(|e| takes_level(e, level))
        .filter_map(|e| reach(e).map(|b| (e, b)))
        .collect();
    let n = walls.len();
    if n < 2 {
        return None;
    }
    // Pieces: a sweep along x over the boxes grown by half the gap.
    let mut parent: Vec<usize> = (0..n).collect();
    let mut order: Vec<usize> = (0..n).collect();
    let boxes: Vec<Box4> = walls.iter().map(|(_, b)| grow(*b, GAP / 2.0)).collect();
    order.sort_by(|&a, &b| boxes[a][0].total_cmp(&boxes[b][0]));
    let mut open: Vec<usize> = Vec::new();
    for &i in &order {
        open.retain(|&j| boxes[j][2] >= boxes[i][0]);
        for &j in &open {
            if meets(&boxes[i], &boxes[j]) {
                let (a, b) = (find(&mut parent, i), find(&mut parent, j));
                if a != b {
                    parent[a] = b;
                }
            }
        }
        open.push(i);
    }
    let mut piece_box: std::collections::HashMap<usize, Box4> = std::collections::HashMap::new();
    let roots: Vec<usize> = (0..n).map(|i| find(&mut parent, i)).collect();
    for i in 0..n {
        let b = boxes[i];
        piece_box.entry(roots[i]).and_modify(|x| *x = join(*x, b)).or_insert(b);
    }
    // The pieces round the seed, and every piece their box meets, until none more; and the first wall's.
    let at = [seed.x, seed.y, seed.x, seed.y];
    let mut taken: std::collections::BTreeSet<usize> =
        piece_box.iter().filter(|(_, b)| meets(b, &at)).map(|(r, _)| *r).collect();
    if taken.is_empty() {
        return None;
    }
    loop {
        let reach = taken.iter().map(|r| piece_box[r]).reduce(join).unwrap_or(at);
        let more: Vec<usize> = piece_box
            .iter()
            .filter(|(r, b)| !taken.contains(r) && meets(b, &reach))
            .map(|(r, _)| *r)
            .collect();
        if more.is_empty() {
            break;
        }
        taken.extend(more);
    }
    taken.insert(roots[0]);
    if taken.len() == piece_box.len() {
        return None;
    }
    Some(
        walls
            .iter()
            .zip(&roots)
            .filter(|(_, r)| taken.contains(r))
            .filter_map(|((e, _), _)| core_wall(e))
            .collect(),
    )
}

#[cfg(test)]
mod wall_pieces_tests {
    use super::*;

    #[test]
    fn an_arc_reaches_past_its_points() {
        let e: Entity = serde_json::from_value(serde_json::json!({
            "kind": "wall", "id": 1, "layerId": "0", "attrs": {},
            "pts": [{ "x": 0.0, "y": 0.0 }, { "x": 2.0, "y": 0.0 }], "bulges": [1.0, 0.0],
            "width": 0.2, "justify": "center"
        }))
        .expect("a wall");
        let b = reach(&e).expect("a box");
        // A half circle of radius 1 below or above the chord, the width round it.
        assert!(b[1] <= -1.2 && b[3] >= 1.2, "{b:?}");
    }
}
