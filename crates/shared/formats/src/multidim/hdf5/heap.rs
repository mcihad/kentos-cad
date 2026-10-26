//! HDF5's heaps (docs/adr/0645 §2): the local heap of an old-style group's names, the global heap of variable-length
//! values (texts, the references of `DIMENSION_LIST`) and the fractal heap of a new-style group's links and an
//! object's many attributes.

use std::collections::HashMap;

use super::io::{Cur, H5, log2};

/// A local heap's data segment.
pub fn local(h: &H5, addr: u64) -> Result<Vec<u8>, String> {
    let mut c = h.at(addr, 8 + 2 * h.l + h.o)?;
    c.sig(b"HEAP", "yerel yığın")?;
    c.skip(4)?;
    let size = c.length()?;
    c.length()?;
    let data = c.addr()?.ok_or("HDF5'in yerel yığını boş.")?;
    if size > 64 << 20 {
        return Err("HDF5'in yerel yığını 64 MB'tan büyük.".into());
    }
    Ok(h.at(data, size as usize)?.b)
}

/// The text at `offset` of a local heap's data, up to its zero.
pub fn local_name(data: &[u8], offset: u64) -> String {
    let s = data.get(offset as usize..).unwrap_or(&[]);
    let end = s.iter().position(|&b| b == 0).unwrap_or(s.len());
    String::from_utf8_lossy(&s[..end]).into_owned()
}

/// Global heap collections read, by address: their objects by index.
#[derive(Default)]
pub struct Global {
    collections: HashMap<u64, HashMap<u32, Vec<u8>>>,
}

impl Global {
    /// Object `index` of the collection at `addr`.
    pub fn object(&mut self, h: &H5, addr: u64, index: u32) -> Result<Vec<u8>, String> {
        if let std::collections::hash_map::Entry::Vacant(slot) = self.collections.entry(addr) {
            let mut c = h.at(addr, 8 + h.l)?;
            c.sig(b"GCOL", "genel yığın")?;
            c.skip(4)?;
            let size = c.length()?;
            if size > 256 << 20 {
                return Err("HDF5'in genel yığını 256 MB'tan büyük.".into());
            }
            let mut c = h.at(addr, size as usize)?;
            c.skip(8 + h.l)?;
            let mut objects = HashMap::new();
            while c.left() >= 8 + h.l {
                let i = c.u16()?;
                if i == 0 {
                    break;
                }
                c.skip(6)?;
                let n = c.length()? as usize;
                let data = c.bytes(n.min(c.left()))?.to_vec();
                let pad = n.div_ceil(8) * 8 - n;
                c.skip(pad.min(c.left()))?;
                objects.insert(u32::from(i), data);
            }
            slot.insert(objects);
        }
        self.collections
            .get(&addr)
            .and_then(|o| o.get(&index))
            .cloned()
            .ok_or_else(|| "HDF5'in genel yığınında bir nesne eksik.".into())
    }
}

/// A fractal heap's header, as far as finding managed and tiny objects needs it.
#[derive(Clone, Debug)]
pub struct Fractal {
    id_len: usize,
    filtered: bool,
    checksummed: bool,
    width: u64,
    start: u64,
    max_direct: u64,
    off_size: usize,
    len_size: usize,
    root: Option<u64>,
    root_rows: u64,
    max_direct_rows: u64,
    first_row_bits: u32,
}

pub fn fractal(h: &H5, addr: u64) -> Result<Fractal, String> {
    let mut c = h.at(addr, 256)?;
    c.sig(b"FRHP", "fraktal yığın")?;
    c.skip(1)?;
    let id_len = c.u16()? as usize;
    let filter_len = c.u16()? as usize;
    let flags = c.u8()?;
    let max_managed = u64::from(c.u32()?);
    c.length()?;
    c.addr()?;
    c.length()?;
    c.addr()?;
    for _ in 0..8 {
        c.length()?;
    }
    let width = u64::from(c.u16()?);
    let start = c.length()?;
    let max_direct = c.length()?;
    let max_heap_bits = u64::from(c.u16()?);
    c.u16()?;
    let root = c.addr()?;
    let root_rows = u64::from(c.u16()?);
    if width == 0
        || start == 0
        || !start.is_power_of_two()
        || !max_direct.is_power_of_two()
        || !width.is_power_of_two()
    {
        return Err("HDF5'in fraktal yığınının başlığı bozuk.".into());
    }
    let max_direct_rows = u64::from(log2(max_direct) - log2(start)) + 2;
    let off_size = max_heap_bits.div_ceil(8) as usize;
    let len_size =
        super::io::limit_enc_size(max_direct).min(super::io::limit_enc_size(max_managed));
    Ok(Fractal {
        id_len,
        filtered: filter_len > 0,
        checksummed: flags & 0x02 != 0,
        width,
        start,
        max_direct,
        off_size,
        len_size,
        root,
        root_rows,
        max_direct_rows,
        first_row_bits: log2(start) + log2(width),
    })
}

impl Fractal {
    fn row_size(&self, r: u64) -> u64 {
        if r == 0 {
            self.start
        } else {
            self.start << (r - 1)
        }
    }

    /// An object by its heap id: a managed object read from its direct block, a tiny one from the id itself.
    pub fn object(&self, h: &H5, id: &[u8]) -> Result<Vec<u8>, String> {
        let first = *id.first().ok_or("HDF5 yığın kimliği boş.")?;
        match (first >> 4) & 0x03 {
            0 => {
                let mut c = Cur::new(id.to_vec(), h.o, h.l);
                c.skip(1)?;
                let off = c.uvar(self.off_size)?;
                let len = c.uvar(self.len_size)?;
                if self.filtered {
                    return Err("HDF5'in süzgeçli fraktal yığını okunmuyor.".into());
                }
                let (block, block_off) = self.locate(h, off)?;
                let at = block + (off - block_off);
                Ok(h.at(at, len as usize)?.b)
            }
            2 => {
                let extended = self.id_len > 18;
                let (n, from) = if extended {
                    (
                        ((usize::from(first & 0x0F)) << 8 | usize::from(*id.get(1).unwrap_or(&0)))
                            + 1,
                        2,
                    )
                } else {
                    (usize::from(first & 0x0F) + 1, 1)
                };
                id.get(from..from + n)
                    .map(<[u8]>::to_vec)
                    .ok_or_else(|| "HDF5'in küçük yığın nesnesi bozuk.".into())
            }
            _ => Err("HDF5'in büyük (huge) yığın nesneleri okunmuyor.".into()),
        }
    }

    /// The direct block holding heap offset `off`: its address and its block offset.
    fn locate(&self, h: &H5, off: u64) -> Result<(u64, u64), String> {
        let root = self.root.ok_or("HDF5'in fraktal yığını boş.")?;
        if self.root_rows == 0 {
            return Ok((root, 0));
        }
        let mut addr = root;
        let mut rows = self.root_rows;
        let mut base = 0u64;
        for _ in 0..64 {
            let direct_rows = rows.min(self.max_direct_rows);
            let indirect_rows = rows.saturating_sub(self.max_direct_rows);
            let entry = h.o + if self.filtered { h.l + 4 } else { 0 };
            let head = 5 + h.o + self.off_size;
            let n = head
                + (direct_rows * self.width) as usize * entry
                + (indirect_rows * self.width) as usize * h.o;
            let mut c = h.at(addr, n)?;
            c.sig(b"FHIB", "fraktal yığının dolaylı bloğu")?;
            c.skip(1 + h.o + self.off_size)?;
            let mut at = base;
            let mut next: Option<(u64, u64, u64)> = None;
            'rows: for r in 0..rows {
                let size = self.row_size(r);
                for _ in 0..self.width {
                    if r < self.max_direct_rows {
                        let a = c.addr()?;
                        if self.filtered {
                            c.skip(h.l + 4)?;
                        }
                        if off >= at && off < at + size {
                            return a.map(|a| (a, at)).ok_or_else(|| {
                                "HDF5'in fraktal yığınında boş bir blok gösteriliyor.".into()
                            });
                        }
                    } else {
                        let a = c.addr()?;
                        if off >= at && off < at + size {
                            let child_rows = u64::from(log2(size) - self.first_row_bits) + 1;
                            next = a.map(|a| (a, child_rows, at));
                            break 'rows;
                        }
                    }
                    at += size;
                }
            }
            match next {
                Some((a, r, b)) => {
                    addr = a;
                    rows = r;
                    base = b;
                }
                None => return Err("HDF5'in fraktal yığınında bir nesne bulunamadı.".into()),
            }
        }
        let _ = (self.max_direct, self.checksummed);
        Err("HDF5'in fraktal yığını çok derin.".into())
    }
}
