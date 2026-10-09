//! Ağlar's form (docs/adr/0209 §10; the web's `model/networkForm.ts`): a
//! network as its window shows and takes it, everything as typed. The
//! tolerance in the project's length unit; the direction's values a list
//! with commas or semicolons; a speed cost's default speed (km/sa) empty for
//! 50. `def_of` turns it into the contract's network or says, in words, what
//! is wrong (the form's own words, then the contract's). New networks take
//! their kind's defaults. Both are held to fixtures/network/v1/form.json
//! (scripts/fixtures/network_form_cases.py).

use kentos_contracts::{
    JunctionLayer, JunctionRole, NETWORK_TOLERANCE, NetworkConnect, NetworkCost, NetworkCostKind,
    NetworkDef, NetworkDirection, NetworkKind, NetworkLayer,
};
use kentos_geometry_core::tools::point_text::js_trim;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EdgeRow {
    pub layer: String,
    pub filter: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JunctionRow {
    pub layer: String,
    pub role: JunctionRole,
    pub filter: String,
    pub closed: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CostRow {
    pub name: String,
    pub kind: NetworkCostKind,
    pub field: String,
    pub unit: String,
    pub speed: String,
}

/// Yön as the form offers it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DirectionKind {
    Both,
    Digitized,
    Field,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NetworkForm {
    pub id: String,
    pub name: String,
    pub kind: NetworkKind,
    pub edges: Vec<EdgeRow>,
    pub junctions: Vec<JunctionRow>,
    pub connect: NetworkConnect,
    pub tolerance: String,
    pub direction: DirectionKind,
    pub field: String,
    pub forward: String,
    pub backward: String,
    pub shut: String,
    pub costs: Vec<CostRow>,
    pub closed: String,
}

/// A speed cost's default speed when none is typed, km/sa.
pub const DEFAULT_SPEED: f64 = 50.0;

/// A field's values when Yön takes them from a field and none are typed yet: the field, forward, backward, closed.
pub const DIRECTION_DEFAULTS: [&str; 4] = ["yon", "FT, ileri", "TF, geri", "N, kapalı"];

/// A number as typed: a dot or a comma for the decimals (`5`, `5,25`, `.5`, `5.`); none when it is not one.
pub fn decimal(text: &str) -> Option<f64> {
    let t = js_trim(text);
    let body = t.strip_prefix(['+', '-']).unwrap_or(t);
    let digits = |s: &str| s.chars().all(|c| c.is_ascii_digit());
    let ok = match body.find(['.', ',']) {
        None => !body.is_empty() && digits(body),
        Some(i) => {
            let (int, frac) = (&body[..i], &body[i + 1..]);
            digits(int) && digits(frac) && !(int.is_empty() && frac.is_empty())
        }
    };
    ok.then(|| t.replace(',', ".").parse().ok()).flatten()
}

/// A number as the form shows it: twelve significant digits (no noise from a unit's conversion), as JavaScript's
/// `String(Number(v.toPrecision(12)))`.
pub fn shown(v: f64) -> String {
    let n: f64 = format!("{v:.11e}").parse().unwrap_or(v);
    kentos_project::new_project::js_number(n)
}

/// A list as typed: split at commas and semicolons, trimmed, empty ones dropped.
pub fn values_of(text: &str) -> Vec<String> {
    text.split([',', ';'])
        .map(js_trim)
        .filter(|v| !v.is_empty())
        .map(str::to_owned)
        .collect()
}

fn joined(list: &[String]) -> String {
    list.join(", ")
}

/// A network as the form shows it; `from_metres` gives the project's length unit.
pub fn form_of(def: &NetworkDef, from_metres: impl Fn(f64) -> f64) -> NetworkForm {
    let (direction, field, forward, backward, shut) = match &def.direction {
        NetworkDirection::Both => (
            DirectionKind::Both,
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        ),
        NetworkDirection::Digitized => (
            DirectionKind::Digitized,
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        ),
        NetworkDirection::Field {
            field,
            forward,
            backward,
            closed,
        } => (
            DirectionKind::Field,
            field.clone(),
            joined(forward),
            joined(backward),
            joined(closed),
        ),
    };
    NetworkForm {
        id: def.id.clone(),
        name: def.name.clone(),
        kind: def.kind,
        edges: def
            .edges
            .iter()
            .map(|e| EdgeRow {
                layer: e.layer.clone(),
                filter: e.filter.clone().unwrap_or_default(),
            })
            .collect(),
        junctions: def
            .junctions
            .iter()
            .map(|j| JunctionRow {
                layer: j.layer.clone(),
                role: j.role,
                filter: j.filter.clone().unwrap_or_default(),
                closed: j.closed.clone().unwrap_or_default(),
            })
            .collect(),
        connect: def.connect,
        tolerance: shown(from_metres(def.tolerance)),
        direction,
        field,
        forward,
        backward,
        shut,
        costs: def
            .costs
            .iter()
            .map(|c| CostRow {
                name: c.name.clone(),
                kind: c.kind,
                field: c.field.clone(),
                unit: c.unit.clone(),
                speed: c.speed.map(shown).unwrap_or_default(),
            })
            .collect(),
        closed: def.closed.clone().unwrap_or_default(),
    }
}

/// A road network's first cost: Süre from the speed field `hiz`, 50 km/sa.
fn road_cost() -> CostRow {
    CostRow {
        name: "Süre".into(),
        kind: NetworkCostKind::Speed,
        field: "hiz".into(),
        unit: String::new(),
        speed: "50".into(),
    }
}

/// A new network of `kind` named `name` with id `id`, its first edge layer `layer`: a road network both ways with
/// Süre from the speed field `hiz` (50 km/sa); a utility network in the direction its pipes were drawn, no costs.
pub fn new_form(
    id: &str,
    name: &str,
    kind: NetworkKind,
    layer: &str,
    from_metres: impl Fn(f64) -> f64,
) -> NetworkForm {
    NetworkForm {
        id: id.to_owned(),
        name: name.to_owned(),
        kind,
        edges: if layer.is_empty() {
            Vec::new()
        } else {
            vec![EdgeRow {
                layer: layer.to_owned(),
                filter: String::new(),
            }]
        },
        junctions: Vec::new(),
        connect: NetworkConnect::Ends,
        tolerance: shown(from_metres(NETWORK_TOLERANCE)),
        direction: if kind == NetworkKind::Road {
            DirectionKind::Both
        } else {
            DirectionKind::Digitized
        },
        field: String::new(),
        forward: String::new(),
        backward: String::new(),
        shut: String::new(),
        costs: if kind == NetworkKind::Road {
            vec![road_cost()]
        } else {
            Vec::new()
        },
        closed: String::new(),
    }
}

/// The first letter upper case, a full stop at the end: the contract's words as a sentence.
fn sentence(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => format!(
            "{}{}.",
            crate::prompt::upper_tr(&c.to_string()),
            chars.as_str()
        ),
        None => String::new(),
    }
}

fn opt(s: &str) -> Option<String> {
    let t = js_trim(s);
    (!t.is_empty()).then(|| t.to_owned())
}

/// The network the form writes, or what is wrong in words: a tolerance or a speed that is not a number (the form's
/// words), then the contract's rules (as a sentence). `to_metres`: the project's length unit to metres.
pub fn def_of(form: &NetworkForm, to_metres: impl Fn(f64) -> f64) -> Result<NetworkDef, String> {
    let name = js_trim(&form.name).to_owned();
    let who = if name.is_empty() {
        "Ağın".to_owned()
    } else {
        format!("“{name}” ağının")
    };
    let Some(t) = decimal(&form.tolerance) else {
        return Err(format!("{who} toleransı bir sayı olmalı (ör. 0,01)."));
    };
    let mut costs = Vec::new();
    for c in &form.costs {
        let cost = js_trim(&c.name).to_owned();
        match c.kind {
            NetworkCostKind::Speed => {
                let typed = js_trim(&c.speed);
                let speed = if typed.is_empty() {
                    Some(DEFAULT_SPEED)
                } else {
                    decimal(typed)
                };
                let Some(speed) = speed else {
                    return Err(format!(
                        "{who} “{cost}” maliyetinin hızı bir sayı olmalı (km/sa)."
                    ));
                };
                costs.push(NetworkCost {
                    name: cost,
                    kind: NetworkCostKind::Speed,
                    field: js_trim(&c.field).to_owned(),
                    unit: String::new(),
                    speed: Some(speed),
                });
            }
            NetworkCostKind::Field => costs.push(NetworkCost {
                name: cost,
                kind: NetworkCostKind::Field,
                field: js_trim(&c.field).to_owned(),
                unit: js_trim(&c.unit).to_owned(),
                speed: None,
            }),
        }
    }
    let def = NetworkDef {
        id: form.id.clone(),
        name,
        kind: form.kind,
        edges: form
            .edges
            .iter()
            .map(|e| NetworkLayer {
                layer: e.layer.clone(),
                filter: opt(&e.filter),
            })
            .collect(),
        junctions: form
            .junctions
            .iter()
            .map(|j| JunctionLayer {
                layer: j.layer.clone(),
                role: j.role,
                filter: opt(&j.filter),
                closed: opt(&j.closed),
            })
            .collect(),
        connect: form.connect,
        tolerance: to_metres(t),
        direction: match form.direction {
            DirectionKind::Both => NetworkDirection::Both,
            DirectionKind::Digitized => NetworkDirection::Digitized,
            DirectionKind::Field => NetworkDirection::Field {
                field: js_trim(&form.field).to_owned(),
                forward: values_of(&form.forward),
                backward: values_of(&form.backward),
                closed: values_of(&form.shut),
            },
        },
        costs,
        closed: opt(&form.closed),
    };
    match def.problem() {
        Some(wrong) => Err(sentence(&wrong)),
        None => Ok(def),
    }
}

/// The expressions of a form, each with its place in words (the window compiles them before Kaydet).
pub fn form_expressions(form: &NetworkForm) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for (i, e) in form.edges.iter().enumerate() {
        if let Some(t) = opt(&e.filter) {
            out.push((format!("{}. kenar katmanının süzgeci", i + 1), t));
        }
    }
    for (i, j) in form.junctions.iter().enumerate() {
        if let Some(t) = opt(&j.filter) {
            out.push((format!("{}. düğüm katmanının süzgeci", i + 1), t));
        }
        if let Some(t) = opt(&j.closed) {
            out.push((format!("{}. düğüm katmanının kapalı ifadesi", i + 1), t));
        }
    }
    if let Some(t) = opt(&form.closed) {
        out.push(("Kapalı kenarların ifadesi".to_owned(), t));
    }
    out
}

/// Whether a form's costs are a new road network's (Süre from `hiz`, 50 km/sa).
fn road_costs(f: &NetworkForm) -> bool {
    match &f.costs[..] {
        [c] => {
            let speed = if c.speed.is_empty() { "50" } else { &c.speed };
            c.name == "Süre"
                && c.kind == NetworkCostKind::Speed
                && c.field == "hiz"
                && decimal(speed) == Some(DEFAULT_SPEED)
        }
        _ => false,
    }
}

/// The form with another kind: what is still the old kind's default becomes the new kind's (the direction: both ways
/// or as drawn; the costs: a road network's Süre or none); what was changed stays.
pub fn retyped(f: &NetworkForm, kind: NetworkKind) -> NetworkForm {
    let mut out = f.clone();
    if f.kind == kind {
        return out;
    }
    out.kind = kind;
    if kind == NetworkKind::Utility {
        if f.direction == DirectionKind::Both {
            out.direction = DirectionKind::Digitized;
        }
        if road_costs(f) {
            out.costs.clear();
        }
    } else {
        if f.direction == DirectionKind::Digitized {
            out.direction = DirectionKind::Both;
        }
        if f.costs.is_empty() {
            out.costs = vec![road_cost()];
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn fixture() -> Value {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../fixtures/network/v1/form.json"
        );
        serde_json::from_str(&std::fs::read_to_string(path).expect("the cases")).expect("JSON")
    }

    fn text(v: &Value, key: &str) -> String {
        v[key].as_str().unwrap_or_default().to_owned()
    }

    fn enum_of<T: serde::de::DeserializeOwned>(v: &Value) -> T {
        serde_json::from_value(v.clone()).expect("a value of the contract")
    }

    fn form(v: &Value) -> NetworkForm {
        let list = |key: &str| v[key].as_array().cloned().unwrap_or_default();
        NetworkForm {
            id: text(v, "id"),
            name: text(v, "name"),
            kind: enum_of(&v["kind"]),
            edges: list("edges")
                .iter()
                .map(|e| EdgeRow {
                    layer: text(e, "layer"),
                    filter: text(e, "filter"),
                })
                .collect(),
            junctions: list("junctions")
                .iter()
                .map(|j| JunctionRow {
                    layer: text(j, "layer"),
                    role: enum_of(&j["role"]),
                    filter: text(j, "filter"),
                    closed: text(j, "closed"),
                })
                .collect(),
            connect: enum_of(&v["connect"]),
            tolerance: text(v, "tolerance"),
            direction: match v["direction"].as_str() {
                Some("field") => DirectionKind::Field,
                Some("digitized") => DirectionKind::Digitized,
                _ => DirectionKind::Both,
            },
            field: text(v, "field"),
            forward: text(v, "forward"),
            backward: text(v, "backward"),
            shut: text(v, "shut"),
            costs: list("costs")
                .iter()
                .map(|c| CostRow {
                    name: text(c, "name"),
                    kind: enum_of(&c["kind"]),
                    field: text(c, "field"),
                    unit: text(c, "unit"),
                    speed: text(c, "speed"),
                })
                .collect(),
            closed: text(v, "closed"),
        }
    }

    fn per_metre(unit: &str) -> f64 {
        match unit {
            "cm" => 100.0,
            "mm" => 1000.0,
            _ => 1.0,
        }
    }

    /// What a form writes in a project's unit, the form's own words, the kind's defaults and networks shown and
    /// written back, as fixtures/network/v1/form.json says them.
    #[test]
    fn the_form_writes_what_the_shared_cases_say() {
        let f = fixture();
        assert_eq!(f["format"], "kentos.network-form");
        for c in f["cases"].as_array().expect("cases") {
            let name = text(c, "name");
            let unit = per_metre(c["unit"].as_str().unwrap_or("m"));
            let got = def_of(&form(&c["form"]), |v| v / unit);
            match c.get("problem").and_then(Value::as_str) {
                Some(words) => assert_eq!(got, Err(words.to_owned()), "{name}"),
                None => assert_eq!(got, Ok(enum_of::<NetworkDef>(&c["network"])), "{name}"),
            }
        }
        for c in f["retype"].as_array().expect("retypes") {
            let name = text(c, "name");
            assert_eq!(
                retyped(&form(&c["form"]), enum_of(&c["kind"])),
                form(&c["expect"]),
                "{name}"
            );
        }
        for n in f["roundTrip"].as_array().expect("networks") {
            let def: NetworkDef = enum_of(n);
            assert_eq!(
                def_of(&form_of(&def, |m| m), |v| v),
                Ok(def.clone()),
                "{}",
                def.name
            );
        }
    }
}
