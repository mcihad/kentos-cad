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
/// visibility; with `locks` its own lock, with `styles` a layer's style.
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
    pub missing: usize,
}

impl StateChanges {
    /// Whether applying it would change nothing.
    pub fn is_empty(&self) -> bool {
        self.visible.is_empty() && self.locked.is_empty() && self.styles.is_empty()
    }
}

/// What applying `state` to the tree changes: the visibility and the lock
/// (when kept) of the nodes it names that the tree still has, the style (when
/// kept) of those that are layers; a node the tree lacks is counted, one the
/// state does not name stays.
pub fn changes(tree: &[LayerNode], state: &LayerState) -> StateChanges {
    let mut all = Vec::new();
    walk(tree, &mut all);
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
    }
    out
}

/// Whether the tree is in the state: one of its nodes at least is in the tree,
/// and applying it would change nothing.
pub fn matches(tree: &[LayerNode], state: &LayerState) -> bool {
    let c = changes(tree, state);
    state.nodes.len() > c.missing && c.is_empty()
}
