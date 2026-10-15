//! The layer states' rules (docs/adr/0177 §4; the web's `model/layerStates.ts`,
//! both run the shared cases fixtures/layers/v1/states.json written by
//! scripts/fixtures/layer_state_cases.py): what a state keeps of the tree,
//! what applying it changes and whether the tree is in it.

use std::collections::HashMap;

use kentos_contracts::{LayerNode, LayerNodeType, LayerState, LayerStateNode, LayerStyle};

/// What a state keeps besides the visibility.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LayerStateParts {
    pub locks: bool,
    pub styles: bool,
}

/// What a saved state keeps: locks when one of its nodes has its lock, styles
/// when one has its style.
pub fn parts_of(state: &LayerState) -> LayerStateParts {
    LayerStateParts {
        locks: state.nodes.iter().any(|n| n.locked.is_some()),
        styles: state.nodes.iter().any(|n| n.style.is_some()),
    }
}

/// Every node of the tree in its order (a group before what it holds).
fn walk<'a>(nodes: &'a [LayerNode], out: &mut Vec<&'a LayerNode>) {
    for n in nodes {
        out.push(n);
        walk(&n.children, out);
    }
}

/// The tree as a state keeps it: every node in its order with its own
/// visibility, freeze and printing (docs/adr/0311 §3); with `locks` its own
/// lock, with `styles` a layer's style.
pub fn capture(tree: &[LayerNode], id: &str, name: &str, parts: LayerStateParts) -> LayerState {
    let mut all = Vec::new();
    walk(tree, &mut all);
    LayerState {
        id: id.to_owned(),
        name: name.to_owned(),
        nodes: all
            .into_iter()
            .map(|n| LayerStateNode {
                node: n.id.clone(),
                visible: n.visible,
                locked: parts.locks.then_some(n.locked),
                style: (parts.styles && n.kind == LayerNodeType::Layer).then(|| n.style.clone()),
                frozen: Some(n.frozen == Some(true)),
                plot: Some(n.plot != Some(false)),
                path: None,
            })
            .collect(),
    }
}

/// What applying a state changes, in its order; `missing` counts its nodes the
/// tree no longer has.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StateChanges {
    pub visible: Vec<(String, bool)>,
    pub locked: Vec<(String, bool)>,
    pub styles: Vec<(String, LayerStyle)>,
    pub frozen: Vec<(String, bool)>,
    pub plot: Vec<(String, bool)>,
    pub missing: usize,
}

impl StateChanges {
    /// Whether applying it would change nothing.
    pub fn is_empty(&self) -> bool {
        self.visible.is_empty()
            && self.locked.is_empty()
            && self.styles.is_empty()
            && self.frozen.is_empty()
            && self.plot.is_empty()
    }
}

/// What applying `state` to the tree changes: the visibility and the lock
/// (when kept) of the nodes it names that the tree still has, the style (when
/// kept) of those that are layers, their freeze and printing (when kept,
/// docs/adr/0311 §3); a node the tree lacks is counted, one the state does
/// not name stays. A freeze that would hold `active` (the active layer, or a
/// group above it) is passed over: the active layer is never frozen.
pub fn changes(tree: &[LayerNode], state: &LayerState) -> StateChanges {
    changes_keeping(tree, state, None)
}

/// [`changes`], the active layer kept thawed.
pub fn changes_keeping(
    tree: &[LayerNode],
    state: &LayerState,
    active: Option<&str>,
) -> StateChanges {
    let mut all = Vec::new();
    walk(tree, &mut all);
    let holding_active: std::collections::HashSet<&str> = match active {
        Some(active) => ancestors_of(tree, active),
        None => std::collections::HashSet::new(),
    };
    let by: HashMap<&str, &LayerNode> = all.into_iter().map(|n| (n.id.as_str(), n)).collect();
    let mut out = StateChanges::default();
    for e in &state.nodes {
        let Some(n) = by.get(e.node.as_str()) else {
            out.missing += 1;
            continue;
        };
        if n.visible != e.visible {
            out.visible.push((e.node.clone(), e.visible));
        }
        if let Some(locked) = e.locked
            && n.locked != locked
        {
            out.locked.push((e.node.clone(), locked));
        }
        if let Some(style) = &e.style
            && n.kind == LayerNodeType::Layer
            && &n.style != style
        {
            out.styles.push((e.node.clone(), style.clone()));
        }
        if let Some(frozen) = e.frozen
            && (n.frozen == Some(true)) != frozen
            && !(frozen && holding_active.contains(n.id.as_str()))
        {
            out.frozen.push((e.node.clone(), frozen));
        }
        if let Some(plot) = e.plot
            && (n.plot != Some(false)) != plot
        {
            out.plot.push((e.node.clone(), plot));
        }
    }
    out
}

/// The node `id` and the groups above it, by id.
fn ancestors_of<'a>(tree: &'a [LayerNode], id: &str) -> std::collections::HashSet<&'a str> {
    fn find<'a>(nodes: &'a [LayerNode], id: &str, path: &mut Vec<&'a str>) -> bool {
        for n in nodes {
            path.push(n.id.as_str());
            if n.id == id || find(&n.children, id, path) {
                return true;
            }
            path.pop();
        }
        false
    }
    let mut path = Vec::new();
    find(tree, id, &mut path);
    path.into_iter().collect()
}

/// Every node's path, its names from the top joined by “ / ” (the tree's
/// `path`), in the tree's order.
fn paths_of(tree: &[LayerNode]) -> Vec<(String, String)> {
    fn walk(nodes: &[LayerNode], above: &str, out: &mut Vec<(String, String)>) {
        for n in nodes {
            let path = if above.is_empty() {
                n.name.clone()
            } else {
                format!("{above} / {}", n.name)
            };
            out.push((n.id.clone(), path.clone()));
            walk(&n.children, &path, out);
        }
    }
    let mut out = Vec::new();
    walk(tree, "", &mut out);
    out
}

/// The states as a file holds them (docs/adr/0311 §3): each node with its
/// path, when the tree has it.
pub fn exported(tree: &[LayerNode], states: &[LayerState]) -> Vec<LayerState> {
    let paths: HashMap<String, String> = paths_of(tree).into_iter().collect();
    states
        .iter()
        .map(|s| LayerState {
            nodes: s
                .nodes
                .iter()
                .map(|n| LayerStateNode {
                    path: paths.get(&n.node).cloned(),
                    ..n.clone()
                })
                .collect(),
            ..s.clone()
        })
        .collect()
}

/// A file's state as this drawing keeps it (docs/adr/0311 §3): a node found
/// by its id, else by its path (names folded, the first in the tree's order),
/// else kept with its id (missing when applied), or left out without one;
/// paths dropped; of a node found twice the first.
pub fn imported(tree: &[LayerNode], state: &LayerState) -> Vec<LayerStateNode> {
    let all = paths_of(tree);
    let ids: std::collections::HashSet<&str> = all.iter().map(|(id, _)| id.as_str()).collect();
    let mut by_path: HashMap<String, &str> = HashMap::new();
    for (id, path) in &all {
        by_path
            .entry(kentos_contracts::fold_name(path))
            .or_insert(id.as_str());
    }
    let mut out: Vec<LayerStateNode> = Vec::with_capacity(state.nodes.len());
    for n in &state.nodes {
        let node = if ids.contains(n.node.as_str()) {
            n.node.clone()
        } else if let Some(id) = n
            .path
            .as_deref()
            .and_then(|p| by_path.get(&kentos_contracts::fold_name(p)))
        {
            (*id).to_owned()
        } else if n.node.is_empty() {
            // Neither its id nor its path: nothing to keep it by.
            continue;
        } else {
            n.node.clone()
        };
        if out.iter().any(|m| m.node == node) {
            continue;
        }
        out.push(LayerStateNode {
            node,
            path: None,
            ..n.clone()
        });
    }
    out
}

/// Whether the tree is in the state: one of its nodes at least is in the tree,
/// and applying it would change nothing.
pub fn matches(tree: &[LayerNode], state: &LayerState) -> bool {
    let c = changes(tree, state);
    state.nodes.len() > c.missing && c.is_empty()
}

/// The first “Durum n” no state is named (Yeni durum kaydet's name).
pub fn next_state_name(states: &[LayerState]) -> String {
    (1..)
        .map(|n| format!("Durum {n}"))
        .find(|name| !states.iter().any(|s| s.name.trim() == name))
        .unwrap_or_default()
}

/// The first “durum-n” no state has as its id.
pub fn next_state_id(states: &[LayerState]) -> String {
    (1..)
        .map(|n| format!("durum-{n}"))
        .find(|id| !states.iter().any(|s| s.id == *id))
        .unwrap_or_default()
}

/// Why `name` (trimmed) may not name a state other than `this`, in the
/// window's words; none when it may (`cad.layers.states`, docs/adr/0248 §3).
pub fn name_refused(states: &[LayerState], name: &str, this: Option<&str>) -> Option<String> {
    if name.is_empty() {
        return Some("Durumun adını yazın.".to_owned());
    }
    states
        .iter()
        .any(|s| s.name.trim() == name && Some(s.id.as_str()) != this)
        .then(|| {
            format!(
                "“{name}” adında bir katman durumu var; başka bir ad yazın ya da o durumu seçip Güncelle'ye basın."
            )
        })
}
