//! A number field's units (plan/eklenti.md ADIM 22, the inventory's "birim seçici"; docs/adr/8057 §64): `units` names
//! the units the user may type the number in besides the field's own, as KentOS's number fields take them (0,03 km,
//! 2 dönüm, 1 m + 20 cm). The field's `unit` is the one the code gets, written after the number; its `value`, `min`,
//! `max` and `step` are in it. The code never converts: 0,03 km typed is 30 m given.
//!
//! KentOS's units, as its settings and reports have them: lengths mm, cm, m, km; areas m², dönüm (1 000 m²) and ha
//! (10 000 m²); angles ° and grad (400 to the turn).

use serde_json::{Map, Value as Json};

/// What a unit measures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Quantity {
    Length,
    Area,
    Angle,
}

/// KentOS's units: each one's symbol, what it measures and how much of its quantity's first unit (m, m², °) it is.
pub const UNITS: [(&str, Quantity, f64); 9] = [
    ("mm", Quantity::Length, 0.001),
    ("cm", Quantity::Length, 0.01),
    ("m", Quantity::Length, 1.0),
    ("km", Quantity::Length, 1000.0),
    ("m²", Quantity::Area, 1.0),
    ("dönüm", Quantity::Area, 1000.0),
    ("ha", Quantity::Area, 10_000.0),
    ("°", Quantity::Angle, 1.0),
    ("grad", Quantity::Angle, 0.9),
];

/// The most units a field takes.
const MOST_UNITS: usize = 4;

/// KentOS's unit written `symbol`: its symbol as the catalog keeps it, what it measures and how much of its quantity's
/// first unit it is; none for a unit KentOS does not have.
pub fn unit(symbol: &str) -> Option<(&'static str, Quantity, f64)> {
    UNITS.iter().find(|(s, _, _)| *s == symbol).copied()
}

/// What `symbol` measures; none for a unit KentOS does not have.
pub fn quantity_of(symbol: &str) -> Option<Quantity> {
    unit(symbol).map(|(_, quantity, _)| quantity)
}

/// `x` to twelve significant digits: a number typed in another unit without the binary fractions' noise (1,1 km is
/// 1100 m, not 1100,0000000000002).
pub fn tidy(x: f64) -> f64 {
    if x == 0.0 || !x.is_finite() {
        return x;
    }
    let digits = 11 - x.abs().log10().floor() as i32;
    let scale = 10f64.powi(digits.clamp(-300, 300));
    (x * scale).round() / scale
}

/// Kapı 1's problems with a number field's `units` at `at` (`/body/<n>`), each with its JSON pointer: a list of two to
/// four of KentOS's units, each once, measuring the same thing, its `unit` among them.
pub fn check(component: &Map<String, Json>, at: &str) -> Vec<(String, String)> {
    let Some(units) = component.get("units") else {
        return Vec::new();
    };
    let place = format!("{at}/units");
    let known = || {
        UNITS
            .iter()
            .map(|(symbol, _, _)| *symbol)
            .collect::<Vec<_>>()
            .join(", ")
    };
    let names: Vec<&str> = units
        .as_array()
        .map(|list| list.iter().filter_map(Json::as_str).collect())
        .unwrap_or_default();
    if units
        .as_array()
        .is_none_or(|list| list.len() != names.len())
        || !(2..=MOST_UNITS).contains(&names.len())
    {
        return vec![(
            place,
            format!(
                "units, sayının yazılabileceği 2 ile {MOST_UNITS} birimin adları olmalı, alanın kendi birimi dahil (ör. \
                 [\"m\", \"km\"])."
            ),
        )];
    }
    let mut out = Vec::new();
    for (i, name) in names.iter().enumerate() {
        if quantity_of(name).is_none() {
            out.push((
                format!("{place}/{i}"),
                format!("“{name}” KentOS'un birimlerinden biri değil: {}.", known()),
            ));
        } else if names[..i].contains(name) {
            out.push((
                format!("{place}/{i}"),
                format!("“{name}” iki kez yazılmış."),
            ));
        }
    }
    if !out.is_empty() {
        return out;
    }
    if names
        .iter()
        .any(|name| quantity_of(name) != quantity_of(names[0]))
    {
        out.push((
            place,
            "units'in birimleri aynı şeyi ölçmeli: uzunluk (mm, cm, m, km), alan (m², dönüm, ha) ya da açı (°, grad)."
                .to_owned(),
        ));
    } else if !component
        .get("unit")
        .and_then(Json::as_str)
        .is_some_and(|unit| names.contains(&unit))
    {
        out.push((
            format!("{at}/unit"),
            "units verilince unit onlardan biri olmalı: kodun aldığı birim; value, min, max ve step onunla yazılır."
                .to_owned(),
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    /// A unit's size is in its kind's first unit; a number typed in another unit loses the binary fractions' noise.
    #[test]
    fn a_units_size_is_in_its_kinds_first_unit() {
        assert_eq!(unit("dönüm"), Some(("dönüm", Quantity::Area, 1000.0)));
        assert_eq!(unit("grad").map(|(_, _, size)| size), Some(0.9));
        assert_eq!(unit("inç"), None);
        assert_eq!(quantity_of("°"), Some(Quantity::Angle));
        assert_eq!(tidy(1.1 * 1000.0), 1100.0);
        assert_eq!(tidy(30.0 / 1000.0), 0.03);
        assert_eq!(tidy(0.0), 0.0);
    }

    /// Kapı 1: two to four of KentOS's units, each once, of one kind, the field's `unit` among them; each mistake
    /// said where it is written.
    #[test]
    fn a_fields_units_are_kentos_units_of_one_kind_with_its_own() {
        let said = |c: serde_json::Value| -> Vec<String> {
            check(c.as_object().expect("a component"), "/body/0")
                .into_iter()
                .map(|(at, _)| at)
                .collect()
        };
        assert!(said(json!({ "unit": "m", "units": ["m", "km"] })).is_empty());
        assert!(
            said(json!({ "unit": "m" })).is_empty(),
            "no units, nothing to say"
        );
        assert_eq!(
            said(json!({ "unit": "m", "units": ["m"] })),
            ["/body/0/units"]
        );
        assert_eq!(
            said(json!({ "unit": "m", "units": "km" })),
            ["/body/0/units"]
        );
        assert_eq!(
            said(json!({ "unit": "m", "units": ["m", "mil"] })),
            ["/body/0/units/1"]
        );
        assert_eq!(
            said(json!({ "unit": "m", "units": ["m", "m"] })),
            ["/body/0/units/1"]
        );
        assert_eq!(
            said(json!({ "unit": "m", "units": ["m", "ha"] })),
            ["/body/0/units"]
        );
        assert_eq!(
            said(json!({ "unit": "cm", "units": ["m", "km"] })),
            ["/body/0/unit"]
        );
        assert_eq!(said(json!({ "units": ["m", "km"] })), ["/body/0/unit"]);
    }
}
