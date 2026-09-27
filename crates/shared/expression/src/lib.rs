//! İfadeler: a small, safe expression language for processing tools
//! (select by expression, field calculator, filters) and the style engine
//! (data-defined values, rule and category renderers). No eval: the source
//! is tokenized, parsed (precedence climbing) and compiled once into a flat
//! program the column engine runs a batch of objects at a time (`program`,
//! `exec`, docs/adr/0100); one object walks the tree with the same rules
//! (`walk`, `scalar`).
//!
//!   Nitelik = 'Arsa' ve $alan > 500
//!   'P' || doldur($sıra, 5)
//!   yuvarla([Tapu alanı] - $alan, 2)
//!
//! Fields: bare names (Parsel) or in brackets ([Tapu alanı]). Text: '…' or
//! "…" (a doubled quote inside is one quote). Variables start with $ (see
//! `library.rs`). Keywords: ve/and, veya/or, değil/not, doğru/true,
//! yanlış/false, boş/null. Operators: = != <> < <= > >= + - * / % ^ and ||
//! (joins text). Since docs/adr/0100 §4: `durum eğer … ise … yoksa … son`
//! (CASE), `içinde` (IN), `arasında … ve` (BETWEEN), `gibi` and `benzer`
//! (LIKE, ILIKE), `boş` / `boş değil` after a value (IS [NOT] NULL), each
//! with `değil` before it where SQL puts NOT. A faithful port of the TypeScript it replaced
//! (`apps/web/src/model/expression/`, docs/adr/0008 “İfade dili”): the same
//! values, the same text, the same errors at the same positions.
//!
//! Its own crate since docs/adr/0100 (it was `kentos_style_core::expr`, which
//! stays as a re-export): the browser (through `kentos-geometry-wasm`), the
//! desktop and the server share it (CLAUDE.md §14).
#![forbid(unsafe_code)]
// User data must never crash the core (a panic traps the WASM module).
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

pub mod api;
pub mod editor;
pub mod exec;
pub mod functions;
pub mod geometry;
pub mod host;
pub mod js;
mod kernels;
pub mod lexer;
pub mod library;
pub mod parser;
pub mod program;
pub mod read;
pub mod rows;
pub mod scalar;
pub mod value;
pub mod walk;

pub use host::{Builtin, FieldDef, FieldRef, FieldSource, FieldType, Geometry, Objects, Schema};
use library::Var;
use parser::{Node, Parser};
use program::Program;
pub use value::Value;

/// What an expression that does not compile says: a message for the dialog
/// and the 1-based position (in UTF-16 code units) it is about.
#[derive(Clone, Debug, PartialEq)]
pub struct CompileError {
    pub message: String,
    pub at: usize,
}

impl CompileError {
    /// "12. karakterde: …" (the dialog's form).
    pub fn text(&self) -> String {
        if self.at > 1 {
            format!("{}. karakterde: {}", self.at, self.message)
        } else {
            self.message.clone()
        }
    }
}

/// The variables an expression reads, so a caller computes only those.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Needs {
    /// Length, area or the anchor (`$uzunluk`, `$alan`, `$y`, `$x`: the store's `measures`).
    pub measured: bool,
    pub vertices: bool,
    pub kind: bool,
    pub layer: bool,
    pub label: bool,
    pub index: bool,
    pub id: bool,
    pub scale: bool,
    /// The area's centroid (`$merkez_y`, `$merkez_x`).
    pub centroid: bool,
    /// The bounding box (`$min_y` … `$genişlik`, `$yükseklik`).
    pub bounds: bool,
}

impl Needs {
    /// What either of two expressions reads.
    pub fn union(self, b: Needs) -> Needs {
        Needs {
            measured: self.measured || b.measured,
            vertices: self.vertices || b.vertices,
            kind: self.kind || b.kind,
            layer: self.layer || b.layer,
            label: self.label || b.label,
            index: self.index || b.index,
            id: self.id || b.id,
            scale: self.scale || b.scale,
            centroid: self.centroid || b.centroid,
            bounds: self.bounds || b.bounds,
        }
    }
}

/// An object's geometry values: `$uzunluk`, `$alan`, and the anchor behind `$y` and `$x`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Measured {
    pub length: Option<f64>,
    pub area: Option<f64>,
    pub anchor: Option<(f64, f64)>,
}

/// What an expression sees of one object (one-object evaluation; many
/// objects are read through `host::Objects`, a batch at a time).
pub trait Scope {
    /// Attribute `i` of the expression's field list (None: the object has no such attribute).
    fn field(&self, i: usize) -> Option<&str>;
    fn measured(&self) -> Measured;
    fn vertices(&self) -> Option<f64>;
    fn kind(&self) -> &str;
    fn layer(&self) -> &str;
    fn label(&self) -> Option<&str>;
    /// 1-based position of the object in the run.
    fn index(&self) -> f64;
    fn id(&self) -> f64;
    /// Denominator of the plot scale while drawing a symbol; None elsewhere.
    fn scale(&self) -> Option<f64>;

    /// A geometry value. By default the ones `measured` and `vertices` give;
    /// a scope that holds the object's shape gives the others too
    /// (`geometry::value`).
    fn geometry(&self, what: Geometry) -> Option<f64> {
        let m = || self.measured();
        match what {
            Geometry::Length => m().length,
            Geometry::Area => m().area,
            Geometry::AnchorY => m().anchor.map(|a| a.0),
            Geometry::AnchorX => m().anchor.map(|a| a.1),
            Geometry::Vertices => self.vertices(),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Expr {
    pub source: String,
    /// Attribute names the expression reads, in order of appearance.
    pub fields: Vec<String>,
    pub needs: Needs,
    /// What each field is (the same order as `fields`): a user-defined typed
    /// field of the schema it was compiled with, else a text attribute.
    pub types: Vec<FieldRef>,
    /// The expression's tree, walked for one object (`walk`).
    root: Node,
    /// The expression compiled for the column engine (docs/adr/0100).
    program: Program,
}

/// An expression whose names are all today's text attributes (no schema).
pub fn compile(source: &str) -> Result<Expr, CompileError> {
    compile_with(source, &Schema::default())
}

/// An expression whose names resolve against the host's fields: a
/// user-defined field of `schema` with its type, else a text attribute.
pub fn compile_with(source: &str, schema: &Schema) -> Result<Expr, CompileError> {
    let mut parser = Parser::new(lexer::tokenize(source)?);
    let root = parser.parse()?;
    let mut needs = Needs::default();
    uses(&root, &mut needs);
    let types = parser.fields.iter().map(|f| schema.resolve(f)).collect();
    Ok(Expr {
        source: source.to_string(),
        fields: parser.fields,
        needs,
        types,
        program: Program::compile(&root),
        root,
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
            Var::CentroidY | Var::CentroidX => needs.centroid = true,
            Var::MinY | Var::MaxY | Var::MinX | Var::MaxX | Var::Width | Var::Height => {
                needs.bounds = true
            }
        },
        Node::Call(_, args) => args.iter().for_each(|a| uses(a, needs)),
        Node::Not(a) | Node::Neg(a) | Node::IsNull(a, _) => uses(a, needs),
        Node::Bin(_, a, b) | Node::Like(a, b, ..) => {
            uses(a, needs);
            uses(b, needs);
        }
        Node::Between(a, b, c, _) => {
            uses(a, needs);
            uses(b, needs);
            uses(c, needs);
        }
        Node::In(a, items, _) => {
            uses(a, needs);
            items.iter().for_each(|i| uses(i, needs));
        }
        Node::Case(whens, otherwise) => {
            for (c, v) in whens {
                uses(c, needs);
                uses(v, needs);
            }
            if let Some(e) = otherwise {
                uses(e, needs);
            }
        }
        Node::Lit(_) | Node::Field(_) => {}
    }
}

impl Expr {
    /// The compiled program, for the column engine's callers (`rows`).
    pub fn program(&self) -> &Program {
        &self.program
    }

    /// The value for one object: empty where JavaScript would have thrown,
    /// and for a number that is not finite. Text borrows from the source
    /// and from the object where it can. Many objects are faster through
    /// the column engine (`rows::evaluate_rows`, `exec`): one call per batch.
    pub fn evaluate<'a>(&'a self, s: &'a dyn Scope) -> Value<'a> {
        // The rules' buffers are made only when an operation needs them.
        match walk::walk(&self.root, s, &mut None) {
            Ok(Value::Num(x)) if !x.is_finite() => Value::Null,
            Ok(v) => v,
            Err(walk::Thrown) => Value::Null,
        }
    }
}

#[cfg(test)]
mod tests;
