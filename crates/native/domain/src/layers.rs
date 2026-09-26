//! The layer tree (`LayerStore` in apps/web/src/model/layers.ts): groups and
//! layers, their visibility, lock and fold, and the active layer.
//!
//! Visibility and lock are inherited: a layer is shown only when it and every
//! group above it are shown, and locked when it or a group above it is. The
//! tree is kept exactly as the file has it (`LayerNode`), so a drawing writes
//! back what it read.

use std::collections::{HashMap, HashSet};

use kentos_contracts::{LayerNode, LayerNodeType, LayerStyle, LineType};

#[derive(Clone, Debug)]
pub struct LayerTree {
    roots: Vec<LayerNode>,
    active: String,
    /// Where each node is: child indices from the roots. An id a file repeats
    /// resolves to its last node in tree order, as the web's index does.
    paths: HashMap<String, Vec<usize>>,
    /// The number in the last `layer-N` id given or read: new layers count on
    /// from it, so an id is never given twice (the web's `uid`).
    counter: u64,
}

/// A layer or group to add (the web's `LayerInit` for `LayerStore.add`).
#[derive(Clone, Debug, PartialEq)]
pub struct NewLayer {
    /// Kept when the tree has no node with it (an import's `import-parsel`);
    /// otherwise the node gets the next `layer-N`.
    pub id: Option<String>,
    pub name: String,
    pub kind: LayerNodeType,
    pub visible: bool,
    pub locked: bool,
    pub style: LayerStyle,
}

impl NewLayer {
    /// A shown, unlocked layer in the default style.
    pub fn layer(name: impl Into<String>) -> Self {
        Self {
            id: None,
            name: name.into(),
            kind: LayerNodeType::Layer,
            visible: true,
            locked: false,
            style: default_style(),
        }
    }

    /// An empty, shown group.
    pub fn group(name: impl Into<String>) -> Self {
        Self {
            kind: LayerNodeType::Group,
            ..Self::layer(name)
        }
    }
}

/// A new node's style when none is given (the web's `defaultStyle`).
pub fn default_style() -> LayerStyle {
    LayerStyle {
        color: "fg".into(),
        line_type: LineType::Continuous,
        line_weight: 0.18,
        fill: None,
        point: None,
        label: None,
        pick_interior: None,
        renderer: None,
    }
}

/// `N` of a `layer-N` id.
fn counted(id: &str) -> Option<u64> {
    id.strip_prefix("layer-")
        .filter(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
        .and_then(|n| n.parse().ok())
}

impl LayerTree {
    /// A tree read from a file. The active layer is kept when it is a layer,
    /// else the first layer is active (web: `LayerStore.reset`).
    pub(crate) fn new(roots: Vec<LayerNode>, active: &str) -> Self {
        let mut paths = HashMap::new();
        index(&roots, &mut Vec::new(), &mut paths);
        // Ids read from a file (`layer-12`) keep the counter ahead of them (web `build`).
        let counter = paths.keys().filter_map(|id| counted(id)).max().unwrap_or(0);
        let mut tree = Self {
            roots,
            active: String::new(),
            paths,
            counter,
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

    /// Shows every node; false when all were shown already (nothing changed, docs/adr/0020).
    pub(crate) fn show_all(&mut self) -> bool {
        let mut changed = false;
        visit_mut(&mut self.roots, &mut Vec::new(), &mut |_, node| {
            changed |= !node.visible;
            node.visible = true;
        });
        changed
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
            // The same name again changes nothing (docs/adr/0020).
            Some(node) if !name.is_empty() && node.name != name => {
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

    /// Adds a node last in `parent` when that is a group, last in the group
    /// of `parent` when that is a layer, last at the top of the tree otherwise
    /// (an unknown `parent` too). The group it goes into is opened (web
    /// `LayerStore.add`). Returns the new node's id.
    pub(crate) fn add(&mut self, new: NewLayer, parent: Option<&str>) -> String {
        let container = parent
            .and_then(|id| Some((self.get(id)?.kind, self.paths.get(id)?.clone())))
            .and_then(|(kind, path)| match kind {
                LayerNodeType::Group => Some(path),
                // A layer's group; none at the top of the tree.
                LayerNodeType::Layer => {
                    Some(path[..path.len() - 1].to_vec()).filter(|p| !p.is_empty())
                }
            });
        let id = match new.id {
            Some(id) if !self.paths.contains_key(&id) => {
                self.counter = self.counter.max(counted(&id).unwrap_or(0));
                id
            }
            _ => self.next_id(),
        };
        let node = LayerNode {
            id: id.clone(),
            name: new.name,
            kind: new.kind,
            visible: new.visible,
            locked: new.locked,
            expanded: true,
            style: new.style,
            children: Vec::new(),
        };
        match container
            .as_deref()
            .and_then(|path| node_mut(&mut self.roots, path))
        {
            Some(group) => {
                group.children.push(node);
                group.expanded = true;
            }
            None => self.roots.push(node),
        }
        self.paths.clear();
        index(&self.roots, &mut Vec::new(), &mut self.paths);
        id
    }

    /// The next `layer-N` no node has.
    fn next_id(&mut self) -> String {
        loop {
            self.counter += 1;
            let id = format!("layer-{}", self.counter);
            if !self.paths.contains_key(&id) {
                return id;
            }
        }
    }

    /// `base` when no node (layer or group) has that name, else `base 2`,
    /// `base 3`… (web `LayerStore.uniqueName`).
    pub fn unique_name(&self, base: &str) -> String {
        let mut names = HashSet::new();
        names_of(&self.roots, &mut names);
        if !names.contains(base) {
            return base.to_owned();
        }
        (2u64..)
            .map(|i| format!("{base} {i}"))
            .find(|name| !names.contains(name.as_str()))
            .unwrap_or_else(|| base.to_owned())
    }
}

fn names_of<'a>(nodes: &'a [LayerNode], out: &mut HashSet<&'a str>) {
    for node in nodes {
        out.insert(node.name.as_str());
        names_of(&node.children, out);
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
