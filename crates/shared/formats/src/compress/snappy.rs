//! Snappy's raw format (no framing; docs/adr/0628 §1), Parquet's SNAPPY codec: the uncompressed length as a varint, then
//! literals and copies. The decoder checks every length and offset against the declared size; the encoder is the greedy
//! one of the reference implementation (a hash of four bytes, matches of at least four within the 64 KiB block).

/// The bytes `input` holds, at most `limit` of them.
pub fn decompress(input: &[u8], limit: usize) -> Result<Vec<u8>, String> {
    let (size, mut i) = varint(input)?;
    if size > limit {
        return Err(format!(
            "Snappy bloğu {size} bayt açılır diyor, sınır {limit}"
        ));
    }
    let mut out: Vec<u8> = Vec::with_capacity(size);
    while i < input.len() {
        let tag = input[i];
        i += 1;
        match tag & 3 {
            0 => {
                let mut len = usize::from(tag >> 2);
                if len >= 60 {
                    let n = len - 59;
                    let bytes = input
                        .get(i..i + n)
                        .ok_or("Snappy değişmezinin boyu eksik")?;
                    len = bytes
                        .iter()
                        .rev()
                        .fold(0usize, |acc, &b| (acc << 8) | usize::from(b));
                    i += n;
                }
                let len = len + 1;
                let bytes = input
                    .get(i..i + len)
                    .ok_or("Snappy değişmezi bloğun dışına taşıyor")?;
                if out.len() + len > size {
                    return Err("Snappy değişmezi bildirilen boyu aşıyor".into());
                }
                out.extend_from_slice(bytes);
                i += len;
            }
            1 => {
                let len = usize::from((tag >> 2) & 7) + 4;
                let low = *input.get(i).ok_or("Snappy kopyası eksik")?;
                i += 1;
                let offset = (usize::from(tag >> 5) << 8) | usize::from(low);
                copy(&mut out, offset, len, size)?;
            }
            2 => {
                let len = usize::from(tag >> 2) + 1;
                let b = input.get(i..i + 2).ok_or("Snappy kopyası eksik")?;
                i += 2;
                let offset = usize::from(u16::from_le_bytes([b[0], b[1]]));
                copy(&mut out, offset, len, size)?;
            }
            _ => {
                let len = usize::from(tag >> 2) + 1;
                let b = input.get(i..i + 4).ok_or("Snappy kopyası eksik")?;
                i += 4;
                let offset = u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize;
                copy(&mut out, offset, len, size)?;
            }
        }
    }
    if out.len() != size {
        return Err(format!(
            "Snappy bloğu {} bayt açıldı, bildirilen {size}",
            out.len()
        ));
    }
    Ok(out)
}

fn varint(input: &[u8]) -> Result<(usize, usize), String> {
    let mut value = 0usize;
    for (i, &b) in input.iter().enumerate().take(5) {
        value |= usize::from(b & 0x7f) << (7 * i);
        if b & 0x80 == 0 {
            return Ok((value, i + 1));
        }
    }
    Err("Snappy bloğunun boyu okunamadı".into())
}

fn copy(out: &mut Vec<u8>, offset: usize, len: usize, size: usize) -> Result<(), String> {
    if offset == 0 || offset > out.len() {
        return Err(format!(
            "Snappy kopyasının uzaklığı ({offset}) yazılanın dışında"
        ));
    }
    if out.len() + len > size {
        return Err("Snappy kopyası bildirilen boyu aşıyor".into());
    }
    let start = out.len() - offset;
    if offset >= len {
        out.extend_from_within(start..start + len);
    } else {
        for k in 0..len {
            let b = out[start + k];
            out.push(b);
        }
    }
    Ok(())
}

const BLOCK: usize = 1 << 16;

/// `input` in Snappy's raw format.
pub fn compress(input: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len() / 2 + 16);
    let mut n = input.len();
    loop {
        let b = (n & 0x7f) as u8;
        n >>= 7;
        if n == 0 {
            out.push(b);
            break;
        }
        out.push(b | 0x80);
    }
    let mut table = vec![0u32; 1 << 14];
    for start in (0..input.len()).step_by(BLOCK) {
        let block = &input[start..(start + BLOCK).min(input.len())];
        compress_block(block, &mut table, &mut out);
    }
    out
}

fn hash(v: u32) -> usize {
    (v.wrapping_mul(0x1e35_a7bd) >> 18) as usize
}

fn read32(b: &[u8], i: usize) -> u32 {
    u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]])
}

fn compress_block(block: &[u8], table: &mut [u32], out: &mut Vec<u8>) {
    table.fill(0);
    let n = block.len();
    let mut literal_start = 0usize;
    if n >= 15 {
        let limit = n - 4;
        let mut i = 1usize;
        while i <= limit {
            let v = read32(block, i);
            let h = hash(v);
            let candidate = table[h] as usize;
            table[h] = i as u32;
            if candidate < i && read32(block, candidate) == v {
                let mut len = 4;
                while i + len < n && block[candidate + len] == block[i + len] {
                    len += 1;
                }
                literal(&block[literal_start..i], out);
                let mut left = len;
                let offset = i - candidate;
                while left > 0 {
                    // Leave at least four for the last piece, as the reference does.
                    let piece = if left > 64 {
                        if left - 64 < 4 { left - 4 } else { 64 }
                    } else {
                        left
                    };
                    emit_copy(offset, piece, out);
                    left -= piece;
                }
                i += len;
                literal_start = i;
                if i <= limit {
                    let back = i - 1;
                    if back >= 1 && back + 4 <= n {
                        table[hash(read32(block, back))] = back as u32;
                    }
                }
                continue;
            }
            i += 1;
        }
    }
    literal(&block[literal_start..], out);
}

fn literal(bytes: &[u8], out: &mut Vec<u8>) {
    if bytes.is_empty() {
        return;
    }
    let n = bytes.len() - 1;
    if n < 60 {
        out.push((n as u8) << 2);
    } else if n < 1 << 8 {
        out.push(60 << 2);
        out.push(n as u8);
    } else if n < 1 << 16 {
        out.push(61 << 2);
        out.extend_from_slice(&(n as u16).to_le_bytes());
    } else if n < 1 << 24 {
        out.push(62 << 2);
        out.extend_from_slice(&(n as u32).to_le_bytes()[..3]);
    } else {
        out.push(63 << 2);
        out.extend_from_slice(&(n as u32).to_le_bytes());
    }
    out.extend_from_slice(bytes);
}

fn emit_copy(offset: usize, len: usize, out: &mut Vec<u8>) {
    if (4..12).contains(&len) && offset < 2048 {
        out.push(1 | (((len - 4) as u8) << 2) | (((offset >> 8) as u8) << 5));
        out.push(offset as u8);
    } else if offset < 1 << 16 {
        out.push(2 | (((len - 1) as u8) << 2));
        out.extend_from_slice(&(offset as u16).to_le_bytes());
    } else {
        out.push(3 | (((len - 1) as u8) << 2));
        out.extend_from_slice(&(offset as u32).to_le_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_goes_in_comes_out() {
        let mut texts: Vec<Vec<u8>> = vec![
            Vec::new(),
            b"a".to_vec(),
            b"abcdabcdabcdabcdabcdabcdabcdabcd".to_vec(),
            "Türkiye Cumhuriyeti ".repeat(5000).into_bytes(),
        ];
        let mut x: u32 = 7;
        let noise: Vec<u8> = (0..200_000)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 17;
                x ^= x << 5;
                (x % 7) as u8
            })
            .collect();
        texts.push(noise);
        for t in texts {
            let c = compress(&t);
            assert_eq!(decompress(&c, t.len()).unwrap(), t);
        }
    }

    #[test]
    fn a_broken_block_is_refused() {
        assert!(decompress(&[5, 0x08], 100).is_err());
        assert!(decompress(&[4, 0x0c, 1, 2, 3, 4, 0x05, 9], 100).is_err());
        assert!(decompress(&[200], 100).is_err());
    }
}
