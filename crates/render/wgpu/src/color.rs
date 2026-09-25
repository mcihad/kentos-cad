//! Colours as the drawing writes them (TODOS.md REN-01): layer and entity
//! colours are data, `#rrggbb` (optionally with alpha) or a theme token that
//! the host's palette resolves (`fg`, `fg-dim`, `ink`, `paper`), as the web
//! does it (`apps/web/src/render/color.ts`). Nothing branches on a layer id.

/// An sRGB colour with straight alpha, 8 bits per channel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Rgba8(pub [u8; 4]);

impl Rgba8 {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self([r, g, b, 255])
    }

    /// `#rgb`, `#rrggbb` or `#rrggbbaa` (the `#` may be left out); `None` for anything else.
    pub fn parse_hex(text: &str) -> Option<Self> {
        let hex = text.trim();
        let hex = hex.strip_prefix('#').unwrap_or(hex);
        if !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        let digit = |i: usize| u8::from_str_radix(hex.get(i..i + 1)?, 16).ok();
        let pair = |i: usize| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok();
        match hex.len() {
            3 => Some(Self([digit(0)? * 17, digit(1)? * 17, digit(2)? * 17, 255])),
            6 => Some(Self([pair(0)?, pair(2)?, pair(4)?, 255])),
            8 => Some(Self([pair(0)?, pair(2)?, pair(4)?, pair(6)?])),
            _ => None,
        }
    }

    /// The same colour with another alpha (0..1).
    pub fn with_alpha(self, alpha: f64) -> Self {
        let [r, g, b, _] = self.0;
        Self([r, g, b, (alpha.clamp(0.0, 1.0) * 255.0).round() as u8])
    }

    /// Channels as 0..1 floats, still sRGB-encoded.
    pub fn to_f32(self) -> [f32; 4] {
        self.0.map(|c| f32::from(c) / 255.0)
    }
}

/// The theme's drawing colours that layer and entity colours may name (the
/// web's `--canvas-*` tokens, DESIGN.md §3.1–3.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Palette {
    /// The area's colour; `paper` names it.
    pub background: Rgba8,
    /// “Ana mürekkep”.
    pub fg: Rgba8,
    /// “İkincil mürekkep”.
    pub fg_dim: Rgba8,
    /// CAD colour 7: white on a dark area, black on a light one.
    pub ink: Rgba8,
}

impl Palette {
    /// A layer or entity colour: a theme token or a hex value; `None` when it is neither.
    pub fn resolve(&self, color: &str) -> Option<Rgba8> {
        match color.trim() {
            "fg" => Some(self.fg),
            "fg-dim" => Some(self.fg_dim),
            "ink" => Some(self.ink),
            "paper" => Some(self.background),
            other => Rgba8::parse_hex(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_colours_read_as_the_web_reads_them() {
        assert_eq!(
            Rgba8::parse_hex("#E06C75"),
            Some(Rgba8([0xe0, 0x6c, 0x75, 255]))
        );
        assert_eq!(
            Rgba8::parse_hex("#7FB2E526"),
            Some(Rgba8([0x7f, 0xb2, 0xe5, 0x26]))
        );
        assert_eq!(
            Rgba8::parse_hex("#abc"),
            Some(Rgba8([0xaa, 0xbb, 0xcc, 255]))
        );
        assert_eq!(Rgba8::parse_hex("ff0000"), Some(Rgba8([255, 0, 0, 255])));
        for bad in ["", "#", "#12", "#12345", "#gg0000", "kırmızı", "#ﬀﬀﬀ"] {
            assert_eq!(Rgba8::parse_hex(bad), None, "{bad}");
        }
    }

    #[test]
    fn tokens_come_from_the_palette() {
        let palette = Palette {
            background: Rgba8::rgb(1, 2, 3),
            fg: Rgba8::rgb(4, 5, 6),
            fg_dim: Rgba8::rgb(7, 8, 9),
            ink: Rgba8::rgb(10, 11, 12),
        };
        assert_eq!(palette.resolve("fg"), Some(palette.fg));
        assert_eq!(palette.resolve("fg-dim"), Some(palette.fg_dim));
        assert_eq!(palette.resolve("ink"), Some(palette.ink));
        assert_eq!(palette.resolve("paper"), Some(palette.background));
        assert_eq!(palette.resolve("#000"), Some(Rgba8::rgb(0, 0, 0)));
        assert_eq!(palette.resolve("accent"), None);
        assert_eq!(Rgba8::rgb(9, 9, 9).with_alpha(0.45).0[3], 115);
    }
}
