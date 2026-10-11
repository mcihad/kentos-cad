//! Küçük parçaları ayıkla (the web's `parts.ts`; docs/adr/0221 §6): the
//! small parts and holes of areas gone, the largest part kept when all are
//! small; what stays unchanged, elevations too.

use kentos_geometry_core::ops::generalize::calls::parts_run;
use kentos_geometry_core::ops::generalize::parts::PartRule;
use kentos_native_application::geometry::shape;
use serde_json::json;

use super::super::convert::{placed, scopes, zs_of};
use super::super::geometry::layer_param;
use super::{finish, measure_param};
use crate::types::{
    Feedback, OutputDef, OutputKind, ParamDef, ParamKind, Resolved, RunContext, RunResult, Target,
    Tool,
};

pub fn tool() -> Tool {
    Tool {
        id: "generalize.dropParts".into(),
        label: "Küçük parçaları ayıkla".into(),
        category: "generalization".into(),
        description: "Çok parçalı alanların küçük parçalarını ve küçük delikleri atar; bütün parçaları küçükse en büyüğü kalır.".into(),
        help: Some([
            "Örnekler: sınır verisindeki kırıntı adaları ve iğne deliklerini temizlemek, sayısallaştırma artıklarını atmak.",
            "Net alanı En küçük alan'dan küçük ya da (Yüzde verilmişse) nesnenin dış halkalarının alanı toplamının yüzdesinden küçük parça ve delikler gider. Parçalar ve Delikler ayrı ayrı açılıp kapanır. Kalan halkalar köşeleri ve kotlarıyla değişmez.",
        ]
        .join("\n\n")),
        keywords: [
            "parça", "delik", "ayıkla", "eliminate polygon part", "kırıntı", "temizle",
        ]
        .map(String::from)
        .to_vec(),
        aliases: ["PARCAAYIKLA", "ELIMINATEPART"].map(String::from).to_vec(),
        icon: Some("dropParts".into()),
        parameters: vec![
            ParamDef::new(
                "input",
                "Alanlar",
                ParamKind::Features {
                    kinds: Some(vec!["polygon".into()]),
                    scopes: scopes(),
                    writes: false,
                },
            )
            .describe("Küçük parçaları ve delikleri atılacak alanlar."),
            measure_param("minArea", "En küçük alan", "m²", 0.0, 10.0, "Bundan küçük parça ve delikler atılır."),
            ParamDef::new(
                "percent",
                "Yüzde",
                ParamKind::Number {
                    min: Some(0.0),
                    max: Some(100.0),
                    integer: false,
                    unit: "%".into(),
                    placeholder: None,
                },
            )
            .default_value(json!(0))
            .describe("Dış halkaların alanı toplamının bu yüzdesinden küçükler de atılır; 0: kullanılmaz."),
            ParamDef::new("parts", "Parçalar", ParamKind::Boolean)
                .default_value(json!(true))
                .describe("Küçük parçalar atılır."),
            ParamDef::new("holes", "Delikler", ParamKind::Boolean)
                .default_value(json!(true))
                .describe("Küçük delikler doldurulur."),
            layer_param("Ayıklanan", "#12A594"),
        ],
        outputs: vec![
            OutputDef::new("trimmed", "Yazılan alanlar", OutputKind::Features),
            OutputDef::new("count", "Atılan parça ve delik sayısı", OutputKind::Number),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run),
    }
}

fn run(v: &Resolved<'_>, _ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let inputs = &v.features("input").entities;
    let rule = PartRule {
        min_area: v.number("minArea").unwrap_or(10.0),
        percent: v.number("percent").unwrap_or(0.0),
        parts: v.flag("parts"),
        holes: v.flag("holes"),
    };
    feedback.progress(0.0, "Parçalar ayıklanıyor");
    let layer = &v.layer("layer").id;
    let (mut parts, mut holes) = (0usize, 0usize);
    let mut add = Vec::new();
    for e in inputs {
        let Some(r) = parts_run(&shape(e), zs_of(e).as_ref(), &rule) else {
            continue;
        };
        parts += r.parts;
        holes += r.holes;
        add.extend(placed(r.out, layer, e.base().attrs.clone()));
    }
    let n = add.len();
    let summary = if parts + holes > 0 {
        format!("{n} nesne yazıldı; {parts} parça ve {holes} delik atıldı.")
    } else {
        format!("{n} nesne yazıldı; atılacak küçük parça ya da delik yok.")
    };
    finish(add, summary, parts + holes, None)
}
