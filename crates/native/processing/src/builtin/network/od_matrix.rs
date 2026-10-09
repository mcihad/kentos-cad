//! Maliyet matrisi (the web's `builtin/network/odMatrix.ts`; docs/adr/0209
//! §7; ArcGIS's OD Cost Matrix): from each origin the cost along the network
//! to each destination, the cheapest first, within an upper bound and the
//! nearest `count` when given. The table (Başlangıç, Varış, Sıra and the
//! chosen cost) goes to the clipboard or a CSV; straight lines from each
//! origin to its destinations are written when asked.

use std::collections::BTreeMap;

use kentos_contracts::EntityGeometry;
use serde_json::{Value, json};

use super::{
    RunNetwork, cost_value, missing_note, name_of, network_param, places_of, points_param,
    reach_param, run_network,
};
use crate::builtin::geometry::{edit_object, layer_param_styled};
use crate::types::{
    ChangeSet, Feedback, OutputDef, OutputKind, ParamDef, ParamKind, Resolved, RunContext,
    RunResult, Target, Tool, Values,
};

pub fn tool() -> Tool {
    Tool {
        id: "network.odMatrix".into(),
        label: "Maliyet matrisi".into(),
        category: "network".into(),
        description: "Başlangıç noktalarından varış noktalarına ağ boyunca maliyetleri tablo olarak çıkarır; isteğe bağlı düz çizgilerle.".into(),
        help: Some([
            "Ağ projenin yol ya da şebeke ağıdır (Ağlar); maliyet uzunluk ya da ağın süresi ve maliyetleridir. Tek yönler ve kapalı yollar gözetilir.",
            "Her başlangıçtan her varışa maliyet, ucuzdan pahalıya sıralanır; En yakın verilirse yalnız o kadar varış, Üst sınır verilirse ondan ucuzlar yazılır. Ulaşılamayan varış yazılmaz.",
            "Tablo Başlangıç, Varış, Sıra ve maliyet sütunlarıdır; panoya ya da CSV’ye alınır. Düz çizgiler açıkken her satır başlangıçtan varışa bir çizgidir.",
        ]
        .join("\n\n")),
        keywords: ["maliyet matrisi", "od matrix", "başlangıç varış", "uzaklık matrisi", "süre matrisi", "ağ"].map(String::from).to_vec(),
        aliases: ["MALIYETMATRISI", "ODMATRIX", "ODCOSTMATRIX"].map(String::from).to_vec(),
        icon: Some("odMatrix".into()),
        parameters: vec![
            network_param("Maliyetlerin bulunacağı ağ ve maliyet."),
            points_param("origins", "Başlangıçlar", None),
            points_param("destinations", "Varışlar", None),
            ParamDef::new("count", "En yakın", ParamKind::Number { min: Some(1.0), max: Some(10_000.0), integer: true, unit: String::new(), placeholder: Some("Hepsi".into()) })
                .optional()
                .default_value(Value::Null)
                .describe("Her başlangıç için en yakın kaç varış; boş: hepsi."),
            ParamDef::new("cutoff", "Üst sınır", ParamKind::Number { min: Some(0.0), max: None, integer: false, unit: String::new(), placeholder: Some("Yok".into()) })
                .optional()
                .default_value(Value::Null)
                .describe("Maliyeti bundan büyük varış yazılmaz."),
            reach_param("Noktalar ağa bu uzaklıktan yakınsa ağın en yakın yerine oturur."),
            ParamDef::new("lines", "Düz çizgiler", ParamKind::Boolean).default_value(json!(false)).describe("Her satır için başlangıçtan varışa düz çizgi yazılır."),
            layer_param_styled("layer", "Çizgilerin katmanı", "Maliyet matrisi", "#00897B", 0.18, None, None).shown_when(lines_on),
        ],
        outputs: vec![OutputDef::new("table", "Maliyet matrisi", OutputKind::Table), OutputDef::new("lines", "Çizgiler", OutputKind::Features)],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run),
    }
}

fn lines_on(values: &Values) -> bool {
    values.get("lines").and_then(Value::as_bool) == Some(true)
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
    let (origins, missing_o) = places_of(&session, &v.features("origins").entities, reach);
    let (targets, missing_t) = places_of(&session, &v.features("destinations").entities, reach);
    missing_note(missing_o.len(), "başlangıç", reach, feedback);
    missing_note(missing_t.len(), "varış", reach, feedback);
    if origins.is_empty() || targets.is_empty() {
        return RunResult::refused("Ağın üstünde başlangıç ya da varış yok; noktaların ağa yakın olduğunu ya da Arama uzaklığını denetleyin.".into());
    }
    feedback.progress(0.0, "Maliyetler hesaplanıyor");
    let k = v.number("count").map(|k| k.max(1.0) as usize);
    let cutoff = v.number("cutoff");
    let from: Vec<_> = origins.iter().map(|(_, p)| *p).collect();
    let to: Vec<_> = targets.iter().map(|(_, p)| *p).collect();
    let rows = match session.nearest_of(&from, &to, reach, k, cutoff, cost, false, &[], false) {
        Ok(r) => r,
        Err(_) => return RunResult::refused("Noktalar ağda bulunamadı.".into()),
    };
    let name = cost_names[cost].clone();
    let lines = v.flag("lines");
    let layer = v.layer("layer").id.clone();
    let mut table: Vec<Vec<String>> = Vec::new();
    let mut add = Vec::new();
    for (i, row) in rows.iter().enumerate() {
        for (rank, f) in row.iter().enumerate() {
            let (o, a) = origins[i];
            let (t, b) = targets[f.target];
            let value = cost_value(Some(f.cost), cost, ctx.units.length_decimals);
            table.push(vec![
                name_of(o),
                name_of(t),
                (rank + 1).to_string(),
                value.clone(),
            ]);
            if lines {
                let attrs = BTreeMap::from([
                    ("Başlangıç".to_owned(), name_of(o)),
                    ("Varış".to_owned(), name_of(t)),
                    ("Sıra".to_owned(), (rank + 1).to_string()),
                    ("Ağ".to_owned(), def.name.clone()),
                    ("Maliyet".to_owned(), name.clone()),
                    (name.clone(), value),
                ]);
                let g = EntityGeometry::Line {
                    a: kentos_contracts::Vec2 { x: a.x, y: a.y },
                    b: kentos_contracts::Vec2 { x: b.x, y: b.y },
                    zs: None,
                };
                if let Some(e) = edit_object(&g, &layer, attrs) {
                    add.push(e);
                }
            }
        }
    }
    let summary = format!(
        "{} başlangıçtan {} varışa {} satır (“{}” ağında {name}).",
        origins.len(),
        targets.len(),
        table.len(),
        def.name
    );
    RunResult {
        changes: (!add.is_empty()).then(|| ChangeSet {
            add,
            ..ChangeSet::default()
        }),
        outputs: [(
            "table".to_owned(),
            json!({ "columns": ["Başlangıç", "Varış", "Sıra", name], "rows": table }),
        )]
        .into_iter()
        .collect(),
        summary: Some(summary),
        ..RunResult::default()
    }
}
