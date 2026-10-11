//! Çizginin parçası, Alanları çizgiye çevir, Çizgileri alana çevir and Köşe
//! ekle (the web's `paths.ts`; docs/adr/0218 §8–§11): the shapes are the
//! core's (`ops::convert::paths`), the elevations carried by docs/adr/0142's rules.

use kentos_contracts::Entity;
use kentos_geometry_core::ops::convert::paths::{
    Density, NoArea, densify_added, line_area, rings_as_lines, substring,
};
use kentos_native_application::geometry::shape;
use kentos_style_core::expr::Value as ExprValue;
use kentos_style_core::expr::rows::As;
use serde_json::json;

use super::super::geometry::layer_param;
use super::{placed, scopes, zs_of};
use crate::expression::evaluate_in;
use crate::types::{
    ChangeSet, EnumOption, Feedback, OutputDef, OutputKind, ParamDef, ParamKind, Resolved, Returns,
    RunContext, RunResult, Target, Tool,
};

fn features(kinds: &[&str], label: &str, description: &str) -> ParamDef {
    ParamDef::new(
        "input",
        label,
        ParamKind::Features {
            kinds: Some(kinds.iter().map(|k| (*k).to_owned()).collect()),
            scopes: scopes(),
            writes: false,
        },
    )
    .describe(description)
}

fn done(add: Vec<Entity>, count: usize, summary: String) -> RunResult {
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

// ── Çizginin parçası ─────────────────────────────────────────────────

pub fn substring_tool() -> Tool {
    Tool {
        id: "convert.substring".into(),
        label: "Çizginin parçası".into(),
        category: "conversion".into(),
        description: "Her çizginin başından verilen iki uzaklık arasındaki parçasını yeni katmana yazar.".into(),
        help: Some([
            "Örnekler: bir yolun iki km arası kesimini, bir hattın ilk 100 metresini, her çizginin uçlarından 5 metre kısaltılmışını almak.",
            "Uzaklıklar yolun başından yol boyunca metredir (yaylar kesin); ifade olduklarından her nesnede başka olabilir: $uzunluk / 2 yolun yarısı. Uzaklıklar 0 ile uzunluk arasına alınır.",
            "Doğru parçaları doğru, yaylar yay kalır; düz bir kenarda kalan parça doğrudur. Başlangıcı bitişinden küçük olmayan, uzaklığı sayı olmayan ve çok parçalı nesne yazılmaz, söylenir. Köşelerin kotları kenarları boyunca taşınır.",
        ]
        .join("\n\n")),
        keywords: [
            "parça", "kesim", "alt çizgi", "km", "uzaklık", "line substring", "segment",
        ]
        .map(String::from)
        .to_vec(),
        aliases: vec!["CIZGIPARCASI".into()],
        icon: Some("lineSubstring".into()),
        parameters: vec![
            features(
                &["line", "polyline", "arc"],
                "Nesneler",
                "Parçası alınacak çizgiler ve yaylar.",
            ),
            ParamDef::new(
                "start",
                "Başlangıç",
                ParamKind::Expression {
                    returns: Returns::Value,
                    of: Some("input".into()),
                    placeholder: Some("0".into()),
                },
            )
            .default_value(json!("0"))
            .describe("Yolun başından uzaklık (m)."),
            ParamDef::new(
                "end",
                "Bitiş",
                ParamKind::Expression {
                    returns: Returns::Value,
                    of: Some("input".into()),
                    placeholder: Some("$uzunluk".into()),
                },
            )
            .default_value(json!("$uzunluk"))
            .describe("Yolun başından uzaklık (m)."),
            layer_param("Parçalar", "#3E63DD"),
        ],
        outputs: vec![
            OutputDef::new("pieces", "Yazılan parçalar", OutputKind::Features),
            OutputDef::new("count", "Parça sayısı", OutputKind::Number),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run_substring),
    }
}

fn numbers(values: &[ExprValue], n: usize) -> Vec<Option<f64>> {
    (0..n)
        .map(|i| match values.get(i) {
            Some(ExprValue::Num(x)) if x.is_finite() => Some(*x),
            _ => None,
        })
        .collect()
}

fn run_substring(v: &Resolved<'_>, ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let inputs = &v.features("input").entities;
    feedback.progress(0.0, "Uzaklıklar hesaplanıyor");
    let layer_name = |id: &str| ctx.layer_name(id).to_owned();
    let how = ctx.evaluation(&layer_name);
    let column = |name: &str| {
        v.expr(name)
            .map(|e| numbers(&evaluate_in(e, inputs, &how, As::Number), inputs.len()))
            .unwrap_or_else(|| vec![None; inputs.len()])
    };
    let (starts, ends) = (column("start"), column("end"));
    let layer = &v.layer("layer").id;
    let (mut add, mut multi, mut unread, mut empty) = (Vec::new(), 0, 0, 0);
    for (i, e) in inputs.iter().enumerate() {
        let piece = match (starts[i], ends[i]) {
            (Some(s0), Some(s1)) => substring(&shape(e), zs_of(e).as_ref(), s0, s1),
            _ => None,
        };
        match piece {
            Some(r) => add.extend(placed(r, layer, e.base().attrs.clone())),
            None if matches!(e, Entity::Polyline(p) if p.parts.as_ref().is_some_and(|p| !p.is_empty())) =>
            {
                multi += 1;
            }
            None if starts[i].is_none() || ends[i].is_none() => unread += 1,
            None => empty += 1,
        }
    }
    if multi > 0 {
        feedback.warn(format!("{multi} çok parçalı çizgi alınmadı."));
    }
    if unread > 0 {
        feedback.warn(format!("{unread} nesnede uzaklık sayı değil; yazılmadı."));
    }
    if empty > 0 {
        feedback.warn(format!(
            "{empty} nesnede başlangıç bitişten küçük değil; yazılmadı."
        ));
    }
    let count = add.len();
    done(
        add,
        count,
        format!("{} nesneden {count} parça yazıldı.", inputs.len()),
    )
}

// ── Alanları çizgiye çevir ───────────────────────────────────────────

pub fn polygons_to_lines() -> Tool {
    Tool {
        id: "convert.polygonsToLines".into(),
        label: "Alanları çizgiye çevir".into(),
        category: "conversion".into(),
        description: "Alanların halkalarını (dış halka, delikler, parçalar) öznitelikleriyle yeni katmana çizgi olarak yazar.".into(),
        help: Some([
            "Örnekler: parsel sınırlarını çizgi katmanına almak, adaların ve yapıların dış çizgilerini ayrı çizmek.",
            "Her halka son köşesi ilki olan bir çoklu çizgidir; yaylar ve kotlar kalır. Çizgi alanın özniteliklerine ek Parça (1 …) ve Halka (0 dış halka, 1 … delikler) alır.",
        ]
        .join("\n\n")),
        keywords: ["alan", "çizgi", "sınır", "halka", "polygons to lines", "boundary"]
            .map(String::from)
            .to_vec(),
        aliases: vec!["SINIRCIZGI".into()],
        icon: Some("polygonToLine".into()),
        parameters: vec![
            features(&["polygon"], "Alanlar", "Sınırları çizgiye çevrilecek alanlar."),
            layer_param("Sınırlar", "#AD7F58"),
        ],
        outputs: vec![
            OutputDef::new("lines", "Yazılan çizgiler", OutputKind::Features),
            OutputDef::new("count", "Çizgi sayısı", OutputKind::Number),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run_rings),
    }
}

fn run_rings(v: &Resolved<'_>, _ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let inputs = &v.features("input").entities;
    feedback.progress(0.0, "Halkalar okunuyor");
    let layer = &v.layer("layer").id;
    let mut add = Vec::new();
    for e in inputs {
        for r in rings_as_lines(&shape(e), zs_of(e).as_ref()) {
            let mut attrs = e.base().attrs.clone();
            attrs.insert("Parça".into(), r.part.to_string());
            attrs.insert("Halka".into(), r.ring.to_string());
            add.extend(placed(r.line, layer, attrs));
        }
    }
    let count = add.len();
    done(
        add,
        count,
        format!("{} alandan {count} çizgi yazıldı.", inputs.len()),
    )
}

// ── Çizgileri alana çevir ────────────────────────────────────────────

pub fn lines_to_polygons() -> Tool {
    Tool {
        id: "convert.linesToPolygons".into(),
        label: "Çizgileri alana çevir".into(),
        category: "conversion".into(),
        description: "Kapalı çoklu çizgileri (isterse açıkları kapatarak) öznitelikleriyle yeni katmana alan olarak yazar.".into(),
        help: Some([
            "Örnekler: sayısallaştırılmış kapalı sınırları alana çevirmek, dosyadan çizgi olarak gelen parselleri alan yapmak.",
            "Uçları en çok 1 µm aralıklı çizgi kapalıdır (çizimin Alana çevir’i); yinelenen son köşe düşer, yaylar ve kotlar kalır. Kapat seçiliyse açık çizginin son köşesi ilkine düz kenarla bağlanır.",
            "Alanı olmayan ve çok parçalı çizgi alan vermez, Atla seçiliyse açık çizgi de yazılmaz; hepsi söylenir. Birbirini kesen çizgilerden alan kurmak Toplu alan’ın işidir.",
        ]
        .join("\n\n")),
        keywords: ["çizgi", "alan", "kapat", "lines to polygons", "polygonize"]
            .map(String::from)
            .to_vec(),
        aliases: vec!["CIZGIALAN".into()],
        icon: Some("lineToPolygon".into()),
        parameters: vec![
            features(&["polyline"], "Çizgiler", "Alana çevrilecek çoklu çizgiler."),
            ParamDef::new(
                "open",
                "Açık çizgiler",
                ParamKind::Choice {
                    options: vec![
                        EnumOption::new("skip", "Atla").hint("Yalnız kapalı çizgiler alan olur"),
                        EnumOption::new("close", "Kapat").hint("Son köşe ilkine düz kenarla bağlanır"),
                    ],
                },
            )
            .default_value(json!("skip")),
            layer_param("Alanlar", "#8E4EC6"),
        ],
        outputs: vec![
            OutputDef::new("areas", "Yazılan alanlar", OutputKind::Features),
            OutputDef::new("count", "Alan sayısı", OutputKind::Number),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run_areas),
    }
}

fn run_areas(v: &Resolved<'_>, _ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let inputs = &v.features("input").entities;
    feedback.progress(0.0, "Çizgiler kapatılıyor");
    let close = v.text("open") == "close";
    let layer = &v.layer("layer").id;
    let (mut add, mut open, mut empty, mut multi) = (Vec::new(), 0, 0, 0);
    for e in inputs {
        match line_area(&shape(e), zs_of(e).as_ref(), close) {
            Ok(r) => add.extend(placed(r, layer, e.base().attrs.clone())),
            Err(NoArea::Open) => open += 1,
            Err(NoArea::Empty) => empty += 1,
            Err(NoArea::MultiPart) => multi += 1,
        }
    }
    if open > 0 {
        feedback.warn(format!("{open} açık çizgi atlandı."));
    }
    if empty > 0 {
        feedback.warn(format!("{empty} çizginin alanı yok; yazılmadı."));
    }
    if multi > 0 {
        feedback.warn(format!("{multi} çok parçalı çizgi alınmadı."));
    }
    let count = add.len();
    done(
        add,
        count,
        format!("{} çizgiden {count} alan yazıldı.", inputs.len()),
    )
}

// ── Köşe ekle ────────────────────────────────────────────────────────

/// The most vertices Köşe ekle adds in a run: past them it is refused before anything is built.
const MOST_ADDED: u64 = 10_000_000;

pub fn densify_tool() -> Tool {
    Tool {
        id: "convert.densify".into(),
        label: "Köşe ekle".into(),
        category: "conversion".into(),
        description: "Çizgilerin ve alanların her kenarını eşit parçalara bölerek köşe ekler; sonucu yeni katmana yazar.".into(),
        help: Some([
            "Örnekler: yeniden izdüşümden önce uzun kenarları sıklaştırmak, bir hattı 1 metrede bir köşeli yapmak, her kenarın ortasına köşe koymak.",
            "Aralıkla her kenar, parçaları aralıktan uzun olmayan en az sayıda eşit parçaya bölünür; Sayıyla her kenara o kadar köşe eklenir. Yaylı kenar aynı yayın parçalarına bölünür: yay kalır, yeni köşeler yayın üstündedir.",
            "Delikler ve parçalar da sıklaşır; doğru çoklu çizgi olur. Yeni köşelerin kotları kenarları boyunca taşınır. Bir çalıştırma en çok 10000000 köşe ekler.",
        ]
        .join("\n\n")),
        keywords: [
            "köşe ekle", "sıklaştır", "yoğunlaştır", "aralık", "densify", "densify by interval",
            "densify by count",
        ]
        .map(String::from)
        .to_vec(),
        aliases: vec!["SIKLASTIR".into(), "KOSEEKLETOPLU".into()],
        icon: Some("densify".into()),
        parameters: vec![
            features(
                &["line", "polyline", "polygon"],
                "Nesneler",
                "Köşe eklenecek çizgiler ve alanlar.",
            ),
            ParamDef::new(
                "method",
                "Yöntem",
                ParamKind::Choice {
                    options: vec![
                        EnumOption::new("interval", "Aralıkla").hint("Parçalar aralıktan uzun olmaz"),
                        EnumOption::new("count", "Sayıyla").hint("Her kenara aynı sayıda köşe"),
                    ],
                },
            )
            .default_value(json!("interval")),
            ParamDef::new(
                "interval",
                "Aralık",
                ParamKind::Number {
                    min: Some(0.001),
                    max: Some(1_000_000.0),
                    integer: false,
                    unit: "m".into(),
                    placeholder: None,
                },
            )
            .default_value(json!(1))
            .shown_when(|v| v.get("method").and_then(serde_json::Value::as_str) != Some("count"))
            .describe("Kenar parçalarının en uzun boyu."),
            ParamDef::new(
                "count",
                "Köşe sayısı",
                ParamKind::Number {
                    min: Some(1.0),
                    max: Some(1000.0),
                    integer: true,
                    unit: String::new(),
                    placeholder: None,
                },
            )
            .default_value(json!(1))
            .shown_when(|v| v.get("method").and_then(serde_json::Value::as_str) == Some("count"))
            .describe("Her kenara eklenecek köşe."),
            layer_param("Sıklaştırılan", "#E5484D"),
        ],
        outputs: vec![
            OutputDef::new("objects", "Yazılan nesneler", OutputKind::Features),
            OutputDef::new("count", "Eklenen köşe sayısı", OutputKind::Number),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run_densify),
    }
}

fn run_densify(v: &Resolved<'_>, _ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let inputs = &v.features("input").entities;
    let by = if v.text("method") == "count" {
        Density::Count(v.number("count").unwrap_or(1.0) as u32)
    } else {
        Density::Interval(v.number("interval").unwrap_or(1.0))
    };
    let shapes: Vec<_> = inputs.iter().map(|e| shape(e)).collect();
    let total: u64 = shapes.iter().map(|s| densify_added(s, by)).sum();
    if total > MOST_ADDED {
        return RunResult::refused(format!(
            "Bu ayarla {total} köşe eklenirdi; bir çalıştırma en çok {MOST_ADDED} köşe ekler. Aralığı büyütün ya da sayıyı küçültün."
        ));
    }
    feedback.progress(0.0, "Köşeler ekleniyor");
    let layer = &v.layer("layer").id;
    let mut add = Vec::with_capacity(inputs.len());
    for (e, s) in inputs.iter().zip(&shapes) {
        let (r, _) = kentos_geometry_core::ops::convert::paths::densify(s, zs_of(e).as_ref(), by);
        add.extend(placed(r, layer, e.base().attrs.clone()));
    }
    done(
        add,
        total as usize,
        format!("{} nesneye {total} köşe eklendi.", inputs.len()),
    )
}
