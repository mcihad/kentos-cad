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
use kentos_commands::DrawingFont;

// The faces and their names are the plotter's: the headless host's PDF embeds the same bytes (docs/adr/0252).

use kentos_geometry_core::text::font_data::{ADDED_FACES, ADDED_KEYS, added_face, given, installed};
use kentos_plot::fonts::{FACE_KEYS, FACES};

/// Puts the faces in Iced's text system before the first frame; later calls do nothing. The added faces (Arial, Times
/// New Roman: the operating system's own where [`load_font_folders`] found them, else their bundled twins) after the
/// web's, then the fonts folders' families (docs/adr/9313), each once.
pub fn load() {
    static LOADED: Once = Once::new();
    LOADED.call_once(|| {
        let Ok(mut system) = iced::advanced::graphics::text::font_system().write() else {
            return;
        };
        let mut loaded: Vec<&'static [u8]> = Vec::new();
        // The twins always (a sheet names them), the system's own faces where they were given.
        let added = (0..ADDED_KEYS.len()).filter_map(added_face);
        let folders = installed().iter().map(|f| f.bytes);
        for face in FACES.into_iter().chain(ADDED_FACES).chain(added).chain(folders) {
            if loaded.iter().any(|f| std::ptr::eq(*f, face)) {
                continue;
            }
            loaded.push(face);
            system.load_font(Cow::Borrowed(face));
        }
    });
}

/// Reads the fonts folders and gives the geometry core their faces (docs/adr/9313), and the operating system's own
/// Arial and Times New Roman where the folders hold none (docs/adr/9311), before the first text is laid out: the
/// program's start calls it, the tests do not (they draw in the bundled faces, the twins' widths the same). The
/// families of Arial and Times New Roman given in place of their twins.
pub fn load_font_folders() -> Vec<&'static str> {
    kentos_native_application::font_folders::load();
    ["arial", "times-new-roman"].into_iter().filter(|f| given(f)).collect()
}

/// Whether the family has an italic face of its own; one without is leant
/// as a browser leans it (docs/adr/0183 §2: `SYNTHETIC_ITALIC`).
pub fn has_italic(font: DrawingFont) -> bool {
    let id = kentos_geometry_core::text::Font::from_id(font.id()).drawn().id();
    FACE_KEYS.iter().chain(ADDED_KEYS.iter()).any(|&(f, _, italic)| italic && f == id)
        || installed().iter().any(|f| f.italic && f.family == id)
}

/// How far a browser leans the letters of a family without an italic face
/// (Skia's synthetic oblique: a quarter of the height, about 14°).
pub const SYNTHETIC_ITALIC: f32 = 0.25;

/// A typeface's family name (the web's CSS family); Arial's and Times New Roman's the operating system's face's when
/// it was given, else their twins' (docs/adr/9311); a fonts folder's family's own, Arial's for one this program's
/// folders do not hold (docs/adr/9313).
pub fn family(font: DrawingFont) -> &'static str {
    match font {
        DrawingFont::Installed(f) if kentos_geometry_core::text::Font::from_id(f.name()).missing() => {
            family(DrawingFont::Arial)
        }
        DrawingFont::Installed(f) => f.name(),
        DrawingFont::Arial if given("arial") => "Arial",
        DrawingFont::Arial => "Arimo",
        DrawingFont::TimesNewRoman if given("times-new-roman") => "Times New Roman",
        DrawingFont::TimesNewRoman => "Liberation Serif",
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
        // Every typeface, Arial and Times New Roman too: a test gives no system face, so their twins draw them
        // (docs/adr/9311), and Times New Roman has its italic.
        for drawing in DrawingFont::ALL {
            let name = family(drawing);
            assert!(families.iter().any(|f| f == name), "{name} is not loaded");
        }
        assert_eq!(family(DrawingFont::Arial), "Arimo");
        assert_eq!(family(DrawingFont::TimesNewRoman), "Liberation Serif");
        assert!(has_italic(DrawingFont::TimesNewRoman) && !has_italic(DrawingFont::Arial));
    }
}
