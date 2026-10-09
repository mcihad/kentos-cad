//! İçindekinden bilgi al (the web's `builtin/infoFromInside.ts`; docs/adr/0200
//! §3; Netcad's İçindekinden Bilgi Al, QGIS "Join attributes by location
//! (summary)"): each target area gets a statistic of the source objects in the
//! relation to it (in it, meeting it, or with their centre in it): how many,
//! the sum, mean, least or most of a field, or the first value. The pairs come
//! from the run's store, the numbers from the core's `ops::statistics`; one
//! undo step.

use kentos_contracts::Entity;
use kentos_domain::Slot;
use kentos_geometry_core::ops::spatial_query::Relation;
use kentos_geometry_core::ops::statistics::{Stat, statistic};
use serde_json::{Value, json};

use super::queries::{AREA_KINDS, QUERY_KINDS, attr, kinds, mean_scale, with_attr};
use crate::types::{
    ChangeSet, EnumOption, Feedback, OutputDef, OutputKind, ParamDef, ParamKind, Patch, Resolved,
    RunContext, RunResult, ScopeKind, Target, Tool,
};

/// A statistic as the summary names it.
pub fn stat_label(stat: Stat) -> &'static str {
    match stat {
        Stat::Count => "sayı",
        Stat::Sum => "toplam",
        Stat::Mean => "ortalama",
        Stat::Min => "en az",
        Stat::Max => "en çok",
        Stat::First => "ilk değer",
    }
}

pub fn tool() -> Tool {
    Tool {
        id: "attributes.fromInside".into(),
        label: "İçindekinden bilgi al".into(),
        category: "attributes".into(),
        description: "Her alana içindeki ya da ona değen nesnelerin sayısını ya da bir alanlarının toplamını, ortalamasını, en azını, en çoğunu yazar.".into(),
        help: Some([
            "Örnekler: parsellere içlerindeki ağaçların sayısı; adalara yapıların taban alanlarının toplamı; mahallelere içindeki yapıların kat ortalaması.",
            "İlişki kaynağın hedefe göre durumudur: İçinde kalan bütünüyle içeride olanları, Kesişen alana değen ya da onu kesenleri, Merkezi içinde ağırlık merkezi içeride olanları alır. Sınır 1 mm içinde sayılır.",
            "Toplam, en az ve en çok kesindir; ortalama yazılacak alan ondalık sayıysa onun basamağına, değilse değerlerin en çok basamağından iki fazlasına yarım çifte yuvarlanır. Sayı olarak okunamayan değerler atlanır ve söylenir. Hiçbir kaynak yoksa Sayı 0 yazar, öbürleri alanı boşaltır.",
        ]
        .join("\n\n")),
        keywords: [
            "içindeki", "içinde", "sayı", "toplam", "ortalama", "mekânsal birleştir", "özet",
            "spatial join", "count points in polygon", "summary",
        ]
        .map(String::from)
        .to_vec(),
        aliases: ["ICBILGI", "ICINDEKIBILGI"].map(String::from).to_vec(),
        icon: Some("infoInside".into()),
        parameters: vec![
            ParamDef::new(
                "target",
                "Hedef alanlar",
                ParamKind::Features {
                    kinds: kinds(&AREA_KINDS),
                    scopes: Some(vec![
                        ScopeKind::Layer,
                        ScopeKind::Selection,
                        ScopeKind::Visible,
                        ScopeKind::All,
                    ]),
                    writes: true,
                },
            )
            .describe("Bilginin yazılacağı alanlar; kilitli katmandakiler alınmaz."),
            ParamDef::new(
                "source",
                "Kaynak nesneler",
                ParamKind::Features {
                    kinds: kinds(&QUERY_KINDS),
                    scopes: Some(vec![
                        ScopeKind::Layer,
                        ScopeKind::All,
                        ScopeKind::Visible,
                        ScopeKind::Selection,
                    ]),
                    writes: false,
                },
            )
            .describe("Sayılan ya da değerleri alınan nesneler."),
            ParamDef::new(
                "relation",
                "İlişki",
                ParamKind::Choice {
                    options: vec![
                        EnumOption::new("within", "İçinde kalan")
                            .hint("Bütünüyle alanın içinde olanlar"),
                        EnumOption::new("intersects", "Kesişen")
                            .hint("Alana değen ya da onu kesenler"),
                        EnumOption::new("centerIn", "Merkezi içinde")
                            .hint("Ağırlık merkezi alanın içinde olanlar"),
                    ],
                },
            )
            .default_value(json!("within")),
            ParamDef::new(
                "stat",
                "İstatistik",
                ParamKind::Choice {
                    options: vec![
                        EnumOption::new("count", "Sayı").hint("Kaynak nesnelerin sayısı"),
                        EnumOption::new("sum", "Toplam").hint("Alanın değerlerinin toplamı"),
                        EnumOption::new("mean", "Ortalama").hint("Alanın değerlerinin ortalaması"),
                        EnumOption::new("min", "En az").hint("Alanın en küçük değeri"),
                        EnumOption::new("max", "En çok").hint("Alanın en büyük değeri"),
                        EnumOption::new("first", "İlk değer")
                            .hint("Çizim sırasıyla ilk dolu değer, olduğu gibi"),
                    ],
                },
            )
            .default_value(json!("count")),
            ParamDef::new(
                "field",
                "Alan",
                ParamKind::Field {
                    of: vec!["source".into()],
                    allow_new: false,
                    multiple: false,
                },
            )
            .shown_when(|v| v.get("stat").and_then(Value::as_str) != Some("count"))
            .describe("Kaynak nesnelerin değeri alınan alanı."),
            ParamDef::new(
                "output",
                "Yazılacak alan",
                ParamKind::Field {
                    of: vec!["target".into()],
                    allow_new: true,
                    multiple: false,
                },
            )
            .default_value(json!("Nesne sayısı"))
            .describe("Listeden var olan bir alanı seçin ya da yeni bir ad yazın."),
        ],
        outputs: vec![
            OutputDef::new("changed", "Değişen nesneler", OutputKind::Features),
            OutputDef::new("count", "Yazılan nesne sayısı", OutputKind::Number),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: Some(preview),
        run: Some(run),
    }
}

fn preview(v: &crate::types::Values) -> Option<String> {
    let output = crate::text::js_trim(v.get("output").and_then(Value::as_str).unwrap_or(""));
    let stat = Stat::from_key(v.get("stat").and_then(Value::as_str).unwrap_or(""))?;
    (!output.is_empty()).then(|| format!("“{output}” alanına {} yazılacak", stat_label(stat)))
}

fn run(v: &Resolved<'_>, ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let targets = &v.features("target").entities;
    let sources = &v.features("source").entities;
    let stat = Stat::from_key(v.text("stat")).unwrap_or(Stat::Count);
    let field = if stat == Stat::Count {
        ""
    } else {
        v.text("field")
    };
    let output = v.text("output");
    let relation = Relation::from_key(v.text("relation")).unwrap_or(Relation::Within);
    feedback.progress(0.0, "İlişkiler deneniyor");
    let ids = |list: &[&Entity]| -> Vec<Slot> { list.iter().map(|e| Slot(e.base().id)).collect() };
    let pairs = ctx
        .geometry
        .relate_pairs(&ids(sources), &ids(targets), relation, 0.0);
    // Each target's sources in the drawing's order: the pairs come source by source.
    let mut members: Vec<Vec<&Entity>> = vec![Vec::new(); targets.len()];
    for (i, j) in pairs {
        members[j].push(sources[i]);
    }
    feedback.progress(0.5, "Değerler hesaplanıyor");
    let mut update = Vec::new();
    let mut skipped = 0usize;
    for (t, list) in targets.iter().zip(&members) {
        let values: Vec<Option<&str>> = list
            .iter()
            .map(|s| (!field.is_empty()).then(|| attr(s, field)).flatten())
            .collect();
        let scale = (stat == Stat::Mean)
            .then(|| mean_scale(ctx, t, output))
            .flatten();
        let (value, skip) = match statistic(&values, stat, scale) {
            Ok(r) => r,
            Err(why) => return RunResult::refused(why),
        };
        skipped += skip;
        if let Some(attrs) = with_attr(ctx, t, output, value.as_deref()) {
            update.push(Patch {
                id: Slot(t.base().id),
                attrs: Some(attrs),
                label: None,
                zs: None,
            });
        }
    }
    if skipped > 0 {
        feedback.warn(format!(
            "{skipped} değer sayı olarak okunamadığı için atlandı."
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
        summary: Some(format!(
            "{count} hedef nesneye “{output}” yazıldı ({}).",
            stat_label(stat)
        )),
        ..RunResult::default()
    }
}
