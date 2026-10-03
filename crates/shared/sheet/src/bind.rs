//! Data-defined properties (design §7): the one list of what can be bound to
//! an expression, which the interface shows as its “ƒ” buttons, and how a
//! bound value is applied to an item.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::kinds::{ItemKind, MapView};
use crate::model::{Item, VarValue};
use crate::units::{Um, norm_mdeg, round_i64, round_um};

/// What a bound value must be.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum BindType {
    Number,
    Text,
    Bool,
}

/// A property an expression can give.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct Bindable {
    pub property: String,
    /// The item kinds that have it (`type` names); empty: every kind.
    pub kinds: Vec<String>,
    pub value: BindType,
    /// The interface's name for it.
    pub label: String,
    /// The unit the expression gives it in: “mm”, “°”, “m”, “%”, or none.
    pub unit: String,
}

const TABLE: &[(&str, &[&str], BindType, &str, &str)] = &[
    ("frame.left", &[], BindType::Number, "Sol", "mm"),
    ("frame.top", &[], BindType::Number, "Üst", "mm"),
    ("frame.width", &[], BindType::Number, "Genişlik", "mm"),
    ("frame.height", &[], BindType::Number, "Yükseklik", "mm"),
    ("rotation", &[], BindType::Number, "Dönüş", "°"),
    ("hidden", &[], BindType::Bool, "Gizli", ""),
    ("opacity", &[], BindType::Number, "Saydamlık", "%"),
    ("text.content", &["text"], BindType::Text, "Metin", ""),
    ("map.scale", &["map"], BindType::Number, "Ölçek", ""),
    (
        "map.center.x",
        &["map"],
        BindType::Number,
        "Merkez Y (doğu)",
        "m",
    ),
    (
        "map.center.y",
        &["map"],
        BindType::Number,
        "Merkez X (kuzey)",
        "m",
    ),
    (
        "map.rotation",
        &["map"],
        BindType::Number,
        "Harita dönüşü",
        "°",
    ),
    (
        "map.grid.interval",
        &["map"],
        BindType::Number,
        "Karelaj aralığı",
        "m",
    ),
    (
        "legend.title",
        &["legend"],
        BindType::Text,
        "Lejant başlığı",
        "",
    ),
    (
        "table.filter",
        &["table"],
        BindType::Text,
        "Tablo süzgeci",
        "",
    ),
];

/// Every bindable property, in the interface's order.
pub fn bindable_properties() -> Vec<Bindable> {
    TABLE
        .iter()
        .map(|(p, k, v, l, u)| Bindable {
            property: (*p).to_owned(),
            kinds: k.iter().map(|s| (*s).to_owned()).collect(),
            value: *v,
            label: (*l).to_owned(),
            unit: (*u).to_owned(),
        })
        .collect()
}

/// The type of `property` on an item of `kind`; none: it cannot be bound there.
pub fn bind_type(property: &str, kind: &ItemKind) -> Option<BindType> {
    let k = kind.type_name();
    TABLE
        .iter()
        .find(|(p, kinds, ..)| *p == property && (kinds.is_empty() || kinds.contains(&k)))
        .map(|(_, _, t, ..)| *t)
}

fn number(v: &VarValue) -> Option<f64> {
    match v {
        VarValue::Number(x) if x.is_finite() => Some(*x),
        VarValue::Text(t) => t.trim().parse::<f64>().ok().filter(|x| x.is_finite()),
        VarValue::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
        _ => None,
    }
}

fn text(v: &VarValue) -> Option<String> {
    match v {
        VarValue::Null => None,
        VarValue::Text(t) => Some(t.clone()),
        VarValue::Number(x) => Some(kentos_expression::js::number::to_string(*x)),
        VarValue::Bool(b) => Some(if *b { "doğru" } else { "yanlış" }.to_owned()),
    }
}

fn truth(v: &VarValue) -> Option<bool> {
    match v {
        VarValue::Bool(b) => Some(*b),
        VarValue::Number(x) => Some(*x != 0.0),
        VarValue::Text(t) => Some(!t.is_empty()),
        VarValue::Null => None,
    }
}

fn mm(x: f64) -> Um {
    round_um(x * 1000.0)
}

/// Sets `property` of `item` to `value`; false when the value does not suit it (the item is left as it was).
pub fn apply(item: &mut Item, property: &str, value: &VarValue) -> bool {
    match property {
        "frame.left" => number(value).is_some_and(|x| {
            item.frame.left = mm(x);
            true
        }),
        "frame.top" => number(value).is_some_and(|x| {
            item.frame.top = mm(x);
            true
        }),
        "frame.width" => number(value).filter(|x| *x >= 1.0).is_some_and(|x| {
            item.frame.width = mm(x);
            true
        }),
        "frame.height" => number(value).filter(|x| *x >= 1.0).is_some_and(|x| {
            item.frame.height = mm(x);
            true
        }),
        "rotation" => number(value).is_some_and(|x| {
            item.rotation = norm_mdeg(round_i64(x * 1000.0));
            true
        }),
        "hidden" => truth(value).is_some_and(|b| {
            item.hidden = b;
            true
        }),
        "opacity" => number(value)
            .filter(|x| (0.0..=100.0).contains(x))
            .is_some_and(|x| {
                item.opacity = round_i64(x) as u8;
                true
            }),
        "text.content" => match (&mut item.kind, text(value)) {
            (ItemKind::Text(t), Some(s)) => {
                t.content = s;
                true
            }
            _ => false,
        },
        "map.scale" => match (&mut item.kind, number(value)) {
            (ItemKind::Map(m), Some(x)) if (1.0..=100_000_000.0).contains(&x) => {
                match &mut m.view {
                    MapView::Fixed(f) => {
                        f.scale = round_i64(x) as u32;
                        true
                    }
                    MapView::Atlas(_) => false,
                }
            }
            _ => false,
        },
        "map.center.x" | "map.center.y" => match (&mut item.kind, number(value)) {
            (ItemKind::Map(m), Some(x)) => match &mut m.view {
                MapView::Fixed(f) => {
                    let mut c = f.center.unwrap_or_default();
                    if property == "map.center.x" {
                        c.x = x;
                    } else {
                        c.y = x;
                    }
                    f.center = Some(c);
                    true
                }
                MapView::Atlas(_) => false,
            },
            _ => false,
        },
        "map.rotation" => match (&mut item.kind, number(value)) {
            (ItemKind::Map(m), Some(x)) => {
                let r = norm_mdeg(round_i64(x * 1000.0));
                match &mut m.view {
                    MapView::Fixed(f) => f.rotation = r,
                    MapView::Atlas(a) => a.rotation = r,
                }
                true
            }
            _ => false,
        },
        "map.grid.interval" => match (&mut item.kind, number(value)) {
            (ItemKind::Map(m), Some(x)) if x > 0.0 => {
                for g in &mut m.grids {
                    g.interval = [x, x];
                }
                true
            }
            _ => false,
        },
        "legend.title" => match (&mut item.kind, text(value)) {
            (ItemKind::Legend(l), Some(s)) => {
                l.title = s;
                true
            }
            _ => false,
        },
        "table.filter" => match (&mut item.kind, text(value)) {
            (ItemKind::Table(t), Some(s)) => match &mut t.source {
                crate::kinds::TableSource::Layer(l) => {
                    l.filter = s;
                    true
                }
                _ => false,
            },
            _ => false,
        },
        _ => false,
    }
}
