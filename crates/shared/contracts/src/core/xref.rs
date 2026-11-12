//! External references (docs/adr/0348; plan/cad.md `CAD-REF-05`…`14`): a block definition that is another drawing file,
//! its source kept beside its content (`BlockDefinition.xref`, `.kcad` schema 83), and the commands that attach, reload,
//! unload, repath, detach and bind it (`cad.xrefs.edit`) and list them with their state (`cad.xrefs.list`).
//!
//! The content is the source's as last loaded: a drawing opens and prints without its sources, and a source's change is
//! said, never taken in silently.
//!
//! A PDF underlay (docs/adr/0351 §2; `CAD-REF-17`) is an external reference whose source is a page of a PDF file
//! (`page`): the page's lines, areas and texts as the definition's objects, in paper metres, with the underlay's look
//! (`underlay`: faded, monochrome, plotted or not).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::entity::Vec2;
use crate::identity::BlockId;

pub const CAD_XREFS_EDIT: &str = "cad.xrefs.edit";
pub const CAD_XREFS_EDIT_VERSION: u32 = 1;
pub const CAD_XREFS_LIST: &str = "cad.xrefs.list";
pub const CAD_XREFS_LIST_VERSION: u32 = 1;

/// How a reference goes on: `attach` (Ekle) is seen in a drawing that references this one too, `overlay` (Bindir) only
/// here.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum XrefKind {
    #[default]
    Attach,
    Overlay,
}

/// Where an external reference's content comes from (docs/adr/0348 §1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct XrefSource {
    /// The source drawing's absolute path, as attached or last found.
    pub path: String,
    /// Its path from the drawing's folder, `/` between its parts, written when the drawing is saved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub relative: Option<String>,
    #[serde(default, skip_serializing_if = "is_attach")]
    #[cfg_attr(feature = "ts", ts(as = "Option<XrefKind>", optional))]
    pub kind: XrefKind,
    /// The SHA-256 of the file's content as loaded, lowercase hex.
    pub digest: String,
    /// Its content let go (Boşalt): its inserts stay and draw nothing.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
    pub unloaded: bool,
    /// The layer group its layers are under, by id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub group: Option<String>,
    /// The reference that brought it, for one nested in another.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub parent: Option<BlockId>,
    /// The source's objects' persistent ids, one for each of the definition's objects in its order: a reload finds
    /// each object again by it and moves the references to its pieces (a definition's objects are numbered by their
    /// order in a file).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub objects: Vec<String>,
    /// A PDF underlay's page, from 1 (docs/adr/0351 §2): the source is a PDF file and this page its content.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub page: Option<u32>,
    /// A PDF underlay's look; absent: as its PDF draws, plotted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub underlay: Option<UnderlayLook>,
}

/// The most a PDF underlay is faded, in percent (AutoCAD's limit).
pub const MAX_UNDERLAY_FADE: u8 = 80;

fn is_true(b: &bool) -> bool {
    *b
}

fn yes() -> bool {
    true
}

/// How a PDF underlay looks (docs/adr/0351 §2).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct UnderlayLook {
    /// Faded toward see-through, 0 to 80 percent.
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<u8>"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "percent")))]
    pub fade: u8,
    /// Drawn in the drawing's ink, its colours left aside.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
    pub monochrome: bool,
    /// Drawn on a sheet (PDF, print); false: on the screen only.
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
    pub plot: bool,
}

fn is_zero(v: &u8) -> bool {
    *v == 0
}

impl Default for UnderlayLook {
    fn default() -> Self {
        Self { fade: 0, monochrome: false, plot: true }
    }
}

impl UnderlayLook {
    /// The look as written: none when it is the default.
    pub fn kept(self) -> Option<Self> {
        (self != Self::default()).then_some(self)
    }
}

/// Lowercase UUID text with hyphens (`01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f`).
fn is_uid(t: &str) -> bool {
    let b = t.as_bytes();
    b.len() == 36
        && b.iter().enumerate().all(|(i, c)| match i {
            8 | 13 | 18 | 23 => *c == b'-',
            _ => c.is_ascii_digit() || (b'a'..=b'f').contains(c),
        })
}

fn is_attach(k: &XrefKind) -> bool {
    *k == XrefKind::Attach
}

impl XrefSource {
    /// What is wrong with it: a relative path, a digest that is not 64 hex digits.
    pub fn problem(&self) -> Option<String> {
        if self.path.trim().is_empty() {
            return Some("dış başvurunun yolu boş".into());
        }
        if self.digest.len() != 64 || !self.digest.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) {
            return Some("dış başvurunun özeti 64 küçük onaltılık rakam olmalı".into());
        }
        if self.relative.as_deref().is_some_and(|r| r.trim().is_empty()) {
            return Some("dış başvurunun göreli yolu boş".into());
        }
        if self.group.as_deref().is_some_and(|g| g.trim().is_empty()) {
            return Some("dış başvurunun katman grubu boş".into());
        }
        if self.objects.iter().any(|u| !is_uid(u)) {
            return Some("dış başvurunun nesne kimlikleri küçük harfli, tireli UUID olmalı".into());
        }
        if self.page == Some(0) {
            return Some("PDF altlığının sayfası 1'den başlar".into());
        }
        if let Some(u) = &self.underlay {
            if self.page.is_none() {
                return Some("altlık görünüşü yalnız PDF altlığında olur".into());
            }
            if u.fade > MAX_UNDERLAY_FADE {
                return Some(format!("PDF altlığının soldurması en çok %{MAX_UNDERLAY_FADE} olabilir"));
            }
            if *u == UnderlayLook::default() {
                return Some("varsayılan altlık görünüşü yazılmaz".into());
            }
        }
        None
    }
}

/// How `bind` turns a reference into a block.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum XrefBindMode {
    /// Its layers stay in their group, now an ordinary one.
    #[default]
    Bind,
    /// Its layers join the drawing's layers of the same path outside the group (made where missing).
    Insert,
}

/// What `cad.xrefs.edit` does.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum XrefsOperation {
    /// A drawing file attached (or overlaid) and placed once.
    Attach {
        /// The source drawing, an absolute path.
        path: String,
        /// `attach` or `overlay`; absent: `attach`.
        #[serde(default)]
        #[cfg_attr(feature = "ts", ts(as = "Option<XrefKind>", optional))]
        mode: XrefKind,
        /// Its name; absent: the file's name without its extension (made unique).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        name: Option<String>,
        /// Where its origin goes; absent: the drawing's 0, 0.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        at: Option<Vec2>,
        /// Its scale; absent: 1.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "scale")))]
        scale: Option<f64>,
        /// Its turn, degrees counter-clockwise; absent: 0.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
        rotation: Option<f64>,
        /// The insert's layer.
        layer_id: String,
        /// A PDF file's page to underlay, from 1 (docs/adr/0351 §2); absent for a PDF: its first page.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        page: Option<u32>,
    },
    /// Its source read again.
    Reload { name: String },
    /// Its content let go; its inserts stay.
    Unload { name: String },
    /// Bound to another file, which is loaded.
    Repath { name: String, path: String },
    /// Its definition, those it brought, its inserts and its emptied layer group removed.
    Detach { name: String },
    /// The project's search paths (docs/adr/0349 §1), all of them in order: a setting, not an undo step.
    Paths { paths: Vec<String> },
    /// A PDF underlay's look (docs/adr/0351 §2): each field given changes; a setting of the definition, one undo step.
    Underlay {
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "percent")))]
        fade: Option<u8>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        monochrome: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        plot: Option<bool>,
    },
    /// Katmanları kaynağa döndür (docs/adr/1060 §5): the drawing's changes to the reference's layers (their look,
    /// shown, locked, frozen, printed) let go, each layer as its source has it; one undo step for the looks. The
    /// project's layer states (a sheet's or a view's own) stay.
    ResetLayers { name: String },
    /// Made an ordinary block, with those it brought.
    Bind {
        name: String,
        #[serde(default)]
        #[cfg_attr(feature = "ts", ts(as = "Option<XrefBindMode>", optional))]
        mode: XrefBindMode,
    },
}

/// Input of `cad.xrefs.edit` v1 (“Dış başvuru”): one undo step.
///
/// Refusals, in this order: `xref_not_found`, `nested_xref`, `not_finite`, `layer_not_found`, `not_a_layer`,
/// `layer_locked`, `invalid_file_path`, `file_not_found`, `import_unreadable`, `pdf_page_not_found`, `not_an_underlay`,
/// `invalid_underlay`, `xref_cycle`, `xref_empty`, `invalid_revision`, `revision_conflict`. Warnings:
/// `objects_left_out`, `layer_hidden`, `pdf_left_out` (what a PDF page holds that an underlay does not take),
/// `ties_break` (a reload or repath with `diff`: ties into the reference's pieces that will break, docs/adr/1060 §4).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct XrefsEdit {
    pub operation: XrefsOperation,
    /// The open drawing's own file, an absolute path, when it has one: a reference to it (or through it) is a cycle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub drawing: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
    /// Whether a reload or a repath says what it changes (`diff`, docs/adr/1060 §4): the pieces kept, changed, new
    /// and gone, and the objects tied into them that follow or break. A plan with it is the preview.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub diff: bool,
}

/// What follows a reference's piece (docs/adr/1060 §4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum XrefFollowerKind {
    Dimension,
    Leader,
    Hatch,
    /// A text whose field reads a piece.
    Text,
    /// A table whose cell reads a piece.
    Table,
}

/// What a reload does to an object tied into the reference's pieces (docs/adr/1060 §4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum XrefFollowEffect {
    /// It follows its piece to where it is now (or its value changes).
    Follows,
    /// A tie of it breaks: its piece is gone or has no such place; it keeps where it was and says why.
    Breaks,
    /// An associative hatch lets its boundary go and stays where it is.
    Unties,
}

/// An object of the drawing a reload or a repath changes through its ties.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct XrefFollower {
    /// Its lasting id.
    pub uid: String,
    pub kind: XrefFollowerKind,
    pub effect: XrefFollowEffect,
}

/// What a reload or a repath changes in the reference's definition (docs/adr/1060 §4): its pieces by their local ids
/// (a piece is the same when its id and content are), and what follows them.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct XrefDiff {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub same: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub changed: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub added: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub removed: u32,
    /// The definitions it brings that it did not before, by name.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub nested_added: Vec<String>,
    /// The definitions it brought and no longer brings, by name.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub nested_removed: Vec<String>,
    /// The drawing's objects tied into its pieces that the step changes, in the drawing's order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub followers: Vec<XrefFollower>,
}

/// Output of `cad.xrefs.edit` v1 (its plan's too: what it would do, nothing written).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct XrefsEdited {
    /// The reference's name.
    pub name: String,
    /// Its definition, by id (none once detached or bound).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub block: Option<BlockId>,
    /// `attach`: the insert placed, by persistent id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub insert: Option<String>,
    /// The objects its content holds.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub objects: u32,
    /// The layers made, by name.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub layers: Vec<String>,
    /// The references nested in it, by name.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub nested: Vec<String>,
    /// `detach`: the inserts removed.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub removed: u32,
    /// `bind`: the layers that joined the drawing's own of the same path, by name.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub merged: Vec<String>,
    /// `paths`: the project's search paths as kept.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub paths: Vec<String>,
    /// `resetLayers`: the layers and groups whose look the source's replaced, by name, in the tree's order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reset: Vec<String>,
    /// `reload` and `repath` with `diff`: what changes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub diff: Option<XrefDiff>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// Input of `cad.xrefs.list` v1 (“Dış başvurular”): read-only.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct XrefsList {
    /// The open drawing's own file, an absolute path, when it has one: a relative path is found from its folder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub drawing: Option<String>,
    /// Whether to say each reference's sheets and named views its inserts show (docs/adr/0349 §3).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub affected: bool,
    /// Whether to review each reference (docs/adr/1060 §1): its revision as used, its depth in the network, what the
    /// drawing shows of it, the references it brought, the layer states naming its layers and, for one missing or
    /// changed, the files in the allowed places that may be it (`candidates`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub review: bool,
}

/// What the drawing shows of a reference (docs/adr/1060 §1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum XrefContent {
    /// Its file's content: the file is there and the same.
    Source,
    /// The content as last loaded, kept in the drawing: its file is not found or is another revision now.
    Cached,
    /// Nothing: unloaded.
    None,
}

/// How a candidate file matches a reference (docs/adr/1060 §2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum XrefMatch {
    /// Its SHA-256 is the one last loaded: the same bytes, moved or renamed.
    Content,
    /// The same file name, other bytes: another drawing, or another revision of it, to be reviewed before it is bound.
    Name,
}

/// A file that may be a reference's source (docs/adr/1060 §2): nothing is bound until Yolu değiştir is run with it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct XrefCandidate {
    /// Its absolute path.
    pub path: String,
    #[serde(rename = "match")]
    pub matched: XrefMatch,
    /// Its SHA-256.
    pub digest: String,
}

/// A sheet viewport's own override of one of a reference's layers (docs/adr/1110 §2; AutoCAD's VP Freeze, VP Color …):
/// that map item draws the layer otherwise, whatever the drawing's own look of it. No `cad.xrefs.*` operation changes
/// it: a reload, a repath and an unload keep the layer's id, `resetLayers` sets back only the drawing's own look.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct XrefViewportOverride {
    /// The sheet, by its name.
    pub sheet: String,
    /// The map item's id (what `cad.sheets.edit`'s `setViewport` takes).
    pub item: String,
    /// The map item, by its name.
    pub viewport: String,
    /// The layer (or group) by its path in the drawing's layer tree: `Dış başvuru: Plan / Mimari / Duvar`.
    pub layer: String,
    /// Not drawn in that viewport.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub frozen: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub plot: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub line_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub line_pattern: Option<String>,
    /// On the paper, micrometres.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-um")))]
    pub line_weight: Option<i32>,
}

/// The most files a review reads to find candidates, over all the references (docs/adr/1060 §2).
pub const MAX_CANDIDATE_FILES: usize = 512;
/// The largest file a review reads to find candidates, in bytes (docs/adr/1060 §2).
pub const MAX_CANDIDATE_BYTES: u64 = 512 * 1024 * 1024;

/// A reference's state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum XrefState {
    /// Its content is its file's.
    Loaded,
    /// Let go (Boşalt).
    Unloaded,
    /// Its file is not there (nor at its relative path).
    Missing,
    /// Its file's content is not what was loaded: Yeniden yükle takes it.
    Changed,
}

/// A reference as `cad.xrefs.list` lists it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct XrefInfo {
    pub name: String,
    pub block: BlockId,
    pub kind: XrefKind,
    /// The file as it is found: its own path, else its relative path's.
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub relative: Option<String>,
    pub state: XrefState,
    /// The reference that brought it, by name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub parent: Option<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub inserts: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub objects: u32,
    /// Its layer group's name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub group: Option<String>,
    /// Where its source was found (docs/adr/0349 §1).
    pub via: crate::AssetVia,
    /// The places looked in, in order, when it was not found.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tried: Vec<String>,
    /// The sheets whose map frames show its inserts, by name (`affected`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sheets: Vec<String>,
    /// The named views whose window meets its inserts, by name (`affected`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub views: Vec<String>,
    /// A PDF underlay's page (docs/adr/0351 §2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub page: Option<u32>,
    /// A PDF underlay's look when it is not the default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub underlay: Option<UnderlayLook>,
    /// `review`: the SHA-256 of the content as last loaded (the revision the drawing uses).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub digest: Option<String>,
    /// `review`: its file's SHA-256 now, when it is another revision (`changed`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub current: Option<String>,
    /// `review`: how deep in the network it is: 0 for one the drawing attached, 1 for one another brought, and so on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub depth: Option<u32>,
    /// `review`: what the drawing shows of it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub content: Option<XrefContent>,
    /// `review`: the references it brought, by name, in the drawing's block order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub brings: Vec<String>,
    /// `review`: the project's layer states that name its layers (a sheet's or a view's own look of them), by name.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub layer_states: Vec<String>,
    /// `review`, one missing or changed: the files that may be it, those of the same content first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub candidates: Vec<XrefCandidate>,
    /// `review`: the sheet viewports' own overrides of its layers (docs/adr/1110 §2), in the book's order of sheets,
    /// their items and the overrides.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub viewport_overrides: Vec<XrefViewportOverride>,
}

/// Output of `cad.xrefs.list` v1.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct XrefsListed {
    /// In the drawing's block order.
    pub xrefs: Vec<XrefInfo>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// The layer group a reference's layers go under (docs/adr/0348 §2).
pub fn xref_group_name(name: &str) -> String {
    format!("Dış başvuru: {name}")
}
