//! The product commands by name (docs/adr/0013, 0253 §4): each command's
//! input read as its own type, run by its handler, its `CommandResult`
//! written back as the catalog's JSON. The protocol (`call.rs`), the change
//! set (`changeset.rs`), the headless host, Python and MCP reach a command
//! through here, so they all run the same handler the tools run.

use kentos_contracts::{
    AddressesDefine, ArcCreate, ArraysEdit, BlocksDefine, BlocksEdit, CallOp, ChangesetApply,
    CircleCreate, CurvesConstruct, DimensionsCreate, DimensionsReassociate, EntitiesArray,
    EntitiesBoundary, EntitiesBreak, EntitiesChamfer, EntitiesCreate, EntitiesDelete,
    EntitiesDivide, EntitiesEdit, EntitiesFillet, EntitiesHatch, EntitiesJoin, EntitiesLengthen,
    EntitiesOffset, EntitiesOrder, EntitiesPick, EntitiesReverse, EntitiesSelect,
    EntitiesSetProperties, EntitiesStretch, EntitiesTransform, EntitiesTrim, EntitiesVertex,
    ImportDxf, LabelsPin, LayersCompose, LayersEdit, LayersFilter, LayersInfo, LayersLabels,
    LayersRenderer, LayersService, LayersSet, LayersStates, LayersTime, LineCreate, NetworkDefine,
    NurbsCreate, PointCreate, PointsConstruct, PolygonCreate, PolylineCreate, RectangleCreate,
    RegularPolygonCreate, ScenariosEdit, SettingsSet, SheetsEdit, SnapFind, StylesEdit, UcsSet,
    UnitsChange, ViewsEdit, codes,
};
use serde_json::Value;

use crate::{
    DESKTOP_COMMANDS, ExecutionContext, address_define, arc, array, arrays_edit, blocks_define,
    blocks_edit, changeset, circle, create, curves_construct, delete, dimensions_create,
    dimensions_reassociate, divide, edit, entities_boundary, entities_break, entities_chamfer,
    entities_extend, entities_fillet, entities_hatch, entities_join, entities_lengthen,
    entities_order, entities_pick, entities_reverse, entities_select, entities_trim,
    entities_vertex, file_import, labels_pin, layers_compose, layers_edit, layers_filter,
    layers_info, layers_labels, layers_renderer, layers_service, layers_set, layers_states,
    layers_time, line, network_define, nurbs, offset, point, points_construct, polygon,
    polygon_regular, polyline, rectangle_create, scenarios_edit, set, settings_set, sheets_edit,
    snap_find, stretch, styles_edit, transform, ucs_set, units_change, views_edit,
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
            kentos_contracts::CAD_LAYERS_LABELS => $then!(layers_labels, LayersLabels),
            kentos_contracts::CAD_LABELS_PIN => $then!(labels_pin, LabelsPin),
            kentos_contracts::CAD_LAYERS_RENDERER => $then!(layers_renderer, LayersRenderer),
            kentos_contracts::CAD_LAYERS_EDIT => $then!(layers_edit, LayersEdit),
            kentos_contracts::CAD_LAYERS_SET => $then!(layers_set, LayersSet),
            kentos_contracts::CAD_LAYERS_STATES => $then!(layers_states, LayersStates),
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
