//! Voronoi alanları and Delaunay üçgenleri (the web's `triangulation.ts`;
//! docs/adr/0219 §2, §3): each point's cell inside a frame round the points,
//! and the points' triangles, from the core's exact triangulation.

use std::collections::BTreeMap;

use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::ops::generate::voronoi::{
    Cell, delaunay_triangles, distinct, frame_of, voronoi,
};
use kentos_native_application::elevation;
use serde_json::json;

use super::super::geometry::{has_elevation, layer_param, new_object};
use super::super::proximity::{field, name_of};
use super::{scopes, sites_of};
use crate::types::{
    ChangeSet, Feedback, OutputDef, OutputKind, ParamDef, ParamKind, Resolved, RunContext,
    RunResult, Target, Tool,
};

fn points(description: &str) -> ParamDef {
    ParamDef::new(
        "input",
        "Noktalar",
        ParamKind::Features {
            kinds: Some(vec!["point".into()]),
            scopes: scopes(),
            writes: false,
        },
    )
    .describe(description)
}

fn written(add: Vec<kentos_commands::Entity>, summary: String) -> RunResult {
    let count = add.len();
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

// ── Voronoi alanları ─────────────────────────────────────────────────

pub fn voronoi_tool() -> Tool {
    Tool {
        id: "generate.voronoi".into(),
        label: "Voronoi alanları".into(),
        category: "generation".into(),
        description: "Her noktanın, kendisine öbür noktalardan daha yakın yerlerini (Thiessen alanı) öznitelikleriyle yeni katmana yazar.".into(),
        help: Some([
            "Örnekler: yağış istasyonlarının etki alanları, okulların ya da sağlık ocaklarının hizmet bölgeleri, ölçü noktalarının sorumluluk alanları.",
            "Alanlar noktaların sınır kutusunun her yandan uzun kenarının Genişletme yüzdesi kadar büyütülmüşüne kırpılır; bir sınırın içinde kalsın isteniyorsa ardından Kırp kullanılır. Komşu alanlar ortak köşelerini aynen paylaşır.",
            "Her alan noktasının özniteliklerini alır; çok noktalı nesnenin her noktası bir alandır, Parça 1, 2 … yazılır. Başka bir noktayla aynı yerdeki nokta alan vermez, söylenir.",
        ]
        .join("\n\n")),
        keywords: ["voronoi", "thiessen", "etki alanı", "en yakın", "bölge", "hücre"]
            .map(String::from)
            .to_vec(),
        aliases: vec!["VORONOI".into(), "THIESSEN".into()],
        icon: Some("voronoi".into()),
        parameters: vec![
            points("Alanları üretilecek noktalar."),
            ParamDef::new(
                "margin",
                "Genişletme",
                ParamKind::Number {
                    min: Some(0.0),
                    max: Some(1000.0),
                    integer: false,
                    unit: "%".into(),
                    placeholder: None,
                },
            )
            .default_value(json!(10))
            .describe("Çerçeve, noktaların sınır kutusunun her yandan uzun kenarının bu yüzdesi kadar büyütülmüşüdür."),
            layer_param("Voronoi", "#3E63DD"),
        ],
        outputs: vec![
            OutputDef::new("cells", "Yazılan alanlar", OutputKind::Features),
            OutputDef::new("count", "Alan sayısı", OutputKind::Number),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run_voronoi),
    }
}

fn run_voronoi(v: &Resolved<'_>, _ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let inputs = &v.features("input").entities;
    let sites = sites_of(inputs);
    feedback.progress(0.0, "Alanlar hesaplanıyor");
    let places: Vec<_> = sites.iter().map(|s| s.p).collect();
    let margin = v.number("margin").unwrap_or(10.0) / 100.0;
    let cells = match frame_of(&places, margin) {
        Some(f) => voronoi(&places, f),
        None => Err(kentos_geometry_core::ops::generate::voronoi::TOO_FEW_SITES.to_owned()),
    };
    let cells = match cells {
        Ok(c) => c,
        Err(why) => return RunResult::refused(why),
    };
    let layer = &v.layer("layer").id;
    let (mut same, mut empty) = (0usize, 0usize);
    let mut add = Vec::new();
    for (s, c) in sites.iter().zip(cells) {
        match c {
            Cell::Same => same += 1,
            Cell::Empty => empty += 1,
            Cell::Ring(pts) => {
                let mut attrs: BTreeMap<String, String> = inputs[s.at].base().attrs.clone();
                if let Some(k) = s.part {
                    attrs.insert("Parça".into(), k.to_string());
                }
                let shape = Shape::Polygon {
                    pts,
                    bulges: None,
                    holes: None,
                    parts: None,
                };
                add.extend(new_object(shape, layer, attrs));
            }
        }
    }
    if same > 0 {
        feedback.warn(format!(
            "{same} nokta başka bir noktayla aynı yerde; alanı yazılmadı."
        ));
    }
    let heights = inputs.iter().filter(|e| has_elevation(e)).count();
    if heights > 0 {
        feedback.warn(format!("{heights} nesnenin kotları sonuca taşınmadı."));
    }
    if empty > 0 {
        feedback.warn(format!(
            "{empty} noktanın alanı çerçevenin içinde alan değil; yazılmadı."
        ));
    }
    let summary = format!("{} noktadan {} alan yazıldı.", sites.len(), add.len());
    written(add, summary)
}

// ── Delaunay üçgenleri ───────────────────────────────────────────────

pub fn delaunay_tool() -> Tool {
    Tool {
        id: "generate.delaunay".into(),
        label: "Delaunay üçgenleri".into(),
        category: "generation".into(),
        description: "Noktaların Delaunay üçgenlerini, köşelerinin adları ve kotlarıyla yeni katmana alan olarak yazar.".into(),
        help: Some([
            "Örnekler: ölçü noktalarından üçgen ağı (TIN) kurmak, komşu noktaları bulmak, yüzeyin yüzlerini görmek.",
            "Üçgenler kesin Delaunay üçgenleridir: hiçbir üçgenin çevrel çemberinde başka nokta yoktur. Aynı çemberdeki dört noktada köşegen girdinin sırasıyla seçilir.",
            "Her üçgen Nokta 1, Nokta 2 ve Nokta 3 alanlarında köşelerinin adlarını taşır: Ad alanındaki değer, boşsa etiket, o da yoksa girdideki sırası; çok noktalı nesnenin noktaları ad/1, ad/2 … Noktaların kotları köşelerin kotu olur. Başka bir noktayla aynı yerdeki nokta bir kez girer, söylenir.",
        ]
        .join("\n\n")),
        keywords: ["delaunay", "üçgen", "tin", "üçgenleme", "triangulation", "üçgen ağı"]
            .map(String::from)
            .to_vec(),
        aliases: vec!["DELAUNAY".into(), "UCGENLE".into()],
        icon: Some("delaunay".into()),
        parameters: vec![
            points("Üçgenlenecek noktalar."),
            field("name", "Ad alanı", "input", false, false)
                .optional()
                .describe("Köşelerin adları bu alandan; boşsa etiket, o da yoksa sıra."),
            layer_param("Üçgenler", "#12A594"),
        ],
        outputs: vec![
            OutputDef::new("triangles", "Yazılan üçgenler", OutputKind::Features),
            OutputDef::new("count", "Üçgen sayısı", OutputKind::Number),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run_delaunay),
    }
}

fn run_delaunay(v: &Resolved<'_>, _ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let inputs = &v.features("input").entities;
    let sites = sites_of(inputs);
    feedback.progress(0.0, "Üçgenler hesaplanıyor");
    let places: Vec<_> = sites.iter().map(|s| s.p).collect();
    let tris = match delaunay_triangles(&places) {
        Ok(t) => t,
        Err(why) => return RunResult::refused(why),
    };
    let field = crate::text::js_trim(v.text("name"));
    let names: Vec<String> = sites
        .iter()
        .map(|s| {
            let n = name_of(inputs[s.at], field, s.at);
            match s.part {
                Some(k) => format!("{n}/{k}"),
                None => n,
            }
        })
        .collect();
    let layer = &v.layer("layer").id;
    let mut add = Vec::with_capacity(tris.len());
    for t in tris {
        let attrs = BTreeMap::from([
            ("Nokta 1".to_owned(), names[t[0]].clone()),
            ("Nokta 2".to_owned(), names[t[1]].clone()),
            ("Nokta 3".to_owned(), names[t[2]].clone()),
        ]);
        let shape = Shape::Polygon {
            pts: t.iter().map(|&i| sites[i].p).collect(),
            bulges: None,
            holes: None,
            parts: None,
        };
        if let Some(mut e) = new_object(shape, layer, attrs) {
            let zs: Vec<Option<f64>> = t.iter().map(|&i| sites[i].z).collect();
            if zs.iter().any(Option::is_some) {
                elevation::assign(&mut e, &[zs]);
            }
            add.push(e);
        }
    }
    let same = sites.len() - distinct(&places).places.len();
    if same > 0 {
        feedback.warn(format!(
            "{same} nokta başka bir noktayla aynı yerde; üçgenlere bir kez girdi."
        ));
    }
    let summary = format!("{} noktadan {} üçgen yazıldı.", sites.len(), add.len());
    written(add, summary)
}
