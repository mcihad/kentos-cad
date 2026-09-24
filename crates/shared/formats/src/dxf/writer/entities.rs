//! The drawing's objects as DXF entities. Every kind has a DXF entity that
//! holds the same data (LWPOLYLINE keeps bulges, ELLIPSE its parameters,
//! XLINE and RAY their direction, HATCH its rings and a user-defined
//! pattern); what DXF cannot say rides along in KentOS's extended data
//! (labels, attributes, symbols, exact arc angles and hatch patterns).
//! Three kinds change form:
//! - a polygon's holes are closed polylines of their own, linked to it;
//! - a spline is a cubic B-spline that is the app's curve span by span
//!   (the shared core's Bézier form), with the app's points as fit points;
//! - a dimension is drawn as its lines, arc and text (the core's explode):
//!   a DXF dimension needs a block and a dimension style of its own, and
//!   another program would redraw it by its own rules.

use kentos_contracts::{
    Bounds, DimensionStyle, Entity, EntityBase, HatchEntity, HatchPatternType, PathEntity,
    SplineEntity, TextEntity, Vec2,
};
use kentos_geometry_core::Vec2 as CoreVec2;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::spline::catmull_rom_beziers;
use kentos_geometry_core::ops::curve_cuts::Cut;
use kentos_geometry_core::ops::explode::explode_entity;

use super::super::aci;
use super::super::xdata::{self, Meta};
use super::layers::Layers;
use super::template::MODEL_SPACE;
use super::{Handles, Out};
use crate::geom::v;
use crate::math::{PI, TAU, deg, norm_angle, rad, sin_cos_deg};
use crate::report::Report;

/// The app's name for a kind (the export report counts by it).
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

/// How the report names a kind (Turkish, as the app's ENTITY_KIND_LABEL).
fn kind_label(e: &Entity) -> &'static str {
    match e {
        Entity::Point(_) => "Nokta",
        Entity::Line(_) => "Çizgi",
        Entity::Polyline(_) => "Çoklu çizgi",
        Entity::Polygon(_) => "Kapalı alan",
        Entity::Circle(_) => "Daire",
        Entity::Arc(_) => "Yay",
        Entity::Ellipse(_) => "Elips",
        Entity::Spline(_) => "Eğri",
        Entity::Xline(_) => "Yardımcı çizgi",
        Entity::Ray(_) => "Işın",
        Entity::Text(_) => "Yazı",
        Entity::Dimension(_) => "Ölçü",
        Entity::Hatch(_) => "Tarama",
    }
}

fn ok(p: Vec2) -> bool {
    p.x.is_finite() && p.y.is_finite()
}

fn all_ok(pts: &[Vec2]) -> bool {
    pts.iter().all(|p| ok(*p))
}

fn nums_ok(xs: &[f64]) -> bool {
    xs.iter().all(|x| x.is_finite())
}

/// Every number of an object is finite (a file can hold nothing else).
fn finite(e: &Entity) -> bool {
    match e {
        Entity::Point(p) => ok(p.p) && p.z.is_none_or(f64::is_finite),
        Entity::Line(l) => ok(l.a) && ok(l.b),
        Entity::Polyline(p) | Entity::Polygon(p) => {
            all_ok(&p.pts)
                && p.bulges.as_deref().is_none_or(nums_ok)
                && p.holes
                    .iter()
                    .flatten()
                    .all(|h| all_ok(&h.pts) && h.bulges.as_deref().is_none_or(nums_ok))
        }
        Entity::Circle(c) => ok(c.c) && c.r.is_finite(),
        Entity::Arc(a) => ok(a.c) && nums_ok(&[a.r, a.a0, a.a1]),
        Entity::Ellipse(e) => ok(e.c) && ok(e.major) && nums_ok(&[e.ratio, e.t0, e.t1]),
        Entity::Spline(s) => all_ok(&s.pts),
        Entity::Xline(x) | Entity::Ray(x) => ok(x.p) && ok(x.dir),
        Entity::Text(t) => ok(t.p) && nums_ok(&[t.height, t.rotation]),
        Entity::Dimension(d) => {
            ok(d.a)
                && ok(d.b)
                && d.c.is_none_or(ok)
                && nums_ok(&[d.offset, d.height])
                && d.angle.is_none_or(f64::is_finite)
        }
        Entity::Hatch(h) => {
            all_ok(&h.ring)
                && h.holes.iter().flatten().all(|r| all_ok(r))
                && nums_ok(&[h.pattern.angle, h.pattern.spacing])
        }
    }
}

fn core(p: Vec2) -> CoreVec2 {
    CoreVec2::new(p.x, p.y)
}

fn app(p: CoreVec2) -> Vec2 {
    v(p.x, p.y)
}

/// A TEXT value that reads back as `s`: one line (DXF text cannot break), a
/// caret in DXF's notation ("^ "), percent signs doubled into "%%%" when the
/// text holds "%%" (AutoCAD's control codes). The bool: line breaks or
/// control characters became spaces.
fn text_value(s: &str) -> (String, bool) {
    let mut changed = false;
    let mut out = String::with_capacity(s.len());
    let percent = s.contains("%%");
    for c in s.chars() {
        match c {
            c if c.is_control() => {
                changed = true;
                out.push(' ');
            }
            '^' => out.push_str("^ "),
            '%' if percent => out.push_str("%%%"),
            c => out.push(c),
        }
    }
    (out, changed)
}

pub(super) struct Writer<'a> {
    pub out: &'a mut Out,
    pub handles: &'a mut Handles,
    pub layers: &'a Layers,
    pub report: &'a mut Report,
    /// Extent of what was written (for the header and the opening view).
    pub extent: Option<Bounds>,
    pub points: bool,
}

impl Writer<'_> {
    fn grow(&mut self, p: Vec2) {
        let b = self.extent.get_or_insert(Bounds {
            min_x: p.x,
            min_y: p.y,
            max_x: p.x,
            max_y: p.y,
        });
        b.min_x = b.min_x.min(p.x);
        b.min_y = b.min_y.min(p.y);
        b.max_x = b.max_x.max(p.x);
        b.max_y = b.max_y.max(p.y);
    }

    fn grow_round(&mut self, c: Vec2, r: f64) {
        self.grow(v(c.x - r, c.y - r));
        self.grow(v(c.x + r, c.y + r));
    }

    /// The entity's head: type, handle, owner, layer and colour. Returns its handle.
    fn begin(&mut self, kind: &str, base: &EntityBase) -> u64 {
        let h = self.handles.take();
        self.out.str(0, kind);
        self.out.handle(5, h);
        self.out.handle(330, MODEL_SPACE);
        self.out.str(100, "AcDbEntity");
        let layers = self.layers;
        let layer = match layers.name_of(&base.layer_id) {
            Some(name) => name,
            None => {
                self.report.note(
                    "Katman",
                    "katmanı çizimde bulunamayan nesneler 0 katmanına yazıldı",
                    0,
                );
                "0"
            }
        };
        self.out.str(8, layer);
        if let Some(c) = &base.color {
            let (dc, note) = aci::from_app(c);
            if let Some(n) = note {
                self.report.note("Nesne rengi", n, 0);
            }
            self.out.int(62, i64::from(dc.aci));
            if let Some(rgb) = dc.rgb {
                self.out.int(420, rgb);
            }
        }
        h
    }

    /// What of the object's base DXF cannot hold.
    fn base_meta(base: &EntityBase) -> Meta {
        Meta {
            label: base.label.clone(),
            attrs: base.attrs.clone(),
            symbol: base.symbol.clone(),
            color: base
                .color
                .as_ref()
                .filter(|c| aci::from_app(c).0.read_back() != **c)
                .cloned(),
            ..Meta::default()
        }
    }

    /// The object's extended data, last of its groups (without the attributes if they are too large for AutoCAD).
    fn end(&mut self, meta: Meta) {
        let mut groups = xdata::groups(&meta);
        if xdata::size(&groups) > xdata::MAX_BYTES {
            self.report.note(
                "Öznitelikler",
                "bir nesnenin etiketi, öznitelikleri ve sembolü AutoCAD'in nesne başına genişletilmiş veri sınırını (16 KB) aştığı için yazılmadı",
                0,
            );
            groups = xdata::groups(&Meta {
                label: None,
                attrs: Default::default(),
                symbol: None,
                ..meta
            });
        }
        self.out.xdata(&groups);
    }

    pub fn entity(&mut self, e: &Entity) {
        if !finite(e) {
            return self.report.skip(
                kind_label(e),
                "sayı olmayan (sonsuz ya da tanımsız) bir değeri var; yazılmadı",
                0,
            );
        }
        let written = match e {
            Entity::Point(p) => {
                self.begin("POINT", &p.base);
                self.out.str(100, "AcDbPoint");
                self.out.xy(10, p.p);
                self.out.real(30, p.z.unwrap_or(0.0));
                self.grow(p.p);
                self.points = true;
                let mut m = Self::base_meta(&p.base);
                // DXF points always have a Z; the reader takes 0 as "none" unless told.
                m.z = p.z == Some(0.0);
                self.end(m);
                true
            }
            Entity::Line(l) => {
                self.begin("LINE", &l.base);
                self.out.str(100, "AcDbLine");
                self.out.xyz(10, l.a);
                self.out.xyz(11, l.b);
                self.grow(l.a);
                self.grow(l.b);
                self.end(Self::base_meta(&l.base));
                true
            }
            Entity::Polyline(p) => self.path(p, false),
            Entity::Polygon(p) => self.path(p, true),
            Entity::Circle(c) => {
                if !(c.r > 0.0) {
                    return self
                        .report
                        .skip("Daire", "yarıçapı sıfır ya da negatif; yazılmadı", 0);
                }
                self.begin("CIRCLE", &c.base);
                self.out.str(100, "AcDbCircle");
                self.out.xyz(10, c.c);
                self.out.real(40, c.r);
                self.grow_round(c.c, c.r);
                self.end(Self::base_meta(&c.base));
                true
            }
            Entity::Arc(a) => {
                if !(a.r > 0.0) {
                    return self
                        .report
                        .skip("Yay", "yarıçapı sıfır ya da negatif; yazılmadı", 0);
                }
                let mut m = Self::base_meta(&a.base);
                self.arc(&a.base, a.c, a.r, a.a0, a.a1, &mut m);
                self.end(m);
                true
            }
            Entity::Ellipse(el) => self.ellipse(e, el),
            Entity::Spline(s) => self.spline(s),
            Entity::Xline(x) | Entity::Ray(x) => {
                if x.dir.x == 0.0 && x.dir.y == 0.0 {
                    return self
                        .report
                        .skip(kind_label(e), "doğrultusu yok; yazılmadı", 0);
                }
                let (kind, class) = match e {
                    Entity::Ray(_) => ("RAY", "AcDbRay"),
                    _ => ("XLINE", "AcDbXline"),
                };
                self.begin(kind, &x.base);
                self.out.str(100, class);
                self.out.xyz(10, x.p);
                self.out.xyz(11, x.dir);
                self.grow(x.p);
                self.end(Self::base_meta(&x.base));
                true
            }
            Entity::Text(t) => self.text(t, &t.base, Self::base_meta(&t.base)),
            Entity::Dimension(d) => self.dimension(d),
            Entity::Hatch(h) => self.hatch(h),
        };
        if written {
            self.report.count(kind_name(e));
        }
    }

    /// An ARC (angles in degrees as DXF has them; the exact radians in the extended data when degrees lose a bit).
    fn arc(&mut self, base: &EntityBase, c: Vec2, r: f64, a0: f64, a1: f64, m: &mut Meta) {
        self.begin("ARC", base);
        self.out.str(100, "AcDbCircle");
        self.out.xyz(10, c);
        self.out.real(40, r);
        self.out.str(100, "AcDbArc");
        let (d0, d1) = (deg(a0), deg(a1));
        self.out.real(50, d0);
        self.out.real(51, d1);
        // What the reader makes of the degrees: radians, turned into [0, 2π).
        let back = |d: f64| norm_angle(rad(d) + 0.0);
        if back(d0) != a0 || back(d1) != a1 {
            m.arc = Some((a0, a1));
        }
        self.grow_round(c, r);
    }

    fn lwpolyline(
        &mut self,
        base: &EntityBase,
        pts: &[Vec2],
        bulges: Option<&[f64]>,
        closed: bool,
        meta: Meta,
    ) -> u64 {
        let h = self.begin("LWPOLYLINE", base);
        self.out.str(100, "AcDbPolyline");
        self.out.int(90, pts.len() as i64);
        self.out.int(70, i64::from(closed));
        for (i, p) in pts.iter().enumerate() {
            self.out.xy(10, *p);
            let b = bulges.and_then(|b| b.get(i)).copied().unwrap_or(0.0);
            // An open path has no segment after its last vertex.
            if b != 0.0 && (closed || i + 1 < pts.len()) {
                self.out.real(42, b);
            }
            self.grow(*p);
        }
        self.end(meta);
        h
    }

    /// A polyline or polygon; a polygon's holes follow as closed polylines naming it.
    fn path(&mut self, p: &PathEntity, closed: bool) -> bool {
        let what = if closed {
            "Kapalı alan"
        } else {
            "Çoklu çizgi"
        };
        if p.pts.len() < 2 {
            self.report.skip(what, "iki köşesi yok; yazılmadı", 0);
            return false;
        }
        let owner = self.lwpolyline(
            &p.base,
            &p.pts,
            p.bulges.as_deref(),
            closed,
            Self::base_meta(&p.base),
        );
        let holes = if closed {
            p.holes.as_deref().unwrap_or(&[])
        } else {
            &[]
        };
        for hole in holes {
            if hole.pts.len() < 2 {
                self.report.skip("Ada", "iki köşesi yok; yazılmadı", 0);
                continue;
            }
            let meta = Meta {
                hole_of: Some(owner),
                ..Meta::default()
            };
            self.lwpolyline(&p.base, &hole.pts, hole.bulges.as_deref(), true, meta);
        }
        if !holes.is_empty() {
            self.report.note(
                "Adalı alan",
                "adaları ayrı kapalı çoklu çizgiler olarak yazıldı (KentOS'a geri okununca yine adalı alan olur)",
                0,
            );
        }
        true
    }

    fn ellipse(&mut self, e: &Entity, el: &kentos_contracts::EllipseEntity) -> bool {
        let long = el.major.x != 0.0 || el.major.y != 0.0;
        if !(el.ratio > 0.0) || !long {
            self.report.skip(
                kind_label(e),
                "ekseni ya da eksen oranı geçersiz; yazılmadı",
                0,
            );
            return false;
        }
        let full = el.t0 == el.t1;
        // DXF wants the minor/major ratio at most 1: the other axis becomes the major one.
        let (major, ratio, t0, t1) = if el.ratio > 1.0 {
            self.report.note(
                "Elips",
                "küçük/büyük eksen oranı 1'den büyüktü; eksenler yer değiştirilerek yazıldı",
                0,
            );
            let minor = v(-el.major.y * el.ratio, el.major.x * el.ratio);
            (
                minor,
                1.0 / el.ratio,
                norm_angle(el.t0 - PI / 2.0),
                norm_angle(el.t1 - PI / 2.0),
            )
        } else {
            (el.major, el.ratio, el.t0, el.t1)
        };
        self.begin("ELLIPSE", &el.base);
        self.out.str(100, "AcDbEllipse");
        self.out.xyz(10, el.c);
        self.out.xyz(11, major);
        self.out.real(40, ratio);
        self.out.real(41, if full { 0.0 } else { t0 });
        self.out.real(42, if full { TAU } else { t1 });
        let r = crate::math::hypot(major.x, major.y);
        self.grow_round(el.c, r);
        self.end(Self::base_meta(&el.base));
        true
    }

    /// A SPLINE: the app's curve as a clamped cubic B-spline of Bézier spans
    /// (every span end is one of the app's points, bit for bit), the points
    /// as fit points, and the extended data saying it is KentOS's curve.
    fn spline(&mut self, s: &SplineEntity) -> bool {
        let pts: Vec<CoreVec2> = s.pts.iter().map(|p| core(*p)).collect();
        let spans = catmull_rom_beziers(&pts, s.closed);
        let Some(first) = spans.first() else {
            self.report.skip("Eğri", "iki noktası yok; yazılmadı", 0);
            return false;
        };
        let mut ctrl: Vec<Vec2> = vec![app(first.ctrl[0])];
        let mut knots: Vec<f64> = vec![0.0; 4];
        let mut t = 0.0;
        for (i, span) in spans.iter().enumerate() {
            ctrl.extend([app(span.ctrl[1]), app(span.ctrl[2]), app(span.ctrl[3])]);
            t += span.dt;
            let times = if i + 1 == spans.len() { 4 } else { 3 };
            knots.extend(std::iter::repeat_n(t, times));
        }
        // A closed curve ends where it started; its fit points say so too.
        let mut fit = s.pts.clone();
        if s.closed && s.pts.len() >= 3 {
            fit.push(s.pts[0]);
        }
        self.begin("SPLINE", &s.base);
        self.out.str(100, "AcDbSpline");
        self.out.real(210, 0.0);
        self.out.real(220, 0.0);
        self.out.real(230, 1.0);
        // Planar; not DXF's "closed", which some readers take as periodic (wrapping the control points).
        self.out.int(70, 8);
        self.out.int(71, 3);
        self.out.int(72, knots.len() as i64);
        self.out.int(73, ctrl.len() as i64);
        self.out.int(74, fit.len() as i64);
        for k in &knots {
            self.out.real(40, *k);
        }
        for p in &ctrl {
            self.out.xyz(10, *p);
            self.grow(*p);
        }
        for p in &fit {
            self.out.xyz(11, *p);
        }
        let mut m = Self::base_meta(&s.base);
        m.curve = Some(s.closed);
        self.end(m);
        true
    }

    fn text(&mut self, t: &TextEntity, base: &EntityBase, meta: Meta) -> bool {
        let (value, changed) = text_value(&t.text);
        if t.text.trim().is_empty() {
            self.report.skip("Yazı", "boş yazı yazılmadı", 0);
            return false;
        }
        if !(t.height > 0.0) {
            self.report
                .skip("Yazı", "yazı yüksekliği sıfır ya da negatif; yazılmadı", 0);
            return false;
        }
        if changed {
            self.report.note(
                "Yazı",
                "satır sonları ve denetim karakterleri boşluk oldu (DXF yazısı tek satırdır)",
                0,
            );
        }
        self.begin("TEXT", base);
        self.out.str(100, "AcDbText");
        self.out.xyz(10, t.p);
        self.out.real(40, t.height);
        self.out.str(1, &value);
        if t.rotation != 0.0 {
            self.out.real(50, t.rotation);
        }
        self.out.str(100, "AcDbText");
        self.grow(t.p);
        self.end(meta);
        true
    }

    /// A dimension as the lines, arc and text the app draws for it (the core's explode, with the value the app formatted).
    fn dimension(&mut self, d: &kentos_contracts::DimensionEntity) -> bool {
        let style = d.style.map(|s| {
            match s {
                DimensionStyle::Aligned => "aligned",
                DimensionStyle::Linear => "linear",
                DimensionStyle::Angular => "angular",
                DimensionStyle::Radius => "radius",
                DimensionStyle::Diameter => "diameter",
            }
            .to_string()
        });
        let shape = Shape::Dimension {
            a: core(d.a),
            b: core(d.b),
            offset: d.offset,
            height: d.height,
            text: d.text.clone(),
            style,
            angle: d.angle,
            c: d.c.map(core),
        };
        let pieces = match explode_entity(&shape, "") {
            Cut::Pieces(p) => p,
            Cut::Error(e) => {
                self.report.skip("Ölçü", &format!("{e} Yazılmadı."), 0);
                return false;
            }
        };
        // The pieces take the dimension's layer and colour, not its attributes (as Patlat does).
        let base = EntityBase {
            label: None,
            attrs: Default::default(),
            symbol: None,
            ..d.base.clone()
        };
        let meta = || Meta {
            color: Self::base_meta(&base).color,
            ..Meta::default()
        };
        for piece in &pieces {
            match &piece.shape {
                Shape::Line { a, b } => {
                    self.begin("LINE", &base);
                    self.out.str(100, "AcDbLine");
                    self.out.xyz(10, app(*a));
                    self.out.xyz(11, app(*b));
                    self.grow(app(*a));
                    self.grow(app(*b));
                    self.end(meta());
                }
                Shape::Arc { c, r, a0, a1 } => {
                    let mut m = meta();
                    self.arc(&base, app(*c), *r, *a0, *a1, &mut m);
                    self.end(m);
                }
                Shape::Text {
                    p,
                    text,
                    height,
                    rotation,
                } => {
                    if text.trim().is_empty() {
                        self.report.note(
                            "Ölçü",
                            "değer yazısı verilmediği için yazısız yazıldı",
                            0,
                        );
                        continue;
                    }
                    let t = TextEntity {
                        base: base.clone(),
                        p: app(*p),
                        text: text.clone(),
                        height: *height,
                        rotation: *rotation,
                    };
                    self.text(&t, &base, meta());
                }
                _ => {}
            }
        }
        self.report.note(
            "Ölçü",
            "çizgi, yay ve yazılarına patlatılarak yazıldı (DXF'te ölçü nesnesi olmaz; KentOS'a ölçü olarak geri okunmaz)",
            0,
        );
        true
    }

    /// A HATCH: the ring and its holes as polyline boundaries, solid or a
    /// user-defined pattern (one family of lines, two at right angles for a cross).
    fn hatch(&mut self, h: &HatchEntity) -> bool {
        if h.ring.len() < 3 {
            self.report
                .skip("Tarama", "sınırının üç köşesi yok; yazılmadı", 0);
            return false;
        }
        let holes: Vec<&Vec<Vec2>> = h.holes.iter().flatten().filter(|r| r.len() >= 3).collect();
        if holes.len() < h.holes.as_ref().map_or(0, Vec::len) {
            self.report
                .skip("Tarama adası", "üç köşesi yok; yazılmadı", 0);
        }
        let solid = h.pattern.kind == HatchPatternType::Solid;
        self.begin("HATCH", &h.base);
        self.out.str(100, "AcDbHatch");
        self.out.xyz(10, v(0.0, 0.0));
        self.out.real(210, 0.0);
        self.out.real(220, 0.0);
        self.out.real(230, 1.0);
        self.out.str(2, if solid { "SOLID" } else { "_USER" });
        self.out.int(70, i64::from(solid));
        self.out.int(71, 0);
        self.out.int(91, 1 + holes.len() as i64);
        // Outer boundary (external polyline), then the islands (polylines).
        self.boundary(&h.ring, 3);
        for hole in &holes {
            self.boundary(hole, 2);
        }
        self.out.int(75, 0);
        self.out.int(76, if solid { 1 } else { 0 });
        let (angle, spacing) = (h.pattern.angle, h.pattern.spacing);
        // What the reader makes of it, to know whether the exact values must ride along.
        let mut read_back = (0.0, 1.0);
        if !solid {
            let families: &[f64] = if h.pattern.kind == HatchPatternType::Cross {
                &[0.0, 90.0]
            } else {
                &[0.0]
            };
            self.out.real(52, angle);
            self.out.real(41, spacing);
            self.out.int(77, i64::from(families.len() == 2));
            self.out.int(78, families.len() as i64);
            for turn in families {
                let a = angle + turn;
                let (s, c) = sin_cos_deg(a);
                let (ox, oy) = (-s * spacing, c * spacing);
                self.out.real(53, a);
                self.out.real(43, 0.0);
                self.out.real(44, 0.0);
                self.out.real(45, ox);
                self.out.real(46, oy);
                self.out.int(79, 0);
                if *turn == 0.0 {
                    read_back = (a.rem_euclid(180.0), (ox * -s + oy * c).abs());
                }
            }
        }
        self.out.int(98, 0);
        for p in &h.ring {
            self.grow(*p);
        }
        let mut m = Self::base_meta(&h.base);
        if read_back != (angle, spacing) {
            m.pattern = Some((angle, spacing));
        }
        self.end(m);
        true
    }

    fn boundary(&mut self, ring: &[Vec2], flags: i64) {
        self.out.int(92, flags);
        self.out.int(72, 0);
        self.out.int(73, 1);
        self.out.int(93, ring.len() as i64);
        for p in ring {
            self.out.xy(10, *p);
        }
        self.out.int(97, 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_values_read_back_as_written() {
        assert_eq!(text_value("Ada 12"), ("Ada 12".into(), false));
        assert_eq!(text_value("x^2 %5"), ("x^ 2 %5".into(), false));
        assert_eq!(text_value("%%d yok"), ("%%%%%%d yok".into(), false));
        assert_eq!(text_value("iki\nsatır"), ("iki satır".into(), true));
    }
}
