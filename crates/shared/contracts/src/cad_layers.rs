//! The service layers' command of the product command catalog
//! (docs/adr/0208 §15): `cad.layers.service` adds a layer drawn from a map
//! service, adds a layer whose objects a service gives (its feed), changes
//! one, or removes a service layer; the project's connections (without
//! their secrets) go in with it. Each is one undo step. Harita servisi,
//! Servisten veri al and the layers' menus write through it on the web
//! (`apps/web/src/product`) and on the desktop (`crates/native/application`);
//! both pass the shared cases in `fixtures/commands/v1`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::fields::LayerField;
use crate::layer::LayerNode;
use crate::service::{FeatureFeed, ServiceConnection, ServiceLayer};

/// Adds, changes or removes a service layer.
pub const CAD_LAYERS_SERVICE: &str = "cad.layers.service";
pub const CAD_LAYERS_SERVICE_VERSION: u32 = 1;

/// What `cad.layers.service` does; it names the undo step.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum LayerServiceOperation {
    /// A layer drawn from a service (“Harita servisi ekle”).
    Add,
    /// A layer for a service's objects, with its fields (“Veri katmanı ekle”).
    AddFeed,
    /// A new name, service or feed (“Servis katmanını değiştir”).
    Update,
    /// A layer drawn from a service taken away (“Servis katmanını sil”).
    Remove,
}

/// Input of `cad.layers.service` v1: one change of the layer tree, as one
/// undo step named after the operation.
///
/// - `add`: `name`, `service`; `parent` and `index` place it;
/// - `addFeed`: `name`, `feed`, `fields` (absent: none); placed as `add`;
/// - `update`: `layer`, and any of `name`, `service` (a service layer's),
///   `feed` (a feed layer's);
/// - `remove`: `layer` (a service layer).
///
/// `parent`: the group it goes into, or a layer whose group it goes into;
/// absent, the top. `index`: its place among the group's nodes, first on
/// top; absent or past the end, last: drawn first, under everything else.
/// `connections`: the project's connections it adds or replaces (by id), in
/// the same step.
///
/// Refusals (`CommandError.code`), checked in this order: `no_layer` (update
/// and remove without `layer`), `empty_name` (add, addFeed; update with a
/// name that is empty or only white space), `no_service` (add),
/// `no_feed` (addFeed), `service_and_feed` (both given), `invalid_service`,
/// `invalid_feed`, `invalid_fields`, `invalid_connection` (the project's
/// connections with these, by `connections_problem`); then
/// `invalid_revision`, `revision_conflict` (status `conflict`),
/// `layer_not_found`, `not_a_layer` (a group), `not_a_service_layer`
/// (update's service on a layer that has none, update's feed on a layer
/// without one, remove on a layer drawn from no service), `layer_has_objects`
/// (update giving a service to a layer with objects), `unknown_connection`
/// (a service or feed naming a connection the project does not have).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayersService {
    pub operation: LayerServiceOperation,
    /// `update`, `remove`: the layer's id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layer: Option<String>,
    /// `add`, `addFeed`: the new layer's name; `update`: a new name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub parent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub index: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub service: Option<ServiceLayer>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub feed: Option<FeatureFeed>,
    /// `addFeed`: the layer's fields (docs/adr/0199 §1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub fields: Option<Vec<LayerField>>,
    /// Connections added to the project, or replacing its own of the same id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub connections: Option<Vec<ServiceConnection>>,
    /// The document revision the input was prepared against, as decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.layers.service` v1.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayersServiced {
    /// The layer added, changed or removed.
    pub layer: String,
    /// The document's revision after the write, as decimal text.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.layers.service` would write (plan mode); nothing is written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayersServicePlan {
    /// The layer as execute would leave it (`add`, `addFeed`: its id the one
    /// it would be given); absent for `remove`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub node: Option<LayerNode>,
    /// The project's connections as execute would leave them.
    pub connections: Vec<ServiceConnection>,
    /// The document revision the plan was made against.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// Writes or takes away a layer's time setting (docs/adr/0210 §11).
pub const CAD_LAYERS_TIME: &str = "cad.layers.time";
pub const CAD_LAYERS_TIME_VERSION: u32 = 1;

/// Input of `cad.layers.time` v1: a layer's time setting written, or taken
/// away (`time` absent or null), as one undo step “Zaman ayarları”. A layer
/// that already has it as given is left as it is (`changed` false, no step).
///
/// Refusals (`CommandError.code`), checked in this order: `invalid_time`
/// (`LayerTime::problem`); then `invalid_revision`, `revision_conflict`
/// (status `conflict`), `layer_not_found`, `not_a_layer` (a group),
/// `service_layer` (a layer drawn from a service: it holds no objects).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayersTime {
    /// The layer's id.
    pub layer: String,
    /// Its new time setting; absent or null takes it away.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub time: Option<crate::temporal::LayerTime>,
    /// The document revision the input was prepared against, as decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.layers.time` v1.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayersTimed {
    pub layer: String,
    /// Whether the setting differed (an undo step was written).
    pub changed: bool,
    /// The document's revision after the write, as decimal text.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.layers.time` would write (plan mode); nothing is written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayersTimePlan {
    /// The layer as execute would leave it.
    pub node: LayerNode,
    pub changed: bool,
    /// The document revision the plan was made against.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// Writes or takes away a layer's filter (docs/adr/0211 §5).
pub const CAD_LAYERS_FILTER: &str = "cad.layers.filter";
pub const CAD_LAYERS_FILTER_VERSION: u32 = 1;

/// Input of `cad.layers.filter` v1: a layer's filter written, or taken away
/// (`filter` absent or null), as one undo step “Katman süzgeci”. A layer that
/// already has it as given is left as it is (`changed` false, no step).
///
/// Refusals (`CommandError.code`), checked in this order: `invalid_filter`
/// (`LayerFilter::problem`), `invalid_expression` (the condition does not
/// compile, or reads `$sıra` or `$ölçek`; the message says where); then
/// `invalid_revision`, `revision_conflict` (status `conflict`),
/// `layer_not_found`, `not_a_layer` (a group), `service_layer` (a layer drawn
/// from a service: it holds no objects).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayersFilter {
    /// The layer's id.
    pub layer: String,
    /// Its new filter; absent or null takes it away.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub filter: Option<crate::layer_filter::LayerFilter>,
    /// The document revision the input was prepared against, as decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.layers.filter` v1.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayersFiltered {
    pub layer: String,
    /// Whether the filter differed (an undo step was written).
    pub changed: bool,
    /// The layer's objects that pass the filter (all of them without one).
    pub passed: u32,
    /// The layer's objects.
    pub total: u32,
    /// The document's revision after the write, as decimal text.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.layers.filter` would write (plan mode); nothing is written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayersFilterPlan {
    /// The layer as execute would leave it.
    pub node: LayerNode,
    pub changed: bool,
    /// The layer's objects that would pass the filter, and all of them.
    pub passed: u32,
    pub total: u32,
    /// The document revision the plan was made against.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// Writes or takes away a layer's renderer (docs/adr/0213 §5).
pub const CAD_LAYERS_RENDERER: &str = "cad.layers.renderer";
pub const CAD_LAYERS_RENDERER_VERSION: u32 = 1;

/// Input of `cad.layers.renderer` v1: a layer's renderer (the style
/// engine's JSON, docs/STYLE.md §4: tek sembol, kategorili, aralıklı,
/// kurallar, and the thematic ones of docs/adr/0213) written, or taken away
/// (`renderer` absent or null: the layer's simple look), as one undo step
/// “Katman stili”. A layer that already has it as given is left as it is
/// (`changed` false, no step). A renderer is the layer's look: a locked
/// layer takes it too.
///
/// Refusals (`CommandError.code`), checked in this order: `invalid_renderer`
/// (the style core's rules, `style::rules::renderer_problem`: an unknown
/// kind, a value out of its range, an expression that does not compile;
/// the message says which); then `invalid_revision`, `revision_conflict`
/// (status `conflict`), `layer_not_found`, `not_a_layer` (a group),
/// `service_layer` (a layer drawn from a service: it holds no objects).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayersRenderer {
    /// The layer's id.
    pub layer: String,
    /// Its new renderer; absent or null takes it away.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "unknown"))]
    pub renderer: Option<serde_json::Value>,
    /// The document revision the input was prepared against, as decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.layers.renderer` v1.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayersRendered {
    pub layer: String,
    /// Whether the renderer differed (an undo step was written).
    pub changed: bool,
    /// The document's revision after the write, as decimal text.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.layers.renderer` would write (plan mode); nothing is written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayersRendererPlan {
    /// The layer as execute would leave it.
    pub node: LayerNode,
    pub changed: bool,
    /// The document revision the plan was made against.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
