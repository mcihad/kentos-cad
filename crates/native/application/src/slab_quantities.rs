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

use kentos_commands::{ArchInfo, SlabFields, SlabRow, SpacePart, TransitionRow};
use kentos_domain::Document;
use kentos_geometry_core::Vec2 as CoreVec2;
use kentos_geometry_core::geom::region::{intersect_area_sets, net_area, union_areas};

use crate::spaces;
pub(crate) use kentos_domain::passages::{Passage, along, baseboard, passages, reads};

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
