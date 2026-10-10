//! Özniteliğe göre böl (the web's `builtin/layers/split.ts`; docs/adr/0217
//! §2; QGIS Split vector layer, ArcGIS Split By Attributes): the objects
//! split into layers by their fields' values, a layer a value in the order
//! the values first come, in a new group named after the fields right above
//! the first object's layer; each layer takes the look and fields of its
//! value's first object's layer. The objects are copied, or moved. One undo
//! step.

use std::collections::HashMap;

use kentos_domain::Slot;
use serde_json::{Value, json};

use super::{copy_of, fields_value, input_param, mode_param};
use crate::parameters::field_names;
use crate::types::{
    ChangeSet, Feedback, OutputDef, OutputKind, ParamDef, ParamKind, Patch, Resolved, RunContext,
    RunResult, ScopeKind, Target, Tool, ToolLayer,
};

/// The most values: past them the run is refused.
const MOST_LAYERS: usize = 1000;

pub fn tool() -> Tool {
    Tool {
        id: "layers.splitByAttribute".into(),
        label: "Özniteliğe göre böl".into(),
        category: "layers".into(),
        description: "Nesneleri bir ya da birkaç özniteliğin değerlerine göre yeni bir gruptaki katmanlara böler: her değer bir katman.".into(),
        help: Some([
            "Örnekler: parselleri mahallelerine, yapıları kullanım ve kat sayılarına, yolları türlerine göre ayrı katmanlarda toplamak.",
            "Değerler baştaki ve sondaki boşluklar atılarak okunur; değeri olmayan “(boş)” katmanına gider; birden çok alanda değerler “ – ” ile birleşir. Katmanlar alanların adını taşıyan yeni bir grupta, değerlerin ilk görüldüğü sırayla açılır ve her biri değerinin ilk nesnesinin katmanının görünüşünü ve alanlarını alır.",
            "Kopyala nesneleri yerinde bırakır; Taşı onları yeni katmanlara geçirir. Kilitli katmandaki nesneler alınmaz. En çok 1000 katmana bölünür.",
        ]
        .join("\n\n")),
        keywords: [
            "böl", "ayır", "katman", "öznitelik", "değer", "grupla", "split",
            "split by attributes", "split vector layer",
        ]
        .map(String::from)
        .to_vec(),
        aliases: vec!["OZNITELIKBOL".into()],
        icon: Some("layerSplitAttr".into()),
        parameters: vec![
            input_param(
                vec![
                    ScopeKind::Layer,
                    ScopeKind::Selection,
                    ScopeKind::Visible,
                    ScopeKind::All,
                ],
                "Bölünecek nesneler; kilitli katmandakiler alınmaz.",
            ),
            ParamDef::new(
                "fields",
                "Alanlar",
                ParamKind::Field {
                    of: vec!["input".into()],
                    allow_new: false,
                    multiple: true,
                },
            )
            .describe("Değerleri katmanları veren alan ya da alanlar."),
            mode_param(),
        ],
        outputs: vec![
            OutputDef::new("table", "Bölme raporu", OutputKind::Table),
            OutputDef::new("count", "Katman sayısı", OutputKind::Number),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run),
    }
}

fn run(v: &Resolved<'_>, _ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let inputs = &v.features("input").entities;
    let names = field_names(true, v.text("fields"));
    feedback.progress(0.0, "Değerler okunuyor");
    // The values in the order they first come, each with its objects' places.
    let mut order: Vec<(String, Vec<usize>)> = Vec::new();
    let mut at: HashMap<String, usize> = HashMap::new();
    let mut group_of = Vec::with_capacity(inputs.len());
    for (i, e) in inputs.iter().enumerate() {
        let value = fields_value(e, &names);
        let g = *at.entry(value.clone()).or_insert_with(|| {
            order.push((value, Vec::new()));
            order.len() - 1
        });
        order[g].1.push(i);
        group_of.push(g);
    }
    if order.len() > MOST_LAYERS {
        return RunResult::refused(format!(
            "{} farklı değer var; en çok {MOST_LAYERS} katmana bölünür. Daha az değerli bir alan seçin.",
            order.len()
        ));
    }
    // The run's group, then a layer a value; objects and patches name them by their keys.
    let mut layers = vec![ToolLayer {
        key: "#grup".into(),
        name: names.join(", "),
        group: true,
        above: inputs.first().map(|e| e.base().layer_id.clone()),
        ..ToolLayer::default()
    }];
    for (k, (value, members)) in order.iter().enumerate() {
        layers.push(ToolLayer {
            key: format!("#{}", k + 1),
            name: value.clone(),
            parent: Some("#grup".into()),
            like: members.first().map(|&i| inputs[i].base().layer_id.clone()),
            ..ToolLayer::default()
        });
    }
    let key = |i: usize| format!("#{}", group_of[i] + 1);
    let moving = v.text("mode") == "move";
    let mut changes = ChangeSet::default();
    if moving {
        changes.update = (0..inputs.len())
            .map(|i| Patch {
                id: Slot(inputs[i].base().id),
                attrs: None,
                label: None,
                zs: None,
                geometry: None,
                layer: Some(key(i)),
            })
            .collect();
    } else {
        changes.add = (0..inputs.len())
            .map(|i| copy_of(inputs[i], &key(i), None))
            .collect();
    }
    let rows: Vec<Value> = order
        .iter()
        .map(|(value, members)| json!([value, members.len().to_string()]))
        .collect();
    let done = if moving { "taşındı" } else { "kopyalandı" };
    RunResult {
        changes: Some(changes),
        layers,
        outputs: [
            (
                "table".to_owned(),
                json!({ "columns": ["Değer", "Nesne"], "rows": rows }),
            ),
            ("count".to_owned(), json!(order.len())),
        ]
        .into_iter()
        .collect(),
        summary: Some(format!(
            "{} nesne {} katmana bölündü ({done}).",
            inputs.len(),
            order.len()
        )),
        ..RunResult::default()
    }
}
