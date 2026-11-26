//! The exact bundled faces shared by drawing layout, desktop and PDF (ADR 0914).
//! Parsing them needs no filesystem, OS font list or network access.

/// Every face, embedded (1.9 MB; OFL, the licences beside them).
pub const DRAWING_FACES: [&[u8]; 22] = [
    include_bytes!("../../../../../apps/desktop/assets/fonts/drawing/ArchitectsDaughter-400.ttf"),
    include_bytes!("../../../../../apps/desktop/assets/fonts/drawing/Arimo-400.ttf"),
    include_bytes!("../../../../../apps/desktop/assets/fonts/drawing/Arimo-500.ttf"),
    include_bytes!("../../../../../apps/desktop/assets/fonts/drawing/Arimo-600.ttf"),
    include_bytes!("../../../../../apps/desktop/assets/fonts/drawing/Arimo-700.ttf"),
    include_bytes!("../../../../../apps/desktop/assets/fonts/drawing/Barlow-400-italic.ttf"),
    include_bytes!("../../../../../apps/desktop/assets/fonts/drawing/Barlow-400.ttf"),
    include_bytes!("../../../../../apps/desktop/assets/fonts/drawing/Barlow-500.ttf"),
    include_bytes!("../../../../../apps/desktop/assets/fonts/drawing/Barlow-600.ttf"),
    include_bytes!("../../../../../apps/desktop/assets/fonts/drawing/CourierPrime-400-italic.ttf"),
    include_bytes!("../../../../../apps/desktop/assets/fonts/drawing/CourierPrime-400.ttf"),
    include_bytes!("../../../../../apps/desktop/assets/fonts/drawing/CourierPrime-700.ttf"),
    include_bytes!("../../../../../apps/desktop/assets/fonts/drawing/IBMPlexMono-400.ttf"),
    include_bytes!("../../../../../apps/desktop/assets/fonts/drawing/IBMPlexMono-500.ttf"),
    include_bytes!("../../../../../apps/desktop/assets/fonts/drawing/Overpass-400.ttf"),
    include_bytes!("../../../../../apps/desktop/assets/fonts/drawing/Overpass-500.ttf"),
    include_bytes!("../../../../../apps/desktop/assets/fonts/drawing/Overpass-600.ttf"),
    include_bytes!("../../../../../apps/desktop/assets/fonts/drawing/Overpass-700.ttf"),
    include_bytes!("../../../../../apps/desktop/assets/fonts/drawing/Quicksand-400.ttf"),
    include_bytes!("../../../../../apps/desktop/assets/fonts/drawing/Quicksand-500.ttf"),
    include_bytes!("../../../../../apps/desktop/assets/fonts/drawing/Quicksand-600.ttf"),
    include_bytes!("../../../../../apps/desktop/assets/fonts/drawing/Quicksand-700.ttf"),
];

/// Each of [`DRAWING_FACES`] as the sheet core names it (`TextPrim.font`, its weight, italic): what a
/// sheet PDF embeds (docs/sheet/design.md §9a).
pub const DRAWING_KEYS: [(&str, u16, bool); 22] = [
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

/// The faces the desktop added after the web paused (docs/adr/9311), numbered after the fallback faces so every other
/// face keeps its number: Arial drawn in Arimo's bytes, Times New Roman in Liberation Serif's, their metric twins
/// (every letter as wide as the named face's). Where the host gives the named face itself ([`give`]), it is drawn in
/// that one; a layout is the same either way.
pub const ADDED_FACES: [&[u8]; 8] = [
    // Arimo 400 … 700, the same bytes as the drawing's (`added_faces_are_their_twins` holds the places).
    DRAWING_FACES[1],
    DRAWING_FACES[2],
    DRAWING_FACES[3],
    DRAWING_FACES[4],
    include_bytes!("../../../../../apps/desktop/assets/fonts/drawing/LiberationSerif-Regular.ttf"),
    include_bytes!("../../../../../apps/desktop/assets/fonts/drawing/LiberationSerif-Bold.ttf"),
    include_bytes!("../../../../../apps/desktop/assets/fonts/drawing/LiberationSerif-Italic.ttf"),
    include_bytes!("../../../../../apps/desktop/assets/fonts/drawing/LiberationSerif-BoldItalic.ttf"),
];

/// Each of [`ADDED_FACES`] as the sheet core names it.
pub const ADDED_KEYS: [(&str, u16, bool); 8] = [
    ("arial", 400, false),
    ("arial", 500, false),
    ("arial", 600, false),
    ("arial", 700, false),
    ("times-new-roman", 400, false),
    ("times-new-roman", 700, false),
    ("times-new-roman", 400, true),
    ("times-new-roman", 700, true),
];

/// Faces the host gives for an added key in place of its bundled twin (docs/adr/9311): the operating system's own
/// Arial and Times New Roman, which KentOS may use but not ship. Given once, before the first text is laid out (the
/// shaper keeps the faces it parsed).
static GIVEN: std::sync::OnceLock<Vec<Option<&'static [u8]>>> = std::sync::OnceLock::new();

/// Gives the host's faces by key (`("arial", 400, false)`), each in place of its bundled twin; a key that is not one of
/// [`ADDED_KEYS`] is left out. False when faces were given, or the core read its own, before.
pub fn give(faces: &[((&str, u16, bool), &'static [u8])]) -> bool {
    let mut given = vec![None; ADDED_KEYS.len()];
    for (key, bytes) in faces {
        if let Some(i) = ADDED_KEYS.iter().position(|k| k == key) {
            given[i] = Some(*bytes);
        }
    }
    GIVEN.set(given).is_ok()
}

/// The bytes of the added face at `i`: the host's own when it gave one, else the bundled twin.
pub fn added_face(i: usize) -> Option<&'static [u8]> {
    let given = GIVEN.get_or_init(|| vec![None; ADDED_KEYS.len()]);
    given
        .get(i)
        .copied()
        .flatten()
        .or_else(|| ADDED_FACES.get(i).copied())
}

/// Whether a face of the family (`arial`) is the host's own rather than the bundled twin.
pub fn given(family: &str) -> bool {
    GIVEN.get().is_some_and(|g| {
        ADDED_KEYS
            .iter()
            .zip(g)
            .any(|((f, _, _), bytes)| *f == family && bytes.is_some())
    })
}

/// Unicode fallback faces. These do not change the seven user-selected drawing families.
pub const FALLBACK_FACES: [&[u8]; 7] = [
    include_bytes!("../../data/text-fonts/NotoSans-400.ttf"),
    include_bytes!("../../data/text-fonts/NotoSans-600.ttf"),
    include_bytes!("../../data/text-fonts/NotoSansArabic-400.ttf"),
    include_bytes!("../../data/text-fonts/NotoSansArabic-600.ttf"),
    include_bytes!("../../data/text-fonts/NotoSansHebrew-400.ttf"),
    include_bytes!("../../data/text-fonts/NotoSansHebrew-600.ttf"),
    include_bytes!("../../data/text-fonts/NotoSansMath-400.ttf"),
];

pub const FALLBACK_KEYS: [(&str, u16, bool); 7] = [
    ("noto-sans", 400, false),
    ("noto-sans", 600, false),
    ("noto-sans-arabic", 400, false),
    ("noto-sans-arabic", 600, false),
    ("noto-sans-hebrew", 400, false),
    ("noto-sans-hebrew", 600, false),
    ("noto-sans-math", 400, false),
];

#[cfg(test)]
mod added_tests {
    use super::*;

    /// Arial's faces are Arimo's own (its metric twin), at the same weights.
    #[test]
    fn added_faces_are_their_twins() {
        for (i, (family, weight, italic)) in ADDED_KEYS.iter().enumerate().take(4) {
            assert_eq!((*family, *italic), ("arial", false));
            assert_eq!(DRAWING_KEYS[i + 1], ("arimo", *weight, false));
        }
    }
}
