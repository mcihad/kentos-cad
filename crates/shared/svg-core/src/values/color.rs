//! Colours (every CSS syntax and keyword, with alpha) and paints (the
//! symbol's colours, references and their fallback chains).

use kentos_geometry_core::jsmath::{js_max, js_min, js_round};
use kentos_style_core::js::text::{is_space, slice, trim};

use super::number::{parse_float, parse_hex, split_on};

/// CSS colour keywords (CSS Color 4), name then hex.
const NAMED: [(&str, &str); 148] = [
    ("aliceblue", "F0F8FF"),
    ("antiquewhite", "FAEBD7"),
    ("aqua", "00FFFF"),
    ("aquamarine", "7FFFD4"),
    ("azure", "F0FFFF"),
    ("beige", "F5F5DC"),
    ("bisque", "FFE4C4"),
    ("black", "000000"),
    ("blanchedalmond", "FFEBCD"),
    ("blue", "0000FF"),
    ("blueviolet", "8A2BE2"),
    ("brown", "A52A2A"),
    ("burlywood", "DEB887"),
    ("cadetblue", "5F9EA0"),
    ("chartreuse", "7FFF00"),
    ("chocolate", "D2691E"),
    ("coral", "FF7F50"),
    ("cornflowerblue", "6495ED"),
    ("cornsilk", "FFF8DC"),
    ("crimson", "DC143C"),
    ("cyan", "00FFFF"),
    ("darkblue", "00008B"),
    ("darkcyan", "008B8B"),
    ("darkgoldenrod", "B8860B"),
    ("darkgray", "A9A9A9"),
    ("darkgreen", "006400"),
    ("darkgrey", "A9A9A9"),
    ("darkkhaki", "BDB76B"),
    ("darkmagenta", "8B008B"),
    ("darkolivegreen", "556B2F"),
    ("darkorange", "FF8C00"),
    ("darkorchid", "9932CC"),
    ("darkred", "8B0000"),
    ("darksalmon", "E9967A"),
    ("darkseagreen", "8FBC8F"),
    ("darkslateblue", "483D8B"),
    ("darkslategray", "2F4F4F"),
    ("darkslategrey", "2F4F4F"),
    ("darkturquoise", "00CED1"),
    ("darkviolet", "9400D3"),
    ("deeppink", "FF1493"),
    ("deepskyblue", "00BFFF"),
    ("dimgray", "696969"),
    ("dimgrey", "696969"),
    ("dodgerblue", "1E90FF"),
    ("firebrick", "B22222"),
    ("floralwhite", "FFFAF0"),
    ("forestgreen", "228B22"),
    ("fuchsia", "FF00FF"),
    ("gainsboro", "DCDCDC"),
    ("ghostwhite", "F8F8FF"),
    ("gold", "FFD700"),
    ("goldenrod", "DAA520"),
    ("gray", "808080"),
    ("green", "008000"),
    ("greenyellow", "ADFF2F"),
    ("grey", "808080"),
    ("honeydew", "F0FFF0"),
    ("hotpink", "FF69B4"),
    ("indianred", "CD5C5C"),
    ("indigo", "4B0082"),
    ("ivory", "FFFFF0"),
    ("khaki", "F0E68C"),
    ("lavender", "E6E6FA"),
    ("lavenderblush", "FFF0F5"),
    ("lawngreen", "7CFC00"),
    ("lemonchiffon", "FFFACD"),
    ("lightblue", "ADD8E6"),
    ("lightcoral", "F08080"),
    ("lightcyan", "E0FFFF"),
    ("lightgoldenrodyellow", "FAFAD2"),
    ("lightgray", "D3D3D3"),
    ("lightgreen", "90EE90"),
    ("lightgrey", "D3D3D3"),
    ("lightpink", "FFB6C1"),
    ("lightsalmon", "FFA07A"),
    ("lightseagreen", "20B2AA"),
    ("lightskyblue", "87CEFA"),
    ("lightslategray", "778899"),
    ("lightslategrey", "778899"),
    ("lightsteelblue", "B0C4DE"),
    ("lightyellow", "FFFFE0"),
    ("lime", "00FF00"),
    ("limegreen", "32CD32"),
    ("linen", "FAF0E6"),
    ("magenta", "FF00FF"),
    ("maroon", "800000"),
    ("mediumaquamarine", "66CDAA"),
    ("mediumblue", "0000CD"),
    ("mediumorchid", "BA55D3"),
    ("mediumpurple", "9370DB"),
    ("mediumseagreen", "3CB371"),
    ("mediumslateblue", "7B68EE"),
    ("mediumspringgreen", "00FA9A"),
    ("mediumturquoise", "48D1CC"),
    ("mediumvioletred", "C71585"),
    ("midnightblue", "191970"),
    ("mintcream", "F5FFFA"),
    ("mistyrose", "FFE4E1"),
    ("moccasin", "FFE4B5"),
    ("navajowhite", "FFDEAD"),
    ("navy", "000080"),
    ("oldlace", "FDF5E6"),
    ("olive", "808000"),
    ("olivedrab", "6B8E23"),
    ("orange", "FFA500"),
    ("orangered", "FF4500"),
    ("orchid", "DA70D6"),
    ("palegoldenrod", "EEE8AA"),
    ("palegreen", "98FB98"),
    ("paleturquoise", "AFEEEE"),
    ("palevioletred", "DB7093"),
    ("papayawhip", "FFEFD5"),
    ("peachpuff", "FFDAB9"),
    ("peru", "CD853F"),
    ("pink", "FFC0CB"),
    ("plum", "DDA0DD"),
    ("powderblue", "B0E0E6"),
    ("purple", "800080"),
    ("rebeccapurple", "663399"),
    ("red", "FF0000"),
    ("rosybrown", "BC8F8F"),
    ("royalblue", "4169E1"),
    ("saddlebrown", "8B4513"),
    ("salmon", "FA8072"),
    ("sandybrown", "F4A460"),
    ("seagreen", "2E8B57"),
    ("seashell", "FFF5EE"),
    ("sienna", "A0522D"),
    ("silver", "C0C0C0"),
    ("skyblue", "87CEEB"),
    ("slateblue", "6A5ACD"),
    ("slategray", "708090"),
    ("slategrey", "708090"),
    ("snow", "FFFAFA"),
    ("springgreen", "00FF7F"),
    ("steelblue", "4682B4"),
    ("tan", "D2B48C"),
    ("teal", "008080"),
    ("thistle", "D8BFD8"),
    ("tomato", "FF6347"),
    ("turquoise", "40E0D0"),
    ("violet", "EE82EE"),
    ("wheat", "F5DEB3"),
    ("white", "FFFFFF"),
    ("whitesmoke", "F5F5F5"),
    ("yellow", "FFFF00"),
    ("yellowgreen", "9ACD32"),
];

/// A colour as hex (#RRGGBB) and alpha.
#[derive(Clone, Debug, PartialEq)]
pub struct Color {
    pub hex: String,
    pub alpha: f64,
}

kentos_geometry_core::json_struct!(Color { hex, alpha });

/// Two hex digits of a channel (0–255, rounded; NaN writes "NAN" as JavaScript does).
pub fn hex2(v: f64) -> String {
    let r = js_round(js_max(0.0, js_min(255.0, v)));
    if r.is_nan() {
        return "NAN".into();
    }
    format!("{:02X}", r as u32)
}

pub fn clamp01(v: f64) -> f64 {
    js_max(0.0, js_min(1.0, v))
}

fn hsl(h: f64, s: f64, l: f64) -> [f64; 3] {
    let k = |n: f64| (n + h / 30.0) % 12.0;
    let a = s * js_min(l, 1.0 - l);
    let f = |n: f64| l - a * js_max(-1.0, js_min(js_min(k(n) - 3.0, 9.0 - k(n)), 1.0));
    [f(0.0) * 255.0, f(8.0) * 255.0, f(4.0) * 255.0]
}

fn hex_digits(s: &str) -> bool {
    s.bytes()
        .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

/// A CSS colour as hex (#RRGGBB) and alpha; None when it is not a colour.
pub fn read_color(v: &str) -> Option<Color> {
    let s = trim(v).to_lowercase();
    if let Some(d) = s.strip_prefix('#')
        && (d.len() == 3 || d.len() == 4)
        && hex_digits(d)
    {
        let b = d.as_bytes();
        let hex: String = format!(
            "#{0}{0}{1}{1}{2}{2}",
            b[0] as char, b[1] as char, b[2] as char
        )
        .to_uppercase();
        let alpha = if d.len() == 4 {
            u32::from_str_radix(&format!("{0}{0}", b[3] as char), 16).unwrap_or(0) as f64 / 255.0
        } else {
            1.0
        };
        return Some(Color { hex, alpha });
    }
    if let Some(d) = s.strip_prefix('#')
        && (d.len() == 6 || d.len() == 8)
        && hex_digits(d)
    {
        let alpha = if d.len() == 8 {
            u32::from_str_radix(&d[6..], 16).unwrap_or(0) as f64 / 255.0
        } else {
            1.0
        };
        return Some(Color {
            hex: format!("#{}", d[..6].to_uppercase()),
            alpha,
        });
    }
    if let Some(&(_, hex)) = NAMED.iter().find(|(n, _)| *n == s) {
        return Some(Color {
            hex: format!("#{hex}"),
            alpha: 1.0,
        });
    }
    // `^(rgba?|hsla?)\(([^)]*)\)$`
    let (func, args) = ["rgba(", "rgb(", "hsla(", "hsl("]
        .iter()
        .find_map(|f| s.strip_prefix(f).map(|rest| (*f, rest)))?;
    let inner = args.strip_suffix(')')?;
    if inner.contains(')') {
        return None;
    }
    let parts = split_on(inner, |c| is_space(c) || c == ',' || c == '/');
    if parts.len() < 3 {
        return None;
    }
    let pct = |t: &str, full: f64| {
        if t.ends_with('%') {
            (parse_float(t) * full) / 100.0
        } else {
            parse_float(t)
        }
    };
    let alpha = match parts.get(3) {
        None => 1.0,
        Some(p) => clamp01(pct(p, 1.0)),
    };
    let rgb = if func.starts_with("rgb") {
        [
            pct(parts[0], 255.0),
            pct(parts[1], 255.0),
            pct(parts[2], 255.0),
        ]
    } else {
        hsl(
            ((parse_float(parts[0]) % 360.0) + 360.0) % 360.0,
            clamp01(parse_float(parts[1]) / 100.0),
            clamp01(parse_float(parts[2]) / 100.0),
        )
    };
    if rgb.iter().any(|c| !c.is_finite()) {
        return None;
    }
    Some(Color {
        hex: format!("#{}{}{}", hex2(rgb[0]), hex2(rgb[1]), hex2(rgb[2])),
        alpha,
    })
}

/// A paint once references are followed: none, the symbol's colours or a colour.
#[derive(Clone, Debug, PartialEq)]
pub enum Flat {
    None,
    Fill,
    Stroke,
    Color(Color),
}

/// A paint as written, before references are followed: a plain paint, or
/// references (each one's fallback the next) ending in an optional plain
/// paint. A chain, not nested fallbacks, so a long one never recurses.
#[derive(Clone, Debug, PartialEq)]
pub enum RawPaint {
    Plain(Flat),
    Url {
        ids: Vec<String>,
        fallback: Option<Flat>,
    },
}

/// `^url\(\s*['"]?#([^'")\s]+)['"]?\s*\)\s*(.*)$` (case-insensitive): the id and what follows.
fn url_ref(s: &str) -> Option<(String, &str)> {
    if !s.get(..4).is_some_and(|p| p.eq_ignore_ascii_case("url(")) {
        return None;
    }
    let mut r = s[4..].trim_start_matches(is_space);
    if r.starts_with('\'') || r.starts_with('"') {
        r = &r[1..];
    }
    r = r.strip_prefix('#')?;
    let id_len: usize = r
        .char_indices()
        .find(|&(_, c)| c == '\'' || c == '"' || c == ')' || is_space(c))
        .map_or(r.len(), |(k, _)| k);
    if id_len == 0 {
        return None;
    }
    let id = r[..id_len].to_string();
    let mut r = &r[id_len..];
    if r.starts_with('\'') || r.starts_with('"') {
        r = &r[1..];
    }
    r = r.trim_start_matches(is_space);
    r = r.strip_prefix(')')?;
    r = r.trim_start_matches(is_space);
    // `.` does not match line terminators: the tail must be one line.
    if r.contains(['\n', '\r', '\u{2028}', '\u{2029}']) {
        return None;
    }
    Some((id, r))
}

/// One level of `rawPaint`: nothing, a plain paint, or a reference and the text after it.
enum Step<'a> {
    Undefined,
    Plain(Flat),
    Url(String, &'a str),
}

fn paint_step(v: &str) -> Step<'_> {
    let s = trim(v);
    let low = s.to_lowercase();
    if s.is_empty() || low == "inherit" {
        return Step::Undefined;
    }
    if low == "none" || low == "transparent" {
        return Step::Plain(Flat::None);
    }
    if low == "currentcolor" || low.starts_with("param(fill") {
        return Step::Plain(Flat::Fill);
    }
    if low.starts_with("param(stroke") {
        return Step::Plain(Flat::Stroke);
    }
    if let Some((id, tail)) = url_ref(s) {
        return Step::Url(id, tail);
    }
    if low.starts_with("url(") {
        return Step::Plain(Flat::None);
    }
    match read_color(s) {
        Some(c) => Step::Plain(Flat::Color(c)),
        None => Step::Undefined,
    }
}

/// A paint value as written; None when it says nothing (empty, `inherit`, unreadable).
pub fn raw_paint(v: Option<&str>) -> Option<RawPaint> {
    let mut ids = Vec::new();
    let mut text = v?;
    loop {
        match paint_step(text) {
            Step::Url(id, tail) => {
                ids.push(id);
                if tail.is_empty() {
                    return Some(RawPaint::Url {
                        ids,
                        fallback: None,
                    });
                }
                text = tail;
            }
            Step::Plain(p) if ids.is_empty() => return Some(RawPaint::Plain(p)),
            Step::Plain(p) => {
                return Some(RawPaint::Url {
                    ids,
                    fallback: Some(p),
                });
            }
            Step::Undefined if ids.is_empty() => return None,
            Step::Undefined => {
                return Some(RawPaint::Url {
                    ids,
                    fallback: None,
                });
            }
        }
    }
}

pub fn with_alpha(hex: &str, alpha: f64) -> String {
    if alpha < 0.999 {
        format!("{hex}{}", hex2(alpha * 255.0))
    } else {
        hex.to_string()
    }
}

/// Near-black colours (the "black" of icon sets: #000, #1D1D1B, #231F20 …) that become the symbol colour.
pub fn is_near_black(hex: &str) -> bool {
    let r = parse_hex(&slice(hex, 1, Some(3)));
    let g = parse_hex(&slice(hex, 3, Some(5)));
    let b = parse_hex(&slice(hex, 5, Some(7)));
    js_max(js_max(r, g), b) <= 48.0
}

/// A paint value as the model's paint (black is the symbol colour); None when it cannot be read.
pub fn read_paint(v: Option<&str>) -> Option<String> {
    match raw_paint(v)? {
        RawPaint::Url { .. } => None,
        RawPaint::Plain(p) => Some(match p {
            Flat::None => "none".into(),
            Flat::Fill => "fill".into(),
            Flat::Stroke => "stroke".into(),
            Flat::Color(c) if c.hex == "#000000" && c.alpha >= 0.999 => "fill".into(),
            Flat::Color(c) => with_alpha(&c.hex, c.alpha),
        }),
    }
}
