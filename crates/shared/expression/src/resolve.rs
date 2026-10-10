//! From the parser's tree to the one the engine evaluates (docs/adr/0214
//! §3): `@name` becomes its value from the schema (a constant, folded with
//! the rest), `şimdi()` and `bugün()` the caller's moment, `@katman_adi`
//! the object's layer; a function that looks at other objects becomes a
//! `WorldCall` (its layer's name, its constant text, its inner expressions
//! compiled on their own) and a `World` node with the object's own
//! arguments. The fields only inner expressions name leave the field list.

use crate::host::Schema;
use crate::library::{Family, Func, Var};
use crate::parser::Node;
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

    fn node(&mut self, n: Node) -> Node {
        let mut b = |n: Box<Node>| Box::new(self.node(*n));
        match n {
            Node::At(name) => self.at(&name),
            Node::Call(Func::Now, _) => self.at("simdi"),
            Node::Call(Func::Today, _) => self.at("tarih"),
            Node::Call(f, args) if f.family() == Family::World => self.world(f, args),
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
            n @ (Node::Lit(_) | Node::Field(_) | Node::Var(_) | Node::World(..)) => n,
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
        let mut own = Vec::new();
        for (arg, role) in args.into_iter().zip(roles(f).unwrap_or_default()) {
            match role {
                Role::Layer => call.layer = text(&arg),
                Role::Text => call.text = text(&arg),
                Role::Value => call.value = Some(self.inner(arg)),
                Role::Group => call.group = Some(self.inner(arg)),
                Role::Condition => call.condition = Some(self.inner(arg)),
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
        Node::World(self.world.len() as u32 - 1, own)
    }
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
