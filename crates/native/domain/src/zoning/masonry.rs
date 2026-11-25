//! Taşıyıcı yığma taş duvarların kalın kısmı (docs/adr/8095): in a traditional cut stone masonry building (docs/adr/8094)
//! the load-bearing stone walls' parts past 25 cm (an exterior wall's) and past 12.50 cm (an interior wall's) are out of
//! katlar alanı apart from the 30 % (Planlı Alanlar İmar Yönetmeliği 5/8, as amended RG 31.12.2022/32060), an exterior
//! wall's out of taban alanı too (20/8-j).
//!
//! - **A stone wall** is a wall of the storey declared load-bearing (docs/adr/8090) that is no curtain wall; its width
//!   is its thickness. A wall thicker than 12.50 cm with no declaration is listed (`undeclared`): its part counts in.
//! - **An exterior wall** is one on the outline of the storey's walls' and columns' bodies (`Document::fire_elements`'s
//!   rule); its outer face is the one with no room's contour beyond it. Its part is the band along its outer face as
//!   deep as the wall is past 25 cm — the wall thinned to 25 cm keeping its inner face — carried past its ends by its
//!   own thickness so that a corner's band is whole, then kept to the storey's wall mass (the rooms' contours less their
//!   net areas) and off the bodies of the exterior walls that are not stone.
//! - **An interior wall's** part is the two bands along its faces, each half as deep as the wall is past 12.50 cm — the
//!   wall thinned about its axis — kept to the wall mass, off the exterior walls' bodies, off the other interior stone
//!   walls' kept cores (their middle 12.50 cm) and off the exterior walls' parts.
//!
//! A wall whose outer face is not found (no room on either side) is left out; one with rooms on both sides is read as
//! an interior wall.

use std::collections::BTreeMap;

use kentos_contracts::Entity;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::arrangement::{Area, Ring};
use kentos_geometry_core::geom::region;
use kentos_geometry_core::geom::wall::{self as wall_geom, Justify, Spec};
use kentos_geometry_core::vec2::Vec2;

use crate::Document;

/// The thickness an exterior stone wall keeps in katlar alanı and taban alanı, metres (PAİY 20/8-j, 5/8).
pub const EXTERIOR_KEPT: f64 = 0.25;

/// The thickness an interior stone wall keeps in katlar alanı, metres (PAİY 5/8; 22/1-m's 9.10.2021 text).
pub const INTERIOR_KEPT: f64 = 0.125;

/// How far past its face a point is taken to find a wall's outer side, metres.
const PROBE: f64 = 0.01;

/// A wall of a storey as the parts are cut from it.
struct Wall {
    pts: Vec<Vec2>,
    bulges: Option<Vec<f64>>,
    spec: Spec,
    exterior: bool,
    stone: bool,
}

/// One storey's walls.
#[derive(Default)]
pub(super) struct StoreyWalls {
    walls: Vec<Wall>,
}

/// A building's walls as the parts are cut: by storey; the stone walls thicker than an interior stone wall keeps and
/// the walls as thick with no load-bearing declaration, by persistent id in the drawing's order.
#[derive(Default)]
pub(super) struct Walls {
    pub storeys: BTreeMap<Option<String>, StoreyWalls>,
    pub thick: Vec<String>,
    pub undeclared: Vec<String>,
}

/// The project's walls on the storeys `traditional` keeps (a traditional cut stone masonry building's, docs/adr/8145).
pub(super) fn walls(doc: &Document, traditional: &dyn Fn(Option<&str>) -> bool) -> Walls {
    let exterior: std::collections::BTreeSet<String> = doc
        .fire_elements()
        .into_iter()
        .filter(|e| e.kind == "wall" && e.exterior)
        .map(|e| e.uid)
        .collect();
    let mut out = Walls::default();
    for (uid, e) in doc.entities_with_uids() {
        let Entity::Wall(w) = e else { continue };
        if w.wall.curtain.is_some() {
            continue;
        }
        if !traditional(e.own_level()) {
            continue;
        }
        let Shape::Wall { pts, bulges, wall: spec } = kentos_formats::walls::core_wall(&w.wall) else { continue };
        let uid = uid.to_string();
        let declared = w.wall.arch.load_bearing.as_ref();
        let stone = declared.is_some_and(|b| b.bearing);
        if spec.width > INTERIOR_KEPT + 1e-9 {
            if stone {
                out.thick.push(uid.clone());
            } else if declared.is_none() {
                out.undeclared.push(uid.clone());
            }
        }
        out.storeys.entry(e.own_level().map(str::to_owned)).or_default().walls.push(Wall {
            pts,
            bulges,
            spec: *spec,
            exterior: exterior.contains(&uid),
            stone,
        });
    }
    out
}

/// The region between two of a wall's lines, `left` and `right` its distances from the path (left positive): a band of
/// `left - right` along it. None when it cannot be drawn (an arc it would collapse) or is no band.
fn band(pts: &[Vec2], bulges: Option<&[f64]>, closed: bool, left: f64, right: f64) -> Option<Area> {
    if left - right <= 1e-9 {
        return None;
    }
    let spec = Spec { closed, width: left - right, justify: Justify::Left, offset: left, ..Spec::default() };
    let (outer, holes) = wall_geom::body(pts, bulges, &spec).ok()?;
    Some(Area {
        outer: Ring { pts: outer, bulges: None },
        holes: holes.unwrap_or_default().into_iter().map(|pts| Ring { pts, bulges: None }).collect(),
    })
}

/// The path carried `by` past each end along its straight end edge (an arc's end stays).
fn carried(pts: &[Vec2], bulges: Option<&[f64]>, by: f64) -> Vec<Vec2> {
    let mut out = pts.to_vec();
    let n = out.len();
    if n < 2 {
        return out;
    }
    let straight = |i: usize| bulges.and_then(|b| b.get(i)).is_none_or(|b| b.abs() < 1e-12);
    let unit = |a: Vec2, b: Vec2| {
        let d = Vec2::new(b.x - a.x, b.y - a.y);
        let l = (d.x * d.x + d.y * d.y).sqrt();
        (l > 1e-12).then(|| Vec2::new(d.x / l, d.y / l))
    };
    if straight(0)
        && let Some(d) = unit(out[1], out[0])
    {
        out[0] = Vec2::new(out[0].x + d.x * by, out[0].y + d.y * by);
    }
    if straight(n - 2)
        && let Some(d) = unit(out[n - 2], out[n - 1])
    {
        out[n - 1] = Vec2::new(out[n - 1].x + d.x * by, out[n - 1].y + d.y * by);
    }
    out
}

/// The point halfway along a polyline.
fn middle(pts: &[Vec2]) -> Option<Vec2> {
    let len = |a: Vec2, b: Vec2| ((b.x - a.x).powi(2) + (b.y - a.y).powi(2)).sqrt();
    let total: f64 = pts.windows(2).map(|w| len(w[0], w[1])).sum();
    let mut left = total / 2.0;
    for w in pts.windows(2) {
        let l = len(w[0], w[1]);
        if l >= left && l > 0.0 {
            let t = left / l;
            return Some(Vec2::new(w[0].x + (w[1].x - w[0].x) * t, w[0].y + (w[1].y - w[0].y) * t));
        }
        left -= l;
    }
    pts.first().copied()
}

/// Whether the point a probe's distance past the wall's line at `d` (left positive) lies in no room's contour.
fn open_beyond(w: &Wall, d: f64, contours: &[Area]) -> Option<bool> {
    let line = wall_geom::line_at(&w.pts, w.bulges.as_deref(), w.spec.closed, d).ok()?;
    let p = middle(&line.outline(w.spec.closed))?;
    Some(!contours.iter().any(|a| region::inside_area(a, p)))
}

/// A storey's stone walls' parts in its wall mass: the exterior walls' (out of taban alanı too) and the interior walls'.
/// `contours` are the storey's rooms' contours, `nets` their net areas.
pub(super) fn parts(walls: &StoreyWalls, contours: &[Area], nets: &[Area]) -> (Vec<Area>, Vec<Area>) {
    let mass = region::subtract_areas(&region::union_areas(contours), &region::union_areas(nets));
    if mass.is_empty() {
        return (Vec::new(), Vec::new());
    }
    let mut outer_bands: Vec<Area> = Vec::new();
    let mut inner_bands: Vec<Area> = Vec::new();
    let mut cores: Vec<Area> = Vec::new();
    let mut exterior_bodies: Vec<Area> = Vec::new();
    let mut not_stone_exterior: Vec<Area> = Vec::new();
    for w in &walls.walls {
        let (closed, bulges) = (w.spec.closed, w.bulges.as_deref());
        let t = w.spec.width;
        let left = wall_geom::left_face(&w.spec);
        let right = left - t;
        let body = band(&w.pts, bulges, closed, left, right);
        if w.exterior {
            exterior_bodies.extend(body.clone());
            if !w.stone {
                not_stone_exterior.extend(body);
                continue;
            }
        }
        if !w.stone {
            continue;
        }
        // An exterior wall's outer face: no room beyond it; rooms on both sides make it an interior wall.
        let outer_left = if w.exterior {
            match (open_beyond(w, left + PROBE, contours), open_beyond(w, right - PROBE, contours)) {
                (Some(true), Some(false)) => Some(true),
                (Some(false), Some(true)) => Some(false),
                (Some(false), Some(false)) => None,
                _ => continue,
            }
        } else {
            None
        };
        match outer_left {
            Some(on_left) => {
                let e = t - EXTERIOR_KEPT;
                if e <= 1e-9 {
                    continue;
                }
                let path = if closed { w.pts.clone() } else { carried(&w.pts, bulges, t) };
                let b = if on_left {
                    band(&path, bulges, closed, left, left - e)
                } else {
                    band(&path, bulges, closed, right + e, right)
                };
                outer_bands.extend(b);
            }
            None => {
                let e = t - INTERIOR_KEPT;
                if e <= 1e-9 {
                    cores.extend(body);
                    continue;
                }
                inner_bands.extend(band(&w.pts, bulges, closed, left, left - e / 2.0));
                inner_bands.extend(band(&w.pts, bulges, closed, right + e / 2.0, right));
                cores.extend(band(&w.pts, bulges, closed, left - e / 2.0, right + e / 2.0));
            }
        }
    }
    let outer = if outer_bands.is_empty() {
        Vec::new()
    } else {
        let kept = region::intersect_area_sets(&[region::union_areas(&outer_bands), mass.clone()]);
        if not_stone_exterior.is_empty() { kept } else { region::subtract_areas(&kept, &not_stone_exterior) }
    };
    let inner = if inner_bands.is_empty() {
        Vec::new()
    } else {
        let kept = region::intersect_area_sets(&[region::union_areas(&inner_bands), mass]);
        let off: Vec<Area> = exterior_bodies.into_iter().chain(cores).chain(outer.iter().cloned()).collect();
        if off.is_empty() { kept } else { region::subtract_areas(&kept, &off) }
    };
    (outer, inner)
}
