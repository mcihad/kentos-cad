//! What a font file holds (docs/adr/9313): each face's family, style, weight and slope as its name table says, read
//! from the bytes alone. The fonts folders' listing and the host's install read files with it.

use rustybuzz::ttf_parser::{self, Face, name_id};

/// The letters of the Turkish alphabet a face must have to write Turkish without a fallback face.
pub const TURKISH: &str = "ÇçĞğİıÖöŞşÜü";

/// One face of a font file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FaceInfo {
    /// The family a person picks (the typographic family, else the family): `Roboto`.
    pub family: String,
    /// Its style within the family (`Bold Italic`); empty when the file names none.
    pub style: String,
    /// Its weight, 100–900 (400 regular, 700 bold).
    pub weight: u16,
    pub italic: bool,
    /// Its place in a collection (`.ttc`); 0 for a file of one face.
    pub index: u32,
    /// Whether it has every letter of [`TURKISH`].
    pub turkish: bool,
    /// Whether it is a variable font (one face drawn at its default weight).
    pub variable: bool,
    /// Whether its licence lets a document embed it (its OS/2 table is not marked restricted).
    pub embeddable: bool,
}

/// Every face the bytes hold, in their order; none for bytes that are not a font, or a face without a family name.
pub fn describe(bytes: &[u8]) -> Vec<FaceInfo> {
    let count = ttf_parser::fonts_in_collection(bytes).unwrap_or(1);
    (0..count).filter_map(|i| face(bytes, i)).collect()
}

/// Whether the bytes are a collection of faces (`.ttc`, `.otc`).
pub fn is_collection(bytes: &[u8]) -> bool {
    ttf_parser::fonts_in_collection(bytes).is_some()
}

fn face(bytes: &[u8], index: u32) -> Option<FaceInfo> {
    let face = Face::parse(bytes, index).ok()?;
    let family = name(&face, name_id::TYPOGRAPHIC_FAMILY).or_else(|| name(&face, name_id::FAMILY))?;
    let style = name(&face, name_id::TYPOGRAPHIC_SUBFAMILY)
        .or_else(|| name(&face, name_id::SUBFAMILY))
        .unwrap_or_default();
    Some(FaceInfo {
        family,
        style,
        weight: face.weight().to_number(),
        italic: face.is_italic() || face.is_oblique(),
        index,
        turkish: TURKISH.chars().all(|c| face.glyph_index(c).is_some()),
        variable: face.is_variable(),
        embeddable: face.permissions() != Some(ttf_parser::Permissions::Restricted),
    })
}

/// The face's name `id` in Unicode, American English first, trimmed; none when it has none.
fn name(face: &Face<'_>, id: u16) -> Option<String> {
    let names: Vec<_> = face.names().into_iter().filter(|n| n.name_id == id && n.is_unicode()).collect();
    let english = names.iter().find(|n| n.language() == ttf_parser::Language::English_UnitedStates);
    english
        .into_iter()
        .chain(names.iter())
        .find_map(|n| n.to_string())
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::font_data::{ADDED_FACES, DRAWING_FACES, FALLBACK_FACES};

    #[test]
    fn the_bundled_faces_say_their_families() {
        let barlow = describe(DRAWING_FACES[6]);
        assert_eq!(barlow.len(), 1);
        assert_eq!(barlow[0].family, "Barlow");
        assert_eq!((barlow[0].weight, barlow[0].italic, barlow[0].turkish), (400, false, true));
        let italic = describe(DRAWING_FACES[5]);
        assert!(italic[0].italic);
        let serif = describe(ADDED_FACES[5]);
        assert_eq!((serif[0].family.as_str(), serif[0].weight), ("Liberation Serif", 700));
        let math = describe(FALLBACK_FACES[6]);
        assert_eq!(math[0].family, "Noto Sans Math");
        assert!(!is_collection(DRAWING_FACES[6]));
    }

    #[test]
    fn bytes_that_are_not_a_font_hold_no_face() {
        assert!(describe(b"not a font").is_empty());
        assert!(describe(&[]).is_empty());
    }
}
