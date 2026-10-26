//! NetCDF-4 and HDF5 files read by KentOS's own code (docs/adr/0645 §2): the superblock (versions 0 to 3), the groups
//! (old style with a symbol table, new style with links in the header or a fractal heap), each dataset's dataspace,
//! datatype, fill value, layout, filters and attributes, and each chunk's place. The datasets become a chunked
//! [`Model`]: NetCDF-4's dimensions from its dimension scales (`DIMENSION_LIST`, `_Netcdf4Dimid`,
//! `_Netcdf4Coordinates`), a plain HDF5 file's axes as `phony_dim_n` as netCDF-C names them; the attributes NetCDF-4
//! keeps for itself are left out. Nothing here does input or output: the host's [`Fetch`] reads ranges.

pub mod btree;
pub mod chunks;
pub mod heap;
pub mod io;
pub mod object;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::chunked::{Chunked, Chunks, Codec, Data, Decoder, Dim, Fetch, Model, Stored, Variable};
use super::netcdf::{Attr, NcType};
use io::{Cur, H5};
use object::{DType, Layout, Msg, Space};

const SIGNATURE: &[u8; 8] = b"\x89HDF\r\n\x1a\n";
const GROUPS_DEEP: usize = 32;
const OBJECTS_MOST: usize = 100_000;
/// The rows of a contiguous dataset read as one piece aim at this many bytes.
const CONTIGUOUS_PIECE: u64 = 1 << 20;

/// Attributes NetCDF-4 keeps for its own bookkeeping.
const HIDDEN: &[&str] = &[
    "CLASS",
    "NAME",
    "REFERENCE_LIST",
    "DIMENSION_LIST",
    "DIMENSION_LABELS",
    "_Netcdf4Dimid",
    "_Netcdf4Coordinates",
    "_nc3_strict",
    "_NCProperties",
    "_IsNetcdf4",
    "_SuperblockVersion",
];
const PURE_DIM: &str = "This is a netCDF dimension but not a netCDF variable";

/// Where the superblock is: at 0 or a power of two from 512 on (a user block before it).
fn find_superblock(h: &H5) -> Result<u64, String> {
    let mut at = 0u64;
    while at + 8 <= h.size {
        if h.read(at, 8)? == SIGNATURE {
            return Ok(at);
        }
        at = if at == 0 { 512 } else { at * 2 };
    }
    Err("HDF5 imzası bulunamadı; dosya NetCDF-4 değil ya da bozuk.".into())
}

/// A dataset or group found walking the file.
struct Object {
    path: String,
    addr: u64,
    msgs: Vec<Msg>,
}

/// A HDF5 (NetCDF-4) file's model.
pub fn open(fetch: Arc<dyn Fetch>, size: u64) -> Result<Model, String> {
    let mut h = H5::new(fetch, size);
    let sb = find_superblock(&h)?;
    let head = h.read(sb, 64.min(size - sb) as usize)?;
    let version = *head.get(8).ok_or("HDF5 süper bloğu eksik.")?;
    let root = match version {
        0 | 1 => {
            h.o = usize::from(head[13]);
            h.l = usize::from(head[14]);
            let skip = if version == 1 { 28 } else { 24 };
            let n = skip + 4 * h.o + 2 * h.o + 24;
            let mut c = Cur::new(h.read(sb, n)?, h.o, h.l);
            c.skip(skip)?;
            h.base = c.addr()?.unwrap_or(0);
            c.addr()?;
            c.addr()?;
            c.addr()?;
            c.addr()?;
            c.addr()?.ok_or("HDF5'in kök grubu yok.")?
        }
        2 | 3 => {
            h.o = usize::from(head[9]);
            h.l = usize::from(head[10]);
            let mut c = Cur::new(h.read(sb, 12 + 4 * h.o)?, h.o, h.l);
            c.skip(12)?;
            h.base = c.addr()?.unwrap_or(0);
            c.addr()?;
            c.addr()?;
            c.addr()?.ok_or("HDF5'in kök grubu yok.")?
        }
        v => return Err(format!("HDF5 süper bloğunun sürümü {v} okunmuyor.")),
    };
    if !matches!(h.o, 2 | 4 | 8) || !matches!(h.l, 2 | 4 | 8) {
        return Err("HDF5 süper bloğundaki adres ve uzunluk boyları okunmuyor.".into());
    }
    let mut objects = Vec::new();
    let mut seen = HashSet::new();
    walk(&h, root, "", 0, &mut objects, &mut seen)?;
    build(&h, version, objects)
}

/// A group's links: name and object address.
fn links(h: &H5, msgs: &[Msg]) -> Result<Vec<(String, u64)>, String> {
    let mut out = Vec::new();
    for m in msgs {
        match m.kind {
            object::LINK => {
                let (name, addr) = object::link(&mut Cur::new(m.data.clone(), h.o, h.l))?;
                if let Some(a) = addr {
                    out.push((name, a));
                }
            }
            object::LINK_INFO => {
                let (heap, names) = object::info(&mut Cur::new(m.data.clone(), h.o, h.l), false)?;
                if let (Some(heap), Some(names)) = (heap, names) {
                    let f = heap::fractal(h, heap)?;
                    for r in btree::v2_records(h, names)? {
                        // Type 5: the name's hash, then the heap id.
                        let id = r.get(4..).unwrap_or(&[]);
                        let data = f.object(h, id)?;
                        let (name, addr) = object::link(&mut Cur::new(data, h.o, h.l))?;
                        if let Some(a) = addr {
                            out.push((name, a));
                        }
                    }
                }
            }
            object::SYMBOL_TABLE => {
                let mut c = Cur::new(m.data.clone(), h.o, h.l);
                let tree = c.addr()?;
                let local = c.addr()?;
                let (Some(tree), Some(local)) = (tree, local) else {
                    continue;
                };
                let names = heap::local(h, local)?;
                for (_, snod) in btree::v1_leaves(h, tree, None)? {
                    let mut c = h.at(snod, 8)?;
                    c.sig(b"SNOD", "sembol düğümü")?;
                    c.skip(2)?;
                    let n = c.u16()? as usize;
                    let entry = 2 * h.o + 24;
                    let mut c = h.at(snod, 8 + n * entry)?;
                    c.skip(8)?;
                    for _ in 0..n {
                        let name_at = c.uvar(h.o)?;
                        let addr = c.addr()?;
                        c.skip(24)?;
                        if let Some(a) = addr {
                            out.push((heap::local_name(&names, name_at), a));
                        }
                    }
                }
            }
            _ => {}
        }
    }
    Ok(out)
}

fn walk(
    h: &H5,
    addr: u64,
    path: &str,
    depth: usize,
    out: &mut Vec<Object>,
    seen: &mut HashSet<u64>,
) -> Result<(), String> {
    if depth > GROUPS_DEEP || !seen.insert(addr) || out.len() > OBJECTS_MOST {
        return Ok(());
    }
    let msgs = object::messages(h, addr)?;
    let group = msgs.iter().any(|m| {
        matches!(
            m.kind,
            object::LINK | object::LINK_INFO | object::SYMBOL_TABLE
        )
    });
    let mut kids = if group { links(h, &msgs)? } else { Vec::new() };
    out.push(Object {
        path: path.to_owned(),
        addr,
        msgs,
    });
    kids.sort_by(|a, b| a.0.cmp(&b.0));
    for (name, a) in kids {
        let p = if path.is_empty() {
            name
        } else {
            format!("{path}/{name}")
        };
        walk(h, a, &p, depth + 1, out, seen)?;
    }
    Ok(())
}

/// An attribute's value as NetCDF has it.
enum Value {
    Attr(Attr),
    /// Object references per element (`DIMENSION_LIST`'s).
    Refs(Vec<Vec<u64>>),
    None,
}

fn nc_type(d: &DType) -> Option<NcType> {
    Some(match d {
        DType::Int {
            size: 1,
            signed: true,
            ..
        } => NcType::Byte,
        DType::Int {
            size: 1,
            signed: false,
            ..
        } => NcType::UByte,
        DType::Int {
            size: 2,
            signed: true,
            ..
        } => NcType::Short,
        DType::Int {
            size: 2,
            signed: false,
            ..
        } => NcType::UShort,
        DType::Int {
            size: 4,
            signed: true,
            ..
        } => NcType::Int,
        DType::Int {
            size: 4,
            signed: false,
            ..
        } => NcType::UInt,
        DType::Int {
            size: 8,
            signed: true,
            ..
        } => NcType::Int64,
        DType::Int {
            size: 8,
            signed: false,
            ..
        } => NcType::UInt64,
        DType::Float { size: 4, .. } => NcType::Float,
        DType::Float { size: 8, .. } => NcType::Double,
        DType::Text { size: 1 } => NcType::Char,
        DType::Enum(b) => return nc_type(b),
        _ => return None,
    })
}

fn little(d: &DType) -> bool {
    match d {
        DType::Int { little, .. } | DType::Float { little, .. } => *little,
        DType::Enum(b) => little(b),
        _ => true,
    }
}

/// Values of type `d` from `raw` as floats.
fn numbers(d: &DType, raw: &[u8]) -> Option<(NcType, Vec<f64>)> {
    let kind = nc_type(d)?;
    let size = kind.size() as usize;
    let mut be = raw[..raw.len() - raw.len() % size.max(1)].to_vec();
    if little(d) && size > 1 {
        for c in be.chunks_exact_mut(size) {
            c.reverse();
        }
    }
    let mut out = Vec::new();
    super::netcdf::decode(kind, &be, &mut out);
    Some((kind, out))
}

fn value(
    h: &H5,
    g: &mut heap::Global,
    d: &DType,
    space: &Space,
    raw: &[u8],
) -> Result<Value, String> {
    let count = space.count() as usize;
    let vlen = 4 + h.o + 4;
    let vlen_item = |g: &mut heap::Global, k: usize| -> Result<Option<(u32, Vec<u8>)>, String> {
        let mut c = Cur::new(
            raw.get(k * vlen..(k + 1) * vlen).unwrap_or(&[]).to_vec(),
            h.o,
            h.l,
        );
        let n = c.u32()?;
        let at = c.addr()?;
        let index = c.u32()?;
        match at {
            Some(a) if n > 0 => Ok(Some((n, g.object(h, a, index)?))),
            _ => Ok(None),
        }
    };
    Ok(match d {
        DType::Text { size } => {
            let texts: Vec<String> = raw
                .chunks(*size.max(&1))
                .take(count)
                .map(|s| {
                    let end = s.iter().position(|&b| b == 0).unwrap_or(s.len());
                    String::from_utf8_lossy(&s[..end]).into_owned()
                })
                .collect();
            Value::Attr(Attr::Text(texts.join(if *size == 1 { "" } else { "\n" })))
        }
        DType::VText => {
            let mut texts = Vec::with_capacity(count);
            for k in 0..count {
                if let Some((n, b)) = vlen_item(g, k)? {
                    let b = &b[..(n as usize).min(b.len())];
                    let end = b.iter().position(|&x| x == 0).unwrap_or(b.len());
                    texts.push(String::from_utf8_lossy(&b[..end]).into_owned());
                } else {
                    texts.push(String::new());
                }
            }
            Value::Attr(Attr::Text(texts.join("\n")))
        }
        DType::Seq(base) if matches!(**base, DType::Ref { .. }) => {
            let mut all = Vec::with_capacity(count);
            for k in 0..count {
                let mut refs = Vec::new();
                if let Some((n, b)) = vlen_item(g, k)? {
                    let mut c = Cur::new(b, h.o, h.l);
                    for _ in 0..n {
                        if let Some(a) = c.addr()? {
                            refs.push(a);
                        }
                    }
                }
                all.push(refs);
            }
            Value::Refs(all)
        }
        d => match numbers(d, raw) {
            Some((NcType::Char, v)) => {
                let s: String = v.iter().map(|&b| b as u8 as char).collect();
                Value::Attr(Attr::Text(s.trim_end_matches('\0').to_owned()))
            }
            Some((kind, values)) => Value::Attr(Attr::Numbers { kind, values }),
            None => Value::None,
        },
    })
}

/// An object's attributes: in its header, and in its fractal heap when there are many.
fn attributes(h: &H5, g: &mut heap::Global, msgs: &[Msg]) -> Result<Vec<(String, Value)>, String> {
    let mut raw = Vec::new();
    for m in msgs {
        match m.kind {
            object::ATTRIBUTE => raw.push(m.data.clone()),
            object::ATTRIBUTE_INFO => {
                let (heap, names) = object::info(&mut Cur::new(m.data.clone(), h.o, h.l), true)?;
                if let (Some(heap), Some(names)) = (heap, names) {
                    let f = heap::fractal(h, heap)?;
                    for r in btree::v2_records(h, names)? {
                        // Type 8: the heap id first (eight bytes).
                        let id = r.get(..8).unwrap_or(&[]);
                        raw.push(f.object(h, id)?);
                    }
                }
            }
            _ => {}
        }
    }
    let mut out = Vec::with_capacity(raw.len());
    for data in raw {
        let (name, d, space, bytes) = object::attribute(&mut Cur::new(data, h.o, h.l))?;
        let v = value(h, g, &d, &space, &bytes)?;
        out.push((name, v));
    }
    Ok(out)
}

/// A dataset as read.
struct Dataset {
    path: String,
    addr: u64,
    space: Space,
    dtype: DType,
    layout: Layout,
    filters: Vec<object::Filter>,
    fill: Option<Vec<u8>>,
    attrs: Vec<(String, Value)>,
}

fn text_attr<'a>(attrs: &'a [(String, Value)], name: &str) -> Option<&'a str> {
    attrs
        .iter()
        .find(|(k, _)| k == name)
        .and_then(|(_, v)| match v {
            Value::Attr(a) => a.text(),
            _ => None,
        })
}

fn number_attr(attrs: &[(String, Value)], name: &str) -> Option<Vec<f64>> {
    attrs
        .iter()
        .find(|(k, _)| k == name)
        .and_then(|(_, v)| match v {
            Value::Attr(Attr::Numbers { values, .. }) => Some(values.clone()),
            _ => None,
        })
}

fn codecs(filters: &[object::Filter], element: usize) -> Result<Vec<Codec>, String> {
    filters
        .iter()
        .map(|f| {
            Ok(match f.id {
                1 => Codec::Zlib,
                2 => Codec::Shuffle(f.values.first().map_or(element, |&v| v as usize).max(1)),
                3 => Codec::Fletcher32,
                32001 => Codec::Blosc,
                32004 => Codec::Lz4Hdf5,
                32015 => Codec::Zstd,
                id => {
                    let name = match id {
                        4 => "szip".to_owned(),
                        5 => "nbit".to_owned(),
                        6 => "scaleoffset".to_owned(),
                        307 => "bzip2".to_owned(),
                        32008 => "bitshuffle".to_owned(),
                        _ if !f.name.is_empty() => f.name.clone(),
                        _ => format!("{id}"),
                    };
                    return Err(format!(
                        "{name} süzgeci açılmıyor; `nccopy -d 4 -s girdi.nc çıktı.nc` ile zlib'e çevirin"
                    ));
                }
            })
        })
        .collect()
}

fn build(h: &H5, sb_version: u8, objects: Vec<Object>) -> Result<Model, String> {
    let mut g = heap::Global::default();
    let mut notes = Vec::new();
    let mut gattrs = Vec::new();
    let mut datasets = Vec::new();
    for o in &objects {
        let has = |k: u16| o.msgs.iter().find(|m| m.kind == k);
        let attrs = match attributes(h, &mut g, &o.msgs) {
            Ok(a) => a,
            Err(e) => {
                notes.push(format!("“{}”: öznitelikleri okunamadı ({e}).", o.path));
                Vec::new()
            }
        };
        let (Some(ds), Some(dt), Some(ly)) = (
            has(object::DATASPACE),
            has(object::DATATYPE),
            has(object::LAYOUT),
        ) else {
            if o.path.is_empty() {
                for (k, v) in attrs {
                    if let Value::Attr(a) = v
                        && !HIDDEN.contains(&k.as_str())
                    {
                        gattrs.push((k, a));
                    }
                }
            }
            continue;
        };
        let space = object::dataspace(&mut Cur::new(ds.data.clone(), h.o, h.l))?;
        let dtype = object::datatype(&mut Cur::new(dt.data.clone(), h.o, h.l))?;
        let layout = object::layout(&mut Cur::new(ly.data.clone(), h.o, h.l), space.dims.len())?;
        let filters = match has(object::FILTERS) {
            Some(m) => object::filters(&mut Cur::new(m.data.clone(), h.o, h.l))?,
            None => Vec::new(),
        };
        let fill = match (has(object::FILL), has(object::FILL_OLD)) {
            (Some(m), _) => object::fill(&mut Cur::new(m.data.clone(), h.o, h.l), false)?,
            (None, Some(m)) => object::fill(&mut Cur::new(m.data.clone(), h.o, h.l), true)?,
            _ => None,
        };
        datasets.push(Dataset {
            path: o.path.clone(),
            addr: o.addr,
            space,
            dtype,
            layout,
            filters,
            fill,
            attrs,
        });
    }
    // Dimensions: every dimension scale's, by its object's address; NetCDF-4's ids by `_Netcdf4Dimid`.
    let mut dims: Vec<Dim> = Vec::new();
    let mut by_addr: HashMap<u64, usize> = HashMap::new();
    let mut by_id: HashMap<i64, usize> = HashMap::new();
    let mut names_used = HashSet::new();
    let leaf = |p: &str| p.rsplit('/').next().unwrap_or(p).to_owned();
    for d in &datasets {
        if text_attr(&d.attrs, "CLASS") != Some("DIMENSION_SCALE") || d.space.dims.len() != 1 {
            continue;
        }
        let mut name = leaf(&d.path);
        if !names_used.insert(name.clone()) {
            name = d.path.clone();
            names_used.insert(name.clone());
        }
        let k = dims.len();
        dims.push(Dim {
            name,
            len: d.space.dims[0],
        });
        by_addr.insert(d.addr, k);
        if let Some(id) = number_attr(&d.attrs, "_Netcdf4Dimid").and_then(|v| v.first().copied()) {
            by_id.insert(id as i64, k);
        }
    }
    let netcdf4 = !dims.is_empty();
    let mut phony: HashMap<u64, usize> = HashMap::new();
    let mut vars = Vec::new();
    for d in &datasets {
        let name = d.path.clone();
        let pure = text_attr(&d.attrs, "NAME").is_some_and(|n| n.starts_with(PURE_DIM));
        if pure {
            continue;
        }
        let Some(kind) = nc_type(&d.dtype) else {
            notes.push(format!(
                "“{name}”: {} türündeki değişken okunmuyor.",
                d.dtype.name()
            ));
            continue;
        };
        let rank = d.space.dims.len();
        // The variable's dimensions.
        let mut vdims: Option<Vec<usize>> = None;
        if text_attr(&d.attrs, "CLASS") == Some("DIMENSION_SCALE") && rank == 1 {
            vdims = by_addr.get(&d.addr).map(|&k| vec![k]);
        }
        if vdims.is_none()
            && let Some((_, Value::Refs(refs))) =
                d.attrs.iter().find(|(k, _)| k == "DIMENSION_LIST")
        {
            let ks: Vec<usize> = refs
                .iter()
                .filter_map(|r| r.first().and_then(|a| by_addr.get(a).copied()))
                .collect();
            if ks.len() == rank {
                vdims = Some(ks);
            }
        }
        if vdims.is_none()
            && let Some(ids) = number_attr(&d.attrs, "_Netcdf4Coordinates")
        {
            let ks: Vec<usize> = ids
                .iter()
                .filter_map(|i| by_id.get(&(*i as i64)).copied())
                .collect();
            if ks.len() == rank {
                vdims = Some(ks);
            }
        }
        let vdims = match vdims {
            Some(v) => v,
            None => d
                .space
                .dims
                .iter()
                .map(|&len| {
                    *phony.entry(len).or_insert_with(|| {
                        let k = dims.len();
                        dims.push(Dim {
                            name: format!("phony_dim_{}", k),
                            len,
                        });
                        k
                    })
                })
                .collect(),
        };
        // A dimension scale's current length may differ from the variable's (an unlimited dimension): the variable's
        // shape is what it holds.
        let ok = vdims
            .iter()
            .zip(&d.space.dims)
            .all(|(&k, &len)| dims[k].len == len);
        if !ok {
            notes.push(format!(
                "“{name}”: boyutları ölçek boyutlarıyla uyuşmuyor; okunmuyor."
            ));
            continue;
        }
        let attrs: Vec<(String, Attr)> = d
            .attrs
            .iter()
            .filter(|(k, _)| !HIDDEN.contains(&k.as_str()))
            .filter_map(|(k, v)| match v {
                Value::Attr(a) => Some((k.clone(), a.clone())),
                _ => None,
            })
            .collect();
        let element = kind.size();
        let fill = d
            .fill
            .as_deref()
            .and_then(|f| numbers(&d.dtype, f))
            .and_then(|(_, v)| v.first().copied())
            .unwrap_or(0.0);
        let stored = Stored {
            kind,
            little: little(&d.dtype),
        };
        let data = match &d.layout {
            Layout::Compact(raw) => match numbers(&d.dtype, raw) {
                Some((_, v)) => Data::Values(v),
                None => continue,
            },
            Layout::Contiguous { address, .. } => {
                let shape = &d.space.dims;
                let mut chunk = vec![1u64; rank];
                if rank > 0 {
                    let last = shape[rank - 1].max(1);
                    chunk[rank - 1] = last;
                    if rank > 1 {
                        let rows = (CONTIGUOUS_PIECE / (last * element)).max(1);
                        chunk[rank - 2] = rows.min(shape[rank - 2].max(1));
                    } else {
                        chunk[0] = (CONTIGUOUS_PIECE / element).clamp(1, last);
                    }
                }
                Data::Chunked(Chunked {
                    chunk,
                    decoder: Decoder::Plain {
                        stored,
                        codecs: Vec::new(),
                    },
                    chunks: match address {
                        Some(a) => Chunks::Contiguous { address: h.abs(*a) },
                        None => Chunks::Table(HashMap::new()),
                    },
                    fortran: false,
                    fill,
                })
            }
            Layout::Chunked { dims: cdims, index } => {
                let codecs = match codecs(&d.filters, element as usize) {
                    Ok(c) => c,
                    Err(e) => {
                        notes.push(format!("“{name}”: {e}."));
                        continue;
                    }
                };
                let want = chunks::Want {
                    shape: &d.space.dims,
                    max: &d.space.max,
                    chunk: cdims,
                    element,
                    filtered: !d.filters.is_empty(),
                };
                let table = match chunks::table(h, index, &want) {
                    Ok(t) => t,
                    Err(e) => {
                        notes.push(format!("“{name}”: parçaları okunamadı ({e})."));
                        continue;
                    }
                };
                Data::Chunked(Chunked {
                    chunk: cdims.clone(),
                    decoder: Decoder::Plain { stored, codecs },
                    chunks: Chunks::Table(table),
                    fortran: false,
                    fill,
                })
            }
            Layout::Virtual => {
                notes.push(format!("“{name}”: sanal (virtual) veri kümesi okunmuyor."));
                continue;
            }
        };
        vars.push(Variable {
            name,
            dims: vdims,
            kind,
            attrs,
            data,
        });
    }
    let _ = sb_version;
    Ok(Model {
        format: if netcdf4 { "NetCDF-4 (HDF5)" } else { "HDF5" }.into(),
        dims,
        attrs: gattrs,
        vars,
        notes,
    })
}
