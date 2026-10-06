//! Kullanılmayanları temizle's rule (docs/adr/0177 §5; the web's
//! `model/layerPurge.ts`, both run the shared cases
//! fixtures/layers/v1/purge.json written by
//! scripts/fixtures/layer_purge_cases.py).
//!
//! What may go: the layers (not the active one, not locked with their groups'
//! locks), the groups (not locked), the block definitions, the project
//! library's symbols and assets. Something stays when anything that stays
//! uses it: a layer an object of the drawing or of a staying definition is on;
//! a group a staying node is under; a definition an insert of the drawing or
//! of a staying definition places; a symbol an object draws with, a staying
//! layer's style names (`ref`) or a template of any library names (`symbol`);
//! an asset a staying symbol of any library or a staying layer's style draws
//! with (`asset`), or a picture of the drawing shows (docs/adr/0192 §2). What is found is the largest set nothing staying uses;
//! removing the checked ones takes the same rule over them alone.

use std::collections::HashSet;

use kentos_contracts::{LayerNode, LayerNodeType};
use serde_json::Value;

/// An object as the rule reads it: its layer, the library symbol it draws
/// with, the definition it places (its id as text), the library image a
/// picture shows (docs/adr/0192 §2).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PurgeObject {
    pub layer: String,
    pub symbol: Option<String>,
    pub block: Option<String>,
    pub asset: Option<String>,
}

/// A block definition and its objects.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PurgeBlock {
    pub id: String,
    pub name: String,
    pub objects: Vec<PurgeObject>,
}

/// A library item of any source; a symbol's and a template's JSON are read for
/// what they name.
#[derive(Clone, Debug, PartialEq)]
pub struct PurgeItem {
    pub id: String,
    pub kind: String,
    pub source: String,
    pub name: String,
    pub symbol: Option<Value>,
    pub template: Option<Value>,
}

/// What the rule reads of a drawing and its library.
#[derive(Clone, Debug, Default)]
pub struct PurgeSource<'a> {
    pub tree: &'a [LayerNode],
    pub active: &'a str,
    pub objects: Vec<PurgeObject>,
    pub blocks: Vec<PurgeBlock>,
    pub library: Vec<PurgeItem>,
}

/// The kinds the window lists, in its order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PurgeKind {
    Layers,
    Groups,
    Blocks,
    Symbols,
    Assets,
}

impl PurgeKind {
    pub const ALL: [PurgeKind; 5] = [
        PurgeKind::Layers,
        PurgeKind::Groups,
        PurgeKind::Blocks,
        PurgeKind::Symbols,
        PurgeKind::Assets,
    ];

    /// The shared cases' key.
    pub fn key(self) -> &'static str {
        match self {
            PurgeKind::Layers => "layers",
            PurgeKind::Groups => "groups",
            PurgeKind::Blocks => "blocks",
            PurgeKind::Symbols => "symbols",
            PurgeKind::Assets => "assets",
        }
    }
}

/// A listed thing: its id and what the window says it by (a node's path, a
/// definition's or an item's name); a layer to be unlocked first `locked`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Found {
    pub id: String,
    pub text: String,
    pub locked: bool,
}

/// What the window lists, each kind in its order: layers and groups in the
/// tree's, definitions in theirs, library items in the project library's.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PurgeFound {
    pub layers: Vec<Found>,
    pub groups: Vec<Found>,
    pub blocks: Vec<Found>,
    pub symbols: Vec<Found>,
    pub assets: Vec<Found>,
}

impl PurgeFound {
    pub fn of(&self, kind: PurgeKind) -> &[Found] {
        match kind {
            PurgeKind::Layers => &self.layers,
            PurgeKind::Groups => &self.groups,
            PurgeKind::Blocks => &self.blocks,
            PurgeKind::Symbols => &self.symbols,
            PurgeKind::Assets => &self.assets,
        }
    }

    /// Whether nothing is listed.
    pub fn is_empty(&self) -> bool {
        PurgeKind::ALL.iter().all(|k| self.of(*k).is_empty())
    }
}

/// Ids by kind: the checked ones, or what goes in the order it goes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PurgeIds {
    pub layers: Vec<String>,
    pub groups: Vec<String>,
    pub blocks: Vec<String>,
    pub symbols: Vec<String>,
    pub assets: Vec<String>,
}

impl PurgeIds {
    pub fn of(&self, kind: PurgeKind) -> &[String] {
        match kind {
            PurgeKind::Layers => &self.layers,
            PurgeKind::Groups => &self.groups,
            PurgeKind::Blocks => &self.blocks,
            PurgeKind::Symbols => &self.symbols,
            PurgeKind::Assets => &self.assets,
        }
    }

    pub fn of_mut(&mut self, kind: PurgeKind) -> &mut Vec<String> {
        match kind {
            PurgeKind::Layers => &mut self.layers,
            PurgeKind::Groups => &mut self.groups,
            PurgeKind::Blocks => &mut self.blocks,
            PurgeKind::Symbols => &mut self.symbols,
            PurgeKind::Assets => &mut self.assets,
        }
    }

    /// How many ids there are of every kind.
    pub fn len(&self) -> usize {
        PurgeKind::ALL.iter().map(|k| self.of(*k).len()).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

type Key = (PurgeKind, String);

fn key(kind: PurgeKind, id: &str) -> Key {
    (kind, id.to_owned())
}

/// Every string under `name` anywhere in a JSON value.
fn strings_at<'a>(value: &'a Value, name: &str, out: &mut Vec<&'a str>) {
    match value {
        Value::Array(list) => list.iter().for_each(|v| strings_at(v, name, out)),
        Value::Object(map) => {
            for (k, v) in map {
                match v {
                    Value::String(s) if k == name => out.push(s),
                    _ => strings_at(v, name, out),
                }
            }
        }
        _ => {}
    }
}

/// Every node with its parent, in the tree's order.
fn walk<'a>(
    nodes: &'a [LayerNode],
    parent: Option<&'a LayerNode>,
    out: &mut Vec<(&'a LayerNode, Option<&'a LayerNode>)>,
) {
    for n in nodes {
        out.push((n, parent));
        walk(&n.children, Some(n), out);
    }
}

fn nodes(tree: &[LayerNode]) -> Vec<(&LayerNode, Option<&LayerNode>)> {
    let mut out = Vec::new();
    walk(tree, None, &mut out);
    out
}

fn node_kind(n: &LayerNode) -> PurgeKind {
    match n.kind {
        LayerNodeType::Layer => PurgeKind::Layers,
        LayerNodeType::Group => PurgeKind::Groups,
    }
}

/// Each node's path (“Kadastro / Bina”) and the locked ones, by their own
/// lock or a group's above them.
fn tree_facts(tree: &[LayerNode]) -> (Vec<(String, String)>, HashSet<String>) {
    fn go(
        nodes: &[LayerNode],
        above: &[String],
        locked_above: bool,
        paths: &mut Vec<(String, String)>,
        locked: &mut HashSet<String>,
    ) {
        for n in nodes {
            let mut path = above.to_vec();
            path.push(n.name.clone());
            paths.push((n.id.clone(), path.join(" / ")));
            let here = locked_above || n.locked;
            if here {
                locked.insert(n.id.clone());
            }
            go(&n.children, &path, here, paths, locked);
        }
    }
    let mut paths = Vec::new();
    let mut locked = HashSet::new();
    go(tree, &[], false, &mut paths, &mut locked);
    (paths, locked)
}

fn removable(src: &PurgeSource<'_>, locked: &HashSet<String>) -> HashSet<Key> {
    let mut out = HashSet::new();
    for (n, _) in nodes(src.tree) {
        if locked.contains(&n.id) {
            continue;
        }
        match n.kind {
            LayerNodeType::Layer if n.id != src.active => {
                out.insert(key(PurgeKind::Layers, &n.id));
            }
            LayerNodeType::Group => {
                out.insert(key(PurgeKind::Groups, &n.id));
            }
            LayerNodeType::Layer => {}
        }
    }
    for b in &src.blocks {
        out.insert(key(PurgeKind::Blocks, &b.id));
    }
    for it in &src.library {
        if it.source == "project" {
            match it.kind.as_str() {
                "symbol" => out.insert(key(PurgeKind::Symbols, &it.id)),
                "asset" => out.insert(key(PurgeKind::Assets, &it.id)),
                _ => false,
            };
        }
    }
    out
}

fn used_by_staying(src: &PurgeSource<'_>, going: &HashSet<Key>) -> HashSet<Key> {
    fn objects(list: &[PurgeObject], used: &mut HashSet<Key>) {
        for o in list {
            used.insert(key(PurgeKind::Layers, &o.layer));
            if let Some(s) = &o.symbol {
                used.insert(key(PurgeKind::Symbols, s));
            }
            if let Some(b) = &o.block {
                used.insert(key(PurgeKind::Blocks, b));
            }
            if let Some(a) = &o.asset {
                used.insert(key(PurgeKind::Assets, a));
            }
        }
    }
    let mut used = HashSet::new();
    objects(&src.objects, &mut used);
    for b in &src.blocks {
        if !going.contains(&key(PurgeKind::Blocks, &b.id)) {
            objects(&b.objects, &mut used);
        }
    }
    for (n, parent) in nodes(src.tree) {
        if going.contains(&key(node_kind(n), &n.id)) {
            continue;
        }
        if let Some(p) = parent {
            used.insert(key(PurgeKind::Groups, &p.id));
        }
        if n.kind == LayerNodeType::Layer
            && let Some(renderer) = &n.style.renderer
        {
            let mut refs = Vec::new();
            strings_at(renderer, "ref", &mut refs);
            used.extend(refs.into_iter().map(|r| key(PurgeKind::Symbols, r)));
            let mut assets = Vec::new();
            strings_at(renderer, "asset", &mut assets);
            used.extend(assets.into_iter().map(|a| key(PurgeKind::Assets, a)));
        }
    }
    for it in &src.library {
        if it.kind == "template"
            && let Some(t) = &it.template
        {
            let mut refs = Vec::new();
            strings_at(t, "symbol", &mut refs);
            used.extend(refs.into_iter().map(|r| key(PurgeKind::Symbols, r)));
        }
        if it.kind == "symbol"
            && !going.contains(&key(PurgeKind::Symbols, &it.id))
            && let Some(s) = &it.symbol
        {
            let mut assets = Vec::new();
            strings_at(s, "asset", &mut assets);
            used.extend(assets.into_iter().map(|a| key(PurgeKind::Assets, a)));
        }
    }
    used
}

/// The largest part of `going` nothing staying uses: what something staying
/// uses is given back, round after round.
fn settle(src: &PurgeSource<'_>, mut going: HashSet<Key>) -> HashSet<Key> {
    loop {
        let used = used_by_staying(src, &going);
        let before = going.len();
        going.retain(|k| !used.contains(k));
        if going.len() == before {
            return going;
        }
    }
}

/// What Kullanılmayanları temizle lists for the drawing.
pub fn found(src: &PurgeSource<'_>) -> PurgeFound {
    let (paths, locked) = tree_facts(src.tree);
    let going = settle(src, removable(src, &locked));
    let on_objects: HashSet<&str> = src.objects.iter().map(|o| o.layer.as_str()).collect();
    let on_blocks: HashSet<&str> = src
        .blocks
        .iter()
        .filter(|b| !going.contains(&key(PurgeKind::Blocks, &b.id)))
        .flat_map(|b| b.objects.iter().map(|o| o.layer.as_str()))
        .collect();
    let mut out = PurgeFound::default();
    for ((n, _), (_, path)) in nodes(src.tree).into_iter().zip(&paths) {
        let entry = |locked: bool| Found {
            id: n.id.clone(),
            text: path.clone(),
            locked,
        };
        match n.kind {
            LayerNodeType::Group => {
                if going.contains(&key(PurgeKind::Groups, &n.id)) {
                    out.groups.push(entry(false));
                }
            }
            LayerNodeType::Layer => {
                if going.contains(&key(PurgeKind::Layers, &n.id)) {
                    out.layers.push(entry(false));
                } else if locked.contains(&n.id)
                    && n.id != src.active
                    && !on_objects.contains(n.id.as_str())
                    && !on_blocks.contains(n.id.as_str())
                {
                    out.layers.push(entry(true));
                }
            }
        }
    }
    let named = |id: &str, name: &str| Found {
        id: id.to_owned(),
        text: name.to_owned(),
        locked: false,
    };
    out.blocks = src
        .blocks
        .iter()
        .filter(|b| going.contains(&key(PurgeKind::Blocks, &b.id)))
        .map(|b| named(&b.id, &b.name))
        .collect();
    for it in src.library.iter().filter(|it| it.source == "project") {
        if it.kind == "symbol" && going.contains(&key(PurgeKind::Symbols, &it.id)) {
            out.symbols.push(named(&it.id, &it.name));
        }
        if it.kind == "asset" && going.contains(&key(PurgeKind::Assets, &it.id)) {
            out.assets.push(named(&it.id, &it.name));
        }
    }
    out
}

/// What removing the checked ones takes, in the order it goes (definitions in
/// rounds, one placed in another after it; groups deepest first, each empty
/// when it goes), and how many checked ones stay because something staying
/// uses them.
pub fn removed(src: &PurgeSource<'_>, checked: &PurgeIds) -> (PurgeIds, usize) {
    let (_, locked) = tree_facts(src.tree);
    let can = removable(src, &locked);
    let asked: HashSet<Key> = PurgeKind::ALL
        .iter()
        .flat_map(|k| checked.of(*k).iter().map(|id| key(*k, id)))
        .collect();
    let going = settle(
        src,
        asked.iter().filter(|k| can.contains(k)).cloned().collect(),
    );
    let has = |kind: PurgeKind, id: &str| going.contains(&key(kind, id));
    let mut out = PurgeIds::default();
    for (n, _) in nodes(src.tree) {
        if n.kind == LayerNodeType::Layer && has(PurgeKind::Layers, &n.id) {
            out.layers.push(n.id.clone());
        }
    }
    fn deepest(nodes: &[LayerNode], has: &dyn Fn(&str) -> bool, out: &mut Vec<String>) {
        for n in nodes {
            deepest(&n.children, has, out);
            if n.kind == LayerNodeType::Group && has(&n.id) {
                out.push(n.id.clone());
            }
        }
    }
    deepest(src.tree, &|id| has(PurgeKind::Groups, id), &mut out.groups);
    let mut left: Vec<&PurgeBlock> = src
        .blocks
        .iter()
        .filter(|b| has(PurgeKind::Blocks, &b.id))
        .collect();
    while !left.is_empty() {
        let inside: HashSet<&str> = left
            .iter()
            .flat_map(|b| b.objects.iter().filter_map(|o| o.block.as_deref()))
            .collect();
        let now: Vec<String> = left
            .iter()
            .filter(|b| !inside.contains(b.id.as_str()))
            .map(|b| b.id.clone())
            .collect();
        if now.is_empty() {
            break;
        }
        left.retain(|b| !now.contains(&b.id));
        out.blocks.extend(now);
    }
    for it in src.library.iter().filter(|it| it.source == "project") {
        if it.kind == "symbol" && has(PurgeKind::Symbols, &it.id) {
            out.symbols.push(it.id.clone());
        }
        if it.kind == "asset" && has(PurgeKind::Assets, &it.id) {
            out.assets.push(it.id.clone());
        }
    }
    let kept = asked.len() - going.len();
    (out, kept)
}
