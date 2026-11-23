//! Bekleyen değişiklikler (docs/adr/9151): a data layer's table when its source cannot be read (offline, the file
//! gone). The edit session's rows wait in the drawing and its journal; the table lists them still, so a changed row's
//! draft is seen beside the new ones: each changed row by its key with its local values and, where a value changed,
//! the one it had when it was taken; each deleted row by its key, faded. None of it is checked against the source until
//! it can be read again; when it can, the source's pages come back and the taken rows show over them by their keys. The
//! line says when the source last answered for the layer, kept with the project's journal (docs/adr/9163).

use kentos_domain::DataChanges;

/// A pending row the table lists when its source cannot be read.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PendingLine {
    /// The row's key.
    pub key: i64,
    /// Deleted (else changed).
    pub deleted: bool,
    /// Each field's text: the local value, with the value as taken after it where it changed.
    pub cells: Vec<String>,
}

/// The changed and deleted rows of `changes` under `fields`, the changed first, by key.
pub(crate) fn pending_lines(changes: &DataChanges, fields: &[String]) -> Vec<PendingLine> {
    let mut out: Vec<PendingLine> = changes
        .changed
        .iter()
        .map(|c| PendingLine {
            key: c.key,
            deleted: false,
            cells: fields
                .iter()
                .map(|f| {
                    let now = c.now.base().attrs.get(f).cloned().unwrap_or_default();
                    let was = c.row.base().attrs.get(f).cloned().unwrap_or_default();
                    if now == was {
                        now
                    } else {
                        let was = if was.is_empty() { "boş".to_owned() } else { was };
                        format!("{now} (önce: {was})")
                    }
                })
                .collect(),
        })
        .collect();
    out.sort_by_key(|l| l.key);
    for key in &changes.deleted {
        let row = changes.originals.get(key).map(|(_, e)| e.clone());
        out.push(PendingLine {
            key: *key,
            deleted: true,
            cells: fields
                .iter()
                .map(|f| row.as_ref().and_then(|e| e.base().attrs.get(f).cloned()).unwrap_or_default())
                .collect(),
        });
    }
    out
}

/// A table cell of a row read from the source (docs/adr/9152): the draft's value where the row is changed here in that
/// field (`local`: the object now and the row as taken), else the source's value now. The table reads [`cell_raw`]
/// since a NULL is told apart (docs/adr/9188); this stays for the tests.
#[cfg(test)]
pub(crate) fn cell_value(
    local: Option<(&kentos_commands::Entity, &kentos_commands::Entity)>,
    page: &std::collections::BTreeMap<String, String>,
    field: &str,
) -> String {
    cell_raw(local, page, field).unwrap_or_default()
}

/// [`cell_value`], none for a value never written (NULL, docs/adr/9188): a page keeps no key for it.
pub(crate) fn cell_raw(
    local: Option<(&kentos_commands::Entity, &kentos_commands::Entity)>,
    page: &std::collections::BTreeMap<String, String>,
    field: &str,
) -> Option<String> {
    match local {
        Some((now, row)) if now.base().attrs.get(field) != row.base().attrs.get(field) => {
            now.base().attrs.get(field).cloned()
        }
        _ => page.get(field).cloned(),
    }
}

/// A data row's value as its field shows it (docs/adr/9188): a code of a value list by its label, as a drawing layer's
/// table shows it; any other as the source gives it.
pub(crate) fn shown_value(field: Option<&kentos_commands::LayerField>, raw: String) -> String {
    match field {
        Some(f) if f.values.as_ref().is_some_and(|v| !v.is_empty()) && !raw.is_empty() => {
            kentos_commands::display_value(f, &raw)
        }
        _ => raw,
    }
}

/// The status line's words for the pending rows shown while the source cannot be read, with when the source last
/// answered for the layer (`verified`, milliseconds since 1970: docs/adr/9163), in the device's time.
pub(crate) fn pending_words(added: usize, changed: usize, deleted: usize, verified: Option<u64>) -> String {
    let last = match verified {
        Some(ms) => crate::cloud::local_time::stamp(ms, crate::cloud::local_time::Zone::system()),
        None => "bilinmiyor".to_owned(),
    };
    format!(
        "Bekleyen değişiklikler: {}; kaynak okunamadığı için doğrulanmadı (son doğrulama: {last})",
        crate::data_edits::counts_words(added, changed, deleted)
    )
}

#[cfg(test)]
#[path = "pending_rows_tests.rs"]
mod pending_rows_tests;
