//! The sheet book, `kentos.sheet/1` (design §3): a project's sheets, their
//! master pages, the metadata of the pictures they use and the project's
//! variables. Fields may be added; an existing field never changes meaning.
//! JSON is camelCase; an unknown field is an error, never dropped.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::kinds::ItemKind;
use crate::style::Stroke;
use crate::units::{Margins, Mdeg, RectUm, SizeUm, Um};

pub const BOOK_SCHEMA: &str = "kentos.sheet/1";

/// An item's persistent key: links (a scale bar's map, a table's continuation) hold it. Unique in a book.
pub type ItemId = String;
pub type SheetId = String;
pub type MasterId = String;

pub(crate) fn yes() -> bool {
    true
}

fn full() -> u8 {
    100
}

/// Every sheet of a project.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct SheetBook {
    /// `kentos.sheet/1`.
    pub schema: String,
    #[serde(default)]
    pub sheets: Vec<Sheet>,
    /// Master pages (ana sayfalar).
    #[serde(default)]
    pub masters: Vec<Master>,
    /// Pictures by the SHA-256 of their bytes; the bytes are the host's asset store's.
    #[serde(default)]
    pub assets: Vec<AssetMeta>,
    /// The project's own variables (`@proje_no`, `@idare` …).
    #[serde(default)]
    pub variables: Vec<Variable>,
}

impl Default for SheetBook {
    fn default() -> Self {
        SheetBook {
            schema: BOOK_SCHEMA.to_owned(),
            sheets: Vec::new(),
            masters: Vec::new(),
            assets: Vec::new(),
            variables: Vec::new(),
        }
    }
}

/// Whose items: a sheet's or a master page's.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct Owner {
    pub kind: OwnerKind,
    pub id: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum OwnerKind {
    Sheet,
    Master,
}

impl Owner {
    pub fn sheet(id: &str) -> Owner {
        Owner {
            kind: OwnerKind::Sheet,
            id: id.to_owned(),
        }
    }

    pub fn master(id: &str) -> Owner {
        Owner {
            kind: OwnerKind::Master,
            id: id.to_owned(),
        }
    }
}

impl SheetBook {
    /// The items of a sheet or a master page.
    pub fn items_of(&self, owner: &Owner) -> Option<&Vec<Item>> {
        match owner.kind {
            OwnerKind::Sheet => self
                .sheets
                .iter()
                .find(|s| s.id == owner.id)
                .map(|s| &s.items),
            OwnerKind::Master => self
                .masters
                .iter()
                .find(|m| m.id == owner.id)
                .map(|m| &m.items),
        }
    }

    pub fn items_of_mut(&mut self, owner: &Owner) -> Option<&mut Vec<Item>> {
        match owner.kind {
            OwnerKind::Sheet => self
                .sheets
                .iter_mut()
                .find(|s| s.id == owner.id)
                .map(|s| &mut s.items),
            OwnerKind::Master => self
                .masters
                .iter_mut()
                .find(|m| m.id == owner.id)
                .map(|m| &mut m.items),
        }
    }

    /// The guides of a sheet or a master page.
    pub fn guides_of_mut(&mut self, owner: &Owner) -> Option<&mut Vec<Guide>> {
        match owner.kind {
            OwnerKind::Sheet => self
                .sheets
                .iter_mut()
                .find(|s| s.id == owner.id)
                .map(|s| &mut s.guides),
            OwnerKind::Master => self
                .masters
                .iter_mut()
                .find(|m| m.id == owner.id)
                .map(|m| &mut m.guides),
        }
    }

    /// The page an owner's frames are given on.
    pub fn page_of(&self, owner: &Owner) -> Option<&Page> {
        match owner.kind {
            OwnerKind::Sheet => self.sheet(&owner.id).map(|s| &s.page),
            OwnerKind::Master => self.master(&owner.id).map(|m| &m.page),
        }
    }

    /// Every owner, sheets first, in order.
    pub fn owners(&self) -> Vec<Owner> {
        self.sheets
            .iter()
            .map(|s| Owner::sheet(&s.id))
            .chain(self.masters.iter().map(|m| Owner::master(&m.id)))
            .collect()
    }

    /// Where an item is: its owner and its index there.
    pub fn find_item(&self, id: &str) -> Option<(Owner, usize)> {
        for s in &self.sheets {
            if let Some(i) = s.items.iter().position(|it| it.id == id) {
                return Some((Owner::sheet(&s.id), i));
            }
        }
        for m in &self.masters {
            if let Some(i) = m.items.iter().position(|it| it.id == id) {
                return Some((Owner::master(&m.id), i));
            }
        }
        None
    }

    pub fn item(&self, id: &str) -> Option<&Item> {
        let (owner, i) = self.find_item(id)?;
        self.items_of(&owner)?.get(i)
    }

    pub fn sheet(&self, id: &str) -> Option<&Sheet> {
        self.sheets.iter().find(|s| s.id == id)
    }

    pub fn sheet_index(&self, id: &str) -> Option<usize> {
        self.sheets.iter().position(|s| s.id == id)
    }

    pub fn master(&self, id: &str) -> Option<&Master> {
        self.masters.iter().find(|m| m.id == id)
    }

    pub fn asset(&self, sha256: &str) -> Option<&AssetMeta> {
        self.assets.iter().find(|a| a.sha256 == sha256)
    }
}

/// One sheet (pafta): its paper, items, guides and how it is exported.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct Sheet {
    pub id: SheetId,
    pub name: String,
    pub page: Page,
    /// The master page drawn under this sheet's items.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub master: Option<MasterId>,
    /// Drawing order: the first is at the bottom.
    #[serde(default)]
    pub items: Vec<Item>,
    #[serde(default)]
    pub guides: Vec<Guide>,
    #[serde(default)]
    pub snap_grid: SnapGrid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub atlas: Option<Atlas>,
    #[serde(default)]
    pub export: ExportDefaults,
    /// The template and revision it was made from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub origin: Option<TemplateOrigin>,
    /// The sheet's own variables (ada, parsel, mahalle …): before the project's in the lookup order (design §7).
    #[serde(default)]
    pub variables: Vec<Variable>,
    /// Layouts for other papers (design §3.2a): the first whose condition the paper meets is used.
    #[serde(default)]
    pub variants: Vec<LayoutVariant>,
    /// The layout in force on this paper; none: the base layout.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub active_variant: Option<String>,
    /// The base layout, kept while a variant is in force.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub base_layout: Option<BaseLayout>,
}

/// A paper size in the table (`data/papers.json`) or a size of one's own.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum Paper {
    A0,
    A1,
    A2,
    A3,
    #[default]
    A4,
    A5,
    B0,
    B1,
    B2,
    B3,
    B4,
    Custom,
}

impl Paper {
    pub const STANDARD: [Paper; 11] = [
        Paper::A0,
        Paper::A1,
        Paper::A2,
        Paper::A3,
        Paper::A4,
        Paper::A5,
        Paper::B0,
        Paper::B1,
        Paper::B2,
        Paper::B3,
        Paper::B4,
    ];

    /// The table's id: "a4", … ("custom" has no row).
    pub fn id(self) -> &'static str {
        match self {
            Paper::A0 => "a0",
            Paper::A1 => "a1",
            Paper::A2 => "a2",
            Paper::A3 => "a3",
            Paper::A4 => "a4",
            Paper::A5 => "a5",
            Paper::B0 => "b0",
            Paper::B1 => "b1",
            Paper::B2 => "b2",
            Paper::B3 => "b3",
            Paper::B4 => "b4",
            Paper::Custom => "custom",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum Orientation {
    #[default]
    Portrait,
    Landscape,
}

/// The paper: its kind, orientation, size (as it lies, width across) and margins.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct Page {
    pub paper: Paper,
    pub orientation: Orientation,
    pub size: SizeUm,
    pub margins: Margins,
    /// The paper's colour; none: white.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub background: Option<String>,
}

impl Page {
    pub fn rect(&self) -> RectUm {
        RectUm::new(0, 0, self.size.width, self.size.height)
    }

    /// The printable area: the page less its margins.
    pub fn margin_rect(&self) -> RectUm {
        self.rect().inset_sides(
            self.margins.top,
            self.margins.right,
            self.margins.bottom,
            self.margins.left,
        )
    }
}

/// A master page: items drawn under every sheet that uses it, laid out from
/// its own page to the sheet's by their constraints (design §3.3).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct Master {
    pub id: MasterId,
    pub name: String,
    /// The page the items' frames are given on.
    pub page: Page,
    #[serde(default)]
    pub items: Vec<Item>,
    #[serde(default)]
    pub guides: Vec<Guide>,
    /// Layouts for other papers (design §3.2a); a sheet on such a paper draws the master's that way.
    #[serde(default)]
    pub variants: Vec<LayoutVariant>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub active_variant: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub base_layout: Option<BaseLayout>,
}

/// When a layout variant applies (design §3.2a): the paper's orientation and, if given, a size range (µm, inclusive).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct VariantCondition {
    pub orientation: Orientation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub min_width: Option<Um>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub max_width: Option<Um>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub min_height: Option<Um>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub max_height: Option<Um>,
}

impl VariantCondition {
    /// Whether a page meets the condition.
    pub fn matches(&self, page: &Page) -> bool {
        let (w, h) = (page.size.width, page.size.height);
        page.orientation == self.orientation
            && self.min_width.is_none_or(|m| w >= m)
            && self.max_width.is_none_or(|m| w <= m)
            && self.min_height.is_none_or(|m| h >= m)
            && self.max_height.is_none_or(|m| h <= m)
    }
}

/// An item's place in a layout, on the layout's reference paper.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct VariantFrame {
    pub item: ItemId,
    pub frame: RectUm,
    #[serde(default)]
    pub rotation: Mdeg,
    #[serde(default)]
    pub constraints: Constraints,
    #[serde(default)]
    pub hidden: bool,
}

/// A layout for the papers its condition names (design §3.2a): the items it
/// mentions are placed from `reference` to the paper by their constraints;
/// the others come from the base layout.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct LayoutVariant {
    /// Unique within its sheet or master page (“dikey”).
    pub id: String,
    /// Shown in the inspector (“Dikey kâğıt”).
    pub name: String,
    pub when: VariantCondition,
    /// The paper the frames are given on.
    pub reference: SizeUm,
    #[serde(default)]
    pub frames: Vec<VariantFrame>,
}

/// The base layout set aside while a variant is in force: every item's place on the paper it had.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct BaseLayout {
    pub reference: SizeUm,
    #[serde(default)]
    pub frames: Vec<VariantFrame>,
}

/// One thing on the paper: the frame and behaviour every kind shares, and the kind's own content.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct Item {
    pub id: ItemId,
    /// Unique in its sheet (or master); commands and scripts name the item by it.
    pub name: String,
    /// The frame before rotation.
    pub frame: RectUm,
    /// Around the frame's centre, clockwise, `[0, 360 000)`.
    #[serde(default)]
    pub rotation: Mdeg,
    #[serde(default)]
    pub constraints: Constraints,
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub hidden: bool,
    /// False: shown while designing, left out of the export.
    #[serde(default = "yes")]
    pub printable: bool,
    /// 0–100.
    #[serde(default = "full")]
    pub opacity: u8,
    /// A line around the frame.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub border: Option<Stroke>,
    /// The frame's background.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub fill: Option<String>,
    /// Space between the frame and the content.
    #[serde(default)]
    pub padding: Um,
    /// The group item this one belongs to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub group: Option<ItemId>,
    /// Data-defined properties (design §7).
    #[serde(default)]
    pub bindings: Vec<Binding>,
    pub kind: ItemKind,
}

impl Item {
    /// A new item of `kind` with every shared property at its default.
    pub fn new(id: &str, name: &str, frame: RectUm, kind: ItemKind) -> Item {
        Item {
            id: id.to_owned(),
            name: name.to_owned(),
            frame,
            rotation: 0,
            constraints: Constraints::default(),
            locked: false,
            hidden: false,
            printable: true,
            opacity: 100,
            border: None,
            fill: None,
            padding: 0,
            group: None,
            bindings: Vec::new(),
            kind,
        }
    }

    pub fn is_group(&self) -> bool {
        matches!(self.kind, ItemKind::Group(_))
    }

    /// The frame with the padding taken off.
    pub fn content_rect(&self) -> RectUm {
        self.frame.inset(self.padding)
    }
}

/// How an item follows its box when the paper changes (design §3.2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct Constraints {
    #[serde(default)]
    pub h: HConstraint,
    #[serde(default)]
    pub v: VConstraint,
    #[serde(default)]
    pub relative_to: ConstraintBox,
}

impl Constraints {
    pub const fn new(h: HConstraint, v: VConstraint) -> Constraints {
        Constraints {
            h,
            v,
            relative_to: ConstraintBox::Margins,
        }
    }
}

/// Left: keeps its distance from the box's left edge; right: from the right
/// edge; leftRight: both (it stretches); center: its centre's offset from the
/// box's centre; scale: position and size in proportion.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum HConstraint {
    #[default]
    Left,
    Right,
    LeftRight,
    Center,
    Scale,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum VConstraint {
    #[default]
    Top,
    Bottom,
    TopBottom,
    Center,
    Scale,
}

/// The box an item's constraints are measured against.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum ConstraintBox {
    Page,
    #[default]
    Margins,
    /// The frame of the group the item belongs to.
    Group,
}

/// A property whose value an expression gives (design §7): `frame.left`, `text.content`, `map.scale` …
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct Binding {
    pub property: String,
    pub expression: String,
}

/// Which coordinate a guide fixes: `x` is an upright line at `left = at`, `y` a level line at `top = at`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum Axis {
    X,
    Y,
}

/// A ruler guide.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct Guide {
    pub id: String,
    pub axis: Axis,
    pub at: Um,
    #[serde(default)]
    pub locked: bool,
}

fn five_mm() -> Um {
    5_000
}

/// The sheet's snapping grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct SnapGrid {
    #[serde(default = "five_mm")]
    pub spacing: Um,
    #[serde(default)]
    pub visible: bool,
    #[serde(default)]
    pub enabled: bool,
}

impl Default for SnapGrid {
    fn default() -> Self {
        SnapGrid {
            spacing: five_mm(),
            visible: false,
            enabled: false,
        }
    }
}

/// A sort key: an expression and its direction.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct SortKey {
    pub expression: String,
    #[serde(default)]
    pub descending: bool,
}

fn atlas_page_name() -> String {
    "@atlas_kimlik".to_owned()
}

/// One page per object of a coverage layer (design §7a).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct Atlas {
    /// The coverage layer; empty: none chosen yet (the preflight says so).
    pub layer: String,
    /// Which objects make pages (an expression; empty: all).
    #[serde(default)]
    pub filter: String,
    #[serde(default)]
    pub sort: Vec<SortKey>,
    /// The page's name (an expression).
    #[serde(default = "atlas_page_name")]
    pub page_name: String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum ExportFormat {
    #[default]
    Svg,
    Png,
}

fn dpi300() -> u16 {
    300
}

fn export_name() -> String {
    "[% @pafta_adi %]".to_owned()
}

/// What an export starts from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct ExportDefaults {
    #[serde(default = "dpi300")]
    pub dpi: u16,
    #[serde(default)]
    pub format: ExportFormat,
    /// The file's name, `[% … %]` text.
    #[serde(default = "export_name")]
    pub file_name: String,
}

impl Default for ExportDefaults {
    fn default() -> Self {
        ExportDefaults {
            dpi: dpi300(),
            format: ExportFormat::Svg,
            file_name: export_name(),
        }
    }
}

/// The template a sheet was made from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct TemplateOrigin {
    pub template_id: String,
    pub revision: u32,
}

/// What a variable holds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum VarKind {
    #[default]
    Text,
    Number,
    Bool,
    /// ISO text (YYYY-AA-GG).
    Date,
}

/// A value: none, true/false, a number or text.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(untagged)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum VarValue {
    #[default]
    Null,
    Bool(bool),
    Number(f64),
    Text(String),
}

impl VarValue {
    pub fn is_null(&self) -> bool {
        matches!(self, VarValue::Null)
    }
}

/// A named value written `@name` in expressions and `[% … %]` text.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct Variable {
    /// Without the `@`: letters, digits and `_` (`ada`, `proje_no`).
    pub name: String,
    /// What the interface asks (“Ada no”).
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub kind: VarKind,
    /// None yet: written `‹ad?›` on the paper and reported by the preflight.
    #[serde(default)]
    pub value: VarValue,
}

/// A picture's file type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum AssetKind {
    Png,
    Jpeg,
    Svg,
}

/// A picture's metadata (design §3.4); its bytes are the asset store's, under the same digest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct AssetMeta {
    /// SHA-256 of the bytes, 64 lowercase hex digits.
    pub sha256: String,
    pub kind: AssetKind,
    pub name: String,
    /// Pixels (an SVG's own width and height).
    pub width: u32,
    pub height: u32,
    pub bytes: u32,
    /// The resolution the file states; none: 96 dpi when shown at its own size.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub dpi: Option<u16>,
}

/// The limits of design §3.4.
pub const ASSET_MAX_BYTES: u32 = 4 * 1024 * 1024;
pub const TEMPLATE_ASSETS_MAX_BYTES: u64 = 8 * 1024 * 1024;
