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
//!
//! The column engine reads the table through `TableSource` (docs/adr/0100);
//! `Table::row` gives one object as a `Scope`, for the style engine's
//! per-object path.

use kentos_geometry_core::entity::Shape;

use super::exec::{self, BATCH, Col, Regs, Slot, Source, Txt};
use super::geometry::Shapes;
use super::host::Geometry;
use super::program::Load;
use super::scalar::{self, Scratch, V};
use super::value::{into_text, to_number, to_text, truthy};
use super::{Expr, Measured, Needs, Scope, Value};
use crate::js::text::utf16_len;

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

    /// One value as the caller asked for it: the rule `evaluate_rows` applies
    /// to every object, and the desktop's processing tools to theirs
    /// (`kentos-processing`, which evaluates objects one by one).
    pub fn convert(self, v: Value<'_>) -> Value<'_> {
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

impl Column {
    /// The values one by one (texts copied out): for a host that holds them
    /// as values, not columns.
    pub fn values(&self) -> Vec<Value<'static>> {
        let mut texts = self.text_lens.iter();
        let mut rest = self.texts.as_str();
        self.kinds
            .iter()
            .zip(&self.numbers)
            .map(|(&k, &x)| match k {
                NUMBER => Value::Num(x),
                BOOL => Value::Bool(x != 0.0),
                TEXT => {
                    let units = texts.next().copied().unwrap_or(0) as usize;
                    // The length is in UTF-16 units: walk the characters until it is reached.
                    let (mut seen, mut bytes) = (0, 0);
                    for c in rest.chars() {
                        if seen >= units {
                            break;
                        }
                        seen += c.len_utf16();
                        bytes += c.len_utf8();
                    }
                    let (t, after) = rest.split_at(bytes.min(rest.len()));
                    rest = after;
                    Value::text(t.to_owned())
                }
                _ => Value::Null,
            })
            .collect()
    }
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
    /// Each text slot's place in `input.texts` (bytes; start `NONE`: no value).
    spans: Vec<(u32, u32)>,
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
        let spans = split(input.texts, input.text_lens)?;
        Ok(Table {
            layout,
            spans,
            input,
        })
    }

    /// Text slot `k` (object × text slots + slot): its text, or None.
    #[inline]
    fn text(&self, k: usize) -> Option<&'a str> {
        let (start, len) = self.spans[k];
        if start == NONE {
            return None;
        }
        let start = start as usize;
        self.input.texts.get(start..start + len as usize)
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
        slot.and_then(|s| t.text(self.i * t.layout.text_slots + s))
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

/// A text slot with no value.
const NONE: u32 = u32::MAX;

/// Splits `texts` into slots by their UTF-16 lengths: each slot's bytes.
fn split(texts: &str, lens: &[i32]) -> Result<Vec<(u32, u32)>, String> {
    let short = || String::from("İfade tablosunun yazıları uzunluklarından kısa.");
    if texts.len() >= NONE as usize {
        return Err("İfade tablosunun yazıları 4 GB'tan uzun.".into());
    }
    let bytes = texts.as_bytes();
    let mut out = Vec::with_capacity(lens.len());
    let mut at = 0;
    for &len in lens {
        if len < 0 {
            out.push((NONE, 0));
            continue;
        }
        let len = len as usize;
        // ASCII text is as long in bytes as in UTF-16 units (most attributes).
        let end = match bytes.get(at..at + len) {
            Some(b) if b.is_ascii() => at + len,
            _ => {
                let mut units = 0;
                let mut end = at;
                for c in texts.get(at..).ok_or_else(short)?.chars() {
                    if units >= len {
                        break;
                    }
                    units += c.len_utf16();
                    end += c.len_utf8();
                }
                if units < len {
                    return Err(short());
                }
                end
            }
        };
        if !texts.is_char_boundary(end) {
            return Err(short());
        }
        out.push((at as u32, (end - at) as u32));
        at = end;
    }
    Ok(out)
}

/// The table as the column engine reads it: a column of a batch per call.
/// `slots` maps the expression's fields to the table's (None: the same).
pub struct TableSource<'t, 'a> {
    pub table: &'t Table<'a>,
    pub slots: Option<&'t [usize]>,
}

impl<'a> Source<'a> for TableSource<'_, 'a> {
    fn fill(&self, load: Load, start: usize, mut slot: Slot<'_, 'a>) {
        let t = self.table;
        let l = &t.layout;
        let n = slot.len();
        let text = |at: Option<usize>, slot: &mut Slot<'_, 'a>| match at {
            Some(s) => {
                for i in 0..n {
                    slot.text(i, t.text((start + i) * l.text_slots + s));
                }
            }
            None => (0..n).for_each(|i| slot.text(i, None)),
        };
        let number = |at: Option<usize>, slot: &mut Slot<'_, 'a>| match at {
            Some(s) => {
                for i in 0..n {
                    let x = t.input.numbers[(start + i) * l.number_slots + s];
                    slot.number(i, (!x.is_nan()).then_some(x));
                }
            }
            None => (0..n).for_each(|i| slot.number(i, None)),
        };
        // Flags of a measures record: 1 length, 2 area, 4 anchor.
        let measure = |flag: u32, k: usize, slot: &mut Slot<'_, 'a>| match t
            .input
            .measures
            .get(start * MEASURE_STRIDE..(start + n) * MEASURE_STRIDE)
        {
            Some(m) => slot.numbers(|i| {
                let r = &m[i * MEASURE_STRIDE..(i + 1) * MEASURE_STRIDE];
                (r[0] as u32 & flag != 0, r[k])
            }),
            None => (0..n).for_each(|i| slot.number(i, None)),
        };
        match load {
            Load::Field(f) => {
                let f = f as usize;
                // A slot past the table's fields (a field the table does not hold) reads none.
                let at = self
                    .slots
                    .map_or(Some(f), |s| s.get(f).copied())
                    .filter(|&s| s < l.text_slots);
                text(at, &mut slot);
            }
            Load::Label => text(l.label, &mut slot),
            Load::Layer | Load::Kind => {
                // Always text: "" when the table does not hold it.
                let at = if load == Load::Layer { l.layer } else { l.kind };
                for i in 0..n {
                    let v = at.and_then(|s| t.text((start + i) * l.text_slots + s));
                    slot.text(i, Some(v.unwrap_or("")));
                }
            }
            Load::Id => {
                for i in 0..n {
                    let x = l.id.map_or(f64::NAN, |s| {
                        t.input.numbers[(start + i) * l.number_slots + s]
                    });
                    slot.number(i, Some(x));
                }
            }
            Load::Vertices => number(l.vertices, &mut slot),
            Load::Length => measure(1, 1, &mut slot),
            Load::Area => measure(2, 2, &mut slot),
            Load::Y => measure(4, 3, &mut slot),
            Load::X => measure(4, 4, &mut slot),
            Load::Index => slot.numbers(|i| (true, (start + i + 1) as f64)),
            // The table carries length, area and anchor (the store's `measures`);
            // the other geometry values come from a host that holds the shapes
            // (`host::Objects`, `geometry::Shapes`).
            Load::CentroidY
            | Load::CentroidX
            | Load::MinY
            | Load::MaxY
            | Load::MinX
            | Load::MaxX
            | Load::Width
            | Load::Height => (0..n).for_each(|i| slot.number(i, None)),
            Load::Scale => {
                let s = t.input.scale;
                (0..n).for_each(|i| slot.number(i, (!s.is_nan()).then_some(s)));
            }
        }
    }
}

/// Evaluates `e` for every object of the table, each value as `want` asks.
pub fn evaluate_rows(e: &Expr, input: &RowsInput, want: As) -> Result<Column, String> {
    let table = Table::new(copy(input), Layout::of(e))?;
    let source = TableSource {
        table: &table,
        slots: None,
    };
    Ok(column(e, &source, input.n, want))
}

/// `evaluate_rows` for objects whose shapes the caller holds (the web's
/// geometry store, docs/adr/0100 §3): the geometry values are read from the
/// shapes (`shape(i)`: object `i`'s), only those the expression reads, and
/// the table carries no `measures`.
pub fn evaluate_rows_on<'s>(
    e: &Expr,
    input: &RowsInput,
    want: As,
    shape: impl Fn(usize) -> Option<&'s Shape>,
) -> Result<Column, String> {
    let layout = Layout::new(
        e.fields.len(),
        Needs {
            measured: false,
            ..e.needs
        },
    );
    let table = Table::new(copy(input), layout)?;
    let source = TableShapes {
        table: TableSource {
            table: &table,
            slots: None,
        },
        shapes: Shapes::new(shape),
    };
    Ok(column(e, &source, input.n, want))
}

fn copy<'a>(input: &RowsInput<'a>) -> RowsInput<'a> {
    RowsInput {
        n: input.n,
        texts: input.texts,
        text_lens: input.text_lens,
        numbers: input.numbers,
        measures: input.measures,
        scale: input.scale,
    }
}

/// The table's text and numbers, the geometry values from shapes.
struct TableShapes<'t, 'a, 's, F: Fn(usize) -> Option<&'s Shape>> {
    table: TableSource<'t, 'a>,
    shapes: Shapes<'s, F>,
}

impl<'a, 's, F: Fn(usize) -> Option<&'s Shape>> Source<'a> for TableShapes<'_, 'a, 's, F> {
    fn fill(&self, load: Load, start: usize, slot: Slot<'_, 'a>) {
        let geometry = match load {
            Load::Length => Geometry::Length,
            Load::Area => Geometry::Area,
            Load::Y => Geometry::AnchorY,
            Load::X => Geometry::AnchorX,
            Load::CentroidY => Geometry::CentroidY,
            Load::CentroidX => Geometry::CentroidX,
            Load::MinY => Geometry::MinY,
            Load::MaxY => Geometry::MaxY,
            Load::MinX => Geometry::MinX,
            Load::MaxX => Geometry::MaxX,
            Load::Width => Geometry::Width,
            Load::Height => Geometry::Height,
            _ => return self.table.fill(load, start, slot),
        };
        self.shapes.fill(geometry, start, slot);
    }
}

/// One expression over `n` objects of a source, a batch at a time, into a column.
pub(crate) fn column<'a>(e: &Expr, source: &dyn Source<'a>, n: usize, want: As) -> Column {
    let mut out = Column {
        kinds: Vec::with_capacity(n),
        numbers: Vec::with_capacity(n),
        ..Column::default()
    };
    let p = e.program();
    let r = p.registers();
    let (mut kinds, mut nums, mut txts) = (
        vec![exec::NULL; r * BATCH],
        vec![0.0; r * BATCH],
        vec![Txt::default(); r * BATCH],
    );
    let mut made = vec![String::new(); r];
    let mut regs = Regs {
        kinds: &mut kinds,
        nums: &mut nums,
        txts: &mut txts,
        made: &mut made,
        stride: BATCH,
    };
    exec::prepare(p, &mut regs, BATCH);
    let mut scratch = Scratch::default();
    let mut tmp = String::new();
    for start in (0..n).step_by(BATCH) {
        let len = BATCH.min(n - start);
        exec::run(p, source, start, len, &mut regs, &mut scratch);
        emit(&mut out, exec::result(p, &regs, len), want, &mut tmp);
    }
    out
}

/// A batch's values into the column, as `want` asks: numbers and
/// true/false directly, text and the text modes by `put`.
pub(crate) fn emit(out: &mut Column, col: Col, want: As, tmp: &mut String) {
    let n = col.k.len();
    let texts = matches!(want, As::Text | As::TextNumber) || col.k.contains(&exec::TEXT);
    if !texts {
        // No text in or out: each value's kind and number, in place.
        let from = out.kinds.len();
        out.kinds.resize(from + n, EMPTY);
        out.numbers.resize(from + n, f64::NAN);
        let (kinds, numbers) = (&mut out.kinds[from..], &mut out.numbers[from..]);
        for i in 0..n {
            let x = col.x[i];
            (kinds[i], numbers[i]) = match (want, col.k[i]) {
                (_, exec::NUM) if !x.is_finite() => (EMPTY, f64::NAN),
                (As::Value | As::Number, exec::NUM) | (As::Number, exec::BOOL) => (NUMBER, x),
                (As::Value, exec::BOOL) => (BOOL, x),
                (As::Bool, exec::NUM | exec::BOOL) => (BOOL, if x != 0.0 { 1.0 } else { 0.0 }),
                _ => (EMPTY, f64::NAN),
            };
        }
        return;
    }
    for i in 0..n {
        put(out, col.view(i), want, tmp);
    }
}

/// One value into the column, as `want` asks: empty where it threw, and a
/// number that is not finite is empty (as `Expr::evaluate` gives it).
pub fn put(out: &mut Column, v: Option<V>, want: As, tmp: &mut String) {
    let v = match v {
        None => V::Null,
        Some(V::Num(x)) if !x.is_finite() => V::Null,
        Some(v) => v,
    };
    let v = match (want, v) {
        (As::Value, v) | (_, v @ V::Null) => v,
        (As::Number, v) => scalar::to_number(v).map_or(V::Null, V::Num),
        (As::Bool, v) => V::Bool(scalar::truthy(v)),
        (As::TextNumber, v) => {
            tmp.clear();
            scalar::push_text(tmp, v);
            scalar::to_number(V::Text(tmp)).map_or(V::Null, V::Num)
        }
        (As::Text, v) => {
            out.kinds.push(TEXT);
            out.numbers.push(f64::NAN);
            let from = out.texts.len();
            scalar::push_text(&mut out.texts, v);
            out.text_lens.push(utf16_len(&out.texts[from..]) as u32);
            return;
        }
    };
    match v {
        V::Null => {
            out.kinds.push(EMPTY);
            out.numbers.push(f64::NAN);
        }
        V::Num(x) => {
            out.kinds.push(NUMBER);
            out.numbers.push(x);
        }
        V::Bool(b) => {
            out.kinds.push(BOOL);
            out.numbers.push(if b { 1.0 } else { 0.0 });
        }
        V::Text(s) => {
            // An array or a map shows as its JSON (docs/adr/0214 §2.1).
            let s = crate::compound::shown(s);
            out.kinds.push(TEXT);
            out.numbers.push(f64::NAN);
            out.text_lens.push(utf16_len(s) as u32);
            out.texts.push_str(s);
        }
    }
}

// ── docs/adr/0214 §3: the world at the browser's boundary ─────────────────

/// The layers an expression's calls look at, as the page sends them: every
/// layer's objects one after another in one table (the layout `Layout`
/// gives the union of what the calls read, `Expr::world_needs`), their ids
/// in the geometry store, and each layer's name and count.
pub struct WorldTable<'a> {
    table: Table<'a>,
    fields: Vec<String>,
    ids: &'a [f64],
    layers: Vec<(String, usize, usize)>,
}

impl<'a> WorldTable<'a> {
    /// Refuses a table whose sizes do not match the counts and the layout.
    pub fn new(
        needs: &crate::WorldNeeds,
        names: Vec<String>,
        counts: &[u32],
        ids: &'a [f64],
        texts: &'a str,
        text_lens: &'a [i32],
        numbers: &'a [f64],
    ) -> Result<WorldTable<'a>, String> {
        if names.len() != counts.len() {
            return Err("Katman tablosunun adları sayılarıyla uyuşmuyor.".into());
        }
        let n: usize = counts.iter().map(|&c| c as usize).sum();
        if ids.len() != n {
            return Err(format!(
                "Katman tablosunda {n} nesne sayıldı, {} kimlik geldi.",
                ids.len()
            ));
        }
        let layout = Layout::new(
            needs.fields.len(),
            Needs {
                measured: false,
                ..needs.needs
            },
        );
        let table = Table::new(
            RowsInput {
                n,
                texts,
                text_lens,
                numbers,
                measures: &[],
                scale: f64::NAN,
            },
            layout,
        )?;
        let mut first = 0;
        let layers = names
            .into_iter()
            .zip(counts)
            .map(|(name, &c)| {
                let l = (name, first, c as usize);
                first += c as usize;
                l
            })
            .collect();
        Ok(WorldTable {
            table,
            fields: needs.fields.clone(),
            ids,
            layers,
        })
    }

    /// Its layers as the world reads them, their shapes from `store`.
    pub fn layers<'w>(
        &'w self,
        store: Option<&'w kentos_geometry_core::store::Store>,
    ) -> Vec<(String, Vec<f64>, TableLayer<'w, 'a>)> {
        self.layers
            .iter()
            .map(|(name, first, n)| {
                (
                    name.clone(),
                    self.ids[*first..first + n].to_vec(),
                    TableLayer {
                        world: self,
                        first: *first,
                        n: *n,
                        store,
                    },
                )
            })
            .collect()
    }
}

/// One layer of a `WorldTable`.
pub struct TableLayer<'w, 'a> {
    world: &'w WorldTable<'a>,
    first: usize,
    n: usize,
    store: Option<&'w kentos_geometry_core::store::Store>,
}

impl crate::world::LayerObjects for TableLayer<'_, '_> {
    fn source<'s>(&'s self, e: &'s Expr) -> Box<dyn Source<'s> + 's> {
        // The expression's fields among the table's (an inner expression reads some of them).
        let slots = e
            .fields
            .iter()
            .map(|f| {
                self.world
                    .fields
                    .iter()
                    .position(|t| t == f)
                    .unwrap_or(usize::MAX)
            })
            .collect();
        Box::new(LayerSource { layer: self, slots })
    }
}

struct LayerSource<'l, 'w, 'a> {
    layer: &'l TableLayer<'w, 'a>,
    slots: Vec<usize>,
}

impl<'s, 'l: 's, 'w: 's, 'a: 's> Source<'s> for LayerSource<'l, 'w, 'a> {
    fn fill(&self, load: Load, start: usize, mut slot: Slot<'_, 's>) {
        let l = self.layer;
        let geometry = match load {
            Load::Length => Some(Geometry::Length),
            Load::Area => Some(Geometry::Area),
            Load::Y => Some(Geometry::AnchorY),
            Load::X => Some(Geometry::AnchorX),
            Load::CentroidY => Some(Geometry::CentroidY),
            Load::CentroidX => Some(Geometry::CentroidX),
            Load::MinY => Some(Geometry::MinY),
            Load::MaxY => Some(Geometry::MaxY),
            Load::MinX => Some(Geometry::MinX),
            Load::MaxX => Some(Geometry::MaxX),
            Load::Width => Some(Geometry::Width),
            Load::Height => Some(Geometry::Height),
            _ => None,
        };
        if let Some(what) = geometry {
            let ids = &l.world.ids[l.first..l.first + l.n];
            let store = l.store;
            Shapes::new(|i| store.and_then(|s| s.get(*ids.get(i)?)).map(|it| &it.shape))
                .fill(what, start, slot);
            return;
        }
        if load == Load::Index {
            // The position in the layer, not in the whole table.
            slot.numbers(|i| (true, (start + i + 1) as f64));
            return;
        }
        TableSource {
            table: &l.world.table,
            slots: Some(&self.slots),
        }
        .fill(load, l.first + start, slot);
    }
}

/// `evaluate_rows_on` with the world the expression's calls to other
/// objects are answered from (docs/adr/0214 §3).
pub fn evaluate_rows_on_in<'s>(
    e: &Expr,
    input: &RowsInput,
    want: As,
    shape: impl Fn(usize) -> Option<&'s Shape>,
    world: Option<&crate::world::World<'_>>,
) -> Result<Column, String> {
    let Some(world) = world.filter(|_| e.looks_around()) else {
        return evaluate_rows_on(e, input, want, shape);
    };
    let layout = Layout::new(
        e.fields.len(),
        Needs {
            measured: false,
            ..e.needs
        },
    );
    let table = Table::new(copy(input), layout)?;
    let source = TableShapes {
        table: TableSource {
            table: &table,
            slots: None,
        },
        shapes: Shapes::new(shape),
    };
    let session = crate::world::Session::new(e, world);
    let with = crate::world::WithWorld {
        inner: &source,
        world: &session,
    };
    Ok(column(e, &with, input.n, want))
}
