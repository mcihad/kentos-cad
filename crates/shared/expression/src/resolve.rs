//! From the parser's tree to the one the engine evaluates (docs/adr/0214
//! §3): `@name` becomes its value from the schema (a constant, folded with
//! the rest), `şimdi()` and `bugün()` the caller's moment, `@katman_adi`
//! the object's layer; a function that looks at other objects becomes a
//! `WorldCall` (its layer's name, its constant text, its inner expressions
//! compiled on their own) and a `World` node with the object's own
//! arguments. The fields only inner expressions name leave the field list.
//! A geometry value and a measure of the calls to other objects speak the
//! schema's unit (docs/adr/0165 §2): metres, or a local project's drawing
//! unit, multiplied in where they are read (an empty value stays empty).

use crate::host::Schema;
use crate::library::{Family, Func, Var};
use crate::parser::{BinOp, Node};
use crate::program::Program;
use crate::value::Value;
use crate::world::WorldCall;
use crate::world::roles::{Role, roles};
use crate::{Expr, FieldRef, FieldSource, FieldType, Needs, uses};

/// The tree resolved.
pub(crate) struct Resolved {
    pub root: Node,
    pub fields: Vec<String>,
    pub world: Vec<WorldCall>,
    /// The `@` names the schema does not know, as written, each once.
    pub unknown: Vec<String>,
}

pub(crate) fn resolve(root: Node, fields: &[String], schema: &Schema) -> Resolved {
    let mut r = Resolver {
        fields,
        schema,
        world: Vec::new(),
        unknown: Vec::new(),
        random: 0,
    };
    let root = r.node(root);
    let (root, fields) = own_fields(root, fields);
    Resolved {
        root,
        fields,
        world: r.world,
        unknown: r.unknown,
    }
}

struct Resolver<'s> {
    fields: &'s [String],
    schema: &'s Schema,
    world: Vec<WorldCall>,
    unknown: Vec<String>,
    /// The unseeded random calls met so far (docs/adr/0600 §3).
    random: u32,
}

impl Resolver<'_> {
    /// `@name`'s value; the object's layer for `@katman_adi`, `@katman`.
    fn at(&mut self, name: &str) -> Node {
        if let Some(v) = self.schema.variable(name) {
            return Node::Lit(v.value.clone());
        }
        let key = crate::js::text::fold_turkish(name);
        if matches!(key.as_str(), "KATMAN_ADI" | "KATMAN") {
            return Node::Var(Var::Layer);
        }
        if !self.unknown.iter().any(|u| u == name) {
            self.unknown.push(name.to_owned());
        }
        Node::Lit(Value::Null)
    }

    /// A random call: with a seed as it is; without, given the call's own number (the run's moment and its place)
    /// and the object's position, so it is neither folded nor shared and draws anew per object and per run
    /// (docs/adr/0600 §3).
    fn random(&mut self, f: Func, args: Vec<Node>) -> Node {
        let mut args: Vec<Node> = args.into_iter().map(|a| self.node(a)).collect();
        let seeded = match f {
            Func::Uuid => false,
            _ => args.len() >= 3,
        };
        if seeded {
            return Node::Call(f, args);
        }
        let moment = self
            .schema
            .variable("simdi")
            .map(|v| crate::value::to_text(&v.value).into_owned());
        self.random += 1;
        let site = crate::more::site_seed(moment.as_deref(), self.random);
        if f == Func::RandF {
            while args.len() < 2 {
                args.push(Node::Lit(Value::Num(if args.is_empty() {
                    0.0
                } else {
                    1.0
                })));
            }
        }
        if f != Func::Uuid {
            args.push(Node::Lit(Value::Null));
        }
        args.push(Node::Lit(Value::Num(site)));
        args.push(Node::Var(Var::Index));
        Node::Call(f, args)
    }

    fn node(&mut self, n: Node) -> Node {
        let mut b = |n: Box<Node>| Box::new(self.node(*n));
        match n {
            Node::At(name) => self.at(&name),
            Node::Call(Func::Now, _) => self.at("simdi"),
            Node::Call(Func::Today, _) => self.at("tarih"),
            Node::Call(f, args) if f.family() == Family::World => self.world(f, args),
            Node::Call(f @ (Func::Rand | Func::RandF | Func::Uuid), args) => self.random(f, args),
            Node::Call(f, args) => Node::Call(f, args.into_iter().map(|a| self.node(a)).collect()),
            Node::Not(a) => Node::Not(b(a)),
            Node::Neg(a) => Node::Neg(b(a)),
            Node::Bin(op, x, y) => {
                let x = b(x);
                Node::Bin(op, x, b(y))
            }
            Node::Case(whens, otherwise) => Node::Case(
                whens
                    .into_iter()
                    .map(|(c, v)| (self.node(c), self.node(v)))
                    .collect(),
                otherwise.map(|e| Box::new(self.node(*e))),
            ),
            Node::In(x, items, negated) => {
                let x = b(x);
                Node::In(
                    x,
                    items.into_iter().map(|i| self.node(i)).collect(),
                    negated,
                )
            }
            Node::Between(x, low, high, negated) => {
                let x = b(x);
                let low = b(low);
                Node::Between(x, low, b(high), negated)
            }
            Node::Like(x, p, fold, negated) => {
                let x = b(x);
                Node::Like(x, b(p), fold, negated)
            }
            Node::IsNull(x, negated) => Node::IsNull(b(x), negated),
            Node::Var(v) => self.var(v),
            Node::Field(i) => self.field(i),
            n @ (Node::Lit(_) | Node::World(..)) => n,
        }
    }

    /// A field the user defined with a type is read as that type (docs/adr/0401 §5): a yes/no field
    /// is true or false (its single form `true`, `false`; anything else empty), a number field a
    /// number (empty where it is not one). A text field, a date (ISO text compares as it is) and an
    /// attribute stay text.
    fn field(&self, i: usize) -> Node {
        let Some(name) = self.fields.get(i) else {
            return Node::Field(i);
        };
        let r = self.schema.resolve(name);
        match (r.source, r.ty) {
            (FieldSource::User, FieldType::Bool) => {
                // The text a layer keeps first; then the true or false a typed host gives.
                let is = |v: Value<'static>| {
                    Node::Bin(BinOp::Eq, Box::new(Node::Field(i)), Box::new(Node::Lit(v)))
                };
                let text = |t: &str| Value::Text(std::borrow::Cow::Owned(t.to_owned()));
                Node::Case(
                    vec![
                        (is(text("true")), Node::Lit(Value::Bool(true))),
                        (is(text("false")), Node::Lit(Value::Bool(false))),
                        (is(Value::Bool(true)), Node::Lit(Value::Bool(true))),
                        (is(Value::Bool(false)), Node::Lit(Value::Bool(false))),
                    ],
                    None,
                )
            }
            (FieldSource::User, FieldType::Number) => {
                Node::Call(Func::Number, vec![Node::Field(i)])
            }
            _ => Node::Field(i),
        }
    }

    /// A geometry value in the schema's unit: lengths and coordinates times
    /// the unit, areas times its square; the rest as it is.
    fn var(&self, v: Var) -> Node {
        let k = self.schema.unit.factor();
        match measure_of(v) {
            Some(Measure::Length) => times(Node::Var(v), k),
            Some(Measure::Area) => times(Node::Var(v), k * k),
            None => Node::Var(v),
        }
    }

    /// An expression over the other objects, compiled on its own.
    fn inner(&mut self, n: Node) -> Box<Expr> {
        // An inner expression holds no call that looks at other objects (the parser saw to it).
        let n = self.node(n);
        let (root, fields) = own_fields(n, self.fields);
        Box::new(inner_expr(root, fields))
    }

    fn world(&mut self, f: Func, args: Vec<Node>) -> Node {
        let text = |n: &Node| match n {
            Node::Lit(Value::Text(t)) => Some(t.to_string()),
            _ => None,
        };
        let mut call = WorldCall {
            func: f,
            layer: None,
            text: None,
            value: None,
            group: None,
            condition: None,
        };
        let k = self.schema.unit.factor();
        // kesişir, kesişen_sayısı and kesişenler's own argument is a distance in the unit; the calls measure in metres.
        let distance = matches!(
            f,
            Func::Intersects | Func::IntersectCount | Func::Intersecting
        );
        let mut own = Vec::new();
        for (arg, role) in args.into_iter().zip(roles(f).unwrap_or_default()) {
            match role {
                Role::Layer => call.layer = text(&arg),
                Role::Text => call.text = text(&arg),
                Role::Value => call.value = Some(self.inner(arg)),
                Role::Group => call.group = Some(self.inner(arg)),
                Role::Condition => call.condition = Some(self.inner(arg)),
                Role::Current if distance => own.push(per(self.node(arg), k)),
                Role::Current => own.push(self.node(arg)),
            }
        }
        // `katmandan`'s key field is read on the other objects as their group.
        if f == Func::FromLayer
            && let Some(name) = call.text.clone()
        {
            call.group = Some(Box::new(inner_expr(Node::Field(0), vec![name])));
        }
        // A layer's name that is not a constant text (the flow's empty input): no layer.
        if roles(f).is_some_and(|r| r.first() == Some(&Role::Layer)) && call.layer.is_none() {
            call.layer = Some(String::new());
        }
        self.world.push(call);
        let node = Node::World(self.world.len() as u32 - 1, own);
        match f {
            Func::Distance | Func::OverlapLength => times(node, k),
            Func::OverlapArea => times(node, k * k),
            _ => node,
        }
    }
}

/// What a geometry value measures: a length (coordinates and distances too) or an area.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Measure {
    Length,
    Area,
}

fn measure_of(v: Var) -> Option<Measure> {
    match v {
        Var::Area => Some(Measure::Area),
        Var::Length
        | Var::Y
        | Var::X
        | Var::CentroidY
        | Var::CentroidX
        | Var::MinY
        | Var::MaxY
        | Var::MinX
        | Var::MaxX
        | Var::Width
        | Var::Height => Some(Measure::Length),
        Var::Vertices | Var::Kind | Var::Layer | Var::Label | Var::Index | Var::Id | Var::Scale => {
            None
        }
    }
}

/// `n × k`, `n` itself in metres (k = 1). The language's product of an empty value is empty.
fn times(n: Node, k: f64) -> Node {
    if k == 1.0 {
        return n;
    }
    Node::Bin(BinOp::Mul, Box::new(n), Box::new(Node::Lit(Value::Num(k))))
}

/// `n ÷ k`, `n` itself in metres: a distance written in the unit, in metres.
fn per(n: Node, k: f64) -> Node {
    if k == 1.0 {
        return n;
    }
    Node::Bin(BinOp::Div, Box::new(n), Box::new(Node::Lit(Value::Num(k))))
}

/// An expression over other objects: its fields text attributes.
fn inner_expr(root: Node, fields: Vec<String>) -> Expr {
    let mut needs = Needs::default();
    uses(&root, &mut needs);
    let types = fields
        .iter()
        .map(|_| FieldRef {
            ty: FieldType::Text,
            source: FieldSource::Attribute,
        })
        .collect();
    Expr {
        source: String::new(),
        fields,
        needs,
        types,
        program: Program::compile(&root),
        root,
        world: Vec::new(),
        unknown_variables: Vec::new(),
    }
}

/// The tree with only the fields it names, numbered in the order it reads them.
fn own_fields(root: Node, all: &[String]) -> (Node, Vec<String>) {
    let mut map: Vec<Option<usize>> = vec![None; all.len()];
    let mut kept: Vec<String> = Vec::new();
    let root = renumber(root, all, &mut map, &mut kept);
    (root, kept)
}

fn renumber(n: Node, all: &[String], map: &mut Vec<Option<usize>>, kept: &mut Vec<String>) -> Node {
    let mut go = |n: Node| renumber(n, all, map, kept);
    match n {
        Node::Field(i) => {
            let Some(name) = all.get(i) else {
                return Node::Lit(Value::Null);
            };
            if map[i].is_none() {
                map[i] = Some(kept.len());
                kept.push(name.clone());
            }
            Node::Field(map[i].unwrap_or(0))
        }
        Node::Call(f, args) => Node::Call(f, args.into_iter().map(go).collect()),
        Node::World(k, args) => Node::World(k, args.into_iter().map(go).collect()),
        Node::Not(a) => Node::Not(Box::new(go(*a))),
        Node::Neg(a) => Node::Neg(Box::new(go(*a))),
        Node::Bin(op, x, y) => {
            let x = go(*x);
            Node::Bin(op, Box::new(x), Box::new(go(*y)))
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
        Node::In(x, items, negated) => {
            let x = go(*x);
            Node::In(Box::new(x), items.into_iter().map(go).collect(), negated)
        }
        Node::Between(x, low, high, negated) => {
            let x = go(*x);
            let low = go(*low);
            Node::Between(Box::new(x), Box::new(low), Box::new(go(*high)), negated)
        }
        Node::Like(x, p, fold, negated) => {
            let x = go(*x);
            Node::Like(Box::new(x), Box::new(go(*p)), fold, negated)
        }
        Node::IsNull(x, negated) => Node::IsNull(Box::new(go(*x)), negated),
        n @ (Node::Lit(_) | Node::Var(_) | Node::At(_)) => n,
    }
}
