//! Seçilenleri çıkar (the web's `builtin/layers/extract.ts`; docs/adr/0217
//! §4; QGIS Extract selected features): the objects copied, or moved, into
//! the result layer; a new result layer takes the look and fields of the
//! first object's layer and goes right above it. Moving leaves the objects
//! already there as they are. One undo step.

use kentos_domain::Slot;
use serde_json::json;

use super::{copy_of, input_param, mode_param, output_param};
use crate::types::{
    ChangeSet, Feedback, OutputDef, OutputKind, Patch, Resolved, RunContext, RunResult, ScopeKind,
    Target, Tool,
};

pub fn tool() -> Tool {
    Tool {
        id: "layers.extract".into(),
        label: "Seçilenleri çıkar".into(),
        category: "layers".into(),
        description: "Seçili nesneleri yeni ya da var olan bir katmana kopyalar ya da taşır.".into(),
        help: Some([
            "Örnekler: bir sorgunun seçtiği parselleri ayrı bir katmanda toplamak; elle seçilen yapıları bir iş katmanına taşımak.",
            "Yeni sonuç katmanı ilk nesnenin katmanının görünüşünü ve alanlarını alır ve onun hemen üstüne açılır. Kopyala nesneleri yerinde bırakır; Taşı onları sonuç katmanına geçirir. Kilitli katmandaki nesneler alınmaz.",
        ]
        .join("\n\n")),
        keywords: [
            "çıkar", "seçili", "seçilen", "katman", "kopyala", "taşı", "extract",
            "extract selected features", "export selection",
        ]
        .map(String::from)
        .to_vec(),
        aliases: vec!["SECILENLERICIKAR".into()],
        icon: Some("extractSelected".into()),
        parameters: vec![
            input_param(
                vec![
                    ScopeKind::Selection,
                    ScopeKind::Layer,
                    ScopeKind::Visible,
                    ScopeKind::All,
                ],
                "Çıkarılacak nesneler; kilitli katmandakiler alınmaz.",
            ),
            mode_param(),
            output_param("Seçilenler", "#0090FF", Some("input")),
        ],
        outputs: vec![
            OutputDef::new("objects", "Çıkarılan nesneler", OutputKind::Features),
            OutputDef::new("count", "Nesne sayısı", OutputKind::Number),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run),
    }
}

fn run(v: &Resolved<'_>, _ctx: &RunContext<'_>, _feedback: &mut dyn Feedback) -> RunResult {
    let inputs = &v.features("input").entities;
    let out = v.layer("output");
    let like = inputs.first().map(|e| e.base().layer_id.clone());
    if v.text("mode") == "move" {
        let moved: Vec<Slot> = inputs
            .iter()
            .filter(|e| e.base().layer_id != out.id)
            .map(|e| Slot(e.base().id))
            .collect();
        let count = moved.len();
        return RunResult {
            changes: Some(ChangeSet {
                update: moved
                    .iter()
                    .map(|&id| Patch {
                        id,
                        attrs: None,
                        label: None,
                        zs: None,
                        geometry: None,
                        layer: Some(out.id.clone()),
                    })
                    .collect(),
                ..ChangeSet::default()
            }),
            outputs: [
                (
                    "objects".to_owned(),
                    json!(moved.iter().map(|s| s.0).collect::<Vec<_>>()),
                ),
                ("count".to_owned(), json!(count)),
            ]
            .into_iter()
            .collect(),
            summary: Some(format!("{count} nesne “{}” katmanına taşındı.", out.name)),
            like,
            ..RunResult::default()
        };
    }
    let count = inputs.len();
    RunResult {
        changes: Some(ChangeSet {
            add: inputs.iter().map(|e| copy_of(e, &out.id, None)).collect(),
            ..ChangeSet::default()
        }),
        outputs: [("count".to_owned(), json!(count))].into_iter().collect(),
        summary: Some(format!(
            "{count} nesne “{}” katmanına kopyalandı.",
            out.name
        )),
        like,
        ..RunResult::default()
    }
}
