//! The product commands by name (docs/adr/0013, 0253 §4): each command's
//! input read as its own type, run by its handler, its `CommandResult`
//! written back as the catalog's JSON. The protocol (`call.rs`), the change
//! set (`changeset.rs`), the headless host, Python and MCP reach a command
//! through here, so they all run the same handler the tools run.

use kentos_contracts::{
    AddressesDefine, ArcCreate, ArraysEdit, BlocksDefine, BlocksEdit, CallOp, ChangesetApply,
    CircleCreate, CurvesConstruct, CurvesEdit, DimensionsCreate, DimensionsReassociate,
    DrawingAudit, EntitiesArray, EntitiesBoundary, EntitiesBreak, EntitiesChamfer, EntitiesCleanup,
    EntitiesCopy, EntitiesCreate, EntitiesDelete, EntitiesDivide, EntitiesEdit, EntitiesExplode,
    EntitiesFillet, EntitiesHatch, EntitiesJoin, EntitiesLengthen, EntitiesMatch, EntitiesOffset,
    EntitiesOrder, EntitiesPaste, EntitiesPick, EntitiesReverse, EntitiesSelect,
    EntitiesSetProperties, EntitiesStretch, EntitiesTransform, EntitiesTrim, EntitiesVertex,
    FieldsUsage, ImportDxf, LabelsPin, LayersCompose, LayersEdit, LayersFilter, LayersFind, LayersInfo, LayersContingents, LayersRules, LayersSubtypes, LayersTable,
    LayersLabels, LayersRenderer, LayersService, LayersSet, LayersStates, LayersTime,
    LayersTranslate, LibraryEdit, LineCreate, LineTypesEdit, NetworkDefine, NurbsCreate,
    PointCreate, PointsConstruct, PolygonCreate, PolylineCreate, RectangleCreate,
    RegularPolygonCreate, ScenariosEdit, SettingsSet, SheetsEdit, SnapFind, StandardsApply,
    StandardsCheck, StylesEdit, StylesImpact, UcsSet, UnitsChange, ViewsEdit, codes,
};
use kentos_contracts::{AttachmentsInspect, SchemaDescribe, BlocksInsert, BlocksParameters, BlocksQuery, BlocksReplace, DomainsEdit, DomainsExport, FieldsMigrate, FormsExport, FormsImport, ImportBlock};
use crate::{
    attachments_inspect, schema_describe, block_params, blocks_query, blocks_replace, domains_edit, domains_export, fields_migrate, file_import_block, forms_export,
    forms_import,
};
use serde_json::Value;

use crate::standards::{apply as standards_apply, check as standards_check};
use crate::{
    DESKTOP_COMMANDS, ExecutionContext, address_define, arc, array, arrays_edit, blocks_define,
    blocks_edit, changeset, circle, create, curves_construct, curves_edit, delete,
    dimensions_create, dimensions_reassociate, divide, drawing_audit, edit, entities_boundary,
    entities_break, entities_chamfer, entities_cleanup, entities_copy, entities_explode,
    entities_extend, entities_fillet, entities_hatch, entities_join, entities_lengthen,
    entities_match, entities_order, entities_paste, entities_pick, entities_reverse,
    entities_select, entities_trim, entities_vertex, file_import, labels_pin, layers_compose,
    fields_usage, layers_edit, layers_filter, layers_find, layers_info, layers_labels, layers_renderer, layers_contingents, layers_rules, layers_subtypes, layers_table,
    layers_service, layers_set, layers_states, layers_time, layers_translate, library_edit, line,
    line_types_edit, network_define, nurbs, offset, point, points_construct, polygon,
    polygon_regular, polyline, rectangle_create, scenarios_edit, set, settings_set, sheets_edit,
    snap_find, stretch, styles_edit, styles_impact, transform, ucs_set, units_change, views_edit,
};
use crate::blocks_insert;
use crate::{
    forms_audit, forms_compare, forms_edit, forms_recompute, forms_test, forms_transition, forms_fill, forms_submit, forms_validate, records, records_duplicate, relations_edit,
};
use kentos_contracts::{
    FormsAudit, FormsCompare, FormsEdit, FormsRecompute, FormsTest, FormsTransition, FormsFill, FormsSubmit, FormsValidate, RecordsCreate, RecordsDuplicate, RelationsEdit,
};

/// Why a command was not run: not a refusal of the command (that is its
/// `CommandResult`), but a command this host does not run or input that is
/// not its type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unrun {
    /// `unknown_command`, `invalid_input`.
    pub code: &'static str,
    pub message: String,
}

/// The version of `command` this host runs; none for a command it does not run.
pub fn version_of(command: &str) -> Option<u32> {
    DESKTOP_COMMANDS
        .iter()
        .find(|(name, _)| *name == command)
        .map(|(_, v)| *v)
}

fn unknown(command: &str) -> Unrun {
    Unrun {
        code: codes::UNKNOWN_COMMAND,
        message: format!("“{command}” bu çizimde çalışan bir komut değil."),
    }
}

fn unreadable(command: &str, e: &serde_json::Error) -> Unrun {
    Unrun {
        code: codes::INVALID_INPUT,
        message: format!("{command} girdisi okunamadı: {e}. Kataloğun girdi şemasına bakın."),
    }
}

fn unwritable(command: &str, e: &serde_json::Error) -> Unrun {
    Unrun {
        code: codes::INVALID_INPUT,
        message: format!("{command} sonucu JSON'a yazılamadı: {e}"),
    }
}

/// Calls `$then!(module, InputType)` for `command`, or answers that it is unknown.
macro_rules! by_command {
    ($command:expr, $then:ident) => {
        match $command {
            kentos_contracts::CAD_POLYGON_CREATE => $then!(polygon, PolygonCreate),
            kentos_contracts::CAD_LINE_CREATE => $then!(line, LineCreate),
            kentos_contracts::CAD_POLYLINE_CREATE => $then!(polyline, PolylineCreate),
            kentos_contracts::CAD_ENTITIES_DELETE => $then!(delete, EntitiesDelete),
            kentos_contracts::CAD_POINT_CREATE => $then!(point, PointCreate),
            kentos_contracts::CAD_CIRCLE_CREATE => $then!(circle, CircleCreate),
            kentos_contracts::CAD_ARC_CREATE => $then!(arc, ArcCreate),
            kentos_contracts::CAD_ENTITIES_TRANSFORM => $then!(transform, EntitiesTransform),
            kentos_contracts::CAD_ENTITIES_EDIT => $then!(edit, EntitiesEdit),
            kentos_contracts::CAD_ENTITIES_ARRAY => $then!(array, EntitiesArray),
            kentos_contracts::CAD_ENTITIES_CREATE => $then!(create, EntitiesCreate),
            kentos_contracts::CAD_ENTITIES_SET => $then!(set, EntitiesSetProperties),
            kentos_contracts::CAD_BLOCKS_DEFINE => $then!(blocks_define, BlocksDefine),
            kentos_contracts::CAD_BLOCKS_EDIT => $then!(blocks_edit, BlocksEdit),
            kentos_contracts::CAD_LAYERS_SERVICE => $then!(layers_service, LayersService),
            kentos_contracts::CAD_NETWORK_DEFINE => $then!(network_define, NetworkDefine),
            kentos_contracts::CAD_ENTITIES_OFFSET => $then!(offset, EntitiesOffset),
            kentos_contracts::CAD_POINTS_CONSTRUCT => $then!(points_construct, PointsConstruct),
            kentos_contracts::CAD_CURVES_CONSTRUCT => $then!(curves_construct, CurvesConstruct),
            kentos_contracts::CAD_SNAP_FIND => $then!(snap_find, SnapFind),
            kentos_contracts::CAD_ENTITIES_SELECT => $then!(entities_select, EntitiesSelect),
            kentos_contracts::CAD_ENTITIES_PICK => $then!(entities_pick, EntitiesPick),
            kentos_contracts::CAD_ENTITIES_VERTEX => $then!(entities_vertex, EntitiesVertex),
            kentos_contracts::CAD_RECTANGLE_CREATE => $then!(rectangle_create, RectangleCreate),
            kentos_contracts::CAD_POLYGON_REGULAR => $then!(polygon_regular, RegularPolygonCreate),
            kentos_contracts::CAD_NURBS_CREATE => $then!(nurbs, NurbsCreate),
            kentos_contracts::CAD_ENTITIES_ORDER => $then!(entities_order, EntitiesOrder),
            kentos_contracts::CAD_UCS_SET => $then!(ucs_set, UcsSet),
            kentos_contracts::CAD_VIEWS_EDIT => $then!(views_edit, ViewsEdit),
            kentos_contracts::CAD_FORMS_EDIT => $then!(forms_edit, FormsEdit),
            kentos_contracts::CAD_FORMS_VALIDATE => $then!(forms_validate, FormsValidate),
            kentos_contracts::CAD_FORMS_SUBMIT => $then!(forms_submit, FormsSubmit),
            kentos_contracts::CAD_FORMS_FILL => $then!(forms_fill, FormsFill),
            kentos_contracts::CAD_FORMS_AUDIT => $then!(forms_audit, FormsAudit),
            kentos_contracts::CAD_FORMS_TEST => $then!(forms_test, FormsTest),
            kentos_contracts::CAD_FORMS_COMPARE => $then!(forms_compare, FormsCompare),
            kentos_contracts::CAD_FORMS_TRANSITION => $then!(forms_transition, FormsTransition),
            kentos_contracts::CAD_DOMAINS_EDIT => $then!(domains_edit, DomainsEdit),
            kentos_contracts::CAD_DOMAINS_EXPORT => $then!(domains_export, DomainsExport),
            kentos_contracts::CAD_FIELDS_MIGRATE => $then!(fields_migrate, FieldsMigrate),
            kentos_contracts::CAD_ATTACHMENTS_INSPECT => $then!(attachments_inspect, AttachmentsInspect),
            kentos_contracts::CAD_SCHEMA_DESCRIBE => $then!(schema_describe, SchemaDescribe),
            kentos_contracts::CAD_FORMS_EXPORT => $then!(forms_export, FormsExport),
            kentos_contracts::CAD_FORMS_IMPORT => $then!(forms_import, FormsImport),
            kentos_contracts::CAD_FORMS_RECOMPUTE => $then!(forms_recompute, FormsRecompute),
            kentos_contracts::CAD_RECORDS_CREATE => $then!(records, RecordsCreate),
            kentos_contracts::CAD_RECORDS_DUPLICATE => $then!(records_duplicate, RecordsDuplicate),
            kentos_contracts::CAD_RELATIONS_EDIT => $then!(relations_edit, RelationsEdit),
            kentos_contracts::CAD_ENTITIES_TRIM => $then!(entities_trim, EntitiesTrim),
            kentos_contracts::CAD_ENTITIES_EXTEND => $then!(entities_extend, EntitiesTrim),
            kentos_contracts::CAD_ENTITIES_FILLET => $then!(entities_fillet, EntitiesFillet),
            kentos_contracts::CAD_ENTITIES_CHAMFER => $then!(entities_chamfer, EntitiesChamfer),
            kentos_contracts::CAD_ENTITIES_STRETCH => $then!(stretch, EntitiesStretch),
            kentos_contracts::CAD_ENTITIES_BREAK => $then!(entities_break, EntitiesBreak),
            kentos_contracts::CAD_ENTITIES_JOIN => $then!(entities_join, EntitiesJoin),
            kentos_contracts::CAD_ENTITIES_REVERSE => $then!(entities_reverse, EntitiesReverse),
            kentos_contracts::CAD_ENTITIES_LENGTHEN => $then!(entities_lengthen, EntitiesLengthen),
            kentos_contracts::CAD_ARRAYS_EDIT => $then!(arrays_edit, ArraysEdit),
            kentos_contracts::CAD_ENTITIES_EXPLODE => $then!(entities_explode, EntitiesExplode),
            kentos_contracts::CAD_CURVES_EDIT => $then!(curves_edit, CurvesEdit),
            kentos_contracts::CAD_ENTITIES_MATCH => $then!(entities_match, EntitiesMatch),
            kentos_contracts::CAD_ENTITIES_CLEANUP => $then!(entities_cleanup, EntitiesCleanup),
            kentos_contracts::CAD_DRAWING_AUDIT => $then!(drawing_audit, DrawingAudit),
            kentos_contracts::CAD_ENTITIES_COPY => $then!(entities_copy, EntitiesCopy),
            kentos_contracts::CAD_ENTITIES_PASTE => $then!(entities_paste, EntitiesPaste),
            kentos_contracts::CAD_ENTITIES_DIVIDE => $then!(divide, EntitiesDivide),
            kentos_contracts::CAD_ENTITIES_BOUNDARY => $then!(entities_boundary, EntitiesBoundary),
            kentos_contracts::CAD_ENTITIES_HATCH => $then!(entities_hatch, EntitiesHatch),
            kentos_contracts::CAD_DIMENSIONS_REASSOCIATE => {
                $then!(dimensions_reassociate, DimensionsReassociate)
            }
            kentos_contracts::CAD_DIMENSIONS_CREATE => $then!(dimensions_create, DimensionsCreate),
            kentos_contracts::CAD_ADDRESSES_DEFINE => $then!(address_define, AddressesDefine),
            kentos_contracts::CAD_LAYERS_TIME => $then!(layers_time, LayersTime),
            kentos_contracts::CAD_SCENARIOS_EDIT => $then!(scenarios_edit, ScenariosEdit),
            kentos_contracts::CAD_LAYERS_FILTER => $then!(layers_filter, LayersFilter),
            kentos_contracts::CAD_LAYERS_COMPOSE => $then!(layers_compose, LayersCompose),
            kentos_contracts::CAD_LAYERS_INFO => $then!(layers_info, LayersInfo),
            kentos_contracts::CAD_LAYERS_TABLE => $then!(layers_table, LayersTable),
            kentos_contracts::CAD_LAYERS_SUBTYPES => $then!(layers_subtypes, LayersSubtypes),
            kentos_contracts::CAD_LAYERS_CONTINGENTS => {
                $then!(layers_contingents, LayersContingents)
            }
            kentos_contracts::CAD_LAYERS_RULES => $then!(layers_rules, LayersRules),
            kentos_contracts::CAD_LAYERS_LABELS => $then!(layers_labels, LayersLabels),
            kentos_contracts::CAD_LABELS_PIN => $then!(labels_pin, LabelsPin),
            kentos_contracts::CAD_LAYERS_RENDERER => $then!(layers_renderer, LayersRenderer),
            kentos_contracts::CAD_LAYERS_EDIT => $then!(layers_edit, LayersEdit),
            kentos_contracts::CAD_LAYERS_SET => $then!(layers_set, LayersSet),
            kentos_contracts::CAD_LAYERS_STATES => $then!(layers_states, LayersStates),
            kentos_contracts::CAD_LAYERS_FIND => $then!(layers_find, LayersFind),
            kentos_contracts::CAD_FIELDS_USAGE => $then!(fields_usage, FieldsUsage),
            kentos_contracts::CAD_LAYERS_TRANSLATE => $then!(layers_translate, LayersTranslate),
            kentos_contracts::CAD_STANDARDS_CHECK => $then!(standards_check, StandardsCheck),
            kentos_contracts::CAD_STANDARDS_APPLY => $then!(standards_apply, StandardsApply),
            kentos_contracts::CAD_LINE_TYPES_EDIT => $then!(line_types_edit, LineTypesEdit),
            kentos_contracts::CAD_LIBRARY_EDIT => $then!(library_edit, LibraryEdit),
            kentos_contracts::CAD_STYLES_IMPACT => $then!(styles_impact, StylesImpact),
            kentos_contracts::CAD_BLOCKS_INSERT => $then!(blocks_insert, BlocksInsert),
            kentos_contracts::CAD_BLOCKS_REPLACE => $then!(blocks_replace, BlocksReplace),
            kentos_contracts::CAD_BLOCKS_QUERY => $then!(blocks_query, BlocksQuery),
            kentos_contracts::CAD_BLOCKS_PARAMETERS => $then!(block_params, BlocksParameters),
            kentos_contracts::FILE_IMPORT_BLOCK => $then!(file_import_block, ImportBlock),
            kentos_contracts::CAD_STYLES_EDIT => $then!(styles_edit, StylesEdit),
            kentos_contracts::CAD_SETTINGS_SET => $then!(settings_set, SettingsSet),
            kentos_contracts::CAD_UNITS_CHANGE => $then!(units_change, UnitsChange),
            kentos_contracts::FILE_IMPORT_DXF => $then!(file_import, ImportDxf),
            kentos_contracts::CAD_SHEETS_EDIT => $then!(sheets_edit, SheetsEdit),
            kentos_contracts::CAD_CHANGESET_APPLY => $then!(changeset, ChangesetApply),
            other => Err(unknown(other)),
        }
    };
}

/// `input` read as `command`'s input and written back: its defaults filled
/// in, its fields in the type's form (what the journal keeps and a
/// request's fingerprint is made of).
pub fn normalize(command: &str, input: Value) -> Result<Value, Unrun> {
    macro_rules! normal {
        ($module:ident, $input:ty) => {{
            let typed: $input =
                serde_json::from_value(input).map_err(|e| unreadable(command, &e))?;
            serde_json::to_value(&typed).map_err(|e| unwritable(command, &e))
        }};
    }
    by_command!(command, normal)
}

/// Runs `op` of `command` with `input` in `cx`: the command's whole answer
/// as the catalog writes it (`status`-tagged).
pub fn run(
    cx: &mut ExecutionContext<'_>,
    command: &str,
    op: CallOp,
    input: Value,
) -> Result<Value, Unrun> {
    macro_rules! go {
        ($module:ident, $input:ty) => {{
            let typed: $input =
                serde_json::from_value(input).map_err(|e| unreadable(command, &e))?;
            match op {
                CallOp::Validate => serde_json::to_value($module::validate(cx, &typed)),
                CallOp::Plan => serde_json::to_value($module::plan(cx, &typed)),
                CallOp::Execute => serde_json::to_value($module::execute(cx, typed)),
            }
            .map_err(|e| unwritable(command, &e))
        }};
    }
    by_command!(command, go)
}
