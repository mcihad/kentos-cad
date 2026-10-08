//! Çizimler arası alışveriş (docs/adr/0193; the web's `model/exchange.ts`):
//! the rules of its three works over drawings in the contract's form, both
//! platforms held to the shared cases fixtures/exchange/v1/cases.json written
//! by scripts/fixtures/exchange_cases.py.
//!
//! - [`selection`]: the selected objects as a drawing of their own, with the
//!   layers, groups, blocks and library items they and their blocks use, the
//!   settings with the kept layers' states; links to objects left out drop.
//! - [`take`]: layers by path, text and dimension styles, library items by
//!   id (a symbol with its images), blocks by name (with those they place and
//!   the symbols their objects draw with), layer states by name, the
//!   project's units and scale, from another drawing; the same names skipped
//!   or replaced.
//! - [`file_block`]: another drawing's objects but pictures and tables as one
//!   block, its base given (the lower left of their extent, [`extent_corner`]);
//!   their layers by path (the missing made), their blocks and styles brought
//!   in under names this drawing has not.
//! - [`layer_take`]: another drawing's layer with its objects (Kaynaklar's
//!   Katman olarak ekle, docs/adr/0199 §7): the layer by path (a met one
//!   kept), the layers their blocks' objects are on, the blocks by name (a met
//!   name is ours), the styles by name (the missing added), the library items
//!   the objects and the made layers draw with; links, ties and a table's
//!   source dropped (the objects take new ids).
//!
//! The rules read and write the drawings as the contract's JSON, as the
//! reference does: an object's style, link and tie are its fields there.

use std::collections::{BTreeSet, HashMap, HashSet};

use kentos_contracts::{BlockId, DocumentSnapshotV2, EntityId};
use kentos_geometry_core::api::json::{FromJson, Json};
use kentos_geometry_core::entity::{Entity as CoreEntity, entity_bounds};
use serde_json::{Map, Value, json};

/// Whether a name met again is kept or taken over.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Same {
    Skip,
    Replace,
}

/// What Başka çizimden al takes: layers by their paths (“Kadastro / Bina”),
/// blocks, styles and states by name, library items by id, the project's
/// units and scale.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Picks {
    pub layers: Vec<String>,
    pub blocks: Vec<String>,
    pub text_styles: Vec<String>,
    pub dimension_styles: Vec<String>,
    pub library: Vec<String>,
    pub layer_states: Vec<String>,
    pub settings: bool,
}

/// The project settings Başka çizimden al takes (never the coordinate
/// systems); the annotation heights too (docs/adr/0205 §1).
pub const TAKEN_SETTINGS: [&str; 9] = [
    "lengthDecimals",
    "areaDecimals",
    "areaUnit",
    "angleUnit",
    "plotScale",
    "drawingFont",
    "drawingUnit",
    "survey",
    "annotation",
];

/// A name as names are compared: Turkish letters folded, case aside,
/// trimmed (layers, styles and states).
pub fn fold(name: &str) -> String {
    let lower: String = name
        .trim()
        .chars()
        .map(|c| match c {
            'İ' => 'i',
            'I' => 'ı',
            c => c,
        })
        .flat_map(char::to_lowercase)
        .collect();
    lower
        .chars()
        .map(|c| match c {
            'ç' => 'c',
            'ğ' => 'g',
            'ı' => 'i',
            'ö' => 'o',
            'ş' => 's',
            'ü' => 'u',
            'â' | 'á' | 'à' | 'ä' => 'a',
            'î' | 'í' | 'ì' | 'ï' => 'i',
            'û' | 'ú' | 'ù' => 'u',
            'ô' | 'ó' | 'ò' => 'o',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            c => c,
        })
        .collect()
}

/// A block's name as blocks compare them (docs/adr/0144).
pub fn block_key(name: &str) -> String {
    name.chars()
        .flat_map(|c| match c {
            'I' => vec!['ı'],
            'İ' => vec!['i'],
            c => c.to_lowercase().collect(),
        })
        .collect()
}

fn str_of<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key).and_then(Value::as_str).unwrap_or("")
}

fn arr<'a>(v: &'a Value, key: &str) -> &'a [Value] {
    v.get(key)
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

fn arr_mut<'a>(v: &'a mut Value, key: &str) -> &'a mut Vec<Value> {
    let slot = &mut v[key];
    if !slot.is_array() {
        *slot = Value::Array(Vec::new());
    }
    match slot {
        Value::Array(list) => list,
        // Made an array just above.
        _ => unreachable!(),
    }
}

/// Every node with its path of names, in the tree's order.
fn walk(nodes: &[Value], above: &[String], out: &mut Vec<(Value, Vec<String>)>) {
    for n in nodes {
        let mut here = above.to_vec();
        here.push(str_of(n, "name").to_owned());
        out.push((n.clone(), here.clone()));
        walk(arr(n, "children"), &here, out);
    }
}

fn walked(tree: &[Value]) -> Vec<(Value, Vec<String>)> {
    let mut out = Vec::new();
    walk(tree, &[], &mut out);
    out
}

fn all_ids(tree: &[Value]) -> HashSet<String> {
    walked(tree)
        .into_iter()
        .map(|(n, _)| str_of(&n, "id").to_owned())
        .collect()
}

fn free_id(id: &str, taken: &mut HashSet<String>) -> String {
    let (mut out, mut k) = (id.to_owned(), 2);
    while taken.contains(&out) {
        out = format!("{id}-{k}");
        k += 1;
    }
    taken.insert(out.clone());
    out
}

/// Names for `incoming` that none of `taken` nor each other have (blocks' rule): `Ad (2)` …
pub fn unique_names(taken: &[String], incoming: &[String]) -> Vec<String> {
    let mut keys: HashSet<String> = taken.iter().map(|t| block_key(t)).collect();
    incoming
        .iter()
        .map(|name| {
            let (mut chosen, mut k) = (name.clone(), 2);
            while keys.contains(&block_key(&chosen)) {
                chosen = format!("{name} ({k})");
                k += 1;
            }
            keys.insert(block_key(&chosen));
            chosen
        })
        .collect()
}

fn strings_at(value: &Value, key: &str, out: &mut BTreeSet<String>) {
    match value {
        Value::Object(m) => {
            for (k, v) in m {
                match v {
                    Value::String(s) if k == key => {
                        out.insert(s.clone());
                    }
                    _ => strings_at(v, key, out),
                }
            }
        }
        Value::Array(list) => list.iter().for_each(|v| strings_at(v, key, out)),
        _ => {}
    }
}

/// The definitions these objects place, and those they place in turn.
fn placed_blocks(entities: &[Value], by_id: &HashMap<String, Value>) -> HashSet<String> {
    let mut out = HashSet::new();
    let mut todo: Vec<String> = entities
        .iter()
        .filter(|e| str_of(e, "kind") == "insert")
        .map(|e| str_of(e, "block").to_owned())
        .collect();
    while let Some(b) = todo.pop() {
        let Some(def) = by_id.get(&b) else { continue };
        if !out.insert(b) {
            continue;
        }
        todo.extend(
            arr(def, "entities")
                .iter()
                .filter(|e| str_of(e, "kind") == "insert")
                .map(|e| str_of(e, "block").to_owned()),
        );
    }
    out
}

fn blocks_by_id(doc: &Value) -> HashMap<String, Value> {
    arr(doc, "blocks")
        .iter()
        .map(|b| (str_of(b, "id").to_owned(), b.clone()))
        .collect()
}

fn to_json(doc: &DocumentSnapshotV2) -> Result<Value, String> {
    serde_json::to_value(doc).map_err(|e| format!("Çizim okunamadı: {e}."))
}

fn from_json(v: Value) -> Result<DocumentSnapshotV2, String> {
    serde_json::from_value(v).map_err(|e| format!("Alışverişin sonucu çizime uymuyor: {e}."))
}

// ── 1. The selection's drawing ────────────────────────────────────────

/// The selected objects (`uids`) as a drawing of their own named `name` (docs/adr/0193 §1).
pub fn selection(
    doc: &DocumentSnapshotV2,
    uids: &[EntityId],
    name: &str,
) -> Result<DocumentSnapshotV2, String> {
    let wanted: Vec<Value> = uids
        .iter()
        .map(|u| serde_json::to_value(u).unwrap_or(Value::Null))
        .collect();
    from_json(selection_json(&to_json(doc)?, &wanted, name))
}

/// [`selection`] over the contract's JSON (the cases' form).
pub fn selection_json(doc: &Value, uids: &[Value], name: &str) -> Value {
    let keep: HashSet<String> = uids
        .iter()
        .filter_map(|u| u.as_str())
        .map(str::to_owned)
        .collect();
    let all_uids = arr(doc, "uids");
    let kept_uids: Vec<Value> = all_uids
        .iter()
        .filter(|u| u.as_str().is_some_and(|u| keep.contains(u)))
        .cloned()
        .collect();
    let entities: Vec<Value> = arr(doc, "entities")
        .iter()
        .zip(all_uids)
        .filter(|(_, u)| u.as_str().is_some_and(|u| keep.contains(u)))
        .map(|(e, _)| e.clone())
        .collect();
    let by_id = blocks_by_id(doc);
    let blocks = placed_blocks(&entities, &by_id);
    let kept_blocks: Vec<Value> = arr(doc, "blocks")
        .iter()
        .filter(|b| blocks.contains(str_of(b, "id")))
        .cloned()
        .collect();
    let used_layers: HashSet<String> = entities
        .iter()
        .chain(kept_blocks.iter().flat_map(|b| arr(b, "entities")))
        .map(|e| str_of(e, "layerId").to_owned())
        .collect();
    fn prune(nodes: &[Value], used: &HashSet<String>) -> Vec<Value> {
        let mut out = Vec::new();
        for n in nodes {
            let kids = prune(arr(n, "children"), used);
            let keep = match str_of(n, "type") {
                "layer" => used.contains(str_of(n, "id")),
                _ => !kids.is_empty(),
            };
            if keep {
                let mut n = n.clone();
                n["children"] = Value::Array(kids);
                out.push(n);
            }
        }
        out
    }
    let tree = prune(arr(doc, "layers"), &used_layers);
    let kept_nodes = all_ids(&tree);
    let leaves: Vec<String> = walked(&tree)
        .into_iter()
        .filter(|(n, _)| str_of(n, "type") == "layer")
        .map(|(n, _)| str_of(&n, "id").to_owned())
        .collect();
    let active = str_of(doc, "activeLayer");
    let active = if leaves.iter().any(|l| l == active) {
        active.to_owned()
    } else {
        leaves.first().cloned().unwrap_or_default()
    };
    let in_keep = |v: &Value| v.as_str().is_some_and(|u| keep.contains(u));
    let out: Vec<Value> = entities
        .into_iter()
        .enumerate()
        .map(|(i, mut e)| {
            e["id"] = json!(i + 1);
            let kind = str_of(&e, "kind").to_owned();
            let Some(o) = e.as_object_mut() else {
                return e;
            };
            if kind == "text" && o.get("labelOf").is_some_and(|u| !in_keep(u)) {
                o.remove("labelOf");
                o.remove("labelScale");
            }
            if kind == "hatch"
                && let Some(a) = o.get("assoc")
            {
                let followed = std::iter::once(a.get("outer").cloned().unwrap_or(Value::Null))
                    .chain(arr(a, "islands").iter().cloned())
                    .chain(arr(a, "cutouts").iter().cloned());
                if !followed.into_iter().all(|u| in_keep(&u)) {
                    o.remove("assoc");
                }
            }
            if kind == "table"
                && let Some(src) = o.get("source").cloned()
                && src.get("objects").is_some()
            {
                let objs: Vec<Value> = arr(&src, "objects")
                    .iter()
                    .filter(|u| in_keep(u))
                    .cloned()
                    .collect();
                if objs.is_empty() {
                    o.remove("source");
                } else {
                    let mut src = src;
                    src["objects"] = Value::Array(objs);
                    o.insert("source".into(), src);
                }
            }
            e
        })
        .collect();
    // The library: what the kept objects, blocks' objects and layers use.
    let items = arr(&doc["styles"], "items");
    let mut symbols = BTreeSet::new();
    for e in out
        .iter()
        .chain(kept_blocks.iter().flat_map(|b| arr(b, "entities")))
    {
        if let Some(s) = e.get("symbol").and_then(Value::as_str) {
            symbols.insert(s.to_owned());
        }
    }
    let layer_styles: Vec<Value> = walked(&tree)
        .into_iter()
        .map(|(n, _)| n["style"].clone())
        .collect();
    for st in &layer_styles {
        strings_at(st, "ref", &mut symbols);
    }
    let mut assets = BTreeSet::new();
    for e in &out {
        if matches!(str_of(e, "kind"), "image" | "raster")
            && let Some(a) = e.get("asset").and_then(Value::as_str)
        {
            assets.insert(a.to_owned());
        }
    }
    for it in items {
        if str_of(it, "kind") == "symbol" && symbols.contains(str_of(it, "id")) {
            strings_at(it, "asset", &mut assets);
        }
    }
    for st in &layer_styles {
        strings_at(st, "asset", &mut assets);
    }
    let kept_items: Vec<Value> = items
        .iter()
        .filter(|it| match str_of(it, "kind") {
            "symbol" => symbols.contains(str_of(it, "id")),
            "asset" => assets.contains(str_of(it, "id")),
            _ => false,
        })
        .cloned()
        .collect();
    let mut settings = doc["settings"].clone();
    let states: Vec<Value> = arr(&settings, "layerStates")
        .iter()
        .filter_map(|st| {
            let nodes: Vec<Value> = arr(st, "nodes")
                .iter()
                .filter(|n| kept_nodes.contains(str_of(n, "node")))
                .cloned()
                .collect();
            (!nodes.is_empty()).then(|| {
                let mut st = st.clone();
                st["nodes"] = Value::Array(nodes);
                st
            })
        })
        .collect();
    if let Some(s) = settings.as_object_mut() {
        if states.is_empty() {
            s.remove("layerStates");
        } else {
            s.insert("layerStates".into(), Value::Array(states));
        }
    }
    let mut result = json!({
        "format": doc["format"],
        "version": 2,
        "name": name,
        "settings": settings,
        "origin": doc["origin"],
        "layers": tree,
        "activeLayer": active,
        "entities": out,
        "uids": kept_uids,
        "styles": { "items": kept_items, "categories": doc["styles"]["categories"] },
    });
    if !kept_blocks.is_empty() {
        result["blocks"] = Value::Array(kept_blocks);
    }
    result
}

// ── 2. Taking from another drawing ────────────────────────────────────

/// The node of `tree` at these names (folded), if any.
fn find_path<'a>(tree: &'a [Value], names: &[String]) -> Option<&'a Value> {
    let (mut nodes, mut found) = (tree, None);
    for name in names {
        let hit = nodes
            .iter()
            .find(|n| fold(str_of(n, "name")) == fold(name))?;
        found = Some(hit);
        nodes = arr(hit, "children");
    }
    found
}

/// Their node ids → their paths.
fn path_of(tree: &[Value]) -> HashMap<String, Vec<String>> {
    walked(tree)
        .into_iter()
        .map(|(n, here)| (str_of(&n, "id").to_owned(), here))
        .collect()
}

/// Our node at the path their node has: a layer only, unless `any` (a state names groups too).
fn our_node_for(
    ours: &Value,
    their_paths: &HashMap<String, Vec<String>>,
    their: &str,
    any: bool,
) -> Option<String> {
    let here = their_paths.get(their)?;
    let hit = find_path(arr(ours, "layers"), here)?;
    (any || str_of(hit, "type") == "layer").then(|| str_of(hit, "id").to_owned())
}

/// The layers of `theirs` at `paths` into `ours` (§2): missing paths made with their groups, a met layer's style replaced.
fn take_layers(
    ours: &mut Value,
    theirs: &Value,
    paths: &HashSet<String>,
    same: Same,
    taken: &mut HashSet<String>,
) {
    let all = walked(arr(theirs, "layers"));
    let by_path: HashMap<Vec<String>, Value> = all
        .iter()
        .map(|(n, here)| (here.clone(), n.clone()))
        .collect();
    for (n, here) in &all {
        if str_of(n, "type") != "layer" || !paths.contains(&here.join(" / ")) {
            continue;
        }
        let mut list = arr_mut(ours, "layers");
        for depth in 0..here.len() {
            let names = &here[..=depth];
            let source = &by_path[names];
            let last = depth == here.len() - 1;
            let at = list
                .iter()
                .position(|m| fold(str_of(m, "name")) == fold(&names[depth]));
            let Some(at) = at else {
                let mut made = source.clone();
                made["id"] = json!(free_id(str_of(source, "id"), taken));
                made["children"] = json!([]);
                list.push(made);
                let end = list.len() - 1;
                list = arr_mut(&mut list[end], "children");
                continue;
            };
            if str_of(&list[at], "type") != str_of(source, "type") {
                break;
            }
            if last && same == Same::Replace {
                list[at]["style"] = source["style"].clone();
            }
            list = arr_mut(&mut list[at], "children");
        }
    }
}

fn take_styles(ours: &mut Value, theirs: &Value, key: &str, names: &HashSet<String>, same: Same) {
    let their_list = arr(&theirs["settings"], key).to_vec();
    let mine = arr_mut(&mut ours["settings"], key);
    for s in their_list {
        if !names.contains(str_of(&s, "name")) {
            continue;
        }
        match mine
            .iter()
            .position(|m| fold(str_of(m, "name")) == fold(str_of(&s, "name")))
        {
            Some(at) => {
                if same == Same::Replace {
                    let id = mine[at]["id"].clone();
                    mine[at] = s.clone();
                    mine[at]["id"] = id;
                }
            }
            None => {
                let mut s = s;
                if mine.iter().any(|m| m["id"] == s["id"]) {
                    s["id"] = json!(uuid::Uuid::now_v7().to_string());
                }
                mine.push(s);
            }
        }
    }
    if mine.is_empty()
        && let Some(o) = ours["settings"].as_object_mut()
    {
        o.remove(key);
    }
}

fn take_items(ours: &mut Value, theirs: &Value, ids: &[String], same: Same) {
    let theirs_items = arr(&theirs["styles"], "items");
    let by_id: HashMap<&str, &Value> = theirs_items
        .iter()
        .map(|it| (str_of(it, "id"), it))
        .collect();
    let mut wanted: Vec<String> = Vec::new();
    for i in ids {
        let Some(it) = by_id.get(i.as_str()) else {
            continue;
        };
        if wanted.contains(i) {
            continue;
        }
        wanted.push(i.clone());
        if str_of(it, "kind") == "symbol" {
            let mut assets = BTreeSet::new();
            strings_at(it, "asset", &mut assets);
            for a in assets {
                if by_id.contains_key(a.as_str()) && !wanted.contains(&a) {
                    wanted.push(a);
                }
            }
        }
    }
    let order: Vec<&str> = theirs_items.iter().map(|it| str_of(it, "id")).collect();
    wanted.sort_by_key(|w| order.iter().position(|o| o == w));
    let mine = arr_mut(&mut ours["styles"], "items");
    for i in wanted {
        let item = by_id[i.as_str()].clone();
        match mine.iter().position(|m| str_of(m, "id") == i) {
            Some(at) => {
                if same == Same::Replace {
                    mine[at] = item;
                }
            }
            None => mine.push(item),
        }
    }
}

/// Their text and dimension style ids → ours by name (none: we have no such style).
fn style_ids(ours: &Value, theirs: &Value) -> HashMap<String, Option<String>> {
    let mut out = HashMap::new();
    for key in ["textStyles", "dimensionStyles"] {
        for s in arr(&theirs["settings"], key) {
            let hit = arr(&ours["settings"], key)
                .iter()
                .find(|m| fold(str_of(m, "name")) == fold(str_of(s, "name")))
                .map(|m| str_of(m, "id").to_owned());
            out.insert(str_of(s, "id").to_owned(), hit);
        }
    }
    out
}

/// One of their objects as one of ours: its layer, its block, its styles; no link, no tie (a block's object has no id).
fn map_object(
    e: &Value,
    layer_of: &dyn Fn(&str) -> String,
    block_of: &HashMap<String, String>,
    styles: &HashMap<String, Option<String>>,
) -> Value {
    let mut e = e.clone();
    e["layerId"] = json!(layer_of(str_of(&e, "layerId")));
    if str_of(&e, "kind") == "insert"
        && let Some(to) = block_of.get(str_of(&e, "block"))
    {
        e["block"] = json!(to);
    }
    let Some(o) = e.as_object_mut() else {
        return e;
    };
    for field in ["textStyle", "dimStyle"] {
        if let Some(id) = o.get(field).and_then(Value::as_str).map(str::to_owned) {
            match styles.get(&id).cloned().flatten() {
                Some(to) => {
                    o.insert(field.into(), json!(to));
                }
                None => {
                    o.remove(field);
                }
            }
        }
    }
    for field in ["labelOf", "labelScale", "assoc"] {
        o.remove(field);
    }
    e
}

fn take_blocks(
    ours: &mut Value,
    theirs: &Value,
    names: &HashSet<String>,
    same: Same,
    their_paths: &HashMap<String, Vec<String>>,
) {
    let by_id = blocks_by_id(theirs);
    let picked: Vec<Value> = arr(theirs, "blocks")
        .iter()
        .filter(|b| names.contains(str_of(b, "name")))
        .map(|b| json!({ "kind": "insert", "block": b["id"] }))
        .collect();
    let wanted = placed_blocks(&picked, &by_id);
    let theirs_blocks: Vec<Value> = arr(theirs, "blocks")
        .iter()
        .filter(|b| wanted.contains(str_of(b, "id")))
        .cloned()
        .collect();
    let mut block_of = HashMap::new();
    for b in &theirs_blocks {
        let hit = arr(ours, "blocks")
            .iter()
            .find(|m| block_key(str_of(m, "name")) == block_key(str_of(b, "name")))
            .map(|m| str_of(m, "id").to_owned());
        block_of.insert(
            str_of(b, "id").to_owned(),
            hit.unwrap_or_else(|| str_of(b, "id").to_owned()),
        );
    }
    // The symbols their objects draw with, when we have none such.
    let have: HashSet<String> = arr(&ours["styles"], "items")
        .iter()
        .map(|it| str_of(it, "id").to_owned())
        .collect();
    let symbols: Vec<String> = theirs_blocks
        .iter()
        .flat_map(|b| arr(b, "entities"))
        .filter_map(|e| e.get("symbol").and_then(Value::as_str))
        .filter(|s| !have.contains(*s))
        .map(str::to_owned)
        .collect();
    take_items(ours, theirs, &symbols, Same::Skip);
    let styles = style_ids(ours, theirs);
    let snapshot = ours.clone();
    let layer_of = |i: &str| our_node_for(&snapshot, their_paths, i, false).unwrap_or_default();
    let mine = arr_mut(ours, "blocks");
    for b in theirs_blocks {
        let entities: Vec<Value> = arr(&b, "entities")
            .iter()
            .map(|e| map_object(e, &layer_of, &block_of, &styles))
            .collect();
        let mut defined = b.clone();
        defined["id"] = json!(block_of[str_of(&b, "id")]);
        defined["entities"] = Value::Array(entities);
        match mine
            .iter()
            .position(|m| block_key(str_of(m, "name")) == block_key(str_of(&b, "name")))
        {
            Some(at) => {
                if same == Same::Replace {
                    defined["name"] = mine[at]["name"].clone();
                    mine[at] = defined;
                }
            }
            None => mine.push(defined),
        }
    }
    if mine.is_empty()
        && let Some(o) = ours.as_object_mut()
    {
        o.remove("blocks");
    }
}

fn take_states(
    ours: &mut Value,
    theirs: &Value,
    names: &HashSet<String>,
    same: Same,
    their_paths: &HashMap<String, Vec<String>>,
) {
    let snapshot = ours.clone();
    let mut taken: HashSet<String> = arr(&ours["settings"], "layerStates")
        .iter()
        .map(|s| str_of(s, "id").to_owned())
        .collect();
    let mine = arr_mut(&mut ours["settings"], "layerStates");
    for s in arr(&theirs["settings"], "layerStates") {
        if !names.contains(str_of(s, "name")) {
            continue;
        }
        let nodes: Vec<Value> = arr(s, "nodes")
            .iter()
            .filter_map(|n| {
                let to = our_node_for(&snapshot, their_paths, str_of(n, "node"), true)?;
                let mut n = n.clone();
                n["node"] = json!(to);
                Some(n)
            })
            .collect();
        match mine
            .iter()
            .position(|m| fold(str_of(m, "name")) == fold(str_of(s, "name")))
        {
            Some(at) => {
                if same == Same::Replace {
                    mine[at]["nodes"] = Value::Array(nodes);
                }
            }
            None => {
                let mut s = s.clone();
                s["id"] = json!(free_id(str_of(&s, "id"), &mut taken));
                s["nodes"] = Value::Array(nodes);
                mine.push(s);
            }
        }
    }
    if mine.is_empty()
        && let Some(o) = ours["settings"].as_object_mut()
    {
        o.remove("layerStates");
    }
}

/// What `picks` names of `theirs` taken into `ours` (docs/adr/0193 §2).
pub fn take(
    ours: &DocumentSnapshotV2,
    theirs: &DocumentSnapshotV2,
    picks: &Picks,
    same: Same,
) -> Result<DocumentSnapshotV2, String> {
    from_json(take_json(&to_json(ours)?, &to_json(theirs)?, picks, same))
}

/// [`take`] over the contract's JSON (the cases' form).
pub fn take_json(ours: &Value, theirs: &Value, picks: &Picks, same: Same) -> Value {
    let mut ours = ours.clone();
    if picks.settings {
        for key in TAKEN_SETTINGS {
            match theirs["settings"].get(key) {
                Some(v) => ours["settings"][key] = v.clone(),
                None => {
                    if let Some(o) = ours["settings"].as_object_mut() {
                        o.remove(key);
                    }
                }
            }
        }
    }
    let mut taken = all_ids(arr(&ours, "layers"));
    let paths: HashSet<String> = picks.layers.iter().cloned().collect();
    take_layers(&mut ours, theirs, &paths, same, &mut taken);
    let their_paths = path_of(arr(theirs, "layers"));
    take_styles(
        &mut ours,
        theirs,
        "textStyles",
        &picks.text_styles.iter().cloned().collect(),
        same,
    );
    take_styles(
        &mut ours,
        theirs,
        "dimensionStyles",
        &picks.dimension_styles.iter().cloned().collect(),
        same,
    );
    take_items(&mut ours, theirs, &picks.library, same);
    take_blocks(
        &mut ours,
        theirs,
        &picks.blocks.iter().cloned().collect(),
        same,
        &their_paths,
    );
    take_states(
        &mut ours,
        theirs,
        &picks.layer_states.iter().cloned().collect(),
        same,
        &their_paths,
    );
    ours
}

// ── 3. A drawing as a block ───────────────────────────────────────────

/// The lower left of these objects' extent, each by its own box as the
/// core measures it (an insert by its point); none for no object.
pub fn extent_corner(entities: &[Value]) -> Option<(f64, f64)> {
    let mut corner: Option<(f64, f64)> = None;
    for e in entities {
        let Ok(json) = Json::parse(&e.to_string()) else {
            continue;
        };
        let Ok(core) = CoreEntity::from_json(&json) else {
            continue;
        };
        let b = entity_bounds(&core.shape);
        if !(b.min_x.is_finite() && b.min_y.is_finite()) {
            continue;
        }
        corner = Some(match corner {
            None => (b.min_x, b.min_y),
            Some((x, y)) => (x.min(b.min_x), y.min(b.min_y)),
        });
    }
    corner
}

/// What [`file_block`] made: the drawing with it, the new block's id, and
/// what was left out or brought in, for the message.
#[derive(Clone, Debug, PartialEq)]
pub struct FileBlock {
    pub drawing: DocumentSnapshotV2,
    pub block: BlockId,
    pub name: String,
    pub images: usize,
    pub tables: usize,
    pub objects: usize,
    pub layers: usize,
    pub nested: usize,
}

/// `theirs` as one block of `ours` named after `file` (docs/adr/0193 §3);
/// `id` the new block's. Refused when it has nothing a block may hold.
pub fn file_block(
    ours: &DocumentSnapshotV2,
    theirs: &DocumentSnapshotV2,
    file: &str,
    id: BlockId,
) -> Result<FileBlock, String> {
    let ours_json = to_json(ours)?;
    let theirs_json = to_json(theirs)?;
    let layers_before = all_ids(arr(&ours_json, "layers")).len();
    let blocks_before = arr(&ours_json, "blocks").len();
    let (mut drawing, left) = file_block_json(&ours_json, &theirs_json, file).ok_or_else(|| {
        format!("“{file}” içinde blok yapılacak nesne yok; resimler ve tablolar bloğa konamaz.")
    })?;
    let made = arr_mut(&mut drawing, "blocks");
    let name = made
        .last()
        .map(|b| str_of(b, "name").to_owned())
        .unwrap_or_default();
    let objects = made.last().map_or(0, |b| arr(b, "entities").len());
    if let Some(b) = made.last_mut() {
        b["id"] = serde_json::to_value(id).map_err(|e| e.to_string())?;
    }
    let layers = all_ids(arr(&drawing, "layers")).len() - layers_before;
    let nested = arr(&drawing, "blocks").len() - blocks_before - 1;
    Ok(FileBlock {
        drawing: from_json(drawing)?,
        block: id,
        name,
        images: left.0,
        tables: left.1,
        objects,
        layers,
        nested,
    })
}

/// [`file_block`] over the contract's JSON (the cases' form): the drawing
/// with the block last (its id `$new`) and the pictures and tables left
/// out; none when nothing is left.
pub fn file_block_json(
    ours: &Value,
    theirs: &Value,
    file: &str,
) -> Option<(Value, (usize, usize))> {
    let mut ours = ours.clone();
    let all = arr(theirs, "entities");
    // A block holds no picture, raster (docs/adr/0204 §9) or table: they are left out and counted with the pictures.
    let kept: Vec<Value> = all
        .iter()
        .filter(|e| !matches!(str_of(e, "kind"), "image" | "raster" | "table"))
        .cloned()
        .collect();
    let left = (
        all.iter()
            .filter(|e| matches!(str_of(e, "kind"), "image" | "raster"))
            .count(),
        all.iter().filter(|e| str_of(e, "kind") == "table").count(),
    );
    let (bx, by) = extent_corner(&kept)?;
    // Layers by path, the missing made.
    let their_paths = path_of(arr(theirs, "layers"));
    let by_id = blocks_by_id(theirs);
    let nested = placed_blocks(&kept, &by_id);
    let nested_blocks: Vec<Value> = arr(theirs, "blocks")
        .iter()
        .filter(|b| nested.contains(str_of(b, "id")))
        .cloned()
        .collect();
    let mut wanted = HashSet::new();
    for e in kept
        .iter()
        .chain(nested_blocks.iter().flat_map(|b| arr(b, "entities")))
    {
        if let Some(here) = their_paths.get(str_of(e, "layerId")) {
            wanted.insert(here.join(" / "));
        }
    }
    let mut taken = all_ids(arr(&ours, "layers"));
    take_layers(&mut ours, theirs, &wanted, Same::Skip, &mut taken);
    // Styles by name, the missing added (the import's rule, docs/adr/0183 §7).
    let mut styles: HashMap<String, Option<String>> = HashMap::new();
    for (key, field) in [("textStyles", "textStyle"), ("dimensionStyles", "dimStyle")] {
        let used: HashSet<String> = kept
            .iter()
            .chain(nested_blocks.iter().flat_map(|b| arr(b, "entities")))
            .filter_map(|e| e.get(field).and_then(Value::as_str).map(str::to_owned))
            .collect();
        let their_list = arr(&theirs["settings"], key).to_vec();
        let mine = arr_mut(&mut ours["settings"], key);
        for s in their_list {
            if !used.contains(str_of(&s, "id")) {
                continue;
            }
            let hit = mine
                .iter()
                .find(|m| fold(str_of(m, "name")) == fold(str_of(&s, "name")))
                .map(|m| str_of(m, "id").to_owned());
            match hit {
                Some(to) => {
                    styles.insert(str_of(&s, "id").to_owned(), Some(to));
                }
                None => {
                    let mut s = s.clone();
                    if mine.iter().any(|m| m["id"] == s["id"]) {
                        s["id"] = json!(uuid::Uuid::now_v7().to_string());
                    }
                    styles.insert(
                        str_of(&s, "id").to_owned(),
                        Some(str_of(&s, "id").to_owned()),
                    );
                    mine.push(s);
                }
            }
        }
        if mine.is_empty()
            && let Some(o) = ours["settings"].as_object_mut()
        {
            o.remove(key);
        }
    }
    // The library items the objects draw with.
    let have: HashSet<String> = arr(&ours["styles"], "items")
        .iter()
        .map(|it| str_of(it, "id").to_owned())
        .collect();
    let symbols: Vec<String> = kept
        .iter()
        .chain(nested_blocks.iter().flat_map(|b| arr(b, "entities")))
        .filter_map(|e| e.get("symbol").and_then(Value::as_str))
        .filter(|s| !have.contains(*s))
        .map(str::to_owned)
        .collect();
    take_items(&mut ours, theirs, &symbols, Same::Skip);
    // The blocks: the nested ones first (their order), then the drawing's own.
    let taken_names: Vec<String> = arr(&ours, "blocks")
        .iter()
        .map(|b| str_of(b, "name").to_owned())
        .collect();
    let mut incoming: Vec<String> = nested_blocks
        .iter()
        .map(|b| str_of(b, "name").to_owned())
        .collect();
    incoming.push(file.to_owned());
    let names = unique_names(&taken_names, &incoming);
    let block_of: HashMap<String, String> = nested_blocks
        .iter()
        .map(|b| (str_of(b, "id").to_owned(), str_of(b, "id").to_owned()))
        .collect();
    let snapshot = ours.clone();
    let layer_of = |i: &str| our_node_for(&snapshot, &their_paths, i, false).unwrap_or_default();
    let mine = arr_mut(&mut ours, "blocks");
    for (b, name) in nested_blocks.iter().zip(&names) {
        let mut b = b.clone();
        b["name"] = json!(name);
        let entities: Vec<Value> = arr(&b, "entities")
            .iter()
            .map(|e| map_object(e, &layer_of, &block_of, &styles))
            .collect();
        b["entities"] = Value::Array(entities);
        mine.push(b);
    }
    let entities: Vec<Value> = kept
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let mut m = map_object(e, &layer_of, &block_of, &styles);
            m["id"] = json!(i + 1);
            m
        })
        .collect();
    let mut block = Map::new();
    block.insert("id".into(), json!("$new"));
    block.insert(
        "name".into(),
        json!(names.last().cloned().unwrap_or_default()),
    );
    block.insert("base".into(), json!({ "x": bx, "y": by }));
    block.insert("entities".into(), Value::Array(entities));
    mine.push(Value::Object(block));
    Some((ours, left))
}

// ── 4. A layer with its objects ───────────────────────────────────────

/// What [`layer_take`] gives: the drawing with the layer (its objects as
/// they were), the objects to add, and how many layers or groups and blocks
/// it made, for the message.
#[derive(Clone, Debug, PartialEq)]
pub struct LayerTake {
    pub drawing: DocumentSnapshotV2,
    pub objects: Vec<kentos_contracts::Entity>,
    pub layers: usize,
    pub blocks: usize,
}

/// Their layer at `path` with its objects into `ours` (docs/adr/0199 §7).
/// Refused when `path` is not a layer of theirs or our node at it is not a
/// layer.
pub fn layer_take(
    ours: &DocumentSnapshotV2,
    theirs: &DocumentSnapshotV2,
    path: &str,
) -> Result<LayerTake, String> {
    let ours_json = to_json(ours)?;
    let theirs_json = to_json(theirs)?;
    let (drawing, objects) = layer_take_json(&ours_json, &theirs_json, path).ok_or_else(|| {
        format!("“{path}” alınamadı: kaynakta böyle bir katman yok ya da bu çizimde aynı yolda katman olmayan bir düğüm var.")
    })?;
    let layers = all_ids(arr(&drawing, "layers")).len() - all_ids(arr(&ours_json, "layers")).len();
    let blocks = arr(&drawing, "blocks").len() - arr(&ours_json, "blocks").len();
    let objects = objects
        .into_iter()
        .map(|o| serde_json::from_value(o).map_err(|e| e.to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(LayerTake {
        drawing: from_json(drawing)?,
        objects,
        layers,
        blocks,
    })
}

/// [`layer_take`] over the contract's JSON (the cases' form): the new
/// drawing and the objects to add, numbered from 1; none when refused.
pub fn layer_take_json(ours: &Value, theirs: &Value, path: &str) -> Option<(Value, Vec<Value>)> {
    let mut ours = ours.clone();
    let target = walked(arr(theirs, "layers"))
        .into_iter()
        .find(|(n, here)| str_of(n, "type") == "layer" && here.join(" / ") == path)
        .map(|(n, _)| str_of(&n, "id").to_owned())?;
    let objects: Vec<Value> = arr(theirs, "entities")
        .iter()
        .filter(|e| str_of(e, "layerId") == target)
        .cloned()
        .collect();
    let nested = placed_blocks(&objects, &blocks_by_id(theirs));
    let nested_blocks: Vec<Value> = arr(theirs, "blocks")
        .iter()
        .filter(|b| nested.contains(str_of(b, "id")))
        .cloned()
        .collect();
    let pieces: Vec<&Value> = objects
        .iter()
        .chain(nested_blocks.iter().flat_map(|b| arr(b, "entities")))
        .collect();
    let their_paths = path_of(arr(theirs, "layers"));
    let mut wanted: HashSet<String> = HashSet::from([path.to_owned()]);
    for e in &pieces {
        if let Some(here) = their_paths.get(str_of(e, "layerId")) {
            wanted.insert(here.join(" / "));
        }
    }
    let before = all_ids(arr(&ours, "layers"));
    let mut taken = before.clone();
    take_layers(&mut ours, theirs, &wanted, Same::Skip, &mut taken);
    our_node_for(&ours, &their_paths, &target, false)?;
    let made_styles: Vec<Value> = walked(arr(&ours, "layers"))
        .into_iter()
        .filter(|(n, _)| !before.contains(str_of(n, "id")))
        .map(|(n, _)| n["style"].clone())
        .collect();
    // Styles by name, the missing added (the import's rule, docs/adr/0183 §7).
    let mut styles: HashMap<String, Option<String>> = HashMap::new();
    for (key, field) in [("textStyles", "textStyle"), ("dimensionStyles", "dimStyle")] {
        let used: HashSet<&str> = pieces
            .iter()
            .filter_map(|e| e.get(field).and_then(Value::as_str))
            .collect();
        let their_list = arr(&theirs["settings"], key).to_vec();
        let mine = arr_mut(&mut ours["settings"], key);
        for s in their_list {
            if !used.contains(str_of(&s, "id")) {
                continue;
            }
            let hit = mine
                .iter()
                .find(|m| fold(str_of(m, "name")) == fold(str_of(&s, "name")))
                .map(|m| str_of(m, "id").to_owned());
            let to = match hit {
                Some(to) => to,
                None => {
                    let mut s = s.clone();
                    if mine.iter().any(|m| m["id"] == s["id"]) {
                        s["id"] = json!(uuid::Uuid::now_v7().to_string());
                    }
                    let to = str_of(&s, "id").to_owned();
                    mine.push(s);
                    to
                }
            };
            styles.insert(str_of(&s, "id").to_owned(), Some(to));
        }
        if mine.is_empty()
            && let Some(o) = ours["settings"].as_object_mut()
        {
            o.remove(key);
        }
    }
    // The library items the objects, their pictures and the made layers draw
    // with, when we have none such.
    let have: HashSet<String> = arr(&ours["styles"], "items")
        .iter()
        .map(|it| str_of(it, "id").to_owned())
        .collect();
    let mut items: Vec<String> = Vec::new();
    for e in &objects {
        if let Some(s) = e.get("symbol").and_then(Value::as_str) {
            items.push(s.to_owned());
        }
        if matches!(str_of(e, "kind"), "image" | "raster")
            && let Some(a) = e.get("asset").and_then(Value::as_str)
        {
            items.push(a.to_owned());
        }
    }
    for st in &made_styles {
        for key in ["ref", "asset"] {
            let mut found = BTreeSet::new();
            strings_at(st, key, &mut found);
            items.extend(found);
        }
    }
    items.retain(|i| !have.contains(i));
    take_items(&mut ours, theirs, &items, Same::Skip);
    // The blocks by name: a met one is ours, the others come with what they place.
    let names: HashSet<String> = nested_blocks
        .iter()
        .map(|b| str_of(b, "name").to_owned())
        .collect();
    take_blocks(&mut ours, theirs, &names, Same::Skip, &their_paths);
    let block_of: HashMap<String, String> = nested_blocks
        .iter()
        .map(|b| {
            let ours_id = arr(&ours, "blocks")
                .iter()
                .find(|m| block_key(str_of(m, "name")) == block_key(str_of(b, "name")))
                .map_or_else(
                    || str_of(b, "id").to_owned(),
                    |m| str_of(m, "id").to_owned(),
                );
            (str_of(b, "id").to_owned(), ours_id)
        })
        .collect();
    let snapshot = ours.clone();
    let layer_of = |i: &str| our_node_for(&snapshot, &their_paths, i, false).unwrap_or_default();
    let out = objects
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let mut m = map_object(e, &layer_of, &block_of, &styles);
            if let Some(o) = m.as_object_mut() {
                o.remove("source");
            }
            m["id"] = json!(i + 1);
            m
        })
        .collect();
    Some((ours, out))
}
