//! Anahtarla birleştir (the web's `builtin/joinByField.ts`; docs/adr/0200 §6;
//! QGIS "Join attributes by field value", ArcGIS Join Field, Netcad Veri
//! Aktar): fields of a source (another layer's objects, or a CSV, TXT or
//! Excel file's rows) are copied into the target objects whose key matches.
//! Keys match trimmed; two numbers as numbers (`007` is `7`), otherwise as
//! texts exactly (the core's `ops::statistics::join_plan`). A key the source
//! has more than once takes its first row; unmatched targets and unused rows
//! are counted and said. One undo step.

use std::collections::BTreeSet;

use kentos_contracts::Entity;
use kentos_domain::Slot;
use kentos_geometry_core::ops::statistics::join_plan;
use serde_json::{Value, json};

use super::queries::{attr, with_attr};
use crate::parameters::{field_names, file_table};
use crate::text::js_trim;
use crate::types::{
    ChangeSet, EnumOption, Feedback, OutputDef, OutputKind, ParamDef, ParamKind, Patch, Resolved,
    RunContext, RunResult, ScopeKind, Target, Tool,
};

pub fn tool() -> Tool {
    let source_field = |name: &str, label: &str, multiple: bool| {
        ParamDef::new(
            name,
            label,
            ParamKind::Field {
                of: vec!["source".into(), "file".into()],
                allow_new: false,
                multiple,
            },
        )
        .optional()
    };
    Tool {
        id: "attributes.joinByField".into(),
        label: "Anahtarla birleştir".into(),
        category: "attributes".into(),
        description: "Ortak bir anahtar alanla (parsel numarası, kimlik) başka bir katmandan ya da CSV, TXT, Excel dosyasından öznitelik aktarır.".into(),
        help: Some([
            "Örnek: tapu kayıtlarının malik ve hisse sütunlarını parsel numarasıyla parsellere aktarmak.",
            "Anahtarlar baştaki ve sondaki boşluklar atılarak karşılaştırılır; iki taraf da sayıysa sayı olarak (007 ile 7 aynı), değilse metin olarak tam. Kaynakta aynı anahtar birden çok kez varsa ilk satır alınır ve söylenir.",
            "Dosyanın ilk satırı sütun adlarıdır; Excel dosyasının ilk sayfası okunur. Aktarılacak alanlar boş bırakılırsa anahtardan başka bütün alanlar aktarılır; önek yeni alanların adının başına eklenir. Kaynakta boş olan değer hedefte alanı boşaltır.",
        ]
        .join("\n\n")),
        keywords: [
            "birleştir", "anahtar", "aktar", "veri aktar", "csv", "excel", "tablo", "ilişkilendir",
            "join", "join field", "lookup",
        ]
        .map(String::from)
        .to_vec(),
        aliases: ["ANAHTARBIRLESTIR", "VERIAKTAR"].map(String::from).to_vec(),
        icon: Some("joinField".into()),
        parameters: vec![
            ParamDef::new(
                "target",
                "Hedef nesneler",
                ParamKind::Features {
                    kinds: None,
                    scopes: Some(vec![
                        ScopeKind::Layer,
                        ScopeKind::Selection,
                        ScopeKind::Visible,
                        ScopeKind::All,
                    ]),
                    writes: true,
                },
            )
            .describe("Özniteliklerin aktarılacağı nesneler; kilitli katmandakiler alınmaz."),
            ParamDef::new(
                "targetKey",
                "Hedef anahtar",
                ParamKind::Field {
                    of: vec!["target".into()],
                    allow_new: false,
                    multiple: false,
                },
            )
            .describe("Hedef nesnelerin anahtar alanı."),
            ParamDef::new(
                "sourceKind",
                "Kaynak",
                ParamKind::Choice {
                    options: vec![
                        EnumOption::new("layer", "Katman").hint("Başka nesnelerin öznitelikleri"),
                        EnumOption::new("file", "Dosya").hint("CSV, TXT ya da Excel dosyası"),
                    ],
                },
            )
            .default_value(json!("layer")),
            ParamDef::new(
                "source",
                "Kaynak nesneler",
                ParamKind::Features {
                    kinds: None,
                    scopes: Some(vec![
                        ScopeKind::Layer,
                        ScopeKind::All,
                        ScopeKind::Visible,
                        ScopeKind::Selection,
                    ]),
                    writes: false,
                },
            )
            .shown_when(|v| v.get("sourceKind").and_then(Value::as_str) != Some("file"))
            .describe("Özniteliklerin alınacağı nesneler."),
            ParamDef::new(
                "file",
                "Dosya",
                ParamKind::File {
                    accept: [".csv", ".txt", ".xlsx"].map(String::from).to_vec(),
                },
            )
            .shown_when(|v| v.get("sourceKind").and_then(Value::as_str) == Some("file"))
            .describe("İlk satırı sütun adları olan CSV, TXT ya da Excel (.xlsx) dosyası."),
            source_field("sourceKey", "Kaynak anahtar", false)
                .describe("Boş bırakılırsa hedef anahtarla aynı ad."),
            source_field("fields", "Aktarılacak alanlar", true)
                .describe("Boş bırakılırsa anahtardan başka bütün alanlar."),
            ParamDef::new(
                "prefix",
                "Önek",
                ParamKind::Text {
                    placeholder: Some("Tapu ".into()),
                    max_length: None,
                    allow_empty: true,
                },
            )
            .default_value(json!(""))
            .describe("Aktarılan alanların adının başına eklenir."),
            ParamDef::new(
                "existing",
                "Var olan değerler",
                ParamKind::Choice {
                    options: vec![
                        EnumOption::new("overwrite", "Üzerine yaz")
                            .hint("Hedefteki değer kaynağınkiyle değişir"),
                        EnumOption::new("empty", "Yalnız boşlara")
                            .hint("Hedefte dolu olan değer kalır"),
                    ],
                },
            )
            .default_value(json!("overwrite")),
        ],
        outputs: vec![
            OutputDef::new("changed", "Değişen nesneler", OutputKind::Features),
            OutputDef::new("count", "Eşleşen nesne sayısı", OutputKind::Number),
        ],
        targets: vec![Target::Client, Target::Worker],
        validate: None,
        preview: None,
        run: Some(run),
    }
}

/// A source as rows: its column names, then a row per object (its
/// attributes' names in their code points' order, a missing one empty) or
/// per line of the file.
fn source_table(
    objects: Option<&[&Entity]>,
    file: Option<&Value>,
) -> (Vec<String>, Vec<Vec<String>>) {
    match objects {
        Some(objects) => {
            let header: Vec<String> = objects
                .iter()
                .flat_map(|e| e.base().attrs.keys().cloned())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            let rows = objects
                .iter()
                .map(|e| {
                    header
                        .iter()
                        .map(|h| attr(e, h).unwrap_or("").to_owned())
                        .collect()
                })
                .collect();
            (header, rows)
        }
        None => file.and_then(file_table).unwrap_or_default(),
    }
}

/// Why a column cannot be read: the names there are.
fn missing(name: &str, header: &[String]) -> String {
    let names: Vec<&str> = header
        .iter()
        .map(String::as_str)
        .filter(|h| !h.is_empty())
        .collect();
    if names.is_empty() {
        format!("Kaynakta “{name}” alanı yok; kaynağın hiç alanı yok.")
    } else {
        format!(
            "Kaynakta “{name}” alanı yok; alanları: {}.",
            names.join(", ")
        )
    }
}

fn run(v: &Resolved<'_>, ctx: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    let targets = &v.features("target").entities;
    let from_file = v.text("sourceKind") == "file";
    let source = &v.features("source").entities;
    let (header, rows) = if from_file {
        source_table(None, v.values.get("file"))
    } else {
        source_table(Some(source), None)
    };
    let target_key = v.text("targetKey");
    let key = match v.text("sourceKey") {
        "" => target_key,
        name => name,
    };
    let Some(k) = header.iter().position(|h| h == key) else {
        return RunResult::refused(missing(key, &header));
    };
    let wanted = field_names(true, v.text("fields"));
    if let Some(name) = wanted.iter().find(|n| !header.contains(n)) {
        return RunResult::refused(missing(name, &header));
    }
    let take: Vec<usize> = if wanted.is_empty() {
        header
            .iter()
            .enumerate()
            .filter(|(i, h)| {
                *i != k && !h.is_empty() && header.iter().position(|x| x == *h) == Some(*i)
            })
            .map(|(i, _)| i)
            .collect()
    } else {
        wanted
            .iter()
            .filter_map(|n| header.iter().position(|h| h == n))
            .collect()
    };
    feedback.progress(0.0, "Anahtarlar eşleniyor");
    let target_keys: Vec<Option<&str>> = targets.iter().map(|t| attr(t, target_key)).collect();
    let source_keys: Vec<Option<&str>> =
        rows.iter().map(|r| r.get(k).map(String::as_str)).collect();
    let plan = join_plan(&target_keys, &source_keys);
    let prefix = v.text("prefix");
    let only_empty = v.text("existing") == "empty";
    let mut update = Vec::new();
    let mut matched = 0usize;
    for (t, m) in targets.iter().zip(&plan.matches) {
        let Some(m) = *m else {
            continue;
        };
        matched += 1;
        let mut current: Entity = (*t).clone();
        let mut changed = false;
        for &i in &take {
            let name = format!("{prefix}{}", header[i]);
            if only_empty && !js_trim(attr(&current, &name).unwrap_or("")).is_empty() {
                continue;
            }
            let cell = rows[m].get(i).map_or("", String::as_str);
            let value = (!cell.is_empty()).then_some(cell);
            if let Some(attrs) = with_attr(ctx, &current, &name, value) {
                current.base_mut().attrs = attrs;
                changed = true;
            }
        }
        if changed {
            update.push(Patch {
                id: Slot(t.base().id),
                attrs: Some(current.base().attrs.clone()),
                label: None,
                zs: None,
            });
        }
    }
    if plan.repeated > 0 {
        feedback.warn(format!(
            "{} anahtar kaynakta birden çok kez var; ilk satırları alındı.",
            plan.repeated
        ));
    }
    if targets.len() > matched {
        feedback.info(format!(
            "{} hedef nesnenin kaynakta eşi yok.",
            targets.len() - matched
        ));
    }
    if plan.unused > 0 {
        feedback.info(format!(
            "{} kaynak satırı hiçbir hedefle eşleşmedi.",
            plan.unused
        ));
    }
    let changed: Vec<u32> = update.iter().map(|u: &Patch| u.id.0).collect();
    let summary = format!(
        "{matched} nesne eşleşti; {} nesnede {} alan aktarıldı.",
        update.len(),
        take.len()
    );
    RunResult {
        changes: Some(ChangeSet {
            update,
            ..ChangeSet::default()
        }),
        outputs: [
            ("changed".to_owned(), json!(changed)),
            ("count".to_owned(), json!(matched)),
        ]
        .into_iter()
        .collect(),
        summary: Some(summary),
        ..RunResult::default()
    }
}
