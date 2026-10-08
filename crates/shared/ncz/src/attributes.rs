//! `@TAB` records, the reference's `_extract_attribute_tables`: every record
//! of each `@TABn` reference, decoded its way (a label, a segment, or the
//! printable values of an unknown shape). They are not tied to any object in
//! the file the reference or this reader can see, so the import counts and
//! names them and attaches nothing (docs/adr/0138).

use std::collections::{BTreeMap, HashSet};

use kentos_formats::watch::Watch;

/// One value of a row: absent (the reference's `None`), an integer, a float
/// or a text.
#[derive(Clone, Debug, PartialEq)]
pub enum Cell {
    None,
    Int(i64),
    Float(f64),
    Text(String),
}

/// One `@TAB` record; the columns keep the order the reference inserts them in.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub row_index: usize,
    pub columns: Vec<(&'static str, Cell)>,
}

/// Every record of one `@TABn` reference, in file order.
#[derive(Clone, Debug, PartialEq)]
pub struct Table {
    pub table_ref: String,
    pub rows: Vec<Row>,
}

/// `_safe_round`: none for a non-finite value, 0 below 1e-12, else
/// `round(value, 6)`, Python's correctly rounded one: the shortest decimal
/// of the value at six places, read back (both steps round the exact binary
/// value, as CPython's do).
fn safe_round(value: f64) -> Cell {
    if !value.is_finite() {
        return Cell::None;
    }
    if value.abs() < 1e-12 {
        return Cell::Float(0.0);
    }
    Cell::Float(format!("{value:.6}").parse().unwrap_or(value))
}

fn number(c: &Cell) -> Option<f64> {
    match c {
        Cell::Float(v) => Some(*v),
        _ => None,
    }
}

/// `_looks_like_xy`.
fn looks_like_xy(cx: &Cell, cy: &Cell) -> bool {
    let (Some(x), Some(y)) = (number(cx), number(cy)) else {
        return false;
    };
    x.is_finite()
        && y.is_finite()
        && x.abs() <= 100_000_000.0
        && y.abs() <= 100_000_000.0
        && (x.abs() >= 1000.0 || y.abs() >= 1000.0)
}

/// One record's bytes and the readers `_parse_attribute_row` uses on them.
struct Chunk<'a>(&'a [u8]);

impl Chunk<'_> {
    fn u16(&self, at: usize) -> i64 {
        match self.0.get(at..at + 2) {
            Some(b) => i64::from(b[0]) | (i64::from(b[1]) << 8),
            None => 0,
        }
    }
    fn u32(&self, at: usize) -> i64 {
        match self.0.get(at..at + 4) {
            Some(b) => i64::from(u32::from_le_bytes([b[0], b[1], b[2], b[3]])),
            None => 0,
        }
    }
    fn f32(&self, at: usize) -> f64 {
        match self.0.get(at..at + 4) {
            Some(b) => f64::from(f32::from_le_bytes([b[0], b[1], b[2], b[3]])),
            None => 0.0,
        }
    }
    fn f64(&self, at: usize) -> f64 {
        match self.0.get(at..at + 8) {
            Some(b) => f64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]),
            None => 0.0,
        }
    }
    fn at_or_zero(&self, at: usize) -> i64 {
        self.0.get(at).map_or(0, |&b| i64::from(b))
    }
}

fn ascii(b: &[u8]) -> String {
    b.iter().map(|&c| char::from(c)).collect()
}

/// `_collect_ascii_fields`, joined the way its caller joins them. The run of
/// printable bytes from each position is worked out once, backwards, so
/// "are these L bytes printable" is one comparison rather than L.
fn ascii_values(chunk: &[u8], table_ref: &str) -> String {
    if chunk.len() < 2 {
        return String::new();
    }
    let mut run = vec![0u32; chunk.len() + 1];
    for i in (0..chunk.len()).rev() {
        run[i] = if (32..127).contains(&chunk[i]) {
            run[i + 1] + 1
        } else {
            0
        };
    }
    let mut values: Vec<&[u8]> = Vec::new();
    let mut seen: HashSet<&[u8]> = HashSet::new();
    for index in 0..chunk.len() - 1 {
        let length = usize::from(chunk[index]);
        if length == 0 || length > 64 || index + 1 + length > chunk.len() {
            continue;
        }
        if (run[index + 1] as usize) < length {
            continue;
        }
        let mut value = &chunk[index + 1..index + 1 + length];
        while let [b' ', rest @ ..] = value {
            value = rest;
        }
        while let [rest @ .., b' '] = value {
            value = rest;
        }
        if value.is_empty() || !seen.insert(value) {
            continue;
        }
        values.push(value);
    }
    values
        .iter()
        .filter(|v| **v != table_ref.as_bytes())
        .map(|v| ascii(v))
        .collect::<Vec<_>>()
        .join(" | ")
}

/// `_parse_attribute_row`.
fn parse_row(chunk: &[u8], table_ref: &str, row_index: usize) -> Row {
    let r = Chunk(chunk);
    let size = chunk.len();
    let mut col: Vec<(&'static str, Cell)> = vec![
        ("row_variant", Cell::Text("unknown".into())),
        ("record_length", Cell::Int(size as i64)),
    ];
    if size >= 11 {
        // `.decode('ascii', errors='ignore').strip('\x00 ')`
        let inline: Vec<u8> = chunk[1..11].iter().copied().filter(|b| *b < 0x80).collect();
        let mut s = inline.as_slice();
        while let [0 | b' ', rest @ ..] = s {
            s = rest;
        }
        while let [rest @ .., 0 | b' '] = s {
            s = rest;
        }
        col.push(("table_ref_inline", Cell::Text(ascii(s))));
    }

    let label_length = if size > 28 { usize::from(chunk[28]) } else { 0 };
    let mut label = String::new();
    if (1..=64).contains(&label_length) && 29 + label_length <= size {
        let bytes = &chunk[29..29 + label_length];
        if bytes.iter().all(|&c| (32..127).contains(&c)) {
            label = ascii(bytes).trim_matches(' ').to_owned();
        }
    }

    if !label.is_empty() {
        let sep = 29 + label_length;
        col[0].1 = Cell::Text("label".into());
        col.extend([
            ("label", Cell::Text(label)),
            ("label_length", Cell::Int(label_length as i64)),
            ("prefix_float", safe_round(r.f32(17))),
            ("code_u16", Cell::Int(r.u16(25))),
            ("separator_1", Cell::Int(r.at_or_zero(sep))),
            ("style_code", Cell::Int(r.u32(sep + 1))),
            ("flag_1", Cell::Int(r.at_or_zero(sep + 5))),
            ("flag_2", Cell::Int(r.at_or_zero(sep + 6))),
            ("flag_3", Cell::Int(r.at_or_zero(sep + 7))),
            ("coord_1_x", safe_round(r.f64(sep + 8))),
            ("coord_1_y", safe_round(r.f64(sep + 16))),
            ("separator_2", Cell::Int(r.at_or_zero(sep + 35))),
            ("scale_float", safe_round(r.f32(sep + 46))),
            ("coord_2_x", safe_round(r.f64(sep + 50))),
            ("coord_2_y", safe_round(r.f64(sep + 58))),
            ("coord_3_x", safe_round(r.f64(sep + 66))),
            ("coord_3_y", safe_round(r.f64(sep + 74))),
        ]);
        return Row {
            row_index,
            columns: col,
        };
    }

    if size >= 119 {
        let c = [17, 25, 45, 53, 87, 95, 103, 111].map(|at| safe_round(r.f64(at)));
        let plausible = looks_like_xy(&c[0], &c[1])
            && looks_like_xy(&c[2], &c[3])
            && looks_like_xy(&c[4], &c[5]);
        if !plausible {
            col.push(("ascii_values", Cell::Text(ascii_values(chunk, table_ref))));
            return Row {
                row_index,
                columns: col,
            };
        }
        let [c0x, c0y, c1x, c1y, c2x, c2y, c3x, c3y] = c;
        col[0].1 = Cell::Text("segment".into());
        col.extend([
            ("coord_0_x", c0x),
            ("coord_0_y", c0y),
            ("style_code", Cell::Int(r.u32(37))),
            ("flag_1", Cell::Int(r.at_or_zero(41))),
            ("flag_2", Cell::Int(r.at_or_zero(42))),
            ("flag_3", Cell::Int(r.at_or_zero(43))),
            ("flag_4", Cell::Int(r.at_or_zero(44))),
            ("coord_1_x", c1x),
            ("coord_1_y", c1y),
            ("separator_2", Cell::Int(r.at_or_zero(72))),
            ("coord_2_x", c2x),
            ("coord_2_y", c2y),
            ("coord_3_x", c3x),
            ("coord_3_y", c3y),
        ]);
        return Row {
            row_index,
            columns: col,
        };
    }

    col.push(("ascii_values", Cell::Text(ascii_values(chunk, table_ref))));
    Row {
        row_index,
        columns: col,
    }
}

/// Every `@TABn` record, sorted by reference; empty when the file has none,
/// `None` when the watch said stop. `from`..`from + width` is the share of
/// the read's progress this walk reports in.
pub fn tables(data: &[u8], watch: &mut dyn Watch, from: u64, width: u64) -> Option<Vec<Table>> {
    struct Marker {
        start: usize,
        table_ref: String,
    }
    let size = data.len().max(1) as u64;
    let mut markers: Vec<Marker> = Vec::new();
    let mut cursor = 0usize;
    let mut next_check = 0usize;
    const STRIDE: usize = 1 << 20;
    loop {
        if cursor >= next_check {
            let done = from + (cursor as u64).min(size) * width / size;
            if !watch.step(done, crate::PROGRESS_TOTAL) {
                return None;
            }
            next_check = cursor + STRIDE;
        }
        let Some(found) = data
            .get(cursor..)
            .and_then(|rest| rest.windows(4).position(|w| w == b"@TAB"))
        else {
            break;
        };
        let marker = cursor + found;
        let mut end = marker + 4;
        while end < data.len() && data[end].is_ascii_digit() {
            end += 1;
        }
        let length = end - marker;
        let start = if marker > 0 && usize::from(data[marker - 1]) == length {
            marker - 1
        } else {
            marker
        };
        markers.push(Marker {
            start,
            table_ref: ascii(&data[marker..end]),
        });
        cursor = end;
    }
    if markers.is_empty() {
        return Some(Vec::new());
    }
    let mut tables: BTreeMap<String, Vec<Row>> = BTreeMap::new();
    for (index, m) in markers.iter().enumerate() {
        let next = match markers.get(index + 1) {
            Some(n) if n.start > m.start => n.start,
            _ => data.len(),
        };
        let end = data.len().min(next);
        if end <= m.start {
            continue;
        }
        let rows = tables.entry(m.table_ref.clone()).or_default();
        let row = parse_row(&data[m.start..end], &m.table_ref, rows.len() + 1);
        rows.push(row);
    }
    Some(
        tables
            .into_iter()
            .filter(|(_, rows)| !rows.is_empty())
            .map(|(table_ref, rows)| Table { table_ref, rows })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_formats::watch::Quiet;

    #[test]
    fn python_rounds_to_six_places() {
        assert_eq!(safe_round(1.2345675), Cell::Float(1.234568));
        assert_eq!(safe_round(1e-13), Cell::Float(0.0));
        assert_eq!(safe_round(f64::NAN), Cell::None);
        assert_eq!(safe_round(4_448_001.25), Cell::Float(4_448_001.25));
    }

    #[test]
    fn markers_are_found_and_grouped() {
        let mut data = b"xx\x05@TAB1 some bytes ".to_vec();
        data.extend_from_slice(b"\x06@TAB23 more");
        data.extend_from_slice(b"\x05@TAB1 again");
        let t = tables(&data, &mut Quiet, 0, 0).expect("read");
        let refs: Vec<(&str, usize)> = t
            .iter()
            .map(|t| (t.table_ref.as_str(), t.rows.len()))
            .collect();
        assert_eq!(refs, [("@TAB1", 2), ("@TAB23", 1)]);
    }
}
