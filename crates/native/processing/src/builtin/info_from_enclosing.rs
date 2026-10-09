//! Çevreleyenden bilgi al (the web's `builtin/infoFromEnclosing.ts`;
//! docs/adr/0200 §3; Netcad's Çevreleyenden Bilgi Al, QGIS "Join attributes by
//! location"): each target object gets a field of the source area its centre
//! is in (a building its parcel's number). Several areas: the first in the
//! drawing's order, counted and said; none: the target stays as it is,
//! counted and said. The centres and the areas are the core's (`Merkezi
//! içinde`); one undo step.

use kentos_contracts::Entity;
use kentos_domain::Slot;
use kentos_geometry_core::ops::spatial_query::Relation;
use serde_json::json;

use super::queries::{AREA_KINDS, QUERY_KINDS, attr, kinds, with_attr};
use crate::types::{
    ChangeSet, Feedback, OutputDef, OutputKind, ParamDef, ParamKind, Patch, Resolved, RunContext,
    RunResult, ScopeKind, Target, Tool,
};

pub fn tool() -> Tool {
    Tool {
        id: "attributes.fromEnclosing".into(),
        label: "Çevreleyenden bilgi al".into(),
        category: "attributes".into(),
        description: "Her nesneye ağırlık merkezinin içinde kaldığı alanın bir alanını yazar: yapıya parsel numarası, parsele mahalle adı.".into(),
        help: Some([
            "Nesnenin merkezi ifadelerdeki $merkez_y, $merkez_x’tir: alanların ağırlık merkezi, öbür nesnelerin yerleşim noktası. Merkez alanın sınırındaysa (1 mm içinde) o alan da sayılır.",
            "Merkez birden çok alanın içindeyse çizim sırasıyla ilki alınır; hiçbirinin içinde değilse nesne değişmez. İkisi de sayılır ve söylenir.",
            "Yazılacak alan boş bırakılırsa kaynağın alanının adı kullanılır. İşlem tek adımda geri alınır.",
        ]
        .join("\n\n")),
        keywords: [
            "çevreleyen", "içinde", "parsel", "ada", "mahalle", "mekânsal birleştir", "spatial join",
            "join attributes by location",
        ]
        .map(String::from)
        .to_vec(),
        aliases: ["CEVREBILGI", "CEVRELEYENBILGI"].map(String::from).to_vec(),
        icon: Some("infoEnclosing".into()),
        parameters: vec![
            ParamDef::new(
                "target",
                "Hedef nesneler",
                ParamKind::Features {
                    kinds: kinds(&QUERY_KINDS),
                    scopes: Some(vec![
                        ScopeKind::Layer,
                        ScopeKind::Selection,
                        ScopeKind::Visible,
                        ScopeKind::All,
                    ]),
                    writes: true,
                },
            )
            .describe("Bilginin yazılacağı nesneler; kilitli katmandakiler alınmaz."),
            ParamDef::new(
                "source",
                "Kaynak alanlar",
                ParamKind::Features {
                    kinds: kinds(&AREA_KINDS),
                    scopes: Some(vec![
                        ScopeKind::Layer,
                        ScopeKind::All,
                        ScopeKind::Visible,
                        ScopeKind::Selection,
                    ]),
                    writes: false,
                },
            )
            .describe("Hedefleri çevreleyen alanlar."),
            ParamDef::new(
                "field",
                "Alan",
                ParamKind::Field {
                    of: vec!["source".into()],
                    allow_new: false,
                    multiple: false,
                },
            )
            .describe("Kaynak alanların değeri alınan alanı."),
            ParamDef::new(
                "output",
                "Yazılacak alan",
                ParamKind::Field {
                    of: vec!["target".into()],
                    allow_new: true,
                    multiple: false,
                },
            )
            .optional()
            .describe("Boş bırakılırsa kaynağın alanının adı."),
        ],
        outputs: vec![
            OutputDef::new("changed", "Değişen nesneler", OutputKind::Features),
            OutputDef::new("count", "Yazılan nesne sayısı", OutputKind::Number),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run),
    }
}

fn run(v: &Resolved<'_>, ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let targets = &v.features("target").entities;
    let sources = &v.features("source").entities;
    let field = v.text("field");
    let output = match v.text("output") {
        "" => field,
        name => name,
    };
    feedback.progress(0.0, "Merkezler deneniyor");
    let ids = |list: &[&Entity]| -> Vec<Slot> { list.iter().map(|e| Slot(e.base().id)).collect() };
    // Each target's areas in the drawing's order (the pairs come target by target, areas in order).
    let mut holders: Vec<Vec<&Entity>> = vec![Vec::new(); targets.len()];
    for (i, j) in ctx
        .geometry
        .relate_pairs(&ids(targets), &ids(sources), Relation::CenterIn, 0.0)
    {
        holders[i].push(sources[j]);
    }
    let (mut many, mut none) = (0usize, 0usize);
    let mut update = Vec::new();
    for (t, list) in targets.iter().zip(&holders) {
        let Some(first) = list.first() else {
            none += 1;
            continue;
        };
        if list.len() > 1 {
            many += 1;
        }
        if let Some(attrs) = with_attr(ctx, t, output, attr(first, field)) {
            update.push(Patch {
                id: Slot(t.base().id),
                attrs: Some(attrs),
                label: None,
                zs: None,
            });
        }
    }
    if many > 0 {
        feedback.warn(format!(
            "{many} nesnenin merkezi birden çok alanın içinde; çizim sırasıyla ilki alındı."
        ));
    }
    if none > 0 {
        feedback.warn(format!(
            "{none} nesnenin merkezi hiçbir alanın içinde değil; değişmedi."
        ));
    }
    let changed: Vec<u32> = update.iter().map(|u: &Patch| u.id.0).collect();
    let count = update.len();
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
        summary: Some(format!("{count} nesneye “{output}” yazıldı.")),
        ..RunResult::default()
    }
}
