//! A mass in the section model (docs/adr/8178): a prism over its footprint from its bottom to its top, on its own
//! EK-4 layer (MM-XXXX-KUTLE), its cut drawn light and unfilled: a study's volume, not a material. The views, the
//! drawn sections, the elevations and the sun's shadows read it; the IFC skips it (its owner is no element's).

use super::*;

/// A mass's look: light, unfilled, on MM-XXXX-KUTLE.
fn look() -> Look {
    Look { key: "KUTLE".into(), layer: "MM-XXXX-KUTLE".into(), heavy: false, fill: None }
}

/// The solid of mass `x` (its storey's floor from `levels`) owned by `owner`.
pub(super) fn solid(x: &kentos_commands::MassFields, levels: &[(String, String, f64, Option<f64>)], owner: Option<String>, m: &mut Model) {
    let (floor, _) = floor_of(levels, x.arch.level.as_deref());
    let (bottom, top) = x.span(floor);
    let rings: Vec<Vec<P>> = x
        .parts
        .iter()
        .flat_map(|part| std::iter::once(&part.outer).chain(&part.holes))
        .map(|r| kentos_geometry_core::entity::polygon_ring(&r.pts.iter().map(|q| p(*q)).collect::<Vec<_>>(), r.bulges.as_deref()))
        .filter(|r| r.len() >= 3)
        .collect();
    if rings.is_empty() || top <= bottom {
        return;
    }
    let solid = Solid { rings, height: Height::Prism { bottom, top }, voids: Vec::new(), marks: Vec::new(), finish: None };
    push(m, solid, look(), owner);
}
