//! The tiles' named tones (docs/adr/0800 §3): a manifest names its tile's
//! colour (`"color": "purple"`), the launcher paints it from here. Every
//! tile colour of the launcher is in this one table, with the ink its glyph
//! is drawn in: white where white reads, dark on the light tones.

use iced::Color;
use kentos_ui::theme::tokens::hex;

/// A tile's tone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Tone {
    Blue,
    Green,
    Yellow,
    Purple,
    Orange,
    Teal,
    Red,
    /// A tile without a colour of its own.
    #[default]
    Gray,
}

/// How a tile is painted: a light top fading to its tone, and the glyph's ink.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Paint {
    pub top: Color,
    pub bottom: Color,
    pub ink: Color,
}

/// The dark ink on the light tones (the KentOS accent's dark ink, DESIGN.md §3.1).
const DARK_INK: Color = hex(0x1b1405);

impl Tone {
    /// Every tone, in the colour wheel's order (the add dialog's swatches).
    pub const ALL: [Tone; 8] = [
        Tone::Blue,
        Tone::Teal,
        Tone::Green,
        Tone::Yellow,
        Tone::Orange,
        Tone::Red,
        Tone::Purple,
        Tone::Gray,
    ];

    /// The manifest's word for it.
    pub const fn key(self) -> &'static str {
        match self {
            Tone::Blue => "blue",
            Tone::Green => "green",
            Tone::Yellow => "yellow",
            Tone::Purple => "purple",
            Tone::Orange => "orange",
            Tone::Teal => "teal",
            Tone::Red => "red",
            Tone::Gray => "gray",
        }
    }

    /// Its name as the interface says it.
    pub const fn name(self) -> &'static str {
        match self {
            Tone::Blue => "Mavi",
            Tone::Green => "Yeşil",
            Tone::Yellow => "Sarı",
            Tone::Purple => "Mor",
            Tone::Orange => "Turuncu",
            Tone::Teal => "Petrol",
            Tone::Red => "Kırmızı",
            Tone::Gray => "Gri",
        }
    }

    /// The tone a manifest names; none for a word the launcher does not know.
    pub fn parse(key: &str) -> Option<Tone> {
        Tone::ALL.into_iter().find(|t| t.key() == key)
    }

    /// The words a manifest may write, for a refusal's sentence.
    pub fn known() -> String {
        Tone::ALL.map(Tone::key).join(", ")
    }

    /// How its tile is painted. The tones are the same in both themes: a
    /// tile is a mark of its own, like a program's icon.
    pub const fn paint(self) -> Paint {
        let (top, bottom, ink) = match self {
            Tone::Blue => (hex(0x5b95f8), hex(0x2f6fe2), Color::WHITE),
            Tone::Green => (hex(0x3cc08a), hex(0x178a5e), Color::WHITE),
            Tone::Yellow => (hex(0xf8d054), hex(0xe7a91a), DARK_INK),
            Tone::Purple => (hex(0xa585f7), hex(0x7552e4), Color::WHITE),
            Tone::Orange => (hex(0xf28c38), hex(0xcf5b10), Color::WHITE),
            Tone::Teal => (hex(0x33b4c6), hex(0x10798f), Color::WHITE),
            Tone::Red => (hex(0xf27c72), hex(0xd2463c), Color::WHITE),
            Tone::Gray => (hex(0x96a2b1), hex(0x667284), Color::WHITE),
        };
        Paint { top, bottom, ink }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// WCAG's relative luminance.
    fn luminance(c: Color) -> f32 {
        let f = |v: f32| {
            if v <= 0.039_28 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * f(c.r) + 0.7152 * f(c.g) + 0.0722 * f(c.b)
    }

    fn contrast(a: Color, b: Color) -> f32 {
        let (x, y) = (luminance(a), luminance(b));
        (x.max(y) + 0.05) / (x.min(y) + 0.05)
    }

    #[test]
    fn keys_go_both_ways_and_every_glyph_reads_on_its_tile() {
        for tone in Tone::ALL {
            assert_eq!(Tone::parse(tone.key()), Some(tone));
            let paint = tone.paint();
            // The glyph is a graphic object: WCAG's 3:1 against the tile's middle.
            let middle = Color::from_rgb(
                (paint.top.r + paint.bottom.r) / 2.0,
                (paint.top.g + paint.bottom.g) / 2.0,
                (paint.top.b + paint.bottom.b) / 2.0,
            );
            assert!(
                contrast(paint.ink, middle) >= 3.0,
                "{tone:?}: {}",
                contrast(paint.ink, middle)
            );
        }
        assert_eq!(
            Tone::parse("Mavi"),
            None,
            "the manifest's words, not the names"
        );
        assert!(Tone::known().starts_with("blue, teal"));
    }
}
