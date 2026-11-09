//! `road.terrains.sheetcontours` v1 (plan/map-cad.md MAP-14, docs/adr/1820): Pafta eş yükselti eğrileri, a surface's
//! contour lines and heights drawn for a sheet by BÖHHBÜY's Article 78(1)(c)–(g) and Ek-3 0001–0005, the values the
//! rule package's. The shared cases are `fixtures/commands/v1/road.terrains.sheetcontours.json`.
//!
//! - A triangle's spacing on the sheet is the interval over its slope, in paper millimetres at the scale (a flat or a
//!   degenerate triangle: none, endless). Under `steepBelow` the triangle draws only major lines; over
//!   `auxiliaryAbove` it draws the auxiliary line half an interval above each level (the core's level lines over the
//!   triangles taken, so a line breaks where the rule changes).
//! - A detail is a closed polyline's or a polygon's rings (straight edges; a bulge's arc read as its chord). A line's
//!   segment is split where it crosses a detail's edge; a part whose middle lies inside a detail (even-odd over its
//!   rings) is left out, its length counted in `cut`; a closed line cut opens, its first and last kept parts joined.
//! - Along each major part, from its first place, a height every `labelEvery` paper mm, the first half that from the
//!   start: the text in the middle, turned along the segment it falls on, its top towards the uphill side (the fall
//!   direction of the triangle under the segment's middle; a flat one keeps the segment's way), its box masked.
//! - Asked for (`spots`, docs/adr/2320; Article 78(1)(ğ)): inside each innermost closed major or minor line (no other
//!   such line's first place inside it, even-odd), the surface's vertex highest above the line's level, else the one
//!   lowest below it (an earlier vertex among equals; a vertex at the level is on the line; a vertex inside a detail is
//!   no ground shown), its height written to the decimetre (half up) with the package's spot height (Ek-3 0004)
//!   beside a point.
//! - Asked for (`slopes`, docs/adr/3120; Article 78(1)(ğ)'s slopes, their bottoms and tops): every corner of a line or
//!   polyline on the slopes' top layers, then on their bottom layers, in the drawing's order, once per place, its
//!   height the surface's there (none off the surface: counted), written as the summits' are. Asked for (`points`,
//!   docs/adr/3520; Article 78(1)(ğ)'s other points the surveyor chose): every point on those layers (each of a
//!   multi-point's), in the drawing's order, once per place, its own measured height when it has one (the surface's
//!   kept beside it, the most they part said), else the surface's (none off it: counted); only its height is written,
//!   the point being the surveyor's. A slope line that is
//!   one of the surface's breaklines has its corners among the surface's vertices: the same height.
//! - The auxiliary lines take the package's pattern (Ek-3 0003, docs/adr/2720) as the project line type “Yardımcı eğri
//!   (‹package›)”, added to the project's types when it has none of the name (the project's own kept when it has); the
//!   objects name it, their built-in type dashed. The types are a project setting, no undo step (docs/adr/0313): undo
//!   takes the lines and leaves the type.

use kentos_commands::{
    CommandResult, ContourRules, CreateOperation, EntitiesCreate, Entity, EntityGeometry, LineType, LineTypeDef, NewObject,
    PatternElement, PatternUnit, SheetContourLabel, SheetContourLine, SheetSlopes, SheetSpot, TerrainsSheetContoured,
    TerrainsSheetContours, TextAlign, Vec2, contour_rules_default,
};
use kentos_road::surface::Surface;
use kentos_road::surface::MAX_LEVELS;

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::road_terrains::{self as common, fail};

type P = [f64; 2];

struct Checked {
    out: TerrainsSheetContoured,
    rules: ContourRules,
    warnings: Vec<kentos_commands::CommandWarning>,
}

/// A detail's rings.
type Detail = Vec<Vec<P>>;

/// Whether `p` lies inside the closed `ring` (even-odd, the crossing to the right of `p`).
fn ring_holds(ring: &[P], p: P) -> bool {
    let n = ring.len();
    let mut holds = false;
    for i in 0..n {
        let (a, b) = (ring[i], ring[(i + 1) % n]);
        if (a[1] > p[1]) != (b[1] > p[1]) {
            let x = a[0] + (p[1] - a[1]) * (b[0] - a[0]) / (b[1] - a[1]);
            if x > p[0] {
                holds = !holds;
            }
        }
    }
    holds
}

/// The characteristic points (docs/adr/2320): a summit or a pit inside each innermost closed major or minor line.
fn spots_of(surface: &Surface, lines: &[SheetContourLine], details: &[Detail]) -> Vec<SheetSpot> {
    let regular: Vec<&SheetContourLine> =
        lines.iter().filter(|l| l.closed && (l.kind == "major" || l.kind == "minor") && l.points.len() >= 3).collect();
    let mut used: Vec<usize> = surface.triangles.iter().flatten().map(|&i| i as usize).collect();
    used.sort_unstable();
    used.dedup();
    let mut out = Vec::new();
    for (k, l) in regular.iter().enumerate() {
        let holds_another =
            regular.iter().enumerate().any(|(j, m)| j != k && m.points.first().is_some_and(|q| ring_holds(&l.points, *q)));
        if holds_another {
            continue;
        }
        let held: Vec<usize> = used
            .iter()
            .copied()
            .filter(|&i| {
                let v = surface.vertices[i];
                v[2] != l.z && ring_holds(&l.points, [v[0], v[1]]) && !details.iter().any(|d| inside(d, [v[0], v[1]]))
            })
            .collect();
        let z = |i: usize| surface.vertices[i][2];
        let top = held.iter().copied().max_by(|&a, &b| z(a).total_cmp(&z(b)).then(b.cmp(&a)));
        let bottom = held.iter().copied().min_by(|&a, &b| z(a).total_cmp(&z(b)).then(a.cmp(&b)));
        let found = match (top, bottom) {
            (Some(t), _) if z(t) > l.z => Some(("summit", t)),
            (_, Some(b)) if z(b) < l.z => Some(("pit", b)),
            _ => None,
        };
        if let Some((kind, i)) = found {
            let v = surface.vertices[i];
            out.push(SheetSpot {
                kind: kind.to_owned(),
                x: v[0],
                y: v[1],
                z: v[2],
                text: decimetre(v[2]),
                surface: None,
            });
        }
    }
    out
}

/// A height to the decimetre as written, half up (docs/adr/2320).
/// The points the surveyor chose (docs/adr/3520): each point on `layers` (a multi-point's each), in the drawing's order,
/// once per place; its own height when it has one (with the surface's there, if any), else the surface's; the points
/// with neither are counted, not written; and how many take their own height.
fn chosen_spots(doc: &kentos_domain::Document, surface: &Surface, layers: &[String]) -> (Vec<SheetSpot>, usize, usize) {
    let mut out = Vec::new();
    let mut seen: Vec<P> = Vec::new();
    let (mut off, mut measured) = (0, 0);
    for e in doc.entities().filter(|e| layers.iter().any(|l| l.trim() == e.base().layer_id)) {
        let Entity::Point(p) = e else { continue };
        let all = std::iter::once((p.p, p.z)).chain(p.parts.iter().flatten().map(|q| (q.p, q.z)));
        for (at, own) in all {
            let c = [at.x, at.y];
            if seen.contains(&c) {
                continue;
            }
            seen.push(c);
            let ground = surface.height(c[0], c[1]).z;
            match (own, ground) {
                (Some(z), g) => {
                    measured += 1;
                    out.push(SheetSpot { kind: "chosen".into(), x: c[0], y: c[1], z, text: decimetre(z), surface: g });
                }
                (None, Some(z)) => out.push(SheetSpot { kind: "chosen".into(), x: c[0], y: c[1], z, text: decimetre(z), surface: None }),
                (None, None) => off += 1,
            }
        }
    }
    (out, off, measured)
}

fn decimetre(z: f64) -> String {
    format!("{:.1}", (z * 10.0 + 0.5).floor() / 10.0)
}

/// The slopes' characteristic points (docs/adr/3120): each corner of a line or polyline on the top layers, then on the
/// bottom ones, in the drawing's order, once per place (the same coordinates), its height the surface's; the corners
/// off the surface are counted, not written.
fn slope_spots(doc: &kentos_domain::Document, surface: &Surface, slopes: &SheetSlopes) -> (Vec<SheetSpot>, usize) {
    let mut out = Vec::new();
    let mut seen: Vec<P> = Vec::new();
    let mut off = 0;
    for (kind, layers) in [("slopeTop", &slopes.top), ("slopeBottom", &slopes.bottom)] {
        for e in doc.entities().filter(|e| layers.iter().any(|l| l.trim() == e.base().layer_id)) {
            let corners: Vec<P> = match e {
                Entity::Line(l) => vec![[l.a.x, l.a.y], [l.b.x, l.b.y]],
                Entity::Polyline(p) => p
                    .pts
                    .iter()
                    .chain(p.parts.iter().flatten().flat_map(|part| part.pts.iter()))
                    .map(|v| [v.x, v.y])
                    .collect(),
                _ => continue,
            };
            for c in corners {
                if seen.contains(&c) {
                    continue;
                }
                seen.push(c);
                match surface.height(c[0], c[1]).z {
                    Some(z) => out.push(SheetSpot { kind: kind.to_owned(), x: c[0], y: c[1], z, text: decimetre(z), surface: None }),
                    None => off += 1,
                }
            }
        }
    }
    (out, off)
}

fn inside(rings: &Detail, p: P) -> bool {
    let mut odd = false;
    for ring in rings {
        let n = ring.len();
        for i in 0..n {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            if (a[1] > p[1]) != (b[1] > p[1]) {
                let x = a[0] + (p[1] - a[1]) / (b[1] - a[1]) * (b[0] - a[0]);
                if p[0] < x {
                    odd = !odd;
                }
            }
        }
    }
    odd
}

/// Where the segment p→q crosses the segment a→b, as p's parameter strictly between 0 and 1.
fn crossing(p: P, q: P, a: P, b: P) -> Option<f64> {
    let (rx, ry) = (q[0] - p[0], q[1] - p[1]);
    let (sx, sy) = (b[0] - a[0], b[1] - a[1]);
    let d = rx * sy - ry * sx;
    if d == 0.0 {
        return None;
    }
    let (wx, wy) = (a[0] - p[0], a[1] - p[1]);
    let t = (wx * sy - wy * sx) / d;
    let u = (wx * ry - wy * rx) / d;
    (t > 0.0 && t < 1.0 && (0.0..=1.0).contains(&u)).then_some(t)
}

/// The parts of a line outside every detail, and the length left out.
fn clip(points: &[P], closed: bool, details: &[Detail]) -> (Vec<Vec<P>>, f64, bool) {
    let n = points.len();
    let segments = if closed { n } else { n - 1 };
    let mut parts: Vec<Vec<P>> = Vec::new();
    let mut current: Vec<P> = Vec::new();
    let mut cut = 0.0;
    let mut touched = false;
    for i in 0..segments {
        let (p, q) = (points[i], points[(i + 1) % n]);
        let mut ts = vec![0.0, 1.0];
        for d in details {
            for ring in d {
                let m = ring.len();
                for j in 0..m {
                    if let Some(t) = crossing(p, q, ring[j], ring[(j + 1) % m]) {
                        ts.push(t);
                    }
                }
            }
        }
        ts.sort_by(f64::total_cmp);
        ts.dedup();
        for w in ts.windows(2) {
            let (t0, t1) = (w[0], w[1]);
            let a = [p[0] + t0 * (q[0] - p[0]), p[1] + t0 * (q[1] - p[1])];
            let b = [p[0] + t1 * (q[0] - p[0]), p[1] + t1 * (q[1] - p[1])];
            let mid = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0];
            if details.iter().any(|d| inside(d, mid)) {
                cut += (b[0] - a[0]).hypot(b[1] - a[1]);
                touched = true;
                if current.len() >= 2 {
                    parts.push(std::mem::take(&mut current));
                } else {
                    current.clear();
                }
            } else {
                if current.is_empty() {
                    current.push(a);
                }
                current.push(b);
            }
        }
    }
    if current.len() >= 2 {
        parts.push(current);
    }
    if closed && !touched {
        return (vec![points.to_vec()], 0.0, true);
    }
    // A closed line cut: its last part runs on into its first through the closing segment.
    if closed && parts.len() >= 2 {
        let first = &parts[0];
        let last = parts.last().map(|l| l[l.len() - 1]);
        if first[0] == points[0] && last == Some(points[0]) {
            let head = parts.remove(0);
            if let Some(tail) = parts.last_mut() {
                tail.extend(head.into_iter().skip(1));
            }
        }
    }
    (parts, cut, false)
}

/// The level's text: three decimals at most, no trailing zeros.
fn level_text(z: f64) -> String {
    let r = (z * 1000.0 + 0.5).floor() / 1000.0;
    format!("{r}")
}

/// The labels along a major part: every `every` metres, the first at half that.
fn labels(surface: &Surface, points: &[P], closed: bool, z: f64, every: f64) -> Vec<SheetContourLabel> {
    let n = points.len();
    let segments: Vec<(P, P)> = (0..if closed { n } else { n - 1 }).map(|i| (points[i], points[(i + 1) % n])).collect();
    let mut out = Vec::new();
    let mut s = every / 2.0;
    let mut walked = 0.0;
    for (p, q) in segments {
        let len = (q[0] - p[0]).hypot(q[1] - p[1]);
        if len == 0.0 {
            continue;
        }
        while s <= walked + len {
            let f = (s - walked) / len;
            let at = [p[0] + f * (q[0] - p[0]), p[1] + f * (q[1] - p[1])];
            let mut d = [(q[0] - p[0]) / len, (q[1] - p[1]) / len];
            // Its top uphill: against the fall direction of the triangle under the segment's middle.
            let mid = [(p[0] + q[0]) / 2.0, (p[1] + q[1]) / 2.0];
            if let Some(aspect) = surface.strictly_in(mid[0], mid[1]).and_then(|t| surface.slope_of(t)).and_then(|s| s.aspect) {
                let uphill = [-aspect.sin(), -aspect.cos()];
                let left = [-d[1], d[0]];
                if left[0] * uphill[0] + left[1] * uphill[1] < 0.0 {
                    d = [-d[0], -d[1]];
                }
            }
            out.push(SheetContourLabel { z, text: level_text(z), at, rotation: d[1].atan2(d[0]).to_degrees() });
            s += every;
        }
        walked += len;
    }
    out
}

fn check(doc: &kentos_domain::Document, input: &TerrainsSheetContours) -> Result<Checked, Stop> {
    common::terrain_project(doc)?;
    let rules: ContourRules = match &input.rules {
        Some(r) => r.clone(),
        None => contour_rules_default()
            .ok_or_else(|| fail(codes::INVALID_CONTOURS, "Yerleşik eğri kural paketi okunamadı.".into(), "rules"))?,
    };
    let positive = |v: f64| v.is_finite() && v > 0.0;
    if rules.id.trim().is_empty()
        || rules.source.trim().is_empty()
        || ![rules.major_weight, rules.minor_weight, rules.auxiliary_weight, rules.steep_below, rules.auxiliary_above, rules.label_every, rules.label_height]
            .into_iter()
            .all(positive)
        || !rules.intervals.iter().all(|i| positive(i.interval))
        // The auxiliary pattern (docs/adr/2720): a line type's elements, at most 12, finite, a dash among them.
        || rules.auxiliary_pattern.len() > 12
        || !rules.auxiliary_pattern.iter().all(|l| l.is_finite())
        || (!rules.auxiliary_pattern.is_empty() && !rules.auxiliary_pattern.iter().any(|l| *l > 0.0))
    {
        return Err(fail(
            codes::INVALID_CONTOURS,
            "Eğri kural paketi eksik: kimliği ve kaynağı yazılı, her değeri sonlu ve sıfırdan büyük olmalı.".into(),
            "rules",
        ));
    }
    // Spot heights need the package's text height (Ek-3 0004): without it they are not written (no height invented).
    let summits = input.spots.unwrap_or(false);
    let slopes = input.slopes.as_ref().filter(|s| !s.top.is_empty() || !s.bottom.is_empty());
    let chosen = !input.points.is_empty();
    let spot_height = match (summits || slopes.is_some() || chosen, rules.spot_height) {
        (false, _) => None,
        (true, Some(h)) if positive(h) => Some(h),
        (true, _) => {
            return Err(fail(
                codes::INVALID_CONTOURS,
                format!(
                    "{} paketi kot noktası yazısının yüksekliğini (spotHeight, Ek-3 0004) vermiyor; {} kotları yazılmaz.",
                    rules.id,
                    if summits { "tepe ve çukur" } else if slopes.is_some() { "şev" } else { "seçilen noktaların" }
                ),
                "rules",
            ));
        }
    };
    let scale = match input.scale {
        Some(0) => return Err(fail(codes::INVALID_CONTOURS, "Ölçek paydası 0 olamaz.".into(), "scale")),
        Some(s) => s,
        None => doc.settings().plot_scale.round().max(1.0) as u32,
    };
    let interval = match input.interval {
        Some(i) => i,
        None => rules.intervals.iter().find(|i| i.scale == scale).map(|i| i.interval).ok_or_else(|| {
            fail(
                codes::INVALID_CONTOURS,
                format!("{} paketi 1/{scale} ölçeği için eğri aralığı vermiyor; aralığı yazın.", rules.id),
                "interval",
            )
        })?,
    };
    let base = input.base.unwrap_or(0.0);
    if !(positive(interval) && base.is_finite()) {
        return Err(fail(
            codes::INVALID_CONTOURS,
            "Eş yükselti aralığı 0'dan büyük bir sayı, taban kotu sonlu bir sayı olmalı.".into(),
            "interval",
        ));
    }
    let warnings = checks::layer(doc, &input.layer_id)?;
    for d in &input.details {
        if doc.layers().get(d.trim()).is_none() {
            return Err(fail(codes::LAYER_NOT_FOUND, format!("“{}” katmanı çizimde yok.", d.trim()), "details"));
        }
    }
    if let Some(s) = slopes {
        for (at, layers) in [("slopes.top", &s.top), ("slopes.bottom", &s.bottom)] {
            if let Some(l) = layers.iter().find(|l| doc.layers().get(l.trim()).is_none()) {
                return Err(fail(codes::LAYER_NOT_FOUND, format!("“{}” katmanı çizimde yok.", l.trim()), at));
            }
        }
    }
    if let Some(l) = input.points.iter().find(|l| doc.layers().get(l.trim()).is_none()) {
        return Err(fail(codes::LAYER_NOT_FOUND, format!("“{}” katmanı çizimde yok.", l.trim()), "points"));
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    let own = common::surfaces(doc);
    let name = input.surface.as_str();
    let Some(def) = own.iter().find(|s| s.id == name).or_else(|| own.iter().find(|s| s.name.to_lowercase() == name.to_lowercase())) else {
        return Err(fail(codes::UNKNOWN_TERRAIN, format!("“{name}” kimlikli ya da adlı yüzey projede yok."), "surface"));
    };
    let made = common::make(doc, def);
    let surface = made.built()?;
    let used: Vec<f64> = surface.triangles.iter().flatten().map(|&i| surface.vertices[i as usize][2]).collect();
    let (Some(lo), Some(hi)) = (used.iter().copied().reduce(f64::min), used.iter().copied().reduce(f64::max)) else {
        return Err(no_contours());
    };
    let (first, last) = (((lo - base) / interval).ceil(), ((hi - base) / interval).floor());
    if last - first + 1.0 > MAX_LEVELS as f64 {
        return Err(fail(codes::INVALID_CONTOURS, "Bu aralıkla yüzeyi 10 000'den çok düzey keser; aralığı büyütün.".into(), "interval"));
    }
    // Each triangle's spacing on the sheet, paper millimetres.
    let mm = 1000.0 / f64::from(scale);
    let spacing: Vec<f64> = (0..surface.triangles.len())
        .map(|t| match surface.slope_of(t as u32) {
            Some(s) if s.slope > 0.0 => interval / s.slope * mm,
            _ => f64::INFINITY,
        })
        .collect();
    let (steep_on, aux_on, labels_on) = (input.steep.unwrap_or(true), input.auxiliary.unwrap_or(true), input.labels.unwrap_or(true));
    let steep: Vec<bool> = spacing.iter().map(|s| steep_on && *s < rules.steep_below).collect();
    let gentle: Vec<bool> = spacing.iter().map(|s| aux_on && *s > rules.auxiliary_above).collect();
    // The details: closed polylines' and polygons' rings, chords for arcs.
    let details: Vec<Detail> = doc
        .entities()
        .filter(|e| input.details.iter().any(|d| d.trim() == e.base().layer_id))
        .filter_map(|e| match e {
            Entity::Polygon(p) => Some(p),
            Entity::Polyline(p) if p.closed => Some(p),
            _ => None,
        })
        .map(|p| {
            let mut rings = vec![p.pts.iter().map(|v| [v.x, v.y]).collect::<Vec<P>>()];
            for h in p.holes.iter().flatten() {
                rings.push(h.pts.iter().map(|v| [v.x, v.y]).collect());
            }
            rings.into_iter().filter(|r| r.len() >= 3).collect::<Detail>()
        })
        .filter(|d: &Detail| !d.is_empty())
        .collect();
    let mut lines: Vec<SheetContourLine> = Vec::new();
    let mut cut = 0.0;
    let mut add = |z: f64, kind: &str, found: Vec<(Vec<P>, bool)>, lines: &mut Vec<SheetContourLine>| {
        for (points, closed) in found {
            let (parts, gone, still) = clip(&points, closed, &details);
            cut += gone;
            for part in parts {
                lines.push(SheetContourLine { z, kind: kind.to_owned(), closed: still, points: part });
            }
        }
    };
    let major_every = i64::from(rules.major_every);
    let mut m = 2.0 * first - 1.0;
    while m <= 2.0 * last + 1.0 {
        let half = m / 2.0;
        if (m as i64).rem_euclid(2) == 0 {
            let k = half as i64;
            let z = base + half * interval;
            let major = major_every > 0 && k.rem_euclid(major_every) == 0;
            let found = if major { surface.level_lines(z, &|_| true) } else { surface.level_lines(z, &|t| !steep[t]) };
            add(z, if major { "major" } else { "minor" }, found, &mut lines);
        } else if aux_on {
            let z = base + half * interval;
            add(z, "auxiliary", surface.level_lines(z, &|t| gentle[t]), &mut lines);
        }
        m += 1.0;
    }
    if lines.is_empty() {
        return Err(no_contours());
    }
    let mut spots = if summits { spots_of(surface, &lines, &details) } else { Vec::new() };
    let summit_count = spots.len();
    let (slope_found, slope_off) = slopes.map_or((Vec::new(), 0), |s| slope_spots(doc, surface, s));
    let (tops, bottoms) = (
        slope_found.iter().filter(|s| s.kind == "slopeTop").count(),
        slope_found.iter().filter(|s| s.kind == "slopeBottom").count(),
    );
    spots.extend(slope_found);
    let (chosen_found, chosen_off, measured) = if chosen { chosen_spots(doc, surface, &input.points) } else { (Vec::new(), 0, 0) };
    let chosen_note = chosen.then(|| {
        let mut parts = Vec::new();
        let widest = chosen_found
            .iter()
            .filter_map(|s| s.surface.map(|g| ((s.z - g).abs(), s)))
            .fold(None::<(f64, &SheetSpot)>, |best, (d, s)| if best.is_none_or(|(b, _)| d > b) { Some((d, s)) } else { best });
        if let Some((d, s)) = widest {
            parts.push(format!(" Kendi kotu yüzeyinkinden en çok {d:.3} m ayrılıyor (Y {:.3}, X {:.3}).", s.x, s.y));
        }
        if chosen_off > 0 {
            parts.push(format!(" Kendi kotu olmayan ve yüzeyin dışında kalan {chosen_off} nokta yazılmadı."));
        }
        (chosen_found.len(), measured, parts.concat())
    });
    spots.extend(chosen_found);
    let every = rules.label_every / mm;
    let labels: Vec<SheetContourLabel> = if labels_on {
        lines.iter().filter(|l| l.kind == "major").flat_map(|l| labels(surface, &l.points, l.closed, l.z, every)).collect()
    } else {
        Vec::new()
    };
    let (n_steep, n_gentle) = (steep.iter().filter(|s| **s).count() as u32, gentle.iter().filter(|g| **g).count() as u32);
    let mut notes = vec![format!(
        "Eğriler {} paketine göre: aralık {} m (1/{scale}), her {}. eğri kalın ({} mm), öbürleri {} mm.",
        rules.id, interval, rules.major_every, rules.major_weight, rules.minor_weight
    )];
    if n_steep > 0 {
        notes.push(format!("Eğri arası paftada {} mm'den az olan {n_steep} üçgende yalnız kalın eğriler çizildi.", rules.steep_below));
    }
    if n_gentle > 0 {
        notes.push(format!("Eğri arası {} mm'yi aşan {n_gentle} üçgende yarım aralıkta kesik yardımcı eğri çizildi.", rules.auxiliary_above));
    }
    if labels_on {
        notes.push(format!(
            "Kalın eğrilere her {} mm'de bir yükseklik yazıldı ({} mm); yazının kutusu maskelenir, eğri bölünmez.",
            rules.label_every, rules.label_height
        ));
    }
    if !input.details.is_empty() {
        notes.push(format!("Detayların içinde kalan {cut:.2} m eğri çizilmedi (m. 78(1)(g))."));
    }
    if let (Some(h), true) = (spot_height, summits) {
        notes.push(format!(
            "Tepe ve çukurlarda {summit_count} karakteristik nokta kotu dm'ye kadar yazıldı ({h} mm, m. 78(1)(ğ)): en içteki kapalı eğrinin içindeki en yüksek ya da en alçak köşe{}.",
            if slopes.is_some() { "" } else { "; şev üstü ve dibi işaretlenmedi" }
        ));
    }
    if let (Some(h), Some(_)) = (spot_height, slopes) {
        let mut note = format!(
            "Şevlerin üstünde {tops}, dibinde {bottoms} karakteristik nokta kotu dm'ye kadar yazıldı ({h} mm, m. 78(1)(ğ)): şev çizgilerinin köşeleri, kotları yüzeyden."
        );
        if slope_off > 0 {
            note.push_str(&format!(" Yüzeyin dışındaki {slope_off} köşe yazılmadı."));
        }
        notes.push(note);
    }
    if let (Some(h), Some((n, measured, tail))) = (spot_height, &chosen_note) {
        notes.push(format!(
            "Ölçmecinin seçtiği {n} noktanın kotu dm'ye kadar yazıldı ({h} mm, m. 78(1)(ğ)): kendi kotu olan {measured} nokta kendi kotuyla, öbürleri yüzeyden; yalnız kot yazılır, nokta ölçmecinindir.{tail}"
        ));
    }
    // docs/adr/2720: the auxiliary lines drawn with the package's pattern as a project line type of its own name.
    let pattern = (!rules.auxiliary_pattern.is_empty() && lines.iter().any(|l| l.kind == "auxiliary"))
        .then(|| pattern_name(&rules));
    if let Some(name) = &pattern {
        let parts: Vec<String> = rules
            .auxiliary_pattern
            .iter()
            .map(|l| if *l > 0.0 { format!("{l} mm çizgi") } else if *l < 0.0 { format!("{} mm boşluk", -l) } else { "nokta".to_owned() })
            .collect();
        notes.push(format!(
            "Yardımcı eğriler “{name}” çizgi tipiyle çizilir (kâğıtta {}); tip projede yoksa eklenir, varsa projedeki kullanılır; tip proje ayarıdır, geri almada kalır.",
            parts.join(", ")
        ));
    }
    Ok(Checked {
        out: TerrainsSheetContoured {
            rules: rules.id.clone(),
            source: rules.source.clone(),
            scale,
            interval,
            lines,
            labels,
            steep: n_steep,
            gentle: n_gentle,
            cut,
            spots,
            pattern,
            uids: Vec::new(),
            revision: doc.revision().to_string(),
            notes,
        },
        rules,
        warnings,
    })
}

/// The project line type the auxiliary lines take: the package's pattern under the package's name (docs/adr/2720).
pub fn pattern_name(rules: &ContourRules) -> String {
    format!("Yardımcı eğri ({})", rules.id)
}

/// The package's pattern as a project line type: paper millimetres, the package its source.
fn pattern_type(rules: &ContourRules) -> LineTypeDef {
    LineTypeDef {
        name: pattern_name(rules),
        description: format!("{} paketinin yardımcı eğrisi", rules.id),
        elements: rules.auxiliary_pattern.iter().map(|l| PatternElement { length: *l, text: None, shape: None }).collect(),
        unit: PatternUnit::Paper,
        source: Some(rules.id.clone()),
    }
}

fn no_contours() -> Stop {
    fail(codes::NO_CONTOURS, "Bu aralıkla yüzeyi kesen eş yükselti eğrisi yok; aralığı ya da taban kotunu değiştirin.".into(), "interval")
}

fn objects(c: &Checked) -> Vec<NewObject> {
    let plain = |geometry: EntityGeometry, weight: Option<f64>, dashed: bool| NewObject {
        geometry,
        color: None,
        line_weight: weight,
        line_type: dashed.then_some(LineType::Dashed),
        line_pattern: if dashed { c.out.pattern.clone() } else { None },
        attrs: None,
        label: None,
        label_of: None,
        label_scale: None,
        symbol: None,
        order: None,
        transparency: None,
    };
    let r = &c.rules;
    let mut out: Vec<NewObject> = c
        .out
        .lines
        .iter()
        .map(|l| {
            let weight = match l.kind.as_str() {
                "major" => r.major_weight,
                "auxiliary" => r.auxiliary_weight,
                _ => r.minor_weight,
            };
            plain(
                EntityGeometry::Polyline {
                    pts: l.points.iter().map(|p| Vec2 { x: p[0], y: p[1] }).collect(),
                    bulges: None,
                    zs: Some(vec![Some(l.z); l.points.len()]),
                    ms: None,
                    parts: None,
                    closed: l.closed,
                    widths: None,
                },
                Some(weight),
                l.kind == "auxiliary",
            )
        })
        .collect();
    let height = r.label_height * f64::from(c.out.scale) / 1000.0;
    // The spot heights (docs/adr/2320): a point at the vertex and its height beside it, half the text's height right.
    let spot = r.spot_height.unwrap_or(0.0) * f64::from(c.out.scale) / 1000.0;
    for s in &c.out.spots {
        // A chosen point is the surveyor's (docs/adr/3520): only its height is written.
        if s.kind != "chosen" {
            out.push(plain(EntityGeometry::Point { p: Vec2 { x: s.x, y: s.y }, z: Some(s.z), parts: None }, None, false));
        }
        out.push(plain(
            EntityGeometry::Text {
                layout: None,
                annotative: None,
                p: Vec2 { x: s.x + spot / 2.0, y: s.y },
                text: s.text.clone(),
                height: spot,
                rotation: 0.0,
                align: Some(TextAlign::MiddleLeft),
                width_factor: None,
                mask: true,
                box_width: None,
                line_spacing: None,
                runs: Vec::new(),
                face: Default::default(),
                path: None,
            },
            None,
            false,
        ));
    }
    out.extend(c.out.labels.iter().map(|l| {
        plain(
            EntityGeometry::Text {
                layout: None,
                annotative: None,
                p: Vec2 { x: l.at[0], y: l.at[1] },
                text: l.text.clone(),
                height,
                rotation: l.rotation,
                align: Some(TextAlign::MiddleCenter),
                width_factor: None,
                mask: true,
                box_width: None,
                line_spacing: None,
                runs: Vec::new(),
                face: Default::default(),
                path: None,
            },
            None,
            false,
        )
    }));
    out
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &TerrainsSheetContours) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed { output: (), warnings: c.warnings },
        Err(stop) => stop.into(),
    }
}

/// The lines and labels execute would write; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &TerrainsSheetContours) -> CommandResult<TerrainsSheetContoured> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed { output: c.out, warnings: c.warnings },
        Err(stop) => stop.into(),
    }
}

/// Writes the lines and labels, one undo step “Pafta eş yükselti eğrileri”, recorded as the command.
pub fn execute(cx: &mut ExecutionContext<'_>, input: TerrainsSheetContours) -> CommandResult<TerrainsSheetContoured> {
    crate::call::recorded(
        cx,
        kentos_commands::ROAD_TERRAINS_SHEET_CONTOURS,
        kentos_commands::ROAD_TERRAINS_SHEET_CONTOURS_VERSION,
        input,
        |cx, input| write(cx, input.clone()),
    )
}

fn write(cx: &mut ExecutionContext<'_>, input: TerrainsSheetContours) -> CommandResult<TerrainsSheetContoured> {
    let c = match check(cx.doc, &input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    // The auxiliary pattern's line type, when the project has none of its name (a project setting, kept by undo as every
    // line type is, docs/adr/0313), and the objects as one step.
    let step = cx.doc.begin_group(crate::create::label(Some(CreateOperation::SheetContours)));
    if let Some(name) = &c.out.pattern
        && !cx.doc.settings().line_types.iter().any(|t| kentos_commands::same_line_type_name(&t.name, name))
    {
        let mut settings = cx.doc.settings().clone();
        settings.line_types.push(pattern_type(&c.rules));
        cx.doc.set_settings(settings);
    }
    let create = EntitiesCreate {
        layer_id: input.layer_id.clone(),
        objects: objects(&c),
        operation: Some(CreateOperation::SheetContours),
        expected_revision: None,
    };
    match crate::create::write_in(cx, create) {
        CommandResult::Completed { output, warnings } => {
            cx.doc.end_group(step);
            CommandResult::Completed {
                output: TerrainsSheetContoured { uids: output.created, revision: cx.doc.revision().to_string(), ..c.out },
                warnings,
            }
        }
        other => {
            cx.doc.cancel_group(step);
            crate::trim::relay(other)
        }
    }
}
