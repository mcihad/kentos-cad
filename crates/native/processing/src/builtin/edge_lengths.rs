//! Kenar uzunluklarını yaz (the web's `builtin/edgeLengths.ts`): every edge
//! of the chosen areas, paths and lines gets its length as text, centred on
//! the edge and turned to read along it, outside (or inside) the shape. An
//! edge two parcels share is written once. Where each label goes and which
//! edges are shared come from the geometry core (docs/adr/0008, S4).

use std::collections::BTreeMap;

use kentos_contracts::{Entity, EntityBase, TextEntity};
use kentos_domain::Slot;
use serde_json::{Value, json};

use crate::text::to_fixed;
use crate::types::{
    ChangeSet, EnumOption, Feedback, NewLayerStyle, OutputDef, OutputKind, ParamDef, ParamKind,
    Resolved, RunContext, RunResult, Target, Tool,
};

fn decimals(x: f64) -> u32 {
    x.clamp(0.0, 100.0) as u32
}

pub fn tool() -> Tool {
    let number =
        |name: &str, label: &str, min: Option<f64>, max: Option<f64>, integer: bool, unit: &str| {
            ParamDef::new(
                name,
                label,
                ParamKind::Number {
                    min,
                    max,
                    integer,
                    unit: unit.into(),
                },
            )
        };
    let affix = |name: &str, label: &str| {
        ParamDef::new(
            name,
            label,
            ParamKind::Text {
                placeholder: None,
                max_length: Some(12),
                allow_empty: true,
            },
        )
        .default_value(json!(""))
        .advanced()
    };
    Tool {
        id: "annotation.edgeLengths".into(),
        label: "Kenar uzunluklarını yaz".into(),
        category: "annotation".into(),
        description: "Alan, çoklu çizgi ve çizgilerin her kenarına uzunluğunu yazar; ortak kenarlar bir kez yazılır.".into(),
        help: Some([
            "Yazı kenarın ortasına, kenar boyunca okunur biçimde konur: kapalı alanlarda dışa (ya da içe), açık çizgilerde sola (ya da sağa).",
            "Yay kenarlarında yay boyu yazılır. Komşu parsellerin ortak kenarı bir kez yazılır; bunu kapatırsanız her alan kendi kenarını yazar.",
        ]
        .join("\n\n")),
        keywords: ["kenar", "uzunluk", "ölçü", "yazı", "edge", "length", "label", "parsel"]
            .map(String::from)
            .to_vec(),
        aliases: ["KENARYAZ", "KENARUZUNLUK"].map(String::from).to_vec(),
        icon: Some("edgeLengths".into()),
        parameters: vec![
            ParamDef::new(
                "input",
                "Nesneler",
                ParamKind::Features {
                    kinds: Some(vec!["polygon".into(), "polyline".into(), "line".into()]),
                    scopes: None,
                    writes: false,
                },
            )
            .describe("Kenar uzunlukları yazılacak alanlar, çoklu çizgiler ve çizgiler.")
            .default_value(json!({ "scope": "selection" })),
            number("decimals", "Ondalık basamak", Some(0.0), Some(6.0), true, "")
                .default_from(|d| json!(d.length_decimals)),
            ParamDef::new(
                "side",
                "Yazının yeri",
                ParamKind::Choice {
                    options: vec![
                        EnumOption::new("outside", "Dışa").hint("Kapalı alanın dışına; açık çizgide soluna"),
                        EnumOption::new("inside", "İçe").hint("Kapalı alanın içine; açık çizgide sağına"),
                    ],
                },
            )
            .default_value(json!("outside")),
            number("textHeight", "Yazı yüksekliği", Some(0.1), Some(50.0), false, "mm")
                .default_value(json!(2))
                .describe("Kâğıt üzerinde, çizim ölçeğine göre."),
            affix("prefix", "Önek"),
            affix("suffix", "Sonek").describe("Örneğin “ m”."),
            number("minLength", "En kısa kenar", Some(0.0), None, false, "m")
                .default_value(json!(0))
                .advanced()
                .describe("Bundan kısa kenarlar yazılmaz."),
            ParamDef::new("shared", "Ortak kenarlara bir kez yaz", ParamKind::Boolean)
                .default_value(json!(true)),
            ParamDef::new(
                "layer",
                "Hedef katman",
                ParamKind::Layer {
                    new_layer_style: NewLayerStyle {
                        color: Some("fg-dim".into()),
                        ..NewLayerStyle::default()
                    },
                },
            )
            .default_value(json!({ "newName": "Kenar ölçüleri" }))
            .describe("Bu adda katman yoksa oluşturulur."),
        ],
        outputs: vec![
            OutputDef::new("labels", "Kenar yazıları", OutputKind::Features),
            OutputDef::new("count", "Yazı sayısı", OutputKind::Number),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: Some(|v| {
            let text = |name: &str| v.get(name).and_then(Value::as_str).unwrap_or("");
            let places = decimals(v.get("decimals").and_then(Value::as_f64).unwrap_or(0.0));
            Some(format!(
                "{}{}{}",
                text("prefix"),
                to_fixed(12.3456, places),
                text("suffix")
            ))
        }),
        run: Some(run),
    }
}

fn run(v: &Resolved<'_>, ctx: &RunContext<'_>, _feedback: &mut dyn Feedback) -> RunResult {
    let height = (v.number("textHeight").unwrap_or(2.0) / 1000.0) * ctx.units.plot_scale;
    let input = &v.features("input").entities;
    let ids: Vec<Slot> = input.iter().map(|e| Slot(e.base().id)).collect();
    // Lines, polylines and polygons have edges; one key per edge whichever
    // way it runs (1 mm grid: parcels share exact corners).
    let (labels, skipped) = ctx.geometry.edge_lengths(
        &ids,
        height,
        v.number("minLength").unwrap_or(0.0),
        v.text("side") == "inside",
        v.flag("shared"),
    );
    let places = decimals(v.number("decimals").unwrap_or(0.0));
    let layer = v.layer("layer");
    let add: Vec<Entity> = labels
        .iter()
        .map(|l| {
            Entity::Text(TextEntity {
                base: EntityBase {
                    id: 0,
                    layer_id: layer.id.clone(),
                    color: None,
                    attrs: BTreeMap::from([
                        ("Tür".to_owned(), "Kenar ölçüsü".to_owned()),
                        ("Uzunluk (m)".to_owned(), to_fixed(l.length, 3)),
                    ]),
                    label: None,
                    symbol: None,
                    line_weight: None,
                },
                p: l.p,
                text: format!(
                    "{}{}{}",
                    v.text("prefix"),
                    to_fixed(l.length, places),
                    v.text("suffix")
                ),
                height,
                rotation: l.rotation,
                align: None,
                width_factor: None,
                mask: false,
                label_of: None,
                label_scale: None,
            })
        })
        .collect();
    let summary = format!(
        "{} nesneye {} kenar uzunluğu yazıldı{}.",
        input.len(),
        add.len(),
        if skipped > 0 {
            format!("; {skipped} ortak kenar bir kez yazıldı")
        } else {
            String::new()
        }
    );
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
