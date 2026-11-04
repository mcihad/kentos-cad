//! A template's pavement on a cross section: its layers, the formation under them and the earthwork measured from
//! the formation (docs/adr/0930, plan/road.md Y26; ROAD-COR-01, 11, ROAD-VOL-09, 19).
//!
//! The layers lie under the finish grade, each its thickness deep, parallel to it (Civil 3D's lane subassemblies,
//! OpenRoads' pavement components, the KGM typical section's aşınma, binder, temel, alttemel). Across the platform
//! they follow its components; at each side's hinge they end:
//!
//! - **vertical** (`Edge::Vertical`): at the hinge, a vertical line closes the layers (Civil 3D's lane closes its
//!   shapes with vertical links);
//! - **on a slope** (`Edge::Slope(h)`): the layers run on outward, parallel to the side's last component, until a line
//!   from the hinge falling `h` horizontal per one vertical meets them (Civil 3D's ShoulderExtendAll, “hold slope”).
//!   With `h` the fill slope, the pavement's edge is the embankment's face.
//!
//! The pavement never stands above the finished line: where a ditch's fore slope falls faster than the edge, the
//! layers are cut by it. Nor does it reach past the section's footprint (where the finished line meets the terrain):
//! out there the earthwork is not measured, and pavement counted there would be taken from a fill that is not. The bottom of the last layer and the edge make the formation (formasyon, the subgrade).
//!
//! Every quantity is the integral, over the offset, of a difference of piecewise linear functions under `min` and
//! `max`: on each stretch between the functions' corners all of them are linear, the lower envelope of a few lines
//! is concave and the upper convex, so cutting the stretch where any two of them cross and where the difference is
//! zero leaves pieces on which the integrand is linear: exact.
//!
//! The earthwork from the formation (Netpro's Formasyon–Zemin) follows from the road's (Yol–Zemin): the pavement is
//! under the finished line, so its part under the ground is dug as well (cut + below) and its part above the ground
//! is not earth (fill − above).

use super::sample::GroundPoint;
use super::template::{Component, Section, Template};

/// How a pavement's layers end at a side's hinge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Edge {
    /// A vertical line at the hinge.
    Vertical,
    /// A line from the hinge falling `h` horizontal per one vertical (1.5 is 3/2), the layers carried out to it.
    Slope(f64),
}

/// A pavement: its layers' thicknesses (m) from the top down and how they end.
#[derive(Clone, Debug, PartialEq)]
pub struct Pavement {
    pub layers: Vec<f64>,
    pub edge: Edge,
}

/// Why a pavement is not built.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PavementError {
    /// No layer, or a layer's thickness not above 0 or not finite (its index).
    Layer(Option<usize>),
    /// The edge's slope not above 0, or not falling faster than the side's last component (the layers would never
    /// meet it): the side (`right`).
    Edge { right: bool },
    /// A side without a component: no platform to lay it on.
    NoPlatform { right: bool },
}

/// A pavement built on a section: each layer's area, the pavement's under and above the ground, the formation line and
/// the layers' bottoms.
#[derive(Clone, Debug, PartialEq)]
pub struct PavementSection {
    /// Each layer's area (m²), from the top down.
    pub layers: Vec<f64>,
    /// The pavement's area under the ground and above it (m²); where the ground is, only.
    pub below: f64,
    pub above: f64,
    /// The formation (the last layer's bottom) from where it meets the left edge to the right: (offset, elevation),
    /// offsets as the section's. The edges themselves are the lines from its ends up to the hinges.
    pub formation: Vec<[f64; 2]>,
    /// Each layer's bottom line, left end to right end (the last is the formation's bottom); for drawing.
    pub bottoms: Vec<Vec<[f64; 2]>>,
}

impl PavementSection {
    pub fn area(&self) -> f64 {
        self.layers.iter().sum()
    }
}

/// A line `z = a + b·u` on a stretch.
#[derive(Clone, Copy, Debug)]
struct Line {
    a: f64,
    b: f64,
}

impl Line {
    fn at(&self, u: f64) -> f64 {
        self.a + self.b * u
    }
}

/// A piecewise linear function of the outward offset: its corners in order; outside them it is not defined.
#[derive(Clone, Debug)]
struct Pl(Vec<[f64; 2]>);

impl Pl {
    /// The line on the piece holding `[u0, u1]` (u0 < u1); none where the function is not defined there.
    fn line(&self, u0: f64, u1: f64) -> Option<Line> {
        let p = &self.0;
        if p.len() < 2 || u0 < p[0][0] || u1 > p[p.len() - 1][0] {
            return None;
        }
        let mid = 0.5 * (u0 + u1);
        let i = p.partition_point(|q| q[0] <= mid).clamp(1, p.len() - 1);
        let (a, b) = (p[i - 1], p[i]);
        if b[0] == a[0] {
            return None;
        }
        let slope = (b[1] - a[1]) / (b[0] - a[0]);
        Some(Line { a: a[1] - slope * a[0], b: slope })
    }

    fn corners(&self) -> impl Iterator<Item = f64> + '_ {
        self.0.iter().map(|p| p[0])
    }
}

/// `∫ max(0, min(hi ∪ caps) − max(lo)) du` over `[u0, u1]` where every function of `hi` and `lo` is defined (the
/// stretches where one is not are left out); a cap counts only where it is defined.
fn integral(hi: &[&Pl], caps: &[Pl], lo: &[&Pl], u0: f64, u1: f64) -> f64 {
    if u1 <= u0 {
        return 0.0;
    }
    let mut stops: Vec<f64> = vec![u0, u1];
    for f in hi.iter().copied().chain(caps.iter()).chain(lo.iter().copied()) {
        stops.extend(f.corners().filter(|u| *u > u0 && *u < u1));
    }
    stops.sort_by(f64::total_cmp);
    stops.dedup();
    let mut total = 0.0;
    for w in stops.windows(2) {
        let (a, b) = (w[0], w[1]);
        if b <= a {
            continue;
        }
        let (Some(mut h), Some(l)) = (
            hi.iter().map(|f| f.line(a, b)).collect::<Option<Vec<Line>>>(),
            lo.iter().map(|f| f.line(a, b)).collect::<Option<Vec<Line>>>(),
        ) else {
            continue;
        };
        // A cap holds only where it is known (the finished line ends at the daylight).
        h.extend(caps.iter().filter_map(|f| f.line(a, b)));
        // Where the envelopes change: any two lines of a set crossing inside the stretch.
        let mut cuts = vec![a, b];
        for set in [&h, &l] {
            for i in 0..set.len() {
                for j in i + 1..set.len() {
                    let db = set[i].b - set[j].b;
                    if db != 0.0 {
                        let u = (set[j].a - set[i].a) / db;
                        if u > a && u < b {
                            cuts.push(u);
                        }
                    }
                }
            }
        }
        cuts.sort_by(f64::total_cmp);
        cuts.dedup();
        for c in cuts.windows(2) {
            let (x0, x1) = (c[0], c[1]);
            if x1 <= x0 {
                continue;
            }
            // On (x0, x1) both envelopes are linear: the difference is, read at the ends from the mid's lines.
            let m = 0.5 * (x0 + x1);
            let pick_h = h.iter().copied().min_by(|p, q| p.at(m).total_cmp(&q.at(m))).expect("a line");
            let pick_l = l.iter().copied().max_by(|p, q| p.at(m).total_cmp(&q.at(m))).expect("a line");
            let (f0, f1) = (pick_h.at(x0) - pick_l.at(x0), pick_h.at(x1) - pick_l.at(x1));
            total += positive_part(x0, x1, f0, f1);
        }
    }
    total
}

/// `∫ max(0, f)` over `[x0, x1]` for `f` linear from `f0` to `f1`.
fn positive_part(x0: f64, x1: f64, f0: f64, f1: f64) -> f64 {
    let w = x1 - x0;
    match (f0 > 0.0, f1 > 0.0) {
        (true, true) => 0.5 * (f0 + f1) * w,
        (false, false) => 0.0,
        (true, false) => 0.5 * f0 * w * f0 / (f0 - f1),
        (false, true) => 0.5 * f1 * w * f1 / (f1 - f0),
    }
}

/// One side's functions outward from the axis.
struct SideFns {
    /// The finish grade, extended past the hinge with the last component's slope where the edge is a slope.
    finish: Pl,
    /// Above which the pavement does not stand: the edge, and the finished line past the hinge.
    caps: Vec<Pl>,
    /// Where the pavement ends outward (m).
    end: f64,
    hinge: [f64; 2],
    sign: f64,
}

/// The finish grade's corners outward (u, z) from the axis at `axis`, and its last slope (per metre).
fn finish_of(components: &[Component], axis: f64) -> (Vec<[f64; 2]>, f64) {
    let mut pts = vec![[0.0, axis]];
    let (mut u, mut z, mut last) = (0.0, axis, 0.0);
    for c in components {
        let (w, s) = match *c {
            Component::Lane { width, slope } | Component::Shoulder { width, slope } => (width, slope),
        };
        u += w;
        z += w * s / 100.0;
        last = s / 100.0;
        pts.push([u, z]);
    }
    (pts, last)
}

fn side_fns(
    components: &[Component],
    built: &super::template::SideSection,
    axis: f64,
    depth: f64,
    edge: Edge,
    right: bool,
) -> Result<SideFns, PavementError> {
    if components.is_empty() {
        return Err(PavementError::NoPlatform { right });
    }
    let sign = if right { 1.0 } else { -1.0 };
    let (mut finish, last) = finish_of(components, axis);
    let hinge = *finish.last().expect("a corner");
    // The finished line outward from the hinge (the ditch, the slope), from the section as built.
    let outward: Vec<[f64; 2]> = built.points.iter().map(|p| [sign * p[0], p[1]]).filter(|p| p[0] >= hinge[0]).collect();
    let mut caps = Vec::new();
    let end = match edge {
        Edge::Vertical => hinge[0],
        Edge::Slope(h) => {
            if !(h.is_finite() && h > 0.0) || last + 1.0 / h <= 0.0 {
                return Err(PavementError::Edge { right });
            }
            let reach = depth / (last + 1.0 / h);
            let end = hinge[0] + reach;
            finish.push([end, hinge[1] + last * reach]);
            caps.push(Pl(vec![hinge, [end, hinge[1] - reach / h]]));
            // Not past the section's footprint: where its finished line ends (the daylight), or the hinge.
            let foot = outward.last().map_or(hinge[0], |p| p[0]);
            if outward.len() >= 2 && foot > hinge[0] {
                caps.push(Pl(outward));
            }
            end.min(foot)
        }
    };
    Ok(SideFns { finish: Pl(finish), caps, end, hinge, sign })
}

/// The ground outward on a side: its stretches on the surface as separate functions.
fn ground_runs(ground: &[GroundPoint], right: bool) -> Vec<Pl> {
    let sign = if right { 1.0 } else { -1.0 };
    let mut pts: Vec<(f64, Option<f64>, bool)> = ground.iter().map(|g| (sign * g.offset, g.z, g.gap)).collect();
    if !right {
        // Read leftward: the gap flag belongs to the stretch before a place in the left-to-right order.
        let n = pts.len();
        let gaps: Vec<bool> = (0..n).map(|i| ground.get(i + 1).is_some_and(|g| g.gap)).collect();
        for (p, gap) in pts.iter_mut().zip(gaps) {
            p.2 = gap;
        }
        pts.reverse();
    }
    let mut runs = Vec::new();
    let mut run: Vec<[f64; 2]> = Vec::new();
    for (u, z, gap) in pts {
        match z {
            Some(z) if !gap => run.push([u, z]),
            Some(z) => {
                if run.len() > 1 {
                    runs.push(Pl(std::mem::take(&mut run)));
                }
                run = vec![[u, z]];
            }
            None => {
                if run.len() > 1 {
                    runs.push(Pl(std::mem::take(&mut run)));
                }
                run.clear();
            }
        }
    }
    if run.len() > 1 {
        runs.push(Pl(run));
    }
    runs
}

fn shifted(f: &Pl, d: f64) -> Pl {
    Pl(f.0.iter().map(|p| [p[0], p[1] - d]).collect())
}

/// The pavement `pavement` lays on `section` (built from `template` at its axis) against `ground` (the natural or the
/// stripped ground; offsets from left to right as the section's).
pub fn build(
    template: &Template,
    section: &Section,
    pavement: &Pavement,
    ground: &[GroundPoint],
) -> Result<PavementSection, PavementError> {
    if pavement.layers.is_empty() {
        return Err(PavementError::Layer(None));
    }
    for (i, t) in pavement.layers.iter().enumerate() {
        if !(t.is_finite() && *t > 0.0) {
            return Err(PavementError::Layer(Some(i)));
        }
    }
    let depths: Vec<f64> = pavement
        .layers
        .iter()
        .scan(0.0, |d, t| {
            *d += t;
            Some(*d)
        })
        .collect();
    let total = *depths.last().expect("a layer");
    let left = side_fns(&template.left.components, &section.left, section.axis, total, pavement.edge, false)?;
    let right = side_fns(&template.right.components, &section.right, section.axis, total, pavement.edge, true)?;

    let mut layers = vec![0.0; depths.len()];
    let (mut below, mut above) = (0.0, 0.0);
    let mut halves: Vec<Vec<Vec<[f64; 2]>>> = Vec::new();
    for (s, right) in [(&left, false), (&right, true)] {
        let mut top_d = 0.0;
        for (i, d) in depths.iter().enumerate() {
            let top = shifted(&s.finish, top_d);
            let bottom = shifted(&s.finish, *d);
            layers[i] += integral(&[&top], &s.caps, &[&bottom], 0.0, s.end);
            top_d = *d;
        }
        let top = shifted(&s.finish, 0.0);
        let bottom = shifted(&s.finish, total);
        for g in ground_runs(ground, right) {
            below += integral(&[&top, &g], &s.caps, &[&bottom], 0.0, s.end);
            above += integral(&[&top], &s.caps, &[&bottom, &g], 0.0, s.end);
        }
        // Lines for drawing, outward: each layer's bottom to the edge; the formation is the last of them.
        let lines: Vec<Vec<[f64; 2]>> = depths.iter().map(|d| edge_cut(&shifted(&s.finish, *d), s, pavement.edge)).collect();
        halves.push(lines.into_iter().map(|l| l.into_iter().map(|p| [s.sign * p[0], p[1]]).collect()).collect());
    }
    // Left to right: the left half reversed, then the right without its axis point.
    let join = |l: &[[f64; 2]], r: &[[f64; 2]]| -> Vec<[f64; 2]> { l.iter().rev().chain(r.iter().skip(1)).copied().collect() };
    let bottoms: Vec<Vec<[f64; 2]>> = (0..depths.len()).map(|i| join(&halves[0][i], &halves[1][i])).collect();
    let formation = bottoms.last().cloned().unwrap_or_default();
    Ok(PavementSection { layers, below, above, formation, bottoms })
}

/// A layer bottom `line` (outward) to where it meets the side's edge: the hinge's foot for a vertical edge, the edge
/// line for a slope.
fn edge_cut(line: &Pl, s: &SideFns, edge: Edge) -> Vec<[f64; 2]> {
    let end = match edge {
        Edge::Vertical => s.hinge[0],
        Edge::Slope(_) => {
            // The edge line from the hinge: where the layer's line meets it.
            let cap = &s.caps[0].0;
            let (h0, h1) = (cap[0], cap[1]);
            let e = Line { b: (h1[1] - h0[1]) / (h1[0] - h0[0]), a: h0[1] - (h1[1] - h0[1]) / (h1[0] - h0[0]) * h0[0] };
            let n = line.0.len();
            let (p, q) = (line.0[n - 2], line.0[n - 1]);
            let l = Line { b: (q[1] - p[1]) / (q[0] - p[0]), a: p[1] - (q[1] - p[1]) / (q[0] - p[0]) * p[0] };
            let u = (l.a - e.a) / (e.b - l.b);
            u.clamp(s.hinge[0], s.end)
        }
    };
    let mut out: Vec<[f64; 2]> = line.0.iter().copied().filter(|p| p[0] < end).collect();
    let i = line.0.partition_point(|p| p[0] < end).clamp(1, line.0.len() - 1);
    let (p, q) = (line.0[i - 1], line.0[i]);
    let z = if q[0] == p[0] { q[1] } else { p[1] + (q[1] - p[1]) * (end - p[0]) / (q[0] - p[0]) };
    out.push([end, z]);
    out
}
