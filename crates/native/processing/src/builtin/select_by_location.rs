//! Konuma göre seç (the web's `builtin/selectByLocation.ts`; docs/adr/0200
//! §2; QGIS "Select by location", Netcad Konumsal Seçim): selects the
//! objects that stand in a relation to any of the reference objects (Ayrık:
//! to none), combined with the current selection as the user chose. The
//! relations are the core's (`ops::spatial_query`): the run's store pairs the
//! inputs with the references. Edits nothing, so there is nothing to undo.

use std::collections::HashSet;

use kentos_domain::Slot;
use kentos_geometry_core::ops::spatial_query::Relation;
use serde_json::{Value, json};

use super::queries::{QUERY_KINDS, kinds, relation_label};
use crate::types::{
    EnumOption, Feedback, OutputDef, OutputKind, ParamDef, ParamKind, Resolved, RunContext,
    RunResult, ScopeKind, Target, Tool,
};

pub fn tool() -> Tool {
    Tool {
        id: "selection.byLocation".into(),
        label: "Konuma göre seç".into(),
        category: "selection".into(),
        description: "Başka nesnelerle konum ilişkisi olan nesneleri seçer: kesişen, içeren, içinde kalan, ayrık, uzaklıkta ya da merkezi içinde.".into(),
        help: Some([
            "Seçilecek nesneler başvuru nesnelerinden en az biriyle ilişkiyi sağlıyorsa seçilir; Ayrık, hiçbiriyle kesişmeyenleri seçer.",
            "Kesişen: ortak bir noktaları vardır (değmek de sayılır). İçeren: başvuru bütünüyle bu nesnenin alanının içindedir. İçinde kalan: nesne bütünüyle başvurunun alanının içindedir. Uzaklıkta: aradaki en kısa uzaklık verilen uzaklıktan büyük değildir. Merkezi içinde: nesnenin ağırlık merkezi ($merkez_y, $merkez_x) başvurunun alanının içinde ya da sınırındadır.",
            "Sınırda olmak ve değmek 1 mm içinde sayılır. Bir nesne kendisiyle karşılaştırılmaz.",
        ]
        .join("\n\n")),
        keywords: [
            "konum", "mekânsal", "mekansal", "sorgu", "kesişen", "içeren", "içinde", "ayrık",
            "uzaklık", "yakın", "select by location", "spatial", "intersect", "within", "contains",
        ]
        .map(String::from)
        .to_vec(),
        aliases: ["KONUMSEC", "KONUMSALSECIM"].map(String::from).to_vec(),
        icon: Some("selectLocation".into()),
        parameters: vec![
            ParamDef::new(
                "input",
                "Seçilecek nesneler",
                ParamKind::Features {
                    kinds: kinds(&QUERY_KINDS),
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
            .describe("İlişkisi denenen nesneler."),
            ParamDef::new(
                "relation",
                "İlişki",
                ParamKind::Choice {
                    options: vec![
                        EnumOption::new("intersects", "Kesişen").hint("Ortak noktası olan"),
                        EnumOption::new("contains", "İçeren").hint("Başvuruyu bütünüyle içine alan"),
                        EnumOption::new("within", "İçinde kalan")
                            .hint("Bütünüyle başvurunun içinde olan"),
                        EnumOption::new("disjoint", "Ayrık").hint("Hiçbir başvuruyla kesişmeyen"),
                        EnumOption::new("near", "Uzaklıkta").hint("Verilen uzaklıktan yakın olan"),
                        EnumOption::new("centerIn", "Merkezi içinde")
                            .hint("Ağırlık merkezi başvurunun içinde olan"),
                    ],
                },
            )
            .default_value(json!("intersects")),
            ParamDef::new(
                "reference",
                "Başvuru nesneleri",
                ParamKind::Features {
                    kinds: kinds(&QUERY_KINDS),
                    scopes: Some(vec![
                        ScopeKind::Selection,
                        ScopeKind::Layer,
                        ScopeKind::Visible,
                        ScopeKind::All,
                    ]),
                    writes: false,
                },
            )
            .default_value(json!({ "scope": "selection" }))
            .describe("İlişkinin karşı tarafı."),
            ParamDef::new(
                "distance",
                "Uzaklık",
                ParamKind::Number {
                    min: Some(0.0),
                    max: None,
                    integer: false,
                    unit: "m".into(),
                    placeholder: None,
                },
            )
            .default_value(json!(10))
            .shown_when(|v| v.get("relation").and_then(Value::as_str) == Some("near"))
            .describe("Bu uzaklıktan yakın ya da tam bu uzaklıkta olanlar seçilir."),
            ParamDef::new(
                "mode",
                "Seçim biçimi",
                ParamKind::Choice {
                    options: vec![
                        EnumOption::new("new", "Yeni seçim").hint("Mevcut seçim bırakılır"),
                        EnumOption::new("add", "Seçime ekle")
                            .hint("İlişkiyi sağlayanlar mevcut seçime eklenir"),
                        EnumOption::new("remove", "Seçimden çıkar")
                            .hint("İlişkiyi sağlayanlar mevcut seçimden çıkar"),
                        EnumOption::new("within", "Seçim içinde ara")
                            .hint("Seçili olanlardan yalnızca ilişkiyi sağlayanlar kalır"),
                    ],
                },
            )
            .default_value(json!("new")),
        ],
        outputs: vec![
            OutputDef::new("matched", "İlişkiyi sağlayanlar", OutputKind::Features),
            OutputDef::new("count", "İlişkiyi sağlayan sayısı", OutputKind::Number),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run),
    }
}

fn run(v: &Resolved<'_>, ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let inputs = &v.features("input").entities;
    let refs = &v.features("reference").entities;
    let relation = Relation::from_key(v.text("relation")).unwrap_or(Relation::Intersects);
    feedback.progress(0.0, "İlişkiler deneniyor");
    let ids = |list: &[&kentos_contracts::Entity]| -> Vec<Slot> {
        list.iter().map(|e| Slot(e.base().id)).collect()
    };
    let input_ids = ids(inputs);
    // Ayrık is asked as Kesişen: the inputs that meet no reference.
    let asked = if relation == Relation::Disjoint {
        Relation::Intersects
    } else {
        relation
    };
    let within = if relation == Relation::Near {
        v.number("distance").unwrap_or(0.0)
    } else {
        0.0
    };
    let met: HashSet<usize> = ctx
        .geometry
        .relate_pairs(&input_ids, &ids(refs), asked, within)
        .into_iter()
        .map(|(i, _)| i)
        .collect();
    let disjoint = relation == Relation::Disjoint;
    let hits: Vec<Slot> = input_ids
        .iter()
        .enumerate()
        .filter(|(i, _)| met.contains(i) != disjoint)
        .map(|(_, id)| *id)
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
        "{} / {} nesne “{}” ilişkisini sağladı; seçimde {distinct} nesne var.",
        hits.len(),
        inputs.len(),
        relation_label(relation)
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
