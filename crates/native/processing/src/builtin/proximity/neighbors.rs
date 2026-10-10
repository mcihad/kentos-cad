//! Komşu alanlar (the web's `builtin/proximity/neighbors.ts`; docs/adr/0215
//! §3.4; ArcGIS Polygon Neighbors): each area's neighbours, both ways round,
//! with the length of boundary they share within the tolerance, those that
//! only meet at a corner (if asked) and those whose insides overlap (if
//! asked) with the overlapping area. A table; if asked, each area's
//! neighbour count and names written to its attributes in one undo step. The
//! pairs are the run's store's (`Store::neighbors`).

use kentos_contracts::Entity;
use kentos_domain::Slot;
use serde_json::{Value, json};

use super::{NEIGHBOR_KINDS, area_text, features, field, length_text, name_of, number};
use crate::builtin::queries::with_attr;
use crate::geometry::NeighborKind;
use crate::types::{
    ChangeSet, Feedback, OutputDef, OutputKind, ParamDef, ParamKind, Patch, Resolved, RunContext,
    RunResult, Target, Tool, Values,
};

/// How two areas neighbour, as the table names it.
pub fn neighbor_label(kind: NeighborKind) -> &'static str {
    match kind {
        NeighborKind::Edge => "Kenar",
        NeighborKind::Corner => "Köşe",
        NeighborKind::Overlap => "Örtüşme",
    }
}

fn writing(v: &Values) -> bool {
    v.get("write").and_then(Value::as_bool) == Some(true)
}

fn flag(name: &str, label: &str, default: bool, description: &str) -> ParamDef {
    ParamDef::new(name, label, ParamKind::Boolean)
        .default_value(json!(default))
        .describe(description)
}

pub fn tool() -> Tool {
    Tool {
        id: "proximity.neighbors".into(),
        label: "Komşu alanlar".into(),
        category: "proximity".into(),
        description: "Alanların komşularını, ortak kenar uzunluklarını, köşeden değenleri ve örtüşenleri tablo olarak verir; isterseniz komşu sayısını ve adlarını yazar.".into(),
        help: Some([
            "Örnekler: bir adanın parsellerinin komşuları ve ortak sınır uzunlukları; ifraz öncesi komşu parsellerin listesi; örtüşen parsellerin bulunması.",
            "İki alanın ortak kenarı sınırlarının birbirinin üstünde kalan parçalarının uzunluğudur: aynı doğrudaki düz kenarlar ve aynı dairedeki yaylar, toleransla. Ortak kenarı toleranstan kısa olup değenler köşe komşusudur; içleri örtüşenler Örtüşme olarak örtüşen alanlarıyla yazılır.",
            "Satırlar iki yönlüdür (A–B ve B–A), girdinin sırasıyla. Adlar Ad alanından; boşsa etiket, o da yoksa sıra. Özniteliğe de yaz açıkken her alana komşu sayısı ve komşularının adları yazılır.",
        ]
        .join("\n\n")),
        keywords: [
            "komşu", "komşu parsel", "ortak kenar", "ortak sınır", "polygon neighbors", "bitişik", "örtüşme", "yakınlık",
        ]
        .map(String::from)
        .to_vec(),
        aliases: ["KOMSU", "KOMSUPARSEL"].map(String::from).to_vec(),
        icon: Some("polygonNeighbors".into()),
        parameters: vec![
            features(
                "input",
                "Alanlar",
                &NEIGHBOR_KINDS,
                true,
                "Komşulukları aranan alanlar; kilitli katmandakiler alınmaz.",
            ),
            number("tolerance", "Tolerans", Some(0.0), Some(1.0), false, "m")
                .default_value(json!(0.001))
                .describe("Bu kadar yakın sınırlar ortak sayılır."),
            flag(
                "corners",
                "Köşe komşuları da",
                false,
                "Yalnız köşeden değenler de yazılır.",
            ),
            flag(
                "overlaps",
                "Örtüşenler de",
                true,
                "İçleri örtüşen alanlar örtüşen alanlarıyla yazılır.",
            ),
            field("name", "Ad alanı", "input", false, false)
                .optional()
                .describe("Alanların tablodaki adı; boşsa etiket, o da yoksa sıra."),
            flag(
                "write",
                "Özniteliğe de yaz",
                false,
                "Her alana komşu sayısı ve komşularının adları.",
            ),
            field("countField", "Komşu sayısı alanı", "input", true, false)
                .default_value(json!("Komşu sayısı"))
                .shown_when(writing),
            field("listField", "Komşular alanı", "input", true, false)
                .default_value(json!("Komşular"))
                .shown_when(writing),
        ],
        outputs: vec![
            OutputDef::new("table", "Komşuluk tablosu", OutputKind::Table),
            OutputDef::new("count", "Komşuluk sayısı", OutputKind::Number),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run),
    }
}

fn run(v: &Resolved<'_>, ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let areas = &v.features("input").entities;
    feedback.progress(0.0, "Komşular aranıyor");
    let slots: Vec<Slot> = areas.iter().map(|e| Slot(e.base().id)).collect();
    let found = ctx.geometry.neighbors(
        &slots,
        v.number("tolerance").unwrap_or(0.001),
        v.flag("corners"),
        v.flag("overlaps"),
    );
    let name = |i: usize| name_of(areas[i], v.text("name"), i);
    let rows: Vec<Vec<String>> = found
        .iter()
        .map(|f| {
            vec![
                name(f.area),
                name(f.neighbor),
                neighbor_label(f.kind).to_owned(),
                length_text(ctx.units, f.length),
                if f.kind == NeighborKind::Overlap {
                    area_text(ctx.units, f.overlap)
                } else {
                    String::new()
                },
            ]
        })
        .collect();
    let write = v.flag("write");
    let mut update = Vec::new();
    if write {
        let mut lists: Vec<Vec<String>> = vec![Vec::new(); areas.len()];
        for f in &found {
            lists[f.area].push(name(f.neighbor));
        }
        for (e, list) in areas.iter().zip(&lists) {
            let mut current: Entity = (*e).clone();
            let mut changed = false;
            let mut put = |field: &str, text: Option<String>| {
                if let Some(attrs) = with_attr(ctx, &current, field, text.as_deref()) {
                    current.base_mut().attrs = attrs;
                    changed = true;
                }
            };
            put(v.text("countField"), Some(list.len().to_string()));
            put(
                v.text("listField"),
                (!list.is_empty()).then(|| list.join(", ")),
            );
            if changed {
                update.push(Patch {
                    id: Slot(e.base().id),
                    attrs: Some(current.base().attrs.clone()),
                    label: None,
                    zs: None,
                });
            }
        }
    }
    let overlapping = found
        .iter()
        .filter(|f| f.kind == NeighborKind::Overlap)
        .count()
        / 2;
    if overlapping > 0 {
        feedback.warn(format!("{overlapping} alan çiftinin içleri örtüşüyor."));
    }
    RunResult {
        changes: write.then(|| ChangeSet {
            update,
            ..ChangeSet::default()
        }),
        outputs: [
            (
                "table".to_owned(),
                json!({
                    "columns": ["Alan", "Komşu", "Komşuluk", "Ortak kenar", "Örtüşen alan"],
                    "rows": rows,
                }),
            ),
            ("count".to_owned(), json!(found.len())),
        ]
        .into_iter()
        .collect(),
        summary: Some(format!(
            "{} alanda {} komşuluk bulundu.",
            areas.len(),
            found.len() / 2
        )),
        ..RunResult::default()
    }
}
