//! İfadeyle seç (the web's `builtin/selectByExpression.ts`, QGIS "Select by
//! expression"): selects the objects for which a condition over attributes
//! and geometry holds, combined with the current selection as the user
//! chose. Edits nothing, so there is nothing to undo; the selection is the
//! result.

use std::collections::HashSet;

use kentos_domain::Slot;
use kentos_style_core::expr::Value as ExprValue;
use kentos_style_core::expr::rows::As;
use serde_json::json;

use crate::expression::evaluate_in;
use crate::types::{
    EnumOption, Feedback, OutputDef, OutputKind, ParamDef, ParamKind, Resolved, Returns,
    RunContext, RunResult, ScopeKind, Target, Tool,
};

pub fn tool() -> Tool {
    Tool {
        id: "selection.byExpression".into(),
        label: "İfadeyle seç".into(),
        category: "selection".into(),
        description: "Özniteliklere ve geometriye göre yazdığınız koşulu sağlayan nesneleri seçer.".into(),
        help: Some([
            "Koşulda alan adları doğrudan (Nitelik) ya da köşeli parantezle ([Tapu alanı]) yazılır, metinler tırnak içindedir ('Arsa'). Geometri için $alan, $uzunluk, $köşe, $katman gibi değişkenler vardır.",
            "Örnekler: Nitelik = 'Arsa' ve $alan > 500; boş(Parsel); içerir(Mahalle, 'cumhuriyet'); $katman = 'Yapı' veya $alan < 50.",
            "Seçim biçimi sonucun mevcut seçimle nasıl birleşeceğini belirler: yeni seçim, seçime ekle, seçimden çıkar ya da yalnızca şu an seçili olanlar arasında ara.",
        ]
        .join("\n\n")),
        keywords: [
            "seç", "sorgu", "filtre", "ifade", "koşul", "öznitelik", "select", "query",
            "expression", "where",
        ]
        .map(String::from)
        .to_vec(),
        aliases: ["IFADESEC", "SORGU"].map(String::from).to_vec(),
        icon: Some("selectExpression".into()),
        parameters: vec![
            ParamDef::new(
                "input",
                "Aranacak nesneler",
                ParamKind::Features {
                    kinds: None,
                    scopes: Some(vec![
                        ScopeKind::All,
                        ScopeKind::Visible,
                        ScopeKind::Layer,
                        ScopeKind::Selection,
                    ]),
                    writes: false,
                },
            )
            .default_value(json!({ "scope": "all" }))
            .describe("Koşulun denendiği nesneler."),
            ParamDef::new(
                "condition",
                "Koşul",
                ParamKind::Expression {
                    returns: Returns::Condition,
                    of: Some("input".into()),
                    placeholder: Some("Nitelik = 'Arsa' ve $alan > 500".into()),
                },
            )
            .default_value(json!("$alan > 500")),
            ParamDef::new(
                "mode",
                "Seçim biçimi",
                ParamKind::Choice {
                    options: vec![
                        EnumOption::new("new", "Yeni seçim").hint("Mevcut seçim bırakılır"),
                        EnumOption::new("add", "Seçime ekle")
                            .hint("Koşulu sağlayanlar mevcut seçime eklenir"),
                        EnumOption::new("remove", "Seçimden çıkar")
                            .hint("Koşulu sağlayanlar mevcut seçimden çıkar"),
                        EnumOption::new("within", "Seçim içinde ara")
                            .hint("Seçili olanlardan yalnızca koşulu sağlayanlar kalır"),
                    ],
                },
            )
            .default_value(json!("new")),
        ],
        outputs: vec![
            OutputDef::new("matched", "Koşulu sağlayanlar", OutputKind::Features),
            OutputDef::new("count", "Koşulu sağlayan sayısı", OutputKind::Number),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run),
    }
}

fn run(v: &Resolved<'_>, ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let list = &v.features("input").entities;
    feedback.progress(0.0, "Koşul deneniyor");
    let ids: Vec<Slot> = list.iter().map(|e| Slot(e.base().id)).collect();
    let layer_name = |id: &str| ctx.layer_name(id).to_owned();
    let met = v
        .expr("condition")
        .map(|e| evaluate_in(e, list, &ctx.evaluation(&layer_name), As::Bool))
        .unwrap_or_default();
    let hits: Vec<Slot> = ids
        .iter()
        .zip(&met)
        .filter(|(_, m)| **m == ExprValue::Bool(true))
        .map(|(id, _)| *id)
        .collect();
    let current = ctx.selection;
    let hit: HashSet<Slot> = hits.iter().copied().collect();
    let select: Vec<Slot> = match v.text("mode") {
        "add" => current.iter().chain(&hits).copied().collect(),
        "remove" => current
            .iter()
            .copied()
            .filter(|id| !hit.contains(id))
            .collect(),
        "within" => current
            .iter()
            .copied()
            .filter(|id| hit.contains(id))
            .collect(),
        _ => hits.clone(),
    };
    let distinct = select.iter().collect::<HashSet<_>>().len();
    let summary = format!(
        "{} / {} nesne koşulu sağladı; seçimde {distinct} nesne var.",
        hits.len(),
        list.len()
    );
    let matched: Vec<u32> = hits.iter().map(|s| s.0).collect();
    RunResult {
        select: Some(select),
        outputs: [
            ("matched".to_owned(), json!(matched)),
            ("count".to_owned(), json!(hits.len())),
        ]
        .into_iter()
        .collect(),
        summary: Some(summary),
        ..RunResult::default()
    }
}
