//! Bekleyen değişiklikler (docs/adr/9151): a data layer's table when its source cannot be read (offline, the file
//! gone). The edit session's rows wait in the drawing and its journal; the table lists them still, so a changed row's
//! draft is seen beside the new ones: each changed row by its key with its local values and, where a value changed,
//! the one it had when it was taken; each deleted row by its key, faded. None of it is checked against the source until
//! it can be read again; when it can, the source's pages come back and the taken rows show over them by their keys.

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

/// The status line's words for the pending rows shown while the source cannot be read.
pub(crate) fn pending_words(added: usize, changed: usize, deleted: usize) -> String {
    format!(
        "Bekleyen değişiklikler: {}; kaynak okunamadığı için doğrulanmadı",
        crate::data_edits::counts_words(added, changed, deleted)
    )
}

#[cfg(test)]
#[path = "pending_rows_tests.rs"]
mod pending_rows_tests;
