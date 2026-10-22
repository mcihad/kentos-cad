//! Host services extending the pure CPU image source (CAD-SHEET-06).
pub use kentos_native_style::picture::{
    Bitmap, Fill, IDENTITY, LineCap, LineJoin, Matrix, NoImages, Node, Picture, Segment, Stroke,
    TextOutline,
};
use std::sync::Arc;

/// The CPU painter needs only pictures/glyphs; GPU map services add streaming tiles and clouds.
pub trait ImageSource: kentos_native_style::picture::ImageSource {
    /// A raster's tile, `raster_tiles::SLOT` × `SLOT` premultiplied RGBA, by
    /// its paint (`raster`: `asset:<id>`, `file:<path>`; `look`: its style's
    /// JSON; `affine`: its pixels' place, whose linear part sizes a shaded
    /// relief) and place (docs/adr/0204 §5) when the host has it; otherwise
    /// none, and the host starts making it and asks for a frame when done.
    fn raster_tile(
        &self,
        _raster: &str,
        _look: &str,
        _affine: &[f64; 6],
        _level: u32,
        _tx: u32,
        _ty: u32,
    ) -> Option<Arc<Vec<u8>>> {
        None
    }

    /// A frame of rasters begins: what the host was asked for before and is
    /// not asked for again by its end may be dropped.
    fn raster_frame(&self) {}

    /// A point cloud's octrees by its paint's `cloud` (`cloud:<hash>`;
    /// docs/adr/0207 §6); none while the host has not opened its files (the
    /// plan alone shows).
    fn cloud_trees(&self, _cloud: &str) -> Option<Arc<super::points::CloudTrees>> {
        None
    }

    /// A node of file `member` of a cloud, its points coloured for `look`
    /// (the style's JSON and the object's colour), when the host has it;
    /// otherwise none, and the host starts making it and asks for a frame when done.
    fn cloud_node(
        &self,
        _cloud: &str,
        _member: u32,
        _node: u32,
        _look: &str,
    ) -> Option<Arc<super::points::CloudNode>> {
        None
    }

    /// A frame of clouds begins: what is not asked for again by its end may be dropped.
    fn cloud_frame(&self) {}

    /// A map service's view by its paint's `service` (docs/adr/0208 §3):
    /// its grid and the transformations to and from the project's system;
    /// none while the host has not read what it needs (its capabilities, its
    /// style, a session), and nothing of it shows.
    fn service_view(&self, _service: &str) -> Option<Arc<super::service_tiles::ServiceView>> {
        None
    }

    /// A service's picture tile when the host has it; otherwise none, and the
    /// host starts getting it and asks for a frame when done.
    fn service_tile(
        &self,
        _service: &str,
        _t: kentos_geometry_core::geom::tiles::TileRef,
    ) -> Option<super::service_tiles::ServiceTile> {
        None
    }

    /// A service's vector tile styled for the display's `zoom` (the host
    /// keeps a few steps of it) when the host has it; otherwise none, as
    /// `service_tile`.
    fn service_vector(
        &self,
        _service: &str,
        _t: kentos_geometry_core::geom::tiles::TileRef,
        _zoom: f64,
    ) -> Option<Arc<super::service_tiles::VectorTile>> {
        None
    }

    /// A frame of services begins: what is not asked for again by its end may be dropped.
    fn service_frame(&self) {}
}

impl ImageSource for NoImages {}
