//! Tarama's and Çoklu tara's choices (docs/adr/0186 §4), one list for both
//! platforms: the patterns Desen (D) steps through and its menu shows
//! (Çizgili, Çapraz, Dolu, the library's, the three gradients), the pattern a
//! choice writes at the session's Ölçek, Açı, İkinci renk and Ters and the
//! drawing's plot scale, and İkinci renk's names.

use crate::api::Op;
use crate::entity::HatchPattern;
use crate::geom::hatch_pattern::{Gradient, library, turn_of};
use crate::op;
use crate::text::edit::fold;

/// What a choice writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Lines,
    Cross,
    Solid,
    /// The library's pattern at this place.
    Library(usize),
    /// A gradient of this shape (`linear`, `cylinder`, `spherical`).
    Gradient(&'static str),
}

/// One of Desen's choices: its name in the prompt, what its menu entry
/// says, what typing it gives, its icon and its wide sample in the menus
/// (docs/adr/0186 §11; `scripts/ui/hatch_icons.py` draws both).
#[derive(Clone, Debug, PartialEq)]
pub struct Choice {
    pub name: String,
    pub label: String,
    pub typed: String,
    pub icon: String,
    pub preview: String,
    pub kind: Kind,
    /// The pattern type it writes (`lines`, `cross`, `solid`, `pattern`, `gradient`).
    pub class: &'static str,
}

crate::json_struct!(out Choice { name, label, typed, icon, preview, class => "kind" });

/// User-defined lines are this far apart on the paper at Ölçek 1 (mm), and turned 45°.
pub const USER_SPACING_MM: f64 = 3.0;
pub const USER_ANGLE: f64 = 45.0;

/// The library's place of ANSI31, the tool's first pattern (Desen's 4th choice).
pub const DEFAULT_CHOICE: usize = 3;

const GRADIENTS: [(&str, &str, &str, &str); 3] = [
    ("linear", "Degrade doğrusal", "hatchGradientLinear", "hatchPreviewGradientLinear"),
    ("cylinder", "Degrade silindir", "hatchGradientCylinder", "hatchPreviewGradientCylinder"),
    ("spherical", "Degrade küre", "hatchGradientSpherical", "hatchPreviewGradientSpherical"),
];

/// İkinci renk's names (the ribbon's drawing colours, white and black).
pub const COLOURS: [(&str, &str); 9] = [
    ("Beyaz", "#FFFFFF"),
    ("Siyah", "#000000"),
    ("Kırmızı", "#E5484D"),
    ("Sarı", "#F2C94C"),
    ("Yeşil", "#5FBF77"),
    ("Camgöbeği", "#4CC3D9"),
    ("Mavi", "#4F8EF7"),
    ("Eflatun", "#C86DD7"),
    ("Gri", "#8C9AAA"),
];

/// A name as a typed one is matched: its case aside, Turkish, and the dotted
/// and the dotless i alike (ANSI31 is typed “ansi31” as often as “ansı31”).
fn folded(s: &str) -> String {
    s.trim()
        .chars()
        .map(|c| match fold(c) {
            'ı' => 'i',
            c => c,
        })
        .collect()
}

/// Desen's choices, in its order: Çizgili, Çapraz, Dolu, the library, the gradients.
pub fn choices() -> Vec<Choice> {
    let choice = |name: &str, label: String, (icon, preview): (String, String), kind: Kind| Choice {
        name: name.to_owned(),
        typed: name.to_lowercase(),
        label,
        icon,
        preview,
        kind,
        class: match kind {
            Kind::Lines => "lines",
            Kind::Cross => "cross",
            Kind::Solid => "solid",
            Kind::Library(_) => "pattern",
            Kind::Gradient(_) => "gradient",
        },
    };
    // A pattern's icon and wide sample by the same word (`hatchSwatchLINE`, `hatchPreviewLINE`).
    let drawn = |word: &str| (format!("hatchSwatch{word}"), format!("hatchPreview{word}"));
    let mut out = vec![
        choice("Çizgili", "Çizgili".to_owned(), drawn("Lines"), Kind::Lines),
        choice("Çapraz", "Çapraz".to_owned(), drawn("Cross"), Kind::Cross),
        choice("Dolu", "Dolu".to_owned(), drawn("Solid"), Kind::Solid),
    ];
    for (i, p) in library().iter().enumerate() {
        out.push(choice(
            p.name,
            format!("{} ({})", p.name, p.description.to_lowercase()),
            drawn(p.name),
            Kind::Library(i),
        ));
    }
    for (shape, name, icon, preview) in GRADIENTS {
        out.push(choice(
            name,
            name.to_owned(),
            (icon.to_owned(), preview.to_owned()),
            Kind::Gradient(shape),
        ));
    }
    out
}

/// The choice a typed name gives (its case aside, Turkish), if any; a
/// gradient by its shape's word too (“küre”: a name with a space cannot be
/// typed where the space gives the command line's Enter).
pub fn choice_named(text: &str) -> Option<usize> {
    let t = folded(text);
    if t.is_empty() {
        return None;
    }
    let list = choices();
    list.iter().position(|c| folded(&c.name) == t).or_else(|| {
        list.iter().position(|c| {
            matches!(c.kind, Kind::Gradient(_))
                && c.name.rsplit(' ').next().is_some_and(|w| folded(w) == t)
        })
    })
}

/// A colour typed for İkinci renk: `#RRGGBB` (any case) or one of `COLOURS`'
/// names; written `#RRGGBB` in capitals.
pub fn colour_of(text: &str) -> Option<String> {
    let t = text.trim();
    if t.len() == 7 && t.starts_with('#') && t[1..].chars().all(|c| c.is_ascii_hexdigit()) {
        return Some(t.to_ascii_uppercase());
    }
    let f = folded(t);
    COLOURS
        .iter()
        .find(|(name, _)| folded(name) == f)
        .map(|(_, hex)| (*hex).to_owned())
}

/// The session's pattern options (Ölçek, Açı, İkinci renk, Ters) and the drawing's plot scale (1:N).
#[derive(Clone, Debug, PartialEq)]
pub struct Options {
    pub choice: usize,
    pub scale: f64,
    pub angle: f64,
    pub color2: String,
    pub inverted: bool,
    pub plot_scale: f64,
}

crate::json_struct!(Options {
    choice,
    scale,
    angle,
    color2,
    inverted,
    plot_scale => "plotScale"
});

/// The pattern a choice writes (docs/adr/0186 §4): Çizgili and Çapraz
/// 3 mm × Ölçek apart on the paper and turned 45° + Açı (from 0 up to 180);
/// a library pattern turned by Açı, its millimetres Ölçek × N / 1000 metres
/// each; a gradient along Açı from the hatch's colour to İkinci renk; Dolu.
pub fn pattern_of(o: &Options) -> HatchPattern {
    let metres = o.scale * o.plot_scale / 1000.0;
    let list = choices();
    let kind = list.get(o.choice).map_or(Kind::Solid, |c| c.kind);
    match kind {
        Kind::Lines | Kind::Cross => HatchPattern::user(
            if kind == Kind::Lines {
                "lines"
            } else {
                "cross"
            },
            turn_of(USER_ANGLE + o.angle) % 180.0,
            USER_SPACING_MM * metres,
        ),
        Kind::Solid => HatchPattern::user("solid", 0.0, 1.0),
        Kind::Library(i) => {
            let p = &library()[i];
            HatchPattern {
                name: Some(p.name.to_owned()),
                scale: Some(metres),
                lines: Some(p.lines.clone()),
                ..HatchPattern::user("pattern", turn_of(o.angle), 1.0)
            }
        }
        Kind::Gradient(shape) => HatchPattern {
            gradient: Some(Gradient {
                shape: shape.to_owned(),
                inverted: o.inverted.then_some(true),
                color2: o.color2.clone(),
            }),
            ..HatchPattern::user("gradient", turn_of(o.angle), 1.0)
        },
    }
}

pub(crate) static OPS: &[Op] = &[
    op!("hatchChoices", || choices()),
    op!("hatchChoiceNamed", |text: String| choice_named(&text)),
    op!("hatchColour", |text: String| colour_of(&text)),
    op!("hatchToolPattern", |options: Options| pattern_of(&options)),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_choices_are_the_adr_s_order() {
        let list = choices();
        assert_eq!(list.len(), 3 + library().len() + 3);
        assert_eq!(list[DEFAULT_CHOICE].name, "ANSI31");
        assert_eq!(choice_named("ansı31"), Some(DEFAULT_CHOICE));
        assert_eq!(choice_named("ansi31"), Some(DEFAULT_CHOICE));
        assert_eq!(list[DEFAULT_CHOICE].typed, "ansi31");
        assert_eq!(choice_named(" ÇİZGİLİ "), Some(0));
        assert_eq!(choice_named("degrade küre"), Some(list.len() - 1));
        // A gradient by its shape's word alone; no other choice by a part of its name.
        assert_eq!(choice_named("KÜRE"), Some(list.len() - 1));
        assert_eq!(choice_named("silindir"), Some(list.len() - 2));
        assert_eq!(choice_named("degrade"), None);
        let names: std::collections::HashSet<&str> = list.iter().map(|c| c.icon.as_str()).collect();
        assert_eq!(names.len(), list.len(), "every choice its own icon");
        let previews: std::collections::HashSet<&str> = list.iter().map(|c| c.preview.as_str()).collect();
        assert_eq!(previews.len(), list.len(), "every choice its own wide sample");
        assert_eq!(list[DEFAULT_CHOICE].preview, "hatchPreviewANSI31");
    }

    #[test]
    fn a_choice_writes_its_pattern() {
        let o = |choice: usize, angle: f64| Options {
            choice,
            scale: 2.0,
            angle,
            color2: "#FFFFFF".into(),
            inverted: true,
            plot_scale: 500.0,
        };
        let lines = pattern_of(&o(0, 150.0));
        assert_eq!(
            (lines.kind.as_str(), lines.angle, lines.spacing),
            ("lines", 15.0, 3.0)
        );
        let ansi = pattern_of(&o(DEFAULT_CHOICE, -30.0));
        assert_eq!(
            (ansi.angle, ansi.scale, ansi.name.as_deref()),
            (330.0, Some(1.0), Some("ANSI31"))
        );
        let sphere = pattern_of(&o(choices().len() - 1, 0.0));
        assert_eq!(
            sphere.gradient.map(|g| (g.shape, g.inverted)),
            Some(("spherical".to_owned(), Some(true)))
        );
        assert_eq!(colour_of("kırmızı").as_deref(), Some("#E5484D"));
        assert_eq!(colour_of("#a0b1c2").as_deref(), Some("#A0B1C2"));
        assert_eq!(colour_of("mor"), None);
    }
}
