//! LAZ's chunks (docs/adr/0207 §2): the `laszip encoded` VLR, the chunk
//! table and one chunk's records in and out through the `laz` crate's record
//! compressors. Each chunk stands alone, so the hosts decompress them on
//! their own threads, in any order.
//!
//! The `laz` crate is trusted with well-formed data only: its items are
//! checked against the record's layout before a decompressor is made, and a
//! decompressor that panics on a hostile chunk is caught (natively) and
//! reported as a fault of the file.

use std::io::Cursor;
use std::panic::{AssertUnwindSafe, catch_unwind};

use laz::laszip::ChunkTableEntry;
use laz::record::{
    LayeredPointRecordCompressor, LayeredPointRecordDecompressor, RecordCompressor,
    RecordDecompressor, SequentialPointRecordCompressor, SequentialPointRecordDecompressor,
};
/// Which layers of LAS 1.4's compression are decompressed; a record's items (a host's compression names them).
pub use laz::{DecompressionSelection, LazItem};
use laz::{LazItemRecordBuilder, LazVlr, LazVlrBuilder};

use crate::las::{Header, Vlr};
use crate::{PcError, Result};

/// LAZ's VLR.
pub const LAZ_USER: &str = "laszip encoded";
pub const LAZ_RECORD: u16 = 22204;
/// The most chunks a chunk table may list.
pub const MAX_CHUNKS: u32 = 10_000_000;

/// A chunk of a LAZ file: where its bytes are, how many points, the first one's index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Chunk {
    pub offset: u64,
    pub bytes: u64,
    pub count: u64,
    pub first: u64,
}

/// LAZ's VLR among the VLRs, checked against the header's records.
pub fn laz_vlr(head: &Header, vlrs: &[Vlr]) -> Result<LazVlr> {
    let v = vlrs
        .iter()
        .find(|v| v.is(LAZ_USER, LAZ_RECORD))
        .ok_or_else(|| PcError::new("LAZ dosyasında LASzip'in VLR'si yok."))?;
    let vlr = catch(|| LazVlr::from_buffer(&v.data))
        .and_then(|r| r.map_err(PcError::from))
        .map_err(|_| PcError::new("LASzip'in VLR'si okunamadı."))?;
    if vlr.items().is_empty() {
        return Err(PcError::new("LASzip'in VLR'si nokta alanı saymıyor."));
    }
    if vlr.items_size() != u64::from(head.record_len) {
        return Err(PcError::new(format!(
            "LASzip'in alanları {} bayt, nokta kaydı {} bayt: dosya tutarsız.",
            vlr.items_size(),
            head.record_len
        )));
    }
    let version = vlr.items()[0].version();
    if !matches!(version, 1..=4) {
        return Err(PcError::new(format!(
            "LASzip sıkıştırmasının {version}. sürümü okunmuyor."
        )));
    }
    Ok(vlr)
}

/// The chunk table read from its bytes (version, count, the arithmetic-coded entries).
pub fn table(vlr: &LazVlr, bytes: &[u8]) -> Result<Vec<ChunkTableEntry>> {
    if bytes.len() >= 8 {
        let n = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        if n > MAX_CHUNKS {
            return Err(PcError::new(format!(
                "Parça tablosu {n} parça diyor; en çok {MAX_CHUNKS} okunur."
            )));
        }
    }
    let variable = vlr.uses_variable_size_chunks();
    let table = catch(|| laz::laszip::ChunkTable::read(&mut Cursor::new(bytes), variable))?
        .map_err(|e| PcError::new(format!("Parça tablosu okunamadı: {e}.")))?;
    Ok(table.as_ref().to_vec())
}

/// Where each chunk is: they follow the 8-byte offset to the table, in order.
pub fn locate(head: &Header, vlr: &LazVlr, entries: &[ChunkTableEntry]) -> Result<Vec<Chunk>> {
    let mut at = u64::from(head.data_offset) + 8;
    let mut first = 0u64;
    let fixed = (!vlr.uses_variable_size_chunks()).then(|| u64::from(vlr.chunk_size()));
    let mut out = Vec::with_capacity(entries.len());
    for e in entries {
        let count = match fixed {
            Some(size) => size.min(head.count.saturating_sub(first)),
            None => e.point_count,
        };
        if count == 0 && first >= head.count {
            break;
        }
        out.push(Chunk {
            offset: at,
            bytes: e.byte_count,
            count,
            first,
        });
        at = at
            .checked_add(e.byte_count)
            .ok_or_else(|| PcError::new("Parça tablosu taşıyor."))?;
        first += count;
    }
    if first != head.count {
        return Err(PcError::new(format!(
            "Parça tablosu {first} nokta sayıyor, başlık {} diyor.",
            head.count
        )));
    }
    Ok(out)
}

/// `count` records from one chunk's bytes, appended to `out`.
pub fn decompress(vlr: &LazVlr, data: &[u8], count: usize, out: &mut Vec<u8>) -> Result<()> {
    decompress_selected(vlr, data, count, out, DecompressionSelection::all())
}

/// What a picture reads of a point (docs/adr/0207 §6): its position and
/// returns, and the class, intensity and colours its look `needs`. LAS 1.4's
/// layered compression (formats 6–10, every COPC) leaves the other layers
/// (GPS time, scan angle, user data, source id, flags, extra bytes and what
/// the look does not read) compressed; their bytes in the records are then
/// not the file's and must not be read. Older formats decompress whole.
pub fn selection_for(needs: crate::nodes::Needs) -> DecompressionSelection {
    let mut s = DecompressionSelection::base().decompress_z();
    if needs.class {
        s = s.decompress_classification();
    }
    if needs.intensity {
        s = s.decompress_intensity();
    }
    if needs.rgb {
        s = s.decompress_rgb();
    }
    s
}

/// [`selection_for`] everything a look may read.
pub fn view_selection() -> DecompressionSelection {
    selection_for(crate::nodes::Needs::ALL)
}

/// [`decompress`] of the layers `selection` names (the others' bytes undefined).
pub fn decompress_selected(
    vlr: &LazVlr,
    data: &[u8],
    count: usize,
    out: &mut Vec<u8>,
    selection: DecompressionSelection,
) -> Result<()> {
    let size = vlr.items_size() as usize;
    let start = out.len();
    let total = count
        .checked_mul(size)
        .ok_or_else(|| PcError::new("Parça çok büyük."))?;
    out.resize(start + total, 0);
    let dst = &mut out[start..];
    let items = vlr.items().clone();
    let res = catch(|| -> Result<()> {
        let src = Cursor::new(data);
        if items[0].version() >= 3 {
            let mut d = LayeredPointRecordDecompressor::new(src);
            d.set_fields_from(&items)?;
            d.set_selection(selection);
            d.decompress_many(dst)?;
        } else {
            let mut d = SequentialPointRecordDecompressor::new(src);
            d.set_fields_from(&items)?;
            d.decompress_many(dst)?;
        }
        Ok(())
    });
    match res {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => {
            out.truncate(start);
            Err(e)
        }
        Err(e) => {
            out.truncate(start);
            Err(e)
        }
    }
}

/// One chunk of `records` compressed with `items`.
pub fn compress(items: &[LazItem], records: &[u8]) -> Result<Vec<u8>> {
    let items = items.to_vec();
    if items.is_empty() {
        return Err(PcError::new("Sıkıştırmanın alanı yok."));
    }
    catch(|| -> Result<Vec<u8>> {
        let dst = Cursor::new(Vec::new());
        if items[0].version() >= 3 {
            let mut c = LayeredPointRecordCompressor::new(dst);
            c.set_fields_from(&items)?;
            c.compress_many(records)?;
            c.done()?;
            Ok(c.into_inner().into_inner())
        } else {
            let mut c = SequentialPointRecordCompressor::new(dst);
            c.set_fields_from(&items)?;
            c.compress_many(records)?;
            c.done()?;
            Ok(c.into_inner().into_inner())
        }
    })?
}

/// LAZ's VLR for records of `format` with `extra` extra bytes, chunks of their own sizes.
pub fn vlr_for(format: u8, extra: u16) -> Result<LazVlr> {
    let items = LazItemRecordBuilder::default_for_point_format_id(format, extra)?;
    Ok(LazVlrBuilder::new(items).with_variable_chunk_size().build())
}

/// The LAZ VLR as the file's VLR.
pub fn vlr_record(vlr: &LazVlr) -> Result<Vlr> {
    let mut data = Vec::new();
    vlr.write_to(&mut data)?;
    Ok(Vlr::new(LAZ_USER, LAZ_RECORD, "https://laszip.org", data))
}

/// A chunk table's bytes for chunks of these point and byte counts.
pub fn table_bytes(vlr: &LazVlr, chunks: &[(u64, u64)]) -> Result<Vec<u8>> {
    let mut t = laz::laszip::ChunkTable::with_capacity(chunks.len());
    for &(point_count, byte_count) in chunks {
        t.push(ChunkTableEntry {
            point_count,
            byte_count,
        });
    }
    let mut out = Vec::new();
    t.write_to(&mut out, vlr)?;
    Ok(out)
}

/// Runs `f`, a panic of the `laz` crate on a hostile chunk becoming a fault.
fn catch<T>(f: impl FnOnce() -> T) -> Result<T> {
    catch_unwind(AssertUnwindSafe(f))
        .map_err(|_| PcError::new("LAZ verisi bozuk: parça çözülemedi."))
}
