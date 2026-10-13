//! Çizginin parçası, Alanları çizgiye çevir, Çizgileri alana çevir and Köşe
//! ekle (docs/adr/0218 §8–§11): paths cut, turned and divided, their
//! elevations carried by docs/adr/0142's rules.

use super::{Elevations, PathOf, Zs, kept, paths, z_along, z_of};
use crate::api::json::{FromJson, Json, read_field};
use crate::entity::{Part, Shape, entity_area, is_multi_part};
use crate::geom::arrangement::Ring;
use crate::geom::bulge::bulge_of_sweep;
use crate::geom::intersect::Edge;
use crate::jsmath::{cos, js_hypot, sin};
use crate::ops::edges::edge_length;
use crate::ops::elevation::{Carry, Elevated, carry_elevations};
use crate::ops::path::{path_of, sub_path};
use crate::vec2::Vec2;

// ── Çizginin parçası ─────────────────────────────────────────────────

/// A path with its elevations, as the elevation rules take it.
fn elevated(p: &PathOf, zs: Option<&Zs>, k: usize) -> Elevated {
    Elevated {
        pts: p.pts.clone(),
        bulges: p.bulges.clone(),
        closed: p.closed,
        zs: (0..p.pts.len()).map(|i| z_of(zs, k, i)).collect(),
    }
}

/// Çizginin parçası (§8): the part of a line, a polyline or an arc from `s0`
/// to `s1` along it, both taken into [0, length] (docs/adr/0140's exact
/// piece: straight edges straight, arcs arcs; a line's a line, an arc's an
/// arc, a piece on one straight edge a line); `None` when `s0` is not less
/// than `s1` or the object has more than one part. Its vertices'
/// elevations: on a source vertex its own, along a source edge linearly.
pub fn substring(s: &Shape, zs: Option<&Zs>, s0: f64, s1: f64) -> Option<Elevations> {
    if !matches!(
        s,
        Shape::Line { .. } | Shape::Polyline { .. } | Shape::Arc { .. }
    ) || is_multi_part(s)
    {
        return None;
    }
    let path = path_of(s)?;
    let (s0, s1) = (s0.clamp(0.0, path.length), s1.clamp(0.0, path.length));
    if !(s0 < s1) {
        return None;
    }
    let piece = sub_path(&path, s0, s1, s).shape;
    let source = paths(s).into_iter().next()?;
    let has = zs.is_some_and(|z| z.iter().flatten().any(Option::is_some));
    let piece_zs = has.then(|| {
        let pts: Vec<Vec2> = match &piece {
            Shape::Line { a, b } => vec![*a, *b],
            Shape::Polyline { pts, .. } => pts.clone(),
            _ => Vec::new(),
        };
        vec![carry_elevations(
            &pts,
            false,
            None,
            &[elevated(&source, zs, 0)],
            Carry::Along,
        )]
    });
    Some(Elevations {
        shape: piece,
        zs: piece_zs.and_then(kept),
    })
}

// ── Alanları çizgiye çevir ───────────────────────────────────────────

/// A ring Alanları çizgiye çevir writes: a polyline whose last vertex is
/// its first, its part and ring.
#[derive(Clone, Debug, PartialEq)]
pub struct RingLine {
    pub line: Elevations,
    pub part: u32,
    pub ring: u32,
}

/// Alanları çizgiye çevir (§9): an area's rings as polylines whose last
/// vertex is their first (the drawing's Çizgiye çevir), the outer ring, its
/// holes, then each other part's, with their arcs and elevations.
pub fn rings_as_lines(s: &Shape, zs: Option<&Zs>) -> Vec<RingLine> {
    if !matches!(s, Shape::Polygon { .. }) {
        return Vec::new();
    }
    paths(s)
        .into_iter()
        .enumerate()
        .filter(|(_, p)| !p.pts.is_empty())
        .map(|(k, p)| {
            let n = p.pts.len();
            let mut pts = p.pts.clone();
            pts.push(p.pts[0]);
            let bulges = p
                .bulges
                .as_ref()
                .filter(|b| b.iter().take(n).any(|&x| x != 0.0))
                .map(|b| {
                    let mut out: Vec<f64> =
                        (0..n).map(|i| b.get(i).copied().unwrap_or(0.0)).collect();
                    out.push(0.0);
                    out
                });
            let mut ring_zs: Vec<Option<f64>> = (0..n).map(|i| z_of(zs, k, i)).collect();
            ring_zs.push(ring_zs[0]);
            RingLine {
                line: Elevations {
                    shape: Shape::Polyline {
                        pts,
                        bulges,
                        holes: None,
                        parts: None,
                        closed: false,
                        widths: None,
                    },
                    zs: kept(vec![ring_zs]),
                },
                part: p.part,
                ring: p.ring,
            }
        })
        .collect()
}

// ── Çizgileri alana çevir ────────────────────────────────────────────

/// Why a line gives no area (§10): it is open (and not to be closed), it
/// has no area, or it has more than one part.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoArea {
    Open,
    Empty,
    MultiPart,
}

/// How far apart a line's ends may be and still close it (the drawing's
/// Alana çevir, docs/adr/0065).
const CLOSE_TOL: f64 = 1e-6;

/// Çizgileri alana çevir (§10): a polyline whose ends meet as an area
/// without its repeated end (its last edge's bulge the closing one); an
/// open one closed by a straight edge when `close`. The area must have an
/// area; its vertices keep their elevations.
pub fn line_area(s: &Shape, zs: Option<&Zs>, close: bool) -> Result<Elevations, NoArea> {
    let Shape::Polyline { pts, bulges, .. } = s else {
        return Err(NoArea::Empty);
    };
    if is_multi_part(s) {
        return Err(NoArea::MultiPart);
    }
    let n = pts.len();
    if n < 2 {
        return Err(NoArea::Empty);
    }
    // A closed polyline is its ring, its last bulge the closing edge's (docs/adr/0258 §1).
    let ring = matches!(s, Shape::Polyline { closed: true, .. });
    let (f, l) = (pts[0], pts[n - 1]);
    let meets = !ring && js_hypot(f.x - l.x, f.y - l.y) <= CLOSE_TOL;
    if !ring && !meets && !close {
        return Err(NoArea::Open);
    }
    let m = if meets { n - 1 } else { n };
    let ring_bulges: Vec<f64> = (0..m)
        .map(|i| {
            if ring || meets || i + 1 < n {
                bulges
                    .as_ref()
                    .and_then(|b| b.get(i))
                    .copied()
                    .unwrap_or(0.0)
            } else {
                0.0
            }
        })
        .collect();
    let area = Shape::Polygon {
        pts: pts[..m].to_vec(),
        bulges: ring_bulges.iter().any(|&b| b != 0.0).then_some(ring_bulges),
        holes: None,
        parts: None,
    };
    if !entity_area(&area).is_some_and(|a| a > 0.0) {
        return Err(NoArea::Empty);
    }
    Ok(Elevations {
        shape: area,
        zs: kept(vec![(0..m).map(|i| z_of(zs, 0, i)).collect()]),
    })
}

// ── Köşe ekle ────────────────────────────────────────────────────────

/// How Köşe ekle divides an edge (§11).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Density {
    /// Into the fewest equal parts no longer than this (m).
    Interval(f64),
    /// With this many new vertices.
    Count(u32),
}

impl FromJson for Density {
    fn from_json(v: &Json) -> Result<Density, String> {
        let Json::Obj(fields) = v else {
            return Err("{ interval } ya da { count } bekleniyordu".into());
        };
        if fields.iter().any(|(k, _)| k == "count") {
            let count: usize = read_field(v, "count")?;
            Ok(Density::Count(
                u32::try_from(count).map_err(|_| "köşe sayısı çok büyük".to_owned())?,
            ))
        } else {
            Ok(Density::Interval(read_field(v, "interval")?))
        }
    }
}

/// The equal parts an edge `len` long is divided into.
fn parts_of(len: f64, by: Density) -> u64 {
    match by {
        Density::Interval(d) if d > 0.0 && len > d => (len / d).ceil() as u64,
        Density::Interval(_) => 1,
        Density::Count(k) => u64::from(k) + 1,
    }
}

/// A line as a two-vertex polyline (what Köşe ekle writes of it).
fn as_path(s: &Shape) -> Shape {
    match s {
        Shape::Line { a, b } => Shape::Polyline {
            pts: vec![*a, *b],
            bulges: None,
            holes: None,
            parts: None,
            closed: false,
            widths: None,
        },
        other => other.clone(),
    }
}

/// How many vertices Köşe ekle would add to the object, without adding them
/// (the tools' limit is read before anything is built).
pub fn densify_added(s: &Shape, by: Density) -> u64 {
    paths(&as_path(s))
        .iter()
        .map(|p| {
            (0..p.edge_count())
                .map(|i| parts_of(edge_length(&p.edge(i)), by) - 1)
                .sum::<u64>()
        })
        .sum()
}

/// Köşe ekle (§11): each edge of a line, a polyline or an area divided into
/// equal parts (an arc into arcs of its circle, so the shape stays); the new
/// vertices' elevations along their edges. The vertices added beside.
pub fn densify(s: &Shape, zs: Option<&Zs>, by: Density) -> (Elevations, u64) {
    let s = as_path(s);
    let mut added = 0;
    let mut new_paths = Vec::new();
    let mut new_zs = Vec::new();
    for (k, p) in paths(&s).iter().enumerate() {
        let n = p.pts.len();
        let (mut pts, mut bulges, mut z) = (Vec::new(), Vec::new(), Vec::new());
        for i in 0..p.edge_count() {
            let j = (i + 1) % n;
            let e = p.edge(i);
            let parts = parts_of(edge_length(&e), by);
            added += parts - 1;
            let sub = match e {
                Edge::Arc { sweep, .. } => bulge_of_sweep(sweep / parts as f64),
                Edge::Seg { .. } => 0.0,
            };
            for step in 0..parts {
                let t = step as f64 / parts as f64;
                pts.push(if step == 0 {
                    p.pts[i]
                } else {
                    match e {
                        Edge::Arc { c, r, a0, sweep } => {
                            let a = a0 + sweep * t;
                            Vec2::new(c.x + r * cos(a), c.y + r * sin(a))
                        }
                        Edge::Seg { a, b } => {
                            Vec2::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
                        }
                    }
                });
                bulges.push(sub);
                z.push(z_along(zs, k, i, j, t));
            }
        }
        if !p.closed && n > 0 {
            pts.push(p.pts[n - 1]);
            bulges.push(0.0);
            z.push(z_of(zs, k, n - 1));
        }
        new_paths.push((pts, bulges));
        new_zs.push(z);
    }
    let shape = rebuild(&s, new_paths);
    (
        Elevations {
            shape,
            zs: kept(new_zs),
        },
        added,
    )
}

/// The bulges, kept when an edge is an arc.
fn bulged(b: Vec<f64>) -> Option<Vec<f64>> {
    b.iter().any(|&x| x != 0.0).then_some(b)
}

/// `s` with its paths replaced, in `paths`' order.
pub(crate) fn rebuild(s: &Shape, new: Vec<(Vec<Vec2>, Vec<f64>)>) -> Shape {
    let mut it = new.into_iter();
    let mut take = move || {
        let (pts, b) = it.next().unwrap_or_default();
        (pts, bulged(b))
    };
    match s {
        Shape::Polyline { parts, closed, .. } => {
            let (pts, bulges) = take();
            let mut new_parts = Vec::new();
            for _ in parts.iter().flatten() {
                let (pts, bulges) = take();
                new_parts.push(Part {
                    pts,
                    bulges,
                    holes: None,
                });
            }
            Shape::Polyline {
                pts,
                bulges,
                holes: None,
                parts: parts.as_ref().map(|_| new_parts),
                closed: *closed,
                widths: None,
            }
        }
        Shape::Polygon { holes, parts, .. } => {
            let (pts, bulges) = take();
            let mut new_holes = Vec::new();
            for _ in holes.iter().flatten() {
                let (pts, bulges) = take();
                new_holes.push(Ring { pts, bulges });
            }
            let mut new_parts = Vec::new();
            for part in parts.iter().flatten() {
                let (pts, bulges) = take();
                let mut part_holes = Vec::new();
                for _ in part.holes.iter().flatten() {
                    let (pts, bulges) = take();
                    part_holes.push(Ring { pts, bulges });
                }
                new_parts.push(Part {
                    pts,
                    bulges,
                    holes: part.holes.as_ref().map(|_| part_holes),
                });
            }
            Shape::Polygon {
                pts,
                bulges,
                holes: holes.as_ref().map(|_| new_holes),
                parts: parts.as_ref().map(|_| new_parts),
            }
        }
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(x: f64, y: f64) -> Vec2 {
        Vec2::new(x, y)
    }

    #[test]
    fn a_polylines_piece_keeps_its_elevations_along() {
        let s = Shape::Polyline {
            pts: vec![v(0.0, 0.0), v(10.0, 0.0), v(10.0, 10.0)],
            bulges: None,
            holes: None,
            parts: None,
            closed: false,
            widths: None,
        };
        let zs = vec![vec![Some(0.0), Some(10.0), Some(20.0)]];
        let got = substring(&s, Some(&zs), 5.0, 15.0).expect("a piece");
        let Shape::Polyline { pts, .. } = &got.shape else {
            panic!("{:?}", got.shape);
        };
        assert_eq!(pts, &vec![v(5.0, 0.0), v(10.0, 0.0), v(10.0, 5.0)]);
        assert_eq!(got.zs, Some(vec![vec![Some(5.0), Some(10.0), Some(15.0)]]));
        assert_eq!(substring(&s, None, 15.0, 5.0), None);
    }

    #[test]
    fn a_closed_line_is_an_area_and_an_open_one_is_closed_when_asked() {
        let open = Shape::Polyline {
            pts: vec![v(0.0, 0.0), v(4.0, 0.0), v(4.0, 3.0)],
            bulges: None,
            holes: None,
            parts: None,
            closed: false,
            widths: None,
        };
        assert_eq!(line_area(&open, None, false), Err(NoArea::Open));
        let area = line_area(&open, None, true).expect("an area");
        assert_eq!(entity_area(&area.shape), Some(6.0));
        let flat = Shape::Polyline {
            pts: vec![v(0.0, 0.0), v(4.0, 0.0)],
            bulges: None,
            holes: None,
            parts: None,
            closed: false,
            widths: None,
        };
        assert_eq!(line_area(&flat, None, true), Err(NoArea::Empty));
    }

    #[test]
    fn densifying_keeps_an_arc_an_arc_and_counts_what_it_adds() {
        let half = Shape::Polygon {
            pts: vec![v(1.0, 0.0), v(-1.0, 0.0)],
            bulges: Some(vec![1.0, 0.0]),
            holes: None,
            parts: None,
        };
        let by = Density::Count(1);
        assert_eq!(densify_added(&half, by), 2);
        let (got, added) = densify(&half, None, by);
        assert_eq!(added, 2);
        let Shape::Polygon { pts, bulges, .. } = &got.shape else {
            panic!("{:?}", got.shape);
        };
        // The top of the half circle, then the chord's middle; the arc's two halves each a quarter.
        assert!((pts[1].x).abs() < 1e-15 && (pts[1].y - 1.0).abs() < 1e-15);
        assert_eq!(pts[3], v(0.0, 0.0));
        let quarter = bulge_of_sweep(std::f64::consts::FRAC_PI_2);
        assert_eq!(bulges.as_deref(), Some(&[quarter, quarter, 0.0, 0.0][..]));
        assert_eq!(densify_added(&half, Density::Interval(2.0)), 1);
    }
}
