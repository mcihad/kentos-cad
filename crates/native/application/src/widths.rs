//! A polyline's widths in what `cad.entities.edit` writes (docs/adr/0258 §2).
//! The core's geometry comes without them, so each edge of a changed or new
//! polyline takes its start and end width from the polylines the edit names,
//! by the shared core's rules (`kentos_geometry_core::ops::widths`), as its
//! vertices take their elevations (`elevation`).

use kentos_contracts::{Entity, Vec2 as Point};
use kentos_geometry_core::Vec2;
use kentos_geometry_core::ops::elevation::Carry;
use kentos_geometry_core::ops::widths::{Widened, carry_widths};

fn v(p: &Point) -> Vec2 {
    Vec2::new(p.x, p.y)
}

/// A polyline's path with its widths; none for another kind or a polyline without them.
pub fn source(e: &Entity) -> Option<Widened> {
    let Entity::Polyline(p) = e else {
        return None;
    };
    let widths = p.widths.clone()?;
    Some(Widened {
        pts: p.pts.iter().map(v).collect(),
        bulges: p.bulges.clone(),
        closed: p.closed,
        widths,
    })
}

/// `entity` with the widths its edges take from `sources` and from `same`,
/// the polyline it replaces. Whether it got any: only a polyline of one part
/// does (a closed or wide polyline is of one part, docs/adr/0258 §1).
pub fn carry(entity: &mut Entity, sources: &[Widened], same: Option<&Widened>, how: Carry) -> bool {
    let Entity::Polyline(p) = entity else {
        return false;
    };
    if p.parts.is_some() {
        p.widths = None;
        return false;
    }
    let pts: Vec<Vec2> = p.pts.iter().map(v).collect();
    p.widths = carry_widths(&pts, p.bulges.as_deref(), p.closed, same, sources, how);
    p.widths.is_some()
}
