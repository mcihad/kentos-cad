//! Changes to the flow (docs/adr/0101), as the dialogs send them: put a
//! node down, connect an output to an input, take a connection or a node
//! away, set a value, add or take an input of a list, move a tree. Each
//! answers with the trees' new texts (canonical) and the node to select.
//! A subtree taken from its place is not lost: it stands where it stood,
//! as a tree of its own.

use super::layout;
use super::tree::{T, binop, func_def, read};
use super::{Ref, Tree};
use crate::Schema;
use crate::js::text::fold_turkish;
use crate::library::{find_function, find_variable};

/// A change to the flow.
#[derive(Clone, Debug, PartialEq)]
pub enum Edit {
    /// A new node from the palette, not connected, its top-left at `at`: a
    /// key of the builder's tree (`func:yuvarla`, `op:+`, `op:durum`,
    /// `var:alan`, `field:Ada`), or `lit:number`, `lit:text`.
    Add {
        key: String,
        at: (f64, f64),
    },
    /// An expression put down as it is written (a field's value from the
    /// help, `'Arsa'`), not connected, its top-left at `at`.
    AddText {
        text: String,
        at: (f64, f64),
    },
    /// Node `from`'s value into input `port` of node `to` (`r`, the result).
    Connect {
        from: String,
        to: String,
        port: usize,
    },
    /// What input `port` of `to` holds goes apart, where it stands.
    Disconnect {
        to: String,
        port: usize,
    },
    /// The node goes; what its inputs held goes apart.
    Remove {
        node: String,
    },
    SetNumber {
        node: String,
        value: f64,
    },
    SetText {
        node: String,
        value: String,
    },
    SetBool {
        node: String,
        value: bool,
    },
    SetNull {
        node: String,
    },
    SetField {
        node: String,
        name: String,
    },
    SetVariable {
        node: String,
        name: String,
    },
    /// Another operator for the same two inputs.
    SetOperator {
        node: String,
        symbol: String,
    },
    /// Another function: the inputs it can take stay.
    SetFunction {
        node: String,
        name: String,
    },
    /// `değil` before içinde, arasında, gibi, benzer, or after boş.
    SetNegated {
        node: String,
        value: bool,
    },
    /// gibi (false) or benzer (true).
    SetFold {
        node: String,
        value: bool,
    },
    /// One more input: a list's item, a `durum`'s condition and value.
    AddPort {
        node: String,
    },
    RemovePort {
        node: String,
        port: usize,
    },
    /// A tree not connected to the result, to a new place.
    Move {
        tree: usize,
        at: (f64, f64),
    },
}

/// The trees as the edit leaves them, and the node to select.
pub type Edited = (Vec<Tree>, Option<String>);

/// A tree as the change works on it: read (None: empty), and its place.
type Held = (Option<T>, Option<(f64, f64)>);

struct State {
    /// The result's tree (None: nothing connected) and the others with their places.
    trees: Vec<Held>,
    /// Where each node stood before the change (by id): subtrees taken apart stay there.
    places: Vec<(String, (f64, f64))>,
}

impl State {
    fn read(trees: &[Tree], schema: &Schema) -> Result<State, String> {
        let mut out = Vec::with_capacity(trees.len().max(1));
        for (i, t) in trees.iter().enumerate() {
            if t.text.trim().is_empty() {
                out.push((None, t.at));
                continue;
            }
            match read(&t.text) {
                Ok(tree) => out.push((Some(tree), t.at)),
                Err(e) if i == 0 => {
                    return Err(format!(
                        "İfade okunamadı ({}); akışta değiştirmek için önce metni düzeltin.",
                        e.text()
                    ));
                }
                Err(e) => return Err(format!("{}. ağaç okunamadı: {}", i, e.text())),
            }
        }
        if out.is_empty() {
            out.push((None, None));
        }
        let flow = super::flow(trees, schema);
        let places = flow
            .nodes
            .iter()
            .map(|n| (n.id.clone(), (n.x, n.y)))
            .collect();
        Ok(State { trees: out, places })
    }

    fn place(&self, id: &str) -> (f64, f64) {
        self.places
            .iter()
            .find(|(i, _)| i == id)
            .map_or((-layout::COLUMN, 0.0), |(_, at)| *at)
    }

    fn get(&self, r: &Ref) -> Option<&T> {
        match r {
            Ref::Result => None,
            Ref::Node { tree, path } => self.trees.get(*tree)?.0.as_ref()?.at(path),
        }
    }

    fn get_mut(&mut self, r: &Ref) -> Option<&mut T> {
        match r {
            Ref::Result => None,
            Ref::Node { tree, path } => self.trees.get_mut(*tree)?.0.as_mut()?.at_mut(path),
        }
    }

    /// A subtree apart, where it stood.
    fn apart(&mut self, t: T, id: &str) {
        if !matches!(t, T::Hole) {
            let at = self.place(id);
            self.trees.push((Some(t), Some(at)));
        }
    }

    /// Takes the subtree at `r` out of its place (an empty input stays, a
    /// root leaves its tree empty) and returns it.
    fn take(&mut self, r: &Ref) -> Result<T, String> {
        let Ref::Node { tree, path } = r else {
            return Err("Sonuç düğümü bir yere bağlanamaz.".into());
        };
        match path.split_last() {
            None => self
                .trees
                .get_mut(*tree)
                .and_then(|(t, _)| t.take())
                .ok_or_else(|| missing(r)),
            Some((&port, parent)) => {
                let node = self
                    .get_mut(&Ref::Node {
                        tree: *tree,
                        path: parent.to_vec(),
                    })
                    .ok_or_else(|| missing(r))?;
                vacate(node, port).ok_or_else(|| missing(r))
            }
        }
    }

    fn into_trees(self) -> Vec<Tree> {
        let mut out = Vec::with_capacity(self.trees.len());
        for (i, (t, at)) in self.trees.into_iter().enumerate() {
            match t {
                Some(t) => out.push(Tree::new(t.text(), if i == 0 { None } else { at })),
                // The result's tree stays, empty; the others go.
                None if i == 0 => out.push(Tree::new("", None)),
                None => {}
            }
        }
        out
    }
}

fn missing(r: &Ref) -> String {
    match r {
        Ref::Result => "Sonuç düğümü".into(),
        Ref::Node { tree, path } => format!("“{}” düğümü yok.", Ref::id(*tree, path)),
    }
}

/// Empties input `port`: an optional last argument or `yoksa` goes, any
/// other input keeps its place, empty. What it held.
fn vacate(node: &mut T, port: usize) -> Option<T> {
    match node {
        T::Call(f, args) => {
            let (least, _) = func_def(*f).arity;
            if port + 1 == args.len() && port >= least {
                args.pop()
            } else {
                args.get_mut(port).map(|a| std::mem::replace(a, T::Hole))
            }
        }
        T::Case(whens, otherwise) if port == 2 * whens.len() => otherwise.take().map(|e| *e),
        _ => node.input_mut(port).map(|a| std::mem::replace(a, T::Hole)),
    }
}

/// Puts `value` into input `port` of `node`, the input it held going back
/// to the caller; an optional argument or `yoksa` not written yet is written.
fn fill(node: &mut T, port: usize, value: T) -> Result<T, String> {
    match node {
        T::Call(f, args) if port >= args.len() => {
            let def = func_def(*f);
            if def.arity.1.is_some_and(|most| port >= most) {
                return Err(format!(
                    "{}() en çok {} değer alır.",
                    def.name,
                    def.arity.1.unwrap_or(0)
                ));
            }
            while args.len() < port {
                args.push(T::Hole);
            }
            args.push(value);
            Ok(T::Hole)
        }
        T::Case(whens, otherwise) if port == 2 * whens.len() && otherwise.is_none() => {
            *otherwise = Some(Box::new(value));
            Ok(T::Hole)
        }
        T::In(_, items, _) if port == items.len() + 1 => {
            items.push(value);
            Ok(T::Hole)
        }
        _ => match node.input_mut(port) {
            Some(slot) => Ok(std::mem::replace(slot, value)),
            None => Err(format!("Bu düğümün {}. girişi yok.", port + 1)),
        },
    }
}

/// A new node for a key of the palette.
fn new_node(key: &str) -> Result<T, String> {
    let unknown = || format!("Paletten bilinmeyen öğe: “{key}”.");
    let (kind, name) = key.split_once(':').ok_or_else(unknown)?;
    Ok(match kind {
        "func" => {
            let f = find_function(name).ok_or_else(unknown)?;
            T::Call(f.func, vec![T::Hole; f.arity.0])
        }
        "var" => T::Var(find_variable(name).ok_or_else(unknown)?.var),
        "field" => T::Field(name.to_string()),
        "lit" => match name {
            "number" => T::Num(0.0),
            "text" => T::Text(String::new()),
            _ => return Err(unknown()),
        },
        "op" => match fold_turkish(name).as_str() {
            "DEGIL" => T::Not(Box::new(T::Hole)),
            "ICINDE" => T::In(Box::new(T::Hole), vec![T::Hole], false),
            "ARASINDA" => T::Between(
                Box::new(T::Hole),
                Box::new(T::Hole),
                Box::new(T::Hole),
                false,
            ),
            "GIBI" => T::Like(Box::new(T::Hole), Box::new(T::Hole), false, false),
            "BENZER" => T::Like(Box::new(T::Hole), Box::new(T::Hole), true, false),
            "DURUM" => T::Case(vec![(T::Hole, T::Hole)], Some(Box::new(T::Hole))),
            "DOGRU" => T::Bool(true),
            "YANLIS" => T::Bool(false),
            "BOS" => T::Null,
            "BOS DEGIL" => T::IsNull(Box::new(T::Hole), true),
            _ => {
                let op = binop(name).ok_or_else(unknown)?;
                T::Bin(op, Box::new(T::Hole), Box::new(T::Hole))
            }
        },
        _ => return Err(unknown()),
    })
}

/// Whether `inner` is `outer` or inside it.
fn within(inner: &Ref, outer: &Ref) -> bool {
    match (inner, outer) {
        (Ref::Node { tree: a, path: p }, Ref::Node { tree: b, path: q }) => {
            a == b && p.starts_with(q)
        }
        _ => false,
    }
}

fn node_ref(id: &str) -> Result<Ref, String> {
    Ref::parse(id).ok_or_else(|| format!("Düğüm kimliği okunamadı: “{id}”."))
}

/// Applies a change; the trees' new texts and the node to select.
pub fn edit(trees: &[Tree], change: &Edit, schema: &Schema) -> Result<Edited, String> {
    let mut s = State::read(trees, schema)?;
    let focus = apply(&mut s, change)?;
    Ok((s.into_trees(), focus))
}

fn apply(s: &mut State, change: &Edit) -> Result<Option<String>, String> {
    match change {
        Edit::Add { key, at } => {
            let t = new_node(key)?;
            s.trees.push((Some(t), Some(*at)));
            Ok(Some((s.trees.len() - 1).to_string()))
        }
        Edit::AddText { text, at } => {
            let t = read(text).map_err(|e| format!("Eklenemedi: {}", e.text()))?;
            s.trees.push((Some(t), Some(*at)));
            Ok(Some((s.trees.len() - 1).to_string()))
        }
        Edit::Connect { from, to, port } => {
            let (from, to) = (node_ref(from)?, node_ref(to)?);
            if within(&to, &from) {
                return Err("Bir düğüm kendi girişine bağlanamaz.".into());
            }
            if s.get(&from).is_none() {
                return Err(missing(&from));
            }
            if to != Ref::Result && s.get(&to).is_none() {
                return Err(missing(&to));
            }
            // What the input held before goes apart where it stood.
            let held_id = match &to {
                Ref::Result => "0".to_string(),
                Ref::Node { tree, path } => {
                    let mut p = path.clone();
                    p.push(*port);
                    Ref::id(*tree, &p)
                }
            };
            let value = s.take(&from)?;
            let new_id = match &to {
                Ref::Result => {
                    let old = s.trees[0].0.replace(value);
                    if let Some(old) = old {
                        s.apart(old, &held_id);
                    }
                    "0".to_string()
                }
                Ref::Node { .. } => {
                    let node = s.get_mut(&to).ok_or_else(|| missing(&to))?;
                    let held = fill(node, *port, value)?;
                    s.apart(held, &held_id);
                    held_id
                }
            };
            // A tree left empty by the move goes (the result's stays).
            Ok(Some(renumber(s, new_id)))
        }
        Edit::Disconnect { to, port } => {
            let to = node_ref(to)?;
            let (held, id) = match &to {
                Ref::Result => (s.trees[0].0.take(), "0".to_string()),
                Ref::Node { tree, path } => {
                    let mut p = path.clone();
                    p.push(*port);
                    let id = Ref::id(*tree, &p);
                    let node = s.get_mut(&to).ok_or_else(|| missing(&to))?;
                    (vacate(node, *port), id)
                }
            };
            if let Some(held) = held {
                s.apart(held, &id);
            }
            let to_id = match &to {
                Ref::Result => "r".to_string(),
                Ref::Node { tree, path } => Ref::id(*tree, path),
            };
            Ok(Some(renumber(s, to_id)))
        }
        Edit::Remove { node } => {
            let r = node_ref(node)?;
            let Ref::Node { tree, path } = &r else {
                return Err("Sonuç düğümü silinmez.".into());
            };
            let id = Ref::id(*tree, path);
            let t = s.take(&r)?;
            for (k, input) in t.inputs().into_iter().enumerate() {
                s.apart(input.clone(), &format!("{id}.{k}"));
            }
            renumber(s, String::new());
            Ok(None)
        }
        Edit::SetNumber { node, value } => leaf(s, node, T::Num(*value)),
        Edit::SetText { node, value } => leaf(s, node, T::Text(value.clone())),
        Edit::SetBool { node, value } => leaf(s, node, T::Bool(*value)),
        Edit::SetNull { node } => leaf(s, node, T::Null),
        Edit::SetField { node, name } => {
            if name.trim().is_empty() {
                return Err("Alan adı boş olamaz.".into());
            }
            leaf(s, node, T::Field(name.clone()))
        }
        Edit::SetVariable { node, name } => {
            let v = find_variable(name).ok_or_else(|| format!("Bilinmeyen değişken: ${name}."))?;
            leaf(s, node, T::Var(v.var))
        }
        Edit::SetOperator { node, symbol } => {
            let op = binop(symbol).ok_or_else(|| format!("Bilinmeyen işleç: “{symbol}”."))?;
            let r = node_ref(node)?;
            match s.get_mut(&r) {
                Some(T::Bin(o, ..)) => *o = op,
                _ => return Err("Yalnız bir işlecin işleci değişir.".into()),
            }
            Ok(Some(node.clone()))
        }
        Edit::SetFunction { node, name } => {
            let f = find_function(name).ok_or_else(|| format!("Bilinmeyen işlev: {name}()."))?;
            let r = node_ref(node)?;
            let extra = match s.get_mut(&r) {
                Some(T::Call(g, args)) => {
                    *g = f.func;
                    let (least, most) = f.arity;
                    let extra = match most {
                        Some(m) if args.len() > m => args.split_off(m),
                        _ => Vec::new(),
                    };
                    while args.len() < least {
                        args.push(T::Hole);
                    }
                    extra
                }
                _ => return Err("Yalnız bir işlevin işlevi değişir.".into()),
            };
            for (k, t) in extra.into_iter().enumerate() {
                let most = f.arity.1.unwrap_or(0);
                s.apart(t, &format!("{node}.{}", most + k));
            }
            Ok(Some(node.clone()))
        }
        Edit::SetNegated { node, value } => {
            let r = node_ref(node)?;
            match s.get_mut(&r) {
                Some(T::In(_, _, n) | T::Between(.., n) | T::Like(.., n) | T::IsNull(_, n)) => {
                    *n = *value
                }
                _ => return Err("Bu düğümün “değil”i yok.".into()),
            }
            Ok(Some(node.clone()))
        }
        Edit::SetFold { node, value } => {
            let r = node_ref(node)?;
            match s.get_mut(&r) {
                Some(T::Like(_, _, fold, _)) => *fold = *value,
                _ => return Err("Yalnız gibi ile benzer arasında geçilir.".into()),
            }
            Ok(Some(node.clone()))
        }
        Edit::AddPort { node } => {
            let r = node_ref(node)?;
            match s.get_mut(&r) {
                Some(T::Call(f, args)) if func_def(*f).arity.1.is_none() => args.push(T::Hole),
                Some(T::In(_, items, _)) => items.push(T::Hole),
                Some(T::Case(whens, _)) => whens.push((T::Hole, T::Hole)),
                _ => return Err("Bu düğüme giriş eklenmez.".into()),
            }
            Ok(Some(node.clone()))
        }
        Edit::RemovePort { node, port } => {
            let r = node_ref(node)?;
            let gone: Vec<(T, usize)> = match s.get_mut(&r) {
                Some(T::Call(f, args))
                    if func_def(*f).arity.1.is_none()
                        && args.len() > func_def(*f).arity.0
                        && *port < args.len() =>
                {
                    vec![(args.remove(*port), *port)]
                }
                Some(T::In(_, items, _))
                    if items.len() > 1 && *port >= 1 && *port <= items.len() =>
                {
                    vec![(items.remove(*port - 1), *port)]
                }
                Some(T::Case(whens, otherwise)) if *port == 2 * whens.len() => otherwise
                    .take()
                    .map(|e| vec![(*e, *port)])
                    .unwrap_or_default(),
                Some(T::Case(whens, _)) if whens.len() > 1 && *port < 2 * whens.len() => {
                    let k = *port / 2;
                    let (c, v) = whens.remove(k);
                    vec![(c, 2 * k), (v, 2 * k + 1)]
                }
                _ => return Err("Bu giriş kaldırılamaz.".into()),
            };
            for (t, k) in gone {
                s.apart(t, &format!("{node}.{k}"));
            }
            Ok(Some(node.clone()))
        }
        Edit::Move { tree, at } => match s.trees.get_mut(*tree) {
            Some((Some(_), place)) if *tree > 0 => {
                *place = Some(*at);
                Ok(Some(tree.to_string()))
            }
            _ => Err("Yalnız sonuca bağlanmamış bir düğüm taşınır.".into()),
        },
    }
}

/// Replaces a leaf (a value, a field, a `$` value, an empty place) with another.
fn leaf(s: &mut State, node: &str, value: T) -> Result<Option<String>, String> {
    let r = node_ref(node)?;
    let t = s.get_mut(&r).ok_or_else(|| missing(&r))?;
    if !t.inputs().is_empty() {
        return Err("Girişleri olan bir düğüm değer olamaz; önce girişlerini ayırın.".into());
    }
    *t = value;
    Ok(Some(node.to_string()))
}

/// Drops the trees a change left empty (the result's stays) and gives the
/// id `id` has after the drop (ids name trees by position).
fn renumber(s: &mut State, id: String) -> String {
    let gone: Vec<usize> = (1..s.trees.len())
        .filter(|&i| s.trees[i].0.is_none())
        .collect();
    let mut i = 0;
    s.trees.retain(|(t, _)| {
        let keep = i == 0 || t.is_some();
        i += 1;
        keep
    });
    match Ref::parse(&id) {
        Some(Ref::Node { tree, path }) => {
            let shift = gone.iter().filter(|&&g| g < tree).count();
            Ref::id(tree - shift, &path)
        }
        _ => id,
    }
}
