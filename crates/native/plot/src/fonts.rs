//! The drawing's typefaces as files (docs/adr/0055): the web's faces as
//! TrueType (apps/desktop/assets/fonts/drawing; scripts/fonts/drawing_fonts.py
//! holds them to the web's), as a sheet PDF embeds them (docs/sheet/design.md
//! §9a) and the desktop loads them into its text system. The desktop's PDF
//! and the headless host's embed the same bytes.

use kentos_contracts::DrawingFont;

/// Every face, embedded (1.9 MB; OFL, the licences beside them).
pub const FACES: [&[u8]; 22] = [
    include_bytes!("../../../../apps/desktop/assets/fonts/drawing/ArchitectsDaughter-400.ttf"),
    include_bytes!("../../../../apps/desktop/assets/fonts/drawing/Arimo-400.ttf"),
    include_bytes!("../../../../apps/desktop/assets/fonts/drawing/Arimo-500.ttf"),
    include_bytes!("../../../../apps/desktop/assets/fonts/drawing/Arimo-600.ttf"),
    include_bytes!("../../../../apps/desktop/assets/fonts/drawing/Arimo-700.ttf"),
    include_bytes!("../../../../apps/desktop/assets/fonts/drawing/Barlow-400-italic.ttf"),
    include_bytes!("../../../../apps/desktop/assets/fonts/drawing/Barlow-400.ttf"),
    include_bytes!("../../../../apps/desktop/assets/fonts/drawing/Barlow-500.ttf"),
    include_bytes!("../../../../apps/desktop/assets/fonts/drawing/Barlow-600.ttf"),
    include_bytes!("../../../../apps/desktop/assets/fonts/drawing/CourierPrime-400-italic.ttf"),
    include_bytes!("../../../../apps/desktop/assets/fonts/drawing/CourierPrime-400.ttf"),
    include_bytes!("../../../../apps/desktop/assets/fonts/drawing/CourierPrime-700.ttf"),
    include_bytes!("../../../../apps/desktop/assets/fonts/drawing/IBMPlexMono-400.ttf"),
    include_bytes!("../../../../apps/desktop/assets/fonts/drawing/IBMPlexMono-500.ttf"),
    include_bytes!("../../../../apps/desktop/assets/fonts/drawing/Overpass-400.ttf"),
    include_bytes!("../../../../apps/desktop/assets/fonts/drawing/Overpass-500.ttf"),
    include_bytes!("../../../../apps/desktop/assets/fonts/drawing/Overpass-600.ttf"),
    include_bytes!("../../../../apps/desktop/assets/fonts/drawing/Overpass-700.ttf"),
    include_bytes!("../../../../apps/desktop/assets/fonts/drawing/Quicksand-400.ttf"),
    include_bytes!("../../../../apps/desktop/assets/fonts/drawing/Quicksand-500.ttf"),
    include_bytes!("../../../../apps/desktop/assets/fonts/drawing/Quicksand-600.ttf"),
    include_bytes!("../../../../apps/desktop/assets/fonts/drawing/Quicksand-700.ttf"),
];

/// Each of [`FACES`] as the sheet core names it (`TextPrim.font`, its weight, italic): what a
/// sheet PDF embeds (docs/sheet/design.md §9a).
pub const FACE_KEYS: [(&str, u16, bool); 22] = [
    ("architects-daughter", 400, false),
    ("arimo", 400, false),
    ("arimo", 500, false),
    ("arimo", 600, false),
    ("arimo", 700, false),
    ("barlow", 400, true),
    ("barlow", 400, false),
    ("barlow", 500, false),
    ("barlow", 600, false),
    ("courier-prime", 400, true),
    ("courier-prime", 400, false),
    ("courier-prime", 700, false),
    ("plex-mono", 400, false),
    ("plex-mono", 500, false),
    ("overpass", 400, false),
    ("overpass", 500, false),
    ("overpass", 600, false),
    ("overpass", 700, false),
    ("quicksand", 400, false),
    ("quicksand", 500, false),
    ("quicksand", 600, false),
    ("quicksand", 700, false),
];

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
