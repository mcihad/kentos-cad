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
