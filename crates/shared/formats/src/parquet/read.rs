//! A Parquet file read (docs/adr/0628 §1): the footer (`PAR1`, its length, the metadata), the schema, and each column
//! chunk's pages — the dictionary, data pages v1 and v2 — decompressed and decoded into its levels and its values.
//! The bytes come from a [`Source`]: the file in memory or read piece by piece, so a large file is never read whole.

use std::borrow::Cow;

use super::encoding::{self, Bytes, Values};
use super::meta::{self, ColumnMeta, FileMeta, PageKind, Physical};
use super::rle;
use super::schema::Schema;
use crate::compress::{self, Codec};

/// Where a Parquet file's bytes come from.
pub trait Source {
    fn size(&self) -> u64;
    fn read_at(&self, offset: u64, len: usize) -> Result<Cow<'_, [u8]>, String>;
}

impl Source for [u8] {
    fn size(&self) -> u64 {
        self.len() as u64
    }

    fn read_at(&self, offset: u64, len: usize) -> Result<Cow<'_, [u8]>, String> {
        let start = usize::try_from(offset).map_err(|_| "Parquet dosyasının dışı")?;
        self.get(start..start.checked_add(len).ok_or("Parquet dosyasının dışı")?)
            .map(Cow::Borrowed)
            .ok_or_else(|| "Parquet dosyası yarım: istenen bayt dosyanın dışında".to_owned())
    }
}

impl Source for Vec<u8> {
    fn size(&self) -> u64 {
        self.len() as u64
    }

    fn read_at(&self, offset: u64, len: usize) -> Result<Cow<'_, [u8]>, String> {
        self.as_slice().read_at(offset, len)
    }
}

/// A file read piece by piece.
pub struct FileSource {
    file: std::sync::Mutex<std::fs::File>,
    size: u64,
}

impl FileSource {
    pub fn open(path: &std::path::Path) -> Result<FileSource, String> {
        let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        let size = file.metadata().map_err(|e| e.to_string())?.len();
        Ok(FileSource {
            file: std::sync::Mutex::new(file),
            size,
        })
    }
}

impl Source for FileSource {
    fn size(&self) -> u64 {
        self.size
    }

    fn read_at(&self, offset: u64, len: usize) -> Result<Cow<'_, [u8]>, String> {
        use std::io::{Read, Seek, SeekFrom};
        if offset
            .checked_add(len as u64)
            .is_none_or(|end| end > self.size)
        {
            return Err("Parquet dosyası yarım: istenen bayt dosyanın dışında".into());
        }
        let mut f = self.file.lock().map_err(|_| "Parquet dosyası okunamadı")?;
        f.seek(SeekFrom::Start(offset)).map_err(|e| e.to_string())?;
        let mut out = vec![0u8; len];
        f.read_exact(&mut out).map_err(|e| e.to_string())?;
        Ok(Cow::Owned(out))
    }
}

/// A column chunk read: its levels (empty when they are all zero) and its values, nulls left out.
#[derive(Clone, Debug, PartialEq)]
pub struct Leaf {
    pub def: Vec<u16>,
    pub rep: Vec<u16>,
    /// The number of slots: values and nulls.
    pub slots: usize,
    pub values: Values,
}

/// A Parquet file opened: its metadata and schema.
pub struct File<S: Source> {
    pub source: S,
    pub meta: FileMeta,
    pub schema: Schema,
}

/// The footer's metadata is limited to this many bytes.
const META_MAX: usize = 256 << 20;

impl<S: Source> File<S> {
    pub fn open(source: S) -> Result<File<S>, String> {
        let size = source.size();
        if size < 12 {
            return Err("Parquet dosyası değil: çok kısa".into());
        }
        let head = source.read_at(0, 4)?;
        let tail = source.read_at(size - 8, 8)?;
        if &tail[4..] == b"PARE" || &head[..] == b"PARE" {
            return Err("şifreli Parquet dosyası açılmaz".into());
        }
        if &head[..] != b"PAR1" || &tail[4..] != b"PAR1" {
            return Err("Parquet dosyası değil: PAR1 imzası yok".into());
        }
        let len = u32::from_le_bytes([tail[0], tail[1], tail[2], tail[3]]) as usize;
        if len > META_MAX || len as u64 > size - 12 {
            return Err(format!(
                "Parquet altbilgisinin boyu ({len} bayt) dosyaya sığmıyor"
            ));
        }
        let footer = source.read_at(size - 8 - len as u64, len)?;
        let meta = meta::read_file_meta(&footer)?;
        let schema = Schema::of(&meta.schema)?;
        for (g, group) in meta.row_groups.iter().enumerate() {
            if group.columns.len() != schema.columns.len() {
                return Err(format!(
                    "Parquet satır grubu {g}: {} sütun parçası var, şemada {} sütun",
                    group.columns.len(),
                    schema.columns.len()
                ));
            }
        }
        drop(footer);
        Ok(File {
            source,
            meta,
            schema,
        })
    }

    /// Column `column` of row group `group`.
    pub fn read(&self, group: usize, column: usize) -> Result<Leaf, String> {
        let chunk = &self.meta.row_groups[group].columns[column];
        let node = &self.schema.nodes[self.schema.columns[column]];
        let physical = node.physical.ok_or("Parquet sütunu türsüz")?;
        if chunk.physical != physical {
            return Err(format!(
                "Parquet sütunu “{}”: parçanın türü {}, şemanınki {}",
                node.name,
                chunk.physical.name(),
                physical.name()
            ));
        }
        read_chunk(
            &self.source,
            chunk,
            physical,
            node.type_length,
            node.def,
            node.rep,
        )
    }
}

/// The bytes a codec makes of a page.
fn open_page(codec: Codec, body: &[u8], size: usize) -> Result<Cow<'_, [u8]>, String> {
    if codec == Codec::None {
        if body.len() != size {
            return Err("Parquet sayfasının boyu başlığıyla tutmuyor".into());
        }
        return Ok(Cow::Borrowed(body));
    }
    if body.is_empty() && size == 0 {
        return Ok(Cow::Borrowed(body));
    }
    compress::decompress(codec, body, size).map(Cow::Owned)
}

fn levels(
    bytes: &[u8],
    encoding: i32,
    max: u16,
    n: usize,
    prefixed: bool,
    out: &mut Vec<u16>,
) -> Result<usize, String> {
    if max == 0 {
        return Ok(0);
    }
    let width = rle::width(u32::from(max));
    let mut v = Vec::with_capacity(n);
    let used = match (encoding, prefixed) {
        (3, true) => rle::read_prefixed(bytes, width, n, &mut v)?,
        (3, false) => rle::read(bytes, width, n, &mut v)?,
        (4, _) => rle::read_msb(bytes, width, n, &mut v)?,
        (e, _) => return Err(format!("Parquet düzey kodlaması {e} tanınmıyor")),
    };
    if v.iter().any(|&l| l > u32::from(max)) {
        return Err("Parquet düzeyi en büyük düzeyi aşıyor".into());
    }
    out.extend(v.into_iter().map(|l| l as u16));
    Ok(used)
}

/// The values of a data page in `encoding`: `n` of them.
fn values(
    bytes: &[u8],
    encoding: i32,
    physical: Physical,
    type_length: usize,
    n: usize,
    dict: Option<&Values>,
    out: &mut Values,
) -> Result<(), String> {
    match encoding {
        0 => {
            encoding::plain(physical, type_length, bytes, n, out)?;
        }
        2 | 8 => {
            let dict = dict.ok_or("Parquet sayfası sözlüklü ama sütunun sözlüğü yok")?;
            let idx = encoding::indices(bytes, n)?;
            out.take(dict, &idx)?;
        }
        3 => match out {
            Values::Bool(o) => encoding::rle_bools(bytes, n, o)?,
            _ => return Err("Parquet RLE kodlaması yalnız evet/hayır değerlerinde olur".into()),
        },
        5 => {
            let (v, _) = encoding::delta_binary(bytes, n)?;
            if v.len() != n {
                return Err("Parquet delta değerleri sayfanın değer sayısıyla tutmuyor".into());
            }
            match out {
                Values::I32(o) => o.extend(v.into_iter().map(|x| x as i32)),
                Values::I64(o) => o.extend(v),
                _ => return Err("Parquet DELTA_BINARY_PACKED yalnız tam sayılarda olur".into()),
            }
        }
        6 | 7 => {
            let o = match out {
                Values::Bytes(o) => o,
                _ => {
                    return Err(
                        "Parquet delta bayt dizisi kodlaması yalnız bayt dizilerinde olur".into(),
                    );
                }
            };
            let before = o.len();
            if encoding == 6 {
                encoding::delta_length(bytes, n, o)?;
            } else {
                encoding::delta_bytes(bytes, n, o)?;
            }
            if o.len() - before != n {
                return Err("Parquet delta değerleri sayfanın değer sayısıyla tutmuyor".into());
            }
        }
        9 => {
            let width = match physical {
                Physical::Float | Physical::Int32 => 4,
                Physical::Double | Physical::Int64 => 8,
                Physical::FixedLenByteArray => type_length,
                _ => return Err("Parquet BYTE_STREAM_SPLIT bu türde olmaz".into()),
            };
            let joined = encoding::byte_stream_split(bytes, width, n)?;
            encoding::plain(physical, type_length, &joined, n, out)?;
        }
        e => return Err(format!("Parquet kodlaması {e} tanınmıyor")),
    }
    Ok(())
}

/// A column chunk's pages decoded.
pub fn read_chunk<S: Source + ?Sized>(
    source: &S,
    chunk: &ColumnMeta,
    physical: Physical,
    type_length: usize,
    max_def: u16,
    max_rep: u16,
) -> Result<Leaf, String> {
    let codec = Codec::from_parquet(chunk.codec)
        .ok_or_else(|| format!("Parquet sıkıştırması {} tanınmıyor", chunk.codec))?;
    let data = u64::try_from(chunk.data_page_offset).map_err(|_| "Parquet sütununun yeri eksi")?;
    let start = chunk
        .dictionary_page_offset
        .and_then(|o| u64::try_from(o).ok())
        .filter(|&o| o >= 4 && o < data)
        .unwrap_or(data);
    // The chunk's pages from the first one, the dictionary's when it has one.
    let total = usize::try_from(chunk.compressed).map_err(|_| "Parquet sütununun boyu eksi")?;
    let size =
        total.min(usize::try_from(source.size().saturating_sub(start)).unwrap_or(usize::MAX));
    let bytes = source.read_at(start, size)?;
    let slots_wanted =
        usize::try_from(chunk.num_values).map_err(|_| "Parquet sütununun değer sayısı eksi")?;
    let mut leaf = Leaf {
        def: Vec::new(),
        rep: Vec::new(),
        slots: 0,
        values: Values::empty(physical),
    };
    let mut dict: Option<Values> = None;
    let mut at = 0usize;
    while leaf.slots < slots_wanted && at < bytes.len() {
        let (h, used) = meta::read_page_header(&bytes[at..])?;
        at += used;
        let body = bytes
            .get(at..at + h.compressed as usize)
            .ok_or("Parquet sayfası sütun parçasının dışına taşıyor")?;
        at += h.compressed as usize;
        let n = h.num_values as usize;
        if n > slots_wanted - leaf.slots && h.kind != PageKind::Dictionary {
            return Err("Parquet sayfaları sütunun değer sayısından çok değer söylüyor".into());
        }
        match h.kind {
            PageKind::Index => {}
            PageKind::Dictionary => {
                let page = open_page(codec, body, h.uncompressed as usize)?;
                if !matches!(h.encoding, 0 | 2) {
                    return Err(format!(
                        "Parquet sözlük sayfasının kodlaması {} tanınmıyor",
                        h.encoding
                    ));
                }
                let mut d = Values::empty(physical);
                encoding::plain(physical, type_length, &page, n, &mut d)?;
                dict = Some(d);
            }
            PageKind::Data => {
                let page = open_page(codec, body, h.uncompressed as usize)?;
                let mut p = 0usize;
                p += levels(&page[p..], h.rep_encoding, max_rep, n, true, &mut leaf.rep)?;
                let first = leaf.def.len();
                p += levels(&page[p..], h.def_encoding, max_def, n, true, &mut leaf.def)?;
                let present = if max_def == 0 {
                    n
                } else {
                    leaf.def[first..].iter().filter(|&&d| d == max_def).count()
                };
                values(
                    &page[p..],
                    h.encoding,
                    physical,
                    type_length,
                    present,
                    dict.as_ref(),
                    &mut leaf.values,
                )?;
                leaf.slots += n;
            }
            PageKind::DataV2 => {
                let (rl, dl) = (h.rep_length as usize, h.def_length as usize);
                let levels_end = rl
                    .checked_add(dl)
                    .filter(|&e| e <= body.len())
                    .ok_or("Parquet v2 sayfasının düzeyleri yarım")?;
                levels(&body[..rl], 3, max_rep, n, false, &mut leaf.rep)?;
                let first = leaf.def.len();
                levels(&body[rl..levels_end], 3, max_def, n, false, &mut leaf.def)?;
                let rest = &body[levels_end..];
                let size = (h.uncompressed as usize)
                    .checked_sub(levels_end)
                    .ok_or("Parquet v2 sayfasının boyu düzeylerinden küçük")?;
                let page = if h.values_compressed {
                    open_page(codec, rest, size)?
                } else {
                    Cow::Borrowed(rest)
                };
                let present = if max_def == 0 {
                    n
                } else {
                    leaf.def[first..].iter().filter(|&&d| d == max_def).count()
                };
                values(
                    &page,
                    h.encoding,
                    physical,
                    type_length,
                    present,
                    dict.as_ref(),
                    &mut leaf.values,
                )?;
                leaf.slots += n;
            }
        }
    }
    if leaf.slots != slots_wanted {
        return Err(format!(
            "Parquet sütununda {} değer okundu, üst verisi {slots_wanted} diyor",
            leaf.slots
        ));
    }
    if (max_def > 0 && leaf.def.len() != leaf.slots)
        || (max_rep > 0 && leaf.rep.len() != leaf.slots)
    {
        return Err("Parquet sütununun düzeyleri değerleriyle tutmuyor".into());
    }
    Ok(leaf)
}

/// A byte array value as text (Parquet's strings are UTF-8).
pub fn text(b: &Bytes, i: usize) -> String {
    String::from_utf8_lossy(b.get(i)).into_owned()
}
