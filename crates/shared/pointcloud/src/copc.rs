//! COPC 1.0 (copc.io, docs/adr/0207 §2, §4): the info VLR (`copc`/1, 160
//! bytes, the first VLR), the octree's keys and the hierarchy's pages
//! (`copc`/1000, 32-byte entries). A node's key is EPT's (depth d and x, y, z
//! below 2ᵈ); its cube is the root cube's 2⁻ᵈ part.

use std::collections::HashMap;

use crate::bytes::{Read, Write};
use crate::las::Vlr;
use crate::{PcError, Result};

pub const COPC_USER: &str = "copc";
pub const INFO_RECORD: u16 = 1;
pub const HIERARCHY_RECORD: u16 = 1000;
pub const INFO_BYTES: usize = 160;
pub const ENTRY_BYTES: usize = 32;
/// The most nodes a hierarchy may hold (docs/adr/0207 §2).
pub const MAX_NODES: usize = 1_000_000;
/// The deepest level read.
pub const MAX_DEPTH: i32 = 24;

/// The info VLR's fields.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Info {
    pub center: [f64; 3],
    pub halfsize: f64,
    pub spacing: f64,
    pub root_offset: u64,
    pub root_size: u64,
    pub gps_min: f64,
    pub gps_max: f64,
}

impl Info {
    pub fn read(data: &[u8]) -> Result<Info> {
        if data.len() < INFO_BYTES {
            return Err(PcError::new("COPC bilgi VLR'si 160 bayttan kısa."));
        }
        let mut r = Read::new(data);
        let center = [r.f64()?, r.f64()?, r.f64()?];
        let halfsize = r.f64()?;
        let spacing = r.f64()?;
        let root_offset = r.u64()?;
        let root_size = r.u64()?;
        let gps_min = r.f64()?;
        let gps_max = r.f64()?;
        if !(center.iter().all(|v| v.is_finite())
            && halfsize.is_finite()
            && halfsize > 0.0
            && spacing.is_finite()
            && spacing > 0.0)
        {
            return Err(PcError::new(
                "COPC'nin kübü ya da nokta aralığı geçersiz (sonlu ve sıfırdan büyük olmalı).",
            ));
        }
        Ok(Info {
            center,
            halfsize,
            spacing,
            root_offset,
            root_size,
            gps_min,
            gps_max,
        })
    }

    pub fn to_vlr(&self) -> Vlr {
        let mut w = Write::new();
        for v in self.center {
            w.f64(v);
        }
        w.f64(self.halfsize).f64(self.spacing);
        w.u64(self.root_offset).u64(self.root_size);
        w.f64(self.gps_min).f64(self.gps_max);
        for _ in 0..11 {
            w.u64(0);
        }
        Vlr::new(COPC_USER, INFO_RECORD, "COPC info VLR", w.bytes)
    }

    /// The root cube's least corner.
    pub fn low(&self) -> [f64; 3] {
        [
            self.center[0] - self.halfsize,
            self.center[1] - self.halfsize,
            self.center[2] - self.halfsize,
        ]
    }
}

/// An octree node's key: depth and place at that depth.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Key {
    pub d: i32,
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl Key {
    pub const ROOT: Key = Key {
        d: 0,
        x: 0,
        y: 0,
        z: 0,
    };

    /// Its child in octant `o` (bit 0 x, bit 1 y, bit 2 z).
    pub fn child(&self, o: u8) -> Key {
        Key {
            d: self.d + 1,
            x: self.x * 2 + i32::from(o & 1),
            y: self.y * 2 + i32::from((o >> 1) & 1),
            z: self.z * 2 + i32::from((o >> 2) & 1),
        }
    }

    pub fn parent(&self) -> Option<Key> {
        (self.d > 0).then(|| Key {
            d: self.d - 1,
            x: self.x >> 1,
            y: self.y >> 1,
            z: self.z >> 1,
        })
    }

    /// Its octant in its parent.
    pub fn octant(&self) -> u8 {
        ((self.x & 1) | ((self.y & 1) << 1) | ((self.z & 1) << 2)) as u8
    }

    /// Whether the key lies inside its level's grid.
    pub fn valid(&self) -> bool {
        (0..=MAX_DEPTH).contains(&self.d)
            && [self.x, self.y, self.z]
                .iter()
                .all(|&v| v >= 0 && (v as i64) < (1i64 << self.d))
    }

    /// The node's side for a root cube of half side `halfsize`.
    pub fn side(&self, halfsize: f64) -> f64 {
        2.0 * halfsize / (1u64 << self.d) as f64
    }

    /// The node's least corner and side.
    pub fn cube(&self, info: &Info) -> ([f64; 3], f64) {
        let side = self.side(info.halfsize);
        let low = info.low();
        (
            [
                low[0] + f64::from(self.x) * side,
                low[1] + f64::from(self.y) * side,
                low[2] + f64::from(self.z) * side,
            ],
            side,
        )
    }
}

/// A hierarchy entry: a node's chunk, a page below, or a node without points.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entry {
    pub key: Key,
    pub offset: u64,
    pub bytes: i32,
    /// > 0 points in the chunk, −1 a page, 0 no points of its own.
    pub count: i32,
}

impl Entry {
    pub fn write(&self, w: &mut Write) {
        w.i32(self.key.d)
            .i32(self.key.x)
            .i32(self.key.y)
            .i32(self.key.z);
        w.u64(self.offset).i32(self.bytes).i32(self.count);
    }
}

/// One page's entries.
pub fn page(bytes: &[u8]) -> Result<Vec<Entry>> {
    if !bytes.len().is_multiple_of(ENTRY_BYTES) {
        return Err(PcError::new(
            "COPC hiyerarşi sayfası 32 baytlık girdilere bölünmüyor.",
        ));
    }
    let mut r = Read::new(bytes);
    let mut out = Vec::with_capacity(bytes.len() / ENTRY_BYTES);
    while r.at() < bytes.len() {
        let key = Key {
            d: r.i32()?,
            x: r.i32()?,
            y: r.i32()?,
            z: r.i32()?,
        };
        let offset = r.u64()?;
        let size = r.i32()?;
        let count = r.i32()?;
        if !key.valid() {
            return Err(PcError::new(format!(
                "COPC hiyerarşisinde geçersiz düğüm: {}-{}-{}-{}.",
                key.d, key.x, key.y, key.z
            )));
        }
        if count < -1 || (count != 0 && size <= 0) {
            return Err(PcError::new(
                "COPC hiyerarşisinin bir girdisinin boyu ya da nokta sayısı geçersiz.",
            ));
        }
        out.push(Entry {
            key,
            offset,
            bytes: size,
            count,
        });
    }
    Ok(out)
}

/// A node with its chunk (or none) and cube.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Node {
    pub key: Key,
    /// Where its chunk is; 0 and 0 when it has no points of its own.
    pub offset: u64,
    pub bytes: u64,
    pub count: u64,
}

/// The whole hierarchy: nodes by key, the pages read so far.
#[derive(Clone, Debug, Default)]
pub struct Hierarchy {
    pub nodes: HashMap<Key, Node>,
    /// Pages still to read: offset and size.
    pub pending: Vec<(u64, u64)>,
    pages_read: usize,
}

impl Hierarchy {
    pub fn new(info: &Info) -> Hierarchy {
        Hierarchy {
            nodes: HashMap::new(),
            pending: vec![(info.root_offset, info.root_size)],
            pages_read: 0,
        }
    }

    /// Takes a page's bytes; the pages it points to join the pending ones.
    pub fn take_page(&mut self, at: (u64, u64), bytes: &[u8], file_size: u64) -> Result<()> {
        self.pending.retain(|p| *p != at);
        self.pages_read += 1;
        if self.pages_read > MAX_NODES {
            return Err(PcError::new("COPC hiyerarşisi çok büyük ya da döngülü."));
        }
        for e in page(bytes)? {
            match e.count {
                -1 => {
                    let p = (e.offset, e.bytes as u64);
                    if p.0.checked_add(p.1).is_none_or(|end| end > file_size) {
                        return Err(PcError::new(
                            "COPC'nin bir hiyerarşi sayfası dosyanın dışında.",
                        ));
                    }
                    self.pending.push(p);
                }
                c => {
                    let bytes = if c > 0 { e.bytes as u64 } else { 0 };
                    if c > 0
                        && e.offset
                            .checked_add(bytes)
                            .is_none_or(|end| end > file_size)
                    {
                        return Err(PcError::new("COPC'nin bir düğümü dosyanın dışında."));
                    }
                    self.nodes.insert(
                        e.key,
                        Node {
                            key: e.key,
                            offset: if c > 0 { e.offset } else { 0 },
                            bytes,
                            count: c.max(0) as u64,
                        },
                    );
                }
            }
            if self.nodes.len() > MAX_NODES {
                return Err(PcError::new(format!(
                    "COPC hiyerarşisinde en çok {MAX_NODES} düğüm okunur."
                )));
            }
        }
        Ok(())
    }

    /// The points every node holds.
    pub fn count(&self) -> u64 {
        self.nodes.values().map(|n| n.count).sum()
    }

    /// Nodes in a fixed order: by depth, then x, y, z.
    pub fn sorted(&self) -> Vec<Node> {
        let mut v: Vec<Node> = self.nodes.values().copied().collect();
        v.sort_by_key(|n| n.key);
        v
    }
}

/// A single page of entries for these nodes (the index writes its hierarchy so).
pub fn page_bytes(nodes: &[Node]) -> Vec<u8> {
    let mut w = Write::new();
    for n in nodes {
        Entry {
            key: n.key,
            offset: n.offset,
            bytes: n.bytes as i32,
            count: n.count as i32,
        }
        .write(&mut w);
    }
    w.bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_go_down_and_up() {
        let k = Key::ROOT.child(5).child(2);
        assert_eq!(
            k,
            Key {
                d: 2,
                x: 2,
                y: 1,
                z: 2
            }
        );
        assert_eq!(k.octant(), 2);
        assert_eq!(k.parent(), Some(Key::ROOT.child(5)));
        assert!(k.valid());
        assert!(
            !Key {
                d: 1,
                x: 2,
                y: 0,
                z: 0
            }
            .valid()
        );
    }

    #[test]
    fn info_round_trips() {
        let i = Info {
            center: [10.0, 20.0, 30.0],
            halfsize: 50.0,
            spacing: 100.0 / 128.0,
            root_offset: 1000,
            root_size: 64,
            gps_min: 1.5,
            gps_max: 2.5,
        };
        let v = i.to_vlr();
        assert_eq!(v.data.len(), INFO_BYTES);
        assert_eq!(Info::read(&v.data).unwrap(), i);
        let (low, side) = Key::ROOT.child(7).cube(&i);
        assert_eq!(low, [10.0, 20.0, 30.0]);
        assert_eq!(side, 50.0);
    }

    #[test]
    fn pages_point_to_pages() {
        let mut w = Write::new();
        Entry {
            key: Key::ROOT,
            offset: 500,
            bytes: 40,
            count: 10,
        }
        .write(&mut w);
        Entry {
            key: Key::ROOT.child(0),
            offset: 900,
            bytes: 32,
            count: -1,
        }
        .write(&mut w);
        let info = Info {
            center: [0.0; 3],
            halfsize: 1.0,
            spacing: 1.0,
            root_offset: 800,
            root_size: 64,
            gps_min: 0.0,
            gps_max: 0.0,
        };
        let mut h = Hierarchy::new(&info);
        h.take_page((800, 64), &w.bytes, 2000).unwrap();
        assert_eq!(h.pending, vec![(900, 32)]);
        assert_eq!(h.count(), 10);
        let mut w2 = Write::new();
        Entry {
            key: Key::ROOT.child(0),
            offset: 540,
            bytes: 20,
            count: 3,
        }
        .write(&mut w2);
        h.take_page((900, 32), &w2.bytes, 2000).unwrap();
        assert!(h.pending.is_empty());
        assert_eq!(h.count(), 13);
        assert!(h.take_page((0, 0), &[0u8; 5], 2000).is_err());
    }
}
