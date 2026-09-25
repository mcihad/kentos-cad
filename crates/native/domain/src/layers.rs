//! The layer tree (`LayerStore` in apps/web/src/model/layers.ts): groups and
//! layers, their visibility, lock and fold, and the active layer.
//!
//! Visibility and lock are inherited: a layer is shown only when it and every
//! group above it are shown, and locked when it or a group above it is. The
//! tree is kept exactly as the file has it (`LayerNode`), so a drawing writes
//! back what it read.

use std::collections::HashMap;

use kentos_contracts::{LayerNode, LayerNodeType, LayerStyle};

#[derive(Clone, Debug)]
pub struct LayerTree {
    roots: Vec<LayerNode>,
    active: String,
    /// Where each node is: child indices from the roots. An id a file repeats
    /// resolves to its last node in tree order, as the web's index does.
    paths: HashMap<String, Vec<usize>>,
}

impl LayerTree {
    /// A tree read from a file. The active layer is kept when it is a layer,
    /// else the first layer is active (web: `LayerStore.reset`).
    pub(crate) fn new(roots: Vec<LayerNode>, active: &str) -> Self {
        let mut paths = HashMap::new();
        index(&roots, &mut Vec::new(), &mut paths);
        let mut tree = Self {
            roots,
            active: String::new(),
            paths,
        };
        tree.active = match tree.get(active) {
            Some(node) if node.kind == LayerNodeType::Layer => active.to_owned(),
            _ => tree
                .leaves()
                .first()
                .map_or_else(|| active.to_owned(), |node| node.id.clone()),
        };
        tree
    }

    /// The top of the tree, as a file writes it.
    pub fn nodes(&self) -> &[LayerNode] {
        &self.roots
    }

    /// The layer new objects go to.
    pub fn active(&self) -> &str {
        &self.active
    }

    pub fn get(&self, id: &str) -> Option<&LayerNode> {
        self.paths.get(id).and_then(|path| node(&self.roots, path))
    }

    /// The group a node is in; `None` at the top of the tree or for an unknown id.
    pub fn parent(&self, id: &str) -> Option<&LayerNode> {
        let path = self.paths.get(id)?;
        node(&self.roots, path.get(..path.len().checked_sub(1)?)?)
    }

    /// Layers (not groups) in tree order.
    pub fn leaves(&self) -> Vec<&LayerNode> {
        let mut out = Vec::new();
        leaves(&self.roots, &mut out);
        out
    }

    /// Whether the node and every group above it are shown. An unknown id is shown (web).
    pub fn is_visible(&self, id: &str) -> bool {
        self.along(id).all(|node| node.visible)
    }

    /// Whether the node or a group above it is locked. An unknown id is not (web).
    pub fn is_locked(&self, id: &str) -> bool {
        self.along(id).any(|node| node.locked)
    }

    /// The node and the groups above it.
    fn along(&self, id: &str) -> impl Iterator<Item = &LayerNode> {
        let path = self.paths.get(id).map_or(&[][..], Vec::as_slice);
        (1..=path.len()).filter_map(|n| node(&self.roots, path.get(..n)?))
    }

    fn node_mut(&mut self, id: &str) -> Option<&mut LayerNode> {
        let path = self.paths.get(id)?;
        node_mut(&mut self.roots, path)
    }

    // ── Changes; whether anything happened (the document decides what is an edit) ──

    pub(crate) fn set_visible(&mut self, id: &str, visible: bool) -> bool {
        match self.node_mut(id) {
            Some(node) if node.visible != visible => {
                node.visible = visible;
                true
            }
            _ => false,
        }
    }

    pub(crate) fn toggle_locked(&mut self, id: &str) -> bool {
        self.node_mut(id)
            .map(|node| node.locked = !node.locked)
            .is_some()
    }

    /// Shows the node, the groups above it and everything below it; hides the
    /// rest. An unknown id changes nothing (the web hides every node then).
    pub(crate) fn isolate(&mut self, id: &str) -> bool {
        let Some(target) = self.paths.get(id).cloned() else {
            return false;
        };
        visit_mut(&mut self.roots, &mut Vec::new(), &mut |path, node| {
            // The node's own path and its groups' are prefixes of the target's; the nodes below it extend it.
            node.visible = target.starts_with(path) || path.starts_with(&target);
        });
        true
    }

    pub(crate) fn show_all(&mut self) {
        visit_mut(&mut self.roots, &mut Vec::new(), &mut |_, node| {
            node.visible = true
        });
    }

    pub(crate) fn set_expanded(&mut self, id: &str, expanded: bool) -> bool {
        match self.node_mut(id) {
            Some(node) if node.expanded != expanded => {
                node.expanded = expanded;
                true
            }
            _ => false,
        }
    }

    /// Renames a node; the name is trimmed and an empty one refused. Giving
    /// the same name again still counts (web).
    pub(crate) fn rename(&mut self, id: &str, name: &str) -> bool {
        let name = name.trim();
        match self.node_mut(id) {
            Some(node) if !name.is_empty() => {
                name.clone_into(&mut node.name);
                true
            }
            _ => false,
        }
    }

    /// Makes a layer the active one; groups and unknown ids are refused.
    pub(crate) fn set_active(&mut self, id: &str) -> bool {
        if self
            .get(id)
            .is_some_and(|node| node.kind == LayerNodeType::Layer)
        {
            id.clone_into(&mut self.active);
            return true;
        }
        false
    }

    pub(crate) fn replace_style(&mut self, id: &str, style: &LayerStyle) {
        if let Some(node) = self.node_mut(id) {
            node.style.clone_from(style);
        }
    }
}

fn index(nodes: &[LayerNode], prefix: &mut Vec<usize>, paths: &mut HashMap<String, Vec<usize>>) {
    for (i, node) in nodes.iter().enumerate() {
        prefix.push(i);
        paths.insert(node.id.clone(), prefix.clone());
        index(&node.children, prefix, paths);
        prefix.pop();
    }
}

fn node<'a>(roots: &'a [LayerNode], path: &[usize]) -> Option<&'a LayerNode> {
    let (first, rest) = path.split_first()?;
    rest.iter()
        .try_fold(roots.get(*first)?, |node, i| node.children.get(*i))
}

fn node_mut<'a>(roots: &'a mut [LayerNode], path: &[usize]) -> Option<&'a mut LayerNode> {
    let (first, rest) = path.split_first()?;
    rest.iter()
        .try_fold(roots.get_mut(*first)?, |node, i| node.children.get_mut(*i))
}

fn leaves<'a>(nodes: &'a [LayerNode], out: &mut Vec<&'a LayerNode>) {
    for node in nodes {
        match node.kind {
            LayerNodeType::Layer => out.push(node),
            LayerNodeType::Group => leaves(&node.children, out),
        }
    }
}

/// Calls `f` with every node and its path, in tree order.
fn visit_mut(
    nodes: &mut [LayerNode],
    prefix: &mut Vec<usize>,
    f: &mut impl FnMut(&[usize], &mut LayerNode),
) {
    for (i, node) in nodes.iter_mut().enumerate() {
        prefix.push(i);
        f(prefix, node);
        visit_mut(&mut node.children, prefix, f);
        prefix.pop();
    }
}
