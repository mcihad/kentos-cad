//! The drawing's typefaces as files (docs/adr/0055): the web's faces as
//! TrueType (paket/fonts/cizim; scripts/fonts/drawing_fonts.py
//! holds them to the web's), as a sheet PDF embeds them (docs/sheet/design.md
//! §9a) and the desktop loads them into its text system. The desktop's PDF
//! and the headless host's embed the same bytes.

use kentos_contracts::DrawingFont;

// Shared byte-for-byte with the Unicode geometry layout. Existing callers retain
// the 22 drawing faces; Unicode fallback faces are requested by the new layout explicitly.
pub use kentos_geometry_core::text::font_data::{DRAWING_FACES as FACES, DRAWING_KEYS as FACE_KEYS};

use kentos_geometry_core::text::font_data::{ADDED_FACES, ADDED_KEYS, added_face, installed};

/// The fonts folders' faces with their keys (docs/adr/9313): the family's name, weight, slope.
fn folder_faces() -> impl Iterator<Item = (&'static [u8], (&'static str, u16, bool))> {
    installed().iter().map(|f| (f.bytes, (f.family, f.weight, f.italic)))
}

/// The faces for a sheet PDF: their TrueType files, as the core names them; Arial's and Times New Roman's the
/// operating system's own where the host gave them (docs/adr/9311: a PDF embeds the letters it uses, as their
/// licences allow), else their bundled twins; then the fonts folders' families (docs/adr/9313; a face whose licence
/// forbids embedding is not read).
pub fn pdf_fonts() -> Vec<kentos_sheet::pdf::PdfFont> {
    let added = ADDED_KEYS
        .iter()
        .enumerate()
        .filter_map(|(i, key)| added_face(i).map(|data| (data, *key)));
    FACES
        .iter()
        .copied()
        .zip(FACE_KEYS)
        .chain(added)
        .chain(folder_faces())
        .map(
            |(data, (font, weight, italic))| kentos_sheet::pdf::PdfFont {
                font: font.to_owned(),
                weight,
                italic,
                data: data.to_vec(),
            },
        )
        .collect()
}

/// Every bundled face with its key, the added twins after the web's (docs/adr/9311), then the fonts folders' families
/// (docs/adr/9313): what a file KentOS writes whole (an SVG's embedded faces, a raster's) carries, never a face of the
/// operating system.
pub fn bundled_faces() -> impl Iterator<Item = (&'static [u8], (&'static str, u16, bool))> {
    FACES.into_iter().zip(FACE_KEYS).chain(ADDED_FACES.into_iter().zip(ADDED_KEYS)).chain(folder_faces())
}

/// The sheet core's id of a drawing typeface: an own face's id, a folder family's name, Arial for a family this
/// program's folders do not hold (docs/adr/9313).
pub fn sheet_font(font: DrawingFont) -> &'static str {
    match font.family() {
        None => font.id(),
        Some(_) => kentos_geometry_core::text::Font::from_id(font.id()).drawn().id(),
    }
}
