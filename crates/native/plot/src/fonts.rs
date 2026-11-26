//! The drawing's typefaces as files (docs/adr/0055): the web's faces as
//! TrueType (apps/desktop/assets/fonts/drawing; scripts/fonts/drawing_fonts.py
//! holds them to the web's), as a sheet PDF embeds them (docs/sheet/design.md
//! §9a) and the desktop loads them into its text system. The desktop's PDF
//! and the headless host's embed the same bytes.

use kentos_contracts::DrawingFont;

// Shared byte-for-byte with the Unicode geometry layout. Existing callers retain
// the 22 drawing faces; Unicode fallback faces are requested by the new layout explicitly.
pub use kentos_geometry_core::text::font_data::{DRAWING_FACES as FACES, DRAWING_KEYS as FACE_KEYS};

use kentos_geometry_core::text::font_data::{ADDED_FACES, ADDED_KEYS, added_face};

/// The faces for a sheet PDF: their TrueType files, as the core names them; Arial's and Times New Roman's the
/// operating system's own where the host gave them (docs/adr/9311: a PDF embeds the letters it uses, as their
/// licences allow), else their bundled twins.
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

/// Every bundled face with its key, the added twins after the web's (docs/adr/9311): what a file KentOS writes whole
/// (an SVG's embedded faces, a raster's) carries, never a face of the operating system.
pub fn bundled_faces() -> impl Iterator<Item = (&'static [u8], (&'static str, u16, bool))> {
    FACES.into_iter().zip(FACE_KEYS).chain(ADDED_FACES.into_iter().zip(ADDED_KEYS))
}

/// The sheet core's id of a drawing typeface.
pub fn sheet_font(font: DrawingFont) -> &'static str {
    match font {
        DrawingFont::Barlow => "barlow",
        DrawingFont::Arimo => "arimo",
        DrawingFont::Overpass => "overpass",
        DrawingFont::Quicksand => "quicksand",
        DrawingFont::ArchitectsDaughter => "architects-daughter",
        DrawingFont::CourierPrime => "courier-prime",
        DrawingFont::PlexMono => "plex-mono",
        DrawingFont::Arial => "arial",
        DrawingFont::TimesNewRoman => "times-new-roman",
    }
}
