//! Elevations of what `cad.entities.edit` writes (docs/adr/0142). The
//! core's geometry comes without them, so each vertex of a changed or new
//! object takes one from the objects the edit names, by the shared core's
//! rules (`kentos_geometry_core::ops::elevation`). A route's measures (M)
//! are carried by the same rules (docs/adr/0224 §2). The web's is
//! `apps/web/src/product/elevation.ts`; both pass the shared cases.

use kentos_commands::{Entity, EntityGeometry, Vec2 as Point};
use kentos_geometry_core::Vec2;
use kentos_geometry_core::ops::elevation::{Carry, Elevated, ON, carry_elevations};

use crate::geometry::{edit_geometry, shape};

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

// ── Kot ver (docs/adr/0142, 0622 §9): what a write does to the vertices' elevations ──

/// Whether an object's kind takes elevations: a point (its `z`), a line, a
/// polyline and an area. The others are left out of Kot ver.
pub fn takes(e: &Entity) -> bool {
    matches!(
        e,
        Entity::Point(_) | Entity::Line(_) | Entity::Polyline(_) | Entity::Polygon(_)
    )
}

fn any(zs: &Option<Vec<Option<f64>>>) -> bool {
    zs.iter().flatten().any(Option::is_some)
}

/// Whether any vertex has an elevation (nothing is allocated).
pub fn has_any(e: &Entity) -> bool {
    match e {
        // Every point and every part too (docs/adr/0174).
        Entity::Point(p) => p.z.is_some() || p.parts.iter().flatten().any(|q| q.z.is_some()),
        Entity::Line(l) => l.za.is_some() || l.zb.is_some(),
        Entity::Polyline(p) => any(&p.zs) || p.parts.iter().flatten().any(|q| any(&q.zs)),
        Entity::Polygon(p) => {
            let holes = |hs: &Option<Vec<kentos_commands::RingGeometry>>| {
                hs.iter().flatten().any(|h| any(&h.zs))
            };
            // Every part's too (docs/adr/0143).
            any(&p.zs)
                || holes(&p.holes)
                || p.parts
                    .iter()
                    .flatten()
                    .any(|q| any(&q.zs) || holes(&q.holes))
        }
        _ => false,
    }
}

/// What a write does to each vertex's elevation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Change {
    /// Every vertex gets this elevation, or none: Sabit and Sıfırla, and an
    /// Öznitelikler cell that takes them all.
    Set(Option<f64>),
    /// Every vertex that has an elevation gets it plus the difference; one
    /// without stays without (Artır).
    Raise(f64),
    /// One end of a line, by its place (0 the start, 1 the end): its
    /// elevation, or none; the other end stays as it is.
    End(usize, Option<f64>),
}

impl Change {
    /// The vertex at `index` of the object's order, with `z` now, after the change.
    pub fn apply(self, z: Option<f64>, index: usize) -> Option<f64> {
        match self {
            Change::Set(z) => z,
            Change::Raise(by) => z.map(|z| z + by),
            Change::End(end, to) if end == index => to,
            Change::End(..) => z,
        }
    }
}

/// The object's own geometry with each vertex's elevation as `change` says,
/// as `cad.entities.edit` takes it in the operation `elevation`: `zs` for a
/// line (its two ends), a polyline and an area (each hole's in its own `zs`),
/// `z` for a point. Every elevation is explicit, so the command writes them
/// as they are. `None` for a kind without elevations.
pub fn geometry_with(e: &Entity, change: Change) -> Option<EntityGeometry> {
    let mut geometry = edit_geometry(shape(e))?;
    let mut next = 0;
    let mut run = |zs: Vec<Option<f64>>| -> Vec<Option<f64>> {
        zs.into_iter()
            .map(|z| {
                let out = change.apply(z, next);
                next += 1;
                out
            })
            .collect()
    };
    match (&mut geometry, e) {
        (EntityGeometry::Point { z, parts, .. }, Entity::Point(p)) => {
            *z = change.apply(p.z, 0);
            // Every point of a multi-point object, in turn (docs/adr/0174).
            for (k, (part, was)) in parts
                .iter_mut()
                .flatten()
                .zip(p.parts.iter().flatten())
                .enumerate()
            {
                part.z = change.apply(was.z, k + 1);
            }
        }
        (EntityGeometry::Line { zs, .. }, Entity::Line(l)) => *zs = Some(run(vec![l.za, l.zb])),
        (EntityGeometry::Polyline { zs, parts, .. }, Entity::Polyline(_)) => {
            // `paths`' order: the polyline, then each other part (docs/adr/0174).
            let mut lines = paths(e).into_iter();
            *zs = lines.next().map(|p| run(p.zs));
            for part in parts.iter_mut().flatten() {
                part.zs = lines.next().map(|p| run(p.zs));
            }
        }
        (
            EntityGeometry::Polygon {
                zs, holes, parts, ..
            },
            Entity::Polygon(_),
        ) => {
            // `paths`' order: the ring, its holes, then each other part's ring and holes.
            let mut rings = paths(e).into_iter();
            *zs = rings.next().map(|p| run(p.zs));
            for hole in holes.iter_mut().flatten() {
                hole.zs = rings.next().map(|p| run(p.zs));
            }
            for part in parts.iter_mut().flatten() {
                part.zs = rings.next().map(|p| run(p.zs));
                for hole in part.holes.iter_mut().flatten() {
                    hole.zs = rings.next().map(|p| run(p.zs));
                }
            }
        }
        _ => return None,
    }
    Some(geometry)
}

/// The elevation of the object's vertex at `at`, or none: no vertex there,
/// or it has none. A line's end counts, and a point is a vertex.
pub fn elevation_at(e: &Entity, at: Vec2) -> Option<f64> {
    let near = |q: &kentos_commands::Vec2| (q.x - at.x).abs() <= ON && (q.y - at.y).abs() <= ON;
    match e {
        // Any point of a multi-point object (docs/adr/0174).
        Entity::Point(p) => p.z.filter(|_| near(&p.p)).or_else(|| {
            p.parts
                .iter()
                .flatten()
                .find(|q| near(&q.p))
                .and_then(|q| q.z)
        }),
        Entity::Line(l) => {
            if near(&l.a) {
                l.za
            } else if near(&l.b) {
                l.zb
            } else {
                None
            }
        }
        Entity::Polyline(_) | Entity::Polygon(_) => paths(e).into_iter().find_map(|path| {
            path.pts
                .iter()
                .zip(&path.zs)
                .find(|(q, z)| z.is_some() && (q.x - at.x).abs() <= ON && (q.y - at.y).abs() <= ON)
                .and_then(|(_, z)| *z)
        }),
        _ => None,
    }
}
