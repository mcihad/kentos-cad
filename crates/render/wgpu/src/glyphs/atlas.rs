//! The glyph atlas (docs/adr/9212 §3): each glyph of the bundled faces made into its signed distance field once
//! ([`super::sdf`]) and laid on pages of single bytes, shelf after shelf. A field is never made again at another size
//! and a page never starts over: a full page opens the next. The pages are kept on the CPU too, so a texture grown for
//! a new page is filled again from them.
//!
//! A frame may stop making fields at a deadline: the glyphs it did not reach wait for the next build (`pending`), so
//! a drawing opened with many faces does not stall one frame.

use std::collections::HashMap;
use std::time::Instant;

use kentos_geometry_core::text::shaping::{FontId, font_bytes};
use ttf_parser::{Face, GlyphId};

use super::sdf::{self, EM_PX, GlyphField};

/// A page's side in pixels.
pub const PAGE: u32 = 2048;
/// Pixels left clear between fields: the sampler reads half a pixel past a field's box.
const GAP: u32 = 1;

/// The first id of the faces given besides the bundled ones ([`extra_face`]).
pub const EXTRA_FACES: u16 = 0x4000;

/// The faces given besides the bundled ones, by their bytes.
static EXTRA: std::sync::Mutex<Vec<&'static [u8]>> = std::sync::Mutex::new(Vec::new());

/// The id of a face the geometry core does not bundle (an interface's: the drawing area's marks are lettered in it,
/// docs/adr/9214), given by its bytes; the same bytes, the same id.
pub fn extra_face(bytes: &'static [u8]) -> FontId {
    let mut all = EXTRA.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let i = match all.iter().position(|b| std::ptr::eq(*b, bytes)) {
        Some(i) => i,
        None => {
            all.push(bytes);
            all.len() - 1
        }
    };
    FontId(EXTRA_FACES + u16::try_from(i).unwrap_or(u16::MAX - EXTRA_FACES))
}

/// A face's bytes: a bundled one's or one given.
fn face_bytes(font: FontId) -> Option<&'static [u8]> {
    match font.0.checked_sub(EXTRA_FACES) {
        Some(i) => EXTRA.lock().unwrap_or_else(std::sync::PoisonError::into_inner).get(usize::from(i)).copied(),
        None => font_bytes(font),
    }
}

/// A glyph of a bundled face (or one given, [`extra_face`]): the geometry core's font index and the glyph's id in it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GlyphKey {
    pub font: FontId,
    pub glyph: u16,
}

/// Where a glyph's field lies.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Slot {
    pub page: u32,
    /// x, y, width, height in 0..1 of the page, y down.
    pub uv: [f32; 4],
    /// The field's box from the glyph's origin in ems, y up: left, bottom, width, height.
    pub em: [f32; 4],
    /// Where its outline's program starts among the atlas's programs, plus one (`vector`); 0 without one.
    pub program: u32,
}

/// A glyph looked up.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Lookup {
    Ready(Slot),
    /// Nothing to draw: a space, a glyph the face does not have, or a field larger than a page.
    Blank,
    /// Its field was not made before the deadline.
    Later,
}

/// Bytes to write into a page of the texture at `x`, `y`.
#[derive(Clone, Debug, PartialEq)]
pub struct PageUpload {
    pub page: u32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

pub struct GlyphAtlas {
    side: u32,
    faces: HashMap<FontId, Option<Face<'static>>>,
    slots: HashMap<GlyphKey, Option<Slot>>,
    /// Each page's bytes, rows from the top.
    pages: Vec<Vec<u8>>,
    /// The last page's shelf: x, y and height.
    shelf: (u32, u32, u32),
    uploads: Vec<PageUpload>,
    /// A glyph was left for later since the last [`GlyphAtlas::take_pending`].
    pending: bool,
    /// The glyphs' outlines as programs, one after another (`vector`).
    programs: Vec<f32>,
}

impl Default for GlyphAtlas {
    fn default() -> Self {
        GlyphAtlas::new(PAGE)
    }
}

impl GlyphAtlas {
    /// An empty atlas of pages `side` pixels square ([`PAGE`]; tests take smaller ones to fill several).
    pub fn new(side: u32) -> GlyphAtlas {
        GlyphAtlas {
            side,
            faces: HashMap::new(),
            slots: HashMap::new(),
            pages: Vec::new(),
            shelf: (0, 0, 0),
            uploads: Vec::new(),
            pending: false,
            programs: Vec::new(),
        }
    }

    pub fn page_side(&self) -> u32 {
        self.side
    }

    pub fn page_count(&self) -> u32 {
        self.pages.len() as u32
    }

    /// The pages' bytes, rows from the top.
    pub fn pages(&self) -> &[Vec<u8>] {
        &self.pages
    }

    /// Glyphs whose fields are made, drawn or blank.
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// Where `key`'s field is; made now unless `deadline` has passed.
    pub fn lookup(&mut self, key: GlyphKey, deadline: Option<Instant>) -> Lookup {
        if let Some(slot) = self.slots.get(&key) {
            return slot.map_or(Lookup::Blank, Lookup::Ready);
        }
        if deadline.is_some_and(|d| Instant::now() >= d) {
            self.pending = true;
            return Lookup::Later;
        }
        let (field, program) = match self.face(key.font) {
            Some(face) => (sdf::field(face, GlyphId(key.glyph)), super::vector::program(face, GlyphId(key.glyph))),
            None => (None, None),
        };
        // The glyph's outline beside its field, for when it is shown past the field's size (docs/adr/9240).
        let slot = field.and_then(|f| self.place(f)).map(|mut slot| {
            if let Some(p) = program {
                slot.program = self.programs.len() as u32 + 1;
                self.programs.extend(p);
            }
            slot
        });
        self.slots.insert(key, slot);
        slot.map_or(Lookup::Blank, Lookup::Ready)
    }

    /// The glyphs' outlines' programs, one after another (a slot's `program` less one is where its starts).
    pub fn programs(&self) -> &[f32] {
        &self.programs
    }

    /// Whether a glyph was left for later since the last call.
    pub fn take_pending(&mut self) -> bool {
        std::mem::take(&mut self.pending)
    }

    /// The fields laid since the last call, to be written into the texture.
    pub fn take_uploads(&mut self) -> Vec<PageUpload> {
        std::mem::take(&mut self.uploads)
    }

    fn face(&mut self, font: FontId) -> Option<&Face<'static>> {
        self.faces
            .entry(font)
            .or_insert_with(|| face_bytes(font).and_then(|bytes| Face::parse(bytes, 0).ok()))
            .as_ref()
    }

    /// Lays a field on the last page's shelf, a new shelf or a new page.
    fn place(&mut self, f: GlyphField) -> Option<Slot> {
        let (w, h) = (f.width + GAP, f.height + GAP);
        if w > self.side || h > self.side {
            return None;
        }
        let (mut x, mut y, mut shelf) = self.shelf;
        if x + w > self.side {
            (x, y, shelf) = (0, y + shelf, 0);
        }
        if self.pages.is_empty() || y + h > self.side {
            self.pages.push(vec![0; (self.side * self.side) as usize]);
            (x, y, shelf) = (0, 0, 0);
        }
        self.shelf = (x + w, y, shelf.max(h));
        let page = self.pages.len() as u32 - 1;
        let bytes = &mut self.pages[page as usize];
        for row in 0..f.height {
            let at = ((y + row) * self.side + x) as usize;
            let from = (row * f.width) as usize;
            bytes[at..at + f.width as usize].copy_from_slice(&f.data[from..from + f.width as usize]);
        }
        let side = self.side as f32;
        let slot = Slot {
            page,
            uv: [x as f32 / side, y as f32 / side, f.width as f32 / side, f.height as f32 / side],
            em: [f.left / EM_PX, f.bottom / EM_PX, f.width as f32 / EM_PX, f.height as f32 / EM_PX],
            program: 0,
        };
        self.uploads.push(PageUpload { page, x, y, width: f.width, height: f.height, data: f.data });
        Some(slot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// The drawing's default face (Barlow 400) and a glyph of it.
    fn key(c: char) -> GlyphKey {
        let font = kentos_geometry_core::text::shaping::select_font("barlow", 400, false).expect("Barlow");
        let face = Face::parse(font_bytes(font).expect("bytes"), 0).expect("a face");
        GlyphKey { font, glyph: face.glyph_index(c).expect("in the face").0 }
    }

    #[test]
    fn a_glyph_is_made_once_and_a_space_is_blank() {
        let mut atlas = GlyphAtlas::default();
        let Lookup::Ready(a) = atlas.lookup(key('A'), None) else { panic!("A has a field") };
        assert_eq!(atlas.take_uploads().len(), 1);
        assert_eq!(atlas.lookup(key('A'), None), Lookup::Ready(a));
        assert!(atlas.take_uploads().is_empty(), "made once");
        assert_eq!(atlas.lookup(key(' '), None), Lookup::Blank);
        // The field's box in ems: an A stands on the baseline about 0.7 em high, the spread round it.
        assert!(a.em[1] < -0.1 && a.em[1] > -0.2, "{:?}", a.em);
        assert!(a.em[3] > 0.8 && a.em[3] < 1.1, "{:?}", a.em);
        // Its bytes are on the page where its rectangle says.
        let (x, y) = ((a.uv[0] * PAGE as f32) as usize, (a.uv[1] * PAGE as f32) as usize);
        let (w, h) = ((a.uv[2] * PAGE as f32) as usize, (a.uv[3] * PAGE as f32) as usize);
        let page = &atlas.pages()[0];
        let inked = (y..y + h).flat_map(|r| (x..x + w).map(move |c| (r, c))).filter(|&(r, c)| page[r * PAGE as usize + c] > 128).count();
        assert!(inked > 100, "{inked} pixels inside the A");
    }

    #[test]
    fn a_full_page_opens_the_next_and_none_starts_over() {
        let mut atlas = GlyphAtlas::new(160);
        let letters: Vec<GlyphKey> = ('A'..='Z').map(key).collect();
        let slots: Vec<Slot> = letters
            .iter()
            .map(|&k| match atlas.lookup(k, None) {
                Lookup::Ready(s) => s,
                other => panic!("{other:?}"),
            })
            .collect();
        assert!(atlas.page_count() >= 3, "{} pages", atlas.page_count());
        assert_eq!(slots.last().map(|s| s.page), Some(atlas.page_count() - 1));
        // Every slot where it was put: the first letters are still there after the last.
        for (k, s) in letters.iter().zip(&slots) {
            assert_eq!(atlas.lookup(*k, None), Lookup::Ready(*s));
            assert!(s.uv[0] + s.uv[2] <= 1.0 && s.uv[1] + s.uv[3] <= 1.0, "{s:?}");
        }
        // No two on a page overlap.
        for (i, a) in slots.iter().enumerate() {
            for b in &slots[i + 1..] {
                let apart = a.page != b.page
                    || a.uv[0] + a.uv[2] <= b.uv[0]
                    || b.uv[0] + b.uv[2] <= a.uv[0]
                    || a.uv[1] + a.uv[3] <= b.uv[1]
                    || b.uv[1] + b.uv[3] <= a.uv[1];
                assert!(apart, "{a:?} {b:?}");
            }
        }
    }

    #[test]
    fn past_the_deadline_a_glyph_waits_and_says_so() {
        let mut atlas = GlyphAtlas::default();
        let past = Instant::now() - Duration::from_millis(1);
        assert_eq!(atlas.lookup(key('B'), Some(past)), Lookup::Later);
        assert!(atlas.take_pending());
        assert!(!atlas.take_pending(), "said once");
        assert!(matches!(atlas.lookup(key('B'), None), Lookup::Ready(_)));
        // A field already made is there past any deadline.
        assert!(matches!(atlas.lookup(key('B'), Some(past)), Lookup::Ready(_)));
        assert!(!atlas.take_pending());
    }
}
