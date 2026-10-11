//! Sınırlayıcı geometri and İçbükey zarf (the web's `bounding.ts`;
//! docs/adr/0219 §4, §5): each object's, all objects' or each field value's
//! convex hull, box, rotated box of least area, least enclosing circle or
//! concave hull, with its measures.

use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::ops::generate::bounding::{Bound, Kind};
use kentos_geometry_core::ops::generate::points::is_curved;
use kentos_geometry_core::ops::generate::{bound_shape, bounding_run, concave_run, hull_area};
use kentos_native_application::geometry::shape;
use serde_json::json;

use super::super::geometry::{GEO_KINDS, layer_param, new_object};
use super::super::proximity::{area_text, bearing_text, length_text};
use super::{empty_note, group_params, grouped_summary, grouping, input_notes, scopes};
use crate::types::{
    ChangeSet, EnumOption, Feedback, OutputDef, OutputKind, ParamDef, ParamKind, Resolved,
    RunContext, RunResult, Target, Tool,
};

fn objects(description: &str) -> ParamDef {
    ParamDef::new(
        "input",
        "Nesneler",
        ParamKind::Features {
            kinds: Some(GEO_KINDS.iter().map(|k| (*k).to_owned()).collect()),
            scopes: scopes(),
            writes: false,
        },
    )
    .describe(description)
}

fn written(add: Vec<kentos_contracts::Entity>, summary: String) -> RunResult {
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

// ── Sınırlayıcı geometri ─────────────────────────────────────────────

pub fn bounding_tool() -> Tool {
    let mut parameters = vec![
        objects("Sarılacak nesneler."),
        ParamDef::new(
            "kind",
            "Tür",
            ParamKind::Choice {
                options: vec![
                    EnumOption::new("hull", "Dışbükey zarf")
                        .hint("Noktaları saran en küçük dışbükey alan"),
                    EnumOption::new("box", "Sınır kutusu").hint("Eksenlere paralel dikdörtgen"),
                    EnumOption::new("rotated", "Döndürülmüş kutu")
                        .hint("En küçük alanlı dikdörtgen"),
                    EnumOption::new("circle", "Daire").hint("En küçük çevreleyen daire"),
                ],
            },
        )
        .default_value(json!("hull")),
    ];
    parameters.extend(group_params("each"));
    parameters.push(layer_param("Sınırlayıcı", "#F76B15"));
    Tool {
        id: "generate.bounding".into(),
        label: "Sınırlayıcı geometri".into(),
        category: "generation".into(),
        description: "Nesneleri saran dışbükey zarfı, sınır kutusunu, en küçük döndürülmüş kutuyu ya da en küçük daireyi ölçüleriyle yeni katmana yazar.".into(),
        help: Some([
            "Örnekler: yapıların döndürülmüş kutusuyla yönünü ve boyunu bulmak, bir nokta kümesinin kapladığı alanı çizmek, adaların sınır kutularını almak, bir kuyunun koruma dairesini çizmek.",
            "Dışbükey zarf noktaları içine alan en küçük dışbükey alandır. Sınır kutusu eksenlere paraleldir. Döndürülmüş kutu en küçük alanlı dikdörtgendir; bir kenarı zarfın bir kenarındadır. Daire en küçük çevreleyen dairedir.",
            "Gruplama Her nesne iken her nesnenin, Hepsi iken bütün nesnelerin, Alana göre iken alanın değeri aynı olanların sınırlayıcısı yazılır. Ölçüler: Zarf alanı; Kutu genişliği ve Kutu yüksekliği; Kutu uzunluğu, Kutu genişliği ve uzun kenarın Kutu semti; Daire yarıçapı. Yaylar, elipsler ve eğriler 0,1 mm içinde noktalarla alınır. Alanı olmayan sonuç (bir nokta, bir doğru) yazılmaz.",
        ]
        .join("\n\n")),
        keywords: [
            "sınırlayıcı", "dışbükey", "zarf", "kutu", "döndürülmüş", "daire", "convex hull", "bounding",
            "envelope", "mbr",
        ]
        .map(String::from)
        .to_vec(),
        aliases: ["SINIRLAYICI", "ZARF", "MBG"].map(String::from).to_vec(),
        icon: Some("boundingGeometry".into()),
        parameters,
        outputs: vec![
            OutputDef::new("bounds", "Yazılan sınırlayıcılar", OutputKind::Features),
            OutputDef::new("count", "Sınırlayıcı sayısı", OutputKind::Number),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run_bounding),
    }
}

fn what(kind: Kind) -> &'static str {
    match kind {
        Kind::Hull => "dışbükey zarf",
        Kind::Box => "sınır kutusu",
        Kind::Rotated => "döndürülmüş kutu",
        Kind::Circle => "daire",
    }
}

fn run_bounding(v: &Resolved<'_>, ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let inputs = &v.features("input").entities;
    let Some(kind) = Kind::from_key(v.text("kind")) else {
        return RunResult::refused(format!(
            "Bilinmeyen sınırlayıcı geometri: {}",
            v.text("kind")
        ));
    };
    let g = grouping(inputs, v.text("group"), v.text("field"));
    feedback.progress(0.0, "Sınırlayıcılar hesaplanıyor");
    let shapes: Vec<Shape> = inputs.iter().map(|e| shape(e)).collect();
    input_notes(
        feedback,
        inputs,
        shapes.iter().filter(|s| is_curved(s)).count(),
    );
    let layer = &v.layer("layer").id;
    let units = ctx.units;
    let mut add = Vec::new();
    let mut empty = 0usize;
    for (b, mut attrs) in bounding_run(&shapes, &g.groups, g.count, kind)
        .into_iter()
        .zip(g.attrs)
    {
        let Some(b) = b else {
            empty += 1;
            continue;
        };
        match &b {
            Bound::Hull { area, .. } => {
                attrs.insert("Zarf alanı".into(), area_text(units, *area));
            }
            Bound::Box { width, height, .. } => {
                attrs.insert("Kutu genişliği".into(), length_text(units, *width));
                attrs.insert("Kutu yüksekliği".into(), length_text(units, *height));
            }
            Bound::Rotated {
                length,
                width,
                bearing,
                ..
            } => {
                attrs.insert("Kutu uzunluğu".into(), length_text(units, *length));
                attrs.insert("Kutu genişliği".into(), length_text(units, *width));
                attrs.insert("Kutu semti".into(), bearing_text(units, *bearing));
            }
            Bound::Circle { r, .. } => {
                attrs.insert("Daire yarıçapı".into(), length_text(units, *r));
            }
        }
        add.extend(new_object(bound_shape(&b), layer, attrs));
    }
    empty_note(feedback, empty);
    let summary = grouped_summary(
        inputs.len(),
        g.count,
        v.text("group"),
        add.len(),
        what(kind),
    );
    written(add, summary)
}

// ── İçbükey zarf ─────────────────────────────────────────────────────

pub fn concave_tool() -> Tool {
    let mut parameters = vec![
        objects("Noktaları sarılacak nesneler."),
        ParamDef::new(
            "ratio",
            "Oran",
            ParamKind::Number {
                min: Some(0.0),
                max: Some(1.0),
                integer: false,
                unit: String::new(),
                placeholder: None,
            },
        )
        .default_value(json!(0.3))
        .describe("0 en oyuk, 1 dışbükey zarf."),
        ParamDef::new("holes", "Delikler", ParamKind::Boolean)
            .default_value(json!(false))
            .describe("Zarfın içinde delikler açılabilir."),
    ];
    parameters.extend(group_params("all"));
    parameters.push(layer_param("İçbükey zarf", "#8E4EC6"));
    Tool {
        id: "generate.concaveHull".into(),
        label: "İçbükey zarf".into(),
        category: "generation".into(),
        description: "Nesnelerin noktalarını saran, içe doğru oyulmuş zarfı (isteğe bağlı delikleriyle) alanıyla yeni katmana yazar.".into(),
        help: Some([
            "Örnekler: bir yerleşimin evlerinin kapladığı alanı, bir nokta bulutunun ya da kümenin gerçek biçimini çizmek.",
            "Noktaların Delaunay üçgenleri dıştan içe aşındırılır: sınırdaki kenarı en uzun olan üçgen önce gider, kenarı Oran’ın verdiği uzunluktan kısa olunca durulur (GEOS’un ve JTS’in kuralı). Oran 0 en oyuk zarfı, 1 dışbükey zarfı verir. Alan tek parça kalır.",
            "Delikler açıkken içteki uzun kenarlı üçgenler sınıra değmeden delik açar. Gruplama ve öznitelikler Sınırlayıcı geometri’ninki gibidir; Zarf alanı yazılır.",
        ]
        .join("\n\n")),
        keywords: ["içbükey", "zarf", "concave hull", "alfa", "kapladığı alan", "küme sınırı"]
            .map(String::from)
            .to_vec(),
        aliases: ["ICBUKEYZARF", "CONCAVEHULL"].map(String::from).to_vec(),
        icon: Some("concaveHull".into()),
        parameters,
        outputs: vec![
            OutputDef::new("hulls", "Yazılan zarflar", OutputKind::Features),
            OutputDef::new("count", "Zarf sayısı", OutputKind::Number),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run_concave),
    }
}

fn run_concave(v: &Resolved<'_>, ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let inputs = &v.features("input").entities;
    let g = grouping(inputs, v.text("group"), v.text("field"));
    feedback.progress(0.0, "Zarflar hesaplanıyor");
    let shapes: Vec<Shape> = inputs.iter().map(|e| shape(e)).collect();
    input_notes(
        feedback,
        inputs,
        shapes.iter().filter(|s| is_curved(s)).count(),
    );
    let ratio = v.number("ratio").unwrap_or(0.3);
    let layer = &v.layer("layer").id;
    let mut add = Vec::new();
    let mut empty = 0usize;
    for (rings, mut attrs) in concave_run(&shapes, &g.groups, g.count, ratio, v.flag("holes"))
        .into_iter()
        .zip(g.attrs)
    {
        let Some((s, area)) = rings.and_then(|r| hull_area(&r)) else {
            empty += 1;
            continue;
        };
        attrs.insert("Zarf alanı".into(), area_text(ctx.units, area));
        add.extend(new_object(s, layer, attrs));
    }
    empty_note(feedback, empty);
    let summary = grouped_summary(
        inputs.len(),
        g.count,
        v.text("group"),
        add.len(),
        "içbükey zarf",
    );
    written(add, summary)
}
