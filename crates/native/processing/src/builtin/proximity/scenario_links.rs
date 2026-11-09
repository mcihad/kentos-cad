//! Altyapı bağlarını çiz (docs/adr/3860; plan/map-cad.md MAP-10): a scenario's areas joined to their nearest object of
//! an infrastructure layer, drawn as lines on a layer of their own, as `cad.scenarios.impact`'s nearness finds them
//! (`scenarios_impact::preview`, docs/adr/3460): from each measured area's centroid to the nearest place of the nearest
//! object, on the field state's side, the scenario's or both. The table says each link; one undo step. A link of no
//! length (the centroid on the line) is said, not drawn. Nearness is not a line's capacity nor a right to connect.

use std::collections::BTreeMap;

use kentos_commands::{Entity, EntityBase, ImpactInfrastructure, ImpactLink, LineEntity, ScenariosImpact, Vec2};
use kentos_native_application::scenarios_impact::preview;
use serde_json::{Value, json};

use crate::text::to_fixed;
use crate::types::{
    EnumOption, Feedback, NewLayerStyle, OutputDef, OutputKind, ParamDef, ParamKind, Resolved, RunContext, RunResult,
    ScopeKind, Target, Tool,
};

/// The links' table.
const COLUMNS: [&str; 5] = ["Taraf", "Alan", "Hat", "Uzaklık", "Yazıldı"];

pub fn tool() -> Tool {
    Tool {
        id: "scenarios.links".into(),
        label: "Altyapı bağlarını çiz".into(),
        category: "proximity".into(),
        description: "Bir senaryonun alanlarını bir altyapı katmanının en yakın nesnesine bağlar; her bağı alanın ağırlık merkezinden nesnenin en yakın yerine bir çizgi olarak kendi katmanına yazar.".into(),
        help: Some([
            "Bağlar Senaryo ölçüleri'nin altyapıya yakınlık ölçüsünün bağlarıdır (cad.scenarios.impact): her ölçülen alanın ağırlık merkezinden altyapı katmanının en yakın nesnesinin en yakın yerine. Senaryonun yerine geçtiği katmansa her taraf kendi katmanıyla ölçülür.",
            "Uzaklık ölçünün eşiğidir: alanlar yakın ya da uzak sayılır; bağ her ölçülen alan için çizilir. Kullanımlar boşsa bütün alanlar ölçülür.",
            "Ağırlık merkezi hattın üstündeyse bağın boyu yoktur: çizilmez, tabloda söylenir. Yakınlık hattın kapasitesi ya da bağlanma hakkı değildir.",
        ]
        .join("\n\n")),
        keywords: ["senaryo", "altyapı", "bağ", "en yakın", "hat", "kanalizasyon", "su", "scenario", "links"]
            .map(String::from)
            .to_vec(),
        aliases: ["ALTYAPIBAGLARI", "SCENARIOLINKS"].map(String::from).to_vec(),
        icon: Some("shortestLine".into()),
        parameters: vec![
            ParamDef::new(
                "scenario",
                "Senaryo",
                ParamKind::Text { placeholder: Some("Senaryo grubunun adı".into()), max_length: Some(200), allow_empty: false },
            )
            .describe("Senaryo grubunun adı ya da kimliği."),
            ParamDef::new(
                "infrastructure",
                "Altyapı katmanı",
                ParamKind::Features { kinds: None, scopes: Some(vec![ScopeKind::Layer]), writes: false },
            )
            .allow_empty()
            .describe("Su, kanalizasyon ya da yol hattının katmanı."),
            ParamDef::new(
                "distance",
                "Uzaklık",
                ParamKind::Number { min: None, max: None, integer: false, unit: "m".into(), placeholder: None },
            )
            .describe("Yakın sayılmanın eşiği; KentOS öntanımlı değer vermez, 0'ı ölçü reddeder."),
            ParamDef::new(
                "uses",
                "Kullanımlar",
                ParamKind::Text { placeholder: Some("Hepsi".into()), max_length: Some(500), allow_empty: true },
            )
            .optional()
            .describe("Ölçülecek kullanımlar, virgülle (konut, ticaret); boş: bütün alanlar."),
            ParamDef::new(
                "side",
                "Taraf",
                ParamKind::Choice {
                    options: vec![
                        EnumOption::new("both", "İkisi"),
                        EnumOption::new("base", "Mevcut durum"),
                        EnumOption::new("alternative", "Senaryo"),
                    ],
                },
            )
            .default_value(json!("both")),
            ParamDef::new(
                "layer",
                "Bağ katmanı",
                ParamKind::Layer {
                    new_layer_style: NewLayerStyle {
                        color: Some("#0090FF".into()),
                        line_weight: Some(0.25),
                        ..NewLayerStyle::default()
                    },
                    above: None,
                    below: None,
                },
            )
            .default_value(json!({ "newName": "Altyapı bağları" }))
            .describe("Bu adda katman yoksa oluşturulur."),
        ],
        outputs: vec![
            OutputDef::new("links", "Bağlar", OutputKind::Table),
            OutputDef::new("count", "Bağ sayısı", OutputKind::Number),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run),
    }
}

/// The scenario group named `text`: by its id, else by its name.
fn scenario_of(ctx: &RunContext<'_>, text: &str) -> String {
    let layers = ctx.doc.layers();
    if layers.get(text).is_some() {
        return text.to_owned();
    }
    layers.nodes().iter().find(|n| n.name == text).map_or_else(|| text.to_owned(), |n| n.id.clone())
}

fn run(v: &Resolved<'_>, ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    feedback.progress(0.0, "Altyapı bağları ölçülüyor");
    let Some(layer) = v.features("infrastructure").layer_id.clone() else {
        return RunResult { refused: Some("Altyapı katmanını seçin.".into()), ..RunResult::default() };
    };
    let Some(distance) = v.number("distance") else {
        return RunResult { refused: Some("Uzaklığı yazın (m); KentOS öntanımlı değer vermez.".into()), ..RunResult::default() };
    };
    let from: Vec<String> = v.text("uses").split(',').map(str::trim).filter(|s| !s.is_empty()).map(str::to_owned).collect();
    let input = ScenariosImpact {
        scenario: scenario_of(ctx, v.text("scenario").trim()),
        infrastructure: Some(ImpactInfrastructure { layer, distance, from }),
        ..ScenariosImpact::default()
    };
    let answer = match preview(ctx.doc, &input) {
        Ok(a) => a,
        Err(why) => return RunResult { refused: Some(why), ..RunResult::default() },
    };
    let Some(near) = answer.near else {
        return RunResult { refused: Some("Senaryo ölçüsü yakınlığı vermedi.".into()), ..RunResult::default() };
    };
    let side = v.text("side");
    let mut links: Vec<(&str, &ImpactLink)> = Vec::new();
    if side != "alternative" {
        links.extend(near.base_links.iter().map(|l| ("Mevcut durum", l)));
    }
    if side != "base" {
        links.extend(near.alternative_links.iter().map(|l| ("Senaryo", l)));
    }
    let target = v.layer("layer");
    let d = ctx.units.length_decimals;
    let length = |m: f64| format!("{} m", to_fixed(m, d));
    let mut rows = Vec::new();
    let mut lines = Vec::new();
    for (name, l) in &links {
        let drawn = l.distance > 0.0;
        rows.push(vec![
            (*name).to_owned(),
            l.area.clone(),
            l.line.clone(),
            length(l.distance),
            if drawn { "evet".to_owned() } else { "boyu yok".to_owned() },
        ]);
        if !drawn {
            continue;
        }
        let mut attrs = BTreeMap::new();
        attrs.insert("Taraf".to_owned(), (*name).to_owned());
        attrs.insert("Alan".to_owned(), l.area.clone());
        attrs.insert("Hat".to_owned(), l.line.clone());
        attrs.insert("Uzaklık".to_owned(), length(l.distance));
        lines.push(Entity::Line(LineEntity {
            base: EntityBase {
                look: Default::default(),
                id: 0,
                layer_id: target.id.clone(),
                color: None,
                attrs,
                label: None,
                symbol: None,
                line_weight: None,
                label_pins: Vec::new(),
                attachments: Vec::new(),
            },
            a: Vec2 { x: l.from[0], y: l.from[1] },
            b: Vec2 { x: l.to[0], y: l.to[1] },
            za: None,
            zb: None,
            ma: None,
            mb: None,
        }));
    }
    let wrote = lines.len();
    let flat = links.len() - wrote;
    let mut summary = format!("“{}”: {} alan en yakın altyapı nesnesine bağlandı; {wrote} bağ yazıldı", answer.name, links.len());
    if flat > 0 {
        summary.push_str(&format!(", {flat} alanın merkezi hattın üstünde (bağı çizilmedi)"));
    }
    summary.push('.');
    let outputs: serde_json::Map<String, Value> =
        [("links".to_owned(), json!({ "columns": COLUMNS, "rows": rows })), ("count".to_owned(), json!(wrote))]
            .into_iter()
            .collect();
    RunResult {
        changes: (wrote > 0).then(|| crate::types::ChangeSet { add: lines, ..Default::default() }),
        outputs,
        summary: Some(summary),
        ..RunResult::default()
    }
}
