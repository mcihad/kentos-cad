//! Parameter definitions in the web's JSON shape (`ParamDef` of the web's
//! processing/types.ts): a model keeps its inputs so (the designer edits
//! them as the web does, and a saved model reads back the same), and tools
//! defined as data, as the shared cases' `t.*` tools, are read from it. A
//! `visibleWhen` written as data is `{ param, equals }`.

use serde_json::{Map, Value};

use crate::types::{
    DefaultValue, EnumOption, NewLayerStyle, ParamDef, ParamKind, Returns, ScopeKind, ShownWhen,
};

fn text(o: &Map<String, Value>, key: &str) -> Option<String> {
    o.get(key).and_then(Value::as_str).map(str::to_owned)
}

fn flag(o: &Map<String, Value>, key: &str) -> bool {
    o.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn number(o: &Map<String, Value>, key: &str) -> Option<f64> {
    o.get(key).and_then(Value::as_f64)
}

fn texts(v: Option<&Value>) -> Option<Vec<String>> {
    Some(
        v?.as_array()?
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
    )
}

fn scope(id: &str) -> Option<ScopeKind> {
    match id {
        "selection" => Some(ScopeKind::Selection),
        "visible" => Some(ScopeKind::Visible),
        "all" => Some(ScopeKind::All),
        "layer" => Some(ScopeKind::Layer),
        _ => None,
    }
}

/// A parameter from the web's JSON; `None` when it is not one (no name, an
/// unknown type).
pub fn param_from_json(v: &Value) -> Option<ParamDef> {
    let o = v.as_object()?;
    let name = o.get("name")?.as_str()?;
    let label = o.get("label").and_then(Value::as_str).unwrap_or(name);
    let kind = match o.get("type")?.as_str()? {
        "features" => ParamKind::Features {
            kinds: texts(o.get("kinds")),
            scopes: texts(o.get("scopes")).map(|s| s.iter().filter_map(|id| scope(id)).collect()),
            writes: flag(o, "writes"),
        },
        "number" => ParamKind::Number {
            min: number(o, "min"),
            max: number(o, "max"),
            integer: flag(o, "integer"),
            unit: text(o, "unit").unwrap_or_default(),
        },
        "string" => ParamKind::Text {
            placeholder: text(o, "placeholder"),
            max_length: o
                .get("maxLength")
                .and_then(Value::as_u64)
                .and_then(|n| usize::try_from(n).ok()),
            allow_empty: flag(o, "allowEmpty"),
        },
        "boolean" => ParamKind::Boolean,
        "enum" => ParamKind::Choice {
            options: o
                .get("options")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|opt| {
                    let value = opt.get("value")?.as_str()?;
                    let label = opt.get("label").and_then(Value::as_str).unwrap_or(value);
                    let option = EnumOption::new(value, label);
                    Some(match opt.get("hint").and_then(Value::as_str) {
                        Some(hint) => option.hint(hint),
                        None => option,
                    })
                })
                .collect(),
        },
        "layer" => ParamKind::Layer {
            new_layer_style: NewLayerStyle {
                color: o
                    .get("newLayerStyle")
                    .and_then(|s| s.get("color"))
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                ..NewLayerStyle::default()
            },
        },
        "point" => ParamKind::Point,
        "expression" => ParamKind::Expression {
            returns: if o.get("returns").and_then(Value::as_str) == Some("condition") {
                Returns::Condition
            } else {
                Returns::Value
            },
            of: text(o, "of"),
            placeholder: text(o, "placeholder"),
        },
        // `of` names one parameter, or a list of them (the first shown is read).
        "field" => ParamKind::Field {
            of: match o.get("of") {
                Some(Value::Array(_)) => texts(o.get("of")).unwrap_or_default(),
                _ => text(o, "of").into_iter().collect(),
            },
            allow_new: flag(o, "allowNew"),
            multiple: flag(o, "multiple"),
        },
        "file" => ParamKind::File {
            accept: texts(o.get("accept")).unwrap_or_default(),
        },
        _ => return None,
    };
    let mut def = ParamDef::new(name, label, kind);
    def.description = text(o, "description");
    def.optional = flag(o, "optional");
    def.advanced = flag(o, "advanced");
    // `'default' in p`: a default written as null is a default.
    if let Some(d) = o.get("default") {
        def.default = Some(DefaultValue::Value(d.clone()));
    }
    if let Some(when) = o.get("visibleWhen")
        && let Some(param) = when.get("param").and_then(Value::as_str)
    {
        def.visible_when = Some(ShownWhen::Equals {
            param: param.to_owned(),
            value: when.get("equals").cloned().unwrap_or(Value::Null),
        });
    }
    if let Some(picks) = o.get("picks")
        && let (Some(option), Some(point)) = (
            picks.get("option").and_then(Value::as_str),
            picks.get("point").and_then(Value::as_str),
        )
    {
        def.picks = Some((option.to_owned(), point.to_owned()));
    }
    Some(def)
}
