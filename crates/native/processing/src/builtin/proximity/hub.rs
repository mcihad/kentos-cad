//! En yakın merkeze bağla (the web's `builtin/proximity/hub.ts`; docs/adr/0215
//! §3.3; QGIS "Distance to nearest hub (line to hub)"): each object's centre
//! joined to the nearest hub's by a line on the output layer, the line
//! carrying the object's attributes, the hub's name (Merkez) and the distance
//! (Uzaklık). Centre to centre, by the run's store (`Store::nearest`).

use kentos_contracts::{Entity, EntityGeometry};
use kentos_domain::Slot;
use serde_json::json;

use super::{PROXIMITY_KINDS, bound, features, field, length_text, max_param, name_of};
use crate::builtin::geometry::{attrs_of, edit_object, layer_param};
use crate::geometry::Measure;
use crate::types::{
    ChangeSet, Feedback, OutputDef, OutputKind, Resolved, RunContext, RunResult, Target, Tool,
};

pub fn tool() -> Tool {
    Tool {
        id: "proximity.hub".into(),
        label: "En yakın merkeze bağla".into(),
        category: "proximity".into(),
        description: "Her nesnenin merkezini en yakın merkezin merkezine bir çizgiyle bağlar; çizgi merkezin adını ve uzaklığı taşır.".into(),
        help: Some([
            "Örnekler: her mahalleyi en yakın sağlık ocağına bağlayan çizgiler; her kuyudan en yakın depoya; her parselden en yakın okula.",
            "Uzaklık merkezden merkezedir: alanın ağırlık merkezi, öbür nesnelerin yer noktası. Eşit uzaklıkta çizimde önce gelen merkez alınır. Çizgi nesnenin özniteliklerini, “Merkez”i (merkezin Ad alanındaki değeri; boşsa etiketi, o da yoksa sırası) ve “Uzaklık”ı taşır.",
            "En çok uzaklık içinde merkezi olmayan nesne bağlanmaz ve sayısı söylenir; merkezi merkezle aynı yerde olan nesneye çizgi çizilmez.",
        ]
        .join("\n\n")),
        keywords: [
            "en yakın merkez", "hub", "nearest hub", "bağla", "örümcek diyagramı", "spider", "yakınlık", "tesis",
        ]
        .map(String::from)
        .to_vec(),
        aliases: ["MERKEZEBAGLA", "ENYAKINMERKEZ"].map(String::from).to_vec(),
        icon: Some("nearestHub".into()),
        parameters: vec![
            features(
                "input",
                "Nesneler",
                &PROXIMITY_KINDS,
                false,
                "Merkeze bağlanacak nesneler.",
            ),
            features(
                "hubs",
                "Merkezler",
                &PROXIMITY_KINDS,
                false,
                "Okullar, sağlık ocakları, depolar …",
            ),
            field("hubName", "Merkez ad alanı", "hubs", false, false)
                .optional()
                .describe("Çizgilere “Merkez” olarak yazılır; boşsa etiket, o da yoksa sıra."),
            max_param().describe("0: sınırsız."),
            layer_param("Merkeze bağlantılar", "#E5732E"),
        ],
        outputs: vec![
            OutputDef::new("lines", "Bağlantılar", OutputKind::Features),
            OutputDef::new("count", "Bağlantı sayısı", OutputKind::Number),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run),
    }
}

fn run(v: &Resolved<'_>, ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let inputs = &v.features("input").entities;
    let hubs = &v.features("hubs").entities;
    let max = v.number("max").unwrap_or(0.0);
    feedback.progress(0.0, "En yakın merkezler aranıyor");
    let ids = |list: &[&Entity]| -> Vec<Slot> { list.iter().map(|e| Slot(e.base().id)).collect() };
    let found = ctx
        .geometry
        .nearest(&ids(inputs), &ids(hubs), 1, bound(max), Measure::Centers);
    let layer = &v.layer("layer").id;
    let mut same = 0usize;
    let mut add = Vec::new();
    for f in &found {
        if f.d == 0.0 {
            same += 1;
            continue;
        }
        let mut attrs = attrs_of(inputs[f.input]);
        attrs.insert(
            "Merkez".to_owned(),
            name_of(hubs[f.target], v.text("hubName"), f.target),
        );
        attrs.insert("Uzaklık".to_owned(), length_text(ctx.units, f.d));
        let line = EntityGeometry::Line {
            a: f.a,
            b: f.b,
            zs: None,
        };
        if let Some(e) = edit_object(&line, layer, attrs) {
            add.push(e);
        }
    }
    let none = inputs.len() - found.len();
    if none > 0 {
        let within = if max > 0.0 {
            "en çok uzaklık içinde "
        } else {
            ""
        };
        feedback.info(format!("{none} nesnenin {within}merkezi yok; bağlanmadı."));
    }
    if same > 0 {
        feedback.info(format!(
            "{same} nesnenin merkezi merkezle aynı yerde; çizgi çizilmedi."
        ));
    }
    let count = add.len();
    RunResult {
        changes: Some(ChangeSet {
            add,
            ..ChangeSet::default()
        }),
        outputs: [("count".to_owned(), json!(count))].into_iter().collect(),
        summary: Some(format!("{count} nesne en yakın merkeze bağlandı.")),
        ..RunResult::default()
    }
}
