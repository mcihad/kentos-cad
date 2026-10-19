//! The shared product command cases (fixtures/commands/v1, docs/adr/0022,
//! 0027, 0029, 0032, 0037, 0047, 0057, 0066) run against the desktop's handlers over the native document. The web
//! runs the same files against its own handlers
//! (apps/web/src/product/fixtures.test.ts); the format is in
//! fixtures/commands/README.md.
//!
//! JSON carries no NaN or ±∞: a step's `nonFinite` puts them into the typed
//! input after it is read, at the paths the errors name.

use std::collections::HashMap;
use std::path::PathBuf;

use kentos_domain::contracts::ChangesetApply;
use kentos_domain::contracts::{
    AddressesDefine, BlockId, BlocksDefine, BlocksEdit, CAD_BLOCKS_DEFINE, CAD_BLOCKS_EDIT,
    CAD_ENTITIES_SET, DimensionsReassociate, EntitiesChamfer, EntitiesCreate, EntitiesFillet,
    AnnotationsScales, EntitiesOffset, EntitiesOrder, EntitiesSetProperties, EntitiesTrim, LabelsPin, LayersCompose,
    FieldsUsage, LayersEdit, LayersFilter, LayersFind, LayersInfo, LayersLabels, LayersContingents, LayersRules, LayersSubtypes, LayersTable, LayersRenderer, LayersService,
    LayersSet, LayersStates, LayersTime, LayersTranslate, LibraryEdit, LineTypesEdit, HatchPatternsEdit,
    NetworkDefine, NurbsCreate, OffsetBy, ScenariosEdit, SettingsSet, SheetsEdit, StandardsApply,
    StandardsCheck, StylesEdit, StylesImpact, UcsSet, UcsTo, ViewsEdit, ViewsOp,
};
use kentos_domain::contracts::{
    ArcCreate, ArrayLayout, CAD_ARC_CREATE, CAD_CIRCLE_CREATE, CAD_ENTITIES_ARRAY,
    CAD_ENTITIES_DELETE, CAD_ENTITIES_EDIT, CAD_ENTITIES_TRANSFORM, CAD_LINE_CREATE,
    CAD_POINT_CREATE, CAD_POLYGON_CREATE, CAD_POLYLINE_CREATE, CircleCreate, DocumentSnapshotV1,
    EntitiesArray, EntitiesDelete, EntitiesEdit, EntitiesTransform, EntityEdit, EntityGeometry,
    LineCreate, PointCreate, PolygonCreate, PolylineCreate, Transform,
};
use kentos_domain::contracts::{
    DimensionMeasure, DimensionsCreate, Division, EntitiesBoundary, EntitiesBreak, EntitiesDivide,
    EntitiesHatch, EntitiesJoin, EntitiesStretch, HatchRegion, ImportDxf, ImportGpkg, ImportPdf, MeasuredPoint,
    UnitsChange,
};
use kentos_domain::{Document, Slot, Uuid};
use kentos_native_application::standards::{apply as standards_apply, check as standards_check};
use kentos_native_application::fields_write;
use kentos_native_application::{processing_batch, processing_run, processing_tools};
use kentos_native_application::{stac_host, stac_prepare, stac_search};
use kentos_native_application::{arch_edit, columns_create, grids_create, ramps_create, stairs_create};
use kentos_native_application::{schedules_create, schedules_edit, tags_check, tags_create, tags_edit};
use kentos_native_application::{assets_check, images_edit, rasters_mosaic, rasters_georef, inserts_clip, sheets_set, spaces_finish, templates_apply, xrefs_edit, xrefs_list, viewmarks_create, viewmarks_edit, viewmarks_references};
use kentos_native_application::sources::{describe as sources_describe, edit as sources_edit, relate as sources_relate, sync as sources_sync};
use kentos_native_application::records_page;
use kentos_native_application::{
    DESKTOP_COMMANDS, ExecutionContext, address_define, annotations_scales, arc, array, blocks_define, blocks_edit,
    circle, curves_construct, delete, dimensions_edit, dimensions_query, dimensions_reassociate, fields_edit, hatches_edit, leaders_edit, tables_edit, fonts_report, texts_check, texts_edit,
    edit, entities_chamfer, entities_extend, entities_fillet, entities_order, entities_pick, entities_relate, crs_transform,
    entities_select, entities_trim, entities_vertex, labels_pin, layers_compose, layers_edit,
    layers_filter, fields_usage, layers_find, layers_info, layers_labels, layers_renderer, layers_service,
    layers_contingents, layers_rules, layers_subtypes, layers_table, layers_set, layers_states, layers_time, layers_translate, line, network_define, nurbs, offset, walls_create, walls_edit, walls_from_lines, walls_joins, openings_create, openings_edit, spaces_create, spaces_edit, spaces_quantities,
    point, points_construct, polygon, polygon_regular, polyline, rectangle_create, scenarios_edit,
    set, settings_set, sheets_edit, snap_find, styles_edit, transform, ucs_set, units_change,
    views_edit,
};
use kentos_native_application::{
    arrays_edit, changeset, constraints_edit, constraints_query, parameters_edit, create, curves_edit, dimensions_create, divide, drawing_audit,
    entities_boundary, entities_break, entities_cleanup, entities_copy, entities_explode,
    entities_hatch, entities_join, entities_lengthen, entities_match, entities_paste,
    entities_reverse, file_import, file_import_csv, file_import_gpkg, file_import_pdf, gis_derivation, sld, stretch,
};
use kentos_native_application::{
    block_params, blocks_freeze, blocks_insert, blocks_proxy, blocks_query, blocks_replace,
    file_import_block, hatch_patterns_edit, library_edit, line_types_edit, styles_impact,
};
use kentos_domain::contracts::{
    FormsAudit, FormsCompare, FormsEdit, FormsRecompute, FormsTest, FormsTransition, DomainsEdit, DomainsExport, FieldsMigrate, AttachmentsInspect, SchemaDescribe, FormsExport, FormsImport, FormsFill, FormsSubmit, FormsValidate, RecordsCreate, RecordsDuplicate, RelationsEdit,
};
use kentos_native_application::{
    forms_audit, forms_compare, forms_edit, forms_recompute, forms_test, forms_transition, domains_edit, domains_export, fields_migrate, attachments_inspect, schema_describe, forms_export, forms_import, forms_fill, forms_submit, forms_validate, records, records_duplicate, relations_edit,
};
use kentos_domain::contracts::{JoinsEdit, JoinsQuery};
use kentos_native_application::{joins_edit, joins_query};
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

const STEP_KEYS: &[&str] = &[
    "op",
    "command",
    "input",
    "nonFinite",
    "result",
    "within",
    "returns",
    "as",
    "id",
    "name",
    "expect",
    "note",
    "relaxConstraints",
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
    match (a, b) {
        (_, Value::String(w)) if w == "$any" => true,
        (Value::Number(x), Value::Number(y)) => match (x.as_f64(), y.as_f64()) {
            (Some(x), Some(y)) => (x - y).abs() <= within,
            _ => false,
        },
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(p, q)| close(p, q, within))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len()
                && x.iter()
                    .all(|(k, v)| y.get(k).is_some_and(|w| close(v, w, within)))
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
        Value::String(text) if (text.contains("$fixture:") || text.contains("$tmp:")) => Value::String(with_fixtures(text)),
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
/// made empty once.
fn tmp_folder() -> PathBuf {
    static FOLDER: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    FOLDER
        .get_or_init(|| {
            let dir = std::env::temp_dir().join(format!("kentos-cases-{}", std::process::id()));
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
fn point_number<'a>(
    pts: &'a mut [kentos_domain::contracts::Vec2],
    rest: &str,
) -> Option<&'a mut f64> {
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
impl Input for kentos_domain::contracts::ConstraintsEdit {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_domain::contracts::ParametersEdit {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_domain::contracts::ConstraintsQuery {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

// The forms' inputs have no number a case makes NaN (docs/adr/0401 §9).
impl Input for FormsEdit {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for FormsValidate {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for FormsSubmit {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for FormsFill {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for FormsAudit {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for LayersContingents {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for LayersRules {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for LayersSubtypes {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for FormsRecompute {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for FormsTransition {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for DomainsEdit {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for DomainsExport {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for FieldsMigrate {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for AttachmentsInspect {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for SchemaDescribe {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for FormsExport {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for FormsImport {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for FormsCompare {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for FormsTest {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for RecordsDuplicate {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for RecordsCreate {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for RelationsEdit {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for JoinsEdit {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for JoinsQuery {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for PolygonCreate {
    /// `pts[1].y`, `bulges[3]`, `holes[0].pts[2].x`, `holes[1].bulges[0]`,
    /// `lineWeight` (which the step's input must give).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        if path == "lineWeight" {
            return self.line_weight.as_mut();
        }
        let (pts, bulges, rest) = match path.strip_prefix("holes[") {
            Some(rest) => {
                let (h, rest) = rest.split_once("].")?;
                let ring = self.holes.as_mut()?.get_mut(h.parse::<usize>().ok()?)?;
                (&mut ring.pts, &mut ring.bulges, rest)
            }
            None => (&mut self.pts, &mut self.bulges, path),
        };
        if rest.starts_with("pts[") {
            point_number(pts, rest)
        } else {
            bulge_number(bulges, rest)
        }
    }
}

impl Input for LineCreate {
    /// `a.x`, `b.y`, `lineWeight` (which the step's input must give).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        if path == "lineWeight" {
            return self.line_weight.as_mut();
        }
        let (end, axis) = path.split_once('.')?;
        let p = match end {
            "a" => &mut self.a,
            "b" => &mut self.b,
            _ => return None,
        };
        match axis {
            "x" => Some(&mut p.x),
            "y" => Some(&mut p.y),
            _ => None,
        }
    }
}

impl Input for PolylineCreate {
    /// `pts[1].y`, `bulges[0]`, `widths[1][0]` (docs/adr/0258), `lineWeight` (which the step's input must give).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        if path == "lineWeight" {
            self.line_weight.as_mut()
        } else if let Some(rest) = path.strip_prefix("widths[") {
            let (i, rest) = rest.split_once("][")?;
            let k: usize = rest.strip_suffix(']')?.parse().ok()?;
            self.widths
                .as_mut()?
                .get_mut(i.parse::<usize>().ok()?)?
                .get_mut(k)
        } else if path.starts_with("pts[") {
            point_number(&mut self.pts, path)
        } else {
            bulge_number(&mut self.bulges, path)
        }
    }
}

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
    pts: &'a mut [kentos_domain::contracts::Vec2],
    name: &str,
    rest: &str,
) -> Option<&'a mut f64> {
    let after = rest.strip_prefix(name)?;
    point_number(pts, &format!("pts{after}"))
}

impl Input for kentos_domain::contracts::NurbsCreate {
    /// `by.ctrl[1].y`, `by.pts[0].x`, `by.weights[2]`, `by.knots[3]`, `lineWeight` (docs/adr/0259).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        use kentos_domain::contracts::NurbsBy;
        if path == "lineWeight" {
            return self.line_weight.as_mut();
        }
        let rest = path.strip_prefix("by.")?;
        match &mut self.by {
            NurbsBy::Control {
                ctrl,
                weights,
                knots,
                ..
            } => named_point(ctrl, "ctrl", rest)
                .or_else(|| list_number(weights.as_deref_mut()?, "weights", rest))
                .or_else(|| list_number(knots.as_deref_mut()?, "knots", rest)),
            NurbsBy::Fit { pts, .. } => named_point(pts, "pts", rest),
        }
    }
}

impl Input for kentos_domain::contracts::WallsCreate {
    /// `walls[0].pts[1].y`, `walls[0].bulges[0]`, `walls[0].width`, `.offset`, `.height`, `lineWeight` (docs/adr/0339).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        if path == "lineWeight" {
            return self.line_weight.as_mut();
        }
        let (i, rest) = path.strip_prefix("walls[")?.split_once("].")?;
        let w = self.walls.get_mut(i.parse::<usize>().ok()?)?;
        match rest {
            "width" => w.width.as_mut(),
            "offset" => w.offset.as_mut(),
            "height" => w.height.as_mut(),
            _ => named_point(&mut w.pts, "pts", rest).or_else(|| list_number(w.bulges.as_deref_mut()?, "bulges", rest)),
        }
    }
}

impl Input for kentos_domain::contracts::WallsFromLines {
    /// `width`, `maxWidth`, `angle`.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "width" => self.width.as_mut(),
            "maxWidth" => self.max_width.as_mut(),
            "angle" => self.angle.as_mut(),
            _ => None,
        }
    }
}

impl Input for kentos_domain::contracts::SpacesCreate {
    /// `spaces[0].seed.x` and `.y`, `.height`, `lineWeight` (docs/adr/0342).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        if path == "lineWeight" {
            return self.line_weight.as_mut();
        }
        let (i, rest) = path.strip_prefix("spaces[")?.split_once("].")?;
        let s = self.spaces.get_mut(i.parse::<usize>().ok()?)?;
        match rest {
            "seed.x" => s.seed.as_mut().map(|p| &mut p.x),
            "seed.y" => s.seed.as_mut().map(|p| &mut p.y),
            "height" => s.height.as_mut(),
            _ => None,
        }
    }
}

impl Input for kentos_domain::contracts::SpacesEdit {
    /// Değiştir's `operation.height`; Böl's `operation.line[k].x` and `.y`; Duvarlardan bul's `operation.seed.x`, `.y`.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        use kentos_domain::contracts::SpacesOperation as Op;
        match &mut self.operation {
            Op::Set(c) if path == "operation.height" => c.height.as_mut()?.as_mut(),
            Op::Split { line } => named_point(line, "operation.line", path),
            Op::Rebound { seed } => match path {
                "operation.seed.x" => seed.as_mut().map(|p| &mut p.x),
                "operation.seed.y" => seed.as_mut().map(|p| &mut p.y),
                _ => None,
            },
            _ => None,
        }
    }
}

impl Input for kentos_domain::contracts::GridsCreate {
    /// `grids[0].pts[k].x` and `.y`, `series.spacings[k]`, `lineWeight` (docs/adr/0343).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        if path == "lineWeight" {
            return self.line_weight.as_mut();
        }
        if let Some(rest) = path.strip_prefix("series.spacings[") {
            let k = rest.strip_suffix(']')?.parse::<usize>().ok()?;
            return self.series.as_mut()?.spacings.get_mut(k);
        }
        let (i, rest) = path.strip_prefix("grids[")?.split_once("].")?;
        let g = self.grids.get_mut(i.parse::<usize>().ok()?)?;
        named_point(&mut g.pts, "pts", rest)
    }
}

impl Input for kentos_domain::contracts::ColumnsCreate {
    /// `columns[0].width`, `.at.x` and `.y`, `lineWeight` (docs/adr/0343).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        if path == "lineWeight" {
            return self.line_weight.as_mut();
        }
        let (i, rest) = path.strip_prefix("columns[")?.split_once("].")?;
        let c = self.columns.get_mut(i.parse::<usize>().ok()?)?;
        match rest {
            "width" => Some(&mut c.width),
            "at.x" => c.at.as_mut().map(|p| &mut p.x),
            "at.y" => c.at.as_mut().map(|p| &mut p.y),
            _ => None,
        }
    }
}

impl Input for kentos_domain::contracts::StairsCreate {
    /// `stairs[0].tread`, `.width`, `.pts[k].x` and `.y`, `lineWeight` (docs/adr/0343).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        if path == "lineWeight" {
            return self.line_weight.as_mut();
        }
        let (i, rest) = path.strip_prefix("stairs[")?.split_once("].")?;
        let s = self.stairs.get_mut(i.parse::<usize>().ok()?)?;
        match rest {
            "tread" => Some(&mut s.tread),
            "width" => Some(&mut s.width),
            _ => named_point(&mut s.pts, "pts", rest),
        }
    }
}

impl Input for kentos_domain::contracts::RampsCreate {
    /// `ramps[0].rise`, `.width`, `.pts[k].x` and `.y`, `lineWeight` (docs/adr/0343).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        if path == "lineWeight" {
            return self.line_weight.as_mut();
        }
        let (i, rest) = path.strip_prefix("ramps[")?.split_once("].")?;
        let r = self.ramps.get_mut(i.parse::<usize>().ok()?)?;
        match rest {
            "rise" => Some(&mut r.rise),
            "width" => Some(&mut r.width),
            _ => named_point(&mut r.pts, "pts", rest),
        }
    }
}

impl Input for kentos_domain::contracts::TagsCreate {
    /// `tags[0].at.x` and `.y`, `tags[0].size`, `lineWeight` (docs/adr/0344).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        if path == "lineWeight" {
            return self.line_weight.as_mut();
        }
        let (i, rest) = path.strip_prefix("tags[")?.split_once("].")?;
        let t = self.tags.get_mut(i.parse::<usize>().ok()?)?;
        match rest {
            "at.x" => t.at.as_mut().map(|p| &mut p.x),
            "at.y" => t.at.as_mut().map(|p| &mut p.y),
            "size" => t.size.as_mut(),
            _ => None,
        }
    }
}

impl Input for kentos_domain::contracts::TagsEdit {
    /// `change.size` (docs/adr/0344).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "change.size" => self.change.size.as_mut().and_then(Option::as_mut),
            _ => None,
        }
    }
}

impl Input for kentos_domain::contracts::TagsCheck {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_domain::contracts::ViewMarksCreate {
    /// `marks[0].points[k].x` and `.y`, `.at.x`, `.radius`, `.angle`, `.size`, `lineWeight` (docs/adr/0345).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        if path == "lineWeight" {
            return self.line_weight.as_mut();
        }
        let (i, rest) = path.strip_prefix("marks[")?.split_once("].")?;
        let m = self.marks.get_mut(i.parse::<usize>().ok()?)?;
        match rest {
            "radius" => m.radius.as_mut(),
            "angle" => m.angle.as_mut(),
            "size" => m.size.as_mut(),
            "at.x" => m.at.as_mut().map(|p| &mut p.x),
            "at.y" => m.at.as_mut().map(|p| &mut p.y),
            _ => named_point(&mut m.points, "points", rest),
        }
    }
}

impl Input for kentos_domain::contracts::ViewMarksEdit {
    /// `operation.size`, `.angle`, `.radius` (docs/adr/0345).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        let kentos_domain::contracts::ViewMarksOperation::Set(c) = &mut self.operation else {
            return None;
        };
        match path {
            "operation.size" => c.size.as_mut().and_then(Option::as_mut),
            "operation.angle" => c.angle.as_mut(),
            "operation.radius" => c.radius.as_mut(),
            _ => None,
        }
    }
}

impl Input for kentos_domain::contracts::ViewMarksReferences {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_domain::contracts::XrefsEdit {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        use kentos_domain::contracts::XrefsOperation as O;
        match (&mut self.operation, path) {
            (O::Attach { at: Some(p), .. }, "operation.at.x") => Some(&mut p.x),
            (O::Attach { at: Some(p), .. }, "operation.at.y") => Some(&mut p.y),
            (O::Attach { scale: Some(s), .. }, "operation.scale") => Some(s),
            (O::Attach { rotation: Some(r), .. }, "operation.rotation") => Some(r),
            _ => None,
        }
    }
}

impl Input for kentos_domain::contracts::XrefsList {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_domain::contracts::AssetsCheck {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_domain::contracts::InsertsClip {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_domain::contracts::ImagesEdit {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_domain::contracts::RastersMosaic {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_domain::contracts::StacSearch {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_domain::contracts::StacPrepare {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_domain::contracts::RastersGeoref {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_domain::contracts::SheetsSet {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_domain::contracts::TemplatesApply {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_domain::contracts::SpacesFinish {
    /// `scale`, `grid.dx`, `.dy`, `.angle` (docs/adr/0345).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "scale" => self.scale.as_mut(),
            "grid.dx" => self.grid.as_mut().map(|g| &mut g.dx),
            "grid.dy" => self.grid.as_mut().map(|g| &mut g.dy),
            "grid.angle" => self.grid.as_mut().map(|g| &mut g.angle),
            _ => None,
        }
    }
}

impl Input for kentos_domain::contracts::SchedulesCreate {
    /// `at.x` and `.y`, `height`, `lineWeight` (docs/adr/0344).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "at.x" => Some(&mut self.at.x),
            "at.y" => Some(&mut self.at.y),
            "height" => self.height.as_mut(),
            "lineWeight" => self.line_weight.as_mut(),
            _ => None,
        }
    }
}

impl Input for kentos_domain::contracts::SchedulesEdit {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_domain::contracts::ArchEdit {
    /// Değiştir's `operation.width`, `.riserHeight`, `.tread`, `.rise` (docs/adr/0343).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        use kentos_domain::contracts::ArchOperation as Op;
        match &mut self.operation {
            Op::Set(c) => match path {
                "operation.width" => c.width.as_mut(),
                "operation.riserHeight" => c.riser_height.as_mut(),
                "operation.tread" => c.tread.as_mut(),
                "operation.rise" => c.rise.as_mut(),
                _ => None,
            },
            _ => None,
        }
    }
}

impl Input for kentos_domain::contracts::SpacesQuantities {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_domain::contracts::OpeningsCreate {
    /// `openings[0].at`, `.distance`, `.width`, `.swing`, `.point.x` and `.y`, `lineWeight` (docs/adr/0341).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        if path == "lineWeight" {
            return self.line_weight.as_mut();
        }
        let (i, rest) = path.strip_prefix("openings[")?.split_once("].")?;
        let o = self.openings.get_mut(i.parse::<usize>().ok()?)?;
        match rest {
            "at" => o.at.as_mut(),
            "distance" => o.distance.as_mut(),
            "width" => o.width.as_mut(),
            "swing" => o.swing.as_mut(),
            "point.x" => o.point.as_mut().map(|p| &mut p.x),
            "point.y" => o.point.as_mut().map(|p| &mut p.y),
            _ => None,
        }
    }
}

impl Input for kentos_domain::contracts::OpeningsEdit {
    /// Değiştir's `operation.width`, `.swing`, `.frame`, `.sill`, `.head`; Taşı's `operation.by` and `.at`; Başka
    /// duvara's `operation.point.x` and `.y`; Kopyala's `operation.offsets[k]` (docs/adr/0341).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        use kentos_domain::contracts::OpeningsOperation as Op;
        match &mut self.operation {
            Op::Set(c) => match path {
                "operation.width" => c.width.as_mut(),
                "operation.swing" => c.swing.as_mut()?.as_mut(),
                "operation.frame" => c.frame.as_mut()?.as_mut(),
                "operation.sill" => c.sill.as_mut()?.as_mut(),
                "operation.head" => c.head.as_mut()?.as_mut(),
                _ => None,
            },
            Op::Move { by, at } => match path {
                "operation.by" => by.as_mut(),
                "operation.at" => at.as_mut(),
                _ => None,
            },
            Op::Rehost { point, .. } => match path {
                "operation.point.x" => point.as_mut().map(|p| &mut p.x),
                "operation.point.y" => point.as_mut().map(|p| &mut p.y),
                _ => None,
            },
            Op::Copy { offsets } => list_number(offsets, "operation.offsets", path),
            _ => None,
        }
    }
}

impl Input for kentos_domain::contracts::WallsJoins {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_domain::contracts::WallsEdit {
    /// `operation.width`, `operation.offset`, `operation.height`; `operation.at.x` and `.y` of Böl.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        use kentos_domain::contracts::WallsOperation;
        let c = match &mut self.operation {
            WallsOperation::Split { at } => {
                return match path {
                    "operation.at.x" => Some(&mut at.x),
                    "operation.at.y" => Some(&mut at.y),
                    _ => None,
                };
            }
            WallsOperation::Set(c) => c,
            _ => return None,
        };
        match path {
            "operation.width" => c.width.as_mut(),
            "operation.offset" => c.offset.as_mut(),
            "operation.height" => c.height.as_mut()?.as_mut(),
            _ => None,
        }
    }
}

impl Input for AnnotationsScales {
    /// `scale`, `rotation`, `offset`, `scales[i]`, `point.x` (which the step's input must give).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "scale" => self.scale.as_mut(),
            "rotation" => self.rotation.as_mut(),
            "offset" => self.offset.as_mut(),
            "point.x" => self.point.as_mut().map(|p| &mut p.x),
            "point.y" => self.point.as_mut().map(|p| &mut p.y),
            _ => {
                let i: usize = path.strip_prefix("scales[")?.strip_suffix(']')?.parse().ok()?;
                self.scales.get_mut(i)
            }
        }
    }
}

impl Input for EntitiesOrder {
    /// No number: the input is ids.
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for EntitiesDelete {
    /// No number: the input is ids.
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for BlocksDefine {
    /// `base.x`, `base.y`.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        coordinate(&mut self.base, "base", path)
    }
}

impl Input for LayersService {
    /// `service.opacity` (which the step's input must give).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "service.opacity" => self.service.as_mut()?.opacity.as_mut(),
            _ => None,
        }
    }
}

/// No number of these is a NaN or ±∞ case (docs/adr/0210 §11).
impl Input for LayersTime {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for ScenariosEdit {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for LayersFilter {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for LayersInfo {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for LayersTable {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for LayersCompose {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for LayersLabels {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for EntitiesTrim {
    /// `picks[0].at.x`, `boundaries.max.y` (which the step's input must give).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        if let Some(rest) = path.strip_prefix("boundaries.") {
            let Some(kentos_domain::contracts::TrimBoundaries::Area { min, max }) =
                &mut self.boundaries
            else {
                return None;
            };
            return match rest {
                "min.x" => Some(&mut min.x),
                "min.y" => Some(&mut min.y),
                "max.x" => Some(&mut max.x),
                "max.y" => Some(&mut max.y),
                _ => None,
            };
        }
        if let Some(rest) = path.strip_prefix("crossing.") {
            let w = self.crossing.as_mut()?;
            return match rest {
                "min.x" => Some(&mut w.min.x),
                "min.y" => Some(&mut w.min.y),
                "max.x" => Some(&mut w.max.x),
                "max.y" => Some(&mut w.max.y),
                _ => None,
            };
        }
        if let Some(rest) = path.strip_prefix("fence[") {
            let (i, rest) = rest.split_once(']')?;
            let p = self.fence.as_mut()?.get_mut(i.parse::<usize>().ok()?)?;
            return match rest {
                ".x" => Some(&mut p.x),
                ".y" => Some(&mut p.y),
                _ => None,
            };
        }
        let rest = path.strip_prefix("picks[")?;
        let (i, rest) = rest.split_once(']')?;
        let p = &mut self.picks.get_mut(i.parse::<usize>().ok()?)?.at;
        match rest {
            ".at.x" => Some(&mut p.x),
            ".at.y" => Some(&mut p.y),
            _ => None,
        }
    }
}

/// `min.x` … `max.y` of a rectangle given as `TrimBoundaries::Area`.
fn area_number<'a>(
    b: &'a mut Option<kentos_domain::contracts::TrimBoundaries>,
    rest: &str,
) -> Option<&'a mut f64> {
    let Some(kentos_domain::contracts::TrimBoundaries::Area { min, max }) = b else {
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

impl Input for kentos_domain::contracts::SnapFind {
    /// `at.x`, `at.y`, `aperture`, `from.x`, `from.y`.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "at.x" => Some(&mut self.at.x),
            "at.y" => Some(&mut self.at.y),
            "aperture" => Some(&mut self.aperture),
            "from.x" => self.from.as_mut().map(|f| &mut f.x),
            "from.y" => self.from.as_mut().map(|f| &mut f.y),
            _ => None,
        }
    }
}

/// A coordinate of a point by its axis name.
fn axis<'a>(p: &'a mut kentos_domain::contracts::Vec2, a: &str) -> Option<&'a mut f64> {
    match a {
        "x" => Some(&mut p.x),
        "y" => Some(&mut p.y),
        _ => None,
    }
}

impl Input for kentos_domain::contracts::RectangleCreate {
    /// `frame.a.x` … `frame.length`, `frame.width`, `frame.area`, `rotation`, `corners.radius`, `corners.distance`.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        use kentos_domain::contracts::{RectangleCorners as C, RectangleFrame as F};
        if path == "rotation" {
            return Some(self.rotation.get_or_insert(0.0));
        }
        let (head, field) = path.split_once('.')?;
        match (head, field) {
            ("frame", field) => match (&mut self.frame, field) {
                (F::Corners { a, .. }, "a.x") | (F::Edge { a, .. }, "a.x") => Some(&mut a.x),
                (F::Corners { a, .. }, "a.y") | (F::Edge { a, .. }, "a.y") => Some(&mut a.y),
                (F::Corners { b, .. }, "b.x") | (F::Edge { b, .. }, "b.x") => Some(&mut b.x),
                (F::Corners { b, .. }, "b.y") | (F::Edge { b, .. }, "b.y") => Some(&mut b.y),
                (F::Size { corner, .. }, "corner.x") | (F::Area { corner, .. }, "corner.x") => {
                    Some(&mut corner.x)
                }
                (F::Size { length, .. }, "length") | (F::Area { length, .. }, "length") => {
                    Some(length)
                }
                (F::Size { width, .. }, "width") | (F::Edge { width, .. }, "width") => Some(width),
                (F::Area { area, .. }, "area") => Some(area),
                _ => None,
            },
            ("corners", "radius") => match &mut self.corners {
                Some(C::Fillet { radius }) => Some(radius),
                _ => None,
            },
            ("corners", "distance") => match &mut self.corners {
                Some(C::Chamfer { distance }) => Some(distance),
                _ => None,
            },
            _ => None,
        }
    }
}

impl Input for kentos_domain::contracts::RegularPolygonCreate {
    /// `frame.center.x`, `frame.a.y` …
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        use kentos_domain::contracts::RegularFrame as F;
        match (&mut self.frame, path) {
            (F::Inscribed { center, .. } | F::Circumscribed { center, .. }, "frame.center.x") => {
                Some(&mut center.x)
            }
            (F::Inscribed { corner, .. }, "frame.corner.y") => Some(&mut corner.y),
            (F::Edge { a, .. }, "frame.a.x") => Some(&mut a.x),
            _ => None,
        }
    }
}

impl Input for kentos_domain::contracts::EntitiesVertex {
    /// `radius`, `through.x`, `through.y`, `widths[0]`, `widths[1]` (docs/adr/0288).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "radius" => self.radius.as_mut(),
            "through.x" => self.through.as_mut().map(|p| &mut p.x),
            "through.y" => self.through.as_mut().map(|p| &mut p.y),
            "widths[0]" => self.widths.as_mut().map(|w| &mut w[0]),
            "widths[1]" => self.widths.as_mut().map(|w| &mut w[1]),
            _ => None,
        }
    }
}

impl Input for kentos_domain::contracts::EntitiesPick {
    /// `at.x`, `at.y`, `tolerance`.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "at.x" => Some(&mut self.at.x),
            "at.y" => Some(&mut self.at.y),
            "tolerance" => Some(&mut self.tolerance),
            _ => None,
        }
    }
}

// Ids and a pattern: no number a case makes NaN (docs/adr/0501).
impl Input for kentos_domain::contracts::EntitiesRelate {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_domain::contracts::EntitiesSelect {
    /// The region's `region.a.x` … `region.radius`, `region.tolerance`, `region.points[i].x`.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        use kentos_domain::contracts::SelectRegion as R;
        let rest = path.strip_prefix("region.")?;
        let (head, x) = rest.split_once('.').unwrap_or((rest, ""));
        match (&mut self.region, head) {
            (R::Window { a, .. } | R::Crossing { a, .. }, "a") => axis(a, x),
            (R::Window { b, .. } | R::Crossing { b, .. }, "b") => axis(b, x),
            (R::Circle { center, .. }, "center") => axis(center, x),
            (R::Containing { at } | R::Point { at, .. }, "at") => axis(at, x),
            (R::Circle { radius, .. }, "radius") => Some(radius),
            (R::Fence { tolerance, .. } | R::Point { tolerance, .. }, "tolerance") => {
                Some(tolerance)
            }
            (R::Fence { points, .. } | R::Polygon { points, .. }, head) => {
                let i: usize = head
                    .strip_prefix("points[")?
                    .strip_suffix(']')?
                    .parse()
                    .ok()?;
                axis(points.get_mut(i)?, x)
            }
            _ => None,
        }
    }
}

/// A coordinate of a point given by its coordinates.
fn measured_number<'a>(
    p: &'a mut kentos_domain::contracts::MeasuredPoint,
    axis: &str,
) -> Option<&'a mut f64> {
    match (p, axis) {
        (kentos_domain::contracts::MeasuredPoint::At { at }, "x") => Some(&mut at.x),
        (kentos_domain::contracts::MeasuredPoint::At { at }, "y") => Some(&mut at.y),
        _ => None,
    }
}

impl Input for kentos_domain::contracts::CrsTransform {
    /// `points[i].x`, `.y`, `.z`, `uncertainty.horizontal`, `.vertical` (docs/adr/0519).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        if let Some(rest) = path.strip_prefix("points[") {
            let (i, axis) = rest.split_once("].")?;
            let p = self.points.get_mut(i.parse::<usize>().ok()?)?;
            return match axis {
                "x" => Some(&mut p.x),
                "y" => Some(&mut p.y),
                "z" => p.z.as_mut(),
                _ => None,
            };
        }
        let u = self.uncertainty.as_mut()?;
        match path {
            "uncertainty.horizontal" => u.horizontal.as_mut(),
            "uncertainty.vertical" => u.vertical.as_mut(),
            _ => None,
        }
    }
}

impl Input for kentos_domain::contracts::PointsConstruct {
    /// `constructions[i].distance`, `.angle`, `.a.at.x` … `.edge.near.y`.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        use kentos_domain::contracts::PointConstruction as C;
        let rest = path.strip_prefix("constructions[")?;
        let (i, rest) = rest.split_once("].")?;
        let c = self.constructions.get_mut(i.parse::<usize>().ok()?)?;
        let (head, axis) = rest.rsplit_once(".at.").unwrap_or((rest, ""));
        match (c, head) {
            (C::Mid { a, .. } | C::Along { a, .. }, "a") => measured_number(a, axis),
            (C::Mid { b, .. } | C::Along { b, .. }, "b") => measured_number(b, axis),
            (C::Track { x, .. }, "x") => measured_number(x, axis),
            (C::Track { y, .. }, "y") => measured_number(y, axis),
            (C::Polar { from, .. }, "from") => measured_number(from, axis),
            (C::Parallel { through, .. }, "through") => measured_number(through, axis),
            (C::Parallel { edge, .. }, "edge.near.x") => Some(&mut edge.near.x),
            (C::Parallel { edge, .. }, "edge.near.y") => Some(&mut edge.near.y),
            (
                C::Extension { distance, .. }
                | C::Parallel { distance, .. }
                | C::Polar { distance, .. }
                | C::Along { distance, .. },
                "distance",
            ) => Some(distance),
            (C::Polar { angle, .. }, "angle") => Some(angle),
            _ => None,
        }
    }
}

impl Input for kentos_domain::contracts::CurvesConstruct {
    /// `constructions[i].a.at.x` … (any point), `.first.near.x` …, `.r`, `.angle`, `.chord`, `.direction`,
    /// `.other`, `.rotation`, `.startAngle`, `.endAngle`.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        use kentos_domain::contracts::CurveConstruction as C;
        let rest = path.strip_prefix("constructions[")?;
        let (i, rest) = rest.split_once("].")?;
        let c = self.constructions.get_mut(i.parse::<usize>().ok()?)?;
        if let Some((head, axis)) = rest.rsplit_once(".at.") {
            let p = match (c, head) {
                (
                    C::CircleTwoPoints { a, .. }
                    | C::CircleThreePoints { a, .. }
                    | C::EllipseAxis { a, .. },
                    "a",
                ) => a,
                (
                    C::CircleTwoPoints { b, .. }
                    | C::CircleThreePoints { b, .. }
                    | C::EllipseAxis { b, .. },
                    "b",
                ) => b,
                (C::CircleThreePoints { c, .. }, "c") => c,
                (
                    C::ArcThreePoints { start, .. }
                    | C::ArcStartCentreEnd { start, .. }
                    | C::ArcStartCentreAngle { start, .. }
                    | C::ArcStartCentreChord { start, .. }
                    | C::ArcStartEndAngle { start, .. }
                    | C::ArcStartEndDirection { start, .. }
                    | C::ArcStartEndRadius { start, .. },
                    "start",
                ) => start,
                (C::ArcThreePoints { mid, .. }, "mid") => mid,
                (
                    C::ArcThreePoints { end, .. }
                    | C::ArcStartCentreEnd { end, .. }
                    | C::ArcStartEndAngle { end, .. }
                    | C::ArcStartEndDirection { end, .. }
                    | C::ArcStartEndRadius { end, .. }
                    | C::ArcContinue { end, .. },
                    "end",
                ) => end,
                (
                    C::ArcStartCentreEnd { centre, .. }
                    | C::ArcStartCentreAngle { centre, .. }
                    | C::ArcStartCentreChord { centre, .. }
                    | C::EllipseCentre { centre, .. },
                    "centre",
                ) => centre,
                (C::EllipseCentre { axis_end, .. }, "axisEnd") => axis_end,
                _ => return None,
            };
            return measured_number(p, axis);
        }
        match (c, rest) {
            (
                C::CircleTangentTangentRadius { first, .. } | C::CircleTangents { first, .. },
                "first.near.x",
            ) => Some(&mut first.near.x),
            (
                C::CircleTangentTangentRadius { first, .. } | C::CircleTangents { first, .. },
                "first.near.y",
            ) => Some(&mut first.near.y),
            (C::CircleTangentTangentRadius { r, .. } | C::ArcStartEndRadius { r, .. }, "r") => {
                Some(r)
            }
            (C::ArcStartCentreAngle { angle, .. } | C::ArcStartEndAngle { angle, .. }, "angle") => {
                Some(angle)
            }
            (C::ArcStartCentreChord { chord, .. }, "chord") => Some(chord),
            (C::ArcStartEndDirection { direction, .. }, "direction") => Some(direction),
            (C::EllipseAxis { other, .. } | C::EllipseCentre { other, .. }, "other") => {
                other.as_mut()
            }
            (C::EllipseAxis { rotation, .. } | C::EllipseCentre { rotation, .. }, "rotation") => {
                rotation.as_mut()
            }
            (
                C::EllipseAxis { start_angle, .. } | C::EllipseCentre { start_angle, .. },
                "startAngle",
            ) => start_angle.as_mut(),
            (C::EllipseAxis { end_angle, .. } | C::EllipseCentre { end_angle, .. }, "endAngle") => {
                end_angle.as_mut()
            }
            _ => None,
        }
    }
}

impl Input for EntitiesStretch {
    /// `window.min.x` … `window.max.y`, `dx`, `dy`.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "dx" => Some(&mut self.dx),
            "dy" => Some(&mut self.dy),
            "window.min.x" => Some(&mut self.window.min.x),
            "window.min.y" => Some(&mut self.window.min.y),
            "window.max.x" => Some(&mut self.window.max.x),
            "window.max.y" => Some(&mut self.window.max.y),
            _ => None,
        }
    }
}

impl Input for EntitiesBreak {
    /// `first.x` … `second.y` (which the step's input must give).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "first.x" => Some(&mut self.first.x),
            "first.y" => Some(&mut self.first.y),
            "second.x" => self.second.as_mut().map(|p| &mut p.x),
            "second.y" => self.second.as_mut().map(|p| &mut p.y),
            _ => None,
        }
    }
}

impl Input for EntitiesJoin {
    /// `tolerance`, `chain.min.x` … (which the step's input must give).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        if path == "tolerance" {
            return self.tolerance.as_mut();
        }
        area_number(&mut self.chain, path.strip_prefix("chain.")?)
    }
}

impl Input for kentos_domain::contracts::EntitiesLengthen {
    /// `change.length`, `change.angle`, `change.percent`, `change.to.x` and `.y`.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        use kentos_domain::contracts::LengthenChange as C;
        match (&mut self.change, path) {
            (C::Delta { length, .. } | C::Total { length, .. }, "change.length") => length.as_mut(),
            (C::Delta { angle, .. } | C::Total { angle, .. }, "change.angle") => angle.as_mut(),
            (C::Percent { percent }, "change.percent") => Some(percent),
            (C::Dynamic { to }, "change.to.x") => Some(&mut to.x),
            (C::Dynamic { to }, "change.to.y") => Some(&mut to.y),
            _ => None,
        }
    }
}

impl Input for kentos_domain::contracts::EntitiesReverse {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_domain::contracts::EntitiesCopy {
    /// `base.x`, `base.y`.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        let b = self.base.as_mut()?;
        match path {
            "base.x" => Some(&mut b.x),
            "base.y" => Some(&mut b.y),
            _ => None,
        }
    }
}

impl Input for kentos_domain::contracts::EntitiesPaste {
    /// `at.x`, `at.y`, `clip.base.x`, `clip.base.y`.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "at.x" => self.at.as_mut().map(|a| &mut a.x),
            "at.y" => self.at.as_mut().map(|a| &mut a.y),
            "clip.base.x" => Some(&mut self.clip.base.x),
            "clip.base.y" => Some(&mut self.clip.base.y),
            _ => None,
        }
    }
}

impl Input for kentos_domain::contracts::DrawingAudit {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_domain::contracts::EntitiesCleanup {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "tolerance" => self.tolerance.as_mut(),
            _ => None,
        }
    }
}

impl Input for kentos_domain::contracts::EntitiesMatch {
    fn number(&mut self, _: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_domain::contracts::CurvesEdit {
    /// `edit.at.x`, `edit.at.y`, `edit.weight`, `edit.tolerance`, a knot's `edit.at`.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        use kentos_domain::contracts::CurveEdit as E;
        match (&mut self.edit, path) {
            (
                E::AddFit { at, .. } | E::MoveFit { at, .. } | E::MoveControl { at, .. },
                "edit.at.x",
            ) => Some(&mut at.x),
            (
                E::AddFit { at, .. } | E::MoveFit { at, .. } | E::MoveControl { at, .. },
                "edit.at.y",
            ) => Some(&mut at.y),
            (E::Weight { weight, .. }, "edit.weight") => Some(weight),
            (E::ToPolyline { tolerance }, "edit.tolerance") => Some(tolerance),
            (E::InsertKnot { at }, "edit.at") => Some(at),
            _ => None,
        }
    }
}

impl Input for kentos_domain::contracts::EntitiesExplode {
    /// `density.length`, `density.tolerance`.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        use kentos_domain::contracts::ExplodeDensity as D;
        match (self.density.as_mut()?, path) {
            (D::Step { length }, "density.length") => Some(length),
            (D::Deviation { tolerance }, "density.tolerance") => Some(tolerance),
            _ => None,
        }
    }
}

impl Input for kentos_domain::contracts::ArraysEdit {
    /// `edit.scale`, `edit.rotation`, `edit.p.x`, `edit.p.y`, a layout's `edit.layout.dx` and `edit.layout.dy`.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        use kentos_domain::contracts::{ArrayEdit as E, ArrayLayout as L};
        match (&mut self.edit, path) {
            (E::Placement { scale, .. }, "edit.scale") => scale.as_mut(),
            (E::Placement { rotation, .. }, "edit.rotation") => rotation.as_mut(),
            (E::Placement { p, .. }, "edit.p.x") => p.as_mut().map(|p| &mut p.x),
            (E::Placement { p, .. }, "edit.p.y") => p.as_mut().map(|p| &mut p.y),
            (
                E::Layout {
                    layout: L::Grid { dx, .. } | L::Line { dx, .. },
                },
                "edit.layout.dx",
            ) => Some(dx),
            (
                E::Layout {
                    layout: L::Grid { dy, .. } | L::Line { dy, .. },
                },
                "edit.layout.dy",
            ) => Some(dy),
            _ => None,
        }
    }
}

impl Input for EntitiesDivide {
    /// `division.length`, `start`, `block.rotation`, `block.scale`, `lineWeight` (which the step's input must give).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match (path, &mut self.division) {
            ("division.length", Division::Step { length }) => Some(length),
            ("start", _) => self.start.as_mut(),
            ("block.rotation", _) => self.block.as_mut().map(|b| &mut b.rotation),
            ("block.scale", _) => self.block.as_mut().map(|b| &mut b.scale),
            ("lineWeight", _) => self.look.line_weight.as_mut(),
            _ => None,
        }
    }
}

impl Input for DimensionsCreate {
    /// `at.x`, `offset`, `height`, `lineWeight`, `measure.a.at.x`, `measure.angle`,
    /// `measure.za`, `measure.of.near.y`, `measure.between[1].x`, `measure.shownCentre.x`, `measure.on.y` …
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        fn xy<'a>(p: &'a mut kentos_contracts::Vec2, rest: &str) -> Option<&'a mut f64> {
            match rest {
                "x" => Some(&mut p.x),
                "y" => Some(&mut p.y),
                _ => None,
            }
        }
        match path {
            "offset" => return self.offset.as_mut(),
            "height" => return self.height.as_mut(),
            "lineWeight" => return self.look.line_weight.as_mut(),
            _ => {}
        }
        if let Some(rest) = path.strip_prefix("at.") {
            return xy(&mut self.at, rest);
        }
        let rest = path.strip_prefix("measure.")?;
        let (field, rest) = rest.split_once('.').unwrap_or((rest, ""));
        fn point(p: &mut MeasuredPoint) -> Option<&mut kentos_contracts::Vec2> {
            match p {
                MeasuredPoint::At { at } => Some(at),
                MeasuredPoint::Place { .. } => None,
            }
        }
        match (&mut self.measure, field) {
            (DimensionMeasure::Aligned { a, .. } | DimensionMeasure::Azimuth { a, .. }, "a")
            | (DimensionMeasure::Linear { a, .. } | DimensionMeasure::Slope { a, .. }, "a")
            | (
                DimensionMeasure::Ordinate { a, .. } | DimensionMeasure::AngleVertex { a, .. },
                "a",
            ) => xy(point(a)?, rest.strip_prefix("at.")?),
            (DimensionMeasure::Aligned { b, .. } | DimensionMeasure::Azimuth { b, .. }, "b")
            | (DimensionMeasure::Linear { b, .. } | DimensionMeasure::Slope { b, .. }, "b")
            | (DimensionMeasure::AngleVertex { b, .. }, "b") => {
                xy(point(b)?, rest.strip_prefix("at.")?)
            }
            (DimensionMeasure::AngleVertex { c, .. }, "c") => {
                xy(point(c)?, rest.strip_prefix("at.")?)
            }
            (
                DimensionMeasure::Linear { angle, .. } | DimensionMeasure::Ordinate { angle, .. },
                "angle",
            ) => angle.as_mut(),
            (DimensionMeasure::Slope { za, .. }, "za") => za.as_mut(),
            (DimensionMeasure::Slope { zb, .. }, "zb") => zb.as_mut(),
            (DimensionMeasure::Elevation { z, .. }, "z") => z.as_mut(),
            (DimensionMeasure::Elevation { a, .. }, "a") => {
                xy(point(a)?, rest.strip_prefix("at.")?)
            }
            (DimensionMeasure::Angle { first, .. }, "first") => {
                xy(&mut first.near, rest.strip_prefix("near.")?)
            }
            (DimensionMeasure::Angle { second, .. }, "second") => {
                xy(&mut second.near, rest.strip_prefix("near.")?)
            }
            (
                DimensionMeasure::AngleArc { of }
                | DimensionMeasure::Radius { of }
                | DimensionMeasure::Diameter { of }
                | DimensionMeasure::ArcLength { of, .. }
                | DimensionMeasure::Jogged { of, .. },
                "of",
            ) => xy(&mut of.near, rest.strip_prefix("near.")?),
            (
                DimensionMeasure::ArcLength {
                    between: Some([p, q]),
                    ..
                },
                f,
            ) if f.starts_with("between[") => match f {
                "between[0]" => xy(p, rest),
                "between[1]" => xy(q, rest),
                _ => None,
            },
            (DimensionMeasure::Jogged { shown_centre, .. }, "shownCentre") => {
                xy(shown_centre, rest)
            }
            (DimensionMeasure::Jogged { on, .. }, "on") => xy(on, rest),
            _ => None,
        }
    }
}

impl Input for EntitiesBoundary {
    /// `at.x`, `at.y`, `boundaries.min.x` …, `lineWeight`.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "at.x" => Some(&mut self.at.x),
            "at.y" => Some(&mut self.at.y),
            "lineWeight" => self.look.line_weight.as_mut(),
            _ => area_number(&mut self.boundaries, path.strip_prefix("boundaries.")?),
        }
    }
}

impl Input for EntitiesHatch {
    /// `pattern.scale`, `pattern.angle`, `region.at.x`, `region.boundaries.min.x` …
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "pattern.scale" => return self.pattern.scale.as_mut(),
            "pattern.angle" => return self.pattern.angle.as_mut(),
            "lineWeight" => return self.look.line_weight.as_mut(),
            // docs/adr/0331 §2.
            "pattern.origin.at.x" | "pattern.origin.at.y" => {
                let Some(kentos_contracts::HatchOrigin::Point { at }) = &mut self.pattern.origin else {
                    return None;
                };
                return Some(if path.ends_with('x') { &mut at.x } else { &mut at.y });
            }
            _ => {}
        }
        let HatchRegion::Point {
            at,
            boundaries,
            also_at,
            gap,
            ..
        } = &mut self.region
        else {
            return None;
        };
        match path {
            "region.at.x" => Some(&mut at.x),
            "region.at.y" => Some(&mut at.y),
            "region.gap" => gap.as_mut(),
            _ if path.starts_with("region.alsoAt[") => {
                point_number(also_at, path.strip_prefix("region.alsoAt")?.replacen('[', "pts[", 1).as_str())
            }
            _ => area_number(boundaries, path.strip_prefix("region.boundaries.")?),
        }
    }
}

impl Input for EntitiesFillet {
    /// `radius` (which the step's input must give).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        (path == "radius").then_some(&mut self.radius)
    }
}

impl Input for EntitiesChamfer {
    /// `distance` (which the step's input must give).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        (path == "distance").then_some(&mut self.distance)
    }
}

impl Input for DimensionsReassociate {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_domain::contracts::DimensionsEdit {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        use kentos_domain::contracts::DimensionEdit as E;
        match (&mut self.edit, path) {
            (
                E::Space {
                    spacing: Some(s), ..
                },
                "edit.spacing",
            ) => Some(s),
            (E::MoveText { along, .. }, "edit.along") => Some(along),
            (E::MoveText { across, .. }, "edit.across") => Some(across),
            _ => None,
        }
    }
}

impl Input for kentos_domain::contracts::LeadersEdit {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        use kentos_domain::contracts::LeaderEdit;
        match (&mut self.edit, path) {
            (LeaderEdit::Align { angle, .. }, "edit.angle") => Some(angle),
            (LeaderEdit::Align { spacing: Some(s), .. }, "edit.spacing") => Some(s),
            _ => None,
        }
    }
}

impl Input for kentos_domain::contracts::HatchesEdit {
    /// `edit.region.at.x`, `edit.region.at.y` (Sınırı yeniden seç).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        use kentos_domain::contracts::{HatchEdit, HatchRegion};
        let HatchEdit::Reselect {
            region: HatchRegion::Point { at, .. },
            ..
        } = &mut self.edit
        else {
            return None;
        };
        match path {
            "edit.region.at.x" => Some(&mut at.x),
            "edit.region.at.y" => Some(&mut at.y),
            _ => None,
        }
    }
}

impl Input for kentos_domain::contracts::TablesEdit {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        use kentos_domain::contracts::{TableEdit, TablesOperation};
        let TablesOperation::Edit { edits, .. } = &mut self.operation else {
            return None;
        };
        // `operation.edits[i].height` or `.width`.
        let rest = path.strip_prefix("operation.edits[")?;
        let (i, field) = rest.split_once("].")?;
        match (edits.get_mut(i.parse::<usize>().ok()?)?, field) {
            (TableEdit::RowHeight { height, .. }, "height") => Some(height),
            (TableEdit::ColumnWidth { width, .. }, "width") => Some(width),
            (TableEdit::Frame { width: Some(w) }, "width") => Some(w),
            _ => None,
        }
    }
}

impl Input for kentos_domain::contracts::FieldsEdit {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_domain::contracts::TextsEdit {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_domain::contracts::TextsCheck {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_domain::contracts::FontsReport {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_domain::contracts::DimensionsQuery {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for LabelsPin {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for LayersRenderer {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for LayersEdit {
    /// `lineWeight` (which the step's input must give, docs/adr/0248 §1).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "lineWeight" => self.line_weight.as_mut(),
            _ => None,
        }
    }
}

impl Input for LayersSet {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for LayersStates {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for FieldsUsage {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for LayersFind {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        (path == "lineWeight").then_some(())?;
        self.line_weight.as_mut()
    }
}

impl Input for LibraryEdit {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for StylesImpact {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for LineTypesEdit {
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        (path == "scale").then_some(())?;
        self.scale.as_mut()
    }
}

impl Input for HatchPatternsEdit {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for StandardsCheck {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for StandardsApply {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for LayersTranslate {
    /// `rules[2].lineWeight`.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        let rest = path.strip_prefix("rules[")?;
        let (i, field) = rest.split_once("].")?;
        let rule = self.rules.get_mut(i.parse::<usize>().ok()?)?;
        (field == "lineWeight").then_some(())?;
        rule.line_weight.as_mut()
    }
}

/// No style's number is a NaN or ±∞ case: the table's rule refuses them as
/// any other value out of its bounds (docs/adr/0248 §4).
impl Input for StylesEdit {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for SheetsEdit {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for ChangesetApply {
    /// A step's numbers cross as JSON, which carries no NaN: none here.
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for SettingsSet {
    /// `plotScale` (which the step's input must give, docs/adr/0248 §5).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "plotScale" => self.plot_scale.as_mut(),
            _ => None,
        }
    }
}

impl Input for ViewsEdit {
    /// A saved view's `op.center.x`, `op.center.y`, `op.width`, `op.height`, `op.twist` (docs/adr/0285 §1).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        let ViewsOp::Save {
            center,
            width,
            height,
            twist,
            ..
        } = &mut self.edit
        else {
            return None;
        };
        match path {
            "edit.center.x" => Some(&mut center.x),
            "edit.center.y" => Some(&mut center.y),
            "edit.width" => Some(width),
            "edit.height" => Some(height),
            "edit.twist" => Some(twist.get_or_insert(0.0)),
            _ => None,
        }
    }
}

impl Input for UcsSet {
    /// `to.angle`; a point's `x` or `y` (`to.origin.x`, `to.point.y`, `to.xPoint.x`, `to.at.y`);
    /// the command's errors name the point (docs/adr/0261 §2).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        if path == "to.angle" {
            return match &mut self.to {
                UcsTo::Frame { angle, .. } | UcsTo::Rotate { angle } => Some(angle),
                _ => None,
            };
        }
        let (point, axis) = path.rsplit_once('.')?;
        let p = match (&mut self.to, point) {
            (UcsTo::Frame { origin: p, .. }, "to.origin")
            | (UcsTo::Origin { point: p }, "to.point")
            | (UcsTo::TwoPoints { origin: p, .. }, "to.origin")
            | (UcsTo::TwoPoints { x_point: p, .. }, "to.xPoint")
            | (UcsTo::Object { at: p, .. }, "to.at") => p,
            _ => return None,
        };
        match axis {
            "x" => Some(&mut p.x),
            "y" => Some(&mut p.y),
            _ => None,
        }
    }
}

impl Input for UnitsChange {
    /// No numbers: two units and a choice (docs/adr/0256 §1).
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for ImportDxf {
    /// No number: a path, names and choices.
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

// No number: a path and a count (docs/adr/0530).
impl Input for kentos_domain::contracts::GisDerivation {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_domain::contracts::ImportCsv {
    /// No number: a path, a layer and column names.
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_domain::contracts::ImportSld {
    /// No number: a path or a text, a layer.
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for ImportPdf {
    /// The numbers a case may give through a placeholder: none, the file's are literal.
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for ImportGpkg {
    /// No number: a path, table names and a choice.
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_domain::contracts::SourcesDescribe {
    /// No number: a path.
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_domain::contracts::SourcesEdit {
    /// No number: paths, a table, a layer and ids.
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_domain::contracts::ProcessingTools {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_domain::contracts::ProcessingRun {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_domain::contracts::ProcessingBatch {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_domain::contracts::FieldsWrite {
    /// No number: a layer, fields and values as text.
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_domain::contracts::SourcesSync {
    /// No number: a layer and a direction.
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_domain::contracts::SourcesRelate {
    /// No number: a relation, an operation and a table's name.
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_domain::contracts::RecordsPage {
    /// No number to move: a layer or a relation, words, a cursor.
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for NetworkDefine {
    /// `network.tolerance` (which the step's input must give).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "network.tolerance" => self.network.as_mut().map(|n| &mut n.tolerance),
            _ => None,
        }
    }
}

impl Input for EntitiesOffset {
    /// `by.distance`, `by.side.x`, `by.point.y` (which the step's input must give).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match (&mut self.by, path) {
            (OffsetBy::Distance { distance, .. }, "by.distance") => Some(distance),
            (OffsetBy::Distance { side: p, .. }, _) | (OffsetBy::Through { point: p }, _) => {
                match path
                    .strip_prefix("by.side.")
                    .or(path.strip_prefix("by.point."))?
                {
                    "x" => Some(&mut p.x),
                    "y" => Some(&mut p.y),
                    _ => None,
                }
            }
        }
    }
}

impl Input for AddressesDefine {
    /// `locator.reach` (which the step's input must give).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "locator.reach" => self.locator.as_mut().map(|l| &mut l.reach),
            _ => None,
        }
    }
}

impl Input for BlocksEdit {
    /// `base.x`, `base.y`; `attributes[0].p.x`, `attributes[1].rotation`,
    /// `attributes[2].height` (which the step's input must give).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        let Some(rest) = path.strip_prefix("attributes[") else {
            return coordinate(self.base.as_mut()?, "base", path);
        };
        let (i, field) = rest.split_once("].")?;
        let a = self
            .attributes
            .as_mut()?
            .get_mut(i.parse::<usize>().ok()?)?;
        match field {
            "rotation" => Some(&mut a.rotation),
            "height" => Some(&mut a.height),
            _ => coordinate(&mut a.p, "p", field),
        }
    }
}

impl Input for kentos_domain::contracts::BlocksReplace {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_domain::contracts::BlocksQuery {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_domain::contracts::BlocksParameters {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_domain::contracts::BlocksProxy {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_domain::contracts::BlocksFreeze {
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_domain::contracts::ImportBlock {
    /// No number to make not finite.
    fn number(&mut self, _path: &str) -> Option<&mut f64> {
        None
    }
}

impl Input for kentos_domain::contracts::BlocksInsert {
    /// `at.x`, `at.y`, `rotation`, `scale`, `yScale` (which the step's input must give).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "rotation" => self.rotation.as_mut(),
            "scale" => self.scale.as_mut(),
            "yScale" => self.y_scale.as_mut(),
            _ => coordinate(&mut self.at, "at", path),
        }
    }
}

impl Input for EntitiesSetProperties {
    /// `lineWeight` (which the step's input must give); the rest is ids and texts.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "lineWeight" => self.line_weight.as_mut()?.as_mut(),
            "lineTypeScale" => self.line_type_scale.as_mut()?.as_mut(),
            "transparency" => self.transparency.as_mut()?.as_mut(),
            _ => None,
        }
    }
}

/// A coordinate of a named point: `c.x`, `p.y`.
fn coordinate<'a>(
    p: &'a mut kentos_domain::contracts::Vec2,
    name: &str,
    path: &str,
) -> Option<&'a mut f64> {
    match path.strip_prefix(name)?.strip_prefix('.')? {
        "x" => Some(&mut p.x),
        "y" => Some(&mut p.y),
        _ => None,
    }
}

impl Input for PointCreate {
    /// `p.x`, `p.y`, `z` (which the step's input must give).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "z" => self.z.as_mut(),
            _ => coordinate(&mut self.p, "p", path),
        }
    }
}

impl Input for CircleCreate {
    /// `c.x`, `c.y`, `r`, `lineWeight` (which the step's input must give).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "r" => Some(&mut self.r),
            "lineWeight" => self.line_weight.as_mut(),
            _ => coordinate(&mut self.c, "c", path),
        }
    }
}

impl Input for ArcCreate {
    /// `c.x`, `c.y`, `r`, `a0`, `a1`, `lineWeight` (which the step's input must give).
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        match path {
            "r" => Some(&mut self.r),
            "lineWeight" => self.line_weight.as_mut(),
            "a0" => Some(&mut self.a0),
            "a1" => Some(&mut self.a1),
            _ => coordinate(&mut self.c, "c", path),
        }
    }
}

impl Input for EntitiesTransform {
    /// `transform.dx`, `transform.center.x`, `transform.angle`, `transform.factor`, `transform.a.y` …
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        let rest = path.strip_prefix("transform.")?;
        match &mut self.transform {
            Transform::Move { dx, dy } => match rest {
                "dx" => Some(dx),
                "dy" => Some(dy),
                _ => None,
            },
            Transform::Rotate { center, angle } => match rest {
                "angle" => Some(angle),
                _ => coordinate(center, "center", rest),
            },
            Transform::Scale { center, factor } => match rest {
                "factor" => Some(factor),
                _ => coordinate(center, "center", rest),
            },
            Transform::Mirror { a, b, .. } => {
                coordinate(a, "a", rest).or_else(|| coordinate(b, "b", rest))
            }
            Transform::Align {
                source,
                target,
                source2,
                target2,
                ..
            } => coordinate(source, "source", rest)
                .or_else(|| coordinate(target, "target", rest))
                .or_else(|| coordinate(source2.as_mut()?, "source2", rest))
                .or_else(|| coordinate(target2.as_mut()?, "target2", rest)),
            // Oturt (docs/adr/0156): `transform.from.x`, `transform.a`, `transform.m[2]`, `transform.h[6]` …
            Transform::Similarity { from, to, a, b } => match rest {
                "a" => Some(a),
                "b" => Some(b),
                _ => coordinate(from, "from", rest).or_else(|| coordinate(to, "to", rest)),
            },
            Transform::Affine { from, to, m } => indexed(m, "m", rest)
                .or_else(|| coordinate(from, "from", rest))
                .or_else(|| coordinate(to, "to", rest)),
            Transform::Projective { from, to, h } => indexed(h, "h", rest)
                .or_else(|| coordinate(from, "from", rest))
                .or_else(|| coordinate(to, "to", rest)),
            // Kauçuk levha (docs/adr/0158): `transform.links[1].to.x` …
            Transform::Rubbersheet { links } => {
                let (at, field) = rest.strip_prefix("links[")?.split_once("].")?;
                let link = links.get_mut(at.parse::<usize>().ok()?)?;
                coordinate(&mut link.from, "from", field)
                    .or_else(|| coordinate(&mut link.to, "to", field))
            }
            // Hizala ve dağıt (docs/adr/0194): `transform.at`.
            Transform::Arrange { at, .. } => match rest {
                "at" => at.as_mut(),
                _ => None,
            },
        }
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

impl Input for EntitiesArray {
    /// `layout.dx`, `layout.dy`, `layout.center.x`, `layout.fill`.
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        let rest = path.strip_prefix("layout.")?;
        match &mut self.layout {
            ArrayLayout::Grid { dx, dy, .. } | ArrayLayout::Line { dx, dy, .. } => match rest {
                "dx" => Some(dx),
                "dy" => Some(dy),
                _ => None,
            },
            ArrayLayout::Polar { center, fill, .. } => match rest {
                "fill" => Some(fill),
                _ => coordinate(center, "center", rest),
            },
            ArrayLayout::Path { spacing, .. } => match rest {
                "spacing" => spacing.as_mut(),
                _ => None,
            },
        }
    }
}

impl Input for EntitiesEdit {
    /// `changes[0].geometry.a.x`, `changes[1].geometry.r`, `changes[0].geometry.pts[1].y`,
    /// `changes[2].geometry.bulges[0]` …
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        // An add's own line weight (docs/adr/0144): `changes[0].lineWeight`.
        if let Some(i) = path
            .strip_prefix("changes[")
            .and_then(|rest| rest.strip_suffix("].lineWeight"))
        {
            return match self.changes.get_mut(i.parse::<usize>().ok()?)? {
                EntityEdit::Add { line_weight, .. } => line_weight.as_mut(),
                _ => None,
            };
        }
        let (i, rest) = path.strip_prefix("changes[")?.split_once("].geometry.")?;
        let geometry = match self.changes.get_mut(i.parse::<usize>().ok()?)? {
            EntityEdit::Update { geometry, .. }
            | EntityEdit::Replace { geometry, .. }
            | EntityEdit::Add { geometry, .. } => geometry,
            EntityEdit::Remove { .. } => return None,
        };
        geometry_number(geometry, rest)
    }
}

impl Input for EntitiesCreate {
    /// `objects[0].geometry.major.x`, `objects[1].geometry.pts[2].y`, `objects[0].geometry.ratio`,
    /// `objects[1].lineWeight`, `objects[0].labelScale` (which the step's input must give) …
    fn number(&mut self, path: &str) -> Option<&mut f64> {
        let (i, rest) = path.strip_prefix("objects[")?.split_once("].")?;
        let object = self.objects.get_mut(i.parse::<usize>().ok()?)?;
        match rest {
            "lineWeight" => object.line_weight.as_mut(),
            "labelScale" => object.label_scale.as_mut(),
            _ => geometry_number(&mut object.geometry, rest.strip_prefix("geometry.")?),
        }
    }
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
            let mut input: $input =
                serde_json::from_value(input).map_err(|e| format!("{at}: girdi okunamadı: {e}"))?;
            put_non_finite(&mut input, step, at)?;
            match op {
                "validate" => {
                    serde_json::to_value($module::validate(&ExecutionContext::new(doc), &input))
                }
                "plan" => serde_json::to_value($module::plan(&ExecutionContext::new(doc), &input)),
                _ => serde_json::to_value($module::execute(&mut ExecutionContext::new(doc), input)),
            }
        }};
    }
    match command {
        CAD_POLYGON_CREATE => run!(polygon, PolygonCreate),
        CAD_LINE_CREATE => run!(line, LineCreate),
        CAD_POLYLINE_CREATE => run!(polyline, PolylineCreate),
        CAD_ENTITIES_DELETE => run!(delete, EntitiesDelete),
        CAD_POINT_CREATE => run!(point, PointCreate),
        CAD_CIRCLE_CREATE => run!(circle, CircleCreate),
        CAD_ARC_CREATE => run!(arc, ArcCreate),
        CAD_ENTITIES_TRANSFORM => run!(transform, EntitiesTransform),
        CAD_ENTITIES_EDIT => run!(edit, EntitiesEdit),
        CAD_ENTITIES_ARRAY => run!(array, EntitiesArray),
        kentos_domain::contracts::CAD_ENTITIES_OFFSET => run!(offset, EntitiesOffset),
        kentos_domain::contracts::CAD_POINTS_CONSTRUCT => {
            run!(points_construct, kentos_domain::contracts::PointsConstruct)
        }
        kentos_domain::contracts::CAD_CURVES_CONSTRUCT => {
            run!(curves_construct, kentos_domain::contracts::CurvesConstruct)
        }
        kentos_domain::contracts::CAD_SNAP_FIND => {
            run!(snap_find, kentos_domain::contracts::SnapFind)
        }
        kentos_domain::contracts::CAD_RECTANGLE_CREATE => {
            run!(rectangle_create, kentos_domain::contracts::RectangleCreate)
        }
        kentos_domain::contracts::CAD_POLYGON_REGULAR => {
            run!(
                polygon_regular,
                kentos_domain::contracts::RegularPolygonCreate
            )
        }
        kentos_domain::contracts::CAD_ENTITIES_VERTEX => {
            run!(entities_vertex, kentos_domain::contracts::EntitiesVertex)
        }
        kentos_domain::contracts::CAD_ENTITIES_PICK => {
            run!(entities_pick, kentos_domain::contracts::EntitiesPick)
        }
        kentos_domain::contracts::CAD_CRS_TRANSFORM => {
            run!(crs_transform, kentos_domain::contracts::CrsTransform)
        }
        kentos_domain::contracts::CAD_ENTITIES_RELATE => {
            run!(entities_relate, kentos_domain::contracts::EntitiesRelate)
        }
        kentos_domain::contracts::CAD_ENTITIES_SELECT => {
            run!(entities_select, kentos_domain::contracts::EntitiesSelect)
        }
        kentos_domain::contracts::CAD_ENTITIES_TRIM => run!(entities_trim, EntitiesTrim),
        kentos_domain::contracts::CAD_ENTITIES_EXTEND => run!(entities_extend, EntitiesTrim),
        kentos_domain::contracts::CAD_ENTITIES_FILLET => run!(entities_fillet, EntitiesFillet),
        kentos_domain::contracts::CAD_ENTITIES_CHAMFER => run!(entities_chamfer, EntitiesChamfer),
        kentos_domain::contracts::CAD_ENTITIES_STRETCH => run!(stretch, EntitiesStretch),
        kentos_domain::contracts::CAD_ENTITIES_BREAK => run!(entities_break, EntitiesBreak),
        kentos_domain::contracts::CAD_ENTITIES_JOIN => run!(entities_join, EntitiesJoin),
        kentos_domain::contracts::CAD_ENTITIES_REVERSE => {
            run!(entities_reverse, kentos_domain::contracts::EntitiesReverse)
        }
        kentos_domain::contracts::CAD_ENTITIES_LENGTHEN => {
            run!(
                entities_lengthen,
                kentos_domain::contracts::EntitiesLengthen
            )
        }
        kentos_domain::contracts::CAD_ENTITIES_DIVIDE => run!(divide, EntitiesDivide),
        kentos_domain::contracts::CAD_ARRAYS_EDIT => {
            run!(arrays_edit, kentos_domain::contracts::ArraysEdit)
        }
        kentos_domain::contracts::CAD_CONSTRAINTS_EDIT => {
            run!(constraints_edit, kentos_domain::contracts::ConstraintsEdit)
        }
        kentos_domain::contracts::CAD_CONSTRAINTS_QUERY => {
            run!(constraints_query, kentos_domain::contracts::ConstraintsQuery)
        }
        kentos_domain::contracts::CAD_PARAMETERS_EDIT => {
            run!(parameters_edit, kentos_domain::contracts::ParametersEdit)
        }
        kentos_domain::contracts::CAD_ENTITIES_EXPLODE => {
            run!(entities_explode, kentos_domain::contracts::EntitiesExplode)
        }
        kentos_domain::contracts::CAD_ENTITIES_MATCH => {
            run!(entities_match, kentos_domain::contracts::EntitiesMatch)
        }
        kentos_domain::contracts::CAD_ENTITIES_CLEANUP => {
            run!(entities_cleanup, kentos_domain::contracts::EntitiesCleanup)
        }
        kentos_domain::contracts::CAD_ENTITIES_COPY => {
            run!(entities_copy, kentos_domain::contracts::EntitiesCopy)
        }
        kentos_domain::contracts::CAD_ENTITIES_PASTE => {
            run!(entities_paste, kentos_domain::contracts::EntitiesPaste)
        }
        kentos_domain::contracts::CAD_DRAWING_AUDIT => {
            run!(drawing_audit, kentos_domain::contracts::DrawingAudit)
        }
        kentos_domain::contracts::CAD_CURVES_EDIT => {
            run!(curves_edit, kentos_domain::contracts::CurvesEdit)
        }
        kentos_domain::contracts::CAD_ENTITIES_BOUNDARY => {
            run!(entities_boundary, EntitiesBoundary)
        }
        kentos_domain::contracts::CAD_ENTITIES_HATCH => run!(entities_hatch, EntitiesHatch),
        kentos_domain::contracts::CAD_DIMENSIONS_REASSOCIATE => {
            run!(dimensions_reassociate, DimensionsReassociate)
        }
        kentos_domain::contracts::CAD_DIMENSIONS_EDIT => {
            run!(dimensions_edit, kentos_domain::contracts::DimensionsEdit)
        }
        kentos_domain::contracts::CAD_DIMENSIONS_QUERY => {
            run!(dimensions_query, kentos_domain::contracts::DimensionsQuery)
        }
        kentos_domain::contracts::CAD_FONTS_REPORT => {
            run!(fonts_report, kentos_domain::contracts::FontsReport)
        }
        kentos_domain::contracts::CAD_LEADERS_EDIT => {
            run!(leaders_edit, kentos_domain::contracts::LeadersEdit)
        }
        kentos_domain::contracts::CAD_HATCHES_EDIT => {
            run!(hatches_edit, kentos_domain::contracts::HatchesEdit)
        }
        kentos_domain::contracts::CAD_TABLES_EDIT => {
            run!(tables_edit, kentos_domain::contracts::TablesEdit)
        }
        kentos_domain::contracts::CAD_FIELDS_EDIT => {
            run!(fields_edit, kentos_domain::contracts::FieldsEdit)
        }
        kentos_domain::contracts::CAD_TEXTS_EDIT => {
            run!(texts_edit, kentos_domain::contracts::TextsEdit)
        }
        kentos_domain::contracts::CAD_TEXTS_CHECK => {
            run!(texts_check, kentos_domain::contracts::TextsCheck)
        }
        kentos_domain::contracts::CAD_DIMENSIONS_CREATE => {
            run!(dimensions_create, DimensionsCreate)
        }
        kentos_domain::contracts::CAD_ENTITIES_CREATE => run!(create, EntitiesCreate),
        kentos_domain::contracts::CAD_NURBS_CREATE => run!(nurbs, NurbsCreate),
        kentos_domain::contracts::CAD_WALLS_CREATE => {
            run!(walls_create, kentos_domain::contracts::WallsCreate)
        }
        kentos_domain::contracts::CAD_WALLS_FROM_LINES => {
            run!(walls_from_lines, kentos_domain::contracts::WallsFromLines)
        }
        kentos_domain::contracts::CAD_WALLS_EDIT => {
            run!(walls_edit, kentos_domain::contracts::WallsEdit)
        }
        kentos_domain::contracts::CAD_WALLS_JOINS => {
            run!(walls_joins, kentos_domain::contracts::WallsJoins)
        }
        kentos_domain::contracts::CAD_OPENINGS_CREATE => {
            run!(openings_create, kentos_domain::contracts::OpeningsCreate)
        }
        kentos_domain::contracts::CAD_OPENINGS_EDIT => {
            run!(openings_edit, kentos_domain::contracts::OpeningsEdit)
        }
        kentos_domain::contracts::CAD_SPACES_CREATE => {
            run!(spaces_create, kentos_domain::contracts::SpacesCreate)
        }
        kentos_domain::contracts::CAD_GRIDS_CREATE => {
            run!(grids_create, kentos_domain::contracts::GridsCreate)
        }
        kentos_domain::contracts::CAD_COLUMNS_CREATE => {
            run!(columns_create, kentos_domain::contracts::ColumnsCreate)
        }
        kentos_domain::contracts::CAD_STAIRS_CREATE => {
            run!(stairs_create, kentos_domain::contracts::StairsCreate)
        }
        kentos_domain::contracts::CAD_RAMPS_CREATE => {
            run!(ramps_create, kentos_domain::contracts::RampsCreate)
        }
        kentos_domain::contracts::CAD_ARCH_EDIT => {
            run!(arch_edit, kentos_domain::contracts::ArchEdit)
        }
        kentos_domain::contracts::CAD_TAGS_CREATE => {
            run!(tags_create, kentos_domain::contracts::TagsCreate)
        }
        kentos_domain::contracts::CAD_TAGS_CHECK => {
            run!(tags_check, kentos_domain::contracts::TagsCheck)
        }
        kentos_domain::contracts::CAD_TAGS_EDIT => {
            run!(tags_edit, kentos_domain::contracts::TagsEdit)
        }
        kentos_domain::contracts::CAD_VIEWMARKS_CREATE => {
            run!(viewmarks_create, kentos_domain::contracts::ViewMarksCreate)
        }
        kentos_domain::contracts::CAD_VIEWMARKS_EDIT => {
            run!(viewmarks_edit, kentos_domain::contracts::ViewMarksEdit)
        }
        kentos_domain::contracts::CAD_VIEWMARKS_REFERENCES => {
            run!(viewmarks_references, kentos_domain::contracts::ViewMarksReferences)
        }
        kentos_domain::contracts::CAD_SPACES_FINISH => {
            run!(spaces_finish, kentos_domain::contracts::SpacesFinish)
        }
        kentos_domain::contracts::CAD_TEMPLATES_APPLY => {
            run!(templates_apply, kentos_domain::contracts::TemplatesApply)
        }
        kentos_domain::contracts::CAD_SHEETS_SET => {
            run!(sheets_set, kentos_domain::contracts::SheetsSet)
        }
        kentos_domain::contracts::CAD_XREFS_EDIT => {
            run!(xrefs_edit, kentos_domain::contracts::XrefsEdit)
        }
        kentos_domain::contracts::CAD_XREFS_LIST => {
            run!(xrefs_list, kentos_domain::contracts::XrefsList)
        }
        kentos_domain::contracts::CAD_ASSETS_CHECK => {
            run!(assets_check, kentos_domain::contracts::AssetsCheck)
        }
        kentos_domain::contracts::CAD_INSERTS_CLIP => {
            run!(inserts_clip, kentos_domain::contracts::InsertsClip)
        }
        kentos_domain::contracts::CAD_IMAGES_EDIT => {
            run!(images_edit, kentos_domain::contracts::ImagesEdit)
        }
        kentos_domain::contracts::CAD_RASTERS_MOSAIC => {
            run!(rasters_mosaic, kentos_domain::contracts::RastersMosaic)
        }
        kentos_domain::contracts::CAD_STAC_SEARCH => {
            run!(stac_search, kentos_domain::contracts::StacSearch)
        }
        kentos_domain::contracts::CAD_STAC_PREPARE => {
            run!(stac_prepare, kentos_domain::contracts::StacPrepare)
        }
        kentos_domain::contracts::CAD_RASTERS_GEOREF => {
            run!(rasters_georef, kentos_domain::contracts::RastersGeoref)
        }
        kentos_domain::contracts::CAD_SCHEDULES_CREATE => {
            run!(schedules_create, kentos_domain::contracts::SchedulesCreate)
        }
        kentos_domain::contracts::CAD_SCHEDULES_EDIT => {
            run!(schedules_edit, kentos_domain::contracts::SchedulesEdit)
        }
        kentos_domain::contracts::CAD_SPACES_EDIT => {
            run!(spaces_edit, kentos_domain::contracts::SpacesEdit)
        }
        kentos_domain::contracts::CAD_SPACES_QUANTITIES => {
            run!(spaces_quantities, kentos_domain::contracts::SpacesQuantities)
        }
        kentos_domain::contracts::CAD_ENTITIES_ORDER => run!(entities_order, EntitiesOrder),
        kentos_domain::contracts::CAD_ANNOTATIONS_SCALES => run!(annotations_scales, AnnotationsScales),
        CAD_ENTITIES_SET => run!(set, EntitiesSetProperties),
        CAD_BLOCKS_DEFINE => run!(blocks_define, BlocksDefine),
        CAD_BLOCKS_EDIT => run!(blocks_edit, BlocksEdit),
        kentos_domain::contracts::CAD_BLOCKS_INSERT => {
            run!(blocks_insert, kentos_domain::contracts::BlocksInsert)
        }
        kentos_domain::contracts::FILE_IMPORT_BLOCK => {
            run!(file_import_block, kentos_domain::contracts::ImportBlock)
        }
        kentos_domain::contracts::CAD_BLOCKS_REPLACE => {
            run!(blocks_replace, kentos_domain::contracts::BlocksReplace)
        }
        kentos_domain::contracts::CAD_BLOCKS_QUERY => {
            run!(blocks_query, kentos_domain::contracts::BlocksQuery)
        }
        kentos_domain::contracts::CAD_BLOCKS_PARAMETERS => {
            run!(block_params, kentos_domain::contracts::BlocksParameters)
        }
        kentos_domain::contracts::CAD_BLOCKS_PROXY => {
            run!(blocks_proxy, kentos_domain::contracts::BlocksProxy)
        }
        kentos_domain::contracts::CAD_BLOCKS_FREEZE => {
            run!(blocks_freeze, kentos_domain::contracts::BlocksFreeze)
        }
        kentos_domain::contracts::CAD_LAYERS_SERVICE => run!(layers_service, LayersService),
        kentos_domain::contracts::CAD_NETWORK_DEFINE => run!(network_define, NetworkDefine),
        kentos_domain::contracts::CAD_ADDRESSES_DEFINE => run!(address_define, AddressesDefine),
        kentos_domain::contracts::CAD_LAYERS_TIME => run!(layers_time, LayersTime),
        kentos_domain::contracts::CAD_SCENARIOS_EDIT => run!(scenarios_edit, ScenariosEdit),
        kentos_domain::contracts::CAD_LAYERS_FILTER => run!(layers_filter, LayersFilter),
        kentos_domain::contracts::CAD_LAYERS_INFO => run!(layers_info, LayersInfo),
        kentos_domain::contracts::CAD_LAYERS_TABLE => run!(layers_table, LayersTable),
        kentos_domain::contracts::CAD_LAYERS_SUBTYPES => run!(layers_subtypes, LayersSubtypes),
        kentos_domain::contracts::CAD_LAYERS_CONTINGENTS => run!(layers_contingents, LayersContingents),
        kentos_domain::contracts::CAD_LAYERS_RULES => run!(layers_rules, LayersRules),
        kentos_domain::contracts::CAD_LAYERS_COMPOSE => run!(layers_compose, LayersCompose),
        kentos_domain::contracts::CAD_LAYERS_LABELS => run!(layers_labels, LayersLabels),
        kentos_domain::contracts::CAD_LABELS_PIN => run!(labels_pin, LabelsPin),
        kentos_domain::contracts::CAD_LAYERS_RENDERER => run!(layers_renderer, LayersRenderer),
        kentos_domain::contracts::CAD_LAYERS_EDIT => run!(layers_edit, LayersEdit),
        kentos_domain::contracts::CAD_LAYERS_SET => run!(layers_set, LayersSet),
        kentos_domain::contracts::CAD_LAYERS_STATES => run!(layers_states, LayersStates),
        kentos_domain::contracts::CAD_LAYERS_FIND => run!(layers_find, LayersFind),
        kentos_domain::contracts::CAD_FIELDS_USAGE => run!(fields_usage, FieldsUsage),
        kentos_domain::contracts::CAD_LAYERS_TRANSLATE => run!(layers_translate, LayersTranslate),
        kentos_domain::contracts::CAD_STANDARDS_CHECK => run!(standards_check, StandardsCheck),
        kentos_domain::contracts::CAD_STANDARDS_APPLY => run!(standards_apply, StandardsApply),
        kentos_domain::contracts::CAD_LINE_TYPES_EDIT => run!(line_types_edit, LineTypesEdit),
        kentos_domain::contracts::CAD_HATCH_PATTERNS_EDIT => run!(hatch_patterns_edit, HatchPatternsEdit),
        kentos_domain::contracts::CAD_LIBRARY_EDIT => run!(library_edit, LibraryEdit),
        kentos_domain::contracts::CAD_STYLES_IMPACT => run!(styles_impact, StylesImpact),
        kentos_domain::contracts::CAD_STYLES_EDIT => run!(styles_edit, StylesEdit),
        kentos_domain::contracts::CAD_SETTINGS_SET => run!(settings_set, SettingsSet),
        kentos_domain::contracts::CAD_UCS_SET => run!(ucs_set, UcsSet),
        kentos_domain::contracts::CAD_VIEWS_EDIT => run!(views_edit, ViewsEdit),
        kentos_domain::contracts::CAD_FORMS_EDIT => run!(forms_edit, FormsEdit),
        kentos_domain::contracts::CAD_FORMS_VALIDATE => run!(forms_validate, FormsValidate),
        kentos_domain::contracts::CAD_FORMS_SUBMIT => run!(forms_submit, FormsSubmit),
        kentos_domain::contracts::CAD_FORMS_FILL => run!(forms_fill, FormsFill),
        kentos_domain::contracts::CAD_FORMS_AUDIT => run!(forms_audit, FormsAudit),
        kentos_domain::contracts::CAD_FORMS_TEST => run!(forms_test, FormsTest),
        kentos_domain::contracts::CAD_FORMS_COMPARE => run!(forms_compare, FormsCompare),
        kentos_domain::contracts::CAD_FORMS_TRANSITION => run!(forms_transition, FormsTransition),
        kentos_domain::contracts::CAD_DOMAINS_EDIT => run!(domains_edit, DomainsEdit),
        kentos_domain::contracts::CAD_DOMAINS_EXPORT => run!(domains_export, DomainsExport),
        kentos_domain::contracts::CAD_FIELDS_MIGRATE => run!(fields_migrate, FieldsMigrate),
        kentos_domain::contracts::CAD_ATTACHMENTS_INSPECT => run!(attachments_inspect, AttachmentsInspect),
        kentos_domain::contracts::CAD_SCHEMA_DESCRIBE => run!(schema_describe, SchemaDescribe),
        kentos_domain::contracts::CAD_FORMS_EXPORT => run!(forms_export, FormsExport),
        kentos_domain::contracts::CAD_FORMS_IMPORT => run!(forms_import, FormsImport),
        kentos_domain::contracts::CAD_FORMS_RECOMPUTE => run!(forms_recompute, FormsRecompute),
        kentos_domain::contracts::CAD_RECORDS_CREATE => run!(records, RecordsCreate),
        kentos_domain::contracts::CAD_RECORDS_DUPLICATE => run!(records_duplicate, RecordsDuplicate),
        kentos_domain::contracts::CAD_RELATIONS_EDIT => run!(relations_edit, RelationsEdit),
        kentos_domain::contracts::CAD_JOINS_EDIT => run!(joins_edit, JoinsEdit),
        kentos_domain::contracts::CAD_JOINS_QUERY => run!(joins_query, JoinsQuery),
        kentos_domain::contracts::CAD_UNITS_CHANGE => run!(units_change, UnitsChange),
        kentos_domain::contracts::FILE_IMPORT_DXF => run!(file_import, ImportDxf),
        kentos_domain::contracts::FILE_IMPORT_GPKG => run!(file_import_gpkg, ImportGpkg),
        kentos_domain::contracts::FILE_IMPORT_CSV => {
            run!(file_import_csv, kentos_domain::contracts::ImportCsv)
        }
        kentos_domain::contracts::CAD_GIS_DERIVATION => {
            run!(gis_derivation, kentos_domain::contracts::GisDerivation)
        }
        kentos_domain::contracts::FILE_IMPORT_SLD => {
            run!(sld, kentos_domain::contracts::ImportSld)
        }
        kentos_domain::contracts::FILE_IMPORT_PDF => run!(file_import_pdf, ImportPdf),
        kentos_domain::contracts::CAD_SOURCES_DESCRIBE => run!(sources_describe, kentos_domain::contracts::SourcesDescribe),
        kentos_domain::contracts::CAD_SOURCES_EDIT => run!(sources_edit, kentos_domain::contracts::SourcesEdit),
        kentos_domain::contracts::CAD_SOURCES_SYNC => run!(sources_sync, kentos_domain::contracts::SourcesSync),
        kentos_domain::contracts::CAD_SOURCES_RELATE => run!(sources_relate, kentos_domain::contracts::SourcesRelate),
        kentos_domain::contracts::CAD_RECORDS_PAGE => run!(records_page, kentos_domain::contracts::RecordsPage),
        kentos_domain::contracts::CAD_FIELDS_WRITE => run!(fields_write, kentos_domain::contracts::FieldsWrite),
        kentos_domain::contracts::CAD_PROCESSING_TOOLS => run!(processing_tools, kentos_domain::contracts::ProcessingTools),
        kentos_domain::contracts::CAD_PROCESSING_RUN => run!(processing_run, kentos_domain::contracts::ProcessingRun),
        kentos_domain::contracts::CAD_PROCESSING_BATCH => run!(processing_batch, kentos_domain::contracts::ProcessingBatch),
        kentos_domain::contracts::CAD_SHEETS_EDIT => run!(sheets_edit, SheetsEdit),
        kentos_domain::contracts::CAD_CHANGESET_APPLY => run!(changeset, ChangesetApply),
        other => return Err(format!("{at}: {other} için koşucu yok")),
    }
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
        return if close(&got, &want, within) {
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
    nodes: &'a [kentos_domain::contracts::LayerNode],
    id: &str,
    parent: Option<&'a str>,
) -> Option<(&'a kentos_domain::contracts::LayerNode, Option<&'a str>)> {
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

/// Every node's id of a layer tree.
fn layer_ids(nodes: &[kentos_domain::contracts::LayerNode]) -> std::collections::HashSet<String> {
    let mut out = std::collections::HashSet::new();
    let mut stack: Vec<&kentos_domain::contracts::LayerNode> = nodes.iter().collect();
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
            ran?
        }
        "validate" | "plan" | "execute" => run_command(
            step["command"].as_str().unwrap_or(command),
            doc,
            state,
            step,
            at,
        )?,
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
                        expect_same(
                            got.get(k).unwrap_or(&Value::Null),
                            &v,
                            &format!("nesne {id}.{k}"),
                            at,
                        )?;
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
                let got = serde_json::to_value(&doc.constraints().parameters).unwrap_or(Value::Null);
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
                        &setup_paths(v),
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
                expect_same(&got, want, "adres verileri", at)?;
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
        Value::String(text) if (text.contains("$fixture:") || text.contains("$tmp:")) => Value::String(with_fixtures(text)),
        Value::Array(items) => Value::Array(items.iter().map(setup_paths).collect()),
        Value::Object(fields) => Value::Object(fields.iter().map(|(k, v)| (k.clone(), setup_paths(v))).collect()),
        other => other.clone(),
    }
}

fn run_case(command: &str, setup: &Value, case: &Value, at: &str) -> Outcome<()> {
    // İşlemler's tools, as every host that has them registers them (docs/adr/0508).
    kentos_processing::command::register();
    // STAC searches are answered from the recorded folder (docs/adr/0528 §2): no network in a shared case.
    stac_host::register(Box::new(stac_host::Recorded::open(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/services/v1/stac"),
    )));
    let setup = &setup_paths(setup);
    let snapshot = DocumentSnapshotV1::from_json(&setup.to_string())
        .map_err(|e| format!("{at}: kurulum: {e}"))?;
    let mut doc = Document::from_snapshot(snapshot).map_err(|e| format!("{at}: kurulum: {e}"))?;
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
    std::env::var("KENTOS_CASES_ONLY")
        .ok()
        .map(|v| v.split(',').map(str::to_owned).filter(|w| !w.is_empty()).collect())
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
    assert!(!files.is_empty(), "{}: no files", dir.display());
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
        assert!(
            DESKTOP_COMMANDS
                .iter()
                .any(|(id, v)| (*id, u64::from(*v)) == command),
            "{file}: {command:?} is not a desktop command"
        );
        commands.push(command.0.to_owned());
        let listed = fixture["cases"].as_array().expect("cases");
        assert!(listed.len() >= least, "{file}: {} cases", listed.len());
        for case in listed {
            cases += 1;
            let name = case["name"].as_str().unwrap_or("?");
            let setup = case.get("setup").unwrap_or(&fixture["setup"]);
            if let Err(problem) = run_case(command.0, setup, case, &format!("{file} › {name}")) {
                problems.push(problem);
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
