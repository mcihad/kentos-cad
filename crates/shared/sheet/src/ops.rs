//! Every edit is a pure operation (design §4): `apply(book, op)` gives the
//! new book and the operations that take it back, exactly. Ids come with the
//! operation (the host makes them), so a replay is deterministic. The
//! interface, a script and an agent use the same operations.
//!
//! The input is brought to its normal form first (`validate::normalize`);
//! the inverse returns that form. Every result is validated: an operation
//! that would break a rule of the book is refused whole.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::error::{Result, SheetError};
use crate::kinds::*;
use crate::layout;
use crate::model::*;
use crate::template::{InstanceIds, InstanceOptions, Template, instantiate};
use crate::units::*;
use crate::validate::{ITEM_MIN, normalize, validate_book, validate_page};

// ── The operations ───────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct AddSheet {
    pub sheet: Sheet,
    /// Where among the sheets; none: last.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub index: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct IdRef {
    pub id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct Rename {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct MoveSheet {
    pub id: SheetId,
    pub to: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct DuplicateSheet {
    pub id: SheetId,
    pub new_id: SheetId,
    pub name: String,
    /// One per item of the sheet, in order.
    pub item_ids: Vec<ItemId>,
    /// Where; none: right after the original.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub index: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct ReplaceSheet {
    pub sheet: Sheet,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct ReplaceMaster {
    pub master: Master,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct SetPage {
    pub owner: Owner,
    pub page: Page,
    /// The items follow their constraints (false: they stay where they are).
    #[serde(default = "yes")]
    pub relayout: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct AddMaster {
    pub master: Master,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub index: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct SetSheetMaster {
    pub sheet: SheetId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub master: Option<MasterId>,
}

/// “Ana sayfadan ayır”: the master's items copied onto the sheet (under its own), the master let go.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct DetachMaster {
    pub sheet: SheetId,
    /// One per item of the master, in order.
    pub item_ids: Vec<ItemId>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct AddItems {
    pub to: Owner,
    pub items: Vec<Item>,
    /// Where in the drawing order; none: on top.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub index: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct IndexedItem {
    pub index: u32,
    pub item: Item,
}

/// Items put back where they were (in rising `index` order).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct InsertItems {
    pub to: Owner,
    pub entries: Vec<IndexedItem>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct ItemIds {
    pub ids: Vec<ItemId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct MoveItems {
    pub ids: Vec<ItemId>,
    pub delta: PointUm,
}

/// A resize handle: a corner or the middle of an edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum Handle {
    N,
    Ne,
    E,
    Se,
    S,
    Sw,
    W,
    Nw,
}

impl Handle {
    /// −1 west/north, 0, +1 east/south.
    pub fn dirs(self) -> (i32, i32) {
        match self {
            Handle::N => (0, -1),
            Handle::Ne => (1, -1),
            Handle::E => (1, 0),
            Handle::Se => (1, 1),
            Handle::S => (0, 1),
            Handle::Sw => (-1, 1),
            Handle::W => (-1, 0),
            Handle::Nw => (-1, -1),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct ResizeItem {
    pub id: ItemId,
    pub handle: Handle,
    /// Where the handle is dragged to, on the paper.
    pub to: PointUm,
    #[serde(default)]
    pub keep_aspect: bool,
    #[serde(default)]
    pub from_center: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct RotateItems {
    pub ids: Vec<ItemId>,
    /// Added to each item's rotation, clockwise.
    pub angle: Mdeg,
    /// Around this point; none: one item around its own centre, several around their box's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub around: Option<PointUm>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct ItemFrame {
    pub id: ItemId,
    pub frame: RectUm,
    #[serde(default)]
    pub rotation: Mdeg,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct SetFrames {
    pub frames: Vec<ItemFrame>,
}

/// A partial change of an item: a JSON merge (objects merge, everything else replaces; `null` clears an optional field).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct SetItemProps {
    pub id: ItemId,
    #[cfg_attr(feature = "ts", ts(type = "Record<string, unknown>"))]
    pub patch: serde_json::Value,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum ReorderTo {
    Front,
    Back,
    Forward,
    Backward,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct Reorder {
    pub ids: Vec<ItemId>,
    pub to: ReorderTo,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct SetOrder {
    pub owner: Owner,
    /// Every item id of the owner, in the new drawing order.
    pub order: Vec<ItemId>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct GroupItems {
    pub ids: Vec<ItemId>,
    /// The new group item (its kind `group`); its frame is made its children's.
    pub group: Item,
    /// Where the group item goes; none: above its topmost child.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub index: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct SetItems {
    pub owner: Owner,
    pub items: Vec<Item>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct SetFlag {
    pub ids: Vec<ItemId>,
    pub value: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum AlignEdge {
    Left,
    /// Centres across.
    Center,
    Right,
    Top,
    /// Centres down.
    Middle,
    Bottom,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum AlignTo {
    Selection,
    Page,
    Margins,
    /// The `key` item.
    KeyItem,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct Align {
    pub ids: Vec<ItemId>,
    pub edge: AlignEdge,
    pub to: AlignTo,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub key: Option<ItemId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum DistributeMode {
    Centers,
    Gaps,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct Distribute {
    pub ids: Vec<ItemId>,
    /// `x`: across, `y`: down.
    pub axis: Axis,
    pub mode: DistributeMode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum SizeDimension {
    Width,
    Height,
    Both,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct MatchSize {
    pub ids: Vec<ItemId>,
    pub dimension: SizeDimension,
    /// Whose size; none: the first id's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub key: Option<ItemId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct AddGuide {
    pub owner: Owner,
    pub guide: Guide,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub index: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct MoveGuide {
    pub owner: Owner,
    pub id: String,
    pub at: Um,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct RemoveGuide {
    pub owner: Owner,
    pub id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct SetSnapGrid {
    pub sheet: SheetId,
    pub grid: SnapGrid,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct SetAtlas {
    pub sheet: SheetId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub atlas: Option<Atlas>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct SetExport {
    pub sheet: SheetId,
    pub export: ExportDefaults,
}

/// A template laid onto an existing sheet: its page, items, guides and questions replace the sheet's; its master and pictures join the book.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct ApplyTemplate {
    pub sheet: SheetId,
    pub template: Template,
    /// `ids.sheet` is ignored: the sheet keeps its id.
    pub ids: InstanceIds,
    #[serde(default)]
    pub options: InstanceOptions,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct SaveVariables {
    /// A sheet's variables; none: the project's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub sheet: Option<SheetId>,
    pub variables: Vec<Variable>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct AddAssets {
    /// Metadata only; one already in the book is skipped.
    pub assets: Vec<AssetMeta>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct RemoveAssets {
    pub sha256: Vec<String>,
}

/// A new layout variant from the current arrangement (design §3.2a); in force at once if the paper calls for it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct AddVariant {
    pub owner: Owner,
    pub id: String,
    pub name: String,
    pub when: VariantCondition,
    /// Its place in the list, where the first match wins; none: first.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub index: Option<u32>,
}

/// A layout variant removed; if it was in force, the layout the paper now calls for takes its place.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct RemoveVariant {
    pub owner: Owner,
    pub id: String,
}

/// A layout variant's name, condition or place in the list; the layout in force stays until the paper changes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct SetVariant {
    pub owner: Owner,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub when: Option<VariantCondition>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub index: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(tag = "op", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum Op {
    AddSheet(AddSheet),
    RemoveSheet(IdRef),
    RenameSheet(Rename),
    MoveSheet(MoveSheet),
    DuplicateSheet(DuplicateSheet),
    ReplaceSheet(ReplaceSheet),
    SetPage(SetPage),
    AddMaster(AddMaster),
    RemoveMaster(IdRef),
    ReplaceMaster(ReplaceMaster),
    SetSheetMaster(SetSheetMaster),
    DetachMaster(DetachMaster),
    AddItems(AddItems),
    InsertItems(InsertItems),
    RemoveItems(ItemIds),
    MoveItems(MoveItems),
    ResizeItem(ResizeItem),
    RotateItems(RotateItems),
    SetFrames(SetFrames),
    SetItemProps(SetItemProps),
    RenameItem(Rename),
    Reorder(Reorder),
    SetOrder(SetOrder),
    Group(Box<GroupItems>),
    Ungroup(IdRef),
    SetItems(SetItems),
    Lock(SetFlag),
    Hide(SetFlag),
    Align(Align),
    Distribute(Distribute),
    MatchSize(MatchSize),
    AddGuide(AddGuide),
    MoveGuide(MoveGuide),
    RemoveGuide(RemoveGuide),
    SetSnapGrid(SetSnapGrid),
    SetAtlas(SetAtlas),
    SetExport(SetExport),
    ApplyTemplate(Box<ApplyTemplate>),
    SaveVariables(SaveVariables),
    AddAssets(AddAssets),
    RemoveAssets(RemoveAssets),
    AddVariant(AddVariant),
    RemoveVariant(RemoveVariant),
    SetVariant(SetVariant),
}

/// The new book, the operations that take it back, and the undo step's name.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct Applied {
    pub book: SheetBook,
    pub inverse: Vec<Op>,
    /// “Taşı: Harita”.
    pub label: String,
}

// ── Helpers ──────────────────────────────────────────────────────────────

fn unknown_item(id: &str) -> SheetError {
    SheetError::new("unknown_item", format!("“{id}” öğesi kitapta yok."))
}

fn unknown_sheet(id: &str) -> SheetError {
    SheetError::new("unknown_sheet", format!("“{id}” paftası kitapta yok."))
}

fn unknown_master(id: &str) -> SheetError {
    SheetError::new("unknown_master", format!("“{id}” ana sayfası kitapta yok."))
}

fn sheet_mut<'a>(b: &'a mut SheetBook, id: &str) -> Result<&'a mut Sheet> {
    b.sheets
        .iter_mut()
        .find(|s| s.id == id)
        .ok_or_else(|| unknown_sheet(id))
}

fn items_mut<'a>(b: &'a mut SheetBook, owner: &Owner) -> Result<&'a mut Vec<Item>> {
    match owner.kind {
        OwnerKind::Sheet => {
            if b.sheet(&owner.id).is_none() {
                return Err(unknown_sheet(&owner.id));
            }
        }
        OwnerKind::Master => {
            if b.master(&owner.id).is_none() {
                return Err(unknown_master(&owner.id));
            }
        }
    }
    b.items_of_mut(owner)
        .ok_or_else(|| unknown_sheet(&owner.id))
}

/// The owner of `ids` (all must share one) and their indexes, in the order given.
fn locate(b: &SheetBook, ids: &[ItemId]) -> Result<(Owner, Vec<usize>)> {
    if ids.is_empty() {
        return Err(SheetError::new(
            "no_items",
            "İşlem için en az bir öğe seçilmeli.",
        ));
    }
    let mut owner: Option<Owner> = None;
    let mut idx = Vec::with_capacity(ids.len());
    let mut seen = BTreeSet::new();
    for id in ids {
        if !seen.insert(id.as_str()) {
            return Err(SheetError::new(
                "duplicate_id",
                format!("“{id}” öğesi işlemde iki kez geçiyor."),
            ));
        }
        let (o, i) = b.find_item(id).ok_or_else(|| unknown_item(id))?;
        match &owner {
            None => owner = Some(o),
            Some(prev) if *prev != o => {
                return Err(SheetError::new(
                    "mixed_owners",
                    "Bir işlemin öğeleri aynı paftada (ya da aynı ana sayfada) olmalı.",
                ));
            }
            _ => {}
        }
        idx.push(i);
    }
    owner
        .map(|o| (o, idx))
        .ok_or_else(|| SheetError::new("no_items", "İşlem için en az bir öğe seçilmeli."))
}

/// Indexes of every item below `id` in its groups.
fn descendants(items: &[Item], id: &str) -> Vec<usize> {
    let mut out = Vec::new();
    let mut frontier = vec![id.to_owned()];
    while let Some(g) = frontier.pop() {
        for (i, it) in items.iter().enumerate() {
            if it.group.as_deref() == Some(g.as_str()) && !out.contains(&i) {
                out.push(i);
                if it.is_group() {
                    frontier.push(it.id.clone());
                }
            }
        }
        if out.len() > items.len() {
            break;
        }
    }
    out.sort_unstable();
    out
}

/// The selected items and everything in the selected groups, each once, in list order.
fn with_descendants(items: &[Item], idx: &[usize]) -> Vec<usize> {
    let mut set = BTreeSet::new();
    for &i in idx {
        set.insert(i);
        if let Some(it) = items.get(i)
            && it.is_group()
        {
            set.extend(descendants(items, &it.id));
        }
    }
    set.into_iter().collect()
}

/// Whether `ancestor` is above item `i` in its groups.
fn is_below(items: &[Item], i: usize, ancestor: &str) -> bool {
    let mut up = items.get(i).and_then(|it| it.group.clone());
    let mut n = 0;
    while let Some(g) = up {
        if g == ancestor {
            return true;
        }
        n += 1;
        if n > items.len() {
            return false;
        }
        up = items
            .iter()
            .find(|x| x.id == g)
            .and_then(|x| x.group.clone());
    }
    false
}

/// The selected items that are not inside another selected one: what moves as a whole.
fn units(items: &[Item], idx: &[usize]) -> Vec<usize> {
    idx.iter()
        .copied()
        .filter(|&i| {
            !idx.iter().any(|&j| {
                j != i
                    && items
                        .get(j)
                        .is_some_and(|g| g.is_group() && is_below(items, i, &g.id))
            })
        })
        .collect()
}

fn unlocked(items: &[Item], idx: &[usize]) -> Result<()> {
    for &i in idx {
        if let Some(it) = items.get(i)
            && it.locked
        {
            return Err(SheetError::new(
                "item_locked",
                format!("“{}” kilitli; önce kilidini açın.", it.name),
            ));
        }
    }
    Ok(())
}

fn frames_of(items: &[Item], idx: &[usize]) -> Vec<ItemFrame> {
    idx.iter()
        .filter_map(|&i| items.get(i))
        .filter(|it| !it.is_group())
        .map(|it| ItemFrame {
            id: it.id.clone(),
            frame: it.frame,
            rotation: it.rotation,
        })
        .collect()
}

fn bounds(it: &Item) -> RectUm {
    rotated_bounds(&it.frame, it.rotation)
}

fn clamp_index(index: Option<u32>, len: usize) -> usize {
    index.map_or(len, |i| (i as usize).min(len))
}

fn translate(items: &mut [Item], idx: &[usize], dx: i64, dy: i64) {
    for &i in idx {
        if let Some(it) = items.get_mut(i) {
            it.frame = it.frame.translate(dx, dy);
        }
    }
}

// ── Applying ─────────────────────────────────────────────────────────────

/// Applies one operation.
pub fn apply(book: &SheetBook, op: &Op) -> Result<Applied> {
    let mut b = book.clone();
    normalize(&mut b);
    let label = describe(&b, op);
    let inverse = step(&mut b, op)?;
    validate_book(&b)?;
    normalize(&mut b);
    Ok(Applied {
        book: b,
        inverse,
        label,
    })
}

/// Applies operations in order, as one step: the inverse undoes all of them.
pub fn apply_all(book: &SheetBook, ops: &[Op]) -> Result<Applied> {
    let mut b = book.clone();
    normalize(&mut b);
    let mut inverse: Vec<Op> = Vec::new();
    let label = match ops {
        [one] => describe(&b, one),
        _ => format!("{} işlem", ops.len()),
    };
    for op in ops {
        let mut inv = step(&mut b, op)?;
        validate_book(&b)?;
        normalize(&mut b);
        inv.extend(inverse);
        inverse = inv;
    }
    Ok(Applied {
        book: b,
        inverse,
        label,
    })
}

/// The sheet or master page whose arrangement an operation edits or lays out, and whether the edit is
/// written through to the layout variant in force (edits are; laying out for a paper is not).
fn layout_owner(b: &SheetBook, op: &Op) -> Option<(Owner, bool)> {
    let of = |id: Option<&ItemId>| id.and_then(|id| b.find_item(id)).map(|(o, _)| (o, true));
    match op {
        Op::AddItems(a) => Some((a.to.clone(), true)),
        Op::InsertItems(i) => Some((i.to.clone(), true)),
        Op::SetItems(s) => Some((s.owner.clone(), true)),
        Op::RemoveItems(r) => of(r.ids.first()),
        Op::MoveItems(m) => of(m.ids.first()),
        Op::ResizeItem(r) => of(Some(&r.id)),
        Op::RotateItems(r) => of(r.ids.first()),
        Op::SetFrames(s) => of(s.frames.first().map(|f| &f.id)),
        Op::SetItemProps(s) => of(Some(&s.id)),
        Op::Group(g) => of(g.ids.first()),
        Op::Ungroup(u) => of(Some(&u.id)),
        Op::Lock(f) | Op::Hide(f) => of(f.ids.first()),
        Op::Align(a) => of(a.ids.first()),
        Op::Distribute(d) => of(d.ids.first()),
        Op::MatchSize(m) => of(m.ids.first()),
        Op::DetachMaster(d) => Some((Owner::sheet(&d.sheet), true)),
        Op::SetPage(p) => Some((p.owner.clone(), false)),
        Op::AddVariant(a) => Some((a.owner.clone(), false)),
        Op::RemoveVariant(r) => Some((r.owner.clone(), false)),
        Op::SetVariant(v) => Some((v.owner.clone(), false)),
        _ => None,
    }
}

fn layout_mut<'a>(b: &'a mut SheetBook, owner: &Owner) -> Result<crate::variants::LayoutMut<'a>> {
    match owner.kind {
        OwnerKind::Sheet => Ok(crate::variants::LayoutMut::of_sheet(sheet_mut(
            b, &owner.id,
        )?)),
        OwnerKind::Master => Ok(crate::variants::LayoutMut::of_master(
            b.masters
                .iter_mut()
                .find(|m| m.id == owner.id)
                .ok_or_else(|| unknown_master(&owner.id))?,
        )),
    }
}

/// The whole sheet or master page as it is, as the operation that puts it back.
fn snapshot(b: &SheetBook, owner: &Owner) -> Option<Op> {
    match owner.kind {
        OwnerKind::Sheet => b
            .sheet(&owner.id)
            .map(|s| Op::ReplaceSheet(ReplaceSheet { sheet: s.clone() })),
        OwnerKind::Master => b
            .master(&owner.id)
            .map(|m| Op::ReplaceMaster(ReplaceMaster { master: m.clone() })),
    }
}

/// One operation with its inverse. On a sheet or master page that has layout
/// variants, an edit is written through to the variant in force (design
/// §3.2a) and the inverse is the owner as it was, which no write-through can
/// make inexact.
fn step(b: &mut SheetBook, op: &Op) -> Result<Vec<Op>> {
    let Some((owner, edit)) = layout_owner(b, op) else {
        return run(b, op);
    };
    let has_layouts = |b: &SheetBook| match owner.kind {
        OwnerKind::Sheet => b
            .sheet(&owner.id)
            .is_some_and(|s| !s.variants.is_empty() || s.active_variant.is_some()),
        OwnerKind::Master => b
            .master(&owner.id)
            .is_some_and(|m| !m.variants.is_empty() || m.active_variant.is_some()),
    };
    let variant_op = matches!(
        op,
        Op::AddVariant(_) | Op::RemoveVariant(_) | Op::SetVariant(_)
    );
    if !variant_op && !has_layouts(b) {
        return run(b, op);
    }
    let undo = snapshot(b, &owner).ok_or_else(|| unknown_sheet(&owner.id))?;
    let before = b.items_of(&owner).cloned().unwrap_or_default();
    run(b, op)?;
    if edit && let Some(page) = b.page_of(&owner).cloned() {
        crate::validate::group_frames(items_mut(b, &owner)?);
        crate::variants::write_through(&mut layout_mut(b, &owner)?, &before, &page);
    }
    Ok(vec![undo])
}

fn run(b: &mut SheetBook, op: &Op) -> Result<Vec<Op>> {
    match op {
        Op::AddSheet(a) => {
            let i = clamp_index(a.index, b.sheets.len());
            b.sheets.insert(i, a.sheet.clone());
            Ok(vec![Op::RemoveSheet(IdRef {
                id: a.sheet.id.clone(),
            })])
        }
        Op::RemoveSheet(r) => {
            let i = b.sheet_index(&r.id).ok_or_else(|| unknown_sheet(&r.id))?;
            let s = b.sheets.remove(i);
            Ok(vec![Op::AddSheet(AddSheet {
                sheet: s,
                index: Some(i as u32),
            })])
        }
        Op::RenameSheet(r) => {
            let s = sheet_mut(b, &r.id)?;
            let old = std::mem::replace(&mut s.name, r.name.clone());
            Ok(vec![Op::RenameSheet(Rename {
                id: r.id.clone(),
                name: old,
            })])
        }
        Op::MoveSheet(m) => {
            let from = b.sheet_index(&m.id).ok_or_else(|| unknown_sheet(&m.id))?;
            let s = b.sheets.remove(from);
            let to = (m.to as usize).min(b.sheets.len());
            b.sheets.insert(to, s);
            Ok(vec![Op::MoveSheet(MoveSheet {
                id: m.id.clone(),
                to: from as u32,
            })])
        }
        Op::DuplicateSheet(d) => {
            let i = b.sheet_index(&d.id).ok_or_else(|| unknown_sheet(&d.id))?;
            let mut s = b.sheets[i].clone();
            if d.item_ids.len() != s.items.len() {
                return Err(SheetError::new(
                    "ids_missing",
                    format!(
                        "Paftanın {} öğesi için {} kimlik verildi.",
                        s.items.len(),
                        d.item_ids.len()
                    ),
                ));
            }
            let map: BTreeMap<String, String> = s
                .items
                .iter()
                .zip(&d.item_ids)
                .map(|(it, n)| (it.id.clone(), n.clone()))
                .collect();
            crate::template::remap_items(&mut s.items, &map);
            crate::variants::remap(&mut s.variants, &mut s.base_layout, &map);
            s.id = d.new_id.clone();
            s.name = d.name.clone();
            let at = d.index.map_or(i + 1, |x| (x as usize).min(b.sheets.len()));
            b.sheets.insert(at, s);
            Ok(vec![Op::RemoveSheet(IdRef {
                id: d.new_id.clone(),
            })])
        }
        Op::ReplaceSheet(r) => {
            let s = sheet_mut(b, &r.sheet.id)?;
            let old = std::mem::replace(s, r.sheet.clone());
            Ok(vec![Op::ReplaceSheet(ReplaceSheet { sheet: old })])
        }
        Op::ReplaceMaster(r) => {
            let m = b
                .masters
                .iter_mut()
                .find(|m| m.id == r.master.id)
                .ok_or_else(|| unknown_master(&r.master.id))?;
            let old = std::mem::replace(m, r.master.clone());
            Ok(vec![Op::ReplaceMaster(ReplaceMaster { master: old })])
        }
        Op::SetPage(p) => set_page(b, p),
        Op::AddVariant(a) => add_variant(b, a),
        Op::RemoveVariant(r) => remove_variant(b, r),
        Op::SetVariant(v) => set_variant(b, v),
        Op::AddMaster(a) => {
            let i = clamp_index(a.index, b.masters.len());
            b.masters.insert(i, a.master.clone());
            Ok(vec![Op::RemoveMaster(IdRef {
                id: a.master.id.clone(),
            })])
        }
        Op::RemoveMaster(r) => {
            let i = b
                .masters
                .iter()
                .position(|m| m.id == r.id)
                .ok_or_else(|| unknown_master(&r.id))?;
            if let Some(s) = b
                .sheets
                .iter()
                .find(|s| s.master.as_deref() == Some(r.id.as_str()))
            {
                return Err(SheetError::new(
                    "master_in_use",
                    format!(
                        "Ana sayfa “{}” paftasında kullanılıyor; önce paftayı ondan ayırın.",
                        s.name
                    ),
                ));
            }
            let m = b.masters.remove(i);
            Ok(vec![Op::AddMaster(AddMaster {
                master: m,
                index: Some(i as u32),
            })])
        }
        Op::SetSheetMaster(s) => {
            if let Some(m) = &s.master
                && b.master(m).is_none()
            {
                return Err(unknown_master(m));
            }
            let sheet = sheet_mut(b, &s.sheet)?;
            let old = std::mem::replace(&mut sheet.master, s.master.clone());
            Ok(vec![Op::SetSheetMaster(SetSheetMaster {
                sheet: s.sheet.clone(),
                master: old,
            })])
        }
        Op::DetachMaster(d) => detach_master(b, d),
        Op::AddItems(a) => {
            let items = items_mut(b, &a.to)?;
            let at = clamp_index(a.index, items.len());
            for (k, it) in a.items.iter().enumerate() {
                items.insert(at + k, it.clone());
            }
            Ok(vec![Op::RemoveItems(ItemIds {
                ids: a.items.iter().map(|i| i.id.clone()).collect(),
            })])
        }
        Op::InsertItems(ins) => {
            let items = items_mut(b, &ins.to)?;
            let mut entries = ins.entries.clone();
            entries.sort_by_key(|e| e.index);
            for e in &entries {
                let at = (e.index as usize).min(items.len());
                items.insert(at, e.item.clone());
            }
            Ok(vec![Op::RemoveItems(ItemIds {
                ids: entries.iter().map(|e| e.item.id.clone()).collect(),
            })])
        }
        Op::RemoveItems(r) => remove_items(b, &r.ids),
        Op::MoveItems(m) => {
            let (owner, idx) = locate(b, &m.ids)?;
            let items = items_mut(b, &owner)?;
            unlocked(items, &idx)?;
            let all = with_descendants(items, &idx);
            translate(items, &all, i64::from(m.delta[0]), i64::from(m.delta[1]));
            Ok(vec![Op::MoveItems(MoveItems {
                ids: m.ids.clone(),
                delta: [m.delta[0].saturating_neg(), m.delta[1].saturating_neg()],
            })])
        }
        Op::ResizeItem(r) => resize(b, r),
        Op::RotateItems(r) => rotate_items(b, r),
        Op::SetFrames(s) => {
            let mut old = Vec::with_capacity(s.frames.len());
            for f in &s.frames {
                let (owner, i) = b.find_item(&f.id).ok_or_else(|| unknown_item(&f.id))?;
                let items = items_mut(b, &owner)?;
                let it = &mut items[i];
                if it.is_group() {
                    continue;
                }
                old.push(ItemFrame {
                    id: it.id.clone(),
                    frame: it.frame,
                    rotation: it.rotation,
                });
                it.frame = f.frame;
                it.rotation = norm_mdeg(i64::from(f.rotation));
            }
            Ok(vec![Op::SetFrames(SetFrames { frames: old })])
        }
        Op::SetItemProps(s) => set_props(b, s),
        Op::RenameItem(r) => {
            let (owner, i) = b.find_item(&r.id).ok_or_else(|| unknown_item(&r.id))?;
            let items = items_mut(b, &owner)?;
            let old = std::mem::replace(&mut items[i].name, r.name.clone());
            Ok(vec![Op::RenameItem(Rename {
                id: r.id.clone(),
                name: old,
            })])
        }
        Op::Reorder(r) => reorder(b, r),
        Op::SetOrder(s) => {
            let items = items_mut(b, &s.owner)?;
            let old: Vec<ItemId> = items.iter().map(|i| i.id.clone()).collect();
            let mut want = s.order.clone();
            let mut have = old.clone();
            want.sort();
            have.sort();
            if want != have {
                return Err(SheetError::new(
                    "bad_order",
                    "Yeni sıra paftanın öğelerinin hepsini birer kez saymalı.",
                ));
            }
            let mut taken: Vec<Option<Item>> = items.drain(..).map(Some).collect();
            for id in &s.order {
                if let Some(slot) = taken
                    .iter_mut()
                    .find(|t| t.as_ref().is_some_and(|x| x.id == *id))
                    && let Some(it) = slot.take()
                {
                    items.push(it);
                }
            }
            Ok(vec![Op::SetOrder(SetOrder {
                owner: s.owner.clone(),
                order: old,
            })])
        }
        Op::Group(g) => group(b, g),
        Op::Ungroup(u) => ungroup(b, &u.id),
        Op::SetItems(s) => {
            let items = items_mut(b, &s.owner)?;
            let old = std::mem::replace(items, s.items.clone());
            Ok(vec![Op::SetItems(SetItems {
                owner: s.owner.clone(),
                items: old,
            })])
        }
        Op::Lock(f) | Op::Hide(f) => {
            let lock = matches!(op, Op::Lock(_));
            let (owner, idx) = locate(b, &f.ids)?;
            let items = items_mut(b, &owner)?;
            let (mut was_on, mut was_off) = (Vec::new(), Vec::new());
            for &i in &idx {
                let it = &mut items[i];
                let flag = if lock { &mut it.locked } else { &mut it.hidden };
                if *flag {
                    was_on.push(it.id.clone());
                } else {
                    was_off.push(it.id.clone());
                }
                *flag = f.value;
            }
            let make = |ids: Vec<ItemId>, value: bool| {
                let f = SetFlag { ids, value };
                if lock { Op::Lock(f) } else { Op::Hide(f) }
            };
            let mut inv = Vec::new();
            if !was_on.is_empty() {
                inv.push(make(was_on, true));
            }
            if !was_off.is_empty() {
                inv.push(make(was_off, false));
            }
            Ok(inv)
        }
        Op::Align(a) => align(b, a),
        Op::Distribute(d) => distribute(b, d),
        Op::MatchSize(m) => match_size(b, m),
        Op::AddGuide(a) => {
            let gs = b
                .guides_of_mut(&a.owner)
                .ok_or_else(|| unknown_sheet(&a.owner.id))?;
            let i = clamp_index(a.index, gs.len());
            gs.insert(i, a.guide.clone());
            Ok(vec![Op::RemoveGuide(RemoveGuide {
                owner: a.owner.clone(),
                id: a.guide.id.clone(),
            })])
        }
        Op::MoveGuide(m) => {
            let gs = b
                .guides_of_mut(&m.owner)
                .ok_or_else(|| unknown_sheet(&m.owner.id))?;
            let g = gs.iter_mut().find(|g| g.id == m.id).ok_or_else(|| {
                SheetError::new("unknown_guide", format!("“{}” kılavuzu yok.", m.id))
            })?;
            if g.locked {
                return Err(SheetError::new(
                    "item_locked",
                    "Kılavuz kilitli; önce kilidini açın.",
                ));
            }
            let old = std::mem::replace(&mut g.at, m.at);
            Ok(vec![Op::MoveGuide(MoveGuide {
                owner: m.owner.clone(),
                id: m.id.clone(),
                at: old,
            })])
        }
        Op::RemoveGuide(r) => {
            let gs = b
                .guides_of_mut(&r.owner)
                .ok_or_else(|| unknown_sheet(&r.owner.id))?;
            let i = gs.iter().position(|g| g.id == r.id).ok_or_else(|| {
                SheetError::new("unknown_guide", format!("“{}” kılavuzu yok.", r.id))
            })?;
            let g = gs.remove(i);
            Ok(vec![Op::AddGuide(AddGuide {
                owner: r.owner.clone(),
                guide: g,
                index: Some(i as u32),
            })])
        }
        Op::SetSnapGrid(s) => {
            let sheet = sheet_mut(b, &s.sheet)?;
            let old = std::mem::replace(&mut sheet.snap_grid, s.grid);
            Ok(vec![Op::SetSnapGrid(SetSnapGrid {
                sheet: s.sheet.clone(),
                grid: old,
            })])
        }
        Op::SetAtlas(s) => {
            let sheet = sheet_mut(b, &s.sheet)?;
            let old = std::mem::replace(&mut sheet.atlas, s.atlas.clone());
            Ok(vec![Op::SetAtlas(SetAtlas {
                sheet: s.sheet.clone(),
                atlas: old,
            })])
        }
        Op::SetExport(s) => {
            let sheet = sheet_mut(b, &s.sheet)?;
            let old = std::mem::replace(&mut sheet.export, s.export.clone());
            Ok(vec![Op::SetExport(SetExport {
                sheet: s.sheet.clone(),
                export: old,
            })])
        }
        Op::ApplyTemplate(a) => apply_template(b, a),
        Op::SaveVariables(s) => {
            let list = match &s.sheet {
                Some(id) => &mut sheet_mut(b, id)?.variables,
                None => &mut b.variables,
            };
            let old = std::mem::replace(list, s.variables.clone());
            Ok(vec![Op::SaveVariables(SaveVariables {
                sheet: s.sheet.clone(),
                variables: old,
            })])
        }
        Op::AddAssets(a) => {
            let mut added = Vec::new();
            for m in &a.assets {
                if b.asset(&m.sha256).is_none() && !added.contains(&m.sha256) {
                    b.assets.push(m.clone());
                    added.push(m.sha256.clone());
                }
            }
            Ok(vec![Op::RemoveAssets(RemoveAssets { sha256: added })])
        }
        Op::RemoveAssets(r) => {
            let mut removed = Vec::new();
            for sha in &r.sha256 {
                let used = b
                    .sheets
                    .iter()
                    .map(|s| &s.items)
                    .chain(b.masters.iter().map(|m| &m.items));
                if used
                    .flat_map(|items| crate::template::used_assets(items))
                    .any(|u| u == *sha)
                {
                    return Err(SheetError::new(
                        "asset_in_use",
                        "Resim bir öğede kullanılıyor; önce o öğeyi silin ya da resmini değiştirin.",
                    ));
                }
                if let Some(i) = b.assets.iter().position(|a| a.sha256 == *sha) {
                    removed.push(b.assets.remove(i));
                }
            }
            Ok(if removed.is_empty() {
                Vec::new()
            } else {
                vec![Op::AddAssets(AddAssets { assets: removed })]
            })
        }
    }
}

fn set_page(b: &mut SheetBook, p: &SetPage) -> Result<Vec<Op>> {
    validate_page("page", &p.page)?;
    let old_page = b
        .page_of(&p.owner)
        .cloned()
        .ok_or_else(|| unknown_sheet(&p.owner.id))?;
    let before: Vec<Item> = items_mut(b, &p.owner)?.clone();
    let mut inv_frames = Vec::new();
    if p.relayout {
        let mut l = layout_mut(b, &p.owner)?;
        crate::variants::settle(&mut l, &old_page, &p.page);
        for (old, new) in before.iter().zip(l.items.iter()) {
            if !old.is_group() && old.frame != new.frame {
                inv_frames.push(ItemFrame {
                    id: old.id.clone(),
                    frame: old.frame,
                    rotation: old.rotation,
                });
            }
        }
    }
    match p.owner.kind {
        OwnerKind::Sheet => sheet_mut(b, &p.owner.id)?.page = p.page.clone(),
        OwnerKind::Master => {
            b.masters
                .iter_mut()
                .find(|m| m.id == p.owner.id)
                .ok_or_else(|| unknown_master(&p.owner.id))?
                .page = p.page.clone()
        }
    }
    let mut inv = vec![Op::SetPage(SetPage {
        owner: p.owner.clone(),
        page: old_page,
        relayout: false,
    })];
    if !inv_frames.is_empty() {
        inv.push(Op::SetFrames(SetFrames { frames: inv_frames }));
    }
    Ok(inv)
}

fn add_variant(b: &mut SheetBook, a: &AddVariant) -> Result<Vec<Op>> {
    let page = b
        .page_of(&a.owner)
        .cloned()
        .ok_or_else(|| unknown_sheet(&a.owner.id))?;
    let l = layout_mut(b, &a.owner)?;
    if l.variants.iter().any(|v| v.id == a.id) {
        return Err(SheetError::new(
            "duplicate_id",
            format!("“{}” yerleşim düzeni zaten var.", a.id),
        ));
    }
    let v = LayoutVariant {
        id: a.id.clone(),
        name: a.name.clone(),
        when: a.when.clone(),
        reference: page.size,
        frames: crate::variants::record(l.items),
    };
    let at = clamp_index(Some(a.index.unwrap_or(0)), l.variants.len());
    l.variants.insert(at, v);
    // In force at once when the paper calls for it: the arrangement is already the variant's.
    if crate::variants::choose(l.variants, &page).is_some_and(|v| v.id == a.id) {
        if l.active.is_none() {
            *l.base = Some(BaseLayout {
                reference: page.size,
                frames: crate::variants::record(l.items),
            });
        }
        *l.active = Some(a.id.clone());
    }
    Ok(Vec::new())
}

fn remove_variant(b: &mut SheetBook, r: &RemoveVariant) -> Result<Vec<Op>> {
    let page = b
        .page_of(&r.owner)
        .cloned()
        .ok_or_else(|| unknown_sheet(&r.owner.id))?;
    let mut l = layout_mut(b, &r.owner)?;
    let i = l
        .variants
        .iter()
        .position(|v| v.id == r.id)
        .ok_or_else(|| {
            SheetError::new(
                "unknown_variant",
                format!("“{}” yerleşim düzeni yok.", r.id),
            )
        })?;
    l.variants.remove(i);
    if l.active.as_deref() == Some(r.id.as_str()) {
        let target = crate::variants::choose(l.variants, &page).map(|v| v.id.clone());
        crate::variants::switch_to(&mut l, target.as_deref(), &page, &page);
    }
    Ok(Vec::new())
}

fn set_variant(b: &mut SheetBook, s: &SetVariant) -> Result<Vec<Op>> {
    let l = layout_mut(b, &s.owner)?;
    let i = l
        .variants
        .iter()
        .position(|v| v.id == s.id)
        .ok_or_else(|| {
            SheetError::new(
                "unknown_variant",
                format!("“{}” yerleşim düzeni yok.", s.id),
            )
        })?;
    let mut v = l.variants.remove(i);
    if let Some(n) = &s.name {
        v.name = n.clone();
    }
    if let Some(w) = &s.when {
        v.when = w.clone();
    }
    let at = clamp_index(Some(s.index.map_or(i as u32, |x| x)), l.variants.len());
    l.variants.insert(at, v);
    Ok(Vec::new())
}

/// A name not yet used in `taken`: the name itself, or with “ 2”, “ 3” … after it.
fn free_name(name: &str, taken: &BTreeSet<String>) -> String {
    if !taken.contains(name) {
        return name.to_owned();
    }
    (2..)
        .map(|n| format!("{name} {n}"))
        .find(|c| !taken.contains(c))
        .unwrap_or_else(|| name.to_owned())
}

fn detach_master(b: &mut SheetBook, d: &DetachMaster) -> Result<Vec<Op>> {
    let sheet = b.sheet(&d.sheet).ok_or_else(|| unknown_sheet(&d.sheet))?;
    let mid = sheet
        .master
        .clone()
        .ok_or_else(|| SheetError::new("no_master", "Bu paftanın ana sayfası yok."))?;
    let master = b.master(&mid).ok_or_else(|| unknown_master(&mid))?.clone();
    if d.item_ids.len() != master.items.len() {
        return Err(SheetError::new(
            "ids_missing",
            format!(
                "Ana sayfanın {} öğesi için {} kimlik verildi.",
                master.items.len(),
                d.item_ids.len()
            ),
        ));
    }
    let mut copies = crate::variants::arranged(&master, &sheet.page);
    let map: BTreeMap<String, String> = master
        .items
        .iter()
        .zip(&d.item_ids)
        .map(|(it, n)| (it.id.clone(), n.clone()))
        .collect();
    crate::template::remap_items(&mut copies, &map);
    let mut taken: BTreeSet<String> = sheet.items.iter().map(|i| i.name.clone()).collect();
    for c in &mut copies {
        c.name = free_name(&c.name, &taken);
        taken.insert(c.name.clone());
    }
    let s = sheet_mut(b, &d.sheet)?;
    for (k, c) in copies.into_iter().enumerate() {
        s.items.insert(k, c);
    }
    s.master = None;
    Ok(vec![
        Op::RemoveItems(ItemIds {
            ids: d.item_ids.clone(),
        }),
        Op::SetSheetMaster(SetSheetMaster {
            sheet: d.sheet.clone(),
            master: Some(mid),
        }),
    ])
}

fn remove_items(b: &mut SheetBook, ids: &[ItemId]) -> Result<Vec<Op>> {
    let (owner, idx) = locate(b, ids)?;
    let items = items_mut(b, &owner)?;
    let all = with_descendants(items, &idx);
    let gone: BTreeSet<&str> = all
        .iter()
        .filter_map(|&i| items.get(i))
        .map(|i| i.id.as_str())
        .collect();
    // A removed table frame another table still continues in would break that table.
    for it in items.iter().filter(|it| !gone.contains(it.id.as_str())) {
        let overflow = match &it.kind {
            ItemKind::Table(t) => Some(&t.overflow),
            ItemKind::CoordinateList(c) => Some(&c.overflow),
            _ => None,
        };
        if let Some(Overflow::ContinueIn(c)) = overflow
            && c.items.iter().any(|t| gone.contains(t.as_str()))
        {
            return Err(SheetError::new(
                "item_in_use",
                format!(
                    "“{}” tablosu silinecek çerçeveye devam ediyor; önce onun taşma ayarını değiştirin.",
                    it.name
                ),
            ));
        }
    }
    let mut entries = Vec::with_capacity(all.len());
    for &i in all.iter().rev() {
        entries.push(IndexedItem {
            index: i as u32,
            item: items.remove(i),
        });
    }
    entries.reverse();
    Ok(vec![Op::InsertItems(InsertItems { to: owner, entries })])
}

fn resize(b: &mut SheetBook, r: &ResizeItem) -> Result<Vec<Op>> {
    let (owner, i) = b.find_item(&r.id).ok_or_else(|| unknown_item(&r.id))?;
    let items = items_mut(b, &owner)?;
    unlocked(items, &[i])?;
    let it = items[i].clone();
    let f = it.frame;
    let c = f.center();
    let p = rotate([f64::from(r.to[0]), f64::from(r.to[1])], c, -it.rotation);
    let (hx, hy) = r.handle.dirs();
    let min = f64::from(ITEM_MIN);
    let (mut l, mut t, mut rr, mut bb) = (
        f64::from(f.left),
        f64::from(f.top),
        f.right() as f64,
        f.bottom() as f64,
    );
    if r.from_center {
        if hx != 0 {
            let half = (p[0] - c[0]).abs().max(min / 2.0);
            l = c[0] - half;
            rr = c[0] + half;
        }
        if hy != 0 {
            let half = (p[1] - c[1]).abs().max(min / 2.0);
            t = c[1] - half;
            bb = c[1] + half;
        }
    } else {
        if hx == 1 {
            rr = p[0].max(l + min);
        } else if hx == -1 {
            l = p[0].min(rr - min);
        }
        if hy == 1 {
            bb = p[1].max(t + min);
        } else if hy == -1 {
            t = p[1].min(bb - min);
        }
    }
    if r.keep_aspect && f.width > 0 && f.height > 0 {
        let ratio = f64::from(f.width) / f64::from(f.height);
        let (w, h) = (rr - l, bb - t);
        let (nw, nh) = if hx != 0 && hy != 0 {
            let s = (w / f64::from(f.width)).max(h / f64::from(f.height));
            (f64::from(f.width) * s, f64::from(f.height) * s)
        } else if hx != 0 {
            (w, w / ratio)
        } else {
            (h * ratio, h)
        };
        let (nw, nh) = (nw.max(min), nh.max(min));
        // The moving edges take the change; an edge handle grows the other axis about its middle.
        if r.from_center || hx == 0 {
            let cx = (l + rr) / 2.0;
            l = cx - nw / 2.0;
            rr = cx + nw / 2.0;
        } else if hx == 1 {
            rr = l + nw;
        } else {
            l = rr - nw;
        }
        if r.from_center || hy == 0 {
            let cy = (t + bb) / 2.0;
            t = cy - nh / 2.0;
            bb = cy + nh / 2.0;
        } else if hy == 1 {
            bb = t + nh;
        } else {
            t = bb - nh;
        }
    }
    // The new local rectangle, turned about the old centre: its fixed edge stays where it was.
    let nc = rotate([(l + rr) / 2.0, (t + bb) / 2.0], c, it.rotation);
    let (w, h) = (
        round_i64(rr - l).max(i64::from(ITEM_MIN)),
        round_i64(bb - t).max(i64::from(ITEM_MIN)),
    );
    let nf = RectUm::new(
        round_um(nc[0] - w as f64 / 2.0),
        round_um(nc[1] - h as f64 / 2.0),
        sat(w),
        sat(h),
    );
    let mut old = frames_of(items, &[i]);
    if it.is_group() {
        let kids = descendants(items, &it.id);
        old = frames_of(items, &kids);
        let c = crate::model::Constraints {
            h: HConstraint::Scale,
            v: VConstraint::Scale,
            relative_to: ConstraintBox::Group,
        };
        for k in kids {
            if !items[k].is_group() {
                items[k].frame = layout::relayout_frame(&items[k].frame, &c, &f, &nf);
            }
        }
    } else {
        items[i].frame = nf;
    }
    Ok(vec![Op::SetFrames(SetFrames { frames: old })])
}

fn rotate_items(b: &mut SheetBook, r: &RotateItems) -> Result<Vec<Op>> {
    let (owner, idx) = locate(b, &r.ids)?;
    let items = items_mut(b, &owner)?;
    unlocked(items, &idx)?;
    let all = with_descendants(items, &idx);
    let leaves: Vec<usize> = all
        .iter()
        .copied()
        .filter(|&i| !items[i].is_group())
        .collect();
    let old = frames_of(items, &leaves);
    let around: Option<[f64; 2]> = match (r.around, units(items, &idx).as_slice()) {
        (Some(p), _) => Some([f64::from(p[0]), f64::from(p[1])]),
        (None, [one]) if !items[*one].is_group() => None,
        (None, us) => {
            let mut u: Option<RectUm> = None;
            for &i in us {
                let bx = bounds(&items[i]);
                u = Some(u.map_or(bx, |x| x.union(&bx)));
            }
            u.map(|x| x.center())
        }
    };
    for &i in &leaves {
        let it = &mut items[i];
        if let Some(o) = around {
            let nc = rotate(it.frame.center(), o, r.angle);
            it.frame.left = round_um(nc[0] - f64::from(it.frame.width) / 2.0);
            it.frame.top = round_um(nc[1] - f64::from(it.frame.height) / 2.0);
        }
        it.rotation = norm_mdeg(i64::from(it.rotation) + i64::from(r.angle));
    }
    Ok(vec![Op::SetFrames(SetFrames { frames: old })])
}

fn set_props(b: &mut SheetBook, s: &SetItemProps) -> Result<Vec<Op>> {
    let Some(patch) = s.patch.as_object() else {
        return Err(SheetError::new(
            "bad_patch",
            "Özellik yaması bir JSON nesnesi olmalı.",
        ));
    };
    for key in ["id", "group"] {
        if patch.contains_key(key) {
            return Err(SheetError::new(
                "bad_patch",
                format!(
                    "“{key}” yamayla değişmez (kimlik sabittir; grup için Grupla ve Grubu çöz)."
                ),
            ));
        }
    }
    let (owner, i) = b.find_item(&s.id).ok_or_else(|| unknown_item(&s.id))?;
    let items = items_mut(b, &owner)?;
    let old = serde_json::to_value(&items[i]).map_err(|e| SheetError::json("Öğe", &e))?;
    if let (Some(new_type), Some(old_type)) = (
        s.patch.get("kind").and_then(|k| k.get("type")),
        old.get("kind").and_then(|k| k.get("type")),
    ) && new_type != old_type
    {
        return Err(SheetError::new(
            "kind_change",
            "Öğenin türü yamayla değişmez: yeni türde bir öğe ekleyin.",
        ));
    }
    let mut merged = old.clone();
    crate::json::merge(&mut merged, &s.patch);
    let item: Item = serde_json::from_value(merged).map_err(|e| {
        SheetError::new(
            "bad_patch",
            format!("Yama öğeye uymuyor: {e}. Alan adlarını ve türlerini denetleyin."),
        )
    })?;
    items[i] = item;
    Ok(vec![Op::SetItemProps(SetItemProps {
        id: s.id.clone(),
        patch: crate::json::reverse(&old, &s.patch),
    })])
}

fn reorder(b: &mut SheetBook, r: &Reorder) -> Result<Vec<Op>> {
    let (owner, idx) = locate(b, &r.ids)?;
    let items = items_mut(b, &owner)?;
    let old: Vec<ItemId> = items.iter().map(|i| i.id.clone()).collect();
    let sel: BTreeSet<usize> = with_descendants(items, &idx).into_iter().collect();
    let mut order: Vec<usize> = (0..items.len()).collect();
    match r.to {
        ReorderTo::Front => {
            order.sort_by_key(|i| (sel.contains(i), *i));
        }
        ReorderTo::Back => {
            order.sort_by_key(|i| (!sel.contains(i), *i));
        }
        ReorderTo::Forward => {
            // Each selected run moves above the next unselected item, from the top down.
            let mut k = order.len();
            while k > 0 {
                k -= 1;
                if sel.contains(&order[k]) && k + 1 < order.len() && !sel.contains(&order[k + 1]) {
                    let mut start = k;
                    while start > 0 && sel.contains(&order[start - 1]) {
                        start -= 1;
                    }
                    let next = order.remove(k + 1);
                    order.insert(start, next);
                    k = start;
                }
            }
        }
        ReorderTo::Backward => {
            let mut k = 0;
            while k < order.len() {
                if sel.contains(&order[k]) && k > 0 && !sel.contains(&order[k - 1]) {
                    let mut end = k;
                    while end + 1 < order.len() && sel.contains(&order[end + 1]) {
                        end += 1;
                    }
                    let prev = order.remove(k - 1);
                    order.insert(end, prev);
                    k = end + 1;
                } else {
                    k += 1;
                }
            }
        }
    }
    let mut taken: Vec<Option<Item>> = items.drain(..).map(Some).collect();
    for i in order {
        if let Some(it) = taken.get_mut(i).and_then(Option::take) {
            items.push(it);
        }
    }
    Ok(vec![Op::SetOrder(SetOrder { owner, order: old })])
}

fn group(b: &mut SheetBook, g: &GroupItems) -> Result<Vec<Op>> {
    if !g.group.is_group() {
        return Err(SheetError::new(
            "bad_group",
            "Grup öğesinin türü “group” olmalı.",
        ));
    }
    let (owner, idx) = locate(b, &g.ids)?;
    let items = items_mut(b, &owner)?;
    let parent = items[idx[0]].group.clone();
    if idx.iter().any(|&i| items[i].group != parent) {
        return Err(SheetError::new(
            "bad_group",
            "Gruplanacak öğeler aynı grupta (ya da hiçbir grupta) olmalı.",
        ));
    }
    for &i in &idx {
        items[i].group = Some(g.group.id.clone());
    }
    let mut gi = g.group.clone();
    gi.group = parent;
    gi.rotation = 0;
    let top = idx.iter().copied().max().unwrap_or(0);
    let at = g.index.map_or(top + 1, |x| (x as usize).min(items.len()));
    items.insert(at, gi);
    Ok(vec![Op::Ungroup(IdRef {
        id: g.group.id.clone(),
    })])
}

fn ungroup(b: &mut SheetBook, id: &str) -> Result<Vec<Op>> {
    let (owner, i) = b.find_item(id).ok_or_else(|| unknown_item(id))?;
    let items = items_mut(b, &owner)?;
    if !items[i].is_group() {
        return Err(SheetError::new(
            "bad_group",
            format!("“{}” bir grup değil.", items[i].name),
        ));
    }
    let before = items.clone();
    let parent = items[i].group.clone();
    for it in items.iter_mut() {
        if it.group.as_deref() == Some(id) {
            it.group = parent.clone();
            if parent.is_none() && it.constraints.relative_to == ConstraintBox::Group {
                it.constraints.relative_to = ConstraintBox::Margins;
            }
        }
    }
    items.remove(i);
    Ok(vec![Op::SetItems(SetItems {
        owner,
        items: before,
    })])
}

fn align(b: &mut SheetBook, a: &Align) -> Result<Vec<Op>> {
    let (owner, idx) = locate(b, &a.ids)?;
    let page = b
        .page_of(&owner)
        .cloned()
        .ok_or_else(|| unknown_sheet(&owner.id))?;
    let key_bounds = match &a.key {
        Some(k) => Some(bounds(b.item(k).ok_or_else(|| unknown_item(k))?)),
        None => None,
    };
    let items = items_mut(b, &owner)?;
    let us = units(items, &idx);
    let target = match a.to {
        AlignTo::Page => page.rect(),
        AlignTo::Margins => page.margin_rect(),
        AlignTo::KeyItem => key_bounds
            .or_else(|| idx.first().map(|&i| bounds(&items[i])))
            .ok_or_else(|| {
                SheetError::new("no_items", "Hizalama için bir anahtar öğe gerekiyor.")
            })?,
        AlignTo::Selection => {
            let mut u: Option<RectUm> = None;
            for &i in &us {
                let bx = bounds(&items[i]);
                u = Some(u.map_or(bx, |x| x.union(&bx)));
            }
            u.ok_or_else(|| SheetError::new("no_items", "Hizalanacak öğe yok."))?
        }
    };
    let all = with_descendants(items, &us);
    unlocked(items, &us)?;
    let old = frames_of(items, &all);
    for &u in &us {
        let bx = bounds(&items[u]);
        // Twice the coordinates keep centres whole.
        let (dx2, dy2) = match a.edge {
            AlignEdge::Left => (2 * (i64::from(target.left) - i64::from(bx.left)), 0),
            AlignEdge::Right => (2 * (target.right() - bx.right()), 0),
            AlignEdge::Center => (
                (2 * i64::from(target.left) + i64::from(target.width))
                    - (2 * i64::from(bx.left) + i64::from(bx.width)),
                0,
            ),
            AlignEdge::Top => (0, 2 * (i64::from(target.top) - i64::from(bx.top))),
            AlignEdge::Bottom => (0, 2 * (target.bottom() - bx.bottom())),
            AlignEdge::Middle => (
                0,
                (2 * i64::from(target.top) + i64::from(target.height))
                    - (2 * i64::from(bx.top) + i64::from(bx.height)),
            ),
        };
        let (dx, dy) = (round_i64(dx2 as f64 / 2.0), round_i64(dy2 as f64 / 2.0));
        let mut moving = vec![u];
        if items[u].is_group() {
            moving.extend(descendants(items, &items[u].id));
        }
        translate(items, &moving, dx, dy);
    }
    Ok(vec![Op::SetFrames(SetFrames { frames: old })])
}

fn distribute(b: &mut SheetBook, d: &Distribute) -> Result<Vec<Op>> {
    let (owner, idx) = locate(b, &d.ids)?;
    let items = items_mut(b, &owner)?;
    let mut us = units(items, &idx);
    if us.len() < 3 {
        return Err(SheetError::new(
            "no_items",
            "Dağıtmak için en az üç öğe seçilmeli.",
        ));
    }
    unlocked(items, &us)?;
    let horiz = d.axis == Axis::X;
    let start = |r: &RectUm| {
        if horiz {
            i64::from(r.left)
        } else {
            i64::from(r.top)
        }
    };
    let size = |r: &RectUm| {
        if horiz {
            i64::from(r.width)
        } else {
            i64::from(r.height)
        }
    };
    us.sort_by(|&a, &c| {
        let (ba, bc) = (bounds(&items[a]), bounds(&items[c]));
        start(&ba)
            .cmp(&start(&bc))
            .then_with(|| items[a].id.cmp(&items[c].id))
    });
    let bs: Vec<RectUm> = us.iter().map(|&i| bounds(&items[i])).collect();
    let n = bs.len();
    let all = with_descendants(items, &us);
    let old = frames_of(items, &all);
    let first = &bs[0];
    let last = &bs[n - 1];
    for k in 1..n - 1 {
        let r = &bs[k];
        let target_start = match d.mode {
            DistributeMode::Centers => {
                let c0 = 2 * start(first) + size(first);
                let c1 = 2 * start(last) + size(last);
                let c = c0 as f64 + (c1 - c0) as f64 * k as f64 / (n - 1) as f64;
                round_i64((c - size(r) as f64) / 2.0)
            }
            DistributeMode::Gaps => {
                let span = start(last) + size(last) - start(first);
                let total: i64 = bs.iter().map(size).sum();
                let gap = (span - total) as f64 / (n - 1) as f64;
                let before: i64 = bs[..k].iter().map(size).sum();
                start(first) + before + round_i64(gap * k as f64)
            }
        };
        let delta = target_start - start(r);
        let u = us[k];
        let mut moving = vec![u];
        if items[u].is_group() {
            moving.extend(descendants(items, &items[u].id));
        }
        if horiz {
            translate(items, &moving, delta, 0);
        } else {
            translate(items, &moving, 0, delta);
        }
    }
    Ok(vec![Op::SetFrames(SetFrames { frames: old })])
}

fn match_size(b: &mut SheetBook, m: &MatchSize) -> Result<Vec<Op>> {
    let (owner, idx) = locate(b, &m.ids)?;
    let key = match &m.key {
        Some(k) => b.item(k).ok_or_else(|| unknown_item(k))?.frame,
        None => {
            b.item(&m.ids[0])
                .ok_or_else(|| unknown_item(&m.ids[0]))?
                .frame
        }
    };
    let items = items_mut(b, &owner)?;
    unlocked(items, &idx)?;
    if idx.iter().any(|&i| items[i].is_group()) {
        return Err(SheetError::new(
            "bad_group",
            "Aynı boy grupta değil, öğelerinde yapılır: grubu çözün ya da öğelerini seçin.",
        ));
    }
    let old = frames_of(items, &idx);
    for &i in &idx {
        let f = &mut items[i].frame;
        if m.dimension != SizeDimension::Height {
            f.width = key.width;
        }
        if m.dimension != SizeDimension::Width {
            f.height = key.height;
        }
    }
    Ok(vec![Op::SetFrames(SetFrames { frames: old })])
}

fn apply_template(b: &mut SheetBook, a: &ApplyTemplate) -> Result<Vec<Op>> {
    let old = b
        .sheet(&a.sheet)
        .cloned()
        .ok_or_else(|| unknown_sheet(&a.sheet))?;
    let mut ids = a.ids.clone();
    ids.sheet = old.id.clone();
    let inst = instantiate(&a.template, &ids, &a.options)?;
    let mut sheet = inst.sheet;
    sheet.name = a.options.name.clone().unwrap_or_else(|| old.name.clone());
    let mut inv = vec![Op::ReplaceSheet(ReplaceSheet { sheet: old })];
    if let Some(m) = inst.master {
        if b.master(&m.id).is_some() {
            return Err(SheetError::new(
                "duplicate_id",
                format!("“{}” ana sayfa kimliği kitapta var.", m.id),
            ));
        }
        inv.push(Op::RemoveMaster(IdRef { id: m.id.clone() }));
        b.masters.push(m);
    }
    let mut added = Vec::new();
    for meta in inst.assets {
        if b.asset(&meta.sha256).is_none() {
            added.push(meta.sha256.clone());
            b.assets.push(meta);
        }
    }
    if !added.is_empty() {
        inv.push(Op::RemoveAssets(RemoveAssets { sha256: added }));
    }
    *sheet_mut(b, &a.sheet)? = sheet;
    Ok(inv)
}

/// The undo step's name: a verb and what it is done to.
pub fn describe(b: &SheetBook, op: &Op) -> String {
    let named = |ids: &[ItemId]| -> String {
        match ids {
            [one] => b.item(one).map_or_else(|| one.clone(), |i| i.name.clone()),
            _ => format!("{} öğe", ids.len()),
        }
    };
    let sheet_name = |id: &str| {
        b.sheet(id)
            .map_or_else(|| id.to_owned(), |s| s.name.clone())
    };
    let variant_name = |owner: &Owner, id: &str| -> String {
        let vs = match owner.kind {
            OwnerKind::Sheet => b.sheet(&owner.id).map(|s| &s.variants),
            OwnerKind::Master => b.master(&owner.id).map(|m| &m.variants),
        };
        vs.and_then(|vs| vs.iter().find(|v| v.id == id))
            .map_or_else(|| id.to_owned(), |v| v.name.clone())
    };
    match op {
        Op::AddSheet(a) => format!("Pafta ekle: {}", a.sheet.name),
        Op::RemoveSheet(r) => format!("Paftayı sil: {}", sheet_name(&r.id)),
        Op::RenameSheet(r) => format!("Paftaya ad ver: {}", r.name),
        Op::MoveSheet(m) => format!("Paftayı taşı: {}", sheet_name(&m.id)),
        Op::DuplicateSheet(d) => format!("Paftayı çoğalt: {}", sheet_name(&d.id)),
        Op::ReplaceSheet(r) => format!("Paftayı değiştir: {}", r.sheet.name),
        Op::SetPage(_) => "Sayfa ayarları".to_owned(),
        Op::AddMaster(a) => format!("Ana sayfa ekle: {}", a.master.name),
        Op::RemoveMaster(_) => "Ana sayfayı sil".to_owned(),
        Op::ReplaceMaster(r) => format!("Ana sayfayı değiştir: {}", r.master.name),
        Op::SetSheetMaster(_) => "Ana sayfa seç".to_owned(),
        Op::DetachMaster(_) => "Ana sayfadan ayır".to_owned(),
        Op::AddItems(a) => match a.items.as_slice() {
            [one] => format!("Ekle: {}", one.name),
            many => format!("Ekle: {} öğe", many.len()),
        },
        Op::InsertItems(i) => format!("Geri koy: {} öğe", i.entries.len()),
        Op::RemoveItems(r) => format!("Sil: {}", named(&r.ids)),
        Op::MoveItems(m) => format!("Taşı: {}", named(&m.ids)),
        Op::ResizeItem(r) => format!("Boyutlandır: {}", named(std::slice::from_ref(&r.id))),
        Op::RotateItems(r) => format!("Döndür: {}", named(&r.ids)),
        Op::SetFrames(s) => format!(
            "Yerleştir: {}",
            named(&s.frames.iter().map(|f| f.id.clone()).collect::<Vec<_>>())
        ),
        Op::SetItemProps(s) => format!("Değiştir: {}", named(std::slice::from_ref(&s.id))),
        Op::RenameItem(r) => format!("Ad ver: {}", r.name),
        Op::Reorder(r) => match r.to {
            ReorderTo::Front => "Öne getir".to_owned(),
            ReorderTo::Back => "Arkaya gönder".to_owned(),
            ReorderTo::Forward => "Bir öne".to_owned(),
            ReorderTo::Backward => "Bir arkaya".to_owned(),
        },
        Op::SetOrder(_) => "Sırayı değiştir".to_owned(),
        Op::Group(g) => format!("Grupla: {}", g.group.name),
        Op::Ungroup(u) => format!("Grubu çöz: {}", named(std::slice::from_ref(&u.id))),
        Op::SetItems(_) => "Öğeleri geri yükle".to_owned(),
        Op::Lock(f) => format!(
            "{}: {}",
            if f.value { "Kilitle" } else { "Kilidi aç" },
            named(&f.ids)
        ),
        Op::Hide(f) => format!(
            "{}: {}",
            if f.value { "Gizle" } else { "Göster" },
            named(&f.ids)
        ),
        Op::Align(a) => format!("Hizala: {}", named(&a.ids)),
        Op::Distribute(d) => format!("Dağıt: {}", named(&d.ids)),
        Op::MatchSize(m) => format!("Aynı boy: {}", named(&m.ids)),
        Op::AddGuide(_) => "Kılavuz ekle".to_owned(),
        Op::MoveGuide(_) => "Kılavuzu taşı".to_owned(),
        Op::RemoveGuide(_) => "Kılavuzu sil".to_owned(),
        Op::SetSnapGrid(_) => "Izgara".to_owned(),
        Op::SetAtlas(_) => "Atlas".to_owned(),
        Op::SetExport(_) => "Dışa aktarma ayarları".to_owned(),
        Op::ApplyTemplate(a) => format!("Şablon uygula: {}", a.template.meta.name),
        Op::SaveVariables(_) => "Değişkenler".to_owned(),
        Op::AddAssets(_) => "Resim ekle".to_owned(),
        Op::RemoveAssets(_) => "Resim kaldır".to_owned(),
        Op::AddVariant(a) => format!("Yerleşim düzeni ekle: {}", a.name),
        Op::RemoveVariant(r) => format!("Yerleşim düzenini sil: {}", variant_name(&r.owner, &r.id)),
        Op::SetVariant(v) => format!(
            "Yerleşim düzeni: {}",
            v.name
                .clone()
                .unwrap_or_else(|| variant_name(&v.owner, &v.id))
        ),
    }
}
