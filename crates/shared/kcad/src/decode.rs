//! Document schemas 2 to 6 read from a payload (docs/specs/kcad-v2.md §6),
//! straight into the contract (`DocumentSnapshotV2`): item by item, no
//! intermediate tree, so memory follows the drawing, not what a file claims.
//! Every map is checked for its keys (unknown ones are refused, required ones
//! must be there) and every value for its type; the objects get the slots
//! 1, 2, 3 … in file order. The objects themselves are read in `objects.rs`,
//! the block definitions in `blocks.rs`.

mod addresses;
mod arrays;
mod blocks;
mod compose;
mod crs;
mod filter;
mod info;
mod labels;
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
    AngleUnit, AnnotationHeights, AnnotationKind, AreaUnit, Bounds, CadWork, ColumnRefers,
    DOCUMENT_FORMAT, DOCUMENT_VERSION, DOCUMENT_VERSION_2, DocumentSnapshotV2, DrawingFont,
    DrawingUnit, FieldChoice, FieldColumn, GeoidModel, HeightKind, HeightReference, LayerField,
    LayerFieldKind, LayerNode, LayerNodeType, LayerSnap, LayerState, LayerStateNode, LayerStyle,
    LengthStyle, LineType, MAX_ANNOTATION_MM, MigrationSource, PointStyle, PointSymbol, ProjectId,
    ProjectSettings, ProjectStyles, SurveySettings, TopologyException, TopologyRule,
    TopologyRuleKind, TopologySettings, Vec2, Workspace, annotation_mm_holds, layer_fields_problem,
    layer_states_problem,
};

use crate::SCHEMAS;
use crate::cbor::{Any, Reader, Seg};
use crate::error::{Code, KcadError};
use crate::watch::Watch;
use objects::{Features, objects};

/// The most items a list reserves before they are read.
const PREALLOCATE: usize = 4096;

/// The drawing in a payload whose container was checked; `watch` hears the
/// project before its objects, then the objects as they are read, and may
/// stop the reading (docs/adr/0030).
pub(crate) fn payload(data: &[u8], watch: &mut dyn Watch) -> Result<DocumentSnapshotV2, KcadError> {
    let mut r = Reader::watched(data, watch);
    read_whole(&mut r)
}

/// What a verifying read does with each object instead of keeping it (docs/adr/0654).
pub(crate) trait Check {
    /// `None` when object `i` as read is the one written, otherwise where it differs.
    fn object(
        &mut self,
        i: usize,
        entity: &kentos_contracts::Entity,
        uid: &kentos_contracts::EntityId,
    ) -> Option<String>;
}

/// `payload` checking each object with `check` as it is read: the drawing returned has no objects (their ids are
/// kept), so a large drawing is not in memory a second time. Every rule of the reading holds but the one that needs
/// the objects together, a service layer's objects, which the writer checked before writing.
pub(crate) fn payload_checked(
    data: &[u8],
    watch: &mut dyn Watch,
    check: &mut dyn Check,
) -> Result<DocumentSnapshotV2, KcadError> {
    let mut r = Reader::checking(data, watch, check);
    read_whole(&mut r)
}

fn read_whole(r: &mut Reader<'_>) -> Result<DocumentSnapshotV2, KcadError> {
    let doc = root(r)?;
    if !r.at_end() {
        return Err(r.fail(Code::CborTrailing, "yükte belgeden sonra fazladan bayt var"));
    }
    Ok(doc)
}

/// A value that must be there, or the missing field's error.
fn required<T>(r: &mut Reader<'_>, value: Option<T>, key: &'static str) -> Result<T, KcadError> {
    value.ok_or_else(|| {
        r.push(Seg::Name(key));
        let e = r.fail(Code::MissingField, "zorunlu alan yok");
        r.pop();
        e
    })
}

fn unknown(r: &mut Reader<'_>) -> KcadError {
    r.fail(Code::UnknownField, "bilinmeyen alan")
}

/// Reads a map's pairs: `each(r, key)` reads the value of one key (the key is on the path).
fn map<'a>(
    r: &mut Reader<'a>,
    mut each: impl FnMut(&mut Reader<'a>, &'a str) -> Result<(), KcadError>,
) -> Result<(), KcadError> {
    let (n, _) = r.map()?;
    let mut previous = None;
    for _ in 0..n {
        let key = r.key(&mut previous)?;
        r.push(Seg::Key(key));
        each(r, key)?;
        r.pop();
    }
    r.leave();
    Ok(())
}

fn list<'a, T>(
    r: &mut Reader<'a>,
    mut item: impl FnMut(&mut Reader<'a>, usize) -> Result<T, KcadError>,
) -> Result<Vec<T>, KcadError> {
    let n = r.array()?;
    // `array` checked that n items can fit the bytes left, but an item in memory
    // may be larger than its smallest encoding (an object, a JSON value): only a
    // little is reserved ahead, the rest grows with the items actually read.
    let mut out = Vec::with_capacity(n.min(PREALLOCATE));
    for i in 0..n {
        r.push(Seg::Index(i));
        out.push(item(r, i)?);
        r.pop();
    }
    r.leave();
    Ok(out)
}

fn text(r: &mut Reader<'_>) -> Result<String, KcadError> {
    r.text().map(str::to_owned)
}

/// A value of an enumerated text: `values` pairs the written text with the value.
fn named<T: Copy>(r: &mut Reader<'_>, values: &[(&str, T)]) -> Result<T, KcadError> {
    let at = r.position();
    let t = r.text()?;
    values
        .iter()
        .find(|(name, _)| *name == t)
        .map(|&(_, v)| v)
        .ok_or_else(|| {
            let known: Vec<&str> = values.iter().map(|(n, _)| *n).collect();
            r.fail_at(
                Code::BadValue,
                at,
                &format!("“{t}” bilinmiyor ({})", known.join(", ")),
            )
        })
}

/// `[x, y]` (§6.3).
fn point(r: &mut Reader<'_>) -> Result<Vec2, KcadError> {
    let at = r.position();
    let n = r.array()?;
    if n != 2 {
        return Err(r.fail_at(Code::BadValue, at, &format!("nokta 2 sayı olmalı, {n} var")));
    }
    let x = r.float()?;
    let y = r.float()?;
    r.leave();
    Ok(Vec2 { x, y })
}

fn points(r: &mut Reader<'_>) -> Result<Vec<Vec2>, KcadError> {
    list(r, |r, _| point(r))
}

fn floats(r: &mut Reader<'_>) -> Result<Vec<f64>, KcadError> {
    list(r, |r, _| r.float())
}

/// `[minX, minY, maxX, maxY]` (§6.3).
fn bounds(r: &mut Reader<'_>) -> Result<Bounds, KcadError> {
    let at = r.position();
    let n = r.array()?;
    if n != 4 {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            &format!("sınırlar 4 sayı olmalı, {n} var"),
        ));
    }
    let b = Bounds {
        min_x: r.float()?,
        min_y: r.float()?,
        max_x: r.float()?,
        max_y: r.float()?,
    };
    r.leave();
    Ok(b)
}

/// 16 bytes, not all zero (§6.3).
fn id16(r: &mut Reader<'_>) -> Result<[u8; 16], KcadError> {
    let (raw, at) = r.bytes()?;
    let Ok(id) = <[u8; 16]>::try_from(raw) else {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            &format!("{} bayt; kimlik 16 bayt olmalı", raw.len()),
        ));
    };
    if id == [0; 16] {
        return Err(r.fail_at(Code::BadValue, at, "kimlik boş (sıfır) olamaz"));
    }
    Ok(id)
}

// ── The root and the document ───────────────────────────────────────────

fn root(r: &mut Reader<'_>) -> Result<DocumentSnapshotV2, KcadError> {
    let not_ours = |r: &Reader<'_>| {
        r.fail(
            Code::SchemaFormat,
            &format!("yük bir KentOS çizimi değil (format ≠ {DOCUMENT_FORMAT})"),
        )
    };
    let version_error = |r: &Reader<'_>, v: &str| {
        let known: Vec<String> = SCHEMAS.iter().map(u32::to_string).collect();
        r.fail(
            Code::SchemaVersion,
            &format!(
                "belge şeması sürümü {v} bu uygulamada okunamıyor (desteklenen: {}); KentOS'u güncelleyin",
                known.join(", ")
            ),
        )
    };
    let mut format_ok = false;
    let mut schema: Option<u32> = None;
    let mut document = None;
    map(r, |r, key| {
        match key {
            "format" => {
                if !matches!(r.any()?, Any::Text(DOCUMENT_FORMAT)) {
                    return Err(not_ours(r));
                }
                format_ok = true;
            }
            "version" => match r.any()? {
                Any::Uint(v) => match SCHEMAS.iter().find(|&&s| u64::from(s) == v) {
                    Some(&known) => schema = Some(known),
                    None => return Err(version_error(r, &v.to_string())),
                },
                _ => return Err(version_error(r, "(sayı değil)")),
            },
            "document" => {
                if !format_ok {
                    return Err(not_ours(r));
                }
                let Some(schema) = schema else {
                    return Err(version_error(r, "(yok)"));
                };
                document = Some(body(r, schema)?);
            }
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    if !format_ok {
        return Err(not_ours(r));
    }
    if schema.is_none() {
        return Err(version_error(r, "(yok)"));
    }
    required(r, document, "document")
}

/// The document; `schema` is its payload's (2, 3 with objects' line weights,
/// 4 with vertex elevations too, 5 with areas' parts, 6 with blocks).
fn body(r: &mut Reader<'_>, schema: u32) -> Result<DocumentSnapshotV2, KcadError> {
    let has = Features::of(schema);
    let mut name = None;
    let mut blocks: Option<blocks::Definitions> = None;
    let mut layers = None;
    let mut origin = None;
    let mut styles = None;
    let mut entities = None;
    let mut home_view = None;
    let mut settings_ = None;
    let mut project_id = None;
    let mut active_layer = None;
    let mut migrated_from = None;
    let mut sheets_ = None;
    let mut requests_ = None;
    let mut arrays_ = None;
    let mut constraints_ = None;
    let mut dimension_sets_ = None;
    let mut casework_ = None;
    map(r, |r, key| {
        match key {
            "name" => name = Some(text(r)?),
            "arrays" if has.arrays => arrays_ = Some(arrays::arrays(r)?),
            "blocks" if has.blocks => blocks = Some(blocks::definitions(r, has)?),
            "layers" => layers = Some(list(r, |r, _| layer(r, has))?),
            "origin" => origin = Some(point(r)?),
            "sheets" if has.sheets => sheets_ = Some(sheets::sheets(r)?),
            "styles" => styles = Some(project_styles(r)?),
            // Name, blocks and layers come first in the encoded order: the project and its
            // blocks are known before its objects.
            "entities" => {
                let none = Default::default();
                let (list, index) = blocks
                    .as_ref()
                    .map_or((&[][..], &none), |(l, i)| (&l[..], i));
                entities = Some(objects(
                    r,
                    name.as_deref(),
                    layers.as_ref().map_or(0, Vec::len),
                    has,
                    list,
                    index,
                )?)
            }
            "casework" if has.casework => casework_ = Some(casework(r)?),
            "homeView" => home_view = Some(bounds(r)?),
            "requests" if has.requests => requests_ = Some(requests::requests(r)?),
            "settings" => settings_ = Some(settings(r, has)?),
            "projectId" => project_id = Some(ProjectId(id16(r)?)),
            "activeLayer" => active_layer = Some(text(r)?),
            "constraints" if has.constraints => constraints_ = Some(constraints(r)?),
            "dimensionSets" if has.dimension_sets => dimension_sets_ = Some(dimension_sets(r, has)?),
            "migratedFrom" => migrated_from = Some(source(r)?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    let name = required(r, name, "name")?;
    let layers = required(r, layers, "layers")?;
    let origin = required(r, origin, "origin")?;
    let styles = required(r, styles, "styles")?;
    let (entities, uids) = required(r, entities, "entities")?;
    let settings = required(r, settings_, "settings")?;
    let active_layer = required(r, active_layer, "activeLayer")?;
    service_links(r, &layers, &settings, &entities)?;
    scenario_tree(r, &layers)?;
    filter_tree(r, &layers)?;
    label_tree(r, &layers)?;
    info_tree(r, &layers)?;
    compose_tree(r, &layers)?;
    let blocks = blocks.map(|(list, _)| list).unwrap_or_default();
    let arrays = arrays_.unwrap_or_default();
    if !arrays.is_empty() {
        let ids: Vec<_> = blocks.iter().map(|b| b.id).collect();
        arrays::check(r, &arrays, &ids, &uids)?;
    }
    let constraints = match constraints_ {
        Some((at, set)) => {
            let known: std::collections::HashSet<kentos_contracts::EntityId> =
                uids.iter().copied().collect();
            if let Some((place, problem)) =
                kentos_contracts::constraints_problem(&set, Some(&known))
            {
                return Err(r.fail_at(Code::BadValue, at, &format!("{place}: {problem}")));
            }
            set
        }
        None => Default::default(),
    };
    Ok(DocumentSnapshotV2 {
        dimension_sets: dimension_sets_.unwrap_or_default(),
        casework: casework_.unwrap_or_default(),
        constraints,
        format: DOCUMENT_FORMAT.to_owned(),
        version: DOCUMENT_VERSION_2,
        name,
        settings,
        origin,
        home_view,
        layers,
        active_layer,
        entities,
        uids,
        styles,
        blocks,
        project_id,
        migrated_from,
        sheets: sheets_,
        requests: requests_.unwrap_or_default(),
        arrays,
    })
}

/// The kitchen runs (schema 127, docs/adr/0727 §5): the contract's JSON in the writer's canonical form, held to its
/// rules.
fn casework(r: &mut Reader<'_>) -> Result<Vec<kentos_contracts::CaseworkRecord>, KcadError> {
    let at = r.position();
    let value = r.opaque()?;
    let runs: Vec<kentos_contracts::CaseworkRecord> = serde_json::from_value(value.clone())
        .map_err(|e| r.fail_at(Code::BadValue, at, &format!("mutfak dizileri okunamadı: {e}")))?;
    if runs.is_empty() || serde_json::to_value(&runs).ok().as_ref() != Some(&value) {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            "mutfak dizisi listesi boş ya da yazıcının kanonik biçiminde değil",
        ));
    }
    if let Some((index, why)) = kentos_contracts::casework_problem(&runs) {
        return Err(r.fail_at(Code::BadValue, at, &format!("casework[{index}]: {why}")));
    }
    Ok(runs)
}

fn dimension_sets(
    r: &mut Reader<'_>,
    has: Features,
) -> Result<Vec<kentos_contracts::DimensionSet>, KcadError> {
    let at = r.position();
    let value = r.opaque()?;
    let sets: Vec<kentos_contracts::DimensionSet> =
        serde_json::from_value(value.clone()).map_err(|e| {
            r.fail_at(
                Code::BadValue,
                at,
                &format!("mimari ölçü dizileri okunamadı: {e}"),
            )
        })?;
    if sets.is_empty() || serde_json::to_value(&sets).ok().as_ref() != Some(&value) {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            "ölçü dizisi listesi boş veya yazıcının kanonik biçiminde değil",
        ));
    }
    if let Some((index, why)) = kentos_contracts::dimension_sets_problem(&sets) {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            &format!("dimensionSets[{index}]: {why}"),
        ));
    }
    if !has.dimension_set_edges
        && let Some(index) = sets.iter().position(|s| s.edge.is_some())
    {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            &format!("dimensionSets[{index}]: kenar (edge) şema 114 ister"),
        ));
    }
    if !has.dimension_set_spans
        && let Some(index) = sets.iter().position(|s| s.span.is_some())
    {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            &format!("dimensionSets[{index}]: cephe ya da mahal ölçüsü (span) şema 128 ister"),
        ));
    }
    if !has.dimension_set_scales
        && let Some(index) = sets.iter().position(|s| {
            !s.scales.is_empty()
                || !s.tight.is_keep()
                || s.members.iter().any(|m| m.shift.is_some())
        })
    {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            &format!(
                "dimensionSets[{index}]: pafta ölçekleri, sığmayan yazının yeri ya da üyenin yazı yeri şema 129 ister"
            ),
        ));
    }
    Ok(sets)
}

/// Schema 73's constraints (docs/adr/0334): the contract value read whole, exactly as the writer writes it (no
/// field at its default, not empty), and its rules that need no objects; the objects' ids are checked once known.
fn constraints(r: &mut Reader<'_>) -> Result<(usize, kentos_contracts::ConstraintSet), KcadError> {
    let at = r.position();
    let value = r.opaque()?;
    let set: kentos_contracts::ConstraintSet =
        serde_json::from_value(value.clone()).map_err(|e| {
            let words = e.to_string();
            let code = if words.contains("unknown field") || words.contains("unknown variant") {
                Code::UnknownField
            } else {
                Code::BadValue
            };
            r.fail_at(code, at, &format!("kısıtlar okunamadı: {words}"))
        })?;
    if serde_json::to_value(&set).ok().as_ref() != Some(&value) {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            "kısıtlar yazıcının biçiminde değil (varsayılan değerinde bir alan yazılmış)",
        ));
    }
    if set.is_empty() {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            "boş kısıt kümesi yazılmaz; kısıt yoksa alan hiç yazılmaz",
        ));
    }
    if let Some((place, problem)) = kentos_contracts::constraints_problem(&set, None) {
        return Err(r.fail_at(Code::BadValue, at, &format!("{place}: {problem}")));
    }
    Ok((at, set))
}

/// Schema 34's tree rules (docs/adr/0210 §2), as `scenarios_problem` says
/// them: a time on a layer that holds objects, scenarios not nested, a
/// scenario layer standing for one base layer.
fn scenario_tree(r: &mut Reader<'_>, layers: &[LayerNode]) -> Result<(), KcadError> {
    let Some(problem) = kentos_contracts::scenarios_problem(layers) else {
        return Ok(());
    };
    r.push(Seg::Name("layers"));
    let e = r.fail(Code::BadValue, &problem);
    r.pop();
    Err(e)
}

/// Schema 35's tree rule (docs/adr/0211 §2), as `filters_problem` says it:
/// a filter on a layer that holds objects.
fn filter_tree(r: &mut Reader<'_>, layers: &[LayerNode]) -> Result<(), KcadError> {
    let Some(problem) = kentos_contracts::filters_problem(layers) else {
        return Ok(());
    };
    r.push(Seg::Name("layers"));
    let e = r.fail(Code::BadValue, &problem);
    r.pop();
    Err(e)
}

/// Schema 40's tree rule (docs/adr/0225 §4, §5), as `infos_problem` says
/// it: actions and a card on a layer that holds objects.
fn info_tree(r: &mut Reader<'_>, layers: &[LayerNode]) -> Result<(), KcadError> {
    let Some(problem) = kentos_contracts::infos_problem(layers) else {
        return Ok(());
    };
    r.push(Seg::Name("layers"));
    let e = r.fail(Code::BadValue, &problem);
    r.pop();
    Err(e)
}

/// Schema 41's tree rule (docs/adr/0226 §3, §4), as `composes_problem` says
/// it: a composition on a layer, its mask cutting into other layers of the tree.
fn compose_tree(r: &mut Reader<'_>, layers: &[LayerNode]) -> Result<(), KcadError> {
    let Some(problem) = kentos_contracts::composes_problem(layers) else {
        return Ok(());
    };
    r.push(Seg::Name("layers"));
    let e = r.fail(Code::BadValue, &problem);
    r.pop();
    Err(e)
}

/// Schema 36's tree rule (docs/adr/0212 §2), as `labels_problem` says it:
/// no group labels.
fn label_tree(r: &mut Reader<'_>, layers: &[LayerNode]) -> Result<(), KcadError> {
    let Some(problem) = kentos_contracts::labels_problem(layers) else {
        return Ok(());
    };
    r.push(Seg::Name("layers"));
    let e = r.fail(Code::BadValue, &problem);
    r.pop();
    Err(e)
}

/// Schema 32's links (docs/adr/0208 §2), as `service_links` says them: a
/// service's and a feed's connection is one of the project's; a layer drawn
/// from a service holds no objects.
fn service_links(
    r: &mut Reader<'_>,
    layers: &[LayerNode],
    settings: &ProjectSettings,
    entities: &[kentos_contracts::Entity],
) -> Result<(), KcadError> {
    let Some(fault) = kentos_contracts::service_links(layers, &settings.connections, entities)
    else {
        return Ok(());
    };
    let segs = match fault {
        kentos_contracts::ServiceLinkFault::Object { index } => {
            vec![Seg::Name("entities"), Seg::Index(index)]
        }
        _ => vec![Seg::Name("layers")],
    };
    for s in &segs {
        r.push(*s);
    }
    let e = r.fail(Code::BadValue, &fault.words());
    for _ in &segs {
        r.pop();
    }
    Err(e)
}

/// The settings; `has` says whether the schema has the drawing unit (11 and
/// up), the second coordinate system (12 and up), the project's own
/// systems and datum choices (13 and up) and the survey settings (14).
fn settings(r: &mut Reader<'_>, has: Features) -> Result<ProjectSettings, KcadError> {
    let (mut srid, mut area_unit, mut angle_unit, mut plot_scale) = (None, None, None, None);
    let (mut drawing_unit, mut second_srid) = (None, None);
    let (mut custom_crs, mut second_custom_crs, mut datum_transforms) = (None, None, Vec::new());
    let mut survey = None;
    let mut layer_states = Vec::new();
    let (mut line_types, mut line_type_scale) = (Vec::new(), None);
    let mut hatch_patterns = Vec::new();
    let mut topology = None;
    let mut annotation = None;
    let mut connections = Vec::new();
    let mut networks = Vec::new();
    let mut variables = Vec::new();
    let mut address_locators = Vec::new();
    let (mut ucs, mut named_ucs) = (None, Vec::new());
    let mut named_views = Vec::new();
    let mut forms = Vec::new();
    let mut domains = Vec::new();
    let mut relations = Vec::new();
    let mut join_views = Vec::new();
    let mut linear_events = Vec::new();
    let mut profile_files = Vec::new();
    let mut lineage = Vec::new();
    let mut map_srid: Option<u32> = None;
    let mut auto_select: Option<bool> = None;
    let mut source_bases: Vec<kentos_contracts::SourceBase> = Vec::new();
    let mut sources = Vec::new();
    let mut length_style = None;
    let mut xref_paths = Vec::new();
    let (mut heights, mut coordinate_epoch) = (None, None);
    let (mut text_styles, mut dimension_styles) = (Vec::new(), Vec::new());
    let mut leader_styles = Vec::new();
    let mut wall_styles = Vec::new();
    let mut arch = None;
    let (mut workspace, mut drawing_font, mut area_decimals, mut length_decimals) =
        (None, None, None, None);
    let mut cad_work = None;
    map(r, |r, key| {
        match key {
            "srid" => srid = Some(r.uint(u64::from(u32::MAX))? as u32),
            "areaUnit" => {
                area_unit = Some(named(
                    r,
                    &[
                        ("m2", AreaUnit::M2),
                        ("donum", AreaUnit::Donum),
                        ("ha", AreaUnit::Ha),
                    ],
                )?)
            }
            "angleUnit" => {
                angle_unit = Some(named(
                    r,
                    &[("grad", AngleUnit::Grad), ("deg", AngleUnit::Deg)],
                )?)
            }
            "plotScale" => plot_scale = Some(r.float()?),
            // Schema 72 (docs/adr/0401 §3): the project's forms, read whole and checked.
            "forms" if has.forms => {
                let at = r.position();
                let value = r.opaque()?;
                let list: Vec<kentos_contracts::ProjectForm> =
                    serde_json::from_value(value.clone()).map_err(|e| {
                        let words = e.to_string();
                        let code = if words.contains("unknown field") {
                            Code::UnknownField
                        } else {
                            Code::BadValue
                        };
                        r.fail_at(code, at, &format!("formlar okunamadı: {words}"))
                    })?;
                // Read back exactly as the writer writes it: no field at its default.
                if serde_json::to_value(&list).ok().as_ref() != Some(&value) {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "formlar yazıcının biçiminde değil (varsayılan değerinde bir alan yazılmış)",
                    ));
                }
                if list.is_empty() {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "boş liste yazılmaz; form yoksa alan hiç yazılmaz",
                    ));
                }
                if let Some(problem) = kentos_contracts::forms_problem(&list) {
                    return Err(r.fail_at(Code::BadValue, at, &problem));
                }
                forms = list;
            }
            // Schema 72 (docs/adr/0430): the project's value lists, read whole and checked.
            "domains" if has.forms => {
                let at = r.position();
                let value = r.opaque()?;
                let list: Vec<kentos_contracts::FieldDomain> =
                    serde_json::from_value(value.clone()).map_err(|e| {
                        let words = e.to_string();
                        let code = if words.contains("unknown field") {
                            Code::UnknownField
                        } else {
                            Code::BadValue
                        };
                        r.fail_at(code, at, &format!("değer listeleri okunamadı: {words}"))
                    })?;
                if serde_json::to_value(&list).ok().as_ref() != Some(&value) {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "değer listeleri yazıcının biçiminde değil (varsayılan değerinde bir alan yazılmış)",
                    ));
                }
                if list.is_empty() {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "boş liste yazılmaz; değer listesi yoksa alan hiç yazılmaz",
                    ));
                }
                if let Some((_, problem)) = kentos_contracts::domains_problem(&list) {
                    return Err(r.fail_at(Code::BadValue, at, &problem));
                }
                domains = list;
            }
            // Schema 74 (docs/adr/0503): the layers bound to their sources, read whole and checked.
            "sources" if has.sources => {
                let at = r.position();
                let value = r.opaque()?;
                let list: Vec<kentos_contracts::SourceLink> = serde_json::from_value(value.clone())
                    .map_err(|e| {
                        let words = e.to_string();
                        let code = if words.contains("unknown field") {
                            Code::UnknownField
                        } else {
                            Code::BadValue
                        };
                        r.fail_at(code, at, &format!("kaynaklar okunamadı: {words}"))
                    })?;
                if serde_json::to_value(&list).ok().as_ref() != Some(&value) {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "kaynaklar yazıcının biçiminde değil (varsayılan değerinde bir alan yazılmış)",
                    ));
                }
                if list.is_empty() {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "boş liste yazılmaz; bağlı katman yoksa alan hiç yazılmaz",
                    ));
                }
                // Schema 134 (docs/adr/0755 §1): a link's CHECK and unique constraints.
                if !has.table_constraints
                    && list
                        .iter()
                        .any(|s| !s.checks.is_empty() || !s.uniques.is_empty())
                {
                    return Err(r.fail_at(
                        Code::UnknownField,
                        at,
                        "kaynakların checks ve uniques'i şema 134'ten (docs/adr/0755)",
                    ));
                }
                if let Some(problem) = kentos_contracts::sources_problem(&list, &[]) {
                    return Err(r.fail_at(Code::BadValue, at, &problem));
                }
                sources = list;
            }
            // Schema 91 (docs/adr/0538 §1): the bound layers' bases, read whole; checked against the links below.
            "sourceBases" if has.source_bases => {
                let at = r.position();
                let value = r.opaque()?;
                let list: Vec<kentos_contracts::SourceBase> = serde_json::from_value(value.clone())
                    .map_err(|e| {
                        let words = e.to_string();
                        let code = if words.contains("unknown field") {
                            Code::UnknownField
                        } else {
                            Code::BadValue
                        };
                        r.fail_at(code, at, &format!("kaynak tabanları okunamadı: {words}"))
                    })?;
                if list.is_empty() {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "boş liste yazılmaz; taban yoksa alan hiç yazılmaz",
                    ));
                }
                // Schema 92 (docs/adr/0553 §1): the field records are unknown before it.
                if !has.source_field_bases && list.iter().any(|b| b.fields.is_some()) {
                    return Err(r.fail_at(
                        Code::UnknownField,
                        at,
                        "kaynak tabanının alan kayıtları şema 92'den önce yok",
                    ));
                }
                source_bases = list;
            }
            // Schema 72 (docs/adr/0405 §1): the project's relations, read whole and checked.
            "relations" if has.forms => {
                let at = r.position();
                let value = r.opaque()?;
                let list: Vec<kentos_contracts::ProjectRelation> =
                    serde_json::from_value(value.clone()).map_err(|e| {
                        let words = e.to_string();
                        let code = if words.contains("unknown field") {
                            Code::UnknownField
                        } else {
                            Code::BadValue
                        };
                        r.fail_at(code, at, &format!("ilişkiler okunamadı: {words}"))
                    })?;
                if serde_json::to_value(&list).ok().as_ref() != Some(&value) {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "ilişkiler yazıcının biçiminde değil (varsayılan değerinde bir alan yazılmış)",
                    ));
                }
                if list.is_empty() {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "boş liste yazılmaz; ilişki yoksa alan hiç yazılmaz",
                    ));
                }
                if let Some(problem) = kentos_contracts::relations_problem(&list) {
                    return Err(r.fail_at(Code::BadValue, at, &problem));
                }
                relations = list;
            }
            // Schema 89 (docs/adr/0527): the project's join views, read whole and checked.
            "joinViews" if has.join_views => {
                let at = r.position();
                let value = r.opaque()?;
                let list: Vec<kentos_contracts::JoinView> = serde_json::from_value(value.clone())
                    .map_err(|e| {
                    let words = e.to_string();
                    let code = if words.contains("unknown field") {
                        Code::UnknownField
                    } else {
                        Code::BadValue
                    };
                    r.fail_at(code, at, &format!("birleşik görünümler okunamadı: {words}"))
                })?;
                if serde_json::to_value(&list).ok().as_ref() != Some(&value) {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "birleşik görünümler yazıcının biçiminde değil (varsayılan değerinde bir alan yazılmış)",
                    ));
                }
                if list.is_empty() {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "boş liste yazılmaz; görünüm yoksa alan hiç yazılmaz",
                    ));
                }
                // Schema 105 (docs/adr/0632 §4): a view's source table and summing columns.
                if !has.join_sources
                    && list
                        .iter()
                        .any(|v| v.source.is_some() || !v.aggregates.is_empty())
                {
                    return Err(r.fail_at(
                        Code::UnknownField,
                        at,
                        "birleşik görünümün source ve aggregates alanları şema 105'indir",
                    ));
                }
                if let Some(problem) = kentos_contracts::join_views_problem(&list) {
                    return Err(r.fail_at(Code::BadValue, at, &problem));
                }
                join_views = list;
            }
            // Schema 102 (docs/adr/0622 §5): Otomatik seçim.
            "autoSelect" if has.layer_geometry => auto_select = Some(r.bool()?),
            // Schema 98 (docs/adr/0583 §3): the map's system.
            "mapSrid" if has.source_crs => {
                let at = r.position();
                let v = r.uint(u64::from(u32::MAX))?;
                // `srid` comes first in the encoded order: the project's own system is known.
                let own = srid;
                let map = u32::try_from(v)
                    .ok()
                    .filter(|&m| m > 0 && Some(m) != own && own != Some(0))
                    .ok_or_else(|| {
                        r.fail_at(
                            Code::BadValue,
                            at,
                            "harita sistemi projeninkinden başka bir EPSG sistemi olmalı; yerel projenin harita sistemi olmaz",
                        )
                    })?;
                map_srid = Some(map);
            }
            // Schema 97 (docs/adr/0569): the project's links of event layers to their routes, read whole and checked.
            "linearEvents" if has.linear_events => {
                let at = r.position();
                let value = r.opaque()?;
                let list: Vec<kentos_contracts::EventLink> = serde_json::from_value(value.clone())
                    .map_err(|e| {
                        let words = e.to_string();
                        let code = if words.contains("unknown field") {
                            Code::UnknownField
                        } else {
                            Code::BadValue
                        };
                        r.fail_at(code, at, &format!("olay bağları okunamadı: {words}"))
                    })?;
                if serde_json::to_value(&list).ok().as_ref() != Some(&value) {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "olay bağları yazıcının biçiminde değil (varsayılan değerinde bir alan yazılmış)",
                    ));
                }
                if list.is_empty() {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "boş liste yazılmaz; bağ yoksa alan hiç yazılmaz",
                    ));
                }
                if let Some(problem) = kentos_contracts::event_links_problem(&list) {
                    return Err(r.fail_at(Code::BadValue, at, &problem));
                }
                linear_events = list;
            }
            // Schema 132 (docs/adr/0735): the data profiles and mappings the project carries, each its document as text;
            // the text is the profile commands' to judge.
            "profileFiles" if has.profile_files => {
                let at = r.position();
                let value = r.opaque()?;
                let list: Vec<kentos_contracts::ProjectProfileFile> =
                    serde_json::from_value(value.clone()).map_err(|e| {
                        let words = e.to_string();
                        let code = if words.contains("unknown field") {
                            Code::UnknownField
                        } else {
                            Code::BadValue
                        };
                        r.fail_at(code, at, &format!("profil dosyaları okunamadı: {words}"))
                    })?;
                if list.is_empty() {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "boş liste yazılmaz; profil yoksa alan hiç yazılmaz",
                    ));
                }
                if let Some(problem) = kentos_contracts::profile_files_problem(&list) {
                    return Err(r.fail_at(Code::BadValue, at, &problem));
                }
                profile_files = list;
            }
            // Schema 101 (docs/adr/0598 §1): what the split and merged objects came from, read whole and checked.
            "lineage" if has.lineage => {
                let at = r.position();
                let value = r.opaque()?;
                let list: Vec<kentos_contracts::LineageRecord> =
                    serde_json::from_value(value.clone()).map_err(|e| {
                        let words = e.to_string();
                        let code = if words.contains("unknown field") {
                            Code::UnknownField
                        } else {
                            Code::BadValue
                        };
                        r.fail_at(code, at, &format!("soy kayıtları okunamadı: {words}"))
                    })?;
                if serde_json::to_value(&list).ok().as_ref() != Some(&value) {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "soy kayıtları yazıcının biçiminde değil",
                    ));
                }
                if list.is_empty() {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "boş liste yazılmaz; kayıt yoksa alan hiç yazılmaz",
                    ));
                }
                if let Some(problem) = kentos_contracts::lineage_problem(&list) {
                    return Err(r.fail_at(Code::BadValue, at, &problem));
                }
                lineage = list;
            }
            // Schema 56 (docs/adr/0313 §1): the project's line types and line type scale.
            "lineTypes" if has.patterns => {
                let at = r.position();
                let value = r.opaque()?;
                let types: Vec<kentos_contracts::LineTypeDef> = serde_json::from_value(value)
                    .map_err(|e| {
                        r.fail_at(Code::BadValue, at, &format!("çizgi tipleri okunamadı: {e}"))
                    })?;
                if types.is_empty() {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "boş liste yazılmaz; çizgi tipi yoksa alan hiç yazılmaz",
                    ));
                }
                if let Some(problem) = kentos_contracts::line_types_problem(&types) {
                    return Err(r.fail_at(Code::BadValue, at, &problem));
                }
                line_types = types;
            }
            // Schema 71 (docs/adr/0331 §1): the project's own hatch patterns, as their JSON.
            "hatchPatterns" if has.hatch_extras => {
                let at = r.position();
                let value = r.opaque()?;
                let defs: Vec<kentos_contracts::HatchPatternDef> = serde_json::from_value(value)
                    .map_err(|e| {
                        r.fail_at(
                            Code::BadValue,
                            at,
                            &format!("tarama desenleri okunamadı: {e}"),
                        )
                    })?;
                if defs.is_empty() {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "boş liste yazılmaz; tarama deseni yoksa alan hiç yazılmaz",
                    ));
                }
                if let Some(problem) = kentos_contracts::hatch_patterns_problem(&defs) {
                    return Err(r.fail_at(Code::BadValue, at, &problem));
                }
                hatch_patterns = defs;
            }
            "lineTypeScale" if has.patterns => {
                let at = r.position();
                let s = r.float()?;
                if !(s.is_finite() && s > 0.0 && s <= kentos_contracts::MAX_LINE_TYPE_SCALE)
                    || s == 1.0
                {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "çizgi tipi ölçeği 0'dan büyük ve en çok 1000 olmalı; 1 yazılmaz",
                    ));
                }
                line_type_scale = Some(s);
            }
            // Schema 135 (docs/adr/0801): what a CAD project is drawn for.
            "cadWork" if has.cad_work => {
                cad_work = Some(named(
                    r,
                    &[
                        ("architecture", CadWork::Architecture),
                        ("map", CadWork::Map),
                    ],
                )?)
            }
            "workspace" => {
                workspace = Some(named(
                    r,
                    &[
                        ("cad", Workspace::Cad),
                        ("gis", Workspace::Gis),
                        ("plan3d", Workspace::Plan3d),
                        ("disaster", Workspace::Disaster),
                        // The former Hibrit mode: a type not asked yet (docs/adr/0165 §1).
                        ("hybrid", Workspace::LegacyHybrid),
                    ],
                )?)
            }
            "drawingFont" => {
                drawing_font = Some(named(
                    r,
                    &[
                        ("barlow", DrawingFont::Barlow),
                        ("arimo", DrawingFont::Arimo),
                        ("overpass", DrawingFont::Overpass),
                        ("quicksand", DrawingFont::Quicksand),
                        ("architects-daughter", DrawingFont::ArchitectsDaughter),
                        ("courier-prime", DrawingFont::CourierPrime),
                        ("plex-mono", DrawingFont::PlexMono),
                    ],
                )?)
            }
            "drawingUnit" if has.drawing_unit => {
                // Inches and feet are schema 50's (docs/adr/0262 §1).
                let units = [
                    ("mm", DrawingUnit::Mm),
                    ("cm", DrawingUnit::Cm),
                    ("m", DrawingUnit::M),
                    ("in", DrawingUnit::In),
                    ("ft", DrawingUnit::Ft),
                ];
                drawing_unit = Some(named(
                    r,
                    if has.imperial {
                        &units[..]
                    } else {
                        &units[..3]
                    },
                )?)
            }
            "lengthStyle" if has.imperial => {
                length_style = Some(named(
                    r,
                    &[
                        ("architectural", LengthStyle::Architectural),
                        ("engineering", LengthStyle::Engineering),
                        ("fractional", LengthStyle::Fractional),
                    ],
                )?)
            }
            "customCrs" if has.custom_crs => custom_crs = Some(crs::crs_definition(r)?),
            "secondCustomCrs" if has.custom_crs => {
                second_custom_crs = Some(crs::crs_definition(r)?)
            }
            "datumTransforms" if has.custom_crs => datum_transforms = crs::datum_transforms(r)?,
            "survey" if has.survey => survey = Some(survey_settings(r, has)?),
            "layerStates" if has.layer_states => {
                layer_states = layer_states_list(r, has.labels, has.frozen, has.patterns)?
            }
            "topology" if has.topology => {
                topology = Some(topology_settings(r, has.exception_digests)?)
            }
            "annotation" if has.annotation => annotation = Some(annotation_heights(r)?),
            "connections" if has.services => connections = services::connections(r)?,
            "networks" if has.networks => {
                networks = networks::networks(
                    r,
                    has.network_turns,
                    has.network_terminals,
                    has.network_configs,
                    has.network_profiles,
                    has.timed_closures,
                )?
            }
            "variables" if has.variables => variables = variables::variables(r)?,
            "addressLocators" if has.addresses => {
                address_locators = addresses::address_locators(r)?
            }
            "ucs" if has.ucs => ucs = Some(ucs::ucs(r)?),
            "namedUcs" if has.ucs => named_ucs = ucs::named_ucs(r)?,
            "heights" if has.heights => heights = Some(height_reference(r)?),
            "coordinateEpoch" if has.heights => {
                let at = r.position();
                let e = r.float()?;
                if let Some(why) = kentos_contracts::epoch_problem(e) {
                    return Err(r.fail_at(Code::BadValue, at, &why));
                }
                coordinate_epoch = Some(e);
            }
            "xrefPaths" if has.xref_paths => {
                let at = r.position();
                let paths = list(r, |r, _| text(r))?;
                if paths.is_empty()
                    || kentos_contracts::sanitized_xref_paths(paths.clone()) != paths
                {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "arama yolları kırpılmış, boş olmayan ve yinelenmeyen en çok 64 yol olmalı",
                    ));
                }
                xref_paths = paths;
            }
            "namedViews" if has.views => named_views = views::named_views(r, has.viewmarks)?,
            "textStyles" if has.styles => text_styles = styles::text_styles(r)?,
            // Schema 67 (docs/adr/0326 §4): the leader styles, as their JSON.
            "wallStyles" if has.walls => {
                let (l, at): (Vec<kentos_contracts::WallStyleDef>, usize) =
                    objects::json_part(r, "duvar stilleri")?;
                if l.is_empty() {
                    return Err(r.fail_at(Code::BadValue, at, "boş duvar stili listesi yazılmaz"));
                }
                if let Some(problem) = kentos_contracts::wall_styles_problem(&l) {
                    return Err(r.fail_at(Code::BadValue, at, &problem));
                }
                if !has.wall_joins
                    && l.iter()
                        .any(|s| s.components.iter().any(|c| c.priority.is_some()))
                {
                    return Err(r.fail_at(
                        Code::UnknownField,
                        at,
                        "bileşenin önceliği şema 76'dan beri yazılır",
                    ));
                }
                wall_styles = l;
            }
            "arch" if has.walls => {
                let (a, at): (kentos_contracts::ArchSettings, usize) =
                    objects::json_part(r, "mimari ayarlar")?;
                if a.is_empty() {
                    return Err(r.fail_at(Code::BadValue, at, "boş mimari ayar yazılmaz"));
                }
                if !has.space_choices && a.has_space_choices() {
                    return Err(r.fail_at(
                        Code::UnknownField,
                        at,
                        "projenin mahal listeleri ve kat önekleri şema 112'den beri yazılır",
                    ));
                }
                if let Some((field, words)) = a.problem() {
                    return Err(r.fail_at(Code::BadValue, at, &format!("{field}: {words}")));
                }
                if !has.level_state && a.has_level_state() {
                    return Err(r.fail_at(
                        Code::UnknownField,
                        at,
                        "projenin etkin katı ve kat görünümü şema 120'den beri yazılır",
                    ));
                }
                if !has.audit_exceptions && a.has_audit_exceptions() {
                    return Err(r.fail_at(
                        Code::UnknownField,
                        at,
                        "mimari denetimin istisnaları şema 130'dan beri yazılır",
                    ));
                }
                if !has.phases && a.phase_view.is_some() {
                    return Err(r.fail_at(
                        Code::UnknownField,
                        at,
                        "projenin evre görünümü şema 82'den beri yazılır",
                    ));
                }
                if !has.spaces && a.space_method.is_some() {
                    return Err(r.fail_at(
                        Code::UnknownField,
                        at,
                        "projenin ölçme yöntemi şema 78'den beri yazılır",
                    ));
                }
                arch = Some(a);
            }
            "leaderStyles" if has.multileader => {
                let (l, at): (Vec<kentos_contracts::LeaderStyleDef>, usize) =
                    objects::json_part(r, "kılavuz stilleri")?;
                if let Some((field, words)) =
                    l.iter().find_map(kentos_contracts::LeaderStyleDef::problem)
                {
                    return Err(r.fail_at(Code::BadValue, at, &format!("{field}: {words}")));
                }
                leader_styles = l;
            }
            "dimensionStyles" if has.styles => {
                dimension_styles =
                    styles::dimension_styles(r, has.annotation, has.dimension_options)?
            }
            // `srid` and `customCrs` come first in the encoded order: the project's own system is known.
            "secondSrid" if has.second_srid => {
                let at = r.position();
                let second = r.uint(u64::from(u32::MAX))? as u32;
                if second == 0 || (srid == Some(0) && custom_crs.is_none()) || srid == Some(second)
                {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "ikinci koordinat sistemi projeninkinden başka bir sistem olmalı; yerel projenin ikinci sistemi olmaz",
                    ));
                }
                second_srid = Some(second);
            }
            "areaDecimals" => area_decimals = Some(r.uint(u64::from(u32::MAX))? as u32),
            "lengthDecimals" => length_decimals = Some(r.uint(u64::from(u32::MAX))? as u32),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    let srid = required(r, srid, "srid")?;
    // A definition of its own is the system of a project without an EPSG
    // code; a second definition is no second EPSG code, and needs a system to
    // be the second of (docs/adr/0168 §1).
    let relation = |key: &'static str, what: &str, r: &mut Reader<'_>| {
        r.push(Seg::Name(key));
        let e = r.fail(Code::BadValue, what);
        r.pop();
        e
    };
    // A feet-and-inches style is an inch or foot drawing's (docs/adr/0262 §2).
    if length_style.is_some() && !drawing_unit.is_some_and(|u: DrawingUnit| u.imperial()) {
        return Err(relation(
            "lengthStyle",
            "ayak ve inç biçimi yalnız inç ya da ayak birimli projede olur",
            r,
        ));
    }
    if custom_crs.is_some() && srid != 0 {
        return Err(relation(
            "customCrs",
            "projenin kendi tanımı yalnız EPSG kodu olmayan (srid 0) projede olur",
            r,
        ));
    }
    if second_custom_crs.is_some() && (second_srid.is_some() || (srid == 0 && custom_crs.is_none()))
    {
        return Err(relation(
            "secondCustomCrs",
            "ikinci sistem ya EPSG kodu ya tanımdır; koordinat sistemi olmayan projenin ikinci sistemi olmaz",
            r,
        ));
    }
    // Ellipsoidal heights and an epoch rest on a datum: a project without a system has neither (docs/adr/0519 §1).
    let system = srid != 0 || custom_crs.is_some();
    if !system
        && heights
            .as_ref()
            .is_some_and(|h: &HeightReference| h.kind == HeightKind::Ellipsoidal)
    {
        return Err(relation(
            "heights",
            "elipsoit yüksekliği bir koordinat sistemi ister; yerel projede yalnız ortometrik yükseklik olur",
            r,
        ));
    }
    if !system && coordinate_epoch.is_some() {
        return Err(relation(
            "coordinateEpoch",
            "koordinat epoku bir koordinat sistemi ister",
            r,
        ));
    }
    if let Some(problem) = kentos_contracts::bases_problem(&source_bases, &sources) {
        return Err(relation("sourceBases", &problem, r));
    }
    // A work is a CAD project's (docs/adr/0801).
    if cad_work.is_some() && workspace != Some(Workspace::Cad) {
        return Err(relation(
            "cadWork",
            "iş yalnız CAD projesinin olur (workspace cad); başka türde ya da türü seçilmemiş projede yazılmaz",
            r,
        ));
    }
    Ok(ProjectSettings {
        source_bases,
        heights,
        coordinate_epoch,
        line_type_scale,
        line_types,
        hatch_patterns,
        srid,
        length_decimals: required(r, length_decimals, "lengthDecimals")?,
        area_decimals: required(r, area_decimals, "areaDecimals")?,
        area_unit: required(r, area_unit, "areaUnit")?,
        angle_unit: required(r, angle_unit, "angleUnit")?,
        plot_scale: required(r, plot_scale, "plotScale")?,
        workspace,
        cad_work,
        drawing_font,
        drawing_unit,
        second_srid,
        custom_crs,
        second_custom_crs,
        datum_transforms,
        survey,
        layer_states,
        text_styles,
        dimension_styles,
        leader_styles,
        wall_styles,
        arch,
        topology,
        annotation,
        connections,
        networks,
        variables,
        address_locators,
        ucs,
        named_ucs,
        named_views,
        forms,
        relations,
        join_views,
        linear_events,
        profile_files,
        lineage,
        map_srid,
        auto_select,
        sources,
        length_style,
        domains,
        xref_paths,
    })
}

/// Schema 30's annotation heights (docs/adr/0205 §1): each kind's paper
/// height, mm; a key the contract does not have is `unknown_field`, a height
/// that does not hold and an empty map (a writer leaves none out but the
/// field) are `bad_value`.
fn annotation_heights(r: &mut Reader<'_>) -> Result<AnnotationHeights, KcadError> {
    let at = r.position();
    let mut heights = AnnotationHeights::default();
    let mut any = false;
    map(r, |r, key| {
        let Some(kind) = AnnotationKind::from_key(key) else {
            return Err(unknown(r));
        };
        let at = r.position();
        let mm = r.float()?;
        if !annotation_mm_holds(mm) {
            return Err(r.fail_at(
                Code::BadValue,
                at,
                &format!(
                    "{} yüksekliği {mm} mm; sıfırdan büyük, en çok {MAX_ANNOTATION_MM} olmalı",
                    kind.label()
                ),
            ));
        }
        heights = std::mem::take(&mut heights).with(kind, Some(mm));
        any = true;
        Ok(())
    })?;
    if !any {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            "yazı yükseklikleri boş; yüksekliği olmayan proje alanı yazmaz",
        ));
    }
    Ok(heights)
}

/// Schema 27's topology settings (docs/adr/0202 §7): the tolerance, the
/// rules (`id`, `kind`, `layer`, `other`, `value`) and the exceptions (`at`,
/// `rule`, `objects`; from schema 99 `digest`), checked whole as the contract checks them
/// (`TopologySettings::problem`).
fn topology_settings(r: &mut Reader<'_>, digests: bool) -> Result<TopologySettings, KcadError> {
    let at = r.position();
    let mut t = TopologySettings::default();
    map(r, |r, key| {
        match key {
            "rules" => {
                t.rules = list(r, |r, _| {
                    let (mut id, mut kind, mut layer, mut other, mut value) =
                        (None, None, None, None, None);
                    map(r, |r, key| {
                        match key {
                            "id" => id = Some(text(r)?),
                            "kind" => {
                                let at = r.position();
                                let k = r.text()?;
                                kind = Some(TopologyRuleKind::of_key(k).ok_or_else(|| {
                                    r.fail_at(
                                        Code::BadValue,
                                        at,
                                        &format!("“{k}” bilinen bir topoloji kuralı değil"),
                                    )
                                })?);
                            }
                            "layer" => layer = Some(text(r)?),
                            "other" => other = Some(text(r)?),
                            "value" => value = Some(r.float()?),
                            _ => return Err(unknown(r)),
                        }
                        Ok(())
                    })?;
                    Ok(TopologyRule {
                        id: required(r, id, "id")?,
                        kind: required(r, kind, "kind")?,
                        layer: required(r, layer, "layer")?,
                        other,
                        value,
                    })
                })?
            }
            "tolerance" => t.tolerance = Some(r.float()?),
            "exceptions" => {
                t.exceptions = list(r, |r, _| {
                    let (mut at, mut rule, mut objects, mut digest) = (None, None, None, None);
                    map(r, |r, key| {
                        match key {
                            "at" => at = Some(point(r)?),
                            "rule" => rule = Some(text(r)?),
                            "objects" => {
                                objects =
                                    Some(list(r, |r, _| id16(r).map(kentos_contracts::EntityId))?)
                            }
                            "digest" if digests => digest = Some(text(r)?),
                            _ => return Err(unknown(r)),
                        }
                        Ok(())
                    })?;
                    Ok(TopologyException {
                        rule: required(r, rule, "rule")?,
                        objects: required(r, objects, "objects")?,
                        at: required(r, at, "at")?,
                        digest,
                    })
                })?
            }
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    match t.problem() {
        Some(problem) => Err(r.fail_at(Code::BadValue, at, &problem)),
        None => Ok(t),
    }
}

/// Schema 19's layer states (docs/adr/0177 §4): each its id, name and nodes,
/// a node its id, visibility and, when kept, its lock and a layer's style;
/// checked whole (`layer_states_problem`): no empty or repeated id or name,
/// no empty or repeated node in a state.
fn layer_states_list(
    r: &mut Reader<'_>,
    engine: bool,
    frozen: bool,
    patterns: bool,
) -> Result<Vec<LayerState>, KcadError> {
    let at = r.position();
    let all = list(r, |r, _| {
        let (mut id, mut name, mut nodes) = (None, None, None);
        map(r, |r, key| {
            match key {
                "id" => id = Some(text(r)?),
                "name" => name = Some(text(r)?),
                "nodes" => {
                    nodes = Some(list(r, |r, _| {
                        layer_state_node(r, engine, frozen, patterns)
                    })?)
                }
                _ => return Err(unknown(r)),
            }
            Ok(())
        })?;
        Ok(LayerState {
            id: required(r, id, "id")?,
            name: required(r, name, "name")?,
            nodes: required(r, nodes, "nodes")?,
        })
    })?;
    match layer_states_problem(&all) {
        Some(problem) => Err(r.fail_at(Code::BadValue, at, &problem)),
        None => Ok(all),
    }
}

/// A state's node; schema 55's `frozen` and `plot` (docs/adr/0311 §3).
fn layer_state_node(
    r: &mut Reader<'_>,
    engine: bool,
    frozen_too: bool,
    patterns: bool,
) -> Result<LayerStateNode, KcadError> {
    let (mut node, mut visible, mut locked, mut style) = (None, None, None, None);
    let (mut frozen, mut plot) = (None, None);
    map(r, |r, key| {
        match key {
            "node" => node = Some(text(r)?),
            "plot" if frozen_too => plot = Some(r.bool()?),
            "style" => style = Some(layer_style(r, engine, patterns)?),
            "frozen" if frozen_too => frozen = Some(r.bool()?),
            "locked" => locked = Some(r.bool()?),
            "visible" => visible = Some(r.bool()?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    Ok(LayerStateNode {
        node: required(r, node, "node")?,
        visible: required(r, visible, "visible")?,
        locked,
        style,
        frozen,
        plot,
        path: None,
    })
}

/// Schema 85's heights (docs/adr/0519 §1): the kind (`ellipsoidal`, `orthometric`; not `unknown`, which a project
/// does not keep), the vertical system and the geoid model, checked by their rules.
fn height_reference(r: &mut Reader<'_>) -> Result<HeightReference, KcadError> {
    let at = r.position();
    let (mut kind, mut vertical_srid, mut geoid) = (None, None, None);
    map(r, |r, key| {
        match key {
            "kind" => {
                let k_at = r.position();
                kind = Some(match text(r)?.as_str() {
                    "ellipsoidal" => HeightKind::Ellipsoidal,
                    "orthometric" => HeightKind::Orthometric,
                    other => {
                        return Err(r.fail_at(
                            Code::BadValue,
                            k_at,
                            &format!(
                                "“{other}” bir yükseklik türü değil (ellipsoidal, orthometric)"
                            ),
                        ));
                    }
                });
            }
            "verticalSrid" => vertical_srid = Some(r.uint(u64::from(u32::MAX))? as u32),
            "geoid" => {
                let (mut name, mut sha256) = (None, None);
                map(r, |r, key| {
                    match key {
                        "name" => name = Some(text(r)?),
                        "sha256" => sha256 = Some(text(r)?),
                        _ => return Err(unknown(r)),
                    }
                    Ok(())
                })?;
                geoid = Some(GeoidModel {
                    name: required(r, name, "name")?,
                    sha256,
                });
            }
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    let h = HeightReference {
        kind: required(r, kind, "kind")?,
        vertical_srid,
        geoid,
    };
    match h.problem() {
        Some((_, why)) => Err(r.fail_at(Code::BadValue, at, &why)),
        None => Ok(h),
    }
}

/// Schema 14's survey settings (docs/adr/0169 §3), checked whole: at least
/// one, k within [−1, 1], tolerances above zero; schema 15's traverse
/// tolerances too; schema 16's ground height within [−500, 9000] m and its
/// reduction to the grid only with one (docs/adr/0171); schema 28's a priori
/// standard deviations above zero, the parts per million and the centering
/// not below (docs/adr/0203 §1).
fn survey_settings(r: &mut Reader<'_>, has: Features) -> Result<SurveySettings, KcadError> {
    let at = r.position();
    let mut s = SurveySettings::default();
    map(r, |r, key| {
        match key {
            "index" => s.index = Some(r.float()?),
            "faceHz" => s.face_hz = Some(r.float()?),
            "faceSlope" => s.face_slope = Some(r.float()?),
            "refraction" => s.refraction = Some(r.float()?),
            "twoWay" if has.traverse_tolerances => s.two_way = Some(r.float()?),
            "traverseAngle" if has.traverse_tolerances => s.traverse_angle = Some(r.float()?),
            "traverseCoord" if has.traverse_tolerances => s.traverse_coord = Some(r.float()?),
            "groundHeight" if has.ground => s.ground_height = Some(r.float()?),
            "reduceToGrid" if has.ground => s.reduce_to_grid = Some(r.bool()?),
            "sigmaDirection" if has.survey_sigmas => s.sigma_direction = Some(r.float()?),
            "sigmaDistance" if has.survey_sigmas => s.sigma_distance = Some(r.float()?),
            "sigmaPpm" if has.survey_sigmas => s.sigma_ppm = Some(r.float()?),
            "sigmaCentering" if has.survey_sigmas => s.sigma_centering = Some(r.float()?),
            "sigmaZenith" if has.survey_sigmas => s.sigma_zenith = Some(r.float()?),
            "sigmaLevelling" if has.survey_sigmas => s.sigma_levelling = Some(r.float()?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    match s.problem() {
        Some(problem) => Err(r.fail_at(Code::BadValue, at, &problem)),
        None => Ok(s),
    }
}

fn source(r: &mut Reader<'_>) -> Result<MigrationSource, KcadError> {
    let at = r.position();
    let (mut format, mut version, mut sha) = (None, None, None);
    map(r, |r, key| {
        match key {
            "format" => format = Some(text(r)?),
            "version" => version = Some(r.uint(u64::from(u32::MAX))?),
            "sourceSha256" => {
                let (raw, at) = r.bytes()?;
                if raw.len() != 32 {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        &format!("{} bayt; SHA-256 32 bayt olmalı", raw.len()),
                    ));
                }
                sha = Some(hex(raw));
            }
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    let format = required(r, format, "format")?;
    let version = required(r, version, "version")?;
    let sha = required(r, sha, "sourceSha256")?;
    if format != DOCUMENT_FORMAT || version != u64::from(DOCUMENT_VERSION) {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            "göç kaynağı yalnız kentos.document sürüm 1 olabilir",
        ));
    }
    Ok(MigrationSource::v1(sha))
}

pub(crate) fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(char::from(DIGITS[usize::from(b >> 4)]));
        out.push(char::from(DIGITS[usize::from(b & 0x0f)]));
    }
    out
}

fn project_styles(r: &mut Reader<'_>) -> Result<ProjectStyles, KcadError> {
    let (mut items, mut categories) = (None, None);
    map(r, |r, key| {
        match key {
            "items" => items = Some(list(r, |r, _| r.opaque())?),
            "categories" => categories = Some(list(r, |r, _| r.opaque())?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    Ok(ProjectStyles {
        items: required(r, items, "items")?,
        categories: required(r, categories, "categories")?,
    })
}

// ── The layer tree (§6.5) ───────────────────────────────────────────────

fn layer(r: &mut Reader<'_>, has: Features) -> Result<LayerNode, KcadError> {
    let (mut id, mut name, mut kind, mut style) = (None, None, None, None);
    let (mut locked, mut visible, mut children, mut expanded) = (None, None, None, None);
    let (mut snap, mut fields, mut service, mut feed) = (None, None, None, None);
    let (mut time, mut scenario, mut replaces) = (None, None, None);
    let mut filter = None;
    let (mut actions, mut card, mut compose) = (None, None, None);
    let (mut plot, mut frozen) = (None, None);
    let mut table_styles = Vec::new();
    let mut subtypes: Option<(usize, kentos_contracts::LayerSubtypes)> = None;
    let mut source_crs: Option<kentos_contracts::LayerSourceCrs> = None;
    let mut dataset: Option<kentos_contracts::DatasetInfo> = None;
    let mut derivation: Option<kentos_contracts::LayerDerivation> = None;
    let mut geometry: Option<kentos_contracts::LayerGeometry> = None;
    let mut contingents: Option<(usize, Vec<kentos_contracts::FieldGroup>)> = None;
    let mut rules: Option<(usize, Vec<kentos_contracts::LayerRule>)> = None;
    map(r, |r, key| {
        match key {
            "id" => id = Some(text(r)?),
            "name" => name = Some(text(r)?),
            "frozen" if has.frozen => {
                if !r.bool()? {
                    return Err(r.fail(
                        Code::BadValue,
                        "frozen yalnız true yazılır; çözülmüş katmanda alan yoktur",
                    ));
                }
                frozen = Some(true);
            }
            "card" if has.attachments => card = Some(info::info_card(r)?),
            "actions" if has.attachments => actions = Some(info::layer_actions(r)?),
            "compose" if has.compose => compose = Some(compose::layer_compose(r)?),
            // Schema 72 (docs/adr/0413 §3): the table's rules, read whole and checked.
            "tableStyles" if has.forms => {
                let at = r.position();
                let value = r.opaque()?;
                let list: Vec<kentos_contracts::TableStyle> = serde_json::from_value(value.clone())
                    .map_err(|e| {
                        let words = e.to_string();
                        let code = if words.contains("unknown field") {
                            Code::UnknownField
                        } else {
                            Code::BadValue
                        };
                        r.fail_at(code, at, &format!("koşullu biçim okunamadı: {words}"))
                    })?;
                if serde_json::to_value(&list).ok().as_ref() != Some(&value) {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "koşullu biçim yazıcının biçiminde değil (varsayılan değerinde bir alan yazılmış)",
                    ));
                }
                if list.is_empty() {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "boş liste yazılmaz; kural yoksa alan hiç yazılmaz",
                    ));
                }
                if let Some((_, problem)) = kentos_contracts::table_styles_problem(&list) {
                    return Err(r.fail_at(Code::BadValue, at, &problem));
                }
                table_styles = list;
            }
            // Schema 72 (docs/adr/0422 §4): the subtypes, read whole; checked with the fields below.
            "subtypes" if has.forms => {
                let at = r.position();
                let value = r.opaque()?;
                let read: kentos_contracts::LayerSubtypes = serde_json::from_value(value.clone())
                    .map_err(|e| {
                    let words = e.to_string();
                    let code = if words.contains("unknown field") {
                        Code::UnknownField
                    } else {
                        Code::BadValue
                    };
                    r.fail_at(code, at, &format!("alt türler okunamadı: {words}"))
                })?;
                if serde_json::to_value(&read).ok().as_ref() != Some(&value) {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "alt türler yazıcının biçiminde değil (varsayılan değerinde bir alan yazılmış)",
                    ));
                }
                subtypes = Some((at, read));
            }
            // Schema 72 (docs/adr/0423 §4): the value combinations, read whole; checked below.
            "contingents" if has.forms => {
                let at = r.position();
                let value = r.opaque()?;
                let read: Vec<kentos_contracts::FieldGroup> = serde_json::from_value(value.clone())
                    .map_err(|e| {
                        let words = e.to_string();
                        let code = if words.contains("unknown field") {
                            Code::UnknownField
                        } else {
                            Code::BadValue
                        };
                        r.fail_at(code, at, &format!("değer bileşimleri okunamadı: {words}"))
                    })?;
                if serde_json::to_value(&read).ok().as_ref() != Some(&value) {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "değer bileşimleri yazıcının biçiminde değil (varsayılan değerinde bir alan yazılmış)",
                    ));
                }
                if read.is_empty() {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "boş liste yazılmaz; grup yoksa alan hiç yazılmaz",
                    ));
                }
                contingents = Some((at, read));
            }
            // Schema 72 (docs/adr/0438 §1): the record rules, read whole; checked below.
            "rules" if has.forms => {
                let at = r.position();
                let value = r.opaque()?;
                let read: Vec<kentos_contracts::LayerRule> = serde_json::from_value(value.clone())
                    .map_err(|e| {
                        let words = e.to_string();
                        let code = if words.contains("unknown field") {
                            Code::UnknownField
                        } else {
                            Code::BadValue
                        };
                        r.fail_at(code, at, &format!("kayıt kuralları okunamadı: {words}"))
                    })?;
                if serde_json::to_value(&read).ok().as_ref() != Some(&value) {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "kayıt kuralları yazıcının biçiminde değil (varsayılan değerinde bir alan yazılmış)",
                    ));
                }
                if read.is_empty() {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "boş liste yazılmaz; kural yoksa alan hiç yazılmaz",
                    ));
                }
                rules = Some((at, read));
            }
            // Schema 102 (docs/adr/0622 §5): what the layer holds.
            "geometry" if has.layer_geometry => {
                let at = r.position();
                geometry = Some(match text(r)?.as_str() {
                    "point" => kentos_contracts::LayerGeometry::Point,
                    "line" => kentos_contracts::LayerGeometry::Line,
                    "area" => kentos_contracts::LayerGeometry::Area,
                    "annotation" => kentos_contracts::LayerGeometry::Annotation,
                    other => {
                        return Err(r.fail_at(
                            Code::BadValue,
                            at,
                            &format!(
                                "katmanın türü “{other}” değil: point, line, area ya da annotation"
                            ),
                        ));
                    }
                });
            }
            // Schema 106 (docs/adr/0633 §3): the layer as a dataset and what it was derived from, read whole.
            "dataset" if has.datasets => {
                let at = r.position();
                let value = r.opaque()?;
                let read: kentos_contracts::DatasetInfo = serde_json::from_value(value.clone())
                    .map_err(|e| {
                        let words = e.to_string();
                        let code = if words.contains("unknown field") {
                            Code::UnknownField
                        } else {
                            Code::BadValue
                        };
                        r.fail_at(code, at, &format!("veri kümesi bilgisi okunamadı: {words}"))
                    })?;
                if serde_json::to_value(&read).ok().as_ref() != Some(&value) {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "veri kümesi bilgisi yazıcının biçiminde değil (boş bir alan yazılmış)",
                    ));
                }
                if let Some(problem) = kentos_contracts::dataset_problem(&read) {
                    return Err(r.fail_at(Code::BadValue, at, &problem));
                }
                dataset = Some(read);
            }
            "derivation" if has.datasets => {
                let at = r.position();
                let value = r.opaque()?;
                let read: kentos_contracts::LayerDerivation = serde_json::from_value(value.clone())
                    .map_err(|e| {
                        let words = e.to_string();
                        let code = if words.contains("unknown field") {
                            Code::UnknownField
                        } else {
                            Code::BadValue
                        };
                        r.fail_at(code, at, &format!("türetim kaydı okunamadı: {words}"))
                    })?;
                if serde_json::to_value(&read).ok().as_ref() != Some(&value) {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "türetim kaydı yazıcının biçiminde değil (boş bir alan yazılmış)",
                    ));
                }
                if let Some(problem) = kentos_contracts::derivation_problem(&read) {
                    return Err(r.fail_at(Code::BadValue, at, &problem));
                }
                derivation = Some(read);
            }
            // Schema 98 (docs/adr/0583 §1): the system its objects came from, read whole and checked.
            "sourceCrs" if has.source_crs => {
                let at = r.position();
                let value = r.opaque()?;
                let read: kentos_contracts::LayerSourceCrs = serde_json::from_value(value.clone())
                    .map_err(|e| {
                        let words = e.to_string();
                        let code = if words.contains("unknown field") {
                            Code::UnknownField
                        } else {
                            Code::BadValue
                        };
                        r.fail_at(code, at, &format!("kaynak sistemi okunamadı: {words}"))
                    })?;
                if serde_json::to_value(&read).ok().as_ref() != Some(&value) {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "kaynak sistemi yazıcının biçiminde değil (boş bir alan yazılmış)",
                    ));
                }
                if let Some(problem) = kentos_contracts::layer_crs_problem(&read) {
                    return Err(r.fail_at(Code::BadValue, at, &problem));
                }
                source_crs = Some(read);
            }
            "plot" if has.look => {
                if r.bool()? {
                    return Err(r.fail(
                        Code::BadValue,
                        "plot yalnız false yazılır; yazdırılan katmanda alan yoktur",
                    ));
                }
                plot = Some(false);
            }
            "filter" if has.filters => filter = Some(filter::layer_filter(r)?),
            "time" if has.temporal => time = Some(temporal::layer_time(r, has.recorded_time)?),
            "scenario" if has.temporal => scenario = Some(temporal::scenario(r)?),
            "replaces" if has.temporal => replaces = Some(text(r)?),
            "snap" if has.layer_snap => snap = Some(layer_snap(r)?),
            "fields" if has.layer_fields => {
                fields = Some(layer_fields(
                    r,
                    has.forms,
                    has.field_kinds,
                    has.withheld_fields,
                    has.field_columns,
                    has.table_constraints,
                )?)
            }
            "service" if has.services => service = Some(services::service_layer(r)?),
            "feed" if has.services => feed = Some(services::feature_feed(r)?),
            "type" => {
                kind = Some(named(
                    r,
                    &[
                        ("group", LayerNodeType::Group),
                        ("layer", LayerNodeType::Layer),
                    ],
                )?)
            }
            "style" => style = Some(layer_style(r, has.labels, has.patterns)?),
            "locked" => locked = Some(r.bool()?),
            "visible" => visible = Some(r.bool()?),
            "children" => children = Some(list(r, |r, _| layer(r, has))?),
            "expanded" => expanded = Some(r.bool()?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    if !table_styles.is_empty() && kind == Some(LayerNodeType::Group) {
        return Err(r.fail(
            Code::BadValue,
            "grubun koşullu biçimi olmaz; yalnız katmanın olur",
        ));
    }
    if let Some((at, read)) = &subtypes {
        if kind == Some(LayerNodeType::Group) {
            return Err(r.fail_at(
                Code::BadValue,
                *at,
                "grubun alt türleri olmaz; yalnız katmanın olur",
            ));
        }
        let none = Vec::new();
        let fields_now: &Vec<_> = fields.as_ref().unwrap_or(&none);
        if let Some((_, problem)) = kentos_contracts::subtypes_problem(fields_now, read) {
            return Err(r.fail_at(Code::BadValue, *at, &problem));
        }
    }
    if let Some((at, read)) = &contingents {
        if kind == Some(LayerNodeType::Group) {
            return Err(r.fail_at(
                Code::BadValue,
                *at,
                "grubun değer bileşimleri olmaz; yalnız katmanın olur",
            ));
        }
        let none = Vec::new();
        let fields_now: &Vec<_> = fields.as_ref().unwrap_or(&none);
        if let Some((_, problem)) = kentos_contracts::contingents_problem(
            fields_now,
            subtypes.as_ref().map(|(_, s)| s),
            read,
        ) {
            return Err(r.fail_at(Code::BadValue, *at, &problem));
        }
    }
    if let Some((at, read)) = &rules {
        if kind == Some(LayerNodeType::Group) {
            return Err(r.fail_at(
                Code::BadValue,
                *at,
                "grubun kayıt kuralları olmaz; yalnız katmanın olur",
            ));
        }
        let none = Vec::new();
        let fields_now: &Vec<_> = fields.as_ref().unwrap_or(&none);
        if let Some((_, problem)) = kentos_contracts::layer_rules_problem(fields_now, read) {
            return Err(r.fail_at(Code::BadValue, *at, &problem));
        }
    }
    if source_crs.is_some() && kind == Some(LayerNodeType::Group) {
        return Err(r.fail(
            Code::BadValue,
            "grubun kaynak sistemi olmaz; yalnız katmanın olur",
        ));
    }
    if geometry.is_some() && kind == Some(LayerNodeType::Group) {
        return Err(r.fail(
            Code::BadValue,
            "grubun türü olmaz; nokta, çizgi, alan ya da açıklama yalnız katmanın türüdür",
        ));
    }
    if (dataset.is_some() || derivation.is_some()) && kind == Some(LayerNodeType::Group) {
        return Err(r.fail(
            Code::BadValue,
            "grubun veri kümesi bilgisi ve türetim kaydı olmaz; yalnız katmanın olur",
        ));
    }
    Ok(LayerNode {
        dataset,
        derivation,
        source_crs,
        geometry,
        rules: rules.map(|(_, c)| c).unwrap_or_default(),
        contingents: contingents.map(|(_, c)| c).unwrap_or_default(),
        subtypes: subtypes.map(|(_, s)| s),
        table_styles,
        plot,
        frozen,
        id: required(r, id, "id")?,
        name: required(r, name, "name")?,
        kind: required(r, kind, "type")?,
        visible: required(r, visible, "visible")?,
        locked: required(r, locked, "locked")?,
        expanded: required(r, expanded, "expanded")?,
        style: required(r, style, "style")?,
        children: required(r, children, "children")?,
        snap: match (snap, kind) {
            (Some(_), Some(LayerNodeType::Group)) => {
                return Err(r.fail(
                    Code::BadValue,
                    "grubun keneti olmaz; kenet yalnız katmanındır",
                ));
            }
            (snap, _) => snap,
        },
        fields: match (fields, kind) {
            (Some(_), Some(LayerNodeType::Group)) => {
                return Err(r.fail(
                    Code::BadValue,
                    "grubun alanları olmaz; alanlar yalnız katmanındır",
                ));
            }
            (fields, _) => fields.unwrap_or_default(),
        },
        service: match (service, kind) {
            (Some(_), _) if feed.is_some() => {
                return Err(r.fail(
                    Code::BadValue,
                    "katman hem servisten çizilir hem nesnelerini bir kaynaktan alır; ikisi birden olmaz",
                ));
            }
            (Some(_), Some(LayerNodeType::Group)) => {
                return Err(r.fail(
                    Code::BadValue,
                    "grubun servisi olmaz; servis yalnız katmanındır",
                ));
            }
            (service, _) => service,
        },
        feed: match (feed, kind) {
            (Some(_), Some(LayerNodeType::Group)) => {
                return Err(r.fail(
                    Code::BadValue,
                    "grubun veri kaynağı olmaz; kaynak yalnız katmanındır",
                ));
            }
            (feed, _) => feed,
        },
        time: match (time, kind) {
            (Some(_), Some(LayerNodeType::Group)) => {
                return Err(r.fail(
                    Code::BadValue,
                    "grubun zamanı olmaz; zaman yalnız katmanındır",
                ));
            }
            (time, _) => time,
        },
        scenario: match (scenario, kind) {
            (Some(_), Some(LayerNodeType::Layer)) => {
                return Err(r.fail(
                    Code::BadValue,
                    "katman senaryo olmaz; yalnız grup senaryodur",
                ));
            }
            (scenario, _) => scenario,
        },
        replaces: match (replaces, kind) {
            (Some(_), Some(LayerNodeType::Group)) => {
                return Err(r.fail(
                    Code::BadValue,
                    "grup bir katmanın yerine geçmez; yalnız katman geçer",
                ));
            }
            (replaces, _) => replaces,
        },
        filter: match (filter, kind) {
            (Some(_), Some(LayerNodeType::Group)) => {
                return Err(r.fail(
                    Code::BadValue,
                    "grubun süzgeci olmaz; süzgeç yalnız katmanındır",
                ));
            }
            (filter, _) => filter,
        },
        actions: match (actions, kind) {
            (Some(_), Some(LayerNodeType::Group)) => {
                return Err(r.fail(
                    Code::BadValue,
                    "grubun eylemi olmaz; eylemler yalnız katmanındır",
                ));
            }
            (actions, _) => actions.unwrap_or_default(),
        },
        card: match (card, kind) {
            (Some(_), Some(LayerNodeType::Group)) => {
                return Err(r.fail(
                    Code::BadValue,
                    "grubun bilgi kartı olmaz; kart yalnız katmanındır",
                ));
            }
            (card, _) => card,
        },
        compose: match (compose, kind) {
            (Some(_), Some(LayerNodeType::Group)) => {
                return Err(r.fail(
                    Code::BadValue,
                    "grubun katman çizimi olmaz; katman çizimi yalnız katmanındır",
                ));
            }
            (compose, _) => compose,
        },
    })
}

/// Schema 26's fields of a layer (docs/adr/0199 §1), checked whole
/// (`layer_fields_problem`): a list that is not empty, each field by its
/// kind's rules, no name twice.
fn layer_fields(
    r: &mut Reader<'_>,
    ids: bool,
    wide: bool,
    withheld: bool,
    columns: bool,
    constraints: bool,
) -> Result<Vec<LayerField>, KcadError> {
    let at = r.position();
    let all = list(r, |r, _| {
        layer_field(r, ids, wide, withheld, columns, constraints)
    })?;
    match layer_fields_problem(&all) {
        Some(problem) => Err(r.fail_at(Code::BadValue, at, &problem)),
        None => Ok(all),
    }
}

/// A layer's field; `ids`: schema 72's stable `id` is read (docs/adr/0401 §2).
fn layer_field(
    r: &mut Reader<'_>,
    ids: bool,
    wide: bool,
    withholds: bool,
    columns: bool,
    constraints: bool,
) -> Result<LayerField, KcadError> {
    let mut id = None;
    let mut withheld = None;
    let mut column = None;
    let (mut name, mut kind, mut alias, mut length, mut scale) = (None, None, None, None, None);
    let (mut min, mut max, mut values, mut default, mut must) = (None, None, None, None, None);
    let mut many = None;
    let (mut split, mut merge) = (None, None);
    let mut domain = None;
    let (mut unique, mut min_out, mut max_out, mut unit) = (None, None, None, None);
    let count = |r: &mut Reader<'_>| r.uint(u64::from(u32::MAX)).map(|n| n as u32);
    map(r, |r, key| {
        match key {
            "id" if ids => id = Some(text(r)?),
            "max" => max = Some(text(r)?),
            "min" => min = Some(text(r)?),
            // A date and time and a time of day are schema 72's (docs/adr/0412 §4), the kinds of docs/adr/0629 §2
            // schema 104's.
            "kind" => {
                let kinds: Vec<(&str, LayerFieldKind)> = LayerFieldKind::ALL
                    .into_iter()
                    .filter(|k| {
                        (ids || !matches!(k, LayerFieldKind::DateTime | LayerFieldKind::Time))
                            && (wide || !k.is_wide())
                    })
                    .map(|k| (k.name(), k))
                    .collect();
                kind = Some(named(r, &kinds)?)
            }
            "name" => name = Some(text(r)?),
            "alias" => alias = Some(text(r)?),
            "scale" => scale = Some(count(r)?),
            "length" => length = Some(count(r)?),
            "values" => values = Some(list(r, |r, _| field_choice(r, ids))?),
            "default" => default = Some(text(r)?),
            "required" => must = Some(r.bool()?),
            "multiple" if ids => many = Some(r.bool()?),
            // Schema 72 (docs/adr/0430): the value list it links.
            "domain" if ids => domain = Some(text(r)?),
            // Schema 72 (docs/adr/0438 §3, §4): unique, the range's left-out ends, the unit.
            "unique" if ids => unique = Some(r.bool()?),
            "minExclusive" if ids => min_out = Some(r.bool()?),
            "maxExclusive" if ids => max_out = Some(r.bool()?),
            "unit" if ids => unit = Some(text(r)?),
            // Schema 107 (docs/adr/0638 §5): its values do not leave the drawing.
            "withheld" if withholds => withheld = Some(r.bool()?),
            // Schema 133 (docs/adr/0753): the bound table's column, what the table makes and what it refers to.
            "column" if columns => column = Some(field_column(r, constraints)?),
            // Schema 72 (docs/adr/0419): the split and merge policies.
            "split" if ids => {
                let all: Vec<(&str, kentos_contracts::SplitPolicy)> =
                    kentos_contracts::SplitPolicy::ALL
                        .into_iter()
                        .map(|p| (crate::encode::split_name(p), p))
                        .collect();
                split = Some(named(r, &all)?)
            }
            "merge" if ids => {
                let all: Vec<(&str, kentos_contracts::MergePolicy)> =
                    kentos_contracts::MergePolicy::ALL
                        .into_iter()
                        .map(|p| (crate::encode::merge_name(p), p))
                        .collect();
                merge = Some(named(r, &all)?)
            }
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    if must == Some(false) || many == Some(false) {
        return Err(r.fail(
            Code::BadValue,
            "required ve multiple yalnız true yazılır; değilse yazılmaz",
        ));
    }
    if unique == Some(false) || min_out == Some(false) || max_out == Some(false) {
        return Err(r.fail(
            Code::BadValue,
            "unique, minExclusive ve maxExclusive yalnız true yazılır; değilse yazılmaz",
        ));
    }
    if withheld == Some(false) {
        return Err(r.fail(
            Code::BadValue,
            "withheld yalnız true yazılır; değilse yazılmaz",
        ));
    }
    Ok(LayerField {
        withheld: withheld == Some(true),
        id,
        name: required(r, name, "name")?,
        alias,
        kind: required(r, kind, "kind")?,
        length,
        scale,
        min,
        max,
        values,
        required: must == Some(true),
        default,
        multiple: many == Some(true),
        split,
        merge,
        domain,
        unique: unique == Some(true),
        min_exclusive: min_out == Some(true),
        max_exclusive: max_out == Some(true),
        unit,
        column,
    })
}

/// A field's bound column (schema 133, docs/adr/0753): `made` and `refers`, from schema 134 `check` (docs/adr/0755 §4),
/// at least one; their texts are checked with the field's (`field_problem`).
fn field_column(r: &mut Reader<'_>, constraints: bool) -> Result<FieldColumn, KcadError> {
    let (mut made, mut refers, mut check) = (None, None, None);
    map(r, |r, key| {
        match key {
            "made" => made = Some(text(r)?),
            "check" if constraints => check = Some(text(r)?),
            "refers" => {
                let (mut table, mut column, mut label) = (None, None, None);
                map(r, |r, key| {
                    match key {
                        "table" => table = Some(text(r)?),
                        "column" => column = Some(text(r)?),
                        "label" => label = Some(text(r)?),
                        _ => return Err(unknown(r)),
                    }
                    Ok(())
                })?;
                refers = Some(ColumnRefers {
                    table: required(r, table, "table")?,
                    column: required(r, column, "column")?,
                    label,
                });
            }
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    if made.is_none() && refers.is_none() && check.is_none() {
        return Err(r.fail(
            Code::BadValue,
            "column boş olamaz: made, refers ya da check yazılır; hiçbiri yoksa column yazılmaz",
        ));
    }
    Ok(FieldColumn {
        made,
        refers,
        check,
    })
}

/// A value of a field's list; `later`: schema 72's `retired` is read (docs/adr/0418).
fn field_choice(r: &mut Reader<'_>, later: bool) -> Result<FieldChoice, KcadError> {
    let (mut code, mut label, mut retired) = (None, None, None);
    let (mut description, mut from, mut until) = (None, None, None);
    map(r, |r, key| {
        match key {
            "code" => code = Some(text(r)?),
            "label" => label = Some(text(r)?),
            "retired" if later => retired = Some(r.bool()?),
            // Schema 72 (docs/adr/0438 §5): what it means and its days.
            "description" if later => description = Some(text(r)?),
            "from" if later => from = Some(text(r)?),
            "until" if later => until = Some(text(r)?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    if retired == Some(false) {
        return Err(r.fail(
            Code::BadValue,
            "retired yalnız true yazılır; değilse yazılmaz",
        ));
    }
    Ok(FieldChoice {
        code: required(r, code, "code")?,
        label: required(r, label, "label")?,
        retired: retired == Some(true),
        description,
        from,
        until,
    })
}

/// A layer's own snapping (docs/adr/0163 §4): `off` (true only) or `kinds`.
fn layer_snap(r: &mut Reader<'_>) -> Result<LayerSnap, KcadError> {
    let mut snap = LayerSnap::default();
    let mut off = None;
    map(r, |r, key| {
        match key {
            "off" => off = Some(r.bool()?),
            "kinds" => snap.kinds = Some(list(r, |r, _| text(r))?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    if off == Some(false) {
        return Err(r.fail(
            Code::BadValue,
            "off yalnız true yazılır; kenet kapalı değilse alan yazılmaz",
        ));
    }
    snap.off = off == Some(true);
    match snap.problem() {
        Some(problem) => Err(r.fail(Code::BadValue, &problem)),
        None => Ok(snap),
    }
}

/// A line type by its name (a layer style's, a dimension's lines'; §6.4).
pub(super) fn line_type_named(r: &mut Reader<'_>) -> Result<LineType, KcadError> {
    named(
        r,
        &[
            ("continuous", LineType::Continuous),
            ("dashed", LineType::Dashed),
            ("dashdot", LineType::Dashdot),
            ("dotted", LineType::Dotted),
        ],
    )
}

fn layer_style(r: &mut Reader<'_>, engine: bool, patterns: bool) -> Result<LayerStyle, KcadError> {
    let (mut fill, mut color, mut label, mut point_) = (None, None, None, None);
    let mut pattern = None;
    let mut labels = None;
    let (mut line_type, mut renderer, mut line_weight, mut pick_interior) =
        (None, None, None, None);
    map(r, |r, key| {
        match key {
            "fill" => fill = Some(text(r)?),
            "color" => color = Some(text(r)?),
            "label" => label = Some(labels::label_style(r, engine)?),
            "labels" if engine => labels = Some(labels::layer_labels(r)?),
            "point" => point_ = Some(point_style(r)?),
            "lineType" => line_type = Some(line_type_named(r)?),
            "linePattern" if patterns => pattern = Some(text(r)?),
            "renderer" => {
                if r.next_is_null() {
                    return Err(r.fail(
                        Code::BadValue,
                        "null yazılmaz; çizici yoksa alan hiç yazılmaz",
                    ));
                }
                renderer = Some(r.opaque()?);
            }
            "lineWeight" => line_weight = Some(r.float()?),
            "pickInterior" => pick_interior = Some(r.bool()?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    Ok(LayerStyle {
        line_pattern: pattern,
        color: required(r, color, "color")?,
        line_type: required(r, line_type, "lineType")?,
        line_weight: required(r, line_weight, "lineWeight")?,
        fill,
        point: point_,
        label,
        pick_interior,
        renderer,
        labels,
    })
}

fn point_style(r: &mut Reader<'_>) -> Result<PointStyle, KcadError> {
    let (mut size, mut symbol) = (None, None);
    map(r, |r, key| {
        match key {
            "size" => size = Some(r.float()?),
            "symbol" => {
                symbol = Some(named(
                    r,
                    &[
                        ("ring", PointSymbol::Ring),
                        ("cross", PointSymbol::Cross),
                        ("triangle", PointSymbol::Triangle),
                    ],
                )?)
            }
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    Ok(PointStyle {
        symbol: required(r, symbol, "symbol")?,
        size: required(r, size, "size")?,
    })
}
