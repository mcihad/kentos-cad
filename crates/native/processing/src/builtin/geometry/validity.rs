//! Geçerliliği denetle and Onar (the web's `builtin/geometry/validity.ts`;
//! docs/adr/0201 §6): what is wrong with areas and paths as written, found by
//! the core (`ops::geoprocess::validity`) at its first place; and each area
//! rebuilt from its own rings in the core's overlay, each path without its
//! repeated vertices, written to the output layer with a report.

use kentos_domain::Slot;
use kentos_geometry_core::ops::geoprocess::calls::{kinds_of, validity_run};
use kentos_geometry_core::ops::geoprocess::validity::{Kind, Repaired, repair};
use serde_json::json;

use super::{
    VERTEX_KINDS, attrs_of, empty_note, features_param, geo_scopes, input_notes, layer_param,
    new_object, shapes,
};
use crate::text::to_fixed;
use crate::types::{
    ChangeSet, Feedback, OutputDef, OutputKind, Resolved, RunContext, RunResult, Target, Tool,
};

/// Geçerliliği denetle's table: the object, its layer, the problem and where it first shows.
const PROBLEM_COLUMNS: [&str; 5] = ["Nesne", "Katman", "Sorun", "Doğu", "Kuzey"];

pub fn validity() -> Tool {
    Tool {
        id: "geometry.validity".into(),
        label: "Geçerliliği denetle".into(),
        category: "geometry".into(),
        description: "Alan ve çizgilerin geometri sorunlarını bulup tabloda gösterir ve sorunlu nesneleri seçer; çizim değişmez.".into(),
        help: Some([
            "Bulunan sorunlar: yinelenen köşe (art arda aynı yerde iki köşe), alanı sıfır olan halka (köşeleri bir doğru üstünde), kendini kesen, kendine değen ya da kendi üstünden geri dönen halka ya da yol, dış halkanın dışına taşan delik, örtüşen delikler. Başladığı yerde biten yol kendini kesmiş sayılmaz.",
            "Her sorun her halkada bir kez, ilk göründüğü yerle yazılır; yer tablonun Doğu ve Kuzey sütunlarındadır. Aynı yerde sayılma payı 1 mikrometredir (çekirdeğin örtüşme toleransı); başka gizli yuvarlama yoktur.",
            "Sorunlu nesneleri yeni katmana düzeltilmiş olarak yazmak için Onar'ı kullanın.",
        ]
        .join("\n\n")),
        keywords: [
            "geçerlilik", "denetle", "geometri hatası", "kendini kesen", "yinelenen köşe",
            "check geometry", "validity", "topoloji",
        ]
        .map(String::from)
        .to_vec(),
        aliases: ["GECERLILIK", "CHECKGEOMETRY"].map(String::from).to_vec(),
        icon: Some("geoValidity".into()),
        parameters: vec![features_param(
            "input",
            "Nesneler",
            &VERTEX_KINDS,
            geo_scopes(),
            "Denetlenecek kapalı alanlar, çoklu çizgiler ve çizgiler.",
        )],
        outputs: vec![
            OutputDef::new("problems", "Sorunlar", OutputKind::Table),
            OutputDef::new("invalid", "Sorunlu nesneler", OutputKind::Features),
            OutputDef::new("count", "Sorun sayısı", OutputKind::Number),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run_validity),
    }
}

fn run_validity(v: &Resolved<'_>, ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let input = &v.features("input").entities;
    feedback.progress(0.0, "Geometriler denetleniyor");
    let found = validity_run(&shapes(input));
    if found.is_empty() {
        return RunResult {
            outputs: [
                ("count".to_owned(), json!(0)),
                ("invalid".to_owned(), json!([])),
            ]
            .into_iter()
            .collect(),
            summary: Some(format!("{} nesnede sorun bulunmadı.", input.len())),
            ..RunResult::default()
        };
    }
    let d = ctx.units.length_decimals;
    let rows: Vec<Vec<String>> = found
        .iter()
        .map(|f| {
            let e = input[f.index];
            vec![
                format!("#{}", e.base().id),
                ctx.layer_name(&e.base().layer_id).to_owned(),
                f.kind.text().to_owned(),
                to_fixed(f.at.x, d),
                to_fixed(f.at.y, d),
            ]
        })
        .collect();
    let mut ids: Vec<u32> = Vec::new();
    for f in &found {
        let id = input[f.index].base().id;
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    let summary = format!(
        "{} nesneden {} nesnede {} sorun bulundu; sorunlu nesneler seçildi.",
        input.len(),
        ids.len(),
        found.len()
    );
    RunResult {
        select: Some(ids.iter().map(|id| Slot(*id)).collect()),
        outputs: [
            (
                "problems".to_owned(),
                json!({ "columns": PROBLEM_COLUMNS, "rows": rows }),
            ),
            ("invalid".to_owned(), json!(ids)),
            ("count".to_owned(), json!(found.len())),
        ]
        .into_iter()
        .collect(),
        summary: Some(summary),
        ..RunResult::default()
    }
}

/// Onar's report: the object, how it was and how it is, and what was wrong.
const REPAIR_COLUMNS: [&str; 4] = ["Nesne", "Önce", "Sonra", "Değişiklik"];

/// An object's state before (0) or after (1) as the report writes it.
fn state(r: &Repaired, after: bool, area_decimals: u32) -> String {
    if after && r.shape.is_none() {
        return "Boş".to_owned();
    }
    let pick = |pair: (usize, usize)| if after { pair.1 } else { pair.0 };
    match r.area {
        Some((a0, a1)) => format!(
            "{} parça, {} delik, {} m²",
            pick(r.parts),
            pick(r.holes),
            to_fixed(if after { a1 } else { a0 }, area_decimals)
        ),
        None => format!("{} köşe", pick(r.vertices)),
    }
}

pub fn repair_tool() -> Tool {
    Tool {
        id: "geometry.repair".into(),
        label: "Onar".into(),
        category: "geometry".into(),
        description: "Alan ve çizgilerin geometri sorunlarını düzeltip nesneleri öznitelikleriyle yeni katmana yazar; neyin değiştiğini tabloda gösterir.".into(),
        help: Some([
            "Sorunlu alan kendi halkalarından yeniden kurulur: kendini kesen halka parçalarına ayrılır, dış halkadan taşan delik halkayla kesilir, örtüşen delikler tek delik olur, yinelenen köşeler düşer; alanı kalmayan halka yazılmaz. Çizgilerde yinelenen köşeler düşer; kendini kesen yol onarılmaz, olduğu gibi yazılır.",
            "Sorunu olmayan nesne olduğu gibi yazılır. Tablo her sorunlu nesnenin önceki ve sonraki parça, delik ve alan sayılarını (çizgide köşe sayısını) ve sorunlarını gösterir. Girdi değişmez.",
        ]
        .join("\n\n")),
        keywords: [
            "onar", "düzelt", "geometri hatası", "repair geometry", "fix geometries",
            "kendini kesen", "temizle",
        ]
        .map(String::from)
        .to_vec(),
        aliases: ["ONAR", "REPAIRGEOMETRY"].map(String::from).to_vec(),
        icon: Some("geoRepair".into()),
        parameters: vec![
            features_param(
                "input",
                "Nesneler",
                &VERTEX_KINDS,
                geo_scopes(),
                "Onarılacak kapalı alanlar, çoklu çizgiler ve çizgiler.",
            ),
            layer_param("Onarılan", "#12A594"),
        ],
        outputs: vec![
            OutputDef::new("repaired", "Yazılan nesneler", OutputKind::Features),
            OutputDef::new("report", "Onarım raporu", OutputKind::Table),
            OutputDef::new("count", "Onarılan nesne sayısı", OutputKind::Number),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run_repair),
    }
}

fn run_repair(v: &Resolved<'_>, ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let input = &v.features("input").entities;
    input_notes(feedback, input, &[], false);
    feedback.progress(0.0, "Geometriler onarılıyor");
    let layer = &v.layer("layer").id;
    let mut add = Vec::new();
    let mut rows: Vec<Vec<String>> = Vec::new();
    let (mut repaired, mut crossing) = (0usize, 0usize);
    let decimals = ctx.units.area_decimals;
    for (e, s) in input.iter().zip(shapes(input)) {
        let r = repair(&s);
        let kinds = kinds_of(&s);
        if !kinds.is_empty() {
            if kinds.iter().any(|k| *k != Kind::PathCrossing) {
                repaired += 1;
            }
            if kinds.contains(&Kind::PathCrossing) {
                crossing += 1;
            }
            let change: Vec<String> = kinds
                .iter()
                .map(|k| match k {
                    Kind::PathCrossing => format!("{} (onarılmaz)", k.text()),
                    _ => k.text().to_owned(),
                })
                .collect();
            rows.push(vec![
                format!("#{}", e.base().id),
                state(&r, false, decimals),
                state(&r, true, decimals),
                change.join(", "),
            ]);
        }
        if let Some(shape) = r.shape
            && let Some(object) = new_object(shape, layer, attrs_of(e))
        {
            add.push(object);
        }
    }
    if crossing > 0 {
        feedback.warn(format!(
            "{crossing} kendini kesen yol onarılmadı; olduğu gibi yazıldı."
        ));
    }
    empty_note(input.len() - add.len(), feedback);
    let count = add.len();
    let mut outputs: crate::types::Values = [("count".to_owned(), json!(repaired))]
        .into_iter()
        .collect();
    if !rows.is_empty() {
        outputs.insert(
            "report".to_owned(),
            json!({ "columns": REPAIR_COLUMNS, "rows": rows }),
        );
    }
    RunResult {
        changes: Some(ChangeSet {
            add,
            ..ChangeSet::default()
        }),
        outputs,
        summary: Some(format!("{count} nesne yazıldı; {repaired} nesne onarıldı.")),
        ..RunResult::default()
    }
}
