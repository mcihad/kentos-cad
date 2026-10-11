//! Ulaşılmazlık kutbu (the web's `pole.ts`; docs/adr/0219 §6): each area's
//! point farthest from its edges, holes too, within the precision; the
//! part's whose pole is farthest in a multi-part area, a circle's centre.

use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::ops::generate::points::is_curved;
use kentos_geometry_core::ops::generate::pole::pole;
use kentos_native_application::geometry::shape;
use serde_json::json;

use super::super::geometry::{AREA_KINDS, layer_param, new_object};
use super::super::proximity::length_text;
use super::{input_notes, scopes};
use crate::types::{
    ChangeSet, Feedback, OutputDef, OutputKind, ParamDef, ParamKind, Resolved, RunContext,
    RunResult, Target, Tool,
};

pub fn tool() -> Tool {
    Tool {
        id: "generate.pole".into(),
        label: "Ulaşılmazlık kutbu".into(),
        category: "generation".into(),
        description: "Her alanın kenarlarından en uzak iç noktasını, kenara uzaklığıyla yeni katmana nokta olarak yazar.".into(),
        help: Some([
            "Örnekler: parsel numaralarını ve etiketleri alanın en geniş yerine koymak, ince uzun ya da L biçimli alanların içinde kalan bir nokta bulmak.",
            "Nokta alanın içindedir; delikler de kenar sayılır. Çok parçalı alanda kutbu en uzak olan parçanınki alınır, dairede merkezi. Hassasiyet kutbun en uzak noktaya ne kadar yakın olacağıdır; küçüldükçe süre uzar.",
            "Nokta alanın özniteliklerini ve kenara uzaklığını (Uzaklık) taşır.",
        ]
        .join("\n\n")),
        keywords: [
            "kutup", "ulaşılmazlık", "polylabel", "iç nokta", "merkez", "etiket noktası",
            "pole of inaccessibility",
        ]
        .map(String::from)
        .to_vec(),
        aliases: ["KUTUP", "POLYLABEL"].map(String::from).to_vec(),
        icon: Some("poleOfInaccessibility".into()),
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
            .describe("Kutbu bulunacak alanlar."),
            ParamDef::new(
                "precision",
                "Hassasiyet",
                ParamKind::Number {
                    min: Some(0.001),
                    max: None,
                    integer: false,
                    unit: "m".into(),
                    placeholder: None,
                },
            )
            .default_value(json!(0.1))
            .describe("Kutbun gerçek en uzak noktaya olabilecek uzaklığı."),
            layer_param("Kutuplar", "#E54666"),
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
    feedback.progress(0.0, "Kutuplar hesaplanıyor");
    let shapes: Vec<Shape> = inputs.iter().map(|e| shape(e)).collect();
    let curved = shapes
        .iter()
        .filter(|s| !matches!(s, Shape::Circle { .. }) && is_curved(s))
        .count();
    input_notes(feedback, inputs, curved);
    let precision = v.number("precision").unwrap_or(0.1);
    let layer = &v.layer("layer").id;
    let mut add = Vec::new();
    let mut empty = 0usize;
    for (e, s) in inputs.iter().zip(&shapes) {
        let Some((p, d)) = pole(s, precision) else {
            empty += 1;
            continue;
        };
        let mut attrs = e.base().attrs.clone();
        attrs.insert("Uzaklık".into(), length_text(ctx.units, d));
        add.extend(new_object(
            Shape::Point {
                p,
                z: None,
                parts: None,
            },
            layer,
            attrs,
        ));
    }
    if empty > 0 {
        feedback.warn(format!("{empty} nesnenin alanı yok; kutbu yazılmadı."));
    }
    let count = add.len();
    let summary = format!("{} nesneden {count} nokta yazıldı.", inputs.len());
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
