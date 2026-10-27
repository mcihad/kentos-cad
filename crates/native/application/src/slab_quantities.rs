//! The quantities a slab and a room's finishes add to `cad.spaces.quantities` (docs/adr/0722 §5).
//!
//! - A passage is a door, or a gap (Boşluk) with no sill above the floor, on a wall (its jambs known). Its axis runs
//!   between its jambs' middles; its band is its wall's thickness about the axis.
//! - A room's baseboard is its rings' length less, for each passage it reads (on its level; every one when it has no
//!   level, as it reads every wall), the part of the passage's axis that a straight edge of its rings runs along inside
//!   the band (parallel to the axis): the doorway it has no baseboard along. An arc edge takes nothing off.
//! - A transition is a passage with a room on each side (the side the room's inside lies on, by its ring's
//!   orientation) whose floor finishes are both known and differ: the strip is the passage's width.
//! - A slab: its rings' area, holes off; the area the voids it reads (rooms marked void, docs/adr/0708: those of its
//!   level, every one when it has none) take out of it, the voids united first; its net area the difference; its perimeter; its volume net area × thickness; its
//!   top its level's elevation and its offset; its edge's runs' length. One left out (`excluded`) is listed, not summed.

use kentos_commands::{ArchInfo, Entity, OpeningForm, SlabFields, SlabRow, SpacePart, TransitionRow, Vec2};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::Vec2 as CoreVec2;
use kentos_geometry_core::geom::region::{intersect_area_sets, net_area, union_areas};

use crate::spaces;

/// How far off the axis's direction an edge may turn and still run along it (a sine), and how far past the band's side
/// it may lie (metres).
const PARALLEL: f64 = 1e-6;
const BAND: f64 = 1e-6;

/// A passage (docs/adr/0722 §5).
pub(crate) struct Passage {
    pub uid: String,
    pub number: Option<String>,
    pub width: f64,
    pub a: CoreVec2,
    pub b: CoreVec2,
    pub half: f64,
    pub level: Option<String>,
}

/// Whether an object on `level` reads a passage or a void on `other`: the same level, or every one without a level.
fn reads(level: Option<&str>, other: Option<&str>) -> bool {
    level.is_none() || level == other
}

/// The drawing's passages, in its order.
pub(crate) fn passages(doc: &Document) -> Vec<Passage> {
    let by_uid = |u: &str| {
        kentos_domain::Uuid::parse_str(u).ok().and_then(|u| doc.slot_of(u)).and_then(|s| doc.get(s))
    };
    doc.entities()
        .filter_map(|e| {
            let Entity::Opening(o) = e else { return None };
            let x = &o.opening;
            let passage = x.form == OpeningForm::Door
                || (x.form == OpeningForm::Opening && x.sill.is_none_or(|s| s <= 0.0));
            if !passage || x.place.len() != 4 {
                return None;
            }
            let p = &x.place;
            let mid = |s: &Vec2, t: &Vec2| CoreVec2::new((s.x + t.x) / 2.0, (s.y + t.y) / 2.0);
            Some(Passage {
                uid: doc.uid(Slot(e.base().id)).map(|u| u.to_string()).unwrap_or_default(),
                number: x.number.clone(),
                width: x.width,
                a: mid(&p[0], &p[1]),
                b: mid(&p[2], &p[3]),
                half: (p[0].x - p[1].x).hypot(p[0].y - p[1].y) / 2.0,
                level: e.level(by_uid).map(str::to_owned),
            })
        })
        .collect()
}

fn signed(pts: &[Vec2]) -> f64 {
    let n = pts.len();
    (0..n).map(|i| pts[i].x * pts[(i + 1) % n].y - pts[(i + 1) % n].x * pts[i].y).sum::<f64>() / 2.0
}

/// How much of `p`'s axis a room's straight edges run along inside its band, and the side the room lies on
/// (+1 left of the axis, −1 right) by its longest such edge; none when no edge does.
pub(crate) fn along(parts: &[SpacePart], p: &Passage) -> Option<(f64, f64)> {
    let (dx, dy) = (p.b.x - p.a.x, p.b.y - p.a.y);
    let len = dx.hypot(dy);
    if len.is_nan() || len <= 0.0 {
        return None;
    }
    let (ux, uy) = (dx / len, dy / len);
    let mut total = 0.0;
    let mut best: Option<(f64, f64)> = None;
    for part in parts {
        for (h, ring) in std::iter::once(&part.outer).chain(&part.holes).enumerate() {
            let n = ring.pts.len();
            // Its inside is to the left of its edges: an outer ring counter-clockwise, a hole clockwise.
            let left = (signed(&ring.pts) > 0.0) == (h == 0);
            for i in 0..n {
                if ring.bulges.as_ref().is_some_and(|b| b.get(i).copied().unwrap_or(0.0) != 0.0) {
                    continue;
                }
                let (s, t) = (ring.pts[i], ring.pts[(i + 1) % n]);
                let (ex, ey) = (t.x - s.x, t.y - s.y);
                let el = ex.hypot(ey);
                if el.is_nan() || el <= 0.0 || (ux * ey - uy * ex).abs() / el > PARALLEL {
                    continue;
                }
                let off = (ux * (s.y - p.a.y) - uy * (s.x - p.a.x)).abs();
                if off > p.half + BAND {
                    continue;
                }
                let (k0, k1) = (ux * (s.x - p.a.x) + uy * (s.y - p.a.y), ux * (t.x - p.a.x) + uy * (t.y - p.a.y));
                let over = k0.max(k1).min(len) - k0.min(k1).max(0.0);
                if over <= 0.0 {
                    continue;
                }
                total += over;
                let side = if (ux * ex + uy * ey > 0.0) == left { 1.0 } else { -1.0 };
                if best.is_none_or(|(o, _)| over > o) {
                    best = Some((over, side));
                }
            }
        }
    }
    best.map(|(_, side)| (total, side))
}

/// A room's baseboard: its rings' length less the passages along it (its level's).
pub(crate) fn baseboard(parts: &[SpacePart], level: Option<&str>, all: &[Passage]) -> f64 {
    let off: f64 = all
        .iter()
        .filter(|p| reads(level, p.level.as_deref()))
        .filter_map(|p| along(parts, p).map(|(o, _)| o))
        .sum();
    (spaces::perimeter_of(parts) - off).max(0.0)
}

/// A room as the floor changes read it: its uid, level, rings and floor finish.
pub(crate) type Room<'a> = (String, Option<String>, &'a [SpacePart], Option<String>);

/// The floor finishes' changes at the passages among `rooms`, in the drawing's order.
pub(crate) fn transitions(rooms: &[Room<'_>], all: &[Passage]) -> Vec<TransitionRow> {
    let mut out = Vec::new();
    for p in all {
        let mut sides: [Option<(f64, &String, &Option<String>)>; 2] = [None, None];
        for (uid, level, parts, floor) in rooms {
            if !reads(level.as_deref(), p.level.as_deref()) {
                continue;
            }
            let Some((over, side)) = along(parts, p) else { continue };
            let k = if side > 0.0 { 0 } else { 1 };
            if sides[k].is_none_or(|(o, ..)| over > o) {
                sides[k] = Some((over, uid, floor));
            }
        }
        if let [Some((_, l, Some(lf))), Some((_, r, Some(rf)))] = sides
            && lf != rf
            && l != r
        {
            out.push(TransitionRow {
                opening: p.uid.clone(),
                number: p.number.clone(),
                width: p.width,
                spaces: [l.clone(), r.clone()],
                floors: [lf.clone(), rf.clone()],
            });
        }
    }
    out
}

/// The elevation of `arch`'s level; none without one.
fn level_elevation(doc: &Document, arch: &ArchInfo) -> Option<f64> {
    let id = arch.level.as_deref()?;
    doc.settings().arch.as_ref()?.levels.iter().find(|l| l.id == id).map(|l| l.elevation)
}

/// A slab measured; `voids` the drawing's voids by level.
pub(crate) fn slab_row(doc: &Document, uid: String, s: &SlabFields, voids: &[(Option<String>, Vec<SpacePart>)]) -> SlabRow {
    let area = spaces::area_of(&s.parts);
    let mine = spaces::areas(&s.parts);
    let theirs: Vec<_> = voids
        .iter()
        .filter(|(level, _)| reads(s.arch.level.as_deref(), level.as_deref()))
        .flat_map(|(_, parts)| spaces::areas(parts))
        .collect();
    let void_area = if theirs.is_empty() {
        0.0
    } else {
        intersect_area_sets(&[mine, union_areas(&theirs)]).iter().map(net_area).sum::<f64>()
    };
    let net = area - void_area;
    let edge_length = s.edge.as_ref().map_or(0.0, |e| {
        e.runs
            .iter()
            .map(|r| {
                let pts: Vec<CoreVec2> = r.pts.iter().map(|p| CoreVec2::new(p.x, p.y)).collect();
                kentos_geometry_core::geom::bulge::bulge_path_length(&pts, r.bulges.as_deref(), false)
            })
            .sum()
    });
    let mut rule = "alan: halkaların alanı, delikler düşülür; boşluk: aynı kattaki boşluk mahallerinin örttüğü alan düşülür; hacim: net alan × kalınlık".to_owned();
    if s.edge.is_some() {
        rule.push_str("; kenar: serbest kenarlar boyunca");
    }
    if s.excluded {
        rule.push_str("; metraj dışı: toplamlara katılmaz");
    }
    SlabRow {
        uid,
        role: s.role,
        level: s.arch.level.clone(),
        material: s.material.clone(),
        thickness: s.thickness,
        offset: s.offset,
        top: level_elevation(doc, &s.arch).map(|e| e + s.offset),
        area,
        void_area,
        net_area: net,
        perimeter: spaces::perimeter_of(&s.parts),
        volume: net * s.thickness,
        edge: s.edge.as_ref().map(|e| e.kind),
        edge_length,
        excluded: s.excluded,
        rule,
    }
}
