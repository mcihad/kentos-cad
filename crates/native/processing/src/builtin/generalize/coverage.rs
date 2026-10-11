//! Ortak kenarları koruyarak sadeleştir (the web's `coverage.ts`; docs/adr/0221
//! §8): the coverage's edges simplified once each, GEOS's
//! CoverageSimplifier's way; neighbours keep sharing their edges.

use kentos_geometry_core::geom::region::net_area;
use kentos_geometry_core::ops::areas::areas_of_entity;
use kentos_geometry_core::ops::generalize::calls::coverage_run;
use kentos_geometry_core::ops::geoprocess::written_vertices;
use serde_json::json;

use super::super::convert::{placed, scopes};
use super::super::geometry::layer_param;
use super::{Inputs, finish, measure_param, table};
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
        id: "generalize.coverageSimplify".into(),
        label: "Ortak kenarları koruyarak sadeleştir".into(),
        category: "generalization".into(),
        description: "Komşu alanların ortak kenarlarını bir kez sadeleştirir; aralarında boşluk ya da örtüşme açılmaz.".into(),
        help: Some([
            "Örnekler: il, ilçe ve mahalle sınırlarını küçük ölçeğe indirmek, parsel katmanının köşelerini azaltmak.",
            "Üç ya da daha çok alanın buluştuğu köşeler kalır; aralarındaki kenarlar Visvalingam–Whyatt'la sadeleşir (GEOS'un ve QGIS'in kuralı): üçgeninin alanı toleransın karesinden küçük köşe atılır, üçgeninde başka bir köşe varsa kalır. Sınırı koru açıkken dış sınır değişmez.",
            "Komşular ortak köşelerini tam aynı koordinatla paylaşmalıdır; paylaşmayanların sınırı ayrı ayrı sadeleşir (önce Topolojik temizlik). Kotlar köşeleriyle kalır.",
        ]
        .join("\n\n")),
        keywords: [
            "sadeleştir", "ortak kenar", "kaplama", "coverage", "simplify", "visvalingam",
            "genelleştir",
        ]
        .map(String::from)
        .to_vec(),
        aliases: ["KAPLAMASADE", "COVERAGESIMPLIFY"].map(String::from).to_vec(),
        icon: Some("coverageSimplify".into()),
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
            .describe("Ortak kenarları birlikte sadeleştirilecek komşu alanlar."),
            measure_param(
                "tolerance",
                "Tolerans",
                "m",
                0.001,
                1.0,
                "Üçgeninin alanı bunun karesinden küçük köşeler atılır.",
            ),
            ParamDef::new("keepBoundary", "Sınırı koru", ParamKind::Boolean)
                .default_value(json!(false))
                .describe("Yalnız ortak kenarlar sadeleşir; dış sınır değişmez."),
            layer_param("Sadeleştirilen kaplama", "#3E63DD"),
        ],
        outputs: vec![
            OutputDef::new("simplified", "Yazılan alanlar", OutputKind::Features),
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
    let inputs = &v.features("input").entities;
    feedback.progress(0.0, "Ortak kenarlar sadeleşiyor");
    let objects = Inputs::of(inputs);
    let results = coverage_run(
        &objects.refs(),
        &objects.zs_refs(),
        v.number("tolerance").unwrap_or(1.0),
        v.flag("keepBoundary"),
    );
    let ad = ctx.units.area_decimals;
    let layer = &v.layer("layer").id;
    let mut add = Vec::new();
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut removed = 0usize;
    for ((e, old), r) in inputs.iter().zip(&objects.shapes).zip(results) {
        let Some(r) = r else { continue };
        let (b, a) = (written_vertices(old), written_vertices(&r.shape));
        if a != b {
            removed += b.saturating_sub(a);
            let net = |s| areas_of_entity(s).iter().map(net_area).sum::<f64>();
            let (a0, a1) = (net(old), net(&r.shape));
            let change = a1 - a0;
            rows.push(vec![
                format!("#{}", e.base().id),
                b.to_string(),
                a.to_string(),
                to_fixed(change, ad),
                to_fixed(change / a0 * 100.0, 2),
            ]);
        }
        add.extend(placed(r, layer, e.base().attrs.clone()));
    }
    let n = add.len();
    let summary = if rows.is_empty() {
        format!("{n} nesne yazıldı; atılacak köşe yok.")
    } else {
        format!(
            "{n} nesne yazıldı; {} nesnede {removed} köşe atıldı.",
            rows.len()
        )
    };
    finish(add, summary, removed, table(&COLUMNS, rows))
}
