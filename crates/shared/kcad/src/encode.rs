//! Document schemas 2 to 6 written from the contract (docs/specs/kcad-v2.md
//! §6) in KentOS CBOR profile 1: the oldest schema that holds what the drawing
//! has (`schema_of`). The writer keeps the rules the reader enforces, so it
//! never writes a file a reader refuses: finite floats, the length and depth
//! limits, unique non-nil ids, no `null` renderer, no holes on a polyline,
//! the block rules (`kentos_contracts::blocks`); otherwise it stops with the
//! place and the reason.
//!
//! Keys are written in their encoded order (RFC 8949 §4.2.1): by hand in the
//! maps whose keys are fixed, sorted per object for the objects, whose fields
//! depend on the kind; the reader checks that order, and the fixtures hold the
//! bytes the independent Python writer gives. The objects are written in
//! `objects.rs`, the block definitions in `blocks.rs`, the enumerations'
//! names in `names.rs`.

mod blocks;
mod crs;
mod names;
mod objects;

use kentos_contracts::{
    DOCUMENT_FORMAT, DOCUMENT_VERSION, DOCUMENT_VERSION_2, DimensionStyle, DocumentSnapshotV2,
    Entity, LabelStyle, LayerNode, LayerNodeType, LayerSnap, LayerState, LayerStateNode,
    LayerStyle, MigrationSource, ProjectSettings, SurveySettings, Vec2, layer_states_problem,
};
use serde_json::Value;

use crate::cbor::{MAX_DEPTH, MAX_ITEMS, MAX_STRING, Seg, Writer, key_order, render};
use crate::error::{Code, KcadError};
use crate::watch::{Step, Watch, report};
use crate::{
    SCHEMA_WITH_BLOCKS, SCHEMA_WITH_CUSTOM_CRS, SCHEMA_WITH_DIMENSIONS, SCHEMA_WITH_DRAWING_UNIT,
    SCHEMA_WITH_ELEVATIONS, SCHEMA_WITH_GROUND, SCHEMA_WITH_LAYER_SNAP, SCHEMA_WITH_LAYER_STATES,
    SCHEMA_WITH_LEADERS, SCHEMA_WITH_LINE_PARTS, SCHEMA_WITH_LINE_WEIGHTS,
    SCHEMA_WITH_LINKED_TEXTS, SCHEMA_WITH_PARAGRAPHS, SCHEMA_WITH_PARTS, SCHEMA_WITH_SECOND_SRID,
    SCHEMA_WITH_SURVEY, SCHEMA_WITH_TEXT_EXTRAS, SCHEMA_WITH_TRAVERSE_TOLERANCES,
};
use names::{
    angle_unit, area_unit, drawing_font, drawing_unit, label_ink, label_placement, line_type,
    point_symbol, workspace,
};

/// The payload of a drawing; `watch` hears the objects as they are written
/// and may stop the writing (docs/adr/0030).
pub(crate) fn payload(
    doc: &DocumentSnapshotV2,
    watch: &mut dyn Watch,
) -> Result<Vec<u8>, KcadError> {
    let mut e = Encoder {
        w: Writer::default(),
        path: Vec::with_capacity(16),
        depth: 0,
        watch,
    };
    e.root(doc)?;
    Ok(e.w.out)
}

struct Encoder<'d> {
    w: Writer,
    path: Vec<Seg<'d>>,
    depth: usize,
    watch: &'d mut dyn Watch,
}

impl<'d> Encoder<'d> {
    fn fail(&self, code: Code, what: &str) -> KcadError {
        KcadError::unwritable(code, &render(&self.path), what)
    }

    /// Tells the watcher how far the writing is; stops when it says so.
    fn report(&mut self, step: Step<'_>) -> Result<(), KcadError> {
        report(self.watch, step)
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
        let schema = schema_of(doc);
        self.open(3, true)?;
        self.key("format");
        self.w.text(DOCUMENT_FORMAT);
        self.key("version");
        self.w.uint(u64::from(schema));
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
        if let Err(fault) = kentos_contracts::blocks::check(&doc.blocks, &doc.entities) {
            let said = crate::blocks::said(&fault, &doc.blocks);
            self.path.push(Seg::Name(said.list));
            self.path.extend(said.path);
            return Err(self.fail(said.code, &said.what));
        }
        let optional = usize::from(!doc.blocks.is_empty())
            + usize::from(doc.home_view.is_some())
            + usize::from(doc.project_id.is_some())
            + usize::from(doc.migrated_from.is_some());
        self.open(7 + optional, true)?;
        // name (4), blocks layers origin styles (6), entities homeView settings (8), projectId (9), activeLayer (11), migratedFrom (12).
        self.key("name");
        self.at(Seg::Name("name"), |e| e.text(&doc.name))?;
        if !doc.blocks.is_empty() {
            self.key("blocks");
            self.at(Seg::Name("blocks"), |e| e.blocks(&doc.blocks))?;
        }
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

    fn settings(&mut self, s: &'d ProjectSettings) -> Result<(), KcadError> {
        let n = 6
            + usize::from(s.workspace.is_some())
            + usize::from(s.drawing_font.is_some())
            + usize::from(s.drawing_unit.is_some())
            + usize::from(s.second_srid.is_some())
            + usize::from(s.custom_crs.is_some())
            + usize::from(s.second_custom_crs.is_some())
            + usize::from(!s.datum_transforms.is_empty())
            + usize::from(s.survey.is_some())
            + usize::from(!s.layer_states.is_empty());
        self.open(n, true)?;
        self.key("srid");
        self.w.uint(u64::from(s.srid));
        if let Some(v) = &s.survey {
            self.key("survey");
            self.at(Seg::Name("survey"), |e| e.survey(v))?;
        }
        self.key("areaUnit");
        self.w.text(area_unit(s.area_unit));
        self.key("angleUnit");
        self.w.text(angle_unit(s.angle_unit));
        if let Some(d) = &s.custom_crs {
            self.key("customCrs");
            self.at(Seg::Name("customCrs"), |e| {
                if s.srid != 0 {
                    return Err(e.fail(
                        Code::BadValue,
                        "projenin kendi tanımı yalnız EPSG kodu olmayan (srid 0) projede olur",
                    ));
                }
                e.crs_definition(d)
            })?;
        }
        self.key("plotScale");
        self.at(Seg::Name("plotScale"), |e| e.float(s.plot_scale))?;
        if let Some(w) = s.workspace {
            self.key("workspace");
            self.w.text(workspace(w));
        }
        if let Some(second) = s.second_srid {
            self.key("secondSrid");
            self.at(Seg::Name("secondSrid"), |e| {
                if s.second() != Some(second) {
                    return Err(e.fail(
                        Code::BadValue,
                        "ikinci koordinat sistemi projeninkinden başka bir sistem olmalı; yerel projenin ikinci sistemi olmaz",
                    ));
                }
                e.w.uint(u64::from(second));
                Ok(())
            })?;
        }
        if let Some(f) = s.drawing_font {
            self.key("drawingFont");
            self.w.text(drawing_font(f));
        }
        if let Some(u) = s.drawing_unit {
            self.key("drawingUnit");
            self.w.text(drawing_unit(u));
        }
        if !s.layer_states.is_empty() {
            self.key("layerStates");
            self.at(Seg::Name("layerStates"), |e| {
                e.layer_states(&s.layer_states)
            })?;
        }
        self.key("areaDecimals");
        self.w.uint(u64::from(s.area_decimals));
        self.key("lengthDecimals");
        self.w.uint(u64::from(s.length_decimals));
        if !s.datum_transforms.is_empty() {
            self.key("datumTransforms");
            self.at(Seg::Name("datumTransforms"), |e| {
                e.datum_transforms(&s.datum_transforms)
            })?;
        }
        if let Some(d) = &s.second_custom_crs {
            self.key("secondCustomCrs");
            self.at(Seg::Name("secondCustomCrs"), |e| {
                if s.second_srid.is_some() || !s.has_system() {
                    return Err(e.fail(
                        Code::BadValue,
                        "ikinci sistem ya EPSG kodu ya tanımdır; koordinat sistemi olmayan projenin ikinci sistemi olmaz",
                    ));
                }
                e.crs_definition(d)
            })?;
        }
        self.close();
        Ok(())
    }

    /// The layer states (docs/adr/0177 §4), checked whole as a reader checks
    /// them: `id`, `name`, `nodes`; a node's `node`, `style`, `locked`, `visible`.
    fn layer_states(&mut self, states: &'d [LayerState]) -> Result<(), KcadError> {
        if let Some(problem) = layer_states_problem(states) {
            return Err(self.fail(Code::BadValue, &problem));
        }
        self.open(states.len(), false)?;
        for (i, state) in states.iter().enumerate() {
            self.at(Seg::Index(i), |e| {
                e.open(3, true)?;
                e.key("id");
                e.text(&state.id)?;
                e.key("name");
                e.text(&state.name)?;
                e.key("nodes");
                e.at(Seg::Name("nodes"), |e| {
                    e.open(state.nodes.len(), false)?;
                    for (j, node) in state.nodes.iter().enumerate() {
                        e.at(Seg::Index(j), |e| e.layer_state_node(node))?;
                    }
                    e.close();
                    Ok(())
                })?;
                e.close();
                Ok(())
            })?;
        }
        self.close();
        Ok(())
    }

    fn layer_state_node(&mut self, n: &'d LayerStateNode) -> Result<(), KcadError> {
        self.open(
            2 + usize::from(n.style.is_some()) + usize::from(n.locked.is_some()),
            true,
        )?;
        self.key("node");
        self.text(&n.node)?;
        if let Some(style) = &n.style {
            self.key("style");
            self.at(Seg::Name("style"), |e| e.layer_style(style))?;
        }
        if let Some(locked) = n.locked {
            self.key("locked");
            self.w.bool(locked);
        }
        self.key("visible");
        self.w.bool(n.visible);
        self.close();
        Ok(())
    }

    /// The survey settings (docs/adr/0169 §3), checked whole as a reader checks them.
    fn survey(&mut self, v: &SurveySettings) -> Result<(), KcadError> {
        if let Some(problem) = v.problem() {
            return Err(self.fail(Code::BadValue, &problem));
        }
        // Floats and the one bool, in the canonical order of their keys.
        let fields = [
            ("index", v.index.map(Ok)),
            ("faceHz", v.face_hz.map(Ok)),
            ("twoWay", v.two_way.map(Ok)),
            ("faceSlope", v.face_slope.map(Ok)),
            ("refraction", v.refraction.map(Ok)),
            ("groundHeight", v.ground_height.map(Ok)),
            ("reduceToGrid", v.reduce_to_grid.map(Err)),
            ("traverseAngle", v.traverse_angle.map(Ok)),
            ("traverseCoord", v.traverse_coord.map(Ok)),
        ];
        self.open(fields.iter().filter(|(_, x)| x.is_some()).count(), true)?;
        for (key, x) in fields {
            match x {
                Some(Ok(x)) => {
                    self.key(key);
                    self.at(Seg::Name(key), |e| e.float(x))?;
                }
                Some(Err(b)) => {
                    self.key(key);
                    self.w.bool(b);
                }
                None => {}
            }
        }
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
        self.open(if n.snap.is_some() { 9 } else { 8 }, true)?;
        // id (2), name snap type (4), style (5), locked (6), visible (7), children expanded (8).
        self.key("id");
        self.at(Seg::Name("id"), |e| e.text(&n.id))?;
        self.key("name");
        self.at(Seg::Name("name"), |e| e.text(&n.name))?;
        if let Some(snap) = &n.snap {
            self.key("snap");
            self.at(Seg::Name("snap"), |e| e.layer_snap(n.kind, snap))?;
        }
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

    /// A layer's own snapping (docs/adr/0163 §4): `off` or `kinds`, on a layer only.
    fn layer_snap(&mut self, kind: LayerNodeType, snap: &'d LayerSnap) -> Result<(), KcadError> {
        if kind == LayerNodeType::Group {
            return Err(self.fail(
                Code::BadValue,
                "grubun keneti yazılmaz; kenet yalnız katmanındır",
            ));
        }
        if let Some(problem) = snap.problem() {
            return Err(self.fail(Code::BadValue, &problem));
        }
        self.open(1, true)?;
        match &snap.kinds {
            None => {
                self.key("off");
                self.w.bool(true);
            }
            Some(kinds) => {
                self.key("kinds");
                self.open(kinds.len(), false)?;
                for (i, k) in kinds.iter().enumerate() {
                    self.at(Seg::Index(i), |e| e.text(k))?;
                }
                self.close();
            }
        }
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

/// The oldest schema that holds the drawing: 20 when a text of it or of a
/// block definition has a box, a line spacing or letter formats
/// (docs/adr/0182 §1), 19 when the project has layer
/// states (docs/adr/0177 §4), 18 when a text of it writes an
/// object's label (`labelOf`, `labelScale`), 17 when a polyline or a point,
/// of it or of a block definition, has parts, 16 when its survey settings
/// name the ground, 15 when they name a traverse tolerance, 14 when the project has survey settings, 13 when it has its own systems or datum choices, 12 when it has
/// a second system, 11 when it names a drawing unit, 10 when a layer has its own
/// snapping, 9 when a dimension of it or of
/// a block definition has one of schema 9's kinds or fields, 8 when it or a
/// block definition has a leader, 7 when a text or an attribute definition has an
/// alignment, a width factor or a mask, 6 when it has block definitions, 5
/// when an area has parts, 4 when an object has a vertex elevation, 3 when
/// one has its own line weight, else 2. A drawing without any stays as it
/// always was, byte for byte. (An insert needs a definition: one without is
/// refused before the schema is written.)
fn schema_of(doc: &DocumentSnapshotV2) -> u32 {
    fn snaps(nodes: &[LayerNode]) -> bool {
        nodes.iter().any(|n| n.snap.is_some() || snaps(&n.children))
    }
    let line_parts = |list: &[Entity]| {
        list.iter().any(|e| match e {
            Entity::Polyline(p) => p.parts.is_some(),
            Entity::Point(p) => p.parts.is_some(),
            _ => false,
        })
    };
    let paragraphs = |list: &[Entity]| {
        list.iter()
            .any(|e| matches!(e, Entity::Text(t) if !t.paragraph.is_plain()))
    };
    if paragraphs(&doc.entities) || doc.blocks.iter().any(|b| paragraphs(&b.entities)) {
        return SCHEMA_WITH_PARAGRAPHS;
    }
    if !doc.settings.layer_states.is_empty() {
        return SCHEMA_WITH_LAYER_STATES;
    }
    // A block definition's texts have no link: only the drawing's (docs/adr/0175 §4).
    if doc
        .entities
        .iter()
        .any(|e| matches!(e, Entity::Text(t) if t.label_of.is_some() || t.label_scale.is_some()))
    {
        return SCHEMA_WITH_LINKED_TEXTS;
    }
    if line_parts(&doc.entities) || doc.blocks.iter().any(|b| line_parts(&b.entities)) {
        return SCHEMA_WITH_LINE_PARTS;
    }
    let s = &doc.settings;
    if s.survey.as_ref().is_some_and(SurveySettings::has_ground) {
        return SCHEMA_WITH_GROUND;
    }
    if s.survey.as_ref().is_some_and(SurveySettings::has_traverse) {
        return SCHEMA_WITH_TRAVERSE_TOLERANCES;
    }
    if s.survey.is_some() {
        return SCHEMA_WITH_SURVEY;
    }
    if s.custom_crs.is_some() || s.second_custom_crs.is_some() || !s.datum_transforms.is_empty() {
        return SCHEMA_WITH_CUSTOM_CRS;
    }
    if doc.settings.second_srid.is_some() {
        return SCHEMA_WITH_SECOND_SRID;
    }
    if doc.settings.drawing_unit.is_some() {
        return SCHEMA_WITH_DRAWING_UNIT;
    }
    if snaps(&doc.layers) {
        return SCHEMA_WITH_LAYER_SNAP;
    }
    let dimensions = |list: &[Entity]| {
        list.iter().any(|e| {
            matches!(e, Entity::Dimension(d) if d.mask
                || d.za.is_some()
                || d.zb.is_some()
                || d.style.is_some_and(DimensionStyle::is_schema_9))
        })
    };
    if dimensions(&doc.entities) || doc.blocks.iter().any(|b| dimensions(&b.entities)) {
        return SCHEMA_WITH_DIMENSIONS;
    }
    let leaders = |list: &[Entity]| list.iter().any(|e| matches!(e, Entity::Leader(_)));
    if leaders(&doc.entities) || doc.blocks.iter().any(|b| leaders(&b.entities)) {
        return SCHEMA_WITH_LEADERS;
    }
    let extras = |list: &[Entity]| {
        list.iter().any(|e| {
            matches!(e, Entity::Text(t) if t.align.is_some() || t.width_factor.is_some() || t.mask)
        })
    };
    if extras(&doc.entities)
        || doc.blocks.iter().any(|b| {
            extras(&b.entities)
                || b.attributes
                    .iter()
                    .any(|a| a.align.is_some() || a.width_factor.is_some())
        })
    {
        return SCHEMA_WITH_TEXT_EXTRAS;
    }
    if !doc.blocks.is_empty() {
        return SCHEMA_WITH_BLOCKS;
    }
    let mut schema = DOCUMENT_VERSION_2;
    for e in &doc.entities {
        if matches!(e, Entity::Polygon(p) if p.parts.is_some()) {
            // The newest: nothing later in the drawing can change it.
            return SCHEMA_WITH_PARTS;
        }
        if has_elevation(e) {
            schema = SCHEMA_WITH_ELEVATIONS;
        } else if e.base().line_weight.is_some() {
            schema = schema.max(SCHEMA_WITH_LINE_WEIGHTS);
        }
    }
    schema
}

/// Whether an object has a vertex elevation (a line's end, a path's or a
/// hole's `zs`, even one whose every vertex is without): schema 4's fields.
fn has_elevation(e: &Entity) -> bool {
    match e {
        Entity::Line(l) => l.za.is_some() || l.zb.is_some(),
        Entity::Polyline(p) | Entity::Polygon(p) => {
            p.zs.is_some()
                || p.holes
                    .as_ref()
                    .is_some_and(|holes| holes.iter().any(|h| h.zs.is_some()))
        }
        _ => false,
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
        let maps: [&[&str]; 29] = [
            &["format", "version", "document"],
            &[
                "name",
                "blocks",
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
                "survey",
                "areaUnit",
                "angleUnit",
                "customCrs",
                "plotScale",
                "workspace",
                "secondSrid",
                "drawingFont",
                "drawingUnit",
                "layerStates",
                "areaDecimals",
                "lengthDecimals",
                "datumTransforms",
                "secondCustomCrs",
            ],
            &["format", "version", "sourceSha256"],
            // A layer state and its node (docs/adr/0177 §4).
            &["id", "name", "nodes"],
            &["node", "style", "locked", "visible"],
            &[
                "id", "name", "snap", "type", "style", "locked", "visible", "children", "expanded",
            ],
            &["off", "kinds"],
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
            // A polygon's hole (docs/adr/0142).
            &["zs", "pts", "bulges"],
            // A polygon's part (docs/adr/0143).
            &["zs", "pts", "holes", "bulges"],
            // A block definition and an attribute definition (docs/adr/0144).
            &[
                "id",
                "base",
                "name",
                "entities",
                "attributes",
                "description",
            ],
            &["p", "tag", "value", "height", "prompt", "rotation"],
            // The project's own systems and datum choices (docs/adr/0168).
            &["name", "system"],
            &[
                "kind",
                "datum",
                "customDatum",
                "scaleFactor",
                "falseEasting",
                "falseNorthing",
                "centralMeridian",
                "latitudeOfOrigin",
            ],
            &["kind", "datum", "customDatum"],
            &["base", "kind", "plane"],
            &["srid", "definition"],
            &["name", "toWgs84", "ellipsoid"],
            &["name", "semiMajor", "inverseFlattening"],
            &["scale", "accuracy", "rotation", "convention", "translation"],
            &["east", "kind", "north", "scale", "rotation"],
            &["a", "b", "c", "d", "e", "f", "kind"],
            &["to", "from", "grid", "name", "helmert"],
            &["id", "file", "size", "accuracy"],
            // The survey settings (docs/adr/0169 §3).
            &[
                "index",
                "faceHz",
                "twoWay",
                "faceSlope",
                "refraction",
                "groundHeight",
                "reduceToGrid",
                "traverseAngle",
                "traverseCoord",
            ],
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
