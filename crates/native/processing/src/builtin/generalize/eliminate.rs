//! Küçükleri komşuya kat (the web's `eliminate.ts`; docs/adr/0221 §7): small
//! areas joined to the large neighbour they share the longest boundary
//! with, or the largest; the report says each joining.

use std::collections::BTreeSet;

use kentos_geometry_core::ops::generalize::calls::{area_shape, eliminate_run};
use kentos_geometry_core::ops::generalize::eliminate::Elimination;
use serde_json::json;

use super::super::convert::scopes;
use super::super::geometry::{AREA_KINDS, attrs_of, input_notes, layer_param, new_object};
use super::{Inputs, finish, measure_param, table};
use crate::text::to_fixed;
use crate::types::{
    EnumOption, Feedback, OutputDef, OutputKind, ParamDef, ParamKind, Resolved, RunContext,
    RunResult, Target, Tool,
};

const COLUMNS: [&str; 4] = ["Küçük alan", "Katıldığı", "Ortak sınır (m)", "Alan (m²)"];

pub fn tool() -> Tool {
    Tool {
        id: "generalize.eliminate".into(),
        label: "Küçükleri komşuya kat".into(),
        category: "generalization".into(),
        description: "Küçük alanları en uzun ortak sınırlı ya da en büyük komşusuna katar (kırıntı parseller, sayısallaştırma artıkları).".into(),
        help: Some([
            "Örnekler: arazi örtüsü verisindeki küçük kırıntıları komşularına katmak, en küçük haritalama birimini sağlamak.",
            "Net alanı En küçük alan'dan küçük alanlar küçüktür. Küçükler küçükten büyüğe, ortak sınırı (1 mm içinde) olan büyük komşulardan seçilene katılır; yalnız küçüklere değen küçük, onlar katıldıktan sonra katılır. Komşusu olmayan küçük olduğu gibi yazılır ve söylenir.",
            "Büyükler öznitelikleriyle yazılır; tablo her katılmayı söyler. Kotlar taşınmaz.",
        ]
        .join("\n\n")),
        keywords: ["kat", "küçük", "eliminate", "kırıntı", "birleştir", "komşu"]
            .map(String::from)
            .to_vec(),
        aliases: ["KUCUKKAT", "ELIMINATE"].map(String::from).to_vec(),
        icon: Some("eliminateSmall".into()),
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
            .describe("Küçükleri komşularına katılacak alanlar."),
            measure_param("minArea", "En küçük alan", "m²", 0.0, 100.0, "Bundan küçük alanlar komşusuna katılır."),
            ParamDef::new(
                "into",
                "Kime",
                ParamKind::Choice {
                    options: vec![
                        EnumOption::new("longest", "En uzun ortak sınır")
                            .hint("Ortak sınırı en uzun komşuya"),
                        EnumOption::new("largest", "En büyük komşu").hint("Alanı en büyük komşuya"),
                    ],
                },
            )
            .default_value(json!("longest")),
            layer_param("Katılan", "#F76B15"),
        ],
        outputs: vec![
            OutputDef::new("result", "Yazılan alanlar", OutputKind::Features),
            OutputDef::new("report", "Katılma raporu", OutputKind::Table),
            OutputDef::new("count", "Katılan alan sayısı", OutputKind::Number),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run),
    }
}

fn run(v: &Resolved<'_>, ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let inputs = &v.features("input").entities;
    input_notes(feedback, inputs, &[], false);
    let rule = Elimination {
        min_area: v.number("minArea").unwrap_or(100.0),
        largest: v.text("into") == "largest",
    };
    feedback.progress(0.0, "Küçük alanlar katılıyor");
    let objects = Inputs::of(inputs);
    let got = eliminate_run(&objects.refs(), &rule);
    let changed: BTreeSet<usize> = got.joined.iter().map(|j| j.into).collect();
    let layer = &v.layer("layer").id;
    let mut add = Vec::new();
    for (i, e) in inputs.iter().enumerate() {
        let Some(areas) = &got.areas[i] else { continue };
        let s = if changed.contains(&i) {
            area_shape(areas)
        } else {
            Some(objects.shapes[i].clone())
        };
        if let Some(s) = s {
            add.extend(new_object(s, layer, attrs_of(e)));
        }
    }
    let (ld, ad) = (ctx.units.length_decimals, ctx.units.area_decimals);
    let rows: Vec<Vec<String>> = got
        .joined
        .iter()
        .map(|j| {
            let size = got.sizes[j.small];
            vec![
                format!("#{}", inputs[j.small].base().id),
                format!("#{}", inputs[j.into].base().id),
                to_fixed(j.shared, ld),
                to_fixed(size, ad),
            ]
        })
        .collect();
    if !got.left.is_empty() {
        feedback.warn(format!(
            "{} küçük alanın büyük komşusu yok; olduğu gibi yazıldı.",
            got.left.len()
        ));
    }
    let n = add.len();
    let joined = got.joined.len();
    let summary = if joined > 0 {
        format!("{n} nesne yazıldı; {joined} küçük alan komşusuna katıldı.")
    } else {
        format!("{n} nesne yazıldı; katılacak küçük alan yok.")
    };
    finish(add, summary, joined, table(&COLUMNS, rows))
}
