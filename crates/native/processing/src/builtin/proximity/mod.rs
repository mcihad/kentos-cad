//! Yakınlık (docs/adr/0215; the web's `builtin/proximity/`): En yakını bul,
//! Uzaklık matrisi, En yakın merkeze bağla, Komşu alanlar and En kısa çizgi,
//! and what they share: the objects they take, the measure's choice, how an
//! object is named in a table or on a line, and how distances, areas and
//! bearings are written (the display rule, the project's decimals and angle
//! unit).

pub mod hub;
pub mod matrix;
pub mod nearest;
pub mod neighbors;
pub mod shortest_line;

use kentos_contracts::{AngleUnit, Entity};
use kentos_geometry_core::display::fixed;
use serde_json::json;

use super::queries::{AREA_KINDS, QUERY_KINDS};
use crate::geometry::Measure;
use crate::types::{Defaults, EnumOption, ParamDef, ParamKind, ScopeKind};

/// What can be measured: what Konuma göre seç takes (docs/adr/0200 §1).
pub const PROXIMITY_KINDS: [&str; 15] = QUERY_KINDS;
/// What can neighbour: the areas.
pub const NEIGHBOR_KINDS: [&str; 5] = AREA_KINDS;

/// The most rows a table of these tools has: past it the run is refused before it measures.
pub const MOST_ROWS: usize = 1_000_000;

/// The scopes the tools offer, a layer first.
pub fn scopes() -> Option<Vec<ScopeKind>> {
    Some(vec![
        ScopeKind::Layer,
        ScopeKind::Selection,
        ScopeKind::Visible,
        ScopeKind::All,
    ])
}

/// A features parameter of these tools.
pub fn features(
    name: &str,
    label: &str,
    kinds: &[&str],
    writes: bool,
    description: &str,
) -> ParamDef {
    ParamDef::new(
        name,
        label,
        ParamKind::Features {
            kinds: Some(kinds.iter().map(|k| (*k).to_owned()).collect()),
            scopes: scopes(),
            writes,
        },
    )
    .describe(description)
}

/// The measure's choice (docs/adr/0215 §2.1).
pub fn measure_param() -> ParamDef {
    ParamDef::new(
        "measure",
        "Ölçü",
        ParamKind::Choice {
            options: vec![
                EnumOption::new("edges", "Kenardan kenara")
                    .hint("En kısa uzaklık; değen, kesişen ya da içinde kalan 0"),
                EnumOption::new("centers", "Merkezden merkeze")
                    .hint("Ağırlık merkezleri (alan) ya da yer noktaları arası"),
            ],
        },
    )
    .default_value(json!("edges"))
}

/// The measure a value names.
pub fn measure_of(key: &str) -> Measure {
    if key == "centers" {
        Measure::Centers
    } else {
        Measure::Edges
    }
}

/// A distance bound, metres; 0: none.
pub fn max_param() -> ParamDef {
    number("max", "En çok uzaklık", Some(0.0), None, false, "m")
        .default_value(json!(0))
        .describe("0: sınırsız. Daha uzaktaki hedef alınmaz.")
}

/// A number parameter.
pub fn number(
    name: &str,
    label: &str,
    min: Option<f64>,
    max: Option<f64>,
    integer: bool,
    unit: &str,
) -> ParamDef {
    ParamDef::new(
        name,
        label,
        ParamKind::Number {
            min,
            max,
            integer,
            unit: unit.into(),
            placeholder: None,
        },
    )
}

/// A field parameter of `of`'s objects.
pub fn field(name: &str, label: &str, of: &str, allow_new: bool, multiple: bool) -> ParamDef {
    ParamDef::new(
        name,
        label,
        ParamKind::Field {
            of: vec![of.into()],
            allow_new,
            multiple,
        },
    )
}

/// An object's name (docs/adr/0215 §2.3): its field's value, else its label,
/// else its place in its list from 1.
pub fn name_of(e: &Entity, field: &str, place: usize) -> String {
    let value = if field.is_empty() {
        ""
    } else {
        e.base()
            .attrs
            .get(field)
            .map(|v| crate::text::js_trim(v))
            .unwrap_or("")
    };
    if !value.is_empty() {
        return value.to_owned();
    }
    let label = crate::text::js_trim(e.base().label.as_deref().unwrap_or(""));
    if label.is_empty() {
        (place + 1).to_string()
    } else {
        label.to_owned()
    }
}

/// A length as the project writes it: its length decimals, the display rule (docs/adr/0149).
pub fn length_text(units: &Defaults, d: f64) -> String {
    fixed(d, units.length_decimals as usize)
}

/// An area as the project writes it.
pub fn area_text(units: &Defaults, a: f64) -> String {
    fixed(a, units.area_decimals as usize)
}

/// A bearing (radians, clockwise from north) in the project's angle unit with four decimals.
pub fn bearing_text(units: &Defaults, t: f64) -> String {
    let pi = std::f64::consts::PI;
    let v = match units.angle_unit {
        AngleUnit::Deg => (t * 180.0) / pi,
        AngleUnit::Grad => (t * 200.0) / pi,
    };
    fixed(v, 4)
}

/// The bound a typed maximum gives: none for 0 or less.
pub fn bound(max: f64) -> f64 {
    if max > 0.0 { max } else { f64::INFINITY }
}
