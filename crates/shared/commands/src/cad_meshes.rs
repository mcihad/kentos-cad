//! A mesh's metadata and the face and values at a place and time (docs/adr/0552; plan/gis.md `CBS-32`).
//!
//! A mesh in a project is a raster whose dataset names a UGRID topology of a NetCDF file (a 2DM with its DAT files
//! becomes such a file when it is added, ADR 0243). `cad.meshes.describe` reads the file's header and its
//! topology, and says what the mesh is: its nodes, faces, edges and triangles, its box and area, its system, and
//! its datasets (variables) with their dimensions, the time steps of the time dimension among them.
//!
//! `cad.meshes.query` finds the face a place lies in (exactly, the lowest triangle of those whose closed area
//! holds it) and answers it by a lasting id (the mesh's persistent id, its topology and the face's place in the
//! file's order) together with the datasets' values at the place: a node dataset's value interpolated linearly in
//! the triangle, a face dataset's value as it is, a vector's magnitude, nothing where a value is missing or the face
//! is inactive. The values are of the step at a moment (`time`: the last step at or before it) or of the slice the
//! raster shows; with `series`, of every step.
//!
//! `cad.meshes.region` answers the faces of an area (docs/adr/0645 §7): a face is in it when its centre (its area's
//! centroid; a non-convex face whose centroid falls outside it, its first triangle's centroid) lies in the closed
//! outer ring and not strictly inside a hole; each dataset's value of a face is its value at the centre as
//! `cad.meshes.query` answers it there, and the statistics are the faces' count, smallest, largest, mean weighted by
//! the faces' areas and the sum of value × area. `cad.meshes.profile` answers the stations of a path (0, `step`,
//! 2·`step` … and the path's end): each station's face and the datasets' values there, as the query does. The face
//! the query finds also says its neighbours (the faces sharing one of its edges).
//!
//! All read the file themselves, only the values they need. None writes anything.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::Vec2;
#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;

pub const CAD_MESHES_DESCRIBE: &str = "cad.meshes.describe";
pub const CAD_MESHES_DESCRIBE_VERSION: u32 = 1;
pub const CAD_MESHES_QUERY: &str = "cad.meshes.query";
pub const CAD_MESHES_QUERY_VERSION: u32 = 1;
pub const CAD_MESHES_REGION: &str = "cad.meshes.region";
pub const CAD_MESHES_REGION_VERSION: u32 = 1;
pub const CAD_MESHES_PROFILE: &str = "cad.meshes.profile";
pub const CAD_MESHES_PROFILE_VERSION: u32 = 1;

/// The most stations one `cad.meshes.profile` answers.
pub const MESH_MOST_STATIONS: u64 = 100_000;
/// The most face ids one `cad.meshes.region` lists (`faces`).
pub const MESH_MOST_LISTED: u64 = 100_000;

/// The most values (steps × the places a value reads) one `cad.meshes.query` reads.
pub const MESH_MOST_VALUES: u64 = 2_000_000;

/// Input of `cad.meshes.describe` v1. Refusals: `invalid_uid`, `entity_not_found`, `not_a_mesh`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MeshesDescribe {
    /// The mesh rasters (persistent ids); none: every mesh of the drawing, in the drawing's order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub uids: Vec<String>,
    /// The open drawing's file, a relative source's folder (the host gives it).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub drawing: Option<String>,
}

/// One of a dataset's dimensions besides its places: a time or another (a layer).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MeshDimension {
    pub name: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub count: u32,
    /// The values are a CF time axis's moments (milliseconds since 1970).
    pub time: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub units: Option<String>,
    /// The first and last value (a time's in milliseconds since 1970).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub first: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub last: Option<f64>,
    /// The first and last value as the windows say them (a time as the time slider does).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub first_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub last_label: Option<String>,
}

/// A dataset of a mesh.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MeshDatasetInfo {
    /// The variable (a vector's x component).
    pub variable: String,
    /// A vector's y component.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub vector: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub long_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub units: Option<String>,
    /// `node` or `face`.
    pub location: String,
    /// Its dimensions besides the places, in the file's order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<MeshDimension>>", optional))]
    pub dims: Vec<MeshDimension>,
}

/// The slice a mesh raster shows: the dataset and each of its dimensions' step.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MeshShown {
    pub variable: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub vector: Option<String>,
    /// The step of each of the dataset's dimensions besides the places.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<MeshStepOf>>", optional))]
    pub steps: Vec<MeshStepOf>,
}

/// A dimension's step shown.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MeshStepOf {
    pub name: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub index: u32,
    /// The value at the step as the windows say it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub label: Option<String>,
}

/// One mesh described.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MeshDescribed {
    /// The mesh raster's persistent id.
    pub uid: String,
    pub layer: String,
    /// The topology's name in the file.
    pub mesh: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub asset: Option<String>,
    /// Whether the file's mesh was read; when not, `why`.
    pub read: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub why: Option<String>,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub nodes: u64,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub faces: u64,
    /// The faces' edges, each once.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub edges: u64,
    /// The faces as triangles (a face of n nodes is n − 2).
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub triangles: u64,
    /// `[x₁, y₁, x₂, y₂]` in the mesh's system.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub bbox: Option<[f64; 4]>,
    /// The faces' area, together, in the mesh's system's unit squared.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub area: Option<f64>,
    /// The edges' mean length.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub mean_edge: Option<f64>,
    /// The mesh's EPSG code, when the file names one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub epsg: Option<u32>,
    /// The coordinates are longitude and latitude.
    pub geographic: bool,
    /// The raster's system in the project (0: none).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub srid: u32,
    pub datasets: Vec<MeshDatasetInfo>,
    /// What the raster shows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub shown: Option<MeshShown>,
    /// What the file holds that is not read, and why.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub notes: Vec<String>,
}

/// Output of `cad.meshes.describe` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MeshesDescribed {
    pub meshes: Vec<MeshDescribed>,
    #[cfg_attr(feature = "schema", schemars(extend("description" = REVISION_TEXT)))]
    pub revision: String,
}

/// Input of `cad.meshes.query` v1. Refusals: `invalid_uid`, `entity_not_found`, `not_a_mesh`,
/// `mesh_variable_not_found`, `invalid_moment`, `too_many_mesh_values`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MeshesQuery {
    /// The place in plan, in the mesh's system.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: Vec2,
    /// The meshes looked in, in this order; none: every visible mesh of the drawing. The first whose face holds the
    /// place answers.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub uids: Vec<String>,
    /// The moment of the values (milliseconds since 1970): each dataset's last time step at or before it; before its
    /// first, no value. None: the steps the raster shows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub time: Option<f64>,
    /// The datasets (variables) asked; none: every dataset of the mesh.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub variables: Vec<String>,
    /// Also answer each time-stepped dataset's value at every step.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub series: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub drawing: Option<String>,
}

/// The face found: its lasting id and what it is.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MeshFace {
    /// The mesh raster's persistent id.
    pub mesh: String,
    /// The topology's name in the file.
    pub topology: String,
    /// The face's place in the file's face order (0 and up).
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub face: u64,
    /// The face's nodes in the file's order (each from 0, in the file's node order).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub nodes: Vec<u32>,
    /// The face's area.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub area: f64,
    /// The faces sharing one of its edges (their places in the file's face order, ascending).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<f64>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub neighbours: Vec<u64>,
}

/// One step of a series.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MeshStepValue {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub step: u32,
    /// The step's moment (milliseconds since 1970) and as the windows say it.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub time: f64,
    pub label: String,
    /// None where the dataset has no value at the place and step.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub value: Option<f64>,
}

/// A dataset's value at the place.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MeshValue {
    pub variable: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub vector: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub units: Option<String>,
    /// `node` or `face`.
    pub location: String,
    /// The time step the value is of, when the dataset has a time dimension.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub step: Option<u32>,
    /// The step's moment (milliseconds since 1970).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub time: Option<f64>,
    /// None where the dataset has no value at the place (missing, an inactive face, no step at the moment).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub value: Option<f64>,
    /// With `series`: the value at every step.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<MeshStepValue>>", optional))]
    pub series: Vec<MeshStepValue>,
}

/// Output of `cad.meshes.query` v1: the face (none outside every mesh) and the values.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MeshesQueried {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub face: Option<MeshFace>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<MeshValue>>", optional))]
    pub values: Vec<MeshValue>,
    /// The values read from the file.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub read: u64,
    #[cfg_attr(feature = "schema", schemars(extend("description" = REVISION_TEXT)))]
    pub revision: String,
}

/// Input of `cad.meshes.region` v1. Refusals: `invalid_uid`, `entity_not_found`, `not_a_mesh`, `invalid_mesh_area`,
/// `mesh_variable_not_found`, `too_many_mesh_values`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MeshesRegion {
    /// The area's outer ring (at least three vertices, a repeated last vertex dropped), in the meshes' system.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub ring: Vec<Vec2>,
    /// Holes in the area.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<Vec<Vec2>>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub holes: Vec<Vec<Vec2>>,
    /// The meshes, in this order; none: every visible mesh of the drawing.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub uids: Vec<String>,
    /// The moment of the values (milliseconds since 1970), as `cad.meshes.query`'s; none: the steps the raster shows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub time: Option<f64>,
    /// The datasets asked; none: every dataset of the mesh.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub variables: Vec<String>,
    /// Also list the faces' places (at most 100 000 a mesh).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub faces: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub drawing: Option<String>,
}

/// A dataset over the faces of an area, or along a path.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MeshZonal {
    pub variable: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub vector: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub units: Option<String>,
    /// `node` or `face`.
    pub location: String,
    /// The time step the values are of, when the dataset has a time dimension.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub step: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub time: Option<f64>,
    /// The faces with a value (region only).
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub count: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub max: Option<f64>,
    /// The mean weighted by the faces' areas.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub mean: Option<f64>,
    /// The sum of value × the face's area.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub integral: Option<f64>,
}

/// One mesh's faces in an area.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MeshRegion {
    /// The mesh raster's persistent id.
    pub mesh: String,
    pub topology: String,
    /// The faces in the area.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub count: u64,
    /// Their areas, together.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub area: f64,
    /// With `faces`: their places in the file's face order, ascending.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<f64>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub faces: Vec<u64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<MeshZonal>>", optional))]
    pub datasets: Vec<MeshZonal>,
}

/// Output of `cad.meshes.region` v1.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MeshesRegioned {
    /// The area's own (the outer ring's less the holes').
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub area: f64,
    /// Each mesh read, in the asked order (a mesh whose file is not read is said by a warning).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<MeshRegion>>", optional))]
    pub meshes: Vec<MeshRegion>,
    /// The values read from the files.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub read: u64,
    #[cfg_attr(feature = "schema", schemars(extend("description" = REVISION_TEXT)))]
    pub revision: String,
}

/// Input of `cad.meshes.profile` v1. Refusals: `invalid_uid`, `entity_not_found`, `not_a_mesh`, `invalid_mesh_path`,
/// `too_many_stations`, `mesh_variable_not_found`, `too_many_mesh_values`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MeshesProfile {
    /// The path (at least two vertices, no part of no length), in the meshes' system.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub path: Vec<Vec2>,
    /// The stations' spacing along the path.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub step: f64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub uids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub time: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub variables: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub drawing: Option<String>,
}

/// A station of a path on a mesh.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MeshStation {
    /// Its distance along the path.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub station: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub x: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub y: f64,
    /// The face it lies in; none off the mesh.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(type = "number", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub face: Option<u64>,
    /// Each dataset's value there, in the datasets' order (null: no value).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub values: Vec<Option<f64>>,
}

/// One mesh's stations.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MeshProfile {
    pub mesh: String,
    pub topology: String,
    /// The datasets the stations' values are of (count, min, max, mean and integral left out).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<MeshZonal>>", optional))]
    pub datasets: Vec<MeshZonal>,
    pub stations: Vec<MeshStation>,
}

/// Output of `cad.meshes.profile` v1.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MeshesProfiled {
    /// The path's length.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub length: f64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<MeshProfile>>", optional))]
    pub meshes: Vec<MeshProfile>,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub read: u64,
    #[cfg_attr(feature = "schema", schemars(extend("description" = REVISION_TEXT)))]
    pub revision: String,
}
