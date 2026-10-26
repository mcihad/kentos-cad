//! Zarr stores read by KentOS's own code (docs/adr/0645 §3): version 2 (`.zgroup`, `.zarray`, `.zattrs`, the
//! consolidated `.zmetadata` xarray writes) and version 3 (`zarr.json`, inline consolidated metadata, sharding).
//! Arrays become a chunked [`Model`]'s variables, their dimensions xarray's `_ARRAY_DIMENSIONS` or v3's
//! `dimension_names` (`phony_dim_n` without), their chunks the store's objects read through the host's [`Fetch`].
//! Codecs: Blosc, zlib, gzip, Zstandard, LZ4, shuffle, crc32c; an array of another codec, a string or complex type is
//! left out with its reason.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use serde_json::Value;

use super::chunked::{
    Chunked, Chunks, Codec, Data, Decoder, Dim, Fetch, Model, Shard, Stored, Variable,
};
use super::netcdf::{Attr, NcType};

/// The most arrays a store may hold.
const ARRAYS_MOST: usize = 10_000;

/// An array as its metadata says it.
struct Array {
    path: String,
    shape: Vec<u64>,
    chunk: Vec<u64>,
    stored: Stored,
    codecs: Vec<Codec>,
    fill: f64,
    /// Whether the store names a fill value (a `null` one is none: an unwritten chunk reads as zeros, as zarr-python
    /// reads it, and no `_FillValue` is said).
    declared: bool,
    fortran: bool,
    separator: char,
    v3: bool,
    shard: Option<Shard>,
    dims: Option<Vec<String>>,
    attrs: serde_json::Map<String, Value>,
}

fn json(bytes: &[u8], what: &str) -> Result<Value, String> {
    serde_json::from_slice(bytes).map_err(|e| format!("Zarr'ın {what} JSON değil: {e}."))
}

/// A store's model: its metadata read through `fetch` (`.zmetadata` or `zarr.json` first; without consolidated
/// metadata the members `list` names, a directory's).
pub fn open(
    fetch: Arc<dyn Fetch>,
    list: &dyn Fn(&str) -> Option<Vec<String>>,
) -> Result<Model, String> {
    let mut arrays = Vec::new();
    let mut gattrs = serde_json::Map::new();
    let mut notes = Vec::new();
    let version;
    if let Some(b) = fetch.object(".zmetadata")? {
        version = 2;
        let v = json(&b, ".zmetadata")?;
        let meta = v
            .get("metadata")
            .and_then(Value::as_object)
            .ok_or("Zarr'ın .zmetadata'sında metadata yok.")?;
        if let Some(a) = meta.get(".zattrs").and_then(Value::as_object) {
            gattrs = a.clone();
        }
        let mut paths: Vec<&String> = meta.keys().filter(|k| k.ends_with(".zarray")).collect();
        paths.sort();
        for k in paths {
            let path = k
                .trim_end_matches(".zarray")
                .trim_end_matches('/')
                .to_owned();
            let attrs = meta
                .get(&format!("{path}/.zattrs"))
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            match array_v2(&path, &meta[k], attrs) {
                Ok(a) => arrays.push(a),
                Err(e) => notes.push(format!("“{path}”: {e}")),
            }
        }
    } else if let Some(b) = fetch.object("zarr.json")? {
        version = 3;
        let root = json(&b, "zarr.json")?;
        if root.get("node_type").and_then(Value::as_str) == Some("array") {
            match array_v3("", &root) {
                Ok(a) => arrays.push(a),
                Err(e) => notes.push(e),
            }
        } else {
            if let Some(a) = root.get("attributes").and_then(Value::as_object) {
                gattrs = a.clone();
            }
            let inline = root
                .get("consolidated_metadata")
                .and_then(|c| c.get("metadata"))
                .and_then(Value::as_object);
            let members: Vec<(String, Value)> = match inline {
                Some(m) => m.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
                None => {
                    let names = list("").ok_or(
                        "Zarr deposunda birleştirilmiş üst veri (consolidated metadata) yok ve üyeleri listelenemiyor; `zarr.consolidate_metadata` ile yazın.",
                    )?;
                    let mut out = Vec::new();
                    for n in names {
                        if let Some(b) = fetch.object(&format!("{n}/zarr.json"))? {
                            out.push((n, json(&b, "zarr.json")?));
                        }
                    }
                    out
                }
            };
            let mut members = members;
            members.sort_by(|a, b| a.0.cmp(&b.0));
            for (path, v) in members {
                if v.get("node_type").and_then(Value::as_str) != Some("array") {
                    continue;
                }
                match array_v3(&path, &v) {
                    Ok(a) => arrays.push(a),
                    Err(e) => notes.push(format!("“{path}”: {e}")),
                }
            }
        }
    } else if let Some(b) = fetch.object(".zgroup")? {
        version = 2;
        json(&b, ".zgroup")?;
        if let Some(b) = fetch.object(".zattrs")?
            && let Some(a) = json(&b, ".zattrs")?.as_object()
        {
            gattrs = a.clone();
        }
        let names = list("").ok_or(
            "Zarr deposunda birleştirilmiş üst veri (.zmetadata) yok ve üyeleri listelenemiyor; `zarr.consolidate_metadata` ile yazın.",
        )?;
        let mut names = names;
        names.sort();
        for n in names {
            let Some(b) = fetch.object(&format!("{n}/.zarray"))? else {
                continue;
            };
            let attrs = match fetch.object(&format!("{n}/.zattrs"))? {
                Some(b) => json(&b, ".zattrs")?
                    .as_object()
                    .cloned()
                    .unwrap_or_default(),
                None => serde_json::Map::new(),
            };
            match array_v2(&n, &json(&b, ".zarray")?, attrs) {
                Ok(a) => arrays.push(a),
                Err(e) => notes.push(format!("“{n}”: {e}")),
            }
        }
    } else if let Some(b) = fetch.object(".zarray")? {
        version = 2;
        let attrs = match fetch.object(".zattrs")? {
            Some(b) => json(&b, ".zattrs")?
                .as_object()
                .cloned()
                .unwrap_or_default(),
            None => serde_json::Map::new(),
        };
        arrays.push(array_v2("", &json(&b, ".zarray")?, attrs)?);
    } else {
        return Err("Zarr deposu değil: .zmetadata, .zgroup, .zarray ya da zarr.json yok.".into());
    }
    if arrays.len() > ARRAYS_MOST {
        return Err(format!("Zarr deposunda {ARRAYS_MOST} diziden çok var."));
    }
    build(version, arrays, gattrs, notes)
}

/// A v2 dtype (`<f4`, `|u1`) as a stored type.
fn dtype_v2(t: &str) -> Result<Stored, String> {
    let little = !t.starts_with('>');
    let code = t.trim_start_matches(['<', '>', '|', '=']);
    let kind = match code {
        "b1" | "u1" => NcType::UByte,
        "i1" => NcType::Byte,
        "i2" => NcType::Short,
        "u2" => NcType::UShort,
        "i4" => NcType::Int,
        "u4" => NcType::UInt,
        "i8" => NcType::Int64,
        "u8" => NcType::UInt64,
        "f4" => NcType::Float,
        "f8" => NcType::Double,
        other => return Err(format!("{other} türündeki dizi okunmuyor.")),
    };
    Ok(Stored { kind, little })
}

fn dtype_v3(t: &str, little: bool) -> Result<Stored, String> {
    let kind = match t {
        "bool" | "uint8" => NcType::UByte,
        "int8" => NcType::Byte,
        "int16" => NcType::Short,
        "uint16" => NcType::UShort,
        "int32" => NcType::Int,
        "uint32" => NcType::UInt,
        "int64" => NcType::Int64,
        "uint64" => NcType::UInt64,
        "float32" => NcType::Float,
        "float64" => NcType::Double,
        other => return Err(format!("{other} türündeki dizi okunmuyor.")),
    };
    Ok(Stored { kind, little })
}

fn fill_of(v: Option<&Value>) -> f64 {
    match v {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0),
        Some(Value::String(s)) => match s.as_str() {
            "NaN" => f64::NAN,
            "Infinity" => f64::INFINITY,
            "-Infinity" => f64::NEG_INFINITY,
            _ => 0.0,
        },
        Some(Value::Bool(b)) => f64::from(u8::from(*b)),
        _ => 0.0,
    }
}

fn u64s(v: Option<&Value>, what: &str) -> Result<Vec<u64>, String> {
    v.and_then(Value::as_array)
        .ok_or_else(|| format!("{what} yok."))?
        .iter()
        .map(|x| x.as_u64().ok_or_else(|| format!("{what} bozuk.")))
        .collect()
}

/// A numcodecs codec (v2's compressor or filter).
fn codec_v2(c: &Value, element: usize) -> Result<Codec, String> {
    let id = c.get("id").and_then(Value::as_str).unwrap_or("");
    Ok(match id {
        "blosc" => Codec::Blosc,
        "zlib" => Codec::Zlib,
        "gzip" => Codec::Gzip,
        "zstd" => Codec::Zstd,
        "lz4" => Codec::Lz4Numcodecs,
        "shuffle" => Codec::Shuffle(
            c.get("elementsize")
                .and_then(Value::as_u64)
                .map_or(element, |n| n as usize),
        ),
        "crc32c" => Codec::Crc32c,
        other => return Err(format!("{other} sıkıştırması ya da süzgeci açılmıyor.")),
    })
}

fn array_v2(path: &str, m: &Value, attrs: serde_json::Map<String, Value>) -> Result<Array, String> {
    let shape = u64s(m.get("shape"), "Dizinin boyu")?;
    let chunk = u64s(m.get("chunks"), "Dizinin parça boyu")?;
    if chunk.len() != shape.len() || chunk.contains(&0) {
        return Err("parça boyu dizinin boyuyla uyuşmuyor.".into());
    }
    let dt = m.get("dtype").and_then(Value::as_str).ok_or("dtype yok.")?;
    let stored = dtype_v2(dt)?;
    let element = stored.kind.size() as usize;
    let mut codecs = Vec::new();
    if let Some(fs) = m.get("filters").and_then(Value::as_array) {
        for f in fs {
            codecs.push(codec_v2(f, element)?);
        }
    }
    if let Some(c) = m.get("compressor").filter(|c| !c.is_null()) {
        codecs.push(codec_v2(c, element)?);
    }
    let dims = attrs
        .get("_ARRAY_DIMENSIONS")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(str::to_owned))
                .collect::<Vec<_>>()
        });
    let mut attrs = attrs;
    attrs.remove("_ARRAY_DIMENSIONS");
    let sep = m
        .get("dimension_separator")
        .and_then(Value::as_str)
        .and_then(|s| s.chars().next())
        .unwrap_or('.');
    Ok(Array {
        path: path.to_owned(),
        shape,
        chunk,
        stored,
        codecs,
        fill: fill_of(m.get("fill_value")),
        declared: !matches!(m.get("fill_value"), None | Some(Value::Null)),
        fortran: m.get("order").and_then(Value::as_str) == Some("F"),
        separator: sep,
        v3: false,
        shard: None,
        dims,
        attrs,
    })
}

/// A shard's inner chunk shape, its codecs, whether its index is at the end, and the index's codecs.
type Sharding = (Vec<u64>, Vec<Codec>, bool, Vec<Codec>);

/// v3 codecs: the array-to-bytes codec's byte order, the bytes-to-bytes codecs, a transpose's Fortran order, sharding.
type CodecsV3 = (bool, Vec<Codec>, bool, Option<Sharding>);

/// v3 codecs: the array-to-bytes codec's byte order, the bytes-to-bytes codecs, a transpose's Fortran order.
fn codecs_v3(list: &[Value]) -> Result<CodecsV3, String> {
    let mut little = true;
    let mut out = Vec::new();
    let mut fortran = false;
    let mut shard = None;
    for c in list {
        let name = c.get("name").and_then(Value::as_str).unwrap_or("");
        let conf = c.get("configuration");
        match name {
            "bytes" => {
                little = conf.and_then(|x| x.get("endian")).and_then(Value::as_str) != Some("big");
            }
            "transpose" => {
                let order = conf.and_then(|x| x.get("order")).and_then(Value::as_array);
                let rev = order.is_some_and(|o| {
                    let n = o.len();
                    o.iter()
                        .enumerate()
                        .all(|(i, v)| v.as_u64() == Some((n - 1 - i) as u64))
                });
                if !rev {
                    return Err("transpose kodeği yalnız ters sırayla (Fortran) okunur.".into());
                }
                fortran = true;
            }
            "gzip" => out.push(Codec::Gzip),
            "zstd" => out.push(Codec::Zstd),
            "blosc" => out.push(Codec::Blosc),
            "crc32c" => out.push(Codec::Crc32c),
            "sharding_indexed" => {
                let conf = conf.ok_or("sharding_indexed'in ayarı yok.")?;
                let inner = u64s(conf.get("chunk_shape"), "Shard'ın iç parça boyu")?;
                let (l, ic, f, s) = codecs_v3(
                    conf.get("codecs")
                        .and_then(Value::as_array)
                        .map_or(&[][..], Vec::as_slice),
                )?;
                if s.is_some() {
                    return Err("iç içe sharding okunmuyor.".into());
                }
                let (_, index_codecs, _, _) = codecs_v3(
                    conf.get("index_codecs")
                        .and_then(Value::as_array)
                        .map_or(&[][..], Vec::as_slice),
                )?;
                let at_end = conf.get("index_location").and_then(Value::as_str) != Some("start");
                little = l;
                fortran = f;
                out = ic;
                shard = Some((inner, index_codecs, at_end, Vec::new()));
            }
            other => return Err(format!("{other} kodeği açılmıyor.")),
        }
    }
    Ok((little, out, fortran, shard))
}

fn array_v3(path: &str, m: &Value) -> Result<Array, String> {
    let shape = u64s(m.get("shape"), "Dizinin boyu")?;
    let grid = m.get("chunk_grid").ok_or("chunk_grid yok.")?;
    if grid.get("name").and_then(Value::as_str) != Some("regular") {
        return Err("yalnız düzenli parça ızgarası okunur.".into());
    }
    let outer = u64s(
        grid.get("configuration").and_then(|c| c.get("chunk_shape")),
        "Dizinin parça boyu",
    )?;
    if outer.len() != shape.len() || outer.contains(&0) {
        return Err("parça boyu dizinin boyuyla uyuşmuyor.".into());
    }
    let dt = m
        .get("data_type")
        .and_then(Value::as_str)
        .ok_or("data_type yok.")?;
    let list = m
        .get("codecs")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let (little, codecs, fortran, sharded) = codecs_v3(&list)?;
    let stored = dtype_v3(dt, little)?;
    let enc = m.get("chunk_key_encoding");
    let enc_name = enc
        .and_then(|e| e.get("name"))
        .and_then(Value::as_str)
        .unwrap_or("default");
    let sep = enc
        .and_then(|e| e.get("configuration"))
        .and_then(|c| c.get("separator"))
        .and_then(Value::as_str)
        .and_then(|s| s.chars().next())
        .unwrap_or(if enc_name == "v2" { '.' } else { '/' });
    let (chunk, shard) = match sharded {
        Some((inner, index_codecs, at_end, _)) => {
            if inner.len() != outer.len()
                || inner.iter().zip(&outer).any(|(i, o)| *i == 0 || o % i != 0)
            {
                return Err("shard'ın iç parçaları dış parçayı bölmüyor.".into());
            }
            (
                inner,
                Some(Shard {
                    outer: outer.clone(),
                    index_at_end: at_end,
                    index_codecs,
                }),
            )
        }
        None => (outer, None),
    };
    let dims = m
        .get("dimension_names")
        .and_then(Value::as_array)
        .and_then(|a| {
            a.iter()
                .map(|x| x.as_str().map(str::to_owned))
                .collect::<Option<Vec<_>>>()
        });
    Ok(Array {
        path: path.to_owned(),
        shape,
        chunk,
        stored,
        codecs,
        fill: fill_of(m.get("fill_value")),
        declared: !matches!(m.get("fill_value"), None | Some(Value::Null)),
        fortran,
        separator: sep,
        v3: enc_name != "v2",
        shard,
        dims,
        attrs: m
            .get("attributes")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default(),
    })
}

/// A JSON attribute as NetCDF's.
fn attr_of(v: &Value) -> Option<Attr> {
    match v {
        Value::String(s) => Some(Attr::Text(s.clone())),
        Value::Bool(b) => Some(Attr::Numbers {
            kind: NcType::Byte,
            values: vec![f64::from(u8::from(*b))],
        }),
        Value::Number(n) => Some(number_attr(std::slice::from_ref(n))),
        Value::Array(a) if !a.is_empty() && a.iter().all(Value::is_number) => {
            let ns: Vec<serde_json::Number> =
                a.iter().filter_map(|x| x.as_number().cloned()).collect();
            Some(number_attr(&ns))
        }
        Value::Array(a) if !a.is_empty() && a.iter().all(Value::is_string) => Some(Attr::Text(
            a.iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join("\n"),
        )),
        _ => None,
    }
}

fn number_attr(ns: &[serde_json::Number]) -> Attr {
    if ns.iter().all(|n| n.is_i64() || n.is_u64()) {
        let values: Vec<f64> = ns.iter().filter_map(serde_json::Number::as_f64).collect();
        let kind = if values
            .iter()
            .all(|v| (i32::MIN as f64..=i32::MAX as f64).contains(v))
        {
            NcType::Int
        } else {
            NcType::Int64
        };
        Attr::Numbers { kind, values }
    } else {
        Attr::Numbers {
            kind: NcType::Double,
            values: ns.iter().filter_map(serde_json::Number::as_f64).collect(),
        }
    }
}

fn attrs_of(m: &serde_json::Map<String, Value>) -> Vec<(String, Attr)> {
    m.iter()
        .filter_map(|(k, v)| attr_of(v).map(|a| (k.clone(), a)))
        .collect()
}

fn build(
    version: u8,
    arrays: Vec<Array>,
    gattrs: serde_json::Map<String, Value>,
    mut notes: Vec<String>,
) -> Result<Model, String> {
    let mut dims: Vec<Dim> = Vec::new();
    let mut by_name: HashMap<String, usize> = HashMap::new();
    let mut phony: BTreeMap<u64, usize> = BTreeMap::new();
    let mut vars = Vec::new();
    for a in arrays {
        let name = a.path.trim_matches('/').to_owned();
        let name = if name.is_empty() {
            "data".to_owned()
        } else {
            name
        };
        let vdims: Vec<usize> = match &a.dims {
            Some(names) if names.len() == a.shape.len() => {
                let mut ks = Vec::with_capacity(names.len());
                let mut clash = false;
                for (n, &len) in names.iter().zip(&a.shape) {
                    match by_name.get(n) {
                        Some(&k) if dims[k].len == len => ks.push(k),
                        Some(_) => clash = true,
                        None => {
                            let k = dims.len();
                            dims.push(Dim {
                                name: n.clone(),
                                len,
                            });
                            by_name.insert(n.clone(), k);
                            ks.push(k);
                        }
                    }
                }
                if clash {
                    notes.push(format!(
                        "“{name}”: boyutlarının boyları başka dizilerinkiyle uyuşmuyor; okunmuyor."
                    ));
                    continue;
                }
                ks
            }
            _ => a
                .shape
                .iter()
                .map(|&len| {
                    *phony.entry(len).or_insert_with(|| {
                        let k = dims.len();
                        dims.push(Dim {
                            name: format!("phony_dim_{k}"),
                            len,
                        });
                        k
                    })
                })
                .collect(),
        };
        let mut attrs = attrs_of(&a.attrs);
        if a.declared && !a.fill.is_nan() && !attrs.iter().any(|(k, _)| k == "_FillValue") {
            attrs.push((
                "_FillValue".into(),
                Attr::Numbers {
                    kind: a.stored.kind,
                    values: vec![a.fill],
                },
            ));
        }
        let prefix = if a.path.is_empty() {
            String::new()
        } else {
            format!("{}/", a.path.trim_matches('/'))
        };
        vars.push(Variable {
            name,
            dims: vdims,
            kind: a.stored.kind,
            attrs,
            data: Data::Chunked(Chunked {
                chunk: a.chunk,
                decoder: Decoder::Plain {
                    stored: a.stored,
                    codecs: a.codecs,
                },
                chunks: Chunks::Keys {
                    prefix,
                    separator: a.separator,
                    v3: a.v3,
                    shard: a.shard,
                },
                fortran: a.fortran,
                fill: a.fill,
            }),
        });
    }
    Ok(Model {
        format: format!("Zarr v{version}"),
        dims,
        attrs: attrs_of(&gattrs),
        vars,
        notes,
    })
}
