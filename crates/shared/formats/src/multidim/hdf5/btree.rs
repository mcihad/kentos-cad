//! HDF5's B-trees (docs/adr/0645 §2): version 1 (an old-style group's symbol nodes, a chunked dataset's chunks) and
//! version 2 (a new-style group's or an object's names in the fractal heap, chunks of a version 4 layout), walked
//! whole: every record or leaf entry they hold.

use std::collections::HashSet;

use super::io::{H5, limit_enc_size, log2};

const NODES_MOST: usize = 1 << 20;

/// A chunk's key in a version 1 B-tree: chunk size, filter mask, offsets with the element dimension last.
pub type ChunkKey = (u32, u32, Vec<u64>);

/// A version 1 B-tree's leaf children: for a group the symbol nodes' addresses; for chunks each child's key (chunk
/// size, filter mask, offsets with the element dimension last) and address.
pub fn v1_leaves(
    h: &H5,
    root: u64,
    chunk_dims: Option<usize>,
) -> Result<Vec<(Option<ChunkKey>, u64)>, String> {
    let key_size = match chunk_dims {
        Some(nd) => 8 + 8 * nd,
        None => h.l,
    };
    let mut out = Vec::new();
    let mut stack = vec![root];
    let mut seen = HashSet::new();
    while let Some(addr) = stack.pop() {
        if !seen.insert(addr) || seen.len() > NODES_MOST {
            continue;
        }
        let mut c = h.at(addr, 8 + 2 * h.o)?;
        c.sig(b"TREE", "B-ağacı düğümü")?;
        let _kind = c.u8()?;
        let level = c.u8()?;
        let used = c.u16()? as usize;
        let n = 8 + 2 * h.o + used * (key_size + h.o) + key_size;
        let mut c = h.at(addr, n)?;
        c.skip(8 + 2 * h.o)?;
        for _ in 0..used {
            let key = match chunk_dims {
                Some(nd) => {
                    let size = c.u32()?;
                    let mask = c.u32()?;
                    let mut offs = Vec::with_capacity(nd);
                    for _ in 0..nd {
                        offs.push(c.u64()?);
                    }
                    Some((size, mask, offs))
                }
                None => {
                    c.length()?;
                    None
                }
            };
            let child = c.addr()?;
            if let Some(child) = child {
                if level == 0 {
                    out.push((key, child));
                } else {
                    stack.push(child);
                }
            }
        }
    }
    Ok(out)
}

/// Every record of a version 2 B-tree, raw.
pub fn v2_records(h: &H5, header: u64) -> Result<Vec<Vec<u8>>, String> {
    let mut c = h.at(header, 16 + h.o + h.l + 2)?;
    c.sig(b"BTHD", "v2 B-ağacı başlığı")?;
    c.skip(2)?;
    let node_size = u64::from(c.u32()?);
    let rec = c.u16()? as usize;
    let depth = c.u16()? as usize;
    c.skip(2)?;
    let root = c.addr()?;
    let root_n = c.u16()? as usize;
    let Some(root) = root else {
        return Ok(Vec::new());
    };
    if rec == 0 || depth > 64 {
        return Err("HDF5'in v2 B-ağacı başlığı bozuk.".into());
    }
    // Each depth's most records and the bytes of their counts (HDF5's `H5B2__hdr_init`).
    let leaf_max = (node_size.saturating_sub(10)) / rec as u64;
    let max_nrec_size = limit_enc_size(leaf_max);
    let mut cum = vec![leaf_max; depth + 1];
    let mut cum_size = vec![0usize; depth + 1];
    for u in 1..=depth {
        let ptr = h.o + max_nrec_size + if u > 1 { cum_size[u - 1] } else { 0 };
        let max = (node_size.saturating_sub(10 + ptr as u64)) / (rec + ptr) as u64;
        cum[u] = (max + 1) * cum[u - 1] + max;
        cum_size[u] = limit_enc_size(cum[u]);
    }
    let _ = log2;
    let mut out = Vec::new();
    // (address, records, depth)
    let mut stack = vec![(root, root_n, depth)];
    let mut seen = HashSet::new();
    while let Some((addr, n, d)) = stack.pop() {
        if !seen.insert(addr) || seen.len() > NODES_MOST {
            continue;
        }
        let mut c = h.at(addr, node_size as usize)?;
        if d == 0 {
            c.sig(b"BTLF", "v2 B-ağacı yaprağı")?;
            c.skip(2)?;
            for _ in 0..n {
                out.push(c.bytes(rec)?.to_vec());
            }
        } else {
            c.sig(b"BTIN", "v2 B-ağacı düğümü")?;
            c.skip(2)?;
            for _ in 0..n {
                out.push(c.bytes(rec)?.to_vec());
            }
            for _ in 0..=n {
                let a = c.addr()?;
                let cn = c.uvar(max_nrec_size)? as usize;
                if d > 1 {
                    c.uvar(cum_size[d - 1])?;
                }
                if let Some(a) = a {
                    stack.push((a, cn, d - 1));
                }
            }
        }
    }
    Ok(out)
}
