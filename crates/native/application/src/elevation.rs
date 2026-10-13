//! Elevations of what `cad.entities.edit` writes (docs/adr/0142). The
//! core's geometry comes without them, so each vertex of a changed or new
//! object takes one from the objects the edit names, by the shared core's
//! rules (`kentos_geometry_core::ops::elevation`). A route's measures (M)
//! are carried by the same rules (docs/adr/0224 §2). The web's is
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
/// one: a line's two ends, a polyline then each other part (docs/adr/0174),
/// a polygon's outer ring then its holes, then each other part's ring and
/// holes (docs/adr/0143); nothing for the other kinds.
pub fn paths(e: &Entity) -> Vec<Elevated> {
    match e {
        Entity::Line(l) => vec![Elevated {
            pts: vec![v(&l.a), v(&l.b)],
            bulges: None,
            closed: false,
            zs: vec![l.za, l.zb],
        }],
        // A closed polyline's closing edge carries too (docs/adr/0258 §1).
        Entity::Polyline(p) => std::iter::once(path(&p.pts, &p.bulges, p.closed, &p.zs))
            .chain(
                p.parts
                    .iter()
                    .flatten()
                    .map(|q| path(&q.pts, &q.bulges, false, &q.zs)),
            )
            .collect(),
        Entity::Polygon(p) => {
            let mut out: Vec<Elevated> = std::iter::once(path(&p.pts, &p.bulges, true, &p.zs))
                .chain(
                    p.holes
                        .iter()
                        .flatten()
                        .map(|h| path(&h.pts, &h.bulges, true, &h.zs)),
                )
                .collect();
            for part in p.parts.iter().flatten() {
                out.push(path(&part.pts, &part.bulges, true, &part.zs));
                out.extend(
                    part.holes
                        .iter()
                        .flatten()
                        .map(|h| path(&h.pts, &h.bulges, true, &h.zs)),
                );
            }
            out
        }
        _ => Vec::new(),
    }
}

/// A route's paths with their measures (M) in the elevations' place
/// (docs/adr/0224 §2): a line's two ends, a polyline then each other part;
/// nothing for the other kinds (an area has none). The core's elevation
/// rules carry measures as they carry elevations.
pub fn measure_paths(e: &Entity) -> Vec<Elevated> {
    match e {
        Entity::Line(l) => vec![Elevated {
            pts: vec![v(&l.a), v(&l.b)],
            bulges: None,
            closed: false,
            zs: vec![l.ma, l.mb],
        }],
        Entity::Polyline(p) => std::iter::once(path(&p.pts, &p.bulges, p.closed, &p.ms))
            .chain(
                p.parts
                    .iter()
                    .flatten()
                    .map(|q| path(&q.pts, &q.bulges, false, &q.ms)),
            )
            .collect(),
        _ => Vec::new(),
    }
}

/// `entity` with the measures its vertices take from `sources` and from
/// `same`, as [`carry`] does for elevations (docs/adr/0224 §2). Whether any
/// vertex got one: an area or another kind gets none.
pub fn carry_measures(
    entity: &mut Entity,
    sources: &[Elevated],
    same: &[Elevated],
    how: Carry,
) -> bool {
    let run = |pts: &[Point], k: usize| -> Option<Vec<Option<f64>>> {
        let pts: Vec<Vec2> = pts.iter().map(v).collect();
        let ms = carry_elevations(&pts, false, same.get(k), sources, how);
        ms.iter().any(Option::is_some).then_some(ms)
    };
    match entity {
        Entity::Line(l) => {
            let ms = run(&[l.a, l.b], 0);
            (l.ma, l.mb) = ms.map_or((None, None), |m| (m[0], m[1]));
            l.ma.is_some() || l.mb.is_some()
        }
        Entity::Polyline(p) => {
            p.ms = run(&p.pts, 0);
            let mut any = p.ms.is_some();
            for (k, part) in p.parts.iter_mut().flatten().enumerate() {
                part.ms = run(&part.pts, k + 1);
                any |= part.ms.is_some();
            }
            any
        }
        _ => false,
    }
}

/// `entity`'s routes given the measures `ms`, in [`measure_paths`]' order,
/// as [`assign`] gives elevations (docs/adr/0224 §2).
pub fn assign_measures(entity: &mut Entity, ms: &[Vec<Option<f64>>]) {
    let take = |k: usize| ms.get(k).filter(|m| m.iter().any(Option::is_some)).cloned();
    match entity {
        Entity::Line(l) => {
            let ends = ms.first();
            l.ma = ends.and_then(|m| m.first().copied().flatten());
            l.mb = ends.and_then(|m| m.get(1).copied().flatten());
        }
        Entity::Polyline(p) => {
            p.ms = take(0);
            for (k, part) in p.parts.iter_mut().flatten().enumerate() {
                part.ms = take(k + 1);
            }
        }
        _ => {}
    }
}

/// `entity`'s paths given the elevations `zs`, in [`paths`]' order (a
/// line's two ends); a path where no vertex has one keeps none
/// (docs/adr/0142). What Oturt writes: the core warps the paths with their
/// elevations (docs/adr/0156 §4).
pub fn assign(entity: &mut Entity, zs: &[Vec<Option<f64>>]) {
    let take = |k: usize| zs.get(k).filter(|z| z.iter().any(Option::is_some)).cloned();
    match entity {
        Entity::Line(l) => {
            let ends = zs.first();
            l.za = ends.and_then(|z| z.first().copied().flatten());
            l.zb = ends.and_then(|z| z.get(1).copied().flatten());
        }
        Entity::Polyline(p) => {
            p.zs = take(0);
            for (k, part) in p.parts.iter_mut().flatten().enumerate() {
                part.zs = take(k + 1);
            }
        }
        Entity::Polygon(p) => {
            p.zs = take(0);
            let mut k = 1;
            for h in p.holes.iter_mut().flatten() {
                h.zs = take(k);
                k += 1;
            }
            for part in p.parts.iter_mut().flatten() {
                part.zs = take(k);
                k += 1;
                for h in part.holes.iter_mut().flatten() {
                    h.zs = take(k);
                    k += 1;
                }
            }
        }
        _ => {}
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
            p.zs = run(&p.pts, p.closed, 0);
            let mut any = p.zs.is_some();
            // Every other part, in `paths`' order (docs/adr/0174).
            for (k, part) in p.parts.iter_mut().flatten().enumerate() {
                part.zs = run(&part.pts, false, k + 1);
                any |= part.zs.is_some();
            }
            any
        }
        Entity::Polygon(p) => {
            p.zs = run(&p.pts, true, 0);
            let mut any = p.zs.is_some();
            let mut k = 1;
            for h in p.holes.iter_mut().flatten() {
                h.zs = run(&h.pts, true, k);
                any |= h.zs.is_some();
                k += 1;
            }
            // Every other part, in `paths`' order (docs/adr/0143).
            for part in p.parts.iter_mut().flatten() {
                part.zs = run(&part.pts, true, k);
                any |= part.zs.is_some();
                k += 1;
                for h in part.holes.iter_mut().flatten() {
                    h.zs = run(&h.pts, true, k);
                    any |= h.zs.is_some();
                    k += 1;
                }
            }
            any
        }
        _ => false,
    }
}
