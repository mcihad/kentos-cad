//! What the three style exchanges (SLD, QML, MapLibre; docs/adr/0516, 0555, 0560, 0571) share for the symbol layers
//! of ADR 0571: base64 for the SVG drawings that travel inside a style file, the id an SVG drawing taken from a file
//! gets in the project's library, and the walk that puts a symbol's SVG drawings (library assets, by id) into the
//! symbol before it is written.

use serde_json::Value;

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Standard base64 with padding.
pub(crate) fn b64_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = (u32::from(chunk[0]) << 16)
            | (u32::from(chunk.get(1).copied().unwrap_or(0)) << 8)
            | u32::from(chunk.get(2).copied().unwrap_or(0));
        out.push(B64[(n >> 18) as usize & 63] as char);
        out.push(B64[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            B64[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            B64[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

/// Base64 (padded or not, white space ignored); none when it is not base64.
pub(crate) fn b64_decode(text: &str) -> Option<Vec<u8>> {
    let mut buf = 0u32;
    let mut bits = 0u32;
    let mut out = Vec::with_capacity(text.len() * 3 / 4);
    for c in text.bytes() {
        if c.is_ascii_whitespace() || c == b'=' {
            continue;
        }
        let v = B64.iter().position(|b| *b == c)? as u32;
        buf = (buf << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }
    Some(out)
}

/// The library id of an SVG drawing a style file carried: a prefix naming the format and 64 bits (FNV-1a) of its text,
/// so the same drawing read twice is one item.
pub(crate) fn asset_id(prefix: &str, svg: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in svg.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{prefix}-{h:016x}")
}

/// An SVG text that is one (`<svg` followed by a blank or `>`): what a library drawing must be.
pub(crate) fn looks_like_svg(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    let mut from = 0;
    while let Some(k) = lower[from..].find("<svg") {
        let at = from + k + 4;
        if lower[at..]
            .chars()
            .next()
            .is_some_and(|c| c == '>' || c.is_whitespace())
        {
            return true;
        }
        from = at;
    }
    false
}

/// The symbol with each SVG marker layer's drawing put in as `svgData` (library assets by their `asset` id), at any
/// depth (a marker line's marker, a pattern's, a centroid's). A layer whose asset the library lacks keeps no drawing.
pub fn inline_assets(value: &mut Value, assets: &dyn Fn(&str) -> Option<String>) {
    match value {
        Value::Object(map) => {
            if map.get("type").and_then(Value::as_str) == Some("svg")
                && let Some(id) = map.get("asset").and_then(Value::as_str)
                && let Some(svg) = assets(id)
            {
                map.insert("svgData".into(), Value::String(svg));
            }
            for v in map.values_mut() {
                inline_assets(v, assets);
            }
        }
        Value::Array(items) => {
            for v in items {
                inline_assets(v, assets);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_goes_both_ways() {
        for s in ["", "f", "fo", "foo", "foob", "fooba", "foobar", "<svg/>"] {
            let e = b64_encode(s.as_bytes());
            assert_eq!(b64_decode(&e).as_deref(), Some(s.as_bytes()), "{s}");
        }
        assert_eq!(b64_encode(b"foobar"), "Zm9vYmFy");
        assert_eq!(b64_encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(b64_decode("Zm9v\nYmFy").as_deref(), Some(&b"foobar"[..]));
        assert_eq!(b64_decode("Zm9v!"), None);
    }

    #[test]
    fn a_drawing_gets_one_id_and_is_found_by_its_tag() {
        assert_eq!(asset_id("qml", "<svg/>"), asset_id("qml", "<svg/>"));
        assert_ne!(asset_id("qml", "<svg/>"), asset_id("qml", "<svg />"));
        assert!(looks_like_svg("<?xml?><SVG xmlns='x'/>"));
        assert!(!looks_like_svg("<svgx/>"));
    }
}
