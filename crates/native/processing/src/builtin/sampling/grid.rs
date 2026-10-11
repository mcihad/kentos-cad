//! Ağ oluştur (the web's `grid.ts`; docs/adr/0220 §2): rectangular,
//! hexagonal or triangular cells, or a rectangular grid's lines, over the
//! objects' box, all of them or those that meet the objects.

use std::collections::BTreeMap;

use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::ops::sampling::calls::grid_of;
use kentos_geometry_core::ops::sampling::grid::{Kind, Spec};
use kentos_native_application::geometry::shape;
use serde_json::{Value, json};

use super::super::geometry::{GEO_KINDS, layer_param, new_object};
use super::super::proximity::length_text;
use super::{scopes, written};
use crate::types::{
    EnumOption, Feedback, OutputDef, OutputKind, ParamDef, ParamKind, Resolved, RunContext,
    RunResult, Target, Tool, Values,
};

fn shape_is(v: &Values, keys: &[&str]) -> bool {
    let s = v.get("shape").and_then(Value::as_str).unwrap_or("rect");
    keys.contains(&s)
}

fn spaced(v: &Values) -> bool {
    shape_is(v, &["rect", "lines"])
}

fn sided(v: &Values) -> bool {
    shape_is(v, &["hexagon", "triangle"])
}

fn cells(v: &Values) -> bool {
    !shape_is(v, &["lines"])
}

fn metres(name: &str, label: &str, fallback: f64, description: &str) -> ParamDef {
    ParamDef::new(
        name,
        label,
        ParamKind::Number {
            min: Some(0.001),
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
    Tool {
        id: "sample.grid".into(),
        label: "Ağ oluştur".into(),
        category: "sampling".into(),
        description: "Nesnelerin kutusunu dikdörtgen, altıgen ya da üçgen hücrelerle ya da ağın çizgileriyle kaplayıp yeni katmana yazar.".into(),
        help: Some([
            "Örnekler: bir mahalleyi sayım için altıgen hücrelere bölmek, pafta için karelaj çizgileri çizmek, örnekleme çerçevesi için balık ağı kurmak.",
            "Koordinatlara otur açıkken ağ koordinat başlangıcına bağlı genel kafese oturur: dikdörtgenler aralığın katlarında başlar, ayrı üretilen ağlar birbirine uyar. Kapalıyken ağ kapsamın sol alt köşesinden başlar. Komşu hücreler köşelerini paylaşır.",
            "Hücreler Satır ve Sütun alır (satırlar yukarıdan, sütunlar soldan); yatay çizgiler Satır ve Kuzey, düşey çizgiler Sütun ve Doğu. Nesnelere değenler seçiliyse yalnız kapsamın nesnelerine değen ya da onları kesen hücreler yazılır (Konuma göre seç'in Kesişen'i). En çok 1 000 000 parça yazılır.",
        ]
        .join("\n\n")),
        keywords: [
            "ağ", "balık ağı", "karelaj", "altıgen", "üçgen", "hücre", "grid", "fishnet",
            "tessellation", "hexagon",
        ]
        .map(String::from)
        .to_vec(),
        aliases: ["AG", "AGOLUSTUR", "FISHNET"].map(String::from).to_vec(),
        icon: Some("fishnet".into()),
        parameters: vec![
            ParamDef::new(
                "input",
                "Kapsam",
                ParamKind::Features {
                    kinds: Some(GEO_KINDS.iter().map(|k| (*k).to_owned()).collect()),
                    scopes: scopes(),
                    writes: false,
                },
            )
            .describe("Ağın kaplayacağı nesneler; ağ onların kutusunu kaplar."),
            ParamDef::new(
                "shape",
                "Biçim",
                ParamKind::Choice {
                    options: vec![
                        EnumOption::new("rect", "Dikdörtgen").hint("Doğu ve kuzey aralıklı hücreler"),
                        EnumOption::new("hexagon", "Altıgen").hint("Düz üstlü düzgün altıgenler"),
                        EnumOption::new("triangle", "Üçgen").hint("Eşkenar üçgenler"),
                        EnumOption::new("lines", "Çizgi").hint("Dikdörtgen ağın çizgileri"),
                    ],
                },
            )
            .default_value(json!("rect")),
            metres("dx", "Doğu aralığı", 100.0, "Hücrenin doğu–batı boyu.").shown_when(spaced),
            metres("dy", "Kuzey aralığı", 100.0, "Hücrenin kuzey–güney boyu.").shown_when(spaced),
            metres("side", "Kenar", 100.0, "Altıgenin ya da üçgenin kenarı.").shown_when(sided),
            ParamDef::new("snap", "Koordinatlara otur", ParamKind::Boolean)
                .default_value(json!(true))
                .describe("Ağ aralığın katlarına oturur; kapalıyken kapsamın sol alt köşesinden başlar."),
            ParamDef::new(
                "keep",
                "Hücreler",
                ParamKind::Choice {
                    options: vec![
                        EnumOption::new("all", "Kutunun hepsi").hint("Kapsamın kutusunu kaplayan bütün hücreler"),
                        EnumOption::new("meeting", "Nesnelere değenler")
                            .hint("Kapsamın nesnelerine değen ya da onları kesen hücreler"),
                    ],
                },
            )
            .default_value(json!("all"))
            .shown_when(cells),
            layer_param("Ağ", "#8E4EC6"),
        ],
        outputs: vec![
            OutputDef::new("cells", "Yazılan parçalar", OutputKind::Features),
            OutputDef::new("count", "Parça sayısı", OutputKind::Number),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run),
    }
}

fn run(v: &Resolved<'_>, ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let inputs = &v.features("input").entities;
    let Some(kind) = Kind::from_key(v.text("shape")) else {
        return RunResult::refused(format!("Bilinmeyen ağ biçimi: {}", v.text("shape")));
    };
    let number = |name: &str| v.number(name).unwrap_or(100.0);
    let spec = Spec {
        kind,
        dx: number("dx"),
        dy: number("dy"),
        side: number("side"),
        snap: v.flag("snap"),
    };
    feedback.progress(0.0, "Ağ kuruluyor");
    let shapes: Vec<Shape> = inputs.iter().map(|e| shape(e)).collect();
    let pieces = match grid_of(&shapes, &spec, v.text("keep") == "meeting") {
        Ok(p) => p,
        Err(why) => return RunResult::refused(why),
    };
    let layer = &v.layer("layer").id;
    let mut add = Vec::with_capacity(pieces.len());
    for p in pieces {
        let mut attrs = BTreeMap::new();
        let shape = if kind == Kind::Lines {
            let (a, b) = (p.pts[0], p.pts[1]);
            if let Some(r) = p.row {
                attrs.insert("Satır".to_owned(), r.to_string());
                attrs.insert("Kuzey".to_owned(), length_text(ctx.units, a.y));
            } else if let Some(c) = p.col {
                attrs.insert("Sütun".to_owned(), c.to_string());
                attrs.insert("Doğu".to_owned(), length_text(ctx.units, a.x));
            }
            Shape::Line { a, b }
        } else {
            if let (Some(r), Some(c)) = (p.row, p.col) {
                attrs.insert("Satır".to_owned(), r.to_string());
                attrs.insert("Sütun".to_owned(), c.to_string());
            }
            Shape::Polygon {
                pts: p.pts,
                bulges: None,
                holes: None,
                parts: None,
            }
        };
        add.extend(new_object(shape, layer, attrs));
    }
    let what = if kind == Kind::Lines {
        "çizgi"
    } else {
        "hücre"
    };
    let summary = format!("{} {what} yazıldı.", add.len());
    written(add, summary)
}
