//! The flow's nodes and where they stand (docs/adr/0101). A tree is laid
//! out right to left: its root, then each input's subtree a column further
//! left, the subtrees stacked in port order and the node centred on them.
//! The result stands at 0, 0 and its expression is centred on its input; a
//! tree not connected stands where it was put. The same trees give the same
//! places on every platform.

use kentos_geometry_core::jsmath::{js_max, js_min};

use super::tree::{T, func_def, symbol, var_def};
use super::types::{self, Type};
use super::{Flow, FlowNode, NodeKind, Port, Ref, Tree};
use crate::Schema;
use crate::editor::check::check;
use crate::parser::BinOp;

/// A node's width.
pub const NODE_W: f64 = 140.0;
/// The title's row.
pub const HEAD: f64 = 26.0;
/// An input's row.
pub const ROW: f64 = 22.0;
/// The value's row at the bottom.
pub const VALUE: f64 = 22.0;
/// Between two columns.
pub const GAP: f64 = 40.0;
/// A column: a node and the gap after it.
pub const COLUMN: f64 = NODE_W + GAP;
/// Between two subtrees stacked in a column.
const STACK: f64 = 14.0;
/// Under the lowest node, where a tree without a place goes.
const APART: f64 = 40.0;

/// A node's height with `ports` inputs.
fn height(ports: usize) -> f64 {
    HEAD + ROW * ports as f64 + VALUE
}

/// A node's height: a constant is its title alone (its value is the
/// title), anything else has its inputs and a row for its value.
fn node_height(t: &T) -> f64 {
    match t {
        T::Num(_) | T::Text(_) | T::Bool(_) | T::Null | T::Hole => HEAD + 4.0,
        _ => height(shape(t).ports.len()),
    }
}

/// The inputs a node shows: each connected one, and the empty ones it may
/// take (an optional argument, a list's next item is added by the host).
struct Shape {
    ports: Vec<(String, Type, bool)>,
    grows: bool,
    removable: bool,
}

fn shape(t: &T) -> Shape {
    let n = t.inputs().len();
    let fixed = |names: &[(&str, Type)]| Shape {
        ports: names
            .iter()
            .map(|&(n, ty)| (n.to_string(), ty, false))
            .collect(),
        grows: false,
        removable: false,
    };
    match t {
        T::Call(f, _) => {
            let def = func_def(*f);
            let (least, most) = def.arity;
            match most {
                Some(most) => Shape {
                    ports: (0..most)
                        .map(|i| {
                            let name = def.args.get(i).map_or("değer", |a| a.0);
                            (name.to_string(), types::takes(*f, i), i >= least)
                        })
                        .collect(),
                    grows: false,
                    removable: false,
                },
                // a, b, c …: as many as there are, at least `least`.
                None => Shape {
                    ports: (0..n.max(least))
                        .map(|i| (letter(i), types::takes(*f, i), false))
                        .collect(),
                    grows: true,
                    removable: n > least,
                },
            }
        }
        T::Not(_) => fixed(&[("koşul", Type::Bool)]),
        T::Neg(_) => fixed(&[("sayı", Type::Number)]),
        T::IsNull(..) => fixed(&[("değer", Type::Any)]),
        T::Bin(op, ..) => {
            let ty = types::operand(*op);
            fixed(&[("a", ty), ("b", ty)])
        }
        T::Like(..) => fixed(&[("değer", Type::Text), ("kalıp", Type::Text)]),
        T::Between(..) => fixed(&[("değer", Type::Any), ("alt", Type::Any), ("üst", Type::Any)]),
        T::In(_, items, _) => Shape {
            ports: std::iter::once(("değer".to_string(), Type::Any, false))
                .chain((0..items.len()).map(|i| (format!("öğe {}", i + 1), Type::Any, false)))
                .collect(),
            grows: true,
            removable: items.len() > 1,
        },
        T::Case(whens, _) => {
            let many = whens.len() > 1;
            let numbered = |w: &str, k: usize| {
                if many {
                    format!("{w} {}", k + 1)
                } else {
                    w.to_string()
                }
            };
            Shape {
                ports: (0..whens.len())
                    .flat_map(|k| {
                        [
                            (numbered("eğer", k), Type::Bool, false),
                            (numbered("ise", k), Type::Any, false),
                        ]
                    })
                    .chain(std::iter::once(("yoksa".to_string(), Type::Any, true)))
                    .collect(),
                grows: true,
                removable: many,
            }
        }
        _ => Shape {
            ports: Vec::new(),
            grows: false,
            removable: false,
        },
    }
}

/// `a`, `b` … `z`, `a2` …: a list's inputs.
fn letter(i: usize) -> String {
    let c = char::from(b'a' + (i % 26) as u8);
    if i < 26 {
        c.to_string()
    } else {
        format!("{c}{}", i / 26 + 1)
    }
}

/// A node's kind, title and help key.
fn face(t: &T) -> (NodeKind, String, Option<String>) {
    match t {
        T::Hole => (NodeKind::Constant, "?".into(), None),
        T::Num(_) => (NodeKind::Number, t.text(), None),
        T::Text(_) => (NodeKind::Text, t.text(), None),
        T::Bool(b) => (
            NodeKind::Constant,
            t.text(),
            Some(format!("op:{}", if *b { "doğru" } else { "yanlış" })),
        ),
        T::Null => (NodeKind::Constant, "boş".into(), Some("op:boş".into())),
        T::Field(name) => (NodeKind::Field, name.clone(), Some(format!("field:{name}"))),
        T::Var(v) => {
            let name = var_def(*v).name;
            (
                NodeKind::Variable,
                format!("${name}"),
                Some(format!("var:{name}")),
            )
        }
        T::Call(f, _) => {
            let name = func_def(*f).name;
            (
                NodeKind::Function,
                name.into(),
                Some(format!("func:{name}")),
            )
        }
        T::Not(_) => (NodeKind::Keyword, "değil".into(), Some("op:değil".into())),
        T::Neg(_) => (NodeKind::Operator, "−".into(), Some("op:-".into())),
        T::Bin(op, ..) => {
            let s = symbol(*op);
            let kind = if matches!(op, BinOp::And | BinOp::Or) {
                NodeKind::Keyword
            } else {
                NodeKind::Operator
            };
            (kind, s.into(), Some(format!("op:{s}")))
        }
        T::Case(..) => (NodeKind::Keyword, "durum".into(), Some("op:durum".into())),
        T::In(_, _, negated) => word("içinde", *negated),
        T::Between(.., negated) => word("arasında", *negated),
        T::Like(_, _, fold, negated) => word(if *fold { "benzer" } else { "gibi" }, *negated),
        T::IsNull(_, negated) => {
            let s = if *negated { "boş değil" } else { "boş" };
            (
                NodeKind::Keyword,
                format!("{s} mi"),
                Some(format!("op:{s}")),
            )
        }
    }
}

fn word(w: &str, negated: bool) -> (NodeKind, String, Option<String>) {
    let title = if negated {
        format!("değil {w}")
    } else {
        w.to_string()
    };
    (NodeKind::Keyword, title, Some(format!("op:{w}")))
}

fn negation(t: &T) -> Option<bool> {
    match t {
        T::In(_, _, n) | T::Between(.., n) | T::Like(.., n) | T::IsNull(_, n) => Some(*n),
        _ => None,
    }
}

fn whole(t: &T) -> bool {
    !matches!(t, T::Hole) && t.inputs().into_iter().all(whole)
}

/// The height of a subtree's block: its node, or its connected inputs'
/// blocks stacked, whichever is taller.
fn block(t: &T) -> f64 {
    let own = node_height(t);
    let kids: Vec<f64> = t
        .inputs()
        .into_iter()
        .filter(|c| !matches!(c, T::Hole))
        .map(block)
        .collect();
    if kids.is_empty() {
        return own;
    }
    let stacked = kids.iter().sum::<f64>() + STACK * (kids.len() - 1) as f64;
    js_max(own, stacked)
}

/// Places a subtree with its root's left edge at `x` and its block's top at `top`.
fn place(t: &T, path: &mut Vec<usize>, x: f64, top: f64, out: &mut Vec<(Vec<usize>, f64, f64)>) {
    let b = block(t);
    let own = node_height(t);
    let kids: Vec<(usize, &T)> = t
        .inputs()
        .into_iter()
        .enumerate()
        .filter(|(_, c)| !matches!(c, T::Hole))
        .collect();
    let stacked = kids.iter().map(|(_, k)| block(k)).sum::<f64>()
        + STACK * kids.len().saturating_sub(1) as f64;
    let mut y = top + (b - stacked) / 2.0;
    for (port, k) in kids {
        path.push(port);
        place(k, path, x - COLUMN, y, out);
        path.pop();
        y += block(k) + STACK;
    }
    out.push((path.clone(), x, top + (b - own) / 2.0));
}

/// Where the root of a subtree placed with its block's top at 0 stands.
fn root_offset(t: &T) -> f64 {
    (block(t) - node_height(t)) / 2.0
}

pub(crate) fn lay_out(read: &[Option<T>], trees: &[Tree], schema: &Schema) -> Flow {
    // The result: its input, from the result's tree.
    let main = read.first().and_then(Option::as_ref);
    let result_h = height(1);
    let mut nodes = vec![FlowNode {
        id: "r".into(),
        kind: NodeKind::Result,
        title: "Sonuç".into(),
        key: None,
        ty: main.map_or(Type::Any, |t| types::of(t, schema)),
        ports: vec![Port {
            name: "değer".into(),
            ty: Type::Any,
            optional: false,
            from: main.map(|_| "0".to_string()),
            note: None,
            removable: false,
            y: HEAD + ROW / 2.0,
        }],
        grows: false,
        negated: None,
        text: main.map(T::text).unwrap_or_default(),
        whole: main.is_some_and(whole),
        error: None,
        warnings: Vec::new(),
        x: 0.0,
        y: 0.0,
        w: NODE_W,
        h: result_h,
    }];
    let mut texts = Vec::with_capacity(trees.len());
    let mut bottom = result_h;
    // Where each tree's root goes: the result's centred on its input, the
    // others where they were put (after the rest when they have no place).
    let mut placed: Vec<(usize, &T, f64, f64)> = Vec::new();
    for (i, t) in read.iter().enumerate() {
        let Some(t) = t else {
            texts.push(trees.get(i).map(|t| t.text.clone()).unwrap_or_default());
            continue;
        };
        texts.push(t.text());
        if i == 0 {
            let port_y = HEAD + ROW / 2.0;
            let top = port_y - HEAD / 2.0 - root_offset(t);
            placed.push((0, t, -COLUMN, top));
            bottom = js_max(bottom, top + block(t));
        }
    }
    for (i, t) in read.iter().enumerate().skip(1) {
        let Some(t) = t else { continue };
        let (x, y) = match trees.get(i).and_then(|tr| tr.at) {
            Some(at) => at,
            None => (-COLUMN, bottom + APART),
        };
        let top = y - root_offset(t);
        placed.push((i, t, x, top));
        bottom = js_max(bottom, top + block(t));
    }
    for &(tree, t, x, top) in &placed {
        let mut at = Vec::new();
        place(t, &mut Vec::new(), x, top, &mut at);
        let (text, spans) = t.write();
        let diagnostics = check(&text, schema);
        for (path, nx, ny) in at {
            let Some(n) = t.at(&path) else { continue };
            let span = spans.iter().find(|(p, ..)| *p == path);
            nodes.push(node(
                n,
                tree,
                &path,
                (nx, ny),
                &text,
                span,
                &spans,
                &diagnostics,
                schema,
            ));
        }
    }
    // Draw order: the result first, then each tree root to leaves (a deterministic order).
    nodes.sort_by_key(|n| order(&n.id));
    let bounds = nodes.iter().fold(
        (
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ),
        |(l, t, r, b), n| {
            (
                js_min(l, n.x),
                js_min(t, n.y),
                js_max(r, n.x + n.w),
                js_max(b, n.y + n.h),
            )
        },
    );
    Flow {
        nodes,
        texts,
        error: None,
        bounds,
    }
}

/// Sorts ids: `r` first, then by tree and path.
fn order(id: &str) -> (usize, Vec<usize>) {
    match Ref::parse(id) {
        Some(Ref::Node { tree, path }) => (tree + 1, path),
        _ => (0, Vec::new()),
    }
}

#[allow(clippy::too_many_arguments)]
fn node(
    t: &T,
    tree: usize,
    path: &[usize],
    (x, y): (f64, f64),
    text: &str,
    span: Option<&(Vec<usize>, usize, usize)>,
    spans: &[(Vec<usize>, usize, usize)],
    diagnostics: &crate::editor::Check,
    schema: &Schema,
) -> FlowNode {
    let id = Ref::id(tree, path);
    let shape = shape(t);
    let inputs = t.inputs();
    let ports: Vec<Port> = shape
        .ports
        .iter()
        .enumerate()
        .map(|(k, (name, ty, optional))| {
            let child = inputs.get(k).filter(|c| !matches!(c, T::Hole));
            Port {
                name: name.clone(),
                ty: *ty,
                optional: *optional,
                from: child.map(|_| format!("{id}.{k}")),
                note: child.and_then(|c| types::note(types::of(c, schema), *ty)),
                removable: shape.removable
                    && k > 0
                    && !matches!(t, T::Case(..) if k + 1 == shape.ports.len()),
                y: HEAD + ROW * k as f64 + ROW / 2.0,
            }
        })
        .collect();
    let (kind, title, key) = face(t);
    let (start, end) = span.map_or((0, 0), |&(_, a, b)| (a, b));
    let own = unit_slice(text, start, end);
    // A diagnostic belongs to the innermost node whose span holds its start.
    let mine = |at: usize| {
        let inner = spans
            .iter()
            .filter(|(_, a, b)| *a <= at && (at < *b || (a == b && at == *a)))
            .min_by_key(|(_, a, b)| b - a);
        inner.is_some_and(|(p, ..)| p.as_slice() == path)
            // An empty input's `?` is its parent's.
            || inner.is_some_and(|(p, ..)| {
                p.len() == path.len() + 1
                    && p.starts_with(path)
                    && matches!(t.inputs().get(p[path.len()]), Some(T::Hole))
            })
    };
    let error = diagnostics
        .error
        .as_ref()
        .filter(|d| mine(d.start))
        .map(|d| d.message.clone());
    let warnings = diagnostics
        .warnings
        .iter()
        .filter(|d| mine(d.start))
        .map(|d| d.message.clone())
        .collect();
    FlowNode {
        id,
        kind,
        title,
        key,
        ty: types::of(t, schema),
        h: node_height(t),
        ports,
        grows: shape.grows,
        negated: negation(t),
        text: own,
        whole: whole(t),
        error,
        warnings,
        x,
        y,
        w: NODE_W,
    }
}

/// `s[start..end]` in UTF-16 units.
fn unit_slice(s: &str, start: usize, end: usize) -> String {
    let u: Vec<u16> = s.encode_utf16().collect();
    String::from_utf16_lossy(&u[start.min(u.len())..end.min(u.len())])
}
