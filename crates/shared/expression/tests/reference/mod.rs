//! The tree evaluator as it was before the column engine (docs/adr/0100):
//! a frozen copy of 23cc8f7's `lib.rs` evaluator, `library.rs` functions,
//! `value.rs` rules and `rows.rs` conversion, for the differential test and
//! the benchmark. It parses with the crate's own parser (the grammar did not
//! change) and uses the old text functions (`js::text`), which the engine no
//! longer calls; the Turkish order is the crate's (`js::collate`, whose unit
//! test holds it to the old way of comparing). Not a product path.
#![allow(dead_code, clippy::all)]

use std::borrow::Cow;
use std::cmp::Ordering;
use std::mem;

use kentos_expression::js::text::{self, MAX_STRING_UNITS, utf16_len};
use kentos_expression::js::{collate, number};
use kentos_expression::lexer::tokenize;
use kentos_expression::library::{Func, Var};
use kentos_expression::parser::{BinOp, Node, Parser};
use kentos_expression::rows::{As, BOOL, Column, EMPTY, MEASURE_STRIDE, NUMBER, RowsInput, TEXT};
use kentos_expression::{CompileError, Measured, Needs, Scope, Value};
use kentos_geometry_core::jsmath::{js_max, js_max_all, js_min, js_min_all, js_round, js_sign};

/// A parsed expression: its tree, the fields and variables it reads.
pub struct Old {
    pub root: Node,
    pub fields: Vec<String>,
    pub needs: Needs,
}

/// The message of a source the old language did not have (docs/adr/0100 §4):
/// CASE, IN, BETWEEN, LIKE, IS NULL, `^`. The differential test leaves it out.
pub const OUTSIDE: &str = "outside the old language";

/// Whether the tree is all the old language's.
fn old(n: &Node) -> bool {
    match n {
        Node::Case(..) | Node::In(..) | Node::Between(..) | Node::Like(..) | Node::IsNull(..) => {
            false
        }
        Node::Bin(BinOp::Pow, ..) => false,
        Node::Bin(_, a, b) => old(a) && old(b),
        Node::Not(a) | Node::Neg(a) => old(a),
        Node::Call(_, args) => args.iter().all(old),
        Node::Lit(_) | Node::Field(_) | Node::Var(_) => true,
    }
}

pub fn compile(source: &str) -> Result<Old, CompileError> {
    let mut parser = Parser::new(tokenize(source)?);
    let root = parser.parse()?;
    if !old(&root) {
        return Err(CompileError {
            message: OUTSIDE.into(),
            at: 0,
        });
    }
    let mut needs = Needs::default();
    uses(&root, &mut needs);
    Ok(Old {
        root,
        fields: parser.fields,
        needs,
    })
}

fn uses(n: &Node, needs: &mut Needs) {
    match n {
        Node::Var(v) => match v {
            Var::Area | Var::Length | Var::Y | Var::X => needs.measured = true,
            Var::Vertices => needs.vertices = true,
            Var::Kind => needs.kind = true,
            Var::Layer => needs.layer = true,
            Var::Label => needs.label = true,
            Var::Index => needs.index = true,
            Var::Id => needs.id = true,
            Var::Scale => needs.scale = true,
            // Variables after 23cc8f7 (the differential test's sources do not use them).
            _ => {}
        },
        Node::Call(_, args) => args.iter().for_each(|a| uses(a, needs)),
        // CASE, IN, BETWEEN, LIKE and IS NULL came after it (docs/adr/0100 §4).
        Node::Case(..) | Node::In(..) | Node::Between(..) | Node::Like(..) | Node::IsNull(..) => {
            panic!("the old language had no {n:?}")
        }
        Node::Not(a) | Node::Neg(a) => uses(a, needs),
        Node::Bin(_, a, b) => {
            uses(a, needs);
            uses(b, needs);
        }
        Node::Lit(_) | Node::Field(_) => {}
    }
}

// ── Old rows.rs: the table and its rows ──────────────────────────────────

/// Where each read value sits in a row.
pub struct OldLayout {
    text_slots: usize,
    label: Option<usize>,
    layer: Option<usize>,
    kind: Option<usize>,
    number_slots: usize,
    id: Option<usize>,
    vertices: Option<usize>,
    measured: bool,
}

impl OldLayout {
    /// The layout of a table read by expressions with `fields` field names
    /// in all (their union) and these variables.
    pub fn new(fields: usize, needs: Needs) -> OldLayout {
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
        OldLayout {
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
}

/// The objects' values as expressions read them, split into slots once.
pub struct OldTable<'a> {
    layout: OldLayout,
    texts: Vec<Option<&'a str>>,
    input: RowsInput<'a>,
}

impl<'a> OldTable<'a> {
    /// Refuses a table whose sizes do not match the layout.
    pub fn new(input: RowsInput<'a>, layout: OldLayout) -> Result<OldTable<'a>, String> {
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
        Ok(OldTable {
            layout,
            texts,
            input,
        })
    }

    /// Object `i` as an expression sees it; `slots` maps the expression's
    /// fields to the table's (None: the same).
    pub fn row<'b>(&'b self, i: usize, slots: Option<&'b [usize]>) -> OldRow<'a, 'b> {
        OldRow {
            i,
            slots,
            table: self,
        }
    }
}

pub struct OldRow<'a, 'b> {
    i: usize,
    slots: Option<&'b [usize]>,
    table: &'b OldTable<'a>,
}

impl OldRow<'_, '_> {
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

impl Scope for OldRow<'_, '_> {
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

impl Old {
    /// The value for one object (old `Expr::evaluate`).
    // Called as the library's evaluate is, from another crate: not folded into the caller's loop.
    #[inline(never)]
    pub fn evaluate<'a>(&'a self, s: &'a dyn Scope) -> Value<'a> {
        match eval(&self.root, s) {
            Ok(Value::Num(x)) if !x.is_finite() => Value::Null,
            Ok(v) => v,
            Err(Thrown) => Value::Null,
        }
    }
}

/// The old `As::convert`.
pub fn convert(want: As, v: Value<'_>) -> Value<'_> {
    match (want, v) {
        (As::Value, v) | (_, v @ Value::Null) => v,
        (As::Number, v) => to_number(&v).map_or(Value::Null, Value::Num),
        (As::Text, v) => Value::Text(into_text(v)),
        (As::Bool, v) => Value::Bool(truthy(&v)),
        (As::TextNumber, v) => to_number(&Value::Text(to_text(&v))).map_or(Value::Null, Value::Num),
    }
}

/// The old `evaluate_rows`: the old table, the tree walked per object.
pub fn evaluate_rows(e: &Old, input: &RowsInput, want: As) -> Result<Column, String> {
    let n = input.n;
    let table = OldTable::new(
        RowsInput {
            n,
            texts: input.texts,
            text_lens: input.text_lens,
            numbers: input.numbers,
            measures: input.measures,
            scale: input.scale,
        },
        OldLayout::new(e.fields.len(), e.needs),
    )?;
    let mut out = Column::default();
    for i in 0..n {
        let row = table.row(i, None);
        match convert(want, e.evaluate(&row)) {
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
                out.text_lens.push(utf16_len(&s) as u32);
                out.texts.push_str(&s);
            }
        }
    }
    Ok(out)
}

// ── Old value.rs ─────────────────────────────────────────────────────────

/// `^[+-]?(\d+(\.\d*)?|\.\d+)([eE][+-]?\d+)?$` on ASCII digits.
fn numeric(s: &str) -> bool {
    let b = s.as_bytes();
    let mut i = 0;
    let digits = |i: &mut usize| {
        let start = *i;
        while *i < b.len() && b[*i].is_ascii_digit() {
            *i += 1;
        }
        *i - start
    };
    if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
        i += 1;
    }
    let whole = digits(&mut i);
    if whole > 0 {
        if i < b.len() && b[i] == b'.' {
            i += 1;
            digits(&mut i);
        }
    } else {
        if !(i < b.len() && b[i] == b'.') {
            return false;
        }
        i += 1;
        if digits(&mut i) == 0 {
            return false;
        }
    }
    if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
        i += 1;
        if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
            i += 1;
        }
        if digits(&mut i) == 0 {
            return false;
        }
    }
    i == b.len()
}

/// A number, or None when the value is empty or not a number. A number
/// written in text passes as it is (even "1e400", which is infinite).
pub fn to_number(v: &Value) -> Option<f64> {
    match v {
        Value::Num(x) => x.is_finite().then_some(*x),
        Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
        Value::Text(s) => {
            let t = text::trim(s);
            if numeric(t) { t.parse().ok() } else { None }
        }
        Value::Null => None,
    }
}

pub fn is_empty(v: &Value) -> bool {
    match v {
        Value::Null => true,
        Value::Text(s) => s.is_empty(),
        _ => false,
    }
}

pub fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        // NaN is not 0.
        Value::Num(x) => *x != 0.0,
        Value::Text(s) => !s.is_empty(),
    }
}

/// Number to text for attributes: integers as they are, others to 12
/// significant digits, so float noise is dropped (0.1 + 0.2 → "0.3").
pub fn number_text(x: f64) -> String {
    if !x.is_finite() || x.trunc() == x {
        return number::to_string(x);
    }
    number::to_string_precision(x, 12)
}

/// Text for writing into an attribute (borrowed from a text value).
pub fn to_text<'b>(v: &'b Value<'_>) -> Cow<'b, str> {
    match v {
        Value::Null => Cow::Borrowed(""),
        Value::Bool(b) => Cow::Borrowed(if *b { "doğru" } else { "yanlış" }),
        Value::Num(x) => Cow::Owned(number_text(*x)),
        Value::Text(s) => Cow::Borrowed(s),
    }
}

/// The value's text, keeping a text value's own (no copy).
pub fn into_text(v: Value<'_>) -> Cow<'_, str> {
    match v {
        Value::Text(s) => s,
        v => Cow::Owned(to_text(&v).into_owned()),
    }
}

/// Equality: numbers by value (within 1e-9, relative above 1), text exactly,
/// and "empty" equals only "empty".
pub fn equals(a: &Value, b: &Value) -> bool {
    if is_empty(a) || is_empty(b) {
        return is_empty(a) && is_empty(b);
    }
    if matches!(a, Value::Bool(_)) || matches!(b, Value::Bool(_)) {
        return truthy(a) == truthy(b);
    }
    if let (Some(na), Some(nb)) = (to_number(a), to_number(b)) {
        return (na - nb).abs() <= 1e-9 * js_max(js_max(1.0, na.abs()), nb.abs());
    }
    to_text(a) == to_text(b)
}

/// Order of two values: numbers numerically (their difference, NaN for ∞ − ∞),
/// text in Turkish order; None when one is empty.
pub fn compare(a: &Value, b: &Value) -> Option<f64> {
    if is_empty(a) || is_empty(b) {
        return None;
    }
    if let (Some(na), Some(nb)) = (to_number(a), to_number(b)) {
        return Some(na - nb);
    }
    Some(match collate::compare_tr(&to_text(a), &to_text(b)) {
        Ordering::Less => -1.0,
        Ordering::Equal => 0.0,
        Ordering::Greater => 1.0,
    })
}

// ── Old library.rs ───────────────────────────────────────────────────────

/// What JavaScript would throw at (a string past V8's longest): the whole
/// expression is then empty, as the TypeScript's `evaluate` caught it.
#[derive(Debug)]
pub struct Thrown;

const POW10: [f64; 13] = [
    1.0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7, 1e8, 1e9, 1e10, 1e11, 1e12,
];

/// `Math.max(0, Math.min(12, Math.round(d)))` as an index (d from `to_number`: never NaN).
fn digits(d: f64) -> usize {
    js_max(0.0, js_min(12.0, js_round(d))) as usize
}

/// A position or length for `slice` and `padStart`: ∞ and anything past the text clamp.
fn units(x: f64, cap: usize) -> usize {
    if x >= cap as f64 { cap } else { x as usize }
}

/// Numeric function: empty in, empty out; a non-finite result is empty.
fn numeric_fn(args: &[Value], f: impl Fn(&mut dyn Iterator<Item = f64>) -> f64) -> Value<'static> {
    if args.iter().any(|a| to_number(a).is_none()) {
        return Value::Null;
    }
    let r = f(&mut args.iter().filter_map(to_number));
    if r.is_finite() {
        Value::Num(r)
    } else {
        Value::Null
    }
}

fn fold(v: &Value) -> String {
    text::fold_turkish(&to_text(v))
}

fn owned(s: String) -> Value<'static> {
    Value::Text(Cow::Owned(s))
}

/// Calls a function on evaluated arguments (`args.len()` is within its
/// arity). A result that is one of the arguments, or a part of one, is
/// moved out of `args` and keeps borrowing what it borrowed.
pub fn call<'a>(f: Func, args: &mut [Value<'a>]) -> Result<Value<'a>, Thrown> {
    let take =
        |args: &mut [Value<'a>], i: usize| args.get_mut(i).map(mem::take).unwrap_or_default();
    let num = |args: &[Value], i: usize| args.get(i).and_then(to_number);
    let null = Value::Null;
    let is_null = |args: &[Value], i: usize| matches!(args.get(i), Some(Value::Null) | None);
    Ok(match f {
        Func::Round => {
            let d = if args.len() > 1 {
                num(args, 1)
            } else {
                Some(0.0)
            };
            let (Some(v), Some(d)) = (num(args, 0), d) else {
                return Ok(null);
            };
            let f = POW10[digits(d)];
            Value::Num(js_round((v + js_sign(v) * f64::EPSILON * v.abs()) * f) / f)
        }
        Func::Text => {
            if args.len() < 2 {
                return Ok(Value::Text(into_text(take(args, 0))));
            }
            let (Some(v), Some(d)) = (num(args, 0), num(args, 1)) else {
                return Ok(null);
            };
            owned(number::to_fixed(v, digits(d) as u32))
        }
        Func::Number => num(args, 0).map_or(null, Value::Num),
        Func::Int => numeric_fn(args, |n| n.next().unwrap_or(f64::NAN).trunc()),
        Func::Abs => numeric_fn(args, |n| n.next().unwrap_or(f64::NAN).abs()),
        Func::Min => numeric_fn(args, |n| js_min_all(n)),
        Func::Max => numeric_fn(args, |n| js_max_all(n)),
        Func::Upper | Func::Lower if is_null(args, 0) => null,
        Func::Upper => owned(text::upper_tr(&to_text(&args[0]))),
        Func::Lower => owned(text::lower_tr(&to_text(&args[0]))),
        Func::Trim if is_null(args, 0) => null,
        Func::Trim => match into_text(take(args, 0)) {
            Cow::Borrowed(b) => Value::Text(Cow::Borrowed(text::trim(b))),
            Cow::Owned(o) => owned(text::trim(&o).to_string()),
        },
        Func::Length if is_null(args, 0) => null,
        Func::Length => Value::Num(text::utf16_len(&to_text(&args[0])) as f64),
        Func::Substr => {
            let len = if args.len() > 2 {
                num(args, 2)
            } else {
                Some(f64::INFINITY)
            };
            let (Some(from), Some(len)) = (num(args, 1), len) else {
                return Ok(null);
            };
            if args[0] == Value::Null {
                return Ok(null);
            }
            let t = to_text(&args[0]);
            let total = text::utf16_len(&t);
            let start = js_max(0.0, js_round(from) - 1.0);
            let end = if len == f64::INFINITY {
                None
            } else {
                Some(units(start + js_max(0.0, js_round(len)), total))
            };
            owned(text::slice(&t, units(start, total), end))
        }
        Func::Pad => {
            let Some(len) = num(args, 1) else {
                return Ok(null);
            };
            if args[0] == Value::Null {
                return Ok(null);
            }
            let fill = match args.get(2) {
                None => u16::from(b'0'),
                Some(c) => text::first_unit(&to_text(c)).unwrap_or(u16::from(b'0')),
            };
            let t = into_text(take(args, 0));
            let target = js_max(0.0, js_round(len));
            if target <= text::utf16_len(&t) as f64 {
                return Ok(Value::Text(t));
            }
            if target > text::MAX_STRING_UNITS as f64 {
                return Err(Thrown);
            }
            let padded = match t {
                Cow::Owned(o) => text::pad_start_owned(o, target as usize, fill),
                Cow::Borrowed(b) => text::pad_start(b, target as usize, fill),
            };
            owned(padded.ok_or(Thrown)?)
        }
        Func::Replace if is_null(args, 0) => null,
        Func::Replace => owned(
            text::split_join(&to_text(&args[0]), &to_text(&args[1]), &to_text(&args[2]))
                .ok_or(Thrown)?,
        ),
        Func::Contains | Func::Starts | Func::Ends if is_null(args, 0) => Value::Bool(false),
        Func::Contains | Func::Starts | Func::Ends => {
            let (s, t) = (fold(&args[0]), fold(&args[1]));
            Value::Bool(match f {
                Func::Contains => s.contains(&t),
                Func::Starts => s.starts_with(&t),
                _ => s.ends_with(&t),
            })
        }
        Func::If => {
            let pick = if truthy(&args[0]) { 1 } else { 2 };
            take(args, pick)
        }
        Func::Empty => Value::Bool(is_empty(&args[0])),
        Func::Coalesce => args
            .iter_mut()
            .find(|a| !is_empty(a))
            .map(mem::take)
            .unwrap_or_default(),
        // Functions after it (docs/adr/0100 §4).
        f => panic!("the old language had no {f:?}"),
    })
}

// ── Old lib.rs ───────────────────────────────────────────────────────────

fn opt(x: Option<f64>) -> Value<'static> {
    x.map_or(Value::Null, Value::Num)
}

fn borrowed(t: &str) -> Value<'_> {
    Value::Text(Cow::Borrowed(t))
}

fn variable(v: Var, s: &dyn Scope) -> Value<'_> {
    match v {
        Var::Area => opt(s.measured().area),
        Var::Length => opt(s.measured().length),
        Var::Y => opt(s.measured().anchor.map(|a| a.0)),
        Var::X => opt(s.measured().anchor.map(|a| a.1)),
        Var::Vertices => opt(s.vertices()),
        Var::Kind => borrowed(s.kind()),
        Var::Layer => borrowed(s.layer()),
        Var::Label => s.label().map_or(Value::Null, borrowed),
        Var::Index => Value::Num(s.index()),
        Var::Id => Value::Num(s.id()),
        Var::Scale => opt(s.scale()),
        // Variables after 23cc8f7 (the differential test's sources do not use them).
        _ => Value::Null,
    }
}

/// Joined text, where JavaScript throws past its longest string. Text
/// either side made already (a join, a number, a padding) is extended in
/// place rather than copied.
fn joined<'a>(a: Cow<'_, str>, b: Cow<'_, str>) -> Result<Value<'a>, Thrown> {
    if a.len() + b.len() > MAX_STRING_UNITS && utf16_len(&a) + utf16_len(&b) > MAX_STRING_UNITS {
        return Err(Thrown);
    }
    let out = match (a, b) {
        (Cow::Owned(mut a), b) => {
            a.push_str(&b);
            a
        }
        (Cow::Borrowed(a), Cow::Owned(mut b)) => {
            b.insert_str(0, a);
            b
        }
        (Cow::Borrowed(a), Cow::Borrowed(b)) => {
            let mut out = String::with_capacity(a.len() + b.len());
            out.push_str(a);
            out.push_str(b);
            out
        }
    };
    Ok(Value::Text(Cow::Owned(out)))
}

fn binary<'a>(op: BinOp, a: Value<'a>, b: Value<'a>) -> Result<Value<'a>, Thrown> {
    Ok(match op {
        BinOp::Or => Value::Bool(truthy(&a) || truthy(&b)),
        BinOp::And => Value::Bool(truthy(&a) && truthy(&b)),
        BinOp::Eq => Value::Bool(equals(&a, &b)),
        BinOp::Ne => Value::Bool(!equals(&a, &b)),
        BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => match compare(&a, &b) {
            None => Value::Bool(false),
            Some(c) => Value::Bool(match op {
                BinOp::Lt => c < 0.0,
                BinOp::Le => c <= 0.0,
                BinOp::Gt => c > 0.0,
                _ => c >= 0.0,
            }),
        },
        BinOp::Join => joined(into_text(a), into_text(b))?,
        BinOp::Add => {
            if a == Value::Null || b == Value::Null {
                return Ok(Value::Null);
            }
            match (to_number(&a), to_number(&b)) {
                (Some(na), Some(nb)) => Value::Num(na + nb),
                _ => joined(into_text(a), into_text(b))?,
            }
        }
        BinOp::Pow => panic!("the old language had no ^"),
        BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem => {
            let (Some(na), Some(nb)) = (to_number(&a), to_number(&b)) else {
                return Ok(Value::Null);
            };
            if matches!(op, BinOp::Div | BinOp::Rem) && nb == 0.0 {
                return Ok(Value::Null);
            }
            Value::Num(match op {
                BinOp::Sub => na - nb,
                BinOp::Mul => na * nb,
                BinOp::Div => na / nb,
                _ => na % nb,
            })
        }
    })
}

fn eval<'a>(n: &'a Node, s: &'a dyn Scope) -> Result<Value<'a>, Thrown> {
    Ok(match n {
        Node::Lit(Value::Text(t)) => borrowed(t),
        Node::Lit(v) => v.clone(),
        Node::Field(i) => s.field(*i).map_or(Value::Null, borrowed),
        Node::Var(v) => variable(*v, s),
        Node::Call(f, args) if args.len() <= 4 => {
            // Most calls take a few arguments: they stay on the stack.
            let mut values: [Value<'a>; 4] = Default::default();
            for (slot, a) in values.iter_mut().zip(args) {
                *slot = eval(a, s)?;
            }
            call(*f, &mut values[..args.len()])?
        }
        Node::Call(f, args) => {
            let mut values = args
                .iter()
                .map(|a| eval(a, s))
                .collect::<Result<Vec<_>, _>>()?;
            call(*f, &mut values)?
        }
        Node::Not(a) => Value::Bool(!truthy(&eval(a, s)?)),
        Node::Neg(a) => match to_number(&eval(a, s)?) {
            Some(v) => Value::Num(-v),
            None => Value::Null,
        },
        Node::Bin(op, a, b) => {
            let a = eval(a, s)?;
            let b = eval(b, s)?;
            binary(*op, a, b)?
        }
        Node::Case(..) | Node::In(..) | Node::Between(..) | Node::Like(..) | Node::IsNull(..) => {
            panic!("the old language had no {n:?}")
        }
    })
}
