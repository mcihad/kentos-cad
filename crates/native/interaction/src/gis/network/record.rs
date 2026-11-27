//! A network analysis as it ran (docs/adr/9334): what İşlemler's history keeps of En kısa yol, Hizmet alanı and Şebeke
//! izleme, as it keeps a processing tool's run, and what the layers they open say they were made of (the derivation of
//! docs/adr/0633 §2). The tools hand it to the host with [`crate::tool::ViewChange::Ran`]; Yeniden aç gives its values
//! back to the tool ([`crate::tool::Tool::reopen`]).

use std::collections::BTreeMap;

use kentos_commands::{DerivationInput, LayerDerivation, NetworkDef};
use kentos_domain::Slot;
use serde_json::{Value, json};

use super::base::{Base, PointKind};
use crate::Vec2;
use crate::format::Format;
use crate::tool::{Context, Name};

/// A network analysis as it ran.
#[derive(Clone, Debug, PartialEq)]
pub struct AnalysisRun {
    /// The tool's command (`tool.netServiceArea`) and name.
    pub tool: String,
    pub label: String,
    /// Its settings and inputs (the network's id, the cost, the stops' places …): Yeniden aç gives them back.
    pub values: BTreeMap<String, Value>,
    /// The run in words, as İşlemler's records say a run.
    pub words: Vec<String>,
    /// What it said when it ended.
    pub summary: String,
    /// The objects it wrote, and the ones it chose.
    pub added: Vec<Slot>,
    pub touched: Vec<Slot>,
    /// The objects of the network's lines it read.
    pub records_in: u64,
    /// When it ran, milliseconds since the Unix epoch.
    pub started: u64,
}

/// The time now, milliseconds since the Unix epoch.
pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

/// A place as a run keeps it, `[east, north]`.
fn place(p: Vec2) -> Value {
    json!([p.x, p.y])
}

/// The places a run kept under `key`.
pub fn places(values: &BTreeMap<String, Value>, key: &str) -> Vec<Vec2> {
    values
        .get(key)
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(|p| {
                    let p = p.as_array()?;
                    let (x, y) = (p.first()?.as_f64()?, p.get(1)?.as_f64()?);
                    (x.is_finite() && y.is_finite()).then(|| Vec2::new(x, y))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// What every network analysis keeps: the network, the cost (none for Şebeke izleme), the stops and barriers, and the
/// time, vehicle and day it asked at.
pub fn common_values(n: &NetworkDef, cost: Option<usize>, base: &Base, cx: &Context<'_>) -> BTreeMap<String, Value> {
    let m = &cx.memory;
    let mut out = BTreeMap::from([
        ("network".to_owned(), json!(n.id)),
        ("stops".to_owned(), Value::Array(base.stops().into_iter().map(place).collect())),
        ("barriers".to_owned(), Value::Array(base.barriers().into_iter().map(place).collect())),
        ("time".to_owned(), json!(m.network_time)),
        ("profile".to_owned(), json!(m.network_profile.as_str())),
        ("day".to_owned(), json!(m.network_day)),
    ]);
    if let Some(c) = cost {
        out.insert("cost".to_owned(), json!(n.cost_names().get(c).copied().unwrap_or("Uzunluk")));
    }
    out
}

/// The network, its cost with its unit (none for Şebeke izleme), the stops (as the tool calls them) and the barriers,
/// in words.
pub fn common_words(n: &NetworkDef, cost: Option<usize>, stops: &str, base: &Base, f: &Format) -> Vec<String> {
    let mut out = vec![format!("Ağ: {} ({})", n.name, n.id)];
    if let Some(c) = cost {
        let names = Base::cost_names(n);
        out.push(format!(
            "Maliyet: {}, {}",
            names.get(c).map_or("Uzunluk", String::as_str),
            Base::cost_unit(n, c, f)
        ));
    }
    out.push(format!("{stops}: {}", base.stops().len()));
    let barriers = base.barriers().len();
    if barriers > 0 {
        out.push(format!("Engeller: {barriers}"));
    }
    out
}

/// A layer the run wrote, in words: new, or one the drawing had.
pub fn layer_words(what: &str, name: &str, opened: &[&str]) -> String {
    let how = if opened.contains(&name) {
        "yeni katman"
    } else {
        "var olan katmana yazıldı"
    };
    format!("{what}: “{name}” ({how})")
}

/// The network's line and junction layers, each once: what an analysis reads.
fn read_layers(n: &NetworkDef) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for id in n.edges.iter().map(|e| &e.layer).chain(n.junctions.iter().map(|j| &j.layer)) {
        if !out.contains(id) {
            out.push(id.clone());
        }
    }
    out
}

/// The objects on the network's lines the drawing holds.
pub fn records_in(n: &NetworkDef, cx: &Context<'_>) -> u64 {
    n.edges
        .iter()
        .map(|e| cx.doc.by_layer(&e.layer).count() as u64)
        .sum()
}

/// The derivation a layer an analysis opens carries (docs/adr/0633 §2): the tool, its values, the network's layers
/// with their content digests now (before it writes), the drawing's revision, the engine and when it ran.
pub fn derivation(tool: &str, label: &str, values: &BTreeMap<String, Value>, n: &NetworkDef, started: u64, cx: &Context<'_>) -> LayerDerivation {
    let doc = &*cx.doc;
    let inputs = read_layers(n)
        .into_iter()
        .filter_map(|id| {
            let node = doc.layers().get(&id)?;
            Some(DerivationInput {
                name: node.name.clone(),
                digest: kentos_native_application::sources::layer_digest(doc, &id),
                source: doc
                    .settings()
                    .sources
                    .iter()
                    .find(|s| s.layer == id)
                    .map(|s| format!("{} › {}", s.path, s.table)),
                layer: id,
            })
        })
        .take(kentos_commands::MAX_DERIVATION_INPUTS)
        .collect();
    LayerDerivation {
        tool: tool.to_owned(),
        label: label.to_owned(),
        tool_version: 1,
        parameters: values.clone(),
        inputs,
        document_revision: doc.revision().to_string(),
        engine: kentos_native_application::lineage::engine(),
        at: kentos_native_application::lineage::utc_at(started),
        extension: None,
        environment: None,
    }
}

/// Yeniden aç's common part: the network, the cost, the time, vehicle and day put back in the tools' memory; the
/// stops and barriers to place again, in their order (barriers first, so that the stops' answers see them).
pub fn reopen_common(values: &BTreeMap<String, Value>, cx: &mut Context<'_>) -> Vec<(PointKind, Vec2)> {
    let text = |key: &str| values.get(key).and_then(Value::as_str).and_then(Name::new);
    let m = &mut cx.memory;
    if let Some(n) = text("network") {
        m.network = n;
    }
    if let Some(c) = text("cost") {
        m.network_cost = c;
    }
    m.network_time = values.get("time").and_then(Value::as_u64).and_then(|t| u32::try_from(t).ok());
    if let Some(p) = text("profile") {
        m.network_profile = p;
    }
    m.network_day = values.get("day").and_then(Value::as_i64);
    places(values, "barriers")
        .into_iter()
        .map(|p| (PointKind::Barrier, p))
        .chain(places(values, "stops").into_iter().map(|p| (PointKind::Stop, p)))
        .collect()
}
