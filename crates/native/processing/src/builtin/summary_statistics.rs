//! Özet istatistik (the web's `builtin/summaryStatistics.ts`; docs/adr/0200
//! §5; QGIS "Statistics by categories", ArcGIS Summary Statistics): a field's
//! figures over the objects, group by group when a grouping field is given:
//! how many objects, how many values read as numbers, the sum, mean, least,
//! most and the sample standard deviation. The table is the core's
//! (`ops::statistics::summarize`); the dialog shows it after the run with
//! Panoya kopyala and CSV olarak kaydet. Edits nothing.

use kentos_geometry_core::ops::statistics::summarize;
use serde_json::json;

use super::queries::attr;
use crate::text::js_trim;
use crate::types::{
    Feedback, OutputDef, OutputKind, ParamDef, ParamKind, Resolved, RunContext, RunResult,
    ScopeKind, Target, Tool,
};

pub fn tool() -> Tool {
    let field = |name: &str, label: &str| {
        ParamDef::new(
            name,
            label,
            ParamKind::Field {
                of: vec!["input".into()],
                allow_new: false,
                multiple: false,
            },
        )
    };
    Tool {
        id: "statistics.summary".into(),
        label: "Özet istatistik".into(),
        category: "analysis".into(),
        description: "Bir alanın toplamını, ortalamasını, en azını, en çoğunu ve standart sapmasını, isterseniz başka bir alanın değerlerine göre gruplayarak tablo olarak verir.".into(),
        help: Some([
            "Örnekler: yapıların taban alanlarının kat sayısına göre toplamı; parsellerin tapu alanlarının ortalaması ve standart sapması.",
            "Değerler sayı olarak okunur (ondalık nokta ya da virgül); okunamayanlar atlanır ve söylenir. Toplam, en az ve en çok kesindir; ortalama ve standart sapma (örneklem) değerlerin en çok basamağından iki fazla basamakla yazılır.",
            "Gruplar adlarının doğal sırasıyla, boş grup “(boş)” en sonda; gruplanınca son satır bütün nesnelerin toplamıdır. Çizim değişmez.",
        ]
        .join("\n\n")),
        keywords: [
            "istatistik", "özet", "toplam", "ortalama", "standart sapma", "grupla", "statistics",
            "summary", "group by", "categories",
        ]
        .map(String::from)
        .to_vec(),
        aliases: ["OZETIST", "ISTATISTIK"].map(String::from).to_vec(),
        icon: Some("statsSummary".into()),
        parameters: vec![
            ParamDef::new(
                "input",
                "Nesneler",
                ParamKind::Features {
                    kinds: None,
                    scopes: Some(vec![
                        ScopeKind::Layer,
                        ScopeKind::Selection,
                        ScopeKind::Visible,
                        ScopeKind::All,
                    ]),
                    writes: false,
                },
            )
            .describe("İstatistiği alınan nesneler."),
            field("field", "Alan").describe("Değerleri sayı olarak okunan alan."),
            field("group", "Grupla")
                .optional()
                .describe("Boş bırakılırsa bütün nesneler tek satırdır."),
        ],
        outputs: vec![
            OutputDef::new("table", "Özet tablosu", OutputKind::Table),
            OutputDef::new("count", "Nesne sayısı", OutputKind::Number),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run),
    }
}

fn run(v: &Resolved<'_>, _ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let list = &v.features("input").entities;
    let field = v.text("field");
    let group = v.text("group");
    feedback.progress(0.0, "Değerler okunuyor");
    let items: Vec<(Option<&str>, Option<&str>)> = list
        .iter()
        .map(|e| {
            (
                (!group.is_empty()).then(|| attr(e, group)).flatten(),
                attr(e, field),
            )
        })
        .collect();
    let table = match summarize(&items, !group.is_empty()) {
        Ok(t) => t,
        Err(why) => return RunResult::refused(why),
    };
    let blank = items
        .iter()
        .filter(|(_, v)| js_trim(v.unwrap_or("")).is_empty())
        .count();
    let read = list.len() - table.skipped - blank;
    if table.skipped > 0 {
        feedback.warn(format!(
            "{} değer sayı olarak okunamadığı için atlandı.",
            table.skipped
        ));
    }
    let summary = if group.is_empty() {
        format!("{} nesnede “{field}”: {read} değer okundu.", list.len())
    } else {
        format!(
            "{} nesnede “{field}”: {read} değer okundu, {} grup.",
            list.len(),
            table.rows.len().saturating_sub(1)
        )
    };
    RunResult {
        outputs: [
            (
                "table".to_owned(),
                json!({ "columns": table.columns, "rows": table.rows }),
            ),
            ("count".to_owned(), json!(list.len())),
        ]
        .into_iter()
        .collect(),
        summary: Some(summary),
        ..RunResult::default()
    }
}
