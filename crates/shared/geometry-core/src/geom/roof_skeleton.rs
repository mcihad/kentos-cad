//! A roof's faces over any simple eaves line by its straight skeleton (docs/adr/8071 §10; 8044 §10): as the roof rises,
//! each edge's line moves into the roof, a sloping edge's `1 / slope` metres a metre of rise (its plane's level line), a
//! gable end's not at all; where the moving lines meet, the roof's faces meet. Its corners move where their two edges'
//! lines meet, and these events change the line, earliest first:
//!
//! - an edge shrinks to nothing: its two corners meet and become one;
//! - a corner turning in reaches an edge across the roof: the line splits in two there;
//! - a loop's last two corners meet, or the two edges between them lie back to back: a ridge closes it.
//!
//! Each sloping edge's face is then its edge and the tracks of the corners that bounded it. A kinetic simulation by
//! brute force: after each event every candidate is worked out again (cubic in the corners, a roof's few dozen), so a
//! split is taken only where the corner meets the edge as it then is (an event-queue algorithm's known fault is to test
//! the edge's first line). The positions of an event are worked out once and shared by every face it bounds.

use crate::Vec2;
use kentos_num::hypot;

/// Why the skeleton cannot be had.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Fail {
    /// Two edges in one line move apart at different speeds (a gable end beside a sloping edge in its line, or two
    /// slopes): the roof would step there.
    Collinear,
    /// No event left and corners still moving: never for a simple line.
    Stuck,
}

/// An edge of the line as it moves: its first line `n·x = c`, moving `speed` along `n` (into the roof) a unit of time.
#[derive(Clone, Copy)]
struct Edge {
    d: Vec2,
    n: Vec2,
    c: f64,
    speed: f64,
}

impl Edge {
    fn offset(&self, t: f64) -> f64 {
        self.c + self.speed * t
    }
}

/// A corner of the moving line: where it started (`node`), where it was at `t0` and its velocity; its two edges, and
/// its neighbours along its loop. `held`: its two edges lie back to back (no velocity), which only an event at the same
/// instant may resolve.
#[derive(Clone, Copy)]
struct Corner {
    node: usize,
    p: Vec2,
    t0: f64,
    w: Vec2,
    held: bool,
    inn: usize,
    out: usize,
    prev: usize,
    next: usize,
    live: bool,
}

impl Corner {
    fn at(&self, t: f64) -> Vec2 {
        Vec2::new(
            self.p.x + (t - self.t0) * self.w.x,
            self.p.y + (t - self.t0) * self.w.y,
        )
    }
}

/// The velocity of a corner between edges `a` (into it) and `b` (out of it): on both moving lines. None for two edges
/// back to back, or in one line moving at different speeds.
fn velocity(a: &Edge, b: &Edge) -> Option<Vec2> {
    let det = a.n.x * b.n.y - a.n.y * b.n.x;
    if det.abs() > 1e-12 {
        return Some(Vec2::new(
            (a.speed * b.n.y - a.n.y * b.speed) / det,
            (a.n.x * b.speed - a.speed * b.n.x) / det,
        ));
    }
    let same = a.n.x * b.n.x + a.n.y * b.n.y > 0.0;
    (same
        && (a.speed - b.speed).abs()
            <= 1e-12 * kentos_num::maximum(kentos_num::maximum(a.speed.abs(), b.speed.abs()), 1.0))
    .then(|| Vec2::new(a.n.x * a.speed, a.n.y * a.speed))
}

fn dot(a: Vec2, b: Vec2) -> f64 {
    a.x * b.x + a.y * b.y
}

fn sub(a: Vec2, b: Vec2) -> Vec2 {
    Vec2::new(a.x - b.x, a.y - b.y)
}

/// What happens next.
#[derive(Clone, Copy)]
enum Event {
    /// The edge out of this corner shrinks to nothing.
    Collapse(usize),
    /// A corner (the first) reaches the edge out of another (the second), at a point.
    Split(usize, usize, Vec2),
}

/// A roof face's points, each with its rise.
pub(crate) type Face = Vec<(Vec2, f64)>;

/// The faces of a counter-clockwise simple `ring` whose edge `k` (from `ring[k]` to the next point) moves `speeds[k]`
/// into the roof a unit of rise (0: a gable end): each edge's face counter-clockwise from its own edge, its points with
/// their rise; none for a gable end.
pub(crate) fn faces(ring: &[Vec2], speeds: &[f64]) -> Result<Vec<Option<Face>>, Fail> {
    let n = ring.len();
    let size = kentos_num::maximum(
        ring.iter().fold(0.0_f64, |m, q| {
            kentos_num::maximum(kentos_num::maximum(m, q.x.abs()), q.y.abs())
        }),
        1.0,
    );
    // Lengths below this are a point; times below this an instant (a unit of time moves a corner about a unit).
    let tol = 1e-9 * size;
    let edges: Vec<Edge> = (0..n)
        .map(|k| {
            let (a, b) = (ring[k], ring[(k + 1) % n]);
            let l = hypot(b.x - a.x, b.y - a.y);
            let d = Vec2::new((b.x - a.x) / l, (b.y - a.y) / l);
            let nn = Vec2::new(-d.y, d.x);
            Edge {
                d,
                n: nn,
                c: dot(nn, a),
                speed: speeds[k],
            }
        })
        .collect();
    let mut nodes: Vec<(Vec2, f64)> = ring.iter().map(|q| (*q, 0.0)).collect();
    // A corner's track or a ridge a loop closes with: from a node to a node, between the faces of two edges.
    let mut arcs: Vec<(usize, usize, usize, usize)> = Vec::new();
    let mut corners: Vec<Corner> = Vec::with_capacity(4 * n);
    for i in 0..n {
        let (inn, out) = ((i + n - 1) % n, i);
        let w = velocity(&edges[inn], &edges[out]).ok_or(Fail::Collinear)?;
        corners.push(Corner {
            node: i,
            p: ring[i],
            t0: 0.0,
            w,
            held: false,
            inn,
            out,
            prev: (i + n - 1) % n,
            next: (i + 1) % n,
            live: true,
        });
    }
    let corner =
        |node: usize, p: Vec2, t: f64, inn: usize, out: usize, prev: usize, next: usize| {
            let w = velocity(&edges[inn], &edges[out]);
            Corner {
                node,
                p,
                t0: t,
                w: w.unwrap_or(Vec2::new(0.0, 0.0)),
                held: w.is_none(),
                inn,
                out,
                prev,
                next,
                live: true,
            }
        };
    let mut t = 0.0_f64;
    for _ in 0..8 * n * n + 64 {
        // Close every loop with no area left (two corners, or edges lying back to back): its corners' tracks end where
        // they are, its edges become the ridges between them.
        let mut seen = vec![false; corners.len()];
        for c in 0..corners.len() {
            if !corners[c].live || seen[c] {
                continue;
            }
            let mut members = vec![c];
            seen[c] = true;
            let mut k = corners[c].next;
            while k != c && !seen[k] {
                members.push(k);
                seen[k] = true;
                k = corners[k].next;
            }
            let pts: Vec<Vec2> = members.iter().map(|&k| corners[k].at(t)).collect();
            let m = pts.len();
            let twice: f64 = (0..m)
                .map(|i| pts[i].x * pts[(i + 1) % m].y - pts[(i + 1) % m].x * pts[i].y)
                .sum();
            if twice.abs() / 2.0 > tol * size {
                continue;
            }
            let mut made: Vec<usize> = Vec::new();
            let mut node_of: Vec<usize> = Vec::with_capacity(m);
            for q in &pts {
                let at = made
                    .iter()
                    .copied()
                    .find(|&nd| hypot(nodes[nd].0.x - q.x, nodes[nd].0.y - q.y) <= tol);
                node_of.push(at.unwrap_or_else(|| {
                    nodes.push((*q, t));
                    made.push(nodes.len() - 1);
                    nodes.len() - 1
                }));
            }
            for (i, &k) in members.iter().enumerate() {
                let cr = corners[k];
                arcs.push((cr.node, node_of[i], cr.inn, cr.out));
                let j = (i + 1) % m;
                if node_of[i] != node_of[j] {
                    arcs.push((node_of[i], node_of[j], cr.out, cr.out));
                }
                corners[k].live = false;
            }
        }
        let live: Vec<usize> = (0..corners.len()).filter(|&c| corners[c].live).collect();
        if live.is_empty() {
            return Ok(chain(&nodes, &arcs, n, speeds));
        }
        let mut best: Option<(f64, Event)> = None;
        let mut take = |at: f64, e: Event| {
            if best.is_none_or(|(b, _)| at < b - tol) {
                best = Some((at, e));
            }
        };
        for &u in &live {
            let (cu, cv) = (corners[u], corners[corners[u].next]);
            let e = &edges[cu.out];
            let length = dot(e.d, sub(cv.at(t), cu.at(t)));
            let rate = dot(e.d, sub(cv.w, cu.w));
            if length <= tol {
                take(t, Event::Collapse(u));
            } else if rate < -1e-15 {
                take(t + length / -rate, Event::Collapse(u));
            }
        }
        for &r in &live {
            let cr = corners[r];
            let (din, dout) = (edges[cr.inn].d, edges[cr.out].d);
            // Only a corner turning in (clockwise in a counter-clockwise loop) can reach an edge across.
            if cr.held || din.x * dout.y - din.y * dout.x >= -1e-12 {
                continue;
            }
            for &u in &live {
                if u == r || corners[u].next == r {
                    continue;
                }
                let (cu, cv) = (corners[u], corners[corners[u].next]);
                let e = &edges[cu.out];
                let gap = dot(e.n, cr.at(t)) - e.offset(t);
                let rate = dot(e.n, cr.w) - e.speed;
                if gap < -tol || rate >= -1e-15 {
                    continue;
                }
                let at = t + kentos_num::maximum(gap, 0.0) / -rate;
                let p = cr.at(at);
                let (a, b) = (cu.at(at), cv.at(at));
                let (s, len) = (dot(e.d, sub(p, a)), dot(e.d, sub(b, a)));
                if len > -tol && s >= -tol && s <= len + tol {
                    take(at, Event::Split(r, u, p));
                }
            }
        }
        let Some((at, event)) = best else {
            return Err(Fail::Stuck);
        };
        // A corner whose edges lie back to back must be gone before time moves on.
        if at > t + tol && live.iter().any(|&c| corners[c].held) {
            return Err(Fail::Collinear);
        }
        t = kentos_num::maximum(at, t);
        match event {
            Event::Collapse(u) => {
                let v = corners[u].next;
                let (cu, cv) = (corners[u], corners[v]);
                let (pu, pv) = (cu.at(t), cv.at(t));
                let p = Vec2::new((pu.x + pv.x) / 2.0, (pu.y + pv.y) / 2.0);
                nodes.push((p, t));
                let node = nodes.len() - 1;
                arcs.push((cu.node, node, cu.inn, cu.out));
                arcs.push((cv.node, node, cv.inn, cv.out));
                corners[u].live = false;
                corners[v].live = false;
                let (prev, next) = (cu.prev, cv.next);
                if prev == v {
                    // The loop was these two: closed at the point.
                    continue;
                }
                let x = corners.len();
                corners.push(corner(node, p, t, cu.inn, cv.out, prev, next));
                corners[prev].next = x;
                corners[next].prev = x;
            }
            Event::Split(r, u, p) => {
                let cr = corners[r];
                let b = corners[u].next;
                nodes.push((p, t));
                let node = nodes.len() - 1;
                arcs.push((cr.node, node, cr.inn, cr.out));
                corners[r].live = false;
                let e = corners[u].out;
                let (x1, x2) = (corners.len(), corners.len() + 1);
                corners.push(corner(node, p, t, cr.inn, e, cr.prev, b));
                corners.push(corner(node, p, t, e, cr.out, u, cr.next));
                corners[cr.prev].next = x1;
                corners[b].prev = x1;
                corners[u].next = x2;
                corners[cr.next].prev = x2;
            }
        }
    }
    Err(Fail::Stuck)
}

/// Each sloping edge's face from the tracks bounding it: from its end round to its start.
fn chain(
    nodes: &[(Vec2, f64)],
    arcs: &[(usize, usize, usize, usize)],
    n: usize,
    speeds: &[f64],
) -> Vec<Option<Vec<(Vec2, f64)>>> {
    (0..n)
        .map(|k| {
            if speeds[k] <= 0.0 {
                return None;
            }
            let mine: Vec<(usize, usize)> = arcs
                .iter()
                .filter(|(a, b, f, g)| a != b && (*f == k || *g == k))
                .map(|(a, b, ..)| (*a, *b))
                .collect();
            let (start, end) = (k, (k + 1) % n);
            let mut used = vec![false; mine.len()];
            let mut ring = vec![nodes[start], nodes[end]];
            let mut at = end;
            while at != start {
                let i =
                    (0..mine.len()).find(|&i| !used[i] && (mine[i].0 == at || mine[i].1 == at))?;
                used[i] = true;
                at = if mine[i].0 == at {
                    mine[i].1
                } else {
                    mine[i].0
                };
                if at != start {
                    ring.push(nodes[at]);
                }
            }
            // Points that came to one (a corner born and gone at one event).
            let mut clean: Vec<(Vec2, f64)> = Vec::with_capacity(ring.len());
            for q in ring {
                if clean
                    .last()
                    .is_none_or(|l: &(Vec2, f64)| hypot(l.0.x - q.0.x, l.0.y - q.0.y) > 1e-9)
                {
                    clean.push(q);
                }
            }
            Some(clean)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Vec2 {
        Vec2::new(x, y)
    }

    fn area(ring: &[(Vec2, f64)]) -> f64 {
        let n = ring.len();
        (0..n)
            .map(|i| ring[i].0.x * ring[(i + 1) % n].0.y - ring[(i + 1) % n].0.x * ring[i].0.y)
            .sum::<f64>()
            / 2.0
    }

    /// A 10 × 6 rectangle, every edge at the same speed: a hip roof, its ridge 3 high from (3, 3) to (7, 3).
    #[test]
    fn a_rectangle_is_a_hip_roof() {
        let ring = [p(0.0, 0.0), p(10.0, 0.0), p(10.0, 6.0), p(0.0, 6.0)];
        let f = faces(&ring, &[1.0; 4]).unwrap();
        let total: f64 = f.iter().flatten().map(|r| area(r)).sum();
        assert!((total - 60.0).abs() < 1e-9, "{total}");
        let south = f[0].as_ref().unwrap();
        assert_eq!(south.len(), 4, "{south:?}");
        assert!(south.iter().any(|(q, z)| (q.x - 3.0).abs() < 1e-9
            && (q.y - 3.0).abs() < 1e-9
            && (z - 3.0).abs() < 1e-9));
        assert!((area(south) - (10.0 + 4.0) / 2.0 * 3.0).abs() < 1e-9);
        assert!((area(f[1].as_ref().unwrap()) - 6.0 * 3.0 / 2.0).abs() < 1e-9);
    }

    /// The same rectangle with gable ends east and west: two faces, each half of it, the ridge the full length.
    #[test]
    fn gable_ends_stand_still() {
        let ring = [p(0.0, 0.0), p(10.0, 0.0), p(10.0, 6.0), p(0.0, 6.0)];
        let f = faces(&ring, &[1.0, 0.0, 1.0, 0.0]).unwrap();
        assert!(f[1].is_none() && f[3].is_none());
        for k in [0, 2] {
            let r = f[k].as_ref().unwrap();
            assert!((area(r) - 30.0).abs() < 1e-9, "{k}: {r:?}");
            assert!(
                r.iter()
                    .all(|(q, z)| (z - kentos_num::minimum(q.y, 6.0 - q.y)).abs() < 1e-9),
                "{k}: {r:?}"
            );
        }
    }

    /// An L (10 × 4 along the south, 4 wide up the west to 8), every edge at one speed: worked out by hand, its two
    /// arms' ridges 2 high at y 2 (x 2 to 8) and x 2 (y 2 to 6), a valley from the inner corner (4, 4) down to... up to
    /// (2, 2), where the west and south edges' hips meet them; each face its trapezoid or triangle, the area all 56.
    #[test]
    fn an_l_turns_in_with_a_valley() {
        let ring = [
            p(0.0, 0.0),
            p(10.0, 0.0),
            p(10.0, 4.0),
            p(4.0, 4.0),
            p(4.0, 8.0),
            p(0.0, 8.0),
        ];
        let f = faces(&ring, &[1.0; 6]).unwrap();
        let rings: Vec<&Vec<(Vec2, f64)>> = f.iter().flatten().collect();
        assert_eq!(rings.len(), 6);
        let total: f64 = rings.iter().map(|r| area(r)).sum();
        assert!((total - 56.0).abs() < 1e-9, "{total}");
        for r in &rings {
            assert!(area(r) > 0.0, "counter-clockwise: {r:?}");
        }
        // South: (0,0) (10,0) (8,2) (2,2): (10 + 6) / 2 · 2 = 16.
        assert!((area(rings[0]) - 16.0).abs() < 1e-9, "{:?}", rings[0]);
        // East: (10,0) (10,4) (8,2): 4.
        assert!((area(rings[1]) - 4.0).abs() < 1e-9, "{:?}", rings[1]);
        // The inner north edge of the south arm, (10,4) → (4,4): (10,4) (4,4) (2,2) (8,2): (6 + 6) / 2 · 2 = 12... its
        // valley side from (4, 4) to (2, 2).
        assert!((area(rings[2]) - 12.0).abs() < 1e-9, "{:?}", rings[2]);
        // The inner east edge of the west arm, (4,4) → (4,8): (4,4) (4,8) (2,6) (2,2): (4 + 4) / 2 · 2 = 8.
        assert!((area(rings[3]) - 8.0).abs() < 1e-9, "{:?}", rings[3]);
        // North: (4,8) (0,8) (2,6): 4. West: (0,8) (0,0) (2,2) (2,6): (8 + 4) / 2 · 2 = 12.
        assert!((area(rings[4]) - 4.0).abs() < 1e-9, "{:?}", rings[4]);
        assert!((area(rings[5]) - 12.0).abs() < 1e-9, "{:?}", rings[5]);
        let top = rings
            .iter()
            .flat_map(|r| r.iter())
            .map(|(_, z)| *z)
            .fold(0.0, kentos_num::maximum);
        assert!((top - 2.0).abs() < 1e-9);
    }

    /// Every point of every face on its edge's plane: its rise its distance from the edge's line over the edge's speed
    /// (`speed` metres a metre of rise), and the faces together the ring's area.
    fn planar(ring: &[Vec2], speeds: &[f64], f: &[Option<Vec<(Vec2, f64)>>], whole: f64) {
        let n = ring.len();
        let mut total = 0.0;
        for (k, face) in f.iter().enumerate() {
            let Some(face) = face else {
                assert!(speeds[k] == 0.0, "{k}: a sloping edge without a face");
                continue;
            };
            let (a, b) = (ring[k], ring[(k + 1) % n]);
            let l = hypot(b.x - a.x, b.y - a.y);
            for (q, z) in face {
                let dist = ((b.x - a.x) * (q.y - a.y) - (b.y - a.y) * (q.x - a.x)) / l;
                assert!(
                    (dist - z * speeds[k]).abs() < 1e-9,
                    "{k}: {q:?} at {z}, {dist} from its edge"
                );
            }
            assert!(area(face) > 1e-12, "{k}: counter-clockwise {face:?}");
            total += area(face);
        }
        assert!((total - whole).abs() < 1e-9, "{total} of {whole}");
    }

    /// An L whose arms differ (4 wide along the south, 6 up the west), worked out by hand: the inner corner's valley
    /// reaches the south edge at (4, 2) when the south arm closes (ridge (4, 2)–(8, 2), 2 high); the west arm goes on
    /// to 3 (ridge (3, 3)–(3, 5)). The south face (0,0) (10,0) (8,2) (4,2) (3,3): 17; the inner ones 8 and 11; the
    /// ends 4 and 9; the west 15.
    #[test]
    fn an_l_whose_arms_differ_closes_the_narrow_one_first() {
        let ring = [
            p(0.0, 0.0),
            p(10.0, 0.0),
            p(10.0, 4.0),
            p(6.0, 4.0),
            p(6.0, 8.0),
            p(0.0, 8.0),
        ];
        let f = faces(&ring, &[1.0; 6]).unwrap();
        planar(&ring, &[1.0; 6], &f, 64.0);
        let areas: Vec<f64> = f.iter().flatten().map(|r| area(r)).collect();
        for (got, want) in areas.iter().zip([17.0, 4.0, 8.0, 11.0, 9.0, 15.0]) {
            assert!((got - want).abs() < 1e-9, "{areas:?}");
        }
        let top = f
            .iter()
            .flatten()
            .flat_map(|r| r.iter())
            .map(|(_, z)| *z)
            .fold(0.0, kentos_num::maximum);
        assert!((top - 3.0).abs() < 1e-9);
    }

    /// A T (a 12 × 4 bar along the south, a 4 wide stem up the middle to 10): two valleys, every point on its plane,
    /// 2 high at most (both 4 wide).
    #[test]
    fn a_t_has_two_valleys() {
        let ring = [
            p(0.0, 0.0),
            p(12.0, 0.0),
            p(12.0, 4.0),
            p(8.0, 4.0),
            p(8.0, 10.0),
            p(4.0, 10.0),
            p(4.0, 4.0),
            p(0.0, 4.0),
        ];
        let f = faces(&ring, &[1.0; 8]).unwrap();
        planar(&ring, &[1.0; 8], &f, 72.0);
        let top = f
            .iter()
            .flatten()
            .flat_map(|r| r.iter())
            .map(|(_, z)| *z)
            .fold(0.0, kentos_num::maximum);
        assert!((top - 2.0).abs() < 1e-9);
    }

    /// An L of two slopes (the south arm's long sides 0.5 a metre of rise apart from the others' 1), gable ends at both
    /// arms' ends: every point on its own plane, nothing lost.
    #[test]
    fn slopes_and_gable_ends_on_an_l() {
        let ring = [
            p(0.0, 0.0),
            p(10.0, 0.0),
            p(10.0, 4.0),
            p(6.0, 4.0),
            p(6.0, 8.0),
            p(0.0, 8.0),
        ];
        let speeds = [2.0, 0.0, 2.0, 1.0, 0.0, 1.0];
        let f = faces(&ring, &speeds).unwrap();
        planar(&ring, &speeds, &f, 64.0);
    }

    /// A shed (one sloping edge, the others gable ends): one face, the whole roof, rising to the far side.
    #[test]
    fn a_shed_is_one_face() {
        let ring = [p(0.0, 0.0), p(10.0, 0.0), p(10.0, 6.0), p(0.0, 6.0)];
        let f = faces(&ring, &[1.0, 0.0, 0.0, 0.0]).unwrap();
        planar(&ring, &[1.0, 0.0, 0.0, 0.0], &f, 60.0);
        assert_eq!(f.iter().flatten().count(), 1);
    }
}
