//! Conversions (docs/adr/0600 §3): `mantıksal`, `ondalık_derece`, `derece_dakika_saniye`, `derece_dakika`, Base64 and
//! the digests. The degrees, minutes and seconds follow QGIS's `QgsCoordinateFormatter` (a longitude wrapped to
//! ±180, a latitude to ±90, rounding that reaches 60 carried up, a zero without sign or hemisphere), with the language's
//! decimal dot.

use std::fmt::Write as _;

use kentos_geometry_core::jsmath::{js_max, js_round};
use sha1::Digest as _;

use crate::compound::shown;
use crate::js::number;
use crate::js::text::{fold_turkish, trim};
use crate::library::Func;
use crate::scalar::{R, Scratch, V, as_text, to_number};

const TRUE_WORDS: [&str; 8] = ["DOGRU", "EVET", "TRUE", "YES", "ON", "1", "E", "Y"];
const FALSE_WORDS: [&str; 9] = ["YANLIS", "HAYIR", "FALSE", "NO", "OFF", "0", "", "H", "N"];

fn to_bool(v: V) -> V<'static> {
    match v {
        V::Null => V::Null,
        V::Bool(b) => V::Bool(b),
        V::Num(x) => V::Bool(x != 0.0 && !x.is_nan()),
        V::Text(t) => {
            let key = fold_turkish(trim(shown(t)));
            if TRUE_WORDS.contains(&key.as_str()) {
                V::Bool(true)
            } else if FALSE_WORDS.contains(&key.as_str()) {
                V::Bool(false)
            } else if let Some(x) = crate::read::number(&key) {
                V::Bool(x != 0.0)
            } else {
                V::Bool(true)
            }
        }
    }
}

/// An angle written as degrees, minutes and seconds, in decimal degrees.
fn to_decimal(t: &str) -> Option<f64> {
    let mut negative = false;
    let mut numbers: Vec<f64> = Vec::with_capacity(3);
    let mut current = String::new();
    let flush = |current: &mut String, numbers: &mut Vec<f64>| -> Option<()> {
        if !current.is_empty() {
            let text = current.replace(',', ".");
            numbers.push(text.parse::<f64>().ok()?);
            current.clear();
        }
        Some(())
    };
    for c in t.trim().chars() {
        match c {
            '0'..='9' | '.' | ',' => current.push(c),
            '-' | '−' if numbers.is_empty() && current.is_empty() => negative = true,
            'S' | 's' | 'W' | 'w' | 'G' | 'g' | 'B' | 'b' => {
                flush(&mut current, &mut numbers)?;
                negative = true;
            }
            'N' | 'n' | 'E' | 'e' | 'K' | 'k' | 'D' | 'd' => flush(&mut current, &mut numbers)?,
            '+' if numbers.is_empty() && current.is_empty() => {}
            _ => flush(&mut current, &mut numbers)?,
        }
    }
    flush(&mut current, &mut numbers)?;
    if numbers.is_empty() || numbers.len() > 3 || numbers[1..].iter().any(|x| *x >= 60.0) {
        return None;
    }
    let d = numbers[0]
        + numbers.get(1).map_or(0.0, |m| m / 60.0)
        + numbers.get(2).map_or(0.0, |s| s / 3600.0);
    Some(if negative { -d } else { d })
}

/// How `derece_dakika_saniye` and `derece_dakika` write the hemisphere and pad.
#[derive(Clone, Copy, PartialEq)]
enum Style {
    Sign,
    /// E/W, N/S.
    Suffix,
    /// D/B, K/G.
    Turkish,
    /// E/W, N/S and minutes and seconds of two digits (QGIS's `aligned`).
    Aligned,
}

fn style_of(v: V, buf: &mut String) -> Option<Style> {
    Some(match fold_turkish(trim(as_text(v, buf))).as_str() {
        "" => Style::Sign,
        "SUFFIX" => Style::Suffix,
        "SONEK" => Style::Turkish,
        "ALIGNED" => Style::Aligned,
        _ => return None,
    })
}

/// The angle wrapped as QGIS wraps a longitude (`x`) or a latitude (`y`).
fn wrapped(v: f64, x_axis: bool) -> f64 {
    let (full, half) = if x_axis {
        (360.0, 180.0)
    } else {
        (180.0, 90.0)
    };
    let w = v % full;
    if w > half {
        w - full
    } else if w < -half {
        w + full
    } else {
        w
    }
}

fn hemisphere(w: f64, x_axis: bool, style: Style) -> &'static str {
    match (style, x_axis, w < 0.0) {
        (Style::Suffix | Style::Aligned, true, true) => "W",
        (Style::Suffix | Style::Aligned, true, false) => "E",
        (Style::Suffix | Style::Aligned, false, true) => "S",
        (Style::Suffix | Style::Aligned, false, false) => "N",
        (Style::Turkish, true, true) => "B",
        (Style::Turkish, true, false) => "D",
        (Style::Turkish, false, true) => "G",
        (Style::Turkish, false, false) => "K",
        _ => "",
    }
}

/// A non-negative number with `p` decimals, padded to `width` with zeros when aligned.
fn push_padded(out: &mut String, x: f64, p: u32, aligned: bool) {
    let mark = out.len();
    number::push_fixed(out, x, p);
    if aligned {
        let width = 2 + if p == 0 { 0 } else { 1 + p as usize };
        let len = out.len() - mark;
        if len < width {
            out.insert_str(mark, &"0".repeat(width - len));
        }
    }
}

fn dms(out: &mut String, v: f64, x_axis: bool, p: u32, style: Style, seconds: bool) {
    let w = wrapped(v, x_axis);
    let k = crate::functions::scale(f64::from(p));
    let a = w.abs();
    let mut deg = a.trunc();
    let mut minutes = (a - deg) * 60.0;
    let mut whole_min = minutes.trunc();
    let mut secs = (minutes - whole_min) * 60.0;
    if seconds {
        if js_round(secs * k) >= 60.0 * k {
            secs = js_max(secs - 60.0, 0.0);
            whole_min += 1.0;
            if whole_min >= 60.0 {
                whole_min -= 60.0;
                deg += 1.0;
            }
        }
    } else if js_round(minutes * k) >= 60.0 * k {
        minutes = js_max(minutes - 60.0, 0.0);
        deg += 1.0;
    }
    let zero = if seconds {
        deg == 0.0 && whole_min == 0.0 && js_round(secs * k) == 0.0
    } else {
        deg == 0.0 && js_round(minutes * k) == 0.0
    };
    let mut hemi = if zero {
        ""
    } else {
        hemisphere(w, x_axis, style)
    };
    if x_axis && seconds && deg == 180.0 && whole_min == 0.0 && js_round(secs * k) == 0.0 {
        hemi = "";
    }
    if style == Style::Sign && w < 0.0 && !zero {
        out.push('-');
    }
    let aligned = style == Style::Aligned;
    let _ = write!(out, "{deg}°");
    if seconds {
        if aligned {
            let _ = write!(out, "{whole_min:02}′");
        } else {
            let _ = write!(out, "{whole_min}′");
        }
        push_padded(out, secs, p, aligned);
        out.push('″');
    } else {
        push_padded(out, minutes, p, aligned);
        out.push('′');
    }
    out.push_str(hemi);
}

const BASE64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn push_base64(out: &mut String, bytes: &[u8]) {
    for chunk in bytes.chunks(3) {
        let n = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(BASE64[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
}

fn from_base64(t: &str) -> Option<Vec<u8>> {
    let t: Vec<u8> = t.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    let pad = t.iter().rev().take_while(|&&b| b == b'=').count();
    if pad > 2 || (pad > 0 && !t.len().is_multiple_of(4)) {
        return None;
    }
    let body = &t[..t.len() - pad];
    if body.len() % 4 == 1 {
        return None;
    }
    let mut out = Vec::with_capacity(body.len() * 3 / 4);
    let (mut acc, mut bits) = (0u32, 0u32);
    for &b in body {
        let v = BASE64.iter().position(|&c| c == b)? as u32;
        acc = (acc << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    Some(out)
}

fn push_hex(out: &mut String, bytes: &[u8]) {
    for b in bytes {
        let _ = write!(out, "{b:02x}");
    }
}

/// MD5 (RFC 1321).
fn md5(data: &[u8]) -> [u8; 16] {
    const S: [u32; 64] = [
        7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5,
        9, 14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10,
        15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
    ];
    const K: [u32; 64] = [
        0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a, 0xa8304613,
        0xfd469501, 0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be, 0x6b901122, 0xfd987193,
        0xa679438e, 0x49b40821, 0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa, 0xd62f105d,
        0x02441453, 0xd8a1e681, 0xe7d3fbc8, 0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed,
        0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a, 0xfffa3942, 0x8771f681, 0x6d9d6122,
        0xfde5380c, 0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70, 0x289b7ec6, 0xeaa127fa,
        0xd4ef3085, 0x04881d05, 0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665, 0xf4292244,
        0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
        0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1, 0xf7537e82, 0xbd3af235, 0x2ad7d2bb,
        0xeb86d391,
    ];
    let mut msg = data.to_vec();
    let bit_len = (data.len() as u64).wrapping_mul(8);
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_le_bytes());
    let (mut a0, mut b0, mut c0, mut d0) = (
        0x6745_2301u32,
        0xEFCD_AB89u32,
        0x98BA_DCFEu32,
        0x1032_5476u32,
    );
    for block in msg.chunks(64) {
        let m: Vec<u32> = block
            .chunks(4)
            .map(|w| u32::from_le_bytes([w[0], w[1], w[2], w[3]]))
            .collect();
        let (mut a, mut b, mut c, mut d) = (a0, b0, c0, d0);
        for i in 0..64 {
            let (f, g) = match i / 16 {
                0 => ((b & c) | (!b & d), i),
                1 => ((d & b) | (!d & c), (5 * i + 1) % 16),
                2 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | !d), (7 * i) % 16),
            };
            let f = f.wrapping_add(a).wrapping_add(K[i]).wrapping_add(m[g]);
            a = d;
            d = c;
            c = b;
            b = b.wrapping_add(f.rotate_left(S[i]));
        }
        a0 = a0.wrapping_add(a);
        b0 = b0.wrapping_add(b);
        c0 = c0.wrapping_add(c);
        d0 = d0.wrapping_add(d);
    }
    let mut out = [0u8; 16];
    for (i, w) in [a0, b0, c0, d0].iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&w.to_le_bytes());
    }
    out
}

fn digest(method: &str, data: &[u8], out: &mut String) -> bool {
    match fold_turkish(trim(method)).as_str() {
        "MD5" => push_hex(out, &md5(data)),
        "SHA1" => push_hex(out, &sha1::Sha1::digest(data)),
        "SHA224" => push_hex(out, &sha2::Sha224::digest(data)),
        "SHA256" => push_hex(out, &sha2::Sha256::digest(data)),
        "SHA384" => push_hex(out, &sha2::Sha384::digest(data)),
        "SHA512" => push_hex(out, &sha2::Sha512::digest(data)),
        _ => return false,
    }
    true
}

pub(super) fn call<'a>(f: Func, args: &[V<'a>], out: &mut String, s: &mut Scratch) -> R<'a> {
    let arg = |i: usize| super::arg(args, i);
    if f != Func::ToBool && arg(0) == V::Null {
        return R::V(V::Null);
    }
    match f {
        Func::ToBool => R::V(to_bool(arg(0))),
        Func::ToDecimal => {
            let t = as_text(arg(0), &mut s.a);
            R::V(to_decimal(shown(t)).map_or(V::Null, V::Num))
        }
        Func::ToDms | Func::ToDm => {
            let (Some(v), Some(p)) = (to_number(arg(0)), to_number(arg(2))) else {
                return R::V(V::Null);
            };
            let axis = fold_turkish(trim(as_text(arg(1), &mut s.a)));
            let x_axis = match axis.as_str() {
                "X" => true,
                "Y" => false,
                _ => return R::V(V::Null),
            };
            let Some(style) = style_of(arg(3), &mut s.b) else {
                return R::V(V::Null);
            };
            dms(
                out,
                v,
                x_axis,
                p.clamp(0.0, 12.0).trunc() as u32,
                style,
                f == Func::ToDms,
            );
            R::Made
        }
        Func::ToBase64 => {
            let t = shown(as_text(arg(0), &mut s.a)).to_owned();
            push_base64(out, t.as_bytes());
            R::Made
        }
        Func::FromBase64 => {
            let bytes = from_base64(as_text(arg(0), &mut s.a));
            match bytes.and_then(|b| String::from_utf8(b).ok()) {
                Some(t) => {
                    out.push_str(&t);
                    super::made(out, out.len() - t.len())
                }
                None => R::V(V::Null),
            }
        }
        Func::Hash | Func::Md5 | Func::Sha256 => {
            let data = shown(as_text(arg(0), &mut s.a)).to_owned();
            let method = match f {
                Func::Md5 => "md5".to_owned(),
                Func::Sha256 => "sha256".to_owned(),
                _ => as_text(arg(1), &mut s.b).to_owned(),
            };
            if digest(&method, data.as_bytes(), out) {
                R::Made
            } else {
                R::V(V::Null)
            }
        }
        _ => R::V(V::Null),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn md5_is_rfc_1321s() {
        let mut out = String::new();
        push_hex(&mut out, &md5(b""));
        assert_eq!(out, "d41d8cd98f00b204e9800998ecf8427e");
        out.clear();
        push_hex(
            &mut out,
            &md5(b"The quick brown fox jumps over the lazy dog"),
        );
        assert_eq!(out, "9e107d9d372bb6826bd81d3542a419d6");
    }

    #[test]
    fn base64_goes_both_ways() {
        let mut out = String::new();
        push_base64(&mut out, "Çorum".as_bytes());
        assert_eq!(from_base64(&out).as_deref(), Some("Çorum".as_bytes()));
        assert_eq!(
            from_base64("S2VudE9T").as_deref(),
            Some(b"KentOS".as_slice())
        );
        assert_eq!(from_base64("!!"), None);
    }
}
