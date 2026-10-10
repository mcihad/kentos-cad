//! A network's input from the drawing (docs/adr/0209 §2, §3; the web's
//! `model/networkInput.ts`): which objects of its layers are its edges and
//! junctions, with what the app evaluates for them (the core gets only the
//! results): each edge layer's objects its filter holds for, in the
//! definition's order and each layer's document order, each object once;
//! their direction field's value, their costs' fields' values and whether the
//! closed edges' expression holds; each junction layer's points with its role
//! and whether its closed expression holds. A point on a layer that is both an
//! edge and a junction layer is a junction, a line on it an edge; any other
//! kind goes to the core, which says it by its kind. An expression's empty
//! answer is “no”; an expression that does not compile matches nothing and is
//! said. The desktop's network thread (its app) and Ağ analizi's İşlemler
//! tools build from it.

use std::collections::HashSet;

use kentos_contracts::{Entity, JunctionRole, NetworkDef, NetworkDirection};
use kentos_geometry_core::ops::network::input::{EdgeValues, JunctionValues, from_store};
use kentos_geometry_core::ops::network::{Graph, NetworkSession, Role, Rules};
use kentos_geometry_core::store::Store;
use kentos_style_core::expr::Value;
use kentos_style_core::expr::rows::As;

use crate::expression::evaluate_all;

/// The objects a network reads and what was evaluated for them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NetworkInput {
    pub edges: Vec<(f64, EdgeValues)>,
    pub junctions: Vec<(f64, JunctionValues)>,
    /// Expressions that did not compile, in words.
    pub problems: Vec<String>,
}

/// Where the objects come from: each layer's in document order, the layers' names, the geometry values of objects
/// for expressions (six numbers each, `kentos_style_core::expr::rows`).
pub struct Source<'a> {
    pub by_layer: &'a dyn Fn(&str) -> Vec<&'a Entity>,
    pub layer_name: &'a dyn Fn(&str) -> String,
    pub measures: &'a dyn Fn(&[&Entity]) -> Vec<f64>,
}

/// Which of `list` an expression holds for (true only); `none` for each when there is no expression (a filter takes
/// every object, a closed expression closes none).
fn holds(
    source: Option<&str>,
    none: bool,
    list: &[&Entity],
    src: &Source<'_>,
    what: &str,
    problems: &mut Vec<String>,
) -> Vec<bool> {
    let text = source.map(crate::text::js_trim).unwrap_or("");
    if text.is_empty() {
        return vec![none; list.len()];
    }
    // A network's filters look at their own objects only (docs/adr/0214 §1).
    let expr = match kentos_expression::compile(text) {
        Ok(e) => e,
        Err(e) => {
            problems.push(format!(
                "{what} değerlendirilemedi ({}); bu ifadeye hiçbir nesne uymuş sayılmadı.",
                e.text()
            ));
            return vec![false; list.len()];
        }
    };
    if list.is_empty() {
        return Vec::new();
    }
    let mut measures = || (src.measures)(list);
    evaluate_all(&expr, list, src.layer_name, &mut measures, As::Bool)
        .iter()
        .map(|v| *v == Value::Bool(true))
        .collect()
}

/// An attribute's value as the core reads it: the text, none when the object has no such attribute.
fn value(e: &Entity, field: &str) -> Option<String> {
    e.base().attrs.get(field).cloned()
}

/// The input of network `def` from `src`.
pub fn network_input(def: &NetworkDef, src: &Source<'_>) -> NetworkInput {
    let mut problems = Vec::new();
    let edge_layers: HashSet<&str> = def.edges.iter().map(|l| l.layer.as_str()).collect();
    let junction_layers: HashSet<&str> = def.junctions.iter().map(|l| l.layer.as_str()).collect();
    let mut taken: HashSet<u32> = HashSet::new();
    let mut edges: Vec<&Entity> = Vec::new();
    for (i, l) in def.edges.iter().enumerate() {
        // A point on a layer that is a junction layer too is that layer's junction.
        let junction = junction_layers.contains(l.layer.as_str());
        let a_junction = |e: &Entity| junction && matches!(e, Entity::Point(_));
        let list: Vec<&Entity> = (src.by_layer)(&l.layer)
            .into_iter()
            .filter(|e| !taken.contains(&e.base().id) && !a_junction(e))
            .collect();
        let ok = holds(
            l.filter.as_deref(),
            true,
            &list,
            src,
            &format!("{}. kenar katmanının süzgeci", i + 1),
            &mut problems,
        );
        for (e, k) in list.into_iter().zip(ok) {
            if k {
                taken.insert(e.base().id);
                edges.push(e);
            }
        }
    }
    let closed = holds(
        def.closed.as_deref(),
        false,
        &edges,
        src,
        "Kapalı kenarların ifadesi",
        &mut problems,
    );
    let field = match &def.direction {
        NetworkDirection::Field { field, .. } => Some(field.as_str()),
        _ => None,
    };
    let edges = edges
        .iter()
        .zip(closed)
        .map(|(e, closed)| {
            (
                f64::from(e.base().id),
                EdgeValues {
                    direction: field.and_then(|f| value(e, f)),
                    costs: def.costs.iter().map(|c| value(e, &c.field)).collect(),
                    closed,
                },
            )
        })
        .collect();
    let mut junctions = Vec::new();
    let mut taken_j: HashSet<u32> = HashSet::new();
    for (i, l) in def.junctions.iter().enumerate() {
        // A line on a layer that is an edge layer too is that layer's edge.
        let edge = edge_layers.contains(l.layer.as_str());
        let list: Vec<&Entity> = (src.by_layer)(&l.layer)
            .into_iter()
            .filter(|e| !taken_j.contains(&e.base().id) && (!edge || matches!(e, Entity::Point(_))))
            .collect();
        let ok = holds(
            l.filter.as_deref(),
            true,
            &list,
            src,
            &format!("{}. düğüm katmanının süzgeci", i + 1),
            &mut problems,
        );
        let mine: Vec<&Entity> = list
            .into_iter()
            .zip(ok)
            .filter(|(_, k)| *k)
            .map(|(e, _)| e)
            .collect();
        let shut = holds(
            l.closed.as_deref(),
            false,
            &mine,
            src,
            &format!("{}. düğüm katmanının kapalı ifadesi", i + 1),
            &mut problems,
        );
        let role = match l.role {
            JunctionRole::Source => Role::Source,
            JunctionRole::Valve => Role::Valve,
            JunctionRole::Junction => Role::Junction,
        };
        for (e, closed) in mine.into_iter().zip(shut) {
            taken_j.insert(e.base().id);
            junctions.push((f64::from(e.base().id), JunctionValues { role, closed }));
        }
    }
    NetworkInput {
        edges,
        junctions,
        problems,
    }
}

/// The network's rules (its connection, tolerance, direction and costs) as the core takes them.
pub fn rules_of(def: &NetworkDef) -> Result<Rules, String> {
    let text = serde_json::to_string(def).map_err(|e| e.to_string())?;
    Rules::from_json(
        &kentos_geometry_core::api::json::Json::parse(&text).map_err(|e| e.to_string())?,
    )
}

/// The network built from `input` over `store`'s objects (the processing tools, the network thread's builds).
pub fn build(
    def: &NetworkDef,
    store: &Store,
    input: &NetworkInput,
) -> Result<NetworkSession, String> {
    let rules = rules_of(def)?;
    let (e, j, skipped) = from_store(store, &input.edges, &input.junctions);
    let (graph, report) = Graph::build(rules, &e, &j);
    Ok(NetworkSession::new(graph, report, skipped))
}

/// A project's network built from a drawing as it is: its definition, the session, and the words of the expressions
/// that did not compile.
pub struct FromDocument {
    pub def: NetworkDef,
    pub session: NetworkSession,
    pub problems: Vec<String>,
}

/// Builds the project's network `id` from `doc`: its layers' objects in a geometry store of their own, the
/// definition's expressions evaluated (`network_input`), the graph built. The İşlemler tools and Python
/// (`kentos.network`) build so; the apps' interactive tools keep theirs built (docs/adr/0209 §12).
pub fn from_document(doc: &kentos_domain::Document, id: &str) -> Result<FromDocument, String> {
    let def = doc
        .settings()
        .networks
        .iter()
        .find(|n| n.id == id)
        .cloned()
        .ok_or_else(|| format!("“{id}” ağı projede yok; Ağlar penceresinden tanımlayın."))?;
    let objects: Vec<&Entity> = def.layers().iter().flat_map(|l| doc.by_layer(l)).collect();
    let mut store = Store::new();
    store.put_many(objects.iter().map(|e| crate::geometry::record(e)));
    let names: std::collections::HashMap<String, String> = doc
        .layers()
        .leaves()
        .into_iter()
        .map(|l| (l.id.clone(), l.name.clone()))
        .collect();
    let by = |l: &str| doc.by_layer(l).collect::<Vec<_>>();
    let name = |l: &str| names.get(l).cloned().unwrap_or_else(|| l.to_owned());
    let measures = |list: &[&Entity]| {
        store.measures(
            &list
                .iter()
                .map(|e| f64::from(e.base().id))
                .collect::<Vec<_>>(),
        )
    };
    let input = network_input(
        &def,
        &Source {
            by_layer: &by,
            layer_name: &name,
            measures: &measures,
        },
    );
    let session = build(&def, &store, &input)?;
    Ok(FromDocument {
        def,
        session,
        problems: input.problems,
    })
}
