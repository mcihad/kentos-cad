//! A haul road network (docs/adr/0955, plan/road.md Y49; ROAD-HAUL-06): the roads a borrow or waste site off the road
//! is reached by, and whether they truly join. Two roads join only where an end of one lies within the connection
//! tolerance of the other, at its end or along it (a T, the other cut there) — as Trimble Business Center joins haul
//! roads; two lines that only cross on the screen do not join, and the crossing is told. A road joins the alignment
//! where one of its ends lies within the reach of the centre line, at the station of its foot; a road crossing the
//! centre line along it does not. A site joins the nearest road within the connection tolerance, cut there.
//!
//! A site's access is the shortest way along the joined roads from the site to the alignment (Dijkstra; the lengths
//! in plan, as the stations are), and where two ways are as short, the one meeting the alignment at the smaller
//! station. Ends that join nothing are told with how far the nearest road or the alignment lies, so a gap just over
//! the tolerance can be closed.

use crate::alignment::Alignment;

/// The most points a road is flattened to.
pub const MAX_POINTS: usize = 200_000;

/// The sag of the chords an arc is flattened to (m): a road's length is short of the arc's by at most L·s/(3R).
pub const SAG: f64 = 1e-5;

/// A road: its object's id and part, and its points in plan (arcs already flattened).
#[derive(Clone, Debug, PartialEq)]
pub struct Road {
    pub id: u64,
    pub part: u32,
    pub points: Vec<[f64; 2]>,
}

/// The points of a path with bulges (a bulge per edge, tan of a quarter of its sweep; closed: one more edge back to
/// the first), arcs flattened to chords of sag at most [`SAG`].
pub fn flatten(points: &[[f64; 2]], bulges: &[f64], closed: bool) -> Vec<[f64; 2]> {
    let n = points.len();
    if n == 0 {
        return Vec::new();
    }
    let edges = if closed { n } else { n - 1 };
    let mut out = vec![points[0]];
    for i in 0..edges {
        let (a, b) = (points[i], points[(i + 1) % n]);
        let bulge = bulges.get(i).copied().unwrap_or(0.0);
        let chord = (b[0] - a[0]).hypot(b[1] - a[1]);
        if bulge.abs() > 1e-12 && chord > 0.0 {
            let sweep = 4.0 * bulge.atan();
            let r = chord / (2.0 * (0.5 * sweep).sin().abs());
            // The centre: from the chord's middle, along its left normal by r·cos(sweep/2) (signed with the sweep).
            let (mx, my) = (0.5 * (a[0] + b[0]), 0.5 * (a[1] + b[1]));
            let (ux, uy) = ((b[0] - a[0]) / chord, (b[1] - a[1]) / chord);
            let h = r * (0.5 * sweep).cos() * sweep.signum();
            let c = [mx - uy * h, my + ux * h];
            let start = (a[1] - c[1]).atan2(a[0] - c[0]);
            // Chords of sag s span 2·acos(1 − s/r).
            let step = 2.0 * (1.0 - (SAG / r).min(1.0)).acos();
            let pieces = ((sweep.abs() / step.max(1e-9)).ceil() as usize).clamp(1, MAX_POINTS);
            for k in 1..pieces {
                let t = start + sweep * k as f64 / pieces as f64;
                out.push([c[0] + r * t.cos(), c[1] + r * t.sin()]);
            }
        }
        out.push(b);
    }
    out
}

/// Something about the network a user should see.
#[derive(Clone, Debug, PartialEq)]
pub enum Problem {
    /// Two roads cross along their length with no join there (`at`): they do not join.
    Crossing { a: usize, b: usize, at: [f64; 2] },
    /// A road crosses the alignment along its length (not at an end): it does not join it there.
    CrossesAlignment { road: usize, station: f64, at: [f64; 2] },
    /// An end that joins nothing: no road, no alignment, no site; `gap` how far the nearest road or the alignment
    /// lies (m), if any.
    Loose { road: usize, end: bool, at: [f64; 2], gap: Option<f64> },
}

/// How a site meets the network.
#[derive(Clone, Debug, PartialEq)]
pub enum Connection {
    /// It reaches the alignment: its access length (m), the station and offset where it joins, and the roads on the
    /// way, from the site.
    Joined { length: f64, station: f64, offset: f64, roads: Vec<usize> },
    /// No road lies within the tolerance; the nearest's distance, if any road.
    NoRoad { nearest: Option<f64> },
    /// It meets a road, but the roads it joins never reach the alignment.
    Cut { road: usize },
}

/// Where a road meets the alignment: its end (false: the start), the station and the offset.
#[derive(Clone, Debug, PartialEq)]
pub struct Junction {
    pub road: usize,
    pub end: bool,
    pub station: f64,
    pub offset: f64,
}

/// The network as read.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Network {
    /// Each road's length (m).
    pub lengths: Vec<f64>,
    /// Where the roads meet the alignment.
    pub junctions: Vec<Junction>,
    /// The joins between roads: the end of `road` (true: its end) and the other road, and whether at its end.
    pub joins: Vec<(usize, bool, usize, bool)>,
    pub problems: Vec<Problem>,
    /// Each site's connection, in the order given.
    pub sites: Vec<Connection>,
}

fn dist(a: [f64; 2], b: [f64; 2]) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

/// The nearest point of a polyline to `p`: its distance, the length along it and the point.
fn foot(points: &[[f64; 2]], along: &[f64], p: [f64; 2]) -> (f64, f64, [f64; 2]) {
    let mut best = (f64::INFINITY, 0.0, points[0]);
    for i in 0..points.len().saturating_sub(1) {
        let (a, b) = (points[i], points[i + 1]);
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let l2 = dx * dx + dy * dy;
        let t = if l2 > 0.0 { (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2).clamp(0.0, 1.0) } else { 0.0 };
        let q = [a[0] + t * dx, a[1] + t * dy];
        let d = dist(p, q);
        if d < best.0 {
            best = (d, along[i] + t * (along[i + 1] - along[i]), q);
        }
    }
    best
}

/// Where segments `a0 a1` and `b0 b1` cross properly (inside both), if they do.
fn crossing(a0: [f64; 2], a1: [f64; 2], b0: [f64; 2], b1: [f64; 2]) -> Option<([f64; 2], f64, f64)> {
    let (rx, ry) = (a1[0] - a0[0], a1[1] - a0[1]);
    let (sx, sy) = (b1[0] - b0[0], b1[1] - b0[1]);
    let den = rx * sy - ry * sx;
    if den.abs() <= 1e-15 * (rx.hypot(ry) * sx.hypot(sy)).max(1e-300) {
        return None;
    }
    let (qx, qy) = (b0[0] - a0[0], b0[1] - a0[1]);
    let t = (qx * sy - qy * sx) / den;
    let u = (qx * ry - qy * rx) / den;
    ((0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u)).then(|| ([a0[0] + t * rx, a0[1] + t * ry], t, u))
}

struct Union(Vec<usize>);

impl Union {
    fn find(&mut self, x: usize) -> usize {
        let mut r = x;
        while self.0[r] != r {
            r = self.0[r];
        }
        let mut y = x;
        while self.0[y] != r {
            let next = self.0[y];
            self.0[y] = r;
            y = next;
        }
        r
    }
    fn join(&mut self, a: usize, b: usize) {
        let (a, b) = (self.find(a), self.find(b));
        if a != b {
            self.0[a.max(b)] = a.min(b);
        }
    }
}

/// The network of `roads` against `alignment` (its centre line `shift` m to the right of the geometry's, offsets right
/// positive, for an offset alignment), and how each of `sites` (points in plan) reaches it: ends join within `connect` (m), a road end joins
/// the alignment within `reach` (m) of its centre line.
pub fn network(alignment: &Alignment, shift: f64, roads: &[Road], sites: &[[f64; 2]], connect: f64, reach: f64) -> Network {
    let along: Vec<Vec<f64>> = roads
        .iter()
        .map(|r| {
            let mut acc = vec![0.0];
            for w in r.points.windows(2) {
                acc.push(acc.last().copied().unwrap_or(0.0) + dist(w[0], w[1]));
            }
            acc
        })
        .collect();
    let lengths: Vec<f64> = along.iter().map(|a| a.last().copied().unwrap_or(0.0)).collect();
    let mut out = Network { lengths: lengths.clone(), ..Network::default() };
    let usable: Vec<bool> = roads.iter().zip(&lengths).map(|(r, l)| r.points.len() >= 2 && *l > 0.0).collect();
    let end_at = |r: usize, end: bool| if end { *roads[r].points.last().unwrap_or(&[0.0, 0.0]) } else { roads[r].points[0] };
    // Nodes: each road's two ends (2r, 2r + 1), then the cuts along roads.
    let mut cuts: Vec<Vec<(f64, usize)>> = vec![Vec::new(); roads.len()];
    let mut nodes = 2 * roads.len();
    let mut joined_end = vec![false; 2 * roads.len()];
    let mut links: Vec<(usize, usize)> = Vec::new();
    for r in 0..roads.len() {
        if usable[r] {
            cuts[r].push((0.0, 2 * r));
            cuts[r].push((lengths[r], 2 * r + 1));
        }
    }
    // Ends against the other roads: an end near another's end merges with it; one near its length cuts it there.
    for r in (0..roads.len()).filter(|r| usable[*r]) {
        for end in [false, true] {
            let p = end_at(r, end);
            let me = 2 * r + usize::from(end);
            // The same road's other end: a closed road.
            if dist(p, end_at(r, !end)) <= connect && end && lengths[r] > connect {
                links.push((me, 2 * r));
                joined_end[me] = true;
                joined_end[2 * r] = true;
            }
            for o in (0..roads.len()).filter(|o| *o != r && usable[*o]) {
                let near_end = [false, true].into_iter().filter(|e| dist(p, end_at(o, *e)) <= connect).min_by(|a, b| {
                    dist(p, end_at(o, *a)).total_cmp(&dist(p, end_at(o, *b)))
                });
                if let Some(e) = near_end {
                    links.push((me, 2 * o + usize::from(e)));
                    joined_end[me] = true;
                    if r < o {
                        out.joins.push((r, end, o, true));
                    }
                    continue;
                }
                let (d, s, _) = foot(&roads[o].points, &along[o], p);
                if d <= connect {
                    let k = nodes;
                    nodes += 1;
                    cuts[o].push((s, k));
                    links.push((me, k));
                    joined_end[me] = true;
                    out.joins.push((r, end, o, false));
                }
            }
        }
    }
    // The alignment: a road end within reach of its centre line joins it at its foot's station.
    let mut junction_of: Vec<Option<usize>> = vec![None; 2 * roads.len()];
    for r in (0..roads.len()).filter(|r| usable[*r]) {
        for end in [false, true] {
            let found = alignment.locate(end_at(r, end));
            if let Some(f) = found.nearest
                && (f.offset - shift).abs() <= reach
            {
                junction_of[2 * r + usize::from(end)] = Some(out.junctions.len());
                out.junctions.push(Junction { road: r, end, station: f.at.station, offset: f.offset - shift });
            }
        }
    }
    // The sites: each on its nearest road within the tolerance, cut there.
    let mut site_node: Vec<Option<(usize, usize)>> = Vec::with_capacity(sites.len());
    let mut site_conn: Vec<Option<Connection>> = Vec::with_capacity(sites.len());
    for p in sites {
        let nearest = (0..roads.len())
            .filter(|r| usable[*r])
            .map(|r| (r, foot(&roads[r].points, &along[r], *p)))
            .min_by(|a, b| a.1.0.total_cmp(&b.1.0));
        match nearest {
            Some((r, (d, s, _))) if d <= connect => {
                let k = nodes;
                nodes += 1;
                cuts[r].push((s, k));
                site_node.push(Some((k, r)));
                site_conn.push(None);
            }
            other => {
                site_node.push(None);
                site_conn.push(Some(Connection::NoRoad { nearest: other.map(|o| o.1.0) }));
            }
        }
    }
    let mut union = Union((0..nodes).collect());
    for (a, b) in &links {
        union.join(*a, *b);
    }
    // The graph: each road's pieces between its cuts.
    let mut adjacency: Vec<Vec<(usize, f64, usize)>> = vec![Vec::new(); nodes];
    for r in (0..roads.len()).filter(|r| usable[*r]) {
        let mut c = cuts[r].clone();
        c.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        for w in c.windows(2) {
            let (a, b) = (union.find(w[0].1), union.find(w[1].1));
            let l = w[1].0 - w[0].0;
            adjacency[a].push((b, l, r));
            adjacency[b].push((a, l, r));
        }
    }
    let target: Vec<Option<usize>> = {
        let mut t = vec![None; nodes];
        for (k, j) in junction_of.iter().enumerate() {
            if let Some(j) = j {
                let root = union.find(k);
                let better = t[root].is_none_or(|old: usize| out.junctions[*j].station < out.junctions[old].station);
                if better {
                    t[root] = Some(*j);
                }
            }
        }
        t
    };
    for (i, sn) in site_node.iter().enumerate() {
        let Some((k, road)) = *sn else {
            out.sites.push(site_conn[i].take().unwrap_or(Connection::NoRoad { nearest: None }));
            continue;
        };
        let start = union.find(k);
        // Dijkstra over the few nodes: the nearest unsettled by a scan.
        let mut d = vec![f64::INFINITY; nodes];
        let mut via: Vec<Option<(usize, usize)>> = vec![None; nodes];
        let mut done = vec![false; nodes];
        d[start] = 0.0;
        while let Some(u) = (0..nodes).filter(|u| !done[*u] && d[*u].is_finite()).min_by(|a, b| d[*a].total_cmp(&d[*b])) {
            done[u] = true;
            for &(v, l, r) in &adjacency[u] {
                if d[u] + l < d[v] {
                    d[v] = d[u] + l;
                    via[v] = Some((u, r));
                }
            }
        }
        let best = (0..nodes)
            .filter_map(|u| target[u].map(|j| (u, j)))
            .filter(|(u, _)| d[*u].is_finite())
            .min_by(|a, b| {
                let (la, lb) = (d[a.0], d[b.0]);
                let same = (la - lb).abs() <= 1e-9 * la.max(lb).max(1.0);
                if same { out.junctions[a.1].station.total_cmp(&out.junctions[b.1].station) } else { la.total_cmp(&lb) }
            });
        match best {
            Some((u, j)) => {
                let mut on = Vec::new();
                let mut v = u;
                while let Some((w, r)) = via[v] {
                    if on.last() != Some(&r) {
                        on.push(r);
                    }
                    v = w;
                }
                if on.is_empty() {
                    on.push(road);
                }
                on.reverse();
                let jn = &out.junctions[j];
                out.sites.push(Connection::Joined { length: d[u], station: jn.station, offset: jn.offset, roads: on });
            }
            None => out.sites.push(Connection::Cut { road }),
        }
    }
    // Crossings along the roads' lengths with no join near: they do not join.
    let joined_near = |a: usize, b: usize, x: [f64; 2], union: &mut Union| -> bool {
        // A node both roads pass through within the tolerance of the crossing.
        let on_a: Vec<(usize, [f64; 2])> = cuts[a].iter().map(|(s, k)| (union.find(*k), point_at(&roads[a].points, &along[a], *s))).collect();
        let on_b: Vec<usize> = cuts[b].iter().map(|(_, k)| union.find(*k)).collect();
        on_a.iter().any(|(k, p)| on_b.contains(k) && dist(*p, x) <= connect)
    };
    for a in (0..roads.len()).filter(|r| usable[*r]) {
        for b in (a + 1..roads.len()).filter(|r| usable[*r]) {
            let mut seen: Vec<[f64; 2]> = Vec::new();
            for i in 0..roads[a].points.len() - 1 {
                for j in 0..roads[b].points.len() - 1 {
                    let pa = &roads[a].points;
                    let pb = &roads[b].points;
                    if let Some((x, _, _)) = crossing(pa[i], pa[i + 1], pb[j], pb[j + 1])
                        && !seen.iter().any(|y| dist(*y, x) <= connect)
                    {
                        seen.push(x);
                        if !joined_near(a, b, x, &mut union) {
                            out.problems.push(Problem::Crossing { a, b, at: x });
                        }
                    }
                }
            }
        }
    }
    // The alignment crossed along a road's length.
    for r in (0..roads.len()).filter(|r| usable[*r]) {
        let pts = &roads[r].points;
        let mut seen: Vec<[f64; 2]> = Vec::new();
        for i in 0..pts.len() - 1 {
            let side = |p: [f64; 2]| alignment.locate(p).nearest.map(|f| f.offset - shift);
            let (Some(o0), Some(o1)) = (side(pts[i]), side(pts[i + 1])) else { continue };
            if (o0 > 0.0) == (o1 > 0.0) || o0 == 0.0 {
                continue;
            }
            // Bisect the sign change along the segment.
            let (mut lo, mut hi) = (0.0, 1.0);
            let at = |t: f64| [pts[i][0] + t * (pts[i + 1][0] - pts[i][0]), pts[i][1] + t * (pts[i + 1][1] - pts[i][1])];
            for _ in 0..60 {
                let m = 0.5 * (lo + hi);
                match side(at(m)) {
                    Some(o) if (o > 0.0) == (o0 > 0.0) => lo = m,
                    _ => hi = m,
                }
            }
            let x = at(0.5 * (lo + hi));
            let ends_here = [false, true].into_iter().any(|e| dist(end_at(r, e), x) <= reach);
            if ends_here || seen.iter().any(|y| dist(*y, x) <= connect) {
                continue;
            }
            seen.push(x);
            if let Some(f) = alignment.locate(x).nearest {
                out.problems.push(Problem::CrossesAlignment { road: r, station: f.at.station, at: x });
            }
        }
    }
    // Loose ends: joined to no road, not at the alignment, no site there.
    let site_points: Vec<[f64; 2]> = sites.to_vec();
    for r in (0..roads.len()).filter(|r| usable[*r]) {
        for end in [false, true] {
            let k = 2 * r + usize::from(end);
            let p = end_at(r, end);
            if joined_end[k] || junction_of[k].is_some() || site_points.iter().any(|s| dist(*s, p) <= connect) {
                continue;
            }
            // Something else joined to this end (another's end merged into it) makes it no loose end.
            let root = union.find(k);
            let others = (0..2 * roads.len()).filter(|x| *x != k && usable[*x / 2]).any(|x| union.find(x) == root)
                || (2 * roads.len()..nodes).any(|x| union.find(x) == root);
            if others {
                continue;
            }
            let to_road = (0..roads.len())
                .filter(|o| *o != r && usable[*o])
                .map(|o| foot(&roads[o].points, &along[o], p).0)
                .fold(f64::INFINITY, f64::min);
            let to_axis = alignment.locate(p).nearest.map_or(f64::INFINITY, |f| (f.offset - shift).abs());
            let gap = to_road.min(to_axis);
            out.problems.push(Problem::Loose { road: r, end, at: p, gap: gap.is_finite().then_some(gap) });
        }
    }
    out
}

/// The point at length `s` along a polyline.
fn point_at(points: &[[f64; 2]], along: &[f64], s: f64) -> [f64; 2] {
    let i = along.partition_point(|x| *x < s).clamp(1, points.len() - 1);
    let (a, b) = (points[i - 1], points[i]);
    let l = along[i] - along[i - 1];
    let t = if l > 0.0 { ((s - along[i - 1]) / l).clamp(0.0, 1.0) } else { 0.0 };
    [a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_quarter_circle_flattens_to_its_length() {
        // A bulge of tan(π/8) is a quarter circle; radius 10 from (10, 0) to (0, 10) about the origin.
        let pts = flatten(&[[10.0, 0.0], [0.0, 10.0]], &[(std::f64::consts::PI / 8.0).tan()], false);
        let length: f64 = pts.windows(2).map(|w| dist(w[0], w[1])).sum();
        let exact = 10.0 * std::f64::consts::FRAC_PI_2;
        assert!(exact - length >= 0.0 && exact - length <= exact * SAG / (3.0 * 10.0) * 1.01, "{length} vs {exact}");
        assert!(pts.iter().all(|p| (p[0].hypot(p[1]) - 10.0).abs() < 1e-9));
    }

    #[test]
    fn crossing_is_found_inside_both() {
        assert!(crossing([0.0, 0.0], [2.0, 2.0], [0.0, 2.0], [2.0, 0.0]).is_some_and(|c| dist(c.0, [1.0, 1.0]) < 1e-12));
        assert!(crossing([0.0, 0.0], [1.0, 1.0], [3.0, 0.0], [3.0, 5.0]).is_none());
    }
}
