//! What running a record again would meet (docs/adr/0529 §2). A run's record (`kentos.run-record` 1) says which
//! revision of the drawing it began with and the content digest of each layer it read. Running its values again is
//! one of three things, and the record keeps them apart:
//!
//! - the drawing is at that revision: the same run on the same drawing, as the record's `parameters` and `selection`
//!   give it;
//! - the drawing has moved on but every layer the run read has the content it had: the same data, so the same
//!   result, whatever else changed;
//! - a layer it read has changed (or is gone): the same values on the current data, which may give another result.
//!
//! The record itself is a value: changing a source afterwards changes nothing in it.

use kentos_commands::ProcessingRecord;
use kentos_domain::Document;

/// What running a record's values again on a drawing meets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Rerun {
    /// The drawing is at the revision the run began with.
    SameRevision,
    /// Another revision, every input layer unchanged.
    SameData,
    /// These input layers have changed since: the current data, not the recorded.
    ChangedData(Vec<String>),
}

/// Which of the three running `record` again on `doc` is.
pub fn rerun_kind(record: &ProcessingRecord, doc: &Document) -> Rerun {
    if doc.revision().to_string() == record.document_revision {
        return Rerun::SameRevision;
    }
    let changed: Vec<String> = record
        .inputs
        .iter()
        .filter(|i| kentos_native_application::sources::layer_digest(doc, &i.layer) != i.digest)
        .map(|i| i.layer.clone())
        .collect();
    if changed.is_empty() {
        Rerun::SameData
    } else {
        Rerun::ChangedData(changed)
    }
}

impl Rerun {
    /// The sentence the history shows beside a run.
    pub fn sentence(&self) -> String {
        match self {
            Rerun::SameRevision => {
                "Yeniden çalıştırma: çizim aynı sürümde; aynı veride aynı sonucu verir.".to_owned()
            }
            Rerun::SameData => {
                "Yeniden çalıştırma: çizim değişti ama okunan katmanlar aynı; aynı veriyle aynı sonucu verir."
                    .to_owned()
            }
            Rerun::ChangedData(layers) => format!(
                "Yeniden çalıştırma: okunan katman değişmiş ({}); güncel veriyle farklı sonuç verebilir.",
                layers.join(", ")
            ),
        }
    }
}
