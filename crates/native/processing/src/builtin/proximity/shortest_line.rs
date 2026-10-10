//! En kısa çizgi (the web's `builtin/proximity/shortestLine.ts`; docs/adr/0215
//! §3.5; QGIS "Shortest line between features"): from each object to its
//! nearest `k` targets, the segment between their nearest points on the
//! output layer, with Kaynak, Hedef, Sıra and Uzaklık. Pairs that meet (0
//! apart) give none and are counted. The search is the run's store's
//! (`Store::nearest`, edge to edge).

use std::collections::BTreeMap;

use kentos_contracts::{Entity, EntityGeometry};
use kentos_domain::Slot;
use serde_json::json;

use super::{PROXIMITY_KINDS, bound, features, field, length_text, max_param, name_of, number};
use crate::builtin::geometry::{edit_object, layer_param};
use crate::geometry::Measure;
use crate::types::{
    ChangeSet, Feedback, OutputDef, OutputKind, Resolved, RunContext, RunResult, Target, Tool,
};

pub fn tool() -> Tool {
    Tool {
        id: "proximity.shortestLine".into(),
        label: "En kısa çizgi".into(),
        category: "proximity".into(),
        description: "Her nesneden en yakın k hedefe en yakın noktalar arası doğru parçaları çizer.".into(),
        help: Some([
            "Örnekler: her yapıdan en yakın yola bağlantı çizgisi; her kuyudan en yakın iki dereye; iki katman arası en kısa bağlantılar.",
            "Çizgi iki nesnenin en yakın noktaları arasındadır (kenardan kenara); eşit uzaklıkta çizimde önce gelen hedef alınır. Çizgiye “Kaynak”, “Hedef” (Ad alanlarından; boşsa etiket, o da yoksa sıra), “Sıra” ve “Uzaklık” yazılır.",
            "Değen ya da kesişen çiftler (uzaklık 0) çizgi vermez; sayıları söylenir.",
        ]
        .join("\n\n")),
        keywords: [
            "en kısa çizgi", "shortest line", "bağlantı", "en yakın", "yakınlık", "dik bağlantı",
        ]
        .map(String::from)
        .to_vec(),
        aliases: ["ENKISACIZGI", "KISACIZGI"].map(String::from).to_vec(),
        icon: Some("shortestLine".into()),
        parameters: vec![
            features(
                "input",
                "Nesneler",
                &PROXIMITY_KINDS,
                false,
                "Çizgilerin başladığı nesneler.",
            ),
            features(
                "targets",
                "Hedefler",
                &PROXIMITY_KINDS,
                false,
                "Çizgilerin vardığı nesneler.",
            ),
            number("k", "En yakın k", Some(1.0), Some(100.0), true, "")
                .default_value(json!(1))
                .describe("Her nesneden kaç hedefe."),
            max_param().describe("0: sınırsız."),
            field("name", "Ad alanı", "input", false, false)
                .optional()
                .describe("Çizgilere “Kaynak” olarak yazılır."),
            field("targetName", "Hedef ad alanı", "targets", false, false)
                .optional()
                .describe("Çizgilere “Hedef” olarak yazılır."),
            layer_param("En kısa çizgiler", "#8A3FFC"),
        ],
        outputs: vec![
            OutputDef::new("lines", "Çizgiler", OutputKind::Features),
            OutputDef::new("count", "Çizgi sayısı", OutputKind::Number),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run),
    }
}

fn run(v: &Resolved<'_>, ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let inputs = &v.features("input").entities;
    let targets = &v.features("targets").entities;
    let k = v.number("k").unwrap_or(1.0).max(1.0) as usize;
    feedback.progress(0.0, "En yakın noktalar aranıyor");
    let ids = |list: &[&Entity]| -> Vec<Slot> { list.iter().map(|e| Slot(e.base().id)).collect() };
    let found = ctx.geometry.nearest(
        &ids(inputs),
        &ids(targets),
        k,
        bound(v.number("max").unwrap_or(0.0)),
        Measure::Edges,
    );
    let layer = &v.layer("layer").id;
    let mut meeting = 0usize;
    let mut last = usize::MAX;
    let mut rank = 0usize;
    let mut add = Vec::new();
    for f in &found {
        rank = if f.input == last { rank + 1 } else { 1 };
        last = f.input;
        if f.d == 0.0 {
            meeting += 1;
            continue;
        }
        let attrs = BTreeMap::from([
            (
                "Kaynak".to_owned(),
                name_of(inputs[f.input], v.text("name"), f.input),
            ),
            (
                "Hedef".to_owned(),
                name_of(targets[f.target], v.text("targetName"), f.target),
            ),
            ("Sıra".to_owned(), rank.to_string()),
            ("Uzaklık".to_owned(), length_text(ctx.units, f.d)),
        ]);
        let line = EntityGeometry::Line {
            a: f.a,
            b: f.b,
            zs: None,
        };
        if let Some(e) = edit_object(&line, layer, attrs) {
            add.push(e);
        }
    }
    if meeting > 0 {
        feedback.info(format!(
            "{meeting} çift değiyor ya da kesişiyor (uzaklık 0); çizgi çizilmedi."
        ));
    }
    let count = add.len();
    RunResult {
        changes: Some(ChangeSet {
            add,
            ..ChangeSet::default()
        }),
        outputs: [("count".to_owned(), json!(count))].into_iter().collect(),
        summary: Some(format!("{count} en kısa çizgi yazıldı.")),
        ..RunResult::default()
    }
}
