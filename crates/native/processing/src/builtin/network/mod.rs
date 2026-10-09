//! Ağ analizi's İşlemler tools (docs/adr/0209 §6, §7, §10; the web's
//! `builtin/network/`): En yakın tesis, Maliyet matrisi and Hizmet alanları.
//! A run builds the network in a store of its own from its layers' objects as
//! the run sees them (`crate::network`); what the tools share is here: the
//! points of a features input found on the network, the names the results
//! say, and costs written as numbers.

pub mod closest_facility;
pub mod od_matrix;
pub mod service_areas;

use kentos_contracts::{Entity, NetworkDef};
use kentos_geometry_core::display::fixed;
use kentos_geometry_core::ops::network::NetworkSession;
use kentos_geometry_core::vec2::Vec2;
use serde_json::Value;

use crate::network::{FromDocument, from_document};
use crate::text::js_trim;
use crate::types::{Feedback, ParamDef, ParamKind, Resolved, RunContext, ScopeKind};

/// A network built for a run: its definition, the cost asked with (its place), its costs' names and the session.
pub struct RunNetwork {
    pub def: NetworkDef,
    pub cost: usize,
    pub cost_names: Vec<String>,
    pub session: NetworkSession,
}

/// The network parameter of the tools.
pub fn network_param(description: &str) -> ParamDef {
    ParamDef::new(
        "network",
        "Ağ",
        ParamKind::Network {
            prefers: kentos_contracts::NetworkKind::Road,
        },
    )
    .describe(description)
}

/// A points input of the tools.
pub fn points_param(name: &str, label: &str, description: Option<&str>) -> ParamDef {
    let p = ParamDef::new(
        name,
        label,
        ParamKind::Features {
            kinds: Some(vec!["point".to_owned()]),
            scopes: Some(vec![
                ScopeKind::Layer,
                ScopeKind::Selection,
                ScopeKind::Visible,
                ScopeKind::All,
            ]),
            writes: false,
        },
    );
    match description {
        Some(d) => p.describe(d),
        None => p,
    }
}

/// Arama uzaklığı.
pub fn reach_param(what: &str) -> ParamDef {
    ParamDef::new(
        "reach",
        "Arama uzaklığı",
        ParamKind::Number {
            min: Some(0.001),
            max: Some(100_000.0),
            integer: false,
            unit: "m".into(),
            placeholder: None,
        },
    )
    .default_value(serde_json::json!(100))
    .advanced()
    .describe(what)
}

/// Builds the run's network; says what building said (expressions that did not compile, objects not taken).
pub fn run_network(
    v: &Resolved<'_>,
    ctx: &RunContext<'_>,
    feedback: &mut dyn Feedback,
) -> Result<RunNetwork, String> {
    let value = v.values.get("network").cloned().unwrap_or(Value::Null);
    let id = value.get("network").and_then(Value::as_str).unwrap_or("");
    let cost = value.get("cost").and_then(Value::as_str).unwrap_or("");
    let FromDocument {
        def,
        session,
        problems,
    } = from_document(ctx.doc, id)?;
    for p in &problems {
        feedback.warn(format!("{}: {p}", def.name));
    }
    if !session.skipped.is_empty() {
        feedback.warn(format!(
            "{}: ağın katmanlarında {} nesne çizgi, çoklu çizgi, yay ya da nokta değil; ağa alınmadı.",
            def.name,
            session.skipped.len()
        ));
    }
    let cost_names: Vec<String> = def.cost_names().into_iter().map(str::to_owned).collect();
    let cost = cost_names.iter().position(|c| c == cost).unwrap_or(0);
    Ok(RunNetwork {
        def,
        cost,
        cost_names,
        session,
    })
}

/// A way as a polyline's geometry: its points, and its bulges when it has an arc (the web's `lineGeometry`).
pub fn line_geometry(
    l: &kentos_geometry_core::ops::network::Line,
) -> kentos_contracts::EntityGeometry {
    let arcs = l.bulges.iter().any(|b| *b != 0.0);
    kentos_contracts::EntityGeometry::Polyline {
        pts: l
            .pts
            .iter()
            .map(|p| kentos_contracts::Vec2 { x: p[0], y: p[1] })
            .collect(),
        bulges: arcs.then(|| l.bulges.clone()),
        zs: None,
        parts: None,
    }
}

/// A point object's place (its first point).
pub fn point_of(e: &Entity) -> Option<Vec2> {
    match e {
        Entity::Point(p) => Some(Vec2::new(p.p.x, p.p.y)),
        _ => None,
    }
}

/// The objects found on the network within `reach` (in their order, with their points) and those that are not.
pub fn places_of<'e>(
    s: &NetworkSession,
    objects: &[&'e Entity],
    reach: f64,
) -> (Vec<(&'e Entity, Vec2)>, Vec<&'e Entity>) {
    let mut found = Vec::new();
    let mut missing = Vec::new();
    for e in objects {
        match point_of(e).filter(|p| s.locate(*p, reach).is_some()) {
            Some(p) => found.push((*e, p)),
            None => missing.push(*e),
        }
    }
    (found, missing)
}

/// A point's name in the results: its label, its `Ad`, else its number.
pub fn name_of(e: &Entity) -> String {
    let b = e.base();
    b.label
        .as_deref()
        .map(js_trim)
        .filter(|l| !l.is_empty())
        .or_else(|| {
            b.attrs
                .get("Ad")
                .map(|a| js_trim(a))
                .filter(|a| !a.is_empty())
        })
        .map_or_else(|| format!("#{}", b.id), str::to_owned)
}

/// A cost's value as an attribute: the length with the project's decimals, the others with two; empty for none.
pub fn cost_value(v: Option<f64>, c: usize, length_decimals: u32) -> String {
    match v.filter(|v| v.is_finite()) {
        Some(v) => fixed(v, if c == 0 { length_decimals as usize } else { 2 }),
        None => String::new(),
    }
}

/// The note about points not found on the network.
pub fn missing_note(missing: usize, what: &str, reach: f64, feedback: &mut dyn Feedback) {
    if missing > 0 {
        feedback.warn(format!(
            "{missing} {what} ağa {} m içinde değil (ya da nokta değil); alınmadı.",
            fixed(reach, 2)
        ));
    }
}
