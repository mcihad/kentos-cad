//! What an object came from (docs/adr/0598): a split's pieces from the object split, a merge's object from the
//! ones merged into it. The drawing keeps the records in its undo history (a split undone takes its records with
//! it) and a bound layer's write puts the records of the objects it writes beside its rows, in the same transaction
//! (`kentos_lineage`). A project setting (`.kcad` schema 101, `lineage`), in the order the steps made them.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::EntityId;

/// The most records a project keeps; the oldest go first when a step would pass it.
pub const MOST_LINEAGE_RECORDS: usize = 100_000;
/// The most parents a record names (a merge of that many objects).
pub const MOST_LINEAGE_PARENTS: usize = 10_000;
/// The longest step name a record keeps, in characters.
pub const MOST_LINEAGE_STEP: usize = 200;

/// How the object came from its parents.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum LineageOperation {
    /// A piece of one object split (Böl, Kır, Parçalara ayır).
    Split,
    /// The object the others were merged into (Birleştir, Parçaları birleştir).
    Merge,
}

impl LineageOperation {
    /// The name a source's table writes.
    pub fn name(self) -> &'static str {
        match self {
            LineageOperation::Split => "split",
            LineageOperation::Merge => "merge",
        }
    }
}

/// One object's parents in one step.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LineageRecord {
    /// The object made or kept by the step.
    pub child: EntityId,
    /// The objects it came from, in the step's order; never itself.
    pub parents: Vec<EntityId>,
    pub operation: LineageOperation,
    /// The undo step's name (“Böl”, “Birleştir”); 1–200 characters.
    pub step: String,
}

/// Why `record` cannot be a project's, or none.
pub fn lineage_record_problem(record: &LineageRecord) -> Option<String> {
    if record.child.is_nil() {
        return Some("nesnenin kimliği boş".into());
    }
    if record.parents.is_empty() || record.parents.len() > MOST_LINEAGE_PARENTS {
        return Some(format!(
            "1 ile {MOST_LINEAGE_PARENTS} arasında atası olmalı"
        ));
    }
    for (i, p) in record.parents.iter().enumerate() {
        if p.is_nil() {
            return Some("bir atasının kimliği boş".into());
        }
        if *p == record.child {
            return Some("nesne kendi atası olamaz".into());
        }
        if record.parents[..i].contains(p) {
            return Some(format!("“{p}” iki kez ata"));
        }
    }
    let chars = record.step.chars().count();
    if chars == 0 || chars > MOST_LINEAGE_STEP || record.step.trim() != record.step {
        return Some(format!(
            "adımın adı boşluksuz başlayıp biten 1–{MOST_LINEAGE_STEP} karakter olmalı"
        ));
    }
    None
}

/// Why `records` cannot be a project's, or none: each record's own rules, at most [`MOST_LINEAGE_RECORDS`], no
/// record twice.
pub fn lineage_problem(records: &[LineageRecord]) -> Option<String> {
    if records.len() > MOST_LINEAGE_RECORDS {
        return Some(format!(
            "Projede en çok {MOST_LINEAGE_RECORDS} soy kaydı olur; {} var.",
            records.len()
        ));
    }
    let mut seen = std::collections::HashSet::new();
    for (i, r) in records.iter().enumerate() {
        if let Some(p) = lineage_record_problem(r) {
            return Some(format!("{}. soy kaydı: {p}.", i + 1));
        }
        if !seen.insert(r) {
            return Some(format!("{}. soy kaydı önceki bir kaydın aynısı.", i + 1));
        }
    }
    None
}

impl std::hash::Hash for LineageRecord {
    fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
        self.child.0.hash(h);
        for p in &self.parents {
            p.0.hash(h);
        }
        self.operation.name().hash(h);
        self.step.hash(h);
    }
}

/// The records `records` with `added` after them, the oldest dropped past [`MOST_LINEAGE_RECORDS`], and an
/// added record that is already there kept once.
pub fn lineage_with(records: &[LineageRecord], added: &[LineageRecord]) -> Vec<LineageRecord> {
    let mut out: Vec<LineageRecord> = records.to_vec();
    for a in added {
        if !out.contains(a) {
            out.push(a.clone());
        }
    }
    if out.len() > MOST_LINEAGE_RECORDS {
        out.drain(..out.len() - MOST_LINEAGE_RECORDS);
    }
    out
}

/// The parents of `uid` and theirs, nearest first, each once: what the records say it came from.
pub fn ancestors_of(records: &[LineageRecord], uid: EntityId) -> Vec<EntityId> {
    let mut out: Vec<EntityId> = Vec::new();
    let mut next = vec![uid];
    while let Some(u) = next.pop() {
        for r in records.iter().filter(|r| r.child == u) {
            for p in &r.parents {
                if *p != uid && !out.contains(p) {
                    out.push(*p);
                    next.insert(0, *p);
                }
            }
        }
    }
    out
}

/// The objects that came from `uid` and theirs, nearest first, each once.
pub fn descendants_of(records: &[LineageRecord], uid: EntityId) -> Vec<EntityId> {
    let mut out: Vec<EntityId> = Vec::new();
    let mut next = vec![uid];
    while let Some(u) = next.pop() {
        for r in records.iter().filter(|r| r.parents.contains(&u)) {
            if r.child != uid && !out.contains(&r.child) {
                out.push(r.child);
                next.insert(0, r.child);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u8) -> EntityId {
        let mut b = [0u8; 16];
        b[15] = n;
        EntityId(b)
    }

    fn rec(child: u8, parents: &[u8], op: LineageOperation) -> LineageRecord {
        LineageRecord {
            child: id(child),
            parents: parents.iter().map(|p| id(*p)).collect(),
            operation: op,
            step: "Böl".into(),
        }
    }

    #[test]
    fn records_keep_their_rules() {
        assert!(lineage_problem(&[rec(2, &[1], LineageOperation::Split)]).is_none());
        assert!(lineage_problem(&[rec(1, &[1], LineageOperation::Split)]).is_some());
        assert!(lineage_problem(&[rec(2, &[], LineageOperation::Split)]).is_some());
        assert!(lineage_problem(&[rec(2, &[1, 1], LineageOperation::Merge)]).is_some());
        let twice = rec(2, &[1], LineageOperation::Split);
        assert!(lineage_problem(&[twice.clone(), twice]).is_some());
        let mut blank = rec(2, &[1], LineageOperation::Split);
        blank.step = " Böl".into();
        assert!(lineage_problem(&[blank]).is_some());
    }

    #[test]
    fn ancestors_and_descendants_walk_the_records() {
        // 1 split into 2 and 3; 3 and 4 merged into 5.
        let records = vec![
            rec(2, &[1], LineageOperation::Split),
            rec(3, &[1], LineageOperation::Split),
            rec(5, &[3, 4], LineageOperation::Merge),
        ];
        assert_eq!(ancestors_of(&records, id(5)), vec![id(3), id(4), id(1)]);
        assert_eq!(descendants_of(&records, id(1)), vec![id(2), id(3), id(5)]);
        assert!(ancestors_of(&records, id(1)).is_empty());
    }
}
