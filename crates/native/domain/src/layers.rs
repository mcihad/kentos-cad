//! The layer tree (`LayerStore` in apps/web/src/model/layers.ts): groups and
//! layers, their visibility, lock and fold, and the active layer.
//!
//! Visibility and lock are inherited: a layer is shown only when it and every
//! group above it are shown, and locked when it or a group above it is. The
//! tree is kept exactly as the file has it (`LayerNode`), so a drawing writes
//! back what it read.

use std::collections::{HashMap, HashSet};

use kentos_contracts::{
    FeatureFeed, LayerField, LayerNode, LayerNodeType, LayerSnap, LayerStyle, LineType,
    ServiceLayer,
};

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

/// A layer or group to add (the web's `LayerInit` for `CadDocument.addLayer`).
#[derive(Clone, Debug, PartialEq)]
pub struct NewLayer {
    /// The node's id (an import's `import-parsel`); the document refuses one
    /// the tree already has. None gives the next `layer-N`.
    pub id: Option<String>,
    pub name: String,
    pub kind: LayerNodeType,
    pub visible: bool,
    pub locked: bool,
    pub style: LayerStyle,
    /// A layer's own snapping (docs/adr/0163 §4), kept by the node it makes
    /// (Başka çizimden al's layers, docs/adr/0193 §2); a group keeps none.
    pub snap: Option<LayerSnap>,
    /// A layer's fields (docs/adr/0199 §1): an import's file gives them
    /// (§6), Başka çizimden al keeps them; a group keeps none.
    pub fields: Vec<LayerField>,
    /// A layer drawn from a map service, or where its objects came from
    /// (docs/adr/0208 §2); a group keeps neither.
    pub service: Option<ServiceLayer>,
    pub feed: Option<FeatureFeed>,
    /// A layer's time setting, a group's scenario and a scenario layer's
    /// base layer (docs/adr/0210 §2): Senaryo oluştur's copies keep their
    /// source's time and stand for it; a group keeps only `scenario`.
    pub time: Option<kentos_contracts::LayerTime>,
    pub scenario: Option<kentos_contracts::ScenarioInfo>,
    pub replaces: Option<String>,
    /// A layer's filter (docs/adr/0211 §2), kept by the node it makes
    /// (Başka çizimden al's layers); a group keeps none.
    pub filter: Option<kentos_contracts::LayerFilter>,
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
            snap: None,
            fields: Vec::new(),
            service: None,
            feed: None,
            time: None,
            scenario: None,
            replaces: None,
            filter: None,
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

    /// A node's names from the top of the tree down (the web's `path`):
    /// “Kadastro / Parsel”; empty for an unknown id.
    pub fn path(&self, id: &str) -> String {
        let Some(path) = self.paths.get(id) else {
            return String::new();
        };
        (1..=path.len())
            .filter_map(|n| node(&self.roots, &path[..n]))
            .map(|n| n.name.as_str())
            .collect::<Vec<_>>()
            .join(" / ")
    }

    /// Layers (not groups) in tree order.
    pub fn leaves(&self) -> Vec<&LayerNode> {
        let mut out = Vec::new();
        leaves(&self.roots, &mut out);
        out
    }

    /// The layers under a node, in tree order: the node itself when it is a
    /// layer (the web's `leavesOf`); none for an unknown id.
    pub fn leaves_of(&self, id: &str) -> Vec<&LayerNode> {
        let mut out = Vec::new();
        if let Some(node) = self.get(id) {
            leaves(std::slice::from_ref(node), &mut out);
        }
        out
    }

    /// Where a node sits: its group (none at the top of the tree) and its
    /// place among the group's nodes (the web's `placeOf`).
    pub fn place_of(&self, id: &str) -> Option<(Option<String>, usize)> {
        let (&index, above) = self.paths.get(id)?.split_last()?;
        Some((
            node(&self.roots, above).map(|group| group.id.clone()),
            index,
        ))
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

    /// A layer's own snapping on the layer, or on every layer of the group (a
    /// group keeps none); `None` takes it off (docs/adr/0163 §4). Whether any changed.
    pub(crate) fn set_snap(&mut self, id: &str, snap: Option<LayerSnap>) -> bool {
        let ids: Vec<String> = self.leaves_of(id).iter().map(|n| n.id.clone()).collect();
        let mut changed = false;
        for leaf in ids {
            if let Some(node) = self.node_mut(&leaf)
                && node.snap != snap
            {
                node.snap = snap.clone();
                changed = true;
            }
        }
        changed
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

    /// Shows these nodes, the groups above them and everything below them;
    /// hides the rest (Katmanı yalıt, docs/adr/0177 §1). Unknown ids are left
    /// out; whether a node's visibility changed (none known, nothing did).
    pub(crate) fn isolate_many(&mut self, ids: &[String]) -> bool {
        let targets: Vec<Vec<usize>> = ids
            .iter()
            .filter_map(|id| self.paths.get(id).cloned())
            .collect();
        if targets.is_empty() {
            return false;
        }
        let mut changed = false;
        visit_mut(&mut self.roots, &mut Vec::new(), &mut |path, node| {
            let shown = targets
                .iter()
                .any(|target| target.starts_with(path) || path.starts_with(target));
            changed |= node.visible != shown;
            node.visible = shown;
        });
        changed
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

    /// Takes a node out of the tree with everything under it (the document's
    /// undoable layer ops: a removal, and the undo of an addition; nothing
    /// else calls it). When the active layer goes with it, the first layer
    /// of the tree becomes active, as a new tree's does (the web's
    /// `detach`). An unknown id changes nothing.
    pub(crate) fn detach(&mut self, id: &str) {
        let Some(path) = self.paths.get(id).cloned() else {
            return;
        };
        let Some((&index, above)) = path.split_last() else {
            return;
        };
        if let Some(list) = list_mut(&mut self.roots, above)
            && index < list.len()
        {
            list.remove(index);
        }
        self.reindex();
        if self
            .get(&self.active)
            .is_none_or(|node| node.kind != LayerNodeType::Layer)
            && let Some(first) = self.leaves().first().map(|node| node.id.clone())
        {
            self.active = first;
        }
    }

    /// Puts a node where `detach` took it from (undo), or where the
    /// document's `add_layer` puts a new one: a copy of it with its
    /// children, flags and style, at `index` in `parent` (or last there); a
    /// group that is gone puts it at the top (the web's `attach`). A node
    /// whose id the tree already has is not put in twice.
    pub(crate) fn attach(&mut self, node: &LayerNode, parent: Option<&str>, index: usize) {
        if self.paths.contains_key(&node.id) {
            return;
        }
        let group = parent
            .and_then(|id| self.paths.get(id).cloned())
            .unwrap_or_default();
        if let Some(list) = list_mut(&mut self.roots, &group) {
            let at = index.min(list.len());
            list.insert(at, node.clone());
        }
        self.reindex();
    }

    fn reindex(&mut self) {
        self.paths.clear();
        index(&self.roots, &mut Vec::new(), &mut self.paths);
    }

    pub(crate) fn replace_style(&mut self, id: &str, style: &LayerStyle) {
        if let Some(node) = self.node_mut(id) {
            node.style.clone_from(style);
        }
    }

    /// A layer's fields (docs/adr/0199 §1), as an undo or a redo puts them.
    pub(crate) fn replace_fields(&mut self, id: &str, fields: &[LayerField]) {
        if let Some(node) = self.node_mut(id) {
            node.fields = fields.to_vec();
        }
    }

    /// A layer's name, map service and source (docs/adr/0208 §2, §10), as an undo or a redo puts them.
    /// A node's time setting, scenario and base layer as they are now.
    pub fn temporal_of(&self, id: &str) -> Option<crate::history::Temporal> {
        self.get(id).map(|n| crate::history::Temporal {
            time: n.time.clone(),
            scenario: n.scenario.clone(),
            replaces: n.replaces.clone(),
        })
    }

    pub(crate) fn replace_temporal(&mut self, id: &str, t: &crate::history::Temporal) {
        if let Some(node) = self.node_mut(id) {
            node.time.clone_from(&t.time);
            node.scenario.clone_from(&t.scenario);
            node.replaces.clone_from(&t.replaces);
        }
    }

    /// A layer's filter as it is now (docs/adr/0211 §2); None for an unknown id.
    pub fn filter_of(&self, id: &str) -> Option<Option<kentos_contracts::LayerFilter>> {
        self.get(id).map(|n| n.filter.clone())
    }

    /// A layer's filter, as an undo or a redo puts it.
    pub(crate) fn replace_filter(
        &mut self,
        id: &str,
        filter: Option<&kentos_contracts::LayerFilter>,
    ) {
        if let Some(node) = self.node_mut(id) {
            node.filter = filter.cloned();
        }
    }

    pub(crate) fn replace_service(&mut self, id: &str, served: &crate::history::Served) {
        if let Some(node) = self.node_mut(id) {
            node.name.clone_from(&served.name);
            node.service.clone_from(&served.by.service);
            node.feed.clone_from(&served.by.feed);
        }
    }

    /// A node as the document's `add_layer` makes it, not yet in the tree:
    /// open, with no children, its id the given one (the counter kept ahead
    /// of a `layer-N`) or the next `layer-N` (the web's `LayerStore.make`).
    pub(crate) fn make(&mut self, new: NewLayer) -> LayerNode {
        let id = match new.id {
            Some(id) => {
                self.counter = self.counter.max(counted(&id).unwrap_or(0));
                id
            }
            None => self.next_id(),
        };
        LayerNode {
            id,
            name: new.name,
            kind: new.kind,
            visible: new.visible,
            locked: new.locked,
            expanded: true,
            snap: new.snap.filter(|_| new.kind == LayerNodeType::Layer),
            fields: if new.kind == LayerNodeType::Layer {
                new.fields
            } else {
                Vec::new()
            },
            service: new.service.filter(|_| new.kind == LayerNodeType::Layer),
            feed: new.feed.filter(|_| new.kind == LayerNodeType::Layer),
            time: new.time.filter(|_| new.kind == LayerNodeType::Layer),
            scenario: new.scenario.filter(|_| new.kind == LayerNodeType::Group),
            replaces: new.replaces.filter(|_| new.kind == LayerNodeType::Layer),
            filter: new.filter.filter(|_| new.kind == LayerNodeType::Layer),
            style: new.style,
            children: Vec::new(),
        }
    }

    /// What [`LayerTree::make`] would give, the counter left as it is: a
    /// node without an id gets the next `layer-N` (a command's plan,
    /// docs/adr/0208 §15).
    pub fn preview(&self, new: NewLayer) -> LayerNode {
        let mut copy = Self {
            roots: Vec::new(),
            active: String::new(),
            paths: self.paths.clone(),
            counter: self.counter,
        };
        copy.make(new)
    }

    /// The ids the next `n` nodes added without an id of their own would get
    /// (`layer-N` in turn), the counter left as it is: a command's plan that
    /// adds several (Senaryo oluştur, docs/adr/0210 §11).
    pub fn next_ids(&self, n: usize) -> Vec<String> {
        let mut copy = Self {
            roots: Vec::new(),
            active: String::new(),
            paths: self.paths.clone(),
            counter: self.counter,
        };
        (0..n).map(|_| copy.next_id()).collect()
    }

    /// The group a node added beside or into `parent` goes into: `parent`
    /// when it is a group, the group of `parent` when it is a layer, none
    /// (the top of the tree) for a top layer or an unknown id (the web's
    /// `containerFor`).
    pub(crate) fn container_for(&self, parent: Option<&str>) -> Option<String> {
        let node = self.get(parent?)?;
        match node.kind {
            LayerNodeType::Group => Some(node.id.clone()),
            LayerNodeType::Layer => self.parent(&node.id).map(|group| group.id.clone()),
        }
    }

    /// How many nodes a group has (the top of the tree for none): where a
    /// node added last goes.
    pub(crate) fn len_of(&self, container: Option<&str>) -> usize {
        match container.and_then(|id| self.get(id)) {
            Some(group) => group.children.len(),
            None => self.roots.len(),
        }
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

/// The nodes of the group at `path`, or the top of the tree for an empty path.
fn list_mut<'a>(roots: &'a mut Vec<LayerNode>, path: &[usize]) -> Option<&'a mut Vec<LayerNode>> {
    if path.is_empty() {
        return Some(roots);
    }
    node_mut(roots, path).map(|group| &mut group.children)
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
