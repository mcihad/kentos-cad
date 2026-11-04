//! Where a sample line set takes its sections, and where its lines cross (docs/adr/0933, plan/road.md Y29;
//! ROAD-SEC-13).
//!
//! **The terrain's breaks.** Along a tangent the existing ground is linear inside a triangle and breaks where the
//! alignment crosses a triangle's edge (a breakline's piece is one): [`ground_breaks`] takes those stations from the
//! terrain profile (docs/adr/0902 §1, exact). On a curve the ground between two breaks is not quite linear (the arc
//! bends inside the triangle's plane); its sagitta's height is read as the triangle's slope across the chord.
//!
//! **Fewer of them.** A section at every break is exact and, on a dense survey, a section every metre or two.
//! [`simplify`] keeps the breaks the Douglas–Peucker way on (station, height): the place farthest in height (not
//! square to the chord: the error that matters is the height read between two sections) from the chord of the
//! stretch is kept while it is more than the tolerance, so between two kept stations the linear ground is within the
//! tolerance of the exact profile everywhere. `most` adds the fewest stations to keep no gap longer.
//!
//! **Lines that cross.** Two square lines at stations s₁, s₂ meet where their normals meet: on a circle at its centre,
//! R from the alignment on the inside; on a clothoid near its evolute. A swath wider than that on the inside makes two
//! sections share ground (the end areas count it twice, reversed past the meeting). [`crossings`] finds every pair
//! meeting within both swaths; [`clip`] shortens the lines at their meetings, nearest first, the answer that keeps
//! the lines square to the alignment (a skewed line is another section, ROAD-SEC-05).

use crate::alignment::Alignment;
use crate::surface::Surface;

/// A place of the centreline's ground profile: its station and height.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Break {
    pub station: f64,
    pub z: f64,
}

/// The centreline's ground profile from `from` to `to`: [`crate::profile::terrain`]'s places where the alignment
/// crosses a kept triangle's edge (exact, docs/adr/0902 §1), and `from`, `to` with the surface's height where it has
/// one; places off the surface are left out.
pub fn ground_breaks(surface: &Surface, al: &Alignment, from: f64, to: f64) -> Vec<Break> {
    let mut out: Vec<Break> = crate::profile::terrain::terrain(surface, al)
        .into_iter()
        .filter(|p| p.crossing && p.station >= from - 1e-9 && p.station <= to + 1e-9)
        .filter_map(|p| p.z.map(|z| Break { station: p.station.clamp(from, to), z }))
        .collect();
    for s in [from, to] {
        if let Some(z) = al.at(s).and_then(|p| surface.height(p.x, p.y).z) {
            out.push(Break { station: s, z });
        }
    }
    out.sort_by(|p, q| p.station.total_cmp(&q.station));
    out.dedup_by(|q, p| (q.station - p.station).abs() <= 1e-9);
    out
}

/// The breaks kept so the linear ground between two kept stations is within `tolerance` (m, in height) of the
/// profile everywhere, the first and last always; `most` (m): a gap longer gets the breaks in it that bring it under
/// `most` where the profile has them. Indexes into `profile`, in order.
pub fn simplify(profile: &[Break], tolerance: f64, most: Option<f64>) -> Vec<usize> {
    let n = profile.len();
    if n <= 2 {
        return (0..n).collect();
    }
    let mut keep = vec![false; n];
    keep[0] = true;
    keep[n - 1] = true;
    let mut stack = vec![(0usize, n - 1)];
    while let Some((i, j)) = stack.pop() {
        if j <= i + 1 {
            continue;
        }
        let (p, q) = (profile[i], profile[j]);
        let span = q.station - p.station;
        let mut worst = (0.0f64, 0usize);
        for (k, m) in profile.iter().enumerate().take(j).skip(i + 1) {
            let line = if span > 0.0 { p.z + (q.z - p.z) * (m.station - p.station) / span } else { p.z };
            let d = (m.z - line).abs();
            if d > worst.0 {
                worst = (d, k);
            }
        }
        if worst.0 > tolerance {
            keep[worst.1] = true;
            stack.push((i, worst.1));
            stack.push((worst.1, j));
        }
    }
    let mut kept: Vec<usize> = (0..n).filter(|k| keep[*k]).collect();
    if let Some(most) = most.filter(|m| *m > 0.0) {
        // In a gap longer than `most`, walking from its start, the last break within `most` of the station kept before
        // is kept, or the first past it when none is within (the profile's own places, so the heights stay exact).
        let mut out = vec![kept[0]];
        for w in kept.windows(2) {
            let (a, b) = (w[0], w[1]);
            let mut last = profile[a].station;
            for k in a + 1..b {
                let next = profile.get(k + 1).map_or(f64::INFINITY, |x| x.station);
                if profile[b].station - last > most + 1e-9 && next - last > most + 1e-9 {
                    out.push(k);
                    last = profile[k].station;
                }
            }
            out.push(b);
        }
        kept = out;
    }
    kept
}

/// How near two lines' meeting may be to an end of either and still not cross them (m).
pub const TOUCH: f64 = 1e-9;

/// Two sample lines meeting: their indexes in the stations (the first the lower), the side (left true), and how far
/// out on each line they meet (m).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Crossing {
    pub first: usize,
    pub second: usize,
    pub left: bool,
    pub at_first: f64,
    pub at_second: f64,
}

/// The place and left normal of the square line at `station`.
fn normal(al: &Alignment, station: f64) -> Option<([f64; 2], [f64; 2])> {
    let p = al.at(station)?;
    Some(([p.x, p.y], [-p.direction.sin(), p.direction.cos()]))
}

/// Every pair of square lines at `stations` (each `widths` (left, right) wide) that meet within both, on the same side
/// of both (a line crossing the alignment itself is not a crossing of sections), by the stations' order.
pub fn crossings(al: &Alignment, stations: &[f64], widths: &[(f64, f64)]) -> Vec<Crossing> {
    let lines: Vec<Option<([f64; 2], [f64; 2])>> = stations.iter().map(|s| normal(al, *s)).collect();
    let mut out = Vec::new();
    for i in 0..stations.len() {
        let Some((p, n)) = lines[i] else { continue };
        let (li, ri) = widths[i];
        for j in i + 1..stations.len() {
            let Some((q, m)) = lines[j] else { continue };
            let (lj, rj) = widths[j];
            // Quick: the two segments' centres farther apart than their half lengths cannot meet.
            let reach = li.max(ri) + lj.max(rj);
            if (q[0] - p[0]).hypot(q[1] - p[1]) > reach {
                continue;
            }
            // p + t·n = q + u·m.
            let det = n[0] * (-m[1]) - n[1] * (-m[0]);
            if det.abs() < 1e-12 {
                continue;
            }
            let d = [q[0] - p[0], q[1] - p[1]];
            let t = (d[0] * (-m[1]) - d[1] * (-m[0])) / det;
            let u = (n[0] * d[1] - n[1] * d[0]) / det;
            // Lines that only touch (a clipped line ends at the meeting) do not cross.
            let within = |x: f64, l: f64, r: f64| x > TOUCH && x < l - TOUCH || x < -TOUCH && -x < r - TOUCH;
            if t * u > 0.0 && within(t, li, ri) && within(u, lj, rj) {
                out.push(Crossing { first: i, second: j, left: t > 0.0, at_first: t.abs(), at_second: u.abs() });
            }
        }
    }
    out
}

/// The widths with lines shortened so no two meet, and the crossings found on the widths given. The meetings are
/// taken nearest first (by the nearer of their two distances out): where both lines still reach a meeting, both are
/// shortened to it; a line already shortened short of it leaves the other as it is. One pass leaves no two lines
/// meeting: a pair meeting on the final widths met on the given ones, and when its turn came either both reached it
/// (and were shortened to it) or one did not (and widths only shorten).
pub fn clip(al: &Alignment, stations: &[f64], widths: &[(f64, f64)]) -> (Vec<(f64, f64)>, Vec<Crossing>) {
    let mut w = widths.to_vec();
    let found = crossings(al, stations, &w);
    let mut order: Vec<&Crossing> = found.iter().collect();
    // Nearest first by the micrometre: on a circle every pair meets at its centre, and rounding must not decide the
    // order among them (the lines' order does).
    let key = |c: &Crossing| ((c.at_first.min(c.at_second) * 1e6).round() as i64, c.first, c.second);
    order.sort_by_key(|c| key(c));
    for c in order {
        let side = |w: &[(f64, f64)], k: usize| if c.left { w[k].0 } else { w[k].1 };
        if side(&w, c.first) > c.at_first + TOUCH && side(&w, c.second) > c.at_second + TOUCH {
            for (k, at) in [(c.first, c.at_first), (c.second, c.at_second)] {
                if c.left {
                    w[k].0 = at;
                } else {
                    w[k].1 = at;
                }
            }
        }
    }
    (w, found)
}
