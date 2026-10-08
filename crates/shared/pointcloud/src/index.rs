//! The COPC index KentOS builds once for a cloud that has none (docs/adr/0207 §4).
//!
//! One pass over the source (in its order) moves each record to format 6, 7 or
//! 8 and puts it, with its index in the source, into the bin of its cube at
//! depth d₀ ([`Distribute`]); the host keeps the bins (a cache folder, the
//! browser's private file system). Each bin is then built alone
//! ([`build_bin`], on any thread): split while a node holds more than
//! [`LEAF_MOST`] points, then from the leaves up every node gives its parent,
//! for each filled cell of the parent's 128³ grid, the point nearest that
//! cell's centre (ties to the earlier in the source), and keeps the rest as its
//! LAZ chunk. The nodes above the bins are built last from what the bins' roots
//! gave ([`Builder::finish`]). The file is LAS 1.4: header, COPC's info VLR,
//! LAZ's VLR (chunks of their own sizes), the system and extra bytes VLRs, the
//! chunks in the order they were built, the chunk table and the hierarchy as
//! one page.

use std::collections::{BTreeMap, HashMap};

use laz::LazVlr;

use crate::bytes::{i32_at, put_i64};
use crate::chunks;
use crate::copc::{self, Info, Key, Node};
use crate::las::{self, Header, Vlr};
use crate::record::{Layout, standard_len};
use crate::{PcError, Result};

/// Cells on a node's side.
pub const GRID: u32 = 128;
/// A node splits when it holds more points than this.
pub const LEAF_MOST: usize = 100_000;
/// The deepest node.
pub const MAX_DEPTH: i32 = 16;
/// Points a bin should hold, on average (docs/adr/0207 §4).
pub const BIN_TARGET: u64 = 2_000_000;
/// A bin holding more is split once more.
pub const BIN_MOST: usize = 8_000_000;
/// The deepest bin level.
pub const MAX_BIN_DEPTH: i32 = 8;
/// Bytes a bin gathers before the host is handed them.
const BIN_FLUSH: usize = 1 << 20;

/// The root cube.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cube {
    pub center: [f64; 3],
    pub half: f64,
}

impl Cube {
    /// The cube round `[x₁, y₁, z₁, x₂, y₂, z₂]`: its centre, half its widest side (1 when it has none).
    pub fn of(b: [f64; 6]) -> Cube {
        let center = [
            (b[0] + b[3]) / 2.0,
            (b[1] + b[4]) / 2.0,
            (b[2] + b[5]) / 2.0,
        ];
        let half = ((b[3] - b[0]) / 2.0)
            .max((b[4] - b[1]) / 2.0)
            .max((b[5] - b[2]) / 2.0);
        Cube {
            center,
            half: if half.is_finite() && half > 0.0 {
                half
            } else {
                1.0
            },
        }
    }

    pub fn low(&self) -> [f64; 3] {
        [
            self.center[0] - self.half,
            self.center[1] - self.half,
            self.center[2] - self.half,
        ]
    }

    /// A node's least corner and side.
    pub fn node(&self, key: Key) -> ([f64; 3], f64) {
        let side = key.side(self.half);
        let low = self.low();
        (
            [
                low[0] + f64::from(key.x) * side,
                low[1] + f64::from(key.y) * side,
                low[2] + f64::from(key.z) * side,
            ],
            side,
        )
    }

    /// Whether a point lies in the cube (its far faces in), give or take a
    /// billionth of its side: the cube of exact bounds holds their own corners
    /// whatever the rounding of its centre and half.
    pub fn holds(&self, p: [f64; 3]) -> bool {
        let low = self.low();
        let slack = self.half * 1e-9;
        (0..3).all(|k| p[k] >= low[k] - slack && p[k] <= low[k] + 2.0 * self.half + slack)
    }
}

/// The cell of a grid of `n` cells from `low` with cells of `cell`, clamped into the grid.
#[inline]
fn cell_of(v: f64, low: f64, cell: f64, n: i64) -> i64 {
    let t = ((v - low) / cell).floor();
    if t.is_nan() {
        0
    } else {
        (t as i64).clamp(0, n - 1)
    }
}

/// What the index is made of: the records' format and numbers, the cube, the bins' depth.
#[derive(Clone, Debug)]
pub struct Plan {
    pub cube: Cube,
    pub scale: [f64; 3],
    pub offset: [f64; 3],
    /// 6, 7 or 8.
    pub format: u8,
    pub extra: u16,
    pub bin_depth: i32,
    /// The system's WKT, when the cloud names one.
    pub wkt: Option<String>,
    /// The source's extra bytes VLR (LASF_Spec 4), written along.
    pub extra_vlr: Option<Vlr>,
    pub gps_standard: bool,
    pub source_id: u16,
    /// A node splits above this many points ([`LEAF_MOST`]; smaller in tests).
    pub leaf_most: usize,
    /// A bin splits above this many points ([`BIN_MOST`]).
    pub bin_most: usize,
}

impl Plan {
    /// A plan for `count` records of `format` (6, 7 or 8) with `extra` extra bytes, round `bounds`.
    pub fn new(
        bounds: [f64; 6],
        count: u64,
        scale: [f64; 3],
        offset: [f64; 3],
        format: u8,
        extra: u16,
    ) -> Plan {
        let mut d = 0;
        while d < MAX_BIN_DEPTH && count / (1u64 << (2 * d)) > BIN_TARGET {
            d += 1;
        }
        Plan {
            cube: Cube::of(bounds),
            scale,
            offset,
            format,
            extra,
            bin_depth: d,
            wkt: None,
            extra_vlr: None,
            gps_standard: false,
            source_id: 0,
            leaf_most: LEAF_MOST,
            bin_most: BIN_MOST,
        }
    }

    pub fn layout(&self) -> Layout {
        Layout::new(
            self.format,
            standard_len(self.format) + usize::from(self.extra),
        )
    }

    /// Bytes a binned point takes: its record and its index in the source.
    pub fn stride(&self) -> usize {
        self.layout().len + 8
    }

    #[inline]
    fn world(&self, r: &[u8]) -> [f64; 3] {
        [
            f64::from(i32_at(r, 0)) * self.scale[0] + self.offset[0],
            f64::from(i32_at(r, 4)) * self.scale[1] + self.offset[1],
            f64::from(i32_at(r, 8)) * self.scale[2] + self.offset[2],
        ]
    }

    /// The key of a point's node at depth `d`.
    fn key_at(&self, p: [f64; 3], d: i32) -> Key {
        let n = 1i64 << d;
        let side = 2.0 * self.cube.half / n as f64;
        let low = self.cube.low();
        Key {
            d,
            x: cell_of(p[0], low[0], side, n) as i32,
            y: cell_of(p[1], low[1], side, n) as i32,
            z: cell_of(p[2], low[2], side, n) as i32,
        }
    }

    /// The point data's offset: the header and the VLRs.
    pub fn data_offset(&self, laz: &LazVlr) -> Result<u32> {
        let mut at = las::HEADER_1_4;
        for v in self.vlrs(laz, &Info::empty())? {
            at += las::VLR_HEAD + v.data.len();
        }
        u32::try_from(at).map_err(|_| PcError::new("VLR'ler çok büyük."))
    }

    /// The VLRs in the order the file holds them (COPC's info first).
    fn vlrs(&self, laz: &LazVlr, info: &Info) -> Result<Vec<Vlr>> {
        let mut v = vec![info.to_vlr(), chunks::vlr_record(laz)?];
        if let Some(w) = &self.wkt {
            let mut data = w.as_bytes().to_vec();
            data.push(0);
            v.push(Vlr::new(
                "LASF_Projection",
                2112,
                "OGC Coordinate System WKT",
                data,
            ));
        }
        if let Some(e) = &self.extra_vlr {
            let mut e = e.clone();
            e.extended = false;
            v.push(e);
        }
        Ok(v)
    }
}

impl Info {
    /// The info with nothing filled in yet (for its size).
    pub fn empty() -> Info {
        Info {
            center: [0.0; 3],
            halfsize: 1.0,
            spacing: 1.0,
            root_offset: 0,
            root_size: 0,
            gps_min: 0.0,
            gps_max: 0.0,
        }
    }
}

/// What a pass over the source found.
#[derive(Clone, Debug)]
pub struct Summary {
    /// Points in each bin.
    pub bins: BTreeMap<Key, u64>,
    pub count: u64,
    /// The points' own bounds, `[x₁, y₁, z₁, x₂, y₂, z₂]`.
    pub bounds: [f64; 6],
    pub gps: [f64; 2],
    pub by_return: [u64; 15],
    /// A point lay outside the plan's cube: build again round `bounds`.
    pub outside: bool,
}

/// The first pass: records into bins.
#[derive(Debug)]
pub struct Distribute {
    plan: Plan,
    buffers: HashMap<Key, Vec<u8>>,
    summary: Summary,
}

impl Distribute {
    pub fn new(plan: Plan) -> Distribute {
        Distribute {
            plan,
            buffers: HashMap::new(),
            summary: Summary {
                bins: BTreeMap::new(),
                count: 0,
                bounds: [
                    f64::INFINITY,
                    f64::INFINITY,
                    f64::INFINITY,
                    f64::NEG_INFINITY,
                    f64::NEG_INFINITY,
                    f64::NEG_INFINITY,
                ],
                gps: [f64::INFINITY, f64::NEG_INFINITY],
                by_return: [0; 15],
                outside: false,
            },
        }
    }

    /// Takes records of the plan's format, the first being the source's point
    /// `first`; gives the bins' bytes to append to them, when a bin has gathered enough.
    pub fn add(&mut self, records: &[u8], first: u64) -> Vec<(Key, Vec<u8>)> {
        let layout = self.plan.layout();
        let mut out = Vec::new();
        for (i, r) in records.chunks_exact(layout.len).enumerate() {
            let p = self.plan.world(r);
            let s = &mut self.summary;
            for (k, &v) in p.iter().enumerate() {
                s.bounds[k] = s.bounds[k].min(v);
                s.bounds[k + 3] = s.bounds[k + 3].max(v);
            }
            if !self.plan.cube.holds(p) {
                s.outside = true;
            }
            let t = layout.gps_time(r);
            s.gps[0] = s.gps[0].min(t);
            s.gps[1] = s.gps[1].max(t);
            let (ret, _) = layout.returns(r);
            if (1..=15).contains(&ret) {
                s.by_return[usize::from(ret) - 1] += 1;
            }
            s.count += 1;
            let key = self.plan.key_at(p, self.plan.bin_depth);
            *s.bins.entry(key).or_insert(0) += 1;
            let buf = self.buffers.entry(key).or_default();
            buf.extend_from_slice(r);
            buf.extend_from_slice(&(first + i as u64).to_le_bytes());
            if buf.len() >= BIN_FLUSH {
                out.push((key, std::mem::take(buf)));
            }
        }
        out
    }

    /// The bins' last bytes and what the pass found.
    pub fn finish(self) -> (Vec<(Key, Vec<u8>)>, Summary) {
        let mut rest: Vec<(Key, Vec<u8>)> = self
            .buffers
            .into_iter()
            .filter(|(_, b)| !b.is_empty())
            .collect();
        rest.sort_by_key(|(k, _)| *k);
        let mut s = self.summary;
        if s.count == 0 {
            s.bounds = [0.0; 6];
            s.gps = [0.0, 0.0];
        }
        (rest, s)
    }
}

/// A built node: its chunk (empty when it keeps no points).
#[derive(Clone, Debug)]
pub struct Chunk {
    pub key: Key,
    pub count: u32,
    pub bytes: Vec<u8>,
}

/// What a bin's build gives.
#[derive(Clone, Debug)]
pub enum BinOut {
    /// Its nodes' chunks, in the order built, and the points its root gives its parent.
    Built { chunks: Vec<Chunk>, up: Vec<u8> },
    /// It held too many points: these bins in its place.
    Split(Vec<(Key, Vec<u8>)>),
}

/// A binned point's place in the source.
#[inline]
fn source_index(entry: &[u8], rec: usize) -> u64 {
    let mut a = [0u8; 8];
    a.copy_from_slice(&entry[rec..rec + 8]);
    u64::from_le_bytes(a)
}

/// Builds a bin: its subtree's chunks and its root's gift to its parent.
pub fn build_bin(plan: &Plan, laz: &LazVlr, key: Key, bytes: &[u8]) -> Result<BinOut> {
    let stride = plan.stride();
    if !bytes.len().is_multiple_of(stride) {
        return Err(PcError::new(
            "Dizinin bir kutusu bozuk: bayt sayısı noktaya bölünmüyor.",
        ));
    }
    let n = bytes.len() / stride;
    if n > plan.bin_most && key.d < MAX_DEPTH {
        let mut parts: BTreeMap<Key, Vec<u8>> = BTreeMap::new();
        for e in bytes.chunks_exact(stride) {
            let k = plan.key_at(plan.world(e), key.d + 1);
            parts.entry(k).or_default().extend_from_slice(e);
        }
        return Ok(BinOut::Split(parts.into_iter().collect()));
    }
    let entries: Vec<&[u8]> = bytes.chunks_exact(stride).collect();
    let mut chunks = Vec::new();
    let idx: Vec<u32> = (0..n as u32).collect();
    let up = subtree(plan, laz, key, idx, &entries, &mut chunks)?;
    let mut up_bytes = Vec::with_capacity(up.len() * stride);
    for i in up {
        up_bytes.extend_from_slice(entries[i as usize]);
    }
    Ok(BinOut::Built {
        chunks,
        up: up_bytes,
    })
}

/// Builds the subtree of `key` over the points `idx` (in their order): its
/// chunks pushed onto `out` (children first), the points it gives its parent returned.
fn subtree(
    plan: &Plan,
    laz: &LazVlr,
    key: Key,
    idx: Vec<u32>,
    entries: &[&[u8]],
    out: &mut Vec<Chunk>,
) -> Result<Vec<u32>> {
    let kept = if idx.len() > plan.leaf_most && key.d < MAX_DEPTH {
        let (low, side) = plan.cube.node(key);
        let mid = [
            low[0] + side / 2.0,
            low[1] + side / 2.0,
            low[2] + side / 2.0,
        ];
        let mut parts: [Vec<u32>; 8] = Default::default();
        for i in idx {
            let p = plan.world(entries[i as usize]);
            let o = usize::from(p[0] >= mid[0])
                | (usize::from(p[1] >= mid[1]) << 1)
                | (usize::from(p[2] >= mid[2]) << 2);
            parts[o].push(i);
        }
        let mut kept = Vec::new();
        for (o, part) in parts.into_iter().enumerate() {
            if part.is_empty() {
                continue;
            }
            kept.extend(subtree(plan, laz, key.child(o as u8), part, entries, out)?);
        }
        kept
    } else {
        idx
    };
    let (up, own) = give(plan, key, &kept, |i| entries[i as usize]);
    out.push(chunk(
        plan,
        laz,
        key,
        own.iter().map(|&i| entries[i as usize]),
    )?);
    Ok(up)
}

/// Splits a node's points into those it gives its parent (per filled cell of
/// the parent's grid, the one nearest the cell's centre; ties to the earlier
/// in the source; in the cells' order) and those it keeps (in their order).
/// The root gives nothing.
fn give<'a, T: Copy>(
    plan: &Plan,
    key: Key,
    pts: &[T],
    entry: impl Fn(T) -> &'a [u8],
) -> (Vec<T>, Vec<T>) {
    let Some(parent) = key.parent() else {
        return (Vec::new(), pts.to_vec());
    };
    let (low, side) = plan.cube.node(parent);
    let cell = side / f64::from(GRID);
    let g = i64::from(GRID);
    let rec = plan.layout().len;
    // The best per cell: (distance², source index, position in `pts`).
    let mut best: HashMap<u32, (f64, u64, usize)> = HashMap::new();
    for (at, &t) in pts.iter().enumerate() {
        let e = entry(t);
        let p = plan.world(e);
        let i = cell_of(p[0], low[0], cell, g);
        let j = cell_of(p[1], low[1], cell, g);
        let k = cell_of(p[2], low[2], cell, g);
        let c = [
            low[0] + (i as f64 + 0.5) * cell,
            low[1] + (j as f64 + 0.5) * cell,
            low[2] + (k as f64 + 0.5) * cell,
        ];
        let d = (p[0] - c[0]) * (p[0] - c[0])
            + (p[1] - c[1]) * (p[1] - c[1])
            + (p[2] - c[2]) * (p[2] - c[2]);
        let id = (i + g * (j + g * k)) as u32;
        let src = source_index(e, rec);
        match best.get_mut(&id) {
            Some(b) if d < b.0 || (d == b.0 && src < b.1) => *b = (d, src, at),
            Some(_) => {}
            None => {
                best.insert(id, (d, src, at));
            }
        }
    }
    let mut cells: Vec<(u32, usize)> = best.into_iter().map(|(c, (_, _, at))| (c, at)).collect();
    cells.sort_unstable_by_key(|&(c, _)| c);
    let mut taken = vec![false; pts.len()];
    let up: Vec<T> = cells
        .iter()
        .map(|&(_, at)| {
            taken[at] = true;
            pts[at]
        })
        .collect();
    let own = pts
        .iter()
        .zip(&taken)
        .filter(|(_, t)| !**t)
        .map(|(p, _)| *p)
        .collect();
    (up, own)
}

/// A node's chunk from its points' entries (record and source index).
fn chunk<'a>(
    plan: &Plan,
    laz: &LazVlr,
    key: Key,
    own: impl Iterator<Item = &'a [u8]>,
) -> Result<Chunk> {
    let rec = plan.layout().len;
    let mut records = Vec::new();
    let mut count = 0u32;
    for e in own {
        records.extend_from_slice(&e[..rec]);
        count += 1;
    }
    let bytes = if count > 0 {
        chunks::compress(laz.items(), &records)?
    } else {
        Vec::new()
    };
    Ok(Chunk { key, count, bytes })
}

/// The second pass and the file's frame: chunks placed, the nodes above the bins, the header, table and hierarchy.
#[derive(Debug)]
pub struct Builder {
    plan: Plan,
    laz: LazVlr,
    summary: Summary,
    data_offset: u32,
    /// Where the next chunk goes.
    at: u64,
    nodes: Vec<Node>,
    /// Chunks' point and byte counts, in the file's order (the chunk table).
    table: Vec<(u64, u64)>,
    /// What the bins' roots gave, by the bin root's key.
    gifts: BTreeMap<Key, Vec<u8>>,
}

/// The bytes to put where, once the index is done.
#[derive(Clone, Debug)]
pub struct Finish {
    /// The nodes above the bins, to append in this order.
    pub chunks: Vec<Chunk>,
    /// Then this: the chunk table and the hierarchy EVLR.
    pub tail: Vec<u8>,
    /// And these over what was written: the header and VLRs, the chunk table's offset.
    pub patches: Vec<(u64, Vec<u8>)>,
    /// The hierarchy as written.
    pub nodes: Vec<Node>,
    pub info: Info,
}

impl Builder {
    pub fn new(plan: Plan, summary: Summary) -> Result<Builder> {
        let laz = chunks::vlr_for(plan.format, plan.extra)?;
        let data_offset = plan.data_offset(&laz)?;
        Ok(Builder {
            at: u64::from(data_offset) + 8,
            data_offset,
            plan,
            laz,
            summary,
            nodes: Vec::new(),
            table: Vec::new(),
            gifts: BTreeMap::new(),
        })
    }

    pub fn laz(&self) -> &LazVlr {
        &self.laz
    }

    pub fn plan(&self) -> &Plan {
        &self.plan
    }

    /// The file's first bytes, written before any chunk: header and VLRs (filled in at the
    /// end) and the 8 bytes of the chunk table's offset.
    pub fn head(&self) -> Result<Vec<u8>> {
        let mut b = self.header_bytes(&Info::empty(), 0)?;
        b.extend_from_slice(&[0u8; 8]);
        Ok(b)
    }

    /// Takes a built bin's chunks (the host appends their bytes in this order) and its root's gift.
    pub fn accept(&mut self, key: Key, chunks: &[Chunk], up: Vec<u8>) {
        for c in chunks {
            self.place(c);
        }
        self.gifts.insert(key, up);
    }

    fn place(&mut self, c: &Chunk) {
        let bytes = c.bytes.len() as u64;
        self.nodes.push(Node {
            key: c.key,
            offset: if c.count > 0 { self.at } else { 0 },
            bytes: if c.count > 0 { bytes } else { 0 },
            count: u64::from(c.count),
        });
        if c.count > 0 {
            self.table.push((u64::from(c.count), bytes));
            self.at += bytes;
        }
    }

    /// The nodes above the bins, then the frame.
    pub fn finish(mut self) -> Result<Finish> {
        let plan = self.plan.clone();
        let stride = plan.stride();
        // Every ancestor of a bin root, deepest first.
        let mut upper: BTreeMap<Key, ()> = BTreeMap::new();
        for k in self.gifts.keys() {
            let mut p = k.parent();
            while let Some(q) = p {
                upper.insert(q, ());
                p = q.parent();
            }
        }
        let mut order: Vec<Key> = upper.into_keys().collect();
        order.sort_by(|a, b| b.d.cmp(&a.d).then(a.cmp(b)));
        let mut gifts = std::mem::take(&mut self.gifts);
        let mut built = Vec::new();
        for key in order {
            let mut kept: Vec<u8> = Vec::new();
            for o in 0..8u8 {
                if let Some(g) = gifts.remove(&key.child(o)) {
                    kept.extend_from_slice(&g);
                }
            }
            let entries: Vec<&[u8]> = kept.chunks_exact(stride).collect();
            let idx: Vec<u32> = (0..entries.len() as u32).collect();
            let (up, own) = give(&plan, key, &idx, |i| entries[i as usize]);
            let c = chunk(
                &plan,
                &self.laz,
                key,
                own.iter().map(|&i| entries[i as usize]),
            )?;
            self.place(&c);
            built.push(c);
            let mut g = Vec::with_capacity(up.len() * stride);
            for i in up {
                g.extend_from_slice(entries[i as usize]);
            }
            gifts.insert(key, g);
        }
        // The hierarchy, by key: the nodes with points and every node above them (a node
        // without points of its own leads to its children; one leading nowhere is left out).
        let mut wanted: std::collections::HashSet<Key> = std::collections::HashSet::new();
        for n in self.nodes.iter().filter(|n| n.count > 0) {
            let mut k = Some(n.key);
            while let Some(q) = k {
                if !wanted.insert(q) {
                    break;
                }
                k = q.parent();
            }
        }
        let mut nodes: Vec<Node> = self
            .nodes
            .iter()
            .filter(|n| wanted.contains(&n.key))
            .copied()
            .collect();
        nodes.sort_by_key(|n| n.key);
        if nodes.len() > copc::MAX_NODES {
            return Err(PcError::new(format!(
                "Dizin {} düğüm oldu; en çok {} düğüm yazılır.",
                nodes.len(),
                copc::MAX_NODES
            )));
        }
        let table_at = self.at;
        let mut tail = chunks::table_bytes(&self.laz, &self.table)?;
        let evlr_start = table_at + tail.len() as u64;
        let page = copc::page_bytes(&nodes);
        let s = &self.summary;
        let info = Info {
            center: plan.cube.center,
            halfsize: plan.cube.half,
            spacing: 2.0 * plan.cube.half / f64::from(GRID),
            root_offset: evlr_start + las::EVLR_HEAD as u64,
            root_size: page.len() as u64,
            gps_min: s.gps[0],
            gps_max: s.gps[1],
        };
        let mut hierarchy = Vlr::new(
            copc::COPC_USER,
            copc::HIERARCHY_RECORD,
            "EPT hierarchy",
            page,
        );
        hierarchy.extended = true;
        tail.extend_from_slice(&hierarchy.to_bytes());
        let header = self.header_bytes(&info, evlr_start)?;
        let mut offset = [0u8; 8];
        put_i64(&mut offset, 0, table_at as i64);
        Ok(Finish {
            chunks: built,
            tail,
            patches: vec![(0, header), (u64::from(self.data_offset), offset.to_vec())],
            nodes,
            info,
        })
    }

    /// The header and VLRs for `info`, the EVLRs starting at `evlr_start`.
    fn header_bytes(&self, info: &Info, evlr_start: u64) -> Result<Vec<u8>> {
        let s = &self.summary;
        let p = &self.plan;
        let head = Header {
            major: 1,
            minor: 4,
            source_id: p.source_id,
            global_encoding: (if p.gps_standard {
                las::ENCODING_GPS_STANDARD
            } else {
                0
            }) | if p.wkt.is_some() {
                las::ENCODING_WKT
            } else {
                0
            },
            guid: [0; 16],
            system: "KentOS CAD".into(),
            software: "KentOS CAD COPC".into(),
            day: 0,
            year: 0,
            header_size: las::HEADER_1_4 as u16,
            data_offset: self.data_offset,
            vlr_count: 0,
            format: p.format,
            compressed: true,
            record_len: p.layout().len as u16,
            count: s.count,
            by_return: s.by_return,
            scale: p.scale,
            offset: p.offset,
            min: [s.bounds[0], s.bounds[1], s.bounds[2]],
            max: [s.bounds[3], s.bounds[4], s.bounds[5]],
            waveform_start: 0,
            evlr_start,
            evlr_count: 1,
        };
        let vlrs = p.vlrs(&self.laz, info)?;
        let mut head = head;
        head.vlr_count = vlrs.len() as u32;
        let mut b = head.to_bytes();
        for v in vlrs {
            b.extend_from_slice(&v.to_bytes());
        }
        if b.len() != self.data_offset as usize {
            return Err(PcError::new("Dizinin başlığı beklenen boyda değil."));
        }
        Ok(b)
    }
}

/// A whole index in memory, for records of the plan's format (tests and small clouds):
/// the file's bytes and its hierarchy.
pub fn build_in_memory(plan: Plan, records: &[u8]) -> Result<(Vec<u8>, Vec<Node>)> {
    let mut plan = plan;
    for attempt in 0..2 {
        let mut dist = Distribute::new(plan.clone());
        let mut bins: BTreeMap<Key, Vec<u8>> = BTreeMap::new();
        for (k, b) in dist.add(records, 0) {
            bins.entry(k).or_default().extend(b);
        }
        let (rest, summary) = dist.finish();
        for (k, b) in rest {
            bins.entry(k).or_default().extend(b);
        }
        // Once round the points' own bounds when the header's were wrong; not again.
        if summary.outside && attempt == 0 {
            plan.cube = Cube::of(summary.bounds);
            continue;
        }
        let mut builder = Builder::new(plan.clone(), summary)?;
        let mut file = builder.head()?;
        let laz = builder.laz().clone();
        let mut queue: Vec<(Key, Vec<u8>)> = bins.into_iter().collect();
        while let Some((key, bytes)) = (!queue.is_empty()).then(|| queue.remove(0)) {
            match build_bin(&plan, &laz, key, &bytes)? {
                BinOut::Built { chunks, up } => {
                    for c in &chunks {
                        file.extend_from_slice(&c.bytes);
                    }
                    builder.accept(key, &chunks, up);
                }
                BinOut::Split(parts) => {
                    for (i, p) in parts.into_iter().enumerate() {
                        queue.insert(i, p);
                    }
                }
            }
        }
        let fin = builder.finish()?;
        for c in &fin.chunks {
            file.extend_from_slice(&c.bytes);
        }
        file.extend_from_slice(&fin.tail);
        for (at, b) in &fin.patches {
            let at = *at as usize;
            file[at..at + b.len()].copy_from_slice(b);
        }
        return Ok((file, fin.nodes));
    }
    Err(PcError::new("Noktalar dizinin kübüne sığmadı."))
}
