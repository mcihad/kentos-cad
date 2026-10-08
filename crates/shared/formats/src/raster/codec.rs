//! A TIFF block's bytes made plain (docs/adr/0204 §1): LZW (TIFF's, most
//! significant bit first, the code width growing one code early), PackBits
//! and Deflate (zlib, else raw); the horizontal (2) and floating point (3)
//! predictors undone; a JPEG-compressed block's stream made whole with the
//! file's tables for the host's decoder.

use super::RasterError;

/// LZW-coded bytes made plain; at most `expected` bytes are kept (a block
/// holds no more), and a stream that ends early gives what it had.
pub fn lzw(data: &[u8], expected: usize) -> Result<Vec<u8>, RasterError> {
    const CLEAR: usize = 256;
    const EOI: usize = 257;
    const MAX: usize = 4096;
    let mut out: Vec<u8> = Vec::with_capacity(expected);
    let mut prefix = vec![0u16; MAX];
    let mut suffix = vec![0u8; MAX];
    let mut first = vec![0u8; MAX];
    let mut length = vec![0u32; MAX];
    for i in 0..256 {
        suffix[i] = i as u8;
        first[i] = i as u8;
        length[i] = 1;
    }
    let bad = || RasterError::new("LZW sıkıştırması bozuk: tabloda olmayan bir kod.");
    let mut next = 258usize;
    let mut width = 9usize;
    let mut old: Option<usize> = None;
    // Bits are taken most significant first through a 64-bit accumulator.
    let mut acc: u64 = 0;
    let mut have = 0u32;
    let mut at = 0usize;
    // Writes the string of `code` at the end of `out`.
    let emit = |out: &mut Vec<u8>, code: usize, prefix: &[u16], suffix: &[u8], length: &[u32]| {
        let n = length[code] as usize;
        let start = out.len();
        out.resize(start + n, 0);
        let mut c = code;
        for k in (0..n).rev() {
            out[start + k] = suffix[c];
            if k > 0 {
                c = usize::from(prefix[c]);
            }
        }
    };
    while out.len() < expected {
        while have < width as u32 && at < data.len() {
            acc = (acc << 8) | u64::from(data[at]);
            at += 1;
            have += 8;
        }
        if have < width as u32 {
            break;
        }
        have -= width as u32;
        let code = ((acc >> have) & ((1u64 << width) - 1)) as usize;
        if code == EOI {
            break;
        }
        if code == CLEAR {
            next = 258;
            width = 9;
            old = None;
            continue;
        }
        match old {
            None => {
                if code > 255 {
                    return Err(bad());
                }
                out.push(code as u8);
            }
            Some(o) => {
                let head = if code < next {
                    if code >= 258 && length[code] == 0 {
                        return Err(bad());
                    }
                    emit(&mut out, code, &prefix, &suffix, &length);
                    first[code]
                } else if code == next {
                    let f = first[o];
                    emit(&mut out, o, &prefix, &suffix, &length);
                    out.push(f);
                    f
                } else {
                    return Err(bad());
                };
                if next < MAX {
                    prefix[next] = o as u16;
                    suffix[next] = head;
                    first[next] = first[o];
                    length[next] = length[o] + 1;
                    next += 1;
                }
                if next + 1 >= (1 << width) && width < 12 {
                    width += 1;
                }
            }
        }
        old = Some(code);
    }
    out.truncate(expected);
    Ok(out)
}

/// PackBits-coded bytes made plain (at most `expected`).
pub fn packbits(data: &[u8], expected: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(expected);
    let mut i = 0;
    while i < data.len() && out.len() < expected {
        let n = data[i] as i8;
        i += 1;
        if n >= 0 {
            let run = n as usize + 1;
            let end = (i + run).min(data.len());
            out.extend_from_slice(&data[i..end]);
            i = end;
        } else if n != -128 {
            let run = 1 + (-(n as i32)) as usize;
            if let Some(&b) = data.get(i) {
                out.extend(std::iter::repeat_n(b, run));
            }
            i += 1;
        }
    }
    out.truncate(expected);
    out
}

/// Deflate-coded bytes made plain: a zlib stream, else raw Deflate.
pub fn inflate(data: &[u8], expected: usize) -> Result<Vec<u8>, RasterError> {
    let limit = expected.max(1);
    match miniz_oxide::inflate::decompress_to_vec_zlib_with_limit(data, limit) {
        Ok(v) => Ok(v),
        Err(e) if !e.output.is_empty() => Ok(e.output),
        Err(_) => miniz_oxide::inflate::decompress_to_vec_with_limit(data, limit)
            .map_err(|_| RasterError::new("Deflate sıkıştırması bozuk: blok açılamadı.")),
    }
}

/// Undoes the horizontal predictor (2) in place: `row` samples of `bytes`
/// bytes each a row (`spp` samples a pixel), `little` endian.
pub fn unpredict_horizontal(
    buf: &mut [u8],
    row_samples: usize,
    spp: usize,
    bytes: usize,
    little: bool,
) {
    if row_samples == 0 || spp == 0 {
        return;
    }
    let row_bytes = row_samples * bytes;
    for row in buf.chunks_exact_mut(row_bytes) {
        match bytes {
            1 => {
                for i in spp..row_samples {
                    row[i] = row[i].wrapping_add(row[i - spp]);
                }
            }
            2 => {
                let get = |r: &[u8], i: usize| {
                    let a = [r[2 * i], r[2 * i + 1]];
                    if little {
                        u16::from_le_bytes(a)
                    } else {
                        u16::from_be_bytes(a)
                    }
                };
                for i in spp..row_samples {
                    let v = get(row, i).wrapping_add(get(row, i - spp));
                    let b = if little {
                        v.to_le_bytes()
                    } else {
                        v.to_be_bytes()
                    };
                    row[2 * i..2 * i + 2].copy_from_slice(&b);
                }
            }
            4 => {
                let get = |r: &[u8], i: usize| {
                    let a = [r[4 * i], r[4 * i + 1], r[4 * i + 2], r[4 * i + 3]];
                    if little {
                        u32::from_le_bytes(a)
                    } else {
                        u32::from_be_bytes(a)
                    }
                };
                for i in spp..row_samples {
                    let v = get(row, i).wrapping_add(get(row, i - spp));
                    let b = if little {
                        v.to_le_bytes()
                    } else {
                        v.to_be_bytes()
                    };
                    row[4 * i..4 * i + 4].copy_from_slice(&b);
                }
            }
            8 => {
                let get = |r: &[u8], i: usize| {
                    let mut a = [0u8; 8];
                    a.copy_from_slice(&r[8 * i..8 * i + 8]);
                    if little {
                        u64::from_le_bytes(a)
                    } else {
                        u64::from_be_bytes(a)
                    }
                };
                for i in spp..row_samples {
                    let v = get(row, i).wrapping_add(get(row, i - spp));
                    let b = if little {
                        v.to_le_bytes()
                    } else {
                        v.to_be_bytes()
                    };
                    row[8 * i..8 * i + 8].copy_from_slice(&b);
                }
            }
            _ => {}
        }
    }
}

/// Undoes the floating point predictor (3) in place, as libtiff does: each
/// row's bytes summed along with a stride of `spp`, then its words put back
/// together from their byte planes (most significant first). The words come
/// out big endian whatever the file's order.
pub fn unpredict_float(buf: &mut [u8], row_samples: usize, spp: usize, bytes: usize) {
    let row_bytes = row_samples * bytes;
    if row_bytes == 0 || spp == 0 {
        return;
    }
    let mut tmp = vec![0u8; row_bytes];
    for row in buf.chunks_exact_mut(row_bytes) {
        for i in spp..row_bytes {
            row[i] = row[i].wrapping_add(row[i - spp]);
        }
        tmp.copy_from_slice(row);
        for w in 0..row_samples {
            for b in 0..bytes {
                row[bytes * w + b] = tmp[b * row_samples + w];
            }
        }
    }
}

/// A JPEG-compressed block's stream made whole: the file's tables (without
/// their end marker) before the block's stream (without its start marker),
/// as libtiff hands it to its decoder.
pub fn jpeg_stream(tables: Option<&[u8]>, data: &[u8]) -> Vec<u8> {
    match tables {
        Some(t) if t.len() > 4 && data.starts_with(&[0xFF, 0xD8]) => {
            let t = t.strip_suffix(&[0xFF, 0xD9]).unwrap_or(t);
            let mut out = Vec::with_capacity(t.len() + data.len());
            out.extend_from_slice(t);
            out.extend_from_slice(&data[2..]);
            out
        }
        _ => data.to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A TIFF LZW coder (the writer's rule) for the round trip.
    fn encode(data: &[u8]) -> Vec<u8> {
        use std::collections::HashMap;
        let mut out: Vec<u8> = Vec::new();
        let mut acc: u64 = 0;
        let mut nbits = 0u32;
        let mut put = |code: u32, width: u32, out: &mut Vec<u8>| {
            acc = (acc << width) | u64::from(code);
            nbits += width;
            while nbits >= 8 {
                out.push((acc >> (nbits - 8)) as u8);
                nbits -= 8;
            }
        };
        let mut table: HashMap<Vec<u8>, u32> = (0..256u32).map(|i| (vec![i as u8], i)).collect();
        let mut next = 258u32;
        let mut width = 9u32;
        put(256, width, &mut out);
        let mut w: Vec<u8> = Vec::new();
        for &c in data {
            let mut wc = w.clone();
            wc.push(c);
            if table.contains_key(&wc) {
                w = wc;
            } else {
                put(table[&w], width, &mut out);
                table.insert(wc, next);
                next += 1;
                // The coder's table is a code ahead of the decoder's: it widens a code later.
                if next >= (1 << width) && width < 12 {
                    width += 1;
                }
                if next >= 4094 {
                    put(256, width, &mut out);
                    table = (0..256u32).map(|i| (vec![i as u8], i)).collect();
                    next = 258;
                    width = 9;
                }
                w = vec![c];
            }
        }
        if !w.is_empty() {
            put(table[&w], width, &mut out);
        }
        put(257, width, &mut out);
        if nbits > 0 {
            out.push((acc << (8 - nbits)) as u8);
        }
        out
    }

    #[test]
    fn lzw_round_trips_long_and_repetitive_runs() {
        let mut data = Vec::new();
        for i in 0..20_000u32 {
            data.push(((i * 7) % 251) as u8);
            if i % 5 == 0 {
                data.extend_from_slice(b"aaaaaaaabbbb");
            }
        }
        let coded = encode(&data);
        assert_eq!(lzw(&coded, data.len()).expect("decodes"), data);
    }

    #[test]
    fn packbits_follows_the_runs() {
        // Apple's example from the TIFF 6.0 specification.
        let coded = [
            0xFE, 0xAA, 0x02, 0x80, 0x00, 0x2A, 0xFD, 0xAA, 0x03, 0x80, 0x00, 0x2A, 0x22, 0xF7,
            0xAA,
        ];
        let plain = [
            0xAA, 0xAA, 0xAA, 0x80, 0x00, 0x2A, 0xAA, 0xAA, 0xAA, 0xAA, 0x80, 0x00, 0x2A, 0x22,
            0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA,
        ];
        assert_eq!(packbits(&coded, plain.len()), plain);
    }

    #[test]
    fn the_predictors_are_undone() {
        let mut row = vec![10u8, 1, 1, 1];
        unpredict_horizontal(&mut row, 4, 1, 1, true);
        assert_eq!(row, [10, 11, 12, 13]);
        // 16-bit little endian, two samples a pixel.
        let mut row: Vec<u8> = [1000u16, 2000, 5, 7]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        unpredict_horizontal(&mut row, 4, 2, 2, true);
        let back: Vec<u16> = row
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        assert_eq!(back, [1000, 2000, 1005, 2007]);
    }
}
