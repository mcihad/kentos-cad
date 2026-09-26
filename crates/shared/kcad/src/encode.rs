//! Document schema 2 written from the contract (docs/specs/kcad-v2.md §6) in
//! KentOS CBOR profile 1. The writer keeps the rules the reader enforces, so it
//! never writes a file a reader refuses: finite floats, the length and depth
//! limits, unique non-nil ids, no `null` renderer, no holes on a polyline;
//! otherwise it stops with the place and the reason.
//!
//! Keys are written in their encoded order (RFC 8949 §4.2.1): by hand in the
//! maps whose keys are fixed, sorted per object for the objects, whose fields
//! depend on the kind; the reader checks that order, and the fixtures hold the
//! bytes the independent Python writer gives. The objects are written in
//! `objects.rs`, the enumerations' names are in `names.rs`.

mod names;
mod objects;

use kentos_contracts::{
    DOCUMENT_FORMAT, DOCUMENT_VERSION, DOCUMENT_VERSION_2, DocumentSnapshotV2, LabelStyle,
    LayerNode, LayerNodeType, LayerStyle, MigrationSource, ProjectSettings, Vec2,
};
use serde_json::Value;

use crate::cbor::{MAX_DEPTH, MAX_ITEMS, MAX_STRING, Seg, Writer, key_order, render};
use crate::error::{Code, KcadError};
use names::{
    angle_unit, area_unit, drawing_font, label_ink, label_placement, line_type, point_symbol,
    workspace,
};

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

#[cfg(test)]
mod tests {
    use super::*;

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
