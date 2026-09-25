//! A whole drawing as stored or sent (`DocumentSnapshotV1`): project
//! settings, the layer tree, the objects and the project's styles.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::entity::{Entity, Vec2};
use crate::layer::LayerNode;

pub const DOCUMENT_FORMAT: &str = "kentos.document";
pub const DOCUMENT_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum AreaUnit {
    M2,
    Donum,
    Ha,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
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
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum Workspace {
    Hybrid,
    Cad,
    Gis,
    Plan3d,
    Disaster,
}

/// Project settings (`ProjectSettingsData`): saved with the drawing, the same for everyone who opens it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
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
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
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
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProjectStyles {
    #[cfg_attr(feature = "ts", ts(type = "unknown[]"))]
    pub items: Vec<serde_json::Value>,
    #[cfg_attr(feature = "ts", ts(type = "unknown[]"))]
    pub categories: Vec<serde_json::Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
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

impl DocumentSnapshotV1 {
    /// Reads a snapshot, refusing other formats and versions instead of guessing.
    pub fn from_json(text: &str) -> Result<Self, String> {
        let head: serde_json::Value =
            serde_json::from_str(text).map_err(|e| format!("JSON değil: {e}"))?;
        if head.get("format").and_then(|f| f.as_str()) != Some(DOCUMENT_FORMAT) {
            return Err("KentOS çizim dosyası değil (format ≠ kentos.document).".into());
        }
        match head.get("version").and_then(|v| v.as_u64()) {
            Some(v) if v == u64::from(DOCUMENT_VERSION) => {}
            Some(v) => {
                return Err(format!(
                    "Çizim dosyası sürümü {v} bu uygulamada okunamıyor (desteklenen: {DOCUMENT_VERSION})."
                ));
            }
            None => return Err("Çizim dosyasında sürüm yok.".into()),
        }
        serde_json::from_value(head).map_err(|e| format!("Çizim dosyası bozuk: {e}"))
    }
}
