//! `cad.entities.drape` v1 (plan/map-cad.md MAP-06, docs/adr/4180): Yüzeyden kot ver. Each vertex of the objects (a point, a line's ends, every part's and hole's of a polyline or an area) takes
//! the project's terrain surface's height under it, read by the surface's own query; a vertex off the surface keeps
//! its own. With `breaks` (ara kırık noktaları) a vertex with the surface's height goes wherever a
//! straight edge crosses an edge of the surface's triangles: in polylines and areas that carry no measures (M) nor
//! widths; an arc is not split, a line keeps its two ends. Docs/adr/4580: with `offset` every height is the surface's
//! plus it; with `range` only one object's vertices from `from` to `to` (a line's ends, a polyline's or an area's ring)
//! take heights, only the edges between them break points; docs/adr/4980: the ring is any part's outer ring or hole
//! (`part`, `ring`), the object's other rings keep theirs. The heights are not kept tied to the surface. One undo step. The shared cases are
//! `fixtures/commands/v1/cad.entities.drape.json`.

use std::collections::{BTreeSet, HashSet};

use kentos_commands::{
    CommandResult, DrapeRange, DrapedObject, EntitiesDrape, EntitiesDraped, Entity, MAX_DRAPED, PathEntity, Vec2,
};
use kentos_domain::{Document, Slot, Uuid};
use kentos_road::surface::Surface;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::road_terrains as terrains;

const STEP: &str = "Yüzeyden kot ver";

/// Two crossings of an edge closer than this along it (m) are one (a crossing through a triangle's corner is found on
/// each of its edges); one this near a vertex is the vertex itself.
const SAME: f64 = 1e-9;

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// The surface's triangles' edges, each once, as plan segments.
fn edges_of(s: &Surface) -> Vec<([f64; 2], [f64; 2])> {
    let mut seen: BTreeSet<(u32, u32)> = BTreeSet::new();
    for t in &s.triangles {
        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            seen.insert((a.min(b), a.max(b)));
        }
    }
    seen.into_iter()
        .map(|(a, b)| {
            let (p, q) = (s.vertices[a as usize], s.vertices[b as usize]);
            ([p[0], p[1]], [q[0], q[1]])
        })
        .collect()
}

/// Where segment pq crosses the surface's edges, as fractions along it strictly between its ends (a crossing within
/// [`SAME`] of an end is that end), sorted, those within [`SAME`] of each other once.
fn crossings(p: Vec2, q: Vec2, edges: &[([f64; 2], [f64; 2])]) -> Vec<f64> {
    let (rx, ry) = (q.x - p.x, q.y - p.y);
    let len = rx.hypot(ry);
    if !(len > 0.0) {
        return Vec::new();
    }
    let (lo_x, hi_x, lo_y, hi_y) = (p.x.min(q.x), p.x.max(q.x), p.y.min(q.y), p.y.max(q.y));
    let mut ts: Vec<f64> = Vec::new();
    for (c, d) in edges {
        if c[0].max(d[0]) < lo_x || c[0].min(d[0]) > hi_x || c[1].max(d[1]) < lo_y || c[1].min(d[1]) > hi_y {
            continue;
        }
        let (sx, sy) = (d[0] - c[0], d[1] - c[1]);
        let denom = rx * sy - ry * sx;
        if denom == 0.0 {
            continue;
        }
        let (wx, wy) = (c[0] - p.x, c[1] - p.y);
        let t = (wx * sy - wy * sx) / denom;
        let u = (wx * ry - wy * rx) / denom;
        if (0.0..=1.0).contains(&u) && t * len > SAME && (1.0 - t) * len > SAME {
            ts.push(t);
        }
    }
    ts.sort_by(f64::total_cmp);
    ts.dedup_by(|b, a| (*b - *a) * len <= SAME);
    ts
}

/// How a ring is draped: the height over the surface and the vertices taken (docs/adr/4580; none: all) of the ring
/// `at` (part, ring; docs/adr/4980).
#[derive(Clone, Copy)]
struct Over {
    offset: f64,
    range: Option<(usize, usize)>,
    at: (usize, usize),
}

impl Over {
    fn takes(&self, k: usize) -> bool {
        self.range.is_none_or(|(a, b)| a <= k && k <= b)
    }

    /// Whether the edge from vertex `k` (to the next) is in the range: the closing edge never is with a range.
    fn edge(&self, k: usize) -> bool {
        self.range.is_none_or(|(a, b)| a <= k && k < b)
    }
}

/// A ring's or a path's vertices draped: each taken takes the surface's height plus the offset (one off it keeps its
/// own; one not taken is left); with `split` each straight edge (bulge 0) taken is split where it crosses the surface's
/// edges, the new vertex with the surface's height plus the offset and no bulge. The new vertices, bulges (as long as
/// the old ones were, per vertex) and elevations; how many were set, how many are off the surface, how many went in.
#[allow(clippy::type_complexity)]
fn ring(
    pts: &[Vec2],
    bulges: Option<&Vec<f64>>,
    zs: Option<&Vec<Option<f64>>>,
    closed: bool,
    surface: &Surface,
    edges: Option<&[([f64; 2], [f64; 2])]>,
    over: Over,
) -> (Vec<Vec2>, Option<Vec<f64>>, Vec<Option<f64>>, u32, u32, u32) {
    let n = pts.len();
    let (mut set, mut outside, mut inserted) = (0u32, 0u32, 0u32);
    let mut out_pts = Vec::with_capacity(n);
    let mut out_zs = Vec::with_capacity(n);
    let mut out_bulges: Option<Vec<f64>> = bulges.map(|_| Vec::with_capacity(n));
    for k in 0..n {
        let p = pts[k];
        let own = zs.and_then(|z| z.get(k).copied().flatten());
        if !over.takes(k) {
            out_zs.push(own);
        } else {
            match surface.height(p.x, p.y).z {
                Some(h) => {
                    set += 1;
                    out_zs.push(Some(h + over.offset));
                }
                None => {
                    outside += 1;
                    out_zs.push(own);
                }
            }
        }
        out_pts.push(p);
        let bulge = bulges.and_then(|b| b.get(k).copied());
        if let (Some(list), Some(b)) = (out_bulges.as_mut(), bulge) {
            list.push(b);
        }
        // The edge from this vertex: to the next, or (closed) back to the first.
        let next = if k + 1 < n {
            Some(pts[k + 1])
        } else if closed && n > 2 {
            Some(pts[0])
        } else {
            None
        };
        let (Some(edges), Some(q)) = (edges, next) else { continue };
        if bulge.unwrap_or(0.0) != 0.0 || !over.edge(k) {
            continue;
        }
        for t in crossings(p, q, edges) {
            let at = Vec2 { x: p.x + t * (q.x - p.x), y: p.y + t * (q.y - p.y) };
            let Some(h) = surface.height(at.x, at.y).z else { continue };
            out_pts.push(at);
            out_zs.push(Some(h + over.offset));
            // An inserted vertex's edge is straight; the list keeps its length's meaning (per vertex) only when the old
            // one had this edge's entry.
            if let (Some(list), Some(_)) = (out_bulges.as_mut(), bulge) {
                list.push(0.0);
            }
            inserted += 1;
        }
    }
    (out_pts, out_bulges, out_zs, set, outside, inserted)
}

/// A ring's vertices, bulges, elevations and whether it closes, by its part (0: the object's own vertices, 1 on: the
/// further parts) and ring (0: the outer one, 1 on: the holes); none when there is no such ring (docs/adr/4980).
#[allow(clippy::type_complexity)]
fn ring_at(
    p: &mut PathEntity,
    area: bool,
    (part, ring): (usize, usize),
) -> Option<(&mut Vec<Vec2>, &mut Option<Vec<f64>>, &mut Option<Vec<Option<f64>>>, bool)> {
    let closed = area || p.closed;
    match (part, ring) {
        (0, 0) => Some((&mut p.pts, &mut p.bulges, &mut p.zs, closed)),
        (0, h) => p.holes.as_mut()?.get_mut(h - 1).map(|r| (&mut r.pts, &mut r.bulges, &mut r.zs, true)),
        (k, 0) => p.parts.as_mut()?.get_mut(k - 1).map(|q| (&mut q.pts, &mut q.bulges, &mut q.zs, area)),
        (k, h) => {
            let q = p.parts.as_mut()?.get_mut(k - 1)?;
            q.holes.as_mut()?.get_mut(h - 1).map(|r| (&mut r.pts, &mut r.bulges, &mut r.zs, true))
        }
    }
}

/// How many vertices the ring `at` has, or why there is none (the field and the message; docs/adr/4980).
fn ring_count(e: &Entity, (part, ring): (usize, usize)) -> Result<usize, (&'static str, String)> {
    let p = match e {
        Entity::Line(_) if part == 0 && ring == 0 => return Ok(2),
        Entity::Line(_) => {
            return Err((
                if part != 0 { "range.part" } else { "range.ring" },
                "Çizginin tek parçası ve tek halkası var: parça ve halka 0 olmalı.".into(),
            ));
        }
        Entity::Polyline(p) | Entity::Polygon(p) => p,
        _ => return Ok(1),
    };
    let parts = 1 + p.parts.as_ref().map_or(0, Vec::len);
    if part >= parts {
        return Err(("range.part", format!("Nesnenin {parts} parçası var (0'dan): {part}. parça yok.")));
    }
    let (outer, holes) = match part {
        0 => (p.pts.len(), p.holes.as_deref().unwrap_or(&[])),
        k => {
            let q = &p.parts.as_deref().unwrap_or(&[])[k - 1];
            (q.pts.len(), q.holes.as_deref().unwrap_or(&[]))
        }
    };
    match ring {
        0 => Ok(outer),
        h if h <= holes.len() => Ok(holes[h - 1].pts.len()),
        h => Err((
            "range.ring",
            format!("{part}. parçanın {} deliği var: {h}. halka yok (0 dış halka, 1'den delikler).", holes.len()),
        )),
    }
}

/// Elevations as stored: none when no vertex has one.
fn stored(zs: Vec<Option<f64>>) -> Option<Vec<Option<f64>>> {
    zs.iter().any(Option::is_some).then_some(zs)
}

/// An object draped, and its row; none for a kind without vertices. With a range (docs/adr/4580) only the vertices in it
/// of the ring it names (docs/adr/4980), a line's ends in it.
fn draped(
    e: &Entity,
    uid: &str,
    surface: &Surface,
    edges: Option<&[([f64; 2], [f64; 2])]>,
    over: Over,
) -> Option<(Entity, DrapedObject)> {
    let mut row = DrapedObject { uid: uid.to_owned(), set: 0, outside: 0, inserted: 0, unsplit: false };
    let mut next = e.clone();
    let count = |s: u32, o: u32, i: u32, row: &mut DrapedObject| {
        row.set += s;
        row.outside += o;
        row.inserted += i;
    };
    match &mut next {
        Entity::Point(p) => match surface.height(p.p.x, p.p.y).z {
            Some(h) => {
                p.z = Some(h + over.offset);
                count(1, 0, 0, &mut row);
            }
            None => count(0, 1, 0, &mut row),
        },
        Entity::Line(l) => {
            for (k, (at, z)) in [(l.a, &mut l.za), (l.b, &mut l.zb)].into_iter().enumerate() {
                if !over.takes(k) {
                    continue;
                }
                match surface.height(at.x, at.y).z {
                    Some(h) => {
                        *z = Some(h + over.offset);
                        count(1, 0, 0, &mut row);
                    }
                    None => count(0, 1, 0, &mut row),
                }
            }
        }
        Entity::Polyline(p) | Entity::Polygon(p) => {
            let area = matches!(e, Entity::Polygon(_));
            // Measures and widths are per vertex: a vertex put in would need its own; such a path is not split.
            let measured = p.ms.as_ref().is_some_and(|m| m.iter().any(Option::is_some))
                || p.parts.iter().flatten().any(|q| q.ms.as_ref().is_some_and(|m| m.iter().any(Option::is_some)))
                || p.widths.is_some();
            let split = if measured { None } else { edges };
            // With a range only the ring it names takes heights (docs/adr/4980); the check found it.
            if over.range.is_some() {
                let (pts, bulges, zs, closed) = ring_at(p, area, over.at)?;
                let (new_pts, new_bulges, new_zs, s, o, i) = ring(pts, bulges.as_ref(), zs.as_ref(), closed, surface, split, over);
                *pts = new_pts;
                *bulges = new_bulges;
                *zs = stored(new_zs);
                count(s, o, i, &mut row);
                row.unsplit = edges.is_some() && measured;
                return Some((next, row));
            }
            let (pts, bulges, zs, s, o, i) = ring(&p.pts, p.bulges.as_ref(), p.zs.as_ref(), area || p.closed, surface, split, over);
            p.pts = pts;
            p.bulges = bulges;
            p.zs = stored(zs);
            count(s, o, i, &mut row);
            let whole = Over { range: None, ..over };
            for h in p.holes.iter_mut().flatten() {
                let (pts, bulges, zs, s, o, i) = ring(&h.pts, h.bulges.as_ref(), h.zs.as_ref(), true, surface, split, whole);
                h.pts = pts;
                h.bulges = bulges;
                h.zs = stored(zs);
                count(s, o, i, &mut row);
            }
            for part in p.parts.iter_mut().flatten() {
                let (pts, bulges, zs, s, o, i) = ring(&part.pts, part.bulges.as_ref(), part.zs.as_ref(), area, surface, split, whole);
                part.pts = pts;
                part.bulges = bulges;
                part.zs = stored(zs);
                count(s, o, i, &mut row);
                for h in part.holes.iter_mut().flatten() {
                    let (pts, bulges, zs, s, o, i) = ring(&h.pts, h.bulges.as_ref(), h.zs.as_ref(), true, surface, split, whole);
                    h.pts = pts;
                    h.bulges = bulges;
                    h.zs = stored(zs);
                    count(s, o, i, &mut row);
                }
            }
            row.unsplit = edges.is_some() && measured;
        }
        _ => return None,
    }
    // A point or a line takes no break point.
    if edges.is_some() && matches!(e, Entity::Point(_) | Entity::Line(_)) {
        row.unsplit = true;
    }
    Some((next, row))
}

/// The objects draped and the answer, or why not.
fn check(doc: &Document, input: &EntitiesDrape) -> Result<(Vec<(Slot, Entity)>, EntitiesDraped), Stop> {
    checks::revision(doc, input.expected_revision.as_deref())?;
    terrains::terrain_project(doc)?;
    let id = crate::road_profiles::surface_id(doc, &input.surface, "surface")?;
    if input.uids.is_empty() || input.uids.len() > MAX_DRAPED {
        return Err(fail(
            codes::INVALID_INPUT,
            format!("1 ile {MAX_DRAPED} arasında nesne verin; {} verildi.", input.uids.len()),
            "uids",
        ));
    }
    let mut seen = HashSet::new();
    for (i, uid) in input.uids.iter().enumerate() {
        if !seen.insert(uid.as_str()) {
            return Err(fail(codes::INVALID_INPUT, format!("“{uid}” nesnesi iki kez verildi."), &format!("uids[{i}]")));
        }
    }
    // Docs/adr/4580: the height over the surface and the range.
    if input.offset.is_some_and(|v| !v.is_finite()) {
        return Err(fail(codes::INVALID_INPUT, "Yüzeyle kot farkı sonlu bir sayı olmalı (m).".into(), "offset"));
    }
    if input.range.is_some() && input.uids.len() != 1 {
        return Err(fail(
            codes::INVALID_INPUT,
            format!("Köşe aralığı tek bir nesneye verilir; {} nesne verildi.", input.uids.len()),
            "range",
        ));
    }
    let mut objects = Vec::with_capacity(input.uids.len());
    for (i, uid) in input.uids.iter().enumerate() {
        let path = format!("uids[{i}]");
        if !checks::is_uid_text(uid) {
            return Err(fail(
                codes::INVALID_UID,
                format!("“{uid}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir."),
                &path,
            ));
        }
        let Some(slot) = Uuid::parse_str(uid).ok().and_then(|u| doc.slot_of(u)) else {
            return Err(fail(codes::ENTITY_NOT_FOUND, format!("“{uid}” kimlikli nesne çizimde yok."), &path));
        };
        let Some(e) = doc.get(slot) else {
            return Err(fail(codes::ENTITY_NOT_FOUND, format!("“{uid}” kimlikli nesne çizimde yok."), &path));
        };
        if !matches!(e, Entity::Point(_) | Entity::Line(_) | Entity::Polyline(_) | Entity::Polygon(_)) {
            return Err(fail(
                codes::NOT_A_PATH,
                format!("“{uid}” nokta, çizgi, çoklu çizgi ya da alan değil: köşesi yok, yüzeyden kot alamaz."),
                &path,
            ));
        }
        if let Some(DrapeRange { from, to, part, ring }) = input.range {
            if matches!(e, Entity::Point(_)) {
                return Err(fail(
                    codes::NOT_A_PATH,
                    "Nokta tek köşelidir: köşe aralığı çizgi, çoklu çizgi ya da alanın köşelerine verilir.".into(),
                    "range",
                ));
            }
            let count = ring_count(e, (part as usize, ring as usize)).map_err(|(at, why)| fail(codes::WRONG_PLACE, why, at))?;
            if !(from < to && (to as usize) < count) {
                let whose = if part == 0 && ring == 0 { "nesnenin".to_owned() } else { format!("{part}. parçanın {ring}. halkasının") };
                return Err(fail(
                    codes::WRONG_PLACE,
                    format!("Köşe aralığı {from}–{to} olamaz: {whose} {count} köşesi var (0'dan), başlangıç bitişten önce olmalı."),
                    "range",
                ));
            }
        }
        objects.push((slot, e.clone(), uid.clone()));
    }
    let def = terrains::surfaces(doc).iter().find(|s| s.id == id).cloned().ok_or(Stop::Cancelled)?;
    let made = terrains::make(doc, &def);
    let surface = made.built()?;
    let edges = input.breaks.then(|| edges_of(surface));
    let over = Over {
        offset: input.offset.unwrap_or(0.0),
        range: input.range.map(|r| (r.from as usize, r.to as usize)),
        at: input.range.map_or((0, 0), |r| (r.part as usize, r.ring as usize)),
    };
    let mut changed = Vec::new();
    let mut rows = Vec::new();
    for (slot, e, uid) in &objects {
        let Some((after, row)) = draped(e, uid, surface, edges.as_deref(), over) else { continue };
        changed.push((*slot, after));
        rows.push(row);
    }
    let (set, outside, inserted) = rows.iter().fold((0u32, 0u32, 0u32), |a, r| (a.0 + r.set, a.1 + r.outside, a.2 + r.inserted));
    if set == 0 {
        return Err(fail(
            codes::OFF_SURFACE,
            match input.range {
                Some(r) => format!(
                    "Köşe aralığının ({}–{}) hiçbir köşesi “{}” yüzeyinin üstünde değil; aralığı yüzeyin içinde seçin.",
                    r.from, r.to, def.name
                ),
                None => format!("Nesnelerin hiçbir köşesi “{}” yüzeyinin üstünde değil; yüzeyin içindeki nesneleri seçin.", def.name),
            },
            "uids",
        ));
    }
    for (i, (_, e, _)) in objects.iter().enumerate() {
        if doc.layers().is_locked(&e.base().layer_id) {
            return Err(fail(
                codes::LAYER_LOCKED,
                format!("“{}” katmanı kilitli: nesnenin kotları yazılamaz. Kilidi açın.", doc.layers().path(&e.base().layer_id)),
                &format!("uids[{i}]"),
            ));
        }
    }
    let out = EntitiesDraped { surface: id, objects: rows, set, outside, inserted, revision: doc.revision().to_string() };
    Ok((changed, out))
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &EntitiesDrape) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// What execute would write; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &EntitiesDrape) -> CommandResult<EntitiesDraped> {
    match check(cx.doc, input) {
        Ok((_, out)) => CommandResult::Completed { output: out, warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

fn write(cx: &mut ExecutionContext<'_>, input: EntitiesDrape) -> CommandResult<EntitiesDraped> {
    let (changed, mut out) = match check(cx.doc, &input) {
        Ok(done) => done,
        Err(stop) => return stop.into(),
    };
    cx.doc.update_many(changed, STEP);
    out.revision = cx.doc.revision().to_string();
    CommandResult::Completed { output: out, warnings: Vec::new() }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: EntitiesDrape) -> CommandResult<EntitiesDraped> {
    crate::call::recorded(cx, kentos_commands::CAD_ENTITIES_DRAPE, kentos_commands::CAD_ENTITIES_DRAPE_VERSION, input, write)
}
