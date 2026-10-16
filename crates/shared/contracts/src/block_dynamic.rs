//! Dynamic blocks (docs/adr/0318): one definition drawn at many sizes and looks. A definition's parameters (a length
//! along a line, a polar end, an angle, a point, a flip, a visibility state, a lookup row) each have a value its
//! inserts may change; its actions (move, stretch, scale, array, rotate, flip) say which of its objects a
//! parameter's change moves and how. An insert keeps only the values that differ from the defaults.
//!
//! The rules here are the definition's and a value's; the reader, the documents and the commands check them alike.
//! A number parameter's expression is checked by the commands (the expression language is theirs).

use std::collections::{BTreeMap, HashSet};

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::blocks::name_key;
use crate::{BlockDefinition, Vec2};

/// A definition's parameters and actions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DynamicBlock {
    pub parameters: Vec<BlockParameter>,
    /// Applied in this order to the definition's objects, each on what the ones before left.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<BlockAction>>", optional))]
    pub actions: Vec<BlockAction>,
}

/// The values a number parameter may take: within `min` and `max`, on `step`s from `min` (from 0 without one), one
/// of `list`; or computed by `expression` from the other number parameters (it is not set then).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NumberRule {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub max: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub step: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<f64>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub list: Vec<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub expression: Option<String>,
}

impl NumberRule {
    pub fn is_free(&self) -> bool {
        *self == NumberRule::default()
    }
}

/// One of a visibility parameter's states: its name and the objects (local ids) it hides.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct VisibilityState {
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<u32>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub hidden: Vec<u32>,
}

/// One row of a lookup: its label and the values it sets.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LookupRow {
    pub label: String,
    pub values: BTreeMap<String, ParamValue>,
}

/// A parameter's value: a number (a length, an angle in radians), a point of the definition's coordinates, a flip
/// or a name (a visibility state, a lookup row).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
#[cfg_attr(feature = "ts", ts(export))]
#[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
pub enum ParamValue {
    Flag(bool),
    Number(f64),
    Text(String),
    Point(Vec2),
}

/// A parameter (docs/adr/0318 §1). Its points are the definition's coordinates.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum BlockParameter {
    /// A length along `from`→`to`: its default the distance between them; a change moves `to` along the line.
    Linear {
        name: String,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        from: Vec2,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        to: Vec2,
        #[serde(default, skip_serializing_if = "NumberRule::is_free")]
        #[cfg_attr(feature = "ts", ts(as = "Option<NumberRule>", optional))]
        rule: NumberRule,
    },
    /// `to` moved anywhere: its value a point, its rule on the distance from `from`.
    Polar {
        name: String,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        from: Vec2,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        to: Vec2,
        #[serde(default, skip_serializing_if = "NumberRule::is_free")]
        #[cfg_attr(feature = "ts", ts(as = "Option<NumberRule>", optional))]
        rule: NumberRule,
    },
    /// An angle about `center`, radians counter-clockwise from east; its default `base`.
    Angle {
        name: String,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        center: Vec2,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
        base: f64,
        #[serde(default, skip_serializing_if = "NumberRule::is_free")]
        #[cfg_attr(feature = "ts", ts(as = "Option<NumberRule>", optional))]
        rule: NumberRule,
    },
    /// A point: its default `at`.
    Point {
        name: String,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        at: Vec2,
    },
    /// A mirror about the line `a`→`b`, off by default.
    Flip {
        name: String,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        a: Vec2,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        b: Vec2,
    },
    /// The objects shown: its first state by default.
    Visibility {
        name: String,
        states: Vec<VisibilityState>,
    },
    /// Rows of values set together, chosen by label; it has no value of its own to keep.
    Lookup { name: String, rows: Vec<LookupRow> },
}

impl BlockParameter {
    pub fn name(&self) -> &str {
        match self {
            BlockParameter::Linear { name, .. }
            | BlockParameter::Polar { name, .. }
            | BlockParameter::Angle { name, .. }
            | BlockParameter::Point { name, .. }
            | BlockParameter::Flip { name, .. }
            | BlockParameter::Visibility { name, .. }
            | BlockParameter::Lookup { name, .. } => name,
        }
    }

    /// Its word, as a message says it.
    pub fn word(&self) -> &'static str {
        match self {
            BlockParameter::Linear { .. } => "doğrusal",
            BlockParameter::Polar { .. } => "kutupsal",
            BlockParameter::Angle { .. } => "açı",
            BlockParameter::Point { .. } => "nokta",
            BlockParameter::Flip { .. } => "çevir",
            BlockParameter::Visibility { .. } => "görünürlük",
            BlockParameter::Lookup { .. } => "tablo",
        }
    }

    /// Its rule, when it is a number parameter.
    pub fn rule(&self) -> Option<&NumberRule> {
        match self {
            BlockParameter::Linear { rule, .. }
            | BlockParameter::Polar { rule, .. }
            | BlockParameter::Angle { rule, .. } => Some(rule),
            _ => None,
        }
    }

    /// Its default value; a lookup has none.
    pub fn default_value(&self) -> Option<ParamValue> {
        Some(match self {
            BlockParameter::Linear { from, to, .. } => ParamValue::Number(distance(*from, *to)),
            BlockParameter::Polar { to, .. } => ParamValue::Point(*to),
            BlockParameter::Angle { base, .. } => ParamValue::Number(*base),
            BlockParameter::Point { at, .. } => ParamValue::Point(*at),
            BlockParameter::Flip { .. } => ParamValue::Flag(false),
            BlockParameter::Visibility { states, .. } => {
                ParamValue::Text(states.first().map(|s| s.name.clone()).unwrap_or_default())
            }
            BlockParameter::Lookup { .. } => return None,
        })
    }
}

/// An action (docs/adr/0318 §2): the objects (local ids) a parameter's change moves.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum BlockAction {
    /// The objects move by the parameter's change (a linear's along its line, a polar's and a point's by the moved
    /// point).
    Move {
        parameter: String,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
        pieces: Vec<u32>,
    },
    /// The objects' points inside the box move by the parameter's change; the rest stay.
    Stretch {
        parameter: String,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
        pieces: Vec<u32>,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        min: Vec2,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        max: Vec2,
    },
    /// The objects scale about the parameter's `from` by its value over its default.
    Scale {
        parameter: String,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
        pieces: Vec<u32>,
    },
    /// Copies of the objects every `spacing` along a linear parameter's line: one for each further `spacing` its
    /// value holds past its default (the definition shows what the default holds).
    Array {
        parameter: String,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
        pieces: Vec<u32>,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        spacing: f64,
    },
    /// The objects turn about an angle parameter's centre by its change.
    Rotate {
        parameter: String,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
        pieces: Vec<u32>,
    },
    /// The objects mirror about a flip parameter's line while it is on.
    Flip {
        parameter: String,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
        pieces: Vec<u32>,
    },
}

impl BlockAction {
    pub fn parameter(&self) -> &str {
        match self {
            BlockAction::Move { parameter, .. }
            | BlockAction::Stretch { parameter, .. }
            | BlockAction::Scale { parameter, .. }
            | BlockAction::Array { parameter, .. }
            | BlockAction::Rotate { parameter, .. }
            | BlockAction::Flip { parameter, .. } => parameter,
        }
    }

    pub fn pieces(&self) -> &[u32] {
        match self {
            BlockAction::Move { pieces, .. }
            | BlockAction::Stretch { pieces, .. }
            | BlockAction::Scale { pieces, .. }
            | BlockAction::Array { pieces, .. }
            | BlockAction::Rotate { pieces, .. }
            | BlockAction::Flip { pieces, .. } => pieces,
        }
    }

    pub fn word(&self) -> &'static str {
        match self {
            BlockAction::Move { .. } => "taşı",
            BlockAction::Stretch { .. } => "esnet",
            BlockAction::Scale { .. } => "ölçekle",
            BlockAction::Array { .. } => "dizi",
            BlockAction::Rotate { .. } => "döndür",
            BlockAction::Flip { .. } => "aynala",
        }
    }

    /// Whether it works on a parameter of this kind.
    pub fn fits(&self, p: &BlockParameter) -> bool {
        use BlockParameter as P;
        match self {
            BlockAction::Move { .. } | BlockAction::Stretch { .. } => {
                matches!(p, P::Linear { .. } | P::Polar { .. } | P::Point { .. })
            }
            BlockAction::Scale { .. } => matches!(p, P::Linear { .. } | P::Polar { .. }),
            BlockAction::Array { .. } => matches!(p, P::Linear { .. }),
            BlockAction::Rotate { .. } => matches!(p, P::Angle { .. }),
            BlockAction::Flip { .. } => matches!(p, P::Flip { .. }),
        }
    }
}

/// The most parameters, actions, states, rows and pieces of one: a dynamic block stays small enough to draw at
/// any value without delay.
pub const MAX_DYNAMIC_ITEMS: usize = 256;

fn distance(a: Vec2, b: Vec2) -> f64 {
    (b.x - a.x).hypot(b.y - a.y)
}

fn finite(p: Vec2) -> bool {
    p.x.is_finite() && p.y.is_finite()
}

/// The slack a number's comparison allows: a hair of its size.
fn slack(x: f64) -> f64 {
    1e-9 * x.abs().max(1.0)
}

/// Why a number rule is not one.
fn rule_problem(r: &NumberRule) -> Option<String> {
    for (word, v) in [("en küçük", r.min), ("en büyük", r.max), ("adım", r.step)] {
        if v.is_some_and(|v| !v.is_finite()) {
            return Some(format!("{word} değer sonlu bir sayı olmalı"));
        }
    }
    if let (Some(a), Some(b)) = (r.min, r.max)
        && a > b
    {
        return Some("en küçük değer en büyükten büyük".into());
    }
    if r.step.is_some_and(|s| s <= 0.0) {
        return Some("adım artı bir sayı olmalı".into());
    }
    if r.list.iter().any(|v| !v.is_finite()) {
        return Some("değer listesinde sonlu olmayan sayı var".into());
    }
    if r.expression.as_deref().is_some_and(|e| e.trim().is_empty()) {
        return Some("ifade boş".into());
    }
    if r.expression.is_some() && (!r.list.is_empty() || r.step.is_some()) {
        return Some("ifadeyle hesaplanan değerin listesi ya da adımı olmaz".into());
    }
    None
}

/// Why a number does not meet a rule.
pub fn number_problem(r: &NumberRule, v: f64) -> Option<String> {
    if !v.is_finite() {
        return Some("sonlu bir sayı olmalı".into());
    }
    if let Some(min) = r.min
        && v < min - slack(min)
    {
        return Some(format!("en küçük değer {min}"));
    }
    if let Some(max) = r.max
        && v > max + slack(max)
    {
        return Some(format!("en büyük değer {max}"));
    }
    if let Some(step) = r.step {
        let from = r.min.unwrap_or(0.0);
        let k = (v - from) / step;
        if (k - k.round()).abs() * step > slack(v) {
            return Some(format!("{from} değerinden {step} adımlarla"));
        }
    }
    if !r.list.is_empty() && !r.list.iter().any(|x| (x - v).abs() <= slack(v)) {
        let list: Vec<String> = r.list.iter().map(|x| x.to_string()).collect();
        return Some(format!("şunlardan biri: {}", list.join(", ")));
    }
    None
}

/// Why a value is not one of a parameter's.
pub fn value_problem(p: &BlockParameter, v: &ParamValue) -> Option<String> {
    use BlockParameter as P;
    match (p, v) {
        (P::Linear { rule, .. } | P::Angle { rule, .. }, ParamValue::Number(x)) => {
            number_problem(rule, *x)
        }
        (P::Polar { from, rule, .. }, ParamValue::Point(q)) => {
            if !finite(*q) {
                return Some("noktanın koordinatları sonlu olmalı".into());
            }
            number_problem(rule, distance(*from, *q)).map(|m| format!("uzaklık: {m}"))
        }
        (P::Point { .. }, ParamValue::Point(q)) => {
            (!finite(*q)).then(|| "noktanın koordinatları sonlu olmalı".into())
        }
        (P::Flip { .. }, ParamValue::Flag(_)) => None,
        (P::Visibility { states, .. }, ParamValue::Text(t)) => {
            (!states.iter().any(|s| s.name == *t)).then(|| {
                let names: Vec<&str> = states.iter().map(|s| s.name.as_str()).collect();
                format!("durumlardan biri olmalı: {}", names.join(", "))
            })
        }
        (P::Lookup { rows, .. }, ParamValue::Text(t)) => (!rows.iter().any(|r| r.label == *t))
            .then(|| {
                let labels: Vec<&str> = rows.iter().map(|r| r.label.as_str()).collect();
                format!("satırlardan biri olmalı: {}", labels.join(", "))
            }),
        (P::Linear { .. } | P::Angle { .. }, _) => Some("bir sayı olmalı".into()),
        (P::Polar { .. } | P::Point { .. }, _) => Some("bir nokta ({x, y}) olmalı".into()),
        (P::Flip { .. }, _) => Some("açık ya da kapalı (true, false) olmalı".into()),
        (P::Visibility { .. } | P::Lookup { .. }, _) => Some("bir ad olmalı".into()),
    }
}

/// Two values the same (numbers and points within a hair).
pub fn same_param_value(a: &ParamValue, b: &ParamValue) -> bool {
    match (a, b) {
        (ParamValue::Number(x), ParamValue::Number(y)) => (x - y).abs() <= slack(*x),
        (ParamValue::Point(p), ParamValue::Point(q)) => {
            (p.x - q.x).abs() <= slack(p.x) && (p.y - q.y).abs() <= slack(p.y)
        }
        _ => a == b,
    }
}

/// Why a definition's dynamic part is not one: its path under the definition and its words. `ids` are the local ids
/// of the definition's objects.
pub fn dynamic_problem(d: &DynamicBlock, ids: &HashSet<u32>) -> Option<(String, String)> {
    let at = |path: String, m: String| Some((path, m));
    if d.parameters.is_empty() {
        return at(
            "parameters".into(),
            "dinamik bloğun en az bir parametresi olmalı".into(),
        );
    }
    if d.parameters.len() > MAX_DYNAMIC_ITEMS || d.actions.len() > MAX_DYNAMIC_ITEMS {
        return at(
            "parameters".into(),
            format!("en çok {MAX_DYNAMIC_ITEMS} parametre ve {MAX_DYNAMIC_ITEMS} eylem olur"),
        );
    }
    let mut names: HashSet<String> = HashSet::new();
    let mut visibility = 0;
    for (i, p) in d.parameters.iter().enumerate() {
        let path = |f: &str| format!("parameters[{i}]{f}");
        let name = p.name();
        if name.trim().is_empty() || name.trim() != name {
            return at(
                path(".name"),
                "parametrenin adı boş olamaz ve boşlukla başlayıp bitemez".into(),
            );
        }
        if !names.insert(name_key(name)) {
            return at(path(".name"), format!("“{name}” adlı iki parametre var"));
        }
        let points: Vec<Vec2> = match p {
            BlockParameter::Linear { from, to, .. } | BlockParameter::Polar { from, to, .. } => {
                vec![*from, *to]
            }
            BlockParameter::Angle { center, .. } => vec![*center],
            BlockParameter::Point { at, .. } => vec![*at],
            BlockParameter::Flip { a, b, .. } => vec![*a, *b],
            _ => Vec::new(),
        };
        if points.iter().any(|q| !finite(*q)) {
            return at(
                path(""),
                format!("“{name}” parametresinin noktaları sonlu olmalı"),
            );
        }
        match p {
            BlockParameter::Linear { from, to, .. }
            | BlockParameter::Polar { from, to, .. }
            | BlockParameter::Flip { a: from, b: to, .. }
                if distance(*from, *to) == 0.0 =>
            {
                return at(
                    path(""),
                    format!("“{name}” parametresinin iki noktası aynı"),
                );
            }
            BlockParameter::Angle { base, .. } if !base.is_finite() => {
                return at(
                    path(".base"),
                    format!("“{name}” parametresinin açısı sonlu olmalı"),
                );
            }
            BlockParameter::Visibility { states, .. } => {
                visibility += 1;
                if visibility > 1 {
                    return at(
                        path(""),
                        "bir blokta tek görünürlük parametresi olur".into(),
                    );
                }
                if states.is_empty() || states.len() > MAX_DYNAMIC_ITEMS {
                    return at(
                        path(".states"),
                        format!(
                            "“{name}” parametresinin 1 ile {MAX_DYNAMIC_ITEMS} arası durumu olmalı"
                        ),
                    );
                }
                let mut seen = HashSet::new();
                for (k, s) in states.iter().enumerate() {
                    if s.name.trim().is_empty() || !seen.insert(name_key(&s.name)) {
                        return at(
                            format!("parameters[{i}].states[{k}].name"),
                            format!(
                                "“{name}” parametresinin durum adları boş olamaz ve bir kez geçer"
                            ),
                        );
                    }
                    if let Some(id) = s.hidden.iter().find(|id| !ids.contains(id)) {
                        return at(
                            format!("parameters[{i}].states[{k}].hidden"),
                            format!(
                                "“{}” durumu blokta olmayan {id} numaralı nesneyi gizliyor",
                                s.name
                            ),
                        );
                    }
                }
            }
            _ => {}
        }
        if let Some(r) = p.rule() {
            if let Some(m) = rule_problem(r) {
                return at(
                    path(".rule"),
                    format!("“{name}” parametresinin kuralı: {m}"),
                );
            }
            if r.expression.is_none() {
                let default = match p.default_value() {
                    Some(ParamValue::Number(x)) => x,
                    Some(ParamValue::Point(q)) => match p {
                        BlockParameter::Polar { from, .. } => distance(*from, q),
                        _ => 0.0,
                    },
                    _ => 0.0,
                };
                if let Some(m) = number_problem(r, default) {
                    return at(
                        path(".rule"),
                        format!("“{name}” parametresinin varsayılan değeri kurala uymuyor ({m})"),
                    );
                }
            }
        }
    }
    let by_name = |n: &str| d.parameters.iter().find(|p| p.name() == n);
    for (i, p) in d.parameters.iter().enumerate() {
        let BlockParameter::Lookup { name, rows } = p else {
            continue;
        };
        if rows.is_empty() || rows.len() > MAX_DYNAMIC_ITEMS {
            return at(
                format!("parameters[{i}].rows"),
                format!("“{name}” tablosunun 1 ile {MAX_DYNAMIC_ITEMS} arası satırı olmalı"),
            );
        }
        let mut labels = HashSet::new();
        for (k, row) in rows.iter().enumerate() {
            let path = format!("parameters[{i}].rows[{k}]");
            if row.label.trim().is_empty() || !labels.insert(name_key(&row.label)) {
                return at(
                    format!("{path}.label"),
                    format!("“{name}” tablosunun satır adları boş olamaz ve bir kez geçer"),
                );
            }
            if row.values.is_empty() {
                return at(
                    format!("{path}.values"),
                    format!("“{}” satırı hiçbir değer vermiyor", row.label),
                );
            }
            for (key, v) in &row.values {
                let Some(target) =
                    by_name(key).filter(|t| !matches!(t, BlockParameter::Lookup { .. }))
                else {
                    return at(
                        format!("{path}.values"),
                        format!(
                            "“{}” satırı olmayan ya da tablo olan “{key}” parametresine değer veriyor",
                            row.label
                        ),
                    );
                };
                if target.rule().is_some_and(|r| r.expression.is_some()) {
                    return at(
                        format!("{path}.values"),
                        format!("“{key}” ifadeyle hesaplanır; tablo ona değer veremez"),
                    );
                }
                if let Some(m) = value_problem(target, v) {
                    return at(
                        format!("{path}.values"),
                        format!("“{}” satırının “{key}” değeri: {m}", row.label),
                    );
                }
            }
        }
    }
    for (i, a) in d.actions.iter().enumerate() {
        let path = |f: &str| format!("actions[{i}]{f}");
        let Some(p) = by_name(a.parameter()) else {
            return at(
                path(".parameter"),
                format!("eylemin “{}” parametresi blokta yok", a.parameter()),
            );
        };
        if !a.fits(p) {
            return at(
                path(".parameter"),
                format!("{} eylemi {} parametresiyle çalışmaz", a.word(), p.word()),
            );
        }
        let pieces = a.pieces();
        if pieces.is_empty() || pieces.len() > MAX_DYNAMIC_ITEMS {
            return at(
                path(".pieces"),
                format!("eylem 1 ile {MAX_DYNAMIC_ITEMS} arası nesneye uygulanır"),
            );
        }
        let mut seen = HashSet::new();
        if let Some(id) = pieces
            .iter()
            .find(|id| !ids.contains(id) || !seen.insert(**id))
        {
            return at(
                path(".pieces"),
                format!("eylemin {id} numaralı nesnesi blokta yok ya da iki kez verilmiş"),
            );
        }
        match a {
            BlockAction::Stretch { min, max, .. }
                if !(finite(*min) && finite(*max) && min.x < max.x && min.y < max.y) =>
            {
                return at(
                    path(""),
                    "esnetmenin kutusu sonlu ve en küçük köşesi en büyüğünden küçük olmalı".into(),
                );
            }
            BlockAction::Array { spacing, .. } if !(spacing.is_finite() && *spacing > 0.0) => {
                return at(
                    path(".spacing"),
                    "dizinin aralığı artı bir sayı olmalı".into(),
                );
            }
            _ => {}
        }
    }
    None
}

/// The local ids of a definition's objects.
pub fn local_ids(b: &BlockDefinition) -> HashSet<u32> {
    b.entities.iter().map(|e| e.base().id).collect()
}

/// Why an insert's kept values are not its definition's: a name it has not, or a value that is not one of its.
pub fn values_problem(
    d: &DynamicBlock,
    values: &BTreeMap<String, ParamValue>,
) -> Option<(String, String)> {
    for (name, v) in values {
        let Some(p) = d.parameters.iter().find(|p| p.name() == name) else {
            return Some((name.clone(), format!("blokta “{name}” parametresi yok")));
        };
        if matches!(p, BlockParameter::Lookup { .. }) {
            return Some((
                name.clone(),
                format!("“{name}” bir tablo; yerleştirme onun değerini tutmaz"),
            ));
        }
        if let Some(m) = value_problem(p, v) {
            return Some((name.clone(), format!("“{name}”: {m}")));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(x: f64, y: f64) -> Vec2 {
        Vec2 { x, y }
    }

    fn door() -> DynamicBlock {
        DynamicBlock {
            parameters: vec![
                BlockParameter::Linear {
                    name: "Genişlik".into(),
                    from: v(0.0, 0.0),
                    to: v(0.9, 0.0),
                    rule: NumberRule {
                        min: Some(0.6),
                        max: Some(1.2),
                        step: Some(0.05),
                        ..Default::default()
                    },
                },
                BlockParameter::Flip {
                    name: "Yön".into(),
                    a: v(0.45, 0.0),
                    b: v(0.45, 1.0),
                },
                BlockParameter::Visibility {
                    name: "Görünüş".into(),
                    states: vec![
                        VisibilityState {
                            name: "Açılım".into(),
                            hidden: vec![],
                        },
                        VisibilityState {
                            name: "Sade".into(),
                            hidden: vec![2],
                        },
                    ],
                },
            ],
            actions: vec![BlockAction::Scale {
                parameter: "Genişlik".into(),
                pieces: vec![1, 2],
            }],
        }
    }

    #[test]
    fn a_door_is_a_dynamic_block_and_its_values_are_checked() {
        let ids: HashSet<u32> = [1, 2, 3].into();
        assert_eq!(dynamic_problem(&door(), &ids), None);
        let width = &door().parameters[0];
        assert_eq!(value_problem(width, &ParamValue::Number(1.0)), None);
        assert!(value_problem(width, &ParamValue::Number(1.02)).is_some());
        assert!(value_problem(width, &ParamValue::Number(1.25)).is_some());
        assert!(value_problem(width, &ParamValue::Flag(true)).is_some());
        let mut bad = door();
        bad.actions.push(BlockAction::Rotate {
            parameter: "Genişlik".into(),
            pieces: vec![1],
        });
        assert!(dynamic_problem(&bad, &ids).is_some());
        let mut bad = door();
        bad.actions[0] = BlockAction::Scale {
            parameter: "Genişlik".into(),
            pieces: vec![9],
        };
        assert!(dynamic_problem(&bad, &ids).is_some());
    }
}
