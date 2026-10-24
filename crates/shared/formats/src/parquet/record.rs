//! A column's records with their meaning (docs/adr/0628 §2): physical values read through their logical type (dates,
//! times, timestamps and INT96, decimals of any width, unsigned and small integers, half floats, UUIDs, strings) and
//! nested columns assembled — structs, lists (three-level, two-level and the legacy forms) and maps — as JSON values.

use std::collections::HashMap;

use serde_json::Value as Json;

use super::encoding::Values;
use super::meta::{Logical, Physical, Repetition, TimeUnit};
use super::nest::{Nested, nest};
use super::read::{File, Leaf, Source};
use super::schema::{Node, Schema};

/// The proleptic Gregorian date of a day count from 1970-01-01 (Howard Hinnant's `civil_from_days`).
pub fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

pub fn date_text(days: i64) -> String {
    let (y, m, d) = civil(days);
    format!("{y:04}-{m:02}-{d:02}")
}

fn per_second(unit: TimeUnit) -> (i64, usize) {
    match unit {
        TimeUnit::Millis => (1_000, 3),
        TimeUnit::Micros => (1_000_000, 6),
        TimeUnit::Nanos => (1_000_000_000, 9),
    }
}

/// `.` and the fraction, its trailing zeros dropped; nothing for none.
fn fraction(frac: i64, digits: usize) -> String {
    if frac == 0 {
        return String::new();
    }
    let s = format!("{frac:0digits$}");
    format!(".{}", s.trim_end_matches('0'))
}

fn clock(seconds: i64, frac: i64, digits: usize) -> String {
    format!(
        "{:02}:{:02}:{:02}{}",
        seconds / 3600,
        seconds / 60 % 60,
        seconds % 60,
        fraction(frac, digits)
    )
}

/// A time of day from midnight in `unit`.
pub fn time_text(value: i64, unit: TimeUnit) -> String {
    let (per, digits) = per_second(unit);
    clock(value.div_euclid(per), value.rem_euclid(per), digits)
}

/// A timestamp from the epoch in `unit`; `Z` when it is UTC's.
pub fn timestamp_text(value: i64, unit: TimeUnit, utc: bool) -> String {
    let (per, digits) = per_second(unit);
    let seconds = value.div_euclid(per);
    let frac = value.rem_euclid(per);
    format!(
        "{}T{}{}",
        date_text(seconds.div_euclid(86_400)),
        clock(seconds.rem_euclid(86_400), frac, digits),
        if utc { "Z" } else { "" }
    )
}

/// INT96 (Impala's timestamp): nanoseconds of the day, then the Julian day, as a UTC timestamp.
pub fn int96_text(b: &[u8; 12]) -> String {
    let nanos = i64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]);
    let julian = i64::from(i32::from_le_bytes([b[8], b[9], b[10], b[11]]));
    let days = julian - 2_440_588;
    let seconds = nanos.div_euclid(1_000_000_000);
    let frac = nanos.rem_euclid(1_000_000_000);
    let day = days + seconds.div_euclid(86_400);
    format!(
        "{}T{}Z",
        date_text(day),
        clock(seconds.rem_euclid(86_400), frac, 9)
    )
}

/// A decimal from its unscaled value, big-endian two's complement of any width.
pub fn decimal_text(be: &[u8], scale: i32) -> String {
    let negative = be.first().is_some_and(|&b| b & 0x80 != 0);
    let mut mag: Vec<u8> = be.to_vec();
    if negative {
        // The magnitude: invert and add one.
        for b in &mut mag {
            *b = !*b;
        }
        for b in mag.iter_mut().rev() {
            let (v, carry) = b.overflowing_add(1);
            *b = v;
            if !carry {
                break;
            }
        }
    }
    // Base 256 to base 10 by repeated division.
    let mut digits = Vec::new();
    while mag.iter().any(|&b| b != 0) {
        let mut rem = 0u32;
        for b in &mut mag {
            let cur = (rem << 8) | u32::from(*b);
            *b = (cur / 10) as u8;
            rem = cur % 10;
        }
        digits.push(b'0' + rem as u8);
    }
    if digits.is_empty() {
        digits.push(b'0');
    }
    digits.reverse();
    let mut s = String::from_utf8(digits).unwrap_or_default();
    if scale > 0 {
        let scale = scale as usize;
        if s.len() <= scale {
            s = format!("{}{s}", "0".repeat(scale + 1 - s.len()));
        }
        s.insert(s.len() - scale, '.');
    } else if scale < 0 {
        s.push_str(&"0".repeat((-scale) as usize));
    }
    if negative { format!("-{s}") } else { s }
}

/// A half float.
pub fn half(bits: u16) -> f32 {
    let sign = if bits & 0x8000 != 0 { -1.0f32 } else { 1.0 };
    let exp = i32::from((bits >> 10) & 0x1f);
    let frac = f32::from(bits & 0x3ff);
    sign * match exp {
        0 => frac * 2f32.powi(-24),
        31 if frac == 0.0 => f32::INFINITY,
        31 => f32::NAN,
        e => (1.0 + frac / 1024.0) * 2f32.powi(e - 15),
    }
}

fn hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(2 + 2 * b.len());
    s.push_str("0x");
    for x in b {
        s.push_str(&format!("{x:02x}"));
    }
    s
}

fn number(v: f64) -> Json {
    serde_json::Number::from_f64(v).map_or(Json::Null, Json::Number)
}

/// Value `i` of `values` with the meaning of column node `node`.
pub fn value(values: &Values, i: usize, node: &Node) -> Json {
    let logical = node.logical.as_ref();
    match values {
        Values::Bool(v) => Json::Bool(v[i]),
        Values::I32(v) => match logical {
            Some(Logical::Date) => Json::String(date_text(i64::from(v[i]))),
            Some(Logical::Time { unit, .. }) => Json::String(time_text(i64::from(v[i]), *unit)),
            Some(Logical::Integer { signed: false, .. }) => Json::from(v[i] as u32),
            Some(Logical::Decimal { scale, .. }) => {
                Json::String(decimal_text(&v[i].to_be_bytes(), *scale))
            }
            _ => Json::from(v[i]),
        },
        Values::I64(v) => match logical {
            Some(Logical::Time { unit, .. }) => Json::String(time_text(v[i], *unit)),
            Some(Logical::Timestamp { utc, unit }) => {
                Json::String(timestamp_text(v[i], *unit, *utc))
            }
            Some(Logical::Integer { signed: false, .. }) => Json::from(v[i] as u64),
            Some(Logical::Decimal { scale, .. }) => {
                Json::String(decimal_text(&v[i].to_be_bytes(), *scale))
            }
            _ => Json::from(v[i]),
        },
        Values::I96(v) => Json::String(int96_text(&v[i])),
        Values::F32(v) => number(f64::from(v[i])),
        Values::F64(v) => number(v[i]),
        Values::Bytes(b) => {
            let raw = b.get(i);
            match logical {
                Some(Logical::String | Logical::Enum | Logical::Json) => {
                    Json::String(String::from_utf8_lossy(raw).into_owned())
                }
                Some(Logical::Decimal { scale, .. }) => Json::String(decimal_text(raw, *scale)),
                Some(Logical::Uuid) if raw.len() == 16 => {
                    let h = hex(raw);
                    Json::String(format!(
                        "{}-{}-{}-{}-{}",
                        &h[2..10],
                        &h[10..14],
                        &h[14..18],
                        &h[18..22],
                        &h[22..]
                    ))
                }
                Some(Logical::Float16) if raw.len() == 2 => {
                    number(f64::from(half(u16::from_le_bytes([raw[0], raw[1]]))))
                }
                _ => Json::String(hex(raw)),
            }
        }
    }
}

/// What a node's records are made of.
#[derive(Clone, Debug)]
pub enum Shape {
    Leaf {
        column: usize,
    },
    Struct {
        node: usize,
        fields: Vec<(String, Shape)>,
    },
    /// The lists of repeated level `level` (0: one per record).
    List {
        level: usize,
        element: Box<Shape>,
    },
}

const DEPTH: usize = 32;

fn element_of(s: &Schema, n: usize, level: usize, depth: usize) -> Result<Shape, String> {
    let node = &s.nodes[n];
    if let Some(c) = node.column {
        return Ok(Shape::Leaf { column: c });
    }
    let mut fields = Vec::new();
    for &c in &node.children {
        fields.push((s.nodes[c].name.clone(), shape_at(s, c, level, depth + 1)?));
    }
    Ok(Shape::Struct { node: n, fields })
}

fn shape_at(s: &Schema, n: usize, level: usize, depth: usize) -> Result<Shape, String> {
    if depth > DEPTH {
        return Err("Parquet sütunu çok derin".into());
    }
    let node = &s.nodes[n];
    if node.repetition == Repetition::Repeated {
        return Ok(Shape::List {
            level,
            element: Box::new(element_of(s, n, level + 1, depth)?),
        });
    }
    if let Some(c) = node.column {
        return Ok(Shape::Leaf { column: c });
    }
    let repeated_child = || -> Result<usize, String> {
        match node.children.as_slice() {
            [c] if s.nodes[*c].repetition == Repetition::Repeated => Ok(*c),
            _ => Err(format!(
                "“{}” listesinin tek tekrarlı çocuğu yok",
                node.name
            )),
        }
    };
    match &node.logical {
        Some(Logical::List) => {
            let r = repeated_child()?;
            let rn = &s.nodes[r];
            let element = if rn.column.is_some() {
                Shape::Leaf {
                    column: rn.column.unwrap_or(0),
                }
            } else if rn.children.len() == 1
                && rn.name != "array"
                && rn.name != format!("{}_tuple", node.name)
            {
                shape_at(s, rn.children[0], level + 1, depth + 1)?
            } else {
                element_of(s, r, level + 1, depth)?
            };
            Ok(Shape::List {
                level,
                element: Box::new(element),
            })
        }
        Some(Logical::Map) => {
            let r = repeated_child()?;
            Ok(Shape::List {
                level,
                element: Box::new(element_of(s, r, level + 1, depth)?),
            })
        }
        _ => element_of(s, n, level, depth),
    }
}

/// The shape of top-level node `n`.
pub fn shape(s: &Schema, n: usize) -> Result<Shape, String> {
    shape_at(s, n, 0, 0)
}

/// The columns of a node read and assembled.
pub struct Assembled {
    pub leaves: HashMap<usize, (Leaf, Nested)>,
    pub records: usize,
}

/// Reads every column under top-level node `top` of row group `group`.
pub fn assemble<S: Source>(file: &File<S>, group: usize, top: usize) -> Result<Assembled, String> {
    let mut leaves = HashMap::new();
    let mut records = None;
    for c in file.schema.columns_under(top) {
        let leaf = file.read(group, c)?;
        let node = &file.schema.nodes[file.schema.columns[c]];
        let n = nest(
            &leaf.def,
            &leaf.rep,
            leaf.slots,
            node.def,
            &file.schema.repeated_defs(c),
        )?;
        if *records.get_or_insert(n.records) != n.records {
            return Err("Parquet sütunlarının kayıt sayıları tutmuyor".into());
        }
        leaves.insert(c, (leaf, n));
    }
    Ok(Assembled {
        leaves,
        records: records.unwrap_or(0),
    })
}

fn first_leaf(shape: &Shape) -> Option<usize> {
    match shape {
        Shape::Leaf { column } => Some(*column),
        Shape::Struct { fields, .. } => fields.iter().find_map(|(_, f)| match f {
            Shape::Leaf { column } => Some(*column),
            Shape::Struct { .. } => first_leaf(f),
            Shape::List { .. } => None,
        }),
        Shape::List { .. } => None,
    }
}

/// The value at position `pos` (a record, or an element of the list level the shape sits in).
pub fn build(s: &Schema, a: &Assembled, shape: &Shape, pos: usize) -> Json {
    match shape {
        Shape::Leaf { column } => {
            let Some((leaf, n)) = a.leaves.get(column) else {
                return Json::Null;
            };
            match n.values.get(pos).copied().flatten() {
                Some(v) => value(&leaf.values, v, &s.nodes[s.columns[*column]]),
                None => Json::Null,
            }
        }
        Shape::Struct { node, fields } => {
            if let Some(c) = first_leaf(shape)
                && let Some((_, n)) = a.leaves.get(&c)
                && n.defs.get(pos).is_some_and(|&d| d < s.nodes[*node].def)
            {
                return Json::Null;
            }
            Json::Object(
                fields
                    .iter()
                    .map(|(name, f)| (name.clone(), build(s, a, f, pos)))
                    .collect(),
            )
        }
        Shape::List { level, element } => {
            let Some(c) = list_leaf(element) else {
                return Json::Null;
            };
            let Some((_, n)) = a.leaves.get(&c) else {
                return Json::Null;
            };
            if !n.present[*level].get(pos).copied().unwrap_or(false) {
                return Json::Null;
            }
            Json::Array(
                n.range(*level, pos)
                    .map(|e| build(s, a, element, e))
                    .collect(),
            )
        }
    }
}

/// A column under a list's element, to read the list's offsets from.
fn list_leaf(shape: &Shape) -> Option<usize> {
    match shape {
        Shape::Leaf { column } => Some(*column),
        Shape::Struct { fields, .. } => fields.iter().find_map(|(_, f)| list_leaf(f)),
        Shape::List { element, .. } => list_leaf(element),
    }
}

/// Every record of top-level node `top` in row group `group` as JSON.
pub fn records<S: Source>(file: &File<S>, group: usize, top: usize) -> Result<Vec<Json>, String> {
    let shape = shape(&file.schema, top)?;
    let a = assemble(file, group, top)?;
    Ok((0..a.records)
        .map(|r| build(&file.schema, &a, &shape, r))
        .collect())
}

/// Whether a column is of a physical type with no meaning of its own (bytes, INT96, an interval).
pub fn opaque(node: &Node) -> bool {
    matches!(
        node.physical,
        Some(Physical::ByteArray | Physical::FixedLenByteArray)
    ) && !matches!(
        node.logical,
        Some(
            Logical::String
                | Logical::Enum
                | Logical::Json
                | Logical::Decimal { .. }
                | Logical::Uuid
                | Logical::Float16
        )
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_times_and_decimals() {
        assert_eq!(date_text(0), "1970-01-01");
        assert_eq!(date_text(-1), "1969-12-31");
        assert_eq!(date_text(19_000), "2022-01-08");
        assert_eq!(time_text(45_296_789, TimeUnit::Millis), "12:34:56.789");
        assert_eq!(
            timestamp_text(-1, TimeUnit::Micros, true),
            "1969-12-31T23:59:59.999999Z"
        );
        assert_eq!(decimal_text(&12_345i32.to_be_bytes(), 2), "123.45");
        assert_eq!(decimal_text(&(-5i64).to_be_bytes(), 3), "-0.005");
        assert_eq!(
            decimal_text(&[0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], 0),
            "-170141183460469231731687303715884105728"
        );
        assert_eq!(half(0x3c00), 1.0);
        assert_eq!(half(0xc000), -2.0);
    }
}
