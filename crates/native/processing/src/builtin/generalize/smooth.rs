//! Yumuşat (the web's `smooth.ts`; docs/adr/0221 §2): Chaikin's corner
//! cutting on lines and areas, the neighbours' shared edges together when
//! asked; elevations go along.

use kentos_geometry_core::jsmath::PI;
use kentos_geometry_core::ops::generalize::calls::smooth_run;
use kentos_geometry_core::ops::generalize::smooth::Smoothing;
use kentos_geometry_core::ops::geoprocess::written_vertices;
use serde_json::json;

use super::super::convert::{placed, scopes};
use super::super::geometry::layer_param;
use super::{Inputs, degrees_param, finish};
use crate::types::{
    Feedback, OutputDef, OutputKind, ParamDef, ParamKind, Resolved, RunContext, RunResult, Target,
    Tool,
};

pub fn tool() -> Tool {
    Tool {
        id: "generalize.smooth".into(),
        label: "Yumuşat".into(),
        category: "generalization".into(),
        description: "Çizgilerin ve alanların köşelerini keserek yumuşatır (Chaikin); komşu alanların ortak kenarları birlikte yumuşayabilir.".into(),
        help: Some([
            "Örnekler: sayısallaştırılmış dere ve yolları yumuşatmak, küçük ölçekli haritada kıyı ve orman sınırlarını yuvarlamak.",
            "Her yinelemede her düz kenar iki ucundan Kesim oranı kadar içeride iki nokta verir; köşe kesilir. Açık çizginin uçları, yayların uçları ve dönüşü En büyük açı'dan büyük köşeler olduğu gibi kalır; yaylar bütün kalır. Kotlar kenar boyunca aynı oranla taşınır.",
            "Ortak kenarları koru açıkken komşu alanların ortak kenarları bir kez yumuşar, iki komşu da aynı sonucu alır; üç ya da daha çok alanın buluştuğu köşeler kıpırdamaz. Komşular ortak köşelerini tam aynı koordinatla paylaşmalıdır.",
        ]
        .join("\n\n")),
        keywords: ["yumuşat", "düzgünleştir", "chaikin", "smooth", "kıvrım", "genelleştir"]
            .map(String::from)
            .to_vec(),
        aliases: ["YUMUSAT", "SMOOTHLINES"].map(String::from).to_vec(),
        icon: Some("smoothLine".into()),
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
            .describe("Yumuşatılacak çoklu çizgiler ve alanlar."),
            ParamDef::new(
                "iterations",
                "Yineleme",
                ParamKind::Number {
                    min: Some(1.0),
                    max: Some(10.0),
                    integer: true,
                    unit: String::new(),
                    placeholder: None,
                },
            )
            .default_value(json!(3))
            .describe("Köşe kesmenin kaç kez yineleneceği."),
            ParamDef::new(
                "offset",
                "Kesim oranı",
                ParamKind::Number {
                    min: Some(0.01),
                    max: Some(0.5),
                    integer: false,
                    unit: String::new(),
                    placeholder: None,
                },
            )
            .default_value(json!(0.25))
            .describe("Kenarın her ucundan kesilen pay; 0,5'te kenarların ortaları kalır."),
            degrees_param(
                "maxAngle",
                "En büyük açı",
                180.0,
                180.0,
                "Bundan çok dönen köşeler keskin kalır.",
            ),
            ParamDef::new("shared", "Ortak kenarları koru", ParamKind::Boolean)
                .default_value(json!(false))
                .describe("Komşu alanların ortak kenarları bir kez yumuşar."),
            layer_param("Yumuşatılan", "#0090FF"),
        ],
        outputs: vec![
            OutputDef::new("smoothed", "Yazılan nesneler", OutputKind::Features),
            OutputDef::new("count", "Yazılan nesne sayısı", OutputKind::Number),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run),
    }
}

fn run(v: &Resolved<'_>, _ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let inputs = &v.features("input").entities;
    let iterations = v.number("iterations").unwrap_or(3.0);
    let s = Smoothing {
        iterations: if iterations >= 1.0 {
            iterations as u32
        } else {
            1
        },
        offset: v.number("offset").unwrap_or(0.25),
        max_turn: v.number("maxAngle").unwrap_or(180.0) * PI / 180.0,
    };
    feedback.progress(0.0, "Köşeler kesiliyor");
    let objects = Inputs::of(inputs);
    let results = smooth_run(&objects.refs(), &objects.zs_refs(), &s, v.flag("shared"));
    let layer = &v.layer("layer").id;
    let (mut before, mut after) = (0, 0);
    let mut add = Vec::new();
    for ((e, s), r) in inputs.iter().zip(&objects.shapes).zip(results) {
        let Some(r) = r else { continue };
        before += written_vertices(s);
        after += written_vertices(&r.shape);
        add.extend(placed(r, layer, e.base().attrs.clone()));
    }
    let count = add.len();
    let summary = format!("{count} nesne yazıldı; köşe sayısı {before} → {after}.");
    finish(add, summary, count, None)
}
