//! An expression compiled for the column engine (docs/adr/0100): a flat list
//! of instructions, each writing one register for a whole batch of objects,
//! its operands registers written before it or constants. Compiling folds
//! what does not depend on the object (`yuvarla(2.5)`, `'P' || '-'`) into
//! constants and gives equal work one register: a field, a variable or a
//! repeated subexpression is computed once per object however often the
//! expression names it.

use std::collections::HashMap;

use crate::functions;
use crate::library::{Func, Var};
use crate::parser::{BinOp, Node};
use crate::scalar::{self, R, Scratch, V};
use crate::value::Value;

/// What a program reads of an object, one register each.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Load {
    /// Attribute `i` of the expression's field list (text; none: empty).
    Field(u32),
    Area,
    Length,
    /// Y (east) and X (north) of the object's anchor.
    Y,
    X,
    Vertices,
    Kind,
    Layer,
    Label,
    /// 1-based position of the object in the run.
    Index,
    Id,
    /// Denominator of the plot scale while drawing a symbol.
    Scale,
    /// The area's centroid (the anchor for other objects), Y east and X north.
    CentroidY,
    CentroidX,
    /// The bounding box and its extents.
    MinY,
    MaxY,
    MinX,
    MaxX,
    Width,
    Height,
}

impl Load {
    fn of(v: Var) -> Load {
        match v {
            Var::Area => Load::Area,
            Var::Length => Load::Length,
            Var::Vertices => Load::Vertices,
            Var::Kind => Load::Kind,
            Var::Layer => Load::Layer,
            Var::Label => Load::Label,
            Var::Y => Load::Y,
            Var::X => Load::X,
            Var::Index => Load::Index,
            Var::Id => Load::Id,
            Var::Scale => Load::Scale,
            Var::CentroidY => Load::CentroidY,
            Var::CentroidX => Load::CentroidX,
            Var::MinY => Load::MinY,
            Var::MaxY => Load::MaxY,
            Var::MinX => Load::MinX,
            Var::MaxX => Load::MaxX,
            Var::Width => Load::Width,
            Var::Height => Load::Height,
        }
    }
}

/// An instruction's input: a register written by an earlier instruction, or a constant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Operand {
    Reg(u32),
    Const(u32),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Ins {
    Load(Load),
    Not(Operand),
    Neg(Operand),
    /// A binary operator on its left and right operands.
    Bin(BinOp, [Operand; 2]),
    Call(Func, Box<[Operand]>),
}

impl Ins {
    pub fn operands(&self) -> &[Operand] {
        match self {
            Ins::Load(_) => &[],
            Ins::Not(a) | Ins::Neg(a) => std::slice::from_ref(a),
            Ins::Bin(_, ab) => ab,
            Ins::Call(_, args) => args,
        }
    }

    /// Whether every operand is a constant (loads have none and are never constant).
    fn constant(&self) -> bool {
        !matches!(self, Ins::Load(_))
            && self
                .operands()
                .iter()
                .all(|o| matches!(o, Operand::Const(_)))
    }
}

/// A constant of the program. `Thrown` is a constant that would have thrown
/// (`doldur('x', 1e12)`): whatever reads it is empty.
#[derive(Clone, Debug, PartialEq)]
pub enum Const {
    Null,
    Num(f64),
    Text(Box<str>),
    Bool(bool),
    Thrown,
}

impl Const {
    pub fn view(&self) -> Option<V<'_>> {
        Some(match self {
            Const::Null => V::Null,
            Const::Num(x) => V::Num(*x),
            Const::Text(s) => V::Text(s),
            Const::Bool(b) => V::Bool(*b),
            Const::Thrown => return None,
        })
    }

    fn of(v: V) -> Const {
        match v {
            V::Null => Const::Null,
            V::Num(x) => Const::Num(x),
            V::Text(s) => Const::Text(s.into()),
            V::Bool(b) => Const::Bool(b),
        }
    }
}

/// A compiled expression. Its registers are its constants first (filled
/// once per run), then one per instruction: an instruction's operands are
/// always registers before its own.
#[derive(Clone, Debug, Default)]
pub struct Program {
    /// Instruction `i` writes register `consts.len() + i`.
    pub code: Vec<Ins>,
    /// The constants the instructions and the result read (no others).
    pub consts: Vec<Const>,
    /// The expression's value.
    pub result: Option<Operand>,
}

impl Program {
    pub fn compile(root: &Node) -> Program {
        let mut b = Builder::default();
        let result = b.node(root);
        let mut p = Program {
            code: b.code,
            consts: b.consts,
            result: Some(result),
        };
        p.drop_unused_constants();
        p
    }

    pub fn constant(&self, c: u32) -> &Const {
        // Operands only name constants the builder made.
        self.consts.get(c as usize).unwrap_or(&Const::Null)
    }

    /// The registers a run needs per object: the constants' and the instructions'.
    pub fn registers(&self) -> usize {
        self.consts.len() + self.code.len()
    }

    /// The register an operand reads.
    pub fn register(&self, o: Operand) -> usize {
        match o {
            Operand::Const(c) => c as usize,
            Operand::Reg(r) => self.consts.len() + r as usize,
        }
    }

    /// Folding leaves constants no instruction reads (the parts of a folded
    /// operation): they go, and the others are numbered again.
    fn drop_unused_constants(&mut self) {
        let mut used = vec![false; self.consts.len()];
        let mut mark = |o: &Operand| {
            if let Operand::Const(c) = o {
                used[*c as usize] = true;
            }
        };
        self.code.iter().flat_map(Ins::operands).for_each(&mut mark);
        self.result.iter().for_each(&mut mark);
        let mut index = vec![0u32; self.consts.len()];
        let mut kept = Vec::new();
        for (i, c) in std::mem::take(&mut self.consts).into_iter().enumerate() {
            if used[i] {
                index[i] = kept.len() as u32;
                kept.push(c);
            }
        }
        self.consts = kept;
        let renumber = |o: &mut Operand| {
            if let Operand::Const(c) = o {
                *c = index[*c as usize];
            }
        };
        for ins in &mut self.code {
            match ins {
                Ins::Load(_) => {}
                Ins::Not(a) | Ins::Neg(a) => renumber(a),
                Ins::Bin(_, ab) => ab.iter_mut().for_each(renumber),
                Ins::Call(_, args) => args.iter_mut().for_each(renumber),
            }
        }
        self.result.iter_mut().for_each(renumber);
    }
}

#[derive(Default)]
struct Builder {
    code: Vec<Ins>,
    consts: Vec<Const>,
    known: HashMap<Ins, u32>,
    scratch: Scratch,
}

impl Builder {
    fn constant(&mut self, c: Const) -> Operand {
        self.consts.push(c);
        Operand::Const(self.consts.len() as u32 - 1)
    }

    fn node(&mut self, n: &Node) -> Operand {
        match n {
            Node::Lit(v) => self.constant(match v {
                Value::Null => Const::Null,
                Value::Num(x) => Const::Num(*x),
                Value::Text(s) => Const::Text(s.as_ref().into()),
                Value::Bool(b) => Const::Bool(*b),
            }),
            Node::Field(i) => self.emit(Ins::Load(Load::Field(*i as u32))),
            Node::Var(v) => self.emit(Ins::Load(Load::of(*v))),
            Node::Not(a) => {
                let a = self.node(a);
                self.op(Ins::Not(a))
            }
            Node::Neg(a) => {
                let a = self.node(a);
                self.op(Ins::Neg(a))
            }
            Node::Bin(op, a, b) => {
                let a = self.node(a);
                let b = self.node(b);
                self.op(Ins::Bin(*op, [a, b]))
            }
            Node::Call(f, args) => {
                let args: Box<[Operand]> = args.iter().map(|a| self.node(a)).collect();
                self.op(Ins::Call(*f, args))
            }
        }
    }

    /// An operation: folded when its operands are constants, empty (thrown)
    /// when one of them threw, else an instruction.
    fn op(&mut self, ins: Ins) -> Operand {
        let thrown = |o: &Operand| matches!(o, Operand::Const(c) if self.consts[*c as usize] == Const::Thrown);
        if ins.operands().iter().any(thrown) {
            // Every operation of the language reads all its operands: one that threw empties it.
            return self.constant(Const::Thrown);
        }
        if ins.constant() {
            let c = self.fold(&ins);
            return self.constant(c);
        }
        self.emit(ins)
    }

    /// The register of an instruction: the one already made for the same work, or a new one.
    fn emit(&mut self, ins: Ins) -> Operand {
        if let Some(&r) = self.known.get(&ins) {
            return Operand::Reg(r);
        }
        let r = self.code.len() as u32;
        self.known.insert(ins.clone(), r);
        self.code.push(ins);
        Operand::Reg(r)
    }

    /// An operation on constants, computed now.
    fn fold(&mut self, ins: &Ins) -> Const {
        let consts = &self.consts;
        let get = |o: &Operand| match o {
            Operand::Const(c) => consts[*c as usize].view().unwrap_or(V::Null),
            Operand::Reg(_) => V::Null,
        };
        let mut out = String::new();
        let r = match ins {
            Ins::Load(_) => R::V(V::Null),
            Ins::Not(a) => R::V(scalar::not(get(a))),
            Ins::Neg(a) => R::V(scalar::neg(get(a))),
            Ins::Bin(op, [a, b]) => {
                scalar::binary(*op, get(a), get(b), &mut out, &mut self.scratch)
            }
            Ins::Call(f, args) => {
                let args: Vec<V> = args.iter().map(get).collect();
                functions::call(*f, &args, &mut out, &mut self.scratch)
            }
        };
        match r {
            R::V(v) => Const::of(v),
            R::Made => Const::Text(out.into()),
            R::Thrown => Const::Thrown,
        }
    }
}
