//! What running a record again would meet (docs/adr/0529 §2, 0593 §3). A run's record (`kentos.run-record` 1) says
//! which revision of the drawing it began with, the content digest of each layer it read, and for an input read from
//! its source the file or database and the file's SHA-256. Running its values again is one of four things, and the
//! record keeps them apart:
//!
//! - the drawing is at that revision and no source file it read has changed: the same run on the same drawing, as
//!   the record's `parameters` and `selection` give it;
//! - the drawing has moved on but every layer and every file the run read has the content it had: the same data, so
//!   the same result, whatever else changed;
//! - a layer or a source file it read has changed (or is gone): the same values on the current data, which may give
//!   another result;
//! - it read a database table: its state then is not known here, so the result may differ.
//!
//! The record itself is a value: changing a source afterwards changes nothing in it.

use kentos_commands::{ProcessingRecord, RunInput};
use kentos_domain::Document;

/// What running a record's values again on a drawing meets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Rerun {
    /// The drawing is at the revision the run began with, no source file changed.
    SameRevision,
    /// Another revision, every input layer and source file unchanged.
    SameData,
    /// These input layers or source tables have changed since: the current data, not the recorded.
    ChangedData(Vec<String>),
    /// These tables were read from a database, whose state then is not kept: it may have changed.
    UnknownSource(Vec<String>),
}

/// An input read from its source: whether its file still holds what was read. None: a database's, not known.
fn source_same(input: &RunInput) -> Option<bool> {
    let path = input.path.as_deref()?;
    if kentos_commands::is_database(path) {
        return None;
    }
    let now = std::fs::File::open(path).ok().and_then(|f| {
        kentos_commands::encoding::sha256_reader_hex(std::io::BufReader::new(f))
            .ok()
            .map(|(_, sha)| sha)
    });
    Some(now.is_some() && now == input.revision)
}

/// Which of the four running `record` again on `doc` is.
pub fn rerun_kind(record: &ProcessingRecord, doc: &Document) -> Rerun {
    // At the same revision the drawing's layers are as they were: only the source files are looked at.
    let same = doc.revision().to_string() == record.document_revision;
    let mut changed: Vec<String> = Vec::new();
    let mut unknown: Vec<String> = Vec::new();
    for i in &record.inputs {
        let name = || i.table.clone().unwrap_or_else(|| i.layer.clone());
        if i.table.is_some() {
            // Read from its source into the run's copy: the source says, not the drawing.
            match source_same(i) {
                Some(true) => {}
                Some(false) => changed.push(name()),
                None => unknown.push(name()),
            }
        } else if !same
            && kentos_native_application::sources::layer_digest(doc, &i.layer) != i.digest
        {
            changed.push(name());
        }
    }
    if !changed.is_empty() {
        return Rerun::ChangedData(changed);
    }
    if !unknown.is_empty() {
        return Rerun::UnknownSource(unknown);
    }
    if same {
        Rerun::SameRevision
    } else {
        Rerun::SameData
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
            Rerun::UnknownSource(tables) => format!(
                "Yeniden çalıştırma: veritabanından okundu ({}); veritabanının o anki hâli bilinmediğinden güncel \
                 veriyle farklı sonuç verebilir.",
                tables.join(", ")
            ),
        }
    }
}
