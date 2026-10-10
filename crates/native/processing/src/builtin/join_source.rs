//! A join's source as rows (Anahtarla birleştir, docs/adr/0200 §6; Veri
//! aktar, docs/adr/0216; the web's `builtin/joinSource.ts`): another layer's
//! objects or a CSV, TXT or Excel file's rows, read the same way by both
//! tools.

use std::collections::BTreeSet;

use kentos_contracts::Entity;
use serde_json::Value;

use super::queries::attr;
use crate::parameters::file_table;

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
