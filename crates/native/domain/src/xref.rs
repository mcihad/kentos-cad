//! An external reference's content (docs/adr/0348 §1, §2): a source drawing as one block definition of ours, over the
//! contract's JSON as the drawing exchange's rules are (`exchange.rs`).
//!
//! - Its layers come under the group “Dış başvuru: ad” with their paths and looks; a layer we already have at a path
//!   keeps its own look (the user's changes stay through a reload).
//! - Its objects but pictures, rasters and tables are the definition's, in the source's order, numbered from 1 as a
//!   file numbers them, at their own coordinates (base 0, 0); the source's persistent ids are kept beside them
//!   (`XrefSource.objects`), so a reload knows each object again and keeps its local id: the ties into its pieces
//!   follow them (a block path is followed by local id, docs/adr/0315 §1).
//! - The blocks they place come along named `ad|blok`, their ids from ours and theirs (the same on every reload). The
//!   source's own attached references come as they are in it, brought by this one (`parent`); its overlaid ones and
//!   their inserts do not.

use std::collections::{HashMap, HashSet};

use kentos_contracts::{BlockId, DocumentSnapshotV2, XrefKind, XrefSource, xref_group_name};
use serde_json::{Map, Value, json};

use crate::exchange::{
    Same, all_ids, arr, arr_mut, blocks_by_id, fold, from_json, map_object, our_node_for, path_of, placed_blocks,
    str_of, take_items, take_layers, to_json, unique_names, walked,
};

/// What [`load`] made: the drawing with the reference's definition (and those it brought), and what the answer says.
#[derive(Clone, Debug, PartialEq)]
pub struct XrefLoad {
    pub drawing: DocumentSnapshotV2,
    /// The objects its definition holds.
    pub objects: usize,
    /// The pictures, rasters and tables left out.
    pub left: usize,
    /// The layers made, by name.
    pub layers: Vec<String>,
    /// The definitions it brought (blocks and nested references), by id and name.
    pub nested: Vec<(BlockId, String)>,
    /// The layer group's id.
    pub group: Option<String>,
}

/// A brought block's id: ours and theirs mixed, the same every time this reference is loaded.
fn nested_id(ours: &str, theirs: &str) -> String {
    let a = uuid::Uuid::parse_str(ours).map(|u| *u.as_bytes()).unwrap_or_default();
    let b = uuid::Uuid::parse_str(theirs).map(|u| *u.as_bytes()).unwrap_or_default();
    let mut c = [0u8; 16];
    for i in 0..16 {
        c[i] = a[i] ^ b[i].rotate_left(3) ^ 0x5a;
    }
    // A version-8 UUID (RFC 9562's free form), so its text reads as one.
    c[6] = (c[6] & 0x0f) | 0x80;
    c[8] = (c[8] & 0x3f) | 0x80;
    uuid::Uuid::from_bytes(c).to_string()
}

/// The overlaid references of a drawing (JSON) and the blocks only they place: none of them comes along.
fn overlaid(theirs: &Value) -> HashSet<String> {
    arr(theirs, "blocks")
        .iter()
        .filter(|b| b.get("xref").and_then(|x| x.get("kind")).and_then(Value::as_str) == Some("overlay"))
        .map(|b| str_of(b, "id").to_owned())
        .collect()
}

/// `theirs` as the reference `name` of `ours`, its definition's id `id` and its source `source` (path, relative path,
/// kind, digest; its objects and group are written here). A definition of ours with this id, and those it brought,
/// are replaced (a reload). Refused when nothing in `theirs` may go in a block.
pub fn load(
    ours: &DocumentSnapshotV2,
    theirs: &DocumentSnapshotV2,
    name: &str,
    id: BlockId,
    source: XrefSource,
) -> Result<XrefLoad, String> {
    let ours_json = to_json(ours)?;
    let theirs_json = to_json(theirs)?;
    let layers_before: HashSet<String> = all_ids(arr(&ours_json, "layers"));
    let (drawing, info) = load_json(&ours_json, &theirs_json, name, &id_text(id), source)
        .ok_or_else(|| format!("“{name}” içinde dış başvuruya konacak nesne yok; resimler, rasterler ve tablolar bloğa konamaz."))?;
    let layers = walked(arr(&drawing, "layers"))
        .into_iter()
        .filter(|(n, _)| str_of(n, "type") == "layer" && !layers_before.contains(str_of(n, "id")))
        .map(|(n, _)| str_of(&n, "name").to_owned())
        .collect();
    let nested = info
        .nested
        .iter()
        .filter_map(|(i, n)| Some((BlockId(*uuid::Uuid::parse_str(i).ok()?.as_bytes()), n.clone())))
        .collect();
    Ok(XrefLoad {
        drawing: from_json(drawing)?,
        objects: info.objects,
        left: info.left,
        layers,
        nested,
        group: info.group,
    })
}

fn id_text(id: BlockId) -> String {
    uuid::Uuid::from_bytes(id.0).to_string()
}

/// What [`load_json`] says beside the drawing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LoadInfo {
    pub objects: usize,
    pub left: usize,
    pub nested: Vec<(String, String)>,
    pub group: Option<String>,
}

/// [`load`] over the contract's JSON (the cases' form); none when nothing may go in the block.
pub fn load_json(ours: &Value, theirs: &Value, name: &str, id: &str, source: XrefSource) -> Option<(Value, LoadInfo)> {
    let mut ours = ours.clone();
    // A reload: the definition and what it brought go (a block another object of ours places too stays); the new ones
    // take their places by id.
    let ours_by_id = blocks_by_id(&ours);
    // A reload keeps each source object's local id: a tie into a piece follows it there (docs/adr/0315 §1).
    let old_ids: HashMap<String, u64> = ours_by_id
        .get(id)
        .map(|old| {
            let objects = old.get("xref").map(|x| arr(x, "objects")).unwrap_or_default();
            objects
                .iter()
                .zip(arr(old, "entities"))
                .filter_map(|(u, e)| Some((u.as_str()?.to_owned(), e.get("id")?.as_u64()?)))
                .collect()
        })
        .unwrap_or_default();
    let mut old_nested: HashSet<String> = ours_by_id
        .get(id)
        .map(|old| placed_blocks(arr(old, "entities"), &ours_by_id))
        .unwrap_or_default();
    old_nested.extend(arr(&ours, "blocks").iter().filter(|b| brought_by(b, id)).map(|b| str_of(b, "id").to_owned()));
    let outside: Vec<Value> = arr(&ours, "entities")
        .iter()
        .filter(|e| !(str_of(e, "kind") == "insert" && str_of(e, "block") == id))
        .cloned()
        .collect();
    let still_used = placed_blocks(&outside, &ours_by_id);
    old_nested.retain(|b| !still_used.contains(b));
    // A block the reference brought before keeps its id when it comes again under the same name: its inserts and the
    // ties into its pieces stay with it.
    let old_ids_by_name: HashMap<String, String> = old_nested
        .iter()
        .filter_map(|b| ours_by_id.get(b).map(|def| (fold(str_of(def, "name")), b.clone())))
        .collect();
    arr_mut(&mut ours, "blocks").retain(|b| str_of(b, "id") != id && !old_nested.contains(str_of(b, "id")));
    let skip = overlaid(theirs);
    let all = arr(theirs, "entities");
    let uids = arr(theirs, "uids");
    let mut kept: Vec<Value> = Vec::new();
    let mut kept_uids: Vec<Value> = Vec::new();
    let mut left = 0;
    for (k, e) in all.iter().enumerate() {
        match str_of(e, "kind") {
            "image" | "raster" | "table" => left += 1,
            "insert" if skip.contains(str_of(e, "block")) => {}
            _ => {
                kept.push(e.clone());
                kept_uids.push(uids.get(k).cloned().unwrap_or(Value::Null));
            }
        }
    }
    if kept.is_empty() {
        return None;
    }
    // Their layers under the reference's group: their tree wrapped, so every path starts with the group's name.
    let group_name = xref_group_name(name);
    let mut wrapped = theirs.clone();
    let mut group = json!({
        "id": "xref-group", "name": group_name, "type": "group", "visible": true, "locked": false, "expanded": true,
        "style": { "color": "fg", "lineType": "continuous", "lineWeight": 0.25 },
        "children": arr(theirs, "layers"),
    });
    if let Some(o) = group.as_object_mut()
        && arr(theirs, "layers").is_empty()
    {
        o.insert("children".into(), json!([]));
    }
    wrapped["layers"] = json!([group]);
    let their_paths = path_of(arr(&wrapped, "layers"));
    let wanted: HashSet<String> = walked(arr(&wrapped, "layers"))
        .into_iter()
        .filter(|(n, _)| str_of(n, "type") == "layer")
        .map(|(_, here)| here.join(" / "))
        .collect();
    let mut taken = all_ids(arr(&ours, "layers"));
    take_layers(&mut ours, &wrapped, &wanted, Same::Skip, &mut taken);
    let group_id = arr(&ours, "layers")
        .iter()
        .find(|n| fold(str_of(n, "name")) == fold(&group_name))
        .map(|n| str_of(n, "id").to_owned());
    // The blocks they place: ordinary ones renamed `name|theirs`, attached references brought as they are.
    let by_id = blocks_by_id(theirs);
    let mut placed = placed_blocks(&kept, &by_id);
    placed.retain(|b| !skip.contains(b));
    let brought: Vec<Value> = arr(theirs, "blocks")
        .iter()
        .filter(|b| placed.contains(str_of(b, "id")))
        .cloned()
        .collect();
    let taken_names: Vec<String> = arr(&ours, "blocks").iter().map(|b| str_of(b, "name").to_owned()).collect();
    let incoming: Vec<String> = brought
        .iter()
        .map(|b| format!("{name}|{}", str_of(b, "name")))
        .collect();
    let names = unique_names(&taken_names, &incoming);
    let block_of: HashMap<String, String> = brought
        .iter()
        .zip(&names)
        .map(|(b, n)| {
            let new = old_ids_by_name.get(&fold(n)).cloned().unwrap_or_else(|| nested_id(id, str_of(b, "id")));
            (str_of(b, "id").to_owned(), new)
        })
        .collect();
    // Styles by name: theirs mapped to ours of the same name; a missing one dropped from the object (the reference
    // draws with the drawing's styles, as AutoCAD's binds its own only on Bind).
    let styles: HashMap<String, Option<String>> = {
        let mut out = HashMap::new();
        for key in ["textStyles", "dimensionStyles"] {
            for st in arr(&theirs["settings"], key) {
                let hit = arr(&ours["settings"], key)
                    .iter()
                    .find(|m| fold(str_of(m, "name")) == fold(str_of(st, "name")))
                    .map(|m| str_of(m, "id").to_owned());
                out.insert(str_of(st, "id").to_owned(), hit);
            }
        }
        out
    };
    // The library items the objects draw with.
    let have: HashSet<String> = arr(&ours["styles"], "items").iter().map(|it| str_of(it, "id").to_owned()).collect();
    let symbols: Vec<String> = kept
        .iter()
        .chain(brought.iter().flat_map(|b| arr(b, "entities")))
        .filter_map(|e| e.get("symbol").and_then(Value::as_str))
        .filter(|s| !have.contains(*s))
        .map(str::to_owned)
        .collect();
    take_items(&mut ours, theirs, &symbols, Same::Skip);
    let snapshot = ours.clone();
    let layer_of = |i: &str| our_node_for(&snapshot, &their_paths, i, false).unwrap_or_default();
    let group_of = |i: &str| our_node_for(&snapshot, &their_paths, i, true);
    let mut nested = Vec::new();
    let mine = arr_mut(&mut ours, "blocks");
    // A brought block kept because another object places it is replaced by its new content.
    let coming: HashSet<String> = block_of.values().cloned().collect();
    mine.retain(|b| !coming.contains(str_of(b, "id")));
    for (b, bname) in brought.iter().zip(&names) {
        let mut b = b.clone();
        let new_id = block_of[str_of(&b, "id")].clone();
        b["id"] = json!(new_id);
        b["name"] = json!(bname);
        let entities: Vec<Value> = arr(&b, "entities")
            .iter()
            .filter(|e| !(str_of(e, "kind") == "insert" && skip.contains(str_of(e, "block"))))
            .map(|e| map_object(e, &layer_of, &block_of, &styles))
            .collect();
        b["entities"] = Value::Array(entities);
        // A nested reference keeps its source; this one brought it; its group is ours now.
        if let Some(x) = b.get_mut("xref").and_then(Value::as_object_mut) {
            x.insert("parent".into(), json!(id));
            match x.get("group").and_then(Value::as_str).and_then(group_of) {
                Some(g) => {
                    x.insert("group".into(), json!(g));
                }
                None => {
                    x.remove("group");
                }
            }
        }
        nested.push((new_id, bname.clone()));
        mine.push(b);
    }
    // Numbered from 1 as a file numbers them; on a reload an object the definition had keeps its id and a new one
    // takes the next after the highest.
    let mut next = old_ids.values().copied().max().unwrap_or(0);
    let entities: Vec<Value> = kept
        .iter()
        .zip(&kept_uids)
        .map(|(e, u)| {
            let mut m = map_object(e, &layer_of, &block_of, &styles);
            let local = u.as_str().and_then(|u| old_ids.get(u).copied()).unwrap_or_else(|| {
                next += 1;
                next
            });
            m["id"] = json!(local);
            m
        })
        .collect();
    let objects = entities.len();
    let mut x = serde_json::to_value(&source).unwrap_or_else(|_| json!({}));
    if let Some(o) = x.as_object_mut() {
        o.remove("unloaded");
        o.remove("parent");
        match &group_id {
            Some(g) => {
                o.insert("group".into(), json!(g));
            }
            None => {
                o.remove("group");
            }
        }
        if kept_uids.iter().all(Value::is_string) {
            o.insert("objects".into(), Value::Array(kept_uids));
        } else {
            o.remove("objects");
        }
    }
    let mut block = Map::new();
    block.insert("id".into(), json!(id));
    block.insert("name".into(), json!(name));
    block.insert("base".into(), json!({ "x": 0.0, "y": 0.0 }));
    block.insert("entities".into(), Value::Array(entities));
    let unit = theirs["settings"].get("drawingUnit").and_then(Value::as_str).unwrap_or("m");
    block.insert("units".into(), json!(unit));
    block.insert("xref".into(), x);
    mine.push(Value::Object(block));
    Some((ours, LoadInfo { objects, left, nested, group: group_id }))
}

/// A definition one reference brought (its `parent`), or a block named `ref|…` it brought.
fn brought_by(b: &Value, id: &str) -> bool {
    b.get("xref").and_then(|x| x.get("parent")).and_then(Value::as_str) == Some(id)
}

/// The source of a reference's definition, if it is one.
pub fn source_of(doc: &crate::Document, block: BlockId) -> Option<XrefSource> {
    doc.block(block).and_then(|b| b.xref.clone())
}

/// Every reference of the drawing, in its block order: the definition's id, name and source.
pub fn references(doc: &crate::Document) -> Vec<(BlockId, String, XrefSource)> {
    doc.blocks()
        .iter()
        .filter_map(|b| b.xref.clone().map(|x| (b.id, b.name.clone(), x)))
        .collect()
}

/// Whether `kind` is an overlay (`Bindir`).
pub fn is_overlay(kind: XrefKind) -> bool {
    kind == XrefKind::Overlay
}

/// What [`bind_layers`] gives: the drawing's layer tree, each group layer's id to the drawing's, and the names that
/// joined an existing layer.
pub type BoundLayers = (Vec<kentos_contracts::LayerNode>, HashMap<String, String>, Vec<String>);

/// Bind › Insert's layers (docs/adr/0348 §3): the layers under the reference's group `group` joined to the drawing's
/// layers of the same path outside it (made with their groups where missing). Gives the drawing's tree with the new
/// nodes, each group layer's id to the drawing's, and the names of those that joined an existing layer.
pub fn bind_layers(ours: &DocumentSnapshotV2, group: &str) -> Result<BoundLayers, String> {
    let mut ours_json = to_json(ours)?;
    let Some((node, _)) = walked(arr(&ours_json, "layers")).into_iter().find(|(n, _)| str_of(n, "id") == group) else {
        return Ok((ours.layers.clone(), HashMap::new(), Vec::new()));
    };
    let theirs = json!({ "layers": arr(&node, "children"), "settings": {}, "styles": {} });
    let their_paths = path_of(arr(&theirs, "layers"));
    let wanted: HashSet<String> = walked(arr(&theirs, "layers"))
        .into_iter()
        .filter(|(n, _)| str_of(n, "type") == "layer")
        .map(|(_, here)| here.join(" / "))
        .collect();
    // Which already were the drawing's, outside the group.
    let before: HashSet<String> = their_paths
        .iter()
        .filter(|(_, here)| our_node_for(&ours_json, &their_paths, &here_id(&their_paths, here), false).is_some())
        .map(|(id, _)| id.clone())
        .collect();
    let mut taken = all_ids(arr(&ours_json, "layers"));
    take_layers(&mut ours_json, &theirs, &wanted, Same::Skip, &mut taken);
    let mut map = HashMap::new();
    let mut merged = Vec::new();
    for (n, _) in walked(arr(&theirs, "layers")) {
        if str_of(&n, "type") != "layer" {
            continue;
        }
        let id = str_of(&n, "id").to_owned();
        if let Some(to) = our_node_for(&ours_json, &their_paths, &id, false) {
            if before.contains(&id) {
                merged.push(str_of(&n, "name").to_owned());
            }
            map.insert(id, to);
        }
    }
    let drawing = from_json(ours_json)?;
    Ok((drawing.layers, map, merged))
}

/// A path's own node id among `paths` (the reverse lookup `our_node_for` needs).
fn here_id(paths: &HashMap<String, Vec<String>>, here: &[String]) -> String {
    paths.iter().find(|(_, p)| p.as_slice() == here).map(|(id, _)| id.clone()).unwrap_or_default()
}
