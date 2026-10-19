//! A stream's filters undone (ISO 32000-1 §7.4): Flate (with the TIFF and PNG predictors), LZW, ASCIIHex, ASCII85
//! and RunLength, in the order the stream names them. The picture filters (DCT, JPX, CCITT, JBIG2) are not a
//! content stream's: an underlay leaves pictures out.

use super::object::{Dict, Obj, is_white};

/// The most bytes one stream decodes to.
pub const MAX_DECODED: usize = 256 * 1024 * 1024;

/// Why a stream could not be read.
#[derive(Debug, PartialEq)]
pub enum FilterError {
    /// A filter of pictures, not undone here.
    Picture(String),
    Broken(String),
}

fn names(o: Option<&Obj>) -> Vec<String> {
    match o {
        Some(Obj::Name(n)) => vec![n.clone()],
        Some(Obj::Array(a)) => a.iter().filter_map(|x| x.name().map(str::to_owned)).collect(),
        _ => Vec::new(),
    }
}

fn parms(o: Option<&Obj>, i: usize) -> Option<&Dict> {
    match o {
        Some(Obj::Dict(d)) if i == 0 => Some(d),
        Some(Obj::Array(a)) => a.get(i).and_then(Obj::dict),
        _ => None,
    }
}

/// The stream's bytes decoded. `dict`'s `Filter` and `DecodeParms` must already be resolved (no references).
pub fn decode(dict: &Dict, raw: &[u8]) -> Result<Vec<u8>, FilterError> {
    let filters = names(dict.get("Filter").or_else(|| dict.get("F")));
    let dp = dict.get("DecodeParms").or_else(|| dict.get("DP"));
    let mut data = raw.to_vec();
    for (i, f) in filters.iter().enumerate() {
        let p = parms(dp, i);
        data = match f.as_str() {
            "FlateDecode" | "Fl" => predict(inflate(&data)?, p)?,
            "LZWDecode" | "LZW" => {
                let early = p.and_then(|d| d.get("EarlyChange")).and_then(Obj::num).unwrap_or(1.0) != 0.0;
                predict(lzw(&data, early)?, p)?
            }
            "ASCIIHexDecode" | "AHx" => ascii_hex(&data),
            "ASCII85Decode" | "A85" => ascii85(&data)?,
            "RunLengthDecode" | "RL" => run_length(&data),
            "Crypt" => data,
            other => return Err(FilterError::Picture(other.to_owned())),
        };
        if data.len() > MAX_DECODED {
            return Err(FilterError::Broken("akış çok büyük".into()));
        }
    }
    Ok(data)
}

fn inflate(data: &[u8]) -> Result<Vec<u8>, FilterError> {
    match miniz_oxide::inflate::decompress_to_vec_zlib_with_limit(data, MAX_DECODED) {
        Ok(v) => Ok(v),
        // A stream cut short or with a broken check sum still gives what it holds.
        Err(e) if !e.output.is_empty() => Ok(e.output),
        Err(_) => miniz_oxide::inflate::decompress_to_vec_with_limit(data, MAX_DECODED)
            .map_err(|_| FilterError::Broken("Flate sıkıştırması açılamadı".into())),
    }
}

fn predict(data: Vec<u8>, p: Option<&Dict>) -> Result<Vec<u8>, FilterError> {
    let Some(p) = p else { return Ok(data) };
    let predictor = p.get("Predictor").and_then(Obj::num).unwrap_or(1.0) as i64;
    if predictor <= 1 {
        return Ok(data);
    }
    let colors = p.get("Colors").and_then(Obj::num).unwrap_or(1.0).max(1.0) as usize;
    let bits = p.get("BitsPerComponent").and_then(Obj::num).unwrap_or(8.0).max(1.0) as usize;
    let columns = p.get("Columns").and_then(Obj::num).unwrap_or(1.0).max(1.0) as usize;
    let bpp = (colors * bits).div_ceil(8).max(1);
    let row = (colors * bits * columns).div_ceil(8);
    if predictor == 2 {
        // TIFF: each byte the difference from the one a pixel before (8 bits only).
        let mut out = data;
        if bits == 8 {
            for r in out.chunks_mut(row.max(1)) {
                for i in bpp..r.len() {
                    r[i] = r[i].wrapping_add(r[i - bpp]);
                }
            }
        }
        return Ok(out);
    }
    // PNG: a filter byte before every row.
    let mut out = Vec::with_capacity(data.len());
    let mut prev = vec![0u8; row];
    for chunk in data.chunks(row + 1) {
        if chunk.len() < 2 {
            break;
        }
        let kind = chunk[0];
        let mut cur = chunk[1..].to_vec();
        cur.resize(row, 0);
        for i in 0..row {
            let a = if i >= bpp { cur[i - bpp] } else { 0 };
            let b = prev[i];
            let c = if i >= bpp { prev[i - bpp] } else { 0 };
            cur[i] = match kind {
                0 => cur[i],
                1 => cur[i].wrapping_add(a),
                2 => cur[i].wrapping_add(b),
                3 => cur[i].wrapping_add(((u16::from(a) + u16::from(b)) / 2) as u8),
                4 => {
                    let pa = (i16::from(b) - i16::from(c)).abs();
                    let pb = (i16::from(a) - i16::from(c)).abs();
                    let pc = (i16::from(a) + i16::from(b) - 2 * i16::from(c)).abs();
                    let pred = if pa <= pb && pa <= pc {
                        a
                    } else if pb <= pc {
                        b
                    } else {
                        c
                    };
                    cur[i].wrapping_add(pred)
                }
                _ => return Err(FilterError::Broken(format!("PNG öngörücüsünün süzgeci {kind}"))),
            };
        }
        out.extend_from_slice(&cur);
        prev = cur;
    }
    Ok(out)
}

fn lzw(data: &[u8], early: bool) -> Result<Vec<u8>, FilterError> {
    let mut out = Vec::new();
    let mut table: Vec<Vec<u8>> = (0..256).map(|i| vec![i as u8]).collect();
    table.push(Vec::new());
    table.push(Vec::new());
    let mut width = 9usize;
    let mut prev: Option<Vec<u8>> = None;
    let (mut acc, mut nbits) = (0u32, 0usize);
    for &b in data {
        acc = acc << 8 | u32::from(b);
        nbits += 8;
        while nbits >= width {
            let code = ((acc >> (nbits - width)) & ((1 << width) - 1)) as usize;
            nbits -= width;
            if code == 256 {
                table.truncate(258);
                width = 9;
                prev = None;
                continue;
            }
            if code == 257 {
                return Ok(out);
            }
            let entry = if code < table.len() {
                table[code].clone()
            } else if let Some(p) = &prev {
                let mut e = p.clone();
                e.push(p[0]);
                e
            } else {
                return Err(FilterError::Broken("LZW kodu tabloda yok".into()));
            };
            out.extend_from_slice(&entry);
            if out.len() > MAX_DECODED {
                return Err(FilterError::Broken("LZW akışı çok büyük".into()));
            }
            if let Some(p) = prev {
                let mut e = p;
                e.push(entry[0]);
                table.push(e);
            }
            prev = Some(entry);
            let size = table.len() + usize::from(early);
            width = if size >= 2048 {
                12
            } else if size >= 1024 {
                11
            } else if size >= 512 {
                10
            } else {
                9
            };
            if table.len() >= 4096 {
                width = 12;
            }
        }
    }
    Ok(out)
}

fn ascii_hex(data: &[u8]) -> Vec<u8> {
    let mut digits = Vec::new();
    for &b in data {
        match b {
            b'>' => break,
            b'0'..=b'9' => digits.push(b - b'0'),
            b'a'..=b'f' => digits.push(b - b'a' + 10),
            b'A'..=b'F' => digits.push(b - b'A' + 10),
            _ => {}
        }
    }
    if digits.len() % 2 == 1 {
        digits.push(0);
    }
    digits.chunks(2).map(|c| c[0] << 4 | c[1]).collect()
}

fn ascii85(data: &[u8]) -> Result<Vec<u8>, FilterError> {
    let mut out = Vec::new();
    let mut group = Vec::with_capacity(5);
    let mut i = 0;
    if data.starts_with(b"<~") {
        i = 2;
    }
    while i < data.len() {
        let b = data[i];
        i += 1;
        if b == b'~' {
            break;
        }
        if is_white(b) {
            continue;
        }
        if b == b'z' && group.is_empty() {
            out.extend_from_slice(&[0, 0, 0, 0]);
            continue;
        }
        if !(b'!'..=b'u').contains(&b) {
            return Err(FilterError::Broken(format!("ASCII85'te ‘{}’", b as char)));
        }
        group.push(u32::from(b - b'!'));
        if group.len() == 5 {
            let v = group.iter().fold(0u64, |a, &d| a * 85 + u64::from(d));
            out.extend_from_slice(&(v as u32).to_be_bytes());
            group.clear();
        }
    }
    if !group.is_empty() {
        let n = group.len();
        while group.len() < 5 {
            group.push(84);
        }
        let v = group.iter().fold(0u64, |a, &d| a * 85 + u64::from(d));
        out.extend_from_slice(&(v as u32).to_be_bytes()[..n - 1]);
    }
    Ok(out)
}

fn run_length(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < data.len() {
        let n = data[i];
        i += 1;
        match n {
            128 => break,
            0..=127 => {
                let end = (i + usize::from(n) + 1).min(data.len());
                out.extend_from_slice(&data[i..end]);
                i = end;
            }
            _ => {
                if let Some(&b) = data.get(i) {
                    out.extend(std::iter::repeat_n(b, 257 - usize::from(n)));
                }
                i += 1;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_text_filters() {
        assert_eq!(ascii_hex(b"48 65 6c6C6f>"), b"Hello");
        assert_eq!(ascii85(b"<~87cURD]i,\"Ebo80~>").unwrap(), b"Hello World!");
        assert_eq!(run_length(&[2, b'a', b'b', b'c', 254, b'x', 128]), b"abcxxx");
    }
}
