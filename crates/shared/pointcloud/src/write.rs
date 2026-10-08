//! LAS and LAZ writers (docs/adr/0207 §7): streaming, for the operations'
//! results. The writer hands out the bytes to append, a piece at a time (a
//! LAZ's records compressed in chunks of [`CHUNK`] points, variable-sized,
//! their table at the end), and at the end the header and the chunk table's
//! offset to write at their places once the counts, the bounds and the
//! returns are known. A file of one input keeps that input's version, point
//! format, scale, offset, VLRs and extra bytes; COPC's own records are not
//! carried (a COPC result is indexed by the host from the LAZ).

use laz::{LazItem, LazVlr};

use crate::bytes::put_i64;
use crate::chunks;
use crate::las::{self, Header, Vlr};
use crate::record::Layout;
use crate::{PcError, Result};

/// Points in a LAZ chunk.
pub const CHUNK: usize = 50_000;

/// What a file is written as.
#[derive(Clone, Debug)]
pub struct Spec {
    /// 2, 3 or 4 (LAS 1.2, 1.3, 1.4).
    pub minor: u8,
    pub format: u8,
    /// The record's length: the format's and the extra bytes.
    pub record_len: u16,
    pub scale: [f64; 3],
    pub offset: [f64; 3],
    /// The VLRs to carry (the system's, the extra bytes'), not LAZ's or COPC's.
    pub vlrs: Vec<Vlr>,
    /// The EVLRs to carry (LAS 1.4), not COPC's hierarchy.
    pub evlrs: Vec<Vlr>,
    pub compressed: bool,
    pub source_id: u16,
    pub global_encoding: u16,
    pub software: String,
}

impl Spec {
    /// A file like the source `head`, with its VLRs and EVLRs but LAZ's and COPC's.
    pub fn like(head: &Header, vlrs: &[Vlr], evlrs: &[Vlr], compressed: bool) -> Spec {
        let keep = |v: &&Vlr| {
            !(v.is(chunks::LAZ_USER, chunks::LAZ_RECORD) || v.user == crate::copc::COPC_USER)
        };
        Spec {
            minor: head.minor.clamp(2, 4),
            format: head.format,
            record_len: head.record_len,
            scale: head.scale,
            offset: head.offset,
            vlrs: vlrs.iter().filter(keep).cloned().collect(),
            evlrs: if head.minor >= 4 {
                evlrs.iter().filter(keep).cloned().collect()
            } else {
                Vec::new()
            },
            compressed,
            source_id: head.source_id,
            global_encoding: head.global_encoding
                & (las::ENCODING_GPS_STANDARD | las::ENCODING_WKT),
            software: "KentOS CAD".to_owned(),
        }
    }

    pub fn layout(&self) -> Layout {
        Layout::new(self.format, usize::from(self.record_len))
    }
}

/// Bytes to write at their places once a file is whole: `(offset, bytes)`.
pub type Patches = Vec<(u64, Vec<u8>)>;

/// A host's compression of whole chunks: each chunk's records (of the items'
/// layout) compressed by `chunks::compress`, answered in their order. A host
/// with threads compresses several at once ([`Writer::put_with`]); the core
/// itself never starts one (it also runs in the browser).
pub type Compress<'a> = dyn FnMut(&[LazItem], &[&[u8]]) -> Result<Vec<Vec<u8>>> + 'a;

/// [`Compress`] one chunk after the other.
pub fn compress_each(items: &[LazItem], chunks: &[&[u8]]) -> Result<Vec<Vec<u8>>> {
    chunks.iter().map(|c| chunks::compress(items, c)).collect()
}

/// A file being written.
pub struct Writer {
    spec: Spec,
    layout: Layout,
    laz: Option<LazVlr>,
    pending: Vec<u8>,
    table: Vec<(u64, u64)>,
    /// Where the next bytes go.
    at: u64,
    data_offset: u32,
    count: u64,
    by_return: [u64; 15],
    min: [f64; 3],
    max: [f64; 3],
}

impl Writer {
    /// A writer and the file's first bytes (its header as far as known, the
    /// VLRs and, for a LAZ, the chunk table's offset to come).
    pub fn new(spec: Spec) -> Result<(Writer, Vec<u8>)> {
        let layout = spec.layout();
        if usize::from(spec.record_len) < crate::record::standard_len(spec.format) {
            return Err(PcError::new("Kaydın boyu nokta biçiminden kısa."));
        }
        if spec.format > 5 && spec.minor < 4 {
            return Err(PcError::new("Nokta biçimi 6–10 yalnız LAS 1.4'te yazılır."));
        }
        let extra = usize::from(spec.record_len) - crate::record::standard_len(spec.format);
        let laz = if spec.compressed {
            Some(chunks::vlr_for(spec.format, extra as u16)?)
        } else {
            None
        };
        let mut w = Writer {
            spec,
            layout,
            laz,
            pending: Vec::new(),
            table: Vec::new(),
            at: 0,
            data_offset: 0,
            count: 0,
            by_return: [0; 15],
            min: [f64::INFINITY; 3],
            max: [f64::NEG_INFINITY; 3],
        };
        // The header's size does not hang on its numbers: written once to measure, once to keep.
        let size = w.head(0)?.len();
        w.data_offset = u32::try_from(size).map_err(|_| PcError::new("VLR'ler çok büyük."))?;
        let mut first = w.head(0)?;
        if w.laz.is_some() {
            first.extend_from_slice(&[0u8; 8]);
        }
        w.at = first.len() as u64;
        Ok((w, first))
    }

    fn vlrs(&self) -> Result<Vec<Vlr>> {
        let mut v = Vec::new();
        if let Some(laz) = &self.laz {
            v.push(chunks::vlr_record(laz)?);
        }
        for x in &self.spec.vlrs {
            let mut x = x.clone();
            x.extended = false;
            if x.data.len() > usize::from(u16::MAX) {
                continue;
            }
            v.push(x);
        }
        Ok(v)
    }

    /// The header and VLRs, the EVLRs starting at `evlr_start`.
    fn head(&self, evlr_start: u64) -> Result<Vec<u8>> {
        let s = &self.spec;
        let vlrs = self.vlrs()?;
        let header_size = match s.minor {
            2 => 227,
            3 => 235,
            _ => las::HEADER_1_4,
        } as u16;
        let empty = self.count == 0;
        let head = Header {
            major: 1,
            minor: s.minor,
            source_id: s.source_id,
            global_encoding: s.global_encoding,
            guid: [0; 16],
            system: "KentOS CAD".into(),
            software: s.software.clone(),
            day: 0,
            year: 0,
            header_size,
            data_offset: self.data_offset,
            vlr_count: vlrs.len() as u32,
            format: s.format,
            compressed: s.compressed,
            record_len: s.record_len,
            count: self.count,
            by_return: self.by_return,
            scale: s.scale,
            offset: s.offset,
            min: if empty { [0.0; 3] } else { self.min },
            max: if empty { [0.0; 3] } else { self.max },
            waveform_start: 0,
            evlr_start: if s.evlrs.is_empty() { 0 } else { evlr_start },
            evlr_count: s.evlrs.len() as u32,
        };
        let mut b = head.to_bytes();
        for v in vlrs {
            b.extend_from_slice(&v.to_bytes());
        }
        Ok(b)
    }

    /// Takes `records` (of the spec's layout): the bytes to append now.
    pub fn put(&mut self, records: &[u8]) -> Result<Vec<u8>> {
        self.put_with(records, 1, &mut compress_each)
    }

    /// [`Writer::put`], a LAZ's whole chunks compressed by `compress` once at
    /// least `least` of them wait (a host compressing them at once on its
    /// threads); the bytes are the same as `put`'s, chunk for chunk.
    pub fn put_with(
        &mut self,
        records: &[u8],
        least: usize,
        compress: &mut Compress<'_>,
    ) -> Result<Vec<u8>> {
        let len = self.layout.len;
        if !records.len().is_multiple_of(len) {
            return Err(PcError::new("Kayıtlar kaydın boyunun katı değil."));
        }
        let wide = self.layout.wide();
        for r in records.chunks_exact(len) {
            let p = [
                f64::from(self.layout.x(r)) * self.spec.scale[0] + self.spec.offset[0],
                f64::from(self.layout.y(r)) * self.spec.scale[1] + self.spec.offset[1],
                f64::from(self.layout.z(r)) * self.spec.scale[2] + self.spec.offset[2],
            ];
            for (k, &v) in p.iter().enumerate() {
                self.min[k] = self.min[k].min(v);
                self.max[k] = self.max[k].max(v);
            }
            let (ret, _) = self.layout.returns(r);
            let most = if wide { 15 } else { 5 };
            if (1..=most).contains(&ret) {
                self.by_return[usize::from(ret) - 1] += 1;
            }
        }
        self.count += (records.len() / len) as u64;
        let Some(laz) = &self.laz else {
            self.at += records.len() as u64;
            return Ok(records.to_vec());
        };
        let items = laz.items().clone();
        self.pending.extend_from_slice(records);
        let size = CHUNK * len;
        let ready = self.pending.len() / size;
        if ready == 0 || ready < least {
            return Ok(Vec::new());
        }
        self.compress_chunks(&items, ready * size, size, compress)
    }

    /// The first `bytes` of the waiting records, in pieces of `size`, compressed and taken into the table.
    fn compress_chunks(
        &mut self,
        items: &[LazItem],
        bytes: usize,
        size: usize,
        compress: &mut Compress<'_>,
    ) -> Result<Vec<u8>> {
        let len = self.layout.len;
        let pieces: Vec<&[u8]> = self.pending[..bytes].chunks(size).collect();
        let packed = compress(items, &pieces)?;
        if packed.len() != pieces.len() {
            return Err(PcError::new("Sıkıştırılan parça sayısı tutmuyor."));
        }
        let mut out = Vec::with_capacity(packed.iter().map(Vec::len).sum());
        for (piece, bytes) in pieces.iter().zip(&packed) {
            self.table
                .push(((piece.len() / len) as u64, bytes.len() as u64));
            self.at += bytes.len() as u64;
            out.extend_from_slice(bytes);
        }
        self.pending.drain(..bytes);
        Ok(out)
    }

    /// The end: the bytes to append (the last chunk, the chunk table, the
    /// EVLRs) and the bytes to write at their places (the header; a LAZ's
    /// chunk table offset).
    pub fn finish(self) -> Result<(Vec<u8>, Patches)> {
        self.finish_with(&mut compress_each)
    }

    /// [`Writer::finish`], the chunks still waiting compressed by `compress`.
    pub fn finish_with(mut self, compress: &mut Compress<'_>) -> Result<(Vec<u8>, Patches)> {
        let mut tail = Vec::new();
        let mut patches = Vec::new();
        if let Some(laz) = self.laz.clone() {
            if !self.pending.is_empty() {
                let (bytes, size) = (self.pending.len(), CHUNK * self.layout.len);
                tail = self.compress_chunks(laz.items(), bytes, size, compress)?;
            }
            let table_at = self.at;
            let table = chunks::table_bytes(&laz, &self.table)?;
            self.at += table.len() as u64;
            tail.extend_from_slice(&table);
            let mut offset = [0u8; 8];
            put_i64(&mut offset, 0, table_at as i64);
            patches.push((u64::from(self.data_offset), offset.to_vec()));
        }
        let evlr_start = self.at;
        for e in &self.spec.evlrs {
            let mut e = e.clone();
            e.extended = true;
            tail.extend_from_slice(&e.to_bytes());
        }
        patches.insert(0, (0, self.head(evlr_start)?));
        Ok((tail, patches))
    }

    /// Points written so far.
    pub fn count(&self) -> u64 {
        self.count
    }

    /// The points' bounds so far, `[x₁, y₁, z₁, x₂, y₂, z₂]` (zeros without points).
    pub fn bounds(&self) -> [f64; 6] {
        if self.count == 0 {
            return [0.0; 6];
        }
        [
            self.min[0],
            self.min[1],
            self.min[2],
            self.max[0],
            self.max[1],
            self.max[2],
        ]
    }
}

/// A whole file in memory (tests and small results).
pub fn write_all(spec: Spec, records: &[u8]) -> Result<Vec<u8>> {
    let (mut w, mut file) = Writer::new(spec)?;
    file.extend(w.put(records)?);
    let (tail, patches) = w.finish()?;
    file.extend(tail);
    for (at, b) in patches {
        let at = usize::try_from(at).map_err(|_| PcError::new("Yama dosyanın dışında."))?;
        file.get_mut(at..at + b.len())
            .ok_or_else(|| PcError::new("Yama dosyanın dışında."))?
            .copy_from_slice(&b);
    }
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Step;
    use crate::source::Opening;

    fn records(layout: &Layout, n: usize) -> Vec<u8> {
        let mut out = vec![0u8; n * layout.len];
        for i in 0..n {
            let r = &mut out[i * layout.len..(i + 1) * layout.len];
            layout.set_xyz(
                r,
                i as i32 * 10,
                (i as i32 * 7) % 1000,
                (i as i32 * 3) % 500,
            );
            layout.set_class(r, (i % 7) as u8);
            r[14] = 0x11; // return 1 of 1
        }
        out
    }

    fn read_back(file: &[u8]) -> (crate::source::Cloud, Vec<u8>) {
        let mut o = Opening::new(file.len() as u64);
        let cloud = loop {
            match o.step().unwrap() {
                Step::Done(c) => break c,
                Step::Need(n) => o.put(
                    n.offset,
                    file[n.offset as usize..(n.offset + n.len) as usize].to_vec(),
                ),
            }
        };
        let mut all = Vec::new();
        for run in cloud.runs() {
            let raw = &file[run.need.offset as usize..(run.need.offset + run.need.len) as usize];
            cloud.records(&run, raw, &mut all).unwrap();
        }
        (cloud, all)
    }

    #[test]
    fn a_laz_of_several_chunks_reads_back() {
        let spec = Spec {
            minor: 4,
            format: 6,
            record_len: 30,
            scale: [0.01; 3],
            offset: [1000.0, 2000.0, 0.0],
            vlrs: vec![],
            evlrs: vec![],
            compressed: true,
            source_id: 3,
            global_encoding: 0,
            software: "test".into(),
        };
        let layout = spec.layout();
        let recs = records(&layout, 2 * CHUNK + 123);
        let file = write_all(spec.clone(), &recs).unwrap();
        let (cloud, back) = read_back(&file);
        assert_eq!(cloud.head.count, (2 * CHUNK + 123) as u64);
        assert_eq!(cloud.chunks.len(), 3);
        assert_eq!(back, recs);
        assert_eq!(cloud.head.min[0], 1000.0);
        assert_eq!(cloud.head.by_return[0], (2 * CHUNK + 123) as u64);
        // Uncompressed, the same records.
        let mut las = spec;
        las.compressed = false;
        let file = write_all(las, &recs).unwrap();
        let (cloud, back) = read_back(&file);
        assert_eq!(back, recs);
        assert!(!cloud.head.compressed);
    }

    /// A host compressing whole chunks several at once writes the file `put` writes, byte for byte.
    #[test]
    fn chunks_compressed_together_are_the_same_bytes() {
        let spec = Spec {
            minor: 4,
            format: 6,
            record_len: 30,
            scale: [0.01; 3],
            offset: [1000.0, 2000.0, 0.0],
            vlrs: vec![],
            evlrs: vec![],
            compressed: true,
            source_id: 3,
            global_encoding: 0,
            software: "test".into(),
        };
        let layout = spec.layout();
        let recs = records(&layout, 3 * CHUNK + 777);
        let one = write_all(spec.clone(), &recs).unwrap();
        let (mut w, mut file) = Writer::new(spec).unwrap();
        let mut calls = Vec::new();
        let mut together = |items: &[LazItem], chunks: &[&[u8]]| {
            calls.push(chunks.len());
            compress_each(items, chunks)
        };
        // In uneven pieces, as a file's reading hands them over.
        for piece in recs.chunks(layout.len * 31_337) {
            file.extend(w.put_with(piece, 2, &mut together).unwrap());
        }
        let (tail, patches) = w.finish_with(&mut together).unwrap();
        file.extend(tail);
        for (at, b) in patches {
            file[at as usize..at as usize + b.len()].copy_from_slice(&b);
        }
        assert!(file == one, "the same bytes");
        assert!(calls.iter().any(|&n| n >= 2), "{calls:?}");
        let (cloud, back) = read_back(&file);
        assert_eq!(cloud.chunks.len(), 4);
        assert_eq!(back, recs);
    }

    #[test]
    fn a_las_1_2_keeps_its_format() {
        let spec = Spec {
            minor: 2,
            format: 3,
            record_len: 34,
            scale: [0.001; 3],
            offset: [0.0; 3],
            vlrs: vec![Vlr::new(
                "LASF_Projection",
                2112,
                "WKT",
                b"PROJCS[\"x\"]\0".to_vec(),
            )],
            evlrs: vec![],
            compressed: true,
            source_id: 0,
            global_encoding: 0,
            software: "test".into(),
        };
        let layout = spec.layout();
        let recs = records(&layout, 10);
        let file = write_all(spec, &recs).unwrap();
        let (cloud, back) = read_back(&file);
        assert_eq!((cloud.head.minor, cloud.head.format), (2, 3));
        assert_eq!(back, recs);
        assert!(cloud.vlrs.iter().any(|v| v.is("LASF_Projection", 2112)));
    }
}
