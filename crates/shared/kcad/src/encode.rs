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

mod addresses;
mod arrays;
mod blocks;
mod compose;
mod crs;
mod filter;
mod info;
mod labels;
mod names;
mod networks;
mod objects;
mod requests;
mod services;
mod sheets;
mod styles;
mod temporal;
mod ucs;
mod variables;
mod views;

use kentos_contracts::{
    AnnotationHeights, AnnotationKind, DOCUMENT_FORMAT, DOCUMENT_VERSION, DOCUMENT_VERSION_2,
    DimensionStyle, DimensionStyleDef, DocumentSnapshotV2, Entity, LayerField, LayerNode,
    LayerNodeType, LayerSnap, LayerState, LayerStateNode, LayerStyle, LengthStyle, MigrationSource,
    HeightKind, HeightReference, ProjectSettings, SurveySettings, TopologySettings, Vec2, layer_fields_problem,
    layer_states_problem,
};
use serde_json::Value;

use crate::cbor::{MAX_DEPTH, MAX_ITEMS, MAX_STRING, Seg, Writer, key_order, render};
use crate::error::{Code, KcadError};
use crate::watch::{Step, Watch, report};
use crate::{
    SCHEMA_WITH_ADDRESSES, SCHEMA_WITH_ANNOTATION, SCHEMA_WITH_ARRAYS, SCHEMA_WITH_ATTACHMENTS,
    SCHEMA_WITH_BLOCKS, SCHEMA_WITH_COMPOSE, SCHEMA_WITH_CUSTOM_CRS, SCHEMA_WITH_DIMENSION_REFS,
    SCHEMA_WITH_DIMENSIONS, SCHEMA_WITH_DRAWING_UNIT, SCHEMA_WITH_ELEVATIONS, SCHEMA_WITH_FILTERS,
    SCHEMA_WITH_GROUND, SCHEMA_WITH_HATCH_PATTERNS, SCHEMA_WITH_IMAGES, SCHEMA_WITH_IMPERIAL,
    SCHEMA_WITH_LABELS, SCHEMA_WITH_LAYER_FIELDS, SCHEMA_WITH_LAYER_SNAP, SCHEMA_WITH_LAYER_STATES,
    SCHEMA_WITH_LEADERS, SCHEMA_WITH_LINE_PARTS, SCHEMA_WITH_LINE_WEIGHTS,
    SCHEMA_WITH_LINKED_TEXTS, SCHEMA_WITH_LOOK, SCHEMA_WITH_MEASURES, SCHEMA_WITH_MULTIDIM,
    SCHEMA_WITH_NETWORKS, SCHEMA_WITH_NURBS, SCHEMA_WITH_PARAGRAPHS, SCHEMA_WITH_PARTS,
    SCHEMA_WITH_POINT_CLOUDS, SCHEMA_WITH_POLYLINE_EXTRAS, SCHEMA_WITH_RASTERS,
    SCHEMA_WITH_REQUESTS, SCHEMA_WITH_SECOND_SRID, SCHEMA_WITH_SERVICES, SCHEMA_WITH_SHEETS,
    SCHEMA_WITH_STYLES, SCHEMA_WITH_SUBREF_EXTRAS, SCHEMA_WITH_SURVEY, SCHEMA_WITH_SURVEY_SIGMAS,
    SCHEMA_WITH_TABLES, SCHEMA_WITH_TEMPORAL, SCHEMA_WITH_TEXT_EXTRAS, SCHEMA_WITH_TEXT_PATHS,
    SCHEMA_WITH_TOPOLOGY, SCHEMA_WITH_TRAVERSE_TOLERANCES, SCHEMA_WITH_UCS, SCHEMA_WITH_VARIABLES,
    SCHEMA_WITH_VIEWS,
};
use names::{
    angle_unit, area_unit, drawing_font, drawing_unit, length_style, line_type, point_symbol,
    workspace,
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
        // Schema 32's links (docs/adr/0208 §2), as a reader checks them.
        if let Some(fault) =
            kentos_contracts::service_links(&doc.layers, &doc.settings.connections, &doc.entities)
        {
            match fault {
                kentos_contracts::ServiceLinkFault::Object { index } => {
                    self.path.push(Seg::Name("entities"));
                    self.path.push(Seg::Index(index));
                }
                _ => self.path.push(Seg::Name("layers")),
            }
            return Err(self.fail(Code::BadValue, &fault.words()));
        }
        // Schema 34's tree rules (docs/adr/0210 §2), as a reader checks them.
        if let Some(problem) = kentos_contracts::scenarios_problem(&doc.layers) {
            self.path.push(Seg::Name("layers"));
            return Err(self.fail(Code::BadValue, &problem));
        }
        // Schema 35's tree rule (docs/adr/0211 §2), as a reader checks it.
        if let Some(problem) = kentos_contracts::filters_problem(&doc.layers) {
            self.path.push(Seg::Name("layers"));
            return Err(self.fail(Code::BadValue, &problem));
        }
        // Schema 36's tree rule (docs/adr/0212 §2), as a reader checks it.
        if let Some(problem) = kentos_contracts::labels_problem(&doc.layers) {
            self.path.push(Seg::Name("layers"));
            return Err(self.fail(Code::BadValue, &problem));
        }
        // Schema 40's tree rule (docs/adr/0225 §4, §5), as a reader checks it.
        if let Some(problem) = kentos_contracts::infos_problem(&doc.layers) {
            self.path.push(Seg::Name("layers"));
            return Err(self.fail(Code::BadValue, &problem));
        }
        let optional = usize::from(!doc.arrays.is_empty())
            + usize::from(!doc.blocks.is_empty())
            + usize::from(doc.home_view.is_some())
            + usize::from(doc.project_id.is_some())
            + usize::from(doc.migrated_from.is_some())
            + usize::from(doc.sheets.is_some())
            + usize::from(!doc.requests.is_empty())
            + usize::from(!doc.constraints.is_empty());
        self.open(7 + optional, true)?;
        // name (4), arrays blocks layers origin sheets styles (6), entities homeView requests settings (8),
        // projectId (9), activeLayer constraints (11), migratedFrom (12).
        self.key("name");
        self.at(Seg::Name("name"), |e| e.text(&doc.name))?;
        if !doc.arrays.is_empty() {
            self.key("arrays");
            self.at(Seg::Name("arrays"), |e| e.arrays(&doc.arrays, &doc.blocks))?;
        }
        if !doc.blocks.is_empty() {
            self.key("blocks");
            self.at(Seg::Name("blocks"), |e| e.blocks(&doc.blocks))?;
        }
        self.key("layers");
        self.at(Seg::Name("layers"), |e| e.layers(&doc.layers))?;
        self.key("origin");
        self.at(Seg::Name("origin"), |e| e.point(&doc.origin))?;
        if let Some(sheets) = &doc.sheets {
            self.key("sheets");
            self.at(Seg::Name("sheets"), |e| e.sheets(sheets))?;
        }
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
        if !doc.requests.is_empty() {
            self.key("requests");
            self.at(Seg::Name("requests"), |e| e.requests(&doc.requests))?;
        }
        self.key("settings");
        self.at(Seg::Name("settings"), |e| e.settings(&doc.settings))?;
        if let Some(id) = &doc.project_id {
            self.key("projectId");
            self.at(Seg::Name("projectId"), |e| e.id(&id.0))?;
        }
        self.key("activeLayer");
        self.at(Seg::Name("activeLayer"), |e| e.text(&doc.active_layer))?;
        // Schema 73 (docs/adr/0334): the constraints, written whole and checked against the objects.
        if !doc.constraints.is_empty() {
            self.key("constraints");
            self.at(Seg::Name("constraints"), |e| {
                let uids: std::collections::HashSet<kentos_contracts::EntityId> = doc.uids.iter().copied().collect();
                if let Some((at, problem)) = kentos_contracts::constraints_problem(&doc.constraints, Some(&uids)) {
                    return Err(e.fail(Code::BadValue, &format!("{at}: {problem}")));
                }
                let value = serde_json::to_value(&doc.constraints)
                    .map_err(|err| e.fail(Code::BadValue, &err.to_string()))?;
                e.plain(&value)
            })?;
        }
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
            + usize::from(!s.xref_paths.is_empty())
            + usize::from(s.heights.is_some())
            + usize::from(s.coordinate_epoch.is_some())
            + usize::from(s.drawing_font.is_some())
            + usize::from(s.drawing_unit.is_some())
            + usize::from(s.length_style.is_some())
            + usize::from(s.second_srid.is_some())
            + usize::from(s.custom_crs.is_some())
            + usize::from(s.second_custom_crs.is_some())
            + usize::from(!s.datum_transforms.is_empty())
            + usize::from(s.survey.is_some())
            + usize::from(!s.layer_states.is_empty())
            + usize::from(!s.text_styles.is_empty())
            + usize::from(!s.dimension_styles.is_empty())
            + usize::from(!s.leader_styles.is_empty())
            + usize::from(!s.wall_styles.is_empty())
            + usize::from(s.arch.is_some())
            + usize::from(s.topology.is_some())
            + usize::from(has_heights(s))
            + usize::from(!s.connections.is_empty())
            + usize::from(!s.networks.is_empty())
            + usize::from(!s.variables.is_empty())
            + usize::from(!s.address_locators.is_empty())
            + usize::from(s.ucs.is_some())
            + usize::from(!s.named_ucs.is_empty())
            + usize::from(!s.named_views.is_empty())
            + usize::from(!s.forms.is_empty())
            + usize::from(!s.domains.is_empty())
            + usize::from(!s.relations.is_empty())
            + usize::from(!s.sources.is_empty())
            + usize::from(!s.line_types.is_empty())
            + usize::from(!s.hatch_patterns.is_empty())
            + usize::from(s.line_type_scale.is_some());
        self.open(n, true)?;
        if let Some(u) = &s.ucs {
            self.key("ucs");
            self.at(Seg::Name("ucs"), |e| e.ucs(u))?;
        }
        // Schema 75 (docs/adr/0339 §3): the levels and the walls' detail, as their JSON.
        if let Some(a) = &s.arch {
            self.key("arch");
            self.at(Seg::Name("arch"), |e| {
                if a.is_empty() {
                    return Err(e.fail(Code::BadValue, "boş mimari ayar yazılmaz"));
                }
                if let Some((field, words)) = a.problem() {
                    return Err(e.fail(Code::BadValue, &format!("{field}: {words}")));
                }
                e.json(&serde_json::to_value(a).unwrap_or_default())
            })?;
        }
        self.key("srid");
        self.w.uint(u64::from(s.srid));
        if !s.forms.is_empty() {
            self.key("forms");
            self.at(Seg::Name("forms"), |e| {
                if let Some(problem) = kentos_contracts::forms_problem(&s.forms) {
                    return Err(e.fail(Code::BadValue, &problem));
                }
                let value = serde_json::to_value(&s.forms)
                    .map_err(|err| e.fail(Code::BadValue, &err.to_string()))?;
                e.plain(&value)
            })?;
        }
        if let Some(v) = &s.survey {
            self.key("survey");
            self.at(Seg::Name("survey"), |e| e.survey(v))?;
        }
        // Schema 72 (docs/adr/0430): the project's value lists, written whole and checked.
        if !s.domains.is_empty() {
            self.key("domains");
            self.at(Seg::Name("domains"), |e| {
                if let Some((_, problem)) = kentos_contracts::domains_problem(&s.domains) {
                    return Err(e.fail(Code::BadValue, &problem));
                }
                let value = serde_json::to_value(&s.domains)
                    .map_err(|err| e.fail(Code::BadValue, &err.to_string()))?;
                e.plain(&value)
            })?;
        }
        // Schema 85 (docs/adr/0519 §1): what the elevations are.
        if let Some(h) = &s.heights {
            self.key("heights");
            self.at(Seg::Name("heights"), |e| e.heights(s, h))?;
        }
        // Schema 74 (docs/adr/0503): the layers bound to their sources, written whole and checked.
        if !s.sources.is_empty() {
            self.key("sources");
            self.at(Seg::Name("sources"), |e| {
                if let Some(problem) = kentos_contracts::sources_problem(&s.sources, &[]) {
                    return Err(e.fail(Code::BadValue, &problem));
                }
                let value = serde_json::to_value(&s.sources)
                    .map_err(|err| e.fail(Code::BadValue, &err.to_string()))?;
                e.plain(&value)
            })?;
        }
        self.key("areaUnit");
        self.w.text(area_unit(s.area_unit));
        if !s.named_ucs.is_empty() {
            self.key("namedUcs");
            self.at(Seg::Name("namedUcs"), |e| e.named_ucs(&s.named_ucs))?;
        }
        if !s.networks.is_empty() {
            self.key("networks");
            self.at(Seg::Name("networks"), |e| e.networks(&s.networks))?;
        }
        if let Some(t) = &s.topology {
            self.key("topology");
            self.at(Seg::Name("topology"), |e| e.topology(t))?;
        }
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
        if !s.line_types.is_empty() {
            self.key("lineTypes");
            self.at(Seg::Name("lineTypes"), |e| e.line_types(&s.line_types))?;
        }
        self.key("plotScale");
        self.at(Seg::Name("plotScale"), |e| e.float(s.plot_scale))?;
        if !s.relations.is_empty() {
            self.key("relations");
            self.at(Seg::Name("relations"), |e| {
                if let Some(problem) = kentos_contracts::relations_problem(&s.relations) {
                    return Err(e.fail(Code::BadValue, &problem));
                }
                let value = serde_json::to_value(&s.relations)
                    .map_err(|err| e.fail(Code::BadValue, &err.to_string()))?;
                e.plain(&value)
            })?;
        }
        if !s.variables.is_empty() {
            self.key("variables");
            self.at(Seg::Name("variables"), |e| e.variables(&s.variables))?;
        }
        if let Some(w) = s.workspace {
            self.key("workspace");
            self.w.text(workspace(w));
        }
        // Schema 84 (docs/adr/0349 §1): the external references' search paths, as kept.
        if !s.xref_paths.is_empty() {
            self.key("xrefPaths");
            self.at(Seg::Name("xrefPaths"), |e| {
                if kentos_contracts::sanitized_xref_paths(s.xref_paths.clone()) != s.xref_paths {
                    return Err(e.fail(
                        Code::BadValue,
                        "arama yolları kırpılmış, boş olmayan ve yinelenmeyen en çok 64 yol olmalı",
                    ));
                }
                e.w.array(s.xref_paths.len());
                for p in &s.xref_paths {
                    e.text(p)?;
                }
                Ok(())
            })?;
        }
        if let Some(a) = s.annotation.as_ref().filter(|_| has_heights(s)) {
            self.key("annotation");
            self.at(Seg::Name("annotation"), |e| e.annotation(a))?;
        }
        if !s.named_views.is_empty() {
            self.key("namedViews");
            self.at(Seg::Name("namedViews"), |e| e.named_views(&s.named_views))?;
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
        if !s.text_styles.is_empty() {
            self.key("textStyles");
            self.at(Seg::Name("textStyles"), |e| e.text_styles(&s.text_styles))?;
        }
        // Schema 75 (docs/adr/0339 §2): the wall styles, as their JSON.
        if !s.wall_styles.is_empty() {
            self.key("wallStyles");
            self.at(Seg::Name("wallStyles"), |e| {
                if let Some(problem) = kentos_contracts::wall_styles_problem(&s.wall_styles) {
                    return Err(e.fail(Code::BadValue, &problem));
                }
                e.json(&serde_json::to_value(&s.wall_styles).unwrap_or_default())
            })?;
        }
        if !s.connections.is_empty() {
            self.key("connections");
            self.at(Seg::Name("connections"), |e| e.connections(&s.connections))?;
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
        if let Some(style) = s.length_style {
            self.key("lengthStyle");
            self.at(Seg::Name("lengthStyle"), |e| {
                if style == LengthStyle::Decimal || !s.drawing_unit.is_some_and(|u| u.imperial()) {
                    return Err(e.fail(
                        Code::BadValue,
                        "ayak ve inç biçimi yalnız inç ya da ayak birimli projede olur; ondalık biçim yazılmaz",
                    ));
                }
                e.w.text(length_style(style));
                Ok(())
            })?;
        }
        self.key("areaDecimals");
        self.w.uint(u64::from(s.area_decimals));
        // Schema 67 (docs/adr/0326 §4): the leader styles, as their JSON.
        if !s.leader_styles.is_empty() {
            self.key("leaderStyles");
            self.at(Seg::Name("leaderStyles"), |e| {
                if let Some((field, words)) = s.leader_styles.iter().find_map(kentos_contracts::LeaderStyleDef::problem) {
                    return Err(e.fail(Code::BadValue, &format!("{field}: {words}")));
                }
                e.json(&serde_json::to_value(&s.leader_styles).unwrap_or_default())
            })?;
        }
        // Schema 71 (docs/adr/0331 §1): the project's own hatch patterns, as their JSON.
        if !s.hatch_patterns.is_empty() {
            self.key("hatchPatterns");
            self.at(Seg::Name("hatchPatterns"), |e| {
                if let Some(problem) = kentos_contracts::hatch_patterns_problem(&s.hatch_patterns) {
                    return Err(e.fail(Code::BadValue, &problem));
                }
                e.json(&serde_json::to_value(&s.hatch_patterns).unwrap_or_default())
            })?;
        }
        if let Some(scale) = s.line_type_scale {
            self.key("lineTypeScale");
            self.at(Seg::Name("lineTypeScale"), |e| {
                if !(scale > 0.0 && scale <= kentos_contracts::MAX_LINE_TYPE_SCALE) || scale == 1.0
                {
                    return Err(e.fail(
                        Code::BadValue,
                        "çizgi tipi ölçeği 0'dan büyük ve en çok 1000 olmalı; 1 yazılmaz",
                    ));
                }
                e.float(scale)
            })?;
        }
        self.key("lengthDecimals");
        self.w.uint(u64::from(s.length_decimals));
        if !s.address_locators.is_empty() {
            self.key("addressLocators");
            self.at(Seg::Name("addressLocators"), |e| {
                e.address_locators(&s.address_locators)
            })?;
        }
        // Schema 85 (docs/adr/0519 §1): the coordinates' epoch.
        if let Some(epoch) = s.coordinate_epoch {
            self.key("coordinateEpoch");
            self.at(Seg::Name("coordinateEpoch"), |e| {
                if let Some(why) = kentos_contracts::epoch_problem(epoch) {
                    return Err(e.fail(Code::BadValue, &why));
                }
                if !s.has_system() {
                    return Err(e.fail(Code::BadValue, "koordinat epoku bir koordinat sistemi ister"));
                }
                e.float(epoch)
            })?;
        }
        if !s.datum_transforms.is_empty() {
            self.key("datumTransforms");
            self.at(Seg::Name("datumTransforms"), |e| {
                e.datum_transforms(&s.datum_transforms)
            })?;
        }
        if !s.dimension_styles.is_empty() {
            self.key("dimensionStyles");
            self.at(Seg::Name("dimensionStyles"), |e| {
                e.dimension_styles(&s.dimension_styles)
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

    /// The project's line types (docs/adr/0313 §1), checked whole as a reader checks them; keys in the canonical
    /// order.
    fn line_types(&mut self, types: &'d [kentos_contracts::LineTypeDef]) -> Result<(), KcadError> {
        if let Some(problem) = kentos_contracts::line_types_problem(types) {
            return Err(self.fail(Code::BadValue, &problem));
        }
        self.open(types.len(), false)?;
        for (i, t) in types.iter().enumerate() {
            self.at(Seg::Index(i), |e| {
                let n =
                    3 + usize::from(t.source.is_some()) + usize::from(!t.description.is_empty());
                e.open(n, true)?;
                // name unit (4), source (6), elements (8), description (11).
                e.key("name");
                e.text(&t.name)?;
                e.key("unit");
                e.w.text(match t.unit {
                    kentos_contracts::PatternUnit::Paper => "paper",
                    kentos_contracts::PatternUnit::Drawing => "drawing",
                });
                if let Some(source) = &t.source {
                    e.key("source");
                    e.text(source)?;
                }
                e.key("elements");
                e.at(Seg::Name("elements"), |e| {
                    e.open(t.elements.len(), false)?;
                    for (k, el) in t.elements.iter().enumerate() {
                        e.at(Seg::Index(k), |e| e.pattern_element(el))?;
                    }
                    e.close();
                    Ok(())
                })?;
                if !t.description.is_empty() {
                    e.key("description");
                    e.text(&t.description)?;
                }
                e.close();
                Ok(())
            })?;
        }
        self.close();
        Ok(())
    }

    /// A pattern's element: text (4), shape (5), length (6).
    fn pattern_element(
        &mut self,
        el: &'d kentos_contracts::PatternElement,
    ) -> Result<(), KcadError> {
        self.open(
            1 + usize::from(el.text.is_some()) + usize::from(el.shape.is_some()),
            true,
        )?;
        if let Some(t) = &el.text {
            self.key("text");
            self.at(Seg::Name("text"), |e| {
                let place = &t.place;
                e.open(2 + place_count(place), true)?;
                e.place_xy(place)?;
                // text (4), scale style (5), upright (7), absolute rotation (8).
                e.key("text");
                e.text(&t.text)?;
                e.place_scale(place)?;
                e.key("style");
                e.text(&t.style)?;
                e.place_turn(place)?;
                e.close();
                Ok(())
            })?;
        }
        if let Some(s) = &el.shape {
            self.key("shape");
            self.at(Seg::Name("shape"), |e| {
                let place = &s.place;
                e.open(2 + place_count(place), true)?;
                e.place_xy(place)?;
                // file name (4), scale (5), upright (7), absolute rotation (8).
                e.key("file");
                e.text(&s.file)?;
                e.key("name");
                e.text(&s.name)?;
                e.place_scale(place)?;
                e.place_turn(place)?;
                e.close();
                Ok(())
            })?;
        }
        self.key("length");
        self.at(Seg::Name("length"), |e| e.float(el.length))?;
        self.close();
        Ok(())
    }

    fn place_xy(&mut self, p: &kentos_contracts::PatternPlace) -> Result<(), KcadError> {
        if p.x != 0.0 {
            self.key("x");
            self.at(Seg::Name("x"), |e| e.float(p.x))?;
        }
        if p.y != 0.0 {
            self.key("y");
            self.at(Seg::Name("y"), |e| e.float(p.y))?;
        }
        Ok(())
    }

    fn place_scale(&mut self, p: &kentos_contracts::PatternPlace) -> Result<(), KcadError> {
        if p.scale != 1.0 {
            self.key("scale");
            self.at(Seg::Name("scale"), |e| e.float(p.scale))?;
        }
        Ok(())
    }

    fn place_turn(&mut self, p: &kentos_contracts::PatternPlace) -> Result<(), KcadError> {
        if p.upright {
            self.key("upright");
            self.w.bool(true);
        }
        if p.absolute {
            self.key("absolute");
            self.w.bool(true);
        }
        if p.rotation != 0.0 {
            self.key("rotation");
            self.at(Seg::Name("rotation"), |e| e.float(p.rotation))?;
        }
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
            2 + usize::from(n.style.is_some())
                + usize::from(n.locked.is_some())
                + usize::from(n.frozen.is_some())
                + usize::from(n.plot.is_some()),
            true,
        )?;
        self.key("node");
        self.text(&n.node)?;
        // Schema 55 (docs/adr/0311 §3): its freeze and printing.
        if let Some(plot) = n.plot {
            self.key("plot");
            self.w.bool(plot);
        }
        if let Some(style) = &n.style {
            self.key("style");
            self.at(Seg::Name("style"), |e| e.layer_style(style))?;
        }
        if let Some(frozen) = n.frozen {
            self.key("frozen");
            self.w.bool(frozen);
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

    /// The topology settings (docs/adr/0202 §7), checked whole as a reader
    /// checks them: `rules` (`id`, `kind`, `layer`, `other`, `value`),
    /// `tolerance`, `exceptions` (`at`, `rule`, `objects`).
    fn topology(&mut self, t: &'d TopologySettings) -> Result<(), KcadError> {
        if let Some(problem) = t.problem() {
            return Err(self.fail(Code::BadValue, &problem));
        }
        self.open(
            usize::from(!t.rules.is_empty())
                + usize::from(t.tolerance.is_some())
                + usize::from(!t.exceptions.is_empty()),
            true,
        )?;
        if !t.rules.is_empty() {
            self.key("rules");
            self.at(Seg::Name("rules"), |e| {
                e.open(t.rules.len(), false)?;
                for (i, r) in t.rules.iter().enumerate() {
                    e.at(Seg::Index(i), |e| {
                        e.open(
                            3 + usize::from(r.other.is_some()) + usize::from(r.value.is_some()),
                            true,
                        )?;
                        e.key("id");
                        e.text(&r.id)?;
                        e.key("kind");
                        e.w.text(r.kind.key());
                        e.key("layer");
                        e.text(&r.layer)?;
                        if let Some(o) = &r.other {
                            e.key("other");
                            e.text(o)?;
                        }
                        if let Some(v) = r.value {
                            e.key("value");
                            e.at(Seg::Name("value"), |e| e.float(v))?;
                        }
                        e.close();
                        Ok(())
                    })?;
                }
                e.close();
                Ok(())
            })?;
        }
        if let Some(tol) = t.tolerance {
            self.key("tolerance");
            self.at(Seg::Name("tolerance"), |e| e.float(tol))?;
        }
        if !t.exceptions.is_empty() {
            self.key("exceptions");
            self.at(Seg::Name("exceptions"), |e| {
                e.open(t.exceptions.len(), false)?;
                for (i, x) in t.exceptions.iter().enumerate() {
                    e.at(Seg::Index(i), |e| {
                        e.open(3, true)?;
                        e.key("at");
                        e.at(Seg::Name("at"), |e| e.point(&x.at))?;
                        e.key("rule");
                        e.text(&x.rule)?;
                        e.key("objects");
                        e.at(Seg::Name("objects"), |e| {
                            e.open(x.objects.len(), false)?;
                            for (j, id) in x.objects.iter().enumerate() {
                                e.at(Seg::Index(j), |e| e.id(&id.0))?;
                            }
                            e.close();
                            Ok(())
                        })?;
                        e.close();
                        Ok(())
                    })?;
                }
                e.close();
                Ok(())
            })?;
        }
        self.close();
        Ok(())
    }

    /// The survey settings (docs/adr/0169 §3), checked whole as a reader checks them.
    /// The annotation heights (docs/adr/0205 §1), each kind given, in the
    /// canonical order of their keys; checked as a reader checks them.
    fn annotation(&mut self, a: &AnnotationHeights) -> Result<(), KcadError> {
        if let Some((key, problem)) = a.problem() {
            return self.at(Seg::Name(key), |e| Err(e.fail(Code::BadValue, &problem)));
        }
        let order = [
            AnnotationKind::Text,
            AnnotationKind::Table,
            AnnotationKind::Leader,
            AnnotationKind::Measure,
            AnnotationKind::Station,
            AnnotationKind::Dimension,
            AnnotationKind::Coordinate,
        ];
        let given: Vec<(AnnotationKind, f64)> = order
            .into_iter()
            .filter_map(|k| a.given(k).map(|mm| (k, mm)))
            .collect();
        self.open(given.len(), true)?;
        for (kind, mm) in given {
            self.key(kind.key());
            self.at(Seg::Name(kind.key()), |e| e.float(mm))?;
        }
        self.close();
        Ok(())
    }

    /// Schema 85's heights (docs/adr/0519 §1), checked: their rules, a kind a project keeps, an ellipsoidal height
    /// only with a system.
    fn heights(&mut self, s: &ProjectSettings, h: &HeightReference) -> Result<(), KcadError> {
        if let Some((_, why)) = h.problem() {
            return Err(self.fail(Code::BadValue, &why));
        }
        let kind = match h.kind {
            HeightKind::Ellipsoidal => "ellipsoidal",
            HeightKind::Orthometric => "orthometric",
            HeightKind::Unknown => {
                return Err(self.fail(Code::BadValue, "bilinmeyen yükseklik yazılmaz; alan yazılmaz"));
            }
        };
        if h.kind == HeightKind::Ellipsoidal && !s.has_system() {
            return Err(self.fail(
                Code::BadValue,
                "elipsoit yüksekliği bir koordinat sistemi ister; yerel projede yalnız ortometrik yükseklik olur",
            ));
        }
        self.open(1 + usize::from(h.geoid.is_some()) + usize::from(h.vertical_srid.is_some()), true)?;
        self.key("kind");
        self.w.text(kind);
        if let Some(g) = &h.geoid {
            self.key("geoid");
            self.at(Seg::Name("geoid"), |e| {
                e.open(1 + usize::from(g.sha256.is_some()), true)?;
                e.key("name");
                e.text(&g.name)?;
                if let Some(sha) = &g.sha256 {
                    e.key("sha256");
                    e.text(sha)?;
                }
                Ok(())
            })?;
        }
        if let Some(v) = h.vertical_srid {
            self.key("verticalSrid");
            self.w.uint(u64::from(v));
        }
        Ok(())
    }

    fn survey(&mut self, v: &SurveySettings) -> Result<(), KcadError> {
        if let Some(problem) = v.problem() {
            return Err(self.fail(Code::BadValue, &problem));
        }
        // Floats and the one bool, in the canonical order of their keys.
        let fields = [
            ("index", v.index.map(Ok)),
            ("faceHz", v.face_hz.map(Ok)),
            ("twoWay", v.two_way.map(Ok)),
            ("sigmaPpm", v.sigma_ppm.map(Ok)),
            ("faceSlope", v.face_slope.map(Ok)),
            ("refraction", v.refraction.map(Ok)),
            ("sigmaZenith", v.sigma_zenith.map(Ok)),
            ("groundHeight", v.ground_height.map(Ok)),
            ("reduceToGrid", v.reduce_to_grid.map(Err)),
            ("sigmaDistance", v.sigma_distance.map(Ok)),
            ("traverseAngle", v.traverse_angle.map(Ok)),
            ("traverseCoord", v.traverse_coord.map(Ok)),
            ("sigmaCentering", v.sigma_centering.map(Ok)),
            ("sigmaDirection", v.sigma_direction.map(Ok)),
            ("sigmaLevelling", v.sigma_levelling.map(Ok)),
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
        let fields = !n.fields.is_empty();
        let compose = n.compose.as_ref();
        self.open(
            8 + usize::from(n.snap.is_some())
                + usize::from(fields)
                + usize::from(n.service.is_some())
                + usize::from(n.feed.is_some())
                + usize::from(n.time.is_some())
                + usize::from(n.scenario.is_some())
                + usize::from(n.replaces.is_some())
                + usize::from(n.filter.is_some())
                + usize::from(!n.actions.is_empty())
                + usize::from(n.card.is_some())
                + usize::from(compose.is_some())
                + usize::from(n.plot == Some(false))
                + usize::from(n.frozen == Some(true))
                + usize::from(!n.table_styles.is_empty())
                + usize::from(n.subtypes.is_some())
                + usize::from(!n.contingents.is_empty())
                + usize::from(!n.rules.is_empty()),
            true,
        )?;
        // id (2), card feed name plot snap time type (4), style (5), fields filter locked (6),
        // actions compose service visible (7), children expanded replaces scenario subtypes (8),
        // contingents tableStyles (11).
        if n.service.is_some() && n.feed.is_some() {
            return Err(self.fail(
                Code::BadValue,
                "katman hem servisten çizilir hem nesnelerini bir kaynaktan alır; ikisi birden olmaz",
            ));
        }
        self.key("id");
        self.at(Seg::Name("id"), |e| e.text(&n.id))?;
        if let Some(card) = &n.card {
            self.key("card");
            self.at(Seg::Name("card"), |e| e.info_card(n.kind, card))?;
        }
        if let Some(feed) = &n.feed {
            self.key("feed");
            self.at(Seg::Name("feed"), |e| e.feed(n.kind, feed))?;
        }
        self.key("name");
        self.at(Seg::Name("name"), |e| e.text(&n.name))?;
        // Yazdır (docs/adr/0260 §4): only `false` is written.
        match n.plot {
            Some(false) => {
                self.key("plot");
                self.w.bool(false);
            }
            Some(true) => {
                return Err(self.fail(
                    Code::BadValue,
                    "plot yalnız false yazılır; yazdırılan katmanda alan yoktur",
                ));
            }
            None => {}
        }
        if let Some(snap) = &n.snap {
            self.key("snap");
            self.at(Seg::Name("snap"), |e| e.layer_snap(n.kind, snap))?;
        }
        if let Some(time) = &n.time {
            self.key("time");
            self.at(Seg::Name("time"), |e| e.layer_time(n.kind, time))?;
        }
        self.key("type");
        self.w.text(match n.kind {
            LayerNodeType::Group => "group",
            LayerNodeType::Layer => "layer",
        });
        // Schema 72 (docs/adr/0438 §1): the record rules, as their JSON, fitting the fields.
        if !n.rules.is_empty() {
            self.key("rules");
            self.at(Seg::Name("rules"), |e| {
                if n.kind == LayerNodeType::Group {
                    return Err(e.fail(Code::BadValue, "grubun kayıt kuralları olmaz; yalnız katmanın olur"));
                }
                if let Some((_, problem)) = kentos_contracts::layer_rules_problem(&n.fields, &n.rules) {
                    return Err(e.fail(Code::BadValue, &problem));
                }
                let value = serde_json::to_value(&n.rules)
                    .map_err(|err| e.fail(Code::BadValue, &err.to_string()))?;
                e.plain(&value)
            })?;
        }
        self.key("style");
        self.at(Seg::Name("style"), |e| e.layer_style(&n.style))?;
        if fields {
            self.key("fields");
            self.at(Seg::Name("fields"), |e| e.layer_fields(n.kind, &n.fields))?;
        }
        if let Some(filter) = &n.filter {
            self.key("filter");
            self.at(Seg::Name("filter"), |e| e.layer_filter(n.kind, filter))?;
        }
        // Dondur (docs/adr/0311 §1): only `true` is written.
        match n.frozen {
            Some(true) => {
                self.key("frozen");
                self.w.bool(true);
            }
            Some(false) => {
                return Err(self.fail(
                    Code::BadValue,
                    "frozen yalnız true yazılır; çözülmüş katmanda alan yoktur",
                ));
            }
            None => {}
        }
        self.key("locked");
        self.w.bool(n.locked);
        if !n.actions.is_empty() {
            self.key("actions");
            self.at(Seg::Name("actions"), |e| {
                e.layer_actions(n.kind, &n.actions)
            })?;
        }
        if let Some(c) = compose {
            self.key("compose");
            self.at(Seg::Name("compose"), |e| e.layer_compose(n.kind, c))?;
        }
        if let Some(service) = &n.service {
            self.key("service");
            self.at(Seg::Name("service"), |e| e.service(n.kind, service))?;
        }
        self.key("visible");
        self.w.bool(n.visible);
        self.key("children");
        self.at(Seg::Name("children"), |e| e.layers(&n.children))?;
        self.key("expanded");
        self.w.bool(n.expanded);
        if let Some(base) = &n.replaces {
            self.key("replaces");
            self.at(Seg::Name("replaces"), |e| e.replaces(n.kind, base))?;
        }
        if let Some(scenario) = &n.scenario {
            self.key("scenario");
            self.at(Seg::Name("scenario"), |e| e.scenario(n.kind, scenario))?;
        }
        // Schema 72 (docs/adr/0422 §4): the subtypes, as their JSON, on a layer whose fields they fit.
        if let Some(subtypes) = &n.subtypes {
            self.key("subtypes");
            self.at(Seg::Name("subtypes"), |e| {
                if n.kind == LayerNodeType::Group {
                    return Err(e.fail(Code::BadValue, "grubun alt türleri olmaz; yalnız katmanın olur"));
                }
                if let Some((_, problem)) = kentos_contracts::subtypes_problem(&n.fields, subtypes) {
                    return Err(e.fail(Code::BadValue, &problem));
                }
                let value = serde_json::to_value(subtypes)
                    .map_err(|err| e.fail(Code::BadValue, &err.to_string()))?;
                e.plain(&value)
            })?;
        }
        // Schema 72 (docs/adr/0423 §4): the value combinations, as their JSON, fitting the fields.
        if !n.contingents.is_empty() {
            self.key("contingents");
            self.at(Seg::Name("contingents"), |e| {
                if n.kind == LayerNodeType::Group {
                    return Err(e.fail(Code::BadValue, "grubun değer bileşimleri olmaz; yalnız katmanın olur"));
                }
                if let Some((_, problem)) =
                    kentos_contracts::contingents_problem(&n.fields, n.subtypes.as_ref(), &n.contingents)
                {
                    return Err(e.fail(Code::BadValue, &problem));
                }
                let value = serde_json::to_value(&n.contingents)
                    .map_err(|err| e.fail(Code::BadValue, &err.to_string()))?;
                e.plain(&value)
            })?;
        }
        // Schema 72 (docs/adr/0413 §3): the table's rules, as their JSON, on a layer.
        if !n.table_styles.is_empty() {
            self.key("tableStyles");
            self.at(Seg::Name("tableStyles"), |e| {
                if n.kind == LayerNodeType::Group {
                    return Err(e.fail(
                        Code::BadValue,
                        "grubun koşullu biçimi olmaz; yalnız katmanın olur",
                    ));
                }
                if let Some((_, problem)) =
                    kentos_contracts::table_styles_problem(&n.table_styles)
                {
                    return Err(e.fail(Code::BadValue, &problem));
                }
                let value = serde_json::to_value(&n.table_styles)
                    .map_err(|err| e.fail(Code::BadValue, &err.to_string()))?;
                e.plain(&value)
            })?;
        }
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

    /// A layer's fields (docs/adr/0199 §1), on a layer only, checked whole.
    fn layer_fields(
        &mut self,
        kind: LayerNodeType,
        fields: &'d [LayerField],
    ) -> Result<(), KcadError> {
        if kind == LayerNodeType::Group {
            return Err(self.fail(
                Code::BadValue,
                "grubun alanları yazılmaz; alanlar yalnız katmanındır",
            ));
        }
        if let Some(problem) = layer_fields_problem(fields) {
            return Err(self.fail(Code::BadValue, &problem));
        }
        self.open(fields.len(), false)?;
        for (i, f) in fields.iter().enumerate() {
            self.at(Seg::Index(i), |e| e.layer_field(f))?;
        }
        self.close();
        Ok(())
    }

    /// A field: max, min (3), kind, name (4), alias, scale (5), length,
    /// values (6), default (7), multiple, required (8); only what it has.
    fn layer_field(&mut self, f: &'d LayerField) -> Result<(), KcadError> {
        let n = 2
            + usize::from(f.id.is_some())
            + usize::from(f.max.is_some())
            + usize::from(f.min.is_some())
            + usize::from(f.alias.is_some())
            + usize::from(f.scale.is_some())
            + usize::from(f.length.is_some())
            + usize::from(f.values.is_some())
            + usize::from(f.default.is_some())
            + usize::from(f.multiple)
            + usize::from(f.required)
            + usize::from(f.split.is_some())
            + usize::from(f.merge.is_some())
            + usize::from(f.domain.is_some())
            + usize::from(f.unique)
            + usize::from(f.min_exclusive)
            + usize::from(f.max_exclusive)
            + usize::from(f.unit.is_some());
        self.open(n, true)?;
        if let Some(id) = &f.id {
            self.key("id");
            self.at(Seg::Name("id"), |e| e.text(id))?;
        }
        if let Some(max) = &f.max {
            self.key("max");
            self.at(Seg::Name("max"), |e| e.text(max))?;
        }
        if let Some(min) = &f.min {
            self.key("min");
            self.at(Seg::Name("min"), |e| e.text(min))?;
        }
        self.key("kind");
        self.w.text(f.kind.name());
        self.key("name");
        self.at(Seg::Name("name"), |e| e.text(&f.name))?;
        // Schema 72 (docs/adr/0438 §4): the unit, in the keys' canonical order.
        if let Some(unit) = &f.unit {
            self.key("unit");
            self.at(Seg::Name("unit"), |e| e.text(unit))?;
        }
        if let Some(alias) = &f.alias {
            self.key("alias");
            self.at(Seg::Name("alias"), |e| e.text(alias))?;
        }
        // Schema 72 (docs/adr/0419): the merge and split policies, in the keys' canonical order.
        if let Some(merge) = f.merge {
            self.key("merge");
            self.w.text(merge_name(merge));
        }
        if let Some(scale) = f.scale {
            self.key("scale");
            self.w.uint(u64::from(scale));
        }
        if let Some(split) = f.split {
            self.key("split");
            self.w.text(split_name(split));
        }
        // Schema 72 (docs/adr/0430): the value list it links.
        if let Some(domain) = &f.domain {
            self.key("domain");
            self.at(Seg::Name("domain"), |e| e.text(domain))?;
        }
        if let Some(length) = f.length {
            self.key("length");
            self.w.uint(u64::from(length));
        }
        // Schema 72 (docs/adr/0438 §3).
        if f.unique {
            self.key("unique");
            self.w.bool(true);
        }
        if let Some(values) = &f.values {
            self.key("values");
            self.at(Seg::Name("values"), |e| {
                e.open(values.len(), false)?;
                for (i, c) in values.iter().enumerate() {
                    e.at(Seg::Index(i), |e| {
                        e.open(
                            2 + usize::from(c.retired)
                                + usize::from(c.description.is_some())
                                + usize::from(c.from.is_some())
                                + usize::from(c.until.is_some()),
                            true,
                        )?;
                        // code from (4), label until (5), retired (7), description (11).
                        e.key("code");
                        e.at(Seg::Name("code"), |e| e.text(&c.code))?;
                        if let Some(from) = &c.from {
                            e.key("from");
                            e.at(Seg::Name("from"), |e| e.text(from))?;
                        }
                        e.key("label");
                        e.at(Seg::Name("label"), |e| e.text(&c.label))?;
                        if let Some(until) = &c.until {
                            e.key("until");
                            e.at(Seg::Name("until"), |e| e.text(until))?;
                        }
                        if c.retired {
                            e.key("retired");
                            e.w.bool(true);
                        }
                        if let Some(d) = &c.description {
                            e.key("description");
                            e.at(Seg::Name("description"), |e| e.text(d))?;
                        }
                        e.close();
                        Ok(())
                    })?;
                }
                e.close();
                Ok(())
            })?;
        }
        if let Some(default) = &f.default {
            self.key("default");
            self.at(Seg::Name("default"), |e| e.text(default))?;
        }
        if f.multiple {
            self.key("multiple");
            self.w.bool(true);
        }
        if f.required {
            self.key("required");
            self.w.bool(true);
        }
        if f.max_exclusive {
            self.key("maxExclusive");
            self.w.bool(true);
        }
        if f.min_exclusive {
            self.key("minExclusive");
            self.w.bool(true);
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
            + usize::from(s.line_pattern.is_some())
            + usize::from(s.fill.is_some())
            + usize::from(s.label.is_some())
            + usize::from(s.labels.is_some())
            + usize::from(s.point.is_some())
            + usize::from(s.renderer.is_some())
            + usize::from(s.pick_interior.is_some());
        self.open(n, true)?;
        // fill (4), color label point (5), labels (6), lineType renderer (8), lineWeight (10), linePattern (11),
        // pickInterior (12).
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
        if let Some(labels) = &s.labels {
            self.key("labels");
            self.at(Seg::Name("labels"), |e| e.layer_labels(labels))?;
        }
        self.key("lineType");
        self.w.text(line_type(s.line_type));
        if let Some(r) = &s.renderer {
            self.key("renderer");
            self.at(Seg::Name("renderer"), |e| e.opaque(r))?;
        }
        self.key("lineWeight");
        self.at(Seg::Name("lineWeight"), |e| e.float(s.line_weight))?;
        if let Some(pattern) = &s.line_pattern {
            self.key("linePattern");
            self.at(Seg::Name("linePattern"), |e| e.text(pattern))?;
        }
        if let Some(pick) = s.pick_interior {
            self.key("pickInterior");
            self.w.bool(pick);
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

    /// A JSON value made here (a contract's part written as its JSON, docs/adr/0318): as `opaque`, keys canonical.
    pub(super) fn json(&mut self, v: &Value) -> Result<(), KcadError> {
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
            Value::Array(list) => {
                self.open(list.len(), false)?;
                for v in list {
                    self.json(v)?;
                }
                self.close();
            }
            Value::Object(map) => {
                let mut pairs: Vec<(&String, &Value)> = map.iter().collect();
                pairs.sort_by(|a, b| key_order(a.0, b.0));
                self.open(pairs.len(), true)?;
                for (k, v) in pairs {
                    self.text(k)?;
                    self.json(v)?;
                }
                self.close();
            }
        }
        Ok(())
    }

    /// A value the contract serialized for the file (the project's forms): written as [`opaque`]
    /// writes it, its place named by the key above it only.
    fn plain(&mut self, v: &Value) -> Result<(), KcadError> {
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
            Value::Array(list) => {
                self.open(list.len(), false)?;
                for v in list {
                    self.plain(v)?;
                }
                self.close();
            }
            Value::Object(map) => {
                let mut pairs: Vec<(&String, &Value)> = map.iter().collect();
                pairs.sort_by(|a, b| key_order(a.0, b.0));
                self.open(pairs.len(), true)?;
                for (k, v) in pairs {
                    self.text(k)?;
                    self.plain(v)?;
                }
                self.close();
            }
        }
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

/// How many of a complex element's placing's keys are written: those other than their defaults.
fn place_count(p: &kentos_contracts::PatternPlace) -> usize {
    usize::from(p.x != 0.0)
        + usize::from(p.y != 0.0)
        + usize::from(p.scale != 1.0)
        + usize::from(p.upright)
        + usize::from(p.absolute)
        + usize::from(p.rotation != 0.0)
}

/// Whether the settings give an annotation height (docs/adr/0205 §1): a
/// field with none is not written.
fn has_heights(s: &ProjectSettings) -> bool {
    s.annotation
        .as_ref()
        .is_some_and(|a| *a != AnnotationHeights::default())
}

/// The oldest schema that holds the drawing: 39 when an object of it or of a
/// block definition has a vertex measure (docs/adr/0224), 35 when a layer has a filter
/// (docs/adr/0211), 34 when a layer has a time
/// setting, a group is a scenario or a layer stands for a base layer
/// (docs/adr/0210), 33 when the project has a
/// network (docs/adr/0209), 32 when a layer is drawn from a
/// service or has a feed, or the project has connections (docs/adr/0208), 31
/// when it has a point cloud or a raster read from an address
/// (docs/adr/0207), 30 when the project has
/// annotation heights or a dimension style lines, or a dimension of it or of
/// a block definition has lines or a leader an arrowhead size or one of
/// AutoCAD's arrowheads (docs/adr/0205), 29 when it has a raster
/// (docs/adr/0204 §2), 28 when its survey settings
/// name an a priori standard deviation (docs/adr/0203 §1), 27 when the
/// project has topology settings (docs/adr/0202 §7), 26 when a layer has fields
/// (docs/adr/0199 §1), 25 when a text of it or of a block definition has a
/// curve (docs/adr/0196), 24 when it has a picture (docs/adr/0192), 23 when a
/// hatch has a pattern, a gradient or a tie (docs/adr/0186), 22 when it has a table
/// (docs/adr/0184), 21 when the project has a text
/// or a dimension style or a text or a dimension of it or of a block
/// definition a face or a look (docs/adr/0183), 20 when a text of it or of a
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
    fn services(nodes: &[LayerNode]) -> bool {
        nodes
            .iter()
            .any(|n| n.service.is_some() || n.feed.is_some() || services(&n.children))
    }
    fn temporal(nodes: &[LayerNode]) -> bool {
        nodes.iter().any(|n| {
            n.time.is_some()
                || n.scenario.is_some()
                || n.replaces.is_some()
                || temporal(&n.children)
        })
    }
    fn filters(nodes: &[LayerNode]) -> bool {
        nodes
            .iter()
            .any(|n| n.filter.is_some() || filters(&n.children))
    }
    fn labelled(nodes: &[LayerNode]) -> bool {
        nodes.iter().any(|n| {
            n.style.labels.is_some()
                || n.style
                    .label
                    .as_ref()
                    .is_some_and(labels::has_engine_fields)
                || labelled(&n.children)
        })
    }
    fn informed(nodes: &[LayerNode]) -> bool {
        nodes
            .iter()
            .any(|n| !n.actions.is_empty() || n.card.is_some() || informed(&n.children))
    }
    fn composed(nodes: &[LayerNode]) -> bool {
        nodes
            .iter()
            .any(|n| n.compose.is_some() || composed(&n.children))
    }
    // Schema 87 (docs/adr/0521): a network's turns or levels, a speed cost's time profile.
    if doc.settings.networks.iter().any(|n| {
        n.turns.is_some() || n.levels.is_some() || n.costs.iter().any(|c| !c.periods.is_empty())
    }) {
        return crate::SCHEMA_WITH_NETWORK_TURNS;
    }
    // Schema 86 (docs/adr/0519 §1): what the elevations are, the coordinates' epoch.
    if doc.settings.heights.is_some() || doc.settings.coordinate_epoch.is_some() {
        return crate::SCHEMA_WITH_HEIGHTS;
    }
    // Schema 85 (docs/adr/0351): an insert's clipping, a PDF underlay.
    let clipped = |es: &[Entity]| es.iter().any(|e| matches!(e, Entity::Insert(i) if i.clipping.is_some()));
    if clipped(&doc.entities)
        || doc.blocks.iter().any(|b| {
            clipped(&b.entities) || b.xref.as_ref().is_some_and(|x| x.page.is_some() || x.underlay.is_some())
        })
    {
        return crate::SCHEMA_WITH_CLIPPING;
    }
    // Schema 84 (docs/adr/0349 §1): the external references' search paths.
    if !doc.settings.xref_paths.is_empty() {
        return crate::SCHEMA_WITH_XREF_PATHS;
    }
    // Schema 83 (docs/adr/0348 §1): an external reference.
    if doc.blocks.iter().any(|b| b.xref.is_some()) {
        return crate::SCHEMA_WITH_XREFS;
    }
    // Schema 82 (docs/adr/0346 §2): the project's phase view.
    if doc.settings.arch.as_ref().is_some_and(|a| a.phase_view.is_some()) {
        return crate::SCHEMA_WITH_PHASES;
    }
    // Schema 81 (docs/adr/0345): a view mark, or a named view with its own id.
    if doc.entities.iter().any(|e| matches!(e, Entity::ViewMark(_)))
        || doc.settings.named_views.iter().any(|v| !v.id.is_empty())
    {
        return crate::SCHEMA_WITH_VIEWMARKS;
    }
    // Schema 80 (docs/adr/0344): a tag, or a table's schedule of the drawing's objects.
    let scheduled = |list: &[Entity]| {
        list.iter().any(|e| {
            matches!(e, Entity::Tag(_))
                || matches!(e, Entity::Table(t) if matches!(t.source, Some(kentos_contracts::TableSource::Schedule { .. })))
        })
    };
    if scheduled(&doc.entities) {
        return crate::SCHEMA_WITH_TAGS;
    }
    // Schema 79 (docs/adr/0343): a grid, a column, a stair or a ramp.
    if doc.entities.iter().any(|e| {
        matches!(
            e,
            Entity::Grid(_) | Entity::Column(_) | Entity::Stair(_) | Entity::Ramp(_)
        )
    }) {
        return crate::SCHEMA_WITH_ARCH_ELEMENTS;
    }
    // Schema 78 (docs/adr/0342): a space, or the project's space method.
    if doc.entities.iter().any(|e| matches!(e, Entity::Space(_)))
        || doc.settings.arch.as_ref().is_some_and(|a| a.space_method.is_some())
    {
        return crate::SCHEMA_WITH_SPACES;
    }
    // Schema 77 (docs/adr/0341): an opening.
    if doc.entities.iter().any(|e| matches!(e, Entity::Opening(_))) {
        return crate::SCHEMA_WITH_OPENINGS;
    }
    // Schema 76 (docs/adr/0340): a wall's joins or cleanup, a component's priority.
    let ranked = |cs: &[kentos_contracts::WallComponent]| cs.iter().any(|c| c.priority.is_some());
    if doc.entities.iter().any(|e| {
        matches!(e, Entity::Wall(w) if !w.wall.joins.is_auto() || !w.wall.cleanup || ranked(&w.wall.components))
    }) || doc.settings.wall_styles.iter().any(|s| ranked(&s.components))
    {
        return crate::SCHEMA_WITH_WALL_JOINS;
    }
    // Schema 75 (docs/adr/0339): a wall, a wall style or the architectural settings.
    if doc.entities.iter().any(|e| matches!(e, Entity::Wall(_)))
        || !doc.settings.wall_styles.is_empty()
        || doc.settings.arch.is_some()
    {
        return crate::SCHEMA_WITH_WALLS;
    }
    // Schema 74 (docs/adr/0503): a layer bound to its source.
    if !doc.settings.sources.is_empty() {
        return crate::SCHEMA_WITH_SOURCES;
    }
    // Schema 72 (docs/adr/0401, 0408, 0410, 0412, 0413, 0418): a form of the project, a layer field's
    // stable id, its several values, a retired value or its date-and-time or time kind, a layer's
    // table rules, or a record without a geometry.
    fn keyed(nodes: &[LayerNode]) -> bool {
        let later = |f: &kentos_contracts::LayerField| {
            f.id.is_some()
                || f.multiple
                || f.values.as_ref().is_some_and(|v| v.iter().any(|c| c.retired))
                || f.split.is_some()
                || f.merge.is_some()
                || f.unique
                || f.min_exclusive
                || f.max_exclusive
                || f.unit.is_some()
                || f.values.as_ref().is_some_and(|v| {
                    v.iter()
                        .any(|c| c.description.is_some() || c.from.is_some() || c.until.is_some())
                })
                || matches!(
                    f.kind,
                    kentos_contracts::LayerFieldKind::DateTime
                        | kentos_contracts::LayerFieldKind::Time
                )
        };
        nodes.iter().any(|n| {
            n.fields.iter().any(later)
                || !n.table_styles.is_empty()
                || n.subtypes.is_some()
                || !n.contingents.is_empty()
                || !n.rules.is_empty()
                || keyed(&n.children)
        })
    }
    // Schema 73 (docs/adr/0334): constraints or parameters.
    if !doc.constraints.is_empty() {
        return crate::SCHEMA_WITH_CONSTRAINTS;
    }
    if !doc.settings.forms.is_empty()
        || keyed(&doc.layers)
        || doc.entities.iter().any(|e| matches!(e, Entity::Record(_)))
    {
        return crate::SCHEMA_WITH_FORMS;
    }
    // Schema 71 (docs/adr/0331): the project's hatch patterns, a pattern's own origin, an annotative hatch.
    if !doc.settings.hatch_patterns.is_empty()
        || doc.entities.iter().any(|e| {
            matches!(e, Entity::Hatch(h) if h.pattern.origin.is_some() || h.annotative.is_some())
        })
        || doc.blocks.iter().any(|b| {
            b.entities.iter().any(|e| {
                matches!(e, Entity::Hatch(h) if h.pattern.origin.is_some() || h.annotative.is_some())
            })
        })
    {
        return crate::SCHEMA_WITH_HATCH_EXTRAS;
    }
    // Schema 70 (docs/adr/0329 §3): a face's tie closed across gaps.
    if doc.entities.iter().any(|e| {
        matches!(e, Entity::Hatch(h) if h.assoc.as_ref().is_some_and(|a| a.gap.is_some()))
    }) {
        return crate::SCHEMA_WITH_HATCH_GAPS;
    }
    // Schema 69 (docs/adr/0328): a hatch's exact boundary or a tie to a face's line work.
    let bounded = |list: &[Entity]| {
        list.iter().any(|e| {
            matches!(e, Entity::Hatch(h)
                if h.boundary.is_some() || h.assoc.as_ref().is_some_and(|a| !a.bounds.is_empty()))
        })
    };
    if bounded(&doc.entities) || doc.blocks.iter().any(|b| bounded(&b.entities)) {
        return crate::SCHEMA_WITH_HATCH_BOUNDARIES;
    }
    // Schema 68 (docs/adr/0327): a table's styles, formulas, fields, linked file or block schedule.
    let tabled = |list: &[Entity]| {
        list.iter().any(|e| {
            matches!(e, Entity::Table(t)
                if !t.styles.is_empty()
                    || !t.formulas.is_empty()
                    || !t.fields.is_empty()
                    || matches!(&t.source, Some(kentos_contracts::TableSource::Blocks { .. }))
                    || matches!(&t.source, Some(kentos_contracts::TableSource::File { path, digest, copy, overrides, .. })
                        if path.is_some() || digest.is_some() || copy.is_some() || !overrides.is_keep()))
        })
    };
    if tabled(&doc.entities) {
        return crate::SCHEMA_WITH_TABLE_CELLS;
    }
    // Schema 67 (docs/adr/0326): a leader's arms, tip, blocks or style, or the project's leader styles.
    let multi = |list: &[Entity]| {
        list.iter().any(|e| {
            matches!(e, Entity::Leader(l)
                if !l.arms.is_empty() || l.tip.is_some() || !l.blocks.is_empty() || l.leader_style.is_some())
        })
    };
    if multi(&doc.entities)
        || doc.blocks.iter().any(|b| multi(&b.entities))
        || !doc.settings.leader_styles.is_empty()
    {
        return crate::SCHEMA_WITH_MULTILEADER;
    }
    // Schema 66 (docs/adr/0325): a text's fields (of the drawing or of a block).
    let fielded = |list: &[Entity]| {
        list.iter().any(|e| matches!(e, Entity::Text(t) if !t.paragraph.fields.is_empty()))
    };
    if fielded(&doc.entities) || doc.blocks.iter().any(|b| fielded(&b.entities)) {
        return crate::SCHEMA_WITH_TEXT_FIELDS;
    }
    // Schema 65 (docs/adr/0324): a text's layout or a stacked fraction (of the drawing or of a block).
    let laid = |list: &[Entity]| {
        list.iter().any(|e| match e {
            Entity::Text(t) => {
                t.paragraph.layout.is_some()
                    || t.paragraph.runs.iter().any(|r| r.script == Some(kentos_contracts::TextScript::Stack))
            }
            _ => false,
        })
    };
    if laid(&doc.entities) || doc.blocks.iter().any(|b| laid(&b.entities)) {
        return crate::SCHEMA_WITH_PARAGRAPH_LAYOUT;
    }
    // Schema 64 (docs/adr/0323): an annotative text, dimension or leader (of the drawing or of a block).
    let scaled = |list: &[Entity]| list.iter().any(|e| kentos_contracts::annotative_of(e).is_some());
    if scaled(&doc.entities) || doc.blocks.iter().any(|b| scaled(&b.entities)) {
        return crate::SCHEMA_WITH_ANNOTATIVE;
    }
    // Schema 63 (docs/adr/0322 §5): a text's or a dimension's overrides (of the drawing or of a block).
    let overridden = |list: &[Entity]| {
        list.iter().any(|e| match e {
            Entity::Text(t) => !t.face.overrides.is_empty(),
            Entity::Dimension(d) => !d.look.overrides.is_empty(),
            _ => false,
        })
    };
    if overridden(&doc.entities) || doc.blocks.iter().any(|b| overridden(&b.entities)) {
        return crate::SCHEMA_WITH_OVERRIDES;
    }
    // Schema 62 (docs/adr/0322): a centre mark, an axis or a level (of the drawing or of a block), or a pair rule.
    let marked = |list: &[Entity]| {
        list.iter().any(|e| {
            matches!(e, Entity::Dimension(d)
                if d.style.is_some_and(DimensionStyle::is_schema_62)
                    || d.refs.as_ref().is_some_and(|r| r.rule == Some(kentos_contracts::DimensionRule::Pair)))
        })
    };
    if marked(&doc.entities) || doc.blocks.iter().any(|b| marked(&b.entities)) {
        return crate::SCHEMA_WITH_DIMENSION_MARKS;
    }
    // Schema 61 (docs/adr/0321): a dimension's options, moved value or breaks (of the drawing or of a block), or a
    // dimension style's options.
    let optioned = |list: &[Entity]| {
        list.iter().any(|e| {
            matches!(e, Entity::Dimension(d)
                if d.look.options.is_some() || d.text_shift.is_some() || !d.breaks.is_empty())
        })
    };
    if optioned(&doc.entities)
        || doc.blocks.iter().any(|b| optioned(&b.entities))
        || doc
            .settings
            .dimension_styles
            .iter()
            .any(|s| s.options.is_some())
    {
        return crate::SCHEMA_WITH_DIMENSION_OPTIONS;
    }
    // Schema 60 (docs/adr/0320 §1): a dimension's references' rule (in the drawing only: a block's dimensions keep none).
    if doc.entities.iter().any(
        |e| matches!(e, Entity::Dimension(d) if d.refs.as_ref().is_some_and(|r| r.rule.is_some())),
    ) {
        return crate::SCHEMA_WITH_DIMENSION_RULES;
    }
    // Schema 59 (docs/adr/0318): a block's dynamic part or an insert's parameter values, of the drawing or of a block.
    let valued = |list: &[Entity]| {
        list.iter()
            .any(|e| matches!(e, kentos_contracts::Entity::Insert(i) if !i.parameters.is_empty()))
    };
    if valued(&doc.entities)
        || doc
            .blocks
            .iter()
            .any(|b| b.dynamic.is_some() || valued(&b.entities))
    {
        return crate::SCHEMA_WITH_DYNAMIC_BLOCKS;
    }
    // Schema 58 (docs/adr/0317 §1, §4): an attribute definition's option, an insert's moved attribute (of the drawing
    // or of a block definition) or a block's card.
    let placed = |list: &[Entity]| {
        list.iter().any(
            |e| matches!(e, kentos_contracts::Entity::Insert(i) if !i.attribute_places.is_empty()),
        )
    };
    if placed(&doc.entities)
        || doc.blocks.iter().any(|b| {
            b.card.is_some()
                || placed(&b.entities)
                || b.attributes
                    .iter()
                    .any(|a| a.options != kentos_contracts::AttributeOptions::default())
        })
    {
        return crate::SCHEMA_WITH_BLOCK_CARDS;
    }
    // Schema 57 (docs/adr/0315 §3, §4): an insert's own y factor, of the drawing or of a block definition, or a
    // block's unit.
    let stretched = |list: &[Entity]| {
        list.iter().any(|e| {
            matches!(e, kentos_contracts::Entity::Insert(i) if i.y_scale.is_some_and(|y| y != i.scale))
        })
    };
    if stretched(&doc.entities)
        || doc
            .blocks
            .iter()
            .any(|b| b.units.is_some() || stretched(&b.entities))
    {
        return crate::SCHEMA_WITH_BLOCK_EXTRAS;
    }
    // Schema 56 (docs/adr/0313): a line type with a pattern, a line type scale, a pattern named by a layer, a layer
    // state's style or an object of the drawing or of a block definition.
    fn patterned(nodes: &[LayerNode]) -> bool {
        nodes
            .iter()
            .any(|n| n.style.line_pattern.is_some() || patterned(&n.children))
    }
    let named = |list: &[Entity]| list.iter().any(|e| e.base().look.line_pattern.is_some());
    if !doc.settings.line_types.is_empty()
        || doc.settings.line_type_scale.is_some()
        || patterned(&doc.layers)
        || doc
            .settings
            .layer_states
            .iter()
            .flat_map(|s| &s.nodes)
            .any(|n| n.style.as_ref().is_some_and(|s| s.line_pattern.is_some()))
        || named(&doc.entities)
        || doc.blocks.iter().any(|b| named(&b.entities))
    {
        return crate::SCHEMA_WITH_PATTERNS;
    }
    // Schema 55 (docs/adr/0311): a frozen node, or a layer state keeping freezes and printing.
    fn frozen(nodes: &[LayerNode]) -> bool {
        nodes
            .iter()
            .any(|n| n.frozen == Some(true) || frozen(&n.children))
    }
    if frozen(&doc.layers)
        || doc
            .settings
            .layer_states
            .iter()
            .flat_map(|s| &s.nodes)
            .any(|n| n.frozen.is_some() || n.plot.is_some())
    {
        return crate::SCHEMA_WITH_FROZEN;
    }
    // Schema 54 (docs/adr/0310): an area's mask, a block definition's too.
    let masks = |list: &[Entity]| list.iter().any(|e| e.base().look.wipeout.is_some());
    if masks(&doc.entities) || doc.blocks.iter().any(|b| masks(&b.entities)) {
        return crate::SCHEMA_WITH_WIPEOUTS;
    }
    // Schema 53 (docs/adr/0300): an associative array.
    if !doc.arrays.is_empty() {
        return SCHEMA_WITH_ARRAYS;
    }
    // Schema 52 (docs/adr/0285): a named view.
    if !doc.settings.named_views.is_empty() {
        return SCHEMA_WITH_VIEWS;
    }
    // Schema 51 (docs/adr/0263): a dimension's reference along an edge or inside a block.
    let extended = |r: &kentos_contracts::SubRef| {
        matches!(r.place, kentos_contracts::SubPlace::Along { .. }) || !r.path.is_empty()
    };
    if doc.entities.iter().any(|e| {
        matches!(e, kentos_contracts::Entity::Dimension(d)
            if d.refs.as_ref().is_some_and(|r| r.each().any(|(_, s)| extended(s))))
    }) {
        return SCHEMA_WITH_SUBREF_EXTRAS;
    }
    // Schema 50 (docs/adr/0262): an inch or foot drawing, or a length style.
    if doc.settings.drawing_unit.is_some_and(|u| u.imperial())
        || doc.settings.length_style.is_some()
    {
        return SCHEMA_WITH_IMPERIAL;
    }
    // Schema 49 (docs/adr/0261): a user coordinate system, current or named.
    if doc.settings.ucs.is_some() || !doc.settings.named_ucs.is_empty() {
        return SCHEMA_WITH_UCS;
    }
    // Schema 48 (docs/adr/0260): an object's look, a block definition's too, or a layer not printed.
    fn unprinted(nodes: &[LayerNode]) -> bool {
        nodes
            .iter()
            .any(|n| n.plot == Some(false) || unprinted(&n.children))
    }
    let looks = |list: &[Entity]| list.iter().any(|e| !e.base().look.is_empty());
    if looks(&doc.entities)
        || doc.blocks.iter().any(|b| looks(&b.entities))
        || unprinted(&doc.layers)
    {
        return SCHEMA_WITH_LOOK;
    }
    // Schema 47 (docs/adr/0259): a NURBS curve, a block definition's too.
    let curves = |list: &[Entity]| list.iter().any(|e| matches!(e, Entity::Nurbs(_)));
    if curves(&doc.entities) || doc.blocks.iter().any(|b| curves(&b.entities)) {
        return SCHEMA_WITH_NURBS;
    }
    // Schema 46 (docs/adr/0258): a closed or a wide polyline, a block definition's too.
    let extras = |list: &[Entity]| {
        list.iter().any(|e| match e {
            Entity::Polyline(p) => p.closed || p.widths.is_some(),
            _ => false,
        })
    };
    if extras(&doc.entities) || doc.blocks.iter().any(|b| extras(&b.entities)) {
        return SCHEMA_WITH_POLYLINE_EXTRAS;
    }
    // Schema 45 (docs/adr/0253 §6): the requests the drawing executed.
    if !doc.requests.is_empty() {
        return SCHEMA_WITH_REQUESTS;
    }
    // Schema 44 (docs/adr/0251): the project's sheet book.
    if doc.sheets.is_some() {
        return SCHEMA_WITH_SHEETS;
    }
    // Schema 43 (docs/adr/0249 §4): a dimension's references (the drawing's only; a block
    // definition's are refused).
    if doc
        .entities
        .iter()
        .any(|e| matches!(e, Entity::Dimension(d) if d.refs.is_some()))
    {
        return SCHEMA_WITH_DIMENSION_REFS;
    }
    // Schema 42 (docs/adr/0229 §2): the project's address data.
    if !doc.settings.address_locators.is_empty() {
        return SCHEMA_WITH_ADDRESSES;
    }
    // Schema 41 (docs/adr/0226 §3, §4): a layer's composition.
    if composed(&doc.layers) {
        return SCHEMA_WITH_COMPOSE;
    }
    // Schema 40 (docs/adr/0225): an object's attachment, a layer's action or card.
    if doc
        .entities
        .iter()
        .any(|e| !e.base().attachments.is_empty())
        || informed(&doc.layers)
    {
        return SCHEMA_WITH_ATTACHMENTS;
    }
    // Schema 39 (docs/adr/0224): a vertex measure of an object of the drawing or of a block definition.
    if doc.entities.iter().any(has_measure)
        || doc
            .blocks
            .iter()
            .any(|b| b.entities.iter().any(has_measure))
    {
        return SCHEMA_WITH_MEASURES;
    }
    // Schema 38 (docs/adr/0243): a raster showing a NetCDF variable, or a mesh's lines.
    if doc.entities.iter().any(|e| match e {
        Entity::Raster(r) => r.raster.dataset.is_some() || r.raster.style.edges.is_some(),
        _ => false,
    }) {
        return SCHEMA_WITH_MULTIDIM;
    }
    // Schema 37 (docs/adr/0214): the project's variables.
    if !doc.settings.variables.is_empty() {
        return SCHEMA_WITH_VARIABLES;
    }
    // Schema 36 (docs/adr/0212): the label engine's fields, a layer's labelling, an object's pins (a layer
    // state's style counts too).
    if labelled(&doc.layers)
        || doc.entities.iter().any(|e| !e.base().label_pins.is_empty())
        || doc
            .settings
            .layer_states
            .iter()
            .flat_map(|s| &s.nodes)
            .any(|n| {
                n.style.as_ref().is_some_and(|st| {
                    st.labels.is_some() || st.label.as_ref().is_some_and(labels::has_engine_fields)
                })
            })
    {
        return SCHEMA_WITH_LABELS;
    }
    // Schema 35 (docs/adr/0211): a layer's filter.
    if filters(&doc.layers) {
        return SCHEMA_WITH_FILTERS;
    }
    // Schema 34 (docs/adr/0210): a layer's time, a scenario, a scenario layer's base layer.
    if temporal(&doc.layers) {
        return SCHEMA_WITH_TEMPORAL;
    }
    // Schema 33 (docs/adr/0209): the project's networks.
    if !doc.settings.networks.is_empty() {
        return SCHEMA_WITH_NETWORKS;
    }
    // Schema 32 (docs/adr/0208): a layer drawn from a service, a layer's
    // feed, or the project's connections.
    if services(&doc.layers) || !doc.settings.connections.is_empty() {
        return SCHEMA_WITH_SERVICES;
    }
    // Schema 31 (docs/adr/0207): a point cloud, or a raster read from an address; the drawing's only.
    if doc.entities.iter().any(|e| match e {
        Entity::PointCloud(_) => true,
        Entity::Raster(r) => r.raster.url.is_some(),
        _ => false,
    }) {
        return SCHEMA_WITH_POINT_CLOUDS;
    }
    // Schema 30 (docs/adr/0205): annotation heights, a dimension's lines, a
    // leader's arrowhead size or one of AutoCAD's arrowheads.
    let annotated = |list: &[Entity]| {
        list.iter().any(|e| match e {
            Entity::Dimension(d) => d.look.has_lines(),
            Entity::Leader(l) => l.arrow_size.is_some() || l.arrow.is_some_and(|a| a.is_added()),
            _ => false,
        })
    };
    if has_heights(&doc.settings)
        || doc
            .settings
            .dimension_styles
            .iter()
            .any(DimensionStyleDef::has_lines)
        || annotated(&doc.entities)
        || doc.blocks.iter().any(|b| annotated(&b.entities))
    {
        return SCHEMA_WITH_ANNOTATION;
    }
    fn schemas(nodes: &[LayerNode]) -> bool {
        nodes
            .iter()
            .any(|n| !n.fields.is_empty() || schemas(&n.children))
    }
    // A raster is the drawing's only (docs/adr/0204 §2).
    if doc.entities.iter().any(|e| matches!(e, Entity::Raster(_))) {
        return SCHEMA_WITH_RASTERS;
    }
    if doc
        .settings
        .survey
        .as_ref()
        .is_some_and(SurveySettings::has_sigmas)
    {
        return SCHEMA_WITH_SURVEY_SIGMAS;
    }
    if doc.settings.topology.is_some() {
        return SCHEMA_WITH_TOPOLOGY;
    }
    if schemas(&doc.layers) {
        return SCHEMA_WITH_LAYER_FIELDS;
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
    let styled = |list: &[Entity]| {
        list.iter().any(|e| match e {
            Entity::Text(t) => !t.face.is_plain(),
            Entity::Dimension(d) => !d.look.is_plain(),
            _ => false,
        })
    };
    let patterned = |list: &[Entity]| {
        list.iter().any(
            |e| matches!(e, Entity::Hatch(h) if h.pattern.has_definition() || h.assoc.is_some()),
        )
    };
    let curved = |list: &[Entity]| {
        list.iter()
            .any(|e| matches!(e, Entity::Text(t) if t.path.is_some()))
    };
    if curved(&doc.entities) || doc.blocks.iter().any(|b| curved(&b.entities)) {
        return SCHEMA_WITH_TEXT_PATHS;
    }
    // A picture is the drawing's only (docs/adr/0192 §1).
    if doc.entities.iter().any(|e| matches!(e, Entity::Image(_))) {
        return SCHEMA_WITH_IMAGES;
    }
    if patterned(&doc.entities) || doc.blocks.iter().any(|b| patterned(&b.entities)) {
        return SCHEMA_WITH_HATCH_PATTERNS;
    }
    if doc.entities.iter().any(|e| matches!(e, Entity::Table(_))) {
        return SCHEMA_WITH_TABLES;
    }
    if !doc.settings.text_styles.is_empty()
        || !doc.settings.dimension_styles.is_empty()
        || styled(&doc.entities)
        || doc.blocks.iter().any(|b| styled(&b.entities))
    {
        return SCHEMA_WITH_STYLES;
    }
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

/// Whether an object has a vertex measure (a line's end, a polyline's or a
/// part's `ms`, even one whose every vertex is without): schema 39's fields.
fn has_measure(e: &Entity) -> bool {
    match e {
        Entity::Line(l) => l.ma.is_some() || l.mb.is_some(),
        Entity::Polyline(p) | Entity::Polygon(p) => {
            p.ms.is_some()
                || p.parts
                    .as_ref()
                    .is_some_and(|parts| parts.iter().any(|q| q.ms.is_some()))
        }
        _ => false,
    }
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

/// A split policy's name in the file (docs/adr/0419).
pub(crate) fn split_name(p: kentos_contracts::SplitPolicy) -> &'static str {
    match p {
        kentos_contracts::SplitPolicy::Copy => "copy",
        kentos_contracts::SplitPolicy::Clear => "clear",
        kentos_contracts::SplitPolicy::Default => "default",
        kentos_contracts::SplitPolicy::Ratio => "ratio",
    }
}

/// A merge policy's name in the file (docs/adr/0419).
pub(crate) fn merge_name(p: kentos_contracts::MergePolicy) -> &'static str {
    match p {
        kentos_contracts::MergePolicy::First => "first",
        kentos_contracts::MergePolicy::Clear => "clear",
        kentos_contracts::MergePolicy::Sum => "sum",
        kentos_contracts::MergePolicy::Min => "min",
        kentos_contracts::MergePolicy::Max => "max",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fixed_keys_are_in_encoded_order() {
        // The maps whose keys the writer writes by hand, in the order it writes them.
        let maps: [&[&str]; 31] = [
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
                "forms",
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
                "id", "name", "snap", "type", "style", "fields", "locked", "visible", "children",
                "expanded",
            ],
            &["off", "kinds"],
            // A layer's field and its value list's entry (docs/adr/0199 §1).
            &[
                "id", "max", "min", "kind", "name", "unit", "alias", "merge", "scale", "split",
                "domain", "length", "unique", "values", "default", "multiple", "required",
                "maxExclusive", "minExclusive",
            ],
            &["code", "from", "label", "until", "retired", "description"],
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
            // A polygon's part (docs/adr/0143), a polyline's with its measures (docs/adr/0224).
            &["ms", "zs", "pts", "holes", "bulges"],
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
