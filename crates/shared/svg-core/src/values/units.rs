//! Transforms, lengths with units and the viewBox mapping with
//! preserveAspectRatio.

use kentos_geometry_core::jsmath::{PI, cos, js_max, js_min, sin, tan};
use kentos_style_core::js::text::{is_space, trim};

use crate::path::{IDENTITY, Matrix, multiply, number_len};

use super::number::{nums, split_ws};

const FUNCS: [&str; 6] = ["matrix", "translate", "scale", "rotate", "skewX", "skewY"];

/// The transform attribute as a matrix (functions applied left to right, as SVG does).
pub fn read_transform(v: Option<&str>) -> Matrix {
    let Some(v) = v.filter(|v| !v.is_empty()) else {
        return IDENTITY;
    };
    let mut m = IDENTITY;
    let mut i = 0;
    // `/(matrix|…|skewY)\s*\(([^)]*)\)/g`: a match anywhere, then on after it.
    while i < v.len() {
        let hit = FUNCS.iter().find_map(|f| {
            let rest = v.get(i..)?.strip_prefix(f)?;
            let rest = rest.trim_start_matches(is_space);
            let inner = rest.strip_prefix('(')?;
            let close = inner.find(')')?;
            let end = v.len() - inner.len() + close + 1;
            Some((*f, &inner[..close], end))
        });
        let Some((f, args, end)) = hit else {
            i += v[i..].chars().next().map_or(1, char::len_utf8);
            continue;
        };
        let a = nums(Some(args));
        let get = |k: usize, d: f64| a.get(k).copied().unwrap_or(d);
        let t: Matrix = match f {
            "matrix" if a.len() == 6 => [a[0], a[1], a[2], a[3], a[4], a[5]],
            "translate" => [1.0, 0.0, 0.0, 1.0, get(0, 0.0), get(1, 0.0)],
            "scale" => [
                get(0, 1.0),
                0.0,
                0.0,
                a.get(1).copied().unwrap_or(get(0, 1.0)),
                0.0,
                0.0,
            ],
            "rotate" => {
                let r = (get(0, 0.0) * PI) / 180.0;
                let c = cos(r);
                let s = sin(r);
                let cx = get(1, 0.0);
                let cy = get(2, 0.0);
                [c, s, -s, c, cx - c * cx + s * cy, cy - s * cx - c * cy]
            }
            "skewX" => [1.0, 0.0, tan((get(0, 0.0) * PI) / 180.0), 1.0, 0.0, 0.0],
            "skewY" => [1.0, tan((get(0, 0.0) * PI) / 180.0), 0.0, 1.0, 0.0, 0.0],
            _ => IDENTITY,
        };
        m = multiply(&m, &t);
        i = end;
    }
    m
}

/// Pixels per unit (CSS: 96 px to the inch).
pub fn px_per(unit: &str) -> Option<f64> {
    Some(match unit {
        "" | "px" => 1.0,
        "mm" => 96.0 / 25.4,
        "cm" => 96.0 / 2.54,
        "in" => 96.0,
        "pt" => 96.0 / 72.0,
        "pc" => 16.0,
        "q" => 96.0 / 101.6,
        _ => return None,
    })
}

const UNITS: [&str; 11] = [
    "px", "mm", "cm", "in", "pt", "pc", "q", "em", "ex", "rem", "%",
];

/// A length: its number and unit (lower case; "" for none).
#[derive(Clone, Debug, PartialEq)]
pub struct Length {
    pub value: f64,
    pub unit: String,
}

kentos_geometry_core::json_struct!(out Length { value, unit });

pub fn parse_length(v: Option<&str>) -> Option<Length> {
    let v = v.unwrap_or("");
    let t = v.trim_start_matches(is_space);
    let n = number_len(t.as_bytes());
    if n == 0 {
        return None;
    }
    let value = t[..n].parse::<f64>().unwrap_or(f64::NAN);
    let rest = t[n..].trim_start_matches(is_space);
    let (unit, rest) = match UNITS.iter().find(|u| {
        rest.len() >= u.len()
            && rest.is_char_boundary(u.len())
            && rest[..u.len()].eq_ignore_ascii_case(u)
    }) {
        Some(u) => (u.to_string(), &rest[u.len()..]),
        None => (String::new(), rest),
    };
    if !rest.chars().all(is_space) {
        return None;
    }
    Some(Length { value, unit })
}

/// A length in user units; % against `percent`, em against `em`.
pub fn to_user(v: Option<&str>, percent: f64, em: f64) -> Option<f64> {
    let l = parse_length(v)?;
    Some(match l.unit.as_str() {
        "%" => (l.value * percent) / 100.0,
        "em" | "rem" => l.value * em,
        "ex" => l.value * em * 0.5,
        u => l.value * px_per(u).unwrap_or(1.0),
    })
}

/// The viewBox → viewport map with preserveAspectRatio (SVG 1.1 §7.8).
pub fn view_box_transform(vb: &[f64], width: f64, height: f64, par: &str) -> Matrix {
    let get = |k: usize| vb.get(k).copied().unwrap_or(f64::NAN);
    let (x, y, w, h) = (get(0), get(1), get(2), get(3));
    let words: Vec<&str> = split_ws(trim(par))
        .into_iter()
        .filter(|t| !t.is_empty() && *t != "defer")
        .collect();
    let align = words.first().copied().unwrap_or("xMidYMid");
    let mut sx = width / w;
    let mut sy = height / h;
    let mut tx = 0.0;
    let mut ty = 0.0;
    if align != "none" {
        let s = if words.get(1) == Some(&"slice") {
            js_max(sx, sy)
        } else {
            js_min(sx, sy)
        };
        sx = s;
        sy = s;
        let ax = if align.contains("xMid") {
            0.5
        } else if align.contains("xMax") {
            1.0
        } else {
            0.0
        };
        let ay = if align.contains("YMid") {
            0.5
        } else if align.contains("YMax") {
            1.0
        } else {
            0.0
        };
        tx = (width - w * s) * ax;
        ty = (height - h * s) * ay;
    }
    [sx, 0.0, 0.0, sy, tx - x * sx, ty - y * sy]
}

pub fn view_box_of(v: Option<&str>) -> Option<Vec<f64>> {
    let a = nums(v);
    (a.len() == 4 && a[2] > 0.0 && a[3] > 0.0).then_some(a)
}
