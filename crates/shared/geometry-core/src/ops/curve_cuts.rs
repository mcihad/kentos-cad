//! Trim, break, extend and offset for the curves that are not paths of
//! segments and circular arcs (`apps/web/src/model/ops/curveCuts.ts`): ellipses (cut
//! in parameter space, pieces stay elliptical arcs) and construction lines
//! (pieces become rays or lines, as AutoCAD does).

use crate::api::json::{ToJson, field};
use crate::entity::{Entity, Shape, ellipse_geom};
use crate::geom::arc::norm_angle;
use crate::geom::ellipse::{
    EllipseGeom, closest_param, ellipse_derivative, ellipse_point, ellipse_sweep, is_full_ellipse,
    line_ellipse, param_of_point, tessellate_ellipse,
};
use crate::geom::intersect::{
    Edge, closest_on_edge, intersect_edges, line_circle_params, line_line, on_edge_arc,
};
use crate::jsmath::{
    TAU, atan2, js_cmp, js_hypot, js_max, js_max_all, js_min, js_min_all, or, stable_sort,
};
use crate::vec2::Vec2;

/// Pieces of a cut object, or why it could not be cut.
pub enum Cut {
    Pieces(Vec<Entity>),
    Error(String),
}

impl ToJson for Cut {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        match self {
            Cut::Pieces(p) => field(out, &mut first, "pieces", p),
            Cut::Error(e) => field(out, &mut first, "error", e),
        }
        out.push('}');
    }
}

/// One new geometry, or why there is none.
pub enum Geometry {
    Ok(Entity),
    Error(String),
}

impl ToJson for Geometry {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        match self {
            Geometry::Ok(g) => field(out, &mut first, "geometry", g),
            Geometry::Error(e) => field(out, &mut first, "error", e),
        }
        out.push('}');
    }
}

// ── Ellipse ────────────────────────────────────────────────────────────

fn full(e: &EllipseGeom) -> EllipseGeom {
    EllipseGeom {
        t0: 0.0,
        t1: 0.0,
        ..*e
    }
}

fn arc_of(e: &EllipseGeom, a: f64, b: f64) -> Entity {
    Entity::new(Shape::Ellipse {
        c: e.c,
        major: e.major,
        ratio: e.ratio,
        t0: norm_angle(e.t0 + a),
        t1: norm_angle(e.t0 + b),
    })
}

/// Parameters where boundaries cross the curve `e` (exact for straight boundaries).
pub fn ellipse_crossings(e: &EllipseGeom, boundaries: &[Edge]) -> Vec<f64> {
    let mut out = Vec::new();
    let chords = tessellate_ellipse(e, 256.0);
    let closed = is_full_ellipse(e);
    let n = chords.len();
    let count = if closed { n } else { n.saturating_sub(1) };
    let edges: Vec<Edge> = (0..count)
        .map(|i| Edge::Seg {
            a: chords[i],
            b: chords[(i + 1) % n],
        })
        .collect();
    for b in boundaries {
        if let Edge::Seg { a: ba, b: bb } = *b {
            for h in line_ellipse(e, ba, bb) {
                if h.u >= -1e-9 && h.u <= 1.0 + 1e-9 {
                    out.push(h.t);
                }
            }
            continue;
        }
        for ed in &edges {
            for h in intersect_edges(ed, b) {
                // Alternate projections: onto the ellipse, onto the circle, until they agree.
                let mut q = h.p;
                for _ in 0..40 {
                    q = closest_on_edge(b, ellipse_point(&full(e), closest_param(&full(e), q))).p;
                }
                out.push(param_of_point(e, q));
            }
        }
    }
    out
}

/// Offsets d (0..sweep) of cut parameters along the curve, sorted and de-duplicated.
fn offsets(e: &EllipseGeom, ts: Vec<f64>) -> Vec<f64> {
    let sw = ellipse_sweep(e);
    let closed = is_full_ellipse(e);
    let mut d: Vec<f64> = ts
        .into_iter()
        .map(|t| norm_angle(t - e.t0))
        .filter(|&x| closed || (x > 1e-9 && x < sw - 1e-9))
        .collect();
    stable_sort(&mut d, &mut |a, b| js_cmp(*a - *b, 0.0));
    let mut out = Vec::with_capacity(d.len());
    for (i, &x) in d.iter().enumerate() {
        if i == 0 || x - d[i - 1] > 1e-9 {
            out.push(x);
        }
    }
    out
}

pub fn trim_ellipse(e: &EllipseGeom, pick: Vec2, boundaries: &[Edge]) -> Cut {
    let cuts = offsets(e, ellipse_crossings(e, boundaries));
    let dp = norm_angle(closest_param(e, pick) - e.t0);
    let sw = ellipse_sweep(e);
    if is_full_ellipse(e) {
        if cuts.len() < 2 {
            return Cut::Error("Elipsi budamak için en az iki kesişim gerekir.".into());
        }
        let below: Vec<f64> = cuts.iter().copied().filter(|&c| c < dp).collect();
        let above: Vec<f64> = cuts.iter().copied().filter(|&c| c > dp).collect();
        let lo = below.last().copied().unwrap_or(cuts[cuts.len() - 1] - TAU);
        let hi = above.first().copied().unwrap_or(cuts[0] + TAU);
        return Cut::Pieces(vec![arc_of(e, hi, lo + TAU)]);
    }
    let lo = js_max_all(std::iter::once(0.0).chain(cuts.iter().copied().filter(|&c| c < dp)));
    let hi = js_min_all(std::iter::once(sw).chain(cuts.iter().copied().filter(|&c| c > dp)));
    if lo <= 1e-9 && hi >= sw - 1e-9 {
        return Cut::Error("Tıklanan kısmı kesen bir kenar yok.".into());
    }
    let mut pieces = Vec::new();
    if lo > 1e-9 {
        pieces.push(arc_of(e, 0.0, lo));
    }
    if hi < sw - 1e-9 {
        pieces.push(arc_of(e, hi, sw));
    }
    Cut::Pieces(pieces)
}

pub fn break_ellipse(e: &EllipseGeom, p1: Vec2, p2: Vec2) -> Cut {
    let d1 = norm_angle(closest_param(e, p1) - e.t0);
    let d2 = norm_angle(closest_param(e, p2) - e.t0);
    let sw = ellipse_sweep(e);
    let same = (d1 - d2).abs() < 1e-9;
    if is_full_ellipse(e) {
        if same {
            return Cut::Error("Elips tek noktadan kırılamaz; ikinci bir nokta gösterin.".into());
        }
        // Removed: counter-clockwise from the first pick to the second.
        return Cut::Pieces(vec![arc_of(e, d2, if d1 > d2 { d1 } else { d1 + TAU })]);
    }
    if same {
        if d1 <= 1e-9 || d1 >= sw - 1e-9 {
            return Cut::Error("Yay ucundan kırılamaz; iç kısmında bir nokta gösterin.".into());
        }
        return Cut::Pieces(vec![arc_of(e, 0.0, d1), arc_of(e, d1, sw)]);
    }
    let lo = js_min(d1, d2);
    let hi = js_max(d1, d2);
    let mut pieces = Vec::new();
    if lo > 1e-9 {
        pieces.push(arc_of(e, 0.0, lo));
    }
    if hi < sw - 1e-9 {
        pieces.push(arc_of(e, hi, sw));
    }
    if pieces.is_empty() {
        Cut::Error("İki nokta yayın tamamını kapsıyor; silmek için Sil kullanın.".into())
    } else {
        Cut::Pieces(pieces)
    }
}

/// Grows the end of an elliptical arc nearer to `pick` along its ellipse to the first boundary.
pub fn extend_ellipse(e: &EllipseGeom, pick: Vec2, boundaries: &[Edge]) -> Geometry {
    if is_full_ellipse(e) {
        return Geometry::Error("Tam elips uzatılamaz.".into());
    }
    let s = ellipse_point(e, e.t0);
    let f = ellipse_point(e, e.t1);
    let at_end = js_hypot(pick.x - f.x, pick.y - f.y) <= js_hypot(pick.x - s.x, pick.y - s.y);
    let gap = TAU - ellipse_sweep(e);
    let mut best = f64::INFINITY;
    for t in ellipse_crossings(&full(e), boundaries) {
        let delta = if at_end {
            norm_angle(t - e.t1)
        } else {
            norm_angle(e.t0 - t)
        };
        if delta > 1e-9 && delta < gap - 1e-9 {
            best = js_min(best, delta);
        }
    }
    if !best.is_finite() {
        return Geometry::Error("Bu yönde ulaşılacak bir sınır yok.".into());
    }
    Geometry::Ok(Entity::new(Shape::Ellipse {
        c: e.c,
        major: e.major,
        ratio: e.ratio,
        t0: if at_end {
            e.t0
        } else {
            norm_angle(e.t0 - best)
        },
        t1: if at_end {
            norm_angle(e.t1 + best)
        } else {
            e.t1
        },
    }))
}

/// Offset of an ellipse at distance d towards `through`: not an ellipse, so a
/// dense polyline through exact offset points (like AutoCAD).
pub fn offset_ellipse(e: &EllipseGeom, d: f64, through: Vec2) -> Geometry {
    let t = closest_param(e, through);
    let q = ellipse_point(e, t);
    let tan = ellipse_derivative(e, t);
    // Counter-clockwise parameterisation: the outside is right of travel.
    let outward = (through.x - q.x) * tan.y - (through.y - q.y) * tan.x > 0.0;
    let a = js_hypot(e.major.x, e.major.y);
    let b = a * e.ratio;
    if !outward && d >= (b * b) / a - 1e-9 {
        return Geometry::Error(
            "Mesafe elipsin en küçük eğrilik yarıçapından büyük; içe doğru öteleme bozulur.".into(),
        );
    }
    let closed = is_full_ellipse(e);
    let n = 512.0;
    let sw = ellipse_sweep(e);
    let end = if closed { n } else { n + 1.0 };
    let mut pts = Vec::new();
    let mut i = 0.0;
    while i < end {
        let u = e.t0 + (sw * i) / n;
        let p = ellipse_point(e, u);
        let dp = ellipse_derivative(e, u);
        let l = or(js_hypot(dp.x, dp.y), 1.0);
        let s = if outward { d } else { -d };
        pts.push(Vec2::new(p.x + (dp.y / l) * s, p.y - (dp.x / l) * s));
        i += 1.0;
    }
    Geometry::Ok(Entity::new(if closed {
        Shape::Polygon {
            pts,
            bulges: None,
            holes: None,
        }
    } else {
        Shape::Polyline {
            pts,
            bulges: None,
            holes: None,
        }
    }))
}

// ── Construction lines ─────────────────────────────────────────────────

pub struct Construction {
    pub p: Vec2,
    pub dir: Vec2,
    pub ray: bool,
}

/// Piece of the line p + dir·t between lo and hi (±∞ allowed).
fn line_piece(e: &Construction, lo: f64, hi: f64) -> Entity {
    let at = |t: f64| Vec2::new(e.p.x + e.dir.x * t, e.p.y + e.dir.y * t);
    Entity::new(if lo == f64::NEG_INFINITY && hi == f64::INFINITY {
        Shape::Xline { p: e.p, dir: e.dir }
    } else if lo == f64::NEG_INFINITY {
        // `|| 0` turns −0 into 0 so reversed axis directions stay clean.
        Shape::Ray {
            p: at(hi),
            dir: Vec2::new(or(-e.dir.x, 0.0), or(-e.dir.y, 0.0)),
        }
    } else if hi == f64::INFINITY {
        Shape::Ray {
            p: at(lo),
            dir: e.dir,
        }
    } else {
        Shape::Line {
            a: at(lo),
            b: at(hi),
        }
    })
}

fn param_on(e: &Construction, p: Vec2) -> f64 {
    (p.x - e.p.x) * e.dir.x + (p.y - e.p.y) * e.dir.y
}

/// Line parameters (metres from p along dir) where boundaries cross, solved from p itself.
fn line_cuts(e: &Construction, boundaries: &[Edge]) -> Vec<f64> {
    let q = Vec2::new(e.p.x + e.dir.x, e.p.y + e.dir.y);
    let mut out = Vec::new();
    for b in boundaries {
        match *b {
            Edge::Seg { a, b: bb } => {
                if let Some(h) = line_line(e.p, q, a, bb)
                    && h.u >= -1e-9
                    && h.u <= 1.0 + 1e-9
                {
                    out.push(h.t);
                }
            }
            Edge::Arc { c, r, a0, sweep } => {
                for t in line_circle_params(e.p, q, c, r) {
                    let x = e.p.x + e.dir.x * t;
                    let y = e.p.y + e.dir.y * t;
                    if on_edge_arc(a0, sweep, atan2(y - c.y, x - c.x)) {
                        out.push(t);
                    }
                }
            }
        }
    }
    let min = if e.ray { 1e-9 } else { f64::NEG_INFINITY };
    let mut out: Vec<f64> = out.into_iter().filter(|&t| t > min).collect();
    stable_sort(&mut out, &mut |a, b| js_cmp(*a - *b, 0.0));
    out
}

pub fn trim_construction(e: &Construction, pick: Vec2, boundaries: &[Edge]) -> Cut {
    let cuts = line_cuts(e, boundaries);
    let tp = param_on(e, pick);
    let start = if e.ray { 0.0 } else { f64::NEG_INFINITY };
    let lo = js_max_all(std::iter::once(start).chain(cuts.iter().copied().filter(|&c| c < tp)));
    let hi =
        js_min_all(std::iter::once(f64::INFINITY).chain(cuts.iter().copied().filter(|&c| c > tp)));
    if lo == start && hi == f64::INFINITY {
        return Cut::Error("Tıklanan kısmı kesen bir kenar yok.".into());
    }
    let mut pieces = Vec::new();
    if lo > start {
        pieces.push(line_piece(e, start, lo));
    }
    if hi < f64::INFINITY {
        pieces.push(line_piece(e, hi, f64::INFINITY));
    }
    Cut::Pieces(pieces)
}

pub fn break_construction(e: &Construction, p1: Vec2, p2: Vec2) -> Cut {
    let t1 = param_on(e, p1);
    let t2 = param_on(e, p2);
    let start = if e.ray { 0.0 } else { f64::NEG_INFINITY };
    let lo = js_max(start, js_min(t1, t2));
    let hi = js_max(t1, t2);
    if (t1 - t2).abs() < 1e-9 {
        if lo <= start + 1e-9 {
            return Cut::Error(
                "Işın başlangıcından kırılamaz; ilerisinde bir nokta gösterin.".into(),
            );
        }
        return Cut::Pieces(vec![
            line_piece(e, start, lo),
            line_piece(e, lo, f64::INFINITY),
        ]);
    }
    let mut pieces = Vec::new();
    if lo > start + 1e-9 {
        pieces.push(line_piece(e, start, lo));
    }
    pieces.push(line_piece(e, hi, f64::INFINITY));
    Cut::Pieces(pieces)
}

pub fn offset_construction(e: &Construction, d: f64, through: Vec2) -> Geometry {
    let side = if e.dir.x * (through.y - e.p.y) - e.dir.y * (through.x - e.p.x) >= 0.0 {
        1.0
    } else {
        -1.0
    };
    let p = Vec2::new(e.p.x - e.dir.y * d * side, e.p.y + e.dir.x * d * side);
    Geometry::Ok(Entity::new(if e.ray {
        Shape::Ray { p, dir: e.dir }
    } else {
        Shape::Xline { p, dir: e.dir }
    }))
}

/// The ellipse or construction line of an entity (callers check the kind first).
pub fn ellipse_of(s: &Shape) -> Option<EllipseGeom> {
    match s {
        Shape::Ellipse {
            c,
            major,
            ratio,
            t0,
            t1,
        } => Some(ellipse_geom(*c, *major, *ratio, *t0, *t1)),
        _ => None,
    }
}

pub fn construction_of(s: &Shape) -> Option<Construction> {
    match s {
        Shape::Xline { p, dir } => Some(Construction {
            p: *p,
            dir: *dir,
            ray: false,
        }),
        Shape::Ray { p, dir } => Some(Construction {
            p: *p,
            dir: *dir,
            ray: true,
        }),
        _ => None,
    }
}

use crate::api::Op;
use crate::op;

fn ellipse_arg(e: &Entity) -> Result<EllipseGeom, String> {
    ellipse_of(&e.shape).ok_or_else(|| "elips bekleniyordu".to_string())
}

fn construction_arg(e: &Entity) -> Result<Construction, String> {
    construction_of(&e.shape).ok_or_else(|| "yardımcı çizgi ya da ışın bekleniyordu".to_string())
}

pub(crate) static OPS: &[Op] = &[
    op!("ellipseCrossings", |e: Entity, boundaries: Vec<Edge>| {
        ellipse_arg(&e).map(|g| ellipse_crossings(&g, &boundaries))
    }),
    op!(
        "trimEllipse",
        |e: Entity, pick: Vec2, boundaries: Vec<Edge>| ellipse_arg(&e).map(|g| trim_ellipse(
            &g,
            pick,
            &boundaries
        ))
    ),
    op!("breakEllipse", |e: Entity, p1: Vec2, p2: Vec2| ellipse_arg(
        &e
    )
    .map(|g| break_ellipse(&g, p1, p2))),
    op!(
        "extendEllipse",
        |e: Entity, pick: Vec2, boundaries: Vec<Edge>| ellipse_arg(&e).map(|g| extend_ellipse(
            &g,
            pick,
            &boundaries
        ))
    ),
    op!("offsetEllipse", |e: Entity, d: f64, through: Vec2| {
        ellipse_arg(&e).map(|g| offset_ellipse(&g, d, through))
    }),
    op!(
        "trimConstruction",
        |e: Entity, pick: Vec2, boundaries: Vec<Edge>| construction_arg(&e)
            .map(|c| trim_construction(&c, pick, &boundaries))
    ),
    op!("breakConstruction", |e: Entity, p1: Vec2, p2: Vec2| {
        construction_arg(&e).map(|c| break_construction(&c, p1, p2))
    }),
    op!("offsetConstruction", |e: Entity, d: f64, through: Vec2| {
        construction_arg(&e).map(|c| offset_construction(&c, d, through))
    }),
];
