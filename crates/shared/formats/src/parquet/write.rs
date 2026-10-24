//! A Parquet file written (docs/adr/0628 §1): top-level columns, optional or required, and structs of required
//! members (GeoParquet's covering box); row groups of a set number of rows, data pages v1 of about a set size with
//! their definition levels in the RLE hybrid and PLAIN values, Snappy or no compression, each chunk's statistics.

use super::encoding::{self, Values};
use super::meta::{
    self, ColumnMeta, Element, FileMeta, Logical, Physical, Repetition, RowGroup, Statistics,
};
use super::rle;
use crate::compress::{Codec, snappy};

/// A column to write: its rows' values, nulls left out.
#[derive(Clone, Debug)]
pub struct Column {
    pub name: String,
    /// The struct it is a member of (members of one struct follow one another); the struct is optional, its members
    /// required.
    pub group: Option<String>,
    pub physical: Physical,
    pub logical: Option<Logical>,
    /// Whether a row may have none (a member's: whether the struct may be missing).
    pub optional: bool,
    /// For each row whether it has a value; empty for a required column.
    pub present: Vec<bool>,
    pub values: Values,
}

/// How the file is written.
#[derive(Clone, Debug)]
pub struct Options {
    pub codec: Codec,
    pub row_group: usize,
    /// About how many bytes of values a page holds.
    pub page_bytes: usize,
    pub key_value: Vec<(String, String)>,
    pub created_by: String,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            codec: Codec::Snappy,
            row_group: 65_536,
            page_bytes: 1 << 20,
            key_value: Vec::new(),
            created_by: "KentOS CAD".into(),
        }
    }
}

fn value_size(v: &Values, i: usize) -> usize {
    match v {
        Values::Bool(_) => 1,
        Values::I32(_) | Values::F32(_) => 4,
        Values::I64(_) | Values::F64(_) => 8,
        Values::I96(_) => 12,
        Values::Bytes(b) => 4 + b.get(i).len(),
    }
}

fn slice(v: &Values, r: std::ops::Range<usize>) -> Values {
    match v {
        Values::Bool(x) => Values::Bool(x[r].to_vec()),
        Values::I32(x) => Values::I32(x[r].to_vec()),
        Values::I64(x) => Values::I64(x[r].to_vec()),
        Values::I96(x) => Values::I96(x[r].to_vec()),
        Values::F32(x) => Values::F32(x[r].to_vec()),
        Values::F64(x) => Values::F64(x[r].to_vec()),
        Values::Bytes(b) => {
            let mut out = encoding::Bytes::default();
            for i in r {
                out.push(b.get(i));
            }
            Values::Bytes(out)
        }
    }
}

fn float_bounds(it: impl Iterator<Item = f64>) -> Option<(f64, f64)> {
    let mut b: Option<(f64, f64)> = None;
    for v in it.filter(|v| !v.is_nan()) {
        b = Some(b.map_or((v, v), |(lo, hi)| (lo.min(v), hi.max(v))));
    }
    // Zero's sign: the smallest is written -0.0, the largest +0.0 (parquet.thrift's ColumnOrder).
    b.map(|(lo, hi)| {
        (
            if lo == 0.0 { -0.0 } else { lo },
            if hi == 0.0 { 0.0 } else { hi },
        )
    })
}

/// The statistics of a chunk's values.
fn statistics(v: &Values, nulls: usize, logical: &Option<Logical>) -> Statistics {
    let mut s = Statistics {
        null_count: Some(nulls as i64),
        ..Statistics::default()
    };
    let (lo, hi): (Option<Vec<u8>>, Option<Vec<u8>>) = match v {
        Values::Bool(x) if !x.is_empty() => (
            Some(vec![u8::from(x.iter().all(|&b| b))]),
            Some(vec![u8::from(x.iter().any(|&b| b))]),
        ),
        Values::I32(x) => match (x.iter().min(), x.iter().max()) {
            (Some(a), Some(b)) => (
                Some(a.to_le_bytes().to_vec()),
                Some(b.to_le_bytes().to_vec()),
            ),
            _ => (None, None),
        },
        Values::I64(x) => match (x.iter().min(), x.iter().max()) {
            (Some(a), Some(b)) => (
                Some(a.to_le_bytes().to_vec()),
                Some(b.to_le_bytes().to_vec()),
            ),
            _ => (None, None),
        },
        Values::F64(x) => match float_bounds(x.iter().copied()) {
            Some((a, b)) => (
                Some(a.to_le_bytes().to_vec()),
                Some(b.to_le_bytes().to_vec()),
            ),
            None => (None, None),
        },
        Values::F32(x) => match float_bounds(x.iter().map(|&v| f64::from(v))) {
            Some((a, b)) => (
                Some((a as f32).to_le_bytes().to_vec()),
                Some((b as f32).to_le_bytes().to_vec()),
            ),
            None => (None, None),
        },
        Values::Bytes(b) if matches!(logical, Some(Logical::String)) && !b.is_empty() => {
            let (mut lo, mut hi) = (b.get(0), b.get(0));
            for i in 1..b.len() {
                let x = b.get(i);
                lo = lo.min(x);
                hi = hi.max(x);
            }
            if lo.len() <= 64 && hi.len() <= 64 {
                (Some(lo.to_vec()), Some(hi.to_vec()))
            } else {
                (None, None)
            }
        }
        _ => (None, None),
    };
    s.min_value = lo;
    s.max_value = hi;
    s
}

fn compress(codec: Codec, body: &[u8]) -> Result<Vec<u8>, String> {
    match codec {
        Codec::None => Ok(body.to_vec()),
        Codec::Snappy => Ok(snappy::compress(body)),
        c => Err(format!("{} sıkıştırmasıyla yazılmaz", c.name())),
    }
}

/// The file's bytes.
pub fn write(columns: &[Column], rows: usize, o: &Options) -> Result<Vec<u8>, String> {
    for c in columns {
        let n = if c.optional {
            c.present.iter().filter(|&&p| p).count()
        } else {
            rows
        };
        if (c.optional && c.present.len() != rows) || c.values.len() != n {
            return Err(format!(
                "“{}” sütununun değerleri satır sayısıyla tutmuyor",
                c.name
            ));
        }
    }
    // The schema: the root, then each top-level column or struct with its members.
    let mut schema = vec![Element {
        name: "schema".into(),
        physical: None,
        type_length: None,
        repetition: None,
        children: Some(0),
        logical: None,
    }];
    let mut top = 0;
    let mut i = 0;
    while i < columns.len() {
        top += 1;
        if let Some(g) = &columns[i].group {
            let members = columns[i..]
                .iter()
                .take_while(|c| c.group.as_ref() == Some(g))
                .count();
            schema.push(Element {
                name: g.clone(),
                physical: None,
                type_length: None,
                repetition: Some(if columns[i].optional {
                    Repetition::Optional
                } else {
                    Repetition::Required
                }),
                children: Some(members as i32),
                logical: None,
            });
            for c in &columns[i..i + members] {
                schema.push(Element {
                    name: c.name.clone(),
                    physical: Some(c.physical),
                    type_length: None,
                    repetition: Some(Repetition::Required),
                    children: None,
                    logical: c.logical.clone(),
                });
            }
            i += members;
        } else {
            let c = &columns[i];
            schema.push(Element {
                name: c.name.clone(),
                physical: Some(c.physical),
                type_length: None,
                repetition: Some(if c.optional {
                    Repetition::Optional
                } else {
                    Repetition::Required
                }),
                children: None,
                logical: c.logical.clone(),
            });
            i += 1;
        }
    }
    schema[0].children = Some(top);

    let mut out: Vec<u8> = b"PAR1".to_vec();
    let mut groups = Vec::new();
    let mut cursor = vec![0usize; columns.len()];
    let step = o.row_group.max(1);
    let mut r0 = 0;
    while r0 < rows {
        let r1 = (r0 + step).min(rows);
        let mut metas = Vec::with_capacity(columns.len());
        let mut group_bytes = 0i64;
        for (ci, c) in columns.iter().enumerate() {
            let present: Vec<bool> = if c.optional {
                c.present[r0..r1].to_vec()
            } else {
                vec![true; r1 - r0]
            };
            let count = present.iter().filter(|&&p| p).count();
            let v0 = cursor[ci];
            cursor[ci] += count;
            let chunk_values = slice(&c.values, v0..v0 + count);
            let path = match &c.group {
                Some(g) => vec![g.clone(), c.name.clone()],
                None => vec![c.name.clone()],
            };
            let start = out.len();
            let mut uncompressed = 0usize;
            // Pages of about `page_bytes` of values.
            let (mut row, mut value) = (0usize, 0usize);
            while row < present.len() {
                let (row_start, value_start) = (row, value);
                let mut bytes = 0usize;
                while row < present.len() && (bytes < o.page_bytes || row == row_start) {
                    if present[row] {
                        bytes += value_size(&chunk_values, value);
                        value += 1;
                    }
                    row += 1;
                }
                let mut body = Vec::with_capacity(bytes + 16);
                if c.optional {
                    let levels: Vec<u32> = present[row_start..row]
                        .iter()
                        .map(|&p| u32::from(p))
                        .collect();
                    let mut l = Vec::new();
                    rle::write(&levels, 1, &mut l);
                    body.extend_from_slice(&(l.len() as u32).to_le_bytes());
                    body.extend_from_slice(&l);
                }
                encoding::write_plain(&slice(&chunk_values, value_start..value), &mut body);
                let packed = compress(o.codec, &body)?;
                let before = out.len();
                meta::write_data_page_header(&mut out, body.len(), packed.len(), row - row_start);
                uncompressed += out.len() - before + body.len();
                out.extend_from_slice(&packed);
            }
            let written = out.len() - start;
            group_bytes += uncompressed as i64;
            metas.push(ColumnMeta {
                physical: c.physical,
                encodings: vec![0, 3],
                path,
                codec: o.codec.parquet(),
                num_values: (r1 - r0) as i64,
                uncompressed: uncompressed as i64,
                compressed: written as i64,
                data_page_offset: start as i64,
                dictionary_page_offset: None,
                statistics: Some(statistics(&chunk_values, r1 - r0 - count, &c.logical)),
            });
        }
        groups.push(RowGroup {
            columns: metas,
            total_byte_size: group_bytes,
            num_rows: (r1 - r0) as i64,
        });
        r0 = r1;
    }
    let footer = meta::write_file_meta(&FileMeta {
        version: 2,
        schema,
        num_rows: rows as i64,
        row_groups: groups,
        key_value: o
            .key_value
            .iter()
            .map(|(k, v)| (k.clone(), Some(v.clone())))
            .collect(),
        created_by: Some(o.created_by.clone()),
    });
    out.extend_from_slice(&footer);
    out.extend_from_slice(&(footer.len() as u32).to_le_bytes());
    out.extend_from_slice(b"PAR1");
    Ok(out)
}
