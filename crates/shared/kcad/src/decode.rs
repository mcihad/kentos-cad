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

// Each discipline's settings in its own file (docs/adr/8054 §10).
mod arch;
mod gis;
mod map;
mod modules;
mod road;
use gis::topology_settings;
use map::{height_reference, survey_settings};
pub(crate) use road::{
    road_has_approval_sources, road_has_cloud_conversions, road_has_cloud_heights,
    road_has_corridor_sampling, road_has_cut_benches, road_has_haul_approval,
    road_has_haul_network, road_has_haul_sorting, road_has_ove, road_has_pavement, road_has_pivot,
    road_has_places, road_has_profile_crossings, road_has_section_breaks,
    road_has_sheet_contour_drawings, road_has_slope_transitions, road_has_speed,
    road_has_volume_curve, road_has_volume_grid, road_has_volume_history, road_has_volume_states,
};

use kentos_contracts::{
    AngleUnit, AnnotationHeights, AnnotationKind, AreaUnit, Bounds, CadWork, ColumnRefers,
    DOCUMENT_FORMAT, DOCUMENT_VERSION, DOCUMENT_VERSION_2, DocumentSnapshotV2, DrawingFont,
    DrawingUnit, FieldChoice, FieldColumn, GeoidModel, HeightKind, HeightReference, LayerField,
    LayerFieldKind, LayerNode, LayerNodeType, LayerSnap, LayerState, LayerStateNode, LayerStyle,
    LengthStyle, LineType, MAX_ANNOTATION_MM, MigrationSource, PointStyle, PointSymbol, ProjectId,
    ProjectSettings, ProjectStyles, SurveySettings, TopologyDecision, TopologyException, TopologyProfile,
    TopologyRule, TopologyScope,
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
    // A newer file is not a broken one: the words say which program wrote it, not "bozuk" (a reader that did not know
    // a road template's pavement called the sample road project broken, docs/adr/0975 §2).
    let version_error = |_: &Reader<'_>, v: &str| {
        let newest = SCHEMAS.last().copied().unwrap_or_default();
        let newer = v.parse::<u32>().is_ok_and(|n| n > newest);
        KcadError::new(
            Code::SchemaVersion,
            if newer {
                format!(
                    "Bu dosya KentOS'un daha yeni bir sürümüyle yazılmış (belge şeması {v}; bu sürüm en çok {newest}'ü okur). \
                     KentOS'u güncelleyip yeniden açın."
                )
            } else {
                format!("Belge şeması sürümü {v} bu uygulamada okunamıyor: bu bir KentOS sürümünün yazdığı şema değil.")
            },
        )
    };
    let mut format_ok = false;
    let mut schema: Option<u32> = None;
    let mut document = None;
    // `modules` comes before `version` (§6.1): checked once the schema is known, before the document.
    let mut modules: Option<modules::Read> = None;
    map(r, |r, key| {
        match key {
            "format" => {
                if !matches!(r.any()?, Any::Text(DOCUMENT_FORMAT)) {
                    return Err(not_ours(r));
                }
                format_ok = true;
            }
            "modules" => modules = Some(modules::read(r)?),
            "version" => match r.any()? {
                Any::Uint(v) => match SCHEMAS.iter().find(|&&s| u64::from(s) == v) {
                    Some(&known) => {
                        schema = Some(known);
                        if let Some(read) = &modules {
                            modules::check(r, known, read)?;
                        }
                    }
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
            "arrays" if has.arrays => arrays_ = Some(arrays::arrays(r,schema)?),
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
            "constraints" if has.constraints => {
                constraints_ = Some(constraints(
                    r,
                    has.arch_bonds,
                    has.arc_edge_bonds,
                    has.boundary_bonds,
                    has.grid_bonds,
                    has.column_side_bonds,
                )?)
            }
            "dimensionSets" if has.dimension_sets => {
                dimension_sets_ = Some(dimension_sets(r, has)?)
            }
            "migratedFrom" => migrated_from = Some(source(r)?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    let name = required(r, name, "name")?;
    let layers = required(r, layers, "layers")?;
    let origin = required(r, origin, "origin")?;
    let styles = required(r, styles, "styles")?;
    if schema < crate::SCHEMA_WITH_SHAPE_RESOURCES
        && styles
            .items
            .iter()
            .any(kentos_contracts::shape_resource::is_resource)
    {
        return Err(r.fail(
            Code::UnknownField,
            "SHX kaynakları belge şeması 164 gerektirir",
        ));
    }
    if let Err(error) = kentos_contracts::shape_resource::check_items(&styles.items) {
        return Err(r.fail(Code::BadValue, &error));
    }
    if schema < crate::SCHEMA_WITH_SHX_FONTS
        && styles.items.iter().any(kentos_contracts::shape_resource::is_font_form)
    {
        return Err(r.fail(
            Code::BadValue,
            "SHX unifont ve Big Font kaynakları belge şeması 184 gerektirir",
        ));
    }
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
        .map_err(|e| {
            r.fail_at(
                Code::BadValue,
                at,
                &format!("mutfak dizileri okunamadı: {e}"),
            )
        })?;
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
fn constraints(
    r: &mut Reader<'_>,
    arch: bool,
    arcs: bool,
    boundaries: bool,
    grids: bool,
    column_sides: bool,
) -> Result<(usize, kentos_contracts::ConstraintSet), KcadError> {
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
    if !arcs && set.constraints.iter().any(|c| {
        c.rule.elements().iter().any(|r| {
            matches!(r.element, kentos_contracts::CadElement::ArcEdge { .. })
        })
    }) {
        return Err(r.fail_at(Code::BadValue, at, "yaylı kenar kısıtları şema 157 ister"));
    }
    // Schema 244 (docs/adr/8049 §3): a column's side only from it on.
    if !column_sides
        && set
            .constraints
            .iter()
            .any(|c| kentos_contracts::rule_names_column_side(&c.rule))
    {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            "kolon yanı kısıtları şema 244'ten önce yazılmaz",
        ));
    }
    // Schema 243 (docs/adr/8049 §2): a grid's line only from it on.
    if !grids
        && set
            .constraints
            .iter()
            .any(|c| kentos_contracts::rule_names_grid(&c.rule))
    {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            "aks kısıtları şema 243'ten önce yazılmaz",
        ));
    }
    // Schema 208 (docs/adr/2580): a boundary edge, a map bond's, only from it on.
    if !boundaries
        && set
            .constraints
            .iter()
            .any(|c| kentos_contracts::rule_is_map_bond(&c.rule))
    {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            "sınır kenarı kısıtları (harita bağları) şema 208'den önce yazılmaz",
        ));
    }
    // Schema 137 (docs/adr/0770): a wall's line or an opening's jamb only from it on.
    if !arch
        && set
            .constraints
            .iter()
            .any(|c| kentos_contracts::rule_is_architectural(&c.rule))
    {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            "duvar çizgisi ve pervaz kısıtları şema 137'den önce yazılmaz",
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
    let mut view_templates = Vec::new();
    let mut forms = Vec::new();
    let mut domains = Vec::new();
    let mut relations = Vec::new();
    let mut join_views = Vec::new();
    let mut linear_events = Vec::new();
    let mut profile_files = Vec::new();
    let mut standard_docs = Vec::new();
    let mut review_notes = Vec::new();
    let mut approved_sheet_templates = Vec::new();
    let mut table_styles = Vec::new();
    let mut standard_exceptions = Vec::new();
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
    let mut road = None;
    let mut map_work = None;
    let mut map_survey = None;
    let mut parcel_register = None;
    let mut zoning_plan = None;
    let mut survey_adjustments = Vec::new();
    let mut monitoring_reviews = Vec::new();
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
            "forms" if has.forms => forms = gis::forms(r)?,
            // Schema 72 (docs/adr/0430): the project's value lists, read whole and checked.
            "domains" if has.forms => domains = gis::domains(r)?,
            // Schema 74 (docs/adr/0503): the layers bound to their sources, read whole and checked.
            "sources" if has.sources => sources = gis::sources(r, has)?,
            // Schema 91 (docs/adr/0538 §1): the bound layers' bases, read whole; checked against the links below.
            "sourceBases" if has.source_bases => source_bases = gis::source_bases(r, has)?,
            // Schema 72 (docs/adr/0405 §1): the project's relations, read whole and checked.
            "relations" if has.forms => relations = gis::relations(r)?,
            // Schema 89 (docs/adr/0527): the project's join views, read whole and checked.
            "joinViews" if has.join_views => join_views = gis::join_views(r, has)?,
            // Schema 102 (docs/adr/0622 §5): Otomatik seçim.
            "autoSelect" if has.layer_geometry => auto_select = Some(r.bool()?),
            // Schema 98 (docs/adr/0583 §3): the map's system.
            // Schema 188 (docs/adr/1200 §2): a map project's work structure, read whole and checked.
            // Schema 194 (docs/adr/1340 §5): a map project's GNSS archive, read whole and checked.
            // Schema 216 (docs/adr/4220): the parcel register, read whole and checked.
            "parcelRegister" if has.parcel_register => {
                parcel_register = map::parcel_register(r, has)?
            }
            // Schema 250 (docs/adr/5040): a zoning plan's kept symbol choices and plan change drafts, read whole and
            // checked.
            "zoningPlan" if has.zoning_drafts => zoning_plan = map::zoning_plan(r)?,
            // Schema 252 (docs/adr/5200): the adjustments the project keeps, read whole and checked.
            // Schema 253 (docs/adr/5220): the reviews the project keeps of its monitoring jobs, read whole and checked.
            "monitoringReviews" if has.monitoring_reviews => {
                monitoring_reviews = map::monitoring_reviews(r)?
            }
            "surveyAdjustments" if has.kept_adjustments => {
                survey_adjustments = map::survey_adjustments(r)?
            }
            "mapSurvey" if has.map_survey => map_survey = map::map_survey(r, has)?,
            "mapWork" if has.map_work => map_work = map::map_work(r, has)?,
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
            "linearEvents" if has.linear_events => linear_events = gis::linear_events(r)?,
            // Schema 132 (docs/adr/0735): the data profiles and mappings the project carries, each its document as text;
            // the text is the profile commands' to judge.
            "profileFiles" if has.profile_files => profile_files = gis::profile_files(r)?,
            "standardDocs" if has.standard_docs => {
                let at = r.position();
                let value = r.opaque()?;
                let list: Vec<kentos_contracts::ProjectStandardDocument> =
                    serde_json::from_value(value).map_err(|e| {
                        let words = e.to_string();
                        let code = if words.contains("unknown field") { Code::UnknownField } else { Code::BadValue };
                        r.fail_at(code, at, &format!("standart belgeleri okunamadı: {words}"))
                    })?;
                if list.is_empty() {
                    return Err(r.fail_at(Code::BadValue, at, "boş standart listesi yazılmaz"));
                }
                if let Some(problem) = kentos_contracts::project_standards_problem(&list) {
                    return Err(r.fail_at(Code::BadValue, at, &problem));
                }
                standard_docs = list;
            }
            // Schema 185 (docs/adr/1126 §2): the table styles, read whole and checked.
            "tableStyles" if has.table_styles => {
                let at = r.position();
                let value = r.opaque()?;
                let list: Vec<kentos_contracts::TableStyleDef> = serde_json::from_value(value).map_err(|e| {
                    let words = e.to_string();
                    let code = if words.contains("unknown field") { Code::UnknownField } else { Code::BadValue };
                    r.fail_at(code, at, &format!("tablo stilleri okunamadı: {words}"))
                })?;
                if list.is_empty() {
                    return Err(r.fail_at(Code::BadValue, at, "boş tablo stili listesi yazılmaz"));
                }
                if let Some(problem) = kentos_contracts::table_style_defs_problem(&list) {
                    return Err(r.fail_at(Code::BadValue, at, &problem));
                }
                table_styles = list;
            }
            // Schema 183 (docs/adr/1100 §3): the approved sheet template revisions, read whole and checked.
            // Schema 190 (docs/adr/1129): the accepted standards findings, read whole and checked.
            "standardExceptions" if has.standard_exceptions => {
                let at = r.position();
                let value = r.opaque()?;
                let list: Vec<kentos_contracts::StandardException> = serde_json::from_value(value).map_err(|e| {
                    let words = e.to_string();
                    let code = if words.contains("unknown field") { Code::UnknownField } else { Code::BadValue };
                    r.fail_at(code, at, &format!("kabul edilmiş standart bulguları okunamadı: {words}"))
                })?;
                if list.is_empty() {
                    return Err(r.fail_at(Code::BadValue, at, "boş kabul edilmiş bulgu listesi yazılmaz"));
                }
                if let Some(problem) = kentos_contracts::standard_exceptions_problem(&list) {
                    return Err(r.fail_at(Code::BadValue, at, &problem));
                }
                standard_exceptions = list;
            }
            "approvedSheetTemplates" if has.approved_sheet_templates => {
                let at = r.position();
                let value = r.opaque()?;
                let list: Vec<kentos_contracts::ApprovedSheetTemplate> =
                    serde_json::from_value(value).map_err(|e| {
                        let words = e.to_string();
                        let code = if words.contains("unknown field") { Code::UnknownField } else { Code::BadValue };
                        r.fail_at(code, at, &format!("onaylı pafta şablonları okunamadı: {words}"))
                    })?;
                if list.is_empty() {
                    return Err(r.fail_at(Code::BadValue, at, "boş onaylı pafta şablonu listesi yazılmaz"));
                }
                if let Some(problem) = kentos_contracts::approved_sheet_templates_problem(&list) {
                    return Err(r.fail_at(Code::BadValue, at, &problem));
                }
                approved_sheet_templates = list;
            }
            // Schema 180 (docs/adr/1000 §2): the review notes, read whole and checked.
            "reviewNotes" if has.review_notes => {
                let at = r.position();
                let value = r.opaque()?;
                let list: Vec<kentos_contracts::ReviewNote> = serde_json::from_value(value).map_err(|e| {
                    let words = e.to_string();
                    let code = if words.contains("unknown field") { Code::UnknownField } else { Code::BadValue };
                    r.fail_at(code, at, &format!("inceleme notları okunamadı: {words}"))
                })?;
                if list.is_empty() {
                    return Err(r.fail_at(Code::BadValue, at, "boş inceleme notu listesi yazılmaz"));
                }
                // Schema 215 (docs/adr/4140): a note's decision.
                if !has.review_decisions && kentos_contracts::review_notes_have_decisions(&list) {
                    return Err(r.fail_at(Code::UnknownField, at, "inceleme notunda decision (şema 215'ten önce yok)"));
                }
                if let Some(problem) = kentos_contracts::review_notes_problem(&list) {
                    return Err(r.fail_at(Code::BadValue, at, &problem));
                }
                review_notes = list;
            }
            // Schema 101 (docs/adr/0598 §1): what the split and merged objects came from, read whole and checked.
            "lineage" if has.lineage => lineage = gis::lineage(r)?,
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
                // Before schema 181 (docs/adr/1001 §1) a pattern held at most a LIN file's 12 elements.
                if !has.long_patterns
                    && let Some(t) = types.iter().find(|t| t.elements.len() > kentos_contracts::MAX_PATTERN_ELEMENTS)
                {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        &format!(
                            "“{}” çizgi tipinin deseni bu şemada en çok {} öğe olur, {} var",
                            t.name,
                            kentos_contracts::MAX_PATTERN_ELEMENTS,
                            t.elements.len()
                        ),
                    ));
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
                // Schema 159 (plan/road.md Y1.1): a road project's work.
                cad_work = Some(if has.road_work {
                    named(
                        r,
                        &[
                            ("architecture", CadWork::Architecture),
                            ("map", CadWork::Map),
                            ("road", CadWork::Road),
                        ],
                    )?
                } else {
                    named(
                        r,
                        &[
                            ("architecture", CadWork::Architecture),
                            ("map", CadWork::Map),
                        ],
                    )?
                })
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
                topology = Some(topology_settings(r, has.exception_digests, has.topology_reviews)?)
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
            "namedViews" if has.views => {
                let at = r.position();
                named_views =
                    views::named_views(r, has.viewmarks, has.view_recipes, has.recipe_detail, has.view_templates, has.options, has.option_combos)?;
                // Schema 231 (docs/adr/8039 §5): a recipe's Tadilat projesi view.
                if !has.renovation_view
                    && named_views.iter().any(|v| {
                        v.recipe.as_ref().is_some_and(|x| {
                            x.phase == Some(kentos_contracts::PhaseView::Renovation)
                        })
                    })
                {
                    return Err(r.fail_at(
                        Code::BadValue,
                        at,
                        "görünüm tarifinin Tadilat projesi görünümü (renovation) şema 231 ister",
                    ));
                }
            }
            // Schema 146 (docs/adr/0783 §2): the project's view templates, read whole as the contract's JSON.
            "viewTemplates" if has.view_templates => view_templates = views::view_templates(r)?,
            "textStyles" if has.styles => text_styles = styles::text_styles(r, has.shx_fonts)?,
            // Schema 67 (docs/adr/0326 §4): the leader styles, as their JSON.
            "wallStyles" if has.walls => wall_styles = arch::wall_styles(r, has)?,
            // Schema 159 (plan/road.md Y2.2): a road project's terrain surfaces' definitions.
            "road" if has.road_work => road = road::road(r, has)?,
            "arch" if has.walls => arch = arch::arch(r, has)?,
            "leaderStyles" if has.multileader => {
                let (l, at): (Vec<kentos_contracts::LeaderStyleDef>, usize) =
                    objects::json_part(r, "kılavuz stilleri")?;
                // Schema 187 (docs/adr/1025): a leader style's text style.
                if !has.leader_text_styles && l.iter().any(|s| s.text_style.is_some()) {
                    return Err(r.fail_at(Code::UnknownField, at, "kılavuz stilinin yazı stili belge şeması 187 gerektirir"));
                }
                if let Some((field, words)) =
                    l.iter().find_map(kentos_contracts::LeaderStyleDef::problem)
                {
                    return Err(r.fail_at(Code::BadValue, at, &format!("{field}: {words}")));
                }
                leader_styles = l;
            }
            "dimensionStyles" if has.styles => {
                dimension_styles =
                    styles::dimension_styles(r, has.annotation, has.dimension_options, has.dimension_text_styles)?
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
    // A view follows a template the project keeps (docs/adr/0783 §2).
    if let Some(problem) = kentos_contracts::templates_problem(&view_templates, &named_views) {
        return Err(relation("namedViews", &problem, r));
    }
    // A work is a CAD project's (docs/adr/0801).
    if cad_work.is_some() && workspace != Some(Workspace::Cad) {
        return Err(relation(
            "cadWork",
            "iş yalnız CAD projesinin olur (workspace cad); başka türde ya da türü seçilmemiş projede yazılmaz",
            r,
        ));
    }
    // A road's settings are a road project's (plan/road.md Y2.2); since schema 195 a map project's terrain surfaces
    // are written there too, and nothing else of a road (docs/adr/1520).
    if let Some(v) = &road {
        let map = cad_work == Some(CadWork::Map) && has.map_surfaces;
        // Since schema 197 a map project's graded pads too (docs/adr/1580); since 198 its drawings (docs/adr/1720).
        let fits = if has.map_drawings {
            v.map_drawn_only()
        } else if has.map_platforms {
            v.map_part_only()
        } else {
            v.surfaces_only()
        };
        // A zero line's drawing (docs/adr/1720) is schema 198's, in any project.
        if !has.map_drawings && v.drawings.iter().any(|d| d.command == "road.terrains.zeroline") {
            return Err(relation("road", "sıfır hattı çizimi şema 198'den önce yazılmaz (road.terrains.zeroline)", r));
        }
        // Cut and fill zones' drawing (docs/adr/1840) is schema 205's, in any project.
        if !has.cutfill_drawings && v.drawings.iter().any(|d| d.command == "road.terrains.cutfill") {
            return Err(relation("road", "kazı ve dolgu bölgeleri çizimi şema 205'ten önce yazılmaz (road.terrains.cutfill)", r));
        }
        // A pipe run profile's drawing and a network as a drawing's source (docs/adr/3560) are schema 212's, in any
        // project.
        if !has.pipe_profile_drawings
            && v.drawings.iter().any(|d| d.command == "cad.pipes.profile" || d.sources.iter().any(|s| s.kind == kentos_contracts::RoadSourceKind::Network))
        {
            return Err(relation("road", "hat profili çizimi ve ağ kaynağı şema 212'den önce yazılmaz (cad.pipes.profile)", r));
        }
        // A sheet's contour lines kept as a drawing, a layer as a drawing's source and drawing again whole
        // (docs/adr/4740) are schema 246's, in any project.
        if !has.sheet_contour_drawings && road_has_sheet_contour_drawings(v) {
            return Err(relation(
                "road",
                "pafta eğrileri çizimi, katman kaynağı ve bütün yeniden üretim şema 246'dan önce yazılmaz (road.terrains.sheetcontours)",
                r,
            ));
        }
        // A drawing record's version and revision (docs/adr/2840) are schema 209's.
        if !has.drawing_provenance && v.drawings.iter().any(|d| d.version.is_some() || d.revision.is_some()) {
            return Err(relation("road", "çizim kaydının işlem sürümü ve çıktı revizyonu şema 209'dan önce yazılmaz", r));
        }
        if cad_work != Some(CadWork::Road) && !(map && fits) {
            return Err(relation(
                "road",
                if map {
                    "harita projesinde yol ayarlarından yalnız arazi yüzeyleri, platformlar ve onlardan yapılan çizimler yazılır (cadWork map)"
                } else {
                    "yol ayarları yalnız yol projesinin olur (cadWork road); arazi yüzeyleri harita projesinin de olur (şema 195)"
                },
                r,
            ));
        }
    }
    // A map's work structure is a map project's (docs/adr/1200 §2).
    if map_survey.is_some() && cad_work != Some(CadWork::Map) {
        return Err(relation("mapSurvey", "GNSS arşivi yalnız harita projesinin olur (cadWork map)", r));
    }
    if map_work.is_some() && cad_work != Some(CadWork::Map) {
        return Err(relation("mapWork", "harita işi yalnız harita projesinin olur (cadWork map)", r));
    }
    Ok(ProjectSettings {
        road,
        map_work,
        map_survey,
        parcel_register,
        zoning_plan,
        survey_adjustments,
        monitoring_reviews,
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
        view_templates,
        forms,
        relations,
        join_views,
        linear_events,
        profile_files,
        standard_docs,
        approved_sheet_templates,
        table_styles,
        standard_exceptions,
        review_notes,
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

mod layers;
pub(crate) use layers::*;
