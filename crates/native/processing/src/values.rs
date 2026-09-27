//! The JSON values of the parameters that have a shape, read and written
//! (the web's `FeaturesValue`, `LayerValue` and `Vec2 | null`).

use kentos_contracts::Vec2;
use serde_json::{Map, Value, json};

/// Which objects a features value reads. `Ids` is what models pass between
/// steps (the objects an earlier step made, chose or changed).
#[derive(Clone, Debug, PartialEq)]
pub enum Scope {
    Selection,
    Visible,
    All,
    Layer(String),
    Ids(Vec<u32>),
}

impl Scope {
    pub fn id(&self) -> &'static str {
        match self {
            Scope::Selection => "selection",
            Scope::Visible => "visible",
            Scope::All => "all",
            Scope::Layer(_) => "layer",
            Scope::Ids(_) => "ids",
        }
    }
}

/// A features parameter's value; `kinds` narrows the tool's kinds for this
/// run (only polygons, say), none means every kind the tool takes.
#[derive(Clone, Debug, PartialEq)]
pub struct FeaturesValue {
    pub scope: Scope,
    pub kinds: Option<Vec<String>>,
}

impl FeaturesValue {
    pub fn scope(scope: Scope) -> Self {
        Self { scope, kinds: None }
    }

    /// A value in its JSON form, when it has the shape of one: a known scope
    /// with what it needs (a layer id, an id list) and kinds as a list of
    /// texts. Whether the parameter offers the scope and takes the kinds is
    /// `parameters::fits`' question.
    pub fn read(v: &Value) -> Option<Self> {
        let o = v.as_object()?;
        let kinds = match o.get("kinds") {
            None => None,
            Some(Value::Array(list)) => Some(
                list.iter()
                    .map(|k| k.as_str().map(str::to_owned))
                    .collect::<Option<Vec<_>>>()?,
            ),
            Some(_) => return None,
        };
        let scope = match o.get("scope")?.as_str()? {
            "selection" => Scope::Selection,
            "visible" => Scope::Visible,
            "all" => Scope::All,
            "layer" => Scope::Layer(o.get("layerId")?.as_str()?.to_owned()),
            "ids" => Scope::Ids(
                o.get("ids")?
                    .as_array()?
                    .iter()
                    .filter_map(|id| {
                        id.as_f64()
                            .filter(|x| x.fract() == 0.0 && *x >= 0.0 && *x <= f64::from(u32::MAX))
                            .map(|x| x as u32)
                    })
                    .collect(),
            ),
            _ => return None,
        };
        Some(Self { scope, kinds })
    }

    pub fn to_json(&self) -> Value {
        let mut o = Map::new();
        o.insert("scope".into(), self.scope.id().into());
        match &self.scope {
            Scope::Layer(id) => {
                o.insert("layerId".into(), id.as_str().into());
            }
            Scope::Ids(ids) => {
                o.insert("ids".into(), json!(ids));
            }
            _ => {}
        }
        if let Some(kinds) = &self.kinds {
            o.insert("kinds".into(), json!(kinds));
        }
        Value::Object(o)
    }
}

/// A layer parameter's value: an existing layer, or a new one made when the tool writes to it.
#[derive(Clone, Debug, PartialEq)]
pub enum LayerValue {
    Existing(String),
    New(String),
}

impl LayerValue {
    pub fn read(v: &Value) -> Option<Self> {
        let o = v.as_object()?;
        if o.contains_key("layerId") {
            return o
                .get("layerId")?
                .as_str()
                .map(|id| LayerValue::Existing(id.into()));
        }
        o.get("newName")?
            .as_str()
            .map(|name| LayerValue::New(name.into()))
    }

    pub fn to_json(&self) -> Value {
        match self {
            LayerValue::Existing(id) => json!({ "layerId": id }),
            LayerValue::New(name) => json!({ "newName": name }),
        }
    }
}

/// A point value (`{ x, y }`); none for null or anything else.
pub fn point(v: &Value) -> Option<Vec2> {
    let o = v.as_object()?;
    Some(Vec2 {
        x: o.get("x")?.as_f64()?,
        y: o.get("y")?.as_f64()?,
    })
}

pub fn point_json(p: Option<Vec2>) -> Value {
    p.map_or(Value::Null, |p| json!({ "x": p.x, "y": p.y }))
}
