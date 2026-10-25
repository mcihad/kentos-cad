//! `cad.datasets.describe` v1 (docs/adr/0633): the drawing's layers as datasets, read only. What the description
//! does not keep is worked out whenever asked:
//!
//! - the objects and their kinds, their box in the project's system and, in a project with a system, in WGS 84
//!   (the box's sides densified, `crs::transform_bounds`);
//! - the system, the system the objects came from, the bound table the layer is a working copy of;
//! - the data dictionary: each field's name, alias, kind, unit, whether it is required, how many values its list
//!   allows and how many objects have a value in it;
//! - for a layer a processing tool made, its derivation record and each input's state now: its content digest
//!   (`sources::layer_digest`) the same as when read (`current`), not (`changed`) or the layer gone (`gone`); the layer
//!   is stale when an input is not current or a layer it came from is stale itself;
//! - the layers it came from (through their own derivations) and the layers made from it, nearest first, each once.
//!
//! With `records` each layer is also given as an OGC API Records record (a GeoJSON Feature of the record core).

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

use kentos_commands::{
    CommandResult, DatasetDescription, DatasetsDescribe, DatasetsDescribed, DerivationInputNow,
    DictionaryEntry, InputState, LayerNode, LayerNodeType,
};
use kentos_domain::Document;
use serde_json::{Value, json};

use crate::ExecutionContext;
use crate::checks::{Stop, error};
use crate::codes;

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// The layers asked for, in the order given; every layer holding objects when none is.
fn targets<'d>(doc: &'d Document, input: &DatasetsDescribe) -> Result<Vec<&'d LayerNode>, Stop> {
    if input.layers.is_empty() {
        return Ok(doc
            .layers()
            .leaves()
            .into_iter()
            .filter(|n| n.kind == LayerNodeType::Layer && n.service.is_none())
            .collect());
    }
    let mut out = Vec::new();
    for (i, id) in input.layers.iter().enumerate() {
        let at = format!("layers.{i}");
        let Some(node) = doc.layers().get(id) else {
            return Err(fail(
                codes::LAYER_NOT_FOUND,
                format!(
                    "“{id}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin."
                ),
                &at,
            ));
        };
        if node.kind != LayerNodeType::Layer {
            return Err(fail(
                codes::NOT_A_LAYER,
                format!(
                    "“{}” bir katman grubu; veri kümesi yalnız katmanın olur. Grubun bir katmanını verin.",
                    node.name
                ),
                &at,
            ));
        }
        out.push(node);
    }
    Ok(out)
}

/// The layers' content digests, each taken once.
struct Digests<'d> {
    doc: &'d Document,
    known: HashMap<String, String>,
}

impl Digests<'_> {
    fn of(&mut self, layer: &str) -> &str {
        if !self.known.contains_key(layer) {
            let d = crate::sources::layer_digest(self.doc, layer);
            self.known.insert(layer.to_owned(), d);
        }
        &self.known[layer]
    }
}

/// Each input's state now: gone, its digest the same or not.
fn inputs_now(
    doc: &Document,
    node: &LayerNode,
    digests: &mut Digests<'_>,
) -> Vec<DerivationInputNow> {
    let Some(d) = &node.derivation else {
        return Vec::new();
    };
    d.inputs
        .iter()
        .map(|i| {
            let state = match doc.layers().get(&i.layer) {
                Some(n) if n.kind == LayerNodeType::Layer => {
                    if digests.of(&i.layer) == i.digest {
                        InputState::Current
                    } else {
                        InputState::Changed
                    }
                }
                _ => InputState::Gone,
            };
            DerivationInputNow {
                layer: i.layer.clone(),
                name: i.name.clone(),
                state,
            }
        })
        .collect()
}

/// Whether `id` is stale: an input of it not current, or a layer it came from stale; each layer looked at once.
fn stale(
    doc: &Document,
    id: &str,
    digests: &mut Digests<'_>,
    seen: &mut HashMap<String, bool>,
) -> bool {
    if let Some(s) = seen.get(id) {
        return *s;
    }
    // A cycle (a layer read by what it made, written over) is not stale through itself.
    seen.insert(id.to_owned(), false);
    let Some(node) = doc.layers().get(id) else {
        return false;
    };
    let now = inputs_now(doc, node, digests);
    let mut out = now.iter().any(|i| i.state != InputState::Current);
    if !out {
        for i in &now {
            if stale(doc, &i.layer, digests, seen) {
                out = true;
                break;
            }
        }
    }
    seen.insert(id.to_owned(), out);
    out
}

/// The layers `id` came from, nearest first, each once.
fn upstream(doc: &Document, id: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut queue: VecDeque<String> = VecDeque::from([id.to_owned()]);
    let mut seen: HashSet<String> = HashSet::from([id.to_owned()]);
    while let Some(at) = queue.pop_front() {
        let Some(d) = doc.layers().get(&at).and_then(|n| n.derivation.as_ref()) else {
            continue;
        };
        for i in &d.inputs {
            if seen.insert(i.layer.clone()) {
                out.push(i.layer.clone());
                queue.push_back(i.layer.clone());
            }
        }
    }
    out
}

/// The layers made from `id`, nearest first, each once.
fn downstream(made_from: &HashMap<&str, Vec<&str>>, id: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut queue: VecDeque<&str> = VecDeque::from([id]);
    let mut seen: HashSet<&str> = HashSet::from([id]);
    while let Some(at) = queue.pop_front() {
        for m in made_from.get(at).into_iter().flatten() {
            if seen.insert(m) {
                out.push((*m).to_owned());
                queue.push_back(m);
            }
        }
    }
    out
}

/// The data dictionary of a layer's fields.
fn dictionary(doc: &Document, node: &LayerNode) -> Vec<DictionaryEntry> {
    let domains = &doc.settings().domains;
    node.fields
        .iter()
        .map(|f| {
            let choices = f.values.as_ref().map(|v| v.len() as u32).or_else(|| {
                let id = f.domain.as_deref()?;
                domains
                    .iter()
                    .find(|d| d.id == id)
                    .and_then(|d| d.values.as_ref())
                    .map(|v| v.len() as u32)
            });
            let filled = doc
                .by_layer(&node.id)
                .filter(|e| {
                    e.base()
                        .attrs
                        .get(&f.name)
                        .is_some_and(|v| !v.trim().is_empty())
                })
                .count() as u32;
            DictionaryEntry {
                name: f.name.clone(),
                alias: f.alias.clone(),
                kind: f.kind,
                required: f.required,
                unit: f.unit.clone(),
                choices,
                filled,
            }
        })
        .collect()
}

/// The objects' box in the project's system.
fn extent(doc: &Document, layer: &str) -> Option<[f64; 4]> {
    let mut b: Option<[f64; 4]> = None;
    for e in doc.by_layer(layer) {
        let s = crate::geometry::shape(e);
        let x = kentos_geometry_core::entity::entity_bounds(&s);
        if !(x.min_x.is_finite()
            && x.max_x.is_finite()
            && x.min_y.is_finite()
            && x.max_y.is_finite())
        {
            continue;
        }
        b = Some(match b {
            None => [x.min_x, x.min_y, x.max_x, x.max_y],
            Some(a) => [
                a[0].min(x.min_x),
                a[1].min(x.min_y),
                a[2].max(x.max_x),
                a[3].max(x.max_y),
            ],
        });
    }
    b
}

/// The layer as an OGC API Records record: a GeoJSON Feature of the record core.
fn record_of(d: &DatasetDescription) -> Value {
    let info = d.dataset.clone().unwrap_or_default();
    let geometry = d.geographic.map_or(Value::Null, |[w, s, e, n]| {
        json!({ "type": "Polygon", "coordinates": [[[w, s], [e, s], [e, n], [w, n], [w, s]]] })
    });
    let mut props = serde_json::Map::new();
    props.insert("type".into(), json!("dataset"));
    props.insert("title".into(), json!(d.title));
    if let Some(t) = &info.description {
        props.insert("description".into(), json!(t));
    }
    if !info.keywords.is_empty() {
        props.insert("keywords".into(), json!(info.keywords));
    }
    props.insert("language".into(), json!({ "code": "tr" }));
    if let Some(p) = &info.responsible {
        let mut c = serde_json::Map::new();
        if let Some(n) = &p.name {
            c.insert("name".into(), json!(n));
        }
        if let Some(o) = &p.organisation {
            c.insert("organization".into(), json!(o));
        }
        if let Some(e) = &p.email {
            c.insert("emails".into(), json!([{ "value": e }]));
        }
        c.insert("roles".into(), json!(["pointOfContact"]));
        props.insert("contacts".into(), json!([c]));
    }
    if let Some(l) = &info.license {
        props.insert("license".into(), json!(l));
    }
    if let Some(r) = &info.constraints {
        props.insert("rights".into(), json!(r));
    }
    if let Some(u) = &info.updated {
        props.insert("updated".into(), json!(u));
    }
    if let Some(s) = d.srid {
        props.insert(
            "kentos:crs".into(),
            json!(format!("http://www.opengis.net/def/crs/EPSG/0/{s}")),
        );
    }
    for (key, v) in [
        ("kentos:scale", info.scale.map(f64::from)),
        ("kentos:resolution", info.resolution),
        ("kentos:accuracy", info.accuracy),
    ] {
        if let Some(v) = v {
            props.insert(key.into(), json!(v));
        }
    }
    props.insert("kentos:objects".into(), json!(d.objects));
    if let Some(dv) = &d.derivation {
        props.insert(
            "kentos:derivedFrom".into(),
            json!({ "tool": dv.tool, "inputs": dv.inputs.iter().map(|i| &i.layer).collect::<Vec<_>>(), "stale": d.stale }),
        );
    }
    let mut out = serde_json::Map::new();
    out.insert("id".into(), json!(d.layer));
    out.insert("type".into(), json!("Feature"));
    out.insert(
        "conformsTo".into(),
        json!(["http://www.opengis.net/spec/ogcapi-records-1/1.0/conf/record-core"]),
    );
    out.insert("geometry".into(), geometry);
    out.insert(
        "time".into(),
        info.updated
            .as_ref()
            .map_or(Value::Null, |u| json!({ "date": u })),
    );
    out.insert("properties".into(), Value::Object(props));
    out.insert("links".into(), json!([]));
    Value::Object(out)
}

fn answer(doc: &Document, input: &DatasetsDescribe) -> Result<DatasetsDescribed, Stop> {
    let nodes = targets(doc, input)?;
    let settings = doc.settings();
    let own = kentos_project::systems::own(settings);
    let crs = own.as_ref().map_or_else(
        || "Yerel (koordinat sistemi yok)".to_owned(),
        |n| n.title.clone(),
    );
    let srid = (settings.srid != 0).then_some(settings.srid);
    // Who was made from whom, over the whole tree.
    let leaves = doc.layers().leaves();
    let mut made_from: HashMap<&str, Vec<&str>> = HashMap::new();
    for n in &leaves {
        for i in n.derivation.iter().flat_map(|d| &d.inputs) {
            made_from
                .entry(i.layer.as_str())
                .or_default()
                .push(n.id.as_str());
        }
    }
    let mut digests = Digests {
        doc,
        known: HashMap::new(),
    };
    let mut seen: HashMap<String, bool> = HashMap::new();
    let mut out = Vec::with_capacity(nodes.len());
    for node in nodes {
        let mut kinds: BTreeMap<String, u32> = BTreeMap::new();
        let mut objects = 0u32;
        for e in doc.by_layer(&node.id) {
            objects += 1;
            *kinds.entry(e.kind().to_owned()).or_default() += 1;
        }
        let ext = extent(doc, &node.id);
        let geographic = ext.and_then(|b| crate::sources::wgs84_bounds(settings, b));
        let source = settings
            .sources
            .iter()
            .find(|s| s.layer == node.id)
            .map(|s| format!("{} › {}", s.path, s.table));
        let inputs = inputs_now(doc, node, &mut digests);
        let stale = node.derivation.is_some() && stale(doc, &node.id, &mut digests, &mut seen);
        let mut d = DatasetDescription {
            layer: node.id.clone(),
            name: node.name.clone(),
            path: doc.layers().path(&node.id),
            title: node
                .dataset
                .as_ref()
                .and_then(|d| d.title.clone())
                .unwrap_or_else(|| node.name.clone()),
            dataset: node.dataset.clone(),
            objects,
            kinds,
            extent: ext,
            geographic,
            srid,
            crs: crs.clone(),
            source_crs: node.source_crs.as_ref().map(|c| c.name.clone()),
            source,
            dictionary: dictionary(doc, node),
            derivation: node.derivation.clone(),
            inputs,
            stale,
            upstream: upstream(doc, &node.id),
            downstream: downstream(&made_from, &node.id),
            record: None,
        };
        if input.records {
            d.record = Some(record_of(&d));
        }
        out.push(d);
    }
    Ok(DatasetsDescribed {
        datasets: out,
        revision: doc.revision().to_string(),
    })
}

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &DatasetsDescribe) -> CommandResult<()> {
    match targets(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// The description; the same as execute's (nothing is written).
pub fn plan(
    cx: &ExecutionContext<'_>,
    input: &DatasetsDescribe,
) -> CommandResult<DatasetsDescribed> {
    match answer(cx.doc, input) {
        Ok(output) => CommandResult::Completed {
            output,
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// The description, read only.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: DatasetsDescribe,
) -> CommandResult<DatasetsDescribed> {
    plan(cx, &input)
}
