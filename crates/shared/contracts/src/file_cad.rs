//! DWG exchange contracts (ADR 0586). These contain no external codec types.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const FILE_IMPORT_DWG: &str = "file.import.dwg";
pub const FILE_IMPORT_DWG_VERSION: u32 = 1;
pub const FILE_EXPORT_DWG: &str = "file.export.dwg";
pub const FILE_EXPORT_DWG_VERSION: u32 = 1;

/// The binary format signature, not the release year of the application saving it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub enum DwgVersion {
    AC1014,
    AC1015,
    AC1018,
    AC1021,
    AC1024,
    AC1027,
    #[default]
    AC1032,
}

impl DwgVersion {
    pub const ALL: [Self; 7] = [
        Self::AC1014,
        Self::AC1015,
        Self::AC1018,
        Self::AC1021,
        Self::AC1024,
        Self::AC1027,
        Self::AC1032,
    ];

    pub fn signature(self) -> &'static str {
        match self {
            Self::AC1014 => "AC1014",
            Self::AC1015 => "AC1015",
            Self::AC1018 => "AC1018",
            Self::AC1021 => "AC1021",
            Self::AC1024 => "AC1024",
            Self::AC1027 => "AC1027",
            Self::AC1032 => "AC1032",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::AC1014 => "R14 (AC1014)",
            Self::AC1015 => "2000 (AC1015)",
            Self::AC1018 => "2004 (AC1018)",
            Self::AC1021 => "2007 (AC1021)",
            Self::AC1024 => "2010 (AC1024)",
            Self::AC1027 => "2013 (AC1027)",
            Self::AC1032 => "2018 biçimi (AC1032)",
        }
    }
}

/// Import uses the same unit, layer routing, block and revision policy as DXF.
pub type ImportDwg = crate::ImportDxf;
pub type DwgImported = crate::DxfImported;

/// Export a drawing in a deliberately selected DWG binary format.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ExportDwg {
    /// Permit explicitly reported polyline approximation or loss. False rejects
    /// a conversion that cannot preserve bulges, widths or vertex elevations.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub allow_polyline_loss: bool,
    pub path: String,
    /// Only the named persistent objects; intersected with layers when both are given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub uids: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layers: Option<Vec<String>>,
    #[serde(default)]
    pub overwrite: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub annotations: Option<crate::DxfAnnotations>,
    /// A DWG file signature. AC1032 is shared by multiple AutoCAD product years.
    #[serde(default)]
    pub version: DwgVersion,
}

/// The export's file receipt; source and target versions are also in its report.
pub type DwgExported = crate::DxfExported;

/// A machine-readable CAD format profile exposed by the shared command catalog.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "ts", ts(export))]
#[serde(rename_all = "camelCase")]
pub struct CadFormatCapabilities {
    pub format: String,
    pub provider: String,
    pub local: bool,
    pub remote_upload: bool,
    pub read_versions: Vec<String>,
    pub write_versions: Vec<String>,
    pub encodings: Vec<String>,
    pub mid_decode_cancellation: bool,
    pub features: Vec<CadFeatureSupport>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "ts", ts(export))]
#[serde(rename_all = "camelCase")]
pub struct CadFeatureSupport {
    pub feature: String,
    /// native / appearance / reference / reportedLoss / rejected.
    pub read: String,
    pub write: String,
    pub note: String,
}

/// Portable paper-space exchange model; entity coordinates are metres on paper.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
#[serde(rename_all = "camelCase")]
pub struct CadLayout {
    pub name: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "mm")))]
    pub width_mm: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "mm")))]
    pub height_mm: f64,
    #[serde(default)]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub tab_order: u32,
    #[serde(default)]
    pub entities: Vec<crate::Entity>,
    #[serde(default)]
    pub viewports: Vec<CadViewport>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
#[serde(rename_all = "camelCase")]
pub struct CadViewport {
    #[serde(default,skip_serializing_if="std::ops::Not::not")]
    #[cfg_attr(feature="ts",ts(as="Option<bool>",optional))]
    pub locked:bool,
    /// Paper-space centre in millimetres, x right, y up.
    #[cfg_attr(feature="schema",schemars(extend("x-unit"="paper-mm")))]
    pub paper_center_mm: crate::Vec2,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "mm")))]
    pub width_mm: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "mm")))]
    pub height_mm: f64,
    /// Model-space centre in metres, WCS.
    pub view_center: crate::Vec2,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub view_height: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
    pub twist: f64,
}
