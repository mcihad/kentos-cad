//! Topolojiyi kurallarla denetle (docs/adr/1980; plan/map-cad.md MAP-07: “GIS'te de Processing, veri formu ve MCP
//! üzerinden aynı denetim sonucu erişilir”): the project's topology rules run from İşlem araç kutusu, a model or a
//! script, read only. The check is `cad.topology.check`'s own (`topology_check::check_on`): the same findings, keys,
//! places, measures, life cycle (Değerlendirilmedi, Hata, İstisna, Yeniden değerlendir) and lapsed exceptions. The
//! findings go to a table, their objects are selected, the counts are numbers. Asked to (docs/adr/2200), the listed
//! findings are also written as points at their places on an error layer, one undo step, as QGIS's Check validity
//! writes its error output and ArcGIS's Export Topology Errors its error features; otherwise nothing is written.

use std::collections::BTreeMap;

use kentos_commands::{
    Entity, EntityBase, PointEntity, PointStyle, PointSymbol, TopologyCheck, TopologyFindingState,
};
use kentos_domain::{Slot, Uuid};
use kentos_geometry_core::ops::topology_rules::problem_label;
use kentos_native_application::topology_check::check_on;
use serde_json::{Value, json};

use crate::text::to_fixed;
use crate::types::{
    Feedback, NewLayerStyle, OutputDef, OutputKind, ParamDef, ParamKind, Resolved, RunContext, RunResult, Target,
    Tool,
};

/// The findings' table.
const COLUMNS: [&str; 8] = ["Kural", "Sorun", "Durum", "Nesneler", "Doğu", "Kuzey", "Ölçü", "Gerekçe"];

/// A finding's state in words.
pub fn state_word(s: TopologyFindingState) -> &'static str {
    match s {
        TopologyFindingState::Open => "Değerlendirilmedi",
        TopologyFindingState::Error => "Hata",
        TopologyFindingState::Exception => "İstisna",
        TopologyFindingState::Lapsed => "Yeniden değerlendir",
    }
}

pub fn tool() -> Tool {
    Tool {
        id: "topology.check".into(),
        label: "Topolojiyi kurallarla denetle".into(),
        category: "geometry".into(),
        description: "Projenin topoloji kurallarını çalıştırır; bulguları durumlarıyla tabloda gösterir ve bulgulu nesneleri seçer; istenirse bulguları hata katmanına nokta olarak yazar.".into(),
        help: Some([
            "Denetim Topoloji kuralları penceresinin ve cad.topology.check komutunun denetimidir: aynı kurallar, aynı bulgular, aynı yerler ve ölçüler. Kurallar projenin kalite profiliyle birlikte Topoloji kuralları penceresinde tanımlanır.",
            "Her bulgunun durumu yaşam döngüsündendir: Değerlendirilmedi, Hata (gözden geçirilip doğrulanmış), İstisna (bilerek bırakılmış) ya da Yeniden değerlendir (işaretlendikten sonra nesnesi ya da kuralı değişmiş). İstisnalar istenmedikçe tabloya girmez, sayılır.",
            "Ölçü çakışma ve boşlukta alan (m²), kısa kenarda uzunluk (m), açıda derecedir. Bulguyu düzeltmek ya da istisna işaretlemek Topoloji kuralları penceresindedir.",
            "Bulguları hata katmanına yaz açıksa tablodaki her bulgu yerinde bir nokta olarak hata katmanına yazılır; noktanın öznitelikleri kural, sorun, durum, nesnelerin kalıcı kimlikleri, ölçü, gerekçe ve bulgunun anahtarıdır. Yazım tek geri alma adımıdır; kapalıyken çizim değişmez.",
        ]
        .join("\n\n")),
        keywords: [
            "topoloji", "kural", "denetle", "çakışma", "boşluk", "kısa kenar", "istisna", "bulgu",
            "topology", "rules", "check",
        ]
        .map(String::from)
        .to_vec(),
        aliases: ["KURALDENETLE", "TOPOLOGYRULESCHECK"].map(String::from).to_vec(),
        icon: Some("topologyCheck".into()),
        parameters: vec![
            ParamDef::new(
                "rules",
                "Kurallar",
                ParamKind::Text { placeholder: Some("Hepsi".into()), max_length: Some(500), allow_empty: true },
            )
            .optional()
            .describe("Çalıştırılacak kuralların kimlikleri, virgülle (r1, r2); boş: projenin bütün kuralları."),
            ParamDef::new("exceptions", "İstisnaları da listele", ParamKind::Boolean).default_value(json!(false)),
            ParamDef::new("write", "Bulguları hata katmanına yaz", ParamKind::Boolean)
                .default_value(json!(false))
                .describe("Açıksa tablodaki bulgular yerlerinde nokta olarak hata katmanına yazılır (tek geri alma adımı)."),
            // The error layer's points are rings, so they stand apart from the lines they sit on.
            ParamDef::new(
                "layer",
                "Hata katmanı",
                ParamKind::Layer {
                    new_layer_style: NewLayerStyle {
                        color: Some("#E5484D".into()),
                        line_weight: Some(0.25),
                        point: Some(PointStyle { symbol: PointSymbol::Ring, size: 10.0 }),
                        ..NewLayerStyle::default()
                    },
                    above: None,
                    below: None,
                },
            )
            .default_value(json!({ "newName": "Topoloji bulguları" }))
            .describe("Bu adda katman yoksa oluşturulur.")
            .shown_when(|v| v.get("write").and_then(Value::as_bool) == Some(true)),
        ],
        outputs: vec![
            OutputDef::new("findings", "Bulgular", OutputKind::Table),
            OutputDef::new("objects", "Bulgulu nesneler", OutputKind::Features),
            OutputDef::new("count", "Bulgu sayısı", OutputKind::Number),
            OutputDef::new("open", "Açık bulgu sayısı", OutputKind::Number),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run),
    }
}

/// The rules asked: the ids between the commas; none when empty.
fn rules_of(text: &str) -> Option<Vec<String>> {
    let ids: Vec<String> = text.split(',').map(str::trim).filter(|s| !s.is_empty()).map(str::to_owned).collect();
    (!ids.is_empty()).then_some(ids)
}

/// A measure as the table writes it: an area or a length to the drawing's decimals, an angle in degrees.
fn measure_text(measure: Option<f64>, kind: Option<&str>, ctx: &RunContext<'_>) -> String {
    match (measure, kind) {
        (Some(m), Some("area")) => format!("{} m²", to_fixed(m, ctx.units.area_decimals)),
        (Some(m), Some("angle")) => format!("{}°", to_fixed(m.to_degrees(), ctx.units.length_decimals)),
        (Some(m), _) => format!("{} m", to_fixed(m, ctx.units.length_decimals)),
        (None, _) => String::new(),
    }
}

fn run(v: &Resolved<'_>, ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    feedback.progress(0.0, "Topoloji kuralları denetleniyor");
    let input = TopologyCheck { rules: rules_of(v.text("rules")), limit: None, preview: None };
    let with_exceptions = v.flag("exceptions");
    let (checked, warnings) = {
        let canceled = || feedback.canceled();
        match check_on(ctx.doc, &input, &canceled) {
            Ok(answer) => answer,
            Err(why) => return RunResult { refused: Some(why), ..RunResult::default() },
        }
    };
    for w in warnings {
        feedback.warn(w.message);
    }
    // An object by its id, as the other tools' tables name it (#12), and its slot.
    let slot_of = |uid: &str| Uuid::parse_str(uid).ok().and_then(|u| ctx.doc.slot_of(u));
    let listed: Vec<_> = checked
        .findings
        .iter()
        .filter(|f| with_exceptions || f.state != TopologyFindingState::Exception)
        .collect();
    let d = ctx.units.length_decimals;
    let rows: Vec<Vec<String>> = listed
        .iter()
        .map(|f| {
            let objects: Vec<String> =
                f.uids.iter().map(|u| slot_of(u).map_or_else(|| u.clone(), |s| format!("#{}", s.0))).collect();
            vec![
                f.rule.clone(),
                problem_label(&f.problem).to_owned(),
                state_word(f.state).to_owned(),
                objects.join(", "),
                to_fixed(f.at.x, d),
                to_fixed(f.at.y, d),
                measure_text(f.measure, f.measure_kind.as_deref(), ctx),
                f.reason.clone().unwrap_or_default(),
            ]
        })
        .collect();
    let mut ids: Vec<u32> = Vec::new();
    for f in &listed {
        for s in f.uids.iter().filter_map(|u| slot_of(u)) {
            if !ids.contains(&s.0) {
                ids.push(s.0);
            }
        }
    }
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for f in &checked.findings {
        *counts.entry(state_word(f.state)).or_default() += 1;
    }
    let said: Vec<String> = ["Değerlendirilmedi", "Hata", "İstisna", "Yeniden değerlendir"]
        .iter()
        .filter_map(|w| counts.get(w).map(|n| format!("{n} {}", w.to_lowercase())))
        .collect();
    let summary = if checked.findings.is_empty() {
        format!("{} kural {} nesnede bulgu vermedi.", checked.rules, checked.objects)
    } else {
        format!(
            "{} kural {} nesnede {} bulgu verdi ({}); bulgulu nesneler seçildi.",
            checked.rules,
            checked.objects,
            checked.total,
            said.join(", ")
        )
    };
    let outputs: serde_json::Map<String, Value> = [
        ("findings".to_owned(), json!({ "columns": COLUMNS, "rows": rows })),
        ("objects".to_owned(), json!(ids)),
        ("count".to_owned(), json!(checked.total)),
        ("open".to_owned(), json!(checked.open)),
    ]
    .into_iter()
    .collect();
    // The error layer (docs/adr/2200): each listed finding a point at its place, its fields the table's with the
    // objects' lasting ids and the finding's key, so a later check matches it.
    let errors: Vec<Entity> = if v.flag("write") {
        let layer = v.layer("layer");
        listed
            .iter()
            .map(|f| {
                let mut attrs = BTreeMap::new();
                attrs.insert("Kural".to_owned(), f.rule.clone());
                attrs.insert("Sorun".to_owned(), problem_label(&f.problem).to_owned());
                attrs.insert("Durum".to_owned(), state_word(f.state).to_owned());
                attrs.insert("Nesneler".to_owned(), f.uids.join(", "));
                let measure = measure_text(f.measure, f.measure_kind.as_deref(), ctx);
                if !measure.is_empty() {
                    attrs.insert("Ölçü".to_owned(), measure);
                }
                if let Some(r) = f.reason.as_ref().filter(|r| !r.is_empty()) {
                    attrs.insert("Gerekçe".to_owned(), r.clone());
                }
                attrs.insert("Anahtar".to_owned(), f.key.clone());
                Entity::Point(PointEntity {
                    base: EntityBase {
                        look: Default::default(),
                        id: 0,
                        layer_id: layer.id.clone(),
                        color: None,
                        attrs,
                        label: None,
                        symbol: None,
                        line_weight: None,
                        label_pins: Vec::new(),
                        attachments: Vec::new(),
                    },
                    p: f.at,
                    z: None,
                    parts: None,
                })
            })
            .collect()
    } else {
        Vec::new()
    };
    let wrote = errors.len();
    // Written: the findings' points are what the run leaves, not the objects' selection.
    let summary = if wrote > 0 {
        format!(
            "{} kural {} nesnede {} bulgu verdi ({}); {wrote} bulgu hata katmanına yazıldı.",
            checked.rules,
            checked.objects,
            checked.total,
            said.join(", ")
        )
    } else {
        summary
    };
    RunResult {
        select: (!ids.is_empty() && wrote == 0).then(|| ids.iter().map(|id| Slot(*id)).collect()),
        changes: (wrote > 0).then(|| crate::types::ChangeSet { add: errors, ..Default::default() }),
        outputs,
        summary: Some(summary),
        ..RunResult::default()
    }
}
