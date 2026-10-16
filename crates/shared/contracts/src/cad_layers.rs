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
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
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
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub passed: u32,
    /// The layer's objects.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
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
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub passed: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
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

/// Writes how a layer is composed into the drawing (docs/adr/0226 §3, §4).
pub const CAD_LAYERS_COMPOSE: &str = "cad.layers.compose";
pub const CAD_LAYERS_COMPOSE_VERSION: u32 = 1;

/// Input of `cad.layers.compose` v1: a layer's opacity, blending mode and
/// mask written as one undo step “Katman çizimi”; the layer's composition
/// becomes exactly what is given (absent: opaque, normal, no mask; nothing
/// given takes it away). A layer that already has it as given is left as it
/// is (`changed` false, no step). It is how the layer is drawn: a locked
/// layer, a service layer and a raster's take it too.
///
/// Refusals (`CommandError.code`), checked in this order: `invalid_compose`
/// (an opacity outside 0–1, path `opacity`; the mask's rule,
/// `LayerMask::problem`, path `mask`); then `invalid_revision`,
/// `revision_conflict` (status `conflict`), `layer_not_found`, `not_a_layer`
/// (a group); then each of the mask's targets in order (path
/// `mask/targets/<i>`): `mask_target_self` (the layer itself),
/// `mask_target_not_found` (a layer the drawing does not have),
/// `mask_target_not_a_layer` (a group). A target deleted later is skipped
/// when the mask is drawn.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayersCompose {
    /// The layer's id.
    pub layer: String,
    /// 0–1; absent, 1 (opaque).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub opacity: Option<f64>,
    /// Absent, normal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub blend: Option<crate::layer_compose::BlendMode>,
    /// The mask its labels and markers cut into other layers; absent, none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub mask: Option<crate::layer_compose::LayerMask>,
    /// The document revision the input was prepared against, as decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

impl LayersCompose {
    /// The composition as the layer keeps it: without what says nothing; None when nothing is left.
    pub fn composition(&self) -> Option<crate::layer_compose::LayerCompose> {
        crate::layer_compose::LayerCompose {
            opacity: self.opacity,
            blend: self.blend,
            mask: self.mask.clone(),
        }
        .normalized()
    }
}

/// Output of `cad.layers.compose` v1.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayersComposed {
    pub layer: String,
    /// Whether it differed (an undo step was written).
    pub changed: bool,
    /// The document's revision after the write, as decimal text.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.layers.compose` would write (plan mode); nothing is written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayersComposePlan {
    /// The layer as execute would leave it.
    pub node: LayerNode,
    pub changed: bool,
    /// The document revision the plan was made against.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// Writes how the attribute table paints a layer's rows and cells (Koşullu biçim, docs/adr/0413).
pub const CAD_LAYERS_TABLE: &str = "cad.layers.table";
pub const CAD_LAYERS_TABLE_VERSION: u32 = 1;

/// Input of `cad.layers.table` v1: a layer's table rules written as one undo step “Koşullu biçim”;
/// the layer's rules become exactly the ones given, in their order (none takes them away). A layer
/// that already has them as given is left as it is (`changed` false, no step).
///
/// Refusals (`CommandError.code`), checked in this order: `invalid_table_styles` (the rules' rule,
/// `table_styles_problem`, path `styles/<i>`); then `invalid_revision`, `revision_conflict` (status
/// `conflict`), `layer_not_found`, `not_a_layer` (a group), `service_layer` (a layer drawn from a
/// service); then each rule in order: `field_not_found` (a field the layer does not have, path
/// `styles/<i>/field`), `invalid_expression` (a condition that does not compile over the layer's
/// fields, path `styles/<i>/condition`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayersTable {
    /// The layer's id.
    pub layer: String,
    /// The rules, in their order: a field's first rule that holds gives its cell's look before the
    /// row's.
    #[serde(default)]
    pub styles: Vec<crate::table_styles::TableStyle>,
    /// The document revision the input was prepared against, as decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Writes a layer's value groups (Değer bileşimleri, docs/adr/0423).
pub const CAD_LAYERS_CONTINGENTS: &str = "cad.layers.contingents";
pub const CAD_LAYERS_CONTINGENTS_VERSION: u32 = 1;

/// Input of `cad.layers.contingents` v1: a layer's value groups written as one undo step “Değer
/// bileşimleri”; the groups given (none takes them away). A layer that already has them is left as it is.
///
/// Refusals, checked in this order: `invalid_revision`, `revision_conflict` (status `conflict`),
/// `layer_not_found`, `not_a_layer`, `service_layer`; then `invalid_contingents` (groups that do not
/// fit the layer's fields, `contingents_problem`, path `groups/<where>`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayersContingents {
    /// The layer's id.
    pub layer: String,
    /// The groups, in their order.
    #[serde(default)]
    pub groups: Vec<crate::contingents::FieldGroup>,
    /// The document revision the input was prepared against, as decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Writes a layer's subtypes (Alt türler, docs/adr/0422).
pub const CAD_LAYERS_SUBTYPES: &str = "cad.layers.subtypes";
pub const CAD_LAYERS_SUBTYPES_VERSION: u32 = 1;

/// Input of `cad.layers.subtypes` v1: a layer's subtypes written as one undo step “Alt türler”; none
/// takes them away. A layer that already has them as given is left as it is (`changed` false).
///
/// Refusals, checked in this order: `invalid_revision`, `revision_conflict` (status `conflict`),
/// `layer_not_found`, `not_a_layer`, `service_layer`; then `invalid_subtypes` (subtypes that do not fit
/// the layer's fields, `subtypes_problem`, path `subtypes/<where>`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayersSubtypes {
    /// The layer's id.
    pub layer: String,
    /// The subtypes; none: the layer has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub subtypes: Option<crate::subtypes::LayerSubtypes>,
    /// The document revision the input was prepared against, as decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Writes a layer's actions and information card (docs/adr/0225 §4, §5).
pub const CAD_LAYERS_INFO: &str = "cad.layers.info";
pub const CAD_LAYERS_INFO_VERSION: u32 = 1;

/// Input of `cad.layers.info` v1: a layer's actions and information card
/// written as one undo step “Bilgi kartı”; absent actions none, an absent
/// or null card the default card (a card with nothing in it is no card). A
/// layer that already has them as given is left as it is (`changed` false,
/// no step). They are how the layer's objects are shown and opened: a
/// locked layer takes them too.
///
/// Refusals (`CommandError.code`), checked in this order: `invalid_actions`
/// (`actions_problem`), `invalid_card` (`InfoCard::problem`),
/// `invalid_template` (an action's target, the card's title or text whose
/// `{…}` does not close, is empty, or whose expression does not compile or
/// looks at other layers; the message says which and where); then
/// `invalid_revision`, `revision_conflict` (status `conflict`),
/// `layer_not_found`, `not_a_layer` (a group), `service_layer` (a layer
/// drawn from a service: it holds no objects).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayersInfo {
    /// The layer's id.
    pub layer: String,
    /// Its actions, in order; absent or empty: none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(
        feature = "ts",
        ts(as = "Option<Vec<crate::layer_info::LayerAction>>", optional)
    )]
    pub actions: Vec<crate::layer_info::LayerAction>,
    /// Its information card; absent or null: the default card.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub card: Option<crate::layer_info::InfoCard>,
    /// The document revision the input was prepared against, as decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.layers.info` v1.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayersInformed {
    pub layer: String,
    /// Whether they differed (an undo step was written).
    pub changed: bool,
    /// The document's revision after the write, as decimal text.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.layers.info` would write (plan mode); nothing is written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayersInfoPlan {
    /// The layer as execute would leave it.
    pub node: LayerNode,
    pub changed: bool,
    /// The document revision the plan was made against.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
