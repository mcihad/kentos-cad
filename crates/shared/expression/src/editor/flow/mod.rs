//! The expression as a flow of nodes (docs/adr/0101): the builder's second
//! view of the same text. Fields, `$` values and constants on the left,
//! operators, functions and the tests in between, the result on the right;
//! an edge is a value going into an input.
//!
//! The text is the flow's state. The result's expression is tree 0; a node
//! put down but not connected yet is the root of another tree, with the
//! place it stands. An input with nothing connected is `?` in the text.
//! Reading a tree is the language's parser (`tree::read`); writing it is
//! canonical (`T::text`), and reading what was written gives the same tree.
//! A host keeps the trees' texts, draws `flow`'s nodes and sends each
//! change as an `Edit`; `edit` answers with the new texts. The web (through
//! WASM) and the desktop get the same nodes, places and texts;
//! `fixtures/expression/v2/flow.json` pins them.
//!
//! Places are in units of a pixel at the dialogs' normal size: the result
//! node's top-left corner is 0, 0, its expression grows to the left, and a
//! tree not connected stands where it was put.

mod edit;
mod layout;
mod tree;
mod types;

pub use edit::{Edit, edit};
pub use layout::{COLUMN, GAP, HEAD, NODE_W, ROW, VALUE};
pub use tree::{T, read};
pub use types::Type;

use super::check::{Diagnostic, check};
use crate::Schema;

/// A tree of the flow as a host keeps it: its text (`?` an empty input)
/// and, for one not connected to the result, where its root stands.
#[derive(Clone, Debug, PartialEq)]
pub struct Tree {
    pub text: String,
    pub at: Option<(f64, f64)>,
}

impl Tree {
    pub fn new(text: impl Into<String>, at: Option<(f64, f64)>) -> Tree {
        Tree {
            text: text.into(),
            at,
        }
    }
}

/// What a node is, for its colours.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeKind {
    Result,
    Field,
    Variable,
    Number,
    Text,
    /// doğru, yanlış, boş.
    Constant,
    Function,
    /// Signs: = + || ^ …
    Operator,
    /// Words: ve, değil, içinde, durum …
    Keyword,
}

impl NodeKind {
    pub fn id(self) -> &'static str {
        match self {
            NodeKind::Result => "result",
            NodeKind::Field => "field",
            NodeKind::Variable => "variable",
            NodeKind::Number => "number",
            NodeKind::Text => "text",
            NodeKind::Constant => "constant",
            NodeKind::Function => "function",
            NodeKind::Operator => "operator",
            NodeKind::Keyword => "keyword",
        }
    }
}

/// An input of a node.
#[derive(Clone, Debug, PartialEq)]
pub struct Port {
    /// As the function's signature names it: `sayı`, `basamak`; `a`, `b`; `eğer 1`.
    pub name: String,
    /// What the input expects (a hint: the language converts).
    pub ty: Type,
    /// Whether the node works without it (left out of the text).
    pub optional: bool,
    /// The node connected to it.
    pub from: Option<String>,
    /// How the value connected is read when it is of another type.
    pub note: Option<String>,
    /// Whether the port can be taken away (one of a list's items).
    pub removable: bool,
    /// Its centre below the node's top edge.
    pub y: f64,
}

/// A node as the flow draws it.
#[derive(Clone, Debug, PartialEq)]
pub struct FlowNode {
    /// `r` for the result; else the tree and the ports from its root: `0`,
    /// `0.1`, `2.0.1`.
    pub id: String,
    pub kind: NodeKind,
    /// `yuvarla`, `=`, `Tapu alanı`, `$alan`, `12.5`, `'Arsa'`, `Sonuç`.
    pub title: String,
    /// The key of its help: `func:yuvarla`, `op:=`, `field:Ada`, `var:alan`.
    pub key: Option<String>,
    /// What it gives.
    pub ty: Type,
    pub ports: Vec<Port>,
    /// Whether an input can be added (a list's item, a `durum`'s condition).
    pub grows: bool,
    /// `değil` before içinde, arasında, gibi, benzer, or after boş: whether it is there.
    pub negated: Option<bool>,
    /// The node's own expression in the tree's text (the host previews it),
    /// and whether it is whole (no empty input in it).
    pub text: String,
    pub whole: bool,
    /// The language's error at this node, and its warnings (a field the objects do not have).
    pub error: Option<String>,
    pub warnings: Vec<String>,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// The flow of a set of trees.
#[derive(Clone, Debug, PartialEq)]
pub struct Flow {
    pub nodes: Vec<FlowNode>,
    /// Each tree's text as the flow writes it (canonical), in the host's order.
    pub texts: Vec<String>,
    /// The result's text does not read as an expression: the language's error.
    pub error: Option<Diagnostic>,
    /// The nodes' extent: left, top, right, bottom.
    pub bounds: (f64, f64, f64, f64),
}

/// A node's place: the result, or a tree and the ports from its root.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Ref {
    Result,
    Node { tree: usize, path: Vec<usize> },
}

impl Ref {
    pub(crate) fn parse(id: &str) -> Option<Ref> {
        if id == "r" {
            return Some(Ref::Result);
        }
        let mut parts = id.split('.').map(|p| p.parse::<usize>().ok());
        let tree = parts.next()??;
        let path = parts.collect::<Option<Vec<usize>>>()?;
        Some(Ref::Node { tree, path })
    }

    pub(crate) fn id(tree: usize, path: &[usize]) -> String {
        let mut s = tree.to_string();
        for p in path {
            s.push('.');
            s.push_str(&p.to_string());
        }
        s
    }
}

/// The nodes of the trees, laid out, with the diagnostics of their texts.
pub fn flow(trees: &[Tree], schema: &Schema) -> Flow {
    let mut read_trees: Vec<Option<T>> = Vec::with_capacity(trees.len());
    let mut error = None;
    for (i, t) in trees.iter().enumerate() {
        if t.text.trim().is_empty() {
            read_trees.push(None);
            continue;
        }
        match read(&t.text) {
            Ok(tree) => read_trees.push(Some(tree)),
            Err(_) => {
                if i == 0 {
                    // The editor's own diagnostic: its span, and the `?` explained.
                    error = check(&t.text, schema).error;
                }
                read_trees.push(None);
            }
        }
    }
    let mut out = layout::lay_out(&read_trees, trees, schema);
    out.error = error;
    out
}

#[cfg(test)]
mod tests;
