//! Sadeleştir (the web's `builtin/geometry/simplify.ts`; docs/adr/0201 §7):
//! Sadeleştir's rule (ADR 0140: a vertex within the tolerance of the chord
//! that would replace it goes, arc edges stay whole) on areas and polylines,
//! written with their attributes to the output layer; the report says each
//! changed object's vertices, its area's change and the largest deviation.

use kentos_geometry_core::ops::geoprocess::calls::simplify_run;
use serde_json::json;

use super::{
    SIMPLIFY_KINDS, attrs_of, features_param, geo_scopes, input_notes, layer_param, new_object,
    shapes,
};
use crate::text::to_fixed;
use crate::types::{
    ChangeSet, Feedback, OutputDef, OutputKind, ParamDef, ParamKind, Resolved, RunContext,
    RunResult, Target, Tool,
};

const COLUMNS: [&str; 6] = [
    "Nesne",
    "Köşe (önce)",
    "Köşe (sonra)",
    "Alan değişimi (m²)",
    "Alan değişimi (%)",
    "En büyük sapma (m)",
];

pub fn tool() -> Tool {
    Tool {
        id: "geometry.simplify".into(),
        label: "Sadeleştir".into(),
        category: "geometry".into(),
        description: "Alan ve çoklu çizgilerden toleranstan az sapan köşeleri atarak yeni katmana yazar; köşe sayılarını, alan değişimini ve en büyük sapmayı tabloda verir.".into(),
        help: Some([
            "Çizimdeki Sadeleştir’in kuralıdır: atılacak köşe, onun yerine geçecek doğrudan toleranstan az uzaktır; yay kenarlar bütün kalır, alan en az üç köşesini korur.",
            "Her nesne sonuçta bir kez yazılır (değişmeyen olduğu gibi). Komşu alanların ortak sınırları ayrı ayrı sadeleşir; ortak sınırı koruyan sadeleştirme ayrı bir iştir.",
        ]
        .join("\n\n")),
        keywords: [
            "sadeleştir", "basitleştir", "genelleştir", "simplify", "generalize", "douglas",
            "köşe azalt",
        ]
        .map(String::from)
        .to_vec(),
        aliases: ["SADELESTIRKATMAN", "GENERALIZE"].map(String::from).to_vec(),
        icon: Some("geoSimplify".into()),
        parameters: vec![
            features_param(
                "input",
                "Nesneler",
                &SIMPLIFY_KINDS,
                geo_scopes(),
                "Sadeleştirilecek kapalı alanlar ve çoklu çizgiler.",
            ),
            ParamDef::new(
                "tolerance",
                "Tolerans",
                ParamKind::Number {
                    min: Some(0.001),
                    max: Some(1000.0),
                    integer: false,
                    unit: "m".into(),
                    placeholder: None,
                },
            )
            .default_value(json!(0.1))
            .describe("Bundan az sapan köşeler atılır."),
            layer_param("Sadeleştirilen", "#AD7F58"),
        ],
        outputs: vec![
            OutputDef::new("simplified", "Yazılan nesneler", OutputKind::Features),
            OutputDef::new("report", "Sadeleştirme raporu", OutputKind::Table),
            OutputDef::new("count", "Atılan köşe sayısı", OutputKind::Number),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run),
    }
}

fn run(v: &Resolved<'_>, ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let input = &v.features("input").entities;
    input_notes(feedback, input, &[], false);
    feedback.progress(0.0, "Köşeler atılıyor");
    let tolerance = v.number("tolerance").unwrap_or(0.1);
    let (ld, ad) = (ctx.units.length_decimals, ctx.units.area_decimals);
    let layer = &v.layer("layer").id;
    let mut add = Vec::new();
    let mut rows: Vec<Vec<String>> = Vec::new();
    let (mut removed, mut worst) = (0usize, 0.0f64);
    for (e, s) in input.iter().zip(shapes(input)) {
        let r = simplify_run(&s, tolerance);
        if r.changed {
            removed += r.vertices[0] - r.vertices[1];
            worst = worst.max(r.deviation);
            let change = r.area.map(|[a0, a1]| a1 - a0);
            let share = r
                .area
                .zip(change)
                .filter(|([a0, _], _)| *a0 > 0.0)
                .map(|([a0, _], c)| c / a0 * 100.0);
            rows.push(vec![
                format!("#{}", e.base().id),
                r.vertices[0].to_string(),
                r.vertices[1].to_string(),
                change.map(|c| to_fixed(c, ad)).unwrap_or_default(),
                share.map(|p| to_fixed(p, 2)).unwrap_or_default(),
                to_fixed(r.deviation, ld),
            ]);
        }
        if let Some(object) = new_object(r.shape, layer, attrs_of(e)) {
            add.push(object);
        }
    }
    let count = add.len();
    let summary = if rows.is_empty() {
        format!("{count} nesne yazıldı; toleranstan az sapan köşe yok.")
    } else {
        format!(
            "{count} nesne yazıldı; {} nesnede {removed} köşe atıldı, en büyük sapma {} m.",
            rows.len(),
            to_fixed(worst, ld)
        )
    };
    let mut outputs: crate::types::Values =
        [("count".to_owned(), json!(removed))].into_iter().collect();
    if !rows.is_empty() {
        outputs.insert(
            "report".to_owned(),
            json!({ "columns": COLUMNS, "rows": rows }),
        );
    }
    RunResult {
        changes: Some(ChangeSet {
            add,
            ..ChangeSet::default()
        }),
        outputs,
        summary: Some(summary),
        ..RunResult::default()
    }
}
