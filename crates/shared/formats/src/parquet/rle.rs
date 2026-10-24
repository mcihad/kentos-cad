//! Parquet's RLE/bit-packing hybrid (levels, dictionary indices, booleans) and its deprecated BIT_PACKED levels
//! (docs/adr/0628 §1): runs of one value and groups of eight packed least significant bit first. Reading stops with an
//! error when the stream ends before the values it must give; writing packs groups of eight and runs of eight or more.

/// The bits a number up to `max` needs.
pub fn width(max: u32) -> u32 {
    32 - max.leading_zeros()
}

fn uleb(bytes: &[u8], at: &mut usize) -> Result<u64, String> {
    let mut v = 0u64;
    for shift in (0..64).step_by(7) {
        let b = *bytes.get(*at).ok_or("Parquet RLE akışı yarım")?;
        *at += 1;
        v |= u64::from(b & 0x7f) << shift;
        if b & 0x80 == 0 {
            return Ok(v);
        }
    }
    Err("Parquet RLE akışında bozuk sayı".into())
}

/// Little-endian bytes from `at`, missing ones zero.
fn word(bytes: &[u8], at: usize) -> u64 {
    match bytes.get(at..at + 8) {
        Some(b) => u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]),
        None => {
            let mut w = 0u64;
            for k in 0..8 {
                if let Some(&b) = bytes.get(at + k) {
                    w |= u64::from(b) << (8 * k);
                }
            }
            w
        }
    }
}

/// `n` values of `width` bits packed least significant bit first.
pub fn unpack(bytes: &[u8], width: u32, n: usize, out: &mut Vec<u32>) {
    if width == 0 {
        out.resize(out.len() + n, 0);
        return;
    }
    let mask = if width == 32 {
        u64::from(u32::MAX)
    } else {
        (1u64 << width) - 1
    };
    for k in 0..n {
        let bit = k * width as usize;
        out.push(((word(bytes, bit / 8) >> (bit % 8)) & mask) as u32);
    }
}

/// `count` values of the hybrid stream `bytes` (no length before it) appended to `out`; the bytes it took.
pub fn read(bytes: &[u8], width: u32, count: usize, out: &mut Vec<u32>) -> Result<usize, String> {
    if width > 32 {
        return Err(format!(
            "Parquet RLE genişliği {width} bit, en çok 32 olabilir"
        ));
    }
    let target = out.len() + count;
    let value_bytes = width.div_ceil(8) as usize;
    let mut at = 0usize;
    while out.len() < target {
        let header = uleb(bytes, &mut at)?;
        let left = target - out.len();
        if header & 1 == 1 {
            let groups = usize::try_from(header >> 1).map_err(|_| "Parquet RLE akışı bozuk")?;
            let n = groups.checked_mul(8).ok_or("Parquet RLE akışı bozuk")?;
            let size = groups
                .checked_mul(width as usize)
                .ok_or("Parquet RLE akışı bozuk")?;
            let take = n.min(left);
            let need = (take * width as usize).div_ceil(8);
            let end = at + size.min(bytes.len().saturating_sub(at));
            if end - at < need {
                return Err("Parquet RLE akışının paketli değerleri yarım".into());
            }
            unpack(&bytes[at..end], width, take, out);
            at = end;
        } else {
            let run = usize::try_from(header >> 1).map_err(|_| "Parquet RLE akışı bozuk")?;
            let b = bytes
                .get(at..at + value_bytes)
                .ok_or("Parquet RLE akışının değeri yarım")?;
            at += value_bytes;
            let mut v = 0u32;
            for (k, &x) in b.iter().enumerate() {
                v |= u32::from(x) << (8 * k);
            }
            if width < 32 && v >> width != 0 {
                return Err("Parquet RLE değeri genişliğine sığmıyor".into());
            }
            if run == 0 {
                return Err("Parquet RLE akışında boş tekrar".into());
            }
            out.resize(out.len() + run.min(left), v);
        }
    }
    Ok(at)
}

/// A level stream of a v1 data page: its four-byte length, then the hybrid; the bytes it took.
pub fn read_prefixed(
    bytes: &[u8],
    width: u32,
    count: usize,
    out: &mut Vec<u32>,
) -> Result<usize, String> {
    let b = bytes.get(..4).ok_or("Parquet düzey akışının boyu yarım")?;
    let n = u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize;
    let body = bytes
        .get(4..4 + n)
        .ok_or("Parquet düzey akışı sayfadan uzun")?;
    read(body, width, count, out)?;
    Ok(4 + n)
}

/// The deprecated BIT_PACKED levels: `count` values packed most significant bit first; the bytes they took.
pub fn read_msb(
    bytes: &[u8],
    width: u32,
    count: usize,
    out: &mut Vec<u32>,
) -> Result<usize, String> {
    let size = (count * width as usize).div_ceil(8);
    let b = bytes
        .get(..size)
        .ok_or("Parquet BIT_PACKED düzeyleri yarım")?;
    for k in 0..count {
        let mut v = 0u32;
        for j in 0..width as usize {
            let bit = k * width as usize + j;
            v = (v << 1) | u32::from((b[bit / 8] >> (7 - bit % 8)) & 1);
        }
        out.push(v);
    }
    Ok(size)
}

fn put_uleb(out: &mut Vec<u8>, mut v: u64) {
    loop {
        let b = (v & 0x7f) as u8;
        v >>= 7;
        if v == 0 {
            out.push(b);
            return;
        }
        out.push(b | 0x80);
    }
}

/// `values` as the hybrid: runs of eight or more equal values as runs, the rest packed in groups of eight (the last
/// group padded with zeros).
pub fn write(values: &[u32], width: u32, out: &mut Vec<u8>) {
    let value_bytes = width.div_ceil(8) as usize;
    let n = values.len();
    let mut i = 0;
    while i < n {
        let mut run = 1;
        while i + run < n && values[i + run] == values[i] {
            run += 1;
        }
        if run >= 8 {
            put_uleb(out, (run as u64) << 1);
            out.extend_from_slice(&values[i].to_le_bytes()[..value_bytes]);
            i += run;
            continue;
        }
        // Packed groups until a run of eight starts at a group's beginning.
        let start = i;
        let mut groups = 0usize;
        while i < n {
            let mut r = 1;
            while i + r < n && values[i + r] == values[i] && r < 8 {
                r += 1;
            }
            if groups > 0 && r >= 8 {
                break;
            }
            groups += 1;
            i = (i + 8).min(n);
        }
        put_uleb(out, ((groups as u64) << 1) | 1);
        let mut acc = 0u64;
        let mut bits = 0u32;
        for k in 0..groups * 8 {
            let v = values.get(start + k).copied().unwrap_or(0);
            acc |= u64::from(v) << bits;
            bits += width;
            while bits >= 8 {
                out.push(acc as u8);
                acc >>= 8;
                bits -= 8;
            }
        }
        if bits > 0 {
            out.push(acc as u8);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_and_packed_groups_go_round() {
        let cases: Vec<Vec<u32>> = vec![
            vec![],
            vec![1],
            vec![0; 100],
            (0..37).map(|k| k % 3).collect(),
            [
                vec![1; 20],
                (0..13).map(|k| k % 2).collect(),
                vec![0; 9],
                vec![1, 0, 1],
            ]
            .concat(),
            (0..1000).map(|k| (k * 7919) % 1024).collect(),
        ];
        for values in cases {
            let w = width(values.iter().copied().max().unwrap_or(0));
            let mut bytes = Vec::new();
            write(&values, w, &mut bytes);
            let mut back = Vec::new();
            let used = read(&bytes, w, values.len(), &mut back).unwrap();
            assert_eq!(back, values);
            assert_eq!(used, bytes.len());
        }
    }

    #[test]
    fn a_short_stream_is_an_error() {
        let mut bytes = Vec::new();
        write(&(0..64).collect::<Vec<u32>>(), 6, &mut bytes);
        for n in 0..bytes.len() {
            let mut out = Vec::new();
            assert!(read(&bytes[..n], 6, 64, &mut out).is_err());
        }
    }

    #[test]
    fn msb_levels_read() {
        // 1, 0, 1, 1 at one bit each, most significant first: 0b1011_0000.
        let mut out = Vec::new();
        assert_eq!(read_msb(&[0b1011_0000], 1, 4, &mut out).unwrap(), 1);
        assert_eq!(out, [1, 0, 1, 1]);
    }
}
