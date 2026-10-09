//! CSS colours as MapLibre styles write them (docs/adr/0208 §9): `#rgb`,
//! `#rgba`, `#rrggbb`, `#rrggbbaa`, `rgb()`, `rgba()`, `hsl()`, `hsla()`
//! (commas or spaces, percentages) and the named colours of CSS Color 4.
//! A colour is `[r, g, b, a]`, each 0 to 1.

pub type Rgba = [f64; 4];

/// The named colours of CSS Color Module Level 4, `transparent` among them.
const NAMED: [(&str, u32); 149] = [
    ("aliceblue", 0xf0f8ff),
    ("antiquewhite", 0xfaebd7),
    ("aqua", 0x00ffff),
    ("aquamarine", 0x7fffd4),
    ("azure", 0xf0ffff),
    ("beige", 0xf5f5dc),
    ("bisque", 0xffe4c4),
    ("black", 0x000000),
    ("blanchedalmond", 0xffebcd),
    ("blue", 0x0000ff),
    ("blueviolet", 0x8a2be2),
    ("brown", 0xa52a2a),
    ("burlywood", 0xdeb887),
    ("cadetblue", 0x5f9ea0),
    ("chartreuse", 0x7fff00),
    ("chocolate", 0xd2691e),
    ("coral", 0xff7f50),
    ("cornflowerblue", 0x6495ed),
    ("cornsilk", 0xfff8dc),
    ("crimson", 0xdc143c),
    ("cyan", 0x00ffff),
    ("darkblue", 0x00008b),
    ("darkcyan", 0x008b8b),
    ("darkgoldenrod", 0xb8860b),
    ("darkgray", 0xa9a9a9),
    ("darkgreen", 0x006400),
    ("darkgrey", 0xa9a9a9),
    ("darkkhaki", 0xbdb76b),
    ("darkmagenta", 0x8b008b),
    ("darkolivegreen", 0x556b2f),
    ("darkorange", 0xff8c00),
    ("darkorchid", 0x9932cc),
    ("darkred", 0x8b0000),
    ("darksalmon", 0xe9967a),
    ("darkseagreen", 0x8fbc8f),
    ("darkslateblue", 0x483d8b),
    ("darkslategray", 0x2f4f4f),
    ("darkslategrey", 0x2f4f4f),
    ("darkturquoise", 0x00ced1),
    ("darkviolet", 0x9400d3),
    ("deeppink", 0xff1493),
    ("deepskyblue", 0x00bfff),
    ("dimgray", 0x696969),
    ("dimgrey", 0x696969),
    ("dodgerblue", 0x1e90ff),
    ("firebrick", 0xb22222),
    ("floralwhite", 0xfffaf0),
    ("forestgreen", 0x228b22),
    ("fuchsia", 0xff00ff),
    ("gainsboro", 0xdcdcdc),
    ("ghostwhite", 0xf8f8ff),
    ("gold", 0xffd700),
    ("goldenrod", 0xdaa520),
    ("gray", 0x808080),
    ("green", 0x008000),
    ("greenyellow", 0xadff2f),
    ("grey", 0x808080),
    ("honeydew", 0xf0fff0),
    ("hotpink", 0xff69b4),
    ("indianred", 0xcd5c5c),
    ("indigo", 0x4b0082),
    ("ivory", 0xfffff0),
    ("khaki", 0xf0e68c),
    ("lavender", 0xe6e6fa),
    ("lavenderblush", 0xfff0f5),
    ("lawngreen", 0x7cfc00),
    ("lemonchiffon", 0xfffacd),
    ("lightblue", 0xadd8e6),
    ("lightcoral", 0xf08080),
    ("lightcyan", 0xe0ffff),
    ("lightgoldenrodyellow", 0xfafad2),
    ("lightgray", 0xd3d3d3),
    ("lightgreen", 0x90ee90),
    ("lightgrey", 0xd3d3d3),
    ("lightpink", 0xffb6c1),
    ("lightsalmon", 0xffa07a),
    ("lightseagreen", 0x20b2aa),
    ("lightskyblue", 0x87cefa),
    ("lightslategray", 0x778899),
    ("lightslategrey", 0x778899),
    ("lightsteelblue", 0xb0c4de),
    ("lightyellow", 0xffffe0),
    ("lime", 0x00ff00),
    ("limegreen", 0x32cd32),
    ("linen", 0xfaf0e6),
    ("magenta", 0xff00ff),
    ("maroon", 0x800000),
    ("mediumaquamarine", 0x66cdaa),
    ("mediumblue", 0x0000cd),
    ("mediumorchid", 0xba55d3),
    ("mediumpurple", 0x9370db),
    ("mediumseagreen", 0x3cb371),
    ("mediumslateblue", 0x7b68ee),
    ("mediumspringgreen", 0x00fa9a),
    ("mediumturquoise", 0x48d1cc),
    ("mediumvioletred", 0xc71585),
    ("midnightblue", 0x191970),
    ("mintcream", 0xf5fffa),
    ("mistyrose", 0xffe4e1),
    ("moccasin", 0xffe4b5),
    ("navajowhite", 0xffdead),
    ("navy", 0x000080),
    ("oldlace", 0xfdf5e6),
    ("olive", 0x808000),
    ("olivedrab", 0x6b8e23),
    ("orange", 0xffa500),
    ("orangered", 0xff4500),
    ("orchid", 0xda70d6),
    ("palegoldenrod", 0xeee8aa),
    ("palegreen", 0x98fb98),
    ("paleturquoise", 0xafeeee),
    ("palevioletred", 0xdb7093),
    ("papayawhip", 0xffefd5),
    ("peachpuff", 0xffdab9),
    ("peru", 0xcd853f),
    ("pink", 0xffc0cb),
    ("plum", 0xdda0dd),
    ("powderblue", 0xb0e0e6),
    ("purple", 0x800080),
    ("rebeccapurple", 0x663399),
    ("red", 0xff0000),
    ("rosybrown", 0xbc8f8f),
    ("royalblue", 0x4169e1),
    ("saddlebrown", 0x8b4513),
    ("salmon", 0xfa8072),
    ("sandybrown", 0xf4a460),
    ("seagreen", 0x2e8b57),
    ("seashell", 0xfff5ee),
    ("sienna", 0xa0522d),
    ("silver", 0xc0c0c0),
    ("skyblue", 0x87ceeb),
    ("slateblue", 0x6a5acd),
    ("slategray", 0x708090),
    ("slategrey", 0x708090),
    ("snow", 0xfffafa),
    ("springgreen", 0x00ff7f),
    ("steelblue", 0x4682b4),
    ("tan", 0xd2b48c),
    ("teal", 0x008080),
    ("thistle", 0xd8bfd8),
    ("tomato", 0xff6347),
    ("turquoise", 0x40e0d0),
    ("violet", 0xee82ee),
    ("wheat", 0xf5deb3),
    ("white", 0xffffff),
    ("whitesmoke", 0xf5f5f5),
    ("yellow", 0xffff00),
    ("yellowgreen", 0x9acd32),
    ("transparent", 0x000000),
];

/// A colour of a style's text; none for one that does not read.
pub fn parse(text: &str) -> Option<Rgba> {
    let t = text.trim().to_ascii_lowercase();
    if let Some(hex) = t.strip_prefix('#') {
        let digit = |c: u8| (c as char).to_digit(16).map(f64::from);
        let b = hex.as_bytes();
        let two = |i: usize| Some((digit(b[i])? * 16.0 + digit(b[i + 1])?) / 255.0);
        let one = |i: usize| Some(digit(b[i])? * 17.0 / 255.0);
        return match b.len() {
            3 => Some([one(0)?, one(1)?, one(2)?, 1.0]),
            4 => Some([one(0)?, one(1)?, one(2)?, one(3)?]),
            6 => Some([two(0)?, two(2)?, two(4)?, 1.0]),
            8 => Some([two(0)?, two(2)?, two(4)?, two(6)?]),
            _ => None,
        };
    }
    if t == "transparent" {
        return Some([0.0, 0.0, 0.0, 0.0]);
    }
    if let Some(&(_, rgb)) = NAMED.iter().find(|(n, _)| *n == t) {
        return Some([
            f64::from((rgb >> 16) & 255) / 255.0,
            f64::from((rgb >> 8) & 255) / 255.0,
            f64::from(rgb & 255) / 255.0,
            1.0,
        ]);
    }
    let open = t.find('(')?;
    let name = t[..open].trim();
    let inner = t[open + 1..].strip_suffix(')')?;
    let parts: Vec<&str> = inner
        .split([',', ' ', '/'])
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect();
    let num = |p: &str, whole: f64| -> Option<f64> {
        match p.strip_suffix('%') {
            Some(v) => v.parse::<f64>().ok().map(|v| v / 100.0 * whole),
            None => p.parse::<f64>().ok(),
        }
    };
    let alpha = |p: Option<&&str>| -> Option<f64> {
        p.map_or(Some(1.0), |a| num(a, 1.0))
            .map(|a| a.clamp(0.0, 1.0))
    };
    match name {
        "rgb" | "rgba" if parts.len() >= 3 => Some([
            (num(parts[0], 255.0)? / 255.0).clamp(0.0, 1.0),
            (num(parts[1], 255.0)? / 255.0).clamp(0.0, 1.0),
            (num(parts[2], 255.0)? / 255.0).clamp(0.0, 1.0),
            alpha(parts.get(3))?,
        ]),
        "hsl" | "hsla" if parts.len() >= 3 => {
            let h = parts[0].trim_end_matches("deg").parse::<f64>().ok()?;
            let s = num(parts[1], 1.0)?.clamp(0.0, 1.0);
            let l = num(parts[2], 1.0)?.clamp(0.0, 1.0);
            let [r, g, b] = hsl(h, s, l);
            Some([r, g, b, alpha(parts.get(3))?])
        }
        _ => None,
    }
}

/// HSL to RGB (CSS Color 4 §7.1).
fn hsl(h: f64, s: f64, l: f64) -> [f64; 3] {
    let h = h.rem_euclid(360.0) / 360.0;
    let f = |n: f64| {
        let k = (n + h * 12.0) % 12.0;
        let a = s * l.min(1.0 - l);
        l - a * (k - 3.0).min(9.0 - k).clamp(-1.0, 1.0)
    };
    [f(0.0), f(8.0), f(4.0)]
}

/// `#rrggbb` of a colour (its alpha left to an opacity).
pub fn hex(c: Rgba) -> String {
    let b = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("#{:02x}{:02x}{:02x}", b(c[0]), b(c[1]), b(c[2]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_form() {
        assert_eq!(parse("#f00"), Some([1.0, 0.0, 0.0, 1.0]));
        assert_eq!(
            parse("#FF000080").map(|c| (c[3] * 255.0).round()),
            Some(128.0)
        );
        assert_eq!(parse("rgba(255, 0, 0, 0.5)"), Some([1.0, 0.0, 0.0, 0.5]));
        assert_eq!(parse("rgb(100% 0% 0% / 50%)"), Some([1.0, 0.0, 0.0, 0.5]));
        let c = parse("hsl(120, 100%, 50%)").unwrap();
        assert!((c[1] - 1.0).abs() < 1e-12 && c[0].abs() < 1e-12);
        let c = parse("hsla(30, 19%, 90%, 0.4)").unwrap();
        assert_eq!(hex(c), "#eae6e1");
        assert_eq!(
            parse("Gainsboro"),
            Some([220.0 / 255.0, 220.0 / 255.0, 220.0 / 255.0, 1.0])
        );
        assert_eq!(parse("transparent").map(|c| c[3]), Some(0.0));
        assert_eq!(parse("nope"), None);
        assert_eq!(parse("#12345"), None);
    }
}
