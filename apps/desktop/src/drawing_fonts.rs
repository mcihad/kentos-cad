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

// The faces and their names are the plotter's: the headless host's PDF embeds the same bytes (docs/adr/0252).
use kentos_plot::fonts::{FACE_KEYS, FACES};
pub use kentos_plot::fonts::pdf_fonts;

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

/// Whether the family has an italic face of its own; one without is leant
/// as a browser leans it (docs/adr/0183 §2: `SYNTHETIC_ITALIC`).
pub fn has_italic(font: DrawingFont) -> bool {
    let id = font.id();
    FACE_KEYS.iter().any(|&(f, _, italic)| italic && f == id)
}

/// How far a browser leans the letters of a family without an italic face
/// (Skia's synthetic oblique: a quarter of the height, about 14°).
pub const SYNTHETIC_ITALIC: f32 = 0.25;

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

    /// Each face as a sheet PDF names it (`FACE_KEYS`): its own file's family, weight and slant.
    #[test]
    fn every_face_is_named_as_the_sheet_core_names_it() {
        use iced::advanced::graphics::text::cosmic_text::skrifa::{self, MetadataProvider};
        let drawings = [
            DrawingFont::Barlow,
            DrawingFont::Arimo,
            DrawingFont::Overpass,
            DrawingFont::Quicksand,
            DrawingFont::ArchitectsDaughter,
            DrawingFont::CourierPrime,
            DrawingFont::PlexMono,
        ];
        for (data, (font, weight, italic)) in FACES.iter().zip(FACE_KEYS) {
            let f = skrifa::FontRef::new(data).expect("a face");
            let a = f.attributes();
            assert_eq!(
                (
                    a.weight.value().round() as u16,
                    a.style != skrifa::attribute::Style::Normal
                ),
                (weight, italic),
                "{font} {weight}"
            );
            let drawing = drawings
                .iter()
                .find(|d| kentos_plot::fonts::sheet_font(**d) == font)
                .expect("a drawing typeface");
            let names: Vec<String> = f
                .localized_strings(skrifa::string::StringId::FAMILY_NAME)
                .map(|s| s.chars().collect())
                .collect();
            assert!(
                names.iter().any(|n| n.starts_with(family(*drawing))),
                "{font}: {names:?}"
            );
        }
    }

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
