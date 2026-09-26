//! A whole drawing as stored or sent: project settings, the layer tree, the
//! objects and the project's styles. `DocumentSnapshotV1` is the v1 `.kcad`
//! (JSON); `DocumentSnapshotV2` is what a binary `.kcad` v2 holds
//! (docs/specs/kcad-v2.md), with every object's persistent id.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::entity::{Entity, Vec2};
use crate::identity::{EntityId, ProjectId};
use crate::layer::LayerNode;

pub const DOCUMENT_FORMAT: &str = "kentos.document";
pub const DOCUMENT_VERSION: u32 = 1;
/// The document schema inside a `.kcad` v2 file (docs/specs/kcad-v2.md §6.1).
pub const DOCUMENT_VERSION_2: u32 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum AreaUnit {
    M2,
    Donum,
    Ha,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum AngleUnit {
    Grad,
    Deg,
}

/// The work mode a project opens in (`app/workspaces.ts`): which menus, ribbon
/// tabs and tools the interface shows. Presentation only, never what the data
/// means: every command still runs in every mode. `plan3d` and `disaster` are
/// announced ("Yakında") and cannot be chosen yet; a file naming them opens in
/// the hybrid presentation. Files written before modes existed have none
/// (hybrid).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum Workspace {
    Hybrid,
    Cad,
    Gis,
    Plan3d,
    Disaster,
}

/// The typeface of the text that is part of the drawing (text objects,
/// dimension values, labels; `app/appearance.ts` DRAWING_FONTS), bundled with
/// the app. A project setting: everyone who opens the project sees the same
/// letters. Files written before it have none (Barlow).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum DrawingFont {
    Barlow,
    Arimo,
    Overpass,
    Quicksand,
    ArchitectsDaughter,
    CourierPrime,
    PlexMono,
}

/// Project settings (`ProjectSettingsData`): saved with the drawing, the same for everyone who opens it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProjectSettings {
    pub srid: u32,
    pub length_decimals: u32,
    pub area_decimals: u32,
    pub area_unit: AreaUnit,
    pub angle_unit: AngleUnit,
    /// Plot scale denominator (1:1000 → 1000).
    pub plot_scale: f64,
    /// Absent in files written before work modes (read as hybrid).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub workspace: Option<Workspace>,
    /// Absent in files written before drawing typefaces (read as Barlow).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub drawing_font: Option<DrawingFont>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct Bounds {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

/// The project's own style library (opaque items in v1, see `style`).
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProjectStyles {
    #[cfg_attr(feature = "ts", ts(type = "unknown[]"))]
    pub items: Vec<serde_json::Value>,
    #[cfg_attr(feature = "ts", ts(type = "unknown[]"))]
    pub categories: Vec<serde_json::Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DocumentSnapshotV1 {
    #[cfg_attr(feature = "ts", ts(type = "\"kentos.document\""))]
    pub format: String,
    #[cfg_attr(feature = "ts", ts(type = "1"))]
    pub version: u32,
    pub name: String,
    pub settings: ProjectSettings,
    /// Local anchor near the data (the GPU works relative to it).
    pub origin: Vec2,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub home_view: Option<Bounds>,
    pub layers: Vec<LayerNode>,
    pub active_layer: String,
    pub entities: Vec<Entity>,
    pub styles: ProjectStyles,
}

/// Where a drawing kept as v2 was migrated from: the v1 file it was opened
/// from (docs/adr/0014, TODOS.md FILE-05, FILE-21). A v2 file keeps it in
/// every later save; it says where the objects' derived ids came from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MigrationSource {
    /// `kentos.document`.
    pub format: String,
    /// 1: the only migration there is.
    pub version: u32,
    /// sha256 of the v1 file's canonical text, 64 lowercase hexadecimal
    /// digits: the namespace of the derived ids comes from it (`V1Identities`).
    pub source_sha256: String,
}

impl MigrationSource {
    /// A v1 `.kcad` whose canonical text has this sha256 (64 lowercase hexadecimal digits).
    pub fn v1(source_sha256: String) -> Self {
        Self {
            format: DOCUMENT_FORMAT.to_owned(),
            version: DOCUMENT_VERSION,
            source_sha256,
        }
    }
}

/// A whole drawing as a binary `.kcad` v2 holds it (docs/specs/kcad-v2.md):
/// what v1 holds, every object's persistent id, the project's id and, for a
/// drawing migrated from v1, where it came from. This is its JSON form, which
/// the browser and the formats WASM module exchange; the file itself is
/// written and read by `kentos-kcad`.
///
/// The objects' `id`s are the open document's slots: a v2 file does not write
/// them, and a reader numbers the objects 1, 2, 3 … in file order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DocumentSnapshotV2 {
    #[cfg_attr(feature = "ts", ts(type = "\"kentos.document\""))]
    pub format: String,
    #[cfg_attr(feature = "ts", ts(type = "2"))]
    pub version: u32,
    pub name: String,
    pub settings: ProjectSettings,
    /// Local anchor near the data (the GPU works relative to it).
    pub origin: Vec2,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub home_view: Option<Bounds>,
    pub layers: Vec<LayerNode>,
    pub active_layer: String,
    /// The objects in document order (drawing order).
    pub entities: Vec<Entity>,
    /// Each object's persistent id, in the order of `entities`.
    pub uids: Vec<EntityId>,
    pub styles: ProjectStyles,
    /// The project's persistent id, when it has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub project_id: Option<ProjectId>,
    /// The v1 file a migrated drawing came from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub migrated_from: Option<MigrationSource>,
}

/// The two fields a reader checks before the rest; every other field is skipped unread.
#[derive(Default, Deserialize)]
struct Head {
    #[serde(default)]
    format: Option<serde_json::Value>,
    #[serde(default)]
    version: Option<serde_json::Value>,
}

impl DocumentSnapshotV1 {
    /// Reads a snapshot, refusing other formats and versions instead of guessing.
    /// A leading byte order mark is skipped, as a browser skips it when it
    /// decodes the file. The format and version are read first without building
    /// the rest; the drawing is then read once, straight into the typed
    /// contract (a large file never becomes a `serde_json::Value` tree).
    pub fn from_json(text: &str) -> Result<Self, String> {
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        let head = match serde_json::from_str::<Head>(text) {
            Ok(head) => head,
            Err(e) if e.is_syntax() || e.is_eof() => return Err(format!("JSON değil: {e}")),
            // Not an object, or its format or version is not plain JSON: the checks below say so.
            Err(_) => Head::default(),
        };
        if head.format.as_ref().and_then(|f| f.as_str()) != Some(DOCUMENT_FORMAT) {
            return Err("KentOS çizim dosyası değil (format ≠ kentos.document).".into());
        }
        match head.version.as_ref().and_then(|v| v.as_u64()) {
            Some(v) if v == u64::from(DOCUMENT_VERSION) => {}
            Some(v) => {
                return Err(format!(
                    "Çizim dosyası sürümü {v} bu uygulamada okunamıyor (desteklenen: {DOCUMENT_VERSION})."
                ));
            }
            None => return Err("Çizim dosyasında sürüm yok.".into()),
        }
        serde_json::from_str(text).map_err(|e| format!("Çizim dosyası bozuk: {e}"))
    }
}
