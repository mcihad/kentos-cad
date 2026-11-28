//! The typeface of the text that is part of the drawing (text objects, dimension values, labels; docs/adr/0055,
//! 0183). A project setting: everyone who opens the project sees the same letters. Files written before it have none
//! (Barlow).
//!
//! It is one of KentOS's own nine faces or a family found in a fonts folder (docs/adr/9313). The fonts folders are
//! the install's `fonts` and the user's (CLAUDE.md §26). Either way it is written as one string. KentOS's faces go
//! by their ids (`courier-prime`). A folder's family goes by its name as its file gives it (`Roboto`).
//!
//! New projects write Arial (docs/adr/9311). The operating system's own Arial and Times New Roman draw where they are
//! installed. Elsewhere their bundled metric twins (Arimo, Liberation Serif) draw, with the same widths.

use std::borrow::Cow;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// The most letters a folder family's name may have.
pub const MAX_FONT_FAMILY: usize = 64;
/// The most folder families one program reads over its life (docs/adr/9313 §2). Names are kept once each, so a
/// stream of made-up names cannot grow the program's memory without end.
pub const MAX_FONT_FAMILIES: usize = 65_536;

/// A family found in a fonts folder (docs/adr/9313), by its name as its file gives it (`Roboto`). A project names it.
/// A program whose fonts folders hold it draws it; any other program draws the project's face in its place and says
/// so (`font_substituted`).
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FontFamily(&'static str);

impl FontFamily {
    /// The family named `name`, trimmed. None for:
    /// - an empty name, one longer than [`MAX_FONT_FAMILY`] or one with a control character;
    /// - the name of one of KentOS's own faces ([`DrawingFont::from_name`] gives those);
    /// - a new name past [`MAX_FONT_FAMILIES`].
    pub fn new(name: &str) -> Option<FontFamily> {
        let name = name.trim();
        if name.is_empty()
            || name.chars().count() > MAX_FONT_FAMILY
            || name.chars().any(char::is_control)
        {
            return None;
        }
        if DrawingFont::own(name).is_some() {
            return None;
        }
        intern(name).map(FontFamily)
    }

    /// Its name (`Roboto`).
    pub fn name(self) -> &'static str {
        self.0
    }
}

impl std::fmt::Debug for FontFamily {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

/// Keeps each family name once for the program's life (a `FontFamily` is `Copy`, as the typeface always was).
fn intern(name: &str) -> Option<&'static str> {
    static NAMES: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());
    let mut names = NAMES
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(kept) = names.iter().find(|n| **n == name) {
        return Some(kept);
    }
    if names.len() >= MAX_FONT_FAMILIES {
        return None;
    }
    let kept: &'static str = Box::leak(name.to_owned().into_boxed_str());
    names.push(kept);
    Some(kept)
}

/// The drawing's typeface: one of KentOS's own faces or a family from a fonts folder (module docs). Ordered as the
/// contract numbers the own faces, the folder families after them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "ts", ts(export, type = "string"))]
pub enum DrawingFont {
    Barlow,
    Arimo,
    Overpass,
    Quicksand,
    ArchitectsDaughter,
    CourierPrime,
    PlexMono,
    /// docs/adr/9311: after the web's seven, so every file keeps its numbers.
    Arial,
    TimesNewRoman,
    /// A family from a fonts folder (docs/adr/9313).
    Installed(FontFamily),
}

impl DrawingFont {
    /// KentOS's own faces, in the contract's order (the typed columns and the core's metrics number them so).
    pub const ALL: [DrawingFont; 9] = [
        DrawingFont::Barlow,
        DrawingFont::Arimo,
        DrawingFont::Overpass,
        DrawingFont::Quicksand,
        DrawingFont::ArchitectsDaughter,
        DrawingFont::CourierPrime,
        DrawingFont::PlexMono,
        DrawingFont::Arial,
        DrawingFont::TimesNewRoman,
    ];

    /// KentOS's own faces in the order a person picks one: Arial and Times New Roman first (docs/adr/9311). The folder
    /// families follow them in a list (docs/adr/9313).
    pub const CHOICES: [DrawingFont; 9] = [
        DrawingFont::Arial,
        DrawingFont::TimesNewRoman,
        DrawingFont::Barlow,
        DrawingFont::Arimo,
        DrawingFont::Overpass,
        DrawingFont::Quicksand,
        DrawingFont::ArchitectsDaughter,
        DrawingFont::CourierPrime,
        DrawingFont::PlexMono,
    ];

    /// A new project's typeface (docs/adr/9311).
    pub const NEW_PROJECT: DrawingFont = DrawingFont::Arial;

    /// Its name in the contract and the file: an own face's id (`courier-prime`), a folder family's name (`Roboto`).
    pub fn id(self) -> &'static str {
        match self {
            DrawingFont::Barlow => "barlow",
            DrawingFont::Arimo => "arimo",
            DrawingFont::Overpass => "overpass",
            DrawingFont::Quicksand => "quicksand",
            DrawingFont::ArchitectsDaughter => "architects-daughter",
            DrawingFont::CourierPrime => "courier-prime",
            DrawingFont::PlexMono => "plex-mono",
            DrawingFont::Arial => "arial",
            DrawingFont::TimesNewRoman => "times-new-roman",
            DrawingFont::Installed(family) => family.name(),
        }
    }

    /// The geometry core's id of the typeface a drawing writes in (none: Barlow).
    pub fn id_of(font: Option<DrawingFont>) -> &'static str {
        font.unwrap_or(DrawingFont::Barlow).id()
    }

    /// The own face named `id`; none for any other name.
    pub fn from_id(id: &str) -> Option<DrawingFont> {
        Self::ALL.into_iter().find(|f| f.id() == id)
    }

    /// The own face by its id or its name in any case (`times-new-roman`, `Times New Roman`).
    pub fn own(name: &str) -> Option<DrawingFont> {
        Self::from_id(name).or_else(|| {
            Self::ALL
                .into_iter()
                .find(|f| f.label().eq_ignore_ascii_case(name))
        })
    }

    /// The typeface a string names: an own face by its id or name, else a folder family by its name. None for a
    /// name no family may have ([`FontFamily::new`]).
    pub fn from_name(name: &str) -> Option<DrawingFont> {
        Self::own(name.trim()).or_else(|| FontFamily::new(name).map(DrawingFont::Installed))
    }

    /// Its name as the interface writes it (`Courier Prime`, `Roboto`).
    pub fn label(self) -> &'static str {
        match self {
            DrawingFont::Barlow => "Barlow",
            DrawingFont::Arimo => "Arimo",
            DrawingFont::Overpass => "Overpass",
            DrawingFont::Quicksand => "Quicksand",
            DrawingFont::ArchitectsDaughter => "Architects Daughter",
            DrawingFont::CourierPrime => "Courier Prime",
            DrawingFont::PlexMono => "IBM Plex Mono",
            DrawingFont::Arial => "Arial",
            DrawingFont::TimesNewRoman => "Times New Roman",
            DrawingFont::Installed(family) => family.name(),
        }
    }

    /// The folder family it is; none for an own face.
    pub fn family(self) -> Option<FontFamily> {
        match self {
            DrawingFont::Installed(family) => Some(family),
            _ => None,
        }
    }
}

/// Why a string names no typeface, for the reader's error.
fn not_a_font(name: &str) -> String {
    let own: Vec<&str> = DrawingFont::ALL.iter().map(|f| f.id()).collect();
    format!(
        "“{name}” bir yazı tipi değil: KentOS'un yüzlerinden biri ({}) ya da yazı tipi klasöründeki bir ailenin 1–{MAX_FONT_FAMILY} harflik adı olmalı",
        own.join(", ")
    )
}

impl Serialize for DrawingFont {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.id())
    }
}

impl<'de> Deserialize<'de> for DrawingFont {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let text = Cow::<str>::deserialize(d)?;
        DrawingFont::from_name(&text).ok_or_else(|| serde::de::Error::custom(not_a_font(&text)))
    }
}

#[cfg(feature = "schema")]
impl schemars::JsonSchema for DrawingFont {
    fn schema_name() -> Cow<'static, str> {
        "DrawingFont".into()
    }

    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        let own: Vec<&str> = DrawingFont::ALL.iter().map(|f| f.id()).collect();
        schemars::json_schema!({
            "description": "KentOS'un yüzlerinden biri ya da yazı tipi klasöründeki bir ailenin adı.",
            "anyOf": [
                { "type": "string", "enum": own },
                {
                    "type": "string",
                    "minLength": 1,
                    "maxLength": MAX_FONT_FAMILY,
                    "description": "Yazı tipi klasöründeki bir ailenin adı (Roboto)."
                }
            ]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn own_faces_keep_their_ids_and_folder_families_their_names() {
        for f in DrawingFont::ALL {
            let text = serde_json::to_string(&f).unwrap_or_default();
            assert_eq!(text, format!("\"{}\"", f.id()));
            assert_eq!(serde_json::from_str::<DrawingFont>(&text).ok(), Some(f));
        }
        let roboto = DrawingFont::from_name(" Roboto ");
        assert_eq!(roboto.map(DrawingFont::id), Some("Roboto"));
        assert_eq!(
            roboto.and_then(DrawingFont::family).map(FontFamily::name),
            Some("Roboto")
        );
        assert_eq!(
            serde_json::to_string(&roboto).ok().as_deref(),
            Some("\"Roboto\"")
        );
        assert_eq!(
            serde_json::from_str::<DrawingFont>("\"Roboto\"").ok(),
            roboto
        );
    }

    #[test]
    fn an_own_face_by_its_name_is_the_own_face() {
        assert_eq!(
            DrawingFont::from_name("Times New Roman"),
            Some(DrawingFont::TimesNewRoman)
        );
        assert_eq!(DrawingFont::from_name("arial"), Some(DrawingFont::Arial));
        assert_eq!(
            DrawingFont::from_name("IBM PLEX MONO"),
            Some(DrawingFont::PlexMono)
        );
        assert_eq!(FontFamily::new("Arial"), None);
    }

    #[test]
    fn a_name_no_family_may_have_is_refused() {
        let long = "x".repeat(MAX_FONT_FAMILY + 1);
        for bad in ["", "   ", "A\u{7}B", long.as_str()] {
            assert_eq!(DrawingFont::from_name(bad), None, "{bad:?}");
            let text = serde_json::to_string(bad).unwrap_or_default();
            let err = serde_json::from_str::<DrawingFont>(&text)
                .err()
                .map(|e| e.to_string())
                .unwrap_or_default();
            assert!(err.contains("bir yazı tipi değil"), "{err}");
        }
        assert!(DrawingFont::from_name(&"x".repeat(MAX_FONT_FAMILY)).is_some());
    }

    #[test]
    fn a_family_is_kept_once() {
        let a = FontFamily::new("Source Sans 3").map(FontFamily::name);
        let b = FontFamily::new("Source Sans 3").map(FontFamily::name);
        assert!(a.zip(b).is_some_and(|(a, b)| std::ptr::eq(a, b)));
        assert_ne!(FontFamily::new("roboto"), FontFamily::new("Roboto"));
    }
}
