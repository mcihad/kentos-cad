//! KentOS's 3D scene core (plan/3d.md D1, docs/adr/0910 §3): what every 3D view computes on the CPU, apart from the
//! GPU and the window, so a road's terrain and corridor, a building, a point cloud or a twin's assets are seen through
//! one camera, one tile hierarchy and one level-of-detail rule.
//!
//! - [`math`]: f64 vectors and column-major 4×4 matrices.
//! - [`camera`]: the f64 camera (perspective or orthographic, an orbit), its view-only vertical exaggeration, its
//!   frustum, the translation-free rotation and projection the GPU takes (reversed depth, an infinite far plane for
//!   perspective), a tile's origin relative to the eye, a pixel's ray and a point's pixel.
//! - [`tile`]: tiles as 3D Tiles 1.1 has them: a key, bounds in the source's coordinates, a geometric error in metres,
//!   replacement or additive refinement, an optional content and children; a hierarchy and its rules.
//! - [`select`]: which tiles to draw and which contents to ask for, by the screen-space error (CesiumJS's rule: the
//!   geometric error times the viewport's height over the distance times 2·tan(fov/2); in orthographic, the error over
//!   a pixel's size), a parent kept until the children that replace it are ready, the asks by error.
//! - [`chunk`]: large triangle meshes cut into tiles of bounded triangles and extent, each with its own f64 origin and
//!   float32 positions relative to it, their source triangles kept (the lineage a pick refines through).
//! - [`pick`]: the first source triangle a pixel's ray meets among the tiles drawn, in f64 with the source's own
//!   coordinates, and the point, its height real.
//!
//! Precision (plan/3d.md §5, SCENE-PREC): the source stays f64. A tile's positions are float32 offsets from its own
//! origin, bounded by the tile's extent; the camera's eye is f64 and only a tile's origin minus the eye reaches the GPU,
//! as float32: the large numbers cancel in f64 before anything is rounded.

pub mod camera;
pub mod chunk;
pub mod math;
pub mod pick;
pub mod select;
pub mod tile;

pub use camera::{Camera, Exaggeration, Frustum, Projection, Viewport, Visibility};
pub use chunk::{Chunked, Content, Limits, SourceMesh, chunk};
pub use pick::{Hit, pick, ray_box, ray_triangle};
pub use select::{Selection, Settings, screen_error, select};
pub use tile::{Aabb, ContentRef, Hierarchy, Refine, Tile, TileKey};
