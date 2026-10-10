//! Köşe noktalarını numarala (the web's `builtin/vertexNumbering.ts`):
//! every corner of the chosen areas (and paths) gets a point named in a
//! fixed format — "P00001", "P00002", … — walked clockwise or
//! counter-clockwise from a chosen start corner. Corners shared by
//! neighbouring parcels get one number; points already on the target layer
//! keep theirs and numbering continues after them.

use std::collections::{BTreeMap, HashSet};

use kentos_contracts::{
    Entity, EntityBase, LabelPlacement, LabelStyle, PointEntity, PointStyle, PointSymbol,
    TextEntity,
};
use kentos_domain::Slot;
use kentos_geometry_core::processing::numbering::{CornerWalk, StartCorner};
use serde_json::{Value, json};

use super::numbering::{Named, NumberFormat, format_number, name_corners, numbered_points};
use crate::geometry::core;
use crate::types::{
    EnumOption, Feedback, NewLayerStyle, OutputDef, OutputKind, ParamDef, ParamKind, Resolved,
    RunContext, RunResult, ScopeKind, Target, Tool, Values,
};

fn number(v: &Values, name: &str) -> f64 {
    v.get(name).and_then(Value::as_f64).unwrap_or(0.0)
}

fn text<'a>(v: &'a Values, name: &str) -> &'a str {
    v.get(name).and_then(Value::as_str).unwrap_or("")
}

fn format_of(v: &Values) -> NumberFormat {
    NumberFormat {
        prefix: text(v, "prefix").into(),
        length: number(v, "length"),
        pad: text(v, "pad").into(),
    }
}

pub fn tool() -> Tool {
    let features = |name: &str, label: &str| {
        ParamDef::new(
            name,
            label,
            ParamKind::Features {
                kinds: Some(vec!["polygon".into(), "polyline".into()]),
                scopes: None::<Vec<ScopeKind>>,
                writes: false,
            },
        )
    };
    let number_param =
        |name: &str, label: &str, min: Option<f64>, max: Option<f64>, integer: bool, unit: &str| {
            ParamDef::new(
                name,
                label,
                ParamKind::Number {
                    min,
                    max,
                    integer,
                    unit: unit.into(),
                    placeholder: None,
                },
            )
        };
    let text_param = |name: &str, label: &str, max_length: usize| {
        ParamDef::new(
            name,
            label,
            ParamKind::Text {
                placeholder: None,
                max_length: Some(max_length),
                allow_empty: true,
            },
        )
    };
    Tool {
        id: "points.numberVertices".into(),
        label: "Köşe noktalarını numarala".into(),
        category: "points".into(),
        description: "Alanların köşelerine belirlediğiniz biçimde numaralı nokta koyar (P00001, P00002 …); yön ve başlangıç köşesi seçilir.".into(),
        help: Some([
            "Her alanın köşeleri seçilen yönde, seçilen başlangıç köşesinden itibaren numaralanır; delikli alanlarda önce dış halka, sonra delikler.",
            "Birden çok alan varsa sıra başlangıç köşelerine göredir: kuzeybatıdakiler önce. “Seçilen noktaya en yakın” başlangıçta noktaya yakın olan alan önce gelir.",
            "Komşu parsellerin ortak köşesi tek numara alır. Hedef katmanda aynı biçimde numaralanmış noktalar varsa onların adı korunur ve numara kaldığı yerden devam eder.",
        ]
        .join("\n\n")),
        keywords: ["numara", "köşe", "nokta", "isim", "vertex", "number", "label", "parsel"]
            .map(String::from)
            .to_vec(),
        aliases: ["KOSENUMARA", "KNUM", "NUMARALA"].map(String::from).to_vec(),
        icon: Some("numberVertices".into()),
        parameters: vec![
            features("input", "Alanlar")
                .describe("Köşeleri numaralanacak kapalı alanlar ve çoklu çizgiler.")
                .default_value(json!({ "scope": "selection" })),
            ParamDef::new(
                "direction",
                "Yön",
                ParamKind::Choice {
                    options: vec![
                        EnumOption::new("cw", "Saat yönünde"),
                        EnumOption::new("ccw", "Saat yönünün tersine"),
                    ],
                },
            )
            .default_value(json!("cw")),
            ParamDef::new(
                "start",
                "Başlangıç köşesi",
                ParamKind::Choice {
                    options: vec![
                        EnumOption::new("northwest", "Kuzeybatı").hint("Kuzeybatı yönüne en yakın köşe"),
                        EnumOption::new("north", "En kuzey").hint("Kuzeyi en büyük köşe"),
                        EnumOption::new("first", "İlk çizilen").hint("Nesnenin ilk köşesi"),
                        EnumOption::new("point", "Seçilen noktaya en yakın")
                            .hint("Haritada gösterdiğiniz noktaya en yakın köşe"),
                    ],
                },
            )
            .describe("Numaralamanın başlayacağı köşe.")
            .default_value(json!("northwest"))
            .picks_point("point", "startPoint"),
            ParamDef::new("startPoint", "Başlangıç noktası", ParamKind::Point)
                .describe("Her alanda bu noktaya en yakın köşeden başlanır.")
                .shown_when(|v| v.get("start").and_then(Value::as_str) == Some("point")),
            text_param("prefix", "Önek", 12)
                .default_value(json!("P"))
                .describe("Numaranın başındaki yazı."),
            number_param("length", "Toplam uzunluk", Some(1.0), Some(30.0), true, "")
                .default_value(json!(6))
                .describe("Önek dahil karakter sayısı: P00001 için 6."),
            text_param("pad", "Doldurma karakteri", 1)
                .default_value(json!("0"))
                .describe("Önek ile rakamlar arasını doldurur; boş bırakılırsa doldurulmaz."),
            number_param("first", "İlk numara", Some(0.0), None, true, "").default_value(json!(1)),
            number_param("step", "Artış", Some(1.0), None, true, "")
                .default_value(json!(1))
                .advanced(),
            ParamDef::new("shared", "Ortak köşelere tek numara", ParamKind::Boolean)
                .default_value(json!(true))
                .describe("Komşu alanların aynı yerdeki köşeleri tek nokta olur."),
            number_param("tolerance", "Aynı nokta toleransı", Some(0.0), Some(1.0), false, "m")
                .default_value(json!(0.001))
                .advanced()
                .shown_when(|v| v.get("shared").and_then(Value::as_bool) == Some(true)),
            ParamDef::new(
                "output",
                "Oluşturulacak",
                ParamKind::Choice {
                    options: vec![
                        EnumOption::new("points", "Nokta").hint("Adı etiket olarak görünen nokta nesnesi"),
                        EnumOption::new("text", "Yazı").hint("Köşenin dışına yazı"),
                        EnumOption::new("both", "Nokta ve yazı"),
                    ],
                },
            )
            .default_value(json!("points")),
            // Empty: the project's Kenar ve köşe yazıları height (docs/adr/0205 §2).
            number_param("textHeight", "Yazı yüksekliği", Some(0.1), Some(50.0), false, "mm")
                .optional()
                .default_value(Value::Null)
                .placeholder("Proje")
                .describe("Kâğıt üzerinde, çizim ölçeğine göre; boş bırakılırsa projenin kenar ve köşe yazıları yüksekliği.")
                .shown_when(|v| v.get("output").and_then(Value::as_str) != Some("points")),
            ParamDef::new(
                "layer",
                "Hedef katman",
                ParamKind::Layer {
                    new_layer_style: NewLayerStyle {
                        color: Some("#E5C07B".into()),
                        point: Some(PointStyle {
                            symbol: PointSymbol::Cross,
                            size: 7.0,
                        }),
                        label: Some(Box::new(LabelStyle {
                            placement: LabelPlacement::Beside,
                            size: 10.5,
                            grow: None,
                            max_size: None,
                            weight: None,
                            template: None,
                            min_feature_px: None,
                            min_scale: None,
                            max_scale: None,
                            ink: None,
                            ..LabelStyle::default()
                        })),
                        ..NewLayerStyle::default()
                    },
                    above: None,
                    below: None,
                },
            )
            .default_value(json!({ "newName": "Köşe noktaları" }))
            .describe("Bu adda katman yoksa oluşturulur."),
        ],
        outputs: vec![
            OutputDef::new("points", "Numaralı noktalar", OutputKind::Features),
            OutputDef::new("count", "Yeni numara sayısı", OutputKind::Number),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: Some(|v| {
            let prefix = text(v, "prefix");
            (!text(v, "pad").is_empty()
                && crate::text::utf16_len(prefix) as f64 >= number(v, "length"))
            .then(|| "Toplam uzunluk önekten uzun olmalı; yoksa doldurma yapılamaz.".into())
        }),
        preview: Some(|v| {
            let f = format_of(v);
            let (first, step) = (number(v, "first"), number(v, "step"));
            Some(format!(
                "{}, {}, {} …",
                format_number(first, &f),
                format_number(first + step, &f),
                format_number(first + 2.0 * step, &f)
            ))
        }),
        run: Some(run),
    }
}

fn base(layer: &str, label: Option<String>, attrs: &BTreeMap<String, String>) -> EntityBase {
    EntityBase {
        id: 0,
        layer_id: layer.into(),
        color: None,
        attrs: attrs.clone(),
        label,
        symbol: None,
        line_weight: None,
        label_pins: Vec::new(),
    }
}

fn run(v: &Resolved<'_>, ctx: &RunContext<'_>, _feedback: &mut dyn Feedback) -> RunResult {
    let format = format_of(&v.values);
    // Areas and paths have corners; the core walks their rings (a polygon's holes after its outer ring).
    let shapes: Vec<&Entity> = v
        .features("input")
        .entities
        .iter()
        .copied()
        .filter(|e| matches!(e.kind(), "polygon" | "polyline"))
        .collect();
    if shapes.is_empty() {
        return RunResult {
            summary: Some("Numaralanacak alan yok.".into()),
            ..RunResult::default()
        };
    }
    let layer = v.layer("layer");
    let shared = v.flag("shared");
    let existing: Vec<Named> = if shared || !layer.is_new {
        ctx.doc
            .by_layer(&layer.id)
            .filter_map(|e| match e {
                Entity::Point(pt) => Some(Named {
                    p: pt.p,
                    name: pt
                        .base
                        .label
                        .clone()
                        .or_else(|| pt.base.attrs.get("Nokta").cloned())
                        .unwrap_or_default(),
                }),
                _ => None,
            })
            .collect()
    } else {
        Vec::new()
    };
    let named = numbered_points(&existing);
    let walk = CornerWalk {
        ccw: v.text("direction") == "ccw",
        start: StartCorner::parse(v.text("start")).unwrap_or(StartCorner::Northwest),
        point: v.point("startPoint").map(core),
        tolerance: v.number("tolerance").unwrap_or(0.001),
        shared,
    };
    let ids: Vec<Slot> = shapes.iter().map(|e| Slot(e.base().id)).collect();
    let existing_points: Vec<_> = if shared {
        named.iter().map(|n| n.p).collect()
    } else {
        Vec::new()
    };
    let found = ctx.geometry.number_corners(&ids, &walk, &existing_points);
    let corners = name_corners(
        &found,
        &named,
        &format,
        v.number("first").unwrap_or(1.0),
        v.number("step").unwrap_or(1.0),
    );
    let created: Vec<_> = corners.iter().filter(|c| c.created).collect();
    let mm = v
        .number("textHeight")
        .unwrap_or(ctx.units.measure_height_mm);
    let height = (mm / 1000.0) * ctx.units.plot_scale;
    let output = v.text("output");
    // Outside the corner, centred on its bisector (placed by the core from
    // the name's width in the drawing's typeface).
    let texts = if output != "points" {
        let at: Vec<_> = created.iter().map(|c| (c.p, c.out)).collect();
        let names: Vec<String> = created.iter().map(|c| c.name.clone()).collect();
        ctx.geometry
            .corner_texts(&at, &names, height, ctx.units.drawing_font)
    } else {
        Vec::new()
    };
    let mut add = Vec::new();
    for (i, c) in created.iter().enumerate() {
        let attrs = BTreeMap::from([
            ("Nokta".to_owned(), c.name.clone()),
            ("Tür".to_owned(), "Köşe noktası".to_owned()),
        ]);
        if output != "text" {
            add.push(Entity::Point(PointEntity {
                base: base(&layer.id, Some(c.name.clone()), &attrs),
                p: c.p,
                z: None,
                parts: None,
            }));
        }
        if output != "points"
            && let Some(p) = texts.get(i)
        {
            add.push(Entity::Text(TextEntity {
                base: base(&layer.id, None, &attrs),
                p: *p,
                text: c.name.clone(),
                height,
                rotation: 0.0,
                align: None,
                width_factor: None,
                mask: false,
                label_of: None,
                label_scale: None,
                paragraph: Default::default(),
                face: Default::default(),
                path: None,
            }));
        }
    }
    // Reused numbers: from points already on the layer, or from a neighbour numbered in this run.
    let before: HashSet<&str> = existing.iter().map(|e| e.name.as_str()).collect();
    let reused = |kept: bool| {
        corners
            .iter()
            .filter(|c| !c.created && before.contains(c.name.as_str()) == kept)
            .map(|c| c.name.as_str())
            .collect::<HashSet<_>>()
            .len()
    };
    let (kept, shared_count) = (reused(true), reused(false));
    let mut notes = Vec::new();
    if shared_count > 0 {
        notes.push(format!(
            "{shared_count} ortak köşe komşularıyla tek numara aldı"
        ));
    }
    if kept > 0 {
        notes.push(format!("{kept} köşe mevcut numarasını korudu"));
    }
    let summary = match (created.first(), created.last()) {
        (Some(first), Some(last)) => format!(
            "{} nesnede {} köşe numaralandı: {} – {}{}.",
            shapes.len(),
            created.len(),
            first.name,
            last.name,
            if notes.is_empty() {
                String::new()
            } else {
                format!("; {}", notes.join("; "))
            }
        ),
        _ => "Yeni numara gerekmedi: bütün köşelerin numarası zaten var.".into(),
    };
    let count = created.len();
    RunResult {
        changes: Some(crate::types::ChangeSet {
            add,
            ..Default::default()
        }),
        outputs: [("count".to_owned(), json!(count))].into_iter().collect(),
        summary: Some(summary),
        ..RunResult::default()
    }
}
