//! Hat boyunca nokta (the web's `along.ts`; docs/adr/0220 §5): points along
//! each path of the lines and of the areas' boundaries, every so far or at
//! equal parts, with their distance, bearing and elevation.

use kentos_geometry_core::ops::sampling::MOST;
use kentos_geometry_core::ops::sampling::along::{Spacing, how_many, marks};
use kentos_geometry_core::ops::sampling::walk::walks_of;
use kentos_native_application::geometry::shape;
use serde_json::{Value, json};

use super::super::convert::{point, zs_of};
use super::super::geometry::{AREA_KINDS, layer_param};
use super::super::proximity::{bearing_text, length_text};
use super::{LINE_KINDS, curved_note, scopes, written};
use crate::types::{
    EnumOption, Feedback, OutputDef, OutputKind, ParamDef, ParamKind, Resolved, RunContext,
    RunResult, Target, Tool, Values,
};

fn by_interval(v: &Values) -> bool {
    v.get("mode").and_then(Value::as_str) != Some("parts")
}

fn by_parts(v: &Values) -> bool {
    !by_interval(v)
}

fn metres(name: &str, label: &str, min: f64, fallback: f64, description: &str) -> ParamDef {
    ParamDef::new(
        name,
        label,
        ParamKind::Number {
            min: Some(min),
            max: None,
            integer: false,
            unit: "m".into(),
            placeholder: None,
        },
    )
    .default_value(json!(fallback))
    .describe(description)
}

pub fn tool() -> Tool {
    let mut kinds: Vec<String> = LINE_KINDS.iter().map(|k| (*k).to_owned()).collect();
    kinds.extend(AREA_KINDS.iter().map(|k| (*k).to_owned()));
    Tool {
        id: "sample.alongLines".into(),
        label: "Hat boyunca nokta".into(),
        category: "sampling".into(),
        description: "Çizgilerin ve alan sınırlarının üstüne aralıkla ya da eşit parçalara noktalar koyar; noktalar uzaklık ve semt alır.".into(),
        help: Some([
            "Örnekler: bir enerji hattına 50 metrede bir direk yeri, bir yolun 20 metrelik enkesit yerleri, bir parsel sınırına eşit aralıklı noktalar koymak.",
            "Aralıkla iken noktalar yolun başından Başlangıç payı ötede başlar, Aralık kadar ilerler, sondan Bitiş payı kadar önce durur; Uçları da açıksa yolun başı ve sonu da eklenir. Eşit parçalara iken yol Parça sayısı kadar eşit parçaya bölünür. Kapalı yolun (alan sınırı, daire) sonu başıdır.",
            "Nokta nesnenin özniteliklerini, yolun başından Uzaklık'ı ve oradaki gidiş doğrultusunu Semt olarak (köşede sonraki kenarınki) alır; birden çok yolu olan nesnede Parça. Kot köşelerin kotlarından doğrusal taşınır. Yaylar kesin; elips ve eğri 0,1 mm'lik kirişlerle.",
        ]
        .join("\n\n")),
        keywords: [
            "hat", "boyunca", "nokta", "aralık", "direk", "enkesit", "points along lines",
            "parçala", "böl",
        ]
        .map(String::from)
        .to_vec(),
        aliases: ["HATNOKTA", "ARALIKNOKTA"].map(String::from).to_vec(),
        icon: Some("pointsAlongLine".into()),
        parameters: vec![
            ParamDef::new(
                "input",
                "Çizgiler",
                ParamKind::Features {
                    kinds: Some(kinds),
                    scopes: scopes(),
                    writes: false,
                },
            )
            .describe("Üstüne nokta konacak çizgiler ve sınırlarına nokta konacak alanlar."),
            ParamDef::new(
                "mode",
                "Yerleşim",
                ParamKind::Choice {
                    options: vec![
                        EnumOption::new("interval", "Aralıkla").hint("Belli uzaklıkta bir"),
                        EnumOption::new("parts", "Eşit parçalara").hint("Yolu eşit parçalara böler"),
                    ],
                },
            )
            .default_value(json!("interval")),
            metres("step", "Aralık", 0.001, 10.0, "İki nokta arası yol boyunca uzaklık.").shown_when(by_interval),
            metres("start", "Başlangıç payı", 0.0, 0.0, "İlk noktanın yolun başından uzaklığı.").shown_when(by_interval),
            metres("end", "Bitiş payı", 0.0, 0.0, "Son noktanın yolun sonuna en az uzaklığı.").shown_when(by_interval),
            ParamDef::new(
                "parts",
                "Parça sayısı",
                ParamKind::Number {
                    min: Some(1.0),
                    max: None,
                    integer: true,
                    unit: String::new(),
                    placeholder: None,
                },
            )
            .default_value(json!(10))
            .describe("Yolun bölüneceği eşit parça sayısı.")
            .shown_when(by_parts),
            ParamDef::new("ends", "Uçları da", ParamKind::Boolean)
                .default_value(json!(true))
                .describe("Yolun başı ve sonu da nokta olur."),
            layer_param("Hat noktaları", "#F76B15"),
        ],
        outputs: vec![
            OutputDef::new("points", "Yazılan noktalar", OutputKind::Features),
            OutputDef::new("count", "Nokta sayısı", OutputKind::Number),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run),
    }
}

fn run(v: &Resolved<'_>, ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let inputs = &v.features("input").entities;
    let spacing = if v.text("mode") == "parts" {
        let n = v.number("parts").unwrap_or(10.0);
        Spacing::Parts(if n >= 1.0 { n as u64 } else { 1 })
    } else {
        Spacing::Interval {
            step: v.number("step").unwrap_or(10.0),
            start: v.number("start").unwrap_or(0.0),
            end: v.number("end").unwrap_or(0.0),
        }
    };
    let ends = v.flag("ends");
    feedback.progress(0.0, "Noktalar yerleştiriliyor");
    let walks: Vec<_> = inputs
        .iter()
        .map(|e| walks_of(&shape(e), zs_of(e).as_ref()))
        .collect();
    let total: f64 = walks
        .iter()
        .flatten()
        .map(|w| how_many(w.length, &spacing, ends))
        .sum();
    if total > MOST as f64 {
        return RunResult::refused(
            "1 000 000'dan çok nokta yazılırdı; sayıyı ya da aralığı değiştirin.".to_owned(),
        );
    }
    curved_note(feedback, inputs);
    let layer = &v.layer("layer").id;
    let mut add = Vec::new();
    for (e, w) in inputs.iter().zip(&walks) {
        for m in marks(w, &spacing, ends) {
            let mut attrs = e.base().attrs.clone();
            attrs.insert("Uzaklık".to_owned(), length_text(ctx.units, m.s));
            attrs.insert("Semt".to_owned(), bearing_text(ctx.units, m.bearing));
            if w.len() > 1 {
                attrs.insert("Parça".to_owned(), (m.path + 1).to_string());
            }
            add.extend(point(m.p, m.z, layer, attrs));
        }
    }
    let summary = format!("{} nesneden {} nokta yazıldı.", inputs.len(), add.len());
    written(add, summary)
}
