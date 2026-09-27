//! Colours as the style engine writes them (the web's `render/color.ts`):
//! `#RRGGBB`, `#RRGGBBAA` or a theme token, read into 0..1 RGBA of the
//! sRGB-encoded values (no linearisation), straight alpha.

/// The theme's colours a symbol may name instead of a hex value (the part of
/// the web's `CanvasPalette` the style engine reads).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StylePalette {
    /// `fg`: the main ink.
    pub fg: String,
    /// `fg-dim`: the secondary ink.
    pub fg_dim: String,
    /// `ink`: CAD colour 7, black on paper and white on a dark ground.
    pub ink: String,
    /// `paper`: the sheet itself, the drawing area's colour (knockouts, halos).
    pub paper: String,
}

impl StylePalette {
    /// The dark theme's canvas (DESIGN.md §3.1).
    pub fn graphite() -> Self {
        Self {
            fg: "#E4EAF0".into(),
            fg_dim: "#A3AFBC".into(),
            ink: "#FFFFFF".into(),
            paper: "#141A21".into(),
        }
    }

    /// The light theme's canvas (DESIGN.md §3.2).
    pub fn paper() -> Self {
        Self {
            fg: "#1E2833".into(),
            fg_dim: "#4D5966".into(),
            ink: "#000000".into(),
            paper: "#F8F9FA".into(),
        }
    }
}

/// A colour with its theme token resolved (`resolveColor`).
pub fn resolve<'a>(color: &'a str, palette: &'a StylePalette) -> &'a str {
    match color {
        "fg" => &palette.fg,
        "fg-dim" => &palette.fg_dim,
        "ink" => &palette.ink,
        "paper" => &palette.paper,
        other => other,
    }
}

/// `parseInt(s, 16)`: the leading hexadecimal digits, NaN when there are none.
fn parse_int_hex(s: &str) -> f64 {
    let mut value: Option<f64> = None;
    for c in s.chars() {
        match c.to_digit(16) {
            Some(d) => value = Some(value.unwrap_or(0.0) * 16.0 + f64::from(d)),
            None => break,
        }
    }
    value.unwrap_or(f64::NAN)
}

/// `"#RRGGBB"` or `"#RRGGBBAA"` (or `"#RGB"`) → 0..1 RGBA, as the web's
/// `parseHex` reads it: a malformed colour gives NaN parts, as there.
pub fn parse_hex(hex: &str, alpha_mul: f64) -> [f64; 4] {
    let h: String = hex.replace('#', "");
    let full: String = if h.chars().count() == 3 {
        h.chars().flat_map(|c| [c, c]).collect()
    } else {
        h
    };
    let part = |from: usize| -> String { full.chars().skip(from).take(2).collect() };
    let r = parse_int_hex(&part(0)) / 255.0;
    let g = parse_int_hex(&part(2)) / 255.0;
    let b = parse_int_hex(&part(4)) / 255.0;
    let a = if full.chars().count() >= 8 {
        parse_int_hex(&part(6)) / 255.0
    } else {
        1.0
    };
    [r, g, b, a * alpha_mul]
}

/// A colour of the palette read into RGBA with its opacity (`Looks.rgba`).
pub fn rgba(color: &str, opacity: f64, palette: &StylePalette) -> [f64; 4] {
    let c = parse_hex(resolve(color, palette), 1.0);
    [c[0], c[1], c[2], c[3] * opacity]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_hex_as_the_web_does() {
        assert_eq!(
            parse_hex("#E5484D33", 1.0),
            [229.0 / 255.0, 72.0 / 255.0, 77.0 / 255.0, 51.0 / 255.0]
        );
        assert_eq!(parse_hex("#fff", 1.0), [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(parse_hex("#000000", 0.5)[3], 0.5);
        assert!(parse_hex("kırmızı", 1.0)[0].is_nan());
    }

    #[test]
    fn resolves_theme_tokens() {
        let p = StylePalette::graphite();
        assert_eq!(resolve("ink", &p), "#FFFFFF");
        assert_eq!(resolve("#123456", &p), "#123456");
    }
}
