//! Document schema 2 written from the contract (docs/specs/kcad-v2.md §6) in
//! KentOS CBOR profile 1. The writer keeps the rules the reader enforces, so it
//! never writes a file a reader refuses: finite floats, the length and depth
//! limits, unique non-nil ids, no `null` renderer, no holes on a polyline;
//! otherwise it stops with the place and the reason.
//!
//! Keys are written in their encoded order (RFC 8949 §4.2.1): by hand in the
//! maps whose keys are fixed, sorted per object for the objects, whose fields
//! depend on the kind; the reader checks that order, and the fixtures hold the
//! bytes the independent Python writer gives.

use std::collections::{BTreeMap, HashSet};

use kentos_contracts::{
    AngleUnit, AreaUnit, DOCUMENT_FORMAT, DOCUMENT_VERSION, DOCUMENT_VERSION_2, DimensionStyle,
    DocumentSnapshotV2, DrawingFont, Entity, EntityId, HatchPattern, HatchPatternType, LabelInk,
    LabelPlacement, LabelStyle, LayerNode, LayerNodeType, LayerStyle, LineType, MigrationSource,
    PointSymbol, ProjectSettings, RingGeometry, Vec2, Workspace,
};
use serde_json::Value;

use crate::cbor::{MAX_DEPTH, MAX_ITEMS, MAX_STRING, Seg, Writer, key_order, render};
use crate::error::{Code, KcadError};

/// The payload of a drawing.
pub(crate) fn payload(doc: &DocumentSnapshotV2) -> Result<Vec<u8>, KcadError> {
    let mut e = Encoder {
        w: Writer::default(),
        path: Vec::with_capacity(16),
        depth: 0,
    };
    e.root(doc)?;
    Ok(e.w.out)
}

/// One field of an object, before the fields are sorted by key.
enum Val<'d> {
    Text(&'d str),
    Name(&'static str),
    Float(f64),
    Bool(bool),
    Uid(&'d EntityId),
    Point(&'d Vec2),
    Points(&'d [Vec2]),
    Floats(&'d [f64]),
    Attrs(&'d BTreeMap<String, String>),
    Rings(&'d [RingGeometry]),
    Loops(&'d [Vec<Vec2>]),
    Pattern(&'d HatchPattern),
}

struct Encoder<'d> {
    w: Writer,
    path: Vec<Seg<'d>>,
    depth: usize,
}

impl<'d> Encoder<'d> {
    fn fail(&self, code: Code, what: &str) -> KcadError {
        KcadError::unwritable(code, &render(&self.path), what)
    }

    fn at<T>(
        &mut self,
        seg: Seg<'d>,
        write: impl FnOnce(&mut Self) -> Result<T, KcadError>,
    ) -> Result<T, KcadError> {
        self.path.push(seg);
        let out = write(self)?;
        self.path.pop();
        Ok(out)
    }

    fn text(&mut self, s: &str) -> Result<(), KcadError> {
        if s.len() as u64 > MAX_STRING {
            return Err(self.fail(
                Code::TooLong,
                &format!("metin {} bayt, en çok {MAX_STRING} olabilir", s.len()),
            ));
        }
        self.w.text(s);
        Ok(())
    }

    fn float(&mut self, x: f64) -> Result<(), KcadError> {
        if !x.is_finite() {
            return Err(self.fail(
                Code::NonFinite,
                "sayı NaN ya da sonsuz; yalnız sonlu sayılar yazılır",
            ));
        }
        self.w.float(x);
        Ok(())
    }

    /// Opens an array or a map of `n` entries (depth and length limits).
    fn open(&mut self, n: usize, map: bool) -> Result<(), KcadError> {
        if self.depth + 1 > MAX_DEPTH {
            return Err(self.fail(
                Code::TooDeep,
                &format!("iç içe derinlik {MAX_DEPTH}'ı aşıyor"),
            ));
        }
        if n as u64 > MAX_ITEMS {
            return Err(self.fail(
                Code::TooLong,
                &format!("{n} öğe, en çok {MAX_ITEMS} olabilir"),
            ));
        }
        if map {
            self.w.map(n);
        } else {
            self.w.array(n);
        }
        self.depth += 1;
        Ok(())
    }

    fn close(&mut self) {
        self.depth -= 1;
    }

    /// A key of a map whose keys are fixed; the caller writes them in order.
    fn key(&mut self, k: &'static str) {
        self.w.text(k);
    }

    fn point(&mut self, p: &Vec2) -> Result<(), KcadError> {
        self.open(2, false)?;
        self.float(p.x)?;
        self.float(p.y)?;
        self.close();
        Ok(())
    }

    fn points(&mut self, list: &'d [Vec2]) -> Result<(), KcadError> {
        self.open(list.len(), false)?;
        for (i, p) in list.iter().enumerate() {
            self.at(Seg::Index(i), |e| e.point(p))?;
        }
        self.close();
        Ok(())
    }

    fn floats(&mut self, list: &'d [f64]) -> Result<(), KcadError> {
        self.open(list.len(), false)?;
        for (i, x) in list.iter().enumerate() {
            self.at(Seg::Index(i), |e| e.float(*x))?;
        }
        self.close();
        Ok(())
    }

    fn id(&mut self, id: &[u8; 16]) -> Result<(), KcadError> {
        if *id == [0; 16] {
            return Err(self.fail(Code::BadValue, "kimlik boş (nil) olamaz"));
        }
        self.w.bytes(id);
        Ok(())
    }

    // ── The root and the document ───────────────────────────────────────

    fn root(&mut self, doc: &'d DocumentSnapshotV2) -> Result<(), KcadError> {
        if doc.format != DOCUMENT_FORMAT || doc.version != DOCUMENT_VERSION_2 {
            return Err(self.fail(
                Code::BadValue,
                &format!(
                    "çizim {} sürüm {}; KCAD 2 yalnız {DOCUMENT_FORMAT} sürüm {DOCUMENT_VERSION_2} taşır",
                    doc.format, doc.version
                ),
            ));
        }
        self.open(3, true)?;
        self.key("format");
        self.w.text(DOCUMENT_FORMAT);
        self.key("version");
        self.w.uint(u64::from(DOCUMENT_VERSION_2));
        self.key("document");
        self.at(Seg::Name("document"), |e| e.body(doc))?;
        self.close();
        Ok(())
    }

    fn body(&mut self, doc: &'d DocumentSnapshotV2) -> Result<(), KcadError> {
        if doc.uids.len() != doc.entities.len() {
            return Err(self.fail(
                Code::BadValue,
                &format!(
                    "{} nesne ama {} kalıcı kimlik var; her nesnenin bir kimliği olmalı",
                    doc.entities.len(),
                    doc.uids.len()
                ),
            ));
        }
        let optional = usize::from(doc.home_view.is_some())
            + usize::from(doc.project_id.is_some())
            + usize::from(doc.migrated_from.is_some());
        self.open(7 + optional, true)?;
        // name (4), layers origin styles (6), entities homeView settings (8), projectId (9), activeLayer (11), migratedFrom (12).
        self.key("name");
        self.at(Seg::Name("name"), |e| e.text(&doc.name))?;
        self.key("layers");
        self.at(Seg::Name("layers"), |e| e.layers(&doc.layers))?;
        self.key("origin");
        self.at(Seg::Name("origin"), |e| e.point(&doc.origin))?;
        self.key("styles");
        self.at(Seg::Name("styles"), |e| {
            e.open(2, true)?;
            e.key("items");
            e.at(Seg::Name("items"), |e| e.opaque_list(&doc.styles.items))?;
            e.key("categories");
            e.at(Seg::Name("categories"), |e| {
                e.opaque_list(&doc.styles.categories)
            })?;
            e.close();
            Ok(())
        })?;
        self.key("entities");
        self.at(Seg::Name("entities"), |e| e.objects(doc))?;
        if let Some(b) = &doc.home_view {
            self.key("homeView");
            self.at(Seg::Name("homeView"), |e| {
                e.open(4, false)?;
                for x in [b.min_x, b.min_y, b.max_x, b.max_y] {
                    e.float(x)?;
                }
                e.close();
                Ok(())
            })?;
        }
        self.key("settings");
        self.at(Seg::Name("settings"), |e| e.settings(&doc.settings))?;
        if let Some(id) = &doc.project_id {
            self.key("projectId");
            self.at(Seg::Name("projectId"), |e| e.id(&id.0))?;
        }
        self.key("activeLayer");
        self.at(Seg::Name("activeLayer"), |e| e.text(&doc.active_layer))?;
        if let Some(source) = &doc.migrated_from {
            self.key("migratedFrom");
            self.at(Seg::Name("migratedFrom"), |e| e.source(source))?;
        }
        self.close();
        Ok(())
    }

    fn settings(&mut self, s: &ProjectSettings) -> Result<(), KcadError> {
        let n = 6 + usize::from(s.workspace.is_some()) + usize::from(s.drawing_font.is_some());
        self.open(n, true)?;
        self.key("srid");
        self.w.uint(u64::from(s.srid));
        self.key("areaUnit");
        self.w.text(area_unit(s.area_unit));
        self.key("angleUnit");
        self.w.text(angle_unit(s.angle_unit));
        self.key("plotScale");
        self.at(Seg::Name("plotScale"), |e| e.float(s.plot_scale))?;
        if let Some(w) = s.workspace {
            self.key("workspace");
            self.w.text(workspace(w));
        }
        if let Some(f) = s.drawing_font {
            self.key("drawingFont");
            self.w.text(drawing_font(f));
        }
        self.key("areaDecimals");
        self.w.uint(u64::from(s.area_decimals));
        self.key("lengthDecimals");
        self.w.uint(u64::from(s.length_decimals));
        self.close();
        Ok(())
    }

    fn source(&mut self, s: &MigrationSource) -> Result<(), KcadError> {
        let digest = parse_hex32(&s.source_sha256);
        let Some(digest) =
            digest.filter(|_| s.format == DOCUMENT_FORMAT && s.version == DOCUMENT_VERSION)
        else {
            return Err(self.fail(
                Code::BadValue,
                "göç kaynağı kentos.document sürüm 1 ve 64 küçük onaltılık haneli bir SHA-256 olmalı",
            ));
        };
        self.open(3, true)?;
        self.key("format");
        self.w.text(DOCUMENT_FORMAT);
        self.key("version");
        self.w.uint(u64::from(DOCUMENT_VERSION));
        self.key("sourceSha256");
        self.w.bytes(&digest);
        self.close();
        Ok(())
    }

    // ── The layer tree ──────────────────────────────────────────────────

    fn layers(&mut self, nodes: &'d [LayerNode]) -> Result<(), KcadError> {
        self.open(nodes.len(), false)?;
        for (i, n) in nodes.iter().enumerate() {
            self.at(Seg::Index(i), |e| e.layer(n))?;
        }
        self.close();
        Ok(())
    }

    fn layer(&mut self, n: &'d LayerNode) -> Result<(), KcadError> {
        self.open(8, true)?;
        // id (2), name type (4), style (5), locked (6), visible (7), children expanded (8).
        self.key("id");
        self.at(Seg::Name("id"), |e| e.text(&n.id))?;
        self.key("name");
        self.at(Seg::Name("name"), |e| e.text(&n.name))?;
        self.key("type");
        self.w.text(match n.kind {
            LayerNodeType::Group => "group",
            LayerNodeType::Layer => "layer",
        });
        self.key("style");
        self.at(Seg::Name("style"), |e| e.layer_style(&n.style))?;
        self.key("locked");
        self.w.bool(n.locked);
        self.key("visible");
        self.w.bool(n.visible);
        self.key("children");
        self.at(Seg::Name("children"), |e| e.layers(&n.children))?;
        self.key("expanded");
        self.w.bool(n.expanded);
        self.close();
        Ok(())
    }

    fn layer_style(&mut self, s: &'d LayerStyle) -> Result<(), KcadError> {
        if matches!(s.renderer, Some(Value::Null)) {
            return Err(self.fail(
                Code::BadValue,
                "çizici null olamaz; çizici yoksa alan boş bırakılır",
            ));
        }
        let n = 3
            + usize::from(s.fill.is_some())
            + usize::from(s.label.is_some())
            + usize::from(s.point.is_some())
            + usize::from(s.renderer.is_some())
            + usize::from(s.pick_interior.is_some());
        self.open(n, true)?;
        // fill (4), color label point (5), lineType renderer (8), lineWeight (10), pickInterior (12).
        if let Some(fill) = &s.fill {
            self.key("fill");
            self.at(Seg::Name("fill"), |e| e.text(fill))?;
        }
        self.key("color");
        self.at(Seg::Name("color"), |e| e.text(&s.color))?;
        if let Some(label) = &s.label {
            self.key("label");
            self.at(Seg::Name("label"), |e| e.label_style(label))?;
        }
        if let Some(p) = &s.point {
            self.key("point");
            self.at(Seg::Name("point"), |e| {
                e.open(2, true)?;
                e.key("size");
                e.at(Seg::Name("size"), |e| e.float(p.size))?;
                e.key("symbol");
                e.w.text(point_symbol(p.symbol));
                e.close();
                Ok(())
            })?;
        }
        self.key("lineType");
        self.w.text(line_type(s.line_type));
        if let Some(r) = &s.renderer {
            self.key("renderer");
            self.at(Seg::Name("renderer"), |e| e.opaque(r))?;
        }
        self.key("lineWeight");
        self.at(Seg::Name("lineWeight"), |e| e.float(s.line_weight))?;
        if let Some(pick) = s.pick_interior {
            self.key("pickInterior");
            self.w.bool(pick);
        }
        self.close();
        Ok(())
    }

    fn label_style(&mut self, l: &'d LabelStyle) -> Result<(), KcadError> {
        let n = 2 + [
            l.ink.is_some(),
            l.grow.is_some(),
            l.weight.is_some(),
            l.max_size.is_some(),
            l.max_scale.is_some(),
            l.min_scale.is_some(),
            l.template.is_some(),
            l.min_feature_px.is_some(),
        ]
        .iter()
        .filter(|&&b| b)
        .count();
        self.open(n, true)?;
        // ink (3), grow size (4), weight (6), maxSize (7), maxScale minScale template (8), placement (9), minFeaturePx (12).
        if let Some(ink) = l.ink {
            self.key("ink");
            self.w.text(label_ink(ink));
        }
        let floats: [(&'static str, Option<f64>); 2] = [("grow", l.grow), ("size", Some(l.size))];
        for (k, v) in floats {
            if let Some(x) = v {
                self.key(k);
                self.at(Seg::Name(k), |e| e.float(x))?;
            }
        }
        if let Some(w) = l.weight {
            self.key("weight");
            self.w.uint(u64::from(w));
        }
        for (k, v) in [
            ("maxSize", l.max_size),
            ("maxScale", l.max_scale),
            ("minScale", l.min_scale),
        ] {
            if let Some(x) = v {
                self.key(k);
                self.at(Seg::Name(k), |e| e.float(x))?;
            }
        }
        if let Some(t) = &l.template {
            self.key("template");
            self.at(Seg::Name("template"), |e| e.text(t))?;
        }
        self.key("placement");
        self.w.text(label_placement(l.placement));
        if let Some(x) = l.min_feature_px {
            self.key("minFeaturePx");
            self.at(Seg::Name("minFeaturePx"), |e| e.float(x))?;
        }
        self.close();
        Ok(())
    }

    // ── Objects ─────────────────────────────────────────────────────────

    fn objects(&mut self, doc: &'d DocumentSnapshotV2) -> Result<(), KcadError> {
        self.open(doc.entities.len(), false)?;
        let mut seen = HashSet::with_capacity(doc.uids.len());
        let mut fields = Vec::with_capacity(16);
        for (i, (entity, uid)) in doc.entities.iter().zip(&doc.uids).enumerate() {
            self.path.push(Seg::Index(i));
            if !seen.insert(uid.0) {
                return Err(self.fail(
                    Code::DuplicateUid,
                    &format!("kalıcı kimlik {uid} iki nesnede var"),
                ));
            }
            self.object(entity, uid, &mut fields)?;
            self.path.pop();
        }
        self.close();
        Ok(())
    }

    fn object(
        &mut self,
        entity: &'d Entity,
        uid: &'d EntityId,
        f: &mut Vec<(&'static str, Val<'d>)>,
    ) -> Result<(), KcadError> {
        let kind = entity.kind();
        let base = entity.base();
        f.clear();
        f.push(("uid", Val::Uid(uid)));
        f.push(("attrs", Val::Attrs(&base.attrs)));
        f.push(("layerId", Val::Text(&base.layer_id)));
        if let Some(c) = &base.color {
            f.push(("color", Val::Text(c)));
        }
        if let Some(l) = &base.label {
            f.push(("label", Val::Text(l)));
        }
        if let Some(s) = &base.symbol {
            f.push(("symbol", Val::Text(s)));
        }
        match entity {
            Entity::Point(e) => {
                f.push(("p", Val::Point(&e.p)));
                if let Some(z) = e.z {
                    f.push(("z", Val::Float(z)));
                }
            }
            Entity::Line(e) => {
                f.push(("a", Val::Point(&e.a)));
                f.push(("b", Val::Point(&e.b)));
            }
            Entity::Polyline(e) | Entity::Polygon(e) => {
                f.push(("pts", Val::Points(&e.pts)));
                if let Some(b) = &e.bulges {
                    f.push(("bulges", Val::Floats(b)));
                }
                if let Some(h) = &e.holes {
                    if matches!(entity, Entity::Polyline(_)) {
                        self.path.push(Seg::Name(kind));
                        return Err(self.fail(
                            Code::BadValue,
                            "çoklu çizginin adası (deliği) olamaz; yalnız kapalı alanın olur",
                        ));
                    }
                    f.push(("holes", Val::Rings(h)));
                }
            }
            Entity::Circle(e) => {
                f.push(("c", Val::Point(&e.c)));
                f.push(("r", Val::Float(e.r)));
            }
            Entity::Arc(e) => {
                f.push(("c", Val::Point(&e.c)));
                f.push(("r", Val::Float(e.r)));
                f.push(("a0", Val::Float(e.a0)));
                f.push(("a1", Val::Float(e.a1)));
            }
            Entity::Ellipse(e) => {
                f.push(("c", Val::Point(&e.c)));
                f.push(("major", Val::Point(&e.major)));
                f.push(("ratio", Val::Float(e.ratio)));
                f.push(("t0", Val::Float(e.t0)));
                f.push(("t1", Val::Float(e.t1)));
            }
            Entity::Spline(e) => {
                f.push(("pts", Val::Points(&e.pts)));
                f.push(("closed", Val::Bool(e.closed)));
            }
            Entity::Xline(e) | Entity::Ray(e) => {
                f.push(("p", Val::Point(&e.p)));
                f.push(("dir", Val::Point(&e.dir)));
            }
            Entity::Text(e) => {
                f.push(("p", Val::Point(&e.p)));
                f.push(("text", Val::Text(&e.text)));
                f.push(("height", Val::Float(e.height)));
                f.push(("rotation", Val::Float(e.rotation)));
            }
            Entity::Dimension(e) => {
                f.push(("a", Val::Point(&e.a)));
                f.push(("b", Val::Point(&e.b)));
                f.push(("offset", Val::Float(e.offset)));
                f.push(("height", Val::Float(e.height)));
                if let Some(t) = &e.text {
                    f.push(("text", Val::Text(t)));
                }
                if let Some(s) = e.style {
                    f.push(("style", Val::Name(dimension_style(s))));
                }
                if let Some(a) = e.angle {
                    f.push(("angle", Val::Float(a)));
                }
                if let Some(c) = &e.c {
                    f.push(("c", Val::Point(c)));
                }
            }
            Entity::Hatch(e) => {
                f.push(("ring", Val::Points(&e.ring)));
                if let Some(h) = &e.holes {
                    f.push(("holes", Val::Loops(h)));
                }
                f.push(("pattern", Val::Pattern(&e.pattern)));
            }
        }
        f.sort_by(|a, b| key_order(a.0, b.0));
        self.open(1, true)?;
        self.key(kind);
        self.path.push(Seg::Name(kind));
        self.open(f.len(), true)?;
        for (k, v) in f.drain(..) {
            self.key(k);
            self.path.push(Seg::Name(k));
            self.val(v)?;
            self.path.pop();
        }
        self.close();
        self.path.pop();
        self.close();
        Ok(())
    }

    fn val(&mut self, v: Val<'d>) -> Result<(), KcadError> {
        match v {
            Val::Text(t) => self.text(t),
            Val::Name(t) => {
                self.w.text(t);
                Ok(())
            }
            Val::Float(x) => self.float(x),
            Val::Bool(b) => {
                self.w.bool(b);
                Ok(())
            }
            Val::Uid(id) => self.id(&id.0),
            Val::Point(p) => self.point(p),
            Val::Points(list) => self.points(list),
            Val::Floats(list) => self.floats(list),
            Val::Attrs(attrs) => {
                let mut pairs: Vec<(&String, &String)> = attrs.iter().collect();
                pairs.sort_by(|a, b| key_order(a.0, b.0));
                self.open(pairs.len(), true)?;
                for (k, v) in pairs {
                    self.text(k)?;
                    self.at(Seg::Key(k), |e| e.text(v))?;
                }
                self.close();
                Ok(())
            }
            Val::Rings(rings) => {
                self.open(rings.len(), false)?;
                for (i, ring) in rings.iter().enumerate() {
                    self.at(Seg::Index(i), |e| {
                        e.open(1 + usize::from(ring.bulges.is_some()), true)?;
                        e.key("pts");
                        e.at(Seg::Name("pts"), |e| e.points(&ring.pts))?;
                        if let Some(b) = &ring.bulges {
                            e.key("bulges");
                            e.at(Seg::Name("bulges"), |e| e.floats(b))?;
                        }
                        e.close();
                        Ok(())
                    })?;
                }
                self.close();
                Ok(())
            }
            Val::Loops(loops) => {
                self.open(loops.len(), false)?;
                for (i, list) in loops.iter().enumerate() {
                    self.at(Seg::Index(i), |e| e.points(list))?;
                }
                self.close();
                Ok(())
            }
            Val::Pattern(p) => {
                self.open(3, true)?;
                // type (4), angle (5), spacing (7).
                self.key("type");
                self.w.text(hatch_pattern(p.kind));
                self.key("angle");
                self.at(Seg::Name("angle"), |e| e.float(p.angle))?;
                self.key("spacing");
                self.at(Seg::Name("spacing"), |e| e.float(p.spacing))?;
                self.close();
                Ok(())
            }
        }
    }

    // ── Opaque parts (§6.7) ─────────────────────────────────────────────

    fn opaque_list(&mut self, list: &'d [Value]) -> Result<(), KcadError> {
        self.open(list.len(), false)?;
        for (i, v) in list.iter().enumerate() {
            self.at(Seg::Index(i), |e| e.opaque(v))?;
        }
        self.close();
        Ok(())
    }

    fn opaque(&mut self, v: &'d Value) -> Result<(), KcadError> {
        match v {
            Value::Null => self.w.null(),
            Value::Bool(b) => self.w.bool(*b),
            Value::Number(n) => {
                if let Some(u) = n.as_u64() {
                    self.w.uint(u);
                } else if let Some(i) = n.as_i64() {
                    self.w.int(i);
                } else if let Some(x) = n.as_f64() {
                    self.float(x)?;
                } else {
                    return Err(self.fail(Code::BadValue, "sayı okunamadı"));
                }
            }
            Value::String(s) => self.text(s)?,
            Value::Array(list) => self.opaque_list(list)?,
            Value::Object(map) => {
                let mut pairs: Vec<(&'d String, &'d Value)> = map.iter().collect();
                pairs.sort_by(|a, b| key_order(a.0, b.0));
                self.open(pairs.len(), true)?;
                for (k, v) in pairs {
                    self.text(k)?;
                    self.at(Seg::Key(k), |e| e.opaque(v))?;
                }
                self.close();
            }
        }
        Ok(())
    }
}

/// 32 bytes from 64 lowercase hexadecimal digits.
fn parse_hex32(text: &str) -> Option<[u8; 32]> {
    let raw = text.as_bytes();
    if raw.len() != 64 {
        return None;
    }
    let digit = |c: u8| match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        _ => None,
    };
    let mut out = [0u8; 32];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = (digit(raw[2 * i])? << 4) | digit(raw[2 * i + 1])?;
    }
    Some(out)
}

// The enumerations as the contract's serde names them (the test below holds them equal).

pub(crate) fn area_unit(u: AreaUnit) -> &'static str {
    match u {
        AreaUnit::M2 => "m2",
        AreaUnit::Donum => "donum",
        AreaUnit::Ha => "ha",
    }
}

pub(crate) fn angle_unit(u: AngleUnit) -> &'static str {
    match u {
        AngleUnit::Grad => "grad",
        AngleUnit::Deg => "deg",
    }
}

pub(crate) fn workspace(w: Workspace) -> &'static str {
    match w {
        Workspace::Hybrid => "hybrid",
        Workspace::Cad => "cad",
        Workspace::Gis => "gis",
        Workspace::Plan3d => "plan3d",
        Workspace::Disaster => "disaster",
    }
}

pub(crate) fn drawing_font(f: DrawingFont) -> &'static str {
    match f {
        DrawingFont::Barlow => "barlow",
        DrawingFont::Arimo => "arimo",
        DrawingFont::Overpass => "overpass",
        DrawingFont::Quicksand => "quicksand",
        DrawingFont::ArchitectsDaughter => "architects-daughter",
        DrawingFont::CourierPrime => "courier-prime",
        DrawingFont::PlexMono => "plex-mono",
    }
}

pub(crate) fn line_type(t: LineType) -> &'static str {
    match t {
        LineType::Continuous => "continuous",
        LineType::Dashed => "dashed",
        LineType::Dashdot => "dashdot",
        LineType::Dotted => "dotted",
    }
}

pub(crate) fn point_symbol(s: PointSymbol) -> &'static str {
    match s {
        PointSymbol::Ring => "ring",
        PointSymbol::Cross => "cross",
        PointSymbol::Triangle => "triangle",
    }
}

pub(crate) fn label_ink(i: LabelInk) -> &'static str {
    match i {
        LabelInk::Fg => "fg",
        LabelInk::FgDim => "fg-dim",
        LabelInk::Label => "label",
    }
}

pub(crate) fn label_placement(p: LabelPlacement) -> &'static str {
    match p {
        LabelPlacement::Center => "center",
        LabelPlacement::Corner => "corner",
        LabelPlacement::Beside => "beside",
        LabelPlacement::Along => "along",
    }
}

pub(crate) fn dimension_style(s: DimensionStyle) -> &'static str {
    match s {
        DimensionStyle::Aligned => "aligned",
        DimensionStyle::Linear => "linear",
        DimensionStyle::Angular => "angular",
        DimensionStyle::Radius => "radius",
        DimensionStyle::Diameter => "diameter",
    }
}

pub(crate) fn hatch_pattern(t: HatchPatternType) -> &'static str {
    match t {
        HatchPatternType::Solid => "solid",
        HatchPatternType::Lines => "lines",
        HatchPatternType::Cross => "cross",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    fn serde_name<T: Serialize>(v: T) -> String {
        match serde_json::to_value(v) {
            Ok(Value::String(s)) => s,
            other => panic!("not a string: {other:?}"),
        }
    }

    #[test]
    fn enumerations_are_written_as_the_contract_names_them() {
        for u in [AreaUnit::M2, AreaUnit::Donum, AreaUnit::Ha] {
            assert_eq!(area_unit(u), serde_name(u));
        }
        for u in [AngleUnit::Grad, AngleUnit::Deg] {
            assert_eq!(angle_unit(u), serde_name(u));
        }
        for w in [
            Workspace::Hybrid,
            Workspace::Cad,
            Workspace::Gis,
            Workspace::Plan3d,
            Workspace::Disaster,
        ] {
            assert_eq!(workspace(w), serde_name(w));
        }
        for f in [
            DrawingFont::Barlow,
            DrawingFont::Arimo,
            DrawingFont::Overpass,
            DrawingFont::Quicksand,
            DrawingFont::ArchitectsDaughter,
            DrawingFont::CourierPrime,
            DrawingFont::PlexMono,
        ] {
            assert_eq!(drawing_font(f), serde_name(f));
        }
        for t in [
            LineType::Continuous,
            LineType::Dashed,
            LineType::Dashdot,
            LineType::Dotted,
        ] {
            assert_eq!(line_type(t), serde_name(t));
        }
        for s in [PointSymbol::Ring, PointSymbol::Cross, PointSymbol::Triangle] {
            assert_eq!(point_symbol(s), serde_name(s));
        }
        for i in [LabelInk::Fg, LabelInk::FgDim, LabelInk::Label] {
            assert_eq!(label_ink(i), serde_name(i));
        }
        for p in [
            LabelPlacement::Center,
            LabelPlacement::Corner,
            LabelPlacement::Beside,
            LabelPlacement::Along,
        ] {
            assert_eq!(label_placement(p), serde_name(p));
        }
        for s in [
            DimensionStyle::Aligned,
            DimensionStyle::Linear,
            DimensionStyle::Angular,
            DimensionStyle::Radius,
            DimensionStyle::Diameter,
        ] {
            assert_eq!(dimension_style(s), serde_name(s));
        }
        for t in [
            HatchPatternType::Solid,
            HatchPatternType::Lines,
            HatchPatternType::Cross,
        ] {
            assert_eq!(hatch_pattern(t), serde_name(t));
        }
        for k in [LayerNodeType::Group, LayerNodeType::Layer] {
            let written = match k {
                LayerNodeType::Group => "group",
                LayerNodeType::Layer => "layer",
            };
            assert_eq!(written, serde_name(k));
        }
    }

    #[test]
    fn the_fixed_keys_are_in_encoded_order() {
        // The maps whose keys the writer writes by hand, in the order it writes them.
        let maps: [&[&str]; 9] = [
            &["format", "version", "document"],
            &[
                "name",
                "layers",
                "origin",
                "styles",
                "entities",
                "homeView",
                "settings",
                "projectId",
                "activeLayer",
                "migratedFrom",
            ],
            &[
                "srid",
                "areaUnit",
                "angleUnit",
                "plotScale",
                "workspace",
                "drawingFont",
                "areaDecimals",
                "lengthDecimals",
            ],
            &["format", "version", "sourceSha256"],
            &[
                "id", "name", "type", "style", "locked", "visible", "children", "expanded",
            ],
            &[
                "fill",
                "color",
                "label",
                "point",
                "lineType",
                "renderer",
                "lineWeight",
                "pickInterior",
            ],
            &[
                "ink",
                "grow",
                "size",
                "weight",
                "maxSize",
                "maxScale",
                "minScale",
                "template",
                "placement",
                "minFeaturePx",
            ],
            &["items", "categories"],
            &["type", "angle", "spacing"],
        ];
        for keys in maps {
            assert!(
                keys.windows(2).all(|w| key_order(w[0], w[1]).is_lt()),
                "{keys:?}"
            );
        }
        assert!(key_order("size", "symbol").is_lt());
        assert!(key_order("pts", "bulges").is_lt());
    }
}
