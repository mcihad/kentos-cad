//! LZ4 (docs/adr/0628 §1): Parquet's LZ4_RAW is one LZ4 block; its deprecated LZ4 codec is, as Apache Arrow reads it,
//! Hadoop's framing (big-endian sizes before each block), an LZ4 frame, or a bare block. Decoding only; every length and
//! offset is checked against the expected size.

/// The bytes of one LZ4 block, exactly `size` of them.
pub fn decompress_block(input: &[u8], size: usize) -> Result<Vec<u8>, String> {
    let mut out = Vec::with_capacity(size);
    block_into(input, &mut out, size)?;
    if out.len() != size {
        return Err(format!(
            "LZ4 bloğu {} bayt açıldı, beklenen {size}",
            out.len()
        ));
    }
    Ok(out)
}

/// Appends one block's bytes to `out`, which may hold earlier blocks a linked block copies from; `end` bounds `out`.
fn block_into(input: &[u8], out: &mut Vec<u8>, end: usize) -> Result<(), String> {
    let mut i = 0usize;
    loop {
        let token = *input.get(i).ok_or("LZ4 bloğu yarım")?;
        i += 1;
        let mut literals = usize::from(token >> 4);
        if literals == 15 {
            loop {
                let b = *input.get(i).ok_or("LZ4 değişmez boyu yarım")?;
                i += 1;
                literals += usize::from(b);
                if b != 255 {
                    break;
                }
            }
        }
        let bytes = input
            .get(i..i + literals)
            .ok_or("LZ4 değişmezi bloğun dışına taşıyor")?;
        if out.len() + literals > end {
            return Err("LZ4 değişmezi beklenen boyu aşıyor".into());
        }
        out.extend_from_slice(bytes);
        i += literals;
        if i == input.len() {
            return Ok(());
        }
        let b = input.get(i..i + 2).ok_or("LZ4 uzaklığı yarım")?;
        i += 2;
        let offset = usize::from(u16::from_le_bytes([b[0], b[1]]));
        let mut len = usize::from(token & 15) + 4;
        if token & 15 == 15 {
            loop {
                let b = *input.get(i).ok_or("LZ4 eşleşme boyu yarım")?;
                i += 1;
                len += usize::from(b);
                if b != 255 {
                    break;
                }
            }
        }
        if offset == 0 || offset > out.len() {
            return Err(format!("LZ4 uzaklığı ({offset}) yazılanın dışında"));
        }
        if out.len() + len > end {
            return Err("LZ4 eşleşmesi beklenen boyu aşıyor".into());
        }
        let start = out.len() - offset;
        if offset >= len {
            out.extend_from_within(start..start + len);
        } else {
            for k in 0..len {
                let v = out[start + k];
                out.push(v);
            }
        }
    }
}

/// Parquet's deprecated LZ4 codec, `size` bytes: Hadoop's framing, else an LZ4 frame, else a bare block.
pub fn decompress_legacy(input: &[u8], size: usize) -> Result<Vec<u8>, String> {
    if let Some(out) = hadoop(input, size) {
        return Ok(out);
    }
    if input.len() >= 4
        && u32::from_le_bytes([input[0], input[1], input[2], input[3]]) == 0x184D_2204
    {
        return frame(input, size);
    }
    decompress_block(input, size)
}

fn hadoop(input: &[u8], size: usize) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(size);
    let mut i = 0usize;
    while i < input.len() {
        let head = input.get(i..i + 8)?;
        let raw = u32::from_be_bytes([head[0], head[1], head[2], head[3]]) as usize;
        let packed = u32::from_be_bytes([head[4], head[5], head[6], head[7]]) as usize;
        i += 8;
        let block = input.get(i..i + packed)?;
        if out.len() + raw > size {
            return None;
        }
        let before = out.len();
        block_into(block, &mut out, before + raw).ok()?;
        if out.len() != before + raw {
            return None;
        }
        i += packed;
    }
    (out.len() == size).then_some(out)
}

/// An LZ4 frame (magic 0x184D2204): its blocks, compressed or stored, into one buffer (linked blocks copy from the ones
/// before); checksums are not verified.
fn frame(input: &[u8], size: usize) -> Result<Vec<u8>, String> {
    let flags = *input.get(4).ok_or("LZ4 çerçevesi yarım")?;
    if flags >> 6 != 1 {
        return Err("LZ4 çerçevesinin sürümü tanınmıyor".into());
    }
    let block_checksum = flags & 0x10 != 0;
    let content_size = flags & 0x08 != 0;
    let dictionary = flags & 0x01 != 0;
    let mut i = 7 + if content_size { 8 } else { 0 } + if dictionary { 4 } else { 0 };
    let mut out = Vec::with_capacity(size);
    loop {
        let b = input.get(i..i + 4).ok_or("LZ4 çerçevesinin bloğu yarım")?;
        i += 4;
        let word = u32::from_le_bytes([b[0], b[1], b[2], b[3]]);
        if word == 0 {
            break;
        }
        let stored = word & 0x8000_0000 != 0;
        let len = (word & 0x7fff_ffff) as usize;
        let block = input
            .get(i..i + len)
            .ok_or("LZ4 çerçevesinin bloğu yarım")?;
        i += len;
        if stored {
            if out.len() + len > size {
                return Err("LZ4 çerçevesi beklenen boyu aşıyor".into());
            }
            out.extend_from_slice(block);
        } else {
            block_into(block, &mut out, size)?;
        }
        if block_checksum {
            i += 4;
        }
    }
    if out.len() != size {
        return Err(format!(
            "LZ4 çerçevesi {} bayt açıldı, beklenen {size}",
            out.len()
        ));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_block_with_an_overlapping_match_opens() {
        // "abc", a match of 9 at offset 3, then the last sequence of no literals: "abcabcabcabc".
        let block = [0x35, b'a', b'b', b'c', 3, 0, 0];
        assert_eq!(decompress_block(&block, 12).unwrap(), b"abcabcabcabc");
        assert!(decompress_block(&block, 11).is_err());
        assert!(decompress_block(&[0x35, b'a', b'b', b'c', 9, 0, 0], 12).is_err());
        // A block may not end on a match.
        assert!(decompress_block(&block[..6], 12).is_err());
    }

    #[test]
    fn hadoop_framing_and_a_bare_block() {
        let block = [0x35, b'a', b'b', b'c', 3, 0, 0];
        let mut framed = 12u32.to_be_bytes().to_vec();
        framed.extend_from_slice(&(block.len() as u32).to_be_bytes());
        framed.extend_from_slice(&block);
        assert_eq!(decompress_legacy(&framed, 12).unwrap(), b"abcabcabcabc");
        assert_eq!(decompress_legacy(&block, 12).unwrap(), b"abcabcabcabc");
    }
}
