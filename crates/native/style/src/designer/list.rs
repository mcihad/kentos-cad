//! The designer's layer list (the web's `designerModel.ts` edits): where a
//! layer is, a layer read and replaced, and the list's edits: add, move,
//! duplicate, remove and switch off. A layer that places markers
//! (`markerLine`, `patternFill`, `centroidMarker`) shows its marker's layers
//! under it, a geometry generator its symbol's (docs/adr/0226 §1); they are
//! edited in the same way. Every edit takes the symbol and the chosen layer
//! and gives the symbol and the layer chosen after it.

use serde_json::{Value, json};

use super::{child_key, has_children, new_layer};

/// Where a layer is: a symbol layer, or a layer of the marker that a symbol layer places.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LayerPath {
    Top(usize),
    /// Layer `.1` of the marker of symbol layer `.0`.
    Child(usize, usize),
}

impl LayerPath {
    /// The path as the fixture writes it: `[i]` or `[i, j]`.
    pub fn to_value(self) -> Value {
        match self {
            LayerPath::Top(i) => json!([i]),
            LayerPath::Child(i, j) => json!([i, j]),
        }
    }

    /// A path from `[i]` or `[i, j]`.
    pub fn from_value(v: &Value) -> Option<LayerPath> {
        let a = v.as_array()?;
        let at = |k: usize| a.get(k).and_then(Value::as_u64).map(|x| x as usize);
        match a.len() {
            1 => Some(LayerPath::Top(at(0)?)),
            2 => Some(LayerPath::Child(at(0)?, at(1)?)),
            _ => None,
        }
    }

    /// The symbol layer it is, or whose marker it is in.
    pub fn top(self) -> usize {
        match self {
            LayerPath::Top(i) | LayerPath::Child(i, _) => i,
        }
    }
}

fn layers(symbol: &Value) -> &[Value] {
    symbol
        .get("layers")
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

/// A parent's child rows: its marker's layers, or its generator symbol's.
fn child_layers(layer: &Value) -> &[Value] {
    layer
        .get(child_key(layer))
        .and_then(|m| m.get("layers"))
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

/// The layer at `p` (`layerAt`).
pub fn layer_at(symbol: &Value, p: LayerPath) -> Option<&Value> {
    let top = layers(symbol).get(p.top())?;
    match p {
        LayerPath::Top(_) => Some(top),
        LayerPath::Child(_, j) if has_children(top) => child_layers(top).get(j),
        LayerPath::Child(..) => None,
    }
}

/// Replaces the layer at `p` (in the marker of its parent for child rows) (`putLayer`).
pub fn put_layer(symbol: &mut Value, p: LayerPath, layer: Value) {
    let Some(list) = symbol.get_mut("layers").and_then(Value::as_array_mut) else {
        return;
    };
    let Some(top) = list.get_mut(p.top()) else {
        return;
    };
    match p {
        LayerPath::Top(_) => *top = layer,
        LayerPath::Child(_, j) => {
            if !has_children(top) {
                return;
            }
            let key = child_key(top);
            if let Some(slot) = top
                .get_mut(key)
                .and_then(|m| m.get_mut("layers"))
                .and_then(Value::as_array_mut)
                .and_then(|l| l.get_mut(j))
            {
                *slot = layer;
            }
        }
    }
}

/// The list a layer lives in, and its place in it.
fn siblings(symbol: &Value, p: LayerPath) -> (Vec<Value>, usize) {
    match p {
        LayerPath::Top(i) => (layers(symbol).to_vec(), i),
        LayerPath::Child(i, j) => (
            layers(symbol)
                .get(i)
                .filter(|l| has_children(l))
                .map(|l| child_layers(l).to_vec())
                .unwrap_or_default(),
            j,
        ),
    }
}

/// Puts back the list a layer lives in: the symbol's layers, or the marker's
/// or the generator symbol's (its other fields kept).
fn set_siblings(symbol: &mut Value, p: LayerPath, list: Vec<Value>) {
    match p {
        LayerPath::Top(_) => {
            if let Some(o) = symbol.as_object_mut() {
                o.insert("layers".into(), Value::Array(list));
            }
        }
        LayerPath::Child(i, _) => {
            let Some(parent) = symbol
                .get_mut("layers")
                .and_then(Value::as_array_mut)
                .and_then(|l| l.get_mut(i))
                .filter(|l| has_children(l))
            else {
                return;
            };
            let key = child_key(parent);
            let mut child = parent
                .get(key)
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            child.insert("layers".into(), Value::Array(list));
            if let Some(o) = parent.as_object_mut() {
                o.insert(key.into(), Value::Object(child));
            }
        }
    }
}

/// A layer id not yet in `list`: its length, or the next free number after it (`uid`).
pub fn uid(list: &[Value]) -> String {
    let used: Vec<&str> = list
        .iter()
        .filter_map(|l| l.get("id").and_then(Value::as_str))
        .collect();
    let mut i = list.len();
    while used.contains(&i.to_string().as_str()) {
        i += 1;
    }
    i.to_string()
}

/// The layer “Katman ekle” may also add into: the chosen child's parent, or
/// the chosen layer that holds a symbol of its own (`addParent`).
pub fn add_parent(symbol: &Value, selected: LayerPath) -> Option<usize> {
    match selected {
        LayerPath::Child(i, _) => Some(i),
        LayerPath::Top(i) => layer_at(symbol, selected)
            .filter(|l| has_children(l))
            .map(|_| i),
    }
}

/// A new layer at the end of the symbol, or of the marker of layer `parent`;
/// the path of the new layer, which is chosen (`addLayer`).
pub fn add_layer(symbol: &mut Value, layer_type: &str, parent: Option<usize>) -> LayerPath {
    match parent {
        None => {
            let kind = symbol
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or("fill")
                .to_owned();
            let mut list = layers(symbol).to_vec();
            let layer = new_layer(layer_type, &uid(&list), &kind);
            list.push(layer);
            let at = list.len() - 1;
            set_siblings(symbol, LayerPath::Top(0), list);
            LayerPath::Top(at)
        }
        Some(i) => {
            let at = LayerPath::Child(i, 0);
            let (mut list, _) = siblings(symbol, at);
            // A generator's symbol's own class; a marker's.
            let context = layers(symbol)
                .get(i)
                .filter(|l| has_children(l))
                .and_then(|l| l.get(child_key(l)))
                .and_then(|c| c.get("type"))
                .and_then(Value::as_str)
                .unwrap_or("marker")
                .to_owned();
            let layer = new_layer(layer_type, &uid(&list), &context);
            list.push(layer);
            let j = list.len() - 1;
            set_siblings(symbol, at, list);
            LayerPath::Child(i, j)
        }
    }
}

/// Whether the chosen layer can move up (`delta` −1) or down (+1) its list (`canMove`).
pub fn can_move(symbol: &Value, selected: LayerPath, delta: i64) -> bool {
    let (list, index) = siblings(symbol, selected);
    let to = index as i64 + delta;
    index < list.len() && to >= 0 && (to as usize) < list.len()
}

fn with_index(p: LayerPath, k: usize) -> LayerPath {
    match p {
        LayerPath::Top(_) => LayerPath::Top(k),
        LayerPath::Child(i, _) => LayerPath::Child(i, k),
    }
}

/// The chosen layer swapped with its neighbour; None when it is at that end (`moveLayer`).
pub fn move_layer(symbol: &mut Value, selected: LayerPath, delta: i64) -> Option<LayerPath> {
    if !can_move(symbol, selected, delta) {
        return None;
    }
    let (mut list, index) = siblings(symbol, selected);
    let to = (index as i64 + delta) as usize;
    list.swap(index, to);
    set_siblings(symbol, selected, list);
    Some(with_index(selected, to))
}

/// A copy of the chosen layer (with a new id) after it; the copy is chosen (`duplicateLayer`).
pub fn duplicate_layer(symbol: &mut Value, selected: LayerPath) -> Option<LayerPath> {
    let (mut list, index) = siblings(symbol, selected);
    let mut copy = list.get(index)?.clone();
    let id = uid(&list);
    if let Some(o) = copy.as_object_mut() {
        o.insert("id".into(), Value::from(id));
    }
    list.insert(index + 1, copy);
    set_siblings(symbol, selected, list);
    Some(with_index(selected, index + 1))
}

/// Whether the chosen layer can go: a symbol keeps one layer at least (a
/// marker may be left empty) (`canRemove`).
pub fn can_remove(symbol: &Value, selected: LayerPath) -> bool {
    let (list, index) = siblings(symbol, selected);
    index < list.len() && !(matches!(selected, LayerPath::Top(_)) && list.len() == 1)
}

/// The symbol without the chosen layer; the layer chosen after it, None when
/// it cannot go (`removeLayer`).
pub fn remove_layer(symbol: &mut Value, selected: LayerPath) -> Option<LayerPath> {
    if !can_remove(symbol, selected) {
        return None;
    }
    let (mut list, index) = siblings(symbol, selected);
    list.remove(index);
    let left = list.len();
    set_siblings(symbol, selected, list);
    Some(match selected {
        LayerPath::Top(_) => LayerPath::Top(index.min(left.saturating_sub(1))),
        LayerPath::Child(i, _) if left == 0 => LayerPath::Top(i),
        LayerPath::Child(i, _) => LayerPath::Child(i, index.min(left - 1)),
    })
}

/// A layer switched on (its `enabled` goes) or off (`enabled: false`) (`setEnabled`).
pub fn set_enabled(symbol: &mut Value, p: LayerPath, on: bool) {
    let Some(mut layer) = layer_at(symbol, p).cloned() else {
        return;
    };
    if let Some(o) = layer.as_object_mut() {
        if on {
            o.remove("enabled");
        } else {
            o.insert("enabled".into(), Value::Bool(false));
        }
    }
    put_layer(symbol, p, layer);
}
