//! `cad.program.check` v1 (read-only, docs/adr/0773; plan/architecture-advanced.md ADV-04): the building program read
//! against the drawing. Each group's totals sum each space once (a space is in one group or none; common and outdoor
//! areas never go into a unit's); a space's area is its own rings' by its own method (the space engine's: open
//! kitchens by room separators, shafts as voids, columns, niches and the finished or the core face all come from it);
//! its net width is the diameter of the largest circle inside it; a corridor's net passage the widest a body may be and
//! still go between all its doors (its rings eroded by half that, every door near one piece); doors are the space's
//! passages (`kentos_domain::passages`), and two spaces whose sides lie on each other open into one another (a room
//! separator). The findings are Mimari denetim's `program` group: they say the measured value, where, and the rule and
//! the brief's version. A long check: its progress said, its stop heard. The shared cases are
//! `fixtures/commands/v1/cad.program.check.json` (`scripts/fixtures/program_cases.py`).

use std::collections::{BTreeMap, HashMap};

use kentos_commands::{
    ArchAuditCheck, ArchAuditFinding, ArchAuditFix, BuildingProgram, ChangesetStep, CommandResult, Entity,
    ProgramCheck, ProgramChecked, ProgramLink, ProgramSpaceRow, ProgramTotals, ProgramUnitRow, SpaceEntity, UnitKind,
    Vec2, ADJACENT_GAP, ADJACENT_OVERLAP, OPEN_GAP,
};
use kentos_domain::passages::{self, Passage};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::Vec2 as V;
use kentos_geometry_core::display::fixed;
use kentos_geometry_core::geom::arrangement::Area;
use kentos_geometry_core::geom::intersect::{Edge, closest_on_edge};
use kentos_geometry_core::geom::region::{inside_area, intersect_areas, net_area, ring_edges};
use kentos_geometry_core::ops::geoprocess::Class;
use kentos_geometry_core::ops::geoprocess::buffer::{Side, buffer};
use serde_json::json;

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;
use crate::spaces::{self, fail};

/// The codes of the program group, in the order its findings come.
pub const CODES: [&str; 9] = [
    codes::PROGRAM_UNIT_MISSING,
    codes::PROGRAM_AREA_BELOW,
    codes::PROGRAM_AREA_ABOVE,
    codes::PROGRAM_WIDTH_NARROW,
    codes::PROGRAM_PASSAGE_NARROW,
    codes::PROGRAM_ADJACENT_MISSING,
    codes::PROGRAM_REACH_MISSING,
    codes::PROGRAM_NO_DOOR,
    codes::PROGRAM_FIXTURE_CLEARANCE,
];

/// How close the pole of inaccessibility and the passage are measured, metres.
const PRECISION: f64 = 1e-4;
/// How near a door must lie to a piece of the eroded space to reach it, beyond half the width, metres.
const DOOR_REACH: f64 = 1e-6;
/// The most halvings of the passage search.
const HALVINGS: usize = 40;

fn core(p: &Vec2) -> V {
    V::new(p.x, p.y)
}

fn wire(p: V) -> Vec2 {
    Vec2 { x: p.x, y: p.y }
}

/// A space as the check reads it.
struct Room<'a> {
    slot: Slot,
    uid: String,
    s: &'a SpaceEntity,
    areas: Vec<Area>,
    area: f64,
    level: Option<String>,
    /// Its straight sides, each oriented so that the room lies on its left.
    sides: Vec<(V, V)>,
}

/// The straight sides of rings oriented with their inside on the left.
fn sides_of(parts: &[kentos_commands::SpacePart]) -> Vec<(V, V)> {
    let mut out = Vec::new();
    for part in parts {
        for (h, ring) in std::iter::once(&part.outer).chain(&part.holes).enumerate() {
            let n = ring.pts.len();
            let signed: f64 = (0..n)
                .map(|i| ring.pts[i].x * ring.pts[(i + 1) % n].y - ring.pts[(i + 1) % n].x * ring.pts[i].y)
                .sum();
            // An outer ring counter-clockwise, a hole clockwise: the inside on the left.
            let keep = (signed > 0.0) == (h == 0);
            for i in 0..n {
                if ring.bulges.as_ref().is_some_and(|b| b.get(i).copied().unwrap_or(0.0) != 0.0) {
                    continue;
                }
                let (a, b) = (core(&ring.pts[i]), core(&ring.pts[(i + 1) % n]));
                if a == b {
                    continue;
                }
                out.push(if keep { (a, b) } else { (b, a) });
            }
        }
    }
    out
}

/// How two rooms' sides meet: the longest stretch along which one runs on the other within `gap` (facing, the rooms
/// on their own sides), halfway between them; none when no stretch is `ADJACENT_OVERLAP` long.
fn facing(a: &[(V, V)], b: &[(V, V)], gap: f64) -> Option<(V, V)> {
    let mut best: Option<(f64, (V, V))> = None;
    for &(s, t) in a {
        let (dx, dy) = (t.x - s.x, t.y - s.y);
        let len = dx.hypot(dy);
        let (ux, uy) = (dx / len, dy / len);
        // Outward: the right of a side whose room is on its left.
        let (ox, oy) = (uy, -ux);
        for &(p, q) in b {
            let (ex, ey) = (q.x - p.x, q.y - p.y);
            let el = ex.hypot(ey);
            // Opposite directions, parallel.
            if (ux * ey - uy * ex).abs() / el > 1e-9 || ux * ex + uy * ey >= 0.0 {
                continue;
            }
            let off = ox * (p.x - s.x) + oy * (p.y - s.y);
            if !(-OPEN_GAP..=gap + 1e-9).contains(&off) {
                continue;
            }
            let (k0, k1) = (ux * (p.x - s.x) + uy * (p.y - s.y), ux * (q.x - s.x) + uy * (q.y - s.y));
            let (lo, hi) = (k0.min(k1).max(0.0), k0.max(k1).min(len));
            let over = hi - lo;
            if over + 1e-9 < ADJACENT_OVERLAP {
                continue;
            }
            let at = |k: f64| V::new(s.x + ux * k + ox * off / 2.0, s.y + uy * k + oy * off / 2.0);
            if best.is_none_or(|(o, _)| over > o) {
                best = Some((over, (at(lo), at(hi))));
            }
        }
    }
    best.map(|(_, m)| m)
}

/// Where a passage meets a room: its face toward the room, from jamb to jamb.
fn door_span(p: &Passage, side: f64) -> (V, V) {
    let (dx, dy) = (p.b.x - p.a.x, p.b.y - p.a.y);
    let len = dx.hypot(dy);
    let (nx, ny) = (-dy / len, dx / len);
    let off = |q: V| V::new(q.x + side * p.half * nx, q.y + side * p.half * ny);
    (off(p.a), off(p.b))
}

/// A placed fixture: its id, level, footprint, clearance zone (when its usage has one) and middle.
type FixtureBox = (String, Option<String>, Area, Option<Area>, V);

/// The distance of a segment from an edge (a line or an arc): its ends', the edge's ends', and an arc's point nearest
/// the segment's point nearest its centre; 0 when they meet.
fn span_edge_distance(s: (V, V), e: &Edge) -> f64 {
    let seg = Edge::Seg { a: s.0, b: s.1 };
    if !kentos_geometry_core::geom::intersect::intersect_edges(&seg, e).is_empty() {
        return 0.0;
    }
    let ends = match *e {
        Edge::Seg { a, b } => [a, b],
        Edge::Arc { c, r, a0, sweep } => [
            V::new(c.x + r * a0.cos(), c.y + r * a0.sin()),
            V::new(c.x + r * (a0 + sweep).cos(), c.y + r * (a0 + sweep).sin()),
        ],
    };
    let mut best = closest_on_edge(e, s.0).d.min(closest_on_edge(e, s.1).d);
    for q in ends {
        best = best.min(closest_on_edge(&seg, q).d);
    }
    if let Edge::Arc { c, .. } = *e {
        let toward = closest_on_edge(&seg, c).p;
        let on_arc = closest_on_edge(e, toward).p;
        best = best.min(closest_on_edge(&seg, on_arc).d);
    }
    best
}

fn distance_to(area: &Area, s: (V, V)) -> f64 {
    if inside_area(area, s.0) || inside_area(area, s.1) {
        return 0.0;
    }
    std::iter::once(&area.outer)
        .chain(&area.holes)
        .flat_map(ring_edges)
        .map(|e| span_edge_distance(s, &e))
        .fold(f64::INFINITY, f64::min)
}

/// Whether every door (its span on the room's face) reaches one piece of the rooms eroded by `w / 2`: a body as wide
/// as `w` touches each door and goes from one to another.
fn passes(areas: &[Area], doors: &[(V, V)], w: f64) -> bool {
    let r = w / 2.0;
    let eroded = if r <= 0.0 { areas.to_vec() } else { buffer(&Class::Areas(areas.to_vec()), -r, Side::Both) };
    if eroded.is_empty() {
        return false;
    }
    let mut piece: Option<usize> = None;
    for &d in doors {
        let near = eroded
            .iter()
            .enumerate()
            .map(|(i, a)| (i, distance_to(a, d)))
            .filter(|(_, x)| *x <= r + DOOR_REACH)
            .min_by(|a, b| a.1.total_cmp(&b.1));
        match (near, piece) {
            (None, _) => return false,
            (Some((i, _)), None) => piece = Some(i),
            (Some((i, _)), Some(j)) if i != j => return false,
            _ => {}
        }
    }
    true
}

/// The net passage between doors: the largest width found by halving from 0 to `width`, to within the precision.
pub fn passage(areas: &[Area], doors: &[(V, V)], width: f64) -> f64 {
    // The net width itself, within the precision: a corridor as wide all along passes whole.
    if passes(areas, doors, width - PRECISION) {
        return width;
    }
    let (mut lo, mut hi) = (0.0, width);
    for _ in 0..HALVINGS {
        if hi - lo <= PRECISION {
            break;
        }
        let mid = (lo + hi) / 2.0;
        if passes(areas, doors, mid) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    lo
}

/// The closest points of two segments.
fn closest(a: (V, V), b: (V, V)) -> (f64, V, V) {
    let (dx, dy) = (a.1.x - a.0.x, a.1.y - a.0.y);
    let (ex, ey) = (b.1.x - b.0.x, b.1.y - b.0.y);
    let la = dx.hypot(dy);
    let lb = ex.hypot(ey);
    if (dx * ey - dy * ex).abs() <= 1e-12 * la * lb {
        // Parallel: the middle of their overlap along a, if any.
        let (ux, uy) = (dx / la, dy / la);
        let k0 = ux * (b.0.x - a.0.x) + uy * (b.0.y - a.0.y);
        let k1 = ux * (b.1.x - a.0.x) + uy * (b.1.y - a.0.y);
        let (lo, hi) = (k0.min(k1).max(0.0), k0.max(k1).min(la));
        if hi >= lo {
            let k = (lo + hi) / 2.0;
            let p = V::new(a.0.x + ux * k, a.0.y + uy * k);
            let c = closest_on_edge(&kentos_geometry_core::geom::intersect::Edge::Seg { a: b.0, b: b.1 }, p);
            return (c.d, p, c.p);
        }
    }
    let seg = |s: (V, V)| kentos_geometry_core::geom::intersect::Edge::Seg { a: s.0, b: s.1 };
    let mut best = (f64::INFINITY, a.0, b.0);
    for (from, to, flip) in [(a.0, b, false), (a.1, b, false), (b.0, a, true), (b.1, a, true)] {
        let c = closest_on_edge(&seg(to), from);
        if c.d < best.0 {
            best = if flip { (c.d, c.p, from) } else { (c.d, from, c.p) };
        }
    }
    best
}

/// Where the passage is narrowest: the middle of the closest points of the two facing sides whose distance is nearest
/// the passage (the first such in their order).
pub fn pinch(sides: &[(V, V)], width: f64) -> Option<V> {
    let mut best: Option<(f64, V)> = None;
    for (i, &a) in sides.iter().enumerate() {
        for &b in &sides[i + 1..] {
            if a.0 == b.0 || a.0 == b.1 || a.1 == b.0 || a.1 == b.1 {
                continue;
            }
            let (d, p, q) = closest(a, b);
            if d <= 1e-9 {
                continue;
            }
            // Facing: each lies on the other's inside (its left).
            let left = |s: (V, V), x: V| (s.1.x - s.0.x) * (x.y - s.0.y) - (s.1.y - s.0.y) * (x.x - s.0.x) > 1e-12;
            if !(left(a, q) && left(b, p)) {
                continue;
            }
            let off = (d - width).abs();
            if best.is_none_or(|(o, _)| off < o - 1e-12) {
                best = Some((off, V::new((p.x + q.x) / 2.0, (p.y + q.y) / 2.0)));
            }
        }
    }
    best.map(|(_, m)| m)
}

/// The net width: the largest circle inside, its diameter (the pole of inaccessibility, to within the precision), and
/// its centre: where a finding on the space points.
pub fn width_of(parts: &[kentos_commands::SpacePart]) -> (f64, Option<V>) {
    use kentos_geometry_core::tessellate::bulge_path;
    parts
        .iter()
        .filter_map(|p| {
            let rings: Vec<Vec<V>> = std::iter::once(&p.outer)
                .chain(&p.holes)
                .map(|r| {
                    let pts: Vec<V> = r.pts.iter().map(core).collect();
                    bulge_path(&pts, r.bulges.as_deref(), true, kentos_geometry_core::geom::curve_outline::CURVE_TOL)
                })
                .collect();
            kentos_geometry_core::labels::geom::pole(&rings, PRECISION).map(|(p, d)| {
                // The outline's centroid when it is as good (a rectangle's middle, not a point along its long axis).
                let c = centroid(&rings[0]);
                let inside = c.is_some_and(|c| rings_hold(&rings, c));
                match c.filter(|_| inside).map(|c| (c, ring_distance(&rings, c))) {
                    Some((c, dc)) if dc >= d - PRECISION => (2.0 * d, c),
                    _ => (2.0 * d, p),
                }
            })
        })
        .fold((0.0, None), |best, (w, p)| if w > best.0 { (w, Some(p)) } else { best })
}

fn centroid(ring: &[V]) -> Option<V> {
    let n = ring.len();
    let (mut a, mut cx, mut cy) = (0.0, 0.0, 0.0);
    for i in 0..n {
        let (p, q) = (ring[i], ring[(i + 1) % n]);
        let k = p.x * q.y - q.x * p.y;
        a += k;
        cx += (p.x + q.x) * k;
        cy += (p.y + q.y) * k;
    }
    (a.abs() > 1e-12).then(|| V::new(cx / (3.0 * a), cy / (3.0 * a)))
}

/// Whether a point lies inside the first ring and outside the others (even–odd).
fn rings_hold(rings: &[Vec<V>], p: V) -> bool {
    let mut inside = false;
    for ring in rings {
        let n = ring.len();
        for i in 0..n {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
                inside = !inside;
            }
        }
    }
    inside
}

fn ring_distance(rings: &[Vec<V>], p: V) -> f64 {
    rings
        .iter()
        .flat_map(|r| (0..r.len()).map(move |i| (r[i], r[(i + 1) % r.len()])))
        .map(|(a, b)| closest_on_edge(&kentos_geometry_core::geom::intersect::Edge::Seg { a, b }, p).d)
        .fold(f64::INFINITY, f64::min)
}

/// Turkish words for an area, a length.
fn m2(v: f64) -> String {
    format!("{} m²", fixed(v, 2))
}

fn m(v: f64) -> String {
    format!("{} m", fixed(v, 2))
}

fn signed(v: f64) -> String {
    if v > 0.0 { format!("+{}", m2(v)) } else { m2(v) }
}

fn space_words(s: &SpaceEntity) -> String {
    let f = &s.space;
    match (f.number.as_deref(), f.name.as_deref().or(f.usage.as_deref())) {
        (Some(n), Some(w)) => format!("“{n} {w}”"),
        (Some(n), None) => format!("“{n}”"),
        (None, Some(w)) => format!("“{w}”"),
        (None, None) => "Mahal".into(),
    }
}

struct Found {
    finding: ArchAuditFinding,
}

fn finding(code: &str, uids: Vec<String>, at: V, message: String, repair: String, fix: Option<ArchAuditFix>) -> Found {
    let key = format!("{code}:{}", uids.join(","));
    // To the centimetre: an exception holds while a move under it changes neither the words nor the place
    // (docs/adr/0773 §5); a meaningful change of the geometry lets it lapse.
    let at = V::new((at.x * 100.0).round() / 100.0, (at.y * 100.0).round() / 100.0);
    Found {
        finding: ArchAuditFinding {
            key,
            check: ArchAuditCheck::Program,
            code: code.to_owned(),
            message,
            repair,
            uids,
            at: wire(at),
            fix,
            exception: false,
            lapsed: false,
            note: None,
        },
    }
}

/// The block's own box (its base at the origin), and the insert's placement.
fn fixture_box(doc: &Document, i: &kentos_commands::InsertEntity) -> Option<[V; 4]> {
    let [x0, y0, x1, y1] = doc.block_box(i.block)?;
    Some([V::new(x0, y0), V::new(x1, y0), V::new(x1, y1), V::new(x0, y1)])
}

fn placed(i: &kentos_commands::InsertEntity, p: V) -> V {
    let m = kentos_geometry_core::block::placement_xy(
        V::new(i.p.x, i.p.y),
        i.scale,
        i.y_scale.unwrap_or(i.scale),
        i.rotation,
        i.mirror,
    );
    V::new(m[0] * p.x + m[2] * p.y + m[4], m[1] * p.x + m[3] * p.y + m[5])
}

fn quad_area(pts: [V; 4]) -> Area {
    use kentos_geometry_core::geom::arrangement::Ring;
    let mut pts = pts.to_vec();
    let signed: f64 = (0..4).map(|k| pts[k].x * pts[(k + 1) % 4].y - pts[(k + 1) % 4].x * pts[k].y).sum();
    if signed < 0.0 {
        pts.reverse();
    }
    Area { outer: Ring { pts, bulges: None }, holes: Vec::new() }
}

/// What is read: the check's input checked, the rooms and the program.
fn read<'a>(doc: &'a Document, input: &ProgramCheck) -> Result<(BuildingProgram, Vec<Room<'a>>), Stop> {
    let program = doc.program();
    if let Some(units) = &input.units {
        for (i, u) in units.iter().enumerate() {
            if program.unit(u).is_none() {
                return Err(fail(codes::UNIT_NOT_FOUND, format!("“{u}” bölümü bina programında yok."), &format!("units[{i}]")));
            }
        }
    }
    if let Some(levels) = &input.levels {
        let known = doc.settings().arch.as_ref().map(|a| a.levels.clone()).unwrap_or_default();
        for (i, l) in levels.iter().enumerate() {
            if !known.iter().any(|k| &k.id == l) {
                return Err(fail(codes::LEVEL_NOT_FOUND, format!("“{l}” katı projede yok."), &format!("levels[{i}]")));
            }
        }
    }
    let mut rooms = Vec::new();
    for e in doc.entities() {
        let Entity::Space(s) = e else { continue };
        let f = &s.space;
        if let Some(units) = &input.units
            && !f.unit.as_ref().is_some_and(|u| units.contains(u))
        {
            continue;
        }
        if let Some(levels) = &input.levels
            && !f.arch.level.as_ref().is_some_and(|l| levels.contains(l))
        {
            continue;
        }
        let slot = Slot(s.base.id);
        let areas = spaces::areas(&f.parts);
        let area = areas.iter().map(net_area).sum();
        rooms.push(Room {
            slot,
            uid: doc.uid(slot).map(|u| u.to_string()).unwrap_or_default(),
            s,
            areas,
            area,
            level: f.arch.level.clone(),
            sides: sides_of(&f.parts),
        });
    }
    Ok((program, rooms))
}

/// The check; `stopped` is asked now and then with how far it has come (done, total): true stops it.
pub(crate) fn checked(
    doc: &Document,
    input: &ProgramCheck,
    stopped: &dyn Fn(u64, u64) -> bool,
) -> Result<ProgramChecked, Stop> {
    let (program, rooms) = read(doc, input)?;
    let with_findings = input.findings.unwrap_or(true);
    let rule = program.rule();
    let all_passages = passages::passages(doc);
    let total = rooms.len() as u64 * 2;
    let mut done = 0u64;
    let step = |d: &mut u64| -> Result<(), Stop> {
        *d += 1;
        if stopped(*d, total) {
            return Err(Stop::Cancelled);
        }
        Ok(())
    };

    // Each room's doors: (passage, point), and its openings into others: (room, point).
    let n = rooms.len();
    let mut doors: Vec<Vec<(usize, (V, V))>> = vec![Vec::new(); n];
    let mut by_passage: HashMap<usize, Vec<(usize, f64)>> = HashMap::new();
    for (ri, r) in rooms.iter().enumerate() {
        for (pi, p) in all_passages.iter().enumerate() {
            if !passages::reads(r.level.as_deref(), p.level.as_deref()) {
                continue;
            }
            if let Some((_, side)) = passages::along(&r.s.space.parts, p) {
                doors[ri].push((pi, door_span(p, side)));
                by_passage.entry(pi).or_default().push((ri, side));
            }
        }
        step(&mut done)?;
    }
    let mut open: Vec<Vec<(usize, (V, V))>> = vec![Vec::new(); n];
    let mut near: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut connected: Vec<Vec<usize>> = vec![Vec::new(); n];
    for i in 0..n {
        for j in i + 1..n {
            if rooms[i].level != rooms[j].level {
                continue;
            }
            if let Some(mid) = facing(&rooms[i].sides, &rooms[j].sides, OPEN_GAP) {
                open[i].push((j, mid));
                open[j].push((i, mid));
                connected[i].push(j);
                connected[j].push(i);
            }
            if facing(&rooms[i].sides, &rooms[j].sides, ADJACENT_GAP).is_some() {
                near[i].push(j);
                near[j].push(i);
            }
        }
    }
    for list in by_passage.values() {
        for (a, &(i, si)) in list.iter().enumerate() {
            for &(j, sj) in &list[a + 1..] {
                if i != j && si * sj < 0.0 && !connected[i].contains(&j) {
                    connected[i].push(j);
                    connected[j].push(i);
                }
            }
        }
    }

    let mut rows = Vec::with_capacity(n);
    let mut found: Vec<Found> = Vec::new();
    for (ri, r) in rooms.iter().enumerate() {
        let f = &r.s.space;
        let brief = f.usage.as_deref().and_then(|u| program.brief(u));
        let (width, middle) = width_of(&f.parts);
        let points: Vec<(V, V)> = doors[ri].iter().map(|d| d.1).chain(open[ri].iter().map(|o| o.1)).collect();
        let count = points.len() as u32;
        let (passage_w, passage_at) = match brief.and_then(|b| b.min_passage) {
            Some(_) if with_findings && points.len() >= 2 && !f.void => {
                let w = passage(&r.areas, &points, width);
                (Some(w), pinch(&r.sides, w))
            }
            _ => (None, None),
        };
        let difference = brief.and_then(|b| b.area_difference(r.area)).filter(|_| !f.void);
        let links = |list: &[String], among: &[usize]| -> Vec<ProgramLink> {
            list.iter()
                .map(|usage| ProgramLink {
                    usage: usage.clone(),
                    by: among
                        .iter()
                        .copied()
                        .filter(|&j| rooms[j].s.space.usage.as_deref() == Some(usage.as_str()))
                        .map(|j| (rooms[j].slot.0, rooms[j].uid.clone()))
                        .collect::<BTreeMap<_, _>>()
                        .into_values()
                        .collect(),
                })
                .collect()
        };
        let adjacent = brief.map(|b| links(&b.adjacent, &near[ri])).unwrap_or_default();
        let reach = brief.map(|b| links(&b.reach, &connected[ri])).unwrap_or_default();
        let at = middle.or_else(|| spaces::label_point(&f.parts).map(|p| core(&p))).unwrap_or(V::new(0.0, 0.0));
        let words = space_words(r.s);
        if with_findings {
            if let Some(u) = &f.unit
                && program.unit(u).is_none()
            {
                found.push(finding(
                    codes::PROGRAM_UNIT_MISSING,
                    vec![r.uid.clone()],
                    at,
                    format!("{words} “{u}” bölümüne bağlı; bina programında bu bölüm yok."),
                    "Bölümü Bina programına ekleyin ya da mahali bölümden çıkarın.".into(),
                    Some(ArchAuditFix {
                        label: "Bölümden çıkar".into(),
                        step: ChangesetStep {
                            command: kentos_commands::CAD_SPACES_EDIT.into(),
                            version: None,
                            input: json!({ "uids": [r.uid], "operation": { "kind": "set", "unit": null } }),
                            name: None,
                        },
                    }),
                ));
            }
            if let (Some(b), Some(d)) = (brief, difference)
                && d != 0.0
            {
                let range = match (b.area_min, b.area_max) {
                    (Some(lo), Some(hi)) => format!("{}–{}", fixed(lo, 2), m2(hi)),
                    (Some(lo), None) => format!("en az {}", m2(lo)),
                    (None, Some(hi)) => format!("en çok {}", m2(hi)),
                    (None, None) => String::new(),
                };
                let (code, repair) = if d < 0.0 {
                    (codes::PROGRAM_AREA_BELOW, "Mahali büyütün ya da tasarım hedefini değiştirin.")
                } else {
                    (codes::PROGRAM_AREA_ABOVE, "Mahali küçültün ya da tasarım hedefini değiştirin.")
                };
                found.push(finding(
                    code,
                    vec![r.uid.clone()],
                    at,
                    format!(
                        "{words}: {} alan {}; tasarım hedefi {range}, fark {} ({rule}; yasal sınır değil).",
                        f.method.label(),
                        m2(r.area),
                        signed(d)
                    ),
                    repair.into(),
                    None,
                ));
            }
            if let Some(min) = brief.and_then(|b| b.min_width)
                && !f.void
                && width < min - PRECISION
            {
                found.push(finding(
                    codes::PROGRAM_WIDTH_NARROW,
                    vec![r.uid.clone()],
                    at,
                    format!("{words}: net en {}; tasarım hedefi en az {}, fark {} ({rule}).", m(width), m(min), m(width - min)),
                    "Mahali genişletin ya da tasarım hedefini değiştirin.".into(),
                    None,
                ));
            }
            if let (Some(min), Some(w)) = (brief.and_then(|b| b.min_passage), passage_w)
                && w < min - PRECISION
            {
                found.push(finding(
                    codes::PROGRAM_PASSAGE_NARROW,
                    vec![r.uid.clone()],
                    passage_at.unwrap_or(at),
                    format!(
                        "{words}: kapılar arası net geçiş {}; tasarım hedefi en az {}, fark {} ({rule}).",
                        m(w),
                        m(min),
                        m(w - min)
                    ),
                    "Dar yeri genişletin (duvarı, kolonu ya da donatıyı kaydırın) ya da tasarım hedefini değiştirin.".into(),
                    None,
                ));
            }
            for l in &adjacent {
                if l.by.is_empty() {
                    found.push(finding(
                        codes::PROGRAM_ADJACENT_MISSING,
                        vec![r.uid.clone()],
                        at,
                        format!("{words}: tasarım hedefi “{}” komşusu istiyor; yanında öyle bir mahal yok ({rule}).", l.usage),
                        "Komşu mahali bu mahalin yanına alın ya da tasarım hedefini değiştirin.".into(),
                        None,
                    ));
                }
            }
            for l in &reach {
                if l.by.is_empty() {
                    found.push(finding(
                        codes::PROGRAM_REACH_MISSING,
                        vec![r.uid.clone()],
                        at,
                        format!("{words}: tasarım hedefi “{}” mahaline geçiş istiyor; kapı ya da açık geçiş yok ({rule}).", l.usage),
                        "Aradaki duvara kapı koyun ya da mahalleri ayırıcıyla açın.".into(),
                        None,
                    ));
                }
            }
            let grouped = f.unit.as_deref().and_then(|u| program.unit(u)).is_some_and(|u| u.kind != UnitKind::Outdoor);
            if grouped && !f.void && count == 0 {
                found.push(finding(
                    codes::PROGRAM_NO_DOOR,
                    vec![r.uid.clone()],
                    at,
                    format!("{words}: kapısı ya da başka mahale açık geçişi yok ({rule})."),
                    "Mahale kapı koyun ya da komşusuna mahal ayırıcıyla açın.".into(),
                    None,
                ));
            }
        }
        rows.push(ProgramSpaceRow {
            uid: r.uid.clone(),
            number: f.number.clone(),
            name: f.name.clone(),
            usage: f.usage.clone(),
            unit: f.unit.clone(),
            level: f.arch.level.clone(),
            method: f.method,
            void: f.void,
            outdoor: brief.is_some_and(|b| b.outdoor),
            area: r.area,
            width,
            doors: count,
            passage: passage_w,
            passage_at: passage_at.map(wire),
            target_min: brief.and_then(|b| b.area_min),
            target_max: brief.and_then(|b| b.area_max),
            target_difference: difference,
            passage_difference: passage_w.zip(brief.and_then(|b| b.min_passage)).map(|(w, m)| w - m),
            adjacent,
            reach,
        });
        step(&mut done)?;
    }

    // The fixtures' clearances.
    if with_findings && !program.clearances.is_empty() {
        let mut boxes: Vec<FixtureBox> = Vec::new();
        for e in doc.entities() {
            let Entity::Insert(i) = e else { continue };
            let Some(name) = doc.blocks().iter().find(|b| b.id == i.block).map(|b| b.name.clone()) else { continue };
            let Some(local) = fixture_box(doc, i) else { continue };
            let foot = quad_area(local.map(|p| placed(i, p)));
            let zone = program.clearance(&name).map(|c| {
                let (x0, x1, y1) = (local[0].x, local[1].x, local[2].y);
                let cx = (x0 + x1) / 2.0;
                let (sx, sy) = (i.scale.abs().max(1e-12), i.y_scale.unwrap_or(i.scale).abs().max(1e-12));
                // The clearance is in the drawing's lengths: in the block's own frame it is divided by its scale.
                let (hw, d) = (c.width / 2.0 / sx, c.depth / sy);
                quad_area([V::new(cx - hw, y1), V::new(cx + hw, y1), V::new(cx + hw, y1 + d), V::new(cx - hw, y1 + d)].map(|p| placed(i, p)))
            });
            let uid = doc.uid(Slot(i.base.id)).map(|u| u.to_string()).unwrap_or_default();
            let middle = doc.fixture_middle(i);
            boxes.push((uid, i.arch.level.clone(), foot, zone, core(&middle)));
        }
        for (k, (uid, level, _, zone, middle)) in boxes.iter().enumerate() {
            let Some(zone) = zone else { continue };
            let Some(room) = rooms.iter().find(|r| r.level == *level && !r.s.space.void && r.areas.iter().any(|a| inside_area(a, *middle))) else {
                continue;
            };
            let whole = net_area(zone);
            let within: f64 = room
                .areas
                .iter()
                .map(|a| intersect_areas(&[zone.clone(), a.clone()]).iter().map(net_area).sum::<f64>())
                .sum();
            let out = (whole - within).max(0.0);
            let mut others: Vec<String> = Vec::new();
            for (j, (other, olevel, foot, _, _)) in boxes.iter().enumerate() {
                if j == k || olevel != level {
                    continue;
                }
                let over: f64 = intersect_areas(&[zone.clone(), foot.clone()]).iter().map(net_area).sum();
                if over > 1e-6 {
                    others.push(other.clone());
                }
            }
            if out > 1e-6 || !others.is_empty() {
                let mut uids = vec![uid.clone()];
                uids.extend(others.iter().cloned());
                let centre = {
                    let p = &zone.outer.pts;
                    V::new(p.iter().map(|q| q.x).sum::<f64>() / 4.0, p.iter().map(|q| q.y).sum::<f64>() / 4.0)
                };
                let mut words = Vec::new();
                if out > 1e-6 {
                    words.push(format!("{} mahalin dışında", m2(out)));
                }
                if !others.is_empty() {
                    words.push(format!("{} donatıya biniyor", others.len()));
                }
                found.push(finding(
                    codes::PROGRAM_FIXTURE_CLEARANCE,
                    uids,
                    centre,
                    format!(
                        "Donatının kullanım alanı ({}) {}; {} içinde ({rule}; tasarım hedefi).",
                        m2(whole),
                        words.join(", "),
                        space_words(room.s)
                    ),
                    "Donatıyı kaydırın ya da döndürün, önündeki alanı boşaltın.".into(),
                    None,
                ));
            }
        }
    }

    // The groups' totals: each space once, in its group or none.
    let mut units: Vec<ProgramUnitRow> = program
        .units
        .iter()
        .map(|u| ProgramUnitRow {
            id: u.id.clone(),
            number: u.number.clone(),
            name: u.name.clone(),
            kind: u.kind,
            building: u.building.clone(),
            wing: u.wing.clone(),
            levels: Vec::new(),
            spaces: 0,
            rooms: 0.0,
            outdoor: 0.0,
            voids: 0.0,
            methods: Vec::new(),
            recorded_area: u.recorded_area,
            recorded_difference: None,
        })
        .collect();
    let mut totals = ProgramTotals::default();
    let order: Vec<String> = doc.settings().arch.as_ref().map(|a| a.levels.iter().map(|l| l.id.clone()).collect()).unwrap_or_default();
    for (r, row) in rooms.iter().zip(&rows) {
        let group = r.s.space.unit.as_deref().and_then(|u| units.iter().position(|x| x.id == u));
        if !row.void {
            totals.all += row.area;
        }
        let Some(g) = group else {
            if row.void {
                totals.voids += row.area;
            } else {
                totals.unassigned += row.area;
            }
            continue;
        };
        let u = &mut units[g];
        u.spaces += 1;
        if let Some(l) = &row.level
            && !u.levels.contains(l)
        {
            u.levels.push(l.clone());
        }
        if !u.methods.contains(&row.method) {
            u.methods.push(row.method);
        }
        if row.void {
            u.voids += row.area;
            totals.voids += row.area;
        } else if row.outdoor {
            u.outdoor += row.area;
        } else {
            u.rooms += row.area;
        }
        if !row.void {
            match u.kind {
                UnitKind::Unit if row.outdoor => totals.unit_outdoor += row.area,
                UnitKind::Unit => totals.units += row.area,
                UnitKind::Common => totals.common += row.area,
                UnitKind::Outdoor => totals.outdoor += row.area,
            }
        }
    }
    for u in &mut units {
        u.levels.sort_by_key(|l| order.iter().position(|o| o == l).unwrap_or(usize::MAX));
        u.recorded_difference = u.recorded_area.map(|a| u.rooms - a);
    }
    if let Some(filter) = &input.units {
        units.retain(|u| filter.contains(&u.id));
    }
    let code_rank = |c: &str| CODES.iter().position(|x| *x == c).unwrap_or(usize::MAX);
    let mut findings: Vec<ArchAuditFinding> = found.into_iter().map(|f| f.finding).collect();
    findings.sort_by(|a, b| {
        code_rank(&a.code)
            .cmp(&code_rank(&b.code))
            .then(a.at.x.total_cmp(&b.at.x))
            .then(a.at.y.total_cmp(&b.at.y))
            .then(a.key.cmp(&b.key))
    });
    // A second finding alike gets its key's `#2` (docs/adr/0732 §1).
    let mut seen: HashMap<String, u32> = HashMap::new();
    for f in &mut findings {
        let n = seen.entry(f.key.clone()).or_insert(0);
        *n += 1;
        if *n > 1 {
            f.key = format!("{}#{n}", f.key);
        }
    }
    Ok(ProgramChecked { rule, units, totals, spaces: rows, findings, revision: doc.revision().to_string() })
}

/// The program group's findings, for Mimari denetim.
pub fn findings(doc: &Document) -> Vec<ArchAuditFinding> {
    if doc.program().is_empty() {
        return Vec::new();
    }
    checked(doc, &ProgramCheck::default(), &|_, _| false).map(|c| c.findings).unwrap_or_default()
}

pub fn validate(cx: &ExecutionContext<'_>, input: &ProgramCheck) -> CommandResult<()> {
    match read(cx.doc, input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &ProgramCheck) -> CommandResult<ProgramChecked> {
    run(cx, input)
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: ProgramCheck) -> CommandResult<ProgramChecked> {
    run(cx, &input)
}

fn run(cx: &ExecutionContext<'_>, input: &ProgramCheck) -> CommandResult<ProgramChecked> {
    let stopped = |done: u64, total: u64| {
        cx.progress(done, total);
        cx.stopped()
    };
    match checked(cx.doc, input, &stopped) {
        Ok(out) => CommandResult::Completed { output: out, warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// [`checked`] watched: `stopped(done, total)` is asked as it goes; true stops it (`cancelled`).
pub fn check_watched(
    doc: &Document,
    input: &ProgramCheck,
    stopped: &dyn Fn(u64, u64) -> bool,
) -> CommandResult<ProgramChecked> {
    match checked(doc, input, stopped) {
        Ok(out) => CommandResult::Completed { output: out, warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// [`checked`] on a document with nothing watching.
pub fn check_in(doc: &Document, input: &ProgramCheck) -> CommandResult<ProgramChecked> {
    match checked(doc, input, &|_, _| false) {
        Ok(out) => CommandResult::Completed { output: out, warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

