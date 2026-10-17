//! A join's source as rows (Anahtarla birleştir, docs/adr/0200 §6; Veri
//! aktar, docs/adr/0216; the web's `builtin/joinSource.ts`): another layer's
//! objects or a CSV, TXT or Excel file's rows, read the same way by both
//! tools.

use std::collections::{BTreeSet, HashMap, HashSet};

use kentos_contracts::Entity;
use kentos_geometry_core::ops::statistics::{JoinPlan, join_key};
use serde_json::{Value, json};

use super::queries::attr;
use crate::parameters::file_table;
use crate::text::js_trim;
use crate::types::{EnumOption, ParamDef, ParamKind};

/// A source as rows: its column names, then a row per object (its
/// attributes' names in their code points' order, a missing one empty) or
/// per line of the file; the cells read where they are.
pub fn source_table<'a>(
    objects: Option<&[&'a Entity]>,
    file: Option<&'a Value>,
) -> (Vec<String>, Vec<Vec<&'a str>>) {
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
                .map(|e| header.iter().map(|h| attr(e, h).unwrap_or("")).collect())
                .collect();
            (header, rows)
        }
        None => file.and_then(file_table).unwrap_or_default(),
    }
}

/// Why a column cannot be read: the names there are.
pub fn missing(name: &str, header: &[String]) -> String {
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

/// Yinelenen anahtar (docs/adr/0439 §4): what a key the source holds more than once does, chosen and not
/// assumed: İlk satırı al (its first row, said), Aktarma (its targets take nothing) or Durdur (the tool
/// does not run).
pub fn duplicates_param() -> ParamDef {
    ParamDef::new(
        "duplicates",
        "Yinelenen anahtar",
        ParamKind::Choice {
            options: vec![
                EnumOption::new("first", "İlk satırı al")
                    .hint("Kaynakta birden çok kez geçen anahtarın ilk satırı alınır ve söylenir"),
                EnumOption::new("skip", "Aktarma").hint(
                    "Kaynakta birden çok kez geçen anahtarın hedeflerine hiçbir şey aktarılmaz",
                ),
                EnumOption::new("stop", "Durdur")
                    .hint("Kaynakta yinelenen anahtar varsa araç çalışmaz"),
            ],
        },
    )
    .default_value(json!("first"))
    .describe("Kaynakta aynı anahtar birden çok kez varsa ne olacağı.")
}

/// The keys the source holds more than once (by the join's rule) and the first such key as written, in the
/// source's order.
pub fn repeated(sources: &[Option<&str>]) -> (HashSet<String>, Option<String>) {
    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut first = None;
    for s in sources.iter().flatten() {
        let Some(k) = join_key(s) else { continue };
        let n = seen.entry(k).or_default();
        *n += 1;
        if *n == 2 && first.is_none() {
            first = Some(js_trim(s).to_owned());
        }
    }
    (
        seen.into_iter()
            .filter(|(_, n)| *n > 1)
            .map(|(k, _)| k)
            .collect(),
        first,
    )
}

/// The plan under the policy `duplicates`: Aktarma unmatches the targets of a repeated key; Durdur
/// refuses with the first repeated key.
pub fn with_policy(
    mut plan: JoinPlan,
    targets: &[Option<&str>],
    sources: &[Option<&str>],
    duplicates: &str,
) -> Result<(JoinPlan, HashSet<String>), String> {
    let (keys, first) = repeated(sources);
    match duplicates {
        "stop" if !keys.is_empty() => Err(format!(
            "Kaynakta {} anahtar birden çok kez var (ilki “{}”); yinelenen anahtar kuralını değiştirin ya da kaynağı düzeltin.",
            keys.len(),
            first.unwrap_or_default()
        )),
        "skip" => {
            for (m, t) in plan.matches.iter_mut().zip(targets) {
                if t.and_then(join_key).is_some_and(|k| keys.contains(&k)) {
                    *m = None;
                }
            }
            Ok((plan, keys))
        }
        _ => Ok((plan, keys)),
    }
}
