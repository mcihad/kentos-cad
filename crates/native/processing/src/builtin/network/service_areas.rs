//! Hizmet alanları (the web's `builtin/network/serviceAreas.ts`; docs/adr/0209
//! §6; ArcGIS's Service Area): the facilities of a layer and the network
//! within each break of the chosen cost, as Hizmet alanı draws it: the areas
//! (the reached lines' buffers, Kenar payı wide; discs or rings; merged or one
//! each) and, when asked, the lines.

use std::collections::BTreeMap;

use kentos_geometry_core::display::fixed;
use kentos_geometry_core::tools::point_text::parse_number;
use serde_json::{Value, json};

use super::{
    RunNetwork, cost_value, line_geometry, missing_note, name_of, network_param, places_of,
    points_param, reach_param, run_network,
};
use crate::builtin::geometry::{edit_object, layer_param_styled, new_object};
use crate::types::{
    ChangeSet, EnumOption, Feedback, OutputDef, OutputKind, ParamDef, ParamKind, Resolved,
    RunContext, RunResult, Target, Tool, Values,
};

/// The breaks as written (“5 10 15”): rising numbers above zero, at most ten; none when they are not.
pub fn breaks_of(text: &str) -> Option<Vec<f64>> {
    let values: Vec<Option<f64>> = text
        .split(|c: char| c.is_whitespace() || c == ';')
        .filter(|v| !v.is_empty())
        .map(parse_number)
        .collect();
    let ok = !values.is_empty()
        && values.len() <= 10
        && values.iter().enumerate().all(|(i, v)| matches!(v, Some(v) if *v > 0.0 && v.is_finite() && (i == 0 || values[i - 1].is_some_and(|p| *v > p))));
    ok.then(|| values.into_iter().flatten().collect())
}

/// A number as a label shows it (“0–300”).
fn plain(v: f64) -> String {
    let s = fixed(v, 6);
    let s = s.trim_end_matches('0');
    s.strip_suffix('.').unwrap_or(s).to_owned()
}

pub fn tool() -> Tool {
    Tool {
        id: "network.serviceAreas".into(),
        label: "Hizmet alanları".into(),
        category: "network".into(),
        description: "Tesislerden ağ boyunca verilen uzaklık ya da sürelerde ulaşılan alanları çizer: aralık başına alan, isteğe bağlı ulaşılan yollarla.".into(),
        help: Some([
            "Ağ projenin yol ya da şebeke ağıdır (Ağlar); Aralıklar maliyetin biriminde artan sayılardır (Uzunluk’ta metre, Süre’de dakika): “5 10 15”.",
            "Alan, ulaşılan yolların Kenar payı kadar tamponudur; Disk her aralığın bütün ulaşılanını, Halka bir öncekinden farkını yazar. Birleşik açıkken bütün tesislere aralık başına tek alan, kapalıyken her tesise ayrı alan yazılır.",
            "Yön Tesisten (tesisten çıkış) ya da Tesise (tesise varış; tek yönlü yollarda farklıdır).",
            "Alanlara Ağ, Maliyet, Tesis, Başlangıç ve Bitiş; çizgilere Aralık (“0–5”) yazılır.",
        ]
        .join("\n\n")),
        keywords: ["hizmet alanı", "service area", "erişim", "izokron", "isochrone", "ulaşım süresi", "ağ", "itfaiye"].map(String::from).to_vec(),
        aliases: ["HIZMETALANLARI", "SERVICEAREAS"].map(String::from).to_vec(),
        icon: Some("serviceAreas".into()),
        parameters: vec![
            network_param("Alanların bulunacağı ağ ve maliyet."),
            points_param("facilities", "Tesisler", Some("Okul, sağlık ocağı, itfaiye istasyonu gibi noktalar.")),
            ParamDef::new("breaks", "Aralıklar", ParamKind::Text { placeholder: Some("5 10 15".into()), max_length: None, allow_empty: false })
                .default_value(json!("500 1000 1500"))
                .describe("Maliyetin biriminde artan sayılar, aralarında boşluk; en çok 10."),
            ParamDef::new(
                "direction",
                "Yön",
                ParamKind::Choice {
                    options: vec![
                        EnumOption::new("from", "Tesisten").hint("Tesisten çıkılarak ulaşılan yerler"),
                        EnumOption::new("to", "Tesise").hint("Tesise bu sürede varılan yerler"),
                    ],
                },
            )
            .default_value(json!("from")),
            ParamDef::new(
                "shape",
                "Biçim",
                ParamKind::Choice {
                    options: vec![
                        EnumOption::new("disc", "Disk").hint("Her aralığın bütün ulaşılanı"),
                        EnumOption::new("ring", "Halka").hint("Bir önceki aralıktan farkı"),
                    ],
                },
            )
            .default_value(json!("disc")),
            ParamDef::new("merged", "Birleşik", ParamKind::Boolean).default_value(json!(true)).describe("Bütün tesislere aralık başına tek alan; kapalıyken her tesise ayrı."),
            ParamDef::new("trim", "Kenar payı", ParamKind::Number { min: Some(0.01), max: Some(10_000.0), integer: false, unit: "m".into(), placeholder: None })
                .default_value(json!(50))
                .describe("Alanın ulaşılan yollardan uzaklığı."),
            reach_param("Tesisler ağa bu uzaklıktan yakınsa ağın en yakın yerine oturur."),
            layer_param_styled("layer", "Çıktı katmanı", "Hizmet alanı", "#1976D2", 0.25, Some("#1976D233"), Some("Bu adda katman yoksa oluşturulur.")),
            ParamDef::new("lines", "Ulaşılan yollar", ParamKind::Boolean).default_value(json!(false)).describe("Aralıklarda ulaşılan yollar da yazılır."),
            layer_param_styled("linesLayer", "Yolların katmanı", "Hizmet alanı çizgileri", "#1565C0", 0.35, None, None).shown_when(lines_on),
        ],
        outputs: vec![OutputDef::new("areas", "Alanlar", OutputKind::Features), OutputDef::new("count", "Alan sayısı", OutputKind::Number)],
        targets: vec![Target::Client, Target::Worker],
        validate: Some(validate),
        preview: None,
        run: Some(run),
    }
}

fn lines_on(values: &Values) -> bool {
    values.get("lines").and_then(Value::as_bool) == Some(true)
}

fn validate(values: &Values) -> Option<String> {
    let text = values.get("breaks").and_then(Value::as_str).unwrap_or("");
    breaks_of(text).is_none().then(|| "“Aralıklar” artan, sıfırdan büyük sayılar olmalı (en çok 10), aralarında boşluk: 5 10 15.".to_owned())
}

fn run(v: &Resolved<'_>, ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let Some(breaks) = breaks_of(v.text("breaks")) else {
        return RunResult::refused("“Aralıklar” artan, sıfırdan büyük sayılar olmalı (en çok 10), aralarında boşluk: 5 10 15.".into());
    };
    let RunNetwork {
        def,
        cost,
        cost_names,
        mut session,
    } = match run_network(v, ctx, feedback) {
        Ok(n) => n,
        Err(why) => return RunResult::refused(why),
    };
    let reach = v.number("reach").unwrap_or(100.0);
    let (facilities, missing) = places_of(&session, &v.features("facilities").entities, reach);
    missing_note(missing.len(), "tesis", reach, feedback);
    if facilities.is_empty() {
        return RunResult::refused("Ağın üstünde tesis yok; noktaların ağa yakın olduğunu ya da Arama uzaklığını denetleyin.".into());
    }
    feedback.progress(0.0, "Hizmet alanları hesaplanıyor");
    let ring = v.text("shape") == "ring";
    let points: Vec<_> = facilities.iter().map(|(_, p)| *p).collect();
    let served = match session.service_area(
        &points,
        &breaks,
        reach,
        cost,
        v.text("direction") == "to",
        !v.flag("merged"),
        &[],
        v.number("trim").unwrap_or(50.0),
        ring,
        true,
    ) {
        Ok(s) => s,
        Err(why) => {
            return RunResult::refused(why.words(&format!("{} m", crate::text::js_number(reach))));
        }
    };
    let name = cost_names[cost].clone();
    let all = facilities
        .iter()
        .map(|(e, _)| name_of(e))
        .collect::<Vec<_>>()
        .join(", ");
    let facility = |f: Option<usize>| f.map_or_else(|| all.clone(), |i| name_of(facilities[i].0));
    let number = |x: f64| cost_value(Some(x), cost, ctx.units.length_decimals);
    let layer = v.layer("layer").id.clone();
    let mut add = Vec::new();
    for a in &served.areas {
        let Some(shape) = a.shape.clone() else {
            continue;
        };
        let start = if ring && a.band > 0 {
            breaks[a.band - 1]
        } else {
            0.0
        };
        let attrs = BTreeMap::from([
            ("Ağ".to_owned(), def.name.clone()),
            ("Maliyet".to_owned(), name.clone()),
            ("Tesis".to_owned(), facility(a.facility)),
            ("Başlangıç".to_owned(), number(start)),
            ("Bitiş".to_owned(), number(breaks[a.band])),
        ]);
        if let Some(e) = new_object(shape, &layer, attrs) {
            add.push(e);
        }
    }
    let areas = add.len();
    let lines = v.flag("lines");
    if lines {
        let to = v.layer("linesLayer").id.clone();
        for l in &served.lines {
            let lo = if l.band > 0 { breaks[l.band - 1] } else { 0.0 };
            let attrs = BTreeMap::from([
                ("Ağ".to_owned(), def.name.clone()),
                ("Maliyet".to_owned(), name.clone()),
                ("Tesis".to_owned(), facility(l.facility)),
                (
                    "Aralık".to_owned(),
                    format!("{}–{}", plain(lo), plain(breaks[l.band])),
                ),
            ]);
            if let Some(e) = edit_object(&line_geometry(&l.line), &to, attrs) {
                add.push(e);
            }
        }
    }
    if areas == 0 {
        feedback.warn("Hiçbir aralıkta ulaşılan yer yok.".into());
    }
    let roads = if lines {
        format!(" ({} yol)", add.len() - areas)
    } else {
        String::new()
    };
    let summary = format!(
        "{} tesisin {} aralıkta {areas} hizmet alanı yazıldı{roads}; “{}” ağında {name}.",
        facilities.len(),
        breaks.len(),
        def.name
    );
    RunResult {
        changes: Some(ChangeSet {
            add,
            ..ChangeSet::default()
        }),
        outputs: [("count".to_owned(), json!(areas))].into_iter().collect(),
        summary: Some(summary),
        ..RunResult::default()
    }
}
