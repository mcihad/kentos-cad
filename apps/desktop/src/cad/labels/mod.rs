//! The drawing's text over the scene (docs/adr/0055): text objects,
//! dimension values and object labels, as the web's overlay draws them
//! (`drawLabels`, apps/web/src/viewport/overlay.ts). Which of them a view
//! shows and where comes from the shared geometry store
//! ([`Spatial::labels`]: visible layer, box in view, size on screen, the
//! label style's scale range and smallest feature); strings, sizes and
//! colours from the object and its layer's label style, or the web's
//! default for its kind. Labels are thinned where they would cover one
//! another (8 px cells, as on the web); text objects and dimension values
//! always draw. Everything is in the project's typeface
//! (drawing_fonts.rs) with a halo of the drawing area's colour.
//!
//! The picture is kept while the view, the drawing and the colours stay;
//! upright text goes through Iced's glyph cache, turned text as outlines.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::rc::Rc;
use std::sync::Mutex;

use iced::advanced::graphics::text::Paragraph;
use iced::advanced::text::{self, Paragraph as _};
use iced::alignment::Vertical;
use iced::widget::canvas::{self, Frame, LineJoin, Path, Stroke};
use iced::{
    Color, Element, Fill, Font, Pixels, Point, Rectangle, Renderer, Size, Theme, Vector, mouse,
};
use kentos_commands::{DrawingFont, Entity, LabelInk};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::labels::engine::{HIDDEN, MASKED, PINNED, UNPLACED};
use kentos_geometry_core::text::paragraph::{self, Run, Script, advance};
use kentos_interaction::spatial::{Grow, LabelSize, default_label};
use kentos_interaction::spatial::LabelParts;
use kentos_interaction::{Format, LabelSpot, Spatial, Vec2};
use kentos_native_application::geometry::core_runs;
use kentos_render_wgpu::Camera;
use kentos_render_wgpu::color::{Palette, Rgba8};

use crate::app::Message;
use crate::drawing_fonts;
use crate::viewport::Canvas;
use kentos_native_style::color::ColorMode;

mod layer;
mod unicode;
#[cfg(test)]
mod parts_tests;

/// The labels a view shows, as last asked of the store, and what they were
/// asked for: the drawing (`drawing`, its changes), the view and the text
/// edited in place. The store walks every object in view to answer (all of
/// them in an overview), so a frame whose view and drawing stay (a pointer
/// move, a button's hover) takes them from here. While the view moves (a pan, the wheel) the last placing is shown as
/// it is, its labels on their ground, and placed again when the view settles (docs/adr/0597): a large drawing's
/// overview is not placed every frame.
#[derive(Default)]
pub struct Spots(RefCell<Option<Kept>>, Rc<RefCell<BandMemo>>);

/// The labels asked for, by the key, the drawing's key and the box they were asked for (true sizes only), and the
/// view's scale they were placed at.
type Kept = (u64, u64, Shown, Option<Bounds>, f64);

/// A view's box in the world.
type Bounds = kentos_geometry_core::geometry::Bounds;

/// The labels shown, each with how much it grows (docs/adr/0205 §5), object by object (docs/adr/0786 §1).
type Shown = Rc<LabelParts>;

impl Spots {
    /// The labels last placed (this frame's, once the labels' layer was built):
    /// a mask's shapes (docs/adr/0226 §3).
    pub fn last(&self) -> Option<Shown> {
        self.0.borrow().as_ref().map(|(_, _, spots, _, _)| spots.clone())
    }

    /// The bands its labels' parts were painted as (docs/adr/0786 §3).
    fn bands(&self) -> Rc<RefCell<BandMemo>> {
        self.1.clone()
    }

    /// The key the kept labels were asked for (a test's).
    #[cfg(test)]
    pub(crate) fn key(&self) -> Option<u64> {
        self.0.borrow().as_ref().map(|(key, _, _, _, _)| *key)
    }

    /// The labels for `key` (the drawing and the view), asked when not known; while `moving`, the last ones asked for
    /// the same drawing (`content`, the key without the view). Asked for `wide`, a box round the view (true sizes,
    /// docs/adr/0626 §7), the last ones serve while the view moves inside it and are asked again, moving or not,
    /// once it leaves it: a text coming into view is never missing. With them, the view's scale they were placed at.
    fn get(
        &self,
        key: u64,
        content: u64,
        moving: bool,
        (view, wide, scale): (Bounds, Option<Bounds>, f64),
        ask: impl FnOnce(Bounds) -> LabelParts,
    ) -> (Shown, f64) {
        let mut kept = self.0.borrow_mut();
        let inside = |asked: &Option<Bounds>| {
            asked.is_none_or(|b| {
                b.min_x <= view.min_x
                    && b.min_y <= view.min_y
                    && b.max_x >= view.max_x
                    && b.max_y >= view.max_y
            })
        };
        // Placed at a scale near this one: a quarter's zoom at most (a label grows and keeps apart with the scale; past
        // that, the old places were drawn over a view they no longer fitted until the view stopped).
        let near = |at: f64| at > 0.0 && scale > 0.0 && (scale / at).max(at / scale) <= 1.25;
        match kept.as_ref() {
            Some((known, _, spots, _, at)) if *known == key => (spots.clone(), *at),
            Some((_, same, spots, asked, at))
                if moving && *same == content && inside(asked) && near(*at) =>
            {
                (spots.clone(), *at)
            }
            _ => {
                let spots = Rc::new(ask(wide.unwrap_or(view)));
                *kept = Some((key, content, spots.clone(), wide, scale));
                (spots, scale)
            }
        }
    }
}

/// The labels' layer over the drawing area; `drawing` tells one open drawing from another.
#[allow(clippy::too_many_arguments)]
pub fn layer<'a>(
    doc: &'a Document,
    drawing: u64,
    spatial: &Spatial,
    kept: &Spots,
    camera: &Camera,
    canvas: Canvas,
    palette: &Palette,
    format: &Format,
    hidden: Option<Slot>,
    preview: Option<Preview>,
    mode: ColorMode,
    size: LabelSize,
    lift: f32,
    engine: Engine,
    shows: Option<&kentos_domain::LayerTree>,
) -> Element<'a, Message> {
    build(
        doc,
        drawing,
        spatial,
        kept,
        camera,
        canvas,
        palette,
        format,
        hidden,
        preview,
        Some(lift),
        mode,
        size,
        engine,
        shows,
    )
}

/// The bound layers' sources, by what they were when the layer last matched them (docs/adr/0533 §4): their id,
/// revision and the area a part holds. A label cache that did not hold them would show a placing made against a
/// source's other revision until the next unrelated change.
pub fn sources_key(doc: &Document) -> Vec<(String, String, Option<[u64; 4]>)> {
    doc.settings()
        .sources
        .iter()
        .map(|s| {
            (
                s.id.clone(),
                s.revision.clone(),
                s.extent.map(|e| e.map(f64::to_bits)),
            )
        })
        .collect()
}

/// What the label engine is asked besides the labels (docs/adr/0212 §4):
/// the unplaced and the hidden too, pinned ones outlined; the main view's
/// labels are kept for the tools' hit test.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Engine {
    pub options: kentos_geometry_core::store::placing::PlaceOptions,
    pub pinned: bool,
    pub keep: bool,
    /// The view is moving (a pan, the wheel): the last placing is kept until it settles (docs/adr/0597).
    pub moving: bool,
}

/// The same through Büyüteç's camera (docs/adr/0181 §5): the drawing's
/// text without the area's grid north, scale bar or axes.
#[allow(clippy::too_many_arguments)]
pub fn lens_layer<'a>(
    doc: &'a Document,
    drawing: u64,
    spatial: &Spatial,
    kept: &Spots,
    camera: &Camera,
    canvas: Canvas,
    palette: &Palette,
    format: &Format,
    hidden: Option<Slot>,
    preview: Option<Preview>,
    mode: ColorMode,
    size: LabelSize,
    engine: Engine,
) -> Element<'a, Message> {
    build(
        doc,
        drawing,
        spatial,
        kept,
        camera,
        canvas,
        palette,
        format,
        hidden,
        preview,
        None,
        mode,
        size,
        Engine {
            keep: false,
            moving: false,
            ..engine
        },
        None,
    )
}

/// A multi-line text being written or edited (paragraph_editor.rs,
/// docs/adr/0182 §4), drawn as it will be: its label records
/// (`paragraph_records`: its mask's box, then its lines), words and runs.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Preview {
    /// Session/revision/font of the immutable prepared editor preview, never persisted.
    pub revision: Option<(u64,u64,u8)>,
    pub text: String,
    pub runs: Vec<Run>,
    pub records: Vec<f64>,
    pub unicode: Option<Rc<kentos_interaction::spatial::UnicodeText>>,
    /// Its face (docs/adr/0183 §2): its typeface, bold, italic and slant.
    pub face: kentos_commands::TextFace,
}

#[allow(clippy::too_many_arguments)]
fn build<'a>(
    doc: &'a Document,
    drawing: u64,
    spatial: &Spatial,
    kept: &Spots,
    camera: &Camera,
    canvas: Canvas,
    palette: &Palette,
    format: &Format,
    hidden: Option<Slot>,
    preview: Option<Preview>,
    map_marks: Option<f32>,
    mode: ColorMode,
    size: LabelSize,
    engine: Engine,
    shows: Option<&kentos_domain::LayerTree>,
) -> Element<'a, Message> {
    // What the labels are of, the view aside: kept while the view moves (docs/adr/0597).
    let mut key = DefaultHasher::new();
    (
        drawing,
        // Every change, other editors' too (a revision counts only this user's).
        doc.generation(),
        hidden.map(|s| s.0),
    )
        .hash(&mut key);
    // How the text is sized on screen, the output scale a plot size reads (docs/adr/0205 §5); what the label
    // engine is asked (docs/adr/0212 §4).
    let asked = Engine {
        moving: false,
        ..engine
    };
    format!("{size:?}{asked:?}").hash(&mut key);
    // The bound layers' sources as the layers last matched them, and the part they hold (docs/adr/0533 §4): a
    // read that changes a label's text or place counts, whatever the change's counter says.
    sources_key(doc).hash(&mut key);
    // The time slider's window: its temporal layers' labels come and go (docs/adr/0210 §6).
    format!("{:?}", spatial.store().time_window()).hash(&mut key);
    // Nokta adları and Nokta kodları: points' texts (docs/adr/1540).
    spatial.point_texts().hash(&mut key);
    // The texts worked out again without a drawing change (an extension's answers, docs/adr/8057 §8): before, the
    // labels kept their old place and words until the next pan, zoom or edit.
    spatial.texts_version().hash(&mut key);
    // The rows round a part that hold room for its labels (docs/adr/0578 §1): they arrive after the layer.
    spatial.store().ghost_count(None).hash(&mut key);
    // A twisted view lays its text out upright and turns it as a picture (docs/adr/0285 §2).
    camera.twist.to_bits().hash(&mut key);
    // Kat görünümü (docs/adr/0715): what it hides, and what it draws otherwise (a roof its eaves alone, docs/adr/8044 §3),
    // changes the words placed. Before, a switch to Yalnız etkin kat kept the hidden level's room numbers and a roof's
    // slopes on the screen until the drawing changed.
    spatial.level_looks().0.hash(&mut key);
    // A view with a theme places the labels of the layers it shows (docs/adr/0523 §2).
    if let Some(tree) = shows {
        for leaf in tree.leaves() {
            (leaf.id.as_str(), tree.is_visible(&leaf.id)).hash(&mut key);
        }
    }
    let content = key.finish();
    (
        camera.center.x.to_bits(),
        camera.center.y.to_bits(),
        camera.scale.to_bits(),
        camera.width.to_bits(),
        camera.height.to_bits(),
    )
        .hash(&mut key);
    let turned = (camera.twist != 0.0).then_some(*camera);
    let camera = &camera.upright();
    // True sizes (docs/adr/0626 §6, §7): nothing grows or keeps apart, so a placing holds for any view; asked for
    // the view and a view's width round it, it serves a pan or a zoom out until the view leaves that box.
    let true_size = size == LabelSize::Print;
    let view = camera.visible_bounds();
    // Asked for a box round the view, every size: the last placing serves a move only inside it (and near its
    // scale, `Spots::get`). Before, outside true sizes no box was asked, the last placing served whatever the view
    // and a zoom out left the edges without labels until the view stopped (the owner: "zoom yaparken kayboluyor").
    let wide = {
        let (w, h) = (view.max_x - view.min_x, view.max_y - view.min_y);
        // True sizes keep a view's width round it (docs/adr/0626 §7); the others a quarter of it: their labels keep
        // apart, so a larger box costs the placing more (the engine adds its own 256 px round it too).
        let k = if true_size { 1.0 } else { 0.25 };
        // At least 256 screen pixels round it, the engine's own margin: a small window's pan stays inside.
        let least = if camera.scale > 0.0 {
            256.0 / camera.scale
        } else {
            0.0
        };
        let (dx, dy) = ((w * k).max(least), (h * k).max(least));
        Some(Bounds {
            min_x: view.min_x - dx,
            min_y: view.min_y - dy,
            max_x: view.max_x + dx,
            max_y: view.max_y + dy,
        })
    };
    let placed_scale = camera.scale;
    let (spots, placed_at) = kept.get(key.finish(), content, engine.moving, (view, wide, placed_scale), |view| {
        let mut spots = spatial.label_parts(
            Vec2::new(view.min_x, view.min_y),
            Vec2::new(view.max_x, view.max_y),
            camera.scale,
            size,
            engine.options,
            engine.keep,
            shows,
            true_size.then(|| cells_near(&view)),
        );
        // A text or a dimension's value being edited in place (the web's `setEditing`).
        if let Some(hidden) = hidden {
            spots.retain(|spot| match spot {
                LabelSpot::Text { slot, .. }
                | LabelSpot::Dimension { slot, .. }
                | LabelSpot::Line { slot, .. }
                | LabelSpot::ParagraphMask { slot, .. } => *slot != hidden,
                _ => true,
            });
        }
        spots
    });
    // What else the picture depends on, the view aside (`look`): the text being written changes it as it is
    // typed; the colours, the typeface and the number formats (a dimension's text).
    let mut look = DefaultHasher::new();
    content.hash(&mut look);
    if let Some(p) = &preview {
        p.text.hash(&mut look);
        format!("{:?}", p.runs).hash(&mut look);
        format!("{:?}", p.face).hash(&mut look);
        // The preview retains the immutable glyph plan. Hash its version, not a
        // megabyte-sized Debug string of every contour on each camera frame.
        p.revision.hash(&mut look);
        for r in &p.records {
            r.to_bits().hash(&mut look);
        }
    }
    let font = doc.settings().drawing_font.unwrap_or(DrawingFont::Barlow);
    (canvas as u8, font as u8, mode as u8).hash(&mut look);
    // Kat görünümü's faint level (docs/adr/0715 §2): another one draws the words again.
    let (level_version, level_looks) = spatial.level_looks();
    level_version.hash(&mut look);
    let faded: Rc<std::collections::HashSet<u32>> =
        Rc::new(level_looks.faded.iter().copied().collect());
    map_marks.map(f32::to_bits).hash(&mut look);
    format!("{format:?}").hash(&mut look);
    let look = look.finish();
    look.hash(&mut key);
    // What an object's bands are painted with besides it and its layer's colour (docs/adr/0786 §3): the view, the
    // colours, the typeface, the number formats and the faint level.
    let mut band_key = DefaultHasher::new();
    (
        camera.center.x.to_bits(),
        camera.center.y.to_bits(),
        camera.scale.to_bits(),
        camera.width.to_bits(),
        camera.height.to_bits(),
        camera.twist.to_bits(),
    )
        .hash(&mut band_key);
    (canvas as u8, font as u8, mode as u8, level_version).hash(&mut band_key);
    format!("{format:?}{palette:?}").hash(&mut band_key);
    let layered = true_size && turned.is_none();
    let labels = Labels {
        doc,
        map_layer_colors: None,
        own_colors: own_colors(doc),
        spots,
        view: *camera,
        camera: Cell::new(*camera),
        colors: colors(canvas, palette).in_mode(mode),
        font,
        format: *format,
        key: key.finish(),
        fence: None,
        current: Cell::new(None),
        faded,
        map_marks,
        preview: preview.map(Rc::new),
        pinned: engine.pinned,
        turned,
        true_size,
        bands: RefCell::new(Vec::new()),
        margin: Cell::new(0.0),
        band_memo: layered.then(|| kept.bands()),
        band_key: band_key.finish(),
        bands_out: layered.then_some(()),
        moving: engine.moving,
        // A moving view keeps a grown text as high on the screen as its placing had it (docs/adr/1064 §6).
        held: match size {
            _ if !engine.moving || placed_at <= 0.0 => None,
            LabelSize::Legible => Some((placed_at, true)),
            LabelSize::Screen { .. } => Some((placed_at, false)),
            _ => None,
        },
    };
    // True sizes keep their picture while the view moves (docs/adr/0626 §7); a twisted view's turns as a picture.
    if true_size && labels.turned.is_none() {
        return Element::new(layer::Layer {
            key: labels.key,
            content: look,
            moving: engine.moving,
            labels,
        });
    }
    canvas::Canvas::new(labels).width(Fill).height(Fill).into()
}

/// Labels as one part (a sheet's map's, a measurement's).
pub(crate) fn parts_of(spots: Vec<(LabelSpot, Grow)>) -> Shown {
    Rc::new(LabelParts {
        parts: vec![spots.into()],
        placed: Rc::new([]),
    })
}

/// The box a table's cells are asked in for the labels asked in `asked` (docs/adr/0786 §2): the asked box and
/// 0.55 of its larger side round it. A placing asked for a box serves every view inside it (`Spots::get`), and a
/// view draws a cell whose words start within a quarter of its larger side and the quarter of it its picture is
/// made round (`Labels::near_view`, labels/layer.rs's margin, rounded to a pixel): half the larger side of a view
/// no larger than the box, and the pixel's rounding (under a twentieth of it for a view over ten pixels).
pub(crate) fn cells_near(asked: &Bounds) -> Bounds {
    let side = (asked.max_x - asked.min_x).max(asked.max_y - asked.min_y) * 0.55;
    Bounds {
        min_x: asked.min_x - side,
        min_y: asked.min_y - side,
        max_x: asked.max_x + side,
        max_y: asked.max_y + side,
    }
}

/// The colours of the drawing's text on a canvas: the web's `--canvas-label`
/// (slate and paper; night dims it, black brightens it) and the halo of the
/// area's own colour.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Colors {
    pub label: Color,
    pub halo: Color,
    pub fg: Color,
    pub fg_dim: Color,
    palette: Palette,
    /// Görünüm kipleri's colour mode (docs/adr/0195 §1); the halo keeps the ground's colour.
    mode: ColorMode,
}

impl Colors {
    /// These colours as Görünüm kipleri's mode shows them (docs/adr/0195 §1).
    pub fn in_mode(mut self, mode: ColorMode) -> Self {
        self.mode = mode;
        self.label = self.shown(self.label);
        self.fg = self.shown(self.fg);
        self.fg_dim = self.shown(self.fg_dim);
        self
    }

    /// A colour as the mode shows it, its alpha kept: in one colour the
    /// palette's ink, in gray its brightness (the batches' rule, `view_rgba`).
    pub(crate) fn shown(&self, c: Color) -> Color {
        match self.mode {
            ColorMode::Color => c,
            ColorMode::Mono => {
                let ink = color(self.palette.ink);
                Color { a: c.a, ..ink }
            }
            ColorMode::Gray => {
                let y = 0.2126 * c.r + 0.7152 * c.g + 0.0722 * c.b;
                Color {
                    r: y,
                    g: y,
                    b: y,
                    a: c.a,
                }
            }
        }
    }

    /// The canvas's accent (the web's `--canvas-accent`): a pinned label's outline (docs/adr/0212 §4).
    fn accent(&self) -> Color {
        Color::from_rgb8(0xf2, 0xb6, 0x32)
    }

    /// A colour the drawing names (a layer's, an object's, a run's) as the mode shows it.
    fn named(&self, name: &str) -> Option<Color> {
        self.palette.resolve(name).map(|c| self.shown(color(c)))
    }
}

fn color(c: Rgba8) -> Color {
    Color::from_rgba8(c.0[0], c.0[1], c.0[2], f32::from(c.0[3]) / 255.0)
}

/// The paper's colours for the drawing's text on a sheet's map, on the screen and in its PDF:
/// the core's (`kentos_sheet::display::paper`, the web's `paperPalette`): #111111 letters on
/// white, whatever the theme; a dimension in a colour of its own keeps it.
pub fn paper_colors() -> Colors {
    use kentos_sheet::display::paper;
    let rgb = |hex: &str| {
        let v = u32::from_str_radix(hex.trim_start_matches('#'), 16).unwrap_or(0);
        Rgba8::rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
    };
    let palette = Palette {
        background: rgb(paper::PAPER),
        fg: rgb(paper::FG),
        fg_dim: rgb(paper::FG_DIM),
        ink: rgb(paper::INK),
    };
    Colors {
        label: color(rgb(paper::LABEL)),
        halo: color(rgb(paper::LABEL_HALO)),
        fg: color(palette.fg),
        fg_dim: color(palette.fg_dim),
        palette,
        mode: ColorMode::Color,
    }
}

pub fn colors(canvas: Canvas, palette: &Palette) -> Colors {
    let label = match canvas {
        Canvas::Slate => Color::from_rgb8(0xc4, 0xcd, 0xd7),
        Canvas::Paper => Color::from_rgb8(0x36, 0x41, 0x4d),
        Canvas::Night => Color::from_rgb8(0x9b, 0xa5, 0xb0),
        Canvas::Black => Color::from_rgb8(0xe8, 0xed, 0xf2),
    };
    Colors {
        label,
        halo: color(palette.background),
        fg: color(palette.fg),
        fg_dim: color(palette.fg_dim),
        palette: *palette,
        mode: ColorMode::Color,
    }
}

/// The look of the label engine's label being drawn (docs/adr/0212 §3.8).
struct Placed {
    slot: Slot,
    color: Color,
    /// Its halo's width (px) and colour; none with a filled background.
    halo: Option<(f32, Color)>,
    font: Font,
    /// Its shadow's offset on screen (px) and colour.
    shadow: Option<(f32, f32, Color)>,
    background: Option<Backdrop>,
    callout: Color,
    manhattan: bool,
    /// Outlined: pinned, with Sabit etiketleri vurgula on.
    pinned: bool,
}

/// A shape behind a placed label.
struct Backdrop {
    shape: kentos_commands::LabelShape,
    fill: Option<Color>,
    stroke: Option<Color>,
    pad: f32,
}

struct Labels<'a> {
    doc: &'a Document,
    map_layer_colors: Option<&'a HashMap<String, String>>,
    /// A CAD drawing's texts in their own colours, else their layers' (the owner's choice, docs/adr/8053 §14);
    /// elsewhere in the label colour, a map's words over its imagery.
    own_colors: bool,
    /// The labels and how each grows (docs/adr/0205 §5).
    spots: Shown,
    /// The view's camera, and the one the label being drawn goes through:
    /// the view's zoomed about a grown label's anchor.
    view: Camera,
    camera: Cell<Camera>,
    colors: Colors,
    font: DrawingFont,
    format: Format,
    /// What the picture depends on: while it holds, the cached picture stays.
    key: u64,
    /// A sheet's map frame: only what is anchored inside it is drawn (sheets.rs).
    fence: Option<Fence>,
    /// The object whose label is being drawn (a sheet PDF puts it in its layer).
    current: Cell<Option<Slot>>,
    /// The objects of the level drawn faint under the active one (docs/adr/0715 §2): their words faint as their lines.
    faded: Rc<std::collections::HashSet<u32>>,
    /// Grid north and the scale bar, or the axes, over the text (not in Büyüteç's window):
    /// how far the scale bar is lifted over the map services' credits.
    map_marks: Option<f32>,
    /// A multi-line text being written or edited, over the rest (docs/adr/0182 §4).
    preview: Option<Rc<Preview>>,
    /// Sabit etiketleri vurgula (docs/adr/0212 §4): pinned labels outlined.
    pinned: bool,
    /// A twisted view (docs/adr/0285 §2): the text is laid out on `view`, the same view with the
    /// north up over the box round the turned area, and drawn turned by the twist as a picture;
    /// the area's own camera draws the marks. None with the north up.
    turned: Option<Camera>,
    /// The drawing area's true sizes (docs/adr/0626 §6): no halos, a text under [`GREEK_PX`] a faint band
    /// (`bands`, filled together once the pieces are drawn), a piece away from the view left out.
    true_size: bool,
    bands: RefCell<Vec<Band>>,
    /// The pixels round the view painted too (labels/layer.rs), for this painting.
    margin: Cell<f32>,
    /// The bands each object's part was painted as, kept while the look holds (`band_key`; docs/adr/0786 §3): the
    /// drawing area's labels' only.
    band_memo: Option<Rc<RefCell<BandMemo>>>,
    band_key: u64,
    /// The bands are drawn by the layer as one mesh (labels/layer.rs), not stroked here.
    bands_out: Option<()>,
    /// The view is moving (docs/adr/0597): a turned text may take its picture kept for another quarter pixel
    /// (label_picture.rs, docs/adr/1064 §6).
    moving: bool,
    /// While the view moves, the scale the labels were placed at and whether a grown text never shrinks below its
    /// true size (Kaybolmasın; Ekranda sabit does not clamp): a grown text drawn as high on the screen as there
    /// (docs/adr/1064 §6). None: drawn by its placing's growth as it is.
    held: Option<(f64, bool)>,
}

/// A sheet's map frame as the labels see it: the camera's picture turned by
/// the map's own turn about the frame's middle; a text is drawn when its
/// anchor is inside the frame, and cut at the frame's edge (design §9a, the
/// PDF's rule; a turned frame's text is cut to its upright box on the screen).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fence {
    /// The camera's middle, in its own pixels.
    pub from: Point,
    /// The frame's size; its middle is where the camera's middle goes.
    pub size: Size,
    /// The map's turn, radians clockwise on the screen.
    pub turn: f32,
    /// The camera's pixels to the frame's (the labels laid out finer than the screen shows them).
    pub scale: f32,
}

impl Fence {
    /// Whether a point of the camera's picture falls inside the frame.
    fn holds(&self, p: Point) -> bool {
        let (s, c) = self.turn.sin_cos();
        let (x, y) = (
            (p.x - self.from.x) * self.scale,
            (p.y - self.from.y) * self.scale,
        );
        let (u, v) = (
            x * c - y * s + self.size.width / 2.0,
            x * s + y * c + self.size.height / 2.0,
        );
        u >= -0.5 && v >= -0.5 && u <= self.size.width + 0.5 && v <= self.size.height + 0.5
    }
}

/// The drawing's text in a sheet's map frame (sheets.rs; the web draws the
/// drawing area's own labels into a map's picture): the same pieces the
/// drawing area draws, with `camera` on the map's view (north up), the
/// frame already turned by the map's turn about its middle, in the paper's
/// colours; a text anchored outside the frame is left out. No north arrow or
/// scale bar.
#[allow(clippy::too_many_arguments)]
pub fn paint_in_map(
    frame: &mut Frame,
    doc: &Document,
    spots: Vec<LabelSpot>,
    camera: Camera,
    colors: Colors,
    font: DrawingFont,
    format: Format,
    fence: Fence,
    map_layer_colors: Option<&HashMap<String, String>>,
    faded: Rc<std::collections::HashSet<u32>>,
) {
    Labels {
        doc,
        map_layer_colors,
        own_colors: false,
        spots: parts_of(spots.into_iter().map(|s| (s, Grow::NONE)).collect()),
        view: camera,
        camera: Cell::new(camera),
        colors,
        font,
        format,
        key: 0,
        fence: Some(fence),
        current: Cell::new(None),
        faded,
        map_marks: None,
        preview: None,
        pinned: false,
        turned: None,
        true_size: false,
        bands: RefCell::new(Vec::new()),
        margin: Cell::new(0.0),
        band_memo: None,
        band_key: 0,
        bands_out: None,
        moving: false,
        held: None,
    }
    .paint(frame);
}

// A label of a sheet's map as the PDF writes it is the plotter's (docs/adr/0252): laid out here.
#[cfg(test)]
pub use kentos_plot::maps::MapLabel;

/// The drawing's text in a sheet's map as [`paint_in_map`] lays it out (the same pieces, the
/// same room between them, the same fence), as a list rather than a picture.
#[allow(clippy::too_many_arguments)]
#[cfg(test)]
pub fn texts_in_map(
    doc: &Document,
    spots: Vec<LabelSpot>,
    camera: Camera,
    colors: Colors,
    font: DrawingFont,
    format: Format,
    fence: Fence,
) -> Vec<MapLabel> {
    collect(doc, spots, camera, colors, font, format, Some(fence), false)
}

/// The drawing's text as the drawing area paints it (docs/adr/8053 §14: a CAD drawing's in their own colours), as a
/// list.
#[cfg(test)]
pub fn texts_on_screen(
    doc: &Document,
    spots: Vec<LabelSpot>,
    camera: Camera,
    colors: Colors,
) -> Vec<MapLabel> {
    collect(
        doc,
        spots,
        camera,
        colors,
        DrawingFont::Barlow,
        Format::default(),
        None,
        own_colors(doc),
    )
}

/// Whether the drawing area paints a drawing's texts in their own colours: a CAD drawing's (docs/adr/8053 §14).
fn own_colors(doc: &Document) -> bool {
    doc.settings().workspace == Some(kentos_commands::Workspace::Cad)
}

#[allow(clippy::too_many_arguments)]
#[cfg(test)]
fn collect(
    doc: &Document,
    spots: Vec<LabelSpot>,
    camera: Camera,
    colors: Colors,
    font: DrawingFont,
    format: Format,
    fence: Option<Fence>,
    own_colors: bool,
) -> Vec<MapLabel> {
    let mut list = Collect {
        doc,
        camera,
        labels: Vec::new(),
    };
    Labels {
        doc,
        map_layer_colors: None,
        own_colors,
        spots: parts_of(spots.into_iter().map(|s| (s, Grow::NONE)).collect()),
        view: camera,
        camera: Cell::new(camera),
        colors,
        font,
        format,
        key: 0,
        fence,
        current: Cell::new(None),
        faded: Rc::default(),
        map_marks: None,
        preview: None,
        pinned: false,
        turned: None,
        true_size: false,
        bands: RefCell::new(Vec::new()),
        margin: Cell::new(0.0),
        band_memo: None,
        band_key: 0,
        bands_out: None,
        moving: false,
        held: None,
    }
    .paint(&mut list);
    list.labels
}

/// The view zoomed `g.k` times about `g.anchor`, the anchor staying where it
/// is on screen (docs/adr/0205 §5): a grown label is drawn through it.
fn grown(view: Camera, g: Grow) -> Camera {
    if g.k == 1.0 || !g.k.is_finite() || g.k <= 0.0 {
        return view;
    }
    Camera {
        center: kentos_render_wgpu::Vec2::new(
            g.anchor.x - (g.anchor.x - view.center.x) / g.k,
            g.anchor.y - (g.anchor.y - view.center.y) / g.k,
        ),
        scale: view.scale * g.k,
        ..view
    }
}

/// A grown label while the view moves (docs/adr/1064 §6): drawn at the scale its placing at `placed` drew it (the
/// growth makes a grown text's screen height the scale's own, so a fresh placing would give the same), the same
/// number each frame, so its picture is found again; never below the view's scale when `clamp` (Kaybolmasın grows a
/// text, never shrinks it). A text its placing drew at its true size stays as the view has it.
fn held_grown(view: Camera, g: Grow, (placed, clamp): (f64, bool)) -> Camera {
    if g.k == 1.0 || !g.k.is_finite() || g.k <= 0.0 || !(view.scale > 0.0) {
        return view;
    }
    // A pan: the scale is the placing's, the growth as it was, to the bit.
    if placed == view.scale {
        return grown(view, g);
    }
    let target = placed * g.k;
    let target = if clamp && target < view.scale { view.scale } else { target };
    let k = view.scale / target;
    Camera {
        center: kentos_render_wgpu::Vec2::new(
            g.anchor.x - (g.anchor.x - view.center.x) * k,
            g.anchor.y - (g.anchor.y - view.center.y) * k,
        ),
        scale: target,
        ..view
    }
}

/// The pieces as the PDF takes them: each anchored on the ground as the drawing area anchors it.
#[cfg(test)]
struct Collect<'a> {
    doc: &'a Document,
    camera: Camera,
    labels: Vec<MapLabel>,
}

#[derive(Default)]
struct State {
    cache: canvas::Cache,
    key: Cell<u64>,
}

impl canvas::Program<Message> for Labels<'_> {
    type State = State;

    fn draw(
        &self,
        state: &State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        if state.key.get() != self.key {
            state.cache.clear();
            state.key.set(self.key);
        }
        vec![state.cache.draw(renderer, bounds.size(), |frame| {
            crate::label_picture::set_twisted(self.turned.is_some());
            crate::label_picture::set_moving(self.moving);
            let Some(area) = self.turned else {
                self.paint(frame);
                return;
            };
            // The upright picture's middle on the area's, turned by the twist (docs/adr/0285 §2).
            frame.with_save(|f| {
                f.translate(Vector::new(
                    (area.width / 2.0) as f32,
                    (area.height / 2.0) as f32,
                ));
                f.rotate(area.twist as f32);
                f.translate(Vector::new(
                    -(self.view.width / 2.0) as f32,
                    -(self.view.height / 2.0) as f32,
                ));
                self.paint(f);
            });
            if let Some(lift) = self.map_marks {
                let ucs = self.format.ucs.map(|u| u[2]);
                frame.marks(&area, &self.colors, self.format.axes, ucs, lift);
            }
        })]
    }
}

/// Where a text's anchor sits on it: the web's `textAlign` and `textBaseline`.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Anchor {
    /// Left end, on the baseline (text objects).
    LeftBaseline,
    /// Middle, on the baseline (dimension values).
    CenterBaseline,
    /// Middle, halfway up (the label engine's lines and letters, docs/adr/0212 §3.8).
    CenterMiddle,
}

/// One text to draw: at a point on screen, turned by `angle` radians
/// clockwise on screen, `size` pixels high, its letters `width_factor`
/// wide; `mask` pixels long, the box filled with the drawing area's colour
/// under it first (a text object's, docs/adr/0145; a dimension value's,
/// docs/adr/0147), 0 for none; `underline` pixels long, a bar under it (a
/// multi-line text's run, docs/adr/0182 §3), 0 for none; `lean` how far its
/// letters' tops move along per pixel up (a slant's tangent, docs/adr/0183
/// §2; a family without an italic face leant as a browser leans it), the
/// mask with them.
struct Piece<'t> {
    text: &'t str,
    at: Point,
    angle: f32,
    size: f32,
    font: Font,
    anchor: Anchor,
    color: Color,
    width_factor: f32,
    mask: f32,
    underline: f32,
    lean: f32,
}

/// A text's typeface, weight, italic and lean as drawn (docs/adr/0183 §2):
/// its own typeface upright at 400 (600 bold, its italic face when italic)
/// leaning by its slant; without one the project's in `legacy` (italic for
/// a text of one line, as always); a run's bold and italic added. A family
/// without an italic face is leant as a browser leans it.
fn face_font(
    face: &kentos_commands::TextFace,
    project: DrawingFont,
    legacy_italic: bool,
    run: Option<&Run>,
) -> (Font, f32) {
    let (bold_run, italic_run) = run.map_or((false, false), |r| (r.bold, r.italic));
    let (family, weight, italic, slant) = match face.font {
        Some(f) => (
            f,
            if face.bold || bold_run { 600 } else { 400 },
            face.italic || italic_run,
            face.oblique.map_or(0.0, |o| o.to_radians().tan() as f32),
        ),
        None => (
            project,
            if bold_run { 600 } else { 400 },
            legacy_italic || italic_run,
            0.0,
        ),
    };
    let synthetic = if italic && !drawing_fonts::has_italic(family) {
        drawing_fonts::SYNTHETIC_ITALIC
    } else {
        0.0
    };
    (
        drawing_fonts::font(family, weight, italic),
        slant + synthetic,
    )
}

/// A block's piece's face as the contract writes it (the core's `Face`).
fn contract_face(face: &kentos_geometry_core::text::face::Face) -> kentos_commands::TextFace {
    kentos_native_application::geometry::contract_face(face.clone())
}

impl Labels<'_> {
    fn layer_color<'s>(&'s self, layer: Option<&'s kentos_commands::LayerNode>) -> Option<&'s str> {
        let layer = layer?;
        self.map_layer_colors
            .and_then(|colors| colors.get(&layer.id))
            .map(String::as_str)
            .or(Some(layer.style.color.as_str()))
    }

    /// A text in its style's embedded SHX font (ADR 1125): its mask, then the font's own strokes, the PDF's and the
    /// CPU picture's lines. False when it is no such text or the font cannot draw it: it is drawn as before, in its
    /// style's face, and the inspection's `text_shx_unsupported` or `text_font_substituted` says so.
    fn shx_text(&self, frame: &mut impl Ink, slot: Slot, t: &kentos_commands::TextEntity, color: Color) -> bool {
        let Some(placed) =
            kentos_native_application::shx_text::place_cached(self.doc.settings(), self.doc.styles(), t)
        else {
            return false;
        };
        let Ok(placed) = placed.as_ref() else {
            return false;
        };
        if let Some(fence) = &self.fence
            && !fence.holds(self.screen(Vec2::new(t.p.x, t.p.y)))
        {
            return true;
        }
        // A word of the level drawn faint under the active one (docs/adr/0715 §2), as `draw` fades it.
        let color = if self.faded.contains(&slot.0) {
            Color { a: color.a * (1.0 - kentos_commands::PHASE_FADED as f32), ..color }
        } else {
            color
        };
        if t.mask {
            let ring: Vec<Point> = placed.outline.iter().map(|p| self.screen(*p)).collect();
            frame.fill_ring(&ring, Color { a: 1.0, ..self.colors.halo }, Some(slot));
        }
        for path in &placed.paths {
            let pts: Vec<Point> = path.iter().map(|p| self.screen(*p)).collect();
            frame.stroke_line(&pts, false, color, Some(slot));
        }
        true
    }

    fn screen(&self, p: Vec2) -> Point {
        let [x, y] = self.camera.get().world_to_screen(p);
        Point::new(x as f32, y as f32)
    }

    /// A layer's (or the object's) colour as the dimension's text takes it:
    /// the label colour for none and the theme's ink, else the colour itself.
    /// A grown leader's arrowhead (docs/adr/0205 §5, §7): its areas solid
    /// and its lines, `k` times its own about its tip, in `color`, over the
    /// one the scene draws as it is.
    fn grown_arrowhead(
        &self,
        frame: &mut impl Ink,
        l: &kentos_commands::LeaderEntity,
        k: f64,
        color: Color,
    ) {
        let s = kentos_native_application::geometry::shape(&Entity::Leader(l.clone()));
        let (Some(layout), Some(tip)) = (
            kentos_geometry_core::geom::leader::layout_of(&s),
            l.pts.first(),
        ) else {
            return;
        };
        let view = self.view;
        let at = |p: Vec2| {
            let q = Vec2::new(tip.x + (p.x - tip.x) * k, tip.y + (p.y - tip.y) * k);
            let [x, y] = view.world_to_screen(q);
            Point::new(x as f32, y as f32)
        };
        for ring in &layout.head.fills {
            let pts: Vec<Point> = ring.iter().map(|p| at(*p)).collect();
            frame.fill_ring(&pts, color, None);
        }
        for line in &layout.head.lines {
            let pts: Vec<Point> = line.pts.iter().map(|p| at(*p)).collect();
            frame.stroke_line(&pts, line.closed, color, None);
        }
    }

    /// A table cell's fill (docs/adr/0327 §1): its colour as the drawing's colours show it.
    fn fill_cell(&self, frame: &mut impl Ink, quad: [Point; 4], fill: &str, of: Option<Slot>) {
        let mut color = self.colors.named(fill).unwrap_or(self.colors.label);
        // On a dark drawing (light letters) a light fill is a tint, so the letters stay read.
        let l = self.colors.label;
        if 0.2126 * l.r + 0.7152 * l.g + 0.0722 * l.b > 0.5 {
            color.a *= 0.3;
        }
        frame.fill_quad(quad, color, of);
    }

    fn ink_of(&self, name: Option<&str>) -> Color {
        match name {
            None | Some("fg" | "fg-dim") => self.colors.label,
            Some(c) => self.colors.named(c).unwrap_or(self.colors.label),
        }
    }

    /// A text's colour (docs/adr/8053 §14): in a CAD drawing and in a sheet's map its own, else its layer's (the
    /// theme's ink the label colour); elsewhere the label colour.
    fn text_ink(&self, own: Option<&str>, layer: Option<&kentos_commands::LayerNode>) -> Color {
        if self.own_colors || self.map_layer_colors.is_some() {
            self.ink_of(own.or(self.layer_color(layer)))
        } else {
            self.colors.label
        }
    }

    /// A block's text's colour (docs/adr/8053 §14): the piece's own, else the layer it follows (`followed`: the
    /// insert's on the block's layer), else the insert's own, else the insert's layer's. Before, a block's text took
    /// the insert's even in a sheet's map: a definition's green words came out in the insert's colour.
    fn piece_ink(
        &self,
        ink: &kentos_interaction::spatial::PieceInk,
        insert: &kentos_commands::EntityBase,
        layer: Option<&kentos_commands::LayerNode>,
    ) -> Color {
        let followed = match ink.layer.as_deref() {
            Some("") => self.layer_color(layer),
            Some(id) => self.layer_color(self.doc.layers().get(id)),
            None => None,
        };
        self.text_ink(
            ink.color
                .as_deref()
                .or(followed)
                .or(insert.color.as_deref()),
            layer,
        )
    }

}

/// Text measured at this size; widths and baselines grow with the size.
const REFERENCE: f32 = 100.0;
/// The halo's width each side (the web strokes 3 px under the text).
const HALO: f32 = 1.5;
/// Text smaller than this on screen is not drawn (the store keeps dimensions to 5 … 240 px).
const SMALLEST: f32 = 1.0;
/// Under true sizes a text lower than this on screen is a faint band, not letters (docs/adr/0626 §6).
const GREEK_PX: f32 = 3.0;

/// A text's length in its height, as the drawing's tables measure it (`text::width_em`, Barlow's: a band and
/// whether a piece reaches into view need no shaping).
fn width_em(text: &str) -> f32 {
    kentos_geometry_core::text::width_em(text, kentos_geometry_core::text::Font::DEFAULT) as f32
}

/// A text too small to read, as a faint band (docs/adr/0626 §6): its line through the middle of the band, the band's
/// thickness and its colour.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Band {
    color: Color,
    thick: f32,
    line: [Point; 2],
}

/// The bands each object's part of the labels was painted as, while the look they were painted in holds
/// (docs/adr/0786 §3): a part that drew nothing but bands (a table's cells or a text too small to read, as in an
/// overview) is its bands again while it is the same part, of the same object, its layer the same colour. The memo
/// holds what it compares (the part, the object), so an address it keeps is never another's.
#[derive(Default)]
pub(crate) struct BandMemo {
    key: Option<(u64, u32)>,
    round: u64,
    kept: HashMap<usize, BandsKept>,
    /// How many parts the last painting took as kept and how many it painted (a measurement's).
    taken: (usize, usize),
}

struct BandsKept {
    part: Rc<[(LabelSpot, Grow)]>,
    entity: std::sync::Arc<Entity>,
    /// A table's cells' words and look (`Labels::cells_print`): the same cells of another table's version paint the
    /// same.
    cells: Option<u64>,
    layer_color: Option<String>,
    bands: Vec<Band>,
    round: u64,
}

impl BandMemo {
    /// A painting begins in `key`'s look, `margin` pixels round the view; another look forgets every part.
    fn begin(&mut self, key: u64, margin: f32) {
        let key = (key, margin.to_bits());
        if self.key != Some(key) {
            self.kept.clear();
            self.key = Some(key);
        }
        self.round += 1;
        self.taken = (0, 0);
    }

    /// `part`'s bands as kept, when it is the part they were painted for, of `entity` (or of another version of a
    /// table whose `cells` print is the same), its layer `layer_color`.
    fn bands_of(
        &mut self,
        part: &Rc<[(LabelSpot, Grow)]>,
        entity: Option<&std::sync::Arc<Entity>>,
        layer_color: Option<String>,
        cells: impl FnOnce() -> Option<u64>,
    ) -> Option<&[Band]> {
        let Some(k) = self.kept.get_mut(&(Rc::as_ptr(part) as *const () as usize)) else {
            self.taken.1 += 1;
            return None;
        };
        let Some(entity) = entity else {
            self.taken.1 += 1;
            return None;
        };
        let same = Rc::ptr_eq(&k.part, part)
            && k.layer_color == layer_color
            && (std::sync::Arc::ptr_eq(&k.entity, entity) || k.cells.is_some() && k.cells == cells());
        if !same {
            self.taken.1 += 1;
            return None;
        }
        k.entity = entity.clone();
        k.round = self.round;
        self.taken.0 += 1;
        Some(&k.bands)
    }

    /// `part` painted as `bands` alone.
    fn keep(
        &mut self,
        part: &Rc<[(LabelSpot, Grow)]>,
        entity: &std::sync::Arc<Entity>,
        (layer_color, cells): (Option<String>, Option<u64>),
        bands: &[Band],
    ) {
        self.kept.insert(
            Rc::as_ptr(part) as *const () as usize,
            BandsKept {
                part: part.clone(),
                entity: entity.clone(),
                cells,
                layer_color,
                bands: bands.to_vec(),
                round: self.round,
            },
        );
    }

    /// The painting ended: the parts it did not meet are forgotten.
    fn end(&mut self) {
        let round = self.round;
        self.kept.retain(|_, k| k.round == round);
    }

    /// How many parts the last painting took as kept and how many it painted again.
    /// (Read by the tests: the painting itself needs no count.)
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn taken(&self) -> (usize, usize) {
        self.taken
    }
}

/// A quick hash of a part's words (`Labels::cells_print`): eight bytes a step, folded and multiplied, as rustc's
/// own `FxHasher`; a print compares a part with itself in another version, nothing is kept by it.
#[derive(Default)]
struct Fx(u64);

impl std::hash::Hasher for Fx {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        let mut chunks = bytes.chunks_exact(8);
        for c in &mut chunks {
            self.write_u64(u64::from_le_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]));
        }
        let mut last = [0u8; 8];
        let rest = chunks.remainder();
        last[..rest.len()].copy_from_slice(rest);
        self.write_u64(u64::from_le_bytes(last) ^ rest.len() as u64);
    }

    fn write_u64(&mut self, n: u64) {
        self.0 = (self.0.rotate_left(5) ^ n).wrapping_mul(0x517c_c1b7_2722_0a95);
    }

    fn write_usize(&mut self, n: usize) {
        self.write_u64(n as u64);
    }
}

/// A picture's ink that counts what is drawn on it (docs/adr/0786 §3): a part whose painting drew nothing on it
/// drew bands alone.
struct Counted<'f, I: Ink> {
    ink: &'f mut I,
    ops: usize,
}

impl<I: Ink> Ink for Counted<'_, I> {
    fn shaped(&mut self,piece:&unicode::Piece<'_>) {self.ops+=1;self.ink.shaped(piece);}
    fn size(&self) -> Size {
        self.ink.size()
    }

    fn piece(&mut self, piece: &Piece<'_>, halo: Color, of: Option<Slot>) {
        self.ops += 1;
        self.ink.piece(piece, halo, of);
    }

    fn fill_quad(&mut self, quad: [Point; 4], color: Color, of: Option<Slot>) {
        self.ops += 1;
        self.ink.fill_quad(quad, color, of);
    }

    fn stroke_segments(&mut self, segments: &[[Point; 2]], color: Color, width: f32) {
        self.ops += 1;
        self.ink.stroke_segments(segments, color, width);
    }

    fn marks(
        &mut self,
        camera: &Camera,
        colors: &Colors,
        axes: kentos_interaction::Axes,
        ucs: Option<f64>,
        lift: f32,
    ) {
        self.ops += 1;
        self.ink.marks(camera, colors, axes, ucs, lift);
    }

    fn fill_ring(&mut self, ring: &[Point], color: Color, of: Option<Slot>) {
        self.ops += 1;
        self.ink.fill_ring(ring, color, of);
    }

    fn stroke_line(&mut self, pts: &[Point], closed: bool, color: Color, of: Option<Slot>) {
        self.ops += 1;
        self.ink.stroke_line(pts, closed, color, of);
    }
}

/// The band of a text too small to read: `length` long and 0.6 of its size thick (a pixel at least, in eighths of
/// one so that bands of a colour stroke together), over its baseline or about its middle as its anchor puts it,
/// turned as it is; its colour faint.
fn band(piece: &Piece<'_>, length: f32) -> Band {
    let length = length.max(1.0);
    let thick = ((piece.size * 0.6).max(1.0) * 8.0).round() / 8.0;
    let x0 = match piece.anchor {
        Anchor::LeftBaseline => 0.0,
        Anchor::CenterBaseline | Anchor::CenterMiddle => -length / 2.0,
    };
    let y = match piece.anchor {
        Anchor::LeftBaseline | Anchor::CenterBaseline => -thick / 2.0,
        Anchor::CenterMiddle => 0.0,
    };
    let (sin, cos) = piece.angle.sin_cos();
    let at = |x: f32, y: f32| Point::new(piece.at.x + x * cos - y * sin, piece.at.y + x * sin + y * cos);
    Band {
        color: Color {
            a: piece.color.a * 0.45,
            ..piece.color
        },
        thick,
        line: [at(x0, y), at(x0 + length, y)],
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Measure {
    width: f32,
    /// The baseline below the top of a line one size high.
    baseline: f32,
}

/// Text widths by face, kept between frames: a pan measures each text once
/// (the web's `widths`). Forgets everything when it grows large.
static MEASURES: Mutex<Option<HashMap<Font, HashMap<String, Measure>>>> = Mutex::new(None);
/// Measured texts kept at most (a dense view offers tens of thousands).
const MEASURES_KEPT: usize = 200_000;

/// A text shaped at the reference size: its width and where its baseline falls.
fn shaped(text: &str, font: Font) -> Measure {
    let p = Paragraph::with_text(text::Text {
        content: text,
        bounds: Size::INFINITE,
        size: Pixels(REFERENCE),
        line_height: text::LineHeight::Relative(1.0),
        font,
        align_x: text::Alignment::Left,
        align_y: Vertical::Top,
        shaping: text::Shaping::Advanced,
        wrapping: text::Wrapping::None,
    });
    let run = p.buffer().layout_runs().next();
    Measure {
        width: run.as_ref().map_or(0.0, |r| r.line_w),
        baseline: run.as_ref().map_or(REFERENCE * 0.8, |r| r.line_y),
    }
}

fn measure(text: &str, font: Font) -> Measure {
    let Ok(mut all) = MEASURES.lock() else {
        return shaped(text, font);
    };
    let all = all.get_or_insert_with(HashMap::new);
    if all.values().map(HashMap::len).sum::<usize>() > MEASURES_KEPT {
        all.clear();
    }
    let known = all.entry(font).or_default();
    if let Some(m) = known.get(text) {
        return *m;
    }
    let m = shaped(text, font);
    known.insert(text.to_owned(), m);
    m
}

/// A text to come, faint, drawn as a text object will be (Etiketleri yazıya
/// çevir's and Koordinat yaz's previews, docs/adr/0175 §3, 0185 §5; the
/// web's `drawTextGhost`): `at` its point on the screen, `size` px high,
/// turned `angle` (screen radians), its point on its alignment (`along` of
/// its width, `up` of its height over its baseline), in its face and width
/// factor; over its mask in `halo` when it will have one.
#[allow(clippy::too_many_arguments)]
pub(crate) fn ghost(
    frame: &mut Frame,
    text: &str,
    at: Point,
    size: f32,
    angle: f32,
    (along, up): (f32, f32),
    (face, width_factor): (&kentos_commands::TextFace, f32),
    drawing: DrawingFont,
    (color, halo): (Color, Color),
    mask: bool,
) {
    if size < SMALLEST || text.is_empty() {
        return;
    }
    // A text with no face of its own has the project's look (docs/adr/0175 §1): italic, leant where the
    // family has no italic face.
    let (font, lean) = face_font(face, drawing, true, None);
    let width = measure(text, font).width * size / REFERENCE * width_factor;
    // From its point to where its baseline starts, in the text's own frame (y down the screen).
    let (dx, dy) = (-along * width, up * size);
    let (sin, cos) = angle.sin_cos();
    let piece = Piece {
        text,
        at: Point::new(at.x + dx * cos - dy * sin, at.y + dx * sin + dy * cos),
        angle,
        size,
        font,
        anchor: Anchor::LeftBaseline,
        color,
        width_factor,
        mask: if mask { width } else { 0.0 },
        underline: 0.0,
        lean,
    };
    draw(frame, &piece, halo);
}

/// A table's cell to come, faint (Tablo ekle's placement, docs/adr/0184 §3;
/// the web's `TablePlaceTool.draw`): its words from where their baseline
/// starts at `at`, `size` px high, upright in the table's face, bold when a
/// heading row's.
#[allow(clippy::too_many_arguments)]
pub(crate) fn cell_ghost(
    frame: &mut Frame,
    text: &str,
    at: Point,
    size: f32,
    bold: bool,
    face: &kentos_commands::TextFace,
    drawing: DrawingFont,
    (color, halo): (Color, Color),
) {
    if size < SMALLEST || text.is_empty() {
        return;
    }
    let heading = Run {
        start: 0,
        end: 0,
        bold,
        italic: false,
        underline: false,
        script: None,
        color: None,
    };
    let (font, lean) = face_font(face, drawing, false, Some(&heading));
    let piece = Piece {
        text,
        at,
        angle: 0.0,
        size,
        font,
        anchor: Anchor::LeftBaseline,
        color,
        width_factor: 1.0,
        mask: 0.0,
        underline: 0.0,
        lean,
    };
    draw(frame, &piece, halo);
}

/// A sample of a text style (the styles' window, docs/adr/0183 §5): `text`
/// from where its baseline starts at `at`, `size` px high, in `face` (else
/// the project's `project`, italic), its letters `width_factor` wide.
#[allow(clippy::too_many_arguments)]
pub(crate) fn sample_text(
    frame: &mut Frame,
    text: &str,
    at: Point,
    size: f32,
    face: &kentos_commands::TextFace,
    width_factor: f32,
    project: DrawingFont,
    (color, halo): (Color, Color),
) {
    let (font, lean) = face_font(face, project, true, None);
    let piece = Piece {
        text,
        at,
        angle: 0.0,
        size,
        font,
        anchor: Anchor::LeftBaseline,
        color,
        width_factor,
        mask: 0.0,
        underline: 0.0,
        lean,
    };
    draw(frame, &piece, halo);
}

/// A sample of a dimension's value (the styles' window): centred on its
/// baseline at `at`, turned `angle` (screen radians), in `font` at 500, over
/// its mask when `mask`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn sample_value(
    frame: &mut Frame,
    text: &str,
    at: Point,
    angle: f32,
    size: f32,
    font: DrawingFont,
    mask: bool,
    (color, halo): (Color, Color),
) {
    let font = drawing_fonts::font(font, 500, false);
    let piece = Piece {
        text,
        at,
        angle,
        size,
        font,
        anchor: Anchor::CenterBaseline,
        color,
        width_factor: 1.0,
        mask: value_mask(mask, text, font, size),
        underline: 0.0,
        lean: 0.0,
    };
    draw(frame, &piece, halo);
}

/// Draws a text with its halo: upright through the glyph cache, turned as outlines.
/// A dimension value's mask (docs/adr/0147): its measured width on screen when it has one, else 0.
fn value_mask(mask: bool, text: &str, font: Font, size: f32) -> f32 {
    if mask {
        measure(text, font).width * size / REFERENCE
    } else {
        0.0
    }
}

/// A piece's underline (docs/adr/0182 §3): from where its baseline starts at
/// `from`, 0.12 of its size under it, 0.06 thick, over a halo.
fn underline(frame: &mut Frame, piece: &Piece<'_>, from: Point, halo: Color) {
    if piece.underline <= 0.0 {
        return;
    }
    let (top, thick) = (piece.size * 0.12, (piece.size * 0.06).max(1.0));
    if halo.a > 0.0 {
        frame.fill(
            &Path::rectangle(
                Point::new(from.x - HALO, from.y + top - HALO),
                Size::new(piece.underline + 2.0 * HALO, thick + 2.0 * HALO),
            ),
            halo,
        );
    }
    frame.fill(
        &Path::rectangle(
            Point::new(from.x, from.y + top),
            Size::new(piece.underline, thick),
        ),
        piece.color,
    );
}

thread_local! {
    /// Shaping a text and taking its glyphs' outlines is most of a turned
    /// text's cost; the drawing's canvas paints on this thread only.
    static OUTLINES: RefCell<HashMap<OutlineKey, Rc<Vec<Path>>>> = RefCell::new(HashMap::new());
}

#[cfg(test)]
mod tests;

mod paint;

mod placed;

mod ink;
use ink::*;

mod turned;
pub use turned::*;

#[cfg(test)]
mod screens;
