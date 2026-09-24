//! The AutoCAD Color Index (ACI): 1–9 named colours, 10–249 hues in five
//! shades and two saturations, 250–255 greys (the values AutoCAD shows).
//! Colour 7 is black on a light sheet and white on a dark one: the app's
//! `ink` token.

/// RGB of an ACI colour 1–255 (0 and 256 are BYBLOCK and BYLAYER, not colours).
pub fn rgb(index: u8) -> [u8; 3] {
    match index {
        1 => [255, 0, 0],
        2 => [255, 255, 0],
        3 => [0, 255, 0],
        4 => [0, 255, 255],
        5 => [0, 0, 255],
        6 => [255, 0, 255],
        7 | 0 => [255, 255, 255],
        8 => [128, 128, 128],
        9 => [192, 192, 192],
        10..=249 => {
            let hue = u32::from((index - 10) / 10);
            let k = (index - 10) % 10;
            // Full-saturation hue at 255, in 24 steps round the wheel.
            let step = |s: u32| -> f64 { f64::from(s) / 4.0 };
            let sextant = hue / 4;
            let f = step(hue % 4);
            let (r, g, b) = match sextant {
                0 => (1.0, f, 0.0),
                1 => (1.0 - f, 1.0, 0.0),
                2 => (0.0, 1.0, f),
                3 => (0.0, 1.0 - f, 1.0),
                4 => (f, 0.0, 1.0),
                _ => (1.0, 0.0, 1.0 - f),
            };
            let value = [255.0, 204.0, 153.0, 127.0, 76.0][usize::from(k / 2)];
            let pastel = k % 2 == 1;
            let ch = |c: f64| -> u8 {
                let x = if pastel {
                    value * (1.0 + c) / 2.0
                } else {
                    value * c
                };
                x.floor().clamp(0.0, 255.0) as u8
            };
            [ch(r), ch(g), ch(b)]
        }
        _ => {
            // 250–255: AutoCAD's greys, from dark to white (not evenly spaced).
            let g = [51, 80, 105, 130, 190, 255][usize::from(index - 250).min(5)];
            [g, g, g]
        }
    }
}

/// The app's colour for an ACI index: `ink` for 7, else "#RRGGBB".
pub fn color(index: u8) -> String {
    if index == 7 {
        return "ink".to_string();
    }
    let [r, g, b] = rgb(index);
    format!("#{r:02X}{g:02X}{b:02X}")
}

/// A true colour (group 420, 0x00RRGGBB) as "#RRGGBB".
pub fn true_color(v: i64) -> String {
    format!("#{:06X}", v & 0xFF_FFFF)
}

/// A colour as a DXF file holds it: the ACI index (group 62) and, for a
/// colour of the app's own, its exact RGB (group 420, AutoCAD 2004 on).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DxfColor {
    pub aci: u8,
    pub rgb: Option<i64>,
}

impl DxfColor {
    /// What the reader makes of it: `ink` for 7, else "#RRGGBB".
    pub fn read_back(self) -> String {
        match self.rgb {
            Some(v) => true_color(v),
            None => color(self.aci),
        }
    }
}

/// "#RGB", "#RRGGBB" or "#RRGGBBAA" as RGB, and whether an alpha was dropped.
fn parse_hex(c: &str) -> Option<([u8; 3], bool)> {
    let h = c.strip_prefix('#')?;
    if !h.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let byte = |s: &str| u8::from_str_radix(s, 16).ok();
    match h.len() {
        3 => {
            let d: Vec<u8> = h.chars().filter_map(|x| byte(&format!("{x}{x}"))).collect();
            Some(([*d.first()?, *d.get(1)?, *d.get(2)?], false))
        }
        6 | 8 => Some((
            [byte(&h[0..2])?, byte(&h[2..4])?, byte(&h[4..6])?],
            h.len() == 8,
        )),
        _ => None,
    }
}

/// How a colour of the app (a hex colour or a theme token) is written, and
/// why when DXF cannot hold it as it is. Theme tokens have no DXF
/// equivalent: `ink` is colour 7 (black on white, white on black) exactly,
/// the others become the nearest fixed colour.
pub fn from_app(c: &str) -> (DxfColor, Option<&'static str>) {
    let aci = |n: u8| DxfColor { aci: n, rgb: None };
    match c {
        "ink" => (aci(7), None),
        "fg" => (
            aci(7),
            Some("“fg” (ana mürekkep) tema rengi DXF'te 7 (siyah/beyaz) oldu"),
        ),
        "fg-dim" => (
            aci(8),
            Some("“fg-dim” (ikincil mürekkep) tema rengi DXF'te 8 (gri) oldu"),
        ),
        "paper" => (
            aci(255),
            Some("“paper” (kâğıt rengi) tema rengi DXF'te 255 (beyaz) oldu"),
        ),
        _ => match parse_hex(c) {
            Some((rgb_, alpha)) => {
                let [r, g, b] = rgb_;
                let v = (i64::from(r) << 16) | (i64::from(g) << 8) | i64::from(b);
                (
                    DxfColor {
                        aci: nearest(rgb_),
                        rgb: Some(v),
                    },
                    alpha.then_some("saydamlığı yazılmadı (DXF renkleri saydamlık taşımaz)"),
                )
            }
            None => (
                aci(7),
                Some("tanınmayan renk 7 (siyah/beyaz) olarak yazıldı"),
            ),
        },
    }
}

/// The ACI index nearest to an RGB colour (exact when the colour is one of them).
pub fn nearest(rgb_: [u8; 3]) -> u8 {
    let mut best = (u32::MAX, 7u8);
    for i in 1..=255u8 {
        let c = rgb(i);
        let d = |a: u8, b: u8| (i32::from(a) - i32::from(b)).unsigned_abs().pow(2);
        let dist = d(c[0], rgb_[0]) + d(c[1], rgb_[1]) + d(c[2], rgb_[2]);
        if dist < best.0 {
            best = (dist, i);
        }
    }
    best.1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_palette_follows_autocad() {
        assert_eq!(rgb(1), [255, 0, 0]);
        assert_eq!(rgb(10), [255, 0, 0]);
        assert_eq!(rgb(11), [255, 127, 127]);
        assert_eq!(rgb(12), [204, 0, 0]);
        assert_eq!(rgb(20), [255, 63, 0]);
        assert_eq!(rgb(21), [255, 159, 127]);
        assert_eq!(rgb(22), [204, 51, 0]);
        assert_eq!(rgb(50), [255, 255, 0]);
        assert_eq!(rgb(90), [0, 255, 0]);
        assert_eq!(rgb(130), [0, 255, 255]);
        assert_eq!(rgb(170), [0, 0, 255]);
        assert_eq!(rgb(210), [255, 0, 255]);
        assert_eq!(rgb(250), [51, 51, 51]);
        assert_eq!(rgb(251), [80, 80, 80]);
        assert_eq!(rgb(252), [105, 105, 105]);
        assert_eq!(rgb(253), [130, 130, 130]);
        assert_eq!(rgb(254), [190, 190, 190]);
        assert_eq!(rgb(255), [255, 255, 255]);
        assert_eq!(color(7), "ink");
        assert_eq!(color(1), "#FF0000");
        assert_eq!(true_color(0x00A0B0C0), "#A0B0C0");
    }

    #[test]
    fn app_colours_are_written_as_the_reader_reads_them_back() {
        let (c, note) = from_app("#7fb2e5");
        assert_eq!((c.rgb, note), (Some(0x7FB2E5), None));
        assert_eq!(c.read_back(), "#7FB2E5");
        assert_eq!(from_app("#F00").0.read_back(), "#FF0000");
        assert_eq!(
            from_app("#FF000080"),
            (
                DxfColor {
                    aci: 1,
                    rgb: Some(0xFF0000)
                },
                Some("saydamlığı yazılmadı (DXF renkleri saydamlık taşımaz)")
            )
        );
        assert_eq!(from_app("ink"), (DxfColor { aci: 7, rgb: None }, None));
        assert_eq!(from_app("ink").0.read_back(), "ink");
        assert_eq!(from_app("fg").0.aci, 7);
        assert_eq!(from_app("fg-dim").0.read_back(), "#808080");
        assert!(
            from_app("fg").1.is_some()
                && from_app("kırmızı").1.is_some()
                && from_app("#12345").1.is_some()
        );
    }

    #[test]
    fn nearest_finds_every_index_of_its_own_colour() {
        for i in [1u8, 3, 5, 8, 9, 30, 41, 142, 251] {
            assert_eq!(rgb(nearest(rgb(i))), rgb(i), "{i}");
        }
        assert_eq!(nearest([250, 5, 5]), 1);
    }
}
