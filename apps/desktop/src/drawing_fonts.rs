//! The drawing's typefaces (docs/adr/0055): text objects, dimension values
//! and labels are drawn in the project's typeface (`DrawingFont`), whatever
//! the interface's, as on the web (`DRAWING_FONTS`, app/appearance.ts). The
//! faces are the web's own (apps/web/src/assets/fonts) as TrueType: their two
//! subsets merged, variable fonts instanced at the weights the drawing asks
//! for (scripts/fonts/drawing_fonts.py; `--check` holds them to the web's).

use std::borrow::Cow;
use std::sync::Once;

use iced::Font;
use iced::font::{Family, Style, Weight};
use kentos_contracts::DrawingFont;

/// Every face, embedded (1.9 MB; OFL, the licences beside them).
const FACES: [&[u8]; 22] = [
    include_bytes!("../assets/fonts/drawing/ArchitectsDaughter-400.ttf"),
    include_bytes!("../assets/fonts/drawing/Arimo-400.ttf"),
    include_bytes!("../assets/fonts/drawing/Arimo-500.ttf"),
    include_bytes!("../assets/fonts/drawing/Arimo-600.ttf"),
    include_bytes!("../assets/fonts/drawing/Arimo-700.ttf"),
    include_bytes!("../assets/fonts/drawing/Barlow-400-italic.ttf"),
    include_bytes!("../assets/fonts/drawing/Barlow-400.ttf"),
    include_bytes!("../assets/fonts/drawing/Barlow-500.ttf"),
    include_bytes!("../assets/fonts/drawing/Barlow-600.ttf"),
    include_bytes!("../assets/fonts/drawing/CourierPrime-400-italic.ttf"),
    include_bytes!("../assets/fonts/drawing/CourierPrime-400.ttf"),
    include_bytes!("../assets/fonts/drawing/CourierPrime-700.ttf"),
    include_bytes!("../assets/fonts/drawing/IBMPlexMono-400.ttf"),
    include_bytes!("../assets/fonts/drawing/IBMPlexMono-500.ttf"),
    include_bytes!("../assets/fonts/drawing/Overpass-400.ttf"),
    include_bytes!("../assets/fonts/drawing/Overpass-500.ttf"),
    include_bytes!("../assets/fonts/drawing/Overpass-600.ttf"),
    include_bytes!("../assets/fonts/drawing/Overpass-700.ttf"),
    include_bytes!("../assets/fonts/drawing/Quicksand-400.ttf"),
    include_bytes!("../assets/fonts/drawing/Quicksand-500.ttf"),
    include_bytes!("../assets/fonts/drawing/Quicksand-600.ttf"),
    include_bytes!("../assets/fonts/drawing/Quicksand-700.ttf"),
];

/// Puts the faces in Iced's text system before the first frame; later calls do nothing.
pub fn load() {
    static LOADED: Once = Once::new();
    LOADED.call_once(|| {
        let Ok(mut system) = iced::advanced::graphics::text::font_system().write() else {
            return;
        };
        for face in FACES {
            system.load_font(Cow::Borrowed(face));
        }
    });
}

/// A typeface's family name (the web's CSS family).
pub fn family(font: DrawingFont) -> &'static str {
    match font {
        DrawingFont::Barlow => "Barlow",
        DrawingFont::Arimo => "Arimo",
        DrawingFont::Overpass => "Overpass",
        DrawingFont::Quicksand => "Quicksand",
        DrawingFont::ArchitectsDaughter => "Architects Daughter",
        DrawingFont::CourierPrime => "Courier Prime",
        DrawingFont::PlexMono => "IBM Plex Mono",
    }
}

/// The face for a weight (400 … 700) and style; the text system takes the
/// nearest weight a family has, as the browser does.
pub fn font(drawing: DrawingFont, weight: u16, italic: bool) -> Font {
    Font {
        family: Family::Name(family(drawing)),
        weight: match weight {
            0..=449 => Weight::Normal,
            450..=549 => Weight::Medium,
            550..=649 => Weight::Semibold,
            _ => Weight::Bold,
        },
        style: if italic { Style::Italic } else { Style::Normal },
        ..Font::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every typeface is in the text system after `load`, with a face of
    /// each weight the web ships.
    #[test]
    fn every_drawing_typeface_is_there() {
        load();
        let mut system = iced::advanced::graphics::text::font_system()
            .write()
            .expect("the text system");
        let families: Vec<String> = system
            .raw()
            .db()
            .faces()
            .flat_map(|f| f.families.iter().map(|(name, _)| name.clone()))
            .collect();
        for drawing in [
            DrawingFont::Barlow,
            DrawingFont::Arimo,
            DrawingFont::Overpass,
            DrawingFont::Quicksand,
            DrawingFont::ArchitectsDaughter,
            DrawingFont::CourierPrime,
            DrawingFont::PlexMono,
        ] {
            let name = family(drawing);
            assert!(families.iter().any(|f| f == name), "{name} is not loaded");
        }
    }
}
