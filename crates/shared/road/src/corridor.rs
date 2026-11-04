//! A road's corridor (docs/adr/0905, plan/road.md Y7; ROAD-COR-06, 08, 09, 10, 11, 13, 16): regions of an alignment
//! each with its template, the stations a region is sampled at, and from the sections built there the feature lines
//! (a coded point of each section joined station to station) and the top surface (consecutive sections joined by a
//! strip of triangles), with the stretches where a section or a side is not solved.
//!
//! The sections themselves are the cross sections' (`crate::section`): the corridor only says where and joins them.

use crate::alignment::Alignment;
use crate::alignment::element::Kind as ElementKind;
use crate::section::SampleLine;
use crate::section::template::{Kind, Section, Side, SideSection, Template};

/// Two stations closer than this are one (1 mm).
pub const SAME_STATION: f64 = 1e-3;
/// The most stations a corridor samples.
pub const MAX_STATIONS: usize = 20_000;

/// Why a station is sampled, strongest first: when two fall within [`SAME_STATION`] the stronger stays.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Why {
    /// A region's start or end.
    Region,
    /// A horizontal geometry point (TS, SC, CS, ST, an element's start).
    Geometry,
    /// A superelevation critical station.
    Critical,
    /// A profile's PVI, a vertical curve's end, its high or low point.
    Profile,
    /// A station the user wrote.
    User,
    /// A division of a curve or a clothoid.
    Curve,
    /// A multiple of the tangent interval.
    Interval,
    /// A division of a vertical curve by its chord height (docs/adr/0939 §1).
    Vertical,
    /// A station added where the section between two stations strays from their straight join (docs/adr/0939 §2).
    Refine,
}

/// A sampled station and why.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Station {
    pub station: f64,
    pub why: Why,
}

/// How often a corridor is sampled: the tangent interval (m), the curve and clothoid interval (m) and an optional
/// chord tolerance (m): a curve's step is at most the chord whose middle stands that far from the arc.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frequency {
    pub tangent: f64,
    pub curve: f64,
    pub sag: Option<f64>,
}

/// Why regions or a frequency make no corridor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CorridorError {
    /// A region whose end is not after its start (its index).
    EmptyRegion { index: usize },
    /// A region off the alignment (its index).
    Outside { index: usize },
    /// Two regions overlap (their indices, the earlier first).
    Overlap { first: usize, second: usize },
    /// An interval not above 0 or a tolerance not above 0.
    BadFrequency,
    /// More stations than [`MAX_STATIONS`].
    TooMany,
}

/// The regions judged: none empty, none off the alignment, none overlapping (touching is allowed); the gaps between
/// them in order.
pub fn check_regions(regions: &[[f64; 2]], length: f64) -> Result<Vec<[f64; 2]>, CorridorError> {
    for (i, r) in regions.iter().enumerate() {
        if !(r[0].is_finite() && r[1].is_finite() && r[1] - r[0] >= SAME_STATION) {
            return Err(CorridorError::EmptyRegion { index: i });
        }
        if r[0] < -1e-9 || r[1] > length + 1e-9 {
            return Err(CorridorError::Outside { index: i });
        }
    }
    let mut order: Vec<usize> = (0..regions.len()).collect();
    order.sort_by(|a, b| regions[*a][0].total_cmp(&regions[*b][0]).then(a.cmp(b)));
    let mut gaps = Vec::new();
    for w in order.windows(2) {
        let (a, b) = (regions[w[0]], regions[w[1]]);
        if b[0] < a[1] - SAME_STATION {
            let (first, second) = if w[0] < w[1] { (w[0], w[1]) } else { (w[1], w[0]) };
            return Err(CorridorError::Overlap { first, second });
        }
        if b[0] > a[1] + SAME_STATION {
            gaps.push([a[1], b[0]]);
        }
    }
    Ok(gaps)
}

/// The step a curve of radius `r` is divided by: the curve interval, or the chord whose middle stands `sag` from the
/// arc when shorter (2·√(2rt − t²); a tolerance not under the radius allows the diameter).
pub fn curve_step(f: &Frequency, r: f64) -> f64 {
    match f.sag {
        Some(t) if r.is_finite() && r > 0.0 => {
            let chord = if t >= r { 2.0 * r } else { 2.0 * (2.0 * r * t - t * t).sqrt() };
            f.curve.min(chord)
        }
        _ => f.curve,
    }
}

/// A region's stations: its ends; the tangents' multiples of the interval (from station 0); each curve and clothoid
/// divided equally (the whole element, by [`curve_step`] at its smallest radius); the elements' starts and ends; the
/// `events` given (critical, profile, user stations); within [`SAME_STATION`] the strongest reason stays.
pub fn stations(al: &Alignment, region: [f64; 2], f: &Frequency, events: &[Station]) -> Result<Vec<Station>, CorridorError> {
    let ok = |v: f64| v.is_finite() && v > 0.0;
    if !(ok(f.tangent) && ok(f.curve) && f.sag.is_none_or(ok)) {
        return Err(CorridorError::BadFrequency);
    }
    let [a, b] = region;
    let inside = |s: f64| s >= a - 1e-9 && s <= b + 1e-9;
    let mut all = vec![Station { station: a, why: Why::Region }, Station { station: b, why: Why::Region }];
    for e in &al.elements {
        let (e0, e1) = (e.station, e.station + e.length);
        if e1 < a - 1e-9 || e0 > b + 1e-9 {
            continue;
        }
        for s in [e0, e1] {
            if inside(s) {
                all.push(Station { station: s, why: Why::Geometry });
            }
        }
        match e.kind {
            ElementKind::Tangent => {
                let (lo, hi) = (e0.max(a), e1.min(b));
                let first = (lo / f.tangent).ceil() as i64;
                let last = (hi / f.tangent).floor() as i64;
                if (last - first) as f64 > MAX_STATIONS as f64 {
                    return Err(CorridorError::TooMany);
                }
                for k in first..=last {
                    all.push(Station { station: k as f64 * f.tangent, why: Why::Interval });
                }
            }
            ElementKind::Arc | ElementKind::Spiral => {
                let k = e.k0.abs().max(e.k1.abs());
                let step = curve_step(f, if k > 0.0 { 1.0 / k } else { f64::INFINITY });
                let n = (e.length / step - 1e-9).ceil().max(1.0);
                if n > MAX_STATIONS as f64 {
                    return Err(CorridorError::TooMany);
                }
                for i in 1..n as usize {
                    let s = e0 + e.length * i as f64 / n;
                    if inside(s) {
                        all.push(Station { station: s, why: Why::Curve });
                    }
                }
            }
        }
        if all.len() > MAX_STATIONS * 2 {
            return Err(CorridorError::TooMany);
        }
    }
    all.extend(events.iter().filter(|e| inside(e.station)).copied());
    all.retain(|s| inside(s.station));
    for s in &mut all {
        s.station = s.station.clamp(a, b);
    }
    all.sort_by(|x, y| x.station.total_cmp(&y.station).then(x.why.cmp(&y.why)));
    let mut out: Vec<Station> = Vec::new();
    let mut anchor = f64::NEG_INFINITY;
    for s in all {
        if s.station - anchor < SAME_STATION {
            let last = out.last_mut().expect("a group has its first");
            if s.why < last.why {
                *last = s;
            }
        } else {
            anchor = s.station;
            out.push(s);
        }
    }
    if out.len() > MAX_STATIONS {
        return Err(CorridorError::TooMany);
    }
    Ok(out)
}

/// A station's cross: its sample line and the section built there, none where it is not solved.
#[derive(Clone, Debug)]
pub struct Cross {
    pub line: SampleLine,
    pub section: Option<Section>,
}

/// A region's crosses in order and its template (its components and ditches give the points their codes).
#[derive(Clone, Debug)]
pub struct RegionCrosses {
    pub template: Template,
    pub crosses: Vec<Cross>,
}

/// A feature line: a code's points (station, x, y, z) in one unbroken run, and its region.
#[derive(Clone, Debug, PartialEq)]
pub struct Feature {
    pub code: String,
    pub region: usize,
    pub points: Vec<[f64; 4]>,
}

/// Why a side is not solved over a stretch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unsolved {
    /// No section: off the profile or the surface at the axis, or no template.
    NoSection,
    /// The slope meets the terrain nowhere on the swath.
    Open,
}

/// A stretch of a side not solved.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stretch {
    pub region: usize,
    pub right: bool,
    pub why: Unsolved,
    pub from: f64,
    pub to: f64,
}

/// What a top surface triangle is part of: a lane or a shoulder of the platform (the template's component it lies on),
/// or a side's slope beyond the hinge, in cut (a ditch with it) or in fill.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Role {
    Lane,
    Shoulder,
    Cut,
    Fill,
}

/// What a corridor's crosses make.
#[derive(Clone, Debug, Default)]
pub struct Built {
    pub features: Vec<Feature>,
    /// The top surface's triangles, counterclockwise in plan.
    pub triangles: Vec<[[f64; 3]; 3]>,
    /// Each triangle's role: the role of the cross segment its edge along the cross lies on.
    pub roles: Vec<Role>,
    pub unsolved: Vec<Stretch>,
    /// The stretches (region, from, to) where consecutive sections cross in plan and no surface is made.
    pub folded: Vec<(usize, f64, f64)>,
    /// The top surface's strips between consecutive solved sections: (region, from, to, the strip's triangles' range
    /// in `triangles`), so a stretch's volume reads its own triangles (docs/adr/0906).
    pub strips: Vec<(usize, f64, f64, std::ops::Range<usize>)>,
}

/// A place at `offset` (right positive) on a sample line.
pub fn place(line: &SampleLine, offset: f64) -> [f64; 2] {
    let (rx, ry) = (line.direction.sin(), -line.direction.cos());
    [line.at[0] + rx * offset, line.at[1] + ry * offset]
}

/// The codes of a side's points, outward from the axis: `axis`, the components' outer edges `c1…cN`, the ditch's toe
/// and its bottom's end, then the daylight point when it is a point of its own.
pub fn side_codes(side: &Side, built: &SideSection) -> Vec<String> {
    let mut out = vec!["axis".to_owned()];
    out.extend((1..=side.components.len()).map(|i| format!("c{i}")));
    if built.kind == Kind::Cut
        && let Some(d) = side.ditch
    {
        out.push("ditchToe".into());
        if d.bottom > 0.0 {
            out.push("ditchBottom".into());
        }
    }
    if out.len() < built.points.len() {
        out.push("daylight".into());
    }
    out.truncate(built.points.len());
    out
}

/// A side's coded points: (code with its side, offset, elevation); the hinge and the daylight as their own codes too.
fn coded(side: &Side, built: &SideSection, right: bool) -> Vec<(String, f64, f64)> {
    let word = if right { "right" } else { "left" };
    let mut out: Vec<(String, f64, f64)> = side_codes(side, built)
        .into_iter()
        .zip(&built.points)
        .skip(1)
        .map(|(c, p)| (format!("{word}.{c}"), p[0], p[1]))
        .collect();
    out.push((format!("{word}.hinge"), built.hinge[0], built.hinge[1]));
    if let Some(d) = built.daylight
        && !out.iter().any(|c| c.0.ends_with(".daylight"))
    {
        out.push((format!("{word}.daylight"), d[0], d[1]));
    }
    out
}

/// A coded point of a cross for the stakeout table (docs/adr/0908): its code (`axis`, `left.c1`, …, as the feature
/// lines name it), its offset from the axis (right positive), its plan place and elevation.
#[derive(Clone, Debug, PartialEq)]
pub struct StakePoint {
    pub code: String,
    pub offset: f64,
    pub at: [f64; 2],
    pub z: f64,
}

/// A cross's coded points left to right (the axis, then each side's points as the feature lines code them), none where
/// the cross has no section. Points at the same offset keep the feature lines' order.
pub fn stake_points(template: &Template, cross: &Cross) -> Vec<StakePoint> {
    let Some(sec) = &cross.section else { return Vec::new() };
    let mut out = vec![StakePoint { code: "axis".into(), offset: 0.0, at: place(&cross.line, 0.0), z: sec.axis }];
    for (side, b, right) in [(&template.left, &sec.left, false), (&template.right, &sec.right, true)] {
        for (code, o, z) in coded(side, b, right) {
            out.push(StakePoint { code, offset: o, at: place(&cross.line, o), z });
        }
    }
    out.sort_by(|a, b| a.offset.total_cmp(&b.offset));
    out
}

/// The cross's line on the top surface, left to right: the whole side, or to the hinge on an open side.
fn top_line(line: &SampleLine, section: &Section, components: [usize; 2]) -> Vec<[f64; 3]> {
    let upto = |s: &SideSection, n: usize| -> Vec<[f64; 2]> {
        if s.kind == Kind::Open { s.points[..=n.min(s.points.len() - 1)].to_vec() } else { s.points.clone() }
    };
    let left = upto(&section.left, components[0]);
    let right = upto(&section.right, components[1]);
    left.iter()
        .rev()
        .chain(right.iter().skip(1))
        .map(|p| {
            let xy = place(line, p[0]);
            [xy[0], xy[1], p[1]]
        })
        .collect()
}

/// The roles of the segments of [`top_line`]'s line, left to right: a side's segment k from the axis is its k-th
/// component while there are components, its slope beyond.
fn top_roles(template: &Template, section: &Section, components: [usize; 2]) -> Vec<Role> {
    let side = |t: &Side, s: &SideSection, n: usize| -> Vec<Role> {
        let used = if s.kind == Kind::Open { n.min(s.points.len() - 1) + 1 } else { s.points.len() };
        (0..used.saturating_sub(1))
            .map(|k| match t.components.get(k) {
                Some(crate::section::template::Component::Lane { .. }) => Role::Lane,
                Some(crate::section::template::Component::Shoulder { .. }) => Role::Shoulder,
                None if s.kind == Kind::Cut => Role::Cut,
                None => Role::Fill,
            })
            .collect()
    };
    let mut out: Vec<Role> = side(&template.left, &section.left, components[0]).into_iter().rev().collect();
    out.extend(side(&template.right, &section.right, components[1]));
    out
}

fn plan_area(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> f64 {
    0.5 * ((b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1]))
}

/// Two plan diagonals within this (m) are a tie in [`strip`].
pub const SAME_DIAGONAL: f64 = 1e-9;

fn plan_distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    (b[0] - a[0]).hypot(b[1] - a[1])
}

/// The strip of triangles between two lines across the road (the earlier station's `a`, the later's `b`, both left to
/// right): from their starts on, the shorter plan diagonal each step, `a` advancing on a tie (diagonals within
/// [`SAME_DIAGONAL`] of each other: a rectangle's two are equal, and a rounding must not choose); each triangle
/// counterclockwise when the stations go forward. Indices into `a` then `b` (offset by `a.len()`).
pub fn strip(a: &[[f64; 3]], b: &[[f64; 3]]) -> Vec<[usize; 3]> {
    let (n, m) = (a.len(), b.len());
    let mut out = Vec::with_capacity(n + m);
    if n == 0 || m == 0 {
        return out;
    }
    let (mut i, mut j) = (0, 0);
    while i + 1 < n || j + 1 < m {
        let take_a = if i + 1 == n {
            false
        } else if j + 1 == m {
            true
        } else {
            plan_distance(a[i + 1], b[j]) <= plan_distance(a[i], b[j + 1]) + SAME_DIAGONAL
        };
        if take_a {
            out.push([i, i + 1, n + j]);
            i += 1;
        } else {
            out.push([i, n + j + 1, n + j]);
            j += 1;
        }
    }
    out
}

/// The feature lines, the top surface and the stretches not solved of a corridor's regions.
/// A cross's coded points (x, y, z) as its feature lines take them: the axis, then each side's points outward; none
/// where the cross has no section.
pub fn coded_points(t: &Template, cross: &Cross) -> Option<Vec<(String, [f64; 3])>> {
    let sec = cross.section.as_ref()?;
    let at = place(&cross.line, 0.0);
    let mut out = vec![("axis".to_owned(), [at[0], at[1], sec.axis])];
    for (side, b, right) in [(&t.left, &sec.left, false), (&t.right, &sec.right, true)] {
        for (code, o, z) in coded(side, b, right) {
            let xy = place(&cross.line, o);
            out.push((code, [xy[0], xy[1], z]));
        }
    }
    Some(out)
}

pub fn assemble(regions: &[RegionCrosses]) -> Built {
    let mut built = Built::default();
    for (ri, region) in regions.iter().enumerate() {
        let t = &region.template;
        let counts = [t.left.components.len(), t.right.components.len()];
        // Feature lines: each code's run, broken where a station has no such point.
        let mut open: Vec<Feature> = Vec::new();
        for cross in &region.crosses {
            let s = cross.line.station;
            let mut here: Vec<(String, [f64; 4])> =
                coded_points(t, cross).unwrap_or_default().into_iter().map(|(c, p)| (c, [s, p[0], p[1], p[2]])).collect();
            let mut next = Vec::new();
            for f in open.drain(..) {
                match here.iter().position(|h| h.0 == f.code) {
                    Some(k) => {
                        let (code, p) = here.remove(k);
                        let mut f = f;
                        f.points.push(p);
                        debug_assert_eq!(f.code, code);
                        next.push(f);
                    }
                    None => built.features.push(f),
                }
            }
            for (code, p) in here {
                next.push(Feature { code, region: ri, points: vec![p] });
            }
            open = next;
        }
        built.features.extend(open);
        // The stretches not solved, side by side.
        for right in [false, true] {
            let mut run: Option<(Unsolved, f64, f64)> = None;
            for cross in &region.crosses {
                let s = cross.line.station;
                let why = match &cross.section {
                    None => Some(Unsolved::NoSection),
                    Some(sec) => {
                        let side = if right { &sec.right } else { &sec.left };
                        (side.kind == Kind::Open).then_some(Unsolved::Open)
                    }
                };
                run = match (run, why) {
                    (Some((w, from, _)), Some(now)) if w == now => Some((w, from, s)),
                    (Some((w, from, to)), now) => {
                        built.unsolved.push(Stretch { region: ri, right, why: w, from, to });
                        now.map(|n| (n, s, s))
                    }
                    (None, now) => now.map(|n| (n, s, s)),
                };
            }
            if let Some((why, from, to)) = run {
                built.unsolved.push(Stretch { region: ri, right, why, from, to });
            }
        }
        // The top surface, section to section.
        for w in region.crosses.windows(2) {
            let (Some(sa), Some(sb)) = (&w[0].section, &w[1].section) else { continue };
            let a = top_line(&w[0].line, sa, counts);
            let b = top_line(&w[1].line, sb, counts);
            let (ra, rb) = (top_roles(t, sa, counts), top_roles(t, sb, counts));
            let picked = strip(&a, &b);
            let tris: Vec<[[f64; 3]; 3]> = picked
                .iter()
                .map(|t| {
                    let p = |k: usize| if k < a.len() { a[k] } else { b[k - a.len()] };
                    [p(t[0]), p(t[1]), p(t[2])]
                })
                .collect();
            // A triangle's edge across the road is on `a` (its first two corners) or on `b` (its last two).
            let roles: Vec<Role> = picked
                .iter()
                .map(|t| {
                    if t[1] < a.len() {
                        ra.get(t[0]).copied().unwrap_or(Role::Lane)
                    } else {
                        rb.get(t[2] - a.len()).copied().unwrap_or(Role::Lane)
                    }
                })
                .collect();
            if tris.iter().any(|t| plan_area(t[0], t[1], t[2]) <= 0.0) {
                let (s0, s1) = (w[0].line.station, w[1].line.station);
                match built.folded.last_mut() {
                    Some(last) if last.0 == ri && (last.2 - s0).abs() < SAME_STATION => last.2 = s1,
                    _ => built.folded.push((ri, s0, s1)),
                }
                continue;
            }
            let at = built.triangles.len();
            built.triangles.extend(tris);
            built.roles.extend(roles);
            built.strips.push((ri, w[0].line.station, w[1].line.station, at..built.triangles.len()));
        }
    }
    built
}
