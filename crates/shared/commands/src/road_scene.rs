//! KentOS Yol's 3D scene (plan/road.md Y12.3, docs/adr/0910 §3; plan/3d.md's first road scene): `file.export.gltf`
//! writes a road project's terrain surfaces and corridors' top surfaces as glTF 2.0 binary (GLB), one mesh each, their
//! positions float32 relative to one origin carried in full by the root node (plan/3d.md SCENE-PREC-04). The desktop's
//! handler is `crates/native/application/src/road_scene.rs`; it and the Python SDK pass the shared cases in
//! `fixtures/commands/v1/file.export.gltf.json`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const FILE_EXPORT_GLTF: &str = "file.export.gltf";
pub const FILE_EXPORT_GLTF_VERSION: u32 = 1;

/// Input of `file.export.gltf` v1: a road project's terrain surfaces and corridors to a GLB file, whole or not at all;
/// the path absolute, ending `.glb`; the surfaces and corridors by id or name (absent: every one; an empty list: none).
///
/// Refusals, in this order: `not_a_road_project`, `invalid_file_path`, `file_exists`, `unknown_terrain`,
/// `unknown_corridor`, `invalid_gltf` (a chosen surface or corridor that does not build or has no surface, nothing to
/// write). Warnings: `export_losses` (one not chosen by name that does not build, a project without a coordinate
/// system).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ExportGltf {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub surfaces: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub corridors: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub overwrite: bool,
}

/// What a mesh of the scene is made from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SceneMeshKind {
    /// A terrain surface's triangles.
    Terrain,
    /// A corridor's top surface.
    Corridor,
}

/// A mesh in the file: what it is made from, its source's id and name, its vertices and triangles, and its bounds
/// (the source's coordinates, x east, y north, z up).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SceneMeshWritten {
    pub kind: SceneMeshKind,
    pub id: String,
    pub name: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub vertices: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub triangles: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub min: [f64; 3],
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub max: [f64; 3],
}

/// Output of `file.export.gltf` v1, and its plan: the meshes, the origin the positions are relative to (the bounds'
/// middle to the whole metre), the project's coordinate system, the largest gap between a vertex and its float32
/// position plus the origin along any axis, the file's size and digest.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GltfExported {
    pub path: String,
    pub meshes: Vec<SceneMeshWritten>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub origin: [f64; 3],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub crs: Option<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub error: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "bytes")))]
    pub bytes: u64,
    pub sha256: String,
}
