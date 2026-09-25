//! One expression over many objects in one call (the browser's boundary):
//! the objects cross as a table of what the expression reads, the values
//! come back as columns. Drawing a layer and a processing run evaluate an
//! expression for thousands of objects; one call per object would cost more
//! than the evaluation.
//!
//! The table's layout follows the expression's `Needs` (the TypeScript side,
//! `apps/web/src/model/expression/expression.ts`, builds the same):
//! - text slots per object: the fields in the expression's order, then the
//!   label, the layer name and the kind label, each only when read; `texts`
//!   holds them one after another and `text_lens` their lengths in UTF-16
//!   code units (−1: none);
//! - number slots per object: the id, then the vertex count (NaN: none),
//!   each only when read;
//! - `measures`: six numbers per object when a geometry value is read
//!   (flags: 1 length, 2 area, 4 anchor; length, area, anchor x, anchor y,
//!   spare), the geometry store's `measures` answer as it is;
//! - the position in the run is the object's index + 1; `scale` is NaN
//!   outside symbol drawing.

use super::value::{into_text, to_number, to_text, truthy};
use super::{Expr, Measured, Needs, Scope, Value};

/// What the caller wants of each value: as it is, a number (empty when it is
/// not one), text, true/false (the last two keep "empty" apart), or the
/// number its text reads as (the style window's classes: 12 significant digits).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum As {
    Value,
    Number,
    Text,
    Bool,
    TextNumber,
}

impl As {
    pub fn from_code(code: u8) -> As {
        match code {
            1 => As::Number,
            2 => As::Text,
            3 => As::Bool,
            4 => As::TextNumber,
            _ => As::Value,
        }
    }

    fn convert(self, v: Value<'_>) -> Value<'_> {
        match (self, v) {
            (As::Value, v) | (_, v @ Value::Null) => v,
            (As::Number, v) => to_number(&v).map_or(Value::Null, Value::Num),
            (As::Text, v) => Value::Text(into_text(v)),
            (As::Bool, v) => Value::Bool(truthy(&v)),
            (As::TextNumber, v) => {
                to_number(&Value::Text(to_text(&v))).map_or(Value::Null, Value::Num)
            }
        }
    }
}

pub struct RowsInput<'a> {
    pub n: usize,
    pub texts: &'a str,
    pub text_lens: &'a [i32],
    pub numbers: &'a [f64],
    pub measures: &'a [f64],
    pub scale: f64,
}

/// The values of one expression for every object: a kind per object (0
/// empty, 1 number, 2 text, 3 true/false), the number (or 0/1) in
/// `numbers`, and the texts one after another with their UTF-16 lengths.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Column {
    pub kinds: Vec<u8>,
    pub numbers: Vec<f64>,
    pub texts: String,
    pub text_lens: Vec<u32>,
}

pub const EMPTY: u8 = 0;
pub const NUMBER: u8 = 1;
pub const TEXT: u8 = 2;
pub const BOOL: u8 = 3;

/// Numbers per object in `measures`.
pub const MEASURE_STRIDE: usize = 6;

/// Where each read value sits in a row.
pub struct Layout {
    text_slots: usize,
    label: Option<usize>,
    layer: Option<usize>,
    kind: Option<usize>,
    number_slots: usize,
    id: Option<usize>,
    vertices: Option<usize>,
    measured: bool,
}

impl Layout {
    /// The layout of a table read by expressions with `fields` field names
    /// in all (their union) and these variables.
    pub fn new(fields: usize, needs: Needs) -> Layout {
        let mut t = fields;
        let slot = |on: bool, next: &mut usize| {
            on.then(|| {
                *next += 1;
                *next - 1
            })
        };
        let label = slot(needs.label, &mut t);
        let layer = slot(needs.layer, &mut t);
        let kind = slot(needs.kind, &mut t);
        let mut k = 0;
        let id = slot(needs.id, &mut k);
        let vertices = slot(needs.vertices, &mut k);
        Layout {
            text_slots: t,
            label,
            layer,
            kind,
            number_slots: k,
            id,
            vertices,
            measured: needs.measured,
        }
    }

    fn of(e: &Expr) -> Layout {
        Layout::new(e.fields.len(), e.needs)
    }
}

/// The objects' values as expressions read them, split into slots once.
pub struct Table<'a> {
    layout: Layout,
    texts: Vec<Option<&'a str>>,
    input: RowsInput<'a>,
}

impl<'a> Table<'a> {
    /// Refuses a table whose sizes do not match the layout.
    pub fn new(input: RowsInput<'a>, layout: Layout) -> Result<Table<'a>, String> {
        let n = input.n;
        if input.text_lens.len() != n * layout.text_slots
            || input.numbers.len() != n * layout.number_slots
            || (layout.measured && input.measures.len() != n * MEASURE_STRIDE)
        {
            return Err(format!(
                "İfade tablosu ifadeye uymuyor ({n} nesne, {} yazı, {} sayı, {} ölçü).",
                input.text_lens.len(),
                input.numbers.len(),
                input.measures.len()
            ));
        }
        let texts = split(input.texts, input.text_lens)?;
        Ok(Table {
            layout,
            texts,
            input,
        })
    }

    /// Object `i` as an expression sees it; `slots` maps the expression's
    /// fields to the table's (None: the same).
    pub fn row<'b>(&'b self, i: usize, slots: Option<&'b [usize]>) -> Row<'a, 'b> {
        Row {
            i,
            slots,
            table: self,
        }
    }
}

pub struct Row<'a, 'b> {
    i: usize,
    slots: Option<&'b [usize]>,
    table: &'b Table<'a>,
}

impl Row<'_, '_> {
    fn text(&self, slot: Option<usize>) -> Option<&str> {
        let t = self.table;
        slot.and_then(|s| t.texts[self.i * t.layout.text_slots + s])
    }

    fn number(&self, slot: Option<usize>) -> Option<f64> {
        let t = self.table;
        let x = t.input.numbers[self.i * t.layout.number_slots + slot?];
        (!x.is_nan()).then_some(x)
    }
}

impl Scope for Row<'_, '_> {
    fn field(&self, i: usize) -> Option<&str> {
        let slot = self.slots.map_or(Some(i), |s| s.get(i).copied());
        self.text(slot)
    }

    fn measured(&self) -> Measured {
        let k = self.i * MEASURE_STRIDE;
        let Some(m) = self.table.input.measures.get(k..k + MEASURE_STRIDE) else {
            return Measured::default();
        };
        let flags = m[0] as u32;
        Measured {
            length: (flags & 1 != 0).then_some(m[1]),
            area: (flags & 2 != 0).then_some(m[2]),
            anchor: (flags & 4 != 0).then_some((m[3], m[4])),
        }
    }

    fn vertices(&self) -> Option<f64> {
        self.number(self.table.layout.vertices)
    }

    fn kind(&self) -> &str {
        self.text(self.table.layout.kind).unwrap_or("")
    }

    fn layer(&self) -> &str {
        self.text(self.table.layout.layer).unwrap_or("")
    }

    fn label(&self) -> Option<&str> {
        self.text(self.table.layout.label)
    }

    fn index(&self) -> f64 {
        (self.i + 1) as f64
    }

    fn id(&self) -> f64 {
        self.number(self.table.layout.id).unwrap_or(f64::NAN)
    }

    fn scale(&self) -> Option<f64> {
        let s = self.table.input.scale;
        (!s.is_nan()).then_some(s)
    }
}

/// Splits `texts` into slots by their UTF-16 lengths.
fn split<'a>(texts: &'a str, lens: &[i32]) -> Result<Vec<Option<&'a str>>, String> {
    let mut out = Vec::with_capacity(lens.len());
    let mut chars = texts.char_indices().peekable();
    for &len in lens {
        if len < 0 {
            out.push(None);
            continue;
        }
        let start = chars.peek().map_or(texts.len(), |c| c.0);
        let mut units = 0;
        while units < len as usize {
            let Some((_, c)) = chars.next() else {
                return Err("İfade tablosunun yazıları uzunluklarından kısa.".into());
            };
            units += c.len_utf16();
        }
        let end = chars.peek().map_or(texts.len(), |c| c.0);
        out.push(Some(&texts[start..end]));
    }
    Ok(out)
}

/// Evaluates `e` for every object of the table, each value as `want` asks.
pub fn evaluate_rows(e: &Expr, input: &RowsInput, want: As) -> Result<Column, String> {
    let n = input.n;
    let table = Table::new(
        RowsInput {
            n,
            texts: input.texts,
            text_lens: input.text_lens,
            numbers: input.numbers,
            measures: input.measures,
            scale: input.scale,
        },
        Layout::of(e),
    )?;
    let mut out = Column {
        kinds: Vec::with_capacity(n),
        numbers: Vec::with_capacity(n),
        ..Column::default()
    };
    for i in 0..n {
        let row = table.row(i, None);
        match want.convert(e.evaluate(&row)) {
            Value::Null => {
                out.kinds.push(EMPTY);
                out.numbers.push(f64::NAN);
            }
            Value::Num(x) => {
                out.kinds.push(NUMBER);
                out.numbers.push(x);
            }
            Value::Bool(b) => {
                out.kinds.push(BOOL);
                out.numbers.push(if b { 1.0 } else { 0.0 });
            }
            Value::Text(s) => {
                out.kinds.push(TEXT);
                out.numbers.push(f64::NAN);
                out.text_lens.push(crate::js::text::utf16_len(&s) as u32);
                out.texts.push_str(&s);
            }
        }
    }
    Ok(out)
}
