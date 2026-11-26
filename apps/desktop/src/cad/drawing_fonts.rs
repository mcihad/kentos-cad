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

use kentos_geometry_core::text::font_data::{ADDED_FACES, ADDED_KEYS, added_face, give, given};
use kentos_plot::fonts::{FACE_KEYS, FACES};

/// Puts the faces in Iced's text system before the first frame; later calls do nothing. The added faces (Arial, Times
/// New Roman: the operating system's own where [`give_system_faces`] found them, else their bundled twins) after the
/// web's, each once.
pub fn load() {
    static LOADED: Once = Once::new();
    LOADED.call_once(|| {
        let Ok(mut system) = iced::advanced::graphics::text::font_system().write() else {
            return;
        };
        let mut loaded: Vec<&'static [u8]> = Vec::new();
        // The twins always (a sheet names them), the system's own faces where they were given.
        let added = (0..ADDED_KEYS.len()).filter_map(added_face);
        for face in FACES.into_iter().chain(ADDED_FACES).chain(added) {
            if loaded.iter().any(|f| std::ptr::eq(*f, face)) {
                continue;
            }
            loaded.push(face);
            system.load_font(Cow::Borrowed(face));
        }
    });
}

/// Where an operating system keeps its own Arial and Times New Roman, by key (docs/adr/9311): KentOS uses them but
/// does not ship them (their licences do not allow it). The install's `fonts` folder and the user's (`kentos-cad/fonts`)
/// are read first, where a licensed copy may be put. A family without a file of its own (Arial 500) takes the nearest
/// weight a browser would.
const SYSTEM_FILES: [((&str, u16, bool), &[&str]); 8] = [
    (("arial", 400, false), &["Arial.ttf", "arial.ttf"]),
    (("arial", 500, false), &["Arial.ttf", "arial.ttf"]),
    (("arial", 600, false), &["Arial Bold.ttf", "arialbd.ttf", "Arial_Bold.ttf"]),
    (("arial", 700, false), &["Arial Bold.ttf", "arialbd.ttf", "Arial_Bold.ttf"]),
    (("times-new-roman", 400, false), &["Times New Roman.ttf", "times.ttf", "Times_New_Roman.ttf"]),
    (("times-new-roman", 700, false), &["Times New Roman Bold.ttf", "timesbd.ttf", "Times_New_Roman_Bold.ttf"]),
    (("times-new-roman", 400, true), &["Times New Roman Italic.ttf", "timesi.ttf", "Times_New_Roman_Italic.ttf"]),
    (("times-new-roman", 700, true), &["Times New Roman Bold Italic.ttf", "timesbi.ttf", "Times_New_Roman_Bold_Italic.ttf"]),
];

/// The folders searched for the files, in order: the install's and the user's `fonts`, then the system's.
fn font_folders() -> Vec<std::path::PathBuf> {
    let mut out: Vec<std::path::PathBuf> = Vec::new();
    out.extend(kentos_launch::install::fonts());
    out.extend(kentos_launch::install::user_fonts());
    let home = std::env::var_os("HOME").map(std::path::PathBuf::from);
    if cfg!(target_os = "macos") {
        out.push("/System/Library/Fonts/Supplemental".into());
        out.push("/Library/Fonts".into());
        out.extend(home.map(|h| h.join("Library/Fonts")));
    } else if cfg!(target_os = "windows") {
        let windir = std::env::var_os("WINDIR").map_or_else(|| "C:\\Windows".into(), std::path::PathBuf::from);
        out.push(windir.join("Fonts"));
        out.extend(std::env::var_os("LOCALAPPDATA").map(|l| std::path::PathBuf::from(l).join("Microsoft/Windows/Fonts")));
    } else {
        out.push("/usr/share/fonts/truetype/msttcorefonts".into());
        out.push("/usr/share/fonts/TTF".into());
        out.push("/usr/share/fonts/truetype".into());
        out.extend(home.map(|h| h.join(".local/share/fonts")));
    }
    out
}

/// A file that is the named family's face: it reads as a font whose family starts with the name.
fn read_face(path: &std::path::Path, family: &str) -> Option<&'static [u8]> {
    use iced::advanced::graphics::text::cosmic_text::skrifa::{self, MetadataProvider};
    let bytes = std::fs::read(path).ok()?;
    let named = skrifa::FontRef::new(&bytes).ok().is_some_and(|f| {
        f.localized_strings(skrifa::string::StringId::FAMILY_NAME)
            .any(|s| s.chars().collect::<String>().starts_with(family))
    });
    // Kept for the program's life: the shaper and the text system borrow the bytes.
    named.then(|| &*Box::leak(bytes.into_boxed_slice()))
}

/// Gives the geometry core the operating system's own Arial and Times New Roman (docs/adr/9311), before the first text
/// is laid out: the program's start calls it, the tests do not (they draw in the bundled twins, the same widths). A key
/// whose file is not found stays its twin's. The families found.
pub fn give_system_faces() -> Vec<&'static str> {
    let folders = font_folders();
    let mut faces: Vec<((&str, u16, bool), &'static [u8])> = Vec::new();
    let mut read: Vec<(std::path::PathBuf, &'static [u8])> = Vec::new();
    for (key, names) in SYSTEM_FILES {
        let family = if key.0 == "arial" { "Arial" } else { "Times New Roman" };
        let found = folders.iter().flat_map(|d| names.iter().map(move |n| d.join(n))).find_map(|path| {
            if let Some((_, bytes)) = read.iter().find(|(p, _)| *p == path) {
                return Some(*bytes);
            }
            let bytes = read_face(&path, family)?;
            read.push((path, bytes));
            Some(bytes)
        });
        if let Some(bytes) = found {
            faces.push((key, bytes));
        }
    }
    give(&faces);
    ["arial", "times-new-roman"].into_iter().filter(|f| given(f)).collect()
}

/// Whether the family has an italic face of its own; one without is leant
/// as a browser leans it (docs/adr/0183 §2: `SYNTHETIC_ITALIC`).
pub fn has_italic(font: DrawingFont) -> bool {
    let id = font.id();
    FACE_KEYS.iter().chain(ADDED_KEYS.iter()).any(|&(f, _, italic)| italic && f == id)
}

/// How far a browser leans the letters of a family without an italic face
/// (Skia's synthetic oblique: a quarter of the height, about 14°).
pub const SYNTHETIC_ITALIC: f32 = 0.25;

/// A typeface's family name (the web's CSS family); Arial's and Times New Roman's the operating system's face's when
/// it was given, else their twins' (docs/adr/9311).
pub fn family(font: DrawingFont) -> &'static str {
    match font {
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
