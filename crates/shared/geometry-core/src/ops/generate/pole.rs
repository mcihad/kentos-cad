//! Ulaşılmazlık kutbu (docs/adr/0219 §6): the point inside an area farthest
//! from its edges, holes too, within a precision (Mapbox's polylabel, the
//! label engine's `labels::geom::pole`); a multi-part area's from the part
//! whose pole is farthest, a circle's its centre. Arcs are taken within 0.1 mm.

use super::points::CURVE_TOL;
use crate::entity::Shape;
use crate::labels::geom::pole as polylabel;
use crate::ops::geoprocess::{Class, class_of};
use crate::tessellate::bulge_path;
use crate::vec2::Vec2;

/// An area's pole and its distance from the area's edges; none for an object that is no area or has none.
pub fn pole(s: &Shape, precision: f64) -> Option<(Vec2, f64)> {
    if let Shape::Circle { c, r } = s {
        return (*r > 0.0).then_some((*c, *r));
    }
    let Class::Areas(areas) = class_of(s) else {
        return None;
    };
    let mut best: Option<(Vec2, f64)> = None;
    for a in &areas {
        let rings: Vec<Vec<Vec2>> = std::iter::once(&a.outer)
            .chain(&a.holes)
            .map(|r| bulge_path(&r.pts, r.bulges.as_deref(), true, CURVE_TOL))
            .collect();
        if let Some((p, d)) = polylabel(&rings, precision)
            && best.is_none_or(|(_, b)| d > b)
        {
            best = Some((p, d));
        }
    }
    best
}
