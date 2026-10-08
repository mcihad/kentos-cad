//! The drawing's objects as DXF entities. Every kind has a DXF entity that
//! holds the same data (LWPOLYLINE keeps bulges, ELLIPSE its parameters,
//! XLINE and RAY their direction, HATCH its rings and a user-defined
//! pattern); what DXF cannot say rides along in KentOS's extended data
//! (labels, attributes, symbols, exact arc angles and hatch patterns).
//! Three kinds change form (and elevations, docs/adr/0142, change a fourth):
//! - a polygon's holes are closed polylines of their own, linked to it; a
//!   multi-part area's parts too, each carrying the object's data, its
//!   holes linked to it (DXF has no such object, docs/adr/0143);
//! - a spline is a cubic B-spline that is the app's curve span by span
//!   (the shared core's Bézier form), with the app's points as fit points;
//! - a dimension is a DXF DIMENSION drawn by an anonymous block of its own
//!   (the app's lines and ticks, the value as MTEXT), with the definition
//!   points another program measures from (`dimension.rs`); an ordinate a
//!   DIMENSION of type 6, an arc length an ARC_DIMENSION, a jogged radius a
//!   LARGE_RADIAL_DIMENSION, Semt and Eğim aligned ones; Zemin is DIMTFILL
//!   and the block's MTEXT over the background (docs/adr/0147 §8);
//! - a polyline or polygon with elevations is a 3D POLYLINE, one Z at each
//!   vertex (a LWPOLYLINE holds a single elevation); a line's are the Z of
//!   its LINE. A vertex with no elevation is written as 0 and KentOS's data
//!   says so, so a KentOS import gets the same drawing back.
//!
//! An insert is an INSERT of its block (docs/adr/0144 §5, `blocks.rs`); a
//! definition's objects are written by a writer of their own, into their
//! block. A leader is a LEADER and the MTEXT of its note (`leader.rs`,
//! docs/adr/0146 §8).

use std::collections::{BTreeMap, HashMap};

use kentos_contracts::blocks::turn_of;
use kentos_contracts::{
    BlockId, Bounds, DimensionArrow, DimensionEntity, DimensionStyle, Entity, EntityBase,
    GradientShape, HatchEntity, HatchGradient, HatchPatternType, InsertEntity, PathEntity,
    SplineEntity, TableAlign, TableEntity, TextAlign, TextEntity, Vec2,
};
use kentos_geometry_core::Vec2 as CoreVec2;
use kentos_geometry_core::geom::hatch_pattern::library_pattern;
use kentos_geometry_core::geom::intersect::Edge;
use kentos_geometry_core::geom::spline::catmull_rom_beziers;
use kentos_geometry_core::geom::table::{DROP, PAD, table_geom};

use super::super::aci;
use super::super::dimension::{self as dim, Definition};
use super::super::xdata::{self, DimMeta, Meta};
use super::blocks::{self, Written};
use super::layers::Layers;
use super::{Handles, Justified, Out};

mod curved;
mod leader;
mod paragraph;
use crate::geom::{has_arcs, v};
use crate::gis::Zs;
use crate::math::{PI, TAU, atan2, deg, hypot, norm_angle, rad, sin_cos_deg};
use crate::num::dxf_real;
use crate::report::Report;

/// A text's mask in DXF, said in the report (docs/adr/0145 §7).
const MASK: &str = "DXF yazısının zemini yoktur: zemin KentOS verisi olarak yazıldı; başka programlar göstermez, KentOS geri okur";

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
        Entity::Insert(_) => "Blok",
        Entity::Leader(_) => "Kılavuz",
        Entity::Table(_) => "Tablo",
        Entity::Image(_) => "Resim",
        Entity::Raster(_) => "Raster",
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

fn zs_ok(zs: &Zs) -> bool {
    zs.iter().flatten().flatten().all(|z| z.is_finite())
}

/// A path's elevations when it has an elevation somewhere and as many
/// entries as vertices (else the path is flat).
fn elevations(zs: &Zs, vertices: usize) -> Option<&[Option<f64>]> {
    zs.as_deref()
        .filter(|z| z.len() == vertices && z.iter().any(Option::is_some))
}

/// What the elevations of an object's vertices need of KentOS's data: the
/// vertices without one (DXF says 0 for them), and that a 0 is data when
/// every elevation is 0 (a DXF reader takes 0 for none).
fn elevation_meta(m: &mut Meta, zs: &[Option<f64>]) {
    m.no_z = zs
        .iter()
        .enumerate()
        .filter_map(|(i, z)| z.is_none().then_some(i))
        .collect();
    m.z = zs.iter().flatten().all(|&z| z == 0.0);
}

/// Every number of an object is finite (a file can hold nothing else).
fn finite(e: &Entity) -> bool {
    match e {
        Entity::Point(p) => {
            ok(p.p)
                && p.z.is_none_or(f64::is_finite)
                && p.parts
                    .iter()
                    .flatten()
                    .all(|q| ok(q.p) && q.z.is_none_or(f64::is_finite))
        }
        Entity::Line(l) => {
            ok(l.a) && ok(l.b) && l.za.is_none_or(f64::is_finite) && l.zb.is_none_or(f64::is_finite)
        }
        Entity::Polyline(p) | Entity::Polygon(p) => {
            let ring = |pts: &[Vec2], zs: &Zs, bulges: Option<&[f64]>| {
                all_ok(pts) && zs_ok(zs) && bulges.is_none_or(nums_ok)
            };
            let holes = |holes: &Option<Vec<kentos_contracts::RingGeometry>>| {
                holes
                    .iter()
                    .flatten()
                    .all(|h| ring(&h.pts, &h.zs, h.bulges.as_deref()))
            };
            ring(&p.pts, &p.zs, p.bulges.as_deref())
                && holes(&p.holes)
                && p.parts.iter().flatten().all(|part| {
                    ring(&part.pts, &part.zs, part.bulges.as_deref()) && holes(&part.holes)
                })
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
                && h.pattern.scale.is_none_or(f64::is_finite)
                && h.pattern.lines.iter().flatten().all(|l| {
                    nums_ok(&[l.angle, l.origin[0], l.origin[1], l.offset[0], l.offset[1]])
                        && nums_ok(&l.dashes)
                })
        }
        Entity::Insert(i) => ok(i.p) && nums_ok(&[i.scale, i.rotation]),
        Entity::Leader(l) => all_ok(&l.pts) && nums_ok(&[l.height, l.rotation]),
        // Never written (docs/adr/0192, 0204, Kapsam dışı): said in the report.
        Entity::Image(_) | Entity::Raster(_) => true,
        Entity::Table(t) => {
            ok(t.p)
                && nums_ok(&[t.height, t.rotation])
                && nums_ok(&t.rows)
                && nums_ok(&t.columns)
                && t.face.oblique.is_none_or(f64::is_finite)
        }
    }
}

fn core(p: Vec2) -> CoreVec2 {
    CoreVec2::new(p.x, p.y)
}

/// Running sums from 0: a table's grid lines from its rows' or columns' sizes.
fn edges(sizes: &[f64]) -> Vec<f64> {
    let mut out = Vec::with_capacity(sizes.len() + 1);
    let mut at = 0.0;
    out.push(at);
    for s in sizes {
        at += s;
        out.push(at);
    }
    out
}

/// A table's own fields as the contract's JSON, without the object's (its
/// layer, colour, label …, which the INSERT and KentOS's data say).
fn table_json(t: &TableEntity) -> String {
    let mut value = serde_json::to_value(t).unwrap_or_default();
    if let Some(map) = value.as_object_mut() {
        for k in [
            "kind",
            "id",
            "layerId",
            "color",
            "attrs",
            "label",
            "symbol",
            "lineWeight",
        ] {
            map.remove(k);
        }
    }
    value.to_string()
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
    /// The dimensions' own blocks (BLOCKS section), and their block records.
    pub blocks: &'a mut Out,
    pub records: &'a mut Vec<(u64, String)>,
    /// The dimensions' blocks written so far: the next is *D and one more.
    pub dimensions: &'a mut usize,
    /// The tables' blocks written so far: the next is *U and one more (docs/adr/0184 §7).
    pub anonymous: &'a mut usize,
    /// What a dimension without a text of its own shows, by object id.
    pub values: &'a BTreeMap<u32, String>,
    /// The project's length decimals and angle unit (a redrawn dimension's text).
    pub decimals: u32,
    pub grads: bool,
    pub handles: &'a mut Handles,
    pub layers: &'a Layers,
    pub report: &'a mut Report,
    /// Extent of what was written (for the header and the opening view; a
    /// definition's in its own coordinates).
    pub extent: Option<Bounds>,
    pub points: bool,
    /// The block record the objects belong to: model space, or a definition's.
    pub owner: u64,
    /// A definition's objects are written (docs/adr/0144 §5): one without a
    /// layer of its own on 0, BYBLOCK where it has no colour or weight of
    /// its own; not counted (the drawing's objects are).
    pub defining: bool,
    /// The blocks' DXF names, and the definitions written so far.
    pub names: &'a HashMap<BlockId, String>,
    pub defined: &'a HashMap<BlockId, Written>,
    /// What an insert's attribute texts show, where (docs/adr/0144 §7).
    pub placing: &'a crate::blocks::Placing,
    /// The STYLE and DIMSTYLE records' names (docs/adr/0183 §7).
    pub styles: &'a super::styles::StyleNames,
    /// How many of the file's unit make a metre (a dimension's DIMLFAC).
    pub per_metre: f64,
    /// The LTYPE records given a handle before the table is written: the
    /// dashed types a dimension's lines take (docs/adr/0205 §6).
    pub ltypes: &'a mut Vec<(kentos_contracts::LineType, u64)>,
    /// AutoCAD's arrow blocks written for leaders' arrowheads, their block
    /// records (docs/adr/0205 §7).
    pub arrows: &'a mut Vec<(kentos_contracts::LeaderArrow, u64)>,
    /// What a block's objects are drawn with now (`block_head`).
    pub pen: Pen,
}

/// What a block's object is drawn with (docs/adr/0205 §6): a colour, a line
/// type and a weight of its own; by default BYBLOCK's colour and none of the
/// others (the dimension's own).
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Pen {
    pub color: Option<aci::DxfColor>,
    pub ltype: Option<&'static str>,
    pub weight: Option<i64>,
}

/// The LTYPE record's handle of `t` in `ltypes`, given one from `handles`
/// the first time; none for a continuous line.
pub(super) fn ltype_handle(
    ltypes: &mut Vec<(kentos_contracts::LineType, u64)>,
    handles: &mut super::Handles,
    t: kentos_contracts::LineType,
) -> Option<u64> {
    if t == kentos_contracts::LineType::Continuous {
        return None;
    }
    if let Some((_, h)) = ltypes.iter().find(|(x, _)| *x == t) {
        return Some(*h);
    }
    let h = handles.take();
    ltypes.push((t, h));
    Some(h)
}

impl Writer<'_> {
    /// A dimension's part's pen (docs/adr/0205 §6): its colour, its line
    /// type (its LTYPE record written with the table) and its weight.
    fn dimension_pen(
        &mut self,
        color: Option<&str>,
        weight: Option<f64>,
        line_type: Option<kentos_contracts::LineType>,
    ) -> Pen {
        let ltype = line_type.and_then(|t| {
            ltype_handle(self.ltypes, self.handles, t).map(|_| super::layers::ltype_name(t))
        });
        Pen {
            color: color.map(|c| aci::from_app(c).0),
            ltype,
            weight: weight.map(|w| super::layers::line_weight(w).0),
        }
    }

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
        self.head(kind, base, h, None);
        h
    }

    /// `begin` with the handle `h` taken before, and the object this one
    /// reacts to, when there is one (a leader's MTEXT its LEADER, docs/adr/0146 §8).
    fn head(&mut self, kind: &str, base: &EntityBase, h: u64, reactor: Option<u64>) {
        self.out.str(0, kind);
        self.out.handle(5, h);
        if let Some(r) = reactor {
            self.out.str(102, "{ACAD_REACTORS");
            self.out.handle(330, r);
            self.out.str(102, "}");
        }
        self.out.handle(330, self.owner);
        self.out.str(100, "AcDbEntity");
        let layers = self.layers;
        let layer = match layers.name_of(&base.layer_id) {
            Some(name) => name,
            // A definition's object without a layer of its own (or one not written) floats on 0.
            None if self.defining => "0",
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
        } else if self.defining {
            // BYBLOCK: the insert's colour, as KentOS draws a block's object without one.
            self.out.int(62, 0);
        }
        if self.defining && base.line_weight.is_none() {
            self.out.int(370, -2);
        }
        // Its own line weight as the nearest one DXF has; the exact one goes in KentOS's data.
        if let Some(w) = base.line_weight {
            let (dxf, rounded) = super::layers::line_weight(w);
            if rounded {
                self.report.note(
                    "Nesne kalınlığı",
                    "DXF'in kalınlıklarından en yakınına yuvarlandı; KentOS kendi değerini geri okur",
                    0,
                );
            }
            self.out.int(370, dxf);
        }
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
            line_weight: base
                .line_weight
                .filter(|w| super::layers::line_weight(*w).1),
            ..Meta::default()
        }
    }

    /// The object's extended data, last of its groups (without the attributes if they are too large for AutoCAD).
    fn end(&mut self, mut meta: Meta) {
        let mut groups = xdata::groups(&meta);
        if xdata::size(&groups) > xdata::MAX_BYTES {
            self.report.note(
                "Öznitelikler",
                "bir nesnenin etiketi, öznitelikleri ve sembolü AutoCAD'in nesne başına genişletilmiş veri sınırını (16 KB) aştığı için yazılmadı",
                0,
            );
            meta.label = None;
            meta.attrs = Default::default();
            meta.symbol = None;
            groups = xdata::groups(&meta);
            // The vertices without an elevation are the last thing to go.
            if xdata::size(&groups) > xdata::MAX_BYTES && !meta.no_z.is_empty() {
                self.report.note(
                    "Kotsuz köşe",
                    "köşe sayısı KentOS verisinin sığacağından çok; kotsuz köşeler KentOS'a geri okununca 0 kotlu olur",
                    0,
                );
                meta.no_z = Vec::new();
                groups = xdata::groups(&meta);
            }
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
                // A multi-point object's every point a POINT with the object's data (docs/adr/0174).
                let points = std::iter::once((p.p, p.z))
                    .chain(p.parts.iter().flatten().map(|q| (q.p, q.z)))
                    .collect::<Vec<_>>();
                for (at, z) in points {
                    self.begin("POINT", &p.base);
                    self.out.str(100, "AcDbPoint");
                    self.out.xy(10, at);
                    self.out.real(30, z.unwrap_or(0.0));
                    self.grow(at);
                    let mut m = Self::base_meta(&p.base);
                    // DXF points always have a Z; the reader takes 0 as "none" unless told.
                    m.z = z == Some(0.0);
                    self.end(m);
                }
                self.points = true;
                if p.parts.as_ref().is_some_and(|ps| !ps.is_empty()) {
                    self.report.note(
                        "Çok noktalı nesne",
                        "DXF'te çok noktalı nesne yok: her noktası nesnenin verisini taşıyan ayrı nokta (POINT) olarak yazıldı (KentOS'a geri okununca ayrı noktalar olur)",
                        0,
                    );
                }
                true
            }
            Entity::Line(l) => {
                self.begin("LINE", &l.base);
                self.out.str(100, "AcDbLine");
                self.out.xy(10, l.a);
                self.out.real(30, l.za.unwrap_or(0.0));
                self.out.xy(11, l.b);
                self.out.real(31, l.zb.unwrap_or(0.0));
                self.grow(l.a);
                self.grow(l.b);
                let mut m = Self::base_meta(&l.base);
                if l.za.is_some() || l.zb.is_some() {
                    elevation_meta(&mut m, &[l.za, l.zb]);
                    self.report.note(
                        "Kot (Z)",
                        "çizgilerin uç kotları LINE'ın Z'si (30, 31) olarak yazıldı",
                        0,
                    );
                    self.without_elevation(&m);
                }
                self.end(m);
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
            // A text along a curve is a block of its letters (docs/adr/0196 §5).
            Entity::Text(t) if t.path.is_some() => self.curved(t),
            // A multi-line text is an MTEXT (docs/adr/0182 §5); a text of one line a TEXT.
            Entity::Text(t) if t.text.contains('\n') || !t.paragraph.is_plain() => {
                self.paragraph(t, Self::base_meta(&t.base))
            }
            Entity::Text(t) => {
                let mut meta = Self::base_meta(&t.base);
                meta.mask = t.mask;
                self.text(t, &t.base, meta)
            }
            Entity::Dimension(d) => self.dimension(d),
            Entity::Hatch(h) => self.hatch(h),
            Entity::Insert(i) => self.insert(i),
            Entity::Leader(l) => self.leader(l),
            Entity::Table(t) => self.table(t),
            // Nor a raster (docs/adr/0204 §10).
            Entity::Raster(_) => {
                self.report.skip(
                    "Raster",
                    "DXF'e raster yazılmaz: DXF rasteri ayrı bir dosyadan okur (IMAGE, IMAGEDEF); rasteri ayrıca verin",
                    0,
                );
                false
            }
            // DXF's IMAGE needs its file beside the drawing (docs/adr/0192, Kapsam dışı).
            Entity::Image(_) => {
                self.report.skip(
                    "Resim",
                    "DXF'e resim yazılmaz: DXF resmi ayrı bir dosyadan okur (IMAGE, IMAGEDEF); resmi ayrıca verin",
                    0,
                );
                false
            }
        };
        if written && !self.defining {
            self.report.count(e.kind());
        }
    }

    /// An INSERT (docs/adr/0144 §5): its block's name, its point, its scale
    /// in X, Y (−Y when mirrored: the definition's x axis) and Z, its turn in
    /// degrees (the exact radians in KentOS's data when degrees lose a bit).
    /// Its extent is its block's, placed. With attribute definitions its
    /// block's (§7): an ATTRIB for each, its text as the insert shows it (its
    /// value, else the default) where KentOS shows it, then SEQEND.
    fn insert(&mut self, i: &InsertEntity) -> bool {
        let (names, defined) = (self.names, self.defined);
        let Some(name) = names.get(&i.block) else {
            self.report.skip(
                "Blok",
                "bloğunun tanımı dışa aktarılanlarda yok; yazılmadı",
                0,
            );
            return false;
        };
        // Only a block that holds itself (through others) is not written before its inserts.
        let Some(block) = defined.get(&i.block) else {
            self.report.skip(
                "Blok",
                &format!("“{name}” bloğu kendini içeriyor; o yerleştirme yazılmadı"),
                0,
            );
            return false;
        };
        if !(i.scale > 0.0) {
            self.report
                .skip("Blok", "ölçeği sıfır ya da negatif; yazılmadı", 0);
            return false;
        }
        let h = self.begin("INSERT", &i.base);
        self.out.str(100, "AcDbBlockReference");
        // Attributes follow.
        let attributed = !block.tags.is_empty();
        if attributed {
            self.out.int(66, 1);
        }
        self.out.str(2, name);
        self.out.xyz(10, i.p);
        self.out.real(41, i.scale);
        self.out.real(42, if i.mirror { -i.scale } else { i.scale });
        self.out.real(43, i.scale);
        let degrees = deg(i.rotation);
        self.out.real(50, degrees);
        let mut m = Self::base_meta(&i.base);
        // Its attributes' values go out once, as ATTRIBs (§7): another program edits those.
        for (tag, _) in &block.tags {
            m.attrs.remove(tag);
        }
        if turn_of(degrees) != i.rotation {
            m.turn = Some(i.rotation);
        }
        match block.extent {
            Some(b) => {
                for (x, y) in [
                    (b.min_x, b.min_y),
                    (b.max_x, b.min_y),
                    (b.max_x, b.max_y),
                    (b.min_x, b.max_y),
                ] {
                    self.grow(blocks::placed(i, block.base, v(x, y)));
                }
            }
            None => self.grow(i.p),
        }
        self.end(m);
        if attributed {
            self.attributes(i, h, &block.tags);
        }
        true
    }

    /// An insert's ATTRIBs and their SEQEND, owned by the insert `owner` (§7).
    fn attributes(&mut self, i: &InsertEntity, owner: u64, tags: &[(String, String)]) {
        let layers = self.layers;
        let layer = layers.name_of(&i.base.layer_id).unwrap_or("0").to_owned();
        let placing = self.placing;
        for (tag, t) in placing.attribute_texts(i) {
            let Some((_, dxf)) = tags.iter().find(|(own, _)| *own == tag) else {
                continue;
            };
            let h = self.handles.take();
            self.out.str(0, "ATTRIB");
            self.out.handle(5, h);
            self.out.handle(330, owner);
            self.out.str(100, "AcDbEntity");
            self.out.str(8, &layer);
            if let Some(c) = &i.base.color {
                let (dc, _) = aci::from_app(c);
                self.out.int(62, i64::from(dc.aci));
                if let Some(rgb) = dc.rgb {
                    self.out.int(420, rgb);
                }
            }
            let value = blocks::one_line(&t.text);
            if value != t.text {
                self.report.note("Öznitelik", blocks::ONE_LINE, 0);
            }
            self.out.str(100, "AcDbText");
            let vertical = self.out.text(
                &value,
                &value,
                &Justified {
                    p: t.p,
                    height: t.height,
                    rotation: t.rotation,
                    align: t.align,
                    width_factor: t.width_factor,
                },
            );
            self.out.str(100, "AcDbAttribute");
            self.out.str(2, dxf);
            self.out.int(70, 0);
            if vertical != 0 {
                self.out.int(74, vertical);
            }
        }
        let h = self.handles.take();
        self.out.str(0, "SEQEND");
        self.out.handle(5, h);
        self.out.handle(330, owner);
        self.out.str(100, "AcDbEntity");
        self.out.str(8, &layer);
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

    /// A LWPOLYLINE; `elevation` is its one elevation (group 38), when it has one.
    fn lwpolyline(
        &mut self,
        base: &EntityBase,
        pts: &[Vec2],
        bulges: Option<&[f64]>,
        closed: bool,
        elevation: Option<f64>,
        meta: Meta,
    ) -> u64 {
        let h = self.begin("LWPOLYLINE", base);
        self.out.str(100, "AcDbPolyline");
        self.out.int(90, pts.len() as i64);
        self.out.int(70, i64::from(closed));
        if let Some(z) = elevation {
            self.out.real(38, z);
        }
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

    /// Says when vertices without an elevation were written as 0 (`elevation_meta`).
    fn without_elevation(&mut self, m: &Meta) {
        if !m.no_z.is_empty() {
            self.report.note(
                "Kotsuz köşe",
                "kotu olmayan köşeler 0 yazıldı (başka programlar onları 0 kotlu görür); KentOS geri okurken kotsuz sayar",
                0,
            );
        }
    }

    /// A 3D POLYLINE: one Z at each vertex (0 where a vertex has none), the
    /// KentOS data of the object between its header and its vertices.
    fn polyline3d(
        &mut self,
        base: &EntityBase,
        pts: &[Vec2],
        zs: &[Option<f64>],
        closed: bool,
        meta: Meta,
    ) -> u64 {
        let h = self.begin("POLYLINE", base);
        self.out.str(100, "AcDb3dPolyline");
        // Vertices follow; the header's point is the origin.
        self.out.int(66, 1);
        self.out.xyz(10, v(0.0, 0.0));
        self.out.int(70, 8 | i64::from(closed));
        self.end(meta);
        let layers = self.layers;
        let layer = layers.name_of(&base.layer_id).unwrap_or("0");
        for (i, p) in pts.iter().enumerate() {
            self.sub_entity("VERTEX", layer);
            self.out.str(100, "AcDbVertex");
            self.out.str(100, "AcDb3dPolylineVertex");
            self.out.xy(10, *p);
            self.out
                .real(30, zs.get(i).copied().flatten().unwrap_or(0.0));
            self.out.int(70, 32);
            self.grow(*p);
        }
        self.sub_entity("SEQEND", layer);
        h
    }

    /// The head of an entity that belongs to another (a VERTEX, a SEQEND): type, handle, owner and layer.
    fn sub_entity(&mut self, kind: &str, layer: &str) {
        let h = self.handles.take();
        self.out.str(0, kind);
        self.out.handle(5, h);
        self.out.handle(330, self.owner);
        self.out.str(100, "AcDbEntity");
        self.out.str(8, layer);
    }

    /// A polyline or polygon (or a hole of one, `outline` false) with elevations. DXF holds
    /// them at the vertex only in a 3D POLYLINE, which has no arcs: a path with arcs keeps its
    /// arcs and holds one elevation for all its vertices (a LWPOLYLINE's 38) when they share
    /// it, else it is written without (both said).
    fn elevated(
        &mut self,
        (base, pts, bulges): (&EntityBase, &[Vec2], Option<&[f64]>),
        zs: &[Option<f64>],
        closed: bool,
        outline: bool,
        mut meta: Meta,
    ) -> u64 {
        if !bulges.is_some_and(has_arcs) {
            elevation_meta(&mut meta, zs);
            if outline {
                self.report.note(
                    "Kot (Z)",
                    "kotlu çoklu çizgi ve alanlar 3B çoklu çizgi (POLYLINE) olarak yazıldı",
                    0,
                );
            }
            self.without_elevation(&meta);
            return self.polyline3d(base, pts, zs, closed, meta);
        }
        let level = zs
            .first()
            .copied()
            .flatten()
            .filter(|z| zs.iter().all(|w| *w == Some(*z)));
        match level {
            Some(z) => {
                if outline {
                    self.report.note(
                        "Kot (Z)",
                        "yaylı çoklu çizginin köşe kotları aynı; DXF'in 3B çoklu çizgisi yay taşımadığı için çoklu çizginin yüksekliği (38) olarak yazıldı",
                        0,
                    );
                }
                meta.z = z == 0.0;
                self.lwpolyline(base, pts, bulges, closed, Some(z), meta)
            }
            None => {
                if outline {
                    self.report.note(
                        "Kot (Z)",
                        "yaylı çoklu çizginin köşe kotları farklı; DXF'in 3B çoklu çizgisi yay taşımadığı için kotlar yazılmadı, yaylar korundu",
                        0,
                    );
                }
                self.lwpolyline(base, pts, bulges, closed, None, meta)
            }
        }
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
        let meta = Self::base_meta(&p.base);
        let owner = match elevations(&p.zs, p.pts.len()) {
            Some(zs) => self.elevated(
                (&p.base, &p.pts, p.bulges.as_deref()),
                zs,
                closed,
                true,
                meta,
            ),
            None => self.lwpolyline(&p.base, &p.pts, p.bulges.as_deref(), closed, None, meta),
        };
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
            let ring = (&p.base, hole.pts.as_slice(), hole.bulges.as_deref());
            match elevations(&hole.zs, hole.pts.len()) {
                Some(zs) => self.elevated(ring, zs, true, false, meta),
                None => self.lwpolyline(ring.0, ring.1, ring.2, true, None, meta),
            };
        }
        if !holes.is_empty() {
            self.report.note(
                "Adalı alan",
                "adaları ayrı kapalı çoklu çizgiler olarak yazıldı (KentOS'a geri okununca yine adalı alan olur)",
                0,
            );
        }
        // A multi-part area's or polyline's other parts: each a polyline with the
        // object's data, closed for an area (docs/adr/0143, 0174).
        let parts = p.parts.as_deref().unwrap_or(&[]);
        for part in parts {
            let part = PathEntity {
                base: p.base.clone(),
                pts: part.pts.clone(),
                bulges: part.bulges.clone(),
                holes: part.holes.clone(),
                zs: part.zs.clone(),
                parts: None,
            };
            self.path(&part, closed);
        }
        if !parts.is_empty() && closed {
            self.report.note(
                "Çok parçalı alan",
                "DXF'te çok parçalı alan yok: her parça nesnenin verisini taşıyan ayrı kapalı çoklu çizgi olarak yazıldı (KentOS'a geri okununca ayrı alanlar olur)",
                0,
            );
        } else if !parts.is_empty() {
            self.report.note(
                "Çok parçalı çoklu çizgi",
                "DXF'te çok parçalı çizgi yok: her parça nesnenin verisini taşıyan ayrı çoklu çizgi olarak yazıldı (KentOS'a geri okununca ayrı çoklu çizgiler olur)",
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
        if t.mask {
            self.report.note("Yazı zemini", MASK, 0);
        }
        self.begin("TEXT", base);
        self.out.str(100, "AcDbText");
        let vertical = self.out.text(
            &value,
            &value,
            &Justified {
                p: t.p,
                height: t.height,
                rotation: t.rotation,
                align: t.align,
                width_factor: t.width_factor,
            },
        );
        // Its style and slant (docs/adr/0183 §7); Standard is DXF's own default, left unsaid as before styles.
        let style = self.styles.text(&t.face);
        if style != "Standard" {
            self.out.str(7, style);
        }
        if let Some(o) = t.face.oblique.filter(|_| t.face.font.is_some()) {
            self.out.real(51, o);
        }
        self.out.str(100, "AcDbText");
        if vertical != 0 {
            self.out.int(73, vertical);
        }
        self.grow(t.p);
        let mut meta = meta;
        meta.face = face_json(&t.face);
        self.end(meta);
        true
    }

    /// A table (docs/adr/0184 §7). DXF's ACAD_TABLE needs a table style
    /// object other programs read differently; the table is written as an
    /// anonymous block (*U) of its lines (LINE) and its cells' words (TEXT,
    /// justified as their columns, so that another program's typeface keeps
    /// them in their cells), in its own frame, placed by an INSERT at its top
    /// left corner, turned as it is. The INSERT carries the table's own
    /// fields in KentOS's data: KentOS reads the table back (where the INSERT
    /// is now, as another program may have moved it). A table too large for
    /// that data stays its lines and words, said in the report.
    fn table(&mut self, t: &TableEntity) -> bool {
        if self.defining {
            self.report
                .skip("Tablo", "blok tanımında tablo olamaz; yazılmadı", 0);
            return false;
        }
        if let Some((_, why)) = t.shape().problem() {
            self.report.skip("Tablo", &format!("{why} Yazılmadı."), 0);
            return false;
        }
        // Its frame: the corner at the origin, unturned; the INSERT places it.
        let local = TableEntity {
            p: v(0.0, 0.0),
            rotation: 0.0,
            ..t.clone()
        };
        let shape = crate::blocks::core_table(&local);
        let Some(g) = table_geom(&shape) else {
            return false;
        };
        let record = self.handles.take();
        *self.anonymous += 1;
        let name = format!("*U{}", *self.anonymous);
        self.records.push((record, name.clone()));
        self.block_begin(record, &name);
        for [a, b] in g.lines() {
            self.block_line(record, app(a), app(b));
        }
        // Kalın çerçeve: its band's four strips filled, the outline other programs show.
        for strip in g.band_strips() {
            self.block_quad(record, strip.map(app));
        }
        let style = self.styles.text(&t.face).to_owned();
        let (xs, ys) = (edges(&t.columns), edges(&t.rows));
        let h = t.height;
        let mut changed = false;
        for (i, row) in t.cells.iter().enumerate() {
            for (j, words) in row.iter().enumerate() {
                if words.trim().is_empty() {
                    continue;
                }
                let (rows, cols) = match t.merge_at(i, j) {
                    Some(r) if (r.row as usize, r.col as usize) == (i, j) => {
                        (r.rows as usize, r.cols as usize)
                    }
                    Some(_) => continue,
                    None => (1, 1),
                };
                let (Some(&x0), Some(&x1), Some(&y0), Some(&y1)) =
                    (xs.get(j), xs.get(j + cols), ys.get(i), ys.get(i + rows))
                else {
                    continue;
                };
                let align = if t.header && i == 0 {
                    TableAlign::Center
                } else {
                    t.aligns
                        .as_ref()
                        .and_then(|a| a.get(j).copied())
                        .unwrap_or_default()
                };
                let (x, justify) = match align {
                    TableAlign::Left => (x0 + PAD * h, None),
                    TableAlign::Center => ((x0 + x1) / 2.0, Some(TextAlign::BaselineCenter)),
                    TableAlign::Right => (x1 - PAD * h, Some(TextAlign::BaselineRight)),
                };
                let at = g.point(x, (y0 + y1) / 2.0 + DROP * h);
                let (value, lost) = text_value(words);
                changed |= lost;
                self.block_text(
                    record,
                    &value,
                    &Justified {
                        p: app(at),
                        height: h,
                        rotation: 0.0,
                        align: justify,
                        width_factor: None,
                    },
                    &style,
                );
            }
        }
        self.block_end(record);
        if changed {
            self.report.note(
                "Tablo",
                "hücrelerdeki denetim karakterleri boşluk oldu (DXF yazısı tek satırdır)",
                0,
            );
        }
        self.begin("INSERT", &t.base);
        self.out.str(100, "AcDbBlockReference");
        self.out.str(2, &name);
        self.out.xyz(10, t.p);
        if t.rotation != 0.0 {
            self.out.real(50, t.rotation);
        }
        for p in crate::blocks::table_corners(t) {
            self.grow(p);
        }
        let mut meta = Self::base_meta(&t.base);
        meta.table = Some(table_json(t));
        // What `end` would keep at most: the table and the data that is not the label, attributes or symbol.
        let bare = Meta {
            label: None,
            attrs: Default::default(),
            symbol: None,
            ..meta.clone()
        };
        if xdata::size(&xdata::groups(&bare)) > xdata::MAX_BYTES {
            meta.table = None;
            self.report.note(
                "Tablo",
                "AutoCAD'in nesne başına genişletilmiş veri sınırını (16 KB) aştığı için tablo olarak değil, yalnız çizgi ve yazıları olarak yazıldı; KentOS'a blok olarak geri okunur",
                0,
            );
        }
        self.report.note(
            "Tablo",
            "DXF'te KentOS tablosu yok: çizgi ve yazılarından bir blok (*U) olarak yazıldı; başka programlar blok olarak gösterir, KentOS tablo olarak geri okur",
            0,
        );
        self.end(meta);
        true
    }

    /// A TEXT in a block (a table's cell, docs/adr/0184 §7): in the block's
    /// colour, justified as `j` says, in `style`.
    fn block_text(&mut self, record: u64, value: &str, j: &Justified, style: &str) {
        self.block_head("TEXT", record);
        let o = &mut *self.blocks;
        o.str(100, "AcDbText");
        let vertical = o.text(value, value, j);
        if style != "Standard" {
            o.str(7, style);
        }
        o.str(100, "AcDbText");
        if vertical != 0 {
            o.int(73, vertical);
        }
    }

    /// A dimension as a DXF DIMENSION: its drawing (the app's lines and
    /// ticks, the dimension arc as an ARC, the value as MTEXT) in an
    /// anonymous block of its own, the type and definition points a CAD
    /// program measures from, and Standard's sizes overridden to the app's
    /// (text height, oblique ticks, gaps, decimals), so a program that
    /// redraws it stays close to it. KentOS's data gives the same dimension
    /// back to KentOS.
    fn dimension(&mut self, d: &DimensionEntity) -> bool {
        if !(d.height > 0.0) {
            self.report
                .skip("Ölçü", "yazı yüksekliği sıfır ya da negatif; yazılmadı", 0);
            return false;
        }
        let Some(l) = dim::layout(d) else {
            self.report.skip(
                "Ölçü",
                "ölçülecek bir uzunluğu ya da açısı yok; yazılmadı",
                0,
            );
            return false;
        };
        let def = dim::definition(d, &l);
        let own = d.text.clone().filter(|t| !t.is_empty());
        let shown = own.clone().or_else(|| self.values.get(&d.base.id).cloned());
        if shown.is_none() && self.defining {
            // The app's values are the drawing's dimensions' (by id); a block's are in its own ids.
            self.report.note(
                "Ölçü",
                "blok içindeki ölçünün değeri bloğunda yazılmadı; başka programlar ölçüyü yeniden çizince ölçer, KentOS kendi ölçüsünü geri okur",
                0,
            );
        }
        let middle = dim::text_middle(&l, d.height);

        // Its lines' pens (docs/adr/0205 §6): the dimension line's (its arrowheads' too), the
        // extension lines', the value's; BYBLOCK where its look names none.
        let look = &d.look;
        let line_pen = self.dimension_pen(
            look.dim_line_color.as_deref(),
            look.dim_line_weight,
            look.dim_line_type,
        );
        let ext_pen = self.dimension_pen(
            look.ext_color.as_deref(),
            look.ext_weight,
            look.ext_line_type,
        );
        let text_pen = self.dimension_pen(look.text_color.as_deref(), None, None);
        // The block: the drawing on layer 0 in the dimension's colour (BYBLOCK), as AutoCAD writes it.
        let record = self.handles.take();
        *self.dimensions += 1;
        let name = format!("*D{}", *self.dimensions);
        self.records.push((record, name.clone()));
        self.block_begin(record, &name);
        let arc = match l.pick.first() {
            Some(&Edge::Arc { c, r, a0, sweep }) => Some((app(c), r, a0, sweep)),
            _ => None,
        };
        // The arc the layout draws as chords is one ARC.
        let on_arc = |p: CoreVec2| {
            arc.is_some_and(|(c, r, _, _)| {
                (hypot(p.x - c.x, p.y - c.y) - r).abs() <= 1e-9 * r.max(1.0)
            })
        };
        for (i, [p, q]) in l.lines.iter().enumerate() {
            self.grow(app(*p));
            self.grow(app(*q));
            if on_arc(*p) && on_arc(*q) {
                continue;
            }
            self.pen = if l.ext.contains(&i) {
                ext_pen
            } else {
                line_pen
            };
            self.block_line(record, app(*p), app(*q));
        }
        self.pen = line_pen;
        if let Some((c, r, a0, sweep)) = arc {
            self.block_arc(record, c, r, a0, a0 + sweep);
        }
        // A look's filled arrowheads (SOLID) and dots (a donut: two half arcs as wide as the radius).
        for ring in l.fills.iter().flatten() {
            if ring.len() == 3 {
                self.block_solid(record, [app(ring[0]), app(ring[1]), app(ring[2])]);
            } else if !ring.is_empty() {
                let n = ring.len() as f64;
                let c = v(
                    ring.iter().map(|p| p.x).sum::<f64>() / n,
                    ring.iter().map(|p| p.y).sum::<f64>() / n,
                );
                let r = hypot(ring[0].x - c.x, ring[0].y - c.y);
                self.block_dot(record, c, r);
            }
        }
        self.pen = text_pen;
        match shown.as_deref() {
            Some(t) if !t.trim().is_empty() => {
                let style = self.styles.value(d.look.font).to_owned();
                self.block_mtext(record, middle, d.height, l.rotation, t, d.mask, &style);
                self.grow(middle);
            }
            _ => self.report.note(
                "Ölçü",
                "değer yazısı verilmediği için bloğu yazısız yazıldı",
                0,
            ),
        }
        self.pen = Pen::default();
        self.block_end(record);

        // The DIMENSION (an ARC_DIMENSION, a LARGE_RADIAL_DIMENSION): its common groups, then its kind's.
        self.begin(def.entity, &d.base);
        self.out.str(100, "AcDbDimension");
        self.out.str(2, &name);
        self.out.xyz(10, def.p10);
        self.out.xyz(11, middle);
        let east = if def.east { dim::ORDINATE_EAST } else { 0 };
        self.out.int(70, def.kind | dim::OWN_BLOCK | east);
        self.out.int(71, 5);
        self.out.real(42, l.value);
        self.out
            .str(1, &own.as_deref().map(dim::mtext_value).unwrap_or_default());
        self.out
            .str(3, self.styles.dimension(d.look.dim_style.as_ref()));
        self.dimension_kind(&def);
        let jogged = d.style == Some(DimensionStyle::Jogged);
        // Its lines' types by their LTYPE records' handles (docs/adr/0205 §6).
        let types = [d.look.dim_line_type, d.look.ext_line_type]
            .map(|t| t.and_then(|t| ltype_handle(self.ltypes, self.handles, t)));
        self.out.xdata(&dim_overrides(
            d.height,
            self.decimals,
            self.grads,
            d.mask,
            jogged,
            &d.look,
            self.per_metre,
            types,
        ));
        if matches!(
            d.look.arrow,
            Some(DimensionArrow::Open | DimensionArrow::Dot)
        ) {
            self.report.note("Ölçü stili", super::styles::ARROW_NOTE, 0);
        }
        // Semt and Eğim have no DXF kind (docs/adr/0147 §8): an aligned one, drawn by its block.
        if matches!(
            d.style,
            Some(DimensionStyle::Azimuth | DimensionStyle::Slope)
        ) {
            self.report.note(
                "Ölçü",
                "semt ve eğim ölçüleri DXF'te hizalı ölçü olarak, kendi çizgileri ve değeriyle yazıldı; başka programlar çizgilerini gösterir, KentOS ölçü olarak geri okur",
                0,
            );
        }
        let mut m = Self::base_meta(&d.base);
        let slope = d.style == Some(DimensionStyle::Slope);
        m.dimension = Some(DimMeta {
            style: dim::style_name(d.style).unwrap_or("").to_string(),
            offset: d.offset,
            height: d.height,
            // Only when MTEXT's notation cannot say it exactly (the reader compares).
            text: own.filter(|t| dim::mtext_value(t) != *t),
            center: (def.entity == "DIMENSION" && def.kind == dim::DIAMETER)
                .then_some((d.a.x, d.a.y)),
            za: d.za.filter(|_| slope),
            zb: d.zb.filter(|_| slope),
        });
        // The value over the drawing's background (also DIMTFILL in its overrides).
        m.mask = d.mask;
        // Its style and look (docs/adr/0183 §7).
        if !d.look.is_plain() {
            m.look = serde_json::to_string(&d.look).ok();
        }
        self.end(m);
        true
    }

    /// The groups of a dimension's kind (its subclass and points).
    fn dimension_kind(&mut self, def: &Definition) {
        let point = |out: &mut Out, code: i32, p: Option<Vec2>| {
            if let Some(p) = p {
                out.xyz(code, p);
            }
        };
        match (def.entity, def.kind) {
            // The arc's ends and centre, their angles; not partial, no leader (docs/adr/0147 §8).
            ("ARC_DIMENSION", _) => {
                self.out.str(100, "AcDbArcDimension");
                point(self.out, 13, def.p13);
                point(self.out, 14, def.p14);
                point(self.out, 15, def.p15);
                self.out.int(70, 0);
                let (start, end) = def.angles.unwrap_or((0.0, 0.0));
                self.out.real(40, start);
                self.out.real(41, end);
                self.out.int(71, 0);
            }
            // The centre shown, the jog's middle, the point on the arc; the jog's angle in the overrides.
            ("LARGE_RADIAL_DIMENSION", _) => {
                self.out.str(100, "AcDbRadialDimensionLarge");
                point(self.out, 13, def.p13);
                point(self.out, 14, def.p14);
                point(self.out, 15, def.p15);
                self.out.real(40, 0.0);
            }
            (_, dim::ORDINATE) => {
                self.out.str(100, "AcDbOrdinateDimension");
                point(self.out, 13, def.p13);
                point(self.out, 14, def.p14);
            }
            (_, dim::ROTATED | dim::ALIGNED) => {
                self.out.str(100, "AcDbAlignedDimension");
                point(self.out, 13, def.p13);
                point(self.out, 14, def.p14);
                if def.kind == dim::ROTATED {
                    if let Some(a) = def.angle {
                        self.out.real(50, a);
                    }
                    self.out.str(100, "AcDbRotatedDimension");
                } else if let (Some(a), Some(b)) = (def.p13, def.p14) {
                    // AutoCAD takes an aligned dimension's direction from its points; ezdxf from group 50.
                    self.out.real(50, deg(atan2(b.y - a.y, b.x - a.x)));
                }
            }
            (_, dim::ANGULAR_3P) => {
                self.out.str(100, "AcDb3PointAngularDimension");
                point(self.out, 13, def.p13);
                point(self.out, 14, def.p14);
                point(self.out, 15, def.p15);
            }
            _ => {
                self.out.str(
                    100,
                    if def.kind == dim::RADIUS {
                        "AcDbRadialDimension"
                    } else {
                        "AcDbDiametricDimension"
                    },
                );
                point(self.out, 15, def.p15);
                self.out.real(40, def.leader.unwrap_or(0.0));
            }
        }
    }

    /// A block entity's head: on layer 0, owned by the block, colour BYBLOCK
    /// unless the pen has one (and its line type and weight, docs/adr/0205 §6).
    fn block_head(&mut self, kind: &str, record: u64) {
        let h = self.handles.take();
        let pen = self.pen;
        let o = &mut *self.blocks;
        o.str(0, kind);
        o.handle(5, h);
        o.handle(330, record);
        o.str(100, "AcDbEntity");
        o.str(8, "0");
        if let Some(name) = pen.ltype {
            o.str(6, name);
        }
        match pen.color {
            Some(c) => {
                o.int(62, i64::from(c.aci));
                if let Some(rgb) = c.rgb {
                    o.int(420, rgb);
                }
            }
            None => o.int(62, 0),
        }
        if let Some(w) = pen.weight {
            o.int(370, w);
        }
    }

    /// An anonymous block's BLOCK (a dimension's, a table's).
    fn block_begin(&mut self, record: u64, name: &str) {
        // 1: anonymous.
        self.block_begin_flags(record, name, 1);
    }

    fn block_begin_flags(&mut self, record: u64, name: &str, flags: i64) {
        let h = self.handles.take();
        let o = &mut *self.blocks;
        o.str(0, "BLOCK");
        o.handle(5, h);
        o.handle(330, record);
        o.str(100, "AcDbEntity");
        o.str(8, "0");
        o.str(100, "AcDbBlockBegin");
        o.str(2, name);
        o.int(70, flags);
        o.xyz(10, v(0.0, 0.0));
        o.str(3, name);
        o.str(1, "");
    }

    fn block_end(&mut self, record: u64) {
        let h = self.handles.take();
        let o = &mut *self.blocks;
        o.str(0, "ENDBLK");
        o.handle(5, h);
        o.handle(330, record);
        o.str(100, "AcDbEntity");
        o.str(8, "0");
        o.str(100, "AcDbBlockEnd");
    }

    fn block_line(&mut self, record: u64, a: Vec2, b: Vec2) {
        self.block_head("LINE", record);
        self.blocks.str(100, "AcDbLine");
        self.blocks.xyz(10, a);
        self.blocks.xyz(11, b);
    }

    /// A filled triangle (a dimension's filled arrowhead, docs/adr/0183 §3).
    fn block_solid(&mut self, record: u64, p: [Vec2; 3]) {
        self.block_head("SOLID", record);
        let o = &mut *self.blocks;
        o.str(100, "AcDbTrace");
        o.xyz(10, p[0]);
        o.xyz(11, p[1]);
        o.xyz(12, p[2]);
        o.xyz(13, p[2]);
    }

    /// A filled four-cornered strip (a table's frame band, docs/adr/0184 §7),
    /// its corners in order round it: SOLID runs 1 2 4 3.
    fn block_quad(&mut self, record: u64, p: [Vec2; 4]) {
        self.block_head("SOLID", record);
        let o = &mut *self.blocks;
        o.str(100, "AcDbTrace");
        o.xyz(10, p[0]);
        o.xyz(11, p[1]);
        o.xyz(12, p[3]);
        o.xyz(13, p[2]);
    }

    /// A filled disc of radius `r` about `c` (a dimension's dot): a closed
    /// polyline of two half circles of radius r/2, r wide (AutoCAD's DONUT).
    fn block_dot(&mut self, record: u64, c: Vec2, r: f64) {
        self.block_head("LWPOLYLINE", record);
        let o = &mut *self.blocks;
        o.str(100, "AcDbPolyline");
        o.int(90, 2);
        o.int(70, 1);
        o.real(43, r);
        o.real(10, c.x - r / 2.0);
        o.real(20, c.y);
        o.real(42, 1.0);
        o.real(10, c.x + r / 2.0);
        o.real(20, c.y);
        o.real(42, 1.0);
    }

    /// A circle of radius `r` about `c` (a blank dot's).
    fn block_circle(&mut self, record: u64, c: Vec2, r: f64) {
        self.block_head("CIRCLE", record);
        let o = &mut *self.blocks;
        o.str(100, "AcDbCircle");
        o.xyz(10, c);
        o.real(40, r);
    }

    /// A polyline through `pts`, closed back to its first when `closed`.
    fn block_polyline(&mut self, record: u64, pts: &[Vec2], closed: bool) {
        self.block_head("LWPOLYLINE", record);
        let o = &mut *self.blocks;
        o.str(100, "AcDbPolyline");
        o.int(90, pts.len() as i64);
        o.int(70, i64::from(closed));
        for p in pts {
            o.real(10, p.x);
            o.real(20, p.y);
        }
    }

    /// Counter-clockwise from a0 to a1 (radians).
    fn block_arc(&mut self, record: u64, c: Vec2, r: f64, a0: f64, a1: f64) {
        self.block_head("ARC", record);
        let o = &mut *self.blocks;
        o.str(100, "AcDbCircle");
        o.xyz(10, c);
        o.real(40, r);
        o.str(100, "AcDbArc");
        o.real(50, deg(norm_angle(a0)));
        o.real(51, deg(norm_angle(a1)));
    }

    /// The value, centred on `middle` along `rotation` (degrees); over the
    /// drawing's background when `mask` (Zemin, docs/adr/0147 §8).
    #[allow(clippy::too_many_arguments)]
    fn block_mtext(
        &mut self,
        record: u64,
        middle: Vec2,
        height: f64,
        rotation: f64,
        text: &str,
        mask: bool,
        style: &str,
    ) {
        self.block_head("MTEXT", record);
        let (s, c) = sin_cos_deg(rotation);
        let o = &mut *self.blocks;
        o.str(100, "AcDbMText");
        o.xyz(10, middle);
        o.real(40, height);
        o.real(41, 0.0);
        // Middle centre, left to right.
        o.int(71, 5);
        o.int(72, 1);
        mtext_chunks(o, &dim::mtext_value(text));
        // The value's typeface (docs/adr/0183 §7): a face record's, else Standard.
        o.str(7, style);
        o.xyz(11, v(c, s));
        if mask {
            // The drawing's background behind it, a tenth of its height round (the app's margin).
            o.int(90, 3);
            o.int(63, 256);
            o.real(45, 1.1);
            o.int(441, 0);
        }
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
        if h.assoc.is_some() {
            self.report.note(
                "Tarama",
                "ilişkili taramalar ilişkisiz yazıldı; DXF'te sınır nesnelerini izlemezler",
                0,
            );
        }
        let p = &h.pattern;
        let kind = p.kind;
        let fill = matches!(kind, HatchPatternType::Solid | HatchPatternType::Gradient);
        self.begin("HATCH", &h.base);
        self.out.str(100, "AcDbHatch");
        self.out.xyz(10, v(0.0, 0.0));
        self.out.real(210, 0.0);
        self.out.real(220, 0.0);
        self.out.real(230, 1.0);
        let name = match kind {
            HatchPatternType::Pattern => p.name.as_deref().unwrap_or("_USER"),
            HatchPatternType::Lines | HatchPatternType::Cross => "_USER",
            HatchPatternType::Solid | HatchPatternType::Gradient => "SOLID",
        };
        self.out.str(2, name);
        self.out.int(70, i64::from(fill));
        self.out.int(71, 0);
        self.out.int(91, 1 + holes.len() as i64);
        // Outer boundary (external polyline), then the islands (polylines).
        self.boundary(&h.ring, 3);
        for hole in &holes {
            self.boundary(hole, 2);
        }
        self.out.int(75, 0);
        // A pattern of the library is AutoCAD's predefined one of that name; another is custom.
        self.out.int(
            76,
            match kind {
                HatchPatternType::Lines | HatchPatternType::Cross => 0,
                HatchPatternType::Pattern if library_pattern(name).is_none() => 2,
                _ => 1,
            },
        );
        let mut m = Self::base_meta(&h.base);
        match kind {
            HatchPatternType::Lines | HatchPatternType::Cross => {
                let (angle, spacing) = (p.angle, p.spacing);
                let families: &[f64] = if kind == HatchPatternType::Cross {
                    &[0.0, 90.0]
                } else {
                    &[0.0]
                };
                self.out.real(52, angle);
                self.out.real(41, spacing);
                self.out.int(77, i64::from(families.len() == 2));
                self.out.int(78, families.len() as i64);
                // What the reader makes of it, to know whether the exact values must ride along.
                let mut read_back = (0.0, 1.0);
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
                if read_back != (angle, spacing) {
                    m.pattern = Some((angle, spacing));
                }
            }
            HatchPatternType::Pattern => {
                // DXF holds the families turned with the pattern and scaled (docs/adr/0186 §9).
                let (alpha, scale) = (p.angle, p.scale.unwrap_or(1.0));
                let lines = p.lines.as_deref().unwrap_or_default();
                self.out.real(52, alpha);
                self.out.real(41, scale);
                self.out.int(77, 0);
                self.out.int(78, lines.len() as i64);
                let (sa, ca) = sin_cos_deg(alpha);
                for l in lines {
                    let a = alpha + l.angle;
                    let (sl, cl) = sin_cos_deg(a);
                    let [ox, oy] = l.origin;
                    let [dx, dy] = l.offset;
                    self.out.real(53, a);
                    self.out.real(43, (ca * ox - sa * oy) * scale);
                    self.out.real(44, (sa * ox + ca * oy) * scale);
                    self.out.real(45, (cl * dx - sl * dy) * scale);
                    self.out.real(46, (sl * dx + cl * dy) * scale);
                    self.out.int(79, l.dashes.len() as i64);
                    for d in &l.dashes {
                        self.out.real(49, d * scale);
                    }
                }
                m.hatch = serde_json::to_string(p).ok();
            }
            // A solid fill has no lines; its angle and spacing ride along when they are not the reader's own.
            HatchPatternType::Solid => {
                if (p.angle, p.spacing) != (0.0, 1.0) {
                    m.pattern = Some((p.angle, p.spacing));
                }
            }
            HatchPatternType::Gradient => {}
        }
        self.out.int(98, 0);
        if let (HatchPatternType::Gradient, Some(g)) = (kind, &p.gradient) {
            self.gradient(&h.base, p.angle, g);
            m.hatch = serde_json::to_string(p).ok();
        }
        for p in &h.ring {
            self.grow(*p);
        }
        // The exact pattern goes first when KentOS's data would not fit.
        if m.hatch.is_some() && xdata::size(&xdata::groups(&m)) > xdata::MAX_BYTES {
            self.report.note(
                "Tarama",
                "bir desenin tam değerleri KentOS verisine sığmadı; KentOS'a DXF'teki değerleriyle geri okunur",
                0,
            );
            m.hatch = None;
        }
        self.end(m);
        true
    }

    /// A gradient's groups (docs/adr/0186 §9): its name (no inverted linear
    /// one: the linear turned half round), angle in radians, two colours,
    /// the first the hatch's own (its layer's when it has none).
    fn gradient(&mut self, base: &EntityBase, angle: f64, g: &HatchGradient) {
        let (name, angle) = match (g.shape, g.inverted) {
            (GradientShape::Linear, false) => ("LINEAR", angle),
            (GradientShape::Linear, true) => ("LINEAR", angle + 180.0),
            (GradientShape::Cylinder, false) => ("CYLINDER", angle),
            (GradientShape::Cylinder, true) => ("INVCYLINDER", angle),
            (GradientShape::Spherical, false) => ("SPHERICAL", angle),
            (GradientShape::Spherical, true) => ("INVSPHERICAL", angle),
        };
        let first = match &base.color {
            Some(c) => aci::from_app(c).0,
            None => self
                .layers
                .colour_of(&base.layer_id)
                .unwrap_or(aci::DxfColor { aci: 7, rgb: None }),
        };
        let second = aci::from_app(&g.color2).0;
        self.out.int(450, 1);
        self.out.int(451, 0);
        self.out.real(460, rad(angle.rem_euclid(360.0)));
        self.out.real(461, 0.0);
        self.out.int(452, 0);
        self.out.real(462, 1.0);
        self.out.int(453, 2);
        for (k, c) in [first, second].into_iter().enumerate() {
            let [r, gr, b] = aci::rgb(c.aci);
            let own = (i64::from(r) << 16) | (i64::from(gr) << 8) | i64::from(b);
            self.out.real(463, k as f64);
            self.out.int(63, i64::from(c.aci));
            self.out.int(421, c.rgb.unwrap_or(own));
        }
        self.out.str(470, name);
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

/// An MTEXT's text: 250-byte pieces in group 3, the rest in group 1 (never splitting a character or a caret pair).
fn mtext_chunks(o: &mut Out, text: &str) {
    let mut rest = text;
    while rest.len() > 250 {
        let mut cut = 250;
        while !rest.is_char_boundary(cut) || rest[..cut].ends_with('^') {
            cut -= 1;
        }
        o.str(3, &rest[..cut]);
        rest = &rest[cut..];
    }
    o.str(1, rest);
}

/// Standard's sizes overridden to the app's for one dimension (AutoCAD's
/// DSTYLE data): text height, oblique ticks, the extension lines' gap and
/// overshoot, the text above the line with the app's gap, aligned with it,
/// and the project's decimals and angle unit, for a program that redraws it;
/// a jogged radius's 45° jog, and the value's fill when it has Zemin
/// (docs/adr/0147 §8).
#[allow(clippy::too_many_arguments)]
fn dim_overrides(
    height: f64,
    decimals: u32,
    grads: bool,
    mask: bool,
    jogged: bool,
    look: &kentos_contracts::DimensionLook,
    per_metre: f64,
    types: [Option<u64>; 2],
) -> Vec<(i32, String)> {
    let mut g: Vec<(i32, String)> = vec![
        (1001, "ACAD".into()),
        (1000, "DSTYLE".into()),
        (1002, "{".into()),
    ];
    let mut real = |code: &str, x: f64| {
        g.push((1070, code.into()));
        g.push((1040, dxf_real(x)));
    };
    // Its look's sizes (docs/adr/0183 §7); a plain look's are the ones dimensions always had.
    real("140", height);
    let size = look.arrow_size_or_default() * height;
    match look.arrow {
        // Oblique ticks: DIMTSZ is half a 45° tick's length along the line.
        None => real("142", size * std::f64::consts::FRAC_1_SQRT_2),
        // Arrowheads (DIMBLK's filled arrow): their size; none, 0 (AutoCAD then draws none).
        Some(arrow) => {
            real("142", 0.0);
            real(
                "41",
                if arrow == DimensionArrow::None {
                    0.0
                } else {
                    size
                },
            );
        }
    }
    real("42", look.ext_offset_or_default() * height);
    real("44", look.ext_beyond_or_default() * height);
    real("147", look.text_gap_or_default() * height);
    // A jogged radius's jog: 45° (DIMJOGANG, radians; docs/adr/0147 §2).
    if jogged {
        real("50", std::f64::consts::FRAC_PI_4);
    }
    // Zemin: the value over the drawing's background (DIMTFILL 1).
    if mask {
        g.push((1070, "69".into()));
        g.push((1070, "1".into()));
    }
    let centre = look.text_place == Some(kentos_contracts::DimensionTextPlace::Centre);
    for (code, value) in [
        ("77", if centre { 0 } else { 1 }),
        ("73", 0),
        ("74", 0),
        ("271", i64::from(look.decimals.unwrap_or(decimals).min(8))),
        ("275", if grads { 2 } else { 0 }),
        ("179", 4),
    ] {
        g.push((1070, code.into()));
        g.push((1070, value.to_string()));
    }
    // The value's prefix and suffix around the number (DIMPOST), its unit (DIMLFAC).
    if let Some(post) = super::styles::dimpost(look.prefix.as_deref(), look.suffix.as_deref()) {
        g.push((1070, "3".into()));
        g.push((1000, post));
    }
    if let Some(f) = super::styles::dimlfac(look.unit, per_metre) {
        g.push((1070, "144".into()));
        g.push((1040, dxf_real(f)));
    }
    // Its lines (docs/adr/0205 §6): colours as ACI numbers (DIMCLRD, DIMCLRE, DIMCLRT), weights
    // (DIMLWD, DIMLWE), types by their LTYPE records (DIMLTYPE, DIMLTEX1, DIMLTEX2); only those it names.
    for (code, color) in [
        ("176", &look.dim_line_color),
        ("177", &look.ext_color),
        ("178", &look.text_color),
    ] {
        if let Some(c) = color {
            g.push((1070, code.into()));
            g.push((1070, aci::from_app(c).0.aci.to_string()));
        }
    }
    for (code, weight) in [("371", look.dim_line_weight), ("372", look.ext_weight)] {
        if let Some(w) = weight {
            g.push((1070, code.into()));
            g.push((1070, super::layers::line_weight(w).0.to_string()));
        }
    }
    let [line, ext] = types;
    for (code, h) in [("345", line), ("346", ext), ("347", ext)] {
        if let Some(h) = h {
            g.push((1070, code.into()));
            g.push((1005, format!("{h:X}")));
        }
    }
    g.push((1002, "}".into()));
    g
}

/// A text's face as its KENTOS data writes it (docs/adr/0183 §7); none for a styleless, faceless text.
fn face_json(face: &kentos_contracts::TextFace) -> Option<String> {
    (!face.is_plain())
        .then(|| serde_json::to_string(face).ok())
        .flatten()
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
