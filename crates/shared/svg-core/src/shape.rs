//! The editor's data as it crosses the boundary (`apps/web/src/style/svg/pathData.ts`,
//! `svgModel.ts`): path nodes and sub-paths as typed values, shapes as JSON
//! objects kept field for field. A shape carries paints, names, groups and
//! flags the geometry never reads; an operation changes the fields it owns
//! and leaves the rest where they were, in their order, as the TypeScript's
//! object spread (`{ ...s, x, y }`) did, so the editor's change detection
//! (JSON text of the drawing) sees no difference where nothing changed.

use kentos_geometry_core::api::json::{FromJson, Json, ToJson, write_str};

/// A point as the editor writes it: `[x, y]`.
pub type Pt = [f64; 2];

/// A path node: a point with optional cubic handles (absolute).
#[derive(Clone, Debug, PartialEq)]
pub struct PathNode {
    pub x: f64,
    pub y: f64,
    /// Incoming handle (control point before this node).
    pub in_: Option<Pt>,
    /// Outgoing handle (control point after this node).
    pub out: Option<Pt>,
    /// How the node editor keeps its handles (editing state only).
    pub ty: Option<String>,
}

impl PathNode {
    pub fn at(x: f64, y: f64) -> PathNode {
        PathNode {
            x,
            y,
            in_: None,
            out: None,
            ty: None,
        }
    }

    pub fn pt(&self) -> Pt {
        [self.x, self.y]
    }
}

/// A run of nodes; a closed one has a segment from the last node back to the first.
#[derive(Clone, Debug, PartialEq)]
pub struct SubPath {
    pub closed: bool,
    pub nodes: Vec<PathNode>,
}

/// JavaScript truthiness of a JSON value (`if (sp.closed)`).
pub fn truthy(v: &Json) -> bool {
    match v {
        Json::Null => false,
        Json::Bool(b) => *b,
        Json::Num(x) => *x != 0.0 && !x.is_nan(),
        // "#NaN" is how a NaN number crosses; any other text is text.
        Json::Str(s) => !s.is_empty() && s != "#NaN",
        Json::Arr(_) | Json::Obj(_) => true,
    }
}

fn opt_pt(v: &Json, name: &str) -> Result<Option<Pt>, String> {
    Option::<Pt>::from_json(v.get(name)).map_err(|e| format!("“{name}”: {e}"))
}

impl FromJson for PathNode {
    fn from_json(v: &Json) -> Result<PathNode, String> {
        if !matches!(v, Json::Obj(_)) {
            return Err("düğüm nesnesi bekleniyordu".into());
        }
        Ok(PathNode {
            x: f64::from_json(v.get("x")).map_err(|e| format!("“x”: {e}"))?,
            y: f64::from_json(v.get("y")).map_err(|e| format!("“y”: {e}"))?,
            in_: opt_pt(v, "in")?,
            out: opt_pt(v, "out")?,
            ty: match v.get("type") {
                Json::Str(s) => Some(s.clone()),
                _ => None,
            },
        })
    }
}

impl ToJson for PathNode {
    fn write_json(&self, out: &mut String) {
        out.push_str("{\"x\":");
        self.x.write_json(out);
        out.push_str(",\"y\":");
        self.y.write_json(out);
        if let Some(p) = &self.in_ {
            out.push_str(",\"in\":");
            p.write_json(out);
        }
        if let Some(p) = &self.out {
            out.push_str(",\"out\":");
            p.write_json(out);
        }
        if let Some(t) = &self.ty {
            out.push_str(",\"type\":");
            write_str(out, t);
        }
        out.push('}');
    }
}

impl FromJson for SubPath {
    fn from_json(v: &Json) -> Result<SubPath, String> {
        if !matches!(v, Json::Obj(_)) {
            return Err("alt yol nesnesi bekleniyordu".into());
        }
        Ok(SubPath {
            closed: truthy(v.get("closed")),
            nodes: Vec::from_json(v.get("nodes")).map_err(|e| format!("“nodes”: {e}"))?,
        })
    }
}

impl ToJson for SubPath {
    fn write_json(&self, out: &mut String) {
        out.push_str("{\"closed\":");
        self.closed.write_json(out);
        out.push_str(",\"nodes\":");
        self.nodes.write_json(out);
        out.push('}');
    }
}

// ── Shapes as JSON objects ─────────────────────────────────────────────

/// A shape (or any object) as its fields in order. A field set to
/// undefined keeps its place (a later spread sets it there) but is left out
/// of the text, as `JSON.stringify` leaves it out.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Obj(pub Vec<(String, Option<Json>)>);

impl FromJson for Obj {
    fn from_json(v: &Json) -> Result<Obj, String> {
        match v {
            Json::Obj(fields) => Ok(Obj(fields
                .iter()
                .map(|(k, v)| (k.clone(), Some(v.clone())))
                .collect())),
            _ => Err("nesne bekleniyordu".into()),
        }
    }
}

impl ToJson for Obj {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        for (k, v) in &self.0 {
            let Some(v) = v else { continue };
            if !first {
                out.push(',');
            }
            first = false;
            write_str(out, k);
            out.push(':');
            v.write_json(out);
        }
        out.push('}');
    }
}

impl Obj {
    /// A field's value; absent and undefined read as null.
    pub fn get(&self, key: &str) -> &Json {
        match self.0.iter().find(|(k, _)| k == key) {
            Some((_, Some(v))) => v,
            _ => &Json::Null,
        }
    }

    /// A field's value when defined (`s.key !== undefined`).
    pub fn field(&self, key: &str) -> Option<Json> {
        match self.0.iter().find(|(k, _)| k == key) {
            Some((_, v)) => v.clone(),
            None => None,
        }
    }

    /// Whether a field is defined (`s.key !== undefined`).
    pub fn has(&self, key: &str) -> bool {
        matches!(self.0.iter().find(|(k, _)| k == key), Some((_, Some(_))))
    }

    /// `s.kind`.
    pub fn kind(&self) -> &str {
        match self.get("kind") {
            Json::Str(s) => s,
            _ => "",
        }
    }

    /// A text field (`s.id`), or None when it is not text.
    pub fn text(&self, key: &str) -> Option<&str> {
        match self.get(key) {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }

    /// A number field as arithmetic reads it: absent, undefined or null is NaN.
    pub fn num(&self, key: &str) -> f64 {
        f64::from_json(self.get(key)).unwrap_or(f64::NAN)
    }

    /// A number field for `??`: None when absent, undefined or null.
    pub fn opt_num(&self, key: &str) -> Option<f64> {
        match self.get(key) {
            Json::Null => None,
            v => Some(f64::from_json(v).unwrap_or(f64::NAN)),
        }
    }

    /// Whether a field is truthy (`if (s.rotate)`).
    pub fn is(&self, key: &str) -> bool {
        truthy(self.get(key))
    }

    /// Sets a field (`{ ...s, key: value }`): in its place when present, else at the end.
    pub fn put(&mut self, key: &str, v: Option<Json>) {
        match self.0.iter_mut().find(|(k, _)| k == key) {
            Some(slot) => slot.1 = v,
            None => self.0.push((key.to_string(), v)),
        }
    }

    pub fn set(&mut self, key: &str, v: Json) {
        self.put(key, Some(v));
    }

    pub fn set_num(&mut self, key: &str, x: f64) {
        self.set(key, Json::Num(x));
    }

    pub fn set_text(&mut self, key: &str, s: &str) {
        self.set(key, Json::Str(s.to_string()));
    }

    /// `{ ...s, key: undefined }`: the field keeps (or takes) its place, undefined.
    pub fn set_undefined(&mut self, key: &str) {
        self.put(key, None);
    }

    /// `delete s.key`.
    pub fn remove(&mut self, key: &str) {
        self.0.retain(|(k, _)| k != key);
    }

    /// A path shape's sub-paths.
    pub fn subs(&self) -> Result<Vec<SubPath>, String> {
        Vec::from_json(self.get("subs")).map_err(|e| format!("“subs”: {e}"))
    }

    pub fn set_subs(&mut self, subs: &[SubPath]) {
        self.set("subs", Json::Arr(subs.iter().map(SubPath::tree).collect()));
    }
}

fn pt_tree(p: &Pt) -> Json {
    Json::Arr(vec![Json::Num(p[0]), Json::Num(p[1])])
}

impl PathNode {
    /// The node as a JSON tree (fields in the order `ToJson` writes them).
    pub fn tree(&self) -> Json {
        let mut f = vec![
            ("x".to_string(), Json::Num(self.x)),
            ("y".to_string(), Json::Num(self.y)),
        ];
        if let Some(p) = &self.in_ {
            f.push(("in".to_string(), pt_tree(p)));
        }
        if let Some(p) = &self.out {
            f.push(("out".to_string(), pt_tree(p)));
        }
        if let Some(t) = &self.ty {
            f.push(("type".to_string(), Json::Str(t.clone())));
        }
        Json::Obj(f)
    }
}

impl SubPath {
    /// The sub-path as a JSON tree.
    pub fn tree(&self) -> Json {
        Json::Obj(vec![
            ("closed".to_string(), Json::Bool(self.closed)),
            (
                "nodes".to_string(),
                Json::Arr(self.nodes.iter().map(PathNode::tree).collect()),
            ),
        ])
    }
}
