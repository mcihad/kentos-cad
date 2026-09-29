//! Elevations of what `cad.entities.edit` writes (docs/adr/0142). The
//! core's geometry comes without them, so each vertex of a changed or new
//! object takes one from the objects the edit names, by the shared core's
//! rules (`kentos_geometry_core::ops::elevation`). The web's is
//! `apps/web/src/product/elevation.ts`; both pass the shared cases.

use kentos_contracts::{Entity, Vec2 as Point};
use kentos_geometry_core::Vec2;
use kentos_geometry_core::ops::elevation::{Carry, Elevated, carry_elevations};

fn v(p: &Point) -> Vec2 {
    Vec2::new(p.x, p.y)
}

fn path(
    pts: &[Point],
    bulges: &Option<Vec<f64>>,
    closed: bool,
    zs: &Option<Vec<Option<f64>>>,
) -> Elevated {
    Elevated {
        pts: pts.iter().map(v).collect(),
        bulges: bulges.clone(),
        closed,
        zs: zs.clone().unwrap_or_else(|| vec![None; pts.len()]),
    }
}

/// An object's paths with their elevations, `None` for a vertex without
/// one: a line's two ends, a polyline, a polygon's outer ring then its
/// holes; nothing for the other kinds.
pub fn paths(e: &Entity) -> Vec<Elevated> {
    match e {
        Entity::Line(l) => vec![Elevated {
            pts: vec![v(&l.a), v(&l.b)],
            bulges: None,
            closed: false,
            zs: vec![l.za, l.zb],
        }],
        Entity::Polyline(p) => vec![path(&p.pts, &p.bulges, false, &p.zs)],
        Entity::Polygon(p) => std::iter::once(path(&p.pts, &p.bulges, true, &p.zs))
            .chain(
                p.holes
                    .iter()
                    .flatten()
                    .map(|h| path(&h.pts, &h.bulges, true, &h.zs)),
            )
            .collect(),
        _ => Vec::new(),
    }
}

/// Whether a vertex of the paths has an elevation.
pub fn elevated(paths: &[Elevated]) -> bool {
    paths.iter().any(|p| p.zs.iter().any(Option::is_some))
}

/// `entity` with the elevations its vertices take from `sources` and from
/// `same`, the paths of the object it replaces (in [`paths`]' order).
/// Whether any vertex got one: an object of a kind without elevations (an
/// arc, a circle …) gets none.
pub fn carry(entity: &mut Entity, sources: &[Elevated], same: &[Elevated], how: Carry) -> bool {
    let run = |pts: &[Point], closed: bool, k: usize| -> Option<Vec<Option<f64>>> {
        let pts: Vec<Vec2> = pts.iter().map(v).collect();
        let zs = carry_elevations(&pts, closed, same.get(k), sources, how);
        zs.iter().any(Option::is_some).then_some(zs)
    };
    match entity {
        Entity::Line(l) => {
            let zs = run(&[l.a, l.b], false, 0);
            (l.za, l.zb) = zs.map_or((None, None), |z| (z[0], z[1]));
            l.za.is_some() || l.zb.is_some()
        }
        Entity::Polyline(p) => {
            p.zs = run(&p.pts, false, 0);
            p.zs.is_some()
        }
        Entity::Polygon(p) => {
            p.zs = run(&p.pts, true, 0);
            let mut any = p.zs.is_some();
            for (i, h) in p.holes.iter_mut().flatten().enumerate() {
                h.zs = run(&h.pts, true, i + 1);
                any |= h.zs.is_some();
            }
            any
        }
        _ => false,
    }
}
