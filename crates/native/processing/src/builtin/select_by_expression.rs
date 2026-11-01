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
            "Gelişmiş ayarlardaki CAD özellik süzgeci, gerçek model yazı yüksekliğini metre cinsinden ve atanan/görünür CAD rengini ayrıca sınırlar. byLayer atanan renk olmadığı anlamına gelir. Görünür renk, katmandan gelen CAD boyasıdır; tematik CBS sembolünün piksel rengi değildir. Blokta seçilen hedef yerleştirmedir, blok tanımı değişmez. Bu değerler bir İşlem modeli olarak kaydedilip paylaşılabilir.",
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
            ParamDef::new("cadFilter", "CAD özellik süzgeci", ParamKind::Boolean).advanced().default_value(json!(false))
                .describe("Yukarıdaki önizleme yalnız ifadeyi sayar. CAD özellikleri Çalıştır sırasında uygulanır."),
            height_limit("textHeightMin", "En küçük yazı yüksekliği"),
            height_limit("textHeightMax", "En büyük yazı yüksekliği"),
            color_limit("assignedColor", "Atanan CAD rengi", "byLayer veya #RRGGBB"),
            color_limit("displayedColor", "Görünür CAD rengi", "#RRGGBB veya palet adı"),
            text_limit("blockPath", "Blok parçası yolu", "Örn. 0/2; boşsa nesnenin kendisi")
                .describe("Parçanın rengini sorgular, yerleştirmeyi seçer. Tanım değişmez; yazı yüksekliği sınırlarıyla birlikte kullanılmaz."),
        ],
        outputs: vec![
            OutputDef::new("matched", "Koşulu sağlayanlar", OutputKind::Features),
            OutputDef::new("count", "Koşulu sağlayan sayısı", OutputKind::Number),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: Some(|values| {
            if values.get("cadFilter").and_then(serde_json::Value::as_bool) != Some(true) { return None; }
            let min = values.get("textHeightMin").and_then(serde_json::Value::as_f64);
            let max = values.get("textHeightMax").and_then(serde_json::Value::as_f64);
            if min.into_iter().chain(max).any(|n| n <= 0.0) { return Some("Yazı yüksekliği sıfırdan büyük olmalı.".into()); }
            if block_path(values.get("blockPath").and_then(serde_json::Value::as_str).unwrap_or_default()).is_err() {
                return Some("Blok yolu 0/2 gibi, en çok 16 adet sıfır tabanlı parça sırası olmalı.".into());
            }
            min.zip(max).filter(|(min,max)| min > max).map(|_| "En küçük yazı yüksekliği en büyük değerden büyük olamaz.".into())
        }),
        preview: None,
        run: Some(run),
    }
}

fn cad_filter(values: &crate::types::Values) -> bool {
    values.get("cadFilter").and_then(serde_json::Value::as_bool) == Some(true)
}

fn height_limit(name: &str, label: &str) -> ParamDef {
    ParamDef::new(name, label, ParamKind::Number {
        min: Some(0.0), max: None, integer: false, unit:"m".into(), placeholder:Some("Sınır yok".into())
    }).optional().default_value(serde_json::Value::Null).advanced().shown_when(cad_filter)
}

fn color_limit(name: &str, label: &str, placeholder: &str) -> ParamDef {
    text_limit(name, label, placeholder).color_picker(name == "assignedColor")
}

fn text_limit(name: &str, label: &str, placeholder: &str) -> ParamDef {
    ParamDef::new(name, label, ParamKind::Text {
        placeholder:Some(placeholder.into()), max_length:Some(128), allow_empty:true
    }).optional().default_value(serde_json::Value::Null).advanced().shown_when(cad_filter)
}

fn block_path(text: &str) -> Result<Vec<u32>, ()> {
    if text.trim().is_empty() { return Ok(Vec::new()); }
    let path: Vec<_> = text.split('/').map(|n| n.trim().parse::<u32>().map_err(|_| ())).collect::<Result<_,_>>()?;
    if path.len() > kentos_commands::SUBREF_PATH_DEPTH { return Err(()); }
    Ok(path)
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
    let mut hits: Vec<Slot> = ids
        .iter()
        .zip(&met)
        .filter(|(_, m)| **m == ExprValue::Bool(true))
        .map(|(id, _)| *id)
        .collect();
    if feedback.canceled() { return RunResult::default(); }
    if cad_filter(&v.values) {
        let color = |name| {
            let value = v.text(name).trim();
            (!value.is_empty()).then(|| value.to_owned())
        };
        let query = kentos_commands::SelectProperties {
            block_path: match block_path(v.text("blockPath")) { Ok(path) => path, Err(()) => return RunResult {refused:Some("Geçersiz blok parçası yolu.".into()), ..RunResult::default()} },
            text_height_min:v.number("textHeightMin"), text_height_max:v.number("textHeightMax"),
            assigned_color:color("assignedColor"), displayed_color:color("displayedColor"), expression:None,
        };
        hits = match kentos_native_application::entities_select::refine_properties(ctx.doc, ctx.geometry.store(), hits, &query) {
            Ok(ids) => ids,
            Err(error) => return RunResult {refused:Some(error.message), ..RunResult::default()},
        };
    }
    if feedback.canceled() { return RunResult::default(); }
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
