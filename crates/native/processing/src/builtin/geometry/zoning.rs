//! İmar planını denetle (docs/adr/3840; plan/map-cad.md MAP-09): the plan's check from İşlem araç kutusu, a model or a
//! script, read only. The check is `cad.zoning.check`'s own (`zoning_check::with_regions`): the same findings, words,
//! areas and the regions a road's corridor takes from a decision or a parcel (docs/adr/3440). The findings go to a
//! table, their objects are selected, the count is a number. Asked to, each region of a finding is written as an area
//! on an error layer, one undo step, as Topolojiyi kurallarla denetle writes its points (docs/adr/2200); the findings
//! without a region (a missing value, a note …) have no place and stay in the table. No legal conformity is claimed.

use std::collections::BTreeMap;

use kentos_commands::{Entity, EntityBase, PathEntity, ZoningCheck};
use kentos_domain::{Slot, Uuid};
use kentos_native_application::zoning_check::{kind_word, with_regions};
use serde_json::{Value, json};

use crate::text::to_fixed;
use crate::types::{
    Feedback, NewLayerStyle, OutputDef, OutputKind, ParamDef, ParamKind, Resolved, RunContext, RunResult, Target,
    Tool,
};

/// The findings' table.
const COLUMNS: [&str; 5] = ["Tür", "Nesneler", "Alan", "Bölge", "İleti"];

pub fn tool() -> Tool {
    Tool {
        id: "zoning.check".into(),
        label: "İmar planını denetle".into(),
        category: "geometry".into(),
        description: "İmar planının katmanlarını İmar denetimi'nin kurallarıyla denetler; bulguları tabloda gösterir ve bulgulu nesneleri seçer; istenirse yola giren bölgeleri hata katmanına alan olarak yazar.".into(),
        help: Some([
            "Denetim İmar denetimi penceresinin ve cad.zoning.check komutunun denetimidir: aynı bulgular, aynı sözler ve alanlar. Planın katmanları imar-plani profilindeki gibidir. Resmî şema ya da mevzuata uygunluk iddia edilmez.",
            "Tolerans bindirme ve boşluk parçalarının en dar kalınlığıdır: daha ince parça sayılmaz.",
            "Bulguları hata katmanına yaz açıksa yola giren karar ve yola giren parsel bulgularının her bölgesi hata katmanına alan olarak yazılır; alanın öznitelikleri tür, nesnelerin kalıcı kimlikleri, bulgunun alanı, bölgenin sırası ve iletidir. Yeri olmayan bulgular (değer verilmemiş, plan notu yok …) tabloda kalır. Yazım tek geri alma adımıdır; kapalıyken çizim değişmez.",
        ]
        .join("\n\n")),
        keywords: ["imar", "plan", "denetle", "karar", "yol", "parsel", "bulgu", "zoning", "plan check"]
            .map(String::from)
            .to_vec(),
        aliases: ["IMARDENETLE", "ZONINGCHECK"].map(String::from).to_vec(),
        icon: Some("zoningCheck".into()),
        parameters: vec![
            ParamDef::new(
                "tolerance",
                "Tolerans",
                ParamKind::Number { min: Some(0.000001), max: None, integer: false, unit: "m".into(), placeholder: None },
            )
            .default_value(json!(0.01))
            .describe("Bindirme ve boşluk parçalarının en dar kalınlığı."),
            ParamDef::new("write", "Bulguları hata katmanına yaz", ParamKind::Boolean)
                .default_value(json!(false))
                .describe("Açıksa yola giren bölgeler hata katmanına alan olarak yazılır (tek geri alma adımı)."),
            ParamDef::new(
                "layer",
                "Hata katmanı",
                ParamKind::Layer {
                    new_layer_style: NewLayerStyle {
                        color: Some("#E5484D".into()),
                        line_weight: Some(0.35),
                        // A faint fill, as the other tools' areas: what lies under it still reads.
                        fill: Some("#E5484D26".into()),
                        ..NewLayerStyle::default()
                    },
                    above: None,
                    below: None,
                },
            )
            .default_value(json!({ "newName": "İmar bulguları" }))
            .describe("Bu adda katman yoksa oluşturulur.")
            .shown_when(|v| v.get("write").and_then(Value::as_bool) == Some(true)),
        ],
        outputs: vec![
            OutputDef::new("findings", "Bulgular", OutputKind::Table),
            OutputDef::new("objects", "Bulgulu nesneler", OutputKind::Features),
            OutputDef::new("count", "Bulgu sayısı", OutputKind::Number),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run),
    }
}

fn run(v: &Resolved<'_>, ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    feedback.progress(0.0, "İmar planı denetleniyor");
    let input = ZoningCheck { tolerance: v.number("tolerance") };
    let checked = match with_regions(ctx.doc, &input) {
        Ok((checked, _)) => checked,
        Err(why) => return RunResult { refused: Some(why), ..RunResult::default() },
    };
    let slot_of = |uid: &str| Uuid::parse_str(uid).ok().and_then(|u| ctx.doc.slot_of(u));
    let area_text = |a: Option<f64>| a.map_or(String::new(), |a| format!("{} m²", to_fixed(a, ctx.units.area_decimals)));
    let rows: Vec<Vec<String>> = checked
        .findings
        .iter()
        .map(|f| {
            let objects: Vec<String> =
                f.uids.iter().map(|u| slot_of(u).map_or_else(|| u.clone(), |s| format!("#{}", s.0))).collect();
            vec![
                kind_word(&f.kind).to_owned(),
                objects.join(", "),
                area_text(f.area),
                if f.regions.is_empty() { String::new() } else { f.regions.len().to_string() },
                f.message.clone(),
            ]
        })
        .collect();
    let mut ids: Vec<u32> = checked.findings.iter().flat_map(|f| f.uids.iter()).filter_map(|u| slot_of(u)).map(|s| s.0).collect();
    ids.sort_unstable();
    ids.dedup();
    let n = checked.findings.len();
    let outputs: serde_json::Map<String, Value> = [
        ("findings".to_owned(), json!({ "columns": COLUMNS, "rows": rows })),
        ("objects".to_owned(), json!(ids)),
        ("count".to_owned(), json!(n)),
    ]
    .into_iter()
    .collect();
    // The error layer: each region of a finding an area, its fields the table's with the objects' lasting ids.
    let errors: Vec<Entity> = if v.flag("write") {
        let layer = v.layer("layer");
        checked
            .findings
            .iter()
            .flat_map(|f| {
                let count = f.regions.len();
                f.regions.iter().enumerate().map(move |(k, r)| (f, k, count, r))
            })
            .map(|(f, k, count, r)| {
                let mut attrs = BTreeMap::new();
                attrs.insert("Tür".to_owned(), kind_word(&f.kind).to_owned());
                attrs.insert("Nesneler".to_owned(), f.uids.join(", "));
                if let Some(a) = f.area {
                    attrs.insert("Bulgu alanı".to_owned(), area_text(Some(a)));
                }
                attrs.insert("Bölge".to_owned(), format!("{}/{count}", k + 1));
                attrs.insert("İleti".to_owned(), f.message.clone());
                Entity::Polygon(PathEntity {
                    space_separator: None,
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
                    pts: r.outer.pts.clone(),
                    bulges: r.outer.bulges.clone(),
                    holes: (!r.holes.is_empty()).then(|| r.holes.clone()),
                    zs: None,
                    ms: None,
                    parts: None,
                    closed: false,
                    widths: None,
                })
            })
            .collect()
    } else {
        Vec::new()
    };
    let wrote = errors.len();
    let with_place = checked.findings.iter().filter(|f| !f.regions.is_empty()).count();
    let said = if n == 0 {
        format!("İmar denetimi {} planda, {} karar alanında bulgu vermedi.", checked.plans, checked.decisions)
    } else {
        format!("İmar denetimi {} planda, {} karar alanında {n} bulgu verdi", checked.plans, checked.decisions)
    };
    let summary = match (n, wrote) {
        (0, _) => said,
        (_, 0) => format!("{said}; bulgulu nesneler seçildi."),
        _ if with_place < n => format!(
            "{said}; {with_place} bulgunun {wrote} bölgesi hata katmanına yazıldı, {} bulgunun yeri yok (tabloda).",
            n - with_place
        ),
        _ => format!("{said}; {with_place} bulgunun {wrote} bölgesi hata katmanına yazıldı."),
    };
    RunResult {
        select: (!ids.is_empty() && wrote == 0).then(|| ids.iter().map(|id| Slot(*id)).collect()),
        changes: (wrote > 0).then(|| crate::types::ChangeSet { add: errors, ..Default::default() }),
        outputs,
        summary: Some(summary),
        ..RunResult::default()
    }
}
