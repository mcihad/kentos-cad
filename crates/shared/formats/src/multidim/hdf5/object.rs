//! An HDF5 object's header (docs/adr/0645 §2): its messages (version 1 and 2 headers, continuation blocks, shared
//! messages followed) and the messages a NetCDF-4 reader needs decoded: dataspace, datatype, fill value, layout,
//! filter pipeline, attribute, link, link and attribute info, symbol table.

use std::collections::HashSet;

use super::io::{Cur, H5};

/// A header message.
#[derive(Clone, Debug)]
pub struct Msg {
    pub kind: u16,
    pub flags: u8,
    pub data: Vec<u8>,
}

pub const DATASPACE: u16 = 0x01;
pub const LINK_INFO: u16 = 0x02;
pub const DATATYPE: u16 = 0x03;
pub const FILL_OLD: u16 = 0x04;
pub const FILL: u16 = 0x05;
pub const LINK: u16 = 0x06;
pub const LAYOUT: u16 = 0x08;
pub const FILTERS: u16 = 0x0B;
pub const ATTRIBUTE: u16 = 0x0C;
pub const CONTINUATION: u16 = 0x10;
pub const SYMBOL_TABLE: u16 = 0x11;
pub const ATTRIBUTE_INFO: u16 = 0x15;

const MESSAGES_MOST: usize = 65_536;

/// The messages of the object whose header is at `addr` (shared ones followed to their object).
pub fn messages(h: &H5, addr: u64) -> Result<Vec<Msg>, String> {
    let mut out = raw_messages(h, addr)?;
    for m in &mut out {
        if m.flags & 0x02 != 0 {
            m.data = shared(h, m.kind, &m.data)?;
            m.flags &= !0x02;
        }
    }
    Ok(out)
}

/// A shared message's own data: the message of its kind in the object the reference names.
fn shared(h: &H5, kind: u16, data: &[u8]) -> Result<Vec<u8>, String> {
    let (o, l) = (h.o, h.l);
    let mut c = Cur::new(data.to_vec(), o, l);
    let version = c.u8()?;
    let typ = c.u8()?;
    if version == 1 {
        c.skip(6)?;
    }
    if version >= 3 && typ == 1 {
        return Err(
            "HDF5 dosyası paylaşılan iletileri bir yığında tutuyor (SOHM); bu okunmuyor.".into(),
        );
    }
    let at = c
        .addr()?
        .ok_or("HDF5'in paylaşılan iletisi boş bir adres gösteriyor.")?;
    raw_messages(h, at)?
        .into_iter()
        .find(|m| m.kind == kind)
        .map(|m| m.data)
        .ok_or_else(|| "HDF5'in paylaşılan iletisi bulunamadı.".into())
}

fn raw_messages(h: &H5, addr: u64) -> Result<Vec<Msg>, String> {
    let mut head = h.at(addr, 16)?;
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    // Continuation blocks still to read: (address, length).
    let mut blocks: Vec<(u64, u64)> = Vec::new();
    if head.b.starts_with(b"OHDR") {
        head.skip(4)?;
        let version = head.u8()?;
        if version != 2 {
            return Err(format!(
                "HDF5 nesne başlığının sürümü {version}; 2 bekleniyordu."
            ));
        }
        let flags = head.u8()?;
        let mut extra = 0usize;
        if flags & 0x20 != 0 {
            extra += 16;
        }
        if flags & 0x10 != 0 {
            extra += 4;
        }
        let size_len = 1usize << (flags & 3);
        let mut c = h.at(addr, 6 + extra + size_len)?;
        c.skip(6 + extra)?;
        let chunk0 = c.uvar(size_len)?;
        let start = addr + (6 + extra + size_len) as u64;
        let order = flags & 0x04 != 0;
        let body = h.at(start, chunk0 as usize)?;
        v2_messages(body, order, &mut out, &mut blocks)?;
        while let Some((at, len)) = blocks.pop() {
            if !seen.insert(at) || out.len() > MESSAGES_MOST {
                continue;
            }
            let mut c = h.at(at, len as usize)?;
            c.sig(b"OCHK", "nesne başlığının devamı")?;
            let n = (len as usize).saturating_sub(8);
            let body = Cur::new(c.bytes(n)?.to_vec(), h.o, h.l);
            v2_messages(body, order, &mut out, &mut blocks)?;
        }
    } else {
        let version = head.u8()?;
        if version != 1 {
            return Err(format!(
                "HDF5 nesne başlığı tanınmıyor (sürüm {version}); dosya bozuk olabilir."
            ));
        }
        head.skip(1)?;
        let total = head.u16()? as usize;
        head.skip(4)?;
        let size = head.u32()? as u64;
        blocks.push((addr + 16, size));
        while let Some((at, len)) = blocks.pop() {
            if !seen.insert(at) || out.len() >= total.max(1) + 64 || out.len() > MESSAGES_MOST {
                continue;
            }
            let mut c = h.at(at, len as usize)?;
            while c.left() >= 8 {
                let kind = c.u16()?;
                let n = c.u16()? as usize;
                let flags = c.u8()?;
                c.skip(3)?;
                let data = c.bytes(n.min(c.left()))?.to_vec();
                if kind == CONTINUATION {
                    let mut d = Cur::new(data, h.o, h.l);
                    if let Some(a) = d.addr()? {
                        blocks.push((a, d.length()?));
                    }
                } else if kind != 0 {
                    out.push(Msg { kind, flags, data });
                }
            }
        }
    }
    Ok(out)
}

fn v2_messages(
    mut c: Cur,
    order: bool,
    out: &mut Vec<Msg>,
    blocks: &mut Vec<(u64, u64)>,
) -> Result<(), String> {
    let head = if order { 6 } else { 4 };
    while c.left() >= head {
        let kind = u16::from(c.u8()?);
        let n = c.u16()? as usize;
        let flags = c.u8()?;
        if order {
            c.skip(2)?;
        }
        if n > c.left() {
            break;
        }
        let data = c.bytes(n)?.to_vec();
        if kind == CONTINUATION {
            let (o, l) = c.sizes();
            let mut d = Cur::new(data, o, l);
            if let Some(a) = d.addr()? {
                blocks.push((a, d.length()?));
            }
        } else if kind != 0 {
            out.push(Msg { kind, flags, data });
        }
    }
    Ok(())
}

/// A datatype as far as a reader of numbers and text needs it.
#[derive(Clone, Debug, PartialEq)]
pub enum DType {
    Int {
        size: usize,
        signed: bool,
        little: bool,
    },
    Float {
        size: usize,
        little: bool,
    },
    /// Fixed-length text of `size` bytes.
    Text {
        size: usize,
    },
    /// Variable-length text.
    VText,
    /// An object reference.
    Ref {
        size: usize,
    },
    /// A variable-length sequence of the base type.
    Seq(Box<DType>),
    /// An enumeration: values of its base integer type.
    Enum(Box<DType>),
    Other {
        class: u8,
        size: usize,
    },
}

impl DType {
    pub fn size(&self) -> usize {
        match self {
            DType::Int { size, .. } | DType::Float { size, .. } | DType::Text { size } => *size,
            DType::Ref { size } | DType::Other { size, .. } => *size,
            DType::VText | DType::Seq(_) => 0,
            DType::Enum(b) => b.size(),
        }
    }

    /// The name a refusal says.
    pub fn name(&self) -> String {
        match self {
            DType::Int { size, signed, .. } => {
                format!("{}int{}", if *signed { "" } else { "u" }, size * 8)
            }
            DType::Float { size, .. } => format!("float{}", size * 8),
            DType::Text { .. } | DType::VText => "metin".into(),
            DType::Ref { .. } => "başvuru".into(),
            DType::Seq(_) => "değişken uzunlukta dizi".into(),
            DType::Enum(_) => "enum".into(),
            DType::Other { class, .. } => match class {
                2 => "zaman".into(),
                4 => "bit alanı".into(),
                5 => "opak".into(),
                6 => "bileşik (compound)".into(),
                10 => "dizi".into(),
                c => format!("sınıf {c}"),
            },
        }
    }
}

/// A datatype message.
pub fn datatype(c: &mut Cur) -> Result<DType, String> {
    let cv = c.u8()?;
    let (class, version) = (cv & 0x0F, cv >> 4);
    let b0 = c.u8()?;
    let b1 = c.u8()?;
    let _b2 = c.u8()?;
    let size = c.u32()? as usize;
    Ok(match class {
        0 => {
            c.skip(4)?;
            DType::Int {
                size,
                signed: b0 & 0x08 != 0,
                little: b0 & 0x01 == 0,
            }
        }
        1 => {
            c.skip(12)?;
            if b0 & 0x40 != 0 {
                return Ok(DType::Other { class: 1, size });
            }
            DType::Float {
                size,
                little: b0 & 0x01 == 0,
            }
        }
        3 => DType::Text { size },
        7 => DType::Ref { size },
        8 => {
            let base = datatype(c)?;
            // The members' names and values follow; the values are of the base type.
            let _ = version;
            DType::Enum(Box::new(base))
        }
        9 => {
            let base = datatype(c)?;
            if b0 & 0x0F == 1 {
                DType::VText
            } else {
                let _ = b1;
                DType::Seq(Box::new(base))
            }
        }
        _ => DType::Other { class, size },
    })
}

/// A dataspace: its dimensions and their limits (`u64::MAX` unlimited); none dimensions for a scalar; `null` for a
/// dataspace without values.
#[derive(Clone, Debug, PartialEq)]
pub struct Space {
    pub dims: Vec<u64>,
    pub max: Vec<u64>,
    pub null: bool,
}

impl Space {
    pub fn count(&self) -> u64 {
        if self.null {
            0
        } else {
            self.dims.iter().product()
        }
    }
}

pub fn dataspace(c: &mut Cur) -> Result<Space, String> {
    let version = c.u8()?;
    let rank = c.u8()? as usize;
    let flags = c.u8()?;
    let mut null = false;
    match version {
        1 => c.skip(5)?,
        2 => null = c.u8()? == 2,
        v => return Err(format!("HDF5 dataspace iletisinin sürümü {v} okunmuyor.")),
    }
    let mut dims = Vec::with_capacity(rank);
    for _ in 0..rank {
        dims.push(c.length()?);
    }
    let mut max = dims.clone();
    if flags & 1 != 0 {
        for m in max.iter_mut() {
            *m = c.length()?;
        }
    }
    Ok(Space { dims, max, null })
}

/// A dataset's layout.
#[derive(Clone, Debug, PartialEq)]
pub enum Layout {
    /// The values in the message itself.
    Compact(Vec<u8>),
    /// One run of the file; none when never written.
    Contiguous {
        address: Option<u64>,
        size: u64,
    },
    /// Chunks of `dims` (the dataspace's rank) found by an index.
    Chunked {
        dims: Vec<u64>,
        index: ChunkIndex,
    },
    Virtual,
}

/// How a chunked dataset's chunks are found.
#[derive(Clone, Debug, PartialEq)]
pub enum ChunkIndex {
    /// A version 1 B-tree (layout message version 3 and older).
    BTree1(Option<u64>),
    /// One chunk, its filtered size and filter mask when filtered.
    Single {
        address: Option<u64>,
        filtered: Option<(u64, u32)>,
    },
    /// Every chunk one after another, unfiltered.
    Implicit(Option<u64>),
    FixedArray {
        address: Option<u64>,
    },
    ExtensibleArray {
        address: Option<u64>,
    },
    BTree2 {
        address: Option<u64>,
    },
}

pub fn layout(c: &mut Cur, rank: usize) -> Result<Layout, String> {
    let version = c.u8()?;
    match version {
        1 | 2 => {
            let nd = c.u8()? as usize;
            let class = c.u8()?;
            c.skip(5)?;
            let address = if class != 0 { c.addr()? } else { None };
            let mut dims = Vec::with_capacity(nd);
            for _ in 0..nd {
                dims.push(u64::from(c.u32()?));
            }
            match class {
                0 => {
                    let n = c.u32()? as usize;
                    Ok(Layout::Compact(c.bytes(n)?.to_vec()))
                }
                1 => {
                    let size: u64 = dims.iter().product();
                    Ok(Layout::Contiguous { address, size })
                }
                _ => {
                    dims.truncate(rank);
                    Ok(Layout::Chunked {
                        dims,
                        index: ChunkIndex::BTree1(address),
                    })
                }
            }
        }
        3 => match c.u8()? {
            0 => {
                let n = c.u16()? as usize;
                Ok(Layout::Compact(c.bytes(n)?.to_vec()))
            }
            1 => {
                let address = c.addr()?;
                let size = c.length()?;
                Ok(Layout::Contiguous { address, size })
            }
            2 => {
                let nd = c.u8()? as usize;
                let address = c.addr()?;
                let mut dims = Vec::with_capacity(nd);
                for _ in 0..nd {
                    dims.push(u64::from(c.u32()?));
                }
                dims.truncate(rank);
                Ok(Layout::Chunked {
                    dims,
                    index: ChunkIndex::BTree1(address),
                })
            }
            _ => Ok(Layout::Virtual),
        },
        4 | 5 => match c.u8()? {
            0 => {
                let n = c.u16()? as usize;
                Ok(Layout::Compact(c.bytes(n)?.to_vec()))
            }
            1 => {
                let address = c.addr()?;
                let size = c.length()?;
                Ok(Layout::Contiguous { address, size })
            }
            2 => {
                let flags = c.u8()?;
                let nd = c.u8()? as usize;
                let enc = c.u8()? as usize;
                let mut dims = Vec::with_capacity(nd);
                for _ in 0..nd {
                    dims.push(c.uvar(enc)?);
                }
                dims.truncate(rank);
                let kind = c.u8()?;
                let index = match kind {
                    1 => {
                        let filtered = if flags & 0x02 != 0 {
                            Some((c.length()?, c.u32()?))
                        } else {
                            None
                        };
                        ChunkIndex::Single {
                            address: c.addr()?,
                            filtered,
                        }
                    }
                    2 => ChunkIndex::Implicit(c.addr()?),
                    3 => {
                        c.skip(1)?;
                        ChunkIndex::FixedArray { address: c.addr()? }
                    }
                    4 => {
                        c.skip(5)?;
                        ChunkIndex::ExtensibleArray { address: c.addr()? }
                    }
                    5 => {
                        c.skip(6)?;
                        ChunkIndex::BTree2 { address: c.addr()? }
                    }
                    k => {
                        return Err(format!("HDF5'in {k} numaralı parça dizini bilinmiyor."));
                    }
                };
                Ok(Layout::Chunked { dims, index })
            }
            _ => Ok(Layout::Virtual),
        },
        v => Err(format!(
            "HDF5 yerleşim (layout) iletisinin sürümü {v} okunmuyor."
        )),
    }
}

/// A filter of the pipeline: its id and client values.
#[derive(Clone, Debug, PartialEq)]
pub struct Filter {
    pub id: u16,
    pub name: String,
    pub values: Vec<u32>,
}

pub fn filters(c: &mut Cur) -> Result<Vec<Filter>, String> {
    let version = c.u8()?;
    let n = c.u8()? as usize;
    if version == 1 {
        c.skip(6)?;
    }
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let id = c.u16()?;
        let name_len = if version == 1 || id >= 256 {
            c.u16()? as usize
        } else {
            0
        };
        let _flags = c.u16()?;
        let nv = c.u16()? as usize;
        let name = if name_len > 0 {
            let raw = c.bytes(name_len)?;
            String::from_utf8_lossy(raw.split(|&b| b == 0).next().unwrap_or(&[])).into_owned()
        } else {
            String::new()
        };
        let mut values = Vec::with_capacity(nv);
        for _ in 0..nv {
            values.push(c.u32()?);
        }
        if version == 1 && nv % 2 == 1 {
            c.skip(4)?;
        }
        out.push(Filter { id, name, values });
    }
    Ok(out)
}

/// A fill value message's value bytes, when defined.
pub fn fill(c: &mut Cur, old: bool) -> Result<Option<Vec<u8>>, String> {
    if old {
        let n = c.u32()? as usize;
        return (n > 0).then(|| c.bytes(n).map(<[u8]>::to_vec)).transpose();
    }
    let version = c.u8()?;
    match version {
        1 | 2 => {
            c.skip(2)?;
            let defined = c.u8()?;
            if version == 2 && defined == 0 {
                return Ok(None);
            }
            let n = c.u32()? as usize;
            Ok((n > 0)
                .then(|| c.bytes(n).map(<[u8]>::to_vec))
                .transpose()?)
        }
        3 => {
            let flags = c.u8()?;
            if flags & 0x20 == 0 {
                return Ok(None);
            }
            let n = c.u32()? as usize;
            Ok(Some(c.bytes(n)?.to_vec()))
        }
        _ => Ok(None),
    }
}

/// An attribute message: name, type, dataspace and the values' bytes.
pub fn attribute(c: &mut Cur) -> Result<(String, DType, Space, Vec<u8>), String> {
    let version = c.u8()?;
    c.skip(1)?;
    let name_len = c.u16()? as usize;
    let dt_len = c.u16()? as usize;
    let ds_len = c.u16()? as usize;
    if version == 3 {
        c.skip(1)?;
    }
    let pad8 = |n: usize| if version == 1 { n.div_ceil(8) * 8 } else { n };
    let raw = c.bytes(pad8(name_len))?;
    let name = String::from_utf8_lossy(
        raw[..name_len.min(raw.len())]
            .split(|&b| b == 0)
            .next()
            .unwrap_or(&[]),
    )
    .into_owned();
    let (o, l) = c.sizes();
    let dt_bytes = c.bytes(pad8(dt_len))?.to_vec();
    let dt = datatype(&mut Cur::new(dt_bytes, o, l))?;
    let ds_bytes = c.bytes(pad8(ds_len))?.to_vec();
    let space = dataspace(&mut Cur::new(ds_bytes, o, l))?;
    let size = match &dt {
        DType::VText | DType::Seq(_) => 4 + o + 4,
        d => d.size(),
    };
    let n = (space.count() as usize).saturating_mul(size).min(c.left());
    Ok((name, dt, space, c.bytes(n)?.to_vec()))
}

/// A link: its name and, for a hard link, the object's address.
pub fn link(c: &mut Cur) -> Result<(String, Option<u64>), String> {
    let _version = c.u8()?;
    let flags = c.u8()?;
    let kind = if flags & 0x08 != 0 { c.u8()? } else { 0 };
    if flags & 0x04 != 0 {
        c.skip(8)?;
    }
    if flags & 0x10 != 0 {
        c.skip(1)?;
    }
    let n = c.uvar(1 << (flags & 3))? as usize;
    let name = String::from_utf8_lossy(c.bytes(n)?).into_owned();
    let addr = if kind == 0 { c.addr()? } else { None };
    Ok((name, addr))
}

/// A link or attribute info message: the fractal heap's address and the name index's.
pub fn info(c: &mut Cur, attribute: bool) -> Result<(Option<u64>, Option<u64>), String> {
    let _version = c.u8()?;
    let flags = c.u8()?;
    if flags & 0x01 != 0 {
        c.skip(if attribute { 2 } else { 8 })?;
    }
    let heap = c.addr()?;
    let names = c.addr()?;
    Ok((heap, names))
}
