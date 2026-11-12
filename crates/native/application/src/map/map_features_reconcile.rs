//! `map.features.reconcile` v1 (plan/map-cad.md MAP-04, docs/adr/1760): candidates of map assets from several sources
//! reconciled through a data profile's classes. A candidate is a point or an open polyline on a source's layer whose
//! class field names a class (its id, or one of its codes as the field's first word's capitals); one that keeps
//! `Eşleştirme` (or the earlier `Uzlaştırma`) was matched before and is not read again. Candidates within the radius (points: the plane's
//! distance; polylines: the larger of the two discrete Hausdorff ways, vertices to the other's segments; docs/adr/3340:
//! an arc's eighths too, to the other's arcs exactly) are one group,
//! joined transitively. A group's representative of a source is its candidate with the highest confidence (none below
//! any number; a tie to the first); the others of that source are duplicates. A group is `existing` when an asset of its
//! one class lies within the radius of a representative, `conflict` when its classes differ, `agreed` with two sources
//! or more, else `single`. Execute writes the agreed groups and the singles a reviewer accepted, whose required fields
//! are all given, on their class's layer, and marks the candidates; one undo step “Detay eşleştirme”. The shared cases
//! are `fixtures/commands/v1/map.features.reconcile.json`.
//!
//! Areas (docs/adr/2140): a closed polygon is a candidate of an area class; two are one group when their overlap ratio,
//! the area of their intersection over that of their union, exceeds `overlap` (the PASCAL VOC criterion, 0.5), by the
//! core's area algebra; an area group's footprint is its most trusted representative's, as a line group's polyline.
//!
//! Field rules (docs/adr/2540): the asset's values, its candidates' and the decisions', pass its class layer's fields as
//! any write to that layer (`check_value`): the canonical text is written; one that does not fit is said in the group's
//! `invalid` and keeps it from being written.
//!
//! Large sets (docs/adr/2940): only the pairs whose boxes meet are tested (a point's or a polyline's widened by the
//! radius, an area's own), through a grid (`box_grid`), and a group's assets there already are looked for among its
//! class layer's objects whose boxes meet its representatives'; the answer is the all-pairs one, the work no longer
//! grows with the square of the candidates.
//!
//! Review notes (docs/adr/4140): each decision that takes (its candidate marked) is recorded as a resolved review note
//! apart from the objects, its author the reviewer, its time the call's `reviewedAt`, bound to the candidate and the
//! asset with their stamps as the step leaves them, its `decision` the decision as given with the source and the
//! asset; in the same undo step. The asset no longer carries `İnceleme` and `Gözden geçiren`.
//!
//! Arcs (docs/adr/3340): a polyline's arcs (its bulges) are curves, not chords. A polyline's points for the discrete
//! Hausdorff are its vertices and, on each arc, the points at its sweep's seven inner eighths; a point's distance to a
//! polyline is the least to its segments and arcs, to an arc exactly (to its circle where the point's direction from the
//! centre lies within the sweep, else to the nearer end). A polyline's box holds its arcs' bulges.

use std::collections::BTreeMap;

use kentos_geometry_core::geom::arrangement::Area;
use kentos_geometry_core::geom::bulge::{BulgeArc, bulge_arc, bulge_at};
use kentos_geometry_core::geom::region::{intersect_area_sets, net_area};
use kentos_geometry_core::ops::areas::areas_of_entity;

use kentos_commands::{
    CandidateDecision, CommandResult, DEFAULT_CODE_FIELD, DataProfile, Entity, FeaturesReconcile, FeaturesReconciled,
    FieldTrouble, MAX_CANDIDATE_SOURCES, MAX_REVIEW_AUTHOR, MAX_REVIEW_LABEL, MAX_REVIEW_NOTES, ProfileGeometry,
    RECONCILED_FIELD, RECONCILED_FIELD_BEFORE, ReappliedDecision, ReconciledCandidate, ReconciledGroup, ReviewDecision,
    ReviewNote, ReviewOrigin, ReviewStatus, Vec2, review_label_ok, review_text_ok,
};
use kentos_domain::{Document, Slot, Uuid};

use crate::ExecutionContext;
use crate::box_grid::{BoxGrid, Bx};
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::profile_files::Shelf;
use crate::profiles::{Place, find, path_words, profile_only};

/// The attributes an asset's provenance is written in; a candidate's are not copied into them.
const PROVENANCE: [&str; 6] = ["Kaynaklar", "Adaylar", "Yayılım", "Örtüşme", "İnceleme", "Gözden geçiren"];

/// The overlap ratio two area candidates must exceed when the input gives none: PASCAL VOC's detection criterion
/// (Everingham et al. 2010, §4.4: “must exceed 0.5”).
pub const DEFAULT_OVERLAP: f64 = 0.5;

/// What a candidate's object is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Point,
    Line,
    Area,
}

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// A candidate read: its object, source, class (profile index), what it said, its confidence, its kind and status.
struct Candidate {
    slot: Slot,
    uid: String,
    source: usize,
    class: Option<usize>,
    read: Option<String>,
    confidence: Option<f64>,
    kind: Kind,
    status: &'static str,
    attrs: BTreeMap<String, String>,
    p: Vec2,
    z: Option<f64>,
    /// A point's place, a polyline's vertices, an area's outer ring closed (its first vertex again at its end).
    pts: Vec<Vec2>,
    /// A polyline's edges' bulges (docs/adr/3340); empty: straight edges.
    bulges: Vec<f64>,
    /// An area's regions and their net area (m²), for the overlap ratio.
    areas: Vec<Area>,
    area: f64,
}

/// A group worked out: its members (indices into the candidates), its representatives, the rest of the answer, and
/// what execute writes for it.
struct Group {
    members: Vec<usize>,
    reps: Vec<usize>,
    row: ReconciledGroup,
    class: Option<usize>,
    attrs: BTreeMap<String, String>,
}

struct Plan {
    candidates: Vec<Candidate>,
    groups: Vec<Group>,
    field: String,
    /// A class's layer by profile index (the classes to write).
    layers: BTreeMap<usize, String>,
    names: Vec<String>,
    reviewer: Option<String>,
    profile: DataProfile,
}

/// The class a field's text names: a class id as written, else the class whose codes hold its first word's capitals.
fn class_named(profile: &DataProfile, text: &str) -> Option<usize> {
    if let Some(i) = profile.classes.iter().position(|c| c.id == text) {
        return Some(i);
    }
    let first = text.split_whitespace().next()?.to_uppercase();
    let letters: String = first.chars().take_while(char::is_ascii_uppercase).collect();
    if letters.is_empty() {
        return None;
    }
    profile.classes.iter().position(|c| c.codes.contains(&letters))
}

fn distance(a: Vec2, b: Vec2) -> f64 {
    (a.x - b.x).hypot(a.y - b.y)
}

/// A point's distance to a segment.
fn to_segment(p: Vec2, a: Vec2, b: Vec2) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let len2 = dx * dx + dy * dy;
    if len2 == 0.0 {
        return distance(p, a);
    }
    let t = (((p.x - a.x) * dx + (p.y - a.y) * dy) / len2).clamp(0.0, 1.0);
    distance(p, Vec2 { x: a.x + t * dx, y: a.y + t * dy })
}

/// A polyline as a curve (docs/adr/3340): its vertices and its edges' bulges (none: straight).
#[derive(Clone, Copy)]
struct Curve<'a> {
    pts: &'a [Vec2],
    bulges: &'a [f64],
}

impl<'a> Curve<'a> {
    fn of(c: &'a Candidate) -> Self {
        Curve { pts: &c.pts, bulges: &c.bulges }
    }

    /// Edge `k`'s arc; none when straight.
    fn arc(&self, k: usize) -> Option<BulgeArc> {
        let at = |p: Vec2| kentos_geometry_core::Vec2::new(p.x, p.y);
        bulge_arc(at(self.pts[k]), at(self.pts[k + 1]), self.bulges.get(k).copied().unwrap_or(0.0))
    }

    /// The points the discrete Hausdorff goes from: the vertices and, on each arc, its sweep's seven inner eighths, in
    /// order along the curve.
    fn samples(&self) -> Vec<Vec2> {
        let mut out = Vec::with_capacity(self.pts.len());
        for k in 0..self.pts.len() {
            out.push(self.pts[k]);
            if k + 1 < self.pts.len()
                && let Some(a) = self.arc(k)
            {
                for e in 1..8 {
                    let angle = a.a0 + a.sweep * f64::from(e) / 8.0;
                    out.push(Vec2 { x: a.c.x + a.r * angle.cos(), y: a.c.y + a.r * angle.sin() });
                }
            }
        }
        out
    }
}

/// Whether `angle` lies within the arc's sweep from its start.
fn within(a: &BulgeArc, angle: f64) -> bool {
    let turn = std::f64::consts::TAU;
    if a.sweep >= 0.0 { (angle - a.a0).rem_euclid(turn) <= a.sweep } else { (a.a0 - angle).rem_euclid(turn) <= -a.sweep }
}

/// A point's distance to an arc from `s` to `e` (docs/adr/3340): to its circle where the point's direction from the
/// centre lies within the sweep, else to the nearer end.
fn to_arc(p: Vec2, a: &BulgeArc, s: Vec2, e: Vec2) -> f64 {
    let angle = (p.y - a.c.y).atan2(p.x - a.c.x);
    let centre = Vec2 { x: a.c.x, y: a.c.y };
    if within(a, angle) { (distance(p, centre) - a.r).abs() } else { distance(p, s).min(distance(p, e)) }
}

/// A point's distance to a curve: the least to its edges (one vertex: the point).
fn to_curve(p: Vec2, b: Curve<'_>) -> f64 {
    if b.pts.len() == 1 {
        return distance(p, b.pts[0]);
    }
    (0..b.pts.len() - 1)
        .map(|k| match b.arc(k) {
            Some(a) => to_arc(p, &a, b.pts[k], b.pts[k + 1]),
            None => to_segment(p, b.pts[k], b.pts[k + 1]),
        })
        .fold(f64::INFINITY, f64::min)
}

/// How far `a`'s points (its vertices and arcs' eighths) go from the curve `b`.
fn one_way(a: Curve<'_>, b: Curve<'_>) -> f64 {
    a.samples().into_iter().map(|p| to_curve(p, b)).fold(0.0f64, f64::max)
}

/// The discrete Hausdorff distance of two polylines: the larger of the two ways.
fn hausdorff(a: Curve<'_>, b: Curve<'_>) -> f64 {
    one_way(a, b).max(one_way(b, a))
}

fn apart(a: &Candidate, b: &Candidate) -> f64 {
    if a.kind == Kind::Point { distance(a.p, b.p) } else { hausdorff(Curve::of(a), Curve::of(b)) }
}

/// A polyline's box with its arcs' bulges (docs/adr/3340): its vertices' and each arc's points due east, north, west
/// and south of its centre that lie within its sweep.
fn curve_bounds(c: Curve<'_>) -> [f64; 4] {
    let mut pts: Vec<Vec2> = c.pts.to_vec();
    for k in 0..c.pts.len().saturating_sub(1) {
        if let Some(a) = c.arc(k) {
            for q in 0..4 {
                let angle = std::f64::consts::FRAC_PI_2 * f64::from(q);
                if within(&a, angle) || within(&a, angle - std::f64::consts::TAU) {
                    pts.push(Vec2 { x: a.c.x + a.r * angle.cos(), y: a.c.y + a.r * angle.sin() });
                }
            }
        }
    }
    bounds(&pts)
}

/// The box round a ring's vertices: west, south, east, north.
fn bounds(pts: &[Vec2]) -> [f64; 4] {
    pts.iter().fold([f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY], |b, p| {
        [b[0].min(p.x), b[1].min(p.y), b[2].max(p.x), b[3].max(p.y)]
    })
}

/// The box a candidate meets the others it may be near within (docs/adr/2940): a point's or a polyline's vertices
/// widened by the radius on every side (two within the radius have meeting boxes, with room to spare), an area's own
/// (two whose overlap ratio is above any number have a common area, so meeting boxes).
fn reach(c: &Candidate, radius: f64) -> Bx {
    let b = curve_bounds(Curve::of(c));
    let r = if c.kind == Kind::Area { 0.0 } else { radius };
    [b[0] - r, b[1] - r, b[2] + r, b[3] + r]
}

/// The pairs of eligible candidates to test, by their indices into the candidates: those whose reaches meet.
fn pairs_to_test(candidates: &[Candidate], eligible: &[usize], radius: f64) -> Vec<(usize, usize)> {
    let grid = BoxGrid::new(eligible.iter().map(|&i| reach(&candidates[i], radius)).collect());
    grid.pairs().into_iter().map(|(a, b)| (eligible[a], eligible[b])).collect()
}

/// An object's box, when it may be an asset there already (a point, a polyline, a polygon).
fn asset_box(e: &Entity) -> Option<Bx> {
    match e {
        Entity::Point(pt) => Some([pt.p.x, pt.p.y, pt.p.x, pt.p.y]),
        Entity::Polyline(pl) if pl.pts.len() >= 2 => {
            let bulges = bulges_of(pl);
            Some(curve_bounds(Curve { pts: &pl.pts, bulges: &bulges }))
        }
        Entity::Polygon(pg) if pg.pts.len() >= 3 => Some(bounds(&pg.pts)),
        _ => None,
    }
}

/// A polyline's edges' bulges, edge by edge (docs/adr/3340).
fn bulges_of(pl: &kentos_commands::PathEntity) -> Vec<f64> {
    (0..pl.pts.len().saturating_sub(1)).map(|k| bulge_at(pl.bulges.as_deref(), k)).collect()
}

/// The overlap ratio of two areas (docs/adr/2140): their intersection's area over their union's, 0 when their boxes
/// are apart.
fn overlap_of(a: (&[Vec2], &[Area], f64), b: (&[Vec2], &[Area], f64)) -> f64 {
    let (p, q) = (bounds(a.0), bounds(b.0));
    if p[0] > q[2] || q[0] > p[2] || p[1] > q[3] || q[1] > p[3] {
        return 0.0;
    }
    let shared = intersect_area_sets(&[a.1.to_vec(), b.1.to_vec()]).iter().map(net_area).fold(0.0, |s, v| s + v);
    let union = a.2 + b.2 - shared;
    if union > 0.0 { shared / union } else { 0.0 }
}

fn overlap(a: &Candidate, b: &Candidate) -> f64 {
    overlap_of((&a.pts, &a.areas, a.area), (&b.pts, &b.areas, b.area))
}

/// A closed ring's centroid by the shoelace formula (its first vertex again at its end), worked from its first vertex
/// so that map coordinates (500 000 m) lose no digits.
fn centroid(ring: &[Vec2]) -> Vec2 {
    let o = ring[0];
    let (mut a, mut cx, mut cy) = (0.0, 0.0, 0.0);
    for w in ring.windows(2) {
        let (p, q) = ((w[0].x - o.x, w[0].y - o.y), (w[1].x - o.x, w[1].y - o.y));
        let cross = p.0 * q.1 - q.0 * p.1;
        a += cross;
        cx += (p.0 + q.0) * cross;
        cy += (p.1 + q.1) * cross;
    }
    if a == 0.0 {
        return o;
    }
    Vec2 { x: o.x + cx / (3.0 * a), y: o.y + cy / (3.0 * a) }
}

/// An area's regions and their net area, by the core's algebra.
fn regions(e: &Entity) -> (Vec<Area>, f64) {
    let areas = areas_of_entity(&crate::geometry::shape(e));
    let total = areas.iter().map(net_area).fold(0.0, |s, v| s + v);
    (areas, total)
}

/// The higher confidence first (none last), then the earlier.
fn better(a: &Candidate, b: &Candidate) -> bool {
    match (a.confidence, b.confidence) {
        (Some(x), Some(y)) => x > y,
        (Some(_), None) => true,
        _ => false,
    }
}

#[allow(clippy::too_many_lines)]
fn check(doc: &Document, input: &FeaturesReconcile) -> Result<Plan, Stop> {
    checks::revision(doc, input.expected_revision.as_deref())?;
    let profile = profile_only(Some(&input.name), None, Shelf::of(doc, input.drawing.as_deref()))?;
    if input.sources.is_empty() || input.sources.len() > MAX_CANDIDATE_SOURCES {
        return Err(fail(
            codes::INVALID_INPUT,
            format!("Aday kaynağı 1 ile {MAX_CANDIDATE_SOURCES} arasında olmalı; adayların katmanlarını verin."),
            "sources",
        ));
    }
    for (i, s) in input.sources.iter().enumerate() {
        let at = format!("sources[{i}]");
        if s.name.trim().is_empty() {
            return Err(fail(codes::INVALID_INPUT, "Kaynağın adı boş olamaz (LiDAR, Ortofoto …).".into(), &format!("{at}.name")));
        }
        // Docs/adr/4140: a review note keeps the name.
        if !review_label_ok(s.name.trim(), MAX_REVIEW_AUTHOR) {
            return Err(fail(
                codes::INVALID_INPUT,
                format!("Kaynağın adı en çok {MAX_REVIEW_AUTHOR} karakter olabilir ve denetim karakteri taşıyamaz."),
                &format!("{at}.name"),
            ));
        }
        if input.sources[..i].iter().any(|o| o.layer == s.layer) {
            return Err(fail(codes::INVALID_INPUT, format!("“{}” katmanı iki kaynakta verilmiş.", s.layer), &format!("{at}.layer")));
        }
        if doc.layers().get(&s.layer).is_none() {
            return Err(fail(codes::LAYER_NOT_FOUND, format!("“{}” kimlikli katman çizimde yok.", s.layer), &format!("{at}.layer")));
        }
    }
    let field = input.field.clone().unwrap_or_else(|| DEFAULT_CODE_FIELD.to_owned());
    if field.trim().is_empty() {
        return Err(fail(codes::INVALID_INPUT, "Sınıf alanının adı boş olamaz.".into(), "field"));
    }
    if input.confidence.as_deref().is_some_and(|c| c.trim().is_empty()) {
        return Err(fail(codes::INVALID_INPUT, "Güven alanının adı boş olamaz.".into(), "confidence"));
    }
    if !(input.radius.is_finite() && input.radius > 0.0) {
        return Err(fail(codes::INVALID_INPUT, "Eşleme yarıçapı 0'dan büyük olmalı (m).".into(), "radius"));
    }
    let overlap_min = input.overlap.unwrap_or(DEFAULT_OVERLAP);
    if !(overlap_min.is_finite() && overlap_min > 0.0 && overlap_min < 1.0) {
        return Err(fail(
            codes::INVALID_INPUT,
            "Örtüşme oranı 0 ile 1 arasında olmalı (kesişimin alanı bölü birleşiminki; verilmezse 0,5).".into(),
            "overlap",
        ));
    }
    // Docs/adr/4140: the review notes' author and time, short labels.
    if let Some(r) = trimmed(input.reviewer.as_deref())
        && !review_label_ok(r, MAX_REVIEW_AUTHOR)
    {
        return Err(fail(
            codes::INVALID_INPUT,
            format!("Gözden geçirenin adı en çok {MAX_REVIEW_AUTHOR} karakter olabilir ve denetim karakteri taşıyamaz."),
            "reviewer",
        ));
    }
    if let Some(t) = trimmed(input.reviewed_at.as_deref())
        && !review_label_ok(t, MAX_REVIEW_LABEL)
    {
        return Err(fail(
            codes::INVALID_INPUT,
            format!("Gözden geçirme zamanı en çok {MAX_REVIEW_LABEL} karakter olabilir (“2026-11-09T16:00:00+03:00”)."),
            "reviewedAt",
        ));
    }
    let source_of = |layer: &str| input.sources.iter().position(|s| s.layer == layer);
    // The decisions, each on a candidate of a source.
    let mut decided: BTreeMap<String, &CandidateDecision> = BTreeMap::new();
    for (i, d) in input.decisions.iter().enumerate() {
        let at = format!("decisions[{i}]");
        if !checks::is_uid_text(&d.candidate) {
            return Err(fail(codes::INVALID_UID, format!("“{}” geçerli bir nesne kimliği değil.", d.candidate), &format!("{at}.candidate")));
        }
        let Some(e) = Uuid::parse_str(&d.candidate).ok().and_then(|u| doc.slot_of(u)).and_then(|s| doc.get(s)) else {
            return Err(fail(codes::ENTITY_NOT_FOUND, format!("“{}” kimlikli nesne çizimde yok.", d.candidate), &format!("{at}.candidate")));
        };
        if source_of(&e.base().layer_id).is_none() {
            return Err(fail(
                codes::INVALID_INPUT,
                format!("“{}” bir aday değil: kaynakların katmanlarında değil.", d.candidate),
                &format!("{at}.candidate"),
            ));
        }
        if let Some(c) = &d.class
            && !profile.classes.iter().any(|k| k.id == *c)
        {
            return Err(fail(codes::INVALID_INPUT, format!("“{c}” “{}” profilinin bir sınıfı değil.", profile.name), &format!("{at}.class")));
        }
        if d.accept && d.reject.is_some() {
            return Err(fail(codes::INVALID_INPUT, "Bir aday hem kabul hem ret edilemez.".into(), &format!("{at}.reject")));
        }
        if d.reject.as_deref().is_some_and(|r| r.trim().is_empty()) {
            return Err(fail(codes::INVALID_INPUT, "Ret nedeni boş olamaz.".into(), &format!("{at}.reject")));
        }
        // Docs/adr/4140: what a review note holds.
        if d.reject.as_deref().is_some_and(|r| !review_text_ok(r.trim())) {
            return Err(fail(
                codes::INVALID_INPUT,
                "Ret nedeni en çok 4000 karakter olabilir ve satır sonu ile sekme dışında denetim karakteri taşıyamaz.".into(),
                &format!("{at}.reject"),
            ));
        }
        // A blank value is left out, as from the asset.
        for (k, v) in &d.fields {
            if !review_label_ok(k, MAX_REVIEW_LABEL) || (!v.trim().is_empty() && !review_text_ok(v.trim())) {
                return Err(fail(
                    codes::INVALID_INPUT,
                    format!("“{k}” alanının adı ya da değeri uzun ya da denetim karakterli: inceleme notuna yazılamaz."),
                    &format!("{at}.fields"),
                ));
            }
        }
        decided.insert(d.candidate.clone(), d);
    }
    // The candidates, in the drawing's order.
    let mut candidates: Vec<Candidate> = Vec::new();
    for (uid, e) in doc.entities_with_uids() {
        let Some(source) = source_of(&e.base().layer_id) else { continue };
        let (kind, p, z, mut pts, bulges) = match e {
            Entity::Point(pt) => (Kind::Point, pt.p, pt.z, vec![pt.p], Vec::new()),
            Entity::Polyline(pl) if !pl.closed && pl.pts.len() >= 2 => {
                (Kind::Line, pl.pts[0], None, pl.pts.clone(), bulges_of(pl))
            }
            Entity::Polygon(pg) if pg.pts.len() >= 3 => (Kind::Area, pg.pts[0], None, pg.pts.clone(), Vec::new()),
            _ => continue,
        };
        let (areas, area) = if kind == Kind::Area { regions(e) } else { (Vec::new(), 0.0) };
        if kind == Kind::Area {
            if !(area > 0.0) {
                continue;
            }
            pts.push(pts[0]);
        }
        let Some(slot) = doc.slot_of(uid) else { continue };
        let attrs = e.base().attrs.clone();
        let uid = uid.to_string();
        let read = attrs.get(&field).map(|t| t.trim().to_owned()).filter(|t| !t.is_empty());
        let decision = decided.get(&uid).copied();
        let class = decision
            .and_then(|d| d.class.as_ref())
            .and_then(|c| profile.classes.iter().position(|k| k.id == *c))
            .or_else(|| read.as_deref().and_then(|t| class_named(&profile, t)));
        let confidence = input
            .confidence
            .as_ref()
            .and_then(|f| attrs.get(f))
            .and_then(|t| t.trim().parse::<f64>().ok())
            .filter(|v| v.is_finite());
        let marked = |key: &str| attrs.get(key).is_some_and(|t| !t.trim().is_empty());
        let status = if marked(RECONCILED_FIELD) || marked(RECONCILED_FIELD_BEFORE) {
            "reviewed"
        } else if decision.is_some_and(|d| d.reject.is_some()) {
            "rejected"
        } else {
            match class.map(|c| profile.classes[c].geometry) {
                None => "unknown",
                Some(ProfileGeometry::Point) if kind == Kind::Point => "member",
                Some(ProfileGeometry::Line) if kind == Kind::Line => "member",
                Some(ProfileGeometry::Area) if kind == Kind::Area => "member",
                Some(_) => "wrongKind",
            }
        };
        candidates.push(Candidate { slot, uid, source, class, read, confidence, kind, status, attrs, p, z, pts, bulges, areas, area });
    }
    // Groups: joined within the radius, the same kind.
    let eligible: Vec<usize> = (0..candidates.len()).filter(|&i| candidates[i].status == "member").collect();
    let mut parent: Vec<usize> = (0..candidates.len()).collect();
    fn root(parent: &mut [usize], i: usize) -> usize {
        let mut r = i;
        while parent[r] != r {
            r = parent[r];
        }
        let mut k = i;
        while parent[k] != r {
            let next = parent[k];
            parent[k] = r;
            k = next;
        }
        r
    }
    let near = |a: &Candidate, b: &Candidate| match a.kind {
        Kind::Area => overlap(a, b) > overlap_min,
        _ => apart(a, b) <= input.radius,
    };
    // Only the pairs whose boxes meet (docs/adr/2940); a set's root is its smallest index whatever the pairs' order.
    for (i, j) in pairs_to_test(&candidates, &eligible, input.radius) {
        if candidates[i].kind == candidates[j].kind && near(&candidates[i], &candidates[j]) {
            let (ri, rj) = (root(&mut parent, i), root(&mut parent, j));
            if ri != rj {
                let (lo, hi) = if ri < rj { (ri, rj) } else { (rj, ri) };
                parent[hi] = lo;
            }
        }
    }
    let mut order: Vec<usize> = Vec::new();
    let mut members_of: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for &i in &eligible {
        let r = root(&mut parent, i);
        if !members_of.contains_key(&r) {
            order.push(r);
        }
        members_of.entry(r).or_default().push(i);
    }
    // The assets there already: a class's layer's objects of its kind.
    let mut layers: BTreeMap<usize, String> = BTreeMap::new();
    let layer_of = |ci: usize| -> Option<String> {
        match find(doc.layers().nodes(), &profile.classes[ci].layer) {
            Place::Layer(id) => Some(id),
            _ => None,
        }
    };
    let names: Vec<String> = input.sources.iter().map(|s| s.name.clone()).collect();
    let candidate_uids: std::collections::HashSet<String> = candidates.iter().map(|c| c.uid.clone()).collect();
    // Each class layer's objects that may be assets there already, in the drawing's order, and their grid, made once
    // (docs/adr/2940).
    let mut assets: BTreeMap<String, (Vec<(String, &Entity)>, BoxGrid)> = BTreeMap::new();
    let mut groups: Vec<Group> = Vec::new();
    for r in order {
        let members = members_of.remove(&r).unwrap_or_default();
        let mut reps: Vec<usize> = Vec::new();
        for &m in &members {
            match reps.iter().position(|&x| candidates[x].source == candidates[m].source) {
                Some(k) => {
                    if better(&candidates[m], &candidates[reps[k]]) {
                        reps[k] = m;
                    }
                }
                None => reps.push(m),
            }
        }
        let mut classes: Vec<usize> = members.iter().filter_map(|&m| candidates[m].class).collect();
        classes.sort_unstable();
        classes.dedup();
        let kind = candidates[members[0]].kind;
        let mut sources: Vec<usize> = reps.iter().map(|&x| candidates[x].source).collect();
        sources.sort_unstable();
        // An area group's smallest overlap ratio between its representatives.
        let mut smallest: Option<f64> = None;
        if kind == Kind::Area {
            for (a, &i) in reps.iter().enumerate() {
                for &j in &reps[a + 1..] {
                    let o = overlap(&candidates[i], &candidates[j]);
                    smallest = Some(smallest.map_or(o, |s: f64| s.min(o)));
                }
            }
        }
        // The place and the spread.
        let (x, y, z, spread) = if kind != Kind::Point {
            let mut chosen = reps[0];
            for &k in &reps[1..] {
                let (a, b) = (&candidates[k], &candidates[chosen]);
                if better(a, b) || (a.confidence == b.confidence && a.source < b.source) {
                    chosen = k;
                }
            }
            let mut most = 0.0f64;
            for (a, &i) in reps.iter().enumerate() {
                for &j in &reps[a + 1..] {
                    most = most.max(hausdorff(Curve::of(&candidates[i]), Curve::of(&candidates[j])));
                }
            }
            let c = &candidates[chosen];
            reps.retain(|&k| k != chosen);
            reps.insert(0, chosen);
            // A line group's place is its polyline's first vertex; an area group's its footprint's centroid.
            let at = if kind == Kind::Area { centroid(&c.pts) } else { c.pts[0] };
            (at.x, at.y, None, most)
        } else {
            let n = reps.len() as f64;
            let (mut sx, mut sy) = (0.0, 0.0);
            for &k in &reps {
                sx += candidates[k].p.x;
                sy += candidates[k].p.y;
            }
            let mean = Vec2 { x: sx / n, y: sy / n };
            let z = if reps.iter().all(|&k| candidates[k].z.is_some()) {
                let mut sz = 0.0;
                for &k in &reps {
                    sz += candidates[k].z.unwrap_or(0.0);
                }
                Some(sz / n)
            } else {
                None
            };
            let spread = reps.iter().map(|&k| distance(candidates[k].p, mean)).fold(0.0, f64::max);
            (mean.x, mean.y, z, spread)
        };
        // An asset of its one class there already.
        let mut existing: Option<String> = None;
        if classes.len() == 1
            && let Some(layer) = layer_of(classes[0])
        {
            let (list, grid) = assets.entry(layer.clone()).or_insert_with(|| {
                let list: Vec<(String, &Entity, Bx)> = doc
                    .entities_with_uids()
                    .filter(|(uid, e)| e.base().layer_id == layer && !candidate_uids.contains(&uid.to_string()))
                    .filter_map(|(uid, e)| asset_box(e).map(|b| (uid.to_string(), e, b)))
                    .collect();
                let grid = BoxGrid::new(list.iter().map(|x| x.2).collect());
                (list.into_iter().map(|(u, e, _)| (u, e)).collect(), grid)
            });
            // Its representatives' reach: an asset near one of them has a box that meets it.
            let q = reps.iter().map(|&k| reach(&candidates[k], input.radius)).fold(
                [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY],
                |a, b| [a[0].min(b[0]), a[1].min(b[1]), a[2].max(b[2]), a[3].max(b[3])],
            );
            for at in grid.meeting(&q) {
                let (uid, e) = (&list[at].0, list[at].1);
                let near = match (e, kind) {
                    (Entity::Point(pt), Kind::Point) => reps.iter().any(|&k| distance(candidates[k].p, pt.p) <= input.radius),
                    (Entity::Polyline(pl), Kind::Line) if pl.pts.len() >= 2 => {
                        let bulges = bulges_of(pl);
                        let there = Curve { pts: &pl.pts, bulges: &bulges };
                        reps.iter().any(|&k| hausdorff(Curve::of(&candidates[k]), there) <= input.radius)
                    }
                    (Entity::Polygon(pg), Kind::Area) if pg.pts.len() >= 3 => {
                        let (areas, area) = regions(e);
                        let mut ring = pg.pts.clone();
                        ring.push(pg.pts[0]);
                        area > 0.0
                            && reps.iter().any(|&k| {
                                let c = &candidates[k];
                                overlap_of((&c.pts, &c.areas, c.area), (&ring, &areas, area)) > overlap_min
                            })
                    }
                    _ => false,
                };
                if near {
                    existing = Some(uid.clone());
                    break;
                }
            }
        }
        let status = if existing.is_some() {
            "existing"
        } else if classes.len() > 1 {
            "conflict"
        } else if sources.len() >= 2 {
            "agreed"
        } else {
            "single"
        };
        // The asset's fields: the representatives' in order of confidence (then of source), the decisions' over them.
        let mut by_trust = reps.clone();
        by_trust.sort_by(|&a, &b| {
            let (ca, cb) = (&candidates[a], &candidates[b]);
            if better(ca, cb) {
                std::cmp::Ordering::Less
            } else if better(cb, ca) {
                std::cmp::Ordering::Greater
            } else {
                ca.source.cmp(&cb.source)
            }
        });
        let mut attrs: BTreeMap<String, String> = BTreeMap::new();
        for &k in &by_trust {
            for (key, value) in &candidates[k].attrs {
                let skip = *key == field
                    || input.confidence.as_ref() == Some(key)
                    || key == RECONCILED_FIELD
                    || key == RECONCILED_FIELD_BEFORE
                    || PROVENANCE.contains(&key.as_str())
                    || value.trim().is_empty();
                if !skip && !attrs.contains_key(key) {
                    attrs.insert(key.clone(), value.clone());
                }
            }
        }
        let mut accepted = false;
        for &m in &members {
            if let Some(d) = decided.get(&candidates[m].uid) {
                for (k, v) in &d.fields {
                    if !v.trim().is_empty() {
                        attrs.insert(k.clone(), v.clone());
                    }
                }
                if d.accept {
                    accepted = true;
                }
            }
        }
        let class = (classes.len() == 1).then(|| classes[0]);
        let missing: Vec<String> = class
            .map(|c| {
                profile.classes[c]
                    .fields
                    .iter()
                    .filter(|f| f.required)
                    .filter(|f| attrs.get(&f.name).is_none_or(|v| v.trim().is_empty()))
                    .map(|f| f.name.clone())
                    .collect()
            })
            .unwrap_or_default();
        // docs/adr/2540: the asset's values through its class layer's field rules (kind, length, list, range), as any
        // write to that layer: the canonical text is written; a value that does not fit keeps the group back.
        let mut invalid: Vec<FieldTrouble> = Vec::new();
        if let Some(c) = class
            && let Some(layer) = layer_of(c)
            && let Some(node) = doc.layers().get(&layer)
        {
            for f in &node.fields {
                let Some(value) = attrs.get(&f.name).cloned() else { continue };
                if value.trim().is_empty() {
                    continue;
                }
                match kentos_commands::check_value(f, &value) {
                    Ok(canonical) => {
                        attrs.insert(f.name.clone(), canonical);
                    }
                    Err(why) => invalid.push(FieldTrouble { field: f.name.clone(), message: why.message }),
                }
            }
        }
        let writes =
            (status == "agreed" || (status == "single" && accepted)) && missing.is_empty() && invalid.is_empty();
        if writes && let Some(c) = class {
            match layer_of(c) {
                Some(layer) => {
                    layers.insert(c, layer);
                }
                None => {
                    let k = &profile.classes[c];
                    return Err(fail(
                        codes::CLASS_LAYER_MISSING,
                        format!("“{}” sınıfının katmanı ({}) çizimde yok; önce “{}” profilini uygulayın.", k.title, path_words(&k.layer), profile.name),
                        "name",
                    ));
                }
            }
        }
        let row = ReconciledGroup {
            class: class.map(|c| profile.classes[c].id.clone()),
            classes: classes.iter().map(|&c| profile.classes[c].id.clone()).collect(),
            status: status.into(),
            sources: sources.iter().map(|&s| names[s].clone()).collect(),
            candidates: members.iter().map(|&m| candidates[m].uid.clone()).collect(),
            x,
            y,
            z,
            spread,
            overlap: smallest,
            asset: existing,
            missing,
            invalid,
            writes,
        };
        groups.push(Group { members, reps, row, class, attrs });
    }
    for (g, group) in groups.iter().enumerate() {
        for &m in &group.members {
            if !group.reps.contains(&m) {
                candidates[m].status = "duplicate";
            }
        }
        let _ = g;
    }
    Ok(Plan { candidates, groups, field, layers, names, reviewer: trimmed(input.reviewer.as_deref()).map(str::to_owned), profile })
}

/// A text trimmed, none when blank.
fn trimmed(text: Option<&str>) -> Option<&str> {
    text.map(str::trim).filter(|t| !t.is_empty())
}

/// A decision's fields that say something (a blank value is left out, as from the asset), trimmed.
fn given_fields(d: &CandidateDecision) -> BTreeMap<String, String> {
    d.fields.iter().filter(|(_, v)| !v.trim().is_empty()).map(|(k, v)| (k.clone(), v.trim().to_owned())).collect()
}

/// What a decision did, in a review note's words (docs/adr/4140): long fields and reasons are left to its decision.
fn decision_words(plan: &Plan, d: &CandidateDecision, source: &str) -> String {
    let mut parts = Vec::new();
    if d.accept {
        parts.push("kabul edildi".to_owned());
    }
    if let Some(c) = &d.class {
        let title = plan.profile.classes.iter().find(|k| k.id == *c).map_or(c.as_str(), |k| k.title.as_str());
        parts.push(format!("sınıfı “{title}” olarak düzeltildi"));
    }
    let fields = given_fields(d);
    if !fields.is_empty() {
        let given: Vec<String> = fields.iter().map(|(k, v)| format!("{k} = {v}")).collect();
        let given = given.join(", ");
        parts.push(if given.chars().count() <= 200 {
            format!("alanları verildi ({given})")
        } else {
            format!("{} alanı verildi", fields.len())
        });
    }
    if let Some(r) = &d.reject {
        let r = r.trim();
        parts.push(if r.chars().count() <= 200 { format!("reddedildi ({r})") } else { "reddedildi (nedeni kararda)".to_owned() });
    }
    format!("Detay eşleştirme, {source} adayı: {}.", parts.join("; "))
}

/// Whether a decision says anything a note records.
fn says(d: &CandidateDecision) -> bool {
    d.accept || d.class.is_some() || !given_fields(d).is_empty() || d.reject.is_some()
}

fn answer(plan: &Plan, written: Vec<String>, marked: u32, notes: Vec<u32>, revision: String) -> FeaturesReconciled {
    let mut group_of: BTreeMap<usize, u32> = BTreeMap::new();
    for (g, group) in plan.groups.iter().enumerate() {
        for &m in &group.members {
            group_of.insert(m, u32::try_from(g).unwrap_or(u32::MAX));
        }
    }
    FeaturesReconciled {
        groups: plan.groups.iter().map(|g| g.row.clone()).collect(),
        candidates: plan
            .candidates
            .iter()
            .enumerate()
            .map(|(i, c)| ReconciledCandidate {
                uid: c.uid.clone(),
                source: plan.names[c.source].clone(),
                class: c.class.map(|k| plan.profile.classes[k].id.clone()),
                read: c.read.clone(),
                confidence: c.confidence,
                group: group_of.get(&i).copied(),
                status: c.status.into(),
            })
            .collect(),
        written,
        marked,
        notes,
        reapplied: Vec::new(),
        revision,
    }
}

/// Docs/adr/4540: the decisions recorded on earlier candidates that this run takes again. For each candidate of this
/// run without a decision of its own (not reviewed, none given), the resolved review notes of this command of its
/// source (by name) whose candidate is still in the drawing (reviewed, on the same source's layer, of the same kind) and
/// near it as two candidates are one group (within the radius; areas: their overlap above the least); the nearest
/// (areas: the largest overlap), then the note's number; a note naming a class the profile no longer has is left. The
/// input with them added as decisions, and which candidate took which note.
fn with_reapplied(doc: &Document, input: &FeaturesReconcile) -> Result<(FeaturesReconcile, Vec<ReappliedDecision>), Stop> {
    if !input.reapply {
        return Ok((input.clone(), Vec::new()));
    }
    let plan = check(doc, input)?;
    let least = input.overlap.unwrap_or(DEFAULT_OVERLAP);
    let given: std::collections::BTreeSet<&str> = input.decisions.iter().map(|d| d.candidate.as_str()).collect();
    let recorded: Vec<(u32, &ReviewDecision)> = doc
        .settings()
        .review_notes
        .iter()
        .filter(|n| n.status == ReviewStatus::Resolved)
        .filter_map(|n| n.decision.as_ref().map(|d| (n.number, d)))
        .filter(|(_, d)| d.command == kentos_commands::MAP_FEATURES_RECONCILE)
        .filter(|(_, d)| d.class.as_ref().is_none_or(|k| plan.profile.classes.iter().any(|c| c.id == *k)))
        .collect();
    let mut out = input.clone();
    let mut taken = Vec::new();
    for c in plan.candidates.iter().filter(|c| c.status != "reviewed" && !given.contains(c.uid.as_str())) {
        let source = plan.names[c.source].trim();
        let mut best: Option<(f64, u32, &ReviewDecision)> = None;
        for &(number, d) in &recorded {
            if d.source.trim() != source {
                continue;
            }
            let Some(old) =
                plan.candidates.iter().find(|o| o.uid == d.candidate && o.status == "reviewed" && o.source == c.source && o.kind == c.kind)
            else {
                continue;
            };
            let score = match c.kind {
                Kind::Area => {
                    let r = overlap(old, c);
                    if r > least { -r } else { continue }
                }
                _ => {
                    let a = apart(old, c);
                    if a <= input.radius { a } else { continue }
                }
            };
            if best.is_none_or(|(s, n, _)| score < s || (score == s && number < n)) {
                best = Some((score, number, d));
            }
        }
        if let Some((_, number, d)) = best {
            out.decisions.push(CandidateDecision {
                candidate: c.uid.clone(),
                class: d.class.clone(),
                accept: d.accept,
                reject: d.reject.clone(),
                fields: d.fields.clone(),
            });
            taken.push(ReappliedDecision { candidate: c.uid.clone(), note: number });
        }
    }
    Ok((out, taken))
}

fn write(cx: &mut ExecutionContext<'_>, input: FeaturesReconcile) -> CommandResult<FeaturesReconciled> {
    // Docs/adr/4540: the recorded decisions taken again, first.
    let (input, reapplied) = match with_reapplied(cx.doc, &input) {
        Ok(x) => x,
        Err(stop) => return stop.into(),
    };
    let mut plan = match check(cx.doc, &input) {
        Ok(p) => p,
        Err(stop) => return stop.into(),
    };
    // The layers written on: a class's, and the sources' whose candidates are marked.
    let locked = |doc: &Document, layer: &str, words: &str, path: &str| -> Option<Stop> {
        doc.layers().is_locked(layer).then(|| {
            fail(codes::LAYER_LOCKED, format!("“{}” katmanı kilitli: {words} Kilidi açın.", doc.layers().path(layer)), path)
        })
    };
    for layer in plan.layers.values() {
        if let Some(stop) = locked(cx.doc, layer, "eşleştirilen varlık oraya yazılamaz.", "name") {
            return stop.into();
        }
    }
    let marks_any = plan
        .groups
        .iter()
        .any(|g| g.row.writes || g.row.status == "existing")
        || plan.candidates.iter().any(|c| c.status == "rejected");
    if marks_any {
        for s in &input.sources {
            let used = plan.candidates.iter().any(|c| plan.names[c.source] == s.name && matches!(c.status, "member" | "duplicate" | "rejected"));
            if used && let Some(stop) = locked(cx.doc, &s.layer, "adaylar işaretlenemez.", "sources") {
                return stop.into();
            }
        }
    }
    let label = "Detay eşleştirme";
    let mut written: Vec<String> = Vec::new();
    let mut marked = 0u32;
    let mut notes: Vec<u32> = Vec::new();
    if !marks_any {
        let mut output = answer(&plan, written, marked, Vec::new(), cx.doc.revision().to_string());
        output.reapplied = reapplied;
        return CommandResult::Completed { output, warnings: Vec::new() };
    }
    // Docs/adr/4140: the decisions that take (their candidates marked), each a review note; room for them first.
    let marked_now: std::collections::BTreeSet<usize> = plan
        .groups
        .iter()
        .filter(|g| g.row.writes || (g.row.status == "existing" && g.row.asset.is_some()))
        .flat_map(|g| g.members.iter().copied())
        .chain(plan.candidates.iter().enumerate().filter(|(_, c)| c.status == "rejected").map(|(i, _)| i))
        .collect();
    let taking: Vec<(usize, &CandidateDecision)> = input
        .decisions
        .iter()
        .filter(|d| says(d))
        .filter_map(|d| plan.candidates.iter().position(|c| c.uid == d.candidate).map(|i| (i, d)))
        .filter(|(i, d)| marked_now.contains(i) && (d.reject.is_none() || plan.candidates[*i].status == "rejected"))
        .collect();
    if cx.doc.settings().review_notes.len() + taking.len() > MAX_REVIEW_NOTES {
        return fail(
            codes::TOO_MANY_REVIEW_NOTES,
            format!("Çizim {MAX_REVIEW_NOTES} inceleme notunu dolduracak; kararlar kaydedilemez. Çözülmüş notları silin."),
            "decisions",
        )
        .into();
    }
    let field = plan.field.clone();
    let reviewer = plan.reviewer.clone();
    let _ = cx.doc.transact(label, |doc| -> Result<(), ()> {
        let mut marks: Vec<(Slot, String)> = Vec::new();
        for g in &mut plan.groups {
            if g.row.writes {
                let (Some(c), Some(first)) = (g.class, g.reps.first().copied()) else { continue };
                let Some(layer) = plan.layers.get(&c).cloned() else { continue };
                let class = &plan.profile.classes[c];
                let mut attrs = g.attrs.clone();
                if let Some(code) = class.codes.first() {
                    attrs.insert(field.clone(), code.clone());
                }
                attrs.insert("Kaynaklar".into(), g.row.sources.join(", "));
                attrs.insert("Adaylar".into(), g.row.candidates.join(", "));
                attrs.insert("Yayılım".into(), format!("{:.3}", g.row.spread));
                if let Some(o) = g.row.overlap {
                    attrs.insert("Örtüşme".into(), format!("{o:.3}"));
                }
                let Some(mut asset) = doc.get(plan.candidates[first].slot).cloned() else { continue };
                if let Entity::Point(pt) = &mut asset {
                    pt.p = Vec2 { x: g.row.x, y: g.row.y };
                    pt.z = g.row.z;
                }
                {
                    let base = asset.base_mut();
                    base.layer_id = layer;
                    base.attrs = attrs;
                }
                let Ok(slots) = doc.add_many(vec![asset], label) else { return Err(()) };
                let Some(uid) = slots.first().and_then(|s| doc.uid(*s)).map(|u| u.to_string()) else { return Err(()) };
                for &m in &g.members {
                    marks.push((plan.candidates[m].slot, uid.clone()));
                }
                g.row.asset = Some(uid.clone());
                written.push(uid);
            } else if g.row.status == "existing"
                && let Some(asset) = g.row.asset.clone()
            {
                for &m in &g.members {
                    marks.push((plan.candidates[m].slot, asset.clone()));
                }
            }
        }
        for d in &input.decisions {
            if let Some(reason) = &d.reject
                && let Some(c) = plan.candidates.iter().find(|c| c.uid == d.candidate && c.status == "rejected")
            {
                marks.push((c.slot, format!("Reddedildi: {}", reason.trim())));
            }
        }
        let updates: Vec<(Slot, Entity)> = marks
            .iter()
            .filter_map(|(slot, text)| {
                let mut e = doc.get(*slot)?.clone();
                e.base_mut().attrs.insert(RECONCILED_FIELD.into(), text.clone());
                Some((*slot, e))
            })
            .collect();
        marked = u32::try_from(updates.len()).unwrap_or(u32::MAX);
        if !updates.is_empty() {
            doc.update_many(updates, label);
        }
        // Docs/adr/4140: each decision that took, a resolved review note bound to its candidate and its asset as the
        // step leaves them.
        let mut book = doc.settings().review_notes.clone();
        for (i, d) in &taking {
            let c = &plan.candidates[*i];
            let Some((_, mark)) = marks.iter().find(|(slot, _)| *slot == c.slot) else { continue };
            let asset = d.reject.is_none().then(|| mark.clone());
            let mut uids = vec![c.uid.clone()];
            uids.extend(asset.clone());
            let anchors: Vec<_> = uids.iter().filter_map(|u| crate::review_notes::anchor_now(doc, u)).collect();
            let source = plan.names[c.source].trim().to_owned();
            let number = book.iter().map(|n| n.number).max().unwrap_or(0) + 1;
            book.push(ReviewNote {
                id: Uuid::now_v7().to_string(),
                number,
                text: match reapplied.iter().find(|r| r.candidate == d.candidate) {
                    // Docs/adr/4540: the note it came from.
                    Some(r) => format!("{} Önceki karar (not {}) yeniden uygulandı.", decision_words(&plan, d, &source), r.note),
                    None => decision_words(&plan, d, &source),
                },
                status: ReviewStatus::Resolved,
                origin: ReviewOrigin::User,
                author: reviewer.clone(),
                created_at: trimmed(input.reviewed_at.as_deref()).map(str::to_owned),
                revision: None,
                at: None,
                sheet: None,
                view: None,
                anchors,
                replies: Vec::new(),
                decision: Some(ReviewDecision {
                    command: kentos_commands::MAP_FEATURES_RECONCILE.into(),
                    candidate: c.uid.clone(),
                    source,
                    accept: d.accept,
                    class: d.class.clone(),
                    fields: given_fields(d),
                    reject: d.reject.as_deref().map(|r| r.trim().to_owned()),
                    asset,
                }),
            });
            notes.push(number);
        }
        if !notes.is_empty() {
            doc.set_review_notes(book, label);
        }
        Ok(())
    });
    let mut output = answer(&plan, written, marked, notes, cx.doc.revision().to_string());
    output.reapplied = reapplied;
    CommandResult::Completed { output, warnings: Vec::new() }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &FeaturesReconcile) -> CommandResult<()> {
    match with_reapplied(cx.doc, input).and_then(|(input, _)| check(cx.doc, &input)) {
        Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// What execute would write; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &FeaturesReconcile) -> CommandResult<FeaturesReconciled> {
    match with_reapplied(cx.doc, input).and_then(|(input, reapplied)| check(cx.doc, &input).map(|p| (p, reapplied))) {
        Ok((p, reapplied)) => {
            let mut output = answer(&p, Vec::new(), 0, Vec::new(), cx.doc.revision().to_string());
            output.reapplied = reapplied;
            CommandResult::Completed { output, warnings: Vec::new() }
        }
        Err(stop) => stop.into(),
    }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: FeaturesReconcile) -> CommandResult<FeaturesReconciled> {
    crate::call::recorded(
        cx,
        kentos_commands::MAP_FEATURES_RECONCILE,
        kentos_commands::MAP_FEATURES_RECONCILE_VERSION,
        input,
        write,
    )
}
