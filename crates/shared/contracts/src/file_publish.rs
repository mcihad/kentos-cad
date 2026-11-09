//! Ordered sheet publication into a fresh delivery folder; the command never overwrites an issued set.
use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;
pub const FILE_PUBLISH_SHEETS: &str = "file.publish.sheets";
pub const FILE_PUBLISH_SHEETS_VERSION: u32 = 1;
fn naming() -> String {
    "{index}-{sheet}".into()
}
fn yes() -> bool {
    true
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
#[serde(rename_all = "camelCase")]
pub enum SheetPublishMode {
    #[default]
    Single,
    Separate,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SheetPublishPin {
    pub revision: String,
    /// Semantic document hash, excluding request replay bookkeeping.
    pub sha256: String,
    /// A full publication plan also pins resolved linked image versions. None requests a document-only guard.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub resources_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub context_sha256: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PublishSheets {
    /// Absolute new or empty delivery folder; existing nonempty folders are refused.
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub drawing: Option<String>,
    /// None means every sheet; an explicit list is used in exactly its supplied order.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub sheets: Option<Vec<String>>,
    #[serde(default)]
    pub mode: SheetPublishMode,
    /// Also deliver the source drawing, local dependencies, bundled fonts and captured plot resources.
    #[serde(default)]
    pub resources: bool,
    /// One PDF's leaf name; .pdf is appended when missing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    /// Separate PDF leaf names: {index} (001...), {sheet}, {id}, {number}, {title}, {revision}, {status}, {subset}. Collisions get a stable numeric suffix.
    #[serde(default = "naming")]
    pub naming: String,
    /// None preserves each page's own physical raster setting.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature="schema",schemars(extend("x-unit"="dpi")))]
    pub dpi: Option<u16>,
    #[serde(default = "yes")]
    pub geo: bool,
    #[serde(default = "yes")]
    pub layers: bool,
    /// PDF/A-2b (ISO 19005-2; docs/adr/3037): every file archival, as the 2026 digital project regulation asks a
    /// drawing (RG 5.8.2026/33331 Md. 4/2): XMP metadata and an sRGB output intent; a CMYK JPEG picture is refused.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
    pub pdfa: bool,
    /// A plan's immutable document pin; changed source refuses publication before any file is exposed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub pin: Option<SheetPublishPin>,
    /// Separate PDFs only (docs/adr/1090 §7): a publication that is stopped or fails keeps its finished pages and a
    /// journal in a hidden sibling folder (`.<folder>.kentos-devam`), never in the delivery folder, so `resume` can go on.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
    pub resumable: bool,
    /// Go on with the stopped publication of this folder: its finished pages are checked by SHA-256 and kept, the
    /// rest are made. Refused unless the drawing, its linked resources, the title context and the options are the ones
    /// it started with (`resume_stale`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
    pub resume: bool,
    /// An update of this earlier delivery folder (absolute; docs/adr/1320 §3), published into the new `path`: a sheet
    /// whose page now draws what that delivery's page drew (equal content digests) and whose delivered PDF still has
    /// its bytes is copied byte for byte with its own publication date; the rest are made again with today's. The
    /// earlier folder is left as it is. Separate PDFs only; not with `resume`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub previous: Option<String>,
}

/// The earlier delivery a publication updates (docs/adr/1320 §3).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
#[serde(rename_all = "camelCase")]
pub struct PreviousPublication {
    pub path: String,
    /// Its `yayin.json`'s SHA-256 when it was read.
    pub sha256: String,
    /// Its publication date.
    pub date: String,
    /// Its source.
    pub source: SheetPublishPin,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
#[serde(rename_all = "camelCase")]
pub struct SheetPublishedFile {
    pub name: String,
    pub sheets: Vec<String>,
    #[cfg_attr(feature="schema",schemars(extend("x-unit"="bytes")))]
    pub bytes: u64,
    pub sha256: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
#[serde(rename_all = "camelCase")]
pub struct SheetPublishIssue {
    pub sheet: String,
    pub code: String,
    pub message: String,
    pub fix: String,
    pub error: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
#[serde(rename_all = "camelCase")]
pub struct SheetsPublished {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub resources: Option<SheetDeliveryResources>,
    pub path: String,
    pub sheets: Vec<String>,
    pub files: Vec<SheetPublishedFile>,
    pub issues: Vec<SheetPublishIssue>,
    pub source: SheetPublishPin,
    /// True after full output/source checks; validate-only deliberately leaves this false.
    pub ready: bool,
    pub manifest: String,
    pub sha256: String,
    /// What is published, sheet by sheet (docs/adr/1090 §7): paper, viewports with their scales, locks and layer
    /// states, the plot profile.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<SheetPublishScope>>"))]
    pub scope: Vec<SheetPublishScope>,
    /// Each written page's audit and content digest (docs/adr/1090 §4), in the published order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<SheetPublishedPage>>"))]
    pub pages: Vec<SheetPublishedPage>,
    /// With `resume`: the sheets whose finished pages were kept from the stopped publication.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<String>>"))]
    pub resumed: Vec<String>,
    /// With `previous`: the earlier delivery, and the sheets whose pages were copied from it unchanged, in the
    /// published order (the others were made again).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub previous: Option<PreviousPublication>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<Vec<String>>"))]
    pub carried: Vec<String>,
}

/// One viewport of a published sheet.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
#[serde(rename_all = "camelCase")]
pub struct SheetPublishFrame {
    pub item: String,
    pub name: String,
    /// model / paper; none for an atlas frame.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "scale")))]
    pub denominator: Option<f64>,
    pub locked: bool,
    /// `drawing` (the drawing's own visibility, which anyone's layer switching changes), `list`, or
    /// `theme:<layer state>`.
    pub layers: String,
    /// The annotation representation chosen; none: the viewport's scale.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "scale")))]
    pub annotation: Option<f64>,
}

/// One sheet of a publication's scope.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
#[serde(rename_all = "camelCase")]
pub struct SheetPublishScope {
    pub sheet: String,
    pub name: String,
    pub number: String,
    pub revision: String,
    /// The page, mm: width, height.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub paper: [f64; 2],
    /// The plot profile it prints with, by name; none: the objects' own looks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub profile: Option<String>,
    pub frames: Vec<SheetPublishFrame>,
    /// Table items on the sheet.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub tables: u32,
}

/// One written page: what it paints and its content digest (docs/adr/1090 §4).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
#[serde(rename_all = "camelCase")]
pub struct SheetPublishedPage {
    pub sheet: String,
    /// The PDF it is in.
    pub file: String,
    /// The page's content digest made without the title context (user, date): equal digests draw the same page.
    pub digest: String,
    /// The date its bytes were made with (the title context's): an update's carried page keeps its earlier one
    /// (docs/adr/1320 §3). None in publications made before.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub date: Option<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub strokes: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub fills: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub even_odd_fills: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub hairlines: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub thin_lines: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub min_stroke_mm: Option<f64>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub texts: u32,
    pub fonts: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub clips: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub transparent: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub images: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub raster_maps: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub placeholders: u32,
    /// The page's audit findings (none an error by itself: the preflight's errors block publication).
    pub findings: Vec<SheetPublishIssue>,
}

pub const FILE_PUBLISH_STATUS: &str = "file.publish.status";
pub const FILE_PUBLISH_STATUS_VERSION: u32 = 1;

/// Input of `file.publish.status` v1 (“Yayın durumu”, docs/adr/1090 §6): a delivery folder's last publication against
/// the drawing as it is now, and a stopped publication's journal. Reads files, writes none.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PublishStatus {
    /// The delivery folder (absolute), as given to `file.publish.sheets`.
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub drawing: Option<String>,
}

/// A published sheet against the drawing now.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
#[serde(rename_all = "camelCase")]
pub enum SheetPublishState {
    /// Its page now draws what was published.
    Current,
    /// Its page now draws otherwise: publish it again.
    Stale,
    /// The publication has no digest for it (made before docs/adr/1090): it cannot be compared.
    Unknown,
    /// The sheet is no longer in the book.
    Removed,
    /// In the book, not in the publication.
    Unpublished,
    /// Its delivered PDF is missing or not the published bytes.
    FileChanged,
    /// Its page cannot be made now; the reason is in `issue`.
    Failed,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
#[serde(rename_all = "camelCase")]
pub struct SheetPublishStatus {
    pub sheet: String,
    pub name: String,
    pub state: SheetPublishState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub file: Option<String>,
    /// The digest published.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub published: Option<String>,
    /// The digest now.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub issue: Option<SheetPublishIssue>,
    /// The date its delivered page was made with.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub date: Option<String>,
    /// What changed in its sources since the delivery (docs/adr/1320 §2); none when the delivery did not list them
    /// (made before, or a page with an atlas or unplaced viewport) or the sheet is gone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub changes: Option<SheetSourceChanges>,
}

/// A published page's sources against the drawing now (docs/adr/1320 §2), by persistent id in drawing order: the
/// objects it drew whose content changed, the ones it draws now and did not, the ones it drew and does not now
/// (deleted, moved out, hidden), whether the sheet's own record changed, and which project parts every page draws
/// with changed. All empty when nothing it reads changed.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
#[serde(rename_all = "camelCase")]
pub struct SheetSourceChanges {
    pub changed: Vec<String>,
    pub added: Vec<String>,
    pub removed: Vec<String>,
    /// The sheet's own items, page or drawing information.
    pub sheet: bool,
    /// Of `blocks`, `book` (masters, templates, profiles), `layers`, `resources` (linked pictures), `settings`,
    /// `styles`, in this order.
    pub parts: Vec<String>,
}

/// Why a publication stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
#[serde(rename_all = "camelCase")]
pub enum PublishStopReason {
    Cancelled,
    Failed,
}

/// A stopped publication's journal: no delivery folder was made; `resume` goes on from it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
#[serde(rename_all = "camelCase")]
pub struct StoppedPublication {
    pub reason: PublishStopReason,
    pub date: String,
    pub source: SheetPublishPin,
    /// The sheets chosen, in order.
    pub sheets: Vec<String>,
    /// Finished pages kept, and the ones still to make.
    pub done: Vec<String>,
    pub remaining: Vec<String>,
    pub files: Vec<SheetPublishedFile>,
    pub issues: Vec<SheetPublishIssue>,
    /// The drawing is still the one it started with: `resume` can go on.
    pub resumable: bool,
}

/// Where a delivery folder stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
#[serde(rename_all = "camelCase")]
pub enum PublishFolderState {
    /// Published: `yayin.json` is there.
    Published,
    /// A publication of it stopped; nothing was delivered.
    Stopped,
    /// Neither.
    None,
}

/// Output of `file.publish.status` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
#[serde(rename_all = "camelCase")]
pub struct PublishStatusReport {
    pub path: String,
    pub state: PublishFolderState,
    /// The drawing now; kept apart from the published source.
    pub current: SheetPublishPin,
    /// The source the delivery was published from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub published: Option<SheetPublishPin>,
    /// The publication's date.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub date: Option<String>,
    /// The published sheets in their order, then the book's unpublished ones.
    pub sheets: Vec<SheetPublishStatus>,
    /// The published sheets to make again (stale, failed or whose file changed), in the published order.
    pub stale: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub stopped: Option<StoppedPublication>,
}


#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
#[serde(rename_all = "camelCase")]
pub enum SheetResourceKind {
    Drawing,
    Dependency,
    Font,
    License,
    PlotProfile,
    PageSetups,
    Manifest,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SheetDeliveryResource {
    /// Normal relative path inside the delivery folder.
    pub path: String,
    pub kind: SheetResourceKind,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit"="bytes")))]
    pub bytes: u64,
    pub sha256: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SheetDeliveryResources {
    /// The portable source drawing, relative to the root publication manifest.
    pub project: String,
    pub files: Vec<SheetDeliveryResource>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit"="bytes")))]
    pub bytes: u64,
}
