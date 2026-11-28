//! The first pass (docs/adr/8061 §7, "Bozuk girdi"): every record's place, found by the engine's scanner (iterative,
//! quote and comment aware), and measured by our own walk before anything is decoded: its length, how deeply its lists
//! nest and how many items they hold. The engine parses lists recursively; a record nested past the limit never
//! reaches it, so a hostile file cannot overflow the stack. A record number given twice and a record the scanner had
//! to drop are findings. The line of every record is counted on the way, for the messages.

use ifc_lite_core::{EntityIndex, EntityScanner};

use super::{Finding, Limits, ReadError, RecordLimit, Severity};

/// Ask the host whether to stop once in this many records.
pub(super) const CANCEL_EVERY: usize = 4096;

/// What the first pass found: every record's span by its number, and its findings.
pub(super) struct Scanned {
    pub(super) index: EntityIndex,
    pub(super) findings: Vec<Finding>,
}

/// Counts lines as a pass moves forward through the file.
pub(super) struct Lines {
    at: usize,
    line: usize,
}

impl Lines {
    pub(super) fn new() -> Self {
        Lines { at: 0, line: 1 }
    }

    /// The line of byte `to`, at or after the last one asked.
    pub(super) fn at(&mut self, bytes: &[u8], to: usize) -> usize {
        let to = to.min(bytes.len());
        if to > self.at {
            self.line += bytes[self.at..to].iter().filter(|&&c| c == b'\n').count();
            self.at = to;
        }
        self.line
    }
}

/// The first pass over `bytes`.
pub(super) fn scan(
    bytes: &[u8],
    limits: &Limits,
    cancel: &dyn Fn() -> bool,
) -> Result<Scanned, ReadError> {
    let mut index = EntityIndex::with_capacity_and_hasher(bytes.len() / 64, Default::default());
    let mut findings = Vec::new();
    let mut scanner = EntityScanner::new(bytes);
    let mut lines = Lines::new();
    let mut n = 0usize;
    while let Some((id, _, start, end)) = scanner.next_entity() {
        n += 1;
        if n > limits.records {
            return Err(ReadError::TooManyRecords {
                limit: limits.records,
            });
        }
        if n.is_multiple_of(CANCEL_EVERY) && cancel() {
            return Err(ReadError::Cancelled);
        }
        let line = lines.at(bytes, start);
        if end.saturating_sub(start) > limits.record_bytes {
            return Err(ReadError::Record {
                line,
                what: RecordLimit::Length,
                limit: limits.record_bytes,
            });
        }
        measure(&bytes[start..end.min(bytes.len())], limits).map_err(|what| ReadError::Record {
            line,
            what,
            limit: match what {
                RecordLimit::Length => limits.record_bytes,
                RecordLimit::Nesting => limits.nesting,
                RecordLimit::Items => limits.items,
            },
        })?;
        if index.insert(id, (start, end)).is_some() {
            findings.push(Finding {
                code: "step_duplicate",
                severity: Severity::Error,
                step: Some(id),
                line: Some(line),
                message: format!(
                    "#{id} numaralı kayıt dosyada birden çok kez var; sonuncusu okundu. Dosyayı yazan programla yeniden dışa aktarın."
                ),
            });
        }
    }
    if let Some(at) = scanner.malformed_record_start() {
        let line = Lines::new().at(bytes, at);
        findings.push(Finding {
            code: "record_malformed",
            severity: Severity::Error,
            step: None,
            line: Some(line),
            message: format!(
                "{line}. satırdaki kayıt okunamadı (kesik ya da biçimsiz); okuma sonrakilerle sürdü. Dosya yarım kalmış olabilir."
            ),
        });
    }
    let oversized = scanner.skipped_oversized_ids();
    if oversized > 0 {
        findings.push(Finding {
            code: "step_number_too_large",
            severity: Severity::Error,
            step: None,
            line: None,
            message: format!(
                "{oversized} kaydın numarası 4 294 967 295'ten büyük; bu kayıtlar okunmadı."
            ),
        });
    }
    if index.is_empty() {
        return Err(ReadError::NoData);
    }
    Ok(Scanned { index, findings })
}

/// Measures one record: how deeply its lists nest and how many items they hold, outside quoted strings and comments.
/// The record's own attribute list is the first level; an item is counted at every comma inside a nested list.
fn measure(record: &[u8], limits: &Limits) -> Result<(), RecordLimit> {
    let (mut depth, mut items) = (0usize, 0usize);
    let (mut quoted, mut comment) = (false, false);
    let mut i = 0;
    while i < record.len() {
        let c = record[i];
        if comment {
            if c == b'*' && record.get(i + 1) == Some(&b'/') {
                comment = false;
                i += 1;
            }
        } else if quoted {
            if c == b'\'' {
                quoted = false;
            }
        } else {
            match c {
                b'\'' => quoted = true,
                b'/' if record.get(i + 1) == Some(&b'*') => {
                    comment = true;
                    i += 1;
                }
                b'(' => {
                    depth += 1;
                    if depth > limits.nesting {
                        return Err(RecordLimit::Nesting);
                    }
                }
                b')' => depth = depth.saturating_sub(1),
                b',' if depth >= 2 => {
                    items += 1;
                    if items > limits.items {
                        return Err(RecordLimit::Items);
                    }
                }
                _ => {}
            }
        }
        i += 1;
    }
    Ok(())
}

/// The record numbers `record` refers to (`#n` outside quoted strings and comments), each once per call of `each`.
pub(super) fn references(record: &[u8], mut each: impl FnMut(u32)) {
    let (mut quoted, mut comment) = (false, false);
    let mut i = 0;
    // The record's own number, before its `=`, is not a reference.
    if let Some(eq) = record.iter().position(|&c| c == b'=') {
        i = eq + 1;
    }
    while i < record.len() {
        let c = record[i];
        if comment {
            if c == b'*' && record.get(i + 1) == Some(&b'/') {
                comment = false;
                i += 1;
            }
        } else if quoted {
            if c == b'\'' {
                quoted = false;
            }
        } else if c == b'\'' {
            quoted = true;
        } else if c == b'/' && record.get(i + 1) == Some(&b'*') {
            comment = true;
            i += 1;
        } else if c == b'#' {
            let mut n: u64 = 0;
            let mut j = i + 1;
            while j < record.len() && record[j].is_ascii_digit() && n <= u64::from(u32::MAX) {
                n = n * 10 + u64::from(record[j] - b'0');
                j += 1;
            }
            if j > i + 1
                && let Ok(n) = u32::try_from(n)
            {
                each(n);
            }
            i = j;
            continue;
        }
        i += 1;
    }
}
