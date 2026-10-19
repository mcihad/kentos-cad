//! A cloud-optimized GeoTIFF from a tiled one (docs/adr/0515): the header, GDAL's structural metadata, every
//! directory with its values, then the tiles from the smallest overview to the full image, each image's row by row.
//! A reader of an address gets the whole structure in its first range and each level's tiles together; GDAL's
//! `validate_cloud_optimized_geotiff.py` accepts the layout.
//!
//! The tiled file is read as it is (classic or BigTIFF, little endian): every directory's entries are kept byte for
//! byte but the tiles' offsets, which name their new places. The tiles' bytes are not touched; the host copies them
//! from the tiled file in [`Plan::copies`]' order after [`Plan::front`], each [`framed`]. Memory is the directories'
//! values.

/// A tile's bytes to copy from the tiled file: where and how many. In the COG they are [`framed`]: GDAL's leader (their
/// count) before them, their last four bytes again after them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Copy {
    pub from: u64,
    pub len: u64,
}

/// The bytes before and after a tile in a COG.
pub const FRAME: u64 = 8;

/// A tile's bytes as the COG holds them: their count (a little-endian `u32`), the bytes, their last four bytes again
/// (GDAL's `BLOCK_LEADER=SIZE_AS_UINT4` and `BLOCK_TRAILER=LAST_4_BYTES_REPEATED`: a reader sees a whole tile, and a
/// tile cut short, in the bytes it fetched).
pub fn framed(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + FRAME as usize);
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(data);
    let mut tail = [0u8; 4];
    let n = data.len().min(4);
    tail[4 - n..].copy_from_slice(&data[data.len() - n..]);
    out.extend_from_slice(&tail);
    out
}

/// The COG: the bytes to write first, then the tiles to copy in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    pub front: Vec<u8>,
    pub copies: Vec<Copy>,
    /// The COG's length.
    pub length: u64,
}

/// GDAL's structural metadata (its “ghost area”), as GDAL's COG driver writes it: the file's directories come before
/// its data, each image's tiles row by row, each tile [`framed`].
const STRUCTURE: &str = "LAYOUT=IFDS_BEFORE_DATA\nBLOCK_ORDER=ROW_MAJOR\nBLOCK_LEADER=SIZE_AS_UINT4\nBLOCK_TRAILER=LAST_4_BYTES_REPEATED\nKNOWN_INCOMPATIBLE_EDITION=NO\n ";

/// At most this many directories (a 2³² × 2³² raster's levels and more).
const MOST_DIRECTORIES: usize = 64;
/// At most this many entries in a directory.
const MOST_ENTRIES: u64 = 1000;
/// At most this many bytes of values in all.
const MOST_VALUES: u64 = 512 << 20;

const TILE_OFFSETS: u16 = 324;
const TILE_COUNTS: u16 = 325;

/// A directory entry: its tag, type, count and value bytes as the file holds them.
#[derive(Clone, Debug)]
struct Entry {
    tag: u16,
    kind: u16,
    count: u64,
    bytes: Vec<u8>,
}

/// A directory: its entries and its tiles' offsets and byte counts.
#[derive(Clone, Debug)]
struct Directory {
    entries: Vec<Entry>,
    offsets: Vec<u64>,
    counts: Vec<u64>,
}

/// The size of a value of TIFF type `kind`.
fn type_size(kind: u16) -> Option<u64> {
    Some(match kind {
        1 | 2 | 6 | 7 => 1,
        3 | 8 => 2,
        4 | 9 | 11 | 13 => 4,
        5 | 10 | 12 | 16 | 17 | 18 => 8,
        _ => return None,
    })
}

fn u16_at(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([b[at], b[at + 1]])
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

fn u64_at(b: &[u8], at: usize) -> u64 {
    let mut v = [0u8; 8];
    v.copy_from_slice(&b[at..at + 8]);
    u64::from_le_bytes(v)
}

/// An entry's values as unsigned integers (SHORT, LONG or LONG8), as offsets and byte counts are written.
fn integers(e: &Entry) -> Result<Vec<u64>, String> {
    let size = match e.kind {
        3 => 2,
        4 | 13 => 4,
        16 | 18 => 8,
        _ => return Err(format!("TIFF'in {} etiketinin türü tam sayı değil.", e.tag)),
    };
    Ok(e.bytes
        .chunks_exact(size)
        .map(|c| match size {
            2 => u64::from(u16_at(c, 0)),
            4 => u64::from(u32_at(c, 0)),
            _ => u64_at(c, 0),
        })
        .collect())
}

/// Reads the tiled TIFF's directories through `read` (`offset`, `len` → bytes) from a file of `size` bytes.
fn directories(
    read: &mut dyn FnMut(u64, u64) -> Result<Vec<u8>, String>,
    size: u64,
) -> Result<(bool, Vec<Directory>), String> {
    let head = read(0, size.min(16))?;
    if head.len() < 8 || head[0] != b'I' || head[1] != b'I' {
        return Err("Dosya küçük sonlu (II) bir TIFF değil.".into());
    }
    let big = match u16_at(&head, 2) {
        42 => false,
        43 if head.len() >= 16 && u16_at(&head, 4) == 8 => true,
        _ => return Err("Dosya TIFF değil.".into()),
    };
    let mut next = if big {
        u64_at(&head, 8)
    } else {
        u64::from(u32_at(&head, 4))
    };
    let (count_size, entry_size, next_size, inline) = if big {
        (8u64, 20u64, 8u64, 8u64)
    } else {
        (2, 12, 4, 4)
    };
    let mut out = Vec::new();
    let mut values = 0u64;
    while next != 0 {
        if out.len() == MOST_DIRECTORIES {
            return Err("TIFF'te çok fazla dizin var.".into());
        }
        if next >= size {
            return Err("TIFF'in dizini dosyanın dışında.".into());
        }
        let c = read(next, count_size)?;
        let n = if big {
            u64_at(&c, 0)
        } else {
            u64::from(u16_at(&c, 0))
        };
        if n == 0 || n > MOST_ENTRIES {
            return Err("TIFF'in dizini bozuk.".into());
        }
        let body = read(next + count_size, n * entry_size + next_size)?;
        if body.len() as u64 != n * entry_size + next_size {
            return Err("TIFF'in dizini dosyanın sonunu aşıyor.".into());
        }
        let mut entries = Vec::with_capacity(n as usize);
        for k in 0..n as usize {
            let at = k * entry_size as usize;
            let tag = u16_at(&body, at);
            let kind = u16_at(&body, at + 2);
            let count = if big {
                u64_at(&body, at + 4)
            } else {
                u64::from(u32_at(&body, at + 4))
            };
            let value_at = at + if big { 12 } else { 8 };
            let Some(unit) = type_size(kind) else {
                return Err(format!(
                    "TIFF'in {tag} etiketinin türü ({kind}) bilinmiyor."
                ));
            };
            let len = count
                .checked_mul(unit)
                .ok_or_else(|| "TIFF'in dizini bozuk.".to_owned())?;
            values += len;
            if values > MOST_VALUES {
                return Err("TIFF'in dizinlerindeki değerler çok büyük.".into());
            }
            let bytes = if len <= inline {
                body[value_at..value_at + len as usize].to_vec()
            } else {
                let from = if big {
                    u64_at(&body, value_at)
                } else {
                    u64::from(u32_at(&body, value_at))
                };
                if from.checked_add(len).is_none_or(|end| end > size) {
                    return Err(format!(
                        "TIFF'in {tag} etiketinin değerleri dosyanın dışında."
                    ));
                }
                read(from, len)?
            };
            entries.push(Entry {
                tag,
                kind,
                count,
                bytes,
            });
        }
        let tail = n as usize * entry_size as usize;
        next = if big {
            u64_at(&body, tail)
        } else {
            u64::from(u32_at(&body, tail))
        };
        if entries.iter().any(|e| e.tag == 273) {
            return Err("TIFF karolu değil (şeritli); COG'a önce karolu yazılmalı.".into());
        }
        let find = |tag: u16| entries.iter().find(|e| e.tag == tag);
        let (Some(o), Some(c)) = (find(TILE_OFFSETS), find(TILE_COUNTS)) else {
            return Err("TIFF karolu değil.".into());
        };
        let offsets = integers(o)?;
        let counts = integers(c)?;
        if offsets.len() != counts.len() {
            return Err("TIFF'in karo sayıları birbirini tutmuyor.".into());
        }
        if offsets
            .iter()
            .zip(&counts)
            .any(|(&o, &c)| o.checked_add(c).is_none_or(|end| end > size))
        {
            return Err("TIFF'in bir karosu dosyanın dışında.".into());
        }
        entries.sort_by_key(|e| e.tag);
        out.push(Directory {
            entries,
            offsets,
            counts,
        });
    }
    if out.is_empty() {
        return Err("TIFF'te görüntü yok.".into());
    }
    Ok((big, out))
}

/// A directory's bytes at `at`: its entries (the tiles' offsets `offsets`), the next directory's place left 0, and
/// its out-of-line values after it. Returns the bytes and where the next directory's offset sits in them.
fn write_directory(big: bool, at: u64, d: &Directory, offsets: &[u64]) -> (Vec<u8>, usize) {
    let (count_size, entry_size, next_size, inline) = if big {
        (8usize, 20usize, 8usize, 8usize)
    } else {
        (2, 12, 4, 4)
    };
    let n = d.entries.len();
    let head = count_size + n * entry_size + next_size;
    let mut dir = Vec::with_capacity(head);
    let mut data: Vec<u8> = Vec::new();
    if big {
        dir.extend_from_slice(&(n as u64).to_le_bytes());
    } else {
        dir.extend_from_slice(&(n as u16).to_le_bytes());
    }
    for e in &d.entries {
        let (kind, count, bytes) = if e.tag == TILE_OFFSETS {
            if big {
                (
                    16u16,
                    offsets.len() as u64,
                    offsets.iter().flat_map(|v| v.to_le_bytes()).collect(),
                )
            } else {
                (
                    4u16,
                    offsets.len() as u64,
                    offsets
                        .iter()
                        .flat_map(|&v| (v as u32).to_le_bytes())
                        .collect::<Vec<u8>>(),
                )
            }
        } else {
            (e.kind, e.count, e.bytes.clone())
        };
        dir.extend_from_slice(&e.tag.to_le_bytes());
        dir.extend_from_slice(&kind.to_le_bytes());
        if big {
            dir.extend_from_slice(&count.to_le_bytes());
        } else {
            dir.extend_from_slice(&(count as u32).to_le_bytes());
        }
        if bytes.len() <= inline {
            let mut v = bytes;
            v.resize(inline, 0);
            dir.extend_from_slice(&v);
        } else {
            if data.len() % 2 == 1 {
                data.push(0);
            }
            let where_ = at + head as u64 + data.len() as u64;
            if big {
                dir.extend_from_slice(&where_.to_le_bytes());
            } else {
                dir.extend_from_slice(&(where_ as u32).to_le_bytes());
            }
            data.extend_from_slice(&bytes);
        }
    }
    let next_at = dir.len();
    dir.extend(std::iter::repeat_n(0u8, next_size));
    dir.extend_from_slice(&data);
    (dir, next_at)
}

/// The COG of the tiled little-endian TIFF read through `read` from a file of `size` bytes. Its first directory is
/// the full image and the others its overviews, from the largest down (as KentOS's writer and GDAL write them).
pub fn plan(
    read: &mut dyn FnMut(u64, u64) -> Result<Vec<u8>, String>,
    size: u64,
) -> Result<Plan, String> {
    let (big, dirs) = directories(read, size)?;
    // The header and the structural metadata.
    let mut front: Vec<u8> = if big {
        vec![0x49, 0x49, 43, 0, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
    } else {
        vec![0x49, 0x49, 42, 0, 0, 0, 0, 0]
    };
    front.extend_from_slice(
        format!(
            "GDAL_STRUCTURAL_METADATA_SIZE={:06} bytes\n{STRUCTURE}",
            STRUCTURE.len()
        )
        .as_bytes(),
    );
    if front.len() % 2 == 1 {
        front.push(0);
    }
    let first = front.len() as u64;
    // The directories' lengths do not hang on the offsets' values: lay them out with zeros first.
    let mut lengths = Vec::with_capacity(dirs.len());
    let mut at = first;
    for d in &dirs {
        let zeros = vec![0u64; d.offsets.len()];
        let (bytes, _) = write_directory(big, at, d, &zeros);
        let len = bytes.len() as u64 + (bytes.len() as u64 % 2);
        lengths.push(len);
        at += len;
    }
    let data_start = at;
    // The tiles from the smallest overview to the full image, each image's in its order.
    let mut offsets: Vec<Vec<u64>> = dirs.iter().map(|d| vec![0u64; d.offsets.len()]).collect();
    let mut copies = Vec::new();
    let mut pos = data_start;
    for (k, d) in dirs.iter().enumerate().rev() {
        for (i, (&from, &len)) in d.offsets.iter().zip(&d.counts).enumerate() {
            if len == 0 {
                continue;
            }
            offsets[k][i] = pos + 4;
            copies.push(Copy { from, len });
            pos += len + FRAME;
        }
    }
    if !big && pos > u64::from(u32::MAX) {
        return Err("COG 4 GB'ı aşıyor; kaynak BigTIFF olarak yazılmalı.".into());
    }
    // The directories with their tiles' places, each naming the next.
    let mut at = first;
    let mut starts = Vec::with_capacity(dirs.len());
    let mut body: Vec<u8> = Vec::new();
    let mut nexts = Vec::with_capacity(dirs.len());
    for (k, d) in dirs.iter().enumerate() {
        starts.push(at);
        let (bytes, next_at) = write_directory(big, at, d, &offsets[k]);
        nexts.push(body.len() + next_at);
        body.extend_from_slice(&bytes);
        if body.len() % 2 == 1 {
            body.push(0);
        }
        at += lengths[k];
    }
    for (k, &n) in nexts.iter().enumerate() {
        let next = starts.get(k + 1).copied().unwrap_or(0);
        if big {
            body[n..n + 8].copy_from_slice(&next.to_le_bytes());
        } else {
            body[n..n + 4].copy_from_slice(&(next as u32).to_le_bytes());
        }
    }
    if big {
        front[8..16].copy_from_slice(&first.to_le_bytes());
    } else {
        front[4..8].copy_from_slice(&(first as u32).to_le_bytes());
    }
    front.extend_from_slice(&body);
    debug_assert_eq!(front.len() as u64, data_start);
    Ok(Plan {
        front,
        copies,
        length: pos,
    })
}

/// The COG of a tiled TIFF held in memory.
pub fn of_bytes(tiff: &[u8]) -> Result<Vec<u8>, String> {
    let size = tiff.len() as u64;
    let mut read = |offset: u64, len: u64| -> Result<Vec<u8>, String> {
        let end = offset.saturating_add(len).min(size);
        Ok(tiff[offset.min(size) as usize..end as usize].to_vec())
    };
    let plan = plan(&mut read, size)?;
    let mut out = plan.front;
    out.reserve((plan.length as usize).saturating_sub(out.len()));
    for c in plan.copies {
        out.extend_from_slice(&framed(&tiff[c.from as usize..(c.from + c.len) as usize]));
    }
    Ok(out)
}

/// Whether a file's first bytes (at least 16 of them past the header) are a COG's that KentOS or GDAL wrote:
/// GDAL's structural metadata naming the directories before the data.
pub fn declared(head: &[u8]) -> bool {
    let at = match head.get(..4) {
        Some([0x49, 0x49, 42, 0]) => 8,
        Some([0x49, 0x49, 43, 0]) => 16,
        _ => return false,
    };
    let Some(rest) = head.get(at..) else {
        return false;
    };
    rest.starts_with(b"GDAL_STRUCTURAL_METADATA_SIZE=")
        && rest.windows(23).any(|w| w == b"LAYOUT=IFDS_BEFORE_DATA")
}

#[cfg(test)]
mod tests {
    use super::super::write::{Geo, Image, Writer};
    use super::*;
    use kentos_contracts::RasterSample;

    use super::super::Samples;

    /// A tiled file of `w` × `h` with its levels, tiles numbered by level and place.
    fn tiled(w: u32, h: u32) -> Vec<u8> {
        let sizes = kentos_geometry_core::geom::raster::level_sizes(w, h);
        let images: Vec<Image> = sizes
            .iter()
            .enumerate()
            .map(|(k, &(w, h))| Image {
                width: w,
                height: h,
                bands: 1,
                sample: RasterSample::U8,
                alpha: false,
                geo: (k == 0).then_some(Geo {
                    affine: [487000.0, 1.0, 0.0, 4420000.0, 0.0, -1.0],
                    epsg: Some(5256),
                    geographic: false,
                }),
                nodata: Some(0.0),
            })
            .collect();
        let (mut wr, header) = Writer::new(images.clone(), 1).expect("writer");
        let mut file = header;
        for (k, img) in images.iter().enumerate() {
            for ty in 0..img.height.div_ceil(256) {
                for tx in 0..img.width.div_ceil(256) {
                    let v = (k as u32 * 37 + ty * 5 + tx + 1) as f64;
                    let s = Samples::filled(RasterSample::U8, 256 * 256, v);
                    file.extend(wr.tile(k, tx, ty, &s).expect("tile"));
                }
            }
        }
        let (tail, header) = wr.finish().expect("finish");
        file.extend(tail);
        file[..header.len()].copy_from_slice(&header);
        file
    }

    #[test]
    fn the_directories_come_first_and_the_smallest_tiles_before_the_full_image() {
        let src = tiled(700, 600);
        let cog = of_bytes(&src).expect("cog");
        assert!(declared(&cog[..128]));
        let (_, dirs) = directories(
            &mut |o, l| Ok(cog[o as usize..(o + l).min(cog.len() as u64) as usize].to_vec()),
            cog.len() as u64,
        )
        .expect("read back");
        assert_eq!(dirs.len(), 3);
        let first_data = dirs.iter().map(|d| d.offsets[0]).collect::<Vec<_>>();
        // Every directory before every tile; the smallest level's tiles first.
        let front = plan(
            &mut |o, l| Ok(src[o as usize..(o + l).min(src.len() as u64) as usize].to_vec()),
            src.len() as u64,
        )
        .expect("plan")
        .front
        .len() as u64;
        assert!(dirs.iter().flat_map(|d| &d.offsets).all(|&o| o >= front));
        assert!(first_data[2] < first_data[1] && first_data[1] < first_data[0]);
        // Each tile's bytes are the source's, in their new place.
        let (_, before) = directories(
            &mut |o, l| Ok(src[o as usize..(o + l).min(src.len() as u64) as usize].to_vec()),
            src.len() as u64,
        )
        .expect("source");
        for (a, b) in before.iter().zip(&dirs) {
            assert_eq!(a.counts, b.counts);
            for ((&oa, &ob), &c) in a.offsets.iter().zip(&b.offsets).zip(&a.counts) {
                assert_eq!(
                    src[oa as usize..(oa + c) as usize],
                    cog[ob as usize..(ob + c) as usize]
                );
            }
            // The other entries byte for byte.
            let others = |d: &Directory| {
                d.entries
                    .iter()
                    .filter(|e| e.tag != TILE_OFFSETS)
                    .map(|e| (e.tag, e.kind, e.count, e.bytes.clone()))
                    .collect::<Vec<_>>()
            };
            assert_eq!(others(a), others(b));
        }
        // The same file gives the same bytes; a COG of a COG is itself.
        assert_eq!(of_bytes(&src).expect("again"), cog);
        assert_eq!(of_bytes(&cog).expect("twice"), cog);
    }

    #[test]
    fn a_stripped_or_big_endian_file_is_refused() {
        assert!(of_bytes(b"MM\0*\0\0\0\x08").is_err());
        assert!(of_bytes(b"II*\0\x08\0\0\0").is_err());
    }
}
