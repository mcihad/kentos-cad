//! The shared product command cases (fixtures/commands/v1, docs/adr/0022,
//! 0027, 0029, 0032, 0037, 0047, 0057, 0066) run against the desktop's handlers over the native document. The web
//! runs the same files against its own handlers
//! (apps/web/src/product/fixtures.test.ts); the format is in
//! fixtures/commands/README.md.
//!
//! JSON carries no NaN or ±∞: a step's `nonFinite` puts them into the typed
//! input after it is read, at the paths the errors name.

mod inputs_core;
mod inputs_arch;
mod inputs_map;
mod inputs_gis;
mod inputs_road;
mod run_core;
use run_core::core_cases;
mod run_arch;
use run_arch::arch_cases;
mod run_map;
use run_map::map_cases;
mod run_gis;
use run_gis::gis_cases;
mod run_road;
use run_road::road_cases;

use std::collections::HashMap;
use std::path::PathBuf;
use kentos_native_application::entities_select::isolation;

use kentos_commands::ChangesetApply;
use kentos_commands::{
    AddressesDefine, AnnotationsScales, BlockId, BlocksDefine, BlocksEdit, CAD_BLOCKS_DEFINE,
    CAD_BLOCKS_EDIT, CAD_ENTITIES_SET, DimensionsReassociate, EntitiesChamfer, EntitiesCreate,
    EntitiesFillet, EntitiesOffset, EntitiesOrder, EntitiesSetProperties, EntitiesTrim,
    FieldsUsage, HatchPatternsEdit, LabelsPin, LayersCompose, LayersContingents, LayersEdit,
    LayersFilter, LayersFind, LayersIndexes, LayersInfo, LayersLabels, LayersRenderer, LayersRules,
    LayersService, LayersSet, LayersStates, LayersSubtypes, LayersTable, LayersTime,
    LayersTranslate, LibraryEdit, LineTypesEdit, NetworkDefine, NurbsCreate, OffsetBy,
    ScenariosEdit, SettingsSet, SheetsCreate, SheetsEdit, SheetsFit, StandardsApply,
    StandardsCheck, StylesEdit, StylesImpact, UcsSet, UcsTo, ViewsEdit, ViewsOp,
};
use kentos_commands::{
    ArcCreate, ArrayLayout, CAD_ARC_CREATE, CAD_CIRCLE_CREATE, CAD_ENTITIES_ARRAY,
    CAD_ENTITIES_DELETE, CAD_ENTITIES_EDIT, CAD_ENTITIES_TRANSFORM, CAD_LINE_CREATE,
    CAD_POINT_CREATE, CAD_POLYGON_CREATE, CAD_POLYLINE_CREATE, CircleCreate, DocumentSnapshotV1,
    EntitiesArray, EntitiesDelete, EntitiesEdit, EntitiesTransform, EntityEdit, EntityGeometry,
    LineCreate, PointCreate, PolygonCreate, PolylineCreate, Transform,
};
use kentos_commands::{
    AttachmentsInspect, DomainsEdit, DomainsExport, FieldsMigrate, FormsAudit, FormsCompare,
    FormsEdit, FormsExport, FormsFill, FormsImport, FormsRecompute, FormsSources, FormsSubmit,
    FormsTest, FormsTransition, FormsValidate, RecordsCreate, RecordsDuplicate, RelationsEdit,
    SchemaDescribe,
};
use kentos_commands::{DatasetsDescribe, DatasetsEdit, JoinsEdit, JoinsQuery};
use kentos_commands::{
    DimensionMeasure, DimensionsCreate, Division, EntitiesBoundary, EntitiesBreak, EntitiesDivide,
    EntitiesHatch, EntitiesJoin, EntitiesStretch, HatchRegion, ImportDxf, ImportGpkg, ImportPdf,
    MeasuredPoint, UnitsChange,
};
use kentos_domain::{Document, Slot, Uuid};
#[cfg(feature = "mod-arch")]
use kentos_native_application::spaces_discover;
use kentos_native_application::{dimension_sets_create, dimension_sets_edit, dimension_sets_query};
#[cfg(feature = "mod-arch")]
use kentos_native_application::arch_exceptions;
#[cfg(feature = "mod-arch")]
use kentos_native_application::arch_recognize;
use kentos_native_application::dimensions_interior;
#[cfg(feature = "mod-arch")]
use kentos_native_application::spaces_separators;
#[cfg(feature = "mod-arch")]
use kentos_native_application::spaces_number;
#[cfg(feature = "mod-arch")]
use kentos_native_application::{program_check, program_edit};
use kentos_native_application::donut_create;
use kentos_native_application::drawing_compare;
use kentos_native_application::{drawing_take, review_notes};
use kentos_native_application::drawing_inspect;
use kentos_native_application::blocks_find;
use kentos_native_application::blocks_convert;
use kentos_native_application::arrays_adopt;
use kentos_native_application::arrays_find;
use kentos_native_application::fields_write;
use kentos_native_application::gpkg_tiles::{read as gpkg_tiles_read, write as gpkg_tiles_write};
#[cfg(feature = "mod-gis")]
use kentos_native_application::layers_crs::{
    assign as layers_crs_assign, reproject as layers_reproject,
};
#[cfg(feature = "mod-gis")]
use kentos_native_application::mesh_areas::{profile as meshes_profile, region as meshes_region};
#[cfg(feature = "mod-gis")]
use kentos_native_application::meshes::{describe as meshes_describe, query as meshes_query};
use kentos_native_application::pointcloud_areas::{
    profile as pointclouds_profile, region as pointclouds_region,
};
use kentos_native_application::pointclouds::{
    describe as pointclouds_describe, query as pointclouds_query,
};
use kentos_native_application::profile_files::{
    import as profiles_import, list as profiles_list, migrate as profiles_migrate,
    remove as profiles_remove,
};
use kentos_native_application::profiles::{apply as profiles_apply, check as profiles_check};
#[cfg(feature = "mod-gis")]
use kentos_native_application::records_page;
use kentos_native_application::sources::{
    describe as sources_describe, edit as sources_edit, reconcile as sources_reconcile,
    relate as sources_relate, sync as sources_sync,
};
#[cfg(feature = "mod-gis")]
use kentos_native_application::sources_query;
use kentos_native_application::standards_palettes;
use kentos_native_application::standards::{
    apply as standards_apply, check as standards_check, conflicts as standards_conflicts,
    resolve as standards_resolve,
};
#[cfg(feature = "mod-gis")]
use kentos_native_application::time_query;
#[cfg(feature = "mod-gis")]
use kentos_native_application::{sensors_ingest, sensors_latest};
use kentos_native_application::{
    DESKTOP_COMMANDS, ExecutionContext, address_define, adjoin_cmd, annotations_scales,
    annotations_write, arc, areas, array, blocks_define, blocks_edit, circle, constructions,
    crs_transform, curves_construct, delete, dimensions_edit, dimensions_query,
    dimensions_reassociate, edit, entities_chamfer, entities_extend, entities_fillet,
    entities_flatten, entities_grade, entities_order, entities_pick, entities_raise,
    entities_relate, entities_select, entities_trim, entities_vertex, fields_edit, fields_usage,
    fonts_report, hatches_edit, labels_pin, labels_report, layers_indexes, layers_labels,
    layers_renderer, layers_translate, leaders_edit, line, map_survey, map_survey_network,
    map_work, map_work_restore, nurbs, objects_derive, offset, plot_styles_import, point,
    points_construct, polygon, polygon_regular, polyline, profiles_carry, profiles_compare,
    profiles_names, rectangle_create, revcloud_create, sector_create, set, settings_set,
    shape_edits, sheets_create, sheets_details, sheets_edit, sheets_fit, sheets_levels, snap_find,
    styles_edit, tables_edit, texts_check, texts_edit, transform, ucs_set, units_change,
    zoning_check, zoning_draft, zoning_symbolize,
};
#[cfg(feature = "mod-arch")]
use kentos_native_application::{
    arch_audit, arch_ifc, casework_edit, casework_query, casework_run, columns_distances,
    elevations_edit, elevations_query, legends, levels_assign, materials_edit, modules_edit,
    modules_query, north, opening_types_edit, openings_create, openings_distances, openings_edit,
    openings_query, options_edit, options_query, revisions, sections_edit, sections_query,
    slabs_create, slabs_edit, spaces_create, spaces_edit, spaces_quantities, views_edit,
    walls_create, walls_distances, walls_edit, walls_from_lines, walls_joins, walls_partition,
    walls_rectangle,
};
#[cfg(feature = "mod-map")]
use kentos_native_application::{
    asbuilt_check, crs_factors, field_package, map_features, map_features_reconcile,
    map_survey_edit, map_work_edit, orthophoto_check, orthophoto_compare, orthophoto_seamlines,
    parcel_register, parcels, parcels_boundaries, parcels_check, pipes_check, pipes_profile,
    pointcloud_network, pointcloud_overlap, pointcloud_place, pointcloud_register, readjustment,
    readjustment_scope, readjustment_sheets, survey_adjust, survey_baselines, survey_points,
};
#[cfg(feature = "mod-gis")]
use kentos_native_application::{
    addresses_check, addresses_compare, addresses_normalize, images_clip, layers_compose,
    layers_contingents, layers_edit, layers_filter, layers_find, layers_info, layers_rules,
    layers_service, layers_set, layers_states, layers_subtypes, layers_table, layers_time,
    linearref_define, network_define, scenarios_edit, scenarios_impact, scenarios_record,
    time_version, topology_clean,
};
#[cfg(feature = "mod-road")]
use kentos_native_application::{
    road_alignments_check, road_alignments_create, road_alignments_edit, road_alignments_fit,
    road_alignments_impact, road_alignments_import, road_alignments_label, road_alignments_query,
    road_corridors_daylights, road_corridors_draw, road_corridors_edit, road_corridors_preflight,
    road_corridors_protected, road_corridors_query, road_corridors_water, road_culverts_closure,
    road_culverts_draw, road_culverts_edit, road_culverts_hydraulics, road_culverts_query,
    road_drainage, road_drainage_draw, road_guardrails_draw, road_guardrails_edit,
    road_guardrails_query, road_hauls_approve, road_hauls_edit, road_hauls_network,
    road_hauls_plan, road_hauls_query, road_hauls_trucks, road_ifc, road_junctions_clearance,
    road_junctions_draw, road_junctions_edit, road_junctions_overlap, road_junctions_query,
    road_junctions_tie, road_landxml, road_pay_edit, road_pay_query, road_platforms_draw,
    road_platforms_edit, road_platforms_query, road_profiles_check, road_profiles_create,
    road_profiles_draw, road_profiles_edit, road_profiles_fit, road_profiles_impact,
    road_profiles_lengths, road_profiles_propose, road_profiles_query, road_profiles_sight,
    road_project_edit, road_project_graph, road_project_produce, road_project_query,
    road_project_rebuild, road_routes_draw, road_routes_find, road_scene, road_sections_draw,
    road_sections_edit, road_sections_query, road_sheets, road_sheets_sections, road_stakeout,
    road_stakeout_send, road_superelevations_check, road_superelevations_edit,
    road_superelevations_query, road_superelevations_transitions, road_superelevations_widen,
    road_swept, road_swept_draw, road_templates_edit, road_terrain_cloud, road_terrains_analysis,
    road_terrains_compare, road_terrains_contours, road_terrains_coverage,
    road_terrains_coverage_draw, road_terrains_create, road_terrains_cutfill,
    road_terrains_density, road_terrains_edit, road_terrains_quality, road_terrains_query,
    road_terrains_repair, road_terrains_section, road_terrains_sheet, road_terrains_sparse,
    road_terrains_stockpile, road_terrains_zeroline, road_volumes_approve, road_volumes_compare,
    road_volumes_edit, road_volumes_history, road_volumes_ledger, road_volumes_progress,
    road_volumes_query, road_volumes_states, road_volumes_uncertainty, road_walls_draw,
    road_walls_edit, road_walls_query, roads_edit,
};
#[cfg(feature = "terrain")]
use kentos_native_application::{entities_drape, file_import_landxml};
#[cfg(feature = "mod-arch")]
use kentos_native_application::{
    arch_edit, beams_create, columns_create, grids_create, railings_create, ramps_create,
    roofs_create, stairs_create,
};
use kentos_native_application::{
    arrays_edit, changeset, constraints_edit, constraints_query, create, curves_edit,
    dimensions_create, divide, drawing_audit, entities_boundary, entities_break, entities_cleanup,
    entities_copy, entities_explode, entities_hatch, entities_join, entities_lengthen,
    entities_match, entities_paste, entities_reverse, file_import, file_import_csv,
    file_import_dwg, file_import_fgb, file_import_geojson, file_import_gml, file_import_gpkg,
    file_import_kml, file_import_parquet, file_import_pdf, file_import_shp, parameters_edit, qml,
    sld, stretch,
};
#[cfg(feature = "mod-gis")]
use kentos_native_application::{gis_derivation, gis_detach, gis_takeback, maplibre};
use kentos_native_application::{
    assets_check, inserts_clip, navigator, rasters_georef, sheets_set, xrefs_edit, xrefs_list,
};
#[cfg(feature = "mod-arch")]
use kentos_native_application::{
    spaces_finish, templates_apply, viewmarks_create, viewmarks_edit, viewmarks_references,
};
#[cfg(feature = "mod-gis")]
use kentos_native_application::{images_edit, rasters_mosaic, rasters_reproject};
use kentos_native_application::relations_edit;
#[cfg(feature = "mod-gis")]
use kentos_native_application::{
    attachments_inspect, domains_edit, domains_export, fields_migrate, forms_audit, forms_compare,
    forms_edit, forms_export, forms_fill, forms_import, forms_recompute, forms_sources,
    forms_submit, forms_test, forms_transition, forms_validate, records, records_duplicate,
    schema_describe,
};
use kentos_native_application::{
    block_params, blocks_freeze, blocks_insert, blocks_proxy, blocks_query, blocks_replace,
    file_import_block, hatch_patterns_edit, library_edit, line_types_edit, styles_impact,
};
use kentos_native_application::datasets_edit;
#[cfg(feature = "mod-gis")]
use kentos_native_application::{datasets_describe, joins_edit, joins_query};
use kentos_native_application::{subnetworks, subnetworks_update};
#[cfg(feature = "mod-gis")]
use kentos_native_application::{processing_batch, processing_run, processing_tools};
use kentos_native_application::{schedules_create, schedules_edit, schedules_query};
#[cfg(feature = "mod-arch")]
use kentos_native_application::{tags_check, tags_create, tags_edit};
use kentos_native_application::{extract_run, extract_table};
#[cfg(feature = "mod-gis")]
use kentos_native_application::{stac_host, stac_prepare, stac_search};
#[cfg(feature = "mod-gis")]
use kentos_native_application::{topology_check, topology_exceptions, topology_repair};
use serde_json::{Value, json};

type Outcome<T> = Result<T, String>;

#[derive(Default)]
struct State {
    /// Revisions taken by `captureRevision`, as the text commands use.
    revisions: HashMap<String, String>,
    /// Persistent ids taken by `captureUid`.
    uids: HashMap<String, Uuid>,
    /// Block ids taken by `captureBlock` (docs/adr/0144).
    blocks: HashMap<String, BlockId>,
    /// The ids of the setup's definitions: a new one is none of them.
    setup_blocks: Vec<BlockId>,
    /// The layer the last command added or would add (`$layer`, docs/adr/0208 §15).
    layer: Option<String>,
    /// The layers the last command that added any added or would add, by
    /// their number (`$layer:1`, `$layer:2` …, docs/adr/0210 §11).
    fresh: Vec<String>,
}

/// What a step of a command this build leaves out answers (docs/adr/8054 §11), the application's own words.
const LEFT_OUT: &str = "bu derlemede yok";

const STEP_KEYS: &[&str] = &[
    "op",
    "command",
    "input",
    "nonFinite",
    "result",
    "within",
    "relative",
    "returns",
    "as",
    "id",
    "name",
    "expect",
    "note",
    "relaxConstraints",
    "from",
    "to",
    "index",
];

/// JSON equality with numbers compared as numbers (the file's `1` is the document's `1.0`);
/// an expected `"$any"` takes whatever is there (a reader's report, docs/adr/0257 §3).
fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (_, Value::String(w)) if w == "$any" => true,
        (Value::Number(x), Value::Number(y)) => x.as_f64() == y.as_f64(),
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(p, q)| same(p, q))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| same(v, w)))
        }
        _ => a == b,
    }
}

/// [`same`] with numbers within `within` of each other (metres, radians): a result whose transcendental values
/// (an arc's angles, a tangent circle's centre) the reference rounds once from 60 digits and the core reaches
/// by a chain of roundings (docs/adr/0289 §4).
fn close(a: &Value, b: &Value, within: f64) -> bool {
    close_by(a, b, within, false)
}

/// [`close`], with `relative` the numbers within `within` of the expected one's size (at least 1): a result whose
/// numbers run from a ratio to thousands of litres, solved by two integrators (docs/adr/0936 §2).
fn close_by(a: &Value, b: &Value, within: f64, relative: bool) -> bool {
    match (a, b) {
        (_, Value::String(w)) if w == "$any" => true,
        (Value::Number(x), Value::Number(y)) => match (x.as_f64(), y.as_f64()) {
            (Some(x), Some(y)) => (x - y).abs() <= if relative { within * y.abs().max(1.0) } else { within },
            _ => false,
        },
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(p, q)| close_by(p, q, within, relative))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len()
                && x.iter()
                    .all(|(k, v)| y.get(k).is_some_and(|w| close_by(v, w, within, relative)))
        }
        _ => a == b,
    }
}

fn expect_same(got: &Value, want: &Value, what: &str, at: &str) -> Outcome<()> {
    if same(got, want) {
        Ok(())
    } else {
        Err(format!("{at}: {what}: beklenen {want}, bulunan {got}"))
    }
}

/// `value` with its `$…` placeholders filled in: `$current` is the
/// document's revision now, `$name` one `captureRevision` took; `$uid:name`,
/// anywhere in a text (an id list, a message), the persistent id
/// `captureUid` took, lowercase with hyphens; `$uidOf:12`, the persistent id
/// the object in slot 12 has now (a copy a command just wrote);
/// `$blockOf:Rögar`, the id of the drawing's block of that name now, and
/// `$block:name`, one `captureBlock` took (docs/adr/0144); `$layer:2`, the
/// second layer the last command that added any added or would add, by
/// their numbers (docs/adr/0210 §11); `$array:0`, the id of the drawing's
/// first associative array now (docs/adr/0301).
fn fill(value: &Value, doc: &Document, state: &State, at: &str) -> Outcome<Value> {
    Ok(match value {
        // “$$…” is the text “$…” itself (a change set's reference to a step, docs/adr/0253 §8).
        Value::String(text) if text.starts_with("$$") => Value::String(text[1..].to_owned()),
        // Whatever the command gives (compared by `same`).
        Value::String(text) if text == "$any" => value.clone(),
        // A fixture file's absolute path, anywhere in a text (docs/adr/0257 §3).
        Value::String(text) if (text.contains("$fixture:") || text.contains("$tmp:")) => {
            Value::String(with_fixtures(text))
        }
        Value::String(text) if text.starts_with("$blockOf:") => {
            let name = &text["$blockOf:".len()..];
            let block = doc
                .block_named(name)
                .ok_or_else(|| format!("{at}: “{name}” bloğu çizimde yok"))?;
            Value::String(block.id.to_text())
        }
        Value::String(text) if text.starts_with("$block:") => {
            let name = &text["$block:".len()..];
            let id = state
                .blocks
                .get(name)
                .ok_or_else(|| format!("{at}: “{name}” blok kimliği alınmadı"))?;
            Value::String(id.to_text())
        }
        Value::String(text) if text.starts_with("$uidOf:") => {
            let slot = text["$uidOf:".len()..]
                .parse::<u32>()
                .map_err(|_| format!("{at}: {text}: yuva bir sayı olmalı"))?;
            let uid = doc
                .uid(Slot(slot))
                .ok_or_else(|| format!("{at}: {slot} yuvasında nesne yok"))?;
            Value::String(uid.to_string())
        }
        // Inside a text (a finding's key names its objects, docs/adr/0732 §1): each `$uidOf:12` its slot's id.
        Value::String(text) if text.contains("$uidOf:") => {
            let mut out = String::new();
            let mut rest = text.as_str();
            while let Some(start) = rest.find("$uidOf:") {
                out.push_str(&rest[..start]);
                let digits: String = rest[start + "$uidOf:".len()..].chars().take_while(char::is_ascii_digit).collect();
                let slot = digits
                    .parse::<u32>()
                    .map_err(|_| format!("{at}: {text}: yuva bir sayı olmalı"))?;
                let uid = doc
                    .uid(Slot(slot))
                    .ok_or_else(|| format!("{at}: {slot} yuvasında nesne yok"))?;
                out.push_str(&uid.to_string());
                rest = &rest[start + "$uidOf:".len() + digits.len()..];
            }
            out.push_str(rest);
            Value::String(out)
        }
        // The drawing's constraint at this place now (docs/adr/0335).
        Value::String(text) if text.starts_with("$constraint:") => {
            let k = text["$constraint:".len()..]
                .parse::<usize>()
                .map_err(|_| format!("{at}: {text}: sıra bir sayı olmalı"))?;
            let c = doc
                .constraints()
                .constraints
                .get(k)
                .ok_or_else(|| format!("{at}: {text}: çizimde {k}. kısıt yok"))?;
            Value::String(c.id.to_string())
        }
        // The drawing's associative array at this place now (docs/adr/0301).
        Value::String(text) if text.starts_with("$array:") => {
            let k = text["$array:".len()..]
                .parse::<usize>()
                .map_err(|_| format!("{at}: {text}: sıra bir sayı olmalı"))?;
            let array = doc
                .arrays()
                .get(k)
                .ok_or_else(|| format!("{at}: {text}: çizimde {k}. dizi yok"))?;
            Value::String(array.id.to_text())
        }
        Value::String(text) if text.contains("$uid:") => Value::String(with_uids(text, state, at)?),
        Value::String(text) if text.starts_with("$layer:") => {
            let k = text["$layer:".len()..]
                .parse::<usize>()
                .ok()
                .filter(|k| *k >= 1)
                .ok_or_else(|| format!("{at}: {text}: sıra 1'den başlayan bir sayı olmalı"))?;
            Value::String(state.fresh.get(k - 1).cloned().ok_or_else(|| {
                format!(
                    "{at}: {text}: son komut {} katman ekledi",
                    state.fresh.len()
                )
            })?)
        }
        Value::String(text) if text == "$layer" => Value::String(
            state
                .layer
                .clone()
                .ok_or_else(|| format!("{at}: $layer: bir komut katman eklemedi"))?,
        ),
        Value::String(text) if text.starts_with('$') => {
            let name = &text[1..];
            let revision = if name == "current" {
                doc.revision().to_string()
            } else {
                state
                    .revisions
                    .get(name)
                    .cloned()
                    .ok_or_else(|| format!("{at}: “{name}” sürümü alınmadı"))?
            };
            Value::String(revision)
        }
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|v| fill(v, doc, state, at))
                .collect::<Outcome<_>>()?,
        ),
        Value::Object(fields) => Value::Object(
            fields
                .iter()
                .map(|(k, v)| Ok((k.clone(), fill(v, doc, state, at)?)))
                .collect::<Outcome<_>>()?,
        ),
        other => other.clone(),
    })
}

/// An expected object with its block ids (`$blockOf:Ad`, `$block:name`,
/// docs/adr/0144) and the persistent ids of the objects it names (`$uidOf:12`:
/// a linked text's object, docs/adr/0175 §4) filled in; nothing else of it
/// is read as a placeholder.
fn block_ids(value: &Value, doc: &Document, state: &State, at: &str) -> Outcome<Value> {
    Ok(match value {
        Value::String(text)
            if text.starts_with("$blockOf:")
                || text.starts_with("$block:")
                || text.starts_with("$uidOf:")
                // A captured object's id: a relation's parent the child refers to (docs/adr/0405).
                || text.starts_with("$uid:")
                || text.starts_with("$layer:")
                // An external reference's source file (docs/adr/0348).
                || (text.contains("$fixture:") || text.contains("$tmp:"))
                || text.starts_with("$$") =>
        {
            fill(value, doc, state, at)?
        }
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|v| block_ids(v, doc, state, at))
                .collect::<Outcome<_>>()?,
        ),
        Value::Object(fields) => Value::Object(
            fields
                .iter()
                .map(|(k, v)| Ok((k.clone(), block_ids(v, doc, state, at)?)))
                .collect::<Outcome<_>>()?,
        ),
        other => other.clone(),
    })
}

/// `text` with every `$fixture:rel/path` (up to a space) replaced by the
/// absolute path of `fixtures/rel/path`.
fn with_fixtures(text: &str) -> String {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures");
    let out = with_root(text, "$fixture:", &root);
    if out.contains("$tmp:") {
        with_root(&out, "$tmp:", &tmp_folder())
    } else {
        out
    }
}

/// `text` with every `marker` path (up to a space) made `root`'s.
fn with_root(text: &str, marker: &str, root: &std::path::Path) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find(marker) {
        out.push_str(&rest[..start]);
        let after = &rest[start + marker.len()..];
        let end = after.find(char::is_whitespace).unwrap_or(after.len());
        out.push_str(&root.join(&after[..end]).display().to_string());
        rest = &after[end..];
    }
    out.push_str(rest);
    out
}

/// The folder `$tmp:name` files are written in (a command that writes a file, docs/adr/0517): the test run's own,
/// made empty once, under the build's own temporary folder (`CARGO_TARGET_TMPDIR`). It was under the system's
/// (`/tmp`), never removed: 91 runs' folders (48 MB each, 3.2 GB) filled a memory-backed `/tmp` until an environment
/// could not be installed (14 November 2026). An earlier run's folder whose process is gone is removed here.
fn tmp_folder() -> PathBuf {
    static FOLDER: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    FOLDER
        .get_or_init(|| {
            let base = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
            let own = std::process::id();
            for entry in std::fs::read_dir(&base).into_iter().flatten().flatten() {
                let name = entry.file_name();
                let Some(pid) = name
                    .to_str()
                    .and_then(|n| n.strip_prefix("kentos-cases-"))
                    .and_then(|p| p.parse::<u32>().ok())
                else {
                    continue;
                };
                // Gone: no such process (where the system says it), or a day old.
                let gone = std::path::Path::new("/proc").is_dir()
                    && !std::path::Path::new(&format!("/proc/{pid}")).exists();
                let old = entry
                    .metadata()
                    .and_then(|m| m.modified())
                    .ok()
                    .and_then(|t| t.elapsed().ok())
                    .is_some_and(|age| age.as_secs() > 24 * 60 * 60);
                if pid != own && (gone || old) {
                    let _ = std::fs::remove_dir_all(entry.path());
                }
            }
            let dir = base.join(format!("kentos-cases-{own}"));
            let _ = std::fs::remove_dir_all(&dir);
            let _ = std::fs::create_dir_all(&dir);
            dir
        })
        .clone()
}

/// `text` with every `$uid:name` replaced by the persistent id `captureUid` took as `name`.
fn with_uids(text: &str, state: &State, at: &str) -> Outcome<String> {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("$uid:") {
        out.push_str(&rest[..start]);
        let after = &rest[start + "$uid:".len()..];
        let end = after
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-'))
            .unwrap_or(after.len());
        let name = &after[..end];
        let uid = state
            .uids
            .get(name)
            .ok_or_else(|| format!("{at}: “{name}” kimliği alınmadı"))?;
        out.push_str(&uid.to_string());
        rest = &after[end..];
    }
    out.push_str(rest);
    Ok(out)
}

/// The number of a point list at `rest` (`pts[1].y`, after any ring prefix).
fn point_number<'a>(pts: &'a mut [kentos_commands::Vec2], rest: &str) -> Option<&'a mut f64> {
    let (i, axis) = rest.strip_prefix("pts[")?.split_once("].")?;
    let p = pts.get_mut(i.parse::<usize>().ok()?)?;
    match axis {
        "x" => Some(&mut p.x),
        "y" => Some(&mut p.y),
        _ => None,
    }
}

/// The bulge at `rest` (`bulges[3]`, after any ring prefix).
fn bulge_number<'a>(bulges: &'a mut Option<Vec<f64>>, rest: &str) -> Option<&'a mut f64> {
    let i = rest.strip_prefix("bulges[")?.strip_suffix(']')?;
    bulges.as_mut()?.get_mut(i.parse::<usize>().ok()?)
}

/// A command's typed input, with the numbers `nonFinite` may name.
trait Input {
    /// The number at an error path; `None` when the input has no such place.
    fn number(&mut self, path: &str) -> Option<&mut f64>;
}

// The constraint commands' numbers are in expressions, not the input's numbers (docs/adr/0335).


// The topology commands take no number (docs/adr/0589).



// The forms' inputs have no number a case makes NaN (docs/adr/0401 §9).































/// `list[i]` of a list of numbers.
fn list_number<'a>(list: &'a mut [f64], name: &str, rest: &str) -> Option<&'a mut f64> {
    let i = rest
        .strip_prefix(name)?
        .strip_prefix('[')?
        .strip_suffix(']')?;
    list.get_mut(i.parse::<usize>().ok()?)
}

/// `name[i].x` of a list of points, as `pts[i].x`.
fn named_point<'a>(
    pts: &'a mut [kentos_commands::Vec2],
    name: &str,
    rest: &str,
) -> Option<&'a mut f64> {
    let after = rest.strip_prefix(name)?;
    point_number(pts, &format!("pts{after}"))
}



















/// A roof's part's `parts[k].at.x` and `.y`, `.width`, `.depth`, `.rotation`, `.rise` (docs/adr/8044 §6).
fn roof_part_number<'a>(
    parts: Option<&'a mut Vec<kentos_commands::RoofPart>>,
    rest: &str,
) -> Option<&'a mut f64> {
    let (k, field) = rest.strip_prefix("parts[")?.split_once("].")?;
    let part = parts?.get_mut(k.parse::<usize>().ok()?)?;
    match field {
        "at.x" => Some(&mut part.at.x),
        "at.y" => Some(&mut part.at.y),
        "width" => Some(&mut part.width),
        "depth" => Some(&mut part.depth),
        "rotation" => part.rotation.as_mut(),
        "rise" => part.rise.as_mut(),
        _ => None,
    }
}













































/// `at.x`, `north.angle`, `size` (docs/adr/0721 §3).

/// `at.x`, `size`, `arc`, the frame's points (docs/adr/0721 §4).

/// `at.x`, `height` (docs/adr/0721 §5).





























/// No number of these is a NaN or ±∞ case (docs/adr/0210 §11).








/// `min.x` … `max.y` of a rectangle given as `TrimBoundaries::Area`.
fn area_number<'a>(
    b: &'a mut Option<kentos_commands::TrimBoundaries>,
    rest: &str,
) -> Option<&'a mut f64> {
    let Some(kentos_commands::TrimBoundaries::Area { min, max }) = b else {
        return None;
    };
    match rest {
        "min.x" => Some(&mut min.x),
        "min.y" => Some(&mut min.y),
        "max.x" => Some(&mut max.x),
        "max.y" => Some(&mut max.y),
        _ => None,
    }
}


/// A coordinate of a point by its axis name.
fn axis<'a>(p: &'a mut kentos_commands::Vec2, a: &str) -> Option<&'a mut f64> {
    match a {
        "x" => Some(&mut p.x),
        "y" => Some(&mut p.y),
        _ => None,
    }
}











// Yeni sürüm oluştur takes no number: its date is text (docs/adr/0622 §9).









// Ids and a pattern: no number a case makes NaN (docs/adr/0501).



/// A coordinate of a point given by its coordinates.
fn measured_number<'a>(
    p: &'a mut kentos_commands::MeasuredPoint,
    axis: &str,
) -> Option<&'a mut f64> {
    match (p, axis) {
        (kentos_commands::MeasuredPoint::At { at }, "x") => Some(&mut at.x),
        (kentos_commands::MeasuredPoint::At { at }, "y") => Some(&mut at.y),
        _ => None,
    }
}



















































/// No style's number is a NaN or ±∞ case: the table's rule refuses them as
/// any other value out of its bounds (docs/adr/0248 §4).









// No number: a path and rows (docs/adr/0634 §1).

// No number: a path and table names (docs/adr/0634 §2).



































// KentOS Yol's alignments (plan/road.md Y3): no number is put at an error's path.




// KentOS Yol's profiles (plan/road.md Y4): no number is put at an error's path.









































































// KentOS Yol's terrain surfaces (plan/road.md Y2.3): no number is put at an error's path.






















/// A coordinate of a named point: `c.x`, `p.y`.
fn coordinate<'a>(p: &'a mut kentos_commands::Vec2, name: &str, path: &str) -> Option<&'a mut f64> {
    match path.strip_prefix(name)?.strip_prefix('.')? {
        "x" => Some(&mut p.x),
        "y" => Some(&mut p.y),
        _ => None,
    }
}





/// `name[i]` of a fixed list of numbers.
fn indexed<'a>(values: &'a mut [f64], name: &str, path: &str) -> Option<&'a mut f64> {
    let i: usize = path
        .strip_prefix(name)?
        .strip_prefix('[')?
        .strip_suffix(']')?
        .parse()
        .ok()?;
    values.get_mut(i)
}




/// An elevation `zs[i]` written with a geometry (docs/adr/0142): only one
/// given as a number can be made non-finite.
fn elevation_number<'a>(zs: &'a mut Option<Vec<Option<f64>>>, rest: &str) -> Option<&'a mut f64> {
    let i: usize = rest.strip_prefix("zs[")?.strip_suffix(']')?.parse().ok()?;
    zs.as_mut()?.get_mut(i)?.as_mut()
}

/// A measure `ms[i]` written with a geometry (docs/adr/0224 §2), as an elevation.
fn measure_number<'a>(ms: &'a mut Option<Vec<Option<f64>>>, rest: &str) -> Option<&'a mut f64> {
    let i: usize = rest.strip_prefix("ms[")?.strip_suffix(']')?.parse().ok()?;
    ms.as_mut()?.get_mut(i)?.as_mut()
}

/// The number of a geometry at `rest`: `a.x`, `r`, `pts[1].y`, `bulges[0]`,
/// `major.y`, `ratio`, `dir.x`, a text's `height`, `ring[2].x`, `pattern.angle` …
fn geometry_number<'a>(geometry: &'a mut EntityGeometry, rest: &str) -> Option<&'a mut f64> {
    match geometry {
        EntityGeometry::Point { p, z, .. } => match rest {
            "z" => z.as_mut(),
            _ => coordinate(p, "p", rest),
        },
        EntityGeometry::Line { a, b, zs, ms } => {
            if rest.starts_with("zs[") {
                return elevation_number(zs, rest);
            }
            if rest.starts_with("ms[") {
                return measure_number(ms, rest);
            }
            coordinate(a, "a", rest).or_else(|| coordinate(b, "b", rest))
        }
        EntityGeometry::Polyline {
            pts,
            bulges,
            zs,
            ms,
            ..
        } => {
            if rest.starts_with("pts[") {
                point_number(pts, rest)
            } else if rest.starts_with("zs[") {
                elevation_number(zs, rest)
            } else if rest.starts_with("ms[") {
                measure_number(ms, rest)
            } else {
                bulge_number(bulges, rest)
            }
        }
        EntityGeometry::Polygon {
            pts,
            bulges,
            zs,
            parts,
            ..
        } => {
            // A part's own numbers: `parts[0].pts[1].x`, `parts[0].bulges[2]`, `parts[0].zs[1]` (docs/adr/0143).
            if let Some(after) = rest.strip_prefix("parts[") {
                let (k, inner) = after.split_once("].")?;
                let part = parts.as_mut()?.get_mut(k.parse::<usize>().ok()?)?;
                return if inner.starts_with("pts[") {
                    point_number(&mut part.pts, inner)
                } else if inner.starts_with("zs[") {
                    elevation_number(&mut part.zs, inner)
                } else {
                    bulge_number(&mut part.bulges, inner)
                };
            }
            if rest.starts_with("pts[") {
                point_number(pts, rest)
            } else if rest.starts_with("zs[") {
                elevation_number(zs, rest)
            } else {
                bulge_number(bulges, rest)
            }
        }
        EntityGeometry::Circle { c, r } => match rest {
            "r" => Some(r),
            _ => coordinate(c, "c", rest),
        },
        EntityGeometry::Arc { c, r, a0, a1 } => match rest {
            "r" => Some(r),
            "a0" => Some(a0),
            "a1" => Some(a1),
            _ => coordinate(c, "c", rest),
        },
        EntityGeometry::Ellipse {
            c,
            major,
            ratio,
            t0,
            t1,
        } => match rest {
            "ratio" => Some(ratio),
            "t0" => Some(t0),
            "t1" => Some(t1),
            _ => coordinate(c, "c", rest).or_else(|| coordinate(major, "major", rest)),
        },
        EntityGeometry::Spline { pts, .. } => point_number(pts, rest),
        EntityGeometry::Nurbs {
            ctrl,
            knots,
            weights,
            ..
        } => named_point(ctrl, "ctrl", rest)
            .or_else(|| list_number(knots, "knots", rest))
            .or_else(|| list_number(weights.as_deref_mut()?, "weights", rest)),
        EntityGeometry::Xline { p, dir } | EntityGeometry::Ray { p, dir } => {
            coordinate(p, "p", rest).or_else(|| coordinate(dir, "dir", rest))
        }
        EntityGeometry::Text {
            p,
            height,
            rotation,
            width_factor,
            box_width,
            line_spacing,
            face,
            ..
        } => match rest {
            "height" => Some(height),
            "rotation" => Some(rotation),
            // The width factor the case gives (docs/adr/0145).
            "widthFactor" => width_factor.as_mut(),
            // A multi-line text's box and spacing the case gives (docs/adr/0182).
            "boxWidth" => box_width.as_mut(),
            "lineSpacing" => line_spacing.as_mut(),
            // A text's slant the case gives (docs/adr/0183).
            "oblique" => face.oblique.as_mut(),
            _ => coordinate(p, "p", rest),
        },
        // A slope's elevations and an arc length's or jogged radius's centre too (docs/adr/0147).
        EntityGeometry::Dimension {
            a,
            b,
            offset,
            height,
            c,
            za,
            zb,
            look,
            ..
        } => match rest {
            "offset" => Some(offset),
            "height" => Some(height),
            "za" => za.as_mut(),
            "zb" => zb.as_mut(),
            // A dimension's sizes the case gives (docs/adr/0183).
            "arrowSize" => look.arrow_size.as_mut(),
            "extOffset" => look.ext_offset.as_mut(),
            "extBeyond" => look.ext_beyond.as_mut(),
            "textGap" => look.text_gap.as_mut(),
            _ => coordinate(a, "a", rest)
                .or_else(|| coordinate(b, "b", rest))
                .or_else(|| c.as_mut().and_then(|c| coordinate(c, "c", rest))),
        },
        EntityGeometry::Insert {
            p,
            scale,
            y_scale,
            rotation,
            ..
        } => match rest {
            "scale" => Some(scale),
            "yScale" => y_scale.as_mut(),
            "rotation" => Some(rotation),
            _ => coordinate(p, "p", rest),
        },
        // A leader's height, turn and vertices (docs/adr/0146).
        EntityGeometry::Leader {
            pts,
            height,
            rotation,
            ..
        } => match rest {
            "height" => Some(height),
            "rotation" => Some(rotation),
            _ => point_number(pts, rest),
        },
        // A table's sizes and corner (docs/adr/0184): `rows[1]`, `columns[0]`, `height`, `p.x` ….
        // A picture's corner, size, turn and opacity (docs/adr/0192).
        EntityGeometry::Image(i) => match rest {
            "width" => Some(&mut i.width),
            "height" => Some(&mut i.height),
            "rotation" => Some(&mut i.rotation),
            "opacity" => i.opacity.as_mut(),
            _ => coordinate(&mut i.p, "p", rest),
        },
        // A raster's affine, its look's numbers and its opacity (docs/adr/0204): `affine[3]`, `style.min` ….
        EntityGeometry::Raster(r) => match rest {
            "opacity" => r.opacity.as_mut(),
            "style.min" => r.style.min.as_mut(),
            "style.max" => r.style.max.as_mut(),
            "style.azimuth" => r.style.azimuth.as_mut(),
            "style.altitude" => r.style.altitude.as_mut(),
            "style.zFactor" => r.style.z_factor.as_mut(),
            "style.nodata" => r.style.nodata.as_mut(),
            _ => {
                let i: usize = rest
                    .strip_prefix("affine[")?
                    .strip_suffix(']')?
                    .parse()
                    .ok()?;
                r.affine.get_mut(i)
            }
        },
        // A wall's path, bulges, width, offset and components' thicknesses (docs/adr/0339): `pts[1].y`, `bulges[0]`,
        // `width`, `offset`, `components[1].thickness`.
        EntityGeometry::Wall(w) => match rest {
            "width" => Some(&mut w.width),
            "offset" => Some(&mut w.offset),
            "height" => w.height.as_mut(),
            _ if rest.starts_with("pts[") => point_number(&mut w.pts, rest),
            _ if rest.starts_with("bulges[") => bulge_number(&mut w.bulges, rest),
            _ => {
                let (k, field) = rest.strip_prefix("components[")?.split_once("].")?;
                let c = w.components.get_mut(k.parse::<usize>().ok()?)?;
                match field {
                    "thickness" => c.thickness.as_mut(),
                    _ => None,
                }
            }
        },
        // A point cloud's bounds, its files', its look's numbers and its opacity
        // (docs/adr/0207 §3): `bounds[2]`, `sources[0].bounds[5]`, `style.size` ….
        EntityGeometry::PointCloud(c) => match rest {
            "opacity" => c.opacity.as_mut(),
            "style.min" => c.style.min.as_mut(),
            "style.max" => c.style.max.as_mut(),
            "style.size" => Some(&mut c.style.size),
            _ => {
                let index = |text: &str| -> Option<usize> {
                    text.strip_prefix("bounds[")?
                        .strip_suffix(']')?
                        .parse()
                        .ok()
                };
                if let Some(i) = index(rest) {
                    return c.bounds.get_mut(i);
                }
                let (k, inner) = rest.strip_prefix("sources[")?.split_once("].")?;
                let k: usize = k.parse().ok()?;
                c.sources.get_mut(k)?.bounds.get_mut(index(inner)?)
            }
        },
        EntityGeometry::Table {
            p,
            rotation,
            height,
            rows,
            columns,
            ..
        } => match rest {
            "height" => Some(height),
            "rotation" => Some(rotation),
            _ => {
                let item = |list: &'a mut Vec<f64>, name: &str| {
                    let i: usize = rest
                        .strip_prefix(name)?
                        .strip_prefix('[')?
                        .strip_suffix(']')?
                        .parse()
                        .ok()?;
                    list.get_mut(i)
                };
                if rest.starts_with("rows[") {
                    item(rows, "rows")
                } else if rest.starts_with("columns[") {
                    item(columns, "columns")
                } else {
                    coordinate(p, "p", rest)
                }
            }
        },
        EntityGeometry::Hatch { ring, pattern, .. } => match rest {
            "pattern.angle" => Some(&mut pattern.angle),
            "pattern.spacing" => Some(&mut pattern.spacing),
            _ => {
                let (i, axis) = rest.strip_prefix("ring[")?.split_once("].")?;
                let p = ring.get_mut(i.parse::<usize>().ok()?)?;
                match axis {
                    "x" => Some(&mut p.x),
                    "y" => Some(&mut p.y),
                    _ => None,
                }
            }
        },
    }
}

/// Puts NaN or ±∞ into the typed input at the paths the step's `nonFinite` names.
fn put_non_finite(input: &mut impl Input, step: &Value, at: &str) -> Outcome<()> {
    let Some(table) = step.get("nonFinite") else {
        return Ok(());
    };
    let table = table
        .as_object()
        .ok_or_else(|| format!("{at}: nonFinite bir nesne olmalı"))?;
    for (path, value) in table {
        let number = match value.as_str() {
            Some("NaN") => f64::NAN,
            Some("Infinity") => f64::INFINITY,
            Some("-Infinity") => f64::NEG_INFINITY,
            _ => {
                return Err(format!(
                    "{at}: {path}: NaN, Infinity ya da -Infinity olmalı"
                ));
            }
        };
        *input
            .number(path)
            .ok_or_else(|| format!("{at}: girdide böyle bir yer yok: {path}"))? = number;
    }
    Ok(())
}

/// One command's `op` on the document, in a frame of its own: `run_op` names hundreds of commands, and in a debug
/// build each one's input and answer would take room in that one frame at once (a deep case, a DWG read, ran out of
/// a test thread's stack).
#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn run_typed<I, V, P, O>(
    op: &str,
    doc: &mut Document,
    input: Value,
    step: &Value,
    at: &str,
    validate: impl FnOnce(&ExecutionContext<'_>, &I) -> V,
    plan: impl FnOnce(&ExecutionContext<'_>, &I) -> P,
    execute: impl FnOnce(&mut ExecutionContext<'_>, I) -> O,
) -> Outcome<serde_json::Result<Value>>
where
    I: serde::de::DeserializeOwned + Input,
    V: serde::Serialize,
    P: serde::Serialize,
    O: serde::Serialize,
{
    let mut input: I =
        serde_json::from_value(input).map_err(|e| format!("{at}: girdi okunamadı: {e}"))?;
    put_non_finite(&mut input, step, at)?;
    Ok(match op {
        "validate" => serde_json::to_value(validate(&ExecutionContext::new(doc), &input)),
        "plan" => serde_json::to_value(plan(&ExecutionContext::new(doc), &input)),
        _ => serde_json::to_value(execute(&mut ExecutionContext::new(doc), input)),
    })
}

/// Runs `op` of the fixture's command on the document; its whole result as the wire carries it.
fn run_op(
    command: &str,
    op: &str,
    doc: &mut Document,
    input: Value,
    step: &Value,
    at: &str,
) -> Outcome<Value> {
    // The step's input, read as the command's type, with its non-finite numbers put in.
    macro_rules! run {
        ($module:ident, $input:ty) => {{
            run_typed::<$input, _, _, _>(
                op,
                doc,
                input,
                step,
                at,
                |cx, i| $module::validate(cx, i),
                |cx, i| $module::plan(cx, i),
                |cx, i| $module::execute(cx, i),
            )?
        }};
    }
    // A command of a discipline this build leaves out (docs/adr/8054 §11): its handler is not named, the step stops
    // the case and the case is left out, not failed.
    #[allow(unused_macros)]
    macro_rules! left_out {
        ($module:ident, $input:ty) => {
            return Err(format!("{at}: {command} {LEFT_OUT}"))
        };
    }
    #[cfg(feature = "mod-arch")]
    macro_rules! arch {
        ($module:ident, $input:ty) => {
            run!($module, $input)
        };
    }
    #[cfg(not(feature = "mod-arch"))]
    macro_rules! arch {
        ($module:ident, $input:ty) => {
            left_out!($module, $input)
        };
    }
    #[cfg(feature = "mod-map")]
    macro_rules! map {
        ($module:ident, $input:ty) => {
            run!($module, $input)
        };
    }
    #[cfg(not(feature = "mod-map"))]
    macro_rules! map {
        ($module:ident, $input:ty) => {
            left_out!($module, $input)
        };
    }
    #[cfg(feature = "mod-gis")]
    macro_rules! gis {
        ($module:ident, $input:ty) => {
            run!($module, $input)
        };
    }
    #[cfg(not(feature = "mod-gis"))]
    macro_rules! gis {
        ($module:ident, $input:ty) => {
            left_out!($module, $input)
        };
    }
    #[cfg(feature = "mod-road")]
    macro_rules! road {
        ($module:ident, $input:ty) => {
            run!($module, $input)
        };
    }
    #[cfg(not(feature = "mod-road"))]
    macro_rules! road {
        ($module:ident, $input:ty) => {
            left_out!($module, $input)
        };
    }
    // A terrain surface's (road design's or mapping's) core commands: LandXML in, drape.
    #[cfg(feature = "terrain")]
    macro_rules! terrain {
        ($module:ident, $input:ty) => {
            run!($module, $input)
        };
    }
    #[cfg(not(feature = "terrain"))]
    macro_rules! terrain {
        ($module:ident, $input:ty) => {
            left_out!($module, $input)
        };
    }
    core_cases!(
        command,
        run,
        arch_cases!(
            command,
            arch,
            map_cases!(
                command,
                map,
                gis_cases!(
                    command,
                    gis,
                    road_cases!(
                        command,
                        road,
                        return Err(format!("{at}: {command} için koşucu yok"))
                    )
                )
            )
        )
    )
    .map_err(|e| format!("{at}: sonuç yazılamadı: {e}"))
}

/// Runs one command step and compares its whole result.
fn run_command(
    command: &str,
    doc: &mut Document,
    state: &mut State,
    step: &Value,
    at: &str,
) -> Outcome<()> {
    let op = step["op"].as_str().unwrap_or("?");
    let input = fill(&step["input"], doc, state, at)?;
    let before = layer_ids(doc.layers().nodes());
    let got = run_op(command, op, doc, input, step, at)?;
    let Some(want) = step.get("result") else {
        return Err(format!("{at}: komut adımında “result” yok"));
    };
    let mut want = want.clone();
    // “$uid”: the persistent id of the object the command wrote, lowercase with hyphens.
    if want.pointer("/output/uid") == Some(&json!("$uid")) {
        let text = got
            .pointer("/output/uid")
            .and_then(Value::as_str)
            .unwrap_or("");
        let slot = got.pointer("/output/id").and_then(Value::as_u64);
        let uid =
            Uuid::parse_str(text).map_err(|_| format!("{at}: kimlik UUID değil: “{text}”"))?;
        let own = slot
            .and_then(|s| u32::try_from(s).ok())
            .and_then(|s| doc.uid(Slot(s)));
        if uid.to_string() != text || own != Some(uid) {
            return Err(format!(
                "{at}: output.uid {text}, yuvadaki nesnenin kimliği {own:?}"
            ));
        }
        if let Some(field) = want.pointer_mut("/output/uid") {
            *field = Value::String(text.to_owned());
        }
    }
    // “$uid” in a list of ids (cad.records.create, docs/adr/0410): each the persistent id of the slot beside it.
    let listed = want
        .pointer("/output/uids")
        .and_then(Value::as_array)
        .is_some_and(|l| l.iter().any(|v| v == "$uid"));
    if listed {
        let n = want
            .pointer("/output/uids")
            .and_then(Value::as_array)
            .map_or(0, Vec::len);
        for i in 0..n {
            if want.pointer(&format!("/output/uids/{i}")) != Some(&json!("$uid")) {
                continue;
            }
            let text = got
                .pointer(&format!("/output/uids/{i}"))
                .and_then(Value::as_str)
                .unwrap_or("");
            let own = got
                .pointer(&format!("/output/ids/{i}"))
                .and_then(Value::as_u64)
                .and_then(|s| u32::try_from(s).ok())
                .and_then(|s| doc.uid(Slot(s)));
            if own.map(|u| u.to_string()).as_deref() != Some(text) {
                return Err(format!(
                    "{at}: output.uids[{i}] {text}, yuvadaki nesnenin kimliği {own:?}"
                ));
            }
            if let Some(field) = want.pointer_mut(&format!("/output/uids/{i}")) {
                *field = Value::String(text.to_owned());
            }
        }
    }
    // “$uid” in `added` (cad.sensors.ingest, docs/adr/0646 §4): each the persistent id of an object the drawing now
    // holds, in the order written (the entities' expectation checks what they are).
    let added = want
        .pointer("/output/added")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    for i in 0..added {
        if want.pointer(&format!("/output/added/{i}")) != Some(&json!("$uid")) {
            continue;
        }
        let text = got
            .pointer(&format!("/output/added/{i}"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let held = Uuid::parse_str(text)
            .ok()
            .and_then(|u| doc.slot_of(u))
            .is_some();
        if !held {
            return Err(format!(
                "{at}: output.added[{i}] {text} çizimdeki bir nesnenin kimliği değil"
            ));
        }
        if let Some(field) = want.pointer_mut(&format!("/output/added/{i}")) {
            *field = Value::String(text.to_owned());
        }
    }
    // “$layer:K”: the layers the command added (execute) or would add (plan), ids the tree did not have, by their number
    // (docs/adr/0210 §11); a command that adds none leaves the last ones.
    let fresh = fresh_layers(&got, &before);
    if !fresh.is_empty() {
        let after = layer_ids(doc.layers().nodes());
        if let Some(id) = fresh
            .iter()
            .find(|id| after.contains(*id) != (op == "execute"))
        {
            return Err(format!(
                "{at}: “{id}” yeni katman ağaçta {}",
                if op == "execute" { "yok" } else { "var" }
            ));
        }
        state.fresh = fresh;
    }
    // “$layer”: the layer the command added (execute) or would add (plan): an id the tree did not have (docs/adr/0208 §15).
    for pointer in ["/output/layer", "/output/node/id"] {
        if want.pointer(pointer) != Some(&json!("$layer")) {
            continue;
        }
        let id = got.pointer(pointer).and_then(Value::as_str).unwrap_or("");
        let after = layer_ids(doc.layers().nodes());
        let fresh = id
            .strip_prefix("layer-")
            .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
            && !before.contains(id)
            && after.contains(id) == (op == "execute");
        if !fresh {
            return Err(format!(
                "{at}: {pointer}: “{id}” yeni bir katmanın kimliği değil"
            ));
        }
        state.layer = Some(id.to_owned());
    }
    let want = fill(&want, doc, state, at)?;
    if let Some(within) = step.get("within").and_then(Value::as_f64) {
        let relative = step.get("relative").and_then(Value::as_bool).unwrap_or(false);
        return if close_by(&got, &want, within, relative) {
            Ok(())
        } else {
            Err(format!(
                "{at}: sonuç (±{within}): beklenen {want}, bulunan {got}"
            ))
        };
    }
    expect_same(&got, &want, "sonuç", at)
}

/// The `layer-N` ids in a result the tree did not have before, by N.
fn fresh_layers(got: &Value, before: &std::collections::HashSet<String>) -> Vec<String> {
    fn walk(v: &Value, before: &std::collections::HashSet<String>, out: &mut Vec<(u64, String)>) {
        match v {
            Value::String(s) => {
                if let Some(n) = s.strip_prefix("layer-").and_then(|n| n.parse::<u64>().ok())
                    && !before.contains(s)
                    && !out.iter().any(|(_, o)| o == s)
                {
                    out.push((n, s.clone()));
                }
            }
            Value::Array(items) => items.iter().for_each(|i| walk(i, before, out)),
            Value::Object(fields) => fields.values().for_each(|f| walk(f, before, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    walk(got, before, &mut out);
    out.sort_by_key(|(n, _)| *n);
    out.into_iter().map(|(_, s)| s).collect()
}

/// The node of `id` in a layer tree and the id of the group it is in.
fn node_with_parent<'a>(
    nodes: &'a [kentos_commands::LayerNode],
    id: &str,
    parent: Option<&'a str>,
) -> Option<(&'a kentos_commands::LayerNode, Option<&'a str>)> {
    for n in nodes {
        if n.id == id {
            return Some((n, parent));
        }
        if let Some(found) = node_with_parent(&n.children, id, Some(&n.id)) {
            return Some(found);
        }
    }
    None
}

/// What a step that writes nothing leaves as it was (docs/adr/0741 §1, plan/gis.md CBS-41): a plan or a validation, a
/// query's execute and a refused one change neither the drawing's revision nor its dirty flag, its undo and redo (the
/// calls' journal may note them: it is the record, not the drawing).
#[derive(Debug, PartialEq)]
struct Held {
    revision: u64,
    dirty: bool,
    undo: bool,
    redo: bool,
}

impl Held {
    fn of(doc: &Document) -> Self {
        Self {
            revision: doc.revision(),
            dirty: doc.is_dirty(),
            undo: doc.can_undo(),
            redo: doc.can_redo(),
        }
    }

    /// Why the step must write nothing: its op, the command's effect or its refusal; none when it may write.
    fn why(step: &Value, name: &str) -> Option<&'static str> {
        static QUERIES: std::sync::OnceLock<std::collections::HashSet<String>> =
            std::sync::OnceLock::new();
        let queries = QUERIES.get_or_init(|| {
            kentos_commands::catalog()
                .commands
                .into_iter()
                .filter(|c| c.effect == kentos_commands::CommandEffect::Query)
                .map(|c| c.id)
                .collect()
        });
        match step["op"].as_str() {
            Some("validate") => Some("doğrulama"),
            Some("plan") => Some("plan"),
            Some("execute") => match step["result"]["status"].as_str() {
                Some("failed" | "conflict" | "needsInput") => Some("ret"),
                _ if queries.contains(name) => Some("sorgu"),
                _ => None,
            },
            _ => None,
        }
    }

    fn same(&self, doc: &Document, why: &str, at: &str) -> Outcome<()> {
        let now = Self::of(doc);
        if now == *self {
            Ok(())
        } else {
            Err(format!(
                "{at}: {why} çizimi değiştirdi: önce {self:?}, sonra {now:?}"
            ))
        }
    }
}

/// Every node's id of a layer tree.
fn layer_ids(nodes: &[kentos_commands::LayerNode]) -> std::collections::HashSet<String> {
    let mut out = std::collections::HashSet::new();
    let mut stack: Vec<&kentos_commands::LayerNode> = nodes.iter().collect();
    while let Some(n) = stack.pop() {
        out.insert(n.id.clone());
        stack.extend(n.children.iter());
    }
    out
}

fn run_step(
    command: &str,
    doc: &mut Document,
    state: &mut State,
    step: &Value,
    at: &str,
) -> Outcome<()> {
    if let Some(fields) = step.as_object() {
        // A misspelt field would otherwise pass unchecked.
        if let Some(key) = fields.keys().find(|k| !STEP_KEYS.contains(&k.as_str())) {
            return Err(format!("{at}: bilinmeyen alan “{key}”"));
        }
    }
    let before = doc.revision();
    let held = Held::of(doc);
    match step["op"].as_str().unwrap_or("?") {
        // A step may run another command first (a dimension tied before it is broken, docs/adr/0296).
        // A write that relaxes the constraints (docs/adr/0337 §1).
        "validate" | "plan" | "execute" if step["relaxConstraints"] == true => {
            doc.set_relax(kentos_domain::Relax::Next);
            let ran = run_command(
                step["command"].as_str().unwrap_or(command),
                doc,
                state,
                step,
                at,
            );
            doc.set_relax(kentos_domain::Relax::Off);
            ran?;
            if let Some(why) = Held::why(step, step["command"].as_str().unwrap_or(command)) {
                held.same(doc, why, at)?;
            }
        }
        "validate" | "plan" | "execute" => {
            let name = step["command"].as_str().unwrap_or(command);
            run_command(name, doc, state, step, at)?;
            if let Some(why) = Held::why(step, name) {
                held.same(doc, why, at)?;
            }
            // A write is the command's own row in the journal (docs/adr/0253 §3, 0548 §4): its name and input, so
            // the history says it and a replay runs it again; never an anonymous edit.
            if step["op"] == "execute" && doc.revision() != before {
                let last = doc.journal().last();
                if !last.is_some_and(|r| {
                    r.kind == kentos_commands::CallKind::Command
                        && r.command.as_deref() == Some(name)
                }) {
                    return Err(format!(
                        "{at}: {name} yazdı ama kayıtta komut olarak yok (son satır {:?} {:?})",
                        last.map(|r| r.kind),
                        last.and_then(|r| r.command.clone())
                    ));
                }
            }
        }
        // The project saved as a .kcad file and opened again (docs/adr/1260 §5): what follows runs on what the file
        // keeps, the undo chain gone.
        "reopen" => {
            let bytes = kentos_kcad::encode(&doc.to_snapshot_v2())
                .map_err(|e| format!("{at}: .kcad yazılamadı: {e}"))?;
            let snapshot =
                kentos_kcad::decode(&bytes).map_err(|e| format!("{at}: .kcad okunamadı: {e}"))?;
            *doc = Document::from_snapshot_v2(snapshot).map_err(|e| format!("{at}: {e}"))?;
        }
        op @ ("undo" | "redo") => {
            let label = if op == "undo" { doc.undo() } else { doc.redo() };
            if let Some(want) = step.get("returns") {
                expect_same(&json!(label), want, "dönen değer", at)?;
            }
        }
        "captureRevision" => {
            let name = step["as"].as_str().ok_or(format!("{at}: “as” yok"))?;
            state
                .revisions
                .insert(name.to_owned(), doc.revision().to_string());
        }
        "captureBlock" => {
            let name = step["as"].as_str().ok_or(format!("{at}: “as” yok"))?;
            let block = step["name"].as_str().ok_or(format!("{at}: “name” yok"))?;
            let id = doc
                .block_named(block)
                .map(|b| b.id)
                .ok_or(format!("{at}: “{block}” bloğu yok"))?;
            state.blocks.insert(name.to_owned(), id);
        }
        "captureUid" => {
            let name = step["as"].as_str().ok_or(format!("{at}: “as” yok"))?;
            let id = step["id"].as_u64().and_then(|n| u32::try_from(n).ok());
            let uid = id
                .and_then(|id| doc.uid(Slot(id)))
                .ok_or(format!("{at}: {id:?} nesnesi yok"))?;
            state.uids.insert(name.to_owned(), uid);
        }
        // A kitchen run's id, by its place in the drawing's runs (docs/adr/0727 §6): `$uid:name` fills it.
        "captureCasework" => {
            let name = step["as"].as_str().ok_or(format!("{at}: “as” yok"))?;
            let index = step["index"].as_u64().unwrap_or(0) as usize;
            let run = doc
                .casework()
                .get(index)
                .ok_or(format!("{at}: {index}. mutfak dizisi yok"))?;
            state.uids.insert(name.to_owned(), Uuid::from_bytes(run.id.0));
        }
        // A file changed by someone else between two steps (another program's edit of a bound table, docs/adr/0538): a
        // fixture's bytes over a `$tmp:` file.
        "copyFile" => {
            let from = with_fixtures(step["from"].as_str().ok_or(format!("{at}: “from” yok"))?);
            let to = step["to"].as_str().ok_or(format!("{at}: “to” yok"))?;
            if !to.starts_with("$tmp:") {
                return Err(format!("{at}: “to” bir $tmp: dosyası olmalı"));
            }
            let to = with_fixtures(to);
            // A file in a folder the case names (the project's `profiller`, docs/adr/0558): the folder is made.
            if let Some(folder) = std::path::Path::new(&to).parent() {
                std::fs::create_dir_all(folder)
                    .map_err(|e| format!("{at}: {}: {e}", folder.display()))?;
            }
            std::fs::copy(&from, &to).map_err(|e| format!("{at}: {from}: {e}"))?;
        }
        // A file moved, deleted or put back between two steps (a source that is not where its link says, docs/adr/0740):
        // a `$tmp:` file renamed to another `$tmp:` path, its time and bytes kept.
        "moveFile" => {
            let from = step["from"].as_str().ok_or(format!("{at}: “from” yok"))?;
            let to = step["to"].as_str().ok_or(format!("{at}: “to” yok"))?;
            if !from.starts_with("$tmp:") || !to.starts_with("$tmp:") {
                return Err(format!("{at}: “from” ve “to” $tmp: dosyaları olmalı"));
            }
            let (from, to) = (with_fixtures(from), with_fixtures(to));
            if let Some(folder) = std::path::Path::new(&to).parent() {
                std::fs::create_dir_all(folder)
                    .map_err(|e| format!("{at}: {}: {e}", folder.display()))?;
            }
            std::fs::rename(&from, &to).map_err(|e| format!("{at}: {from}: {e}"))?;
        }
        other => return Err(format!("{at}: bilinmeyen işlem “{other}”")),
    }
    check(
        doc,
        state,
        step.get("expect"),
        before,
        at,
        step.get("within").and_then(Value::as_f64),
    )
}

/// An object's rings summed up whatever their starting corner: `corners` every corner of every ring (outer rings,
/// holes, parts) sorted, `area` the shoelace area of the outer rings less the holes', `parts` and `holes` counted.
fn shape_summary(entity: &Value) -> Value {
    fn ring(v: &Value) -> Vec<(f64, f64)> {
        v.as_array()
            .map(|pts| {
                pts.iter()
                    .filter_map(|p| Some((p["x"].as_f64()?, p["y"].as_f64()?)))
                    .collect()
            })
            .unwrap_or_default()
    }
    fn area(r: &[(f64, f64)]) -> f64 {
        let n = r.len();
        let (ox, oy) = r.first().copied().unwrap_or((0.0, 0.0));
        (0..n)
            .map(|i| {
                let (a, b) = (r[i], r[(i + 1) % n]);
                (a.0 - ox) * (b.1 - oy) - (b.0 - ox) * (a.1 - oy)
            })
            .sum::<f64>()
            .abs()
            / 2.0
    }
    let mut outers = vec![ring(&entity["pts"])];
    let mut holes: Vec<Vec<(f64, f64)>> = entity["holes"]
        .as_array()
        .map(|h| h.iter().map(|h| ring(&h["pts"])).collect())
        .unwrap_or_default();
    for part in entity["parts"].as_array().into_iter().flatten() {
        outers.push(ring(&part["pts"]));
        holes.extend(
            part["holes"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|h| ring(&h["pts"])),
        );
    }
    let net: f64 =
        outers.iter().map(|r| area(r)).sum::<f64>() - holes.iter().map(|r| area(r)).sum::<f64>();
    let mut corners: Vec<(f64, f64)> = outers.iter().chain(&holes).flatten().copied().collect();
    corners.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    json!({
        "corners": corners.iter().map(|(x, y)| json!([x, y])).collect::<Vec<_>>(),
        "area": (net * 1e6).round() / 1e6,
        "parts": outers.len(),
        "holes": holes.len(),
    })
}

/// The document after a step as `expect` says; with `within`, the objects' numbers within it (a transcendental
/// value the reference computes in another order, docs/adr/0320).
fn check(
    doc: &Document,
    state: &State,
    expect: Option<&Value>,
    before: u64,
    at: &str,
    within: Option<f64>,
) -> Outcome<()> {
    let Some(expect) = expect else {
        return Ok(());
    };
    let expect = expect
        .as_object()
        .ok_or(format!("{at}: expect bir nesne olmalı"))?;
    let empty = serde_json::Map::new();
    for (key, want) in expect {
        match key.as_str() {
            "ids" => {
                let ids: Vec<u32> = doc.entities().map(|e| e.base().id).collect();
                expect_same(&json!(ids), want, "nesneler", at)?;
            }
            "entities" => {
                for (id, entity) in want.as_object().unwrap_or(&empty) {
                    let slot = id
                        .parse::<u32>()
                        .map(Slot)
                        .map_err(|_| format!("{at}: {id}"))?;
                    let got = doc
                        .get(slot)
                        .map(serde_json::to_value)
                        .transpose()
                        .map_err(|e| format!("{at}: {e}"))?
                        .unwrap_or(Value::Null);
                    let entity = block_ids(entity, doc, state, at)?;
                    match within {
                        Some(w) if !close(&got, &entity, w) => {
                            return Err(format!(
                                "{at}: nesne {id} (±{w}): beklenen {entity}, bulunan {got}"
                            ));
                        }
                        Some(_) => {}
                        None => expect_same(&got, &entity, &format!("nesne {id}"), at)?,
                    }
                }
            }
            // Only the fields named of each object, each whole (docs/adr/0257 §3: a reader's
            // object holds more than a case should know of it).
            "fields" => {
                for (id, fields) in want.as_object().unwrap_or(&empty) {
                    let slot = id
                        .parse::<u32>()
                        .map(Slot)
                        .map_err(|_| format!("{at}: {id}"))?;
                    let got = doc
                        .get(slot)
                        .map(serde_json::to_value)
                        .transpose()
                        .map_err(|e| format!("{at}: {e}"))?
                        .ok_or_else(|| format!("{at}: {id} yuvasında nesne yok"))?;
                    for (k, v) in fields.as_object().unwrap_or(&empty) {
                        let v = block_ids(v, doc, state, at)?;
                        let have = got.get(k).unwrap_or(&Value::Null);
                        match within {
                            // Coordinates a reference moved by PROJ (docs/adr/0563 §1): within the step's `within`.
                            Some(w) if !close(have, &v, w) => {
                                return Err(format!(
                                    "{at}: nesne {id}.{k} (±{w}): beklenen {v}, bulunan {have}"
                                ));
                            }
                            Some(_) => {}
                            None => expect_same(have, &v, &format!("nesne {id}.{k}"), at)?,
                        }
                    }
                }
            }
            // Only the fields named of each layer-tree node, by id; `parent` is the
            // id of the group it is in, null at the top (docs/adr/0257 §3).
            "nodes" => {
                for (id, fields) in want.as_object().unwrap_or(&empty) {
                    let (node, parent) = node_with_parent(doc.layers().nodes(), id, None)
                        .ok_or_else(|| format!("{at}: “{id}” düğümü ağaçta yok"))?;
                    let got = serde_json::to_value(node).map_err(|e| format!("{at}: {e}"))?;
                    for (k, v) in fields.as_object().unwrap_or(&empty) {
                        let got = if k == "parent" {
                            parent.map_or(Value::Null, |p| json!(p))
                        } else {
                            got.get(k).cloned().unwrap_or(Value::Null)
                        };
                        expect_same(&got, v, &format!("düğüm {id}.{k}"), at)?;
                    }
                }
            }
            "canUndo" => expect_same(&json!(doc.can_undo()), want, "canUndo", at)?,
            // The drawing's constraints' kinds in order and its parameters (docs/adr/0337).
            "constraintKinds" => {
                let kinds: Vec<Value> = doc
                    .constraints()
                    .constraints
                    .iter()
                    .map(|c| {
                        serde_json::to_value(&c.rule).map_or(Value::Null, |r| r["kind"].clone())
                    })
                    .collect();
                expect_same(&json!(kinds), want, "kısıtların türleri", at)?;
            }
            "parameters" => {
                let got =
                    serde_json::to_value(&doc.constraints().parameters).unwrap_or(Value::Null);
                expect_same(&got, want, "parametreler", at)?;
            }
            "canRedo" => expect_same(&json!(doc.can_redo()), want, "canRedo", at)?,
            "dirty" => expect_same(&json!(doc.is_dirty()), want, "dirty", at)?,
            "revision" => {
                let got = if doc.revision() == before {
                    "same"
                } else {
                    "changed"
                };
                expect_same(&json!(got), want, "sürüm", at)?;
            }
            "uids" => {
                for (id, name) in want.as_object().unwrap_or(&empty) {
                    let slot = id
                        .parse::<u32>()
                        .map(Slot)
                        .map_err(|_| format!("{at}: {id}"))?;
                    let uid = doc
                        .uid(slot)
                        .ok_or(format!("{at}: {id} nesnesinin kalıcı kimliği yok"))?;
                    match name.as_str() {
                        Some("new") if state.uids.values().any(|u| *u == uid) => {
                            return Err(format!("{at}: {id} nesnesinin kimliği yeni değil"));
                        }
                        Some("new") => {}
                        Some(name) if state.uids.get(name) == Some(&uid) => {}
                        Some(name) => {
                            return Err(format!(
                                "{at}: {id} nesnesinin kimliği {uid}, “{name}” {:?}",
                                state.uids.get(name)
                            ));
                        }
                        None => return Err(format!("{at}: kimlik adı metin olmalı")),
                    }
                }
            }
            // The drawing's definitions in order, each without its id (docs/adr/0144).
            "blocks" => {
                let got: Vec<Value> = doc
                    .blocks()
                    .iter()
                    .map(|b| {
                        let mut v = serde_json::to_value(&**b).map_err(|e| format!("{at}: {e}"))?;
                        if let Some(fields) = v.as_object_mut() {
                            fields.remove("id");
                        }
                        Ok(v)
                    })
                    .collect::<Outcome<_>>()?;
                // A block's insert names a block by `$blockOf:Ad` (a block whose id the case cannot know).
                let wanted = block_ids(want, doc, state, at)?;
                expect_same(&json!(got), &wanted, "bloklar", at)?;
            }
            // The drawing's associative arrays in order, each without its id (docs/adr/0300); the expected
            // block and items by `$blockOf:Ad` and `$uidOf:12`.
            "dimensionSets" => {
                let got = serde_json::to_value(
                    doc.dimension_sets()
                        .iter()
                        .map(|s| &**s)
                        .collect::<Vec<_>>(),
                )
                .map_err(|e| format!("{at}: {e}"))?;
                let wanted = fill(want, doc, state, at)?;
                match within {
                    // A recipe turned with its objects (docs/adr/0730 §4): its places within the step's `within`.
                    Some(w) if !close(&got, &wanted, w) => {
                        return Err(format!("{at}: duvar ölçü grupları (±{w}): beklenen {wanted}, bulunan {got}"));
                    }
                    Some(_) => {}
                    None => expect_same(&got, &wanted, "duvar ölçü grupları", at)?,
                }
            }
            "arrays" => {
                let got: Vec<Value> = doc
                    .arrays()
                    .iter()
                    .map(|a| {
                        let mut v = serde_json::to_value(&**a).map_err(|e| format!("{at}: {e}"))?;
                        if let Some(fields) = v.as_object_mut() {
                            fields.remove("id");
                        }
                        Ok(v)
                    })
                    .collect::<Outcome<_>>()?;
                let wanted = block_ids(want, doc, state, at)?;
                expect_same(&json!(got), &wanted, "ilişkili diziler", at)?;
            }
            // The drawing's kitchen runs in order, each without its id (a UUIDv7); its objects and walls by `$uidOf:N`
            // (docs/adr/0727 §5).
            "casework" => {
                let got: Vec<Value> = doc
                    .casework()
                    .iter()
                    .map(|r| {
                        let mut v = serde_json::to_value(&**r).map_err(|e| format!("{at}: {e}"))?;
                        if let Some(fields) = v.as_object_mut() {
                            fields.remove("id");
                        }
                        Ok(v)
                    })
                    .collect::<Outcome<_>>()?;
                let got = json!(got);
                let wanted = fill(want, doc, state, at)?;
                match within {
                    Some(w) if !close(&got, &wanted, w) => {
                        return Err(format!("{at}: mutfak dizileri (±{w}): beklenen {wanted}, bulunan {got}"));
                    }
                    Some(_) => {}
                    None => expect_same(&got, &wanted, "mutfak dizileri", at)?,
                }
            }
            // An external reference's source by its name, without its objects' ids (docs/adr/0351 §2: a PDF's
            // objects are numbered from their content).
            "xrefs" => {
                for (name, want) in want.as_object().unwrap_or(&empty) {
                    let got = doc
                        .block_named(name)
                        .and_then(|b| b.xref.as_ref())
                        .map(serde_json::to_value)
                        .transpose()
                        .map_err(|e| format!("{at}: {e}"))?
                        .map(|mut v| {
                            if let Some(m) = v.as_object_mut() {
                                m.remove("objects");
                            }
                            v
                        })
                        .unwrap_or(Value::Null);
                    let want = block_ids(want, doc, state, at)?;
                    expect_same(&got, &want, &format!("“{name}” dış başvurusu"), at)?;
                }
            }
            // A definition's id by its name: "new" (none the setup had nor
            // `captureBlock` took), "$block:name" or the id itself.
            "blockIds" => {
                for (name, id) in want.as_object().unwrap_or(&empty) {
                    let got = doc
                        .block_named(name)
                        .map(|b| b.id)
                        .ok_or(format!("{at}: “{name}” bloğu yok"))?;
                    match id.as_str() {
                        Some("new")
                            if state.setup_blocks.contains(&got)
                                || state.blocks.values().any(|b| *b == got) =>
                        {
                            return Err(format!("{at}: “{name}” bloğunun kimliği yeni değil"));
                        }
                        Some("new") => {}
                        Some(text) => {
                            let wanted = fill(&json!(text), doc, state, at)?;
                            expect_same(
                                &json!(got.to_text()),
                                &wanted,
                                &format!("“{name}” bloğunun kimliği"),
                                at,
                            )?;
                        }
                        None => return Err(format!("{at}: blok kimliği metin olmalı")),
                    }
                }
            }
            // Only the fields named of the drawing's block of each name (docs/adr/0726 §2): `$inserts` the names of the
            // blocks its inserts place, in order; `$count` how many objects it holds.
            "blockFields" => {
                for (name, fields) in want.as_object().unwrap_or(&empty) {
                    let b = doc
                        .block_named(name)
                        .ok_or(format!("{at}: “{name}” bloğu yok"))?;
                    let got = serde_json::to_value(b).map_err(|e| format!("{at}: {e}"))?;
                    for (k, v) in fields.as_object().unwrap_or(&empty) {
                        let have = match k.as_str() {
                            "$inserts" => json!(
                                b.entities
                                    .iter()
                                    .filter_map(|e| match e {
                                        kentos_commands::Entity::Insert(i) =>
                                            doc.block(i.block).map(|d| d.name.clone()),
                                        _ => None,
                                    })
                                    .collect::<Vec<_>>()
                            ),
                            "$count" => json!(b.entities.len()),
                            _ => got.get(k).cloned().unwrap_or(Value::Null),
                        };
                        let what = format!("“{name}” bloğunun {k} alanı");
                        match within {
                            Some(w) if !close(&have, v, w) => {
                                return Err(format!("{at}: {what} (±{w}): beklenen {v}, bulunan {have}"));
                            }
                            Some(_) => {}
                            None => expect_same(&have, v, &what, at)?,
                        }
                    }
                }
            }
            // The layer tree, every node whole; `$layer` the one a command added (docs/adr/0208 §15).
            "layers" => {
                let got =
                    serde_json::to_value(doc.layers().nodes()).map_err(|e| format!("{at}: {e}"))?;
                expect_same(&got, &fill(want, doc, state, at)?, "katmanlar", at)?;
            }
            // The project's connections (docs/adr/0208 §2).
            "connections" => {
                let got = serde_json::to_value(&doc.settings().connections)
                    .map_err(|e| format!("{at}: {e}"))?;
                expect_same(&got, want, "bağlantılar", at)?;
            }
            // The project's networks (docs/adr/0209 §2).
            "networks" => {
                let got = serde_json::to_value(&doc.settings().networks)
                    .map_err(|e| format!("{at}: {e}"))?;
                expect_same(&got, want, "ağlar", at)?;
            }
            // The active layer and the drawing's name (docs/adr/0248 §2, §5).
            "activeLayer" => expect_same(
                &json!(doc.layers().active()),
                &fill(want, doc, state, at)?,
                "etkin katman",
                at,
            )?,
            "name" => expect_same(&json!(doc.name()), want, "çizimin adı", at)?,
            // The sheet book's sheets as the command lists them (docs/adr/0252 §2).
            "sheets" => {
                let got =
                    serde_json::to_value(kentos_native_application::sheets_edit::sheets_in(doc))
                        .map_err(|e| format!("{at}: {e}"))?;
                expect_same(&got, want, "paftalar", at)?;
            }
            // The book's plot profiles, and the profile each sheet's plot settings name (docs/adr/1130).
            "plotProfiles" | "sheetProfiles" => {
                let book = match doc.sheets() {
                    Some(s) => kentos_sheet::project::read_project_sheets(s)
                        .map_err(|e| format!("{at}: kitap okunamadı: {}", e.message))?
                        .0,
                    None => Default::default(),
                };
                let got = if key == "plotProfiles" {
                    serde_json::to_value(&book.plot_profiles).map_err(|e| format!("{at}: {e}"))?
                } else {
                    Value::Object(
                        book.sheets
                            .iter()
                            .map(|s| {
                                (
                                    s.id.clone(),
                                    json!(s.plot.as_ref().and_then(|p| p.settings.profile.clone())),
                                )
                            })
                            .collect(),
                    )
                };
                expect_same(&got, want, key, at)?;
            }
            // The project's text and dimension styles without their ids: a new one's is a UUIDv7 (docs/adr/0312 §3).
            "styleNames" => {
                let without_ids = |v: Value| match v {
                    Value::Array(list) => Value::Array(
                        list.into_iter()
                            .map(|mut x| {
                                if let Some(o) = x.as_object_mut() {
                                    o.remove("id");
                                }
                                x
                            })
                            .collect(),
                    ),
                    other => other,
                };
                let settings = doc.settings();
                let got = json!({
                    "text": without_ids(serde_json::to_value(&settings.text_styles).map_err(|e| format!("{at}: {e}"))?),
                    "dimension": without_ids(serde_json::to_value(&settings.dimension_styles).map_err(|e| format!("{at}: {e}"))?),
                });
                expect_same(&got, want, "stiller", at)?;
            }
            // The project's settings, only the keys given; an absent one is null (docs/adr/0248).
            "settings" => {
                let got = serde_json::to_value(doc.settings()).map_err(|e| format!("{at}: {e}"))?;
                for (k, v) in want.as_object().unwrap_or(&empty) {
                    expect_same(
                        got.get(k).unwrap_or(&Value::Null),
                        &fill(&setup_paths(v), doc, state, at)?,
                        &format!("ayar {k}"),
                        at,
                    )?;
                }
            }
            // The project's style library, each item's id, kind, name and path (docs/adr/0314 §3).
            "library" => {
                let got = json!(kentos_native_application::library_edit::listed(doc));
                expect_same(&got, want, "kitaplık", at)?;
            }
            // Where the view opens and the drawing's anchor (docs/adr/0256 §1).
            "homeView" => {
                let got =
                    serde_json::to_value(doc.home_view()).map_err(|e| format!("{at}: {e}"))?;
                expect_same(&got, want, "ana görünüm", at)?;
            }
            "origin" => {
                let got = serde_json::to_value(doc.origin()).map_err(|e| format!("{at}: {e}"))?;
                expect_same(&got, want, "çapa", at)?;
            }
            // The project's address data (docs/adr/0229 §2).
            "addressLocators" => {
                let got = serde_json::to_value(&doc.settings().address_locators)
                    .map_err(|e| format!("{at}: {e}"))?;
                expect_same(&got, &fill(want, doc, state, at)?, "adres verileri", at)?;
            }
            // A road project's terrain surfaces' definitions (plan/road.md Y2.3); null: none.
            "road" => {
                let mut got = serde_json::to_value(&doc.settings().road)
                    .map_err(|e| format!("{at}: {e}"))?;
                let wanted = fill(want, doc, state, at)?;
                // What the road commands drew (docs/adr/0911 §1) is compared only where a case names it.
                for key in ["drawings", "autoRebuild", "rebuildWhole"] {
                    if wanted.get(key).is_none()
                        && let Some(o) = got.as_object_mut()
                    {
                        o.remove(key);
                    }
                }
                if got.as_object().is_some_and(serde_json::Map::is_empty) {
                    got = Value::Null;
                }
                match within {
                    // An alignment fitted to a line (docs/adr/0908 §12): its PIs within the step's `within`.
                    Some(w) if !close(&got, &wanted, w) => {
                        return Err(format!("{at}: yol ayarları (±{w}): beklenen {wanted}, bulunan {got}"));
                    }
                    Some(_) => {}
                    None => expect_same(&got, &wanted, "yol ayarları", at)?,
                }
            }
            // The project's links of event layers to their routes (docs/adr/0569 §2).
            "linearEvents" => {
                let got = serde_json::to_value(&doc.settings().linear_events)
                    .map_err(|e| format!("{at}: {e}"))?;
                expect_same(&got, &fill(want, doc, state, at)?, "olay bağları", at)?;
            }
            // An area's shape whatever corner its rings start from (Çakışma and Topoloji's results, docs/adr/0622 §9):
            // every corner of every ring sorted, the net area, the parts and holes; its edges straight.
            "shapes" => {
                for (id, want) in want.as_object().unwrap_or(&empty) {
                    let slot = id
                        .parse::<u32>()
                        .map(Slot)
                        .map_err(|_| format!("{at}: {id}"))?;
                    let got = doc
                        .get(slot)
                        .map(serde_json::to_value)
                        .transpose()
                        .map_err(|e| format!("{at}: {e}"))?
                        .ok_or_else(|| format!("{at}: {id} yuvasında nesne yok"))?;
                    expect_same(
                        &shape_summary(&got),
                        want,
                        &format!("nesne {id} biçimi"),
                        at,
                    )?;
                }
            }
            // A misspelt expectation would otherwise pass unchecked.
            other => return Err(format!("{at}: bilinmeyen beklenti “{other}”")),
        }
    }
    Ok(())
}

/// `value` with every `$fixture:` path in its texts made absolute (a bound layer's file, docs/adr/0503).
fn setup_paths(value: &Value) -> Value {
    match value {
        Value::String(text) if (text.contains("$fixture:") || text.contains("$tmp:")) => {
            Value::String(with_fixtures(text))
        }
        Value::Array(items) => Value::Array(items.iter().map(setup_paths).collect()),
        Value::Object(fields) => Value::Object(
            fields
                .iter()
                .map(|(k, v)| (k.clone(), setup_paths(v)))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// Reads `fixtures/geoid/v1`'s grids into the core under their SHA-256, once.
fn register_crs_models() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/geoid/v1");
        for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            let Ok(bytes) = std::fs::read(entry.path()) else {
                continue;
            };
            if let Ok(model) = kentos_native_application::crs_models::read(&bytes) {
                kentos_geometry_core::crs::geoid::register(
                    &kentos_native_application::crs_models::sha256(&bytes),
                    model.grid,
                );
            }
        }
    });
}

fn run_case(command: &str, setup: &Value, case: &Value, at: &str) -> Outcome<()> {
    // İşlemler's tools, as every host that has them registers them (docs/adr/0508).
    kentos_processing::command::register();
    // STAC searches are answered from the recorded folder (docs/adr/0528 §2): no network in a shared case.
    stac_host::register(Box::new(stac_host::Recorded::open(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/services/v1/stac"),
    )));
    // The geoid models and velocity grids of the shared cases (docs/adr/0631), as a device's library gives them.
    register_crs_models();
    let setup = &setup_paths(setup);
    // A drawing of the file's v2 form carries its objects' persistent ids (docs/adr/0576 §3): a revision file and the
    // open drawing then share them. Else the v1 form, whose ids are derived.
    let mut doc = if setup["version"] == 2 {
        let snapshot: kentos_commands::DocumentSnapshotV2 =
            serde_json::from_value(setup.clone()).map_err(|e| format!("{at}: kurulum: {e}"))?;
        Document::from_snapshot_v2(snapshot).map_err(|e| format!("{at}: kurulum: {e}"))?
    } else {
        let snapshot = DocumentSnapshotV1::from_json(&setup.to_string())
            .map_err(|e| format!("{at}: kurulum: {e}"))?;
        Document::from_snapshot(snapshot).map_err(|e| format!("{at}: kurulum: {e}"))?
    };
    if doc.is_dirty() || doc.can_undo() {
        return Err(format!("{at}: kurulumdan sonra belge temiz değil"));
    }
    let steps = case["steps"].as_array().ok_or(format!("{at}: adım yok"))?;
    if steps.is_empty() {
        return Err(format!("{at}: adım yok"));
    }
    let mut state = State {
        setup_blocks: doc.blocks().iter().map(|b| b.id).collect(),
        ..State::default()
    };
    for (i, step) in steps.iter().enumerate() {
        let op = step["op"].as_str().unwrap_or("?");
        run_step(
            command,
            &mut doc,
            &mut state,
            step,
            &format!("{at} › {} {op}", i + 1),
        )?;
    }
    Ok(())
}

/// Runs every file of `dir`: each a desktop command's, with at least `least`
/// cases. Answers the commands they were for, the cases run and what failed.
/// `KENTOS_CASES_ONLY=cad.walls,cad.nurbs` runs only the files whose names hold one of the words.
fn only_cases() -> Option<Vec<String>> {
    std::env::var("KENTOS_CASES_ONLY").ok().map(|v| {
        v.split(',')
            .map(str::to_owned)
            .filter(|w| !w.is_empty())
            .collect()
    })
}

fn run_folder(dir: &std::path::Path, least: usize) -> (Vec<String>, usize, Vec<String>) {
    let only = only_cases();
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|x| x == "json"))
        .filter(|path| {
            only.as_ref().is_none_or(|words| {
                let name = path.to_string_lossy();
                words.iter().any(|w| name.contains(w.as_str()))
            })
        })
        .collect();
    files.sort();
    // KENTOS_CASES_ONLY may name no file of this folder (scripts/dev/changed-fixtures.mjs names another's).
    assert!(
        !files.is_empty() || only.is_some(),
        "{}: no files",
        dir.display()
    );
    let mut problems = Vec::new();
    let mut cases = 0;
    let mut commands = Vec::new();
    for path in &files {
        let file = path
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        let fixture: Value =
            serde_json::from_str(&std::fs::read_to_string(path).expect("fixture file"))
                .expect("JSON");
        assert_eq!(fixture["format"], "kentos.command-cases", "{file}");
        assert_eq!(fixture["version"], 1, "{file}");
        let command = (
            fixture["command"].as_str().unwrap_or(""),
            fixture["commandVersion"].as_u64().unwrap_or(0),
        );
        if fixture["host"] == "headless-file" {
            assert!(
                kentos_commands::FILE_COMMANDS
                    .iter()
                    .any(|(id, v)| (*id, u64::from(*v)) == command),
                "{file}: a file fixture must name an actual file command"
            );
            assert!(fixture["cases"].as_array().unwrap().len() >= least);
            // These cases run in kentos-headless/tests/headless/sheet_publish.rs, with real filesystem effects.
            continue;
        }
        assert!(
            DESKTOP_COMMANDS
                .iter()
                .any(|(id, v)| (*id, u64::from(*v)) == command),
            "{file}: {command:?} is not a desktop command"
        );
        commands.push(command.0.to_owned());
        let listed = fixture["cases"].as_array().expect("cases");
        assert!(listed.len() >= least, "{file}: {} cases", listed.len());
        // A command of a discipline this build leaves out (docs/adr/8054 §11): its cases run in a build with it.
        if kentos_native_application::registry::left_out(command.0) {
            continue;
        }
        for case in listed {
            cases += 1;
            let name = case["name"].as_str().unwrap_or("?");
            let setup = case.get("setup").unwrap_or(&fixture["setup"]);
            if let Err(problem) = run_case(command.0, setup, case, &format!("{file} › {name}")) {
                // A case that steps through such a command too; a build with every module never says so.
                if !problem.contains(LEFT_OUT) {
                    problems.push(problem);
                }
            }
        }
    }
    (commands, cases, problems)
}

#[test]
fn every_command_case_matches_the_desktop_handlers() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/commands/v1");
    let (commands, cases, problems) = run_folder(&dir, 20);
    assert!(
        problems.is_empty(),
        "{} of {cases} cases failed:\n{}",
        problems.len(),
        problems.join("\n")
    );
    // Every command the desktop runs has its cases (all the files run).
    for (id, _) in DESKTOP_COMMANDS.iter().filter(|_| only_cases().is_none()) {
        assert!(
            commands.iter().any(|c| c == id),
            "{id}: no file in fixtures/commands/v1"
        );
    }
}

/// The desktop's own cases (fixtures/commands/v1/desktop, docs/adr/0207 §10):
/// point clouds, which only the desktop adds yet. The web's runner reads the
/// folder above only.
#[test]
fn the_desktops_own_cases_match_its_handlers() {
    let dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/commands/v1/desktop");
    let (_, cases, problems) = run_folder(&dir, 1);
    assert!(
        problems.is_empty(),
        "{} of {cases} cases failed:\n{}",
        problems.len(),
        problems.join("\n")
    );
}
