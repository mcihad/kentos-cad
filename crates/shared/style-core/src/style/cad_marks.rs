//! CAD linetype marks keep the source phase, corners and R/A/U policy.
//! Templates are frozen bundled-font polygons; no backend shapes text.
use super::{
    compile::{Env, to_world},
    model::Unit,
    prim::{FillPaint, Sink, StrokeStyle},
};
use kentos_geometry_core::jsmath::{atan2, cos, js_hypot, js_max, js_sign, sin};
use kentos_geometry_core::{
    Vec2,
    api::json::Json,
    entity::Shape,
    geom::{
        arc,
        bulge::{BulgeArc, bulge_arc, bulge_at},
        ellipse::{EllipseGeom, ellipse_derivative, ellipse_point, ellipse_sweep, major_length},
        quadrature::integrate,
    },
};

/// Exact carrier distances and tangents; rendering tessellation never changes
/// the repetition phase. Straight callers retain their original path order.
pub(super) struct Route {
    pub(super) edges: Vec<Edge>,
    pub(super) total: f64,
    pub(super) ellipse: Option<EllipseRoute>,
    /// A NURBS curve's own arc length (ADR 1122): its parameter at a distance.
    pub(super) nurbs: Option<NurbsRoute>,
}

/// A NURBS carrier measured on the curve itself, span by span (`Nurbs::length_between`).
pub(super) struct NurbsRoute {
    pub(super) curve: kentos_geometry_core::geom::nurbs::OwnedNurbs,
}
impl NurbsRoute {
    fn sample(&self, distance: f64) -> Result<(Vec2, f64), String> {
        let c = self.curve.view();
        let u = c.param_at_length(distance);
        let d = c.derivs(u, 1);
        if !u.is_finite() || !d[0].x.is_finite() || !d[1].x.is_finite() || (d[1].x == 0. && d[1].y == 0.) {
            return Err("NURBS üzerindeki çizgi tipi işaretinin konumu doğrulanamadı; eksik çıktı üretilmedi.".into());
        }
        Ok((d[0], atan2(d[1].y, d[1].x)))
    }
}

/// Integrate normalized speed so large model axes cannot overflow quadrature.
/// Eight cached intervals bound each inverse solve; Newton stays bracketed and
/// returns an explicit error if its arc-distance residual does not converge.
pub(super) struct EllipseRoute {
    pub(super) total: f64,
    pub(super) geom: EllipseGeom,
    radius: f64,
    knots: Vec<(f64, f64)>,
}
impl EllipseRoute {
    fn new(geom: EllipseGeom) -> Result<Self, String> {
        let radius = major_length(&geom);
        let sweep = ellipse_sweep(&geom);
        if !radius.is_finite()
            || radius <= 0.
            || !geom.c.x.is_finite()
            || !geom.c.y.is_finite()
            || !geom.ratio.is_finite()
            || geom.ratio <= 0.
            || geom.ratio > 1.
            || !geom.t0.is_finite()
            || !sweep.is_finite()
            || sweep <= 0.
            || geom.t0 + sweep <= geom.t0
        {
            return Err("Çizgi tipi elips yolu geçersiz.".into());
        }
        let mut route = Self {
            total: 0.,
            geom,
            radius,
            knots: vec![(geom.t0, 0.)],
        };
        let mut length = 0.;
        for i in 0..8 {
            let a = geom.t0 + sweep * i as f64 / 8.;
            let b = geom.t0 + sweep * (i + 1) as f64 / 8.;
            length += route.run(a, b);
            route.knots.push((b, length));
        }
        if !length.is_finite() || !(length * radius).is_finite() || length <= 0. {
            return Err("Çizgi tipi elips uzunluğu geçersiz.".into());
        }
        route.total = length * radius;
        Ok(route)
    }
    fn speed(&self, t: f64) -> f64 {
        js_hypot(sin(t), self.geom.ratio * cos(t))
    }
    fn run(&self, a: f64, b: f64) -> f64 {
        if a == b {
            0.
        } else {
            integrate(&|t| self.speed(t), a, b, 1e-14)
        }
    }
    /// The arc distance from the start to parameter `t` (`t0 ≤ t ≤ t0 + sweep`).
    pub(super) fn distance(&self, t: f64) -> f64 {
        let k = self
            .knots
            .windows(2)
            .position(|k| t <= k[1].0)
            .unwrap_or(self.knots.len().saturating_sub(2));
        let (start, base) = self.knots[k];
        (base + self.run(start, t)) * self.radius
    }
    fn sample(&self, distance: f64, tolerance: f64) -> Result<(Vec2, f64), String> {
        let wanted = distance / self.radius;
        let edge = self
            .knots
            .windows(2)
            .find(|k| wanted <= k[1].1)
            .ok_or("Elips işareti yolun dışında.")?;
        let ((start, base), (end, finish)) = (edge[0], edge[1]);
        let target = (wanted - base).clamp(0., finish - base);
        let mut lo = start;
        let mut hi = end;
        let mut t = start + (end - start) * target / (finish - base);
        let tolerance = tolerance / self.radius;
        for _ in 0..64 {
            let residual = self.run(start, t) - target;
            if residual.abs() <= tolerance {
                let at = ellipse_point(&self.geom, t);
                let d = ellipse_derivative(&self.geom, t);
                if at.x.is_finite() && at.y.is_finite() && d.x.is_finite() && d.y.is_finite() {
                    return Ok((at, atan2(d.y, d.x)));
                }
                break;
            }
            if residual > 0. {
                hi = t;
            } else {
                lo = t;
            }
            let speed = self.speed(t);
            let next = t - residual / speed;
            t = if next.is_finite() && next > lo && next < hi {
                next
            } else {
                (lo + hi) * 0.5
            };
        }
        Err(
            "Elips üzerindeki çizgi tipi işaretinin konumu doğrulanamadı; eksik çıktı üretilmedi."
                .into(),
        )
    }
}
pub(super) struct Edge {
    pub(super) a: Vec2,
    pub(super) b: Vec2,
    pub(super) arc: Option<BulgeArc>,
    pub(super) start: f64,
    pub(super) length: f64,
}
impl Edge {
    fn sample(&self, distance: f64) -> (Vec2, f64) {
        let f = ((distance - self.start) / self.length).clamp(0., 1.);
        if let Some(a) = self.arc {
            let angle = a.a0 + a.sweep * f;
            (
                Vec2::new(a.c.x + a.r * cos(angle), a.c.y + a.r * sin(angle)),
                angle + js_sign(a.sweep) * std::f64::consts::FRAC_PI_2,
            )
        } else {
            (
                Vec2::new(
                    self.a.x + (self.b.x - self.a.x) * f,
                    self.a.y + (self.b.y - self.a.y) * f,
                ),
                atan2(self.b.y - self.a.y, self.b.x - self.a.x),
            )
        }
    }
}
impl Route {
    /// The point on the source curve itself at `distance` from its start (ADR 1122).
    pub(super) fn point_at(&self, distance: f64) -> Option<Vec2> {
        let d = distance.clamp(0., self.total);
        if let Some(e) = &self.ellipse {
            return e.sample(d, js_max(self.total, 1.) * 1e-13).ok().map(|s| s.0);
        }
        if let Some(n) = &self.nurbs {
            return n.sample(d).ok().map(|s| s.0);
        }
        let edge = self
            .edges
            .iter()
            .find(|e| d <= e.start + e.length)
            .or(self.edges.last())?;
        Some(edge.sample(d).0)
    }
    fn push(&mut self, a: Vec2, b: Vec2, arc: Option<BulgeArc>) -> Result<(), String> {
        let length = arc.map_or_else(|| js_hypot(b.x - a.x, b.y - a.y), |a| a.r * a.sweep.abs());
        if arc.is_some_and(|v| {
            !v.c.x.is_finite()
                || !v.c.y.is_finite()
                || !v.r.is_finite()
                || v.r <= 0.
                || !v.a0.is_finite()
                || !v.sweep.is_finite()
        }) || !length.is_finite()
            || !a.x.is_finite()
            || !a.y.is_finite()
            || !b.x.is_finite()
            || !b.y.is_finite()
        {
            return Err("Çizgi tipi yolu geçersiz.".into());
        }
        if length > 0. {
            self.edges.push(Edge {
                a,
                b,
                arc,
                start: self.total,
                length,
            });
            self.total += length;
        }
        if !self.total.is_finite() {
            return Err("Çizgi tipi yol uzunluğu geçersiz.".into());
        }
        Ok(())
    }
    pub(super) fn polyline(
        pts: &[Vec2],
        bulges: Option<&[f64]>,
        closed: bool,
    ) -> Result<Self, String> {
        let mut route = Self {
            edges: Vec::new(),
            total: 0.,
            ellipse: None,
            nurbs: None,
        };
        for i in 0..if closed {
            pts.len()
        } else {
            pts.len().saturating_sub(1)
        } {
            let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
            let bulge = bulge_at(bulges, i);
            if !bulge.is_finite() {
                return Err("Çizgi tipi yay değeri geçersiz.".into());
            }
            route.push(a, b, bulge_arc(a, b, bulge))?;
        }
        Ok(route)
    }
    pub(super) fn of(shape: &Shape) -> Option<Result<Self, String>> {
        let (c, r, a0, sweep) = match shape {
            Shape::Polyline {
                pts,
                bulges,
                closed,
                holes,
                parts,
                ..
            } if holes.as_ref().is_none_or(Vec::is_empty)
                && parts.as_ref().is_none_or(Vec::is_empty)
                && kentos_geometry_core::geom::bulge::has_bulges(bulges.as_deref()) =>
            {
                return Some(Self::polyline(pts, bulges.as_deref(), *closed));
            }
            Shape::Arc { c, r, a0, a1 } => (*c, *r, *a0, arc::sweep(*a0, *a1)),
            Shape::Circle { c, r } => (*c, *r, 0., std::f64::consts::TAU),
            Shape::Ellipse {
                c,
                major,
                ratio,
                t0,
                t1,
            } => {
                return Some(
                    EllipseRoute::new(EllipseGeom {
                        c: *c,
                        major: *major,
                        ratio: *ratio,
                        t0: *t0,
                        t1: *t1,
                    })
                    .map(|ellipse| Self {
                        total: ellipse.total,
                        edges: Vec::new(),
                        ellipse: Some(ellipse),
                        nurbs: None,
                    }),
                );
            }
            Shape::Nurbs { .. } => {
                let curve = kentos_geometry_core::geom::nurbs::of(shape)?;
                let total = curve.length();
                if !total.is_finite() || total <= 0. {
                    return Some(Err("Çizgi tipi NURBS yolunun uzunluğu geçersiz.".into()));
                }
                let curve = kentos_geometry_core::geom::nurbs::OwnedNurbs {
                    degree: curve.degree,
                    ctrl: curve.ctrl.to_vec(),
                    knots: curve.knots.to_vec(),
                    weights: curve.weights.map(<[f64]>::to_vec),
                };
                return Some(Ok(Self {
                    edges: Vec::new(),
                    total,
                    ellipse: None,
                    nurbs: Some(NurbsRoute { curve }),
                }));
            }
            _ => return None,
        };
        let mut route = Self {
            edges: Vec::new(),
            total: 0.,
            ellipse: None,
            nurbs: None,
        };
        let point = |a: f64| Vec2::new(c.x + r * cos(a), c.y + r * sin(a));
        Some(
            route
                .push(
                    point(a0),
                    point(a0 + sweep),
                    Some(BulgeArc { c, r, a0, sweep }),
                )
                .map(|_| route),
        )
    }
}

/// Intrinsic CAD strokes only: custom offsets/effects keep the existing symbol
/// path. Carrier tessellation stays unchanged; marks use the original curve.
pub(super) fn compile_curved(
    shape: &Shape,
    parts: &[super::compile::Geom],
    symbol: &super::model::Symbol,
    values: &dyn super::compile::Values,
    env: &Env,
    sink: &mut dyn Sink,
    level: f64,
) -> bool {
    use super::{
        compile::{Geom, emit_line_layer_on, stroke_style},
        model::{Layer, SymbolType},
    };
    let [Layer::SimpleLine(l)] = symbol.layers.as_slice() else {
        return false;
    };
    // Marks, or a CAD line type's dash measured on the curve (ADR 1122).
    let dashed = env.cad_phase && l.dash.as_ref().is_some_and(|d| !d.is_empty());
    if symbol.kind != SymbolType::Line
        || (l.cad_marks.is_none() && !dashed)
        || l.offset.is_some()
        || l.shift.is_some()
        || l.base.effects.is_some()
    {
        return false;
    }
    let Some(route) = Route::of(shape) else {
        return false;
    };
    // A plain dash on a curve the route refuses keeps the ordinary drawing:
    // only marks need the route to be drawn at all.
    if l.cad_marks.is_none() && route.is_err() {
        return false;
    }
    let lines: Vec<(&[Vec2], bool)> = match parts {
        [Geom::Line(paths)] if paths.len() == 1 || (l.cad_marks.is_none() && !paths.is_empty()) => {
            paths.iter().map(|(p, c)| (p.as_slice(), *c)).collect()
        }
        [Geom::Fill(rings)] if rings.len() == 1 => vec![(rings[0].as_slice(), true)],
        _ => return false,
    };
    for (path, closed) in lines {
        match &route {
            Ok(route) => emit_line_layer_on(
                &symbol.layers[0],
                (path, closed),
                level,
                values,
                env,
                sink,
                Some(route),
            ),
            Err(error) => {
                if let Some(mut style) = stroke_style(l, level, values, env) {
                    style.line_type_loss = Some(error.clone());
                    sink.stroke(&style, path, closed);
                }
            }
        }
    }
    true
}

#[derive(Clone, Debug)]
pub struct Mark {
    at: f64,
    cap: f64,
    height: f64,
    x: f64,
    y: f64,
    rotation: f64,
    absolute: bool,
    upright: bool,
    polygons: Vec<MarkPolygon>,
    paths: Vec<Vec<Vec2>>,
    cost: usize,
}
#[derive(Clone, Debug)]
struct MarkPolygon {
    rings: Vec<Vec<Vec2>>,
    indices: Vec<u32>,
}
#[derive(Clone, Debug)]
pub struct Marks {
    period: f64,
    marks: Vec<Mark>,
    name: String,
}
const MAX_POINTS: usize = 500_000;
const MAX_MARKS: usize = 2000;
fn number(v: &Json) -> Result<f64, String> {
    match v {
        Json::Num(n) if n.is_finite() => Ok(*n),
        _ => Err("Çizgi tipi işaretinde geçersiz sayı.".into()),
    }
}
fn array(v: &Json) -> Result<&[Json], String> {
    match v {
        Json::Arr(a) => Ok(a),
        _ => Err("Çizgi tipi işaretinin konturu geçersiz.".into()),
    }
}
impl Marks {
    pub fn read(v: &Json) -> Option<Result<Self, String>> {
        if matches!(v, Json::Null) {
            return None;
        }
        Some(Self::parse(v))
    }
    fn parse(v: &Json) -> Result<Self, String> {
        let period = number(v.get("period"))?;
        if period <= 0. {
            return Err("Çizgi tipinin tekrar uzunluğu pozitif olmalı.".into());
        }
        let mut marks = Vec::new();
        let mut cost = 0;
        let list = array(v.get("marks"))?;
        if list.len() > 12 {
            return Err("Çizgi tipi işaret sayısı sınırı aşıldı.".into());
        }
        for m in list {
            let before = cost;
            let mut polygons = Vec::new();
            for p in array(m.get("polygons"))? {
                let mut rings = Vec::new();
                for r in array(p)? {
                    let mut points = Vec::new();
                    for p in array(r)? {
                        let pair = array(p)?;
                        if pair.len() != 2 {
                            return Err("Çizgi tipi noktası geçersiz.".into());
                        }
                        points.push(Vec2::new(number(&pair[0])?, number(&pair[1])?));
                        cost += 1;
                        if cost > 32_768 {
                            return Err("Çizgi tipi kaynak konturu sınırı aşıldı.".into());
                        }
                    }
                    if points.len() < 3 {
                        return Err("Çizgi tipi konturunda en az üç nokta gerekli.".into());
                    }
                    rings.push(points);
                }
                if rings.is_empty() {
                    return Err("Çizgi tipi konturu boş.".into());
                }
                let points: Vec<_> = rings.iter().flatten().copied().collect();
                let sizes: Vec<_> = rings.iter().map(Vec::len).collect();
                let indices = kentos_geometry_core::triangulate::triangulate_many(
                    &points,
                    &sizes,
                    &[rings.len()],
                );
                if indices.is_empty() {
                    return Err("Çizgi tipi konturu üçgenlenemedi.".into());
                }
                polygons.push(MarkPolygon { rings, indices });
            }
            let mark = Mark {
                at: number(m.get("at"))?,
                cap: number(m.get("cap"))?,
                height: number(m.get("height"))?,
                x: number(m.get("x"))?,
                y: number(m.get("y"))?,
                rotation: number(m.get("rotation"))?.to_radians(),
                absolute: matches!(m.get("absolute"), Json::Bool(true)),
                upright: matches!(m.get("upright"), Json::Bool(true)),
                polygons,
                paths: if matches!(m.get("paths"), Json::Null) {
                    Vec::new()
                } else {
                    let mut paths = Vec::new();
                    for path in array(m.get("paths"))? {
                        let mut points = Vec::new();
                        for p in array(path)? {
                            let pair = array(p)?;
                            if pair.len() != 2 {
                                return Err("SHX şekil noktası geçersiz.".into());
                            }
                            points.push(Vec2::new(number(&pair[0])?, number(&pair[1])?));
                            cost += 1;
                            if cost > 32_768 {
                                return Err("Çizgi tipi kaynak konturu sınırı aşıldı.".into());
                            }
                        }
                        if points.len() < 2 {
                            return Err("SHX şekil yolunda en az iki nokta gerekli.".into());
                        }
                        paths.push(points);
                    }
                    paths
                },
                cost: cost - before,
            };
            if mark.cap < 0.
                || (mark.cap == 0. && mark.paths.is_empty())
                || mark.height <= 0.
                || mark.at < 0.
                || mark.at > period
            {
                return Err("Çizgi tipi işaretinin boyu veya fazı geçersiz.".into());
            }
            marks.push(mark);
        }
        Ok(Self {
            period,
            marks,
            name: match v.get("name") {
                Json::Str(s) => s.clone(),
                _ => "Çizgi tipi".into(),
            },
        })
    }
    pub fn source(&self) -> &str {
        &self.name
    }
    /// Validate the whole path before painting. A limit never returns a
    /// misleading partially drawn prefix, and never loops through hidden cycles.
    pub fn placements<'a>(
        &'a self,
        path: &[Vec2],
        closed: bool,
        phase: f64,
        unit: Unit,
        env: &Env,
    ) -> Result<Vec<Placement<'a>>, String> {
        if path.len() < 2 {
            return Ok(Vec::new());
        }
        let route = Route::polyline(path, None, closed)?;
        self.placements_on(&route, phase, unit, env)
    }
    pub(super) fn placements_on<'a>(
        &'a self,
        route: &Route,
        phase: f64,
        unit: Unit,
        env: &Env,
    ) -> Result<Vec<Placement<'a>>, String> {
        if !phase.is_finite() {
            return Err("Çizgi tipi başlangıç fazı geçersiz.".into());
        }
        let period = to_world(self.period, unit, env);
        if !period.is_finite() || period <= 0. {
            return Err("Çizgi tipi tekrar uzunluğu çizim ölçeğinde geçersiz.".into());
        }
        let total = route.total;
        let mut placed = Vec::new();
        let mut cost = 0usize;
        for m in &self.marks {
            let at = to_world(m.at, unit, env);
            let mut distance = at + phase.rem_euclid(period);
            if distance >= period && at < period {
                distance -= period;
            }
            let count = js_max(((total - distance) / period).ceil(), 0.);
            if count > MAX_MARKS as f64
                || placed.len().saturating_add(count as usize) > MAX_MARKS
                || cost.saturating_add((count as usize).saturating_mul(m.cost)) > MAX_POINTS
            {
                return Err("Çizgi tipi işaretleri çizim karmaşıklığı sınırını aşıyor; eksik çıktı üretilmedi.".into());
            }
            let mut segment = 0;
            for _ in 0..count as usize {
                if total - distance <= period * 1e-10 {
                    break;
                }
                let (at, tangent) = if let Some(ellipse) = &route.ellipse {
                    ellipse.sample(distance, period * 1e-11)?
                } else if let Some(nurbs) = &route.nurbs {
                    nurbs.sample(distance)?
                } else {
                    while segment + 1 < route.edges.len()
                        && route.edges[segment].start + route.edges[segment].length <= distance
                    {
                        segment += 1;
                    }
                    let Some(edge) = route.edges.get(segment) else {
                        break;
                    };
                    edge.sample(distance)
                };
                placed.push(Placement {
                    mark: m,
                    at,
                    tangent,
                });
                cost = cost.saturating_add(m.cost);
                distance += period;
            }
        }
        Ok(placed)
    }
}
pub struct Placement<'a> {
    mark: &'a Mark,
    pub at: Vec2,
    pub tangent: f64,
}
impl Placement<'_> {
    pub fn cost(&self) -> usize {
        self.mark.cost
    }
    pub fn paint(&self, style: &StrokeStyle, unit: Unit, env: &Env, sink: &mut dyn Sink) {
        let m = self.mark;
        let height = to_world(m.height, unit, env);
        let (ts, tc) = (sin(self.tangent), cos(self.tangent));
        let x = to_world(m.x, unit, env);
        let y = to_world(m.y, unit, env);
        let origin = Vec2::new(self.at.x + tc * x - ts * y, self.at.y + ts * x + tc * y);
        let base = if m.absolute { 0. } else { self.tangent };
        let mut angle = m.rotation + base;
        if m.upright && cos(angle) < 0. {
            angle += std::f64::consts::PI;
        }
        let (s, c) = (sin(angle), cos(angle));
        let paint = FillPaint::Solid {
            color: style.color.clone(),
            opacity: style.opacity,
            level: style.level,
        };
        if !m.paths.is_empty() {
            let mut plain = style.clone();
            plain.dash = None;
            plain.dash_offset = 0.;
            // An SHX shape (cap 0) turns about its own origin; SHX text (cap 1,
            // ADR 1120 §3) about the same midpoint as a filled text's glyphs.
            for path in &m.paths {
                let points: Vec<_> = path
                    .iter()
                    .map(|p| {
                        if m.cap == 0. {
                            return Vec2::new(
                                origin.x + height * (c * p.x - s * p.y),
                                origin.y + height * (s * p.x + c * p.y),
                            );
                        }
                        let x = p.x * height;
                        let y = (p.y - m.cap * 0.5) * height;
                        Vec2::new(
                            origin.x - sin(base) * m.cap * 0.5 * height + c * x - s * y,
                            origin.y + cos(base) * m.cap * 0.5 * height + s * x + c * y,
                        )
                    })
                    .collect();
                sink.cad_shape(&plain, &points);
            }
        }
        for polygon in &m.polygons {
            let rings: Vec<Vec<Vec2>> = polygon
                .rings
                .iter()
                .map(|r| {
                    r.iter()
                        .map(|p| {
                            // Rotation pivot is midway between baseline and cap height.
                            let x = p.x * height;
                            let y = (p.y - m.cap * 0.5) * height;
                            Vec2::new(
                                origin.x - sin(base) * m.cap * 0.5 * height + c * x - s * y,
                                origin.y + cos(base) * m.cap * 0.5 * height + s * x + c * y,
                            )
                        })
                        .collect()
                })
                .collect();
            sink.cad_mark_mesh(&paint, &rings, &polygon.indices);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::prim::{PrimUnit, PrimitiveList};
    use super::*;
    use std::collections::HashMap;
    fn marks(rotation: f64, absolute: bool, upright: bool) -> Marks {
        let s = serde_json::json!({"period":4.,"marks":[{"at":3.,"cap":1.,"height":2.,"x":0.5,"y":-0.25,
            "rotation":rotation,"absolute":absolute,"upright":upright,"polygons":[[[[0.,0.],[1.,0.],[0.,1.]]]]}]});
        Marks::read(&Json::parse(&s.to_string()).unwrap())
            .unwrap()
            .unwrap()
    }
    #[test]
    fn ellipse_arc_distance_matches_independent_50_digit_elliptic_integrals() {
        // mpmath 1.3.0 Legendre E, 50 decimal digits; NIST DLMF 19.30.
        // The implementation uses normalized adaptive Gauss quadrature instead.
        let cases = [
            (
                0.5,
                0.0,
                std::f64::consts::TAU,
                4.844224110273838,
                vec![
                    (
                        0.6,
                        0.5996527729281464,
                        0.4001301513006689,
                        0.9277291812335188,
                    ),
                    (
                        1.4,
                        -0.1886584527989604,
                        0.49102138145591545,
                        1.7605922154095321,
                    ),
                    (
                        2.2,
                        -0.9161435650074582,
                        0.20042515329632737,
                        2.729147858930695,
                    ),
                    (
                        3.0,
                        -0.6202885168850302,
                        -0.3921868673925,
                        4.043278499771425,
                    ),
                    (
                        3.8,
                        0.16663605360504063,
                        -0.49300923562316207,
                        4.879806012374701,
                    ),
                    (
                        4.6,
                        0.9011903450343653,
                        -0.21671176826423239,
                        5.8348970802208004,
                    ),
                ],
            ),
            (
                0.01,
                5.4,
                1.6,
                0.6119369060417719,
                vec![
                    (0.0, 0.6346928759426343, -0.007727644875559873, 5.4),
                    (
                        0.12,
                        0.7546871498047105,
                        -0.006560848313439676,
                        5.567566124900263,
                    ),
                    (
                        0.4,
                        0.9657975741647686,
                        0.0025929721505918305,
                        6.545479765801798,
                    ),
                    (
                        0.55,
                        0.8158341423400449,
                        0.005782859605699272,
                        6.899811464010851,
                    ),
                ],
            ),
        ];
        for (ratio, t0, sweep, length, points) in cases {
            for c in [Vec2::new(0., 0.), Vec2::new(1_000_000., 4_500_000.)] {
                let metric = EllipseRoute::new(EllipseGeom {
                    c,
                    major: Vec2::new(0.6, 0.8),
                    ratio,
                    t0,
                    t1: t0 + sweep,
                })
                .unwrap();
                assert_eq!(metric.knots.len(), 9);
                assert!((metric.knots.last().unwrap().1 - length).abs() < 1e-12);
                for &(distance, x, y, t) in &points {
                    let (at, direction) = metric.sample(distance, 1e-12).unwrap();
                    let want = Vec2::new(c.x + 0.6 * x - 0.8 * y, c.y + 0.8 * x + 0.6 * y);
                    assert!(
                        js_hypot(at.x - want.x, at.y - want.y) < 1e-8,
                        "ratio={ratio}: {at:?} vs {want:?}"
                    );
                    let dx = -0.6 * sin(t) - 0.8 * ratio * cos(t);
                    let dy = -0.8 * sin(t) + 0.6 * ratio * cos(t);
                    let angle = atan2(dy, dx);
                    assert!((sin(direction) - sin(angle)).abs() < 1e-10);
                    assert!((cos(direction) - cos(angle)).abs() < 1e-10);
                }
            }
        }
        for ratio in [0., -0.5, f64::NAN, 1.1] {
            assert!(
                EllipseRoute::new(EllipseGeom {
                    c: Vec2::new(0., 0.),
                    major: Vec2::new(1., 0.),
                    ratio,
                    t0: 0.,
                    t1: 0.
                })
                .is_err()
            );
        }
    }
    #[test]
    fn ellipse_marks_keep_full_distance_and_bound_dense_inverse_sampling() {
        let aspects = HashMap::new();
        let env = env(&aspects);
        let mut m = marks(0., false, false);
        let started = std::time::Instant::now();
        for ratio in [1., 0.5, 0.001] {
            let shape = Shape::Ellipse {
                c: Vec2::new(0., 0.),
                major: Vec2::new(1., 0.),
                ratio,
                t0: 0.,
                t1: 0.,
            };
            let route = Route::of(&shape).unwrap().unwrap();
            m.period = route.total / 1000.;
            m.marks[0].at = 0.;
            let placed = m.placements_on(&route, 0., Unit::M, &env).unwrap();
            assert_eq!(placed.len(), 1000);
            assert!((placed[0].at.x - 1.).abs() < 1e-12);
            assert!((placed[0].at.y).abs() < 1e-12);
            m.period = route.total / 2001.;
            assert!(m.placements_on(&route, 0., Unit::M, &env).is_err());
        }
        eprintln!("3000 bounded ellipse marks: {:?}", started.elapsed());
    }
    #[test]
    fn analytic_arc_anchors_and_tangents_ignore_tessellation_for_both_directions() {
        let aspects = HashMap::new();
        let env = env(&aspects);
        let mut marks = marks(0., false, false);
        marks.period = 0.37;
        marks.marks[0].at = 0.13;
        for sign in [-1., 1.] {
            let route = Route::polyline(
                &[Vec2::new(2., 0.), Vec2::new(-2., 0.)],
                Some(&[sign]),
                false,
            )
            .unwrap();
            for phase in [0., 0.17, -0.19, 0.37 * 100_000. + 0.11] {
                let placed = marks.placements_on(&route, phase, Unit::M, &env).unwrap();
                let first = (0.13 + phase.rem_euclid(0.37)) % 0.37;
                assert_eq!(
                    placed.len(),
                    ((2. * std::f64::consts::PI - first) / 0.37).ceil() as usize
                );
                for (i, p) in placed.iter().enumerate() {
                    let angle = sign * (first + i as f64 * 0.37) / 2.;
                    assert!((p.at.x - 2. * angle.cos()).abs() < 1e-12);
                    assert!((p.at.y - 2. * angle.sin()).abs() < 1e-12);
                    assert!(
                        (p.tangent - (angle + sign * std::f64::consts::FRAC_PI_2)).abs() < 1e-12
                    );
                }
            }
        }
    }
    #[test]
    fn mixed_line_arc_closed_seam_and_exact_vertex_keep_original_travel_distance() {
        let aspects = HashMap::new();
        let env = env(&aspects);
        let mut m = marks(0., false, false);
        m.period = 1.;
        m.marks[0].at = 0.;
        let route = Route::polyline(
            &[Vec2::new(0., 0.), Vec2::new(1., 0.), Vec2::new(3., 0.)],
            Some(&[0., 1.]),
            false,
        )
        .unwrap();
        let placed = m.placements_on(&route, 0., Unit::M, &env).unwrap();
        assert_eq!(placed.len(), 5);
        // At the vertex use the outgoing arc tangent, not the preceding line.
        assert!((placed[1].tangent.sin() + 1.).abs() < 1e-12);
        assert!((placed[2].at.x - (2. - 1f64.cos())).abs() < 1e-12);
        assert!((placed[2].at.y + 1f64.sin()).abs() < 1e-12);
        let circle = Route::of(&Shape::Circle {
            c: Vec2::new(0., 0.),
            r: 1.,
        })
        .unwrap()
        .unwrap();
        m.period = std::f64::consts::FRAC_PI_2;
        let placed = m.placements_on(&circle, 0., Unit::M, &env).unwrap();
        assert_eq!(placed.len(), 4);
        assert!((placed[3].at.y + 1.).abs() < 1e-12);
        assert!(
            Route::polyline(
                &[Vec2::new(0., 0.), Vec2::new(1., 0.)],
                Some(&[f64::INFINITY]),
                false
            )
            .is_err()
        );
    }
    #[test]
    fn intrinsic_curve_marks_reach_the_public_primitive_compiler_and_preserve_carrier() {
        use super::super::build::compile_one;
        let entities = [
            serde_json::json!({"kind":"arc","c":{"x":0.,"y":0.},"r":1.,"a0":0.,"a1":std::f64::consts::PI}),
            serde_json::json!({"kind":"circle","c":{"x":0.,"y":0.},"r":1.}),
            serde_json::json!({"kind":"polyline","pts":[{"x":1.,"y":0.},{"x":-1.,"y":0.}],"bulges":[1.]}),
        ];
        for (i, mut entity) in entities.into_iter().enumerate() {
            entity["id"] = 1.into();
            entity["layerId"] = "a".into();
            entity["attrs"] = serde_json::json!({});
            let mut input = serde_json::json!({"plotScale":1.,"entity":entity,"symbol":{"type":"line","layers":[{
                "id":"l","type":"simpleLine","unit":"m","color":"ink","width":0.01,
                "cadMarks":{"period":0.5,"marks":[{"at":0.,"cap":0.,"height":0.1,"x":0.,"y":0.,"rotation":0.,
                "polygons":[],"paths":[[[0.,0.],[0.,1.]]]}]}}]}});
            let out = compile_one(&Json::parse(&input.to_string()).unwrap()).unwrap();
            let out: serde_json::Value = serde_json::from_str(&out).unwrap();
            let strokes = out["strokes"].as_array().unwrap();
            assert_eq!(strokes.len(), if i == 1 { 14 } else { 8 });
            for (n, stroke) in strokes[1..].iter().enumerate() {
                let angle = n as f64 * 0.5;
                let p = &stroke["path"][0];
                assert!(
                    (p["x"].as_f64().unwrap() - angle.cos()).abs() < 1e-12,
                    "{i}: {p}"
                );
                assert!(
                    (p["y"].as_f64().unwrap() - angle.sin()).abs() < 1e-12,
                    "{i}: {p}"
                );
            }
            input["symbol"]["layers"][0]
                .as_object_mut()
                .unwrap()
                .remove("cadMarks");
            let plain: serde_json::Value = serde_json::from_str(
                &compile_one(&Json::parse(&input.to_string()).unwrap()).unwrap(),
            )
            .unwrap();
            assert_eq!(strokes[0], plain["strokes"][0], "carrier changed");
        }
    }
    #[test]
    fn shape_contours_remain_open_unfilled_and_rotate_about_the_source_origin() {
        use super::super::prim::PrimUnit;
        let raw = serde_json::json!({"period":4.,"marks":[{"at":0.,"cap":0.,"height":2.,"x":0.5,"y":0.,
            "rotation":90.,"polygons":[],"paths":[[[0.,-1.],[0.,1.]]]}]});
        let marks = Marks::read(&Json::parse(&raw.to_string()).unwrap())
            .unwrap()
            .unwrap();
        let aspects = HashMap::new();
        let env = env(&aspects);
        let placed = marks
            .placements(
                &[Vec2::new(0., 0.), Vec2::new(1., 0.)],
                false,
                0.,
                Unit::M,
                &env,
            )
            .unwrap();
        assert_eq!(placed.len(), 1);
        assert_eq!(placed[0].cost(), 2);
        let style = StrokeStyle {
            color: "#123456".into(),
            opacity: 0.5,
            width: 0.2,
            unit: PrimUnit::World,
            dash: Some(vec![1., 1.]),
            dash_offset: 1.,
            line_type_loss: None,
            cap: "round".into(),
            join: "miter".into(),
            blur: 0.,
            level: 3.,
            ground: false,
        };
        let mut sink = PrimitiveList::default();
        placed[0].paint(&style, Unit::M, &env, &mut sink);
        let out: serde_json::Value = serde_json::from_str(&sink.json()).unwrap();
        assert_eq!(out["strokes"].as_array().unwrap().len(), 1);
        assert!(out["fills"].as_array().unwrap().is_empty());
        let stroke = &out["strokes"][0];
        assert_eq!(stroke["closed"], false);
        assert!(stroke["style"]["dash"].is_null());
        assert_eq!(stroke["style"]["color"], "#123456");
        assert_eq!(stroke["style"]["width"], 0.2);
        assert!((stroke["path"][0]["x"].as_f64().unwrap() - 2.5).abs() < 1e-12);
        assert!((stroke["path"][1]["x"].as_f64().unwrap() + 1.5).abs() < 1e-12);
    }
    /// SHX text strokes (cap 1) turn about the mid-cap point of filled glyphs (ADR 1120 §3).
    #[test]
    fn shx_text_strokes_turn_about_the_same_point_as_filled_glyphs() {
        let aspects = HashMap::new();
        let env = env(&aspects);
        let ring = [[0., 0.], [1., 0.], [1., 1.]];
        let read = |m: serde_json::Value| {
            Marks::read(&Json::parse(&serde_json::json!({"period":4.,"marks":[m]}).to_string()).unwrap())
                .unwrap()
                .unwrap()
        };
        let text = read(serde_json::json!({"at":0.,"cap":1.,"height":2.,"x":0.5,"y":-0.25,"rotation":30.,
            "polygons":[],"paths":[ring]}));
        let fill = read(serde_json::json!({"at":0.,"cap":1.,"height":2.,"x":0.5,"y":-0.25,"rotation":30.,
            "polygons":[[ring]]}));
        let path = [Vec2::new(1., 2.), Vec2::new(3., 5.)];
        let style = StrokeStyle {
            color: "#123456".into(),
            opacity: 1.,
            width: 0.2,
            unit: super::super::prim::PrimUnit::World,
            dash: None,
            dash_offset: 0.,
            line_type_loss: None,
            cap: "round".into(),
            join: "miter".into(),
            blur: 0.,
            level: 0.,
            ground: false,
        };
        let paint = |m: &Marks| {
            let mut sink = PrimitiveList::default();
            for p in m.placements(&path, false, 0., Unit::M, &env).unwrap() {
                p.paint(&style, Unit::M, &env, &mut sink);
            }
            serde_json::from_str::<serde_json::Value>(&sink.json()).unwrap()
        };
        let strokes = paint(&text);
        let fills = paint(&fill);
        let stroke = strokes["strokes"][0]["path"].as_array().unwrap();
        let fill = fills["fills"][0]["rings"][0].as_array().unwrap();
        assert_eq!(stroke.len(), 3);
        for (a, b) in stroke.iter().zip(fill) {
            for k in ["x", "y"] {
                assert!((a[k].as_f64().unwrap() - b[k].as_f64().unwrap()).abs() < 1e-12, "{a} {b}");
            }
        }
    }
    fn env<'a>(aspects: &'a HashMap<String, f64>) -> Env<'a> {
        Env {
            plot_scale: 50.,
            aspects,
            screen: false,
            cad_phase: false,
            moment: None,
        }
    }
    #[test]
    fn corner_and_closed_seam_keep_distance_without_dropping_marks() {
        let aspects = HashMap::new();
        let env = env(&aspects);
        let m = marks(0., false, false);
        let pts = [
            Vec2::new(0., 0.),
            Vec2::new(2., 0.),
            Vec2::new(2., 2.),
            Vec2::new(0., 2.),
        ];
        let p = m.placements(&pts, true, 0., Unit::M, &env).unwrap();
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].at, Vec2::new(2., 1.));
        assert_eq!(p[1].at, Vec2::new(0., 1.));
        assert!((p[0].tangent - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
        assert!((p[1].tangent + std::f64::consts::FRAC_PI_2).abs() < 1e-12);
        assert!(m.placements(&pts, true, f64::NAN, Unit::M, &env).is_err());
        assert!(
            m.placements(
                &[Vec2::new(0., 0.), Vec2::new(1e12, 0.)],
                false,
                0.,
                Unit::M,
                &env
            )
            .is_err()
        );
    }
    #[test]
    fn relative_absolute_and_upright_rotate_about_half_cap_without_scaling_offsets_by_s() {
        let aspects = HashMap::new();
        let env = env(&aspects);
        let style = StrokeStyle {
            color: "ink".into(),
            opacity: 0.5,
            width: 0.1,
            unit: PrimUnit::World,
            dash: None,
            dash_offset: 0.,
            line_type_loss: None,
            cap: "butt".into(),
            join: "miter".into(),
            blur: 0.,
            level: 0.,
            ground: false,
        };
        let path = [Vec2::new(10., 0.), Vec2::new(0., 0.)];
        // Anchor at (7,0), source offset rotated with tangent: (6.5,.25).
        for (absolute, upright, first) in [
            (false, false, [6.5, 0.25]),
            (true, false, [6.5, 0.25]),
            (false, true, [6.5, -1.75]),
        ] {
            let m = marks(0., absolute, upright);
            let p = m.placements(&path, false, 0., Unit::M, &env).unwrap();
            let mut out = PrimitiveList::default();
            p[0].paint(&style, Unit::M, &env, &mut out);
            let all: serde_json::Value = serde_json::from_str(&out.json()).unwrap();
            let value = &all["fills"][0];
            let xy = &value["rings"][0][0];
            for (i, e) in first.into_iter().enumerate() {
                assert!(
                    (xy[["x", "y"][i]].as_f64().unwrap() - e).abs() < 1e-12,
                    "{value}"
                );
            }
            assert_eq!(value["paint"]["opacity"], 0.5);
        }
    }
    #[test]
    fn line_marks_follow_stroke_visibility_and_keep_the_fill_filter_after_drawing() {
        use super::super::batch::BatchSink;
        let paint = FillPaint::Solid {
            color: "ink".into(),
            opacity: 1.,
            level: 0.,
        };
        let rings = vec![vec![
            Vec2::new(0., 0.),
            Vec2::new(2., 0.),
            Vec2::new(0., 2.),
        ]];
        for (fills, strokes, has_ink) in [
            (true, false, true),
            (false, true, false),
            (true, true, false),
        ] {
            let mut sink = BatchSink::new(Vec2::new(0., 0.));
            sink.hide(fills, strokes);
            sink.cad_mark(&paint, &rings);
            if fills {
                sink.fill(&paint, &rings);
            }
            let made = sink.finish();
            assert_eq!(!made.data.is_empty(), has_ink);
            if has_ink {
                assert_eq!(made.data.len(), 6, "ordinary area stayed hidden");
            }
        }
    }
    #[test]
    fn frozen_mark_mesh_preserves_counters_visibility_tiles_and_contour_fallback() {
        use super::super::batch::BatchSink;
        let raw = serde_json::json!({"period":4.,"marks":[{"at":0.,"cap":1.,"height":1.,"x":0.,"y":0.,
            "rotation":0.,"polygons":[[[[0.,0.],[4.,0.],[4.,4.],[0.,4.]],[[1.,1.],[1.,3.],[3.,3.],[3.,1.]]]]}]});
        let m = Marks::read(&Json::parse(&raw.to_string()).unwrap())
            .unwrap()
            .unwrap();
        let polygon = &m.marks[0].polygons[0];
        assert_eq!(polygon.indices.len(), 24);
        let paint = FillPaint::Solid {
            color: "ink".into(),
            opacity: 0.5,
            level: 0.,
        };
        for origin in [Vec2::new(0., 0.), Vec2::new(1_000_000., 4_500_000.)] {
            let rings: Vec<Vec<_>> = polygon
                .rings
                .iter()
                .map(|r| {
                    r.iter()
                        .map(|p| Vec2::new(p.x + origin.x, p.y + origin.y))
                        .collect()
                })
                .collect();
            let mut contours = PrimitiveList::default();
            contours.cad_mark_mesh(&paint, &rings, &polygon.indices);
            let original = contours.json();
            let mut direct = PrimitiveList::default();
            direct.cad_mark(&paint, &rings);
            assert_eq!(
                original,
                direct.json(),
                "vector/PDF contour path stays unchanged"
            );
            for (hide_fills, hide_strokes) in
                [(false, false), (true, false), (false, true), (true, true)]
            {
                for valid in [true, false] {
                    let mut sink = BatchSink::new(origin);
                    sink.hide(hide_fills, hide_strokes);
                    sink.cad_mark_mesh(
                        &paint,
                        &rings,
                        if valid {
                            &polygon.indices
                        } else {
                            &[u32::MAX, 0, 1]
                        },
                    );
                    // An ordinary fill still obeys its filter after the CAD mesh.
                    sink.fill(&paint, &rings);
                    let packed = sink.finish();
                    let repeats =
                        if hide_strokes { 0. } else { 1. } + if hide_fills { 0. } else { 1. };
                    let mut area = 0.;
                    for triangle in packed.data.chunks_exact(6) {
                        let t: Vec<_> = triangle.iter().map(|v| f64::from(*v)).collect();
                        area += ((t[2] - t[0]) * (t[5] - t[1]) - (t[3] - t[1]) * (t[4] - t[0]))
                            .abs()
                            * 0.5;
                        let x = (t[0] + t[2] + t[4]) / 3.;
                        let y = (t[1] + t[3] + t[5]) / 3.;
                        assert!(
                            !(x > 1. + 1e-6 && x < 3. - 1e-6 && y > 1. + 1e-6 && y < 3. - 1e-6),
                            "counter filled"
                        );
                    }
                    assert!(
                        (area - 12. * repeats).abs() < 1e-6,
                        "{origin:?} valid={valid}: {area} != {}",
                        12. * repeats
                    );
                }
            }
        }
        let invalid = serde_json::json!({"period":4.,"marks":[{"at":0.,"cap":1.,"height":1.,"x":0.,"y":0.,"rotation":0.,
            "polygons":[[[[0.,0.],[1.,0.],[2.,0.]]]]}]});
        assert!(
            Marks::read(&Json::parse(&invalid.to_string()).unwrap())
                .unwrap()
                .is_err()
        );
    }

    #[test]
    fn aggregate_glyph_workload_is_reserved_atomically_across_paths() {
        use super::super::batch::BatchSink;
        let mut sink = BatchSink::new(Vec2::new(0., 0.));
        assert!(sink.reserve_cad_points(1_900_000));
        assert!(!sink.reserve_cad_points(200_000));
        assert!(
            sink.reserve_cad_points(100_000),
            "a refused path consumed no budget"
        );
        assert!(!sink.reserve_cad_points(1));
        assert!(sink.reserve_cad_points(0));
    }
}
