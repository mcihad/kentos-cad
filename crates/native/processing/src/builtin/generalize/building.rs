//! Bina sadeleştir (the web's `building.ts`; docs/adr/0221 §4): Ruas's
//! short-edge removal on buildings; a building whose result would grow to
//! more than twice its area or be invalid stays as it is.

use kentos_geometry_core::geom::region::net_area;
use kentos_geometry_core::jsmath::PI;
use kentos_geometry_core::ops::areas::areas_of_entity;
use kentos_geometry_core::ops::generalize::building::Building;
use kentos_geometry_core::ops::generalize::calls::{BuildingCase, building_run};
use kentos_geometry_core::ops::geoprocess::written_vertices;
use kentos_native_application::geometry::shape;

use super::super::convert::scopes;
use super::super::geometry::{attrs_of, input_notes, layer_param, new_object};
use super::{degrees_param, finish, measure_param, table};
use crate::text::to_fixed;
use crate::types::{
    Feedback, OutputDef, OutputKind, ParamDef, ParamKind, Resolved, RunContext, RunResult, Target,
    Tool,
};

const COLUMNS: [&str; 5] = [
    "Nesne",
    "Köşe (önce)",
    "Köşe (sonra)",
    "Alan değişimi (m²)",
    "Alan değişimi (%)",
];

pub fn tool() -> Tool {
    Tool {
        id: "generalize.building".into(),
        label: "Bina sadeleştir".into(),
        category: "generalization".into(),
        description: "Binaların en kısa kenardan kısa kenarlarını komşularına göre atar: basamaklar, çıkıntılar, girintiler ve pahlar gider, dik açılar kalır.".into(),
        help: Some([
            "Örnekler: büyük ölçekli bina ayak izlerini 1/5000 haritaya indirmek, küçük girinti ve çıkıntıları atmak.",
            "En kısa kenardan başlanır (Ruas'ın kuralı): komşu kenarları dikse uzatılıp buluşur; aynı yöne giderse (çıkıntı ya da girinti) kısa yanın ucundan kesilir; ters yöne giderse (basamak) iki kenar ortadan geçen tek kenar olur. Sonra kısa kenarlar yeniden aranır. Yaylı kenarlar ve yanlarındakiler kalır.",
            "Sadeleşen bina iki katından büyük ya da geçersiz çıkarsa olduğu gibi yazılır ve söylenir. Kotlar taşınmaz.",
        ]
        .join("\n\n")),
        keywords: [
            "bina", "sadeleştir", "genelleştir", "simplify building", "basamak", "çıkıntı",
            "girinti",
        ]
        .map(String::from)
        .to_vec(),
        aliases: ["BINASADE", "SIMPLIFYBUILDING"].map(String::from).to_vec(),
        icon: Some("buildingSimplify".into()),
        parameters: vec![
            ParamDef::new(
                "input",
                "Binalar",
                ParamKind::Features {
                    kinds: Some(vec!["polygon".into()]),
                    scopes: scopes(),
                    writes: false,
                },
            )
            .describe("Sadeleştirilecek bina alanları."),
            measure_param("minEdge", "En kısa kenar", "m", 0.01, 3.0, "Bundan kısa kenarlar atılır."),
            degrees_param(
                "parallel",
                "Paralel sınırı",
                45.0,
                20.0,
                "Komşu kenarların paralel sayıldığı açı farkı.",
            ),
            degrees_param(
                "orthogonal",
                "Dik sınırı",
                45.0,
                20.0,
                "Komşu kenarların dik sayıldığı açı farkı.",
            ),
            layer_param("Sadeleştirilen binalar", "#D6409F"),
        ],
        outputs: vec![
            OutputDef::new("simplified", "Yazılan binalar", OutputKind::Features),
            OutputDef::new("report", "Sadeleştirme raporu", OutputKind::Table),
            OutputDef::new("count", "Atılan kenar sayısı", OutputKind::Number),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run),
    }
}

fn net(s: &kentos_geometry_core::entity::Shape) -> f64 {
    areas_of_entity(s).iter().map(net_area).sum()
}

fn run(v: &Resolved<'_>, ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let inputs = &v.features("input").entities;
    input_notes(feedback, inputs, &[], false);
    let rule = Building {
        min_edge: v.number("minEdge").unwrap_or(3.0),
        parallel: v.number("parallel").unwrap_or(20.0) * PI / 180.0,
        orthogonal: v.number("orthogonal").unwrap_or(20.0) * PI / 180.0,
    };
    feedback.progress(0.0, "Kısa kenarlar atılıyor");
    let ad = ctx.units.area_decimals;
    let layer = &v.layer("layer").id;
    let mut add = Vec::new();
    let mut rows: Vec<Vec<String>> = Vec::new();
    let (mut edits, mut grew, mut invalid) = (0usize, 0usize, 0usize);
    for e in inputs {
        let old = shape(e);
        let Some(r) = building_run(&old, &rule) else {
            continue;
        };
        match r.case {
            BuildingCase::Grew => grew += 1,
            BuildingCase::Invalid => invalid += 1,
            BuildingCase::Changed => {
                edits += r.edits;
                let (a0, a1) = (net(&old), net(&r.shape));
                let change = a1 - a0;
                rows.push(vec![
                    format!("#{}", e.base().id),
                    written_vertices(&old).to_string(),
                    written_vertices(&r.shape).to_string(),
                    to_fixed(change, ad),
                    to_fixed(change / a0 * 100.0, 2),
                ]);
            }
            BuildingCase::Same => {}
        }
        add.extend(new_object(r.shape, layer, attrs_of(e)));
    }
    if grew > 0 {
        feedback.warn(format!(
            "{grew} binanın sadeleşmişi iki katından büyük çıktı; olduğu gibi yazıldı."
        ));
    }
    if invalid > 0 {
        feedback.warn(format!(
            "{invalid} binanın sadeleşmişi geçersiz çıktı; olduğu gibi yazıldı."
        ));
    }
    let n = add.len();
    let summary = if rows.is_empty() {
        format!("{n} nesne yazıldı; atılacak kısa kenar yok.")
    } else {
        format!(
            "{n} nesne yazıldı; {} binada {edits} kısa kenar atıldı.",
            rows.len()
        )
    };
    finish(add, summary, edits, table(&COLUMNS, rows))
}
