//! Uzat-kısalt, AutoCAD LENGTHEN (`apps/web/src/model/ops/lengthen.ts`): a line, an
//! arc, an elliptical arc or an open polyline gets a new total length, changed
//! at one end. Shortening cuts the path there (arcs exactly); lengthening
//! continues the end segment along its direction or on its own circle, an
//! elliptical arc on its own ellipse (docs/adr/0298). [`lengthen`] gives the
//! new length by the command's methods: a difference, a percentage, a total,
//! an arc's included angle changed or given, or toward a point.

use crate::api::Op;
use crate::entity::{Entity, MULTI_PART_REFUSED, Shape, entity_geometry, is_multi_part};
use crate::geom::arc::{norm_angle, sweep as ccw_sweep};
use crate::geom::bulge::{bulge_arc, bulge_at, bulge_of_sweep};
use crate::geom::ellipse::{
    EllipseGeom, ellipse_length, ellipse_sweep, is_full_ellipse, param_of_point,
};
use crate::geom::intersect::Edge;
use crate::jsmath::{PI, TAU, atan2, cos, js_hypot, js_max, js_sign, or, sin};
use crate::op;
use crate::ops::curve_cuts::Geometry;
use crate::ops::path::{nearest_s, path_of, sub_path};
use crate::vec2::Vec2;

/// A closed polyline has no end to move (docs/adr/0258 §1), nor a whole ellipse.
fn can_lengthen(e: &Shape) -> bool {
    matches!(
        e,
        Shape::Line { .. } | Shape::Arc { .. } | Shape::Polyline { closed: false, .. }
    ) || elliptical_arc(e).is_some()
}

/// An elliptical arc's geometry; none for a whole ellipse or another kind.
fn elliptical_arc(e: &Shape) -> Option<EllipseGeom> {
    let Shape::Ellipse {
        c,
        major,
        ratio,
        t0,
        t1,
    } = *e
    else {
        return None;
    };
    let g = EllipseGeom {
        c,
        major,
        ratio,
        t0,
        t1,
    };
    (!is_full_ellipse(&g)).then_some(g)
}

/// The length along the ellipse through `sweep` of its parameter, from its start (`forward`) or back from its end.
fn ellipse_run(g: &EllipseGeom, sweep: f64, forward: bool) -> f64 {
    let end = g.t0 + ellipse_sweep(g);
    ellipse_length(&if forward {
        EllipseGeom {
            t1: g.t0 + sweep,
            ..*g
        }
    } else {
        EllipseGeom {
            t0: end - sweep,
            t1: end,
            ..*g
        }
    })
}

/// The sweep whose length along the ellipse is `length` (below a whole turn), from its start (`forward`) or
/// back from its end, by bisection: the length grows with the sweep.
fn ellipse_sweep_for(g: &EllipseGeom, length: f64, forward: bool) -> f64 {
    let (mut lo, mut hi) = (0.0, TAU);
    for _ in 0..80 {
        let mid = (lo + hi) / 2.0;
        if ellipse_run(g, mid, forward) < length {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    (lo + hi) / 2.0
}

/// Current length of what Uzat-kısalt accepts, or None.
pub fn length_of(e: &Shape) -> Option<f64> {
    if !can_lengthen(e) {
        return None;
    }
    if let Some(g) = elliptical_arc(e) {
        return Some(ellipse_length(&g));
    }
    path_of(e).map(|p| p.length)
}

/// Whether a click at p is nearer the end (true) or the start of the entity.
pub fn near_end(e: &Shape, p: Vec2) -> bool {
    if let Some(g) = elliptical_arc(e) {
        let sw = ellipse_sweep(&g);
        let t = norm_angle(param_of_point(&g, p) - g.t0);
        // Past the end, nearer the end than the start round the rest of the turn.
        return if t <= sw {
            t > sw / 2.0
        } else {
            t - sw < TAU - t
        };
    }
    path_of(e).is_some_and(|path| nearest_s(&path, p) > path.length / 2.0)
}

/// Why Uzat-kısalt cannot give an object a length (docs/adr/0298 §3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Why {
    /// Not a line, an arc, an elliptical arc or an open polyline.
    NotLengthenable,
    /// A multi-part polyline: which part's end would move is not known.
    MultiPart,
    /// The object has no length to change (a one-vertex polyline, a line whose ends meet).
    NoLength,
    /// The new length is not above zero.
    InvalidLength,
    /// An arc (or an arc edge, an elliptical arc) would close on itself.
    Closes,
    /// An angle was given for something other than an arc.
    AngleNeedsArc,
}

impl Why {
    /// The words the web's tool has always said.
    pub fn message(self) -> &'static str {
        match self {
            Why::NotLengthenable => {
                "Uzat-kısalt çizgi, yay, elips yayı ve açık çoklu çizgide çalışır."
            }
            Why::MultiPart => MULTI_PART_REFUSED,
            Why::NoLength => "Nesnenin uzunluğu yok.",
            Why::InvalidLength => "Yeni uzunluk sıfırdan büyük olmalı.",
            Why::Closes => {
                "Yay bu uzunlukta kendini kapatır; en çok bir tam daireden kısa olabilir."
            }
            Why::AngleNeedsArc => "Açıyla yalnız yay uzatılır ya da kısaltılır.",
        }
    }
}

/// How the new length is given (AutoCAD LENGTHEN's options, docs/adr/0298 §2).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Change {
    /// The length changed by this much (negative shortens).
    Delta(f64),
    /// The length as this percentage of what it is.
    Percent(f64),
    /// The new total length.
    Total(f64),
    /// An arc's included angle changed by this many radians (negative shortens).
    DeltaAngle(f64),
    /// An arc's new included angle, radians.
    TotalAngle(f64),
    /// The moving end as near this point as the object's own path lets it (Dinamik).
    Toward(Vec2),
}

/// An object lengthened: its new geometry, and its length before and after.
#[derive(Clone, Debug)]
pub struct Lengthened {
    pub entity: Entity,
    pub before: f64,
    pub after: f64,
}

/// `e` given a new length by `change` at its end (`at_end`) or start.
pub fn lengthen(e: &Entity, at_end: bool, change: Change) -> Result<Lengthened, Why> {
    let s = &e.shape;
    if !can_lengthen(s) {
        return Err(Why::NotLengthenable);
    }
    if is_multi_part(s) {
        return Err(Why::MultiPart);
    }
    let before = length_of(s).filter(|l| *l > 0.0).ok_or(Why::NoLength)?;
    let after = match change {
        Change::Delta(d) => before + d,
        Change::Percent(p) => (before * p) / 100.0,
        Change::Total(t) => t,
        Change::DeltaAngle(a) | Change::TotalAngle(a) => {
            let Shape::Arc { r, a0, a1, .. } = *s else {
                return Err(Why::AngleNeedsArc);
            };
            let sweep = if matches!(change, Change::DeltaAngle(_)) {
                ccw_sweep(a0, a1) + a
            } else {
                a
            };
            if sweep >= TAU - 1e-9 {
                return Err(Why::Closes);
            }
            r * sweep
        }
        Change::Toward(p) => length_toward(s, at_end, p).ok_or(Why::NotLengthenable)?,
    };
    if !(after > 1e-9) {
        return Err(Why::InvalidLength);
    }
    let entity = lengthened(e, at_end, after)?;
    Ok(Lengthened {
        entity,
        before,
        after,
    })
}

pub fn lengthen_entity(e: &Entity, at_end: bool, new_length: f64) -> Geometry {
    match lengthened(e, at_end, new_length) {
        Ok(g) => Geometry::Ok(g),
        Err(why) => Geometry::Error(why.message().into()),
    }
}

/// `e` with the new total length `new_length`, changed at its end or start.
fn lengthened(e: &Entity, at_end: bool, new_length: f64) -> Result<Entity, Why> {
    let s = &e.shape;
    if !can_lengthen(s) {
        return Err(Why::NotLengthenable);
    }
    // Which part's end would move is not known (docs/adr/0174).
    if is_multi_part(s) {
        return Err(Why::MultiPart);
    }
    let path = if elliptical_arc(s).is_some() {
        None
    } else {
        Some(path_of(s).ok_or(Why::NoLength)?)
    };
    if !(new_length > 1e-9) {
        return Err(Why::InvalidLength);
    }
    if let Some(g) = elliptical_arc(s) {
        // On its own ellipse: the moving end's parameter where the length along it is the new one.
        let l = ellipse_length(&g);
        if new_length
            >= ellipse_length(&EllipseGeom {
                t1: g.t0 + TAU,
                ..g
            }) - 1e-9
        {
            return Err(Why::Closes);
        }
        if (new_length - l).abs() <= 1e-12 * js_max(1.0, l) {
            return Ok(entity_geometry(e));
        }
        let end = g.t0 + ellipse_sweep(&g);
        let (t0, t1) = if at_end {
            (
                g.t0,
                norm_angle(g.t0 + ellipse_sweep_for(&g, new_length, true)),
            )
        } else {
            (
                norm_angle(end - ellipse_sweep_for(&g, new_length, false)),
                g.t1,
            )
        };
        return Ok(Entity::new(Shape::Ellipse {
            c: g.c,
            major: g.major,
            ratio: g.ratio,
            t0,
            t1,
        }));
    }
    let Some(path) = path else {
        return Err(Why::NoLength);
    };
    let l = path.length;
    if (new_length - l).abs() <= 1e-12 * js_max(1.0, l) {
        return Ok(entity_geometry(e));
    }
    if new_length < l {
        let g = if at_end {
            sub_path(&path, 0.0, new_length, s)
        } else {
            sub_path(&path, l - new_length, l, s)
        };
        // A short polyline stays a polyline.
        if let (Shape::Polyline { .. }, Shape::Line { a, b }) = (s, &g.shape) {
            return Ok(Entity::new(Shape::Polyline {
                pts: vec![*a, *b],
                bulges: None,
                holes: None,
                parts: None,
                closed: false,
                widths: None,
            }));
        }
        return Ok(g);
    }
    let delta = new_length - l;
    match s {
        Shape::Line { a, b } => {
            let (from, to) = if at_end { (*a, *b) } else { (*b, *a) };
            let k = new_length / l;
            let end = Vec2::new(from.x + (to.x - from.x) * k, from.y + (to.y - from.y) * k);
            Ok(Entity::new(if at_end {
                Shape::Line { a: *a, b: end }
            } else {
                Shape::Line { a: end, b: *b }
            }))
        }
        Shape::Arc { c, r, a0, a1 } => {
            let sw = ccw_sweep(*a0, *a1) + delta / r;
            if sw >= TAU - 1e-9 {
                return Err(Why::Closes);
            }
            Ok(Entity::new(if at_end {
                Shape::Arc {
                    c: *c,
                    r: *r,
                    a0: *a0,
                    a1: norm_angle(a0 + sw),
                }
            } else {
                Shape::Arc {
                    c: *c,
                    r: *r,
                    a0: norm_angle(a1 - sw),
                    a1: *a1,
                }
            }))
        }
        Shape::Polyline {
            pts: src,
            bulges: src_bulges,
            ..
        } => {
            let n = src.len();
            // pathOf already refused it; the indexing below needs an end segment.
            if n < 2 {
                return Err(Why::NotLengthenable);
            }
            let mut pts = src.clone();
            let mut bulges: Vec<f64> = (0..n).map(|i| bulge_at(src_bulges.as_deref(), i)).collect();
            // The end segment, walked away from the rest of the path.
            let seg = if at_end { n - 2 } else { 0 };
            let (fixed, moving) = if at_end {
                (pts[n - 2], pts[n - 1])
            } else {
                (pts[1], pts[0])
            };
            match bulge_arc(pts[seg], pts[seg + 1], bulges[seg]) {
                None => {
                    let len = js_hypot(moving.x - fixed.x, moving.y - fixed.y);
                    let k = (len + delta) / len;
                    let end = Vec2::new(
                        fixed.x + (moving.x - fixed.x) * k,
                        fixed.y + (moving.y - fixed.y) * k,
                    );
                    pts[if at_end { n - 1 } else { 0 }] = end;
                }
                Some(arc) => {
                    // Grow the arc on its circle: the sweep gains delta / r in its own direction.
                    let sw = arc.sweep + js_sign(arc.sweep) * (delta / arc.r);
                    if sw.abs() >= TAU - 1e-9 {
                        return Err(Why::Closes);
                    }
                    let a0 = if at_end {
                        arc.a0
                    } else {
                        arc.a0 + arc.sweep - sw
                    };
                    let start = Vec2::new(arc.c.x + cos(a0) * arc.r, arc.c.y + sin(a0) * arc.r);
                    let end = Vec2::new(
                        arc.c.x + cos(a0 + sw) * arc.r,
                        arc.c.y + sin(a0 + sw) * arc.r,
                    );
                    if at_end {
                        pts[n - 1] = end;
                    } else {
                        pts[0] = start;
                    }
                    bulges[seg] = bulge_of_sweep(sw);
                }
            }
            let bulges = bulges.iter().any(|&x| x != 0.0).then_some(bulges);
            // Lengthening moves an end vertex only: every edge keeps its widths (docs/adr/0298 §4).
            let widths = match s {
                Shape::Polyline { widths, .. } => widths.clone(),
                _ => None,
            };
            Ok(Entity::new(Shape::Polyline {
                pts,
                bulges,
                holes: None,
                parts: None,
                closed: false,
                widths,
            }))
        }
        _ => Err(Why::NotLengthenable),
    }
}

/// The total length that puts the moving end at the point nearest to p.
pub fn length_toward(e: &Shape, at_end: bool, p: Vec2) -> Option<f64> {
    // An elliptical arc: the moving end on its own ellipse where the point is seen from its centre.
    if let Some(g) = elliptical_arc(e) {
        let t = param_of_point(&g, p);
        let sweep = if at_end {
            norm_angle(t - g.t0)
        } else {
            norm_angle(g.t0 + ellipse_sweep(&g) - t)
        };
        return (sweep > 1e-12).then(|| ellipse_run(&g, sweep, at_end));
    }
    let path = path_of(e)?;
    if !can_lengthen(e) {
        return None;
    }
    let l = path.length;
    let edges = &path.edges;
    let last = if at_end {
        edges[edges.len() - 1]
    } else {
        edges[0]
    };
    let before = if at_end {
        path.cum[edges.len() - 1]
    } else {
        0.0
    };
    let last_len = if at_end {
        l - path.cum[edges.len() - 1]
    } else if edges.len() > 1 {
        path.cum[1]
    } else {
        l
    };
    let inside = if at_end {
        nearest_s(&path, p)
    } else {
        l - nearest_s(&path, p)
    };
    match last {
        Edge::Seg { a, b } => {
            let (from, to) = if at_end { (a, b) } else { (b, a) };
            let len = js_hypot(to.x - from.x, to.y - from.y);
            let t = ((p.x - from.x) * (to.x - from.x) + (p.y - from.y) * (to.y - from.y)) / len;
            Some(if t > len { before + t } else { inside })
        }
        Edge::Arc { c, r, a0, sweep } => {
            // Arc: angle travelled from the fixed end of the segment, in its direction.
            let dir = or(js_sign(sweep), 1.0);
            let fixed_angle = if at_end { a0 } else { a0 + sweep };
            let walk = if at_end { dir } else { -dir };
            let theta = norm_angle((atan2(p.y - c.y, p.x - c.x) - fixed_angle) * walk);
            let travelled = theta * r;
            // Past the end but not so far round that it would rather be the start again.
            Some(if travelled > last_len && theta < PI + sweep.abs() / 2.0 {
                l - last_len + travelled
            } else {
                inside
            })
        }
    }
}

pub(crate) static OPS: &[Op] = &[
    op!("lengthOf", |e: Entity| length_of(&e.shape)),
    op!("nearEnd", |e: Entity, p: Vec2| near_end(&e.shape, p)),
    op!(
        "lengthenEntity",
        |e: Entity, at_end: bool, new_length: f64| lengthen_entity(&e, at_end, new_length)
    ),
    op!("lengthToward", |e: Entity, at_end: bool, p: Vec2| {
        length_toward(&e.shape, at_end, p)
    }),
];
