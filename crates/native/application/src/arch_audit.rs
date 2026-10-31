//! `cad.arch.audit` v1 (read-only, docs/adr/0703): the drawing's architectural problems in one list, each with what
//! resolves it and, where a command plainly does, its call. Walls are checked level by level (walls of different
//! levels never join, docs/adr/0639): the joins' issues are `cad.walls.joins`'s; a free end that stops short of another
//! wall's body, or of another free end, is said. The other groups read the followers' stored `broken` fields, the
//! spaces' rings and the numbers. Given profiles, the ramps and the stairs are measured against them (docs/adr/0723 §4,
//! 0724 §2).
//! Since docs/adr/0732: short and overlapping walls, overlapping room separators, empty tags, the stairs' and the ramps'
//! own checks and broken kitchen runs; each finding's lasting key; the part of the drawing reported on (`within`); the
//! project's exceptions, bound to the findings as they were marked.
//! It writes nothing. The shared cases are `fixtures/commands/v1/cad.arch.audit.json`,
//! `cad.arch.audit.accessibility.json`, `cad.arch.audit.stairs.json` and `cad.arch.audit.scope.json`
//! (`scripts/fixtures/arch_audit_cases.py`, `ramp_access_cases.py`, `stair_profile_cases.py`).

use std::collections::{BTreeMap, HashSet};

use kentos_commands::{
    ARCH_AUDIT_MAX_GAP, ARCH_AUDIT_MIN_AREA, ARCH_AUDIT_MIN_LENGTH, ARCH_AUDIT_MIN_WIDTH, ArchAudit, ArchAuditCheck,
    ArchAuditExceptionView, ArchAuditFinding, ArchAuditFix, ArchAudited, ArchLevel, ChangesetStep, CommandResult,
    CommandWarning, Entity, Vec2,
};
use sha2::{Digest, Sha256};
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
const CODES: [(ArchAuditCheck, &[&str]); 15] = [
    (
        ArchAuditCheck::Walls,
        &[
            codes::WALL_JOIN_OVERLAP,
            codes::WALL_JOIN_CROWDED,
            codes::WALL_JOIN_ACUTE,
            codes::WALL_JOIN_UNRESOLVED,
            codes::ARCH_WALL_END_GAP,
            codes::ARCH_WALL_ENDS_APART,
            codes::ARCH_WALL_SHORT,
            codes::ARCH_WALL_OVERLAP,
            codes::ARCH_SEPARATOR_OVERLAP,
        ],
    ),
    (
        ArchAuditCheck::Openings,
        &[codes::ARCH_OPENING_BROKEN, codes::TAG_MISSING_TYPE, codes::ARCH_OPENING_TYPE_STALE],
    ),
    (
        ArchAuditCheck::Spaces,
        &[codes::ARCH_SPACE_BROKEN, codes::ARCH_SPACE_SMALL, codes::ARCH_SPACE_NARROW, codes::ARCH_SLAB_BROKEN],
    ),
    (ArchAuditCheck::Numbers, &[codes::TAG_DUPLICATE_NUMBER]),
    (ArchAuditCheck::Tags, &[codes::TAG_TARGET_GONE, codes::TAG_EMPTY]),
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
            codes::STAIR_SHORT,
            codes::STAIR_LONG,
            codes::STAIR_RISE_LEVEL,
            codes::STAIR_RISER_OVER_PROFILE,
            codes::STAIR_TREAD_SHORT,
            codes::STAIR_STEP_OVER_PROFILE,
            codes::STAIR_WIDTH_NARROW,
            codes::STAIR_LANDING_SHORT,
            codes::STAIR_WINDER_NARROW,
            codes::STAIR_RAILING_MISSING,
        ],
    ),
    (ArchAuditCheck::Ramps, &[codes::RAMP_NO_RUN, codes::RAMP_SLOPE_MISMATCH]),
    (ArchAuditCheck::Casework, &[codes::ARCH_CASEWORK_BROKEN]),
    (
        ArchAuditCheck::Doors,
        &[codes::ARCH_DOOR_SWING_CLASH, codes::ARCH_DOOR_FRONT_BLOCKED, codes::ARCH_DOOR_CLEAR_NARROW],
    ),
];

/// The codes only a profile finds (docs/adr/0723 §4, 0724 §2): an exception of one is not stale while its profile is not
/// asked.
const RAMP_PROFILE_CODES: [&str; 4] = [
    codes::RAMP_SLOPE_OVER_PROFILE,
    codes::RAMP_LONG_RUN_SLOPE,
    codes::RAMP_LANDING_SHORT,
    codes::RAMP_TURN_NARROW,
];
const STAIR_PROFILE_CODES: [&str; 7] = [
    codes::STAIR_RISER_OVER_PROFILE,
    codes::STAIR_TREAD_SHORT,
    codes::STAIR_STEP_OVER_PROFILE,
    codes::STAIR_WIDTH_NARROW,
    codes::STAIR_LANDING_SHORT,
    codes::STAIR_WINDER_NARROW,
    codes::STAIR_RAILING_MISSING,
];

/// The thresholds, checked: finite, then at least zero.
struct Limits {
    max_gap: f64,
    min_area: f64,
    min_width: f64,
    min_length: f64,
}

fn limits(input: &ArchAudit) -> Result<Limits, Stop> {
    let given = [
        (input.max_gap, "maxGap", "Boşluk eşiği", "m"),
        (input.min_area, "minArea", "Küçük mahal eşiği", "m²"),
        (input.min_width, "minWidth", "Dar mahal eşiği", "m"),
        (input.min_length, "minLength", "Kısa duvar eşiği", "m"),
        (input.min_clear_width, "minClearWidth", "En az net geçiş", "m"),
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
        min_length: input.min_length.unwrap_or(ARCH_AUDIT_MIN_LENGTH),
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
impl Audit<'_> {
    /// A slab bound to its walls or its space that its source no longer gives (docs/adr/0771 §4): its last outline
    /// kept; Düzelt detaches it (Ayır), keeping that outline its own.
    fn bound_slabs(&self) -> Vec<ArchAuditFinding> {
        let mut out = Vec::new();
        for e in self.doc.entities() {
            let Entity::Slab(s) = e else { continue };
            let Some(why) = s.slab.bound.as_ref().and_then(|b| b.broken) else { continue };
            let uid = self.uid_of(e);
            let fix = (!self.locked(e)).then(|| {
                fix("Ayır: son sınırı bağımsız kontur yap", "cad.slabs.edit", json!({"uids": [uid], "operation": {"kind": "detach"}}))
            });
            out.push(finding(
                ArchAuditCheck::Spaces,
                codes::ARCH_SLAB_BROKEN,
                format!("{}: {}", s.slab.role.label(), why.words()),
                "Kaynağa bağla ile döşemeyi yeniden bağlayın ya da Ayır ile son sınırını bağımsız kontur yapın.",
                vec![uid],
                space_point(&s.slab.parts),
                fix,
            ));
        }
        out
    }
}

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
        key: format!("{code}:{}", uids.join(",")),
        check,
        code: code.into(),
        message,
        repair: repair.into(),
        uids,
        at: wire(at),
        fix,
        exception: false,
        lapsed: false,
        note: None,
        nature: None,
    }
}

/// The digest an exception binds a finding by (docs/adr/0732 §3): the SHA-256 of the canonical JSON of its code, its
/// objects, its words and its place, its first eight bytes in lowercase hexadecimal.
pub fn finding_digest(f: &ArchAuditFinding) -> String {
    let value = json!({"at": {"x": f.at.x, "y": f.at.y}, "code": f.code, "message": f.message, "uids": f.uids});
    let text = kentos_commands::canonical::canonical_json(&value);
    Sha256::digest(text.as_bytes())[..8]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// A path's length: its edges', an arc's along it.
fn path_length(pts: &[V], bulges: Option<&[f64]>, closed: bool) -> f64 {
    bulge_path_edges(pts, bulges, closed)
        .iter()
        .map(|e| match *e {
            Edge::Seg { a, b } => (b.x - a.x).hypot(b.y - a.y),
            Edge::Arc { r, sweep, .. } => r * sweep.abs(),
        })
        .sum()
}

/// Two straight runs laid along each other (docs/adr/0732 §4): how long they overlap along the first, where the
/// overlap's middle is on it, and how far apart they lie across it; none when they are not parallel within a degree.
fn along_each_other(a: [V; 2], b: [V; 2]) -> Option<(f64, V, f64)> {
    let (dx, dy) = (a[1].x - a[0].x, a[1].y - a[0].y);
    let (ex, ey) = (b[1].x - b[0].x, b[1].y - b[0].y);
    let (la, lb) = (dx.hypot(dy), ex.hypot(ey));
    if la <= 1e-12 || lb <= 1e-12 || (dx * ey - dy * ex).abs() > 1f64.to_radians().sin() * la * lb {
        return None;
    }
    let (ux, uy) = (dx / la, dy / la);
    let along = |p: V| (p.x - a[0].x) * ux + (p.y - a[0].y) * uy;
    let across = |p: V| ((p.y - a[0].y) * ux - (p.x - a[0].x) * uy).abs();
    let (t0, t1) = (along(b[0]), along(b[1]));
    let (lo, hi) = (t0.min(t1).max(0.0), t0.max(t1).min(la));
    let length = hi - lo;
    let mid = (lo + hi) / 2.0;
    Some((length, V::new(a[0].x + ux * mid, a[0].y + uy * mid), (across(b[0]) + across(b[1])) / 2.0))
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
            let mut f = finding(
                ArchAuditCheck::Walls,
                codes::ARCH_WALL_END_GAP,
                format!("Duvarın {lower} ucuyla öbür duvar arasında {gap:.3} m boşluk var; birleşmiyor."),
                "Ucu öbür duvara uzatın; mahal ancak duvarlar birleşince kapanır.",
                vec![uid(e.wall), uid(j)],
                e.at,
                fix,
            );
            // The end in its key: a wall's two ends may each stop short of one other wall (docs/adr/0732 §1).
            f.key.push(':');
            f.key.push_str(key);
            out.push(f);
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
        out.extend(self.short_walls(&walls, limits));
        out.extend(self.overlapping_walls(&walls));
        out
    }

    /// The walls' openings' hosts, by the walls' persistent ids.
    fn hosts(&self) -> HashSet<String> {
        self.doc
            .entities()
            .filter_map(|e| match e {
                Entity::Opening(o) => Some(o.opening.host.clone()),
                _ => None,
            })
            .collect()
    }

    /// Leftover pieces (docs/adr/0732 §4): open walls shorter than the least length, each removed by its fix unless it
    /// holds an opening or is locked.
    fn short_walls(&self, walls: &[WallAt<'_>], limits: &Limits) -> Vec<ArchAuditFinding> {
        let hosts = self.hosts();
        let mut out = Vec::new();
        for w in walls {
            if w.spec.closed || w.pts.len() < 2 {
                continue;
            }
            let length = path_length(&w.pts, w.bulges.as_deref(), false);
            if length >= limits.min_length {
                continue;
            }
            let uid = self.uid_of(w.entity);
            let (a, b) = (w.pts[0], w.pts[w.pts.len() - 1]);
            let fix = (!self.locked(w.entity) && !hosts.contains(&uid)).then(|| {
                fix("Kısa duvar parçasını sil", "cad.entities.delete", json!({"uids": [uid.clone()]}))
            });
            out.push(finding(
                ArchAuditCheck::Walls,
                codes::ARCH_WALL_SHORT,
                format!("Duvar {length:.3} m; çok kısa bir parça."),
                "Artık bir parçaysa silin; bilerek kısaysa istisna yapın.",
                vec![uid],
                V::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0),
                fix,
            ));
        }
        out
    }

    /// Straight walls of one level drawn over each other along their length (docs/adr/0732 §4): parallel within a
    /// degree, nearer across than their half thicknesses together and overlapping along for more than the thicker one
    /// (a join's corner is not). Said once a pair, where they overlap longest; a copy of the other (its path the same
    /// way round, or the other way round when centred) is removed by its fix, the later one unless it holds an opening.
    fn overlapping_walls(&self, walls: &[WallAt<'_>]) -> Vec<ArchAuditFinding> {
        let runs: Vec<Option<Vec<[V; 2]>>> = walls.iter().map(center_runs).collect();
        let hosts = self.hosts();
        let mut out = Vec::new();
        for i in 0..walls.len() {
            for j in i + 1..walls.len() {
                let (Some(ra), Some(rb)) = (&runs[i], &runs[j]) else { continue };
                if walls[i].level != walls[j].level {
                    continue;
                }
                let (ta, tb) = (walls[i].spec.width, walls[j].spec.width);
                let best = ra
                    .iter()
                    .flat_map(|a| rb.iter().filter_map(move |b| along_each_other(*a, *b)))
                    .filter(|(length, _, across)| {
                        *length > ta.max(tb) + JOIN_TOLERANCE && *across < (ta + tb) / 2.0 - JOIN_TOLERANCE
                    })
                    .max_by(|x, y| x.0.total_cmp(&y.0));
                let Some((length, at, _)) = best else { continue };
                let (ua, ub) = (self.uid_of(walls[i].entity), self.uid_of(walls[j].entity));
                let fix = same_wall(walls[i].entity, walls[j].entity)
                    .then(|| {
                        [(j, &ub), (i, &ua)]
                            .into_iter()
                            .find(|(k, u)| !self.locked(walls[*k].entity) && !hosts.contains(*u))
                            .map(|(_, u)| fix("Yinelenen duvarı sil", "cad.entities.delete", json!({"uids": [u]})))
                    })
                    .flatten();
                out.push(finding(
                    ArchAuditCheck::Walls,
                    codes::ARCH_WALL_OVERLAP,
                    format!("İki duvar {length:.3} m boyunca üst üste çizilmiş."),
                    "Fazla duvarı silin ya da kısaltın; mahal sınırında biri yok sayılabilir.",
                    vec![ua, ub],
                    at,
                    fix,
                ));
            }
        }
        out
    }

    /// Room separators of one level over each other on one line (docs/adr/0732 §4; Revit's “Highlighted Room
    /// Separation Lines Overlap”): straight edges within a millimetre across, overlapping for more than a millimetre.
    /// A copy of the other (the same points, either way round) is removed by its fix, the later one.
    fn separators(&self) -> Vec<ArchAuditFinding> {
        let seps: Vec<(&Entity, &kentos_commands::PathEntity)> = self
            .doc
            .entities()
            .filter_map(|e| match e {
                Entity::Polyline(p) if p.space_separator.is_some() => Some((e, p)),
                _ => None,
            })
            .collect();
        let edges: Vec<Vec<[V; 2]>> = seps.iter().map(|(_, p)| straight_edges(p)).collect();
        let level = |p: &kentos_commands::PathEntity| p.space_separator.as_ref().and_then(|a| a.level.clone());
        let mut out = Vec::new();
        for i in 0..seps.len() {
            for j in i + 1..seps.len() {
                if level(seps[i].1) != level(seps[j].1) {
                    continue;
                }
                let best = edges[i]
                    .iter()
                    .flat_map(|a| edges[j].iter().filter_map(move |b| along_each_other(*a, *b)))
                    .filter(|(length, _, across)| *length > 1e-3 && *across <= 1e-3)
                    .max_by(|x, y| x.0.total_cmp(&y.0));
                let Some((length, at, _)) = best else { continue };
                let (ua, ub) = (self.uid_of(seps[i].0), self.uid_of(seps[j].0));
                let twin = same_points(&seps[i].1.pts, &seps[j].1.pts);
                let fix = (twin && !self.locked(seps[j].0))
                    .then(|| fix("Yinelenen ayırıcıyı sil", "cad.entities.delete", json!({"uids": [ub.clone()]})));
                out.push(finding(
                    ArchAuditCheck::Walls,
                    codes::ARCH_SEPARATOR_OVERLAP,
                    format!("İki mahal ayırıcı {length:.3} m boyunca üst üste."),
                    "Fazla ayırıcıyı silin; mahal sınırını bir ayırıcı çizer.",
                    vec![ua, ub],
                    at,
                    fix,
                ));
            }
        }
        out
    }
}

/// A straight wall's body's middle line, run by run: the middle of its faces (none when an edge is an arc or it cannot
/// be drawn).
fn center_runs(w: &WallAt<'_>) -> Option<Vec<[V; 2]>> {
    if w.bulges.as_ref().is_some_and(|b| b.iter().any(|x| *x != 0.0)) {
        return None;
    }
    let lines = wall_geom::lines(&w.pts, None, &w.spec).ok()?;
    let (left, right) = (lines.first()?, lines.last()?);
    if left.pts.len() != right.pts.len() || left.pts.len() < 2 {
        return None;
    }
    let mid: Vec<V> = left
        .pts
        .iter()
        .zip(&right.pts)
        .map(|(l, r)| V::new((l.x + r.x) / 2.0, (l.y + r.y) / 2.0))
        .collect();
    let mut runs: Vec<[V; 2]> = mid.windows(2).map(|p| [p[0], p[1]]).collect();
    if w.spec.closed && mid.len() > 2 {
        runs.push([mid[mid.len() - 1], mid[0]]);
    }
    Some(runs)
}

/// A path's straight edges (an arc's left out), its closing one too when closed.
fn straight_edges(p: &kentos_commands::PathEntity) -> Vec<[V; 2]> {
    let n = p.pts.len();
    let bulge = |k: usize| p.bulges.as_ref().and_then(|b| b.get(k).copied()).unwrap_or(0.0);
    let count = if p.closed && n > 2 { n } else { n.saturating_sub(1) };
    (0..count)
        .filter(|&k| bulge(k) == 0.0)
        .map(|k| [core(p.pts[k]), core(p.pts[(k + 1) % n])])
        .collect()
}

/// Whether two point lists are the same, either way round.
fn same_points(a: &[Vec2], b: &[Vec2]) -> bool {
    a.len() == b.len() && (a.iter().zip(b).all(|(p, q)| p == q) || a.iter().zip(b.iter().rev()).all(|(p, q)| p == q))
}

/// Whether wall `b` is a copy of wall `a`: the same path the same way round with the same width, justification,
/// offset and closing, or the other way round when both are centred on their paths.
fn same_wall(a: &Entity, b: &Entity) -> bool {
    let (Entity::Wall(a), Entity::Wall(b)) = (a, b) else { return false };
    let (a, b) = (&a.wall, &b.wall);
    let straight = |w: &kentos_commands::WallFields| w.bulges.as_ref().is_none_or(|x| x.iter().all(|v| *v == 0.0));
    if !(straight(a) && straight(b) && a.width == b.width && a.closed == b.closed && a.pts.len() == b.pts.len()) {
        return false;
    }
    let forward = a.justify == b.justify && a.offset == b.offset && a.pts.iter().zip(&b.pts).all(|(p, q)| p == q);
    let centred = |w: &kentos_commands::WallFields| w.justify == kentos_commands::WallJustify::Center && w.offset == 0.0;
    let backward = centred(a) && centred(b) && a.pts.iter().zip(b.pts.iter().rev()).all(|(p, q)| p == q);
    forward || backward
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
            // Behind its type's version (docs/adr/0772 §4).
            if o.follows_type()
                && let Some(t) = o.style.as_deref().and_then(|s| self.doc.opening_types().iter().find(|t| t.id == s))
                && o.type_version.unwrap_or(0) < t.version
            {
                let n = n.map_or_else(|| "Numarasız açıklık".to_owned(), |n| format!("“{n}” açıklığı"));
                let fix = (!self.locked(e)).then(|| {
                    fix(
                        "Tipi yeniden uygula",
                        "cad.openings.edit",
                        json!({"uids": [self.uid_of(e)], "operation": {"kind": "refresh"}}),
                    )
                });
                out.push(finding(
                    ArchAuditCheck::Openings,
                    codes::ARCH_OPENING_TYPE_STALE,
                    format!(
                        "{n} “{}” tipinin {}. sürümünü taşıyor; tipin son sürümü {}.",
                        t.name,
                        o.type_version.unwrap_or(0),
                        t.version
                    ),
                    "Kilidi açıp tipi yeniden uygulayın (Tipe döndür) ya da açıklığı tipten ayırın.",
                    vec![self.uid_of(e)],
                    at,
                    fix,
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
        let mut out = self.bound_slabs();
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
                // Its words empty: its object has none of its template's values (docs/adr/0344's 27 October addendum).
                if t.tag.text.trim().is_empty() {
                    out.push(finding(
                        ArchAuditCheck::Tags,
                        codes::TAG_EMPTY,
                        "Etiketin yazısı boş: nesnesinin şablondaki değerleri yok.".into(),
                        "Nesnenin değerlerini verin (numara, ad, tür) ya da etiketin şablonunu değiştirin.",
                        vec![self.uid_of(e)],
                        core(t.tag.at),
                        None,
                    ));
                }
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
        B::MixedSources => {
            "Ölçü dizisinin duvarları artık farklı katlarda; ölçüler son yerlerinde duruyor."
        }
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
        let mut out = self.stairs_own();
        let Some((profile_id, rules)) = profile else {
            return out;
        };
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

impl Audit<'_> {
    /// The stairs' own checks (docs/adr/0343 §2, 0713; docs/adr/0732 §4): a walking line too short for the risers, one
    /// longer than the stair, risers that do not make their level's height (fixed by taking the height from the level
    /// unless locked). Blondel's rule is information, not a finding.
    fn stairs_own(&self) -> Vec<ArchAuditFinding> {
        let mut out = Vec::new();
        for e in self.doc.entities() {
            let Entity::Stair(st) = e else { continue };
            let uid = self.uid_of(e);
            let start = st.stair.pts.first().copied().map_or(V::new(0.0, 0.0), core);
            for w in crate::arch_elements::stair_warnings(self.doc, &st.stair, "stair") {
                let (message, repair, fix) = match w.code.as_str() {
                    codes::STAIR_SHORT => {
                        let (what, repair) = split(&w.message);
                        (what, repair, None)
                    }
                    codes::STAIR_LONG => (
                        w.message.clone(),
                        "Yolu son rıhtta bitirin (Merdiven'in Sığdır'ı) ya da rıht sayısını artırın.".to_owned(),
                        None,
                    ),
                    codes::STAIR_RISE_LEVEL => {
                        let (what, _) = split(&w.message);
                        let fix = (!self.locked(e)).then(|| {
                            fix(
                                "Yüksekliği kattan al",
                                "cad.arch.edit",
                                json!({"uids": [uid.clone()], "operation": {"kind": "set", "heightFromLevel": true}}),
                            )
                        });
                        (
                            what,
                            "Rıht yüksekliğini katın yüksekliğinden alın ya da rıht sayısını değiştirin.".to_owned(),
                            fix,
                        )
                    }
                    _ => continue,
                };
                out.push(finding(ArchAuditCheck::Stairs, &w.code, message, repair, vec![uid.clone()], start, fix));
            }
        }
        out
    }

    /// The ramps' own checks (docs/adr/0343 §2, 0732 §4): no sloped run left after the landings, a given slope its rise
    /// and sloped run do not make (fixed by giving the slope they make unless locked).
    fn ramps(&self) -> Vec<ArchAuditFinding> {
        let mut out = Vec::new();
        for e in self.doc.entities() {
            let Entity::Ramp(r) = e else { continue };
            let uid = self.uid_of(e);
            let start = r.ramp.pts.first().copied().map_or(V::new(0.0, 0.0), core);
            for w in crate::arch_elements::ramp_warnings(&r.ramp, "ramp") {
                let (message, repair, fix) = match w.code.as_str() {
                    codes::RAMP_NO_RUN => (
                        w.message.clone(),
                        "Yolu uzatın ya da sahanlığı kısaltın.".to_owned(),
                        None,
                    ),
                    codes::RAMP_SLOPE_MISMATCH => {
                        let shape = kentos_formats::arch_elements::core_ramp(&r.ramp);
                        let made = match &shape {
                            Shape::Ramp { pts, ramp } => {
                                let run = arch::ramp_run(&arch::Walk::of(pts), ramp);
                                (run > 1e-9).then(|| r.ramp.rise / run * 100.0)
                            }
                            _ => None,
                        };
                        let fix = made.filter(|_| !self.locked(e)).map(|slope| {
                            fix(
                                format!("Eğimi %{} yap", fixed(slope, 2)),
                                "cad.arch.edit",
                                json!({"uids": [uid.clone()], "operation": {"kind": "set", "slope": slope}}),
                            )
                        });
                        (
                            w.message.clone(),
                            "Eğimi yüksekliğin eğimli uzunluğa oranına eşitleyin ya da yüksekliği veya yolu değiştirin."
                                .to_owned(),
                            fix,
                        )
                    }
                    _ => continue,
                };
                out.push(finding(ArchAuditCheck::Ramps, &w.code, message, repair, vec![uid.clone()], start, fix));
            }
        }
        out
    }

    /// Kitchen runs that no longer follow (docs/adr/0727 §5, 0732 §4): their wall gone or their recipe not laid; each
    /// released by its fix unless its layer is locked. A run none of whose objects is left is not said.
    fn casework(&self) -> Vec<ArchAuditFinding> {
        let mut out = Vec::new();
        for run in self.doc.casework() {
            let Some(b) = run.broken else { continue };
            let uids: Vec<String> = run
                .members
                .iter()
                .map(|m| m.uid.to_text())
                .filter(|u| {
                    kentos_domain::Uuid::parse_str(u)
                        .ok()
                        .and_then(|u| self.doc.slot_of(u))
                        .is_some()
                })
                .collect();
            if uids.is_empty() {
                continue;
            }
            let message = match b {
                kentos_commands::CaseworkBreak::WallGone => {
                    "Mutfak dizisinin duvarı çizimde yok; dolaplar son yerlerinde duruyor."
                }
                kentos_commands::CaseworkBreak::NotBuilt => {
                    "Mutfak dizisi tarifinden kurulamıyor; dolaplar son yerlerinde duruyor."
                }
            };
            let fix = (!self.doc.layers().is_locked(&run.recipe.layer_id)).then(|| {
                fix(
                    "Diziyi bırak",
                    "cad.casework.edit",
                    json!({"run": run.id.to_text(), "edit": {"kind": "release"}}),
                )
            });
            out.push(finding(
                ArchAuditCheck::Casework,
                codes::ARCH_CASEWORK_BROKEN,
                message.into(),
                "Duvarı geri getirin ya da diziyi bırakın; bırakılan dizinin dolapları serbest nesneler olur.",
                uids,
                core(run.recipe.from),
                fix,
            ));
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

/// The part of the drawing asked (docs/adr/0732 §2), checked: its levels the project's (`level_not_found`), its objects
/// the drawing's (`entity_not_found`); the objects by their slots.
struct Scope {
    levels: Option<Vec<Option<String>>>,
    uids: Option<HashSet<String>>,
}

fn scope_of(doc: &Document, input: &ArchAudit) -> Result<Scope, Stop> {
    let Some(within) = &input.within else {
        return Ok(Scope { levels: None, uids: None });
    };
    let levels = doc.settings().arch.as_ref().map(|a| a.levels.as_slice()).unwrap_or_default();
    for (i, id) in within.levels.iter().flatten().enumerate() {
        if let Some(id) = id
            && !levels.iter().any(|l| &l.id == id)
        {
            return Err(Stop::failed(error(
                codes::LEVEL_NOT_FOUND,
                kentos_commands::level_problem(Some(id), levels)
                    .unwrap_or_else(|| format!("Projede “{id}” kimlikli kat yok.")),
                Some(format!("within.levels[{i}]")),
            )));
        }
    }
    for (i, u) in within.uids.iter().flatten().enumerate() {
        if kentos_domain::Uuid::parse_str(u).ok().and_then(|u| doc.slot_of(u)).is_none() {
            return Err(Stop::failed(error(
                codes::ENTITY_NOT_FOUND,
                format!("“{u}” kimlikli nesne çizimde yok."),
                Some(format!("within.uids[{i}]")),
            )));
        }
    }
    Ok(Scope {
        levels: within.levels.clone(),
        uids: within.uids.as_ref().map(|u| u.iter().cloned().collect()),
    })
}

impl Scope {
    /// Whether finding `f` is in the part asked: one of its objects on one of the levels, one of them among the objects.
    fn holds(&self, doc: &Document, f: &ArchAuditFinding) -> bool {
        let by_uid = |u: &str| {
            kentos_domain::Uuid::parse_str(u)
                .ok()
                .and_then(|u| doc.slot_of(u))
                .and_then(|s| doc.get(s))
        };
        let on_level = self.levels.as_ref().is_none_or(|levels| {
            f.uids.iter().any(|u| {
                let level = by_uid(u).and_then(|e| e.level(by_uid)).map(str::to_owned);
                levels.contains(&level)
            })
        });
        on_level && self.uids.as_ref().is_none_or(|set| f.uids.iter().any(|u| set.contains(u)))
    }
}

/// Every finding of the groups asked, in their order, each with its lasting key (`#2`, `#3` for a second and a third
/// alike, docs/adr/0732 §1); the codes those groups could find (a profile's only when it is asked).
fn found(doc: &Document, input: &ArchAudit) -> Result<(Vec<ArchAuditFinding>, HashSet<&'static str>), Stop> {
    let limits = limits(input)?;
    let (ramp_rules, stair_rules) = profiles_of(input)?;
    let level = fix_level(doc, input)?;
    let audit = Audit { doc };
    // The doors (docs/adr/0772 §5) only when asked by name: an older call's findings stay as they were.
    let asked = |c: ArchAuditCheck| {
        input.checks.contains(&c) || (input.checks.is_empty() && c != ArchAuditCheck::Doors)
    };
    let mut findings = Vec::new();
    let mut run: HashSet<&'static str> = HashSet::new();
    for (check, codes) in CODES {
        if !asked(check) {
            continue;
        }
        run.extend(codes.iter().copied().filter(|c| {
            (ramp_rules.is_some() || !RAMP_PROFILE_CODES.contains(c))
                && (stair_rules.is_some() || !STAIR_PROFILE_CODES.contains(c))
        }));
        let mut found = match check {
            ArchAuditCheck::Walls => {
                let mut f = audit.walls(&limits);
                f.extend(audit.separators());
                f
            }
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
            ArchAuditCheck::Ramps => audit.ramps(),
            ArchAuditCheck::Casework => audit.casework(),
            ArchAuditCheck::Doors => crate::door_audit::findings(
                doc,
                input.min_clear_width,
                |code, message, repair, slots, at, nature| {
                    let mut f = finding(
                        ArchAuditCheck::Doors,
                        code,
                        message,
                        repair,
                        slots.iter().map(|s| audit.uid(*s)).collect(),
                        at,
                        None,
                    );
                    f.nature = Some(nature);
                    f
                },
            ),
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
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    for f in &mut findings {
        let n = seen.entry(f.key.clone()).or_insert(0);
        *n += 1;
        if *n > 1 {
            f.key = format!("{}#{n}", f.key);
        }
    }
    Ok((findings, run))
}

/// The answer and its warnings: the findings in the part asked, the project's exceptions laid on them (docs/adr/0732
/// §3) and listed as asked.
fn answer(doc: &Document, input: &ArchAudit) -> Result<(ArchAudited, Vec<CommandWarning>), Stop> {
    let (all, run) = found(doc, input)?;
    let scope = scope_of(doc, input)?;
    let exceptions = doc.settings().arch.as_ref().map(|a| a.audit_exceptions.as_slice()).unwrap_or_default();
    let stale: Vec<String> = exceptions
        .iter()
        .filter(|x| run.contains(x.code.as_str()) && !all.iter().any(|f| f.key == x.key))
        .map(|x| x.key.clone())
        .collect();
    let view = input.exceptions.unwrap_or_default();
    let (mut outside, mut excepted, mut lapsed) = (0u32, 0u32, 0u32);
    let mut findings = Vec::new();
    for mut f in all {
        if !scope.holds(doc, &f) {
            outside += 1;
            continue;
        }
        if let Some(x) = exceptions.iter().find(|x| x.key == f.key) {
            f.note = x.note.clone();
            if x.digest == finding_digest(&f) {
                f.exception = true;
                excepted += 1;
            } else {
                f.lapsed = true;
                lapsed += 1;
            }
        }
        let marked = f.exception || f.lapsed;
        let listed = match view {
            ArchAuditExceptionView::Hide => !f.exception,
            ArchAuditExceptionView::Show => true,
            ArchAuditExceptionView::Only => marked,
        };
        if listed {
            findings.push(f);
        }
    }
    let warnings = (lapsed > 0)
        .then(|| CommandWarning {
            code: codes::EXCEPTION_LAPSED.into(),
            message: format!(
                "{lapsed} istisnanın bulgusu verildiğinden beri değişti; bulgular yeniden açık. İstisnayı yeniden verin ya da kaldırın."
            ),
            path: None,
        })
        .into_iter()
        .collect();
    Ok((
        ArchAudited {
            findings,
            outside,
            excepted,
            stale,
            revision: doc.revision().to_string(),
        },
        warnings,
    ))
}

/// Every finding of `input`'s groups with its key, the part asked and the exceptions left aside (`cad.arch.exceptions`
/// names findings by them), and the codes they could find.
pub(crate) fn every_finding(
    doc: &Document,
    input: &ArchAudit,
) -> Result<(Vec<ArchAuditFinding>, HashSet<&'static str>), Stop> {
    found(doc, input)
}

pub fn validate(cx: &ExecutionContext<'_>, input: &ArchAudit) -> CommandResult<()> {
    match limits(input)
        .and_then(|_| profiles_of(input).map(|_| ()))
        .and_then(|_| fix_level(cx.doc, input).map(|_| ()))
        .and_then(|_| scope_of(cx.doc, input).map(|_| ()))
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
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// The findings. Writes nothing.
pub fn execute(cx: &mut ExecutionContext<'_>, input: ArchAudit) -> CommandResult<ArchAudited> {
    plan_in(cx.doc, &input)
}
