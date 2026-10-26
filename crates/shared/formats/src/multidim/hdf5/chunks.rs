//! A chunked dataset's chunks (docs/adr/0645 §2): where each lies, its stored size and filter mask, by its chunk
//! coordinates, from any of HDF5's chunk indexes (version 1 B-tree; and of layout version 4 single chunk, implicit,
//! fixed array, extensible array, version 2 B-tree). An unwritten chunk is not listed.

use std::collections::HashMap;

use super::super::chunked::Place;
use super::btree;
use super::io::{Cur, H5, log2};
use super::object::ChunkIndex;

/// The most chunks a dataset may list.
pub const CHUNKS_MOST: u64 = 16 << 20;

/// What finding a dataset's chunks needs.
pub struct Want<'a> {
    pub shape: &'a [u64],
    pub max: &'a [u64],
    pub chunk: &'a [u64],
    pub element: u64,
    pub filtered: bool,
}

impl Want<'_> {
    fn grid(&self) -> Vec<u64> {
        self.shape
            .iter()
            .zip(self.chunk)
            .map(|(s, c)| s.div_ceil((*c).max(1)))
            .collect()
    }

    fn chunk_bytes(&self) -> u64 {
        self.chunk.iter().product::<u64>() * self.element
    }
}

/// Chunk coordinates (in chunks) of linear index `i` over `grid` (row-major).
fn coords_of(mut i: u64, grid: &[u64]) -> Vec<u64> {
    let mut out = vec![0u64; grid.len()];
    for d in (0..grid.len()).rev() {
        let g = grid[d].max(1);
        out[d] = i % g;
        i /= g;
    }
    out
}

/// Every chunk the index lists.
pub fn table(h: &H5, index: &ChunkIndex, w: &Want<'_>) -> Result<HashMap<Vec<u64>, Place>, String> {
    let grid = w.grid();
    let total: u64 = grid.iter().product();
    if total > CHUNKS_MOST {
        return Err(format!(
            "Değişken {total} parçadan oluşuyor; en çok {CHUNKS_MOST} okunur."
        ));
    }
    let mut out = HashMap::new();
    let range = |a: u64, len: u64, mask: u32| Place::Range {
        offset: h.abs(a),
        len,
        mask,
    };
    match index {
        ChunkIndex::BTree1(root) => {
            let Some(root) = root else { return Ok(out) };
            let rank = w.chunk.len();
            for (key, addr) in btree::v1_leaves(h, *root, Some(rank + 1))? {
                let Some((size, mask, offs)) = key else {
                    continue;
                };
                let coords: Vec<u64> = offs[..rank]
                    .iter()
                    .zip(w.chunk)
                    .map(|(o, c)| o / (*c).max(1))
                    .collect();
                out.insert(coords, range(addr, u64::from(size), mask));
            }
        }
        ChunkIndex::Single { address, filtered } => {
            if let Some(a) = address {
                let (len, mask) = filtered.unwrap_or((w.chunk_bytes(), 0));
                out.insert(vec![0; grid.len()], range(*a, len, mask));
            }
        }
        ChunkIndex::Implicit(address) => {
            if let Some(a) = address {
                let b = w.chunk_bytes();
                for i in 0..total {
                    out.insert(coords_of(i, &grid), range(a + i * b, b, 0));
                }
            }
        }
        ChunkIndex::FixedArray { address } => {
            if let Some(a) = address {
                fixed_array(h, *a, w, &grid, &mut out)?;
            }
        }
        ChunkIndex::ExtensibleArray { address } => {
            if let Some(a) = address {
                extensible_array(h, *a, w, &grid, &mut out)?;
            }
        }
        ChunkIndex::BTree2 { address } => {
            if let Some(a) = address {
                let rank = grid.len();
                for r in btree::v2_records(h, *a)? {
                    // A filtered record's size field: what is left of the record (its length follows the format's
                    // version, so it is read from the record, not worked out).
                    let size_len = r.len().saturating_sub(h.o + 4 + 8 * rank).clamp(1, 8);
                    let mut c = Cur::new(r, h.o, h.l);
                    let addr = c.addr()?;
                    let (len, mask) = if w.filtered {
                        (c.uvar(size_len)?, c.u32()?)
                    } else {
                        (w.chunk_bytes(), 0)
                    };
                    let mut coords = Vec::with_capacity(rank);
                    for _ in 0..rank {
                        coords.push(c.u64()?);
                    }
                    if let Some(addr) = addr {
                        out.insert(coords, range(addr, len, mask));
                    }
                }
            }
        }
    }
    Ok(out)
}

/// One array element: the chunk's address, its stored size and mask (a filtered element's size field is what the
/// element's size leaves).
fn element(c: &mut Cur, w: &Want<'_>, filtered: bool) -> Result<Option<(u64, u64, u32)>, String> {
    let size_len = c.b.len().saturating_sub(c.sizes().0 + 4).clamp(1, 8);
    let a = c.addr()?;
    let (len, mask) = if filtered {
        (c.uvar(size_len)?, c.u32()?)
    } else {
        (w.chunk_bytes(), 0)
    };
    Ok(a.map(|a| (a, len, mask)))
}

fn fixed_array(
    h: &H5,
    addr: u64,
    w: &Want<'_>,
    grid: &[u64],
    out: &mut HashMap<Vec<u64>, Place>,
) -> Result<(), String> {
    let mut c = h.at(addr, 12 + h.l + h.o)?;
    c.sig(b"FAHD", "sabit dizi başlığı")?;
    c.skip(1)?;
    let client = c.u8()?;
    let esize = c.u8()? as usize;
    let page_bits = c.u8()?;
    let n = c.length()?;
    let Some(block) = c.addr()? else {
        return Ok(());
    };
    let filtered = client == 1;
    if n > CHUNKS_MOST {
        return Err("Sabit dizide çok fazla parça var.".into());
    }
    let page = 1u64 << page_bits.min(32);
    let paged = n > page;
    let pages = n.div_ceil(page);
    let bitmap = if paged { pages.div_ceil(8) as usize } else { 0 };
    let head = 6 + h.o + bitmap;
    let total = if paged {
        head + 4 + (n as usize * esize) + pages as usize * 4
    } else {
        head + n as usize * esize + 4
    };
    let mut c = h.at(block, total)?;
    c.sig(b"FADB", "sabit dizi veri bloğu")?;
    c.skip(2 + h.o)?;
    let bits = c.bytes(bitmap)?.to_vec();
    if paged {
        c.skip(4)?;
    }
    for i in 0..n {
        if paged && i > 0 && i % page == 0 {
            c.skip(4)?;
        }
        let p = i / page;
        let live = !paged
            || bits
                .get((p / 8) as usize)
                .is_some_and(|b| b >> (7 - p % 8) & 1 == 1);
        let mut e = Cur::new(c.bytes(esize)?.to_vec(), h.o, h.l);
        if !live {
            continue;
        }
        if let Some((a, len, mask)) = element(&mut e, w, filtered)? {
            out.insert(
                coords_of(i, grid),
                Place::Range {
                    offset: h.abs(a),
                    len,
                    mask,
                },
            );
        }
    }
    Ok(())
}

/// An extensible array's super block: its data blocks' count and elements each.
struct Sblk {
    ndblks: u64,
    dblk_nelmts: u64,
    start_idx: u64,
    start_dblk: u64,
}

/// Where an index element's bytes go: its index and its bytes.
type Put<'a> = dyn FnMut(u64, &[u8]) -> Result<(), String> + 'a;

fn extensible_array(
    h: &H5,
    addr: u64,
    w: &Want<'_>,
    grid: &[u64],
    out: &mut HashMap<Vec<u64>, Place>,
) -> Result<(), String> {
    let mut c = h.at(addr, 16 + 6 * h.l + h.o)?;
    c.sig(b"EAHD", "genişleyen dizi başlığı")?;
    c.skip(1)?;
    let client = c.u8()?;
    let esize = c.u8()? as usize;
    let max_bits = u32::from(c.u8()?);
    let idx_elmts = u64::from(c.u8()?);
    let min_dblk = u64::from(c.u8()?);
    let min_ptrs = u64::from(c.u8()?);
    let page_bits = u32::from(c.u8()?);
    for _ in 0..6 {
        c.length()?;
    }
    let Some(iblock) = c.addr()? else {
        return Ok(());
    };
    let filtered = client == 1;
    if min_dblk == 0 || min_ptrs == 0 || max_bits > 63 {
        return Err("Genişleyen dizi başlığı bozuk.".into());
    }
    // The super blocks' table (HDF5's `H5EA__hdr_init`).
    let nsblks = 1 + (max_bits - log2(min_dblk)) as u64;
    let mut sblks = Vec::with_capacity(nsblks as usize);
    let (mut start_idx, mut start_dblk) = (0u64, 0u64);
    for u in 0..nsblks {
        let ndblks = 1u64 << (u / 2);
        let dblk_nelmts = min_dblk << u.div_ceil(2);
        sblks.push(Sblk {
            ndblks,
            dblk_nelmts,
            start_idx,
            start_dblk,
        });
        start_idx += ndblks * dblk_nelmts;
        start_dblk += ndblks;
    }
    let iblock_sblks = 2 * u64::from(log2(min_ptrs));
    let ndblk_addrs = 2 * (min_ptrs - 1);
    let nsblk_addrs = nsblks.saturating_sub(iblock_sblks);
    let arr_off = (max_bits as usize).div_ceil(8);
    let n = 6 + h.o + idx_elmts as usize * esize + (ndblk_addrs + nsblk_addrs) as usize * h.o + 4;
    let mut c = h.at(iblock, n)?;
    c.sig(b"EAIB", "genişleyen dizi dizin bloğu")?;
    c.skip(2 + h.o)?;
    let total: u64 = grid.iter().product();
    let mut put = |i: u64, e: &[u8]| -> Result<(), String> {
        if i >= total {
            return Ok(());
        }
        let mut e = Cur::new(e.to_vec(), h.o, h.l);
        if let Some((a, len, mask)) = element(&mut e, w, filtered)? {
            out.insert(
                coords_of(i, grid),
                Place::Range {
                    offset: h.abs(a),
                    len,
                    mask,
                },
            );
        }
        Ok(())
    };
    for i in 0..idx_elmts {
        let e = c.bytes(esize)?.to_vec();
        put(i, &e)?;
    }
    let mut dblks = Vec::with_capacity(ndblk_addrs as usize);
    for _ in 0..ndblk_addrs {
        dblks.push(c.addr()?);
    }
    let mut sblk_addrs = Vec::with_capacity(nsblk_addrs as usize);
    for _ in 0..nsblk_addrs {
        sblk_addrs.push(c.addr()?);
    }
    let page = 1u64 << page_bits.min(32);
    let read_dblk = |a: u64, s: &Sblk, put: &mut Put<'_>, first: u64| -> Result<(), String> {
        let n = s.dblk_nelmts;
        let paged = n > page;
        let pages = n.div_ceil(page);
        let len =
            6 + h.o + arr_off + n as usize * esize + 4 + if paged { pages as usize * 4 } else { 0 };
        let mut c = h.at(a, len)?;
        c.sig(b"EADB", "genişleyen dizi veri bloğu")?;
        c.skip(2 + h.o + arr_off)?;
        if paged {
            c.skip(4)?;
        }
        for k in 0..n {
            if paged && k > 0 && k % page == 0 {
                c.skip(4)?;
            }
            let e = c.bytes(esize)?.to_vec();
            put(first + k, &e)?;
        }
        Ok(())
    };
    for (u, s) in sblks.iter().enumerate() {
        let u = u as u64;
        let first_elmt = idx_elmts + s.start_idx;
        if first_elmt >= total {
            break;
        }
        if u < iblock_sblks {
            for d in 0..s.ndblks {
                if let Some(Some(a)) = dblks.get((s.start_dblk + d) as usize) {
                    read_dblk(*a, s, &mut put, first_elmt + d * s.dblk_nelmts)?;
                }
            }
        } else if let Some(Some(sa)) = sblk_addrs.get((u - iblock_sblks) as usize) {
            let paged = s.dblk_nelmts > page;
            let bitmap = if paged {
                (s.ndblks * s.dblk_nelmts.div_ceil(page).div_ceil(8)) as usize
            } else {
                0
            };
            let len = 6 + h.o + arr_off + bitmap + s.ndblks as usize * h.o + 4;
            let mut c = h.at(*sa, len)?;
            c.sig(b"EASB", "genişleyen dizi üst bloğu")?;
            c.skip(2 + h.o + arr_off + bitmap)?;
            for d in 0..s.ndblks {
                if let Some(a) = c.addr()? {
                    read_dblk(a, s, &mut put, first_elmt + d * s.dblk_nelmts)?;
                }
            }
        }
    }
    Ok(())
}
