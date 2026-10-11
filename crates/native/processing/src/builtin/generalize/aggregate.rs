//! Alanları kümele (the web's `aggregate.ts`; docs/adr/0221 §5): areas no
//! farther apart than the distance joined by the closing's bridges, group
//! by group; each result with how many objects it holds.

use std::collections::BTreeMap;

use std::sync::atomic::{AtomicUsize, Ordering};

use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::ops::generalize::aggregate::{Aggregate, Aggregation, close, clusters};
use kentos_geometry_core::ops::generalize::calls::polygon_shape;
use serde_json::json;

use super::super::convert::scopes;
use super::super::geometry::{AREA_KINDS, input_notes, layer_param, new_object};
use super::{Inputs, finish, measure_param};
use crate::types::{
    Feedback, OutputDef, OutputKind, ParamDef, ParamKind, Resolved, RunContext, RunResult, Target,
    Tool,
};

pub fn tool() -> Tool {
    Tool {
        id: "generalize.aggregate".into(),
        label: "Alanları kümele".into(),
        category: "generalization".into(),
        description: "Birbirine kümeleme uzaklığından yakın alanları aralarına köprü kurarak birleştirir (yapı adaları, orman ve göl kümeleri).".into(),
        help: Some([
            "Örnekler: küçük ölçekte binaları yapı adalarına toplamak, dağınık orman lekelerini birleştirmek.",
            "Aralarındaki uzaklık kümeleme uzaklığını aşmayan alanlar bir kümedir. Kümenin birleşimi uzaklığın yarısı kadar dışa, sonra içe tamponlanır (kapanış); iki ya da daha çok nesneye değen dolgular köprü olur. Girintileri doldur açıkken tek nesnenin dar girintileri de dolar.",
            "Sonuç Nesne sayısı'nı ve Grupla'nın değerini alır; öbür öznitelikler alınmaz. En küçük boşluktan küçük delikler doldurulur, en küçük alandan küçük sonuçlar yazılmaz. Gruplar birbirleriyle birleşmez; kotlar taşınmaz.",
        ]
        .join("\n\n")),
        keywords: [
            "kümele", "birleştir", "aggregate", "yapı adası", "köprü", "genelleştir",
        ]
        .map(String::from)
        .to_vec(),
        aliases: ["KUMELE", "AGGREGATE"].map(String::from).to_vec(),
        icon: Some("aggregateAreas".into()),
        parameters: vec![
            ParamDef::new(
                "input",
                "Alanlar",
                ParamKind::Features {
                    kinds: Some(AREA_KINDS.iter().map(|k| (*k).to_owned()).collect()),
                    scopes: scopes(),
                    writes: false,
                },
            )
            .describe("Kümelenecek alanlar."),
            measure_param(
                "distance",
                "Kümeleme uzaklığı",
                "m",
                0.001,
                10.0,
                "Bu uzaklıktan yakın alanlar birleşir.",
            ),
            measure_param("minArea", "En küçük alan", "m²", 0.0, 0.0, "Bundan küçük sonuçlar yazılmaz."),
            measure_param("minHole", "En küçük boşluk", "m²", 0.0, 0.0, "Bundan küçük delikler doldurulur."),
            ParamDef::new("fillNotches", "Girintileri doldur", ParamKind::Boolean)
                .default_value(json!(false))
                .describe("Tek alanın dar girintileri de doldurulur."),
            ParamDef::new(
                "group",
                "Grupla",
                ParamKind::Field {
                    of: vec!["input".into()],
                    allow_new: false,
                    multiple: false,
                },
            )
            .optional()
            .describe("Bu alanın değeri aynı olanlar birlikte kümelenir."),
            layer_param("Kümelenen alanlar", "#E5484D"),
        ],
        outputs: vec![
            OutputDef::new("aggregated", "Yazılan alanlar", OutputKind::Features),
            OutputDef::new("count", "Yazılan alan sayısı", OutputKind::Number),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run),
    }
}

/// Threads closing clusters at once: the machine's but one, at most eight.
fn threads() -> usize {
    std::thread::available_parallelism()
        .map_or(1, |n| n.get().saturating_sub(1))
        .clamp(1, 8)
}

/// A group's clusters closed, several at once (each on its own: a cluster's
/// closing does not reach another's); the results in the clusters' order.
fn closed_all(shapes: &[&Shape], rule: &Aggregation) -> (Vec<Aggregate>, usize) {
    let list = clusters(shapes, rule.distance);
    let next = AtomicUsize::new(0);
    let mut done: Vec<(usize, Vec<Aggregate>, usize)> = std::thread::scope(|s| {
        let handles: Vec<_> = (0..threads().min(list.len()).max(1))
            .map(|_| {
                s.spawn(|| {
                    let mut mine = Vec::new();
                    loop {
                        let k = next.fetch_add(1, Ordering::Relaxed);
                        let Some(members) = list.get(k) else { break };
                        let (got, small) = close(shapes, members, rule);
                        mine.push((k, got, small));
                    }
                    mine
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().unwrap_or_else(|e| std::panic::resume_unwind(e)))
            .collect()
    });
    done.sort_by_key(|(k, _, _)| *k);
    let mut results = Vec::new();
    let mut dropped = 0;
    for (_, got, small) in done {
        results.extend(got);
        dropped += small;
    }
    (results, dropped)
}

fn run(v: &Resolved<'_>, _ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let inputs = &v.features("input").entities;
    input_notes(feedback, inputs, &[], false);
    let rule = Aggregation {
        distance: v.number("distance").unwrap_or(10.0),
        min_area: v.number("minArea").unwrap_or(0.0),
        min_hole: v.number("minHole").unwrap_or(0.0),
        fill_notches: v.flag("fillNotches"),
    };
    let group = v.text("group");
    let value = |e: &kentos_contracts::Entity| {
        if group.is_empty() {
            String::new()
        } else {
            e.base().attrs.get(group).cloned().unwrap_or_default()
        }
    };
    // The groups in the order their values first come.
    let mut names: Vec<String> = Vec::new();
    for e in inputs {
        let name = value(e);
        if !names.contains(&name) {
            names.push(name);
        }
    }
    feedback.progress(0.0, "Alanlar kümeleniyor");
    let objects = Inputs::of(inputs);
    let layer = &v.layer("layer").id;
    let mut add = Vec::new();
    let mut dropped = 0;
    for name in names {
        let index: Vec<usize> = (0..inputs.len())
            .filter(|&i| value(inputs[i]) == name)
            .collect();
        let shapes: Vec<&Shape> = index.iter().map(|&i| &objects.shapes[i]).collect();
        let (results, small) = closed_all(&shapes, &rule);
        dropped += small;
        for r in results {
            let mut attrs = BTreeMap::new();
            attrs.insert("Nesne sayısı".to_owned(), r.members.len().to_string());
            if !group.is_empty() {
                attrs.insert(group.to_owned(), name.clone());
            }
            add.extend(new_object(polygon_shape(&r.area), layer, attrs));
        }
    }
    if dropped > 0 {
        feedback.warn(format!(
            "{dropped} kümelenmiş alan en küçük alandan küçük; yazılmadı."
        ));
    }
    let count = add.len();
    let summary = format!("{} alandan {count} kümelenmiş alan yazıldı.", inputs.len());
    finish(add, summary, count, None)
}
