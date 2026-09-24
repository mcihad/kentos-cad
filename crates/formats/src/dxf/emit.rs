//! DXF entities to the app's objects. Each entity is mapped through the
//! transform it lives under (its object coordinate system, then every block
//! insertion above it) and becomes the most faithful kind the model has:
//! a circle stays a circle under rotation and uniform scale and becomes an
//! ellipse under a non-uniform one; bulges stay bulges unless the map
//! stretches them (then the arcs are sampled, and that is reported).
//! Identity maps copy coordinates bit for bit.

use std::collections::{BTreeMap, HashMap};

use kentos_contracts::{
    ArcEntity, Bounds, CircleEntity, ConstructionEntity, EllipseEntity, Entity, EntityBase, HatchEntity, HatchPattern, HatchPatternType, LineEntity, PathEntity, PointEntity, SplineEntity,
    TextEntity, Vec2,
};

use super::aci;
use super::entity::{Color, Kind, Parsed, Vertex, P3};
use super::hatch::{Edge, Hatch, Path};
use super::strings::{mtext_lines, text_codes};
use crate::geom::{arc_points, bulge_arc, bulge_ring_points, dist, ellipse_from, finite, inside, ocs_tf, signed_area2, v, Similarity, Tf};
use crate::math::{atan2, cos, deg, hypot, norm_angle, rad, sin, sin_cos_deg, TAU};
use crate::nurbs;
use crate::report::Report;

/// Blocks nest at most this deep (a block that inserts itself is caught earlier).
const MAX_DEPTH: usize = 24;
/// Curves sampled into points stay within this of the true curve (1 mm).
const SAMPLE_TOLERANCE: f64 = 1e-3;
/// Text width per character in ems, as the app estimates it (entities.ts textBox).
const EM_PER_CHAR: f64 = 0.55;

#[derive(Clone, Debug)]
pub struct Block {
    pub base: P3,
    pub entities: Vec<Parsed>,
    pub xref: bool,
}

/// What the tables and blocks sections defined (read-only while entities are converted).
#[derive(Default)]
pub struct Library {
    pub blocks: HashMap<String, Block>,
    /// Upper-case layer name → the layer's colour for BYBLOCK children of inserts on it.
    pub layer_colors: HashMap<String, String>,
    /// Upper-case text style name → fixed height (0: none).
    pub style_heights: HashMap<String, f64>,
}

/// Where an entity lives: the transform down to world XY, the Z map of
/// point elevations, and what layer-0 and BYBLOCK children of an insert take.
#[derive(Clone)]
pub struct Ctx {
    pub tf: Tf,
    pub z_scale: f64,
    pub z_offset: f64,
    pub layer: Option<String>,
    pub byblock: String,
    /// Upper-case names of the blocks being inserted (cycle guard).
    pub chain: Vec<String>,
    /// Inside a dimension's block: its definition points are not drawing content.
    pub in_dimension: bool,
}

impl Ctx {
    pub fn model() -> Ctx {
        Ctx { tf: Tf::IDENTITY, z_scale: 1.0, z_offset: 0.0, layer: None, byblock: "ink".to_string(), chain: Vec::new(), in_dimension: false }
    }
}

pub struct Out {
    pub entities: Vec<Entity>,
    pub report: Report,
    pub per_layer: HashMap<String, u32>,
    pub bounds: Option<Bounds>,
    pub limit: usize,
    pub truncated: u32,
}

impl Out {
    pub fn new(limit: usize) -> Out {
        Out { entities: Vec::new(), report: Report::default(), per_layer: HashMap::new(), bounds: None, limit, truncated: 0 }
    }

    fn extend_bounds(&mut self, p: Vec2) {
        if !finite(p) {
            return;
        }
        let b = self.bounds.get_or_insert(Bounds { min_x: p.x, min_y: p.y, max_x: p.x, max_y: p.y });
        b.min_x = b.min_x.min(p.x);
        b.min_y = b.min_y.min(p.y);
        b.max_x = b.max_x.max(p.x);
        b.max_y = b.max_y.max(p.y);
    }
}

fn base(layer: &str, color: Option<String>) -> EntityBase {
    EntityBase { id: 0, layer_id: layer.to_string(), color, attrs: BTreeMap::new(), label: None, symbol: None }
}

fn xy(p: P3) -> Vec2 {
    v(p[0], p[1])
}

/// The world transform of an entity in object coordinates at `elevation`.
fn ocs(ctx: &Ctx, extrusion: P3, elevation: f64) -> Option<Tf> {
    Some(ocs_tf(extrusion, elevation)?.then(&ctx.tf))
}

fn kind_name(e: &Entity) -> &'static str {
    match e {
        Entity::Point(_) => "point",
        Entity::Line(_) => "line",
        Entity::Polyline(_) => "polyline",
        Entity::Polygon(_) => "polygon",
        Entity::Circle(_) => "circle",
        Entity::Arc(_) => "arc",
        Entity::Ellipse(_) => "ellipse",
        Entity::Spline(_) => "spline",
        Entity::Xline(_) => "xline",
        Entity::Ray(_) => "ray",
        Entity::Text(_) => "text",
        Entity::Dimension(_) => "dimension",
        Entity::Hatch(_) => "hatch",
    }
}

/// Points that tell where an object lies (for the extent shown before the import).
fn anchor_points(e: &Entity) -> Vec<Vec2> {
    match e {
        Entity::Point(p) => vec![p.p],
        Entity::Line(l) => vec![l.a, l.b],
        Entity::Polyline(p) | Entity::Polygon(p) => p.pts.clone(),
        Entity::Circle(c) => vec![v(c.c.x - c.r, c.c.y - c.r), v(c.c.x + c.r, c.c.y + c.r)],
        Entity::Arc(a) => vec![v(a.c.x - a.r, a.c.y - a.r), v(a.c.x + a.r, a.c.y + a.r)],
        Entity::Ellipse(e) => {
            let r = hypot(e.major.x, e.major.y);
            vec![v(e.c.x - r, e.c.y - r), v(e.c.x + r, e.c.y + r)]
        }
        Entity::Spline(s) => s.pts.clone(),
        Entity::Xline(x) | Entity::Ray(x) => vec![x.p],
        Entity::Text(t) => vec![t.p],
        Entity::Dimension(d) => vec![d.a, d.b],
        Entity::Hatch(h) => h.ring.clone(),
    }
}

pub struct Emitter<'l> {
    pub lib: &'l Library,
    pub out: Out,
}

impl<'l> Emitter<'l> {
    fn push(&mut self, e: Entity) {
        if self.out.entities.len() >= self.out.limit {
            self.out.truncated += 1;
            return;
        }
        let layer = match &e {
            Entity::Point(x) => &x.base.layer_id,
            Entity::Line(x) => &x.base.layer_id,
            Entity::Polyline(x) | Entity::Polygon(x) => &x.base.layer_id,
            Entity::Circle(x) => &x.base.layer_id,
            Entity::Arc(x) => &x.base.layer_id,
            Entity::Ellipse(x) => &x.base.layer_id,
            Entity::Spline(x) => &x.base.layer_id,
            Entity::Xline(x) | Entity::Ray(x) => &x.base.layer_id,
            Entity::Text(x) => &x.base.layer_id,
            Entity::Dimension(x) => &x.base.layer_id,
            Entity::Hatch(x) => &x.base.layer_id,
        };
        *self.out.per_layer.entry(layer.clone()).or_insert(0) += 1;
        self.out.report.count(kind_name(&e));
        for p in anchor_points(&e) {
            self.out.extend_bounds(p);
        }
        self.out.entities.push(e);
    }

    fn skip(&mut self, what: &str, reason: &str, line: u32) {
        self.out.report.skip(what, reason, line);
    }

    fn note(&mut self, what: &str, reason: &str, line: u32) {
        self.out.report.note(what, reason, line);
    }

    /// The layer an object goes on: children on layer 0 take the insert's layer.
    fn layer_of(&self, e: &Parsed, ctx: &Ctx) -> String {
        match (&ctx.layer, e.common.layer.as_str()) {
            (Some(l), "0") => l.clone(),
            _ => e.common.layer.clone(),
        }
    }

    /// The colour override: none for BYLAYER, the insert's colour for BYBLOCK.
    fn color_of(&self, e: &Parsed, ctx: &Ctx) -> Option<String> {
        match e.common.color {
            Color::ByLayer => None,
            Color::ByBlock => Some(ctx.byblock.clone()),
            Color::Aci(n) => Some(aci::color(n)),
            Color::True(rgb) => Some(aci::true_color(rgb)),
        }
    }

    pub fn emit(&mut self, e: &Parsed, ctx: &Ctx) {
        if e.common.paper {
            self.skip("Kâğıt uzayı nesnesi", "pafta düzenindeki (kâğıt uzayı) nesneler alınmaz; yalnız model uzayı alınır", e.line);
            return;
        }
        if e.common.invisible {
            self.skip(&e.name, "görünmez olarak işaretli", e.line);
            return;
        }
        let layer = self.layer_of(e, ctx);
        let color = self.color_of(e, ctx);
        let b = || base(&layer, color.clone());
        let ext = e.common.extrusion;
        match &e.kind {
            Kind::Line { a, b: bb } => {
                let (a, bb) = (ctx.tf.apply(xy(*a)), ctx.tf.apply(xy(*bb)));
                self.push(Entity::Line(LineEntity { base: b(), a, b: bb }));
            }
            Kind::Point { p } => {
                if ctx.in_dimension {
                    return;
                }
                let z = ctx.z_offset + ctx.z_scale * p[2];
                self.push(Entity::Point(PointEntity { base: b(), p: ctx.tf.apply(xy(*p)), z: (z != 0.0).then_some(z) }));
            }
            Kind::Circle { c, r } => {
                let Some(m) = ocs(ctx, ext, c[2]) else { return self.skip(&e.name, "doğrultusu (210) geçersiz", e.line) };
                if !(*r > 0.0) {
                    return self.skip(&e.name, "yarıçapı sıfır ya da negatif", e.line);
                }
                self.circle_or_arc(m, xy(*c), *r, None, b(), e);
            }
            Kind::Arc { c, r, a0, a1 } => {
                let Some(m) = ocs(ctx, ext, c[2]) else { return self.skip(&e.name, "doğrultusu (210) geçersiz", e.line) };
                if !(*r > 0.0) {
                    return self.skip(&e.name, "yarıçapı sıfır ya da negatif", e.line);
                }
                let span = (a1 - a0).rem_euclid(360.0);
                self.circle_or_arc(m, xy(*c), *r, (span != 0.0).then_some((*a0, *a1)), b(), e);
            }
            Kind::Ellipse { c, major, ratio, t0, t1 } => self.ellipse(ctx, ext, *c, *major, *ratio, *t0, *t1, b(), e),
            Kind::LwPolyline { pts, bulges, closed, elevation, width } => {
                let Some(m) = ocs(ctx, ext, *elevation) else { return self.skip(&e.name, "doğrultusu (210) geçersiz", e.line) };
                if *width {
                    self.note("Genişlikli çoklu çizgi", "genişlik (kalınlık) alınmadı; çizgi ekseniyle geldi", e.line);
                }
                let pts: Vec<Vec2> = pts.iter().map(|p| v(p[0], p[1])).collect();
                self.path(m, pts, bulges.clone(), *closed, b(), e);
            }
            Kind::Polyline { flags, verts, elevation, width } => self.polyline(ctx, ext, *flags, verts, *elevation, *width, b(), e),
            Kind::Spline { flags, degree, knots, weights, ctrl, fit } => self.spline(ctx, *flags, *degree, knots, weights, ctrl, fit, b(), e),
            Kind::Text { p, p2, height, rotation, text, halign, valign, width, style, attrib, hidden } => {
                if *hidden {
                    return self.skip("Görünmez öznitelik (ATTRIB)", "blok özniteliği görünmez olarak işaretli", e.line);
                }
                let _ = attrib;
                self.text(ctx, ext, *p, *p2, *height, *rotation, text, *halign, *valign, *width, style, b(), e);
            }
            Kind::MText { p, height, attach, xdir, rotation, text, spacing, style } => self.mtext(ctx, ext, *p, *height, *attach, *xdir, *rotation, text, *spacing, style, b(), e),
            Kind::Face { pts, solid } => {
                // SOLID and TRACE are in object coordinates and run 1 2 4 3; 3DFACE is in world coordinates.
                let m = if *solid { ocs(ctx, ext, pts[0][2]) } else { Some(ctx.tf) };
                let Some(m) = m else { return self.skip(&e.name, "doğrultusu (210) geçersiz", e.line) };
                let order: [usize; 4] = if *solid { [0, 1, 3, 2] } else { [0, 1, 2, 3] };
                let mut ring: Vec<Vec2> = order.iter().map(|&i| m.apply(xy(pts[i]))).collect();
                ring.dedup();
                if ring.len() > 1 && ring.first() == ring.last() {
                    ring.pop();
                }
                if ring.len() < 3 {
                    return self.skip(&e.name, "alanı olmayan (köşeleri çakışık) yüzey", e.line);
                }
                self.push(Entity::Polygon(PathEntity { base: b(), pts: ring, bulges: None, holes: None }));
                if *solid {
                    self.note("Dolu alan (SOLID, TRACE)", "kapalı alan olarak alındı; dolgusu katman stilinden gelir", e.line);
                } else {
                    self.note("3B yüz (3DFACE)", "kapalı alan olarak alındı; Z değerleri alınmadı", e.line);
                }
            }
            Kind::Insert { name, p, scale, rotation, cols, rows, dc, dr, attribs } => {
                self.insert(ctx, e, &layer, name, *p, *scale, *rotation, *cols, *rows, *dc, *dr);
                // Attributes are already placed in the insert's own frame.
                for a in attribs {
                    self.emit(a, ctx);
                }
            }
            Kind::Hatch(h) => self.hatch(ctx, ext, h, b(), e),
            Kind::Block { block, what } => self.anonymous_block(ctx, e, &layer, block, what),
            Kind::Xline { p, dir, ray } => {
                let d = ctx.tf.linear(xy(*dir));
                let l = hypot(d.x, d.y);
                if !(l > 0.0) {
                    return self.skip(&e.name, "doğrultusu yok (dünya düzlemine dik)", e.line);
                }
                let c = ConstructionEntity { base: b(), p: ctx.tf.apply(xy(*p)), dir: v(d.x / l, d.y / l) };
                self.push(if *ray { Entity::Ray(c) } else { Entity::Xline(c) });
            }
            Kind::Leader { pts } => {
                let pts: Vec<Vec2> = pts.iter().map(|p| ctx.tf.apply(xy(*p))).collect();
                if pts.len() < 2 {
                    return self.skip("Kılavuz (LEADER)", "iki noktası yok", e.line);
                }
                self.push(Entity::Polyline(PathEntity { base: b(), pts, bulges: None, holes: None }));
                self.note("Kılavuz (LEADER)", "çoklu çizgi olarak alındı; ok başı ve bağlı yazı ayrı", e.line);
            }
            Kind::Unsupported(name) => {
                let reason = match name.as_str() {
                    "IMAGE" | "WIPEOUT" | "OLE2FRAME" | "OLEFRAME" | "PDFUNDERLAY" | "DWFUNDERLAY" | "DGNUNDERLAY" => "raster görüntü ve gömülü/altlık nesneleri alınmaz",
                    "3DSOLID" | "BODY" | "REGION" | "SURFACE" | "PLANESURFACE" | "EXTRUDEDSURFACE" | "LOFTEDSURFACE" | "REVOLVEDSURFACE" | "SWEPTSURFACE" | "MESH" | "POLYFACE" => {
                        "3B katı, yüzey ve bölge nesneleri alınmaz"
                    }
                    "MLINE" => "çoklu çizgi (MLINE) alınmaz; AutoCAD'de patlatıp (EXPLODE) yeniden kaydedin",
                    "MULTILEADER" | "MLEADER" => "çok kılavuzlu açıklama (MLEADER) alınmaz; AutoCAD'de patlatıp yeniden kaydedin",
                    "VIEWPORT" => "görünüm pencereleri pafta düzenine aittir",
                    "SHAPE" => "şekil (SHAPE) yazı tipi dosyası gerektirir; alınmaz",
                    "TOLERANCE" => "geometrik tolerans çerçevesi alınmaz",
                    "ATTDEF" => "blok öznitelik tanımı çizim nesnesi değildir",
                    _ => "bu nesne türü tanınmıyor",
                };
                if name != "ATTDEF" {
                    self.skip(name, reason, e.line);
                }
            }
        }
    }

    /// A circle (or an arc from `angles`, degrees counter-clockwise) under a map: a circle or arc while the map keeps shapes, else an ellipse.
    fn circle_or_arc(&mut self, m: Tf, c: Vec2, r: f64, angles: Option<(f64, f64)>, b: EntityBase, e: &Parsed) {
        let center = m.apply(c);
        match m.similarity() {
            Some(Similarity { scale, angle, mirror }) => {
                let r = r * scale;
                match angles {
                    None => self.push(Entity::Circle(CircleEntity { base: b, c: center, r })),
                    Some((a0, a1)) => {
                        let (a0, a1) = (rad(a0), rad(a1));
                        let (s, t) = if mirror { (angle - a1, angle - a0) } else { (a0 + angle, a1 + angle) };
                        self.push(Entity::Arc(ArcEntity { base: b, c: center, r, a0: norm_angle(s), a1: norm_angle(t) }));
                    }
                }
            }
            None => {
                let u = m.linear(v(r, 0.0));
                let w = m.linear(v(0.0, r));
                let (t0, t1) = match angles {
                    None => (0.0, 0.0),
                    Some((a0, a1)) => {
                        let (a0, a1) = (rad(a0), rad(a1));
                        (a0, if a1 > a0 { a1 } else { a1 + TAU })
                    }
                };
                match ellipse_from(center, u, w, t0, t1, angles.is_none()) {
                    Some(el) => {
                        self.push(Entity::Ellipse(EllipseEntity { base: b, c: el.c, major: el.major, ratio: el.ratio, t0: el.t0, t1: el.t1 }));
                        self.note(&e.name, "eşit olmayan ölçekle eklendiği için elips oldu", e.line);
                    }
                    None => self.skip(&e.name, "görüş doğrultusuna dik (çizgi gibi görünen) çember", e.line),
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn ellipse(&mut self, ctx: &Ctx, ext: P3, c: P3, major: P3, ratio: f64, t0: f64, t1: f64, b: EntityBase, e: &Parsed) {
        if !(ratio > 0.0 && ratio <= 1.0) {
            return self.skip(&e.name, "eksen oranı 0 ile 1 arasında değil", e.line);
        }
        let full = (t1 - t0).abs() >= TAU - 1e-9 || t1 == t0;
        if ctx.tf.is_identity() && ext == [0.0, 0.0, 1.0] && major[2] == 0.0 {
            // The model keeps DXF's own form: copy it.
            let (t0, t1) = if full { (0.0, 0.0) } else { (norm_angle(t0), norm_angle(t1)) };
            if !(hypot(major[0], major[1]) > 0.0) {
                return self.skip(&e.name, "büyük ekseni sıfır", e.line);
            }
            return self.push(Entity::Ellipse(EllipseEntity { base: b, c: xy(c), major: xy(major), ratio, t0, t1 }));
        }
        // The minor axis is the normal × major (both in world coordinates); only their plan views count.
        let l = (ext[0] * ext[0] + ext[1] * ext[1] + ext[2] * ext[2]).sqrt();
        if !(l > 0.0) {
            return self.skip(&e.name, "doğrultusu (210) geçersiz", e.line);
        }
        let n = [ext[0] / l, ext[1] / l, ext[2] / l];
        let minor = [(n[1] * major[2] - n[2] * major[1]) * ratio, (n[2] * major[0] - n[0] * major[2]) * ratio];
        let u = ctx.tf.linear(v(major[0], major[1]));
        let w = ctx.tf.linear(v(minor[0], minor[1]));
        let (t0, t1) = if t1 > t0 { (t0, t1) } else { (t0, t1 + TAU) };
        match ellipse_from(ctx.tf.apply(xy(c)), u, w, t0, t1, full) {
            Some(el) => self.push(Entity::Ellipse(EllipseEntity { base: b, c: el.c, major: el.major, ratio: el.ratio, t0: el.t0, t1: el.t1 })),
            None => self.skip(&e.name, "görüş doğrultusuna dik (çizgi gibi görünen) elips", e.line),
        }
    }

    /// A vertex path (LWPOLYLINE, 2D POLYLINE) in object coordinates: bulges kept while the map keeps shapes.
    fn path(&mut self, m: Tf, pts: Vec<Vec2>, mut bulges: Vec<f64>, closed: bool, b: EntityBase, e: &Parsed) {
        let mut pts = pts;
        bulges.resize(pts.len(), 0.0);
        // A closed path that repeats its first vertex: the repeat is not a corner.
        if closed && pts.len() > 1 && pts.first() == pts.last() {
            pts.pop();
            bulges.pop();
        }
        if pts.len() < 2 {
            return self.skip(&e.name, "iki köşesi yok", e.line);
        }
        let arcs = bulges.iter().any(|&x| x != 0.0);
        let (pts, bulges) = match (arcs, m.similarity()) {
            (_, Some(s)) => {
                let pts: Vec<Vec2> = pts.iter().map(|p| m.apply(*p)).collect();
                let bulges: Vec<f64> = if s.mirror { bulges.iter().map(|x| -x).collect() } else { bulges };
                (pts, bulges)
            }
            (false, None) => (pts.iter().map(|p| m.apply(*p)).collect(), bulges),
            (true, None) => {
                // A stretched arc is an elliptic arc; the model's paths hold circular arcs only.
                let ring = if closed { bulge_ring_points(&pts, &bulges) } else { open_path_points(&pts, &bulges) };
                self.note(&e.name, "yaylı kenarları eşit olmayan ölçekle eklendiği için noktalara bölündü", e.line);
                let n = ring.len();
                (ring.iter().map(|p| m.apply(*p)).collect(), vec![0.0; n])
            }
        };
        let open = !closed;
        let mut bulges = bulges;
        if open {
            bulges.truncate(pts.len().saturating_sub(1));
        }
        let bulges = bulges.iter().any(|&x| x != 0.0).then_some(bulges);
        if closed && (pts.len() >= 3 || bulges.is_some()) {
            self.push(Entity::Polygon(PathEntity { base: b, pts, bulges, holes: None }));
        } else {
            self.push(Entity::Polyline(PathEntity { base: b, pts, bulges, holes: None }));
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn polyline(&mut self, ctx: &Ctx, ext: P3, flags: i64, verts: &[Vertex], elevation: f64, width: bool, b: EntityBase, e: &Parsed) {
        if flags & (16 | 64) != 0 {
            return self.skip("3B ağ (POLYLINE)", "çokyüzlü ve ızgara ağları alınmaz", e.line);
        }
        // Spline frame control points are not on the curve.
        let verts: Vec<&Vertex> = verts.iter().filter(|x| x.flags & 16 == 0).collect();
        let closed = flags & 1 == 1;
        if width {
            self.note("Genişlikli çoklu çizgi", "genişlik (kalınlık) alınmadı; çizgi ekseniyle geldi", e.line);
        }
        if flags & 8 != 0 {
            // 3D polyline: world coordinates, straight segments.
            let pts: Vec<Vec2> = verts.iter().map(|x| xy(x.p)).collect();
            return self.path(ctx.tf, pts, Vec::new(), closed, b, e);
        }
        let Some(m) = ocs(ctx, ext, elevation) else { return self.skip(&e.name, "doğrultusu (210) geçersiz", e.line) };
        let pts: Vec<Vec2> = verts.iter().map(|x| xy(x.p)).collect();
        let bulges: Vec<f64> = verts.iter().map(|x| x.bulge).collect();
        self.path(m, pts, bulges, closed, b, e);
    }

    #[allow(clippy::too_many_arguments)]
    fn spline(&mut self, ctx: &Ctx, flags: i64, degree: i64, knots: &[f64], weights: &[f64], ctrl: &[P3], fit: &[P3], b: EntityBase, e: &Parsed) {
        let closed = flags & 1 == 1;
        let ours = e.common.xdata.iter().any(|(_, s)| s == super::XDATA_SPLINE);
        if fit.len() >= 2 {
            let mut pts: Vec<Vec2> = fit.iter().map(|p| ctx.tf.apply(xy(*p))).collect();
            if closed && pts.len() > 2 && pts.first() == pts.last() {
                pts.pop();
            }
            if !ours {
                self.note("Eğri (SPLINE)", "geçiş noktalarından KentOS eğrisi (Catmull-Rom) olarak kuruldu; noktalar arasındaki biçim küçük farklar gösterebilir", e.line);
            }
            if ctx.tf.similarity().is_none() {
                self.note("Eğri (SPLINE)", "eşit olmayan ölçekle eklendi; geçiş noktaları dönüştürüldü, eğri yaklaşık", e.line);
            }
            return self.push(Entity::Spline(SplineEntity { base: b, pts, closed }));
        }
        let Ok(p) = usize::try_from(degree) else { return self.skip("Eğri (SPLINE)", "derecesi geçersiz", e.line) };
        if p == 0 || ctrl.len() < 2 || knots.len() != ctrl.len() + p + 1 {
            return self.skip("Eğri (SPLINE)", "denetim noktası ve düğüm sayıları tutarsız", e.line);
        }
        // B-splines keep their shape under affine maps: transform the control points, then sample.
        let cps: Vec<Vec2> = ctrl.iter().map(|q| ctx.tf.apply(xy(*q))).collect();
        let w = (flags & 4 != 0 && weights.len() == cps.len()).then_some(weights);
        let pts = if p == 1 && w.is_none() { cps } else {
            match nurbs::sample(p, knots, &cps, w, SAMPLE_TOLERANCE) {
                Some(pts) => {
                    self.note("Denetim noktalı eğri (SPLINE)", "1 mm içinde çoklu çizgiye çevrildi", e.line);
                    pts
                }
                None => return self.skip("Eğri (SPLINE)", "eğri hesaplanamadı (düğümler geçersiz)", e.line),
            }
        };
        self.path(Tf::IDENTITY, pts, Vec::new(), closed, b, e);
    }

    #[allow(clippy::too_many_arguments)]
    fn text(&mut self, ctx: &Ctx, ext: P3, p: P3, p2: Option<P3>, height: f64, rotation: f64, text: &str, halign: i64, valign: i64, width: f64, style: &str, b: EntityBase, e: &Parsed) {
        let text = text_codes(text);
        if text.trim().is_empty() {
            return self.skip(&e.name, "boş yazı", e.line);
        }
        let height = if height > 0.0 { height } else { self.lib.style_heights.get(&style.to_uppercase()).copied().unwrap_or(0.0) };
        if !(height > 0.0) {
            return self.skip(&e.name, "yazı yüksekliği yok", e.line);
        }
        let Some(m) = ocs(ctx, ext, p[2]) else { return self.skip(&e.name, "doğrultusu (210) geçersiz", e.line) };
        let (s, c) = sin_cos_deg(rotation);
        let (mut anchor, mut rot) = (v(p[0], p[1]), rotation);
        // Justified text: 11 is the anchor (AutoCAD puts the start point in 10 too, other writers may not).
        if (halign != 0 || valign != 0)
            && let Some(q) = p2
        {
            {
                let q = v(q[0], q[1]);
                let w = text.chars().count() as f64 * EM_PER_CHAR * height * width.abs().max(0.01);
                if halign == 3 || halign == 5 {
                    // Aligned and fit: the text runs from 10 to 11.
                    rot = deg(atan2(q.y - p[1], q.x - p[0]));
                } else if anchor == q || (p[0] == 0.0 && p[1] == 0.0) {
                    let along = match halign {
                        1 | 4 => w / 2.0,
                        2 => w,
                        _ => 0.0,
                    };
                    let up = match (halign, valign) {
                        (4, _) | (_, 2) => height / 2.0,
                        (_, 3) => height,
                        (_, 1) => -0.2 * height,
                        _ => 0.0,
                    };
                    anchor = v(q.x - c * along + s * up, q.y - s * along - c * up);
                    self.note("Hizalı yazı", "KentOS yazıları sol alt köşeden yerleşir; konum yazı genişliği tahmin edilerek hesaplandı", e.line);
                }
            }
        }
        self.push_text(m, anchor, rot, height, text, b);
    }

    /// A text at `anchor` whose baseline runs at `rotation` degrees (object coordinates).
    fn push_text(&mut self, m: Tf, anchor: Vec2, rotation: f64, height: f64, text: String, b: EntityBase) {
        let (s, c) = sin_cos_deg(rotation);
        let d = m.linear(v(c, s));
        let u = m.linear(v(-s, c));
        let ld = hypot(d.x, d.y);
        if !(ld > 0.0) {
            return;
        }
        // The file's own angle when nothing turns it (bit for bit), else the mapped baseline's.
        let mut rot = if m.is_identity() { rotation } else { deg(atan2(d.y, d.x)) };
        // Mirrored text stays readable, as the app's own mirror does (MIRRTEXT 0).
        if m.det() < 0.0 {
            rot += 180.0;
        }
        let rot = if (0.0..360.0).contains(&rot) { rot } else { rot.rem_euclid(360.0) };
        let h = if m.is_identity() { height } else { height * (d.x * u.y - d.y * u.x).abs() / ld };
        self.push(Entity::Text(TextEntity { base: b, p: m.apply(anchor), text, height: h, rotation: if rot >= 360.0 { 0.0 } else { rot } }));
    }

    #[allow(clippy::too_many_arguments)]
    fn mtext(&mut self, ctx: &Ctx, ext: P3, p: P3, height: f64, attach: i64, xdir: Option<P3>, rotation: Option<f64>, text: &str, spacing: f64, style: &str, b: EntityBase, e: &Parsed) {
        let lines = mtext_lines(text);
        if lines.iter().all(|l| l.trim().is_empty()) {
            return self.skip("Çok satırlı yazı (MTEXT)", "boş yazı", e.line);
        }
        let height = if height > 0.0 { height } else { self.lib.style_heights.get(&style.to_uppercase()).copied().unwrap_or(0.0) };
        if !(height > 0.0) {
            return self.skip("Çok satırlı yazı (MTEXT)", "yazı yüksekliği yok", e.line);
        }
        // Direction: the x axis vector (world), else the rotation (radians) in the object plane.
        let dir = match (xdir, rotation) {
            (Some(x), _) if hypot(x[0], x[1]) > 0.0 => {
                let l = hypot(x[0], x[1]);
                v(x[0] / l, x[1] / l)
            }
            (_, Some(r)) => {
                let Some(o) = ocs_tf(ext, 0.0) else { return self.skip("Çok satırlı yazı (MTEXT)", "doğrultusu (210) geçersiz", e.line) };
                let d = o.linear(v(cos(r), sin(r)));
                let l = hypot(d.x, d.y).max(f64::MIN_POSITIVE);
                v(d.x / l, d.y / l)
            }
            _ => v(1.0, 0.0),
        };
        let up = v(-dir.y, dir.x);
        let angle = deg(atan2(dir.y, dir.x));
        let gap = height * 5.0 / 3.0 * if spacing > 0.0 { spacing } else { 1.0 };
        let n = lines.len() as f64;
        let block = height + (n - 1.0) * gap;
        let row = (attach.clamp(1, 9) - 1) / 3;
        let col = (attach.clamp(1, 9) - 1) % 3;
        let top = match row {
            0 => 0.0,
            1 => block / 2.0,
            _ => block,
        };
        let mut kept = 0;
        for (i, line) in lines.iter().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let w = line.chars().count() as f64 * EM_PER_CHAR * height;
            let along = match col {
                1 => -w / 2.0,
                2 => -w,
                _ => 0.0,
            };
            let down = top - height - i as f64 * gap;
            let at = v(p[0] + dir.x * along + up.x * down, p[1] + dir.y * along + up.y * down);
            self.push_text(ctx.tf, at, angle, height, line.clone(), b.clone());
            kept += 1;
        }
        if kept > 1 {
            self.note("Çok satırlı yazı (MTEXT)", "satırlarına bölündü; biçimlendirme kaldırıldı, konum yazı genişliği tahmin edilerek hesaplandı", e.line);
        } else if text.contains('\\') || text.contains('{') {
            self.note("Çok satırlı yazı (MTEXT)", "biçimlendirme (yazı tipi, renk, boyut) kaldırıldı", e.line);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn insert(&mut self, ctx: &Ctx, e: &Parsed, layer: &str, name: &str, p: P3, scale: P3, rotation: f64, cols: i64, rows: i64, dc: f64, dr: f64) {
        let key = name.to_uppercase();
        let Some(block) = self.lib.blocks.get(&key) else { return self.skip("Blok (INSERT)", &format!("“{name}” blok tanımı dosyada yok"), e.line) };
        if block.xref {
            return self.skip("Dış başvuru (XREF)", "dış başvurulu çizimler alınmaz; AutoCAD'de bağlayıp (BIND) yeniden kaydedin", e.line);
        }
        if ctx.chain.contains(&key) || ctx.chain.len() >= MAX_DEPTH {
            return self.skip("Blok (INSERT)", &format!("“{name}” bloğu kendini içeriyor ya da çok derin iç içe"), e.line);
        }
        let Some(o) = ocs_tf(e.common.extrusion, p[2]) else { return self.skip("Blok (INSERT)", "doğrultusu (210) geçersiz", e.line) };
        let [sx, sy, sz] = scale;
        if sx == 0.0 || sy == 0.0 {
            return self.skip("Blok (INSERT)", "ölçeği sıfır", e.line);
        }
        let byblock = match e.common.color {
            Color::ByLayer => self.lib.layer_colors.get(&layer.to_uppercase()).cloned().unwrap_or_else(|| "ink".to_string()),
            Color::ByBlock => ctx.byblock.clone(),
            Color::Aci(n) => aci::color(n),
            Color::True(rgb) => aci::true_color(rgb),
        };
        let nz = super::extrusion_z(e.common.extrusion);
        let mut chain = ctx.chain.clone();
        chain.push(key);
        // An array insert (MINSERT); a huge one stops at the object limit.
        let cells = (cols.clamp(1, 10_000) as usize).saturating_mul(rows.clamp(1, 10_000) as usize);
        for cell in 0..cells {
            if self.out.entities.len() >= self.out.limit {
                self.out.truncated += 1;
                break;
            }
            let (col, row) = ((cell % cols.max(1) as usize) as f64, (cell / cols.max(1) as usize) as f64);
            let local = Tf::translate(-block.base[0], -block.base[1])
                .then(&Tf::scale(sx, sy))
                .then(&Tf::translate(col * dc, row * dr))
                .then(&Tf::rotate_deg(rotation))
                .then(&Tf::translate(p[0], p[1]))
                .then(&o)
                .then(&ctx.tf);
            let child = Ctx {
                tf: local,
                z_scale: ctx.z_scale * sz * nz,
                z_offset: ctx.z_offset + ctx.z_scale * nz * (p[2] - sz * block.base[2]),
                layer: Some(layer.to_string()),
                byblock: byblock.clone(),
                chain: chain.clone(),
                in_dimension: ctx.in_dimension,
            };
            for child_entity in &block.entities {
                self.emit(child_entity, &child);
            }
        }
        if ctx.chain.is_empty() {
            self.note("Blok (INSERT)", "KentOS'ta blok nesnesi yok: bloklar patlatılarak, içindeki nesneler olarak alındı", e.line);
        }
    }

    fn anonymous_block(&mut self, ctx: &Ctx, e: &Parsed, layer: &str, block: &str, what: &str) {
        let key = block.to_uppercase();
        let Some(b) = self.lib.blocks.get(&key) else { return self.skip(what, "çizim bloğu dosyada yok; ölçü yeniden kurulamadı", e.line) };
        let Some(o) = ocs_tf(e.common.extrusion, 0.0) else { return self.skip(what, "doğrultusu (210) geçersiz", e.line) };
        let byblock = match e.common.color {
            Color::ByLayer => self.lib.layer_colors.get(&layer.to_uppercase()).cloned().unwrap_or_else(|| "ink".to_string()),
            Color::ByBlock => ctx.byblock.clone(),
            Color::Aci(n) => aci::color(n),
            Color::True(rgb) => aci::true_color(rgb),
        };
        let mut chain = ctx.chain.clone();
        chain.push(key);
        let child = Ctx { tf: o.then(&ctx.tf), layer: Some(layer.to_string()), byblock, chain, in_dimension: true, ..ctx.clone() };
        for x in &b.entities {
            self.emit(x, &child);
        }
        self.note(what, "çizgi ve yazılara patlatılarak alındı (KentOS ölçüsü olarak düzenlenemez)", e.line);
    }

    fn hatch(&mut self, ctx: &Ctx, ext: P3, h: &Hatch, b: EntityBase, e: &Parsed) {
        let Some(m) = ocs(ctx, ext, h.elevation) else { return self.skip("Tarama (HATCH)", "doğrultusu (210) geçersiz", e.line) };
        let mut rings: Vec<Vec<Vec2>> = Vec::new();
        let mut curved = false;
        for path in &h.paths {
            let (ring, arcs) = path_points(path);
            curved |= arcs;
            let mut ring: Vec<Vec2> = ring.into_iter().map(|p| m.apply(p)).collect();
            ring.dedup();
            if ring.len() > 1 && ring.first() == ring.last() {
                ring.pop();
            }
            if ring.len() >= 3 && ring.iter().all(|p| finite(*p)) {
                rings.push(ring);
            }
        }
        if rings.is_empty() {
            return self.skip("Tarama (HATCH)", "sınırı okunamadı ya da alanı yok", e.line);
        }
        if curved {
            self.note("Tarama (HATCH)", "sınırdaki yaylar ve eğriler parçalı alındı (72 parça/tur, uygulamanın taramaları gibi)", e.line);
        }
        let pattern = self.pattern(m, h, e);
        // Nesting: even depth is hatched, odd depth is an island of its container.
        let mut order: Vec<usize> = (0..rings.len()).collect();
        let area = |r: &Vec<Vec2>| signed_area2(r).abs();
        order.sort_by(|&a, &b| area(&rings[b]).total_cmp(&area(&rings[a])));
        let mut parent: Vec<Option<usize>> = vec![None; rings.len()];
        let mut depth = vec![0usize; rings.len()];
        for (k, &i) in order.iter().enumerate() {
            let probe = rings[i][0];
            // The smallest larger ring around it is its container.
            if let Some(&j) = order[..k].iter().rev().find(|&&j| inside(probe, &rings[j])) {
                parent[i] = Some(j);
                depth[i] = depth[j] + 1;
            }
        }
        let max_depth = match h.style {
            2 => 0,
            1 => 1,
            _ => usize::MAX,
        };
        for &i in &order {
            if !depth[i].is_multiple_of(2) || depth[i] > max_depth {
                continue;
            }
            let holes: Vec<Vec<Vec2>> = order.iter().filter(|&&j| parent[j] == Some(i) && depth[j] <= max_depth).map(|&j| rings[j].clone()).collect();
            self.push(Entity::Hatch(HatchEntity { base: b.clone(), ring: rings[i].clone(), holes: (!holes.is_empty()).then_some(holes), pattern: pattern.clone() }));
        }
    }

    fn pattern(&mut self, m: Tf, h: &Hatch, e: &Parsed) -> HatchPattern {
        let scale = m.det().abs().sqrt().max(f64::MIN_POSITIVE);
        // An angle (degrees) in object coordinates as the world angle of its direction.
        let world_angle = |a: f64| -> f64 {
            let (s, c) = sin_cos_deg(a);
            let d = m.linear(v(c, s));
            let w = if m.is_identity() { a } else { deg(atan2(d.y, d.x)) };
            w.rem_euclid(180.0)
        };
        if h.solid || h.name.eq_ignore_ascii_case("SOLID") {
            if h.gradient {
                self.note("Tarama (HATCH)", "degrade dolgu düz dolgu olarak alındı", e.line);
            }
            return HatchPattern { kind: HatchPatternType::Solid, angle: 0.0, spacing: 1.0 };
        }
        let spacing_of = |l: &super::hatch::PatternLine| -> f64 {
            let (s, c) = sin_cos_deg(l.angle);
            (l.offset[0] * -s + l.offset[1] * c).abs()
        };
        let lines: Vec<_> = h.lines.iter().filter(|l| spacing_of(l) > 0.0).collect();
        let dashed = lines.iter().any(|l| l.dashes > 0);
        let (kind, angle, spacing) = match lines.as_slice() {
            [one] => (HatchPatternType::Lines, one.angle, spacing_of(one)),
            [a, b] if ((a.angle - b.angle).rem_euclid(180.0) - 90.0).abs() < 1e-6 && (spacing_of(a) - spacing_of(b)).abs() <= 1e-9 * spacing_of(a) => (HatchPatternType::Cross, a.angle, spacing_of(a)),
            [first, ..] => {
                self.note("Tarama (HATCH)", &format!("“{}” deseni ilk çizgi ailesiyle yaklaşık alındı", h.name), e.line);
                (HatchPatternType::Lines, first.angle, spacing_of(first))
            }
            [] => {
                // No line data: the common ANSI patterns by name, at the pattern's angle and scale.
                let s = 3.175 * if h.scale > 0.0 { h.scale } else { 1.0 };
                let (kind, base) = match h.name.to_uppercase().as_str() {
                    "ANSI37" | "NET" | "ANSI38" => (HatchPatternType::Cross, if h.name.eq_ignore_ascii_case("NET") { 0.0 } else { 45.0 }),
                    "LINE" => (HatchPatternType::Lines, 0.0),
                    _ => (HatchPatternType::Lines, 45.0),
                };
                self.note("Tarama (HATCH)", &format!("“{}” deseninin çizgileri dosyada yok; yaklaşık alındı", h.name), e.line);
                (kind, base + h.angle, s)
            }
        };
        if dashed {
            self.note("Tarama (HATCH)", "kesikli desen çizgileri düz çizgi olarak alındı", e.line);
        }
        let spacing = if m.is_identity() { spacing } else { spacing * scale };
        HatchPattern { kind, angle: world_angle(angle), spacing }
    }
}

/// An open bulged path as points (arcs sampled), ending at the last vertex.
fn open_path_points(pts: &[Vec2], bulges: &[f64]) -> Vec<Vec2> {
    let mut out = vec![pts[0]];
    for i in 0..pts.len() - 1 {
        match bulge_arc(pts[i], pts[i + 1], bulges.get(i).copied().unwrap_or(0.0)) {
            Some((c, r, a0, sweep)) => arc_points(c, r, a0, sweep, Some(pts[i + 1]), &mut out),
            None => out.push(pts[i + 1]),
        }
    }
    out
}

/// A clockwise arc: DXF stores it mirrored (angles negated). The start that
/// meets the previous edge decides when a writer stored it the other way.
fn arc_edge(c: Vec2, r: f64, a0: f64, a1: f64, ccw: bool, prev: Option<Vec2>, out: &mut Vec<Vec2>) {
    let at = |d: f64| {
        let (s, co) = sin_cos_deg(d);
        v(c.x + r * co, c.y + r * s)
    };
    let (start, end, sweep) = if ccw {
        let span = (a1 - a0).rem_euclid(360.0);
        (a0, a1, if span == 0.0 { 360.0 } else { span })
    } else {
        let mirrored = (-a0, -a1);
        let as_is = (a0, a1);
        let (s, e) = match prev {
            Some(p) if dist(at(as_is.0), p) < dist(at(mirrored.0), p) => as_is,
            _ => mirrored,
        };
        let span = (s - e).rem_euclid(360.0);
        (s, e, -(if span == 0.0 { 360.0 } else { span }))
    };
    out.push(at(start));
    arc_points(c, r, rad(start), rad(sweep), Some(at(end)), out);
}

/// A boundary path's points (object coordinates) and whether it had curves.
fn path_points(path: &Path) -> (Vec<Vec2>, bool) {
    match path {
        Path::Poly { pts, bulges } => {
            let pts: Vec<Vec2> = pts.iter().map(|p| v(p[0], p[1])).collect();
            let curved = bulges.iter().any(|&b| b != 0.0);
            (bulge_ring_points(&pts, bulges), curved)
        }
        Path::Edges(edges) => {
            let mut out: Vec<Vec2> = Vec::new();
            let mut curved = false;
            for edge in edges {
                let prev = out.last().copied();
                match edge {
                    Edge::Line { a, b } => {
                        out.push(v(a[0], a[1]));
                        out.push(v(b[0], b[1]));
                    }
                    Edge::Arc { c, r, a0, a1, ccw } => {
                        curved = true;
                        arc_edge(v(c[0], c[1]), *r, *a0, *a1, *ccw, prev, &mut out);
                    }
                    Edge::Ellipse { c, major, ratio, a0, a1, ccw } => {
                        curved = true;
                        let (a0, a1) = if *ccw { (*a0, *a1) } else { (-*a0, -*a1) };
                        // Stored angles to parameters of the ellipse.
                        let param = |a: f64| atan2(sin(rad(a)) / ratio.max(1e-12), cos(rad(a)));
                        let (t0, mut t1) = (param(a0), param(a1));
                        if *ccw {
                            if t1 <= t0 {
                                t1 += TAU;
                            }
                        } else if t1 >= t0 {
                            t1 -= TAU;
                        }
                        let (mx, my) = (major[0], major[1]);
                        let (nx, ny) = (-my * ratio, mx * ratio);
                        let n = (((t1 - t0).abs() / TAU) * 72.0).ceil().max(1.0) as usize;
                        for i in 0..=n {
                            let t = t0 + (t1 - t0) * i as f64 / n as f64;
                            out.push(v(c[0] + mx * cos(t) + nx * sin(t), c[1] + my * cos(t) + ny * sin(t)));
                        }
                    }
                    Edge::Spline { degree, knots, ctrl, weights } => {
                        curved = true;
                        let cps: Vec<Vec2> = ctrl.iter().map(|p| v(p[0], p[1])).collect();
                        let w = (weights.len() == cps.len()).then_some(weights.as_slice());
                        if let Some(pts) = nurbs::sample(*degree, knots, &cps, w, SAMPLE_TOLERANCE) {
                            out.extend(pts);
                        } else {
                            out.extend(cps);
                        }
                    }
                }
            }
            (out, curved)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_clockwise_arc_edge_is_read_mirrored() {
        // A clockwise quarter from 90° to 0° is stored as 270° → 360°.
        let mut out = Vec::new();
        arc_edge(v(0.0, 0.0), 1.0, 270.0, 360.0, false, None, &mut out);
        assert!(dist(out[0], v(0.0, 1.0)) < 1e-12 && dist(*out.last().expect("end"), v(1.0, 0.0)) < 1e-12, "{out:?}");
        // Counter-clockwise: as stored.
        let mut out = Vec::new();
        arc_edge(v(0.0, 0.0), 2.0, 0.0, 90.0, true, None, &mut out);
        assert!(dist(out[0], v(2.0, 0.0)) < 1e-12 && dist(*out.last().expect("end"), v(0.0, 2.0)) < 1e-12);
        assert_eq!(out.len(), 19);
    }
}
