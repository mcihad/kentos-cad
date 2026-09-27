//! The column engine (docs/adr/0100): a compiled program run over the
//! objects a batch at a time. A register holds one value per object of the
//! batch, as columns (a kind, a number, a text), so an instruction is one
//! loop over the batch: no call through a trait or allocation per object,
//! numbers stay numbers, and the text an instruction makes goes to its
//! register's one buffer. What the objects hold comes in through `Source`,
//! one column of a batch per call.
//!
//! The instructions themselves are `kernels`.

use crate::kernels::{binary, bit, call, constant, neg, not, round_to, text_constant};
use crate::library::Func;
use crate::program::{Const, Ins, Load, Program};
use crate::scalar::{R, Scratch, V};

/// Objects per batch: enough that an instruction's dispatch is noise, few
/// enough that a program's registers stay in the cache.
pub const BATCH: usize = 256;

/// Kinds of a register's value (the first four are also `rows`' column kinds).
pub const NULL: u8 = 0;
pub const NUM: u8 = 1;
pub const TEXT: u8 = 2;
pub const BOOL: u8 = 3;
/// What JavaScript would have thrown at (text past V8's longest string): the
/// whole expression is then empty, as the TypeScript's `evaluate` caught it.
pub const THROWN: u8 = 4;

/// A register's text: borrowed from the objects or the program, or made by
/// the register's instruction (`start` and `len` in bytes of its buffer).
#[derive(Clone, Copy, Debug)]
pub enum Txt<'a> {
    Ref(&'a str),
    Own { start: usize, len: usize },
}

impl Default for Txt<'_> {
    fn default() -> Self {
        Txt::Own { start: 0, len: 0 }
    }
}

/// Where a load writes the values of a batch's objects.
pub struct Slot<'r, 'a> {
    kinds: &'r mut [u8],
    nums: &'r mut [f64],
    txts: &'r mut [Txt<'a>],
}

impl<'a> Slot<'_, 'a> {
    /// Objects in the batch.
    pub fn len(&self) -> usize {
        self.kinds.len()
    }

    pub fn is_empty(&self) -> bool {
        self.kinds.is_empty()
    }

    /// Object `i`'s value: a number, or empty.
    #[inline]
    pub fn number(&mut self, i: usize, x: Option<f64>) {
        match x {
            Some(x) => {
                self.kinds[i] = NUM;
                self.nums[i] = x;
            }
            None => self.kinds[i] = NULL,
        }
    }

    /// Every object's value from `f`: its number when the flag says it has one.
    #[inline]
    pub fn numbers(&mut self, f: impl Fn(usize) -> (bool, f64)) {
        for (i, (k, x)) in self.kinds.iter_mut().zip(self.nums.iter_mut()).enumerate() {
            let (has, v) = f(i);
            *k = if has { NUM } else { NULL };
            *x = v;
        }
    }

    /// Object `i`'s value: text, or empty.
    #[inline]
    pub fn text(&mut self, i: usize, t: Option<&'a str>) {
        match t {
            Some(t) => {
                self.kinds[i] = TEXT;
                self.txts[i] = Txt::Ref(t);
            }
            None => self.kinds[i] = NULL,
        }
    }
}

/// What a program reads of the objects, a column of a batch at a time.
pub trait Source<'a> {
    /// Writes `load` for objects `start .. start + slot.len()` into `slot`.
    fn fill(&self, load: Load, start: usize, slot: Slot<'_, 'a>);
}

/// A run's registers: register `r`'s value for object `i` at `r * stride + i`,
/// and the text its instruction made in `made[r]`.
pub struct Regs<'r, 'a> {
    pub kinds: &'r mut [u8],
    pub nums: &'r mut [f64],
    pub txts: &'r mut [Txt<'a>],
    pub made: &'r mut [String],
    pub stride: usize,
}

/// A register's values for the objects of the batch.
#[derive(Clone, Copy)]
pub struct Col<'r, 'a> {
    pub k: &'r [u8],
    pub x: &'r [f64],
    pub t: &'r [Txt<'a>],
    /// The text the register's instruction made.
    pub made: &'r str,
}

impl<'r, 'a: 'r> Col<'r, 'a> {
    pub(crate) const NONE: Col<'static, 'static> = Col {
        k: &[],
        x: &[],
        t: &[],
        made: "",
    };

    /// Object `i`'s value (None: it threw).
    #[inline]
    pub fn view(&self, i: usize) -> Option<V<'r>> {
        Some(match self.k[i] {
            NULL => V::Null,
            NUM => V::Num(self.x[i]),
            BOOL => V::Bool(self.x[i] != 0.0),
            TEXT => V::Text(self.text(i)),
            _ => return None,
        })
    }

    /// Object `i`'s text (its kind is TEXT).
    #[inline]
    pub(crate) fn text(&self, i: usize) -> &'r str {
        match self.t[i] {
            Txt::Ref(s) => s,
            Txt::Own { start, len } => self.made.get(start..start + len).unwrap_or_default(),
        }
    }

    /// Object `i`'s text as it was borrowed (from the objects or the
    /// program), when a run did not make it.
    pub fn borrowed(&self, i: usize) -> Option<&'a str> {
        match (self.k[i], self.t[i]) {
            (TEXT, Txt::Ref(s)) => Some(s),
            _ => None,
        }
    }

    /// Object `i`'s truth without reading its text (None: it threw).
    #[inline]
    pub(crate) fn truth(&self, i: usize) -> Option<bool> {
        Some(match self.k[i] {
            NUM | BOOL => self.x[i] != 0.0,
            TEXT => match self.t[i] {
                Txt::Ref(s) => !s.is_empty(),
                Txt::Own { len, .. } => len > 0,
            },
            THROWN => return None,
            _ => false,
        })
    }

    /// Whether object `i` holds a finite number (the fast loops' case).
    #[inline]
    pub(crate) fn finite(&self, i: usize) -> bool {
        (self.k[i] == NUM) & self.x[i].is_finite()
    }
}

/// Where an instruction writes its values.
pub(crate) struct Out<'r, 'a> {
    pub(crate) k: &'r mut [u8],
    pub(crate) x: &'r mut [f64],
    pub(crate) t: &'r mut [Txt<'a>],
    /// The text the instruction makes.
    pub(crate) made: &'r mut String,
}

impl Out<'_, '_> {
    /// A result by the rules: text borrowed from an argument is copied into
    /// the register's buffer, text the operation made at `mark` is taken where it is.
    pub(crate) fn put(&mut self, i: usize, r: R, mark: usize) {
        match r {
            R::V(V::Null) => self.k[i] = NULL,
            R::V(V::Num(x)) => {
                self.k[i] = NUM;
                self.x[i] = x;
            }
            R::V(V::Bool(b)) => {
                self.k[i] = BOOL;
                self.x[i] = bit(b);
            }
            R::V(V::Text(s)) => {
                self.made.truncate(mark);
                self.made.push_str(s);
                self.text_at(i, mark, s.len());
            }
            R::Made => self.text_at(i, mark, self.made.len() - mark),
            R::Thrown => self.k[i] = THROWN,
        }
        if !matches!(r, R::Made | R::V(V::Text(_))) {
            self.made.truncate(mark);
        }
    }

    fn text_at(&mut self, i: usize, start: usize, len: usize) {
        self.k[i] = TEXT;
        self.t[i] = Txt::Own { start, len };
    }

    /// Object `i`'s value by the rules.
    pub(crate) fn rule<'v>(&mut self, i: usize, f: impl FnOnce(&mut String) -> R<'v>) {
        let mark = self.made.len();
        let r = f(self.made);
        self.put(i, r, mark);
    }
}

/// Writes the program's constants into their registers for `rows` objects;
/// they stay for every batch of the run.
pub fn prepare<'a>(p: &'a Program, regs: &mut Regs<'_, 'a>, rows: usize) {
    for (c, value) in p.consts.iter().enumerate() {
        let at = c * regs.stride;
        let (k, x, t) = match value {
            Const::Null => (NULL, 0.0, Txt::default()),
            Const::Num(x) => (NUM, *x, Txt::default()),
            Const::Bool(b) => (BOOL, bit(*b), Txt::default()),
            Const::Text(s) => (TEXT, 0.0, Txt::Ref(s)),
            Const::Thrown => (THROWN, 0.0, Txt::default()),
        };
        regs.kinds[at..at + rows].fill(k);
        regs.nums[at..at + rows].fill(x);
        regs.txts[at..at + rows].fill(t);
    }
}

/// Runs the program's instructions for objects `start .. start + n`
/// (n ≤ the stride; `prepare` first), leaving every register's values in `regs`.
pub fn run<'a>(
    p: &Program,
    src: &dyn Source<'a>,
    start: usize,
    n: usize,
    regs: &mut Regs<'_, 'a>,
    scratch: &mut Scratch,
) {
    let stride = regs.stride;
    let first = p.consts.len();
    for (d, ins) in p.code.iter().enumerate() {
        let at = (first + d) * stride;
        let (in_k, out_k) = regs.kinds.split_at_mut(at);
        let (in_x, out_x) = regs.nums.split_at_mut(at);
        let (in_t, out_t) = regs.txts.split_at_mut(at);
        let (in_made, out_made) = regs.made.split_at_mut(first + d);
        let Some(made) = out_made.first_mut() else {
            return;
        };
        made.clear();
        let mut out = Out {
            k: &mut out_k[..n],
            x: &mut out_x[..n],
            t: &mut out_t[..n],
            made,
        };
        let col = |o| {
            let r = p.register(o);
            let at = r * stride;
            Col {
                k: &in_k[at..at + n],
                x: &in_x[at..at + n],
                t: &in_t[at..at + n],
                made: in_made.get(r).map_or("", String::as_str),
            }
        };
        match ins {
            Ins::Load(l) => src.fill(
                *l,
                start,
                Slot {
                    kinds: out.k,
                    nums: out.x,
                    txts: out.t,
                },
            ),
            Ins::Not(a) => not(col(*a), &mut out),
            Ins::Neg(a) => neg(col(*a), &mut out),
            Ins::Bin(op, [a, b]) => {
                // Which side, if either, is a text constant `=` can compare directly.
                let text = match (
                    text_constant(constant(p, *a)),
                    text_constant(constant(p, *b)),
                ) {
                    (_, Some(c)) => Some((c, false)),
                    (Some(c), _) => Some((c, true)),
                    _ => None,
                };
                binary(*op, col(*a), col(*b), text, &mut out, scratch);
            }
            Ins::Call(Func::Round, args)
                if args.len() == 2
                    && let Some(Const::Num(d)) = constant(p, args[1])
                    && d.is_finite() =>
            {
                round_to(col(args[0]), *d, &mut out, scratch);
            }
            Ins::Call(f, args) => {
                // Most calls take a few arguments: their columns stay on the stack.
                const FEW: usize = 8;
                let mut few = [Col::NONE; FEW];
                let many: Vec<Col>;
                let cols: &[Col] = if args.len() <= FEW {
                    for (c, &o) in few.iter_mut().zip(args.iter()) {
                        *c = col(o);
                    }
                    &few[..args.len()]
                } else {
                    many = args.iter().map(|&o| col(o)).collect();
                    &many
                };
                call(*f, cols, &mut out, scratch);
            }
        }
    }
}

/// The program's values for the objects of the batch after a run.
pub fn result<'r, 'a>(p: &Program, regs: &'r Regs<'_, 'a>, n: usize) -> Col<'r, 'a> {
    let r = p.result.map_or(0, |o| p.register(o));
    let at = r * regs.stride;
    Col {
        k: &regs.kinds[at..at + n],
        x: &regs.nums[at..at + n],
        t: &regs.txts[at..at + n],
        made: regs.made.get(r).map_or("", String::as_str),
    }
}
