//! The drawing's text on the GPU (docs/adr/9212 §4, stage 2): the pieces of text objects, of a block's texts and of
//! the lines of a multi-line text without a mask go to the drawing area's renderer as glyphs
//! (`kentos_render_wgpu::glyphs`) instead of onto the canvas. Placing and layout stay the canvas's own, to the glyph:
//! a piece is shaped as the canvas shapes it (`shaped`: Iced's text at [`REFERENCE`]) and each glyph's face is found
//! among the drawing's faces by its bytes. A piece with a mask or an underline, or with a glyph from a face that is not
//! the drawing's (the text system fell back to another for a letter they lack), stays on the canvas, as does
//! everything else the labels draw: dimension values, the label engine's labels, cells, bands, masks.
//!
//! The canvas draws its picture in the frame's draw, before the scene is prepared (Iced draws the widgets, then
//! prepares and renders their primitives): a painting leaves its texts in the [`GlyphSlot`] and the scene draws them
//! in the same frame. A text is anchored on the map where the canvas puts it and placed from there in pixels, turning
//! with the view as the canvas's twisted picture turns, and growing with a zoom as the canvas's moved picture grows
//! until the labels are placed again.

use super::*;
use std::collections::HashSet;
use std::sync::{Arc, PoisonError};

use iced::advanced::graphics::text::cosmic_text::fontdb;
use kentos_geometry_core::text::shaping::FontId;
use kentos_render_wgpu::glyphs::{GlyphKey, GlyphPlace, GlyphSet, GlyphText, GlyphUnit};

/// Whether the drawing area's texts go to its renderer: `KENTOS_DIAG_TEXT=canvas` keeps them all on the canvas, for a
/// measurement or a comparison.
pub fn on() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var("KENTOS_DIAG_TEXT").as_deref() != Ok("canvas"))
}

/// The drawing area's texts for its renderer, as the last painting left them.
#[derive(Clone, Debug, Default)]
pub struct GlyphSlot(Arc<Mutex<GlyphSet>>);

impl GlyphSlot {
    /// The texts now (their list is shared, not copied).
    pub fn get(&self) -> GlyphSet {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    fn put(&self, texts: Vec<GlyphText>, placed_px: f64) {
        let mut set = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        if texts.is_empty() && set.texts.is_empty() {
            return;
        }
        set.version += 1;
        set.texts = Arc::new(texts);
        set.placed_px = placed_px;
    }
}

/// A painting's texts for the GPU, and whether the spot being painted may send its pieces there.
pub(super) struct Sink {
    slot: GlyphSlot,
    texts: RefCell<Vec<GlyphText>>,
    open: Cell<bool>,
    /// The objects whose multi-line text has a mask (a `ParagraphMask` comes before its lines): their lines stay on the
    /// canvas over it.
    masked: RefCell<HashSet<u32>>,
}

impl Sink {
    pub(super) fn new(slot: GlyphSlot) -> Sink {
        Sink {
            slot,
            texts: RefCell::new(Vec::new()),
            open: Cell::new(false),
            masked: RefCell::new(HashSet::new()),
        }
    }

    /// A painting starts.
    pub(super) fn begin(&self) {
        self.texts.borrow_mut().clear();
        self.masked.borrow_mut().clear();
        self.open.set(false);
    }

    /// A spot's painting starts: whether its pieces may go to the GPU.
    pub(super) fn spot(&self, spot: &LabelSpot) {
        let open = match spot {
            LabelSpot::ParagraphMask { slot, .. } => {
                self.masked.borrow_mut().insert(slot.0);
                false
            }
            LabelSpot::Text { .. } | LabelSpot::PieceText { .. } => true,
            LabelSpot::Line { slot, unicode: None, .. } => !self.masked.borrow().contains(&slot.0),
            _ => false,
        };
        self.open.set(open);
    }

    /// The spots are done: what follows (a text being written, the bands, the marks) stays on the canvas.
    pub(super) fn close(&self) {
        self.open.set(false);
    }

    /// The painting is done: its texts to the renderer, placed at `placed_px` (the view's map pixels a metre).
    pub(super) fn publish(&self, placed_px: f64) {
        self.slot.put(std::mem::take(&mut *self.texts.borrow_mut()), placed_px);
    }

    /// The piece as glyphs, when it may go: true when it went (or has nothing to draw).
    fn take(&self, piece: &Piece<'_>, halo: Color, view: &Camera) -> bool {
        if !self.open.get() || piece.mask > 0.0 || piece.underline > 0.0 || piece.size < SMALLEST {
            return false;
        }
        if piece.text.trim().is_empty() {
            return true;
        }
        let Some(run) = run(piece.text, piece.font) else {
            return false;
        };
        let at = view.screen_to_map(f64::from(piece.at.x), f64::from(piece.at.y));
        self.texts.borrow_mut().push(GlyphText {
            at: [at.x, at.y],
            unit: GlyphUnit::Px { turns: true },
            color: rgba(piece.color),
            halo: rgba(halo),
            halo_px: if halo.a > 0.0 { HALO } else { 0.0 },
            glyphs: places(piece, &run),
        });
        true
    }
}

fn rgba(c: Color) -> Rgba8 {
    let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    Rgba8([byte(c.r), byte(c.g), byte(c.b), byte(c.a)])
}

/// The canvas with the GPU beside it: a piece its spot may send to the GPU goes there, everything else onto the
/// canvas. Counted as the canvas is (`Counted`): a part whose words went to the GPU did not draw bands alone.
pub(super) struct Split<'f, 's> {
    pub(super) frame: &'f mut Frame,
    pub(super) sink: Option<&'s Sink>,
    /// The view the pieces are placed on (a twisted view's upright one).
    pub(super) view: &'s Camera,
}

impl Ink for Split<'_, '_> {
    fn size(&self) -> Size {
        Frame::size(self.frame)
    }

    fn piece(&mut self, piece: &Piece<'_>, halo: Color, of: Option<Slot>) {
        if self.sink.is_some_and(|sink| sink.take(piece, halo, self.view)) {
            return;
        }
        <Frame as Ink>::piece(self.frame, piece, halo, of);
    }

    fn shaped(&mut self, piece: &unicode::Piece<'_>) {
        <Frame as Ink>::shaped(self.frame, piece);
    }

    fn fill_quad(&mut self, quad: [Point; 4], color: Color, of: Option<Slot>) {
        <Frame as Ink>::fill_quad(self.frame, quad, color, of);
    }

    fn stroke_segments(&mut self, segments: &[[Point; 2]], color: Color, width: f32) {
        <Frame as Ink>::stroke_segments(self.frame, segments, color, width);
    }

    fn marks(
        &mut self,
        camera: &Camera,
        colors: &Colors,
        axes: kentos_interaction::Axes,
        ucs: Option<f64>,
        lift: f32,
        per_unit: f64,
    ) {
        <Frame as Ink>::marks(self.frame, camera, colors, axes, ucs, lift, per_unit);
    }

    fn fill_ring(&mut self, ring: &[Point], color: Color, of: Option<Slot>) {
        <Frame as Ink>::fill_ring(self.frame, ring, color, of);
    }

    fn stroke_line(&mut self, pts: &[Point], closed: bool, color: Color, of: Option<Slot>) {
        <Frame as Ink>::stroke_line(self.frame, pts, closed, color, of);
    }
}

/// A text shaped as the canvas shapes it: each glyph with its face among the drawing's and its pen position from the
/// text's start on its baseline (ems, y up); the width, and the baseline under the top of a line one size high (ems).
#[derive(Debug, PartialEq)]
pub(super) struct Run {
    pub(super) glyphs: Vec<(GlyphKey, [f32; 2])>,
    pub(super) width: f32,
    pub(super) baseline: f32,
}

/// Shaped texts by face and words; none for a text the GPU does not draw.
type Runs = HashMap<Font, HashMap<String, Option<Arc<Run>>>>;

/// The shaped texts, kept between paintings as the widths are (`MEASURES`).
static RUNS: Mutex<Option<Runs>> = Mutex::new(None);

/// `text` in `font` as glyphs of the drawing's faces; none when a glyph is from another face, or it is more than a line.
pub(super) fn run(text: &str, font: Font) -> Option<Arc<Run>> {
    let mut all = RUNS.lock().unwrap_or_else(PoisonError::into_inner);
    let all = all.get_or_insert_with(HashMap::new);
    if all.values().map(HashMap::len).sum::<usize>() > MEASURES_KEPT {
        all.clear();
    }
    let known = all.entry(font).or_default();
    if let Some(run) = known.get(text) {
        return run.clone();
    }
    let run = shape(text, font).map(Arc::new);
    known.insert(text.to_owned(), run.clone());
    run
}

fn shape(text: &str, font: Font) -> Option<Run> {
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
    let mut lines = p.buffer().layout_runs();
    let line = lines.next()?;
    if lines.next().is_some() {
        return None;
    }
    // A glyph's pen as the text system draws it (`LayoutGlyph::physical`): its x and the offset along, its y less the
    // offset up, from the line's baseline (y down).
    let pens: Vec<(fontdb::ID, u16, f32, f32)> = line
        .glyphs
        .iter()
        .map(|g| (g.font_id, g.glyph_id, g.x + g.font_size * g.x_offset, g.y - g.font_size * g.y_offset))
        .collect();
    let (width, baseline) = (line.line_w, line.line_y);
    drop(p);
    let mut glyphs = Vec::with_capacity(pens.len());
    for (id, glyph, x, y) in pens {
        glyphs.push((GlyphKey { font: drawing_face(id)?, glyph }, [x / REFERENCE, -y / REFERENCE]));
    }
    Some(Run { glyphs, width: width / REFERENCE, baseline: baseline / REFERENCE })
}

/// The drawing's face a face of the text system is (by its bytes: Iced keeps its own copy); none for any other.
fn drawing_face(id: fontdb::ID) -> Option<FontId> {
    static KNOWN: Mutex<Option<HashMap<fontdb::ID, Option<FontId>>>> = Mutex::new(None);
    let mut known = KNOWN.lock().unwrap_or_else(PoisonError::into_inner);
    let known = known.get_or_insert_with(HashMap::new);
    if let Some(face) = known.get(&id) {
        return *face;
    }
    let face = {
        let mut system = iced::advanced::graphics::text::font_system()
            .write()
            .unwrap_or_else(PoisonError::into_inner);
        system
            .raw()
            .db()
            .with_face_data(id, |data, index| {
                (index == 0)
                    .then(|| kentos_plot::fonts::FACES.iter().position(|f| f.len() == data.len() && *f == data))
                    .flatten()
            })
            .flatten()
            .and_then(|i| u16::try_from(i).ok())
            .map(FontId)
    };
    known.insert(id, face);
    face
}

/// The piece's glyphs' places from its anchor, in pixels with y up, as the canvas draws it (labels/turned.rs): its
/// glyphs from where its anchor puts the text, leaning (the lean over the width factor: the letters lean before they
/// are narrowed), narrowed by the width factor, turned by its angle.
pub(super) fn places(piece: &Piece<'_>, run: &Run) -> Vec<GlyphPlace> {
    let s = f64::from(piece.size);
    let narrow = f64::from(piece.width_factor);
    let lean = f64::from(piece.lean) / narrow;
    let along = match piece.anchor {
        Anchor::LeftBaseline => 0.0,
        Anchor::CenterBaseline | Anchor::CenterMiddle => -f64::from(run.width) * s / 2.0,
    };
    // The baseline from the anchor, down: a text centred halfway up has its line's top half its size over it.
    let down = match piece.anchor {
        Anchor::LeftBaseline | Anchor::CenterBaseline => 0.0,
        Anchor::CenterMiddle => -s / 2.0 + f64::from(run.baseline) * s,
    };
    let (sin, cos) = f64::from(piece.angle).sin_cos();
    // A vector of the text's own frame (y down) on the screen, y up.
    let out = |x: f64, y: f64| {
        let x = (x - lean * y) * narrow;
        [cos * x - sin * y, -(sin * x + cos * y)]
    };
    let (ex, ey) = (out(s, 0.0), out(0.0, -s));
    run.glyphs
        .iter()
        .map(|(key, [x, y])| {
            let p = out(along + s * f64::from(*x), down - s * f64::from(*y));
            GlyphPlace { key: *key, transform: [ex[0], ex[1], ey[0], ey[1], p[0], p[1]] }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn piece<'t>(text: &'t str, anchor: Anchor, angle: f32) -> Piece<'t> {
        Piece {
            text,
            at: Point::new(100.0, 50.0),
            angle,
            size: 20.0,
            font: drawing_fonts::font(DrawingFont::Barlow, 400, false),
            anchor,
            color: Color::BLACK,
            width_factor: 1.0,
            mask: 0.0,
            underline: 0.0,
            lean: 0.0,
        }
    }

    /// A text in a drawing's face is shaped into that face's glyphs, as wide as the canvas measures it.
    #[test]
    fn a_text_is_the_drawing_faces_glyphs_as_wide_as_measured() {
        drawing_fonts::load();
        let font = drawing_fonts::font(DrawingFont::Barlow, 400, false);
        let run = run("Kentos Ağ", font).expect("drawn on the GPU");
        let barlow = kentos_geometry_core::text::shaping::select_font("barlow", 400, false).expect("Barlow");
        assert_eq!(run.glyphs.len(), 9);
        assert!(run.glyphs.iter().all(|(k, _)| k.font == barlow), "{:?}", run.glyphs);
        assert!((run.width * REFERENCE - measure("Kentos Ağ", font).width).abs() < 1e-3);
        assert!(run.glyphs.windows(2).all(|w| w[1].1[0] > w[0].1[0]), "left to right");
        assert!(run.glyphs.iter().all(|(_, p)| p[1].abs() < 1e-6), "on the baseline");
        // A second time from the kept runs.
        assert!(Arc::ptr_eq(&run, &super::run("Kentos Ağ", font).expect("kept")));
    }

    /// A piece's glyphs where its anchor puts them: from its left end on the baseline, round its middle, and turned.
    #[test]
    fn a_piece_places_its_glyphs_from_its_anchor_as_the_canvas_does() {
        drawing_fonts::load();
        let left = piece("Ada 12", Anchor::LeftBaseline, 0.0);
        let run = run(left.text, left.font).expect("drawn on the GPU");
        let width = f64::from(run.width) * 20.0;
        let places = super::places(&left, &run);
        let first = places[0].transform;
        assert_eq!(first[..4], [20.0, 0.0, 0.0, 20.0], "an em of 20 px, upright, y up");
        assert!(first[4].abs() < 1e-6 && first[5].abs() < 1e-6, "{first:?}");
        let centred = super::places(&piece("Ada 12", Anchor::CenterBaseline, 0.0), &run);
        assert!((centred[0].transform[4] + width / 2.0).abs() < 1e-4, "half its width left");
        // Turned a quarter to the left on the screen (a world rotation of 90°: the angle is the screen's, clockwise):
        // its letters go up the screen, their tops to the left.
        let turned = super::places(&piece("Ada 12", Anchor::LeftBaseline, -std::f32::consts::FRAC_PI_2), &run);
        let [a, b, c, d, _, _] = turned[0].transform;
        // (The angle is a float32's: a quarter turn leaves a millionth of a pixel.)
        assert!(a.abs() < 1e-5 && (b - 20.0).abs() < 1e-5, "x along up: {a} {b}");
        assert!((c + 20.0).abs() < 1e-5 && d.abs() < 1e-5, "y up to the left: {c} {d}");
        let last = turned.last().expect("glyphs").transform;
        assert!(last[4].abs() < 1e-4 && last[5] > width * 0.7, "the last letter up the screen: {last:?}");
    }

    /// A piece with a mask or an underline, and every piece of a spot that may not send them, stays on the canvas.
    #[test]
    fn a_masked_underlined_or_other_spots_piece_stays_on_the_canvas() {
        drawing_fonts::load();
        let sink = Sink::new(GlyphSlot::default());
        let view = Camera::default();
        let plain = piece("Ada", Anchor::LeftBaseline, 0.0);
        assert!(!sink.take(&plain, Color::WHITE, &view), "not open before a text's spot");
        sink.open.set(true);
        assert!(sink.take(&plain, Color::WHITE, &view));
        assert!(!sink.take(&Piece { mask: 30.0, ..plain }, Color::WHITE, &view));
        assert!(!sink.take(&Piece { underline: 30.0, ..plain }, Color::WHITE, &view));
        sink.publish(1.0);
        let set = sink.slot.get();
        assert_eq!((set.version, set.texts.len()), (1, 1));
        assert_eq!(set.texts[0].halo_px, HALO);
        // A painting with nothing for the GPU after one with texts empties the set; another such does not count.
        sink.publish(1.0);
        sink.publish(1.0);
        assert_eq!((sink.slot.get().version, sink.slot.get().texts.len()), (2, 0));
    }
}
