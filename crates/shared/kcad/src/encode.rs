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
    DimensionStyle, DimensionStyleDef, DocumentSnapshotV2, Entity, HeightKind, HeightReference,
    LayerField, LayerNode, LayerNodeType, LayerSnap, LayerState, LayerStateNode, LayerStyle,
    LengthStyle, MigrationSource, ProjectSettings, SurveySettings, TopologyDecision, TopologyScope,
    TopologySettings, Vec2,
    Workspace, layer_fields_problem, layer_states_problem,
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
    objects: Objs<'_>,
    watch: &mut dyn Watch,
) -> Result<Vec<u8>, KcadError> {
    let mut e = Encoder {
        w: Writer::default(),
        path: Vec::with_capacity(16),
        depth: 0,
        watch,
        objs: objects,
    };
    e.root(doc)?;
    Ok(e.w.out)
}

/// A drawing's objects as the writer reads them: the snapshot's own, or the open document's shared ones, which a save
/// does not copy (docs/adr/0654).
#[derive(Clone, Copy)]
pub(crate) enum Objs<'a> {
    Plain(&'a [Entity]),
    Shared(&'a [std::sync::Arc<Entity>]),
}

impl<'a> Objs<'a> {
    pub(crate) fn len(self) -> usize {
        match self {
            Objs::Plain(l) => l.len(),
            Objs::Shared(l) => l.len(),
        }
    }
    pub(crate) fn get(self, i: usize) -> Option<&'a Entity> {
        match self {
            Objs::Plain(l) => l.get(i),
            Objs::Shared(l) => l.get(i).map(|e| &**e),
        }
    }
    pub(crate) fn iter(self) -> impl Iterator<Item = &'a Entity> + Clone {
        (0..self.len()).filter_map(move |i| self.get(i))
    }
    /// `kentos_contracts::blocks::check` over these objects.
    fn blocks_check(
        self,
        blocks: &[kentos_contracts::BlockDefinition],
    ) -> Result<(), kentos_contracts::blocks::BlockFault> {
        match self {
            Objs::Plain(l) => kentos_contracts::blocks::check(blocks, l),
            Objs::Shared(l) => kentos_contracts::blocks::check_of(blocks, l),
        }
    }
    /// `kentos_contracts::service_links` over these objects.
    fn service_fault(
        self,
        layers: &[LayerNode],
        connections: &[kentos_contracts::ServiceConnection],
    ) -> Option<kentos_contracts::ServiceLinkFault> {
        match self {
            Objs::Plain(l) => kentos_contracts::service_links(layers, connections, l),
            Objs::Shared(l) => kentos_contracts::service_links_of(layers, connections, l),
        }
    }
}

struct Encoder<'d> {
    w: Writer,
    path: Vec<Seg<'d>>,
    depth: usize,
    watch: &'d mut dyn Watch,
    objs: Objs<'d>,
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
    /// A field's bound column (schema 133, docs/adr/0753): `made`, then (schema 134, docs/adr/0755) `check`, then
    /// `refers` (`label`, `table`, `column`).
    fn field_column(&mut self, c: &kentos_contracts::FieldColumn) -> Result<(), KcadError> {
        self.open(
            usize::from(c.made.is_some())
                + usize::from(c.check.is_some())
                + usize::from(c.refers.is_some()),
            true,
        )?;
        if let Some(made) = &c.made {
            self.key("made");
            self.at(Seg::Name("made"), |e| e.text(made))?;
        }
        if let Some(check) = &c.check {
            self.key("check");
            self.at(Seg::Name("check"), |e| e.text(check))?;
        }
        if let Some(r) = &c.refers {
            self.key("refers");
            self.at(Seg::Name("refers"), |e| {
                e.open(2 + usize::from(r.label.is_some()), true)?;
                if let Some(label) = &r.label {
                    e.key("label");
                    e.at(Seg::Name("label"), |e| e.text(label))?;
                }
                e.key("table");
                e.at(Seg::Name("table"), |e| e.text(&r.table))?;
                e.key("column");
                e.at(Seg::Name("column"), |e| e.text(&r.column))
            })?;
        }
        Ok(())
    }

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
        let schema = schema_of(doc, self.objs);
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
        if doc.uids.len() != self.objs.len() {
            return Err(self.fail(
                Code::BadValue,
                &format!(
                    "{} nesne ama {} kalıcı kimlik var; her nesnenin bir kimliği olmalı",
                    self.objs.len(),
                    doc.uids.len()
                ),
            ));
        }
        if let Err(fault) = self.objs.blocks_check(&doc.blocks) {
            let said = crate::blocks::said(&fault, &doc.blocks);
            self.path.push(Seg::Name(said.list));
            self.path.extend(said.path);
            return Err(self.fail(said.code, &said.what));
        }
        // Schema 32's links (docs/adr/0208 §2), as a reader checks them.
        if let Some(fault) = self
            .objs
            .service_fault(&doc.layers, &doc.settings.connections)
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
            + usize::from(!doc.dimension_sets.is_empty())
            + usize::from(!doc.casework.is_empty())
            + usize::from(!doc.constraints.is_empty());
        self.open(7 + optional, true)?;
        // name (4), arrays blocks layers origin sheets styles (6), casework entities homeView requests settings (8),
        // projectId (9), activeLayer constraints (11), migratedFrom (12), dimensionSets (13).
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
            if let Err(error) = kentos_contracts::shape_resource::check_items(&doc.styles.items) {
                return Err(e.fail(Code::BadValue, &error));
            }
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
        // Schema 127 (docs/adr/0727): the kitchen runs, written whole and held to the contract's rules.
        if !doc.casework.is_empty() {
            self.key("casework");
            self.at(Seg::Name("casework"), |e| {
                if let Some((index, why)) = kentos_contracts::casework_problem(&doc.casework) {
                    return Err(e.fail(Code::BadValue, &format!("[{index}]: {why}")));
                }
                let value = serde_json::to_value(&doc.casework)
                    .map_err(|err| e.fail(Code::BadValue, &err.to_string()))?;
                e.plain(&value)
            })?;
        }
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
                let uids: std::collections::HashSet<kentos_contracts::EntityId> =
                    doc.uids.iter().copied().collect();
                if let Some((at, problem)) =
                    kentos_contracts::constraints_problem(&doc.constraints, Some(&uids))
                {
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
        if !doc.dimension_sets.is_empty() {
            self.key("dimensionSets");
            self.at(Seg::Name("dimensionSets"), |e| {
                if let Some((index, why)) =
                    kentos_contracts::dimension_sets_problem(&doc.dimension_sets)
                {
                    return Err(e.fail(Code::BadValue, &format!("[{index}]: {why}")));
                }
                let value = serde_json::to_value(&doc.dimension_sets)
                    .map_err(|err| e.fail(Code::BadValue, &err.to_string()))?;
                e.plain(&value)
            })?;
        }
        self.close();
        Ok(())
    }

    fn settings(&mut self, s: &'d ProjectSettings) -> Result<(), KcadError> {
        let n = 6
            + usize::from(s.workspace.is_some())
            + usize::from(s.cad_work.is_some())
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
            + usize::from(!s.view_templates.is_empty())
            + usize::from(!s.forms.is_empty())
            + usize::from(!s.domains.is_empty())
            + usize::from(!s.relations.is_empty())
            + usize::from(!s.join_views.is_empty())
            + usize::from(!s.linear_events.is_empty())
            + usize::from(!s.profile_files.is_empty())
            + usize::from(!s.standard_docs.is_empty())
            + usize::from(!s.review_notes.is_empty())
            + usize::from(s.parcel_register.is_some())
            + usize::from(s.zoning_plan.is_some())
            + usize::from(!s.survey_adjustments.is_empty())
            + usize::from(!s.monitoring_reviews.is_empty())
            + usize::from(!s.approved_sheet_templates.is_empty())
            + usize::from(!s.table_styles.is_empty())
            + usize::from(!s.standard_exceptions.is_empty())
            + usize::from(!s.lineage.is_empty())
            + usize::from(s.map_srid.is_some())
            + usize::from(!s.sources.is_empty())
            + usize::from(!s.source_bases.is_empty())
            + usize::from(!s.line_types.is_empty())
            + usize::from(!s.hatch_patterns.is_empty())
            + usize::from(s.line_type_scale.is_some())
            + usize::from(s.auto_select.is_some())
            + usize::from(s.road.is_some())
            + usize::from(s.map_work.is_some())
            + usize::from(s.map_survey.is_some());
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
        // Schema 159 (plan/road.md Y2.2): a road project's terrain surfaces' definitions, as their JSON (after `arch`,
        // before `srid`); a road project's only.
        if let Some(v) = &s.road {
            self.key("road");
            self.at(Seg::Name("road"), |e| {
                if v.is_empty() {
                    return Err(e.fail(Code::BadValue, "boş yol ayarı yazılmaz"));
                }
                // A map project's terrain surfaces are written there too, nothing else of a road (docs/adr/1520).
                let map = s.cad_work == Some(kentos_contracts::CadWork::Map);
                if s.cad_work != Some(kentos_contracts::CadWork::Road) && !(map && v.map_drawn_only()) {
                    return Err(e.fail(
                        Code::BadValue,
                        if map {
                            "harita projesinde yol ayarlarından yalnız arazi yüzeyleri, platformlar ve onlardan yapılan çizimler yazılır (cadWork map)"
                        } else {
                            "yol ayarları yalnız yol projesinin olur (cadWork road); arazi yüzeyleri harita projesinin de olur"
                        },
                    ));
                }
                if let Some((field, words)) = v.problem() {
                    return Err(e.fail(Code::BadValue, &format!("{field}: {words}")));
                }
                e.json(&serde_json::to_value(v).unwrap_or_default())
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
        // Schema 135 (docs/adr/0801): what a CAD project is drawn for (the first of the 7-letter keys); a CAD
        // project's only.
        if let Some(w) = s.cad_work {
            self.key("cadWork");
            if s.workspace != Some(Workspace::Cad) {
                return Err(self.fail(
                    Code::BadValue,
                    "cadWork yalnız CAD projesinde (workspace cad) yazılır",
                ));
            }
            self.w.text(w.id());
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
        // Schema 101 (docs/adr/0598 §1): what the split and merged objects came from, written whole and checked.
        if !s.lineage.is_empty() {
            self.key("lineage");
            self.at(Seg::Name("lineage"), |e| {
                if let Some(problem) = kentos_contracts::lineage_problem(&s.lineage) {
                    return Err(e.fail(Code::BadValue, &problem));
                }
                let value = serde_json::to_value(&s.lineage)
                    .map_err(|err| e.fail(Code::BadValue, &err.to_string()))?;
                e.plain(&value)
            })?;
        }
        // Schema 98 (docs/adr/0583 §3): the map's system (between the 7-letter keys).
        if let Some(srid) = s.map_srid {
            self.key("mapSrid");
            if srid == 0 || srid == s.srid || s.srid == 0 {
                return Err(self.fail(
                    Code::BadValue,
                    "mapSrid yerel sistem ya da projenin kendi sistemi olamaz; projeninki için alan yazılmaz",
                ));
            }
            self.w.uint(u64::from(srid));
        }
        // Schema 188 (docs/adr/1200 §2): a map project's work structure, as its JSON (after mapSrid); a map project's
        // only.
        if let Some(v) = &s.map_work {
            self.key("mapWork");
            self.at(Seg::Name("mapWork"), |e| {
                if v.is_empty() {
                    return Err(e.fail(Code::BadValue, "boş harita işi yazılmaz"));
                }
                if s.cad_work != Some(kentos_contracts::CadWork::Map) {
                    return Err(e.fail(Code::BadValue, "harita işi yalnız harita projesinin olur (cadWork map)"));
                }
                if let Some((field, words)) = v.problem() {
                    return Err(e.fail(Code::BadValue, &format!("{field}: {words}")));
                }
                e.json(&serde_json::to_value(v).unwrap_or_default())
            })?;
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
        if !s.join_views.is_empty() {
            self.key("joinViews");
            self.at(Seg::Name("joinViews"), |e| {
                if let Some(problem) = kentos_contracts::join_views_problem(&s.join_views) {
                    return Err(e.fail(Code::BadValue, &problem));
                }
                let value = serde_json::to_value(&s.join_views)
                    .map_err(|err| e.fail(Code::BadValue, &err.to_string()))?;
                e.plain(&value)
            })?;
        }
        if !s.line_types.is_empty() {
            self.key("lineTypes");
            self.at(Seg::Name("lineTypes"), |e| e.line_types(&s.line_types))?;
        }
        // Schema 194 (docs/adr/1340 §5): a map project's GNSS archive, as its JSON (after lineTypes); a map project's
        // only.
        if let Some(v) = &s.map_survey {
            self.key("mapSurvey");
            self.at(Seg::Name("mapSurvey"), |e| {
                if v.is_empty() {
                    return Err(e.fail(Code::BadValue, "boş GNSS arşivi yazılmaz"));
                }
                if s.cad_work != Some(kentos_contracts::CadWork::Map) {
                    return Err(e.fail(Code::BadValue, "GNSS arşivi yalnız harita projesinin olur (cadWork map)"));
                }
                if let Some((field, words)) = v.problem() {
                    return Err(e.fail(Code::BadValue, &format!("{field}: {words}")));
                }
                e.json(&serde_json::to_value(v).unwrap_or_default())
            })?;
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
        // Schema 102 (docs/adr/0622 §5): Otomatik seçim, only when the project chose it.
        if let Some(on) = s.auto_select {
            self.key("autoSelect");
            self.w.bool(on);
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
        // Schema 250 (docs/adr/5040): a zoning plan's kept symbol choices and plan change drafts, as their JSON.
        if let Some(data) = &s.zoning_plan {
            self.key("zoningPlan");
            self.at(Seg::Name("zoningPlan"), |e| {
                if data.is_empty() {
                    return Err(e.fail(Code::BadValue, "boş imar planı verisi yazılmaz"));
                }
                if let Some(problem) = kentos_contracts::zoning_plan_problem(data) {
                    return Err(e.fail(Code::BadValue, &problem));
                }
                let value = serde_json::to_value(data).map_err(|err| e.fail(Code::BadValue, &err.to_string()))?;
                e.plain(&value)
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
        // Schema 180 (docs/adr/1000 §2): the review notes, as their JSON.
        if !s.review_notes.is_empty() {
            self.key("reviewNotes");
            self.at(Seg::Name("reviewNotes"), |e| {
                if let Some(problem) = kentos_contracts::review_notes_problem(&s.review_notes) {
                    return Err(e.fail(Code::BadValue, &problem));
                }
                let value = serde_json::to_value(&s.review_notes)
                    .map_err(|err| e.fail(Code::BadValue, &err.to_string()))?;
                e.plain(&value)
            })?;
        }
        // Schema 91 (docs/adr/0538 §1): the bound layers' bases.
        if !s.source_bases.is_empty() {
            self.key("sourceBases");
            self.at(Seg::Name("sourceBases"), |e| {
                if let Some(problem) = kentos_contracts::bases_problem(&s.source_bases, &s.sources)
                {
                    return Err(e.fail(Code::BadValue, &problem));
                }
                let value = serde_json::to_value(&s.source_bases)
                    .map_err(|err| e.fail(Code::BadValue, &err.to_string()))?;
                e.plain(&value)
            })?;
        }
        // Schema 185 (docs/adr/1126 §2): the table styles, as their JSON (after sourceBases: the keys' canonical order).
        if !s.table_styles.is_empty() {
            self.key("tableStyles");
            self.at(Seg::Name("tableStyles"), |e| {
                if let Some(problem) = kentos_contracts::table_style_defs_problem(&s.table_styles) {
                    return Err(e.fail(Code::BadValue, &problem));
                }
                let value = serde_json::to_value(&s.table_styles)
                    .map_err(|err| e.fail(Code::BadValue, &err.to_string()))?;
                e.plain(&value)
            })?;
        }
        self.key("areaDecimals");
        self.w.uint(u64::from(s.area_decimals));
        // Schema 67 (docs/adr/0326 §4): the leader styles, as their JSON.
        if !s.leader_styles.is_empty() {
            self.key("leaderStyles");
            self.at(Seg::Name("leaderStyles"), |e| {
                if let Some((field, words)) = s
                    .leader_styles
                    .iter()
                    .find_map(kentos_contracts::LeaderStyleDef::problem)
                {
                    return Err(e.fail(Code::BadValue, &format!("{field}: {words}")));
                }
                e.json(&serde_json::to_value(&s.leader_styles).unwrap_or_default())
            })?;
        }
        // Schema 97 (docs/adr/0569): the project's links of event layers to their routes (written between the
        // 12-letter keys: canonical order is by length, then bytes).
        if !s.linear_events.is_empty() {
            self.key("linearEvents");
            self.at(Seg::Name("linearEvents"), |e| {
                if let Some(problem) = kentos_contracts::event_links_problem(&s.linear_events) {
                    return Err(e.fail(Code::BadValue, &problem));
                }
                let value = serde_json::to_value(&s.linear_events)
                    .map_err(|err| e.fail(Code::BadValue, &err.to_string()))?;
                e.plain(&value)
            })?;
        }
        // Schema 132 (docs/adr/0735): the data profiles and mappings the project carries, each its document as text
        // (after `linearEvents`, the last of the 12-letter keys).
        if !s.profile_files.is_empty() {
            self.key("profileFiles");
            self.at(Seg::Name("profileFiles"), |e| {
                if let Some(problem) = kentos_contracts::profile_files_problem(&s.profile_files) {
                    return Err(e.fail(Code::BadValue, &problem));
                }
                let value = serde_json::to_value(&s.profile_files)
                    .map_err(|err| e.fail(Code::BadValue, &err.to_string()))?;
                e.plain(&value)
            })?;
        }
        if !s.standard_docs.is_empty() {
            self.key("standardDocs");
            self.at(Seg::Name("standardDocs"), |e| {
                if let Some(problem) = kentos_contracts::project_standards_problem(&s.standard_docs) {
                    return Err(e.fail(Code::BadValue, &problem));
                }
                let value = serde_json::to_value(&s.standard_docs)
                    .map_err(|err| e.fail(Code::BadValue, &err.to_string()))?;
                e.plain(&value)
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
        // Schema 146 (docs/adr/0783 §2): the project's view templates, as their JSON; "viewTemplates" sorts after
        // "lineTypeScale".
        if !s.view_templates.is_empty() {
            self.key("viewTemplates");
            self.at(Seg::Name("viewTemplates"), |e| {
                if let Some(problem) = kentos_contracts::templates_problem(&s.view_templates, &s.named_views) {
                    return Err(e.fail(Code::BadValue, &problem));
                }
                e.json(&serde_json::to_value(&s.view_templates).unwrap_or_default())
            })?;
        }
        self.key("lengthDecimals");
        self.w.uint(u64::from(s.length_decimals));
        // Schema 216 (docs/adr/4220): the parcel register, as its JSON.
        if let Some(register) = &s.parcel_register {
            self.key("parcelRegister");
            self.at(Seg::Name("parcelRegister"), |e| {
                if register.is_empty() {
                    return Err(e.fail(Code::BadValue, "boş parsel kayıtları yazılmaz"));
                }
                if let Some(problem) = kentos_contracts::parcel_register_problem(register) {
                    return Err(e.fail(Code::BadValue, &problem));
                }
                let value =
                    serde_json::to_value(register).map_err(|err| e.fail(Code::BadValue, &err.to_string()))?;
                e.plain(&value)
            })?;
        }
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
                    return Err(e.fail(
                        Code::BadValue,
                        "koordinat epoku bir koordinat sistemi ister",
                    ));
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
        // Schema 253 (docs/adr/5220): the monitoring reviews the project keeps, as their JSON (17 letters: before
        // surveyAdjustments, bytewise).
        if !s.monitoring_reviews.is_empty() {
            self.key("monitoringReviews");
            self.at(Seg::Name("monitoringReviews"), |e| {
                if let Some(problem) = kentos_contracts::monitoring_reviews_problem(&s.monitoring_reviews) {
                    return Err(e.fail(Code::BadValue, &problem));
                }
                let value =
                    serde_json::to_value(&s.monitoring_reviews).map_err(|err| e.fail(Code::BadValue, &err.to_string()))?;
                e.plain(&value)
            })?;
        }
        // Schema 252 (docs/adr/5200): the adjustments the project keeps, as their JSON.
        if !s.survey_adjustments.is_empty() {
            self.key("surveyAdjustments");
            self.at(Seg::Name("surveyAdjustments"), |e| {
                if let Some(problem) = kentos_contracts::kept_adjustments_problem(&s.survey_adjustments) {
                    return Err(e.fail(Code::BadValue, &problem));
                }
                let value =
                    serde_json::to_value(&s.survey_adjustments).map_err(|err| e.fail(Code::BadValue, &err.to_string()))?;
                e.plain(&value)
            })?;
        }
        // Schema 190 (docs/adr/1129): the accepted standards findings, as their JSON (18 letters: before the longest).
        if !s.standard_exceptions.is_empty() {
            self.key("standardExceptions");
            self.at(Seg::Name("standardExceptions"), |e| {
                if let Some(problem) = kentos_contracts::standard_exceptions_problem(&s.standard_exceptions) {
                    return Err(e.fail(Code::BadValue, &problem));
                }
                let value = serde_json::to_value(&s.standard_exceptions)
                    .map_err(|err| e.fail(Code::BadValue, &err.to_string()))?;
                e.plain(&value)
            })?;
        }
        // Schema 183 (docs/adr/1100 §3): the approved sheet template revisions, as their JSON (the longest key, last).
        if !s.approved_sheet_templates.is_empty() {
            self.key("approvedSheetTemplates");
            self.at(Seg::Name("approvedSheetTemplates"), |e| {
                if let Some(problem) = kentos_contracts::approved_sheet_templates_problem(&s.approved_sheet_templates) {
                    return Err(e.fail(Code::BadValue, &problem));
                }
                let value = serde_json::to_value(&s.approved_sheet_templates)
                    .map_err(|err| e.fail(Code::BadValue, &err.to_string()))?;
                e.plain(&value)
            })?;
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
/// Whether a surface of the road settings pastes another (docs/adr/3920).
pub(crate) fn road_has_pastes(r: &kentos_contracts::RoadSettings) -> bool {
    r.surfaces.iter().any(|s| s.edits.iter().any(|e| matches!(e, kentos_contracts::SurfaceEdit::Paste { .. })))
}

fn has_heights(s: &ProjectSettings) -> bool {
    s.annotation
        .as_ref()
        .is_some_and(|a| *a != AnnotationHeights::default())
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
                "id",
                "max",
                "min",
                "kind",
                "name",
                "unit",
                "alias",
                "merge",
                "scale",
                "split",
                "domain",
                "length",
                "unique",
                "values",
                "default",
                "multiple",
                "required",
                "withheld",
                "maxExclusive",
                "minExclusive",
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

mod schema;
pub(crate) use schema::*;

mod layers;

mod map;

mod gis;
