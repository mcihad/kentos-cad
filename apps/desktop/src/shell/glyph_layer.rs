//! The drawing area's glyph layer as the rest of the desktop reaches it (plan/render.md RM-14, docs/adr/9212 §4):
//! whether the area's texts go to its renderer, the slot a painting leaves its texts in for the renderer
//! (`kentos_render_wgpu::glyphs`), and a text shaped into glyphs of the faces the layer holds. The drawing's labels
//! (cad/labels) and the area's overlays (gpu_frame.rs) send their texts through these.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use iced::advanced::graphics::text::Paragraph;
use iced::advanced::graphics::text::cosmic_text::fontdb;
use iced::advanced::text::{self, Paragraph as _};
use iced::alignment::Vertical;
use iced::{Font, Pixels, Size};
use kentos_geometry_core::text::shaping::FontId;
use kentos_render_wgpu::glyphs::{GlyphKey, GlyphSet, GlyphText};

/// Text is measured and shaped at this size; widths and baselines grow with the size.
pub(crate) const REFERENCE: f32 = 100.0;
/// Measured and shaped texts kept at most (a dense view offers tens of thousands).
pub(crate) const MEASURES_KEPT: usize = 200_000;

/// Whether the drawing area's texts go to its renderer: `KENTOS_DIAG_TEXT=canvas` keeps them all on the canvas, for a
/// measurement or a comparison.
pub fn on() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var("KENTOS_DIAG_TEXT").as_deref() != Ok("canvas"))
}

/// The drawing area's texts for its renderer, as the last painting left them, and what that painting was of (a layer's
/// key, `put_keyed`; 0 for none).
#[derive(Clone, Debug, Default)]
pub struct GlyphSlot(Arc<Mutex<GlyphSet>>, Arc<std::sync::atomic::AtomicU64>);

impl GlyphSlot {
    /// The texts now (their list is shared, not copied).
    pub fn get(&self) -> GlyphSet {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Texts placed at `placed_px` (the view's map pixels a metre; 0 for the screen's), a new version. Versions count
    /// over every slot: the area's renderer keeps a layer by its place in the list and its version, and a list whose
    /// slots shift (a data layer's labels shown) must not find another slot's set of the same count there.
    pub fn put(&self, texts: Vec<GlyphText>, placed_px: f64) {
        self.put_keyed(0, texts, placed_px);
    }

    /// Texts painted for `key` (`holds` tells it again).
    pub fn put_keyed(&self, key: u64, texts: Vec<GlyphText>, placed_px: f64) {
        static VERSIONS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let mut set = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        self.1.store(key, std::sync::atomic::Ordering::Relaxed);
        if texts.is_empty() && set.texts.is_empty() {
            return;
        }
        set.version = VERSIONS.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
        set.texts = Arc::new(texts);
        set.placed_px = placed_px;
    }
}

impl GlyphSlot {
    /// No texts: its layer is not drawn (and is painted again when shown, whatever its key).
    pub fn clear(&self) {
        self.put(Vec::new(), 0.0);
    }

    /// Whether it holds what was painted for `key`: a layer shown again for the same picture is not painted again.
    pub fn holds(&self, key: u64) -> bool {
        key != 0 && self.1.load(std::sync::atomic::Ordering::Relaxed) == key
    }
}

/// A text shaped as the canvas shapes it: each glyph with its face among the drawing's and its pen position from the
/// text's start on its baseline (ems, y up); the width, and the baseline under the top of a line one size high (ems).
#[derive(Debug, PartialEq)]
pub struct Run {
    pub glyphs: Vec<(GlyphKey, [f32; 2])>,
    pub width: f32,
    pub baseline: f32,
}

/// Shaped texts by face and words; none for a text the GPU does not draw.
type Runs = HashMap<Font, HashMap<String, Option<Arc<Run>>>>;

/// The shaped texts, kept between paintings as the widths are (`MEASURES`).
static RUNS: Mutex<Option<Runs>> = Mutex::new(None);

/// `text` in `font` shaped as the canvas shapes it, its glyphs the drawing's faces' (an overlay's text, RM-14).
pub fn text_run(text: &str, font: Font) -> Option<Arc<Run>> {
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
        .map(|g| {
            (
                g.font_id,
                g.glyph_id,
                g.x + g.font_size * g.x_offset,
                g.y - g.font_size * g.y_offset,
            )
        })
        .collect();
    let (width, baseline) = (line.line_w, line.line_y);
    drop(p);
    let mut glyphs = Vec::with_capacity(pens.len());
    for (id, glyph, x, y) in pens {
        glyphs.push((
            GlyphKey {
                font: drawing_face(id)?,
                glyph,
            },
            [x / REFERENCE, -y / REFERENCE],
        ));
    }
    Some(Run {
        glyphs,
        width: width / REFERENCE,
        baseline: baseline / REFERENCE,
    })
}

/// The glyph layer's face a face of the text system is (by its bytes: Iced keeps its own copy): a drawing face, or an
/// interface face given to the layer (the screen's marks are lettered in one, docs/adr/9214); none for any other (the
/// system's own, not bundled).
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
                let same = |f: &&[u8]| f.len() == data.len() && *f == data;
                if index != 0 {
                    return None;
                }
                // Any of the core's faces: the drawing's, the fallbacks, the added ones (docs/adr/9311).
                match kentos_geometry_core::text::shaping::font_of_bytes(data)
                    .filter(|id| !kentos_geometry_core::text::shaping::is_fallback(*id))
                {
                    Some(id) => Some(id),
                    None => kentos_ui::theme::typography::FONTS
                        .iter()
                        .find(|f| same(f))
                        .map(|f| kentos_render_wgpu::glyphs::extra_face(f)),
                }
            })
            .flatten()
    };
    known.insert(id, face);
    face
}
