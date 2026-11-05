//! A template's platform on a cross section and its slopes tied to the terrain (docs/adr/0903 §3–§5, ROAD-SEC-04, 05,
//! 07; ROAD-COR-01…04, 10). Each side is built outward from the axis at the design elevation: its components (lanes,
//! shoulders: a width and a cross slope in percent, falling outward negative) set the elevations to the hinge; there the
//! terrain decides: above the hinge a cut, an optional ditch (its fore slope down to its depth, its bottom) then the cut
//! slope up; otherwise a fill, the fill slope down. Where a slope first meets the terrain is the daylight point, exact
//! (two lines' crossing); a slope that meets it nowhere on the swath leaves the side open. The cut and fill areas are
//! the integral of the design line less the terrain between the outermost points, exact for lines.

use super::sample::GroundPoint;

/// A component of a side, outward from the axis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Component {
    /// A lane: its width and cross slope (percent, falling outward negative).
    Lane { width: f64, slope: f64 },
    /// A shoulder (banket).
    Shoulder { width: f64, slope: f64 },
}

impl Component {
    fn parts(&self) -> (f64, f64) {
        match *self {
            Component::Lane { width, slope } | Component::Shoulder { width, slope } => (width, slope),
        }
    }
}

/// A ditch in a cut: its fore slope (horizontal per one vertical), its depth under the hinge and its bottom's width.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ditch {
    pub fore: f64,
    pub depth: f64,
    pub bottom: f64,
}

/// A side of a template: its components and its end condition, the cut and fill slopes (horizontal per one vertical,
/// 1.5 is 3/2), an optional ditch in the cut and the cut slope's benches (docs/adr/0958).
#[derive(Clone, Debug, PartialEq)]
pub struct Side {
    pub components: Vec<Component>,
    pub cut: f64,
    pub fill: f64,
    pub ditch: Option<Ditch>,
    pub bench: Option<CutBench>,
}

/// A cut slope's benches (berme, docs/adr/0958; ROAD-GRADE-03): every `height` m up the slope a bench `width` m wide
/// whose surface falls `fall` percent outward (toward the face above, where its ditch takes the water).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CutBench {
    pub height: f64,
    pub width: f64,
    pub fall: f64,
}

/// The most benches a cut slope makes: past them the slope does not meet the terrain.
pub const MAX_BENCHES: usize = 200;

/// A template: its left and right sides.
#[derive(Clone, Debug, PartialEq)]
pub struct Template {
    pub left: Side,
    pub right: Side,
}

/// How a side ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Cut,
    Fill,
    /// No daylight on the swath: the slope meets the terrain nowhere (or the terrain is missing at the hinge).
    Open,
}

/// A side as built: its points outward from the axis (offset, signed as the section's, and elevation), the hinge,
/// the daylight point when found, and how it ends.
#[derive(Clone, Debug, PartialEq)]
pub struct SideSection {
    pub points: Vec<[f64; 2]>,
    pub hinge: [f64; 2],
    pub daylight: Option<[f64; 2]>,
    pub kind: Kind,
}

/// A cross section built: the axis elevation, its sides, the cut and fill areas (m²).
#[derive(Clone, Debug, PartialEq)]
pub struct Section {
    pub axis: f64,
    pub left: SideSection,
    pub right: SideSection,
    pub cut: f64,
    pub fill: f64,
}

/// Why a template builds no section.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TemplateError {
    /// A width not above 0, a slope not finite, an end slope or a ditch's fore slope or depth not above 0, a ditch's
    /// bottom below 0: the side (`left`, `right`) and the component's index (none for the end condition).
    Bad { right: bool, component: Option<usize> },
}

/// What is wrong with a side.
fn judge(side: &Side, right: bool) -> Result<(), TemplateError> {
    for (i, c) in side.components.iter().enumerate() {
        let (w, s) = c.parts();
        if !(w.is_finite() && w > 0.0 && s.is_finite()) {
            return Err(TemplateError::Bad { right, component: Some(i) });
        }
    }
    let positive = |v: f64| v.is_finite() && v > 0.0;
    let ditch_ok =
        side.ditch.is_none_or(|d| positive(d.fore) && positive(d.depth) && d.bottom.is_finite() && d.bottom >= 0.0);
    let bench_ok = side.bench.is_none_or(|b| positive(b.height) && positive(b.width) && b.fall.is_finite() && (0.0..=50.0).contains(&b.fall));
    if !(positive(side.cut) && positive(side.fill) && ditch_ok && bench_ok) {
        return Err(TemplateError::Bad { right, component: None });
    }
    Ok(())
}

/// The terrain outward from the axis on one side: (distance outward, height, the stretch before is off the surface),
/// in order. A place's distance outward is its offset on the right, minus it on the left.
fn outward(ground: &[GroundPoint], right: bool) -> Vec<(f64, Option<f64>, bool)> {
    let sign = if right { 1.0 } else { -1.0 };
    let mut out: Vec<(f64, Option<f64>, bool)> = Vec::with_capacity(ground.len());
    if right {
        for g in ground {
            out.push((g.offset, g.z, g.gap));
        }
    } else {
        // Read leftward: the stretch before a place is the one the right-to-left order gives.
        for (i, g) in ground.iter().enumerate().rev() {
            let gap = ground.get(i + 1).is_some_and(|n| n.gap);
            out.push((sign * g.offset, g.z, gap));
        }
    }
    out
}

/// The terrain's height at `u` outward; none off the surface.
fn height(t: &[(f64, Option<f64>, bool)], u: f64) -> Option<f64> {
    let i = t.partition_point(|p| p.0 < u);
    if i < t.len() && t[i].0 == u {
        return t[i].1;
    }
    if i == 0 || i == t.len() || t[i].2 {
        return None;
    }
    let (a, b) = (t[i - 1], t[i]);
    let (za, zb) = (a.1?, b.1?);
    Some(za + (zb - za) * (u - a.0) / (b.0 - a.0))
}

/// Where the line from (u0, z0) rising `m` per metre outward first meets the terrain at or beyond u0.
fn meet(t: &[(f64, Option<f64>, bool)], u0: f64, z0: f64, m: f64) -> Option<[f64; 2]> {
    meet_within(t, u0, z0, m, f64::INFINITY)
}

/// Where the line from (u0, z0) rising `m` per metre outward first meets the terrain between u0 and u1.
fn meet_within(t: &[(f64, Option<f64>, bool)], u0: f64, z0: f64, m: f64, u1: f64) -> Option<[f64; 2]> {
    let line = |u: f64| z0 + m * (u - u0);
    for w in t.windows(2) {
        let ((ua, za, _), (ub0, zb0, gap)) = (w[0], w[1]);
        if ub0 < u0 || gap {
            continue;
        }
        if ua > u1 {
            break;
        }
        let (Some(za), Some(zb0)) = (za, zb0) else { continue };
        // The stretch up to where the line ends.
        let (ub, zb) = if ub0 > u1 { (u1, za + (zb0 - za) * (u1 - ua) / (ub0 - ua)) } else { (ub0, zb0) };
        // The stretch from where the slope starts.
        let a = ua.max(u0);
        let ga = za + (zb - za) * (a - ua) / (ub - ua);
        let (fa, fb) = (line(a) - ga, line(ub) - zb);
        if fa == 0.0 {
            return Some([a, ga]);
        }
        if (fa < 0.0) != (fb < 0.0) || fb == 0.0 {
            // f is linear on [a, ub]: its zero.
            let u = a + (ub - a) * fa / (fa - fb);
            return Some([u, line(u)]);
        }
    }
    None
}

fn side(side: &Side, axis: f64, ground: &[GroundPoint], right: bool) -> SideSection {
    let t = outward(ground, right);
    let sign = if right { 1.0 } else { -1.0 };
    let mut pts = vec![[0.0, axis]];
    let (mut u, mut z) = (0.0, axis);
    for c in &side.components {
        let (w, s) = c.parts();
        u += w;
        z += w * s / 100.0;
        pts.push([u, z]);
    }
    let hinge = [u, z];
    let (kind, daylight) = match height(&t, u) {
        None => (Kind::Open, None),
        Some(g) if g > z => {
            // A cut: the ditch, then the cut slope up.
            let (mut bu, mut bz) = (u, z);
            if let Some(d) = side.ditch {
                bu += d.depth * d.fore;
                bz -= d.depth;
                pts.push([bu, bz]);
                if d.bottom > 0.0 {
                    bu += d.bottom;
                    pts.push([bu, bz]);
                }
            }
            match side.bench {
                // The benched slope (docs/adr/0958): face, bench, face …; its corners up to where it meets the terrain.
                Some(b) => {
                    let reach = t.last().map_or(bu, |p| p.0);
                    let corners = bench_path([bu, bz], 1.0 / side.cut, b, reach);
                    let (mut hit, before) = (None, pts.len());
                    for w in corners.windows(2) {
                        let (a, e) = (w[0], w[1]);
                        let m = (e[1] - a[1]) / (e[0] - a[0]);
                        if let Some(p) = meet_within(&t, a[0], a[1], m, e[0]) {
                            hit = Some(p);
                            break;
                        }
                        pts.push(e);
                    }
                    match hit {
                        Some(p) => (Kind::Cut, Some(p)),
                        None => {
                            // Not met: the corners pushed are no slope.
                            pts.truncate(before);
                            (Kind::Open, None)
                        }
                    }
                }
                None => match meet(&t, bu, bz, 1.0 / side.cut) {
                    Some(p) => (Kind::Cut, Some(p)),
                    None => (Kind::Open, None),
                },
            }
        }
        Some(_) => match meet(&t, u, z, -1.0 / side.fill) {
            Some(p) => (Kind::Fill, Some(p)),
            None => (Kind::Open, None),
        },
    };
    if let Some(p) = daylight
        && pts.last().is_none_or(|q| (q[0] - p[0]).abs() > 1e-12 || (q[1] - p[1]).abs() > 1e-12)
    {
        pts.push(p);
    }
    let flip = |p: [f64; 2]| [sign * p[0], p[1]];
    SideSection {
        points: pts.into_iter().map(flip).collect(),
        hinge: flip(hinge),
        daylight: daylight.map(flip),
        kind,
    }
}

/// A benched cut slope's corners from `start` (docs/adr/0958): a face rising `m` per metre outward until it is `height`
/// above its foot, a bench `width` wide falling `fall` percent outward, again, until past `reach` outward (or
/// [`MAX_BENCHES`] benches). The last corner is beyond `reach`.
pub fn bench_path(start: [f64; 2], m: f64, b: CutBench, reach: f64) -> Vec<[f64; 2]> {
    let mut out = vec![start];
    let [mut u, mut z] = start;
    for _ in 0..MAX_BENCHES {
        u += b.height / m;
        z += b.height;
        out.push([u, z]);
        if u > reach {
            return out;
        }
        u += b.width;
        z -= b.width * b.fall / 100.0;
        out.push([u, z]);
        if u > reach {
            return out;
        }
    }
    // The last face, long enough to leave the swath.
    let tail = (reach - u).max(0.0) + 1.0;
    out.push([u + tail, z + m * tail]);
    out
}

/// How a side's slope meets the terrain (docs/adr/0951, ROAD-COR-10): what [`build`]'s `Open` leaves together, apart.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Daylight {
    /// It meets it at `offset` (outward, m): the slope line crosses the terrain `crossings` times on the swath (the
    /// first is the daylight); the next crossing lies `next` m further out (none: no other); the daylight moves
    /// `sensitivity` m outward per metre the terrain there is higher or lower, 1/|m − g| for the slope's and the
    /// terrain's gradients there.
    Hit { offset: f64, crossings: u32, next: Option<f64>, sensitivity: f64, cut: bool },
    /// No terrain at the hinge.
    Hinge,
    /// The terrain stops at `at` (outward, m) before the slope meets it: a hole or the surface's edge on the swath.
    Gap { at: f64 },
    /// The swath ends at `end` (outward, m) before the slope meets the terrain: the section is too short.
    Swath { end: f64 },
}

/// Each side's daylight, left then right, for a template built at `axis` on `ground`: the slope from the hinge (past a
/// cut's ditch) run outward along the swath, its crossings with the terrain counted (a sign change between two
/// places, or a place on the line), stopped where the terrain does.
pub fn daylights(template: &Template, axis: f64, ground: &[GroundPoint]) -> [Daylight; 2] {
    [diagnose(&template.left, axis, ground, false), diagnose(&template.right, axis, ground, true)]
}

fn diagnose(side: &Side, axis: f64, ground: &[GroundPoint], right: bool) -> Daylight {
    let t = outward(ground, right);
    let (mut u, mut z) = (0.0, axis);
    for c in &side.components {
        let (w, s) = c.parts();
        u += w;
        z += w * s / 100.0;
    }
    let Some(g) = height(&t, u) else { return Daylight::Hinge };
    let cut = g > z;
    let (u0, z0, m) = if cut {
        let (mut bu, mut bz) = (u, z);
        if let Some(d) = side.ditch {
            bu += d.depth * d.fore;
            bz -= d.depth;
            bu += d.bottom;
        }
        (bu, bz, 1.0 / side.cut)
    } else {
        (u, z, -1.0 / side.fill)
    };
    // The slope: a line, or a benched cut's corners (docs/adr/0958).
    let reach_end = t.last().map_or(u0, |p| p.0);
    let corners: Vec<[f64; 2]> = match (cut, side.bench) {
        (true, Some(b)) => bench_path([u0, z0], m, b, reach_end),
        _ => Vec::new(),
    };
    let line = |x: f64| -> f64 {
        if corners.len() < 2 {
            return z0 + m * (x - u0);
        }
        let i = corners.partition_point(|c| c[0] < x).clamp(1, corners.len() - 1);
        let (a, b) = (corners[i - 1], corners[i]);
        a[1] + (b[1] - a[1]) * (x - a[0]) / (b[0] - a[0])
    };
    // The terrain's places from u0 outward while it lasts: (outward, f = line − terrain, the terrain's gradient after).
    let mut places: Vec<(f64, f64)> = Vec::new();
    let mut stop: Option<f64> = None;
    let last = t.last().map_or(u0, |p| p.0);
    match height(&t, u0) {
        Some(g0) => places.push((u0, line(u0) - g0)),
        // A slope starting at the swath's end (within a micrometre: a cut's ditch pushed it to 6 m on a 6 m swath whose
        // terrain ended at 5.999999999999998) has no swath left, not a hole.
        None if last - u0 <= 1e-6 => return Daylight::Swath { end: u0 },
        None => return Daylight::Gap { at: u0 },
    }
    for w in t.windows(2) {
        let ((_, _, _), (ub, zb, gap)) = (w[0], w[1]);
        if ub <= u0 {
            continue;
        }
        if gap || w[0].1.is_none() && w[0].0 >= u0 {
            stop = Some(places.last().map_or(u0, |p| p.0));
            break;
        }
        let Some(zb) = zb else {
            stop = Some(places.last().map_or(u0, |p| p.0));
            break;
        };
        places.push((ub, line(ub) - zb));
    }
    // A benched slope's corners within the places are places too: between them and the terrain's, both are lines.
    if corners.len() > 1
        && let Some(end) = places.last().map(|p| p.0)
    {
        for c in &corners[1..] {
            if c[0] > u0 && c[0] < end
                && let Some(g) = height(&t, c[0])
                && !places.iter().any(|p| p.0 == c[0])
            {
                places.push((c[0], line(c[0]) - g));
            }
        }
        places.sort_by(|a, b| a.0.total_cmp(&b.0));
    }
    // The crossings: a place on the line, or a sign change inside a stretch.
    let mut hits: Vec<(f64, f64)> = Vec::new();
    for (i, &(x, f)) in places.iter().enumerate() {
        if f == 0.0 {
            let grad = places.get(i + 1).or(i.checked_sub(1).and_then(|k| places.get(k))).map_or(0.0, |q| {
                let (xa, fa, xb, fb) = if q.0 > x { (x, f, q.0, q.1) } else { (q.0, q.1, x, f) };
                // f's slope is the line's less the terrain's.
                (fb - fa) / (xb - xa)
            });
            hits.push((x, grad));
        }
        if let Some(&(xb, fb)) = places.get(i + 1)
            && f != 0.0
            && fb != 0.0
            && (f < 0.0) != (fb < 0.0)
        {
            hits.push((x + (xb - x) * f / (f - fb), (fb - f) / (xb - x)));
        }
    }
    // A break within a micrometre of the swath's last place is its end: the surface's section can end in a sliver
    // off it (5e-15 m seen on a 6 m swath), which is no hole in the terrain.
    if stop.is_some_and(|at| last - at <= 1e-6) {
        stop = None;
    }
    match hits.first() {
        Some(&(at, df)) => Daylight::Hit {
            offset: at,
            crossings: hits.len() as u32,
            next: hits.get(1).map(|h| h.0 - at),
            // f′ = m − g: one metre of terrain height moves the zero by 1/|f′|.
            sensitivity: if df != 0.0 { 1.0 / df.abs() } else { f64::INFINITY },
            cut,
        },
        None => match stop {
            Some(at) => Daylight::Gap { at },
            None => Daylight::Swath { end: places.last().map_or(u0, |p| p.0) },
        },
    }
}

/// The cut and fill areas between the design line (offsets in order) and the terrain.
fn areas(design: &[[f64; 2]], ground: &[GroundPoint]) -> (f64, f64) {
    let (cut, fill, _, _) = areas_moments(design, ground);
    (cut, fill)
}

/// The cut and fill areas between the design line and the terrain and their first moments about the alignment
/// (`∫ o dA`, the offset right positive; docs/adr/0934 §1): for a linear depth f over [x₀, x₁] the area is
/// `(f₀ + f₁)(x₁ − x₀)/2` and the moment `(x₁ − x₀)(f₀(2x₀ + x₁) + f₁(x₀ + 2x₁))/6`, both split where f changes sign.
fn areas_moments(design: &[[f64; 2]], ground: &[GroundPoint]) -> (f64, f64, f64, f64) {
    let t: Vec<(f64, Option<f64>, bool)> = ground.iter().map(|g| (g.offset, g.z, g.gap)).collect();
    let (lo, hi) = (design[0][0], design[design.len() - 1][0]);
    let mut stops: Vec<f64> = design.iter().map(|p| p[0]).chain(ground.iter().map(|g| g.offset)).filter(|o| *o >= lo && *o <= hi).collect();
    stops.sort_by(f64::total_cmp);
    stops.dedup();
    let line = |o: f64| {
        let i = design.partition_point(|p| p[0] < o).clamp(1, design.len() - 1);
        let (a, b) = (design[i - 1], design[i]);
        if b[0] == a[0] { b[1] } else { a[1] + (b[1] - a[1]) * (o - a[0]) / (b[0] - a[0]) }
    };
    let (mut cut, mut fill, mut qcut, mut qfill) = (0.0, 0.0, 0.0, 0.0);
    for w in stops.windows(2) {
        let (a, b) = (w[0], w[1]);
        let mid = 0.5 * (a + b);
        if height(&t, mid).is_none() {
            continue;
        }
        let (Some(ga), Some(gb)) = (height(&t, a), height(&t, b)) else { continue };
        let (fa, fb) = (line(a) - ga, line(b) - gb);
        // The integral of a linear f and of o·f, split where f changes sign.
        let mut add = |x0: f64, x1: f64, f0: f64, f1: f64| {
            let area = 0.5 * (f0 + f1) * (x1 - x0);
            let moment = (x1 - x0) * (f0 * (2.0 * x0 + x1) + f1 * (x0 + 2.0 * x1)) / 6.0;
            if f0 + f1 > 0.0 {
                fill += area;
                qfill += moment;
            } else {
                cut -= area;
                qcut -= moment;
            }
        };
        if (fa < 0.0 && fb > 0.0) || (fa > 0.0 && fb < 0.0) {
            let x = a + (b - a) * fa / (fa - fb);
            add(a, x, fa, 0.0);
            add(x, b, 0.0, fb);
        } else {
            add(a, b, fa, fb);
        }
    }
    (cut, fill, qcut, qfill)
}

/// The cut and fill's first moments about the alignment (m³ per metre: `∫ o dA`, the offset right positive) of the
/// section's earthwork against the ground lowered by `strip` (the same areas as [`earthwork`]'s cut and fill), for the
/// correction of their volumes on a curve (docs/adr/0934 §1).
pub fn moments(section: &Section, ground: &[GroundPoint], strip: f64) -> (f64, f64) {
    let design: Vec<[f64; 2]> = section.left.points.iter().rev().chain(section.right.points.iter().skip(1)).copied().collect();
    let lowered: Vec<GroundPoint> = ground.iter().map(|g| GroundPoint { z: g.z.map(|z| z - strip), ..*g }).collect();
    let (_, _, qcut, qfill) = areas_moments(&design, &lowered);
    (qcut, qfill)
}

/// The section a template builds with its axis at `axis` on the terrain `ground` (offsets from left to right).
pub fn build(template: &Template, axis: f64, ground: &[GroundPoint]) -> Result<Section, TemplateError> {
    judge(&template.left, false)?;
    judge(&template.right, true)?;
    let left = side(&template.left, axis, ground, false);
    let right = side(&template.right, axis, ground, true);
    let design: Vec<[f64; 2]> = left.points.iter().rev().chain(right.points.iter().skip(1)).copied().collect();
    let (cut, fill) = areas(&design, ground);
    Ok(Section { axis, left, right, cut, fill })
}

/// The fill's foundation benched (diş kazısı, docs/adr/0906 §6): where the ground falls across steeper than `slope`
/// percent under a fill, steps `height` (m) high are cut into it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bench {
    pub slope: f64,
    pub height: f64,
}

/// A section's earthwork with stripping and benching (m²): the cut and fill against the ground stripped, the topsoil
/// stripped and the benches cut.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Earthwork {
    pub cut: f64,
    pub fill: f64,
    pub strip: f64,
    pub bench: f64,
}

/// The section's earthwork (docs/adr/0906 §6): the topsoil `strip` (m) thick taken off under its footprint (its design
/// line's span, where the terrain is), so the cut and fill are against the ground lowered by it and the topsoil is
/// `strip` times the footprint's width; then, where the stripped ground under a fill falls steeper than the bench's
/// slope, steps of its height whose sawtooth leaves half the height on average: the bench is that width times half the
/// height. The bench is its own quantity: it is refilled with what it gave, neither cut nor fill.
pub fn earthwork(section: &Section, ground: &[GroundPoint], strip: f64, bench: Option<Bench>) -> Earthwork {
    let design: Vec<[f64; 2]> = section.left.points.iter().rev().chain(section.right.points.iter().skip(1)).copied().collect();
    let lowered: Vec<GroundPoint> = ground.iter().map(|g| GroundPoint { z: g.z.map(|z| z - strip), ..*g }).collect();
    let (cut, fill) = areas(&design, &lowered);
    let t: Vec<(f64, Option<f64>, bool)> = lowered.iter().map(|g| (g.offset, g.z, g.gap)).collect();
    let (lo, hi) = (design[0][0], design[design.len() - 1][0]);
    let mut stops: Vec<f64> = design.iter().map(|p| p[0]).chain(ground.iter().map(|g| g.offset)).filter(|o| *o >= lo && *o <= hi).collect();
    stops.sort_by(f64::total_cmp);
    stops.dedup();
    let line = |o: f64| {
        let i = design.partition_point(|p| p[0] < o).clamp(1, design.len() - 1);
        let (a, b) = (design[i - 1], design[i]);
        if b[0] == a[0] { b[1] } else { a[1] + (b[1] - a[1]) * (o - a[0]) / (b[0] - a[0]) }
    };
    let (mut covered, mut benched) = (0.0, 0.0);
    for w in stops.windows(2) {
        let (a, b) = (w[0], w[1]);
        if height(&t, 0.5 * (a + b)).is_none() {
            continue;
        }
        let (Some(ga), Some(gb)) = (height(&t, a), height(&t, b)) else { continue };
        covered += b - a;
        let Some(bn) = bench else { continue };
        if ((gb - ga) / (b - a)).abs() * 100.0 < bn.slope {
            continue;
        }
        let (fa, fb) = (line(a) - ga, line(b) - gb);
        benched += match (fa > 0.0, fb > 0.0) {
            (true, true) => b - a,
            (false, false) => 0.0,
            (true, false) => (b - a) * fa / (fa - fb),
            (false, true) => (b - a) * fb / (fb - fa),
        };
    }
    Earthwork { cut, fill, strip: strip * covered, bench: bench.map_or(0.0, |bn| benched * bn.height / 2.0) }
}
