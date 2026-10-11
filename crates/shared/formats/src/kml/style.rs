//! KML's styles as KentOS reads and writes them (docs/adr/0222 §3, §4): a
//! colour is `aabbggrr` in hexadecimal (alpha first, then blue, green, red);
//! a line's width is in pixels, a plot line weight is in millimetres (a
//! pixel is 0.2646 mm at 96 dpi; the weight read is the nearest of the
//! standard ones).

use roxmltree::Node;

/// A colour: red, green, blue, alpha.
pub type Rgba = [u8; 4];

/// The standard plot line weights (mm).
pub const WEIGHTS: [f64; 9] = [0.13, 0.18, 0.25, 0.35, 0.5, 0.7, 1.0, 1.4, 2.0];

/// Millimetres per pixel at 96 dpi.
pub const MM_PER_PIXEL: f64 = 25.4 / 96.0;

/// What a Style gives: its line's colour and width, its area's colour and
/// whether it is filled and outlined, its icon's colour.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Style {
    pub line_color: Option<Rgba>,
    pub line_width: Option<f64>,
    pub poly_color: Option<Rgba>,
    pub fill: bool,
    pub outline: bool,
    pub icon_color: Option<Rgba>,
}

fn child<'a, 'i>(n: Node<'a, 'i>, name: &str) -> Option<Node<'a, 'i>> {
    n.children()
        .find(|c| c.is_element() && c.tag_name().name() == name)
}

fn text(n: Option<Node<'_, '_>>) -> Option<String> {
    Some(n?.text()?.trim().to_owned()).filter(|t| !t.is_empty())
}

/// A KML colour (`aabbggrr`), none when it is not eight hexadecimal digits.
pub fn parse_color(t: &str) -> Option<Rgba> {
    let t = t.trim().trim_start_matches('#');
    if t.len() != 8 || !t.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(&t[i..i + 2], 16).ok();
    Some([byte(6)?, byte(4)?, byte(2)?, byte(0)?])
}

/// A boolean as KML writes it (`1`, `0`, `true`, `false`), `default` when absent or neither.
fn flag(t: Option<String>, default: bool) -> bool {
    match t.as_deref() {
        Some("1" | "true") => true,
        Some("0" | "false") => false,
        _ => default,
    }
}

impl Style {
    /// A `Style` element's colours and widths.
    pub fn of(n: Node<'_, '_>) -> Style {
        let line = child(n, "LineStyle");
        let poly = child(n, "PolyStyle");
        let icon = child(n, "IconStyle");
        Style {
            line_color: line
                .and_then(|l| text(child(l, "color")))
                .and_then(|t| parse_color(&t)),
            line_width: line
                .and_then(|l| text(child(l, "width")))
                .and_then(|t| t.parse::<f64>().ok())
                .filter(|w| w.is_finite() && *w > 0.0),
            poly_color: poly
                .and_then(|p| text(child(p, "color")))
                .and_then(|t| parse_color(&t)),
            fill: flag(poly.and_then(|p| text(child(p, "fill"))), true),
            outline: flag(poly.and_then(|p| text(child(p, "outline"))), true),
            icon_color: icon
                .and_then(|i| text(child(i, "color")))
                .and_then(|t| parse_color(&t)),
        }
    }
}

/// A colour as KentOS writes it: `#rrggbb` (the alpha is not an object's).
pub fn kml_color(c: Rgba) -> String {
    format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
}

/// A KML width in pixels as the nearest standard plot line weight (mm).
pub fn line_weight(px: f64) -> f64 {
    let mm = px * MM_PER_PIXEL;
    WEIGHTS
        .iter()
        .copied()
        .min_by(|a, b| (a - mm).abs().total_cmp(&(b - mm).abs()))
        .unwrap_or(0.25)
}

/// A KentOS colour (`#rrggbb`, `#rgb`) as KML writes it with `alpha`: `aabbggrr`; none for a theme token.
pub fn to_kml(color: &str, alpha: u8) -> Option<String> {
    let h = color.strip_prefix('#')?;
    let full = match h.len() {
        6 => h.to_owned(),
        3 => h.chars().flat_map(|c| [c, c]).collect(),
        _ => return None,
    };
    if !full.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let f = full.to_lowercase();
    Some(format!("{alpha:02x}{}{}{}", &f[4..6], &f[2..4], &f[0..2]))
}
