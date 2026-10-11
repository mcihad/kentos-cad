//! Dik açılandır (the web's `square.ts`; docs/adr/0221 §3): corners near a
//! right angle or near straight made exact, iD's and QGIS's rule; the other
//! corners stay, elevations go with their vertices.

use kentos_geometry_core::ops::generalize::calls::square_run;
use kentos_geometry_core::ops::generalize::orthogonal::Squaring;
use kentos_native_application::geometry::shape;
use serde_json::json;

use super::super::convert::{placed, scopes, zs_of};
use super::super::geometry::layer_param;
use super::{degrees_param, finish};
use crate::text::to_fixed;
use crate::types::{
    Feedback, OutputDef, OutputKind, ParamDef, ParamKind, Resolved, RunContext, RunResult, Target,
    Tool,
};

pub fn tool() -> Tool {
    Tool {
        id: "generalize.orthogonalize".into(),
        label: "Dik açılandır".into(),
        category: "generalization".into(),
        description: "Dik açıya ve düzlüğe yakın köşeleri tam dik ve düz yapar (binalar, parseller); öbür köşeler olduğu gibi kalır.".into(),
        help: Some([
            "Örnekler: sayısallaştırılmış bina köşelerini dik yapmak, el çizimi yapı adalarını düzeltmek.",
            "Açısı dik açıya ya da düzlüğe Açı eşiği kadar yakın köşeler açıortayları boyunca adım adım kaydırılır (iD'nin ve QGIS'in kuralı); en kareye yakın sonuç yazılır. Daha uzak açılı köşeler (örneğin 45°) kalır. Açık çizginin uçları ve yayların uçları kıpırdamaz; kotlar köşeleriyle gider.",
        ]
        .join("\n\n")),
        keywords: ["dik", "açı", "kare", "orthogonalize", "bina", "doksan", "square"]
            .map(String::from)
            .to_vec(),
        aliases: ["DIKACI", "ORTHOGONALIZE"].map(String::from).to_vec(),
        icon: Some("orthogonalize".into()),
        parameters: vec![
            ParamDef::new(
                "input",
                "Nesneler",
                ParamKind::Features {
                    kinds: Some(vec!["polygon".into(), "polyline".into()]),
                    scopes: scopes(),
                    writes: false,
                },
            )
            .describe("Köşeleri dik açılandırılacak alanlar ve çoklu çizgiler."),
            degrees_param(
                "threshold",
                "Açı eşiği",
                45.0,
                15.0,
                "Dik açıya ya da düzlüğe bu kadar yakın köşeler düzeltilir.",
            ),
            ParamDef::new(
                "iterations",
                "En çok yineleme",
                ParamKind::Number {
                    min: Some(1.0),
                    max: Some(10000.0),
                    integer: true,
                    unit: String::new(),
                    placeholder: None,
                },
            )
            .default_value(json!(1000))
            .describe("Köşelerin en çok kaç adımda kaydırılacağı."),
            layer_param("Dik açılandırılan", "#7C66DC"),
        ],
        outputs: vec![
            OutputDef::new("squared", "Yazılan nesneler", OutputKind::Features),
            OutputDef::new("count", "Düzeltilen nesne sayısı", OutputKind::Number),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run),
    }
}

fn run(v: &Resolved<'_>, ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let inputs = &v.features("input").entities;
    let iterations = v.number("iterations").unwrap_or(1000.0);
    let rule = Squaring {
        threshold: v.number("threshold").unwrap_or(15.0),
        iterations: if iterations >= 1.0 {
            iterations as u32
        } else {
            1
        },
    };
    feedback.progress(0.0, "Köşeler düzeltiliyor");
    let layer = &v.layer("layer").id;
    let (mut moved, mut worst) = (0usize, 0.0f64);
    let mut add = Vec::new();
    for e in inputs {
        let Some(r) = square_run(&shape(e), zs_of(e).as_ref(), &rule) else {
            continue;
        };
        if r.moved {
            moved += 1;
        }
        worst = worst.max(r.shift);
        add.extend(placed(r.out, layer, e.base().attrs.clone()));
    }
    let n = add.len();
    let summary = if moved > 0 {
        format!(
            "{n} nesne yazıldı; {moved} nesnenin köşeleri dik açılandı, en büyük kayma {} m.",
            to_fixed(worst, ctx.units.length_decimals)
        )
    } else {
        format!("{n} nesne yazıldı; dik açıya ya da düzlüğe yakın köşe yok.")
    };
    finish(add, summary, moved, None)
}
