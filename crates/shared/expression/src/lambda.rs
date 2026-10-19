//! Inner expressions (docs/adr/0601): `dizi_her`, `dizi_süz`, `değişkenle` and `dene`. The resolver takes the body's
//! parts that do not read the function's own values out of it (`hoist`): they are evaluated where the call stands, once
//! per object, by the column engine like any other operand. What stays in the body reads only its frame (the item and its
//! position, or the variable, then those parts) and constants, so it is walked per item without the object (`apply`):
//! the tree walker and the column engine give the same values from the same code.

use std::borrow::Cow;

use crate::compound::{self, Item};
use crate::exec::{self, BATCH, Col, Out, Regs, Slot, Source, Txt};
use crate::library::Func;
use crate::parser::{Lambda, Node};
use crate::program::{Load, Operand, Program};
use crate::scalar::{R, Scratch, V, to_number, truthy};
use crate::value::Value;
use crate::walk::{Thrown, walk};
use crate::{Measured, Scope};

/// Takes out of `n` (the body, `depth` frames inside it) every largest part that reads no value of the frame being
/// resolved and is not a constant, into `captured`, leaving a read of the frame's slot `own + its place` in its stead.
/// A taken part's reads of outer frames are counted from where the call stands.
pub(crate) fn hoist(n: Node, depth: u8, own: u8, captured: &mut Vec<Node>) -> Node {
    if !matches!(n, Node::Lit(_)) && free(&n, depth) {
        let slot = own as usize + captured.len();
        captured.push(lift(n, depth + 1));
        return Node::Local(depth, slot.min(u8::MAX as usize) as u8);
    }
    let mut go = |n: Node| hoist(n, depth, own, captured);
    match n {
        Node::Call(f, args) => Node::Call(f, args.into_iter().map(go).collect()),
        Node::World(k, args) => Node::World(k, args.into_iter().map(go).collect()),
        Node::Not(a) => Node::Not(Box::new(go(*a))),
        Node::Neg(a) => Node::Neg(Box::new(go(*a))),
        Node::IsNull(a, negated) => Node::IsNull(Box::new(go(*a)), negated),
        Node::Bin(op, a, b) => {
            let a = go(*a);
            Node::Bin(op, Box::new(a), Box::new(go(*b)))
        }
        Node::Like(a, b, fold, negated) => {
            let a = go(*a);
            Node::Like(Box::new(a), Box::new(go(*b)), fold, negated)
        }
        Node::Between(a, b, c, negated) => {
            let a = go(*a);
            let b = go(*b);
            Node::Between(Box::new(a), Box::new(b), Box::new(go(*c)), negated)
        }
        Node::In(a, items, negated) => {
            let a = go(*a);
            Node::In(Box::new(a), items.into_iter().map(go).collect(), negated)
        }
        Node::Case(whens, otherwise) => {
            let whens = whens
                .into_iter()
                .map(|(c, v)| {
                    let c = go(c);
                    (c, go(v))
                })
                .collect();
            Node::Case(whens, otherwise.map(|e| Box::new(go(*e))))
        }
        // An inner expression's arguments and taken parts stand in this body; its own body reads only its frame.
        Node::Lambda(l) => {
            let Lambda {
                func,
                args,
                captured: inner,
                body,
                otherwise,
                own: inner_own,
            } = *l;
            let args = args.into_iter().map(&mut go).collect();
            let inner = inner.into_iter().map(&mut go).collect();
            let otherwise = otherwise.map(go);
            Node::Lambda(Box::new(Lambda {
                func,
                args,
                captured: inner,
                body,
                otherwise,
                own: inner_own,
            }))
        }
        n => n,
    }
}

/// Whether `n` reads nothing of the frame being resolved (`depth` frames out from `n`; inside an inner body one more).
fn free(n: &Node, depth: u8) -> bool {
    let mut ok = true;
    each(n, depth, &mut |m, d| {
        if let Node::Local(up, _) = m
            && *up == d
        {
            ok = false;
        }
    });
    ok
}

/// `n`'s reads of outer frames counted from `drop` frames further out.
fn lift(n: Node, drop: u8) -> Node {
    map(n, 0, &mut |m, d| match m {
        Node::Local(up, slot) if up > d => Node::Local(up - drop, slot),
        m => m,
    })
}

/// Visits every node of `n` with the number of inner expressions' bodies it stands in (from `depth`).
fn each(n: &Node, depth: u8, f: &mut impl FnMut(&Node, u8)) {
    f(n, depth);
    let mut go = |m: &Node| each(m, depth, f);
    match n {
        Node::Call(_, args) | Node::World(_, args) => args.iter().for_each(go),
        Node::Not(a) | Node::Neg(a) | Node::IsNull(a, _) => go(a),
        Node::Bin(_, a, b) | Node::Like(a, b, ..) => {
            go(a);
            go(b);
        }
        Node::Between(a, b, c, _) => {
            go(a);
            go(b);
            go(c);
        }
        Node::In(a, items, _) => {
            go(a);
            items.iter().for_each(go);
        }
        Node::Case(whens, otherwise) => {
            for (c, v) in whens {
                go(c);
                go(v);
            }
            if let Some(e) = otherwise {
                go(e);
            }
        }
        Node::Lambda(l) => {
            l.args.iter().for_each(&mut go);
            l.captured.iter().for_each(&mut go);
            if let Some(e) = &l.otherwise {
                go(e);
            }
            each(&l.body, depth + 1, f);
        }
        _ => {}
    }
}

/// `n` rebuilt with `f` applied to every node, bottom up, with the number of bodies it stands in.
fn map(n: Node, depth: u8, f: &mut impl FnMut(Node, u8) -> Node) -> Node {
    let n = match n {
        Node::Call(fu, args) => {
            Node::Call(fu, args.into_iter().map(|a| map(a, depth, f)).collect())
        }
        Node::World(k, args) => {
            Node::World(k, args.into_iter().map(|a| map(a, depth, f)).collect())
        }
        Node::Not(a) => Node::Not(Box::new(map(*a, depth, f))),
        Node::Neg(a) => Node::Neg(Box::new(map(*a, depth, f))),
        Node::IsNull(a, negated) => Node::IsNull(Box::new(map(*a, depth, f)), negated),
        Node::Bin(op, a, b) => {
            let a = map(*a, depth, f);
            Node::Bin(op, Box::new(a), Box::new(map(*b, depth, f)))
        }
        Node::Like(a, b, fold, negated) => {
            let a = map(*a, depth, f);
            Node::Like(Box::new(a), Box::new(map(*b, depth, f)), fold, negated)
        }
        Node::Between(a, b, c, negated) => {
            let a = map(*a, depth, f);
            let b = map(*b, depth, f);
            Node::Between(
                Box::new(a),
                Box::new(b),
                Box::new(map(*c, depth, f)),
                negated,
            )
        }
        Node::In(a, items, negated) => {
            let a = map(*a, depth, f);
            Node::In(
                Box::new(a),
                items.into_iter().map(|i| map(i, depth, f)).collect(),
                negated,
            )
        }
        Node::Case(whens, otherwise) => {
            let whens = whens
                .into_iter()
                .map(|(c, v)| {
                    let c = map(c, depth, f);
                    (c, map(v, depth, f))
                })
                .collect();
            Node::Case(whens, otherwise.map(|e| Box::new(map(*e, depth, f))))
        }
        Node::Lambda(l) => {
            let l = *l;
            Node::Lambda(Box::new(Lambda {
                func: l.func,
                args: l.args.into_iter().map(|a| map(a, depth, f)).collect(),
                captured: l.captured.into_iter().map(|a| map(a, depth, f)).collect(),
                body: map(l.body, depth + 1, f),
                otherwise: l.otherwise.map(|e| map(e, depth, f)),
                own: l.own,
            }))
        }
        n => n,
    };
    f(n, depth)
}

/// Whether the body still calls another layer: one whose own arguments read the item (docs/adr/0601 §2).
pub(crate) fn reads_world(n: &Node) -> bool {
    let mut found = false;
    each(n, 0, &mut |m, _| found |= matches!(m, Node::World(..)));
    found
}

/// An inner expression compiled for the column engine: its operands are its arguments, its taken parts, then `dene`'s
/// value when the body gives none.
#[derive(Clone, Debug)]
pub struct Code {
    pub func: Func,
    pub body: Node,
    /// The body compiled: it loads its frame's slot `s` as the field `s`.
    pub program: Program,
    pub args: usize,
    pub captured: usize,
    pub own: u8,
    pub otherwise: bool,
}

impl Code {
    pub(crate) fn of(l: &Lambda) -> Code {
        Code {
            func: l.func,
            program: Program::compile(&l.body),
            body: l.body.clone(),
            args: l.args.len(),
            captured: l.captured.len(),
            own: l.own,
            otherwise: l.otherwise.is_some(),
        }
    }
}

/// The frame a body is walked in: its own values, then the taken parts.
struct Frame<'f> {
    values: Vec<Value<'f>>,
}

impl Scope for Frame<'_> {
    fn field(&self, _: usize) -> Option<&str> {
        None
    }
    fn measured(&self) -> Measured {
        Measured::default()
    }
    fn vertices(&self) -> Option<f64> {
        None
    }
    fn kind(&self) -> &str {
        ""
    }
    fn layer(&self) -> &str {
        ""
    }
    fn label(&self) -> Option<&str> {
        None
    }
    fn index(&self) -> f64 {
        0.0
    }
    fn id(&self) -> f64 {
        0.0
    }
    fn scale(&self) -> Option<f64> {
        None
    }
    fn local(&self, slot: usize) -> Value<'_> {
        match self.values.get(slot) {
            Some(Value::Text(t)) => Value::Text(Cow::Borrowed(t)),
            Some(v) => v.clone(),
            None => Value::Null,
        }
    }
}

fn borrowed(v: V<'_>) -> Value<'_> {
    match v {
        V::Null => Value::Null,
        V::Num(x) => Value::Num(x),
        V::Text(t) => Value::Text(Cow::Borrowed(t)),
        V::Bool(b) => Value::Bool(b),
    }
}

/// A body's value as a result.
fn give(v: Value, out: &mut String) -> R<'static> {
    match v {
        Value::Null => R::V(V::Null),
        Value::Num(x) if x.is_finite() => R::V(V::Num(x)),
        Value::Num(_) => R::V(V::Null),
        Value::Bool(b) => R::V(V::Bool(b)),
        Value::Text(t) => {
            let mark = out.len();
            out.push_str(&t);
            crate::arrays::made(out, mark)
        }
    }
}

/// The value of an inner expression on its operands (None: one that would have thrown), text written at the end of
/// `out`. A body that throws empties the whole, except in `dene`.
pub(crate) fn apply(
    code: &Code,
    ops: &[Option<V>],
    out: &mut String,
    sc: &mut Option<Scratch>,
) -> R<'static> {
    let (args, rest) = ops.split_at(code.args.min(ops.len()));
    let (captured, otherwise) = rest.split_at(code.captured.min(rest.len()));
    if code.func == Func::Try {
        let value = if captured.iter().all(Option::is_some) {
            body(code, &[], captured, sc)
        } else {
            Err(Thrown)
        };
        return match value {
            Ok(v) if !matches!(v, Value::Null) => give(v, out),
            _ => match otherwise.first() {
                Some(Some(v)) => give(borrowed(*v), out),
                Some(None) => R::Thrown,
                None => R::V(V::Null),
            },
        };
    }
    if args.iter().chain(captured).any(Option::is_none) {
        return R::Thrown;
    }
    let arg = |i: usize| args.get(i).copied().flatten().unwrap_or(V::Null);
    match code.func {
        Func::WithVariable => match body(code, &[borrowed(arg(0))], captured, sc) {
            Ok(v) => give(v, out),
            Err(Thrown) => R::Thrown,
        },
        Func::ArrayForeach | Func::ArrayFilter => {
            let items = match arg(0) {
                V::Null => Vec::new(),
                a => match compound::array_items(a) {
                    Some(items) => items,
                    None => return R::V(V::Null),
                },
            };
            let filter = code.func == Func::ArrayFilter;
            // `dizi_süz`'s limit: 0 or none keeps every item that passes.
            let limit = match to_number(arg(1)) {
                Some(n) if filter && n >= 1.0 => n.trunc() as usize,
                _ => usize::MAX,
            };
            let mut kept: Vec<Item> = Vec::with_capacity(if filter { 0 } else { items.len() });
            for (i, item) in items.iter().enumerate() {
                if kept.len() >= limit {
                    break;
                }
                let own = [borrowed(item.view()), Value::Num(i as f64)];
                let v = match body(code, &own, captured, sc) {
                    Ok(v) => v,
                    Err(Thrown) => return R::Thrown,
                };
                if filter {
                    if truthy(v.view()) {
                        kept.push(item.clone());
                    }
                } else {
                    kept.push(Item::of(v.view()));
                }
            }
            crate::arrays::give_array(&kept, out)
        }
        _ => R::V(V::Null),
    }
}

/// The body walked in its frame.
fn body<'v>(
    code: &Code,
    own: &[Value<'v>],
    captured: &[Option<V<'v>>],
    sc: &mut Option<Scratch>,
) -> Result<Value<'static>, Thrown> {
    let mut values = Vec::with_capacity(own.len() + captured.len());
    values.extend(own.iter().cloned());
    values.extend(captured.iter().map(|v| borrowed(v.unwrap_or(V::Null))));
    let frame = Frame { values };
    walk(&code.body, &frame, sc).map(Value::into_owned)
}

/// A body's frame for a chunk of items, slot by slot (None: a value that threw).
struct Frames<'f> {
    slots: Vec<Vec<Option<V<'f>>>>,
}

impl<'f> Source<'f> for Frames<'f> {
    fn fill(&self, load: Load, start: usize, mut slot: Slot<'_, 'f>) {
        let values = match load {
            Load::Field(s) => self.slots.get(s as usize),
            _ => None,
        };
        for i in 0..slot.len() {
            let v = values.map_or(Some(V::Null), |vs| vs.get(start + i).copied().flatten());
            slot.value(i, v);
        }
    }
}

/// A body's registers for one batch of items, kept for the chunks of a call.
struct Store<'f> {
    kinds: Vec<u8>,
    nums: Vec<f64>,
    txts: Vec<Txt<'f>>,
    made: Vec<String>,
}

impl<'f> Store<'f> {
    fn new(p: &Program) -> Store<'f> {
        let size = p.registers() * BATCH;
        let mut store = Store {
            kinds: vec![0; size],
            nums: vec![0.0; size],
            txts: vec![Txt::default(); size],
            made: vec![String::new(); p.registers()],
        };
        exec::prepare(p, &mut store.regs(), BATCH);
        store
    }

    fn regs(&mut self) -> Regs<'_, 'f> {
        Regs {
            kinds: &mut self.kinds,
            nums: &mut self.nums,
            txts: &mut self.txts,
            made: &mut self.made,
            stride: BATCH,
        }
    }

    /// The body run on `count` frames: its values.
    fn run<'s>(
        &'s mut self,
        p: &Program,
        frames: &Frames<'f>,
        count: usize,
        scratch: &mut Scratch,
    ) -> Col<'s, 'f> {
        exec::run(p, frames, 0, count, &mut self.regs(), scratch);
        let r = p.result.map_or(0, |o| p.register(o));
        let at = r * BATCH;
        Col {
            k: &self.kinds[at..at + count],
            x: &self.nums[at..at + count],
            t: &self.txts[at..at + count],
            made: self.made.get(r).map_or("", String::as_str),
        }
    }
}

/// An inner expression for the objects of a batch (`n`), into `out`: the body is its program run over every item of
/// every object (or once per object for `dene`), a batch of items at a time (docs/adr/0601 §5).
pub(crate) fn column(
    code: &Code,
    ops: &[Operand],
    cols: &[Col],
    n: usize,
    out: &mut Out,
    scratch: &mut Scratch,
) {
    let (args, rest) = cols.split_at(code.args.min(cols.len()));
    let (captured, otherwise) = rest.split_at(code.captured.min(rest.len()));
    let p = &code.program;
    match code.func {
        // Once per object: `dene`'s frame is its taken parts, `değişkenle`'s its value then them.
        Func::Try | Func::WithVariable => {
            let try_ = code.func == Func::Try;
            let frame: Vec<&Col> = if try_ {
                captured.iter().collect()
            } else {
                args.iter().chain(captured).collect()
            };
            let mut store = Store::new(p);
            let mut at = 0;
            while at < n {
                let count = (n - at).min(BATCH);
                let frames = Frames {
                    slots: frame
                        .iter()
                        .map(|c| (at..at + count).map(|i| c.view(i)).collect())
                        .collect(),
                };
                let r = store.run(p, &frames, count, scratch);
                for j in 0..count {
                    let i = at + j;
                    let mark = out.made.len();
                    if !try_ {
                        // A value or a taken part that threw empties the whole, as everywhere.
                        let thrown = frame.iter().any(|c| c.view(i).is_none());
                        match r.view(j) {
                            Some(v) if !thrown => out.put(i, R::V(v), mark),
                            _ => out.put(i, R::Thrown, mark),
                        }
                        continue;
                    }
                    match r.view(j) {
                        Some(v) if !matches!(v, V::Null) => out.put(i, R::V(v), mark),
                        _ => match otherwise.first().map(|c| c.view(i)) {
                            Some(Some(v)) => out.put(i, R::V(v), mark),
                            Some(None) => out.put(i, R::Thrown, mark),
                            None => out.put(i, R::V(V::Null), mark),
                        },
                    }
                }
                at += count;
            }
        }
        Func::ArrayForeach | Func::ArrayFilter => items(code, ops, args, captured, n, out, scratch),
        _ => out.k.fill(crate::exec::NULL),
    }
}

/// `değişkenle` written into its body (docs/adr/0601 §5): the value stands where the body reads the variable, so the
/// program computes it once like any repeated part; the body's reads of outer frames come one frame nearer. None when the
/// value is too large to copy so often.
pub(crate) fn inline(body: &Node, value: &Node) -> Option<Node> {
    let mut reads = 0usize;
    each(body, 0, &mut |m, d| {
        reads += usize::from(matches!(m, Node::Local(0, 0)) && d == 0)
    });
    let mut size = 0usize;
    each(value, 0, &mut |_, _| size += 1);
    if reads.saturating_mul(size) > 256 {
        return None;
    }
    Some(map(body.clone(), 0, &mut |m, d| match m {
        Node::Local(up, 0) if up == d && d == 0 => value.clone(),
        Node::Local(up, s) if up > d => Node::Local(up - 1, s),
        m => m,
    }))
}

/// What a row of `dizi_her` or `dizi_süz` gives, in order: an item of `lists[k]` (its row, the list, its position, its
/// frame in the chunk), or the whole row at once.
#[derive(Clone, Copy)]
enum Entry {
    Item {
        row: usize,
        list: usize,
        at: usize,
        last: bool,
        frame: usize,
    },
    Null(usize),
    Thrown(usize),
    Empty(usize),
}

/// `dizi_her` and `dizi_süz` for a batch of objects: every item of every object in chunks of `BATCH`, the rows written
/// in order (an object's array is one text).
fn items(
    code: &Code,
    ops: &[Operand],
    args: &[Col],
    captured: &[Col],
    n: usize,
    out: &mut Out,
    scratch: &mut Scratch,
) {
    let filter = code.func == Func::ArrayFilter;
    let own = code.own as usize;
    let p = &code.program;
    // The arrays; a constant one is read once.
    let constant = matches!(ops.first(), Some(Operand::Const(_)));
    let mut lists: Vec<Vec<Item>> = Vec::new();
    let mut rows: Vec<Entry> = Vec::with_capacity(n);
    let mut limits: Vec<usize> = Vec::with_capacity(n);
    for i in 0..n {
        let limit = match args.get(1).map(|c| c.view(i)) {
            None => Some(usize::MAX),
            Some(None) => None,
            Some(Some(v)) => Some(match to_number(v) {
                Some(x) if filter && x >= 1.0 => x.trunc() as usize,
                _ => usize::MAX,
            }),
        };
        limits.push(limit.unwrap_or(0));
        let array = args.first().map_or(Some(V::Null), |c| c.view(i));
        if limit.is_none() || array.is_none() || captured.iter().any(|c| c.view(i).is_none()) {
            rows.push(Entry::Thrown(i));
            continue;
        }
        let list = if constant && !lists.is_empty() {
            Some(0)
        } else {
            match array {
                Some(V::Null) => Some(Vec::new()),
                Some(v) => compound::array_items(v),
                None => None,
            }
            .map(|items| {
                lists.push(items);
                lists.len() - 1
            })
        };
        rows.push(match list {
            None => Entry::Null(i),
            Some(k) if lists[k].is_empty() => Entry::Empty(i),
            Some(k) => Entry::Item {
                row: i,
                list: k,
                at: 0,
                last: false,
                frame: 0,
            },
        });
    }
    let mut store = Store::new(p);
    let mut entries: Vec<Entry> = Vec::with_capacity(BATCH);
    let mut slots: Vec<Vec<Option<V>>> = vec![Vec::with_capacity(BATCH); own + captured.len()];
    // The row being written: where its text starts, how many items it kept, whether one threw.
    let (mut mark, mut kept, mut failed) = (0usize, 0usize, false);
    let mut row = 0usize;
    let mut at = 0usize;
    loop {
        // A chunk of frames, and the rows without items between them.
        entries.clear();
        slots.iter_mut().for_each(Vec::clear);
        let mut frames = 0usize;
        while row < n && frames < BATCH {
            match rows[row] {
                Entry::Item { list, .. } => {
                    let len = lists[list].len();
                    entries.push(Entry::Item {
                        row,
                        list,
                        at,
                        last: at + 1 == len,
                        frame: frames,
                    });
                    slots[0].push(Some(lists[list][at].view()));
                    if own > 1 {
                        slots[1].push(Some(V::Num(at as f64)));
                    }
                    for (c, col) in captured.iter().enumerate() {
                        slots[own + c].push(col.view(row));
                    }
                    frames += 1;
                    at += 1;
                    if at == len {
                        row += 1;
                        at = 0;
                    }
                }
                whole => {
                    entries.push(whole);
                    row += 1;
                }
            }
        }
        if entries.is_empty() {
            break;
        }
        let frame_set = Frames {
            slots: std::mem::take(&mut slots),
        };
        let r = store.run(p, &frame_set, frames, scratch);
        for &e in &entries {
            match e {
                Entry::Null(i) => out.put(i, R::V(V::Null), out.made.len()),
                Entry::Thrown(i) => out.put(i, R::Thrown, out.made.len()),
                Entry::Empty(i) => {
                    let m = out.made.len();
                    out.made.push(compound::ARRAY);
                    out.made.push_str("[]");
                    out.put(i, R::Made, m);
                }
                Entry::Item {
                    row: i,
                    list,
                    at,
                    last,
                    frame,
                } => {
                    if at == 0 {
                        mark = out.made.len();
                        out.made.push(compound::ARRAY);
                        out.made.push('[');
                        kept = 0;
                        failed = false;
                    }
                    // `dizi_süz` reads no item past its limit.
                    if !(filter && kept >= limits[i]) {
                        match r.view(frame) {
                            None => failed = true,
                            Some(v) if filter => {
                                if truthy(v) {
                                    if kept > 0 {
                                        out.made.push(',');
                                    }
                                    compound::push_item(out.made, &lists[list][at]);
                                    kept += 1;
                                }
                            }
                            Some(v) => {
                                if at > 0 {
                                    out.made.push(',');
                                }
                                compound::push_value_json(out.made, v);
                            }
                        }
                    }
                    if last {
                        if failed {
                            out.put(i, R::Thrown, mark);
                        } else {
                            out.made.push(']');
                            let r = crate::arrays::made(out.made, mark);
                            out.put(i, r, mark);
                        }
                    }
                }
            }
        }
        slots = frame_set.slots;
    }
}
