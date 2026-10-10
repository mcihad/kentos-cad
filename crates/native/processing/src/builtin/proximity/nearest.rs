//! En yakını bul (the web's `builtin/proximity/nearest.ts`; docs/adr/0215
//! §3.1; ArcGIS Near, QGIS "Join attributes by nearest"): each object gets
//! its nearest target's distance, the target's chosen fields and, if asked,
//! the bearing between them, under a prefix. The search is the run's store's
//! (`Store::nearest`, edge to edge or centre to centre); one undo step.

use std::collections::HashMap;

use kentos_contracts::Entity;
use kentos_domain::Slot;
use serde_json::{Value, json};

use super::{
    PROXIMITY_KINDS, bearing_text, bound, features, field, length_text, max_param, measure_of,
    measure_param,
};
use crate::builtin::queries::{attr, with_attr};
use crate::geometry::NearestFound;
use crate::parameters::field_names;
use crate::types::{
    ChangeSet, Feedback, OutputDef, OutputKind, ParamDef, ParamKind, Patch, Resolved, RunContext,
    RunResult, Target, Tool, Values,
};

pub fn tool() -> Tool {
    Tool {
        id: "proximity.nearest".into(),
        label: "En yakını bul".into(),
        category: "proximity".into(),
        description: "Her nesneye en yakın hedefin uzaklığını, istenen alanlarını ve semtini yazar.".into(),
        help: Some([
            "Örnekler: her parsele en yakın durağın adı ve uzaklığı; her yapıya en yakın yangın musluğu; her kuyuya en yakın derenin uzaklığı.",
            "Kenardan kenara ölçü en kısa uzaklıktır: değen, kesişen ya da hedefin içinde kalan nesne için 0. Merkezden merkeze ölçü alanların ağırlık merkezleri, öbür nesnelerin yer noktaları arasıdır. Eşit uzaklıkta çizimde önce gelen hedef alınır; nesne kendisinin hedefi olmaz.",
            "Yazılan alanlar önekle adlanır: “<önek>uzaklık”, alınan her alan için “<önek><alan>”, istenirse “<önek>semt” (projenin açı biriminde, kuzeyden saat yönünde). En çok uzaklık içinde hedefi olmayan nesnenin bu alanları boşaltılır.",
        ]
        .join("\n\n")),
        keywords: [
            "en yakın", "yakınlık", "uzaklık", "near", "nearest", "join by nearest", "komşu", "mesafe",
        ]
        .map(String::from)
        .to_vec(),
        aliases: ["ENYAKIN", "YAKINBUL", "NEAR"].map(String::from).to_vec(),
        icon: Some("nearestFeature".into()),
        parameters: vec![
            features(
                "input",
                "Nesneler",
                &PROXIMITY_KINDS,
                true,
                "Yakınlık bilgisinin yazılacağı nesneler; kilitli katmandakiler alınmaz.",
            ),
            features(
                "targets",
                "Hedefler",
                &PROXIMITY_KINDS,
                false,
                "En yakını aranan nesneler.",
            ),
            measure_param(),
            max_param(),
            field("fields", "Alınacak alanlar", "targets", false, true)
                .optional()
                .describe("Hedefin bu alanları önekle yazılır; boşsa yalnız uzaklık."),
            ParamDef::new(
                "prefix",
                "Önek",
                ParamKind::Text {
                    placeholder: None,
                    max_length: Some(24),
                    allow_empty: true,
                },
            )
            .default_value(json!("Yakın "))
            .describe("Yazılan alanların adlarının başı: “Yakın uzaklık”, “Yakın Ad”."),
            ParamDef::new("bearing", "Semt de yaz", ParamKind::Boolean)
                .default_value(json!(false))
                .describe("En yakın noktalar (merkezden merkeze ölçüde merkezler) arası semt."),
        ],
        outputs: vec![
            OutputDef::new("changed", "Değişen nesneler", OutputKind::Features),
            OutputDef::new("count", "Hedefi bulunan nesne sayısı", OutputKind::Number),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: Some(preview),
        run: Some(run),
    }
}

fn preview(v: &Values) -> Option<String> {
    let prefix = v.get("prefix").and_then(Value::as_str).unwrap_or("");
    Some(format!("“{prefix}uzaklık” alanına yazılacak"))
}

fn run(v: &Resolved<'_>, ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let inputs = &v.features("input").entities;
    let targets = &v.features("targets").entities;
    let max = v.number("max").unwrap_or(0.0);
    feedback.progress(0.0, "En yakın hedefler aranıyor");
    let ids = |list: &[&Entity]| -> Vec<Slot> { list.iter().map(|e| Slot(e.base().id)).collect() };
    let found = ctx.geometry.nearest(
        &ids(inputs),
        &ids(targets),
        1,
        bound(max),
        measure_of(v.text("measure")),
    );
    let mut nearest: HashMap<usize, NearestFound> = HashMap::new();
    for f in found {
        nearest.entry(f.input).or_insert(f);
    }
    let wanted = field_names(true, v.text("fields"));
    let prefix = v.text("prefix");
    let bearing = v.flag("bearing");
    let mut update = Vec::new();
    let mut none = 0usize;
    for (i, e) in inputs.iter().enumerate() {
        let f = nearest.get(&i);
        if f.is_none() {
            none += 1;
        }
        let target = f.map(|f| targets[f.target]);
        let mut current: Entity = (*e).clone();
        let mut changed = false;
        let mut put = |name: String, text: Option<String>| {
            if let Some(attrs) = with_attr(ctx, &current, &name, text.as_deref()) {
                current.base_mut().attrs = attrs;
                changed = true;
            }
        };
        put(
            format!("{prefix}uzaklık"),
            f.map(|f| length_text(ctx.units, f.d)),
        );
        for name in &wanted {
            put(
                format!("{prefix}{name}"),
                target.and_then(|t| attr(t, name)).map(str::to_owned),
            );
        }
        if bearing {
            put(
                format!("{prefix}semt"),
                f.filter(|f| f.d > 0.0)
                    .map(|f| bearing_text(ctx.units, f.bearing)),
            );
        }
        if changed {
            update.push(Patch {
                id: Slot(e.base().id),
                attrs: Some(current.base().attrs.clone()),
                label: None,
                zs: None,
            });
        }
    }
    if none > 0 {
        let within = if max > 0.0 {
            "en çok uzaklık içinde "
        } else {
            ""
        };
        feedback.info(format!(
            "{none} nesnenin {within}hedefi yok; alanları boşaltıldı."
        ));
    }
    let changed: Vec<u32> = update.iter().map(|u: &Patch| u.id.0).collect();
    let count = inputs.len() - none;
    RunResult {
        changes: Some(ChangeSet {
            update,
            ..ChangeSet::default()
        }),
        outputs: [
            ("changed".to_owned(), json!(changed)),
            ("count".to_owned(), json!(count)),
        ]
        .into_iter()
        .collect(),
        summary: Some(format!(
            "{count} nesneye en yakın hedef yazıldı (“{prefix}uzaklık”)."
        )),
        ..RunResult::default()
    }
}
