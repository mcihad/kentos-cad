//! The document's journal and request ledger (docs/adr/0253 §3, §6).
//!
//! The journal (`kentos.calls/1`) says, in order, what was written: a row a
//! command's write (its row comes from the command's seal, `step.rs`), a row
//! an edit made without a command (an undo step a tool or a window
//! recorded, a change of the project's parts). A row waits while its step
//! is inside an open transaction or group: it is recorded with the step it
//! becomes, and dropped with a step taken back. Undo and redo mark the rows
//! of their step undone and done. The journal is of this session; it keeps
//! the last [`JOURNAL_ROWS`] rows.
//!
//! The ledger keeps the requests a command executed (their id, the call's
//! fingerprint, the answer then), the last [`REQUEST_LEDGER`]: a request
//! sent again is answered from it and not applied twice. It is kept in the
//! `.kcad` file (`requests`, docs/specs/kcad-v2.md §6.13), so a request whose
//! answer was lost with the process is still known in the saved drawing.

use std::collections::{HashMap, VecDeque};

use kentos_contracts::{
    CallKind, CallRecord, CallStatus, JOURNAL_IDS, JOURNAL_ROWS, ProjectPart, REQUEST_LEDGER,
    RequestRecord,
};

use crate::document::Document;
use crate::history::Op;
use crate::identity::Uuid;

/// A row waiting for its step, and its request.
#[derive(Clone, Debug)]
struct Waiting {
    row: CallRecord,
    request: Option<RequestRecord>,
}

/// A request in the ledger: its record, its row and its undo step in this
/// session (none for one read from the file).
#[derive(Clone, Debug)]
struct Entry {
    record: RequestRecord,
    call: Option<u64>,
    step: Option<u64>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Calls {
    rows: VecDeque<CallRecord>,
    waiting: Vec<Waiting>,
    /// The last row number given.
    given: u64,
    ledger: VecDeque<Entry>,
}

impl Calls {
    /// How many rows wait.
    pub(crate) fn waiting(&self) -> usize {
        self.waiting.len()
    }

    /// Drops the rows that began waiting after `mark` (their step was taken back).
    pub(crate) fn drop_waiting(&mut self, mark: usize) {
        self.waiting.truncate(mark);
    }

    /// Queues a command's row; its number.
    pub(crate) fn queue(&mut self, mut row: CallRecord, request: Option<RequestRecord>) -> u64 {
        self.given += 1;
        row.seq = self.given;
        self.waiting.push(Waiting { row, request });
        self.given
    }

    fn push_row(&mut self, row: CallRecord) {
        self.rows.push_back(row);
        while self.rows.len() > JOURNAL_ROWS {
            self.rows.pop_front();
        }
    }

    fn push_entry(&mut self, entry: Entry) {
        self.ledger
            .retain(|e| e.record.request_id != entry.record.request_id);
        self.ledger.push_back(entry);
        while self.ledger.len() > REQUEST_LEDGER {
            self.ledger.pop_front();
        }
    }

    /// A copy for a reading copy of the document: no rows, the same ledger.
    pub(crate) fn reading_copy(&self) -> Calls {
        Calls {
            rows: VecDeque::new(),
            waiting: Vec::new(),
            given: self.given,
            ledger: self.ledger.clone(),
        }
    }

    /// Takes the last recorded row off the journal (a reading copy hands its
    /// job's row on, patch.rs).
    pub(crate) fn take_last_row(&mut self) -> Option<CallRecord> {
        self.rows.pop_back()
    }

    /// Takes request `id` off the ledger (patch.rs).
    pub(crate) fn take_request(&mut self, id: &str) -> Option<RequestRecord> {
        let i = self
            .ledger
            .iter()
            .rposition(|e| e.record.request_id == id)?;
        self.ledger.remove(i).map(|e| e.record)
    }

    /// Undo or redo of step `step`.
    pub(crate) fn set_status(&mut self, step: u64, status: CallStatus) {
        for row in self.rows.iter_mut().filter(|r| r.step == Some(step)) {
            row.status = status;
        }
        for entry in self.ledger.iter_mut().filter(|e| e.step == Some(step)) {
            entry.record.status = status;
        }
    }
}

/// The ids of what `ops` added, changed and removed, in that order, each list
/// cut at [`JOURNAL_IDS`] (whether one was).
pub(crate) fn ids_of(ops: &[Op]) -> (Vec<String>, Vec<String>, Vec<String>, bool) {
    // The first state before and the last after, by persistent id, in order.
    let mut first: HashMap<Uuid, (bool, bool)> = HashMap::new();
    let mut order: Vec<Uuid> = Vec::new();
    for op in ops {
        let (uid, before, after) = match op {
            Op::Add(s) => (s.uid, false, true),
            Op::Remove(s) => (s.uid, true, false),
            Op::Update { after, .. } => (after.uid, true, true),
            _ => continue,
        };
        match first.get_mut(&uid) {
            Some(state) => state.1 = after,
            None => {
                first.insert(uid, (before, after));
                order.push(uid);
            }
        }
    }
    let (mut created, mut changed, mut removed) = (Vec::new(), Vec::new(), Vec::new());
    let mut truncated = false;
    for uid in order {
        let list = match first.get(&uid) {
            Some((false, true)) => &mut created,
            Some((true, true)) => &mut changed,
            Some((true, false)) => &mut removed,
            _ => continue,
        };
        if list.len() < JOURNAL_IDS {
            list.push(uid.to_string());
        } else {
            truncated = true;
        }
    }
    (created, changed, removed, truncated)
}

/// A row with nothing but its kind, its name and its revisions.
fn bare(kind: CallKind, label: &str, base: u64, revision: u64) -> CallRecord {
    CallRecord {
        seq: 0,
        kind,
        label: (!label.is_empty()).then(|| label.to_owned()),
        command: None,
        version: None,
        input: None,
        input_digest: None,
        request_id: None,
        fingerprint: None,
        base: base.to_string(),
        revision: revision.to_string(),
        context: None,
        algorithm: None,
        seed: None,
        created: Vec::new(),
        changed: Vec::new(),
        removed: Vec::new(),
        truncated: false,
        warnings: Vec::new(),
        parts: Vec::new(),
        effects: Vec::new(),
        step: None,
        status: CallStatus::Done,
    }
}

/// What a part's edit is called in the journal.
fn part_label(part: ProjectPart) -> &'static str {
    match part {
        ProjectPart::Name => "Çizimin adı",
        ProjectPart::Settings => "Proje ayarları",
        ProjectPart::Styles => "Stil kitaplığı",
        ProjectPart::Sheets => "Pafta kitabı",
        ProjectPart::LayerFlags => "Katmanlar",
    }
}

impl Document {
    /// Records the waiting rows with their step `step` (none: no undo step),
    /// and their requests in the ledger. With none waiting and `edit`, the
    /// last undo step was recorded without a command: its edit row.
    pub(crate) fn flush_calls(&mut self, step: Option<u64>, edit: bool) {
        let revision = self.edits;
        if self.calls.waiting.is_empty() {
            if edit && let Some(last) = self.last_step() {
                let (created, changed, removed, truncated) = ids_of(&last.ops);
                let mut row = bare(
                    CallKind::Edit,
                    &last.label,
                    revision.saturating_sub(1),
                    revision,
                );
                row.created = created;
                row.changed = changed;
                row.removed = removed;
                row.truncated = truncated;
                row.step = step;
                self.calls.given += 1;
                row.seq = self.calls.given;
                self.calls.push_row(row);
            }
            return;
        }
        let waiting = std::mem::take(&mut self.calls.waiting);
        for Waiting { mut row, request } in waiting {
            row.step = step;
            row.revision = revision.to_string();
            row.status = CallStatus::Done;
            let call = row.seq;
            self.calls.push_row(row);
            if let Some(mut record) = request {
                record.revision = revision.to_string();
                record.status = CallStatus::Done;
                self.calls.push_entry(Entry {
                    record,
                    call: Some(call),
                    step,
                });
            }
        }
    }

    /// A part outside the undo history written without a command: its edit
    /// row, joined to the last row when that was the same part's.
    pub(crate) fn note_part(&mut self, part: ProjectPart, base: u64) {
        let revision = self.edits;
        if let Some(last) = self.calls.rows.back_mut()
            && last.kind == CallKind::Edit
            && last.step.is_none()
            && last.parts == [part]
            && last.revision == base.to_string()
        {
            last.revision = revision.to_string();
            return;
        }
        let mut row = bare(CallKind::Edit, part_label(part), base, revision);
        row.parts = vec![part];
        self.calls.given += 1;
        row.seq = self.calls.given;
        self.calls.push_row(row);
    }

    /// A part outside the undo history is about to be written: kept by the
    /// open step, if there is one; whether there is (`keep`).
    pub(crate) fn before_part(&mut self, part: ProjectPart) -> bool {
        self.keep(part)
    }

    /// A part outside the undo history was written (`changed`): an edit row
    /// when no step is open (`framed` false).
    pub(crate) fn after_part(&mut self, part: ProjectPart, framed: bool, changed: bool, base: u64) {
        if changed && !framed {
            self.note_part(part, base);
        }
    }

    /// A command's row written outside any step: a command that wrote a
    /// file and nothing of the document (docs/adr/0253 §10). Recorded at
    /// once, or with the open transaction or group. Its number.
    pub fn record_call(&mut self, row: CallRecord, request: Option<RequestRecord>) -> u64 {
        let seq = self.calls.queue(row, request);
        if !self.is_busy() {
            self.flush_calls(None, false);
        }
        seq
    }

    /// The journal's rows, oldest first (`kentos.calls/1`).
    pub fn journal(&self) -> impl Iterator<Item = &CallRecord> {
        self.calls.rows.iter()
    }

    /// The journal's row `seq`, recorded or still waiting.
    pub fn journal_row(&self, seq: u64) -> Option<&CallRecord> {
        self.calls.rows.iter().find(|r| r.seq == seq).or_else(|| {
            self.calls
                .waiting
                .iter()
                .map(|w| &w.row)
                .find(|r| r.seq == seq)
        })
    }

    /// The request `id` executed here, recorded or still waiting in an open
    /// group; its record (a waiting one is done, its revision not yet final).
    pub fn request(&self, id: &str) -> Option<&RequestRecord> {
        self.calls
            .waiting
            .iter()
            .rev()
            .filter_map(|w| w.request.as_ref())
            .find(|r| r.request_id == id)
            .or_else(|| {
                self.calls
                    .ledger
                    .iter()
                    .rev()
                    .map(|e| &e.record)
                    .find(|r| r.request_id == id)
            })
    }

    /// Gives row `call`'s request its answer (the answer is made once the
    /// write is recorded); a larger answer than the ledger keeps stays none.
    pub fn set_request_answer(&mut self, call: u64, answer: String) {
        if answer.len() > kentos_contracts::REQUEST_ANSWER_LIMIT {
            return;
        }
        if let Some(w) = self.calls.waiting.iter_mut().find(|w| w.row.seq == call) {
            if let Some(r) = &mut w.request {
                r.answer = Some(answer);
            }
            return;
        }
        if let Some(e) = self.calls.ledger.iter_mut().find(|e| e.call == Some(call)) {
            e.record.answer = Some(answer);
        }
    }

    /// The ledger as the file keeps it, oldest first.
    pub fn requests(&self) -> Vec<RequestRecord> {
        self.calls.ledger.iter().map(|e| e.record.clone()).collect()
    }

    /// The ledger a file kept (on opening it).
    pub(crate) fn load_requests(&mut self, requests: Vec<RequestRecord>) {
        self.calls.ledger.clear();
        for record in requests {
            self.calls.push_entry(Entry {
                record,
                call: None,
                step: None,
            });
        }
    }
}
