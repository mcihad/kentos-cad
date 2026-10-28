//! `cad.arch.audit` v1 (read-only, docs/adr/0703): the drawing's architectural problems in one list, each with what
//! resolves it and, where a command plainly does, its call. Walls are checked level by level (walls of different
//! levels never join, docs/adr/0639): the joins' issues are `cad.walls.joins`'s; a free end that stops short of another
//! wall's body, or of another free end, is said. The other groups read the followers' stored `broken` fields, the
//! spaces' rings and the numbers. Given profiles, the ramps and the stairs are measured against them (docs/adr/0723 §4,
//! 0724 §2).
//! It writes nothing. The shared cases are `fixtures/commands/v1/cad.arch.audit.json` and
//! `cad.arch.audit.accessibility.json` (`scripts/fixtures/arch_audit_cases.py`, `ramp_access_cases.py`).

use std::collections::{BTreeMap, HashSet};

use kentos_commands::{
    ARCH_AUDIT_MAX_GAP, ARCH_AUDIT_MIN_AREA, ARCH_AUDIT_MIN_WIDTH, ArchAudit, ArchAuditCheck, ArchAuditFinding,
    ArchAuditFix, ArchAudited, ArchLevel, ChangesetStep, CommandResult, Entity, Vec2,
};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::Vec2 as V;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::arrangement::winding;
use kentos_geometry_core::geom::bulge::{bulge_path_edges, segment_tangent};
use kentos_geometry_core::geom::intersect::{Edge, closest_on_edge, ray_edge};
use kentos_geometry_core::geom::wall::{self as wall_geom, JoinMode, Spec};
use kentos_geometry_core::geom::arch;
use kentos_geometry_core::geom::wall_join::{End, JOIN_TOLERANCE, WallIn, solve};
use kentos_geometry_core::display::fixed;
use serde_json::json;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;

/// Each group's codes, in the order its findings come (docs/adr/0703 §2).
const CODES: [(ArchAuditCheck, &[&str]); 12] = [
    (
        ArchAuditCheck::Walls,
        &[
            codes::WALL_JOIN_OVERLAP,
            codes::WALL_JOIN_CROWDED,
            codes::WALL_JOIN_ACUTE,
            codes::WALL_JOIN_UNRESOLVED,
            codes::ARCH_WALL_END_GAP,
            codes::ARCH_WALL_ENDS_APART,
        ],
    ),
    (ArchAuditCheck::Openings, &[codes::ARCH_OPENING_BROKEN, codes::TAG_MISSING_TYPE]),
    (
        ArchAuditCheck::Spaces,
        &[codes::ARCH_SPACE_BROKEN, codes::ARCH_SPACE_SMALL, codes::ARCH_SPACE_NARROW],
    ),
    (ArchAuditCheck::Numbers, &[codes::TAG_DUPLICATE_NUMBER]),
    (ArchAuditCheck::Tags, &[codes::TAG_TARGET_GONE]),
    (
        ArchAuditCheck::Dimensions,
        &[codes::ARCH_DIMENSION_SET_BROKEN, codes::ARCH_DIMENSION_BROKEN],
    ),
    (ArchAuditCheck::Grids, &[codes::ARCH_GRID_LABEL_DUPLICATE]),
    (ArchAuditCheck::Columns, &[codes::ARCH_COLUMN_GRID_GONE]),
    (ArchAuditCheck::Viewmarks, &[codes::ARCH_VIEWMARK_TARGET_GONE]),
    (ArchAuditCheck::Levels, &[codes::ARCH_LEVEL_MISSING]),
    (
        ArchAuditCheck::Accessibility,
        &[codes::RAMP_SLOPE_OVER_PROFILE, codes::RAMP_LONG_RUN_SLOPE, codes::RAMP_LANDING_SHORT, codes::RAMP_TURN_NARROW],
    ),
    (
        ArchAuditCheck::Stairs,
        &[
            codes::STAIR_RISER_OVER_PROFILE,
            codes::STAIR_TREAD_SHORT,
            codes::STAIR_STEP_OVER_PROFILE,
            codes::STAIR_WIDTH_NARROW,
            codes::STAIR_LANDING_SHORT,
            codes::STAIR_WINDER_NARROW,
            codes::STAIR_RAILING_MISSING,
        ],
    ),
];

/// The thresholds, checked: finite, then at least zero.
struct Limits {
    max_gap: f64,
    min_area: f64,
    min_width: f64,
}

fn limits(input: &ArchAudit) -> Result<Limits, Stop> {
    let given = [
        (input.max_gap, "maxGap", "Boşluk eşiği", "m"),
        (input.min_area, "minArea", "Küçük mahal eşiği", "m²"),
        (input.min_width, "minWidth", "Dar mahal eşiği", "m"),
    ];
    for (v, path, what, _) in given {
        if let Some(v) = v {
            checks::finite(v, what, "Sonlu bir sayı verin.", path)?;
        }
    }
    for (v, path, what, unit) in given {
        if let Some(v) = v.filter(|v| *v < 0.0) {
            return Err(Stop::failed(error(
                codes::INVALID_ARCH_AUDIT,
                format!("{what} {v} {unit}; sıfır ya da daha büyük olmalı."),
                Some(path.into()),
            )));
        }
    }
    Ok(Limits {
        max_gap: input.max_gap.unwrap_or(ARCH_AUDIT_MAX_GAP),
        min_area: input.min_area.unwrap_or(ARCH_AUDIT_MIN_AREA),
        min_width: input.min_width.unwrap_or(ARCH_AUDIT_MIN_WIDTH),
    })
}

fn wire(p: V) -> Vec2 {
    Vec2 { x: p.x, y: p.y }
}

fn core(p: Vec2) -> V {
    V::new(p.x, p.y)
}

/// The first sentence of a follower's words (with its full stop), and the rest.
fn split(words: &str) -> (String, String) {
    match words.split_once(". ") {
        Some((what, rest)) => (format!("{what}."), rest.to_owned()),
        None => (words.to_owned(), String::new()),
    }
}

/// A fix: its words and its call.
fn fix(label: impl Into<String>, command: &str, input: serde_json::Value) -> ArchAuditFix {
    ArchAuditFix {
        label: label.into(),
        step: ChangesetStep {
            command: command.into(),
            version: None,
            input,
            name: None,
        },
    }
}

fn finding(
    check: ArchAuditCheck,
    code: &str,
    message: String,
    repair: impl Into<String>,
    uids: Vec<String>,
    at: V,
    fix: Option<ArchAuditFix>,
) -> ArchAuditFinding {
    ArchAuditFinding {
        check,
        code: code.into(),
        message,
        repair: repair.into(),
        uids,
        at: wire(at),
        fix,
    }
}

struct Audit<'a> {
    doc: &'a Document,
}

impl Audit<'_> {
    fn uid(&self, slot: Slot) -> String {
        self.doc.uid(slot).map(|u| u.to_string()).unwrap_or_default()
    }

    fn uid_of(&self, e: &Entity) -> String {
        self.uid(Slot(e.base().id))
    }

    fn locked(&self, e: &Entity) -> bool {
        self.doc.layers().is_locked(&e.base().layer_id)
    }
}

// ── Walls ───────────────────────────────────────────────────────────────────────────────────────────────────────

/// A wall as the joins solve it: its slot, path and body.
struct WallAt<'a> {
    entity: &'a Entity,
    pts: Vec<V>,
    bulges: Option<Vec<f64>>,
    spec: Spec,
    level: Option<String>,
}

/// A wall's body outline: its two faces, an open wall's ends capped square (none when it cannot be drawn).
struct Body {
    edges: Vec<Edge>,
    /// A closed wall's two face rings (the body between them); an open wall's one ring.
    rings: Vec<Vec<Edge>>,
}

fn reversed(e: &Edge) -> Edge {
    match *e {
        Edge::Seg { a, b } => Edge::Seg { a: b, b: a },
        Edge::Arc { c, r, a0, sweep } => Edge::Arc {
            c,
            r,
            a0: a0 + sweep,
            sweep: -sweep,
        },
    }
}

fn body_of(w: &WallAt<'_>) -> Option<Body> {
    let lines = wall_geom::lines(&w.pts, w.bulges.as_deref(), &w.spec).ok()?;
    let (left, right) = (lines.first()?, lines.last()?);
    let bulges = |l: &wall_geom::Line| (!l.bulges.is_empty()).then(|| l.bulges.clone());
    let l_edges = bulge_path_edges(&left.pts, bulges(left).as_deref(), w.spec.closed);
    let r_edges = bulge_path_edges(&right.pts, bulges(right).as_deref(), w.spec.closed);
    if w.spec.closed {
        let edges = l_edges.iter().chain(&r_edges).copied().collect();
        return Some(Body {
            edges,
            rings: vec![l_edges, r_edges],
        });
    }
    let (l0, l1) = (*left.pts.first()?, *left.pts.last()?);
    let (r0, r1) = (*right.pts.first()?, *right.pts.last()?);
    let mut ring = l_edges;
    ring.push(Edge::Seg { a: l1, b: r1 });
    ring.extend(r_edges.iter().rev().map(reversed));
    ring.push(Edge::Seg { a: r0, b: l0 });
    Some(Body {
        edges: ring.clone(),
        rings: vec![ring],
    })
}

impl Body {
    /// Whether `p` is inside or on it (within the joins' tolerance of its outline).
    fn holds(&self, p: V) -> bool {
        if self.edges.iter().any(|e| closest_on_edge(e, p).d <= JOIN_TOLERANCE) {
            return true;
        }
        let inside = |ring: &[Edge]| winding(ring, p).abs() > std::f64::consts::PI;
        match self.rings.as_slice() {
            [a, b] => inside(a) != inside(b),
            [a] => inside(a),
            _ => false,
        }
    }

    /// The nearest distance along a ray from `p` along the unit `dir` to its outline.
    fn hit(&self, p: V, dir: V) -> Option<f64> {
        self.edges
            .iter()
            .flat_map(|e| ray_edge(p, dir, e, 0.0))
            .filter(|t| *t > 0.0)
            .min_by(f64::total_cmp)
    }
}

/// A wall end: its wall, which end, where, its direction out of the wall.
#[derive(Clone, Copy)]
struct EndAt {
    wall: usize,
    end: End,
    at: V,
    out: V,
}

fn ends_of(walls: &[WallAt<'_>]) -> Vec<EndAt> {
    let mut out = Vec::new();
    for (i, w) in walls.iter().enumerate() {
        let n = w.pts.len();
        if w.spec.closed || n < 2 {
            continue;
        }
        let b = |k: usize| w.bulges.as_ref().and_then(|bs| bs.get(k).copied()).unwrap_or(0.0);
        let start = segment_tangent(w.pts[0], w.pts[1], b(0), false);
        let end = segment_tangent(w.pts[n - 2], w.pts[n - 1], b(n - 2), true);
        out.push(EndAt {
            wall: i,
            end: End::Start,
            at: w.pts[0],
            out: V::new(-start.x, -start.y),
        });
        out.push(EndAt {
            wall: i,
            end: End::End,
            at: w.pts[n - 1],
            out: end,
        });
    }
    out
}

fn end_words(end: End) -> (&'static str, &'static str) {
    match end {
        End::Start => ("Başlangıç", "başlangıç"),
        End::End => ("Bitiş", "bitiş"),
    }
}

fn mode_label(m: JoinMode) -> &'static str {
    kentos_formats::walls::join_back(m).label()
}

impl Audit<'_> {
    fn walls(&self, limits: &Limits) -> Vec<ArchAuditFinding> {
        let walls: Vec<WallAt<'_>> = self
            .doc
            .entities()
            .filter_map(|e| {
                let Entity::Wall(w) = e else { return None };
                let Shape::Wall { pts, bulges, wall } = crate::geometry::shape(e) else {
                    return None;
                };
                Some(WallAt {
                    entity: e,
                    pts,
                    bulges,
                    spec: wall,
                    level: w.wall.arch.level.clone(),
                })
            })
            .collect();
        let ins: Vec<WallIn<'_>> = walls
            .iter()
            .map(|w| WallIn {
                pts: &w.pts,
                bulges: w.bulges.as_deref(),
                spec: &w.spec,
            })
            .collect();
        let solved = solve(&ins, Some(&[]));
        let uid = |k: usize| self.uid_of(walls[k].entity);
        let mut out = Vec::new();
        for issue in &solved.issues {
            let (what, repair) = issue.code.message();
            let fix = issue.fix.and_then(|f| {
                let w = &walls[f.wall];
                if self.locked(w.entity) {
                    return None;
                }
                let (word, _) = end_words(f.end);
                let key = match f.end {
                    End::Start => "start",
                    End::End => "end",
                };
                let mode = kentos_formats::walls::join_back(f.mode);
                Some(fix(
                    format!("{word} ucunu {} yap", mode_label(f.mode)),
                    "cad.walls.edit",
                    json!({"uids": [uid(f.wall)], "operation": {"kind": "joins", key: mode}}),
                ))
            });
            out.push(finding(
                ArchAuditCheck::Walls,
                issue.code.code(),
                what.into(),
                repair,
                issue.walls.iter().map(|&k| uid(k)).collect(),
                issue.at,
                fix,
            ));
        }
        let joined: HashSet<(usize, bool)> = solved
            .joints
            .iter()
            .flat_map(|j| j.members.iter())
            .filter_map(|m| m.end.map(|e| (m.wall, e == End::Start)))
            .collect();
        let bodies: Vec<Option<Body>> = walls.iter().map(body_of).collect();
        let ends = ends_of(&walls);
        let same_level = |a: usize, b: usize| walls[a].level == walls[b].level;
        let free: Vec<EndAt> = ends
            .iter()
            .copied()
            .filter(|e| {
                let w = &walls[e.wall];
                let k = usize::from(e.end == End::End);
                w.spec.joins[k] != JoinMode::None
                    && !joined.contains(&(e.wall, e.end == End::Start))
                    && !ends.iter().any(|o| {
                        (o.wall, o.end) != (e.wall, e.end)
                            && same_level(o.wall, e.wall)
                            && (o.at.x - e.at.x).hypot(o.at.y - e.at.y) <= JOIN_TOLERANCE
                    })
                    && !bodies.iter().enumerate().any(|(j, b)| {
                        j != e.wall && same_level(j, e.wall) && b.as_ref().is_some_and(|b| b.holds(e.at))
                    })
            })
            .collect();
        let mut gapped = HashSet::new();
        for e in &free {
            let best = bodies
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != e.wall && same_level(*j, e.wall))
                .filter_map(|(j, b)| b.as_ref()?.hit(e.at, e.out).map(|t| (t, j)))
                .min_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
            let Some((gap, j)) = best.filter(|(gap, _)| *gap <= limits.max_gap) else {
                continue;
            };
            gapped.insert((e.wall, e.end == End::Start));
            let other = &walls[j];
            let crosses = bulge_path_edges(&other.pts, other.bulges.as_deref(), other.spec.closed)
                .iter()
                .any(|edge| !ray_edge(e.at, e.out, edge, 0.0).is_empty());
            let (word, lower) = end_words(e.end);
            let key = match e.end {
                End::Start => "start",
                End::End => "end",
            };
            let fix = (crosses && !self.locked(walls[e.wall].entity)).then(|| {
                fix(
                    format!("{word} ucunu öbür duvara uzat"),
                    "cad.walls.edit",
                    json!({"uids": [uid(e.wall)], "operation": {"kind": "extend", "to": uid(j), "end": key}}),
                )
            });
            out.push(finding(
                ArchAuditCheck::Walls,
                codes::ARCH_WALL_END_GAP,
                format!("Duvarın {lower} ucuyla öbür duvar arasında {gap:.3} m boşluk var; birleşmiyor."),
                "Ucu öbür duvara uzatın; mahal ancak duvarlar birleşince kapanır.",
                vec![uid(e.wall), uid(j)],
                e.at,
                fix,
            ));
        }
        let rest: Vec<&EndAt> = free
            .iter()
            .filter(|e| !gapped.contains(&(e.wall, e.end == End::Start)))
            .collect();
        for (a, p) in rest.iter().enumerate() {
            for q in &rest[a + 1..] {
                if !same_level(p.wall, q.wall) {
                    continue;
                }
                let dist = (q.at.x - p.at.x).hypot(q.at.y - p.at.y);
                if !(dist > JOIN_TOLERANCE && dist <= limits.max_gap) {
                    continue;
                }
                let mid = V::new((p.at.x + q.at.x) / 2.0, (p.at.y + q.at.y) / 2.0);
                let (message, uids) = if p.wall == q.wall {
                    (
                        format!("Duvarın iki ucu arasında {dist:.3} m boşluk var; duvar kapanmıyor."),
                        vec![uid(p.wall)],
                    )
                } else {
                    let (x, y) = (p.wall.min(q.wall), p.wall.max(q.wall));
                    (
                        format!("İki duvarın uçları arasında {dist:.3} m boşluk var; birleşmiyor."),
                        vec![uid(x), uid(y)],
                    )
                };
                out.push(finding(
                    ArchAuditCheck::Walls,
                    codes::ARCH_WALL_ENDS_APART,
                    message,
                    "Bir ucu tutamaçla öbür uca taşıyın (Uç kenetiyle); uçlar buluşunca duvarlar birleşir.",
                    uids,
                    mid,
                    None,
                ));
            }
        }
        out
    }
}

// ── Openings, spaces, numbers, tags ─────────────────────────────────────────────────────────────────────────────

/// An opening's point: its place's points' mean.
/// A grid's middle: its line's, or its arc's.
fn grid_middle(g: &kentos_commands::GridFields) -> V {
    match g.pts.as_slice() {
        [a, b] => kentos_geometry_core::geom::bulge::segment_mid(
            core(*a),
            core(*b),
            g.bulges.as_ref().and_then(|x| x.first().copied()).unwrap_or(0.0),
        ),
        _ => V::new(0.0, 0.0),
    }
}

/// Whether grid `g` lies on grid `h`: the same ends, the other way round with its arc turned over too, within a
/// nanometre.
fn lies_on(g: &kentos_commands::GridFields, h: &kentos_commands::GridFields) -> bool {
    let near = |p: Vec2, q: Vec2| (p.x - q.x).abs() <= 1e-9 && (p.y - q.y).abs() <= 1e-9;
    let bulge = |f: &kentos_commands::GridFields| f.bulges.as_ref().and_then(|x| x.first().copied()).unwrap_or(0.0);
    let ([a, b], [c, d]) = (g.pts.as_slice(), h.pts.as_slice()) else {
        return false;
    };
    (near(*a, *c) && near(*b, *d) && (bulge(g) - bulge(h)).abs() <= 1e-12)
        || (near(*a, *d) && near(*b, *c) && (bulge(g) + bulge(h)).abs() <= 1e-12)
}

fn opening_point(place: &[Vec2]) -> V {
    if place.is_empty() {
        return V::new(0.0, 0.0);
    }
    let n = place.len() as f64;
    V::new(
        place.iter().map(|p| p.x).sum::<f64>() / n,
        place.iter().map(|p| p.y).sum::<f64>() / n,
    )
}

/// A space's point: its first part's outer ring's centroid by its vertices (their mean when it has no area).
fn space_point(parts: &[kentos_commands::SpacePart]) -> V {
    let Some(ring) = parts.first().map(|p| &p.outer.pts).filter(|r| !r.is_empty()) else {
        return V::new(0.0, 0.0);
    };
    let n = ring.len();
    let (mut a, mut cx, mut cy) = (0.0, 0.0, 0.0);
    for i in 0..n {
        let (p, q) = (ring[i], ring[(i + 1) % n]);
        let k = p.x * q.y - q.x * p.y;
        a += k;
        cx += (p.x + q.x) * k;
        cy += (p.y + q.y) * k;
    }
    if a == 0.0 {
        let m = n as f64;
        return V::new(
            ring.iter().map(|p| p.x).sum::<f64>() / m,
            ring.iter().map(|p| p.y).sum::<f64>() / m,
        );
    }
    V::new(cx / (3.0 * a), cy / (3.0 * a))
}

fn space_label(s: &kentos_commands::SpaceFields) -> String {
    let words: Vec<&str> = [s.number.as_deref(), s.name.as_deref()]
        .into_iter()
        .flatten()
        .filter(|w| !w.is_empty())
        .collect();
    if words.is_empty() {
        "Adsız mahal".into()
    } else {
        words.join(" ")
    }
}

const SMALL_REPAIR: &str = "İki duvar arasında kalan bir boşluk mahal sayılmış olabilir; gereksizse mahalı silin.";

impl Audit<'_> {
    fn openings(&self) -> Vec<ArchAuditFinding> {
        let mut out = Vec::new();
        for e in self.doc.entities() {
            let Entity::Opening(o) = e else { continue };
            let o = &o.opening;
            let n = o.number.as_deref().filter(|n| !n.is_empty());
            let at = opening_point(&o.place);
            if let Some(b) = o.broken {
                let (mut what, repair) = split(b.words());
                if let Some(n) = n {
                    what = format!("{} ({n}).", what.trim_end_matches('.'));
                }
                out.push(finding(
                    ArchAuditCheck::Openings,
                    codes::ARCH_OPENING_BROKEN,
                    what,
                    repair,
                    vec![self.uid_of(e)],
                    at,
                    None,
                ));
            }
            if o.style.as_deref().is_none_or(str::is_empty) {
                let message = match n {
                    Some(n) => format!("“{n}” açıklığının türü yok."),
                    None => "Numarasız açıklığın türü yok.".into(),
                };
                out.push(finding(
                    ArchAuditCheck::Openings,
                    codes::TAG_MISSING_TYPE,
                    message,
                    "Öznitelikler'de açıklığın Tür'ünü seçin.",
                    vec![self.uid_of(e)],
                    at,
                    None,
                ));
            }
        }
        out
    }

    fn spaces(&self, limits: &Limits) -> Vec<ArchAuditFinding> {
        let mut out = Vec::new();
        for e in self.doc.entities() {
            let Entity::Space(s) = e else { continue };
            let s = &s.space;
            let label = space_label(s);
            let at = space_point(&s.parts);
            if let Some(b) = s.broken {
                let (what, repair) = split(b.words());
                out.push(finding(
                    ArchAuditCheck::Spaces,
                    codes::ARCH_SPACE_BROKEN,
                    format!("{label}: {what}"),
                    repair,
                    vec![self.uid_of(e)],
                    at,
                    None,
                ));
            }
            // A void (a shaft, docs/adr/0708) is small and narrow by design: no leftover-piece finding.
            if s.void {
                continue;
            }
            let area = crate::spaces::area_of(&s.parts);
            let perimeter = crate::spaces::perimeter_of(&s.parts);
            if area < limits.min_area {
                out.push(finding(
                    ArchAuditCheck::Spaces,
                    codes::ARCH_SPACE_SMALL,
                    format!("{label}: alanı {area:.3} m²; çok küçük bir parça."),
                    SMALL_REPAIR,
                    vec![self.uid_of(e)],
                    at,
                    None,
                ));
            } else if perimeter > 0.0 && 2.0 * area / perimeter < limits.min_width {
                out.push(finding(
                    ArchAuditCheck::Spaces,
                    codes::ARCH_SPACE_NARROW,
                    format!(
                        "{label}: ortalama eni {:.3} m; çok dar bir parça.",
                        2.0 * area / perimeter
                    ),
                    SMALL_REPAIR,
                    vec![self.uid_of(e)],
                    at,
                    None,
                ));
            }
        }
        out
    }

    fn numbers(&self) -> Vec<ArchAuditFinding> {
        // The tags check's grouping (docs/adr/0344 §2): openings and spaces apart, in first appearance order.
        let mut order: Vec<(bool, String)> = Vec::new();
        let mut groups: BTreeMap<(bool, String), Vec<&Entity>> = BTreeMap::new();
        for e in self.doc.entities() {
            let key = match e {
                Entity::Opening(o) => o.opening.number.as_ref().map(|n| (true, n.clone())),
                Entity::Space(s) => s.space.number.as_ref().map(|n| (false, n.clone())),
                _ => None,
            };
            let Some(key) = key.filter(|(_, n)| !n.is_empty()) else { continue };
            if !groups.contains_key(&key) {
                order.push(key.clone());
            }
            groups.entry(key).or_default().push(e);
        }
        let mut out = Vec::new();
        for key in order {
            let list = &groups[&key];
            if list.len() < 2 {
                continue;
            }
            let at = match list[0] {
                Entity::Opening(o) => opening_point(&o.opening.place),
                Entity::Space(s) => space_point(&s.space.parts),
                _ => V::new(0.0, 0.0),
            };
            let what = if key.0 { "açıklıkta" } else { "mahalde" };
            out.push(finding(
                ArchAuditCheck::Numbers,
                codes::TAG_DUPLICATE_NUMBER,
                format!("“{}” numarası {} {what}.", key.1, list.len()),
                "Numaralardan birini değiştirin (Öznitelikler ya da Numarala).",
                list.iter().map(|e| self.uid_of(e)).collect(),
                at,
                None,
            ));
        }
        out
    }

    fn tags(&self) -> Vec<ArchAuditFinding> {
        let mut out = Vec::new();
        for e in self.doc.entities() {
            let Entity::Tag(t) = e else { continue };
            if self.doc.tag_target(&t.tag.of).is_some() {
                continue;
            }
            let message = if t.tag.text.is_empty() {
                "Etiketin nesnesi çizimde yok.".to_owned()
            } else {
                format!("“{}” etiketinin nesnesi çizimde yok.", t.tag.text.replace('\n', " "))
            };
            let fix = (!self.locked(e)).then(|| {
                fix(
                    "Etiketi sil",
                    "cad.entities.delete",
                    json!({"uids": [self.uid_of(e)]}),
                )
            });
            out.push(finding(
                ArchAuditCheck::Tags,
                codes::TAG_TARGET_GONE,
                message,
                "Nesneyi geri getirin ya da etiketi silin.",
                vec![self.uid_of(e)],
                core(t.tag.at),
                fix,
            ));
        }
        out
    }
}

// ── Dimensions, columns, view marks, levels ─────────────────────────────────────────────────────────────────────

fn set_words(b: kentos_commands::DimensionSetBreak) -> &'static str {
    use kentos_commands::DimensionSetBreak as B;
    match b {
        B::HostGone => "Ölçü dizisinin duvarı çizimde yok; ölçüler son yerlerinde duruyor.",
        B::UnsupportedWall => {
            "Ölçü dizisinin duvar kenarı artık ölçülemiyor (pahlı köşe, yay ya da olmayan kenar); ölçüler son yerlerinde duruyor."
        }
        B::OpeningUnplaced => "Ölçü dizisinin bir açıklığı duvarına yerleşmiyor; ölçüler son yerlerinde duruyor.",
    }
}

impl Audit<'_> {
    fn dimensions(&self) -> Vec<ArchAuditFinding> {
        let mut out = Vec::new();
        let mut members: HashSet<String> = HashSet::new();
        for set in self.doc.dimension_sets() {
            let ms: Vec<String> = set
                .members
                .iter()
                .map(|m| m.uid.to_string())
                .filter(|u| {
                    kentos_domain::Uuid::parse_str(u)
                        .ok()
                        .and_then(|u| self.doc.slot_of(u))
                        .is_some()
                })
                .collect();
            members.extend(ms.iter().cloned());
            let Some(b) = set.broken else { continue };
            let first = ms
                .first()
                .and_then(|u| kentos_domain::Uuid::parse_str(u).ok())
                .and_then(|u| self.doc.slot_of(u))
                .and_then(|s| self.doc.get(s));
            let at = match first {
                Some(Entity::Dimension(d)) => core(d.a),
                _ => core(set.at),
            };
            let fix = (!self.doc.layers().is_locked(&set.layer_id)).then(|| {
                fix(
                    "Diziyi bağımsız ölçülere ayır",
                    "cad.dimensionsets.edit",
                    json!({"id": set.id, "operation": {"kind": "detach"}}),
                )
            });
            out.push(finding(
                ArchAuditCheck::Dimensions,
                codes::ARCH_DIMENSION_SET_BROKEN,
                set_words(b).into(),
                "Duvarı ya da açıklığı düzeltin; dizi kendiliğinden güncellenir.",
                ms,
                at,
                fix,
            ));
        }
        for e in self.doc.entities() {
            let Entity::Dimension(d) = e else { continue };
            let uid = self.uid_of(e);
            if members.contains(&uid) {
                continue;
            }
            let Some(refs) = d.refs.as_deref() else { continue };
            let broken: Vec<&str> = [("a", &refs.a), ("b", &refs.b), ("c", &refs.c)]
                .into_iter()
                .filter(|(_, r)| r.as_ref().is_some_and(|r| r.broken.is_some()))
                .map(|(k, _)| k)
                .collect();
            if broken.is_empty() {
                continue;
            }
            let fix = (!self.locked(e)).then(|| {
                let points: Vec<serde_json::Value> =
                    broken.iter().map(|k| json!({"point": k, "to": null})).collect();
                fix(
                    "Kopan noktaları serbest bırak",
                    "cad.dimensions.reassociate",
                    json!({"uid": uid, "points": points}),
                )
            });
            out.push(finding(
                ArchAuditCheck::Dimensions,
                codes::ARCH_DIMENSION_BROKEN,
                format!(
                    "İlişkili ölçünün bağı koptu ({}); noktaları son yerlerinde duruyor.",
                    broken.join(", ")
                ),
                "Yeniden bağla ile noktaları bir nesneye bağlayın ya da serbest bırakın.",
                vec![uid],
                core(d.a),
                fix,
            ));
        }
        out
    }

    /// Grids sharing a label on one level (docs/adr/0725 §6): each one after the first said with the first; its fix
    /// deletes it when it lies on the first and no column stands on it, else gives it the next label no grid of the
    /// level has and no fix before gave.
    fn grids(&self) -> Vec<ArchAuditFinding> {
        let grids: Vec<(&Entity, &kentos_commands::GridFields)> = self
            .doc
            .entities()
            .filter_map(|e| match e {
                Entity::Grid(g) => Some((e, &g.grid)),
                _ => None,
            })
            .collect();
        let tied: HashSet<String> = self
            .doc
            .entities()
            .filter_map(|e| match e {
                Entity::Column(c) => c.column.grids.clone(),
                _ => None,
            })
            .flatten()
            .collect();
        let mut taken: HashSet<(Option<String>, String)> = grids
            .iter()
            .map(|(_, g)| (g.arch.level.clone(), g.label.clone()))
            .collect();
        let mut order: Vec<(Option<String>, String)> = Vec::new();
        let mut groups: BTreeMap<(Option<String>, String), Vec<usize>> = BTreeMap::new();
        for (i, (_, g)) in grids.iter().enumerate() {
            let key = (g.arch.level.clone(), g.label.clone());
            if !groups.contains_key(&key) {
                order.push(key.clone());
            }
            groups.entry(key).or_default().push(i);
        }
        let mut out = Vec::new();
        for key in order {
            let list = &groups[&key];
            let (first_e, first) = grids[list[0]];
            for &k in &list[1..] {
                let (e, g) = grids[k];
                let uid = self.uid_of(e);
                let fix = (!self.locked(e)).then(|| {
                    if lies_on(g, first) && !tied.contains(&uid) {
                        fix(
                            "Üst üste binen aksı sil",
                            "cad.entities.delete",
                            json!({"uids": [uid]}),
                        )
                    } else {
                        let mut label = kentos_commands::next_label(&g.label);
                        while taken.contains(&(key.0.clone(), label.clone())) {
                            label = kentos_commands::next_label(&label);
                        }
                        taken.insert((key.0.clone(), label.clone()));
                        fix(
                            format!("Aksı “{label}” olarak adlandır"),
                            "cad.arch.edit",
                            json!({"uids": [uid], "operation": {"kind": "set", "label": label}}),
                        )
                    }
                });
                out.push(finding(
                    ArchAuditCheck::Grids,
                    codes::ARCH_GRID_LABEL_DUPLICATE,
                    format!("“{}” etiketi bu katta birden çok aksta.", g.label),
                    "Akslardan birini yeniden adlandırın (Öznitelikler ya da Aksları numarala); üst üste binen fazla aksı silin.",
                    vec![self.uid_of(first_e), uid],
                    grid_middle(g),
                    fix,
                ));
            }
        }
        out
    }

    fn columns(&self) -> Vec<ArchAuditFinding> {
        let mut out = Vec::new();
        for e in self.doc.entities() {
            let Entity::Column(c) = e else { continue };
            let Some(b) = c.column.broken else { continue };
            let (what, repair) = split(b.words());
            let fix = (!self.locked(e)).then(|| {
                fix(
                    "Kolonu akslardan ayır",
                    "cad.arch.edit",
                    json!({"uids": [self.uid_of(e)], "operation": {"kind": "detach"}}),
                )
            });
            out.push(finding(
                ArchAuditCheck::Columns,
                codes::ARCH_COLUMN_GRID_GONE,
                what,
                repair,
                vec![self.uid_of(e)],
                core(c.column.at),
                fix,
            ));
        }
        out
    }

    fn viewmarks(&self) -> Vec<ArchAuditFinding> {
        let mut out = Vec::new();
        for e in self.doc.entities() {
            let Entity::ViewMark(v) = e else { continue };
            let Some(b) = v.mark.broken else { continue };
            let (what, repair) = split(b.words());
            let message = if v.mark.mark.is_empty() {
                what
            } else {
                format!("“{}” işareti: {what}", v.mark.mark)
            };
            let fix = (!self.locked(e)).then(|| {
                fix(
                    "İşareti hedefinden ayır",
                    "cad.viewmarks.edit",
                    json!({"uids": [self.uid_of(e)], "operation": {"kind": "detach"}}),
                )
            });
            let at = v.mark.points.first().copied().map_or(V::new(0.0, 0.0), core);
            out.push(finding(
                ArchAuditCheck::Viewmarks,
                codes::ARCH_VIEWMARK_TARGET_GONE,
                message,
                repair,
                vec![self.uid_of(e)],
                at,
                fix,
            ));
        }
        out
    }

    /// Objects without a level (docs/adr/0717 §3): a wall, a space, a column, a stair, a ramp or a room separator on no
    /// level or on one the project lacks; each fixed by giving it `level` (`cad.levels.assign`) unless locked.
    fn levels(&self, level: Option<&ArchLevel>) -> Vec<ArchAuditFinding> {
        let levels = self
            .doc
            .settings()
            .arch
            .as_ref()
            .map(|a| a.levels.as_slice())
            .unwrap_or_default();
        let mut out = Vec::new();
        for e in self.doc.entities() {
            if !crate::levels_assign::lacks_level(e, levels) {
                continue;
            }
            let first = |pts: &[Vec2]| pts.first().copied().map_or(V::new(0.0, 0.0), core);
            let (who, at) = match e {
                Entity::Wall(w) => ("Duvarın", first(&w.wall.pts)),
                Entity::Space(s) => ("Mahalın", space_point(&s.space.parts)),
                Entity::Column(c) => ("Kolonun", core(c.column.at)),
                Entity::Stair(s) => ("Merdivenin", first(&s.stair.pts)),
                Entity::Ramp(r) => ("Rampanın", first(&r.ramp.pts)),
                Entity::Polyline(p) => ("Mahal ayırıcının", first(&p.pts)),
                _ => continue,
            };
            let message = match e.own_level() {
                Some(id) => format!("{who} katı “{id}” projede yok."),
                None => format!("{who} katı yok; projede {} kat var.", levels.len()),
            };
            let fix = level.filter(|_| !self.locked(e)).map(|l| {
                fix(
                    format!("Katını {} yap", l.name),
                    "cad.levels.assign",
                    json!({"level": l.id, "uids": [self.uid_of(e)]}),
                )
            });
            out.push(finding(
                ArchAuditCheck::Levels,
                codes::ARCH_LEVEL_MISSING,
                message,
                "Düzelt ya da Kata ver ile katını verin; Öznitelikler'de Kat (L) de seçilir.",
                vec![self.uid_of(e)],
                at,
                fix,
            ));
        }
        out
    }
}

impl Audit<'_> {
    /// The ramps measured against `profile` (docs/adr/0723 §4): too steep for their rise, a long run too steep, a landing
    /// too short, too narrow where they turn. The last two are fixed by `cad.arch.edit` unless locked; the first two
    /// need the ramp drawn again (longer, or with Rampa's En uzun kol).
    fn accessibility(&self, profile: Option<&(String, kentos_commands::RampRules)>) -> Vec<ArchAuditFinding> {
        let Some((profile_id, rules)) = profile else {
            return Vec::new();
        };
        let n = |v: f64| format!("{v}");
        let mut out = Vec::new();
        for e in self.doc.entities() {
            let Entity::Ramp(r) = e else { continue };
            let shape = kentos_formats::arch_elements::core_ramp(&r.ramp);
            let kentos_geometry_core::entity::Shape::Ramp { pts, ramp } = &shape else { continue };
            let walk = arch::Walk::of(pts);
            let run = arch::ramp_run(&walk, ramp);
            let rise = r.ramp.rise.abs();
            let uid = self.uid_of(e);
            let start = pts.first().copied().unwrap_or(V::new(0.0, 0.0));
            if run > 1e-9 && rise > 0.0 {
                let slope = rise / run * 100.0;
                if let Some(max) = rules.slope_for(rise)
                    && slope > max + 1e-9
                {
                    out.push(finding(
                        ArchAuditCheck::Accessibility,
                        codes::RAMP_SLOPE_OVER_PROFILE,
                        format!(
                            "Rampanın eğimi %{}; {} m kot farkı için en çok %{}.",
                            fixed(slope, 2),
                            fixed(rise, 2),
                            n(max)
                        ),
                        format!(
                            "Rampayı uzatın ya da kot farkını azaltın; eğim en çok %{} olmalı ({}, mevzuat onayı değildir).",
                            n(max),
                            profile_id
                        ),
                        vec![uid.clone()],
                        start,
                        None,
                    ));
                }
                for (k, (a, b)) in arch::ramp_runs(&walk, ramp).into_iter().enumerate() {
                    if b - a > rules.long_run.over + 1e-9 && slope > rules.long_run.max + 1e-9 {
                        out.push(finding(
                            ArchAuditCheck::Accessibility,
                            codes::RAMP_LONG_RUN_SLOPE,
                            format!(
                                "Rampanın {}. eğimli kolu {} m, eğimi %{}; {} m'den uzun kolda en çok %{}.",
                                k + 1,
                                fixed(b - a, 2),
                                fixed(slope, 2),
                                n(rules.long_run.over),
                                n(rules.long_run.max)
                            ),
                            "Rampayı En uzun kol ile ara sahanlıklı yeniden çizin ya da eğimini düşürün.",
                            vec![uid.clone()],
                            walk.point(a).0,
                            None,
                        ));
                    }
                }
            }
            let fixable = !self.locked(e);
            let lands = arch::ramp_landings(&walk, ramp);
            if let Some(&(a, b)) = lands.first()
                && ramp.landing() < rules.landing - 1e-9
            {
                out.push(finding(
                    ArchAuditCheck::Accessibility,
                    codes::RAMP_LANDING_SHORT,
                    format!("Rampanın sahanlığı {} m; en az {} m olmalı.", fixed(ramp.landing(), 2), fixed(rules.landing, 2)),
                    "Sahanlığı uzatın.",
                    vec![uid.clone()],
                    walk.point((a + b) / 2.0).0,
                    fixable.then(|| {
                        fix(
                            format!("Sahanlığı {} m yap", fixed(rules.landing, 2)),
                            "cad.arch.edit",
                            json!({"uids": [uid.clone()], "operation": {"kind": "set", "landing": rules.landing}}),
                        )
                    }),
                ));
            }
            if let Some(&k) = arch::turn_points(pts).first()
                && r.ramp.width < rules.turn - 1e-9
            {
                out.push(finding(
                    ArchAuditCheck::Accessibility,
                    codes::RAMP_TURN_NARROW,
                    format!(
                        "Rampa dönüşte {} m geniş; yön değiştiren sahanlık en az {} × {} m olmalı.",
                        fixed(r.ramp.width, 2),
                        fixed(rules.turn, 2),
                        fixed(rules.turn, 2)
                    ),
                    "Rampayı genişletin.",
                    vec![uid.clone()],
                    pts[k],
                    fixable.then(|| {
                        fix(
                            format!("Genişliği {} m yap", fixed(rules.turn, 2)),
                            "cad.arch.edit",
                            json!({"uids": [uid.clone()], "operation": {"kind": "set", "width": rules.turn}}),
                        )
                    }),
                ));
            }
        }
        out
    }
}

impl Audit<'_> {
    /// The stairs measured against `profile`'s stair rules (docs/adr/0724 §2): a riser too high, a tread too short, two
    /// risers and a tread outside their range, a flight too narrow, a landing shorter than the flight is wide, a winding
    /// stair's tread too narrow at its narrow end, a side without a railing. A flight's width, a landing and the
    /// railings are fixed by `cad.arch.edit` unless locked; the others need the stair designed again.
    fn stairs(&self, profile: Option<&(String, kentos_commands::StairRules)>) -> Vec<ArchAuditFinding> {
        let Some((profile_id, rules)) = profile else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for e in self.doc.entities() {
            let Entity::Stair(st) = e else { continue };
            let x = &st.stair;
            let shape = kentos_formats::arch_elements::core_stair(x);
            let kentos_geometry_core::entity::Shape::Stair { pts, stair } = &shape else { continue };
            let uid = self.uid_of(e);
            let start = pts.first().copied().unwrap_or(V::new(0.0, 0.0));
            let fixable = !self.locked(e);
            let regulation = format!("{profile_id}, mevzuat onayı değildir");
            let mut say = |code: &str, message: String, repair: String, at: V, fix: Option<ArchAuditFix>| {
                out.push(finding(ArchAuditCheck::Stairs, code, message, repair, vec![uid.clone()], at, fix));
            };
            if x.riser_height > rules.riser + 1e-9 {
                say(
                    codes::STAIR_RISER_OVER_PROFILE,
                    format!("Merdivenin rıhtı {} m; en çok {} m olmalı.", fixed(x.riser_height, 3), fixed(rules.riser, 2)),
                    format!("Rıht sayısını artırın; merdiven uzar ({regulation})."),
                    start,
                    None,
                );
            }
            if x.tread < rules.tread - 1e-9 {
                say(
                    codes::STAIR_TREAD_SHORT,
                    format!("Merdivenin basamağı {} m; en az {} m olmalı.", fixed(x.tread, 3), fixed(rules.tread, 2)),
                    format!("Basamağı derinleştirin; merdiven uzar ({regulation})."),
                    start,
                    None,
                );
            }
            let step = 2.0 * x.riser_height + x.tread;
            if step < rules.step[0] - 1e-9 || step > rules.step[1] + 1e-9 {
                say(
                    codes::STAIR_STEP_OVER_PROFILE,
                    format!(
                        "Merdivenin 2a+b'si {} m; {}–{} m olmalı.",
                        fixed(step, 3),
                        fixed(rules.step[0], 2),
                        fixed(rules.step[1], 2)
                    ),
                    format!("Rıhtı ya da basamağı değiştirin ({regulation})."),
                    start,
                    None,
                );
            }
            if x.width < rules.width - 1e-9 {
                say(
                    codes::STAIR_WIDTH_NARROW,
                    format!("Merdivenin kolu {} m geniş; en az {} m olmalı.", fixed(x.width, 2), fixed(rules.width, 2)),
                    format!("Merdiveni genişletin ({regulation})."),
                    start,
                    fixable.then(|| {
                        fix(
                            format!("Genişliği {} m yap", fixed(rules.width, 2)),
                            "cad.arch.edit",
                            json!({"uids": [uid.clone()], "operation": {"kind": "set", "width": rules.width}}),
                        )
                    }),
                );
            }
            let walk = arch::Walk::curved(pts, stair.bulge);
            if rules.landing_width
                && let Some(&(a, b)) = arch::stair_landings(&walk, stair).first()
                && stair.landing() < x.width - 1e-9
            {
                say(
                    codes::STAIR_LANDING_SHORT,
                    format!(
                        "Merdivenin sahanlığı {} m; en az kol genişliği ({} m) kadar olmalı.",
                        fixed(stair.landing(), 2),
                        fixed(x.width, 2)
                    ),
                    format!("Sahanlığı kol genişliği kadar yapın ({regulation})."),
                    walk.point((a + b) / 2.0).0,
                    fixable.then(|| {
                        fix(
                            "Sahanlığı kol genişliği yap",
                            "cad.arch.edit",
                            json!({"uids": [uid.clone()], "operation": {"kind": "set", "landing": null}}),
                        )
                    }),
                );
            }
            if let Some(inner) = arch::stair_measure(pts, stair).inner_tread
                && inner < rules.winder - 1e-9
            {
                say(
                    codes::STAIR_WINDER_NARROW,
                    format!(
                        "Döner merdivenin basamağı dar uçta {} m; en az {} m olmalı.",
                        fixed(inner, 3),
                        fixed(rules.winder, 2)
                    ),
                    format!("Yarıçapı büyütün ya da merdiveni daraltın ({regulation})."),
                    start,
                    None,
                );
            }
            if rules.both_railings && x.railings != Some(kentos_commands::Railings::Both) {
                let missing = match x.railings {
                    Some(kentos_commands::Railings::Left) => "Merdivenin sağ yanında korkuluk yok",
                    Some(kentos_commands::Railings::Right) => "Merdivenin sol yanında korkuluk yok",
                    _ => "Merdivende korkuluk yok",
                };
                say(
                    codes::STAIR_RAILING_MISSING,
                    format!("{missing}; iki yanında olmalı."),
                    format!("İki yanına korkuluk koyun ({regulation})."),
                    start,
                    fixable.then(|| {
                        fix(
                            "İki yana korkuluk koy",
                            "cad.arch.edit",
                            json!({"uids": [uid.clone()], "operation": {"kind": "set", "railings": "both"}}),
                        )
                    }),
                );
            }
        }
        out
    }
}

/// The profiles asked, from the catalog: the ramp rules and the stair rules with their profiles' ids
/// (`invalid_arch_audit` for one the catalog lacks, or a second with ramp or with stair rules).
type ProfileRules = (Option<(String, kentos_commands::RampRules)>, Option<(String, kentos_commands::StairRules)>);

fn profiles_of(input: &ArchAudit) -> Result<ProfileRules, Stop> {
    let mut ramp = None;
    let mut stair = None;
    let refuse = |k: usize, words: String| {
        Stop::failed(error(codes::INVALID_ARCH_AUDIT, words, Some(format!("profiles[{k}]"))))
    };
    for (k, id) in input.profiles.iter().enumerate() {
        let Some(p) = kentos_commands::arch_profile(id) else {
            let known: Vec<&str> = kentos_commands::ARCH_PROFILES.iter().map(|(i, _)| *i).collect();
            return Err(refuse(k, format!("“{id}” profili yok; şunlardan birini verin: {}.", known.join(", "))));
        };
        if let Some(r) = p.ramp {
            if ramp.is_some() {
                return Err(refuse(k, "Rampalar için bir profil verilir; ikinci rampa profilini kaldırın.".into()));
            }
            ramp = Some((p.id.clone(), r));
        }
        if let Some(s) = p.stair {
            if stair.is_some() {
                return Err(refuse(k, "Merdivenler için bir profil verilir; ikinci merdiven profilini kaldırın.".into()));
            }
            stair = Some((p.id.clone(), s));
        }
    }
    Ok((ramp, stair))
}

/// The level a level finding's fix gives: `level`, the project's (`level_not_found`), or the ground level (the one at
/// elevation 0, else the first); none while the project has no levels.
fn fix_level<'a>(doc: &'a Document, input: &ArchAudit) -> Result<Option<&'a ArchLevel>, Stop> {
    let levels = doc
        .settings()
        .arch
        .as_ref()
        .map(|a| a.levels.as_slice())
        .unwrap_or_default();
    match &input.level {
        Some(id) => match levels.iter().find(|l| &l.id == id) {
            Some(l) => Ok(Some(l)),
            None => Err(Stop::failed(error(
                codes::LEVEL_NOT_FOUND,
                kentos_commands::level_problem(Some(id), levels)
                    .unwrap_or_else(|| format!("Projede “{id}” kimlikli kat yok.")),
                Some("level".into()),
            ))),
        },
        None => Ok(levels.iter().find(|l| l.elevation == 0.0).or(levels.first())),
    }
}

fn answer(doc: &Document, input: &ArchAudit) -> Result<ArchAudited, Stop> {
    let limits = limits(input)?;
    let (ramp_rules, stair_rules) = profiles_of(input)?;
    let level = fix_level(doc, input)?;
    let audit = Audit { doc };
    let asked = |c: ArchAuditCheck| input.checks.is_empty() || input.checks.contains(&c);
    let mut findings = Vec::new();
    for (check, codes) in CODES {
        if !asked(check) {
            continue;
        }
        let mut found = match check {
            ArchAuditCheck::Walls => audit.walls(&limits),
            ArchAuditCheck::Openings => audit.openings(),
            ArchAuditCheck::Spaces => audit.spaces(&limits),
            ArchAuditCheck::Numbers => audit.numbers(),
            ArchAuditCheck::Tags => audit.tags(),
            ArchAuditCheck::Dimensions => audit.dimensions(),
            ArchAuditCheck::Grids => audit.grids(),
            ArchAuditCheck::Columns => audit.columns(),
            ArchAuditCheck::Viewmarks => audit.viewmarks(),
            ArchAuditCheck::Levels => audit.levels(level),
            ArchAuditCheck::Accessibility => audit.accessibility(ramp_rules.as_ref()),
            ArchAuditCheck::Stairs => audit.stairs(stair_rules.as_ref()),
        };
        let rank = |code: &str| codes.iter().position(|c| *c == code).unwrap_or(codes.len());
        // Stable: within one code and one place, the order they were found in.
        found.sort_by(|a, b| {
            rank(&a.code)
                .cmp(&rank(&b.code))
                .then(a.at.x.total_cmp(&b.at.x))
                .then(a.at.y.total_cmp(&b.at.y))
        });
        findings.extend(found);
    }
    Ok(ArchAudited {
        findings,
        revision: doc.revision().to_string(),
    })
}

pub fn validate(cx: &ExecutionContext<'_>, input: &ArchAudit) -> CommandResult<()> {
    match limits(input)
        .and_then(|_| profiles_of(input).map(|_| ()))
        .and_then(|_| fix_level(cx.doc, input).map(|_| ()))
    {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// The findings; the same answer as [`execute`].
pub fn plan(cx: &ExecutionContext<'_>, input: &ArchAudit) -> CommandResult<ArchAudited> {
    plan_in(cx.doc, input)
}

/// The findings without a context, for a panel.
pub fn plan_in(doc: &Document, input: &ArchAudit) -> CommandResult<ArchAudited> {
    match answer(doc, input) {
        Ok(output) => CommandResult::Completed {
            output,
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// The findings. Writes nothing.
pub fn execute(cx: &mut ExecutionContext<'_>, input: ArchAudit) -> CommandResult<ArchAudited> {
    plan_in(cx.doc, &input)
}
