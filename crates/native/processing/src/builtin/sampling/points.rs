//! Düzenli noktalar and Rastgele noktalar (the web's `points.ts`;
//! docs/adr/0220 §3, §4): a lattice's points in areas, shifted by seeded
//! numbers when asked; seeded random points in areas and on paths, at
//! least so far apart.

use kentos_geometry_core::ops::sampling::random::seed_of;
use kentos_geometry_core::ops::sampling::regular::regular;
use kentos_geometry_core::ops::sampling::scatter::scatter;
use kentos_geometry_core::ops::sampling::{MOST, count_of, inside_of, target_of};
use kentos_native_application::geometry::shape;
use kentos_style_core::expr::Value as ExprValue;
use kentos_style_core::expr::rows::As;
use serde_json::{Value, json};

use super::super::convert::{point, zs_of};
use super::super::geometry::{AREA_KINDS, layer_param};
use super::super::proximity::length_text;
use super::{LINE_KINDS, curved_note, scopes, written};
use crate::expression::evaluate_in;
use crate::types::{
    Feedback, OutputDef, OutputKind, ParamDef, ParamKind, Resolved, Returns, RunContext, RunResult,
    Target, Tool, Values,
};

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

fn whole(
    name: &str,
    label: &str,
    min: f64,
    max: Option<f64>,
    fallback: f64,
    description: &str,
) -> ParamDef {
    ParamDef::new(
        name,
        label,
        ParamKind::Number {
            min: Some(min),
            max,
            integer: true,
            unit: String::new(),
            placeholder: None,
        },
    )
    .default_value(json!(fallback))
    .describe(description)
}

fn shifted(v: &Values) -> bool {
    v.get("shift")
        .and_then(Value::as_f64)
        .is_some_and(|s| s > 0.0)
}

fn apart(v: &Values) -> bool {
    v.get("min")
        .and_then(Value::as_f64)
        .is_some_and(|s| s > 0.0)
}

// ── Düzenli noktalar ─────────────────────────────────────────────────

pub fn regular_tool() -> Tool {
    Tool {
        id: "sample.regular".into(),
        label: "Düzenli noktalar".into(),
        category: "sampling".into(),
        description: "Alanların içine doğu ve kuzey aralıklarıyla düzenli noktalar koyar, isterse noktaları tohumla rastgele kaydırır.".into(),
        help: Some([
            "Örnekler: orman envanterinin örnek alanlarını, toprak örneklemesinin noktalarını, bir paftanın karelaj noktalarını koymak.",
            "Koordinatlara otur açıkken noktalar aralığın katlarındadır (yuvarlak koordinatlar); kapalıyken bütün alanların kutusunun köşesinden yarım aralık içeride, hücrelerin ortalarındadır. Alanın sınırındaki nokta da alınır, deliklerdeki alınmaz; ortak sınırdaki nokta iki alana da yazılır.",
            "Rastgele kaydırma her noktayı her eksende en çok o kadar kaydırır; aynı tohum aynı noktaları verir. Nokta alanın özniteliklerini alır. En çok 1 000 000 nokta yazılır.",
        ]
        .join("\n\n")),
        keywords: [
            "düzenli", "nokta", "örnekleme", "karelaj", "kafes", "sistematik", "regular points",
            "sampling",
        ]
        .map(String::from)
        .to_vec(),
        aliases: ["DUZENLINOKTA", "KARELAJNOKTA"].map(String::from).to_vec(),
        icon: Some("regularPoints".into()),
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
            .describe("Noktaların konacağı alanlar."),
            metres("dx", "Doğu aralığı", 0.001, 10.0, "Noktalar arası doğu–batı uzaklığı."),
            metres("dy", "Kuzey aralığı", 0.001, 10.0, "Noktalar arası kuzey–güney uzaklığı."),
            ParamDef::new("snap", "Koordinatlara otur", ParamKind::Boolean)
                .default_value(json!(true))
                .describe("Noktalar aralığın katlarında; kapalıyken hücrelerin ortalarında."),
            metres("shift", "Rastgele kaydırma", 0.0, 0.0, "Her noktanın her eksende en çok kayacağı uzaklık; 0 kaydırmaz."),
            whole("seed", "Tohum", 0.0, None, 1.0, "Aynı tohum aynı kaydırmaları verir.").shown_when(shifted),
            layer_param("Düzenli noktalar", "#3E63DD"),
        ],
        outputs: vec![
            OutputDef::new("points", "Yazılan noktalar", OutputKind::Features),
            OutputDef::new("count", "Nokta sayısı", OutputKind::Number),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run_regular),
    }
}

fn run_regular(v: &Resolved<'_>, _ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let inputs = &v.features("input").entities;
    feedback.progress(0.0, "Noktalar sınanıyor");
    let objects: Vec<_> = inputs.iter().map(|e| inside_of(&shape(e))).collect();
    let got = regular(
        &objects,
        v.number("dx").unwrap_or(10.0),
        v.number("dy").unwrap_or(10.0),
        v.flag("snap"),
        v.number("shift").unwrap_or(0.0),
        seed_of(v.number("seed").unwrap_or(1.0)),
    );
    let lists = match got {
        Ok(l) => l,
        Err(why) => return RunResult::refused(why.to_owned()),
    };
    curved_note(feedback, inputs);
    let layer = &v.layer("layer").id;
    let mut add = Vec::new();
    let mut empty = 0usize;
    for (e, pts) in inputs.iter().zip(lists) {
        if pts.is_empty() {
            empty += 1;
        }
        for p in pts {
            add.extend(point(p, None, layer, e.base().attrs.clone()));
        }
    }
    if empty > 0 {
        feedback.info(format!("{empty} alana nokta düşmedi."));
    }
    let summary = format!("{} alana {} nokta yazıldı.", inputs.len(), add.len());
    written(add, summary)
}

// ── Rastgele noktalar ────────────────────────────────────────────────

pub fn random_tool() -> Tool {
    let mut kinds: Vec<String> = AREA_KINDS.iter().map(|k| (*k).to_owned()).collect();
    kinds.extend(LINE_KINDS.iter().map(|k| (*k).to_owned()));
    Tool {
        id: "sample.random".into(),
        label: "Rastgele noktalar".into(),
        category: "sampling".into(),
        description: "Alanların içine ya da çizgilerin üstüne nesne başına istenen sayıda rastgele nokta koyar; aynı tohum aynı noktaları verir.".into(),
        help: Some([
            "Örnekler: tabakalı olmayan rastgele örnekleme noktaları, denetim için rastgele parsel noktaları, bir yol üstünde rastgele gözlem yerleri.",
            "Nokta sayısı nesne başınadır ve ifade olabilir: $alan / 10000 * 5 hektarda beş nokta. En az uzaklık yazılan bütün noktalar arasındadır; sağlanamazsa nokta Deneme kadar yeniden aranır, bulunamazsa yazılmaz ve söylenir. Aynı tohum her çalıştırmada ve her platformda aynı noktaları verir.",
            "Alandaki nokta alanın kutusunda rastgele aranır, kutusuna göre çok ince alan söylenir. Çizgideki nokta yolun uzunluğu boyunca rastgeledir; Uzaklık (yolun başından), çok parçalıda Parça ve köşelerin kotlarından kotunu alır. Nokta nesnenin özniteliklerini alır.",
        ]
        .join("\n\n")),
        keywords: [
            "rastgele", "nokta", "örnekleme", "tohum", "random points", "sampling", "stokastik",
        ]
        .map(String::from)
        .to_vec(),
        aliases: ["RASTGELENOKTA", "RANDOMPOINTS"].map(String::from).to_vec(),
        icon: Some("randomPoints".into()),
        parameters: vec![
            ParamDef::new(
                "input",
                "Nesneler",
                ParamKind::Features {
                    kinds: Some(kinds),
                    scopes: scopes(),
                    writes: false,
                },
            )
            .describe("Noktaların içine konacağı alanlar ya da üstüne konacağı çizgiler."),
            ParamDef::new(
                "count",
                "Nokta sayısı",
                ParamKind::Expression {
                    returns: Returns::Value,
                    of: Some("input".into()),
                    placeholder: Some("10".into()),
                },
            )
            .default_value(json!("10"))
            .describe("Nesne başına nokta sayısı (tam kısmı)."),
            metres("min", "En az uzaklık", 0.0, 0.0, "Yazılan iki nokta arası en az uzaklık; 0 sınırsız."),
            whole("tries", "Deneme", 1.0, Some(1000.0), 10.0, "En az uzaklık sağlanamazsa bir nokta için en çok aday.").shown_when(apart),
            whole("seed", "Tohum", 0.0, None, 1.0, "Aynı tohum aynı noktaları verir."),
            layer_param("Rastgele noktalar", "#E54666"),
        ],
        outputs: vec![
            OutputDef::new("points", "Yazılan noktalar", OutputKind::Features),
            OutputDef::new("count", "Nokta sayısı", OutputKind::Number),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run_random),
    }
}

fn run_random(v: &Resolved<'_>, ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let inputs = &v.features("input").entities;
    feedback.progress(0.0, "Noktalar aranıyor");
    let layer_name = |id: &str| ctx.layer_name(id).to_owned();
    let how = ctx.evaluation(&layer_name);
    let values = v
        .expr("count")
        .map(|e| evaluate_in(e, inputs, &how, As::Number))
        .unwrap_or_default();
    let counts: Vec<Option<u64>> = (0..inputs.len())
        .map(|i| match values.get(i) {
            Some(ExprValue::Num(x)) => count_of(Some(*x)),
            _ => None,
        })
        .collect();
    let unread = counts.iter().filter(|c| c.is_none()).count();
    let n: Vec<u64> = counts.iter().map(|c| c.unwrap_or(0)).collect();
    if n.iter().map(|&c| c as f64).sum::<f64>() > MOST as f64 {
        return RunResult::refused(
            "1 000 000'dan çok nokta yazılırdı; sayıyı ya da aralığı değiştirin.".to_owned(),
        );
    }
    let targets: Vec<_> = inputs
        .iter()
        .map(|e| target_of(&shape(e), zs_of(e).as_ref()))
        .collect();
    let tries = v.number("tries").unwrap_or(10.0);
    let tries = if tries >= 1.0 {
        tries.min(1000.0) as u32
    } else {
        1
    };
    let got = scatter(
        &targets,
        &n,
        v.number("min").unwrap_or(0.0),
        tries,
        seed_of(v.number("seed").unwrap_or(1.0)),
    );
    if unread > 0 {
        feedback.warn(format!("{unread} nesnede nokta sayısı okunamadı; atlandı."));
    }
    if got.refused > 0 {
        feedback.warn(format!(
            "{} nokta en az uzaklık sağlanamadığı için yazılmadı.",
            got.refused
        ));
    }
    if got.thin > 0 {
        feedback.warn(format!(
            "{} alan kutusuna göre çok ince: {} noktası 10 000 denemede bulunamadı.",
            got.thin, got.thin_lost
        ));
    }
    curved_note(feedback, inputs);
    let layer = &v.layer("layer").id;
    let mut add = Vec::new();
    for ((e, dots), target) in inputs.iter().zip(&got.dots).zip(&targets) {
        let many = matches!(target, kentos_geometry_core::ops::sampling::scatter::Target::Paths(w) if w.len() > 1);
        for d in dots {
            let mut attrs = e.base().attrs.clone();
            if let Some(s) = d.s {
                attrs.insert("Uzaklık".to_owned(), length_text(ctx.units, s));
                if many {
                    attrs.insert("Parça".to_owned(), (d.path + 1).to_string());
                }
            }
            add.extend(point(d.p, d.z, layer, attrs));
        }
    }
    let summary = format!("{} nesneye {} nokta yazıldı.", inputs.len(), add.len());
    written(add, summary)
}
