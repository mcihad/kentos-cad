//! A road's mass haul (Brückner) diagram read as layers (docs/adr/0907, plan/road.md Y9; ROAD-HAUL-01, 07): the mass
//! ordinate `M(s)` (the running cut less fill, in the volume sheet's state) is a polyline along the stations, a borrow
//! or waste site a vertical step in it. Against a balance line `b`, where `f = M − b` crosses zero is a balance point;
//! between two balance points is a loop, hauled forward where `f > 0` (the cut is behind the fill), backward where
//! `f < 0`.
//!
//! The diagram is read as horizontal layers: at a level `y` above the line (below it for `f < 0`), each stretch of
//! stations where `|f| ≥ y` is a layer of volume `dy` carried across its width. A loop's balanced volume is its height,
//! its haul moment (volume × distance, m³·m) the area between the curve and the line, its average haul the moment over
//! the volume. With a free haul distance `F`, a layer's haul up to `F` is free and beyond it overhaul: the overhaul
//! volume is the layers wider than `F`, the overhaul moment `∫ max(0, w − F) dy` (Civil 3D's balance point method, read
//! layer by layer). A layer that reaches an end of the diagram without a balance point has no destination there: its
//! volume is unbalanced, waste where `f > 0` and borrow where `f < 0`, and is not hauled.
//!
//! Everything is exact for the polyline: between two consecutive vertex levels each layer's ends lie on fixed segments,
//! so its width is linear in `y` and the integrals are closed-form.

/// One loop between two balance points, or an end stretch that does not close.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Loop {
    pub from: f64,
    pub to: f64,
    /// Hauled forward (the curve above the line) or backward.
    pub forward: bool,
    /// The volume balanced in it (m³).
    pub volume: f64,
    /// The volume × distance hauled (m³·m).
    pub moment: f64,
    /// The volume hauled beyond the free haul distance (m³), and its distance beyond it (m³·m).
    pub over_volume: f64,
    pub over_moment: f64,
    /// The volume with no destination on the line (m³): waste forward, borrow backward.
    pub unbalanced: f64,
}

/// The diagram against a balance line: its balance points, its loops (and the open end stretches), the totals.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Diagram {
    pub balance: Vec<f64>,
    pub loops: Vec<Loop>,
    pub volume: f64,
    pub moment: f64,
    pub over_volume: f64,
    pub over_moment: f64,
    /// The volume left over: to be wasted, and to be borrowed (m³).
    pub waste: f64,
    pub borrow: f64,
}

/// The polyline split where `f` changes sign: the stations where it crosses (or touches) zero.
fn crossings(points: &[[f64; 2]], b: f64) -> Vec<f64> {
    let mut out: Vec<f64> = Vec::new();
    let push = |s: f64, out: &mut Vec<f64>| {
        if out.last().is_none_or(|l| *l != s) {
            out.push(s);
        }
    };
    for (i, p) in points.iter().enumerate() {
        let f = p[1] - b;
        if f == 0.0 && i > 0 && i + 1 < points.len() {
            push(p[0], &mut out);
        }
        if let Some(q) = points.get(i + 1) {
            let g = q[1] - b;
            if (f < 0.0 && g > 0.0) || (f > 0.0 && g < 0.0) {
                push(p[0] + (q[0] - p[0]) * f / (f - g), &mut out);
            }
        }
    }
    out
}

/// The ordinate at station `s`: where the curve steps there, the value after the step (`after`) or before it.
fn value(points: &[[f64; 2]], s: f64, after: bool) -> f64 {
    let at: Vec<usize> = (0..points.len()).filter(|&i| points[i][0] == s).collect();
    if let (Some(&i), Some(&j)) = (at.first(), at.last()) {
        return points[if after { j } else { i }][1];
    }
    let i = points.partition_point(|p| p[0] < s).clamp(1, points.len() - 1);
    let (p, q) = (points[i - 1], points[i]);
    p[1] + (q[1] - p[1]) * (s - p[0]) / (q[0] - p[0])
}

/// The polyline between stations `a` and `c`, as `g = sign·(M − b)`: its value after any step at `a`, the points
/// strictly between, its value before any step at `c`.
fn piece(points: &[[f64; 2]], b: f64, sign: f64, a: f64, c: f64) -> Vec<[f64; 2]> {
    let mut out = vec![[a, sign * (value(points, a, true) - b)]];
    out.extend(points.iter().filter(|p| p[0] > a && p[0] < c).map(|p| [p[0], sign * (p[1] - b)]));
    out.push([c, sign * (value(points, c, false) - b)]);
    out
}

/// The layers of `g ≥ 0` (a polyline along the stations): the balanced volume and moment, the overhaul past `free`,
/// and the volume of the layers that reach an open end (`open_start`, `open_end`).
fn layers(g: &[[f64; 2]], free: f64, open_start: bool, open_end: bool) -> (f64, f64, f64, f64, f64) {
    let mut levels: Vec<f64> = g.iter().map(|p| p[1].max(0.0)).collect();
    levels.push(0.0);
    levels.sort_by(f64::total_cmp);
    levels.dedup();
    let (mut volume, mut moment, mut over_v, mut over_m, mut open) = (0.0, 0.0, 0.0, 0.0, 0.0);
    // A layer's end: on a segment (linear in y) or at a boundary station.
    let cross = |i: usize, y: f64| -> f64 {
        let (p, q) = (g[i], g[i + 1]);
        if q[1] == p[1] { p[0] } else { p[0] + (q[0] - p[0]) * (y - p[1]) / (q[1] - p[1]) }
    };
    for w in levels.windows(2) {
        let (y0, y1) = (w[0], w[1]);
        let dy = y1 - y0;
        let ym = 0.5 * (y0 + y1);
        // The layer's ends at the middle level: (segment or none for the boundary, entering).
        let mut ends: Vec<Option<usize>> = Vec::new();
        let mut inside = g[0][1] >= ym;
        if inside {
            ends.push(None);
        }
        for i in 0..g.len() - 1 {
            let (a, c) = (g[i][1] - ym, g[i + 1][1] - ym);
            if (a < 0.0) != (c < 0.0) {
                ends.push(Some(i));
                inside = !inside;
            }
        }
        if inside {
            ends.push(None);
        }
        let last = g.len() - 1;
        for pair in ends.chunks(2) {
            let [l, r] = [pair[0], pair[1]];
            if (l.is_none() && open_start) || (r.is_none() && open_end) {
                open += dy;
                continue;
            }
            let x = |e: Option<usize>, y: f64, left: bool| match e {
                Some(i) => cross(i, y),
                None => g[if left { 0 } else { last }][0],
            };
            let w0 = x(r, y0, false) - x(l, y0, true);
            let w1 = x(r, y1, false) - x(l, y1, true);
            volume += dy;
            moment += 0.5 * (w0 + w1) * dy;
            // ∫ max(0, w − free) dy and the measure where w > free, for a linear w.
            let (e0, e1) = (w0 - free, w1 - free);
            if e0 >= 0.0 && e1 >= 0.0 {
                over_m += 0.5 * (e0 + e1) * dy;
                if e0 > 0.0 || e1 > 0.0 {
                    over_v += dy;
                }
            } else if e0 > 0.0 || e1 > 0.0 {
                let (hi, lo) = if e0 > 0.0 { (e0, e1) } else { (e1, e0) };
                let part = dy * hi / (hi - lo);
                over_m += 0.5 * hi * part;
                over_v += part;
            }
        }
    }
    (volume, moment, over_v, over_m, open)
}

/// The diagram of the mass ordinate `points` (station, ordinate; stations not decreasing, a repeated station a step)
/// against the balance line `b`, with the free haul distance `free` (m; 0: all hauling is overhaul).
pub fn diagram(points: &[[f64; 2]], b: f64, free: f64) -> Diagram {
    let mut out = Diagram::default();
    if points.len() < 2 {
        return out;
    }
    let balance = crossings(points, b);
    let (first, last) = (points[0][0], points[points.len() - 1][0]);
    let mut cuts = vec![first];
    cuts.extend(balance.iter().copied().filter(|s| *s > first && *s < last));
    cuts.push(last);
    cuts.dedup();
    for (k, w) in cuts.windows(2).enumerate() {
        let (a, c) = (w[0], w[1]);
        if c <= a {
            continue;
        }
        // The loop's side: the sign of f in its middle.
        let probe = piece(points, b, 1.0, a, c);
        let mid = probe.iter().map(|p| p[1]).fold(0.0, |m: f64, v| if v.abs() > m.abs() { v } else { m });
        if mid == 0.0 {
            continue;
        }
        let forward = mid > 0.0;
        let g = piece(points, b, if forward { 1.0 } else { -1.0 }, a, c);
        let open_start = k == 0 && g[0][1] > 0.0;
        let open_end = k + 2 == cuts.len() && g[g.len() - 1][1] > 0.0;
        let (volume, moment, over_volume, over_moment, unbalanced) = layers(&g, free, open_start, open_end);
        out.volume += volume;
        out.moment += moment;
        out.over_volume += over_volume;
        out.over_moment += over_moment;
        if forward {
            out.waste += unbalanced;
        } else {
            out.borrow += unbalanced;
        }
        out.loops.push(Loop { from: a, to: c, forward, volume, moment, over_volume, over_moment, unbalanced });
    }
    out.balance = balance;
    out
}
