//! What the atlas draws its images from, as the host hands them over: an
//! SVG drawing as a small vector scene (paths with fills and strokes, groups
//! with opacity and luminance masks), a raster image as pixels, and a text
//! as glyph outlines. The host reads files and fonts (the desktop: SVG with
//! the SVG core's path and colour readers, glyphs from its text system); this
//! crate only rasterizes (`raster.rs`), so it knows no XML and no fonts.

use std::sync::Arc;

use kentos_native_style::batches::AtlasImage;

/// A path command, in the picture's user units (y down, as in SVG).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Segment {
    Move([f32; 2]),
    Line([f32; 2]),
    Quad([f32; 2], [f32; 2]),
    Cubic([f32; 2], [f32; 2], [f32; 2]),
    Close,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineCap {
    Butt,
    Round,
    Square,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineJoin {
    Miter,
    Round,
    Bevel,
}

/// A fill: sRGB colour with straight alpha, and its rule.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fill {
    pub color: [u8; 4],
    pub even_odd: bool,
}

/// A stroke in user units (it scales with the path's transform, as in SVG).
#[derive(Clone, Debug, PartialEq)]
pub struct Stroke {
    pub color: [u8; 4],
    pub width: f32,
    pub cap: LineCap,
    pub join: LineJoin,
    pub miter: f32,
    /// On and off lengths and the offset into them.
    pub dash: Option<(Vec<f32>, f32)>,
}

/// An SVG matrix `a b c d e f`: x' = a·x + c·y + e, y' = b·x + d·y + f.
pub type Matrix = [f32; 6];

pub const IDENTITY: Matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    /// A shape: its path under `transform`, filled then stroked.
    Path {
        segments: Vec<Segment>,
        transform: Matrix,
        fill: Option<Fill>,
        stroke: Option<Stroke>,
    },
    /// Children drawn together: faded by `opacity` and cut by a luminance
    /// `mask` (white shows, black hides), each only when set.
    Group {
        opacity: f32,
        mask: Option<Vec<Node>>,
        children: Vec<Node>,
    },
}

/// What an SVG or raster image is.
#[derive(Clone, Debug, PartialEq)]
pub enum Picture {
    /// A vector drawing: `view` is its viewBox (x, y, width, height), fitted
    /// into the image as `preserveAspectRatio="xMidYMid meet"` does.
    Vector { view: [f32; 4], nodes: Vec<Node> },
    /// Pixels, sRGB with straight alpha, row by row.
    Bitmap {
        width: u32,
        height: u32,
        rgba: Vec<u8>,
    },
}

/// A text's glyphs at `size` pixels: outlines with the baseline at y = 0 (y
/// down), the first glyph starting at x = 0, and the advance of the whole run.
#[derive(Clone, Debug, PartialEq)]
pub struct TextOutline {
    pub size: f32,
    pub advance: f32,
    pub glyphs: Vec<Vec<Segment>>,
}

/// A picture object's pixels (docs/adr/0192 §3): sRGB with straight alpha, row by row.
#[derive(Clone, Debug, PartialEq)]
pub struct Bitmap {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Where the atlas gets pictures and glyphs from (the host keeps them by key).
pub trait ImageSource {
    /// The picture of an SVG or raster image; None when it cannot be read
    /// (the atlas then draws the missing-image box).
    fn picture(&self, image: &AtlasImage) -> Option<Arc<Picture>>;

    /// A text's glyph outlines in a face of the web's font stack `font`
    /// (None: the interface's), `weight` and `italic`.
    fn text(
        &self,
        text: &str,
        font: Option<&str>,
        weight: f64,
        italic: bool,
    ) -> Option<Arc<TextOutline>>;

    /// A picture object's pixels by its key (`asset:<id>`, `file:<path>`;
    /// docs/adr/0192 §3); none while the host has none (light grey stands in).
    fn bitmap(&self, _key: &str) -> Option<Arc<Bitmap>> {
        None
    }

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

/// No pictures and no glyphs: shapes, strokes and fills still draw (tests, previews without fonts).
pub struct NoImages;

impl ImageSource for NoImages {
    fn picture(&self, _: &AtlasImage) -> Option<Arc<Picture>> {
        None
    }

    fn text(&self, _: &str, _: Option<&str>, _: f64, _: bool) -> Option<Arc<TextOutline>> {
        None
    }
}
