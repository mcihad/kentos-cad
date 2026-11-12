//! What a network reads of the drawing (docs/adr/0209 §12, docs/adr/0641 §3): on the interface's thread, its
//! layers' objects with the definition's expressions evaluated and the edges' paths from the geometry store. A
//! whole read takes every object; after an edit only the objects that changed since the last read are read again
//! (the document's change journal names them), and the network thread puts the new input together from its last one
//! and them (`assemble`), in the order a whole read gives. Every object of the network's layers is read on its own (a
//! network's expressions look at their own object only, docs/adr/0214 §1), so the two give the same input. A
//! definition that lists a layer twice for one kind (the first filter that holds takes an object) or picks a device's
//! configuration from an attribute (its words count every point) is read whole.

use std::collections::{HashMap, HashSet};

use kentos_commands::{Entity, NetworkDef};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::ops::network::input::{from_store, turns_from_store};
use kentos_geometry_core::ops::network::{EdgeIn, JunctionIn, TurnIn};
use kentos_geometry_core::store::Store;
use kentos_processing::network::{Source, network_input};

/// What a whole read gives the network thread.
#[derive(Clone, Debug, PartialEq)]
pub struct Whole {
    pub edges: Vec<EdgeIn>,
    pub junctions: Vec<JunctionIn>,
    pub turns: Vec<TurnIn>,
    pub skipped: Vec<(f64, &'static str)>,
    pub problems: Vec<String>,
}

/// What a read after an edit gives: the members of each kind in order (object ids) and the inputs of the objects
/// read again; an object not read again keeps the input it had.
#[derive(Clone, Debug, PartialEq)]
pub struct Changed {
    /// The objects read again: their inputs are the new ones or none.
    pub again: HashSet<u32>,
    pub edge_order: Vec<u32>,
    pub edges: Vec<EdgeIn>,
    pub junction_order: Vec<u32>,
    pub junctions: Vec<JunctionIn>,
    pub turn_order: Vec<u32>,
    pub turns: Vec<TurnIn>,
    pub skipped: Vec<(f64, &'static str)>,
    pub problems: Vec<String>,
}

/// The objects a read took, by kind: none when its geometry gave the kind's input, else its kind's word (skipped).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Members {
    edges: HashMap<u32, Option<&'static str>>,
    junctions: HashMap<u32, Option<&'static str>>,
    turns: HashMap<u32, Option<&'static str>>,
}

impl Members {
    /// Whether the read took object `id` (any kind).
    pub fn holds(&self, id: u32) -> bool {
        self.edges.contains_key(&id)
            || self.junctions.contains_key(&id)
            || self.turns.contains_key(&id)
    }
}

/// The inputs the network thread keeps between builds.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Inputs {
    pub edges: Vec<EdgeIn>,
    pub junctions: Vec<JunctionIn>,
    pub turns: Vec<TurnIn>,
}

/// What evaluating some objects gave: the inputs and each kind's members.
struct Read {
    edges: Vec<EdgeIn>,
    junctions: Vec<JunctionIn>,
    turns: Vec<TurnIn>,
    members: Members,
    edge_skipped: Vec<(f64, &'static str)>,
    junction_skipped: Vec<(f64, &'static str)>,
    turn_skipped: Vec<(f64, &'static str)>,
    problems: Vec<String>,
}

/// The objects of a kind a read took: none when its geometry gave the kind's input, else its kind's word.
fn took<T>(
    list: &[(f64, T)],
    skipped: &[(f64, &'static str)],
) -> HashMap<u32, Option<&'static str>> {
    let mut out: HashMap<u32, Option<&'static str>> =
        list.iter().map(|(id, _)| (*id as u32, None)).collect();
    for (id, kind) in skipped {
        out.insert(*id as u32, Some(kind));
    }
    out
}

/// Evaluates `def` over the objects `by` gives (each layer's, in document order).
fn evaluate(
    def: &NetworkDef,
    model: &Document,
    store: &Store,
    by: &HashMap<&str, Vec<&Entity>>,
) -> Read {
    let by_layer = |l: &str| by.get(l).cloned().unwrap_or_default();
    let names: HashMap<String, String> = model
        .layers()
        .leaves()
        .into_iter()
        .map(|l| (l.id.clone(), l.name.clone()))
        .collect();
    let name = |id: &str| names.get(id).cloned().unwrap_or_else(|| id.to_owned());
    let measures = |list: &[&Entity]| {
        store.measures(
            &list
                .iter()
                .map(|e| f64::from(e.base().id))
                .collect::<Vec<_>>(),
        )
    };
    let input = network_input(
        def,
        &Source {
            by_layer: &by_layer,
            layer_name: &name,
            measures: &measures,
        },
    );
    let (edges, _, edge_skipped) = from_store(store, &input.edges, &[]);
    let (_, junctions, junction_skipped) = from_store(store, &[], &input.junctions);
    let (turns, turn_skipped) = turns_from_store(store, &input.turns);
    let members = Members {
        edges: took(&input.edges, &edge_skipped),
        junctions: took(&input.junctions, &junction_skipped),
        turns: took(&input.turns, &turn_skipped),
    };
    Read {
        edges,
        junctions,
        turns,
        members,
        edge_skipped,
        junction_skipped,
        turn_skipped,
        problems: input.problems,
    }
}

/// Every object of the network's layers read.
pub fn read_whole(def: &NetworkDef, model: &Document, store: &Store) -> (Whole, Members) {
    let layers: HashSet<&str> = def.layers().into_iter().collect();
    let mut by: HashMap<&str, Vec<&Entity>> = HashMap::new();
    for l in &layers {
        by.insert(l, model.by_layer(l).collect());
    }
    let read = evaluate(def, model, store, &by);
    let mut skipped = read.edge_skipped;
    skipped.extend(read.junction_skipped);
    skipped.extend(read.turn_skipped);
    (
        Whole {
            edges: read.edges,
            junctions: read.junctions,
            turns: read.turns,
            skipped,
            problems: read.problems,
        },
        read.members,
    )
}

/// Whether `def` can be read again object by object (see the module).
pub fn readable_in_part(def: &NetworkDef) -> bool {
    let once = |layers: Vec<&str>| {
        let set: HashSet<&str> = layers.iter().copied().collect();
        set.len() == layers.len()
    };
    once(def.edges.iter().map(|e| e.layer.as_str()).collect())
        && once(def.junctions.iter().map(|j| j.layer.as_str()).collect())
        && once(
            def.turns
                .iter()
                .flat_map(|t| t.layers.iter().map(|l| l.layer.as_str()))
                .collect(),
        )
        && def
            .terminals
            .as_ref()
            .is_none_or(|t| t.devices.iter().all(|d| d.by.is_none()))
}

/// The objects `changed` (ids) read again: what the network thread needs with its last input, and the members now
/// (the last read's `members`, those read again replaced).
/// None when the definition is read whole (`readable_in_part`).
pub fn read_changed(
    def: &NetworkDef,
    model: &Document,
    store: &Store,
    members: Members,
    changed: &HashSet<u32>,
) -> Option<(Changed, Members)> {
    if !readable_in_part(def) {
        return None;
    }
    let layers: HashSet<&str> = def.layers().into_iter().collect();
    let mut by: HashMap<&str, Vec<&Entity>> = HashMap::new();
    for id in changed {
        if let Some(e) = model.get(Slot(*id))
            && let Some(l) = layers.get(e.base().layer_id.as_str())
        {
            by.entry(l).or_default().push(e);
        }
    }
    let read = evaluate(def, model, store, &by);
    // The members now: the last read's, those read again replaced.
    let mut now = members;
    for kind in [&mut now.edges, &mut now.junctions, &mut now.turns] {
        for id in changed {
            kind.remove(id);
        }
    }
    now.edges.extend(read.members.edges.iter());
    now.junctions.extend(read.members.junctions.iter());
    now.turns.extend(read.members.turns.iter());
    // Each kind's members in a whole read's order: its layers in the definition's order, each in document order.
    let order = |layers: Vec<&str>, took: &HashMap<u32, Option<&'static str>>| -> Vec<u32> {
        layers
            .into_iter()
            .flat_map(|l| model.by_layer(l))
            .map(|e| e.base().id)
            .filter(|id| took.contains_key(id))
            .collect()
    };
    let edge_order = order(
        def.edges.iter().map(|e| e.layer.as_str()).collect(),
        &now.edges,
    );
    let junction_order = order(
        def.junctions.iter().map(|j| j.layer.as_str()).collect(),
        &now.junctions,
    );
    let turn_order = order(
        def.turns
            .iter()
            .flat_map(|t| t.layers.iter().map(|l| l.layer.as_str()))
            .collect(),
        &now.turns,
    );
    let skipped_of = |order: &[u32], took: &HashMap<u32, Option<&'static str>>| {
        order
            .iter()
            .filter_map(|id| took.get(id).copied().flatten().map(|k| (f64::from(*id), k)))
            .collect::<Vec<_>>()
    };
    let mut skipped = skipped_of(&edge_order, &now.edges);
    skipped.extend(skipped_of(&junction_order, &now.junctions));
    skipped.extend(skipped_of(&turn_order, &now.turns));
    Some((
        Changed {
            again: changed.clone(),
            edge_order,
            edges: read.edges,
            junction_order,
            junctions: read.junctions,
            turn_order,
            turns: read.turns,
            skipped,
            problems: read.problems,
        },
        now,
    ))
}

/// The items of each id in `order`: from `fresh` when read `again` (none when it gave none), else moved out of `old`
/// (an id's items lie together in both); and each item's place in `old` (`u32::MAX`: read again).
fn regroup<T>(
    old: Vec<T>,
    fresh: Vec<T>,
    order: &[u32],
    again: &HashSet<u32>,
    id: impl Fn(&T) -> f64,
) -> (Vec<T>, Vec<u32>) {
    let runs = |list: &[T]| {
        let mut at: HashMap<u32, (usize, usize)> = HashMap::with_capacity(list.len());
        let mut k = 0;
        while k < list.len() {
            let i = id(&list[k]);
            let mut e = k + 1;
            while e < list.len() && id(&list[e]) == i {
                e += 1;
            }
            at.entry(i as u32).or_insert((k, e));
            k = e;
        }
        at
    };
    let (old_at, fresh_at) = (runs(&old), runs(&fresh));
    let mut old: Vec<Option<T>> = old.into_iter().map(Some).collect();
    let mut fresh: Vec<Option<T>> = fresh.into_iter().map(Some).collect();
    let mut out = Vec::with_capacity(old.len() + fresh.len());
    let mut was = Vec::with_capacity(old.len() + fresh.len());
    for k in order {
        match (again.contains(k), fresh_at.get(k), old_at.get(k)) {
            (true, Some(&(a, b)), _) => {
                out.extend(fresh[a..b].iter_mut().filter_map(Option::take));
                was.resize(out.len(), u32::MAX);
            }
            (false, _, Some(&(a, b))) => {
                for (w, item) in old[a..b].iter_mut().enumerate() {
                    if let Some(item) = item.take() {
                        out.push(item);
                        was.push((a + w) as u32);
                    }
                }
            }
            _ => {}
        }
    }
    (out, was)
}

/// The network thread's new input and which item of its last one each new edge and junction is (`u32::MAX`: read
/// again), as `Graph::rebuild_known` takes them.
pub struct Assembled {
    pub inputs: Inputs,
    pub edge_was: Vec<u32>,
    pub junction_was: Vec<u32>,
}

/// The network thread's new input: its last one with the objects read again (`Changed`).
pub fn assemble(old: Inputs, changed: Changed) -> Assembled {
    let Changed {
        again,
        edge_order,
        edges,
        junction_order,
        junctions,
        turn_order,
        turns,
        ..
    } = changed;
    let (edges, edge_was) = regroup(old.edges, edges, &edge_order, &again, |e| e.id);
    let (junctions, junction_was) =
        regroup(old.junctions, junctions, &junction_order, &again, |j| j.id);
    let (turns, _) = regroup(old.turns, turns, &turn_order, &again, |t| t.id);
    Assembled {
        inputs: Inputs {
            edges,
            junctions,
            turns,
        },
        edge_was,
        junction_was,
    }
}
