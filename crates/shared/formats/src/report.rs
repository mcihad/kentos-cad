//! Collects what a reader or writer did: objects per kind, what it left out
//! and what it converted, each with a count and the first source lines, so
//! the report stays short however large the file is.

use std::collections::{BTreeMap, HashMap};

use kentos_contracts::{ExportReport, ImportReport, ReportItem, SourceFact};

/// Source lines kept per report line.
const LINES_KEPT: usize = 5;

#[derive(Default)]
struct Items {
    list: Vec<ReportItem>,
    index: HashMap<(String, String), usize>,
}

impl Items {
    fn add(&mut self, what: &str, reason: &str, line: u32, n: u32) {
        let key = (what.to_string(), reason.to_string());
        let i = *self.index.entry(key).or_insert_with(|| {
            self.list.push(ReportItem {
                what: what.to_string(),
                count: 0,
                reason: reason.to_string(),
                lines: Vec::new(),
            });
            self.list.len() - 1
        });
        let item = &mut self.list[i];
        item.count += n;
        if line > 0 && item.lines.len() < LINES_KEPT && item.lines.last() != Some(&line) {
            item.lines.push(line);
        }
    }
}

#[derive(Default)]
pub struct Report {
    counts: BTreeMap<String, u32>,
    skipped: Items,
    notes: Items,
    source: Vec<SourceFact>,
}

impl Report {
    /// One object of `kind` was read (or written).
    pub fn count(&mut self, kind: &str) {
        *self.counts.entry(kind.to_string()).or_insert(0) += 1;
    }

    /// Something at `line` (0: no line) was left out, and why.
    pub fn skip(&mut self, what: &str, reason: &str, line: u32) {
        self.skipped.add(what, reason, line, 1);
    }

    /// Something at `line` was read with a change the user should know about.
    pub fn note(&mut self, what: &str, reason: &str, line: u32) {
        self.notes.add(what, reason, line, 1);
    }

    /// `n` of the same thing at once.
    pub fn note_n(&mut self, what: &str, reason: &str, line: u32, n: u32) {
        if n > 0 {
            self.notes.add(what, reason, line, n);
        }
    }

    pub fn fact(&mut self, label: &str, value: impl Into<String>) {
        self.source.push(SourceFact {
            label: label.to_string(),
            value: value.into(),
        });
    }

    pub fn total(&self) -> u32 {
        self.counts.values().sum()
    }

    pub fn import(self) -> ImportReport {
        ImportReport {
            counts: self.counts,
            skipped: self.skipped.list,
            notes: self.notes.list,
            source: self.source,
        }
    }

    pub fn export(self) -> ExportReport {
        ExportReport {
            counts: self.counts,
            notes: self.notes.list,
            skipped: self.skipped.list,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn items_are_grouped_by_what_and_why_with_the_first_lines() {
        let mut r = Report::default();
        for line in 1..=9 {
            r.skip("IMAGE", "raster görüntüler alınmaz", line * 10);
        }
        r.skip("IMAGE", "başka neden", 3);
        r.count("line");
        r.count("line");
        let rep = r.import();
        assert_eq!(rep.counts.get("line"), Some(&2));
        assert_eq!(rep.skipped.len(), 2);
        assert_eq!(rep.skipped[0].count, 9);
        assert_eq!(rep.skipped[0].lines, vec![10, 20, 30, 40, 50]);
        assert_eq!(rep.skipped[1].count, 1);
    }
}
