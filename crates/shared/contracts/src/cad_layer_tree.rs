//! The layer tree's commands of the product command catalog (docs/adr/0248):
//! `cad.layers.edit` adds, restyles, moves, removes, copies, merges and
//! purges layers and groups and gives a layer its fields, each as one undo
//! step; `cad.layers.set` renames, shows, hides, locks, unlocks, isolates
//! and makes active, the tree's own changes (edits, not undo steps,
//! docs/adr/0020); `cad.layers.states` saves, updates, renames, removes and
//! applies the project's named layer states (docs/adr/0177 §4). The
//! Katmanlar panel and its windows write through them on the desktop
//! (`crates/native/application`); the shared cases are in
//! `fixtures/commands/v1`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::fields::LayerField;
use crate::layer::{LayerNodeType, LineType};

/// Whether `color` may be a layer's colour: `#rrggbb` (either case) or one
/// of the theme's tokens `fg`, `fg-dim`, `ink`, `paper` (`LayerStyle::color`).
pub fn layer_color_ok(color: &str) -> bool {
    matches!(color, "fg" | "fg-dim" | "ink" | "paper") || crate::is_hex_colour(color)
}

// ── cad.layers.edit ─────────────────────────────────────────────────────────

/// Adds, restyles, moves, removes, copies, merges or purges layers, or gives
/// a layer its fields, as one undo step.
pub const CAD_LAYERS_EDIT: &str = "cad.layers.edit";
pub const CAD_LAYERS_EDIT_VERSION: u32 = 1;

/// What `cad.layers.edit` does; it names the undo step.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum LayerEditOperation {
    /// A new layer or group (“Katman ekle”, “Grup ekle”).
    Add,
    /// A node's colour, line type or line weight (“Katman rengi”, “Çizgi
    /// tipi”, “Çizgi kalınlığı”; more than one: “Katman stili”).
    Style,
    /// A node into another group or place (“Katmanı taşı”).
    Move,
    /// A layer or a group with everything under it and the objects on them
    /// (“Katman sil”, docs/adr/0072).
    Remove,
    /// A layer copied with its objects (“Katmanı kopyala”, docs/adr/0177 §3).
    Duplicate,
    /// Layers' objects moved to a target layer and the layers removed
    /// (“Katmanları birleştir”, docs/adr/0177 §3).
    Merge,
    /// A layer's fields (“Alanlar”, docs/adr/0199 §3).
    Fields,
    /// The empty layers, the groups left empty and the definitions nothing
    /// places (“Kullanılmayanları temizle”, docs/adr/0177 §5).
    Purge,
}

/// A field renamed on a layer's objects (`cad.layers.edit`'s `fields`): the
/// key `from` becomes `to`, its values kept.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FieldRename {
    pub from: String,
    pub to: String,
}

/// Input of `cad.layers.edit` v1: one change of the layer tree as one undo
/// step named after it.
///
/// - `add`: `kind` (absent: a layer), `name` (absent: “Yeni katman” or “Yeni
///   grup”, made one of its kind as “Yeni katman 2”), `parent` (a group it
///   goes into, or a layer whose group it goes into; absent: the top),
///   `index` (its place among the group's nodes, first on top; absent or
///   past the end: last), `color`, `lineType`, `lineWeight` (absent: `fg`,
///   continuous, 0.18 mm), `visible` (absent: true), `locked` (absent:
///   false), `activate` (a new layer becomes the active one in the same step;
///   a group never does);
/// - `style`: `layer`, and any of `color`, `lineType`, `lineWeight`; a locked
///   node takes it too (a look, as `cad.layers.renderer`'s);
/// - `move`: `layer`, `parent` (a group; absent: the top), `index` (as add's);
/// - `remove`: `layer`; the objects on it (on every layer under a group) go
///   in the same step;
/// - `duplicate`: `layer`, `name` (absent: “<ad> kopyası”, made one of its
///   kind); the copy goes last into the layer's group with its style,
///   visibility and fields, unlocked, and copies of its objects;
/// - `merge`: `layer` (the target), `layers` (the sources; the target among
///   them and repeats are passed over); the sources' objects go to the
///   target and the sources are removed. A source that is the active layer
///   gives that to the target first, outside the step (a tree change, as
///   the eye);
/// - `fields`: `layer`, `fields` (absent: none), `renames` (keys moved on
///   its objects, all at once);
/// - `purge`: `layers` (the layers and groups to take, by id; absent: every
///   one found), `blocks` (the definitions, by id; absent: every one
///   found); what something staying uses stays and is counted in `kept`.
///
/// Refusals (`CommandError.code`), checked in this order: `no_layer`
/// (style, move, remove, duplicate, merge and fields without `layer`),
/// `no_layers` (merge without a source), `empty_name` (add and duplicate
/// with a name that is empty or only white space), `nothing_to_set` (style
/// without a colour, line type or line weight), `invalid_color` (not
/// `#rrggbb` nor `fg`, `fg-dim`, `ink`, `paper`), `invalid_line_weight`
/// (not from 0 to 100 mm), `invalid_fields` (`fields_problem`); then
/// `invalid_revision`, `revision_conflict` (status `conflict`); then, for the
/// node: `layer_not_found` (`layer`, `parent`, `layers[i]`), `not_a_group`
/// (move's `parent` a layer), `move_into_itself` (move's `parent` the node or
/// under it), `not_a_layer` (duplicate, merge's target and sources, fields: a
/// group), `service_layer` (duplicate, merge's target and sources, fields: a
/// layer drawn from a map service), `layer_locked` (move: the node or the
/// group it goes into locked, by itself or a group above; duplicate, merge's
/// target and sources: locked), `unknown_block` (purge's `blocks[i]`),
/// `layer_refused` (what the drawing refuses: remove's last, active or
/// locked node, docs/adr/0072; fields' renames giving an object two
/// attributes of one name). A change that changes nothing (the same style,
/// the same place, the same fields, nothing to purge) writes nothing and
/// answers `changed` false.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayersEdit {
    pub operation: LayerEditOperation,
    /// The node it acts on; merge's target.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layer: Option<String>,
    /// Merge's sources; purge's layers and groups.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layers: Option<Vec<String>>,
    /// Purge's block definitions, by their ids.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub blocks: Option<Vec<String>>,
    /// Add's kind of node; absent: a layer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub kind: Option<LayerNodeType>,
    /// Add's and duplicate's name, trimmed.
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
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub line_type: Option<LineType>,
    /// add, style: a project line type by its name (docs/adr/0313 §1), its nearest built-in written as the line type
    /// unless `lineType` is given; null: none (a `lineType` given alone takes it away too).
    #[serde(
        default,
        with = "crate::cad_properties::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<String>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub line_pattern: Option<Option<String>>,
    /// Plot line weight, mm.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub line_weight: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub visible: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub locked: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub activate: Option<bool>,
    /// The layer's fields (docs/adr/0199 §1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub fields: Option<Vec<LayerField>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub renames: Option<Vec<FieldRename>>,
    /// The document revision the input was prepared against, as decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

impl LayersEdit {
    /// The operation with nothing else given (the fields are set after).
    pub fn new(operation: LayerEditOperation) -> Self {
        Self {
            line_pattern: None,
            operation,
            layer: None,
            layers: None,
            blocks: None,
            kind: None,
            name: None,
            parent: None,
            index: None,
            color: None,
            line_type: None,
            line_weight: None,
            visible: None,
            locked: None,
            activate: None,
            fields: None,
            renames: None,
            expected_revision: None,
        }
    }
}

/// Output of `cad.layers.edit` v1, and what plan says it would do.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayersEdited {
    /// The node added or copied (a new `layer-N`), the node changed, merge's
    /// target; absent for purge.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layer: Option<String>,
    /// Whether anything was written (an undo step).
    pub changed: bool,
    /// The objects removed (remove), copied (duplicate) or moved (merge).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub objects: u32,
    /// The nodes taken away: merge's sources, purge's layers then groups
    /// (deepest first).
    pub removed: Vec<String>,
    /// Purge's definitions taken away, in the order they went.
    pub blocks: Vec<String>,
    /// Purge's asked ones that stay because something staying uses them.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub kept: u32,
    /// The document's revision after the write (plan: the one it was made
    /// against), as decimal text.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

// ── cad.layers.set ──────────────────────────────────────────────────────────

/// Renames, shows, hides, locks, unlocks, isolates or makes active: the
/// tree's own changes, edits but no undo steps.
pub const CAD_LAYERS_SET: &str = "cad.layers.set";
pub const CAD_LAYERS_SET_VERSION: u32 = 1;

/// What `cad.layers.set` does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum LayerSetOperation {
    Rename,
    Visible,
    Locked,
    Active,
    Isolate,
    ShowAll,
    /// Yazdır (docs/adr/0260 §4): whether the node is printed.
    Plot,
    /// Dondur / Çöz (docs/adr/0311 §1): whether the node is frozen.
    Frozen,
    /// Bütün katmanları çöz: every node thawed.
    ThawAll,
    /// Yalıtımı kaldır: the nodes the last `isolate` hid, still hidden, shown again.
    Unisolate,
}

/// Input of `cad.layers.set` v1: one of the tree's own changes, as the eye,
/// the lock and the name in the Katmanlar panel make it. Each is an edit of
/// the drawing (it gets dirty, its revision moves) but no undo step
/// (docs/adr/0020); making a layer active is kept in the file but is no
/// edit (the revision stays).
///
/// - `rename`: `layer`, `name` (trimmed);
/// - `plot`: `layer`, `value` (docs/adr/0260 §4): a node not printed is drawn
///   on screen but not on a sheet, its PDF or a print; a group's covers
///   everything under it;
/// - `visible`, `locked`: `layer`, `value`; a group's lock holds for
///   everything under it, a hidden group hides everything under it;
/// - `active`: `layer`, a layer holding objects;
/// - `isolate`: `layers`: these nodes, the groups above them and everything
///   under them shown, the rest hidden; the drawing remembers the nodes it
///   hid (for this session, not in the file);
/// - `unisolate`: the nodes the last `isolate` hid that are still hidden
///   shown again; those shown by hand meanwhile stay as they are; nothing
///   remembered changes nothing;
/// - `showAll`: every node shown; a frozen node stays frozen;
/// - `frozen`: `layer`, `value` (docs/adr/0311 §1): a frozen node is neither
///   drawn, picked, snapped to nor counted in the extent; a group's covers
///   everything under it;
/// - `thawAll`: every node thawed.
///
/// The eye, the lock, the freeze, printing, the name and the active layer
/// are the project's (kept in the file); only what `isolate` hid is the
/// session's (docs/adr/0311 §2).
///
/// Refusals (`CommandError.code`), checked in this order: `no_layer`
/// (rename, visible, locked, plot, frozen, active without `layer`),
/// `no_layers` (isolate without a node), `empty_name` (rename with a name
/// empty or only white space), `no_value` (visible, locked, plot, frozen
/// without `value`); then `invalid_revision`, `revision_conflict` (status
/// `conflict`); `layer_not_found` (`layer`, `layers[i]`), `not_a_layer`
/// (active: a group), `service_layer` (active: a layer drawn from a map
/// service holds no objects), `layer_frozen` (active: a frozen layer, or one
/// in a frozen group), `active_frozen` (frozen: the active layer, or a group
/// holding it). A change that changes nothing answers `changed` false.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayersSet {
    pub operation: LayerSetOperation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layers: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub value: Option<bool>,
    /// The document revision the input was prepared against, as decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

impl LayersSet {
    /// The operation on `layer` (none: `isolate`'s and `showAll`'s).
    pub fn new(operation: LayerSetOperation, layer: Option<String>) -> Self {
        Self {
            operation,
            layer,
            layers: None,
            name: None,
            value: None,
            expected_revision: None,
        }
    }
}

/// Output of `cad.layers.set` v1, and what plan says it would do.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayersChanged {
    /// Whether the tree changed.
    pub changed: bool,
    /// The layer active afterwards.
    pub active: String,
    /// The document's revision afterwards (plan: the one it was made
    /// against), as decimal text.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

// ── cad.layers.states ───────────────────────────────────────────────────────

/// Saves, updates, renames, removes or applies a named layer state.
pub const CAD_LAYERS_STATES: &str = "cad.layers.states";
pub const CAD_LAYERS_STATES_VERSION: u32 = 1;

/// What `cad.layers.states` does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum LayerStatesOperation {
    Save,
    Update,
    Rename,
    Remove,
    Apply,
    /// The states as a file (docs/adr/0311 §3).
    Export,
    /// A file's states added to the project's (docs/adr/0311 §3).
    Import,
}

/// A file of exported layer states (docs/adr/0311 §3): each node with its
/// path, so another drawing finds it by its names.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayerStatesFile {
    /// Always `kentos.layer-states`.
    pub format: String,
    /// Always 1.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub version: u32,
    pub states: Vec<crate::LayerState>,
}

/// A layer states file's format and version.
pub const LAYER_STATES_FORMAT: &str = "kentos.layer-states";
pub const LAYER_STATES_VERSION: u32 = 1;

/// What is wrong with a layer states file: its format or version, an empty
/// or repeated name, an empty node or a node twice in a state.
pub fn layer_states_file_problem(file: &LayerStatesFile) -> Option<String> {
    if file.format != LAYER_STATES_FORMAT || file.version != LAYER_STATES_VERSION {
        return Some(format!(
            "dosya {LAYER_STATES_FORMAT} sürüm {LAYER_STATES_VERSION} değil (“{}” sürüm {})",
            file.format, file.version
        ));
    }
    for (i, s) in file.states.iter().enumerate() {
        let name = s.name.trim();
        if name.is_empty() {
            return Some(format!("{}. durumun adı boş", i + 1));
        }
        if file.states[..i].iter().any(|t| t.name.trim() == name) {
            return Some(format!("“{name}” adlı durum dosyada iki kez var"));
        }
        for (j, n) in s.nodes.iter().enumerate() {
            if n.node.is_empty() && n.path.as_deref().is_none_or(str::is_empty) {
                return Some(format!(
                    "“{name}” durumunda kimliği ve yolu boş bir düğüm var"
                ));
            }
            if s.nodes[..j]
                .iter()
                .any(|m| m.node == n.node && m.path == n.path)
            {
                return Some(format!("“{name}” durumunda bir düğüm iki kez var"));
            }
        }
    }
    None
}

/// Input of `cad.layers.states` v1 (docs/adr/0177 §4).
///
/// - `save`: the tree as it is, as a new state: `name` (absent: the first
///   free “Durum n”), `locks` (absent: false), `styles` (absent: true); its
///   id the first free `durum-n`;
/// - `update`: `state` saved again from the tree as it is, keeping `locks`
///   and `styles` (absent: what it kept);
/// - `rename`: `state`, `name` (trimmed);
/// - `remove`: `state`;
/// - `apply`: `state`: the visibility and the locks (when kept) of the
///   nodes it names that the tree still has become what it kept (the tree's
///   own changes, no undo step), their styles (when kept) in one undo step
///   “Katman durumu: <ad>”; their freeze and printing (when the state keeps
///   them, docs/adr/0311 §3) as the visibility;
/// - `export`: the states named in `states` (absent: every one) as a file
///   (`file` in the answer), each node with its path; nothing written;
/// - `import`: `file`'s states added: a node is found by its id, else by its
///   path (the names from the top, case and Turkish letters folded), else
///   kept as it is and counted `missing` when applied; a state whose name
///   the project has is skipped, or with `replace` written over it (its id
///   kept); a new one gets the next free `durum-n`.
///
/// The states are the project's settings: saving, updating, renaming and
/// removing make the drawing dirty and are no undo step.
///
/// Refusals (`CommandError.code`), checked in this order: `no_state`
/// (update, rename, remove, apply without `state`), `empty_name` (rename,
/// or save with a name, empty or only white space); then
/// `no_file` (import without `file`), `invalid_states` (a file against its
/// rules, `layer_states_file_problem`); then `invalid_revision`,
/// `revision_conflict` (status `conflict`); `state_not_found` (`state`,
/// export's `states[i]`), `duplicate_name` (another state has the name,
/// spaces at its ends aside).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayersStates {
    pub operation: LayerStatesOperation,
    /// The state's id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub state: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub locks: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub styles: Option<bool>,
    /// Export's states, by id; absent, every one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub states: Option<Vec<String>>,
    /// Import's file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub file: Option<LayerStatesFile>,
    /// Import: a state whose name the project has written over (absent: skipped).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub replace: Option<bool>,
    /// The document revision the input was prepared against, as decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

impl LayersStates {
    /// The operation on `state` (none: `save`'s).
    pub fn new(operation: LayerStatesOperation, state: Option<String>) -> Self {
        Self {
            operation,
            state,
            name: None,
            locks: None,
            styles: None,
            states: None,
            file: None,
            replace: None,
            expected_revision: None,
        }
    }
}

/// Output of `cad.layers.states` v1, and what plan says it would do.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayerStatesDone {
    /// The state saved, updated, renamed, removed or applied.
    pub state: String,
    /// Whether anything changed.
    pub changed: bool,
    /// Apply's nodes shown or hidden, locked or unlocked, restyled.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub visible: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub locked: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub styles: u32,
    /// Apply's nodes frozen or thawed, and printed or not (docs/adr/0311 §3).
    #[serde(default)]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub frozen: u32,
    #[serde(default)]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub plot: u32,
    /// Apply's nodes the tree no longer has.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub missing: u32,
    /// Import's states added, written over and skipped (their name the
    /// project's, without `replace`).
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub added: u32,
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub replaced: u32,
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub skipped: u32,
    /// Export's file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub file: Option<LayerStatesFile>,
    /// The document's revision afterwards (plan: the one it was made
    /// against), as decimal text.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

fn is_zero(n: &u32) -> bool {
    *n == 0
}
