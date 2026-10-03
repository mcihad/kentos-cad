//! The sheet's typefaces (`DRAWING_FONTS` ids of the core's text table) as
//! Iced fonts. The faces themselves are the host's: the desktop embeds the
//! web's files as TrueType (apps/desktop/assets/fonts/drawing) and loads them
//! before the first frame; [`load_dir`] reads such a folder (the tests and the
//! screens use the desktop's). Where a host has not loaded a face, the text
//! system falls back to another font: the core measured with the face, so
//! the line may look a little longer or shorter than laid out.

use std::borrow::Cow;
use std::path::Path;

use iced::Font;
use iced::font::{Family, Style, Weight};

/// The family name of a `DRAWING_FONTS` id (the web's CSS family); none for an id the core does not know.
pub fn family(font: &str) -> Option<&'static str> {
    Some(match font {
        "barlow" => "Barlow",
        "arimo" => "Arimo",
        "overpass" => "Overpass",
        "quicksand" => "Quicksand",
        "architects-daughter" => "Architects Daughter",
        "courier-prime" => "Courier Prime",
        "plex-mono" => "IBM Plex Mono",
        _ => return None,
    })
}

/// The Iced font of a `DRAWING_FONTS` id at a weight (400 … 700) and style;
/// the text system takes the nearest weight a family has, as a browser does.
pub fn font(id: &str, weight: u16, italic: bool) -> Font {
    Font {
        family: family(id).map_or(Family::SansSerif, Family::Name),
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

/// Loads every `.ttf` and `.otf` of `dir` into Iced's text system; the
/// number loaded. Nothing is loaded twice by the text system itself.
pub fn load_dir(dir: &Path) -> usize {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    let mut files: Vec<_> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case("ttf") || e.eq_ignore_ascii_case("otf"))
        })
        .collect();
    files.sort();
    let Ok(mut system) = iced::advanced::graphics::text::font_system().write() else {
        return 0;
    };
    let mut loaded = 0;
    for file in files {
        if let Ok(bytes) = std::fs::read(&file) {
            system.load_font(Cow::Owned(bytes));
            loaded += 1;
        }
    }
    loaded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_typeface_of_the_core_has_a_family() {
        for id in kentos_sheet::text::fonts() {
            assert!(family(id).is_some(), "{id}");
        }
        assert_eq!(font("plex-mono", 500, false).weight, Weight::Medium);
        assert_eq!(font("bilinmeyen", 400, true).family, Family::SansSerif);
    }
}
