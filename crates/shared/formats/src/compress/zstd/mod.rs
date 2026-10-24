//! Zstandard decoding (RFC 8878; docs/adr/0628 §1), Parquet's ZSTD codec: frames (skippable ones passed over), raw, RLE
//! and compressed blocks, the literals section (raw, RLE, Huffman with one or four streams, the tree reused), the
//! sequences section and the content checksum (xxHash64). Dictionaries are refused. Every length is checked against
//! the caller's limit, so a hostile stream cannot make it allocate past it.

mod bits;
mod fse;
mod huffman;
mod sequences;
mod xxhash;

use huffman::Huffman;
use sequences::Tables;

const MAGIC: u32 = 0xFD2F_B528;
const BLOCK_MAX: usize = 128 * 1024;

/// The bytes the frames of `input` hold, at most `limit` of them.
pub fn decompress(input: &[u8], limit: usize) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < input.len() {
        let magic = read_u32(input, i)?;
        if magic & 0xFFFF_FFF0 == 0x184D_2A50 {
            let size = read_u32(input, i + 4)? as usize;
            i = i
                .checked_add(8 + size)
                .filter(|&e| e <= input.len())
                .ok_or("ZSTD atlanacak çerçevesi yarım")?;
            continue;
        }
        if magic != MAGIC {
            return Err("ZSTD çerçevesi tanınmıyor".into());
        }
        i = frame(input, i + 4, &mut out, limit)?;
    }
    Ok(out)
}

fn read_u32(b: &[u8], i: usize) -> Result<u32, String> {
    let s = b.get(i..i + 4).ok_or("ZSTD verisi yarım")?;
    Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

/// One frame from `i` (after its magic) into `out`; where the next frame starts.
fn frame(input: &[u8], mut i: usize, out: &mut Vec<u8>, limit: usize) -> Result<usize, String> {
    let descriptor = *input.get(i).ok_or("ZSTD çerçeve başlığı yarım")?;
    i += 1;
    if descriptor & 0x08 != 0 {
        return Err("ZSTD çerçeve başlığının ayrılmış biti dolu".into());
    }
    let single = descriptor & 0x20 != 0;
    let checksum = descriptor & 0x04 != 0;
    let mut window: Option<u64> = None;
    if !single {
        let w = *input.get(i).ok_or("ZSTD pencere boyu yarım")?;
        i += 1;
        let log = 10 + u64::from(w >> 3);
        let base = 1u64 << log;
        window = Some(base + (base / 8) * u64::from(w & 7));
    }
    let dictionary = match descriptor & 3 {
        0 => 0,
        1 => 1,
        2 => 2,
        _ => 4,
    };
    let id = input
        .get(i..i + dictionary)
        .ok_or("ZSTD sözlük kimliği yarım")?;
    if id.iter().any(|&b| b != 0) {
        return Err("sözlüklü ZSTD verisi açılmaz".into());
    }
    i += dictionary;
    let content_bytes = match descriptor >> 6 {
        0 => usize::from(single),
        1 => 2,
        2 => 4,
        _ => 8,
    };
    let content = if content_bytes == 0 {
        None
    } else {
        let b = input
            .get(i..i + content_bytes)
            .ok_or("ZSTD içerik boyu yarım")?;
        let mut v = 0u64;
        for (k, &x) in b.iter().enumerate() {
            v |= u64::from(x) << (8 * k);
        }
        i += content_bytes;
        Some(if content_bytes == 2 { v + 256 } else { v })
    };
    if let Some(c) = content
        && c > (limit - out.len().min(limit)) as u64
    {
        return Err(format!(
            "ZSTD çerçevesi {c} bayt açılır diyor, sınırı aşıyor"
        ));
    }
    let block_max = window
        .or(content)
        .map_or(BLOCK_MAX, |w| (w as usize).clamp(1, BLOCK_MAX));
    let start = out.len();
    let mut tables = Tables::new();
    let mut tree: Option<Huffman> = None;
    loop {
        let h = input.get(i..i + 3).ok_or("ZSTD blok başlığı yarım")?;
        let header = u32::from(h[0]) | (u32::from(h[1]) << 8) | (u32::from(h[2]) << 16);
        i += 3;
        let last = header & 1 != 0;
        let size = (header >> 3) as usize;
        if size > block_max {
            return Err("ZSTD bloğu en büyük blok boyunu aşıyor".into());
        }
        match (header >> 1) & 3 {
            0 => {
                let raw = input.get(i..i + size).ok_or("ZSTD ham bloğu yarım")?;
                if out.len() + size > limit {
                    return Err("ZSTD açılan veri sınırı aşıyor".into());
                }
                out.extend_from_slice(raw);
                i += size;
            }
            1 => {
                let b = *input.get(i).ok_or("ZSTD RLE bloğu yarım")?;
                if out.len() + size > limit {
                    return Err("ZSTD açılan veri sınırı aşıyor".into());
                }
                out.resize(out.len() + size, b);
                i += 1;
            }
            2 => {
                let block = input.get(i..i + size).ok_or("ZSTD bloğu yarım")?;
                compressed(block, &mut tables, &mut tree, out, start, limit)?;
                i += size;
            }
            _ => return Err("ZSTD bloğunun türü ayrılmış".into()),
        }
        if last {
            break;
        }
    }
    if let Some(c) = content
        && (out.len() - start) as u64 != c
    {
        return Err(format!(
            "ZSTD çerçevesi {} bayt açıldı, başlığı {c} diyor",
            out.len() - start
        ));
    }
    if checksum {
        let sum = read_u32(input, i)?;
        i += 4;
        if xxhash::xxh64(&out[start..], 0) as u32 != sum {
            return Err("ZSTD içerik sağlaması tutmuyor".into());
        }
    }
    Ok(i)
}

/// A compressed block: its literals, then its sequences.
fn compressed(
    block: &[u8],
    tables: &mut Tables,
    tree: &mut Option<Huffman>,
    out: &mut Vec<u8>,
    frame_start: usize,
    limit: usize,
) -> Result<(), String> {
    let b0 = *block.first().ok_or("ZSTD bloğu boş")?;
    let kind = b0 & 3;
    let format = (b0 >> 2) & 3;
    let byte = |k: usize| -> Result<u32, String> {
        block
            .get(k)
            .map(|&b| u32::from(b))
            .ok_or_else(|| "ZSTD değişmez başlığı yarım".to_owned())
    };
    let (literals, used): (Vec<u8>, usize) = match kind {
        0 | 1 => {
            let (size, head) = match format {
                0 | 2 => ((b0 >> 3) as usize, 1),
                1 => (((b0 >> 4) as u32 + (byte(1)? << 4)) as usize, 2),
                _ => (
                    ((b0 >> 4) as u32 + (byte(1)? << 4) + (byte(2)? << 12)) as usize,
                    3,
                ),
            };
            if size > BLOCK_MAX {
                return Err("ZSTD değişmezleri blok boyunu aşıyor".into());
            }
            if kind == 0 {
                let raw = block
                    .get(head..head + size)
                    .ok_or("ZSTD değişmezleri yarım")?;
                (raw.to_vec(), head + size)
            } else {
                let b = *block.get(head).ok_or("ZSTD değişmezleri yarım")?;
                (vec![b; size], head + 1)
            }
        }
        _ => {
            let (regenerated, packed, head, four) = match format {
                0 | 1 => {
                    let v = u32::from(b0) | (byte(1)? << 8) | (byte(2)? << 16);
                    ((v >> 4) & 0x3ff, (v >> 14) & 0x3ff, 3, format == 1)
                }
                2 => {
                    let v = u32::from(b0) | (byte(1)? << 8) | (byte(2)? << 16) | (byte(3)? << 24);
                    ((v >> 4) & 0x3fff, v >> 18, 4, true)
                }
                _ => {
                    let v = u64::from(b0)
                        | (u64::from(byte(1)?) << 8)
                        | (u64::from(byte(2)?) << 16)
                        | (u64::from(byte(3)?) << 24)
                        | (u64::from(byte(4)?) << 32);
                    (
                        ((v >> 4) & 0x3ffff) as u32,
                        ((v >> 22) & 0x3ffff) as u32,
                        5,
                        true,
                    )
                }
            };
            let (regenerated, packed) = (regenerated as usize, packed as usize);
            if regenerated > BLOCK_MAX {
                return Err("ZSTD değişmezleri blok boyunu aşıyor".into());
            }
            let data = block
                .get(head..head + packed)
                .ok_or("ZSTD değişmezleri yarım")?;
            let streams = if kind == 2 {
                let (h, used) = huffman::read_tree(data)?;
                *tree = Some(h);
                data.get(used..).ok_or("ZSTD değişmezleri yarım")?
            } else {
                data
            };
            let h = tree
                .as_ref()
                .ok_or("ZSTD önceki Huffman ağacını yineliyor ama önceki ağaç yok")?;
            (h.decode(streams, four, regenerated)?, head + packed)
        }
    };
    sequences::execute(
        block.get(used..).ok_or("ZSTD dizi bölümü yok")?,
        &literals,
        tables,
        out,
        frame_start,
        limit,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_and_rle_blocks_open() {
        // A frame with a single segment of 5 bytes: one RLE block of 3 'a', one last raw block "bc".
        let mut f = MAGIC.to_le_bytes().to_vec();
        f.push(0x20); // single segment, one byte content size
        f.push(5);
        let rle = (3u32 << 3) | (1 << 1);
        f.extend_from_slice(&rle.to_le_bytes()[..3]);
        f.push(b'a');
        let raw = (2u32 << 3) | 1;
        f.extend_from_slice(&raw.to_le_bytes()[..3]);
        f.extend_from_slice(b"bc");
        assert_eq!(decompress(&f, 100).unwrap(), b"aaabc");
        assert!(decompress(&f, 4).is_err());
        let mut skippable = vec![0x50, 0x2a, 0x4d, 0x18, 2, 0, 0, 0, 9, 9];
        skippable.extend_from_slice(&f);
        assert_eq!(decompress(&skippable, 100).unwrap(), b"aaabc");
    }
}
