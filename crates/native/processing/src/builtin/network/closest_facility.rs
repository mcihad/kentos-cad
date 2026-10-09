//! En yakın tesis (the web's `builtin/network/closestFacility.ts`; docs/adr/0209
//! §7; ArcGIS's Closest Facility): for each incident the `count` cheapest
//! facilities along the network, within an upper bound when one is given;
//! from the incident to the facility or the other way. Each way is written
//! to the output layer as a polyline with its incident, facility, rank and
//! costs; the same rows are the table. Equal costs: the facility first in the
//! list first.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use super::{
    RunNetwork, cost_value, line_geometry, missing_note, name_of, network_param, places_of,
    points_param, reach_param, run_network,
};
use crate::builtin::geometry::{edit_object, layer_param_styled};
use crate::text::js_number;
use crate::types::{
    ChangeSet, EnumOption, Feedback, OutputDef, OutputKind, ParamDef, ParamKind, Resolved,
    RunContext, RunResult, Target, Tool, Values,
};

pub fn tool() -> Tool {
    Tool {
        id: "network.closestFacility".into(),
        label: "En yakın tesis".into(),
        category: "network".into(),
        description: "Her olay noktası için ağ boyunca en yakın tesisleri (okul, sağlık ocağı, itfaiye) bulur; yollarını ve maliyetlerini yazar.".into(),
        help: Some([
            "Ağ projenin yol ya da şebeke ağıdır (Ağlar); maliyet uzunluk ya da ağın süresi ve maliyetleridir. Tek yönler ve kapalı yollar gözetilir.",
            "Olaylar ve tesisler noktalardır; ağa Arama uzaklığından uzak olan nokta alınmaz ve söylenir.",
            "Her olayın en yakın tesisleri Tesis sayısı kadar, maliyeti küçükten büyüğe yazılır; Üst sınır verilirse ondan pahalı tesis alınmaz. Eşit maliyette listede önce gelen tesis önce gelir.",
            "Her yol çıktı katmanına çoklu çizgi olarak yazılır: Olay, Tesis, Sıra, Ağ, Maliyet ve her maliyetin toplamıyla; aynı satırlar tabloda.",
        ]
        .join("\n\n")),
        keywords: ["en yakın tesis", "closest facility", "en yakın", "itfaiye", "okul", "hastane", "ağ", "rota", "yol"].map(String::from).to_vec(),
        aliases: ["ENYAKINTESIS", "CLOSESTFACILITY"].map(String::from).to_vec(),
        icon: Some("closestFacility".into()),
        parameters: vec![
            network_param("Yolların bulunacağı ağ ve maliyet."),
            points_param("incidents", "Olaylar", Some("Tesislerin aranacağı noktalar (adres, olay yeri).")),
            points_param("facilities", "Tesisler", Some("Okul, sağlık ocağı, itfaiye istasyonu gibi noktalar.")),
            ParamDef::new("count", "Tesis sayısı", ParamKind::Number { min: Some(1.0), max: Some(10.0), integer: true, unit: String::new(), placeholder: None })
                .default_value(json!(1))
                .describe("Her olay için en yakın kaç tesis."),
            ParamDef::new("cutoff", "Üst sınır", ParamKind::Number { min: Some(0.0), max: None, integer: false, unit: String::new(), placeholder: Some("Yok".into()) })
                .optional()
                .default_value(Value::Null)
                .describe("Maliyeti bundan büyük tesis alınmaz (maliyetin biriminde: m, dk ya da alanın birimi)."),
            ParamDef::new(
                "direction",
                "Yön",
                ParamKind::Choice {
                    options: vec![
                        EnumOption::new("toFacility", "Olaydan tesise").hint("Olaydan çıkılır (hasta tesise gider)"),
                        EnumOption::new("fromFacility", "Tesisten olaya").hint("Tesisten çıkılır (itfaiye olaya gider)"),
                    ],
                },
            )
            .default_value(json!("toFacility")),
            reach_param("Noktalar ağa bu uzaklıktan yakınsa ağın en yakın yerine oturur."),
            layer_param_styled("layer", "Çıktı katmanı", "En yakın tesis", "#7B1FA2", 0.5, None, Some("Bu adda katman yoksa oluşturulur.")),
        ],
        outputs: vec![
            OutputDef::new("routes", "Yollar", OutputKind::Features),
            OutputDef::new("table", "En yakın tesisler", OutputKind::Table),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: Some(preview),
        run: Some(run),
    }
}

fn preview(values: &Values) -> Option<String> {
    let count = js_number(values.get("count").and_then(Value::as_f64).unwrap_or(1.0));
    match values
        .get("cutoff")
        .and_then(Value::as_f64)
        .filter(|c| *c != 0.0)
    {
        Some(c) => Some(format!("En yakın {count} tesis, en çok {}", js_number(c))),
        None => Some(format!("En yakın {count} tesis")),
    }
}

fn run(v: &Resolved<'_>, ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
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
    let (incidents, missing_i) = places_of(&session, &v.features("incidents").entities, reach);
    let (facilities, missing_f) = places_of(&session, &v.features("facilities").entities, reach);
    missing_note(missing_i.len(), "olay", reach, feedback);
    missing_note(missing_f.len(), "tesis", reach, feedback);
    if incidents.is_empty() || facilities.is_empty() {
        return RunResult::refused("Ağın üstünde olay ya da tesis yok; noktaların ağa yakın olduğunu ya da Arama uzaklığını denetleyin.".into());
    }
    feedback.progress(0.0, "En yakın tesisler aranıyor");
    let k = v.number("count").unwrap_or(1.0).max(1.0) as usize;
    let cutoff = v.number("cutoff");
    let reverse = v.text("direction") == "fromFacility";
    let origins: Vec<_> = incidents.iter().map(|(_, p)| *p).collect();
    let targets: Vec<_> = facilities.iter().map(|(_, p)| *p).collect();
    let rows = match session.nearest_of(
        &origins,
        &targets,
        reach,
        Some(k),
        cutoff,
        cost,
        reverse,
        &[],
        true,
    ) {
        Ok(r) => r,
        Err(_) => return RunResult::refused("Noktalar ağda bulunamadı.".into()),
    };
    let decimals = ctx.units.length_decimals;
    let layer = &v.layer("layer").id;
    let mut add = Vec::new();
    let mut table: Vec<Vec<String>> = Vec::new();
    let mut none = 0;
    for (i, row) in rows.iter().enumerate() {
        if row.is_empty() {
            none += 1;
        }
        for (rank, f) in row.iter().enumerate() {
            let values: Vec<String> = (0..cost_names.len())
                .map(|c| {
                    cost_value(
                        f.totals.as_ref().and_then(|t| t.get(c).copied().flatten()),
                        c,
                        decimals,
                    )
                })
                .collect();
            let incident = name_of(incidents[i].0);
            let facility = name_of(facilities[f.target].0);
            let mut line = vec![incident.clone(), facility.clone(), (rank + 1).to_string()];
            line.extend(values.iter().cloned());
            table.push(line);
            if let Some(l) = f.line.as_ref().filter(|l| l.pts.len() >= 2) {
                let mut attrs = BTreeMap::from([
                    ("Olay".to_owned(), incident),
                    ("Tesis".to_owned(), facility),
                    ("Sıra".to_owned(), (rank + 1).to_string()),
                    ("Ağ".to_owned(), def.name.clone()),
                    ("Maliyet".to_owned(), cost_names[cost].clone()),
                ]);
                for (c, name) in cost_names.iter().enumerate() {
                    attrs.insert(name.clone(), values[c].clone());
                }
                if let Some(e) = edit_object(&line_geometry(l), layer, attrs) {
                    add.push(e);
                }
            }
        }
    }
    if none > 0 {
        feedback.warn(format!(
            "{none} olaya {}ulaşılabilen tesis yok.",
            if cutoff.is_some() {
                "üst sınır içinde "
            } else {
                ""
            }
        ));
    }
    let mut columns = vec!["Olay".to_owned(), "Tesis".to_owned(), "Sıra".to_owned()];
    columns.extend(cost_names.iter().cloned());
    let summary = format!(
        "{} olay için {} tesis bulundu; {} yol “{}” ağında {} ile yazıldı.",
        incidents.len(),
        table.len(),
        add.len(),
        def.name,
        cost_names[cost]
    );
    RunResult {
        changes: Some(ChangeSet {
            add,
            ..ChangeSet::default()
        }),
        outputs: [(
            "table".to_owned(),
            json!({ "columns": columns, "rows": table }),
        )]
        .into_iter()
        .collect(),
        summary: Some(summary),
        ..RunResult::default()
    }
}
