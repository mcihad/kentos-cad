//! The drawing's typefaces as files (docs/adr/0055): the web's faces as
//! TrueType (apps/desktop/assets/fonts/drawing; scripts/fonts/drawing_fonts.py
//! holds them to the web's), as a sheet PDF embeds them (docs/sheet/design.md
//! §9a) and the desktop loads them into its text system. The desktop's PDF
//! and the headless host's embed the same bytes.

use kentos_contracts::DrawingFont;

// Shared byte-for-byte with the Unicode geometry layout. Existing callers retain
// the 22 drawing faces; Unicode fallback faces are requested by the new layout explicitly.
pub use kentos_geometry_core::text::font_data::{DRAWING_FACES as FACES, DRAWING_KEYS as FACE_KEYS};

/// The faces for a sheet PDF: their TrueType files, as the core names them.
pub fn pdf_fonts() -> Vec<kentos_sheet::pdf::PdfFont> {
    FACES
        .iter()
        .zip(FACE_KEYS)
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
    }
}
