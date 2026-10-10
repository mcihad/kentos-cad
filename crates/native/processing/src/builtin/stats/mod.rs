//! Mekânsal istatistik (docs/adr/0238; the web's `builtin/stats/`): Ortalama
//! ve Ortanca merkez, Standart uzaklık, Yön dağılımı, En yakın komşu, Moran
//! I, Sıcak nokta (Gi\*), DBSCAN and k-ortalamalar. Every run is the shared
//! core's (`ops::spatial_stats`), the same functions the web reaches through
//! WASM, its texts written there; what the tools share is here: the refusal
//! on a geographic project and a run's answer turned into the tool's result.

pub mod tools;

use std::collections::BTreeMap;

use kentos_contracts::Entity;
use kentos_geometry_core::crs::System;
use kentos_geometry_core::ops::spatial_stats::StatsRun;
use kentos_project::systems::own;
use serde_json::json;

use super::geometry::new_object;
use crate::types::{ChangeSet, Feedback, RunContext, RunResult};

/// Why a geographic project is refused (§2).
pub const GEOGRAPHIC: &str =
    "Mekânsal istatistik projeksiyonlu koordinat ister: projenin sistemi coğrafi.";

/// Sıcak noktalar' look (§9): a category of “Güven sınıfı” a class, hot to cold, in its colour (ColorBrewer's RdBu) as
/// an area, a line and a point; the legend lists them. The web's `HOT_RENDERER`.
pub fn hot_renderer() -> serde_json::Value {
    let classes = [
        ("3", "Sıcak nokta, %99 güven", "#B2182B"),
        ("2", "Sıcak nokta, %95 güven", "#EF8A62"),
        ("1", "Sıcak nokta, %90 güven", "#FDDBC7"),
        ("0", "Anlamlı değil", "#D9D9D9"),
        ("-1", "Soğuk nokta, %90 güven", "#D1E5F0"),
        ("-2", "Soğuk nokta, %95 güven", "#67A9CF"),
        ("-3", "Soğuk nokta, %99 güven", "#2166AC"),
    ];
    let categories: Vec<serde_json::Value> = classes
        .iter()
        .map(|(value, label, color)| {
            json!({
                "value": value,
                "label": label,
                "symbols": {
                    "fill": { "type": "fill", "layers": [
                        { "id": "f", "type": "simpleFill", "color": color },
                        { "id": "o", "type": "simpleLine", "color": "#FFFFFF", "width": 0.13, "unit": "mm" },
                    ] },
                    "line": { "type": "line", "layers": [
                        { "id": "l", "type": "simpleLine", "color": color, "width": 0.6, "unit": "mm", "cap": "round", "join": "round" },
                    ] },
                    "marker": { "type": "marker", "layers": [
                        { "id": "p", "type": "shape", "shape": "circle", "size": 8, "unit": "px", "fill": color, "stroke": "#FFFFFF", "strokeWidth": 0.8 },
                    ] },
                },
            })
        })
        .collect();
    json!({ "type": "categorized", "expr": "[Güven sınıfı]", "categories": categories })
}

/// Kümeler' look (§10): the cluster's colour by `([Küme] - 1) % 10` (the core's colours turning round; JavaScript's
/// remainder keeps the sign, so the noise, 0, is −1), the noise grey; a filled point, an area, a line. The web's
/// `CLUSTER_RENDERER`.
pub fn cluster_renderer() -> serde_json::Value {
    use kentos_geometry_core::ops::spatial_stats::clusters::{CLUSTER_COLORS, NOISE_COLOR};
    let symbols = |color: &str| {
        json!({
            "fill": { "type": "fill", "layers": [
                { "id": "f", "type": "simpleFill", "color": color },
                { "id": "o", "type": "simpleLine", "color": "#FFFFFF", "width": 0.13, "unit": "mm" },
            ] },
            "line": { "type": "line", "layers": [
                { "id": "l", "type": "simpleLine", "color": color, "width": 0.6, "unit": "mm", "cap": "round", "join": "round" },
            ] },
            "marker": { "type": "marker", "layers": [
                { "id": "p", "type": "shape", "shape": "circle", "size": 7, "unit": "px", "fill": color, "stroke": "#FFFFFF", "strokeWidth": 0.8 },
            ] },
        })
    };
    let mut categories: Vec<serde_json::Value> = CLUSTER_COLORS
        .iter()
        .enumerate()
        .map(|(k, color)| {
            json!({
                "value": k.to_string(),
                "label": format!("Küme {}, {}, {} …", k + 1, k + 11, k + 21),
                "symbols": symbols(color),
            })
        })
        .collect();
    categories.push(json!({ "value": "-1", "label": "Gürültü", "symbols": symbols(NOISE_COLOR) }));
    json!({ "type": "categorized", "expr": "([Küme] - 1) % 10", "categories": categories })
}

/// Whether the project's coordinates are degrees.
pub fn geographic(ctx: &RunContext<'_>) -> bool {
    matches!(
        own(ctx.doc.settings()).and_then(|n| n.system),
        Some(System::Geographic { .. })
    )
}

/// A copy of an object on `layer`: its geometry (elevations too) and
/// attributes with the run's added, in the run's colour; its own colour,
/// symbol, label, weight and label pins left behind.
fn copy_of(e: &Entity, layer: &str, added: &[[String; 2]], color: &str) -> Entity {
    let mut c = e.clone();
    let b = c.base_mut();
    b.id = 0;
    b.layer_id = layer.to_owned();
    for [k, v] in added {
        b.attrs.insert(k.clone(), v.clone());
    }
    b.color = Some(color.to_owned());
    b.label = None;
    b.symbol = None;
    b.line_weight = None;
    b.label_pins = Vec::new();
    c
}

/// The tool's result from the core's answer: notes said, new objects (or
/// the input's copies) on `layer`, the table and the numbers among the
/// outputs (`count` with what was written when there is a layer).
pub fn result_of(
    run: StatsRun,
    feedback: &mut dyn Feedback,
    input: &[&Entity],
    layer: Option<&str>,
) -> RunResult {
    for t in run.infos {
        feedback.info(t);
    }
    for t in run.warnings {
        feedback.warn(t);
    }
    let mut add = Vec::new();
    if let Some(layer) = layer {
        for o in run.objects {
            let attrs: BTreeMap<String, String> =
                o.attrs.into_iter().map(|[k, v]| (k, v)).collect();
            add.extend(new_object(o.shape, layer, attrs));
        }
        for c in &run.copies {
            add.push(copy_of(input[c.index], layer, &c.attrs, c.color));
        }
    }
    let mut outputs = crate::types::Values::new();
    if let Some(t) = run.table {
        outputs.insert(
            "table".to_owned(),
            json!({ "columns": t.columns, "rows": t.rows }),
        );
    }
    for n in run.numbers {
        outputs.insert(n.name.to_owned(), json!(n.value));
    }
    if layer.is_some() {
        outputs.insert("count".to_owned(), json!(add.len()));
    }
    RunResult {
        changes: (!add.is_empty()).then(|| ChangeSet {
            add,
            ..ChangeSet::default()
        }),
        outputs,
        summary: Some(run.summary),
        ..RunResult::default()
    }
}
