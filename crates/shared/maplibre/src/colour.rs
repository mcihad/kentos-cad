//! A style's colour text as the reader takes it (docs/adr/0635): `#rgb` … `#rrggbbaa`, `rgb()`, `rgba()`, `hsl()`,
//! `hsla()` and the colour names, each as a colour (`#rrggbb`) and its opacity.

use kentos_formats::sld::colour;
use serde_json::json;

/// A CSS colour as a colour and its opacity.
pub(crate) fn css_in(text: &str) -> Option<(String, f64)> {
    let t = text.trim().to_ascii_lowercase();
    if t.starts_with('#') {
        return colour(&json!(t));
    }
    let named = |rgb: &str, a: f64| Some((rgb.to_owned(), a));
    match t.as_str() {
        "black" => return named("#000000", 1.0),
        "white" => return named("#ffffff", 1.0),
        "red" => return named("#ff0000", 1.0),
        "green" => return named("#008000", 1.0),
        "lime" => return named("#00ff00", 1.0),
        "blue" => return named("#0000ff", 1.0),
        "yellow" => return named("#ffff00", 1.0),
        "cyan" | "aqua" => return named("#00ffff", 1.0),
        "magenta" | "fuchsia" => return named("#ff00ff", 1.0),
        "gray" | "grey" => return named("#808080", 1.0),
        "orange" => return named("#ffa500", 1.0),
        "purple" => return named("#800080", 1.0),
        "pink" => return named("#ffc0cb", 1.0),
        "brown" => return named("#a52a2a", 1.0),
        "transparent" => return named("#000000", 0.0),
        _ => {}
    }
    let (func, args) = t.split_once('(')?;
    let args = args.strip_suffix(')')?;
    let nums: Vec<&str> = args
        .split(|c: char| c == ',' || c == '/' || c.is_whitespace())
        .filter(|s| !s.is_empty())
        .collect();
    let part = |s: &str, full: f64| -> Option<f64> {
        match s.strip_suffix('%') {
            Some(p) => p.parse::<f64>().ok().map(|p| p / 100.0 * full),
            None => s.trim_end_matches("deg").parse::<f64>().ok(),
        }
    };
    let alpha = |i: usize| -> Option<f64> {
        match nums.get(i) {
            Some(s) => part(s, 1.0).map(|a| a.clamp(0.0, 1.0)),
            None => Some(1.0),
        }
    };
    match func {
        "rgb" | "rgba" if nums.len() >= 3 => {
            let c = |i: usize| part(nums[i], 255.0).map(|x| x.round().clamp(0.0, 255.0) as u8);
            Some((
                format!("#{:02x}{:02x}{:02x}", c(0)?, c(1)?, c(2)?),
                alpha(3)?,
            ))
        }
        "hsl" | "hsla" if nums.len() >= 3 => {
            let h = part(nums[0], 360.0)?.rem_euclid(360.0) / 360.0;
            let s = part(nums[1], 1.0)?.clamp(0.0, 1.0);
            let l = part(nums[2], 1.0)?.clamp(0.0, 1.0);
            let q = if l < 0.5 {
                l * (1.0 + s)
            } else {
                l + s - l * s
            };
            let p = 2.0 * l - q;
            let channel = |mut t: f64| {
                if t < 0.0 {
                    t += 1.0;
                }
                if t > 1.0 {
                    t -= 1.0;
                }
                let v = if t < 1.0 / 6.0 {
                    p + (q - p) * 6.0 * t
                } else if t < 0.5 {
                    q
                } else if t < 2.0 / 3.0 {
                    p + (q - p) * (2.0 / 3.0 - t) * 6.0
                } else {
                    p
                };
                (v * 255.0).round().clamp(0.0, 255.0) as u8
            };
            Some((
                format!(
                    "#{:02x}{:02x}{:02x}",
                    channel(h + 1.0 / 3.0),
                    channel(h),
                    channel(h - 1.0 / 3.0)
                ),
                alpha(3)?,
            ))
        }
        _ => None,
    }
}
