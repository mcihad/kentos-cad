//! Gruplayarak birleştir (the web's `builtin/geometry/dissolve.ts`; docs/adr/0201
//! §4; ArcGIS Dissolve): the objects grouped by a field's value (all one
//! group without one), each group's areas joined in the core's overlay, its
//! paths and points gathered; one object a group, or each joined part on its
//! own. A group's object carries the group's value, how many objects it
//! joined and the sums of the fields asked for (`kentos.statistics/1`).

use std::collections::BTreeMap;

use kentos_geometry_core::ops::geoprocess::calls::dissolve_run;
use kentos_geometry_core::ops::statistics::{Stat, statistic};
use serde_json::json;

use super::{GEO_KINDS, features_param, geo_scopes, input_notes, layer_param, new_object, shapes};
use crate::builtin::queries::attr;
use crate::text::js_trim;
use crate::types::{
    ChangeSet, Feedback, OutputDef, OutputKind, ParamDef, ParamKind, Resolved, RunContext,
    RunResult, Target, Tool,
};

pub fn tool() -> Tool {
    let field = |name: &str, label: &str, multiple: bool| {
        ParamDef::new(
            name,
            label,
            ParamKind::Field {
                of: vec!["input".into()],
                allow_new: false,
                multiple,
            },
        )
        .optional()
    };
    Tool {
        id: "geometry.dissolve".into(),
        label: "Gruplayarak birleştir".into(),
        category: "geometry".into(),
        description: "Nesneleri bir alanın değerine göre gruplayıp her grubu tek nesnede birleştirir: alanların ortak sınırları kalkar, istenen alanların değerleri toplanır.".into(),
        help: Some([
            "Örnekler: parselleri ada numarasına göre birleştirip adaları elde etmek; aynı kullanım türündeki alanları tek alan yapmak.",
            "Grupla boşsa bütün nesneler tek gruptur. Alanlar birleşir, ortak sınırları kalkar; çizgiler ve noktalar grubun çok parçalı nesnesi olur. Çok parçalı kapalıysa her bağlı parça ayrı nesne olarak, büyükten küçüğe yazılır.",
            "Sonucun öznitelikleri: grup alanı ve değeri, “Nesne sayısı” ve Toplanacak alanların toplamları. Toplamlar kesindir; sayı olarak okunamayan değerler atlanır ve söylenir.",
        ]
        .join("\n\n")),
        keywords: ["birleştir", "grupla", "dissolve", "erit", "ada", "tevhit", "topla", "merge"]
            .map(String::from)
            .to_vec(),
        aliases: ["GRUPBIRLESTIR", "DISSOLVE"].map(String::from).to_vec(),
        icon: Some("geoDissolve".into()),
        parameters: vec![
            features_param(
                "input",
                "Nesneler",
                &GEO_KINDS,
                geo_scopes(),
                "Birleştirilecek alanlar, çizgiler ve noktalar.",
            ),
            field("group", "Grupla", false)
                .describe("Değeri aynı olan nesneler bir grup olur; boşsa hepsi tek grup."),
            field("sums", "Toplanacak alanlar", true)
                .describe("Her grubun bu alanlardaki değerleri toplanır."),
            ParamDef::new("multi", "Çok parçalı", ParamKind::Boolean)
                .default_value(json!(true))
                .describe("Açıkken grup başına tek nesne; kapalıyken her bağlı parça ayrı nesne."),
            layer_param("Birleştirilen", "#6E56CF"),
        ],
        outputs: vec![
            OutputDef::new("dissolved", "Birleştirilen nesneler", OutputKind::Features),
            OutputDef::new("count", "Yazılan nesne sayısı", OutputKind::Number),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run),
    }
}

/// The names a multiple field parameter holds, each trimmed (the web's `fieldNames`).
pub fn field_names(value: &str) -> Vec<&str> {
    value
        .split(',')
        .map(js_trim)
        .filter(|n| !n.is_empty())
        .collect()
}

fn run(v: &Resolved<'_>, _ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let input = &v.features("input").entities;
    let group = v.text("group");
    let sums = field_names(v.text("sums"));
    input_notes(feedback, input, &[], false);
    // Groups by their first object's place; a missing value is the empty one.
    let mut keys: Vec<String> = Vec::new();
    let groups: Vec<usize> = input
        .iter()
        .map(|e| {
            let key = if group.is_empty() {
                ""
            } else {
                attr(e, group).unwrap_or("")
            };
            match keys.iter().position(|k| k == key) {
                Some(k) => k,
                None => {
                    keys.push(key.to_owned());
                    keys.len() - 1
                }
            }
        })
        .collect();
    feedback.progress(0.0, "Gruplar birleştiriliyor");
    let parts = dissolve_run(&shapes(input), &groups, v.flag("multi"));
    let counts: Vec<usize> = (0..keys.len())
        .map(|k| groups.iter().filter(|g| **g == k).count())
        .collect();
    let mut skipped = 0usize;
    let mut totals: Vec<Vec<Option<String>>> = Vec::new();
    for name in &sums {
        let mut column = Vec::with_capacity(keys.len());
        for k in 0..keys.len() {
            let values: Vec<Option<&str>> = input
                .iter()
                .zip(&groups)
                .filter(|(_, g)| **g == k)
                .map(|(e, _)| attr(e, name))
                .collect();
            match statistic(&values, Stat::Sum, None) {
                Ok((value, skip)) => {
                    skipped += skip;
                    column.push(value);
                }
                Err(why) => return RunResult::refused(why),
            }
        }
        totals.push(column);
    }
    if skipped > 0 {
        feedback.warn(format!(
            "{skipped} değer sayı olarak okunamadığı için toplanmadı."
        ));
    }
    let layer = &v.layer("layer").id;
    let add: Vec<_> = parts
        .into_iter()
        .filter_map(|(g, s)| {
            let mut attrs = BTreeMap::new();
            if !group.is_empty() {
                attrs.insert(group.to_owned(), keys[g].clone());
            }
            attrs.insert("Nesne sayısı".to_owned(), counts[g].to_string());
            for (name, column) in sums.iter().zip(&totals) {
                if let Some(total) = &column[g] {
                    attrs.insert((*name).to_owned(), total.clone());
                }
            }
            new_object(s, layer, attrs)
        })
        .collect();
    let count = add.len();
    let summary = if group.is_empty() {
        format!(
            "{} nesne birleştirildi; {count} nesne yazıldı.",
            input.len()
        )
    } else {
        format!(
            "{} nesne {} grupta birleştirildi; {count} nesne yazıldı.",
            input.len(),
            keys.len()
        )
    };
    RunResult {
        changes: Some(ChangeSet {
            add,
            ..ChangeSet::default()
        }),
        outputs: [("count".to_owned(), json!(count))].into_iter().collect(),
        summary: Some(summary),
        ..RunResult::default()
    }
}
