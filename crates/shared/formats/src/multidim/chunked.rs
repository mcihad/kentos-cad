//! Chunked multidimensional files read as classic NetCDF (docs/adr/0645 §1). NetCDF-4 (HDF5), Zarr and GRIB keep
//! their values in pieces (chunks, objects, messages), compressed and in their own byte order; every reader of a
//! NetCDF slice in KentOS (the raster of a CF grid, a UGRID mesh's datasets, Zaman serisi, Mesh hesaplayıcı) reads a
//! classic file by byte ranges. A [`Model`] says what such a file holds: its dimensions, attributes and variables, each
//! variable's chunks and how they decode. A [`VirtualFile`] is that model laid out as a CDF-5 file: its header made
//! here, each variable's values big-endian one after another; a range of it is made on demand from the chunks the
//! range touches, which the host's [`Fetch`] reads. Decoded chunks are kept within a budget, so a slice read row by
//! row decodes each chunk once.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use super::netcdf::{Attr, NcType, pad};
use super::write::{VarOut, file_layout};

/// What a chunked file's bytes are read through: ranges of the file, or (Zarr) objects of the store.
pub trait Fetch: Send + Sync {
    /// `len` bytes from `offset` of the file.
    fn range(&self, offset: u64, len: u64) -> Result<Vec<u8>, String>;
    /// An object of a store by its key; none when the store does not hold it (a chunk never written).
    fn object(&self, key: &str) -> Result<Option<Vec<u8>>, String> {
        Err(format!(
            "“{key}” nesnesi okunamaz: bu kaynak bir dosya, nesne deposu değil."
        ))
    }
}

/// A dimension of the model.
#[derive(Clone, Debug, PartialEq)]
pub struct Dim {
    pub name: String,
    pub len: u64,
}

/// How a stored value's bytes are: its type and byte order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stored {
    pub kind: NcType,
    pub little: bool,
}

/// One step of a chunk's decoding, in the order the writer applied them (decoding runs them backwards).
#[derive(Clone, Debug, PartialEq)]
pub enum Codec {
    /// Deflate in a zlib stream (HDF5's filter 1, Zarr's zlib), raw Deflate taken too.
    Zlib,
    Gzip,
    Zstd,
    /// HDF5's LZ4 filter (32004): the original size and block size big-endian, each block's size and bytes.
    Lz4Hdf5,
    /// numcodecs' LZ4: the original size little-endian, one LZ4 block.
    Lz4Numcodecs,
    /// A Blosc frame (HDF5's 32001, Zarr's blosc).
    Blosc,
    /// Bytes of an element of `n` bytes laid plane by plane (HDF5's filter 2, Zarr v2's shuffle).
    Shuffle(usize),
    /// HDF5's Fletcher-32: four bytes of checksum at the end.
    Fletcher32,
    /// Zarr v3's crc32c: four bytes of checksum at the end.
    Crc32c,
}

impl Codec {
    /// The name a refusal says.
    pub fn name(&self) -> &'static str {
        match self {
            Codec::Zlib => "zlib",
            Codec::Gzip => "gzip",
            Codec::Zstd => "zstd",
            Codec::Lz4Hdf5 | Codec::Lz4Numcodecs => "lz4",
            Codec::Blosc => "blosc",
            Codec::Shuffle(_) => "shuffle",
            Codec::Fletcher32 => "fletcher32",
            Codec::Crc32c => "crc32c",
        }
    }
}

/// Where a chunk's bytes are.
#[derive(Clone, Debug, PartialEq)]
pub enum Place {
    /// A range of the file; `mask`'s bit `i` set when the writer skipped codec `i` for this chunk (HDF5).
    Range { offset: u64, len: u64, mask: u32 },
    /// An object of the store.
    Object(String),
    /// A chunk inside a Zarr v3 shard: the shard's object and the inner chunk's place in its grid of chunks.
    InShard {
        key: String,
        inner: u64,
        inners: u64,
        index_at_end: bool,
        index_codecs: Vec<Codec>,
    },
}

/// How a chunk's bytes become values.
#[derive(Clone, Debug, PartialEq)]
pub enum Decoder {
    /// The codecs, then values of the stored type.
    Plain { stored: Stored, codecs: Vec<Codec> },
    /// A GRIB message: the field's values decoded (docs/adr/0645 §4), as 64-bit floats.
    Grib,
}

/// Where a model's variable finds a chunk: by its chunk coordinates.
#[derive(Clone, Debug, PartialEq)]
pub enum Chunks {
    /// Each chunk listed (HDF5's index, GRIB's messages); a chunk not listed is never written: fill.
    Table(HashMap<Vec<u64>, Place>),
    /// One run of the file (HDF5's contiguous layout), read as chunks of whole rows of the last dimension
    /// (`chunk` says how many): a chunk's place is worked out from its coordinates.
    Contiguous { address: u64 },
    /// Zarr's keys: `prefix` and the coordinates joined by `separator`; a v3 key's `c` and its separator.
    Keys {
        prefix: String,
        separator: char,
        v3: bool,
        shard: Option<Shard>,
    },
}

/// A Zarr v3 shard's inner chunks.
#[derive(Clone, Debug, PartialEq)]
pub struct Shard {
    /// The outer chunk (the shard's) shape; the variable's `chunk` is the inner one.
    pub outer: Vec<u64>,
    pub index_at_end: bool,
    pub index_codecs: Vec<Codec>,
}

/// A variable's chunked values.
#[derive(Clone, Debug, PartialEq)]
pub struct Chunked {
    pub chunk: Vec<u64>,
    pub decoder: Decoder,
    pub chunks: Chunks,
    /// A chunk's values laid with the first dimension fastest (Zarr's order "F").
    pub fortran: bool,
    /// The value an unwritten chunk holds.
    pub fill: f64,
}

/// A variable's values.
#[derive(Clone, Debug, PartialEq)]
pub enum Data {
    /// All of them, read when the file was opened (small coordinates, GRIB's axes).
    Values(Vec<f64>),
    Chunked(Chunked),
}

/// A variable of the model.
#[derive(Clone, Debug, PartialEq)]
pub struct Variable {
    pub name: String,
    pub dims: Vec<usize>,
    /// The type it is given in the virtual file (big-endian there).
    pub kind: NcType,
    pub attrs: Vec<(String, Attr)>,
    pub data: Data,
}

/// What a chunked file holds.
#[derive(Clone, Debug, PartialEq)]
pub struct Model {
    /// The format's name as the window says it (“NetCDF-4 (HDF5)”, “Zarr v2”, “GRIB2”).
    pub format: String,
    pub dims: Vec<Dim>,
    pub attrs: Vec<(String, Attr)>,
    pub vars: Vec<Variable>,
    /// What was left out and why (variables of types not read, codecs not known).
    pub notes: Vec<String>,
}

/// The most a decoded chunk may take.
pub const CHUNK_MOST: u64 = 1 << 30;
/// Decoded chunks kept at most, in bytes.
pub const CACHE_BUDGET: usize = 256 << 20;
/// The global attribute the virtual file names its format by.
pub const FORMAT_ATTR: &str = "_KentOSFormat";

#[derive(Debug)]
struct Laid {
    begin: u64,
    /// The values' bytes (unpadded).
    bytes: u64,
    shape: Vec<u64>,
}

/// Decoded chunks by (variable, chunk), the oldest first out.
#[derive(Debug, Default)]
struct Cache {
    chunks: HashMap<(usize, Vec<u64>), Arc<Vec<u8>>>,
    order: Vec<(usize, Vec<u64>)>,
    held: usize,
    /// Zarr v3 shards whole, by key.
    shards: HashMap<String, Arc<Vec<u8>>>,
    shard_order: Vec<String>,
}

impl Cache {
    fn keep(&mut self, key: (usize, Vec<u64>), bytes: Arc<Vec<u8>>) {
        self.held += bytes.len();
        self.order.push(key.clone());
        self.chunks.insert(key, bytes);
        while self.held > CACHE_BUDGET && self.order.len() > 1 {
            let old = self.order.remove(0);
            if let Some(b) = self.chunks.remove(&old) {
                self.held -= b.len();
            }
        }
    }

    fn keep_shard(&mut self, key: String, bytes: Arc<Vec<u8>>) {
        self.held += bytes.len();
        self.shard_order.push(key.clone());
        self.shards.insert(key, bytes);
        while self.held > CACHE_BUDGET && self.shard_order.len() > 1 {
            let old = self.shard_order.remove(0);
            if let Some(b) = self.shards.remove(&old) {
                self.held -= b.len();
            }
        }
    }
}

/// A chunked file seen as a classic NetCDF file.
pub struct VirtualFile {
    model: Arc<Model>,
    header: Vec<u8>,
    laid: Vec<Laid>,
    size: u64,
    fetch: Arc<dyn Fetch>,
    cache: Mutex<Cache>,
}

impl std::fmt::Debug for VirtualFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VirtualFile")
            .field("format", &self.model.format)
            .field("size", &self.size)
            .finish()
    }
}

impl VirtualFile {
    /// The model laid out as a CDF-5 file over `fetch`.
    pub fn new(model: Model, fetch: Arc<dyn Fetch>) -> Result<VirtualFile, String> {
        let dims: Vec<(String, u64)> = model.dims.iter().map(|d| (d.name.clone(), d.len)).collect();
        let mut gattrs = model.attrs.clone();
        gattrs.push((FORMAT_ATTR.into(), Attr::Text(model.format.clone())));
        let outs: Vec<VarOut> = model
            .vars
            .iter()
            .map(|v| VarOut {
                name: v.name.clone(),
                dims: v.dims.clone(),
                kind: v.kind,
                attrs: v.attrs.clone(),
            })
            .collect();
        let layout = file_layout_five(&dims, &gattrs, &outs)?;
        let laid = model
            .vars
            .iter()
            .enumerate()
            .map(|(k, v)| Laid {
                begin: layout.begins[k],
                bytes: layout.sizes[k],
                shape: v
                    .dims
                    .iter()
                    .map(|&d| model.dims.get(d).map_or(0, |x| x.len))
                    .collect(),
            })
            .collect::<Vec<_>>();
        let size = laid
            .last()
            .map_or(layout.header.len() as u64, |l| l.begin + pad(l.bytes));
        Ok(VirtualFile {
            model: Arc::new(model),
            header: layout.header,
            laid,
            size,
            fetch,
            cache: Mutex::new(Cache::default()),
        })
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn model(&self) -> &Model {
        &self.model
    }

    /// `len` bytes of the virtual file from `offset`: the header's, a variable's values made from its chunks, the
    /// padding between them zeros.
    pub fn read(&self, offset: u64, len: u64) -> Result<Vec<u8>, String> {
        let end = offset
            .checked_add(len)
            .filter(|&e| e <= self.size)
            .ok_or("Dosya beklenenden kısa.")?;
        let n = usize::try_from(len).map_err(|_| "Okunacak parça çok büyük.".to_owned())?;
        let mut out = vec![0u8; n];
        let head = self.header.len() as u64;
        if offset < head {
            let e = end.min(head);
            out[..(e - offset) as usize].copy_from_slice(&self.header[offset as usize..e as usize]);
        }
        for (k, l) in self.laid.iter().enumerate() {
            let (a, b) = (l.begin.max(offset), (l.begin + l.bytes).min(end));
            if a >= b {
                continue;
            }
            let v = &self.model.vars[k];
            let size = v.kind.size();
            // Whole values only: a range starting or ending inside a value is read value-wide and cut.
            let e0 = (a - l.begin) / size;
            let e1 = (b - l.begin).div_ceil(size);
            let bytes = self.values(k, e0, e1)?;
            let from = (a - l.begin - e0 * size) as usize;
            let to = (a - offset) as usize;
            let n = (b - a) as usize;
            out[to..to + n].copy_from_slice(&bytes[from..from + n]);
        }
        Ok(out)
    }

    /// Values `e0`…`e1` (row-major) of variable `k`, big-endian of its type.
    fn values(&self, k: usize, e0: u64, e1: u64) -> Result<Vec<u8>, String> {
        let v = &self.model.vars[k];
        let size = v.kind.size() as usize;
        let shape = &self.laid[k].shape;
        match &v.data {
            Data::Values(values) => {
                let a = (e0 as usize).min(values.len());
                let b = (e1 as usize).min(values.len());
                let mut out = Vec::with_capacity((e1 - e0) as usize * size);
                super::write::encode(v.kind, &values[a..b], &mut out);
                out.resize((e1 - e0) as usize * size, 0);
                Ok(out)
            }
            Data::Chunked(c) => {
                let mut out = vec![0u8; (e1 - e0) as usize * size];
                let rank = shape.len();
                if rank == 0 {
                    let chunk = self.chunk(k, c, &[], size)?;
                    let n = out.len().min(chunk.len());
                    out[..n].copy_from_slice(&chunk[..n]);
                    return Ok(out);
                }
                let mut e = e0;
                let mut idx = vec![0u64; rank];
                let mut coords = vec![0u64; rank];
                let mut inner = vec![0u64; rank];
                while e < e1 {
                    // The element's multi-index.
                    let mut r = e;
                    for d in (0..rank).rev() {
                        let s = shape[d].max(1);
                        idx[d] = r % s;
                        r /= s;
                    }
                    for d in 0..rank {
                        let cs = c.chunk[d].max(1);
                        coords[d] = idx[d] / cs;
                        inner[d] = idx[d] % cs;
                    }
                    let last = rank - 1;
                    let run = if c.fortran {
                        1
                    } else {
                        (c.chunk[last] - inner[last])
                            .min(shape[last] - idx[last])
                            .min(e1 - e)
                    };
                    let chunk = self.chunk(k, c, &coords, size)?;
                    let at = if c.fortran {
                        // Fortran order: the first dimension fastest.
                        let mut g = 0u64;
                        let mut stride = 1u64;
                        for (i, n) in inner.iter().zip(&c.chunk).take(rank) {
                            g += i * stride;
                            stride *= n;
                        }
                        g
                    } else {
                        let mut f = 0u64;
                        for (i, n) in inner.iter().zip(&c.chunk).take(rank) {
                            f = f * n + i;
                        }
                        f
                    } as usize
                        * size;
                    let to = (e - e0) as usize * size;
                    let n = run as usize * size;
                    if let Some(src) = chunk.get(at..at + n) {
                        out[to..to + n].copy_from_slice(src);
                    }
                    e += run;
                }
                Ok(out)
            }
        }
    }

    /// A chunk of variable `k` decoded, big-endian of its type (its whole shape, the fill where unwritten).
    fn chunk(
        &self,
        k: usize,
        c: &Chunked,
        coords: &[u64],
        size: usize,
    ) -> Result<Arc<Vec<u8>>, String> {
        let key = (k, coords.to_vec());
        if let Some(b) = self
            .cache
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .chunks
            .get(&key)
        {
            return Ok(b.clone());
        }
        let v = &self.model.vars[k];
        let count: u64 = c.chunk.iter().product::<u64>().max(1);
        let want = count
            .checked_mul(size as u64)
            .filter(|&b| b <= CHUNK_MOST)
            .ok_or_else(|| format!("“{}” değişkeninin bir parçası 1 GB'tan büyük.", v.name))?;
        let place = match &c.chunks {
            Chunks::Table(t) => t.get(coords).cloned(),
            Chunks::Contiguous { address } => {
                contiguous_place(*address, &self.laid[k].shape, &c.chunk, coords, size as u64)
            }
            Chunks::Keys {
                prefix,
                separator,
                v3,
                shard,
            } => Some(zarr_place(
                prefix,
                *separator,
                *v3,
                shard.as_ref(),
                &c.chunk,
                coords,
            )),
        };
        let mask = match &place {
            Some(Place::Range { mask, .. }) => *mask,
            _ => 0,
        };
        let raw = match place {
            None => None,
            Some(p) => self.bytes_at(&p)?,
        };
        let bytes = match raw {
            None => fill_bytes(v.kind, c.fill, count as usize),
            Some(raw) => {
                let values = match &c.decoder {
                    Decoder::Plain { stored, codecs } => {
                        let used: Vec<Codec> = codecs
                            .iter()
                            .enumerate()
                            .filter(|(i, _)| *i >= 32 || mask >> i & 1 == 0)
                            .map(|(_, c)| c.clone())
                            .collect();
                        let plain = decode_codecs(
                            &used,
                            &raw,
                            want as usize * stored.kind.size() as usize / size,
                        )
                        .map_err(|e| {
                            format!("“{}” değişkeninin bir parçası açılamadı: {e}", v.name)
                        })?;
                        to_big(stored, v.kind, &plain, count as usize, c.fill)
                    }
                    Decoder::Grib => {
                        let field = super::grib::decode::field(&raw, mask)
                            .map_err(|e| format!("“{}” alanı açılamadı: {e}", v.name))?;
                        // GRIB's variables are doubles; a missing point stays NaN (not NetCDF's default fill).
                        let mut out = Vec::with_capacity(want as usize);
                        for x in field {
                            out.extend_from_slice(&x.to_be_bytes());
                        }
                        out
                    }
                };
                let mut values = values;
                values.resize(want as usize, 0);
                values
            }
        };
        let bytes = Arc::new(bytes);
        self.cache
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .keep(key, bytes.clone());
        Ok(bytes)
    }

    /// A place's bytes; none for an object the store does not hold.
    fn bytes_at(&self, p: &Place) -> Result<Option<Vec<u8>>, String> {
        match p {
            Place::Range { offset, len, .. } => self.fetch.range(*offset, *len).map(Some),
            Place::Object(key) => self.fetch.object(key),
            Place::InShard {
                key,
                inner,
                inners,
                index_at_end,
                index_codecs,
            } => {
                let held = self
                    .cache
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .shards
                    .get(key)
                    .cloned();
                let shard = match held {
                    Some(s) => s,
                    None => match self.fetch.object(key)? {
                        None => return Ok(None),
                        Some(b) => {
                            let b = Arc::new(b);
                            self.cache
                                .lock()
                                .unwrap_or_else(PoisonError::into_inner)
                                .keep_shard(key.clone(), b.clone());
                            b
                        }
                    },
                };
                shard_chunk(&shard, *inner, *inners, *index_at_end, index_codecs)
            }
        }
    }
}

/// A CDF-5 header for the model (its variables may pass 4 GiB and its types need it).
fn file_layout_five(
    dims: &[(String, u64)],
    gattrs: &[(String, Attr)],
    vars: &[VarOut],
) -> Result<super::write::FileLayout, String> {
    let l = file_layout(dims, gattrs, vars)?;
    if l.version == 5 {
        return Ok(l);
    }
    // CDF-2 cannot hold the unsigned and 64-bit types: lay it out as CDF-5 always (begins are 64-bit there too).
    super::write::file_layout_version(dims, gattrs, vars, 5)
}

/// A chunk of a contiguous run: whole rows of the last dimension (all of a rank-1 variable's `chunk` values), cut at
/// the variable's end.
fn contiguous_place(
    address: u64,
    shape: &[u64],
    chunk: &[u64],
    coords: &[u64],
    size: u64,
) -> Option<Place> {
    let rank = shape.len();
    let mut start = vec![0u64; rank];
    for d in 0..rank {
        start[d] = coords[d] * chunk[d];
        if start[d] >= shape[d].max(1) && shape[d] > 0 {
            return None;
        }
    }
    let mut flat = 0u64;
    for d in 0..rank {
        flat = flat * shape[d] + start[d];
    }
    // The chunk's values lie one after another when every dimension past the first cut one is whole.
    let count = if rank == 0 {
        1
    } else {
        let cut = (0..rank).find(|&d| chunk[d] != 1).unwrap_or(rank - 1);
        let mut n = (shape[cut] - start[cut]).min(chunk[cut]);
        for s in &shape[cut + 1..rank] {
            n *= s;
        }
        n
    };
    Some(Place::Range {
        offset: address + flat * size,
        len: count * size,
        mask: 0,
    })
}

/// A Zarr chunk's place: its key (v2 `0.1.2`, v3 `c/0/1/2`), or inside its shard.
fn zarr_place(
    prefix: &str,
    sep: char,
    v3: bool,
    shard: Option<&Shard>,
    inner_chunk: &[u64],
    coords: &[u64],
) -> Place {
    let join = |c: &[u64]| -> String {
        let parts: Vec<String> = c.iter().map(u64::to_string).collect();
        if v3 {
            if parts.is_empty() {
                "c".to_owned()
            } else {
                format!("c{sep}{}", parts.join(&sep.to_string()))
            }
        } else if parts.is_empty() {
            "0".to_owned()
        } else {
            parts.join(&sep.to_string())
        }
    };
    match shard {
        None => Place::Object(format!("{prefix}{}", join(coords))),
        Some(s) => {
            let per: Vec<u64> = s
                .outer
                .iter()
                .zip(inner_chunk)
                .map(|(o, i)| (o / i.max(&1)).max(1))
                .collect();
            let outer: Vec<u64> = coords.iter().zip(&per).map(|(c, p)| c / p).collect();
            let mut inner = 0u64;
            for (c, p) in coords.iter().zip(&per) {
                inner = inner * p + c % p;
            }
            Place::InShard {
                key: format!("{prefix}{}", join(&outer)),
                inner,
                inners: per.iter().product(),
                index_at_end: s.index_at_end,
                index_codecs: s.index_codecs.clone(),
            }
        }
    }
}

/// An inner chunk's bytes out of a shard: its index (offset and size, little-endian, `u64::MAX` twice for an empty
/// chunk) at the shard's end or start.
fn shard_chunk(
    shard: &[u8],
    inner: u64,
    inners: u64,
    at_end: bool,
    index_codecs: &[Codec],
) -> Result<Option<Vec<u8>>, String> {
    let crc = index_codecs.contains(&Codec::Crc32c);
    let index_len = (inners * 16 + if crc { 4 } else { 0 }) as usize;
    if shard.len() < index_len {
        return Err("Zarr parçasının (shard) dizini eksik.".into());
    }
    let index = if at_end {
        &shard[shard.len() - index_len..]
    } else {
        &shard[..index_len]
    };
    let at = (inner * 16) as usize;
    let rd = |b: &[u8]| u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]);
    let (off, len) = (rd(&index[at..at + 8]), rd(&index[at + 8..at + 16]));
    if off == u64::MAX && len == u64::MAX {
        return Ok(None);
    }
    let (a, b) = (off as usize, off.saturating_add(len) as usize);
    shard
        .get(a..b)
        .map(|s| Some(s.to_vec()))
        .ok_or_else(|| "Zarr parçasının (shard) dizini dosyanın dışını gösteriyor.".into())
}

/// `count` fill values, big-endian of `kind`.
fn fill_bytes(kind: NcType, fill: f64, count: usize) -> Vec<u8> {
    let mut one = Vec::new();
    super::write::encode(kind, &[fill], &mut one);
    if fill.is_nan() && !matches!(kind, NcType::Float | NcType::Double) {
        one.iter_mut().for_each(|b| *b = 0);
    }
    if kind == NcType::Float && fill.is_nan() {
        one = f32::NAN.to_be_bytes().to_vec();
    }
    if kind == NcType::Double && fill.is_nan() {
        one = f64::NAN.to_be_bytes().to_vec();
    }
    one.repeat(count)
}

/// Stored values as big-endian bytes of the variable's type (the same type: the bytes turned round when little).
fn to_big(stored: &Stored, kind: NcType, plain: &[u8], count: usize, fill: f64) -> Vec<u8> {
    let s = stored.kind.size() as usize;
    if stored.kind == kind {
        let mut out = plain.to_vec();
        out.resize(count * s, 0);
        if stored.little && s > 1 {
            for c in out.chunks_exact_mut(s) {
                c.reverse();
            }
        }
        return out;
    }
    // Another type: through the values (fill where short).
    let mut be = plain.to_vec();
    if stored.little && s > 1 {
        for c in be.chunks_exact_mut(s) {
            c.reverse();
        }
    }
    let mut values = Vec::new();
    super::netcdf::decode(stored.kind, &be, &mut values);
    values.resize(count, fill);
    let mut out = Vec::new();
    super::write::encode(kind, &values, &mut out);
    out
}

/// A chunk's bytes through its codecs backwards; `size` the plain size expected.
pub fn decode_codecs(codecs: &[Codec], raw: &[u8], size: usize) -> Result<Vec<u8>, String> {
    let mut bytes = raw.to_vec();
    for c in codecs.iter().rev() {
        bytes = match c {
            Codec::Zlib => crate::raster::codec::inflate(&bytes, size.max(1)).map_err(|e| e.0)?,
            Codec::Gzip => crate::compress::gzip::decompress(&bytes, size)?,
            Codec::Zstd => crate::compress::zstd::decompress(&bytes, size)?,
            Codec::Lz4Hdf5 => lz4_hdf5(&bytes, size)?,
            Codec::Lz4Numcodecs => {
                let n = bytes
                    .get(..4)
                    .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)
                    .ok_or("LZ4 parçası eksik")?;
                crate::compress::lz4::decompress_block(&bytes[4..], n)?
            }
            Codec::Blosc => super::blosc::decompress(&bytes)?,
            Codec::Shuffle(n) => unshuffle(&bytes, *n),
            Codec::Fletcher32 | Codec::Crc32c => {
                let n = bytes.len().saturating_sub(4);
                bytes.truncate(n);
                bytes
            }
        };
    }
    Ok(bytes)
}

/// HDF5's LZ4 filter's frame.
fn lz4_hdf5(b: &[u8], size: usize) -> Result<Vec<u8>, String> {
    let rd8 = |at: usize| -> Option<u64> {
        b.get(at..at + 8)
            .map(|s| u64::from_be_bytes([s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7]]))
    };
    let rd4 = |at: usize| -> Option<u32> {
        b.get(at..at + 4)
            .map(|s| u32::from_be_bytes([s[0], s[1], s[2], s[3]]))
    };
    let total = rd8(0).ok_or("LZ4 başlığı eksik")? as usize;
    let block = rd4(8).ok_or("LZ4 başlığı eksik")? as usize;
    if total > size.max(1).saturating_mul(2) + (1 << 20) || block == 0 {
        return Err("LZ4 başlığı bozuk".into());
    }
    let mut out = Vec::with_capacity(total);
    let mut at = 12;
    while out.len() < total {
        let want = block.min(total - out.len());
        let n = rd4(at).ok_or("LZ4 bloğu eksik")? as usize;
        at += 4;
        let src = b.get(at..at + n).ok_or("LZ4 bloğu eksik")?;
        at += n;
        if n == want {
            out.extend_from_slice(src);
        } else {
            out.extend(crate::compress::lz4::decompress_block(src, want)?);
        }
    }
    Ok(out)
}

/// Bytes laid plane by plane (the shuffle) made element by element again.
pub fn unshuffle(b: &[u8], n: usize) -> Vec<u8> {
    if n <= 1 {
        return b.to_vec();
    }
    let count = b.len() / n;
    let mut out = vec![0u8; b.len()];
    for k in 0..n {
        let plane = &b[k * count..(k + 1) * count];
        for (i, &v) in plane.iter().enumerate() {
            out[i * n + k] = v;
        }
    }
    // The tail that is not a whole element stays as it was.
    out[count * n..].copy_from_slice(&b[count * n..]);
    out
}

/// A file a chunked reader reads (NetCDF-4 and HDF5, GRIB1 and GRIB2) seen as classic NetCDF; none for a classic
/// NetCDF file or another file (`head` its first bytes, at least eight).
pub fn open_file(
    fetch: Arc<dyn Fetch>,
    size: u64,
    head: &[u8],
) -> Result<Option<VirtualFile>, String> {
    let model = match super::netcdf::sniff(head) {
        super::netcdf::Sniff::Hdf5 => super::hdf5::open(fetch.clone(), size)?,
        super::netcdf::Sniff::Grib => super::grib::open(fetch.clone(), size)?,
        _ => return Ok(None),
    };
    VirtualFile::new(model, fetch).map(Some)
}

/// A Zarr store seen as classic NetCDF: its objects through `fetch`, its members (without consolidated metadata) as
/// `list` names them.
pub fn open_zarr(
    fetch: Arc<dyn Fetch>,
    list: &dyn Fn(&str) -> Option<Vec<String>>,
) -> Result<VirtualFile, String> {
    let model = super::zarr::open(fetch.clone(), list)?;
    VirtualFile::new(model, fetch)
}
