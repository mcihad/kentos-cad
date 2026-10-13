//! A job's write carried from the reading copy it computed on to the
//! document it was copied from (docs/adr/0253 §9).
//!
//! [`Document::take_patch`] takes, on the copy, what the job's command
//! wrote: its undo step, the project's parts as they are after it, its
//! journal row and its request. [`Document::apply_patch`] writes it to the
//! document as one step:
//!
//! - when the document did not change since the copy was made, as it is:
//!   the same ops, the same slots, the same parts;
//! - when it did, only a write of objects, and only if every object it
//!   changes or removes is still as the copy had it: its additions take new
//!   slots, its changes and removals find their objects by persistent id,
//!   and the linked texts, hatches and dimensions follow on the document;
//! - otherwise nothing, and the reason.

use std::collections::HashMap;
use std::sync::Arc;

use kentos_contracts::{
    CallKind, CallRecord, ProjectPart, ProjectSettings, ProjectSheets, ProjectStyles, RequestRecord,
};

use crate::document::Document;
use crate::history::{Op, Step};
use crate::identity::{Slot, Uuid};
use crate::layers::LayerFlags;
use crate::store::Stored;

/// What a job wrote on its copy (see the module).
#[derive(Clone, Debug)]
pub struct Patch {
    label: String,
    ops: Vec<Op>,
    name: String,
    settings: ProjectSettings,
    styles: ProjectStyles,
    sheets: Option<Arc<ProjectSheets>>,
    flags: LayerFlags,
    active: String,
    row: Option<CallRecord>,
    request: Option<RequestRecord>,
}

impl Patch {
    /// The parts outside the undo history the job changed (its row says).
    fn parts(&self) -> &[ProjectPart] {
        self.row.as_ref().map_or(&[], |r| r.parts.as_slice())
    }
}

/// Why a patch was not written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatchRefused {
    /// The objects that are no longer as the job saw them.
    pub uids: Vec<Uuid>,
    pub message: String,
}

impl Document {
    /// On a reading copy after a job's command: what it wrote (see the
    /// module). The copy keeps nothing of it.
    pub fn take_patch(&mut self) -> Patch {
        let step = self.take_last_step();
        let row = self
            .calls
            .take_last_row()
            .filter(|r| r.kind == CallKind::Command);
        let request = row
            .as_ref()
            .and_then(|r| r.request_id.as_deref())
            .and_then(|id| self.calls.take_request(id));
        Patch {
            label: step.as_ref().map(|s| s.label.clone()).unwrap_or_default(),
            ops: step.map(|s| s.ops).unwrap_or_default(),
            name: self.name.clone(),
            settings: self.settings.clone(),
            styles: self.styles.clone(),
            sheets: self.sheets.clone(),
            flags: self.layers.flags(),
            active: self.layers.active().to_owned(),
            row,
            request,
        }
    }

    /// Writes a job's patch (see the module); `base` is the revision and the
    /// generation the document had when the job's copy was made. The row's
    /// number in the journal when it is written.
    pub fn apply_patch(
        &mut self,
        patch: Patch,
        base: (u64, u64),
    ) -> Result<Option<u64>, PatchRefused> {
        if self.is_busy() {
            return Err(PatchRefused {
                uids: Vec::new(),
                message: "Belgede açık bir işlem ya da grup var; iş o bitince yazılır.".into(),
            });
        }
        let unchanged = self.revision() == base.0 && self.generation() == base.1;
        let ops = if unchanged {
            patch.ops.clone()
        } else {
            self.rebase(&patch)?
        };
        let parts = patch.parts().to_vec();
        let Patch {
            label,
            name,
            settings,
            styles,
            sheets,
            flags,
            active,
            row,
            request,
            ..
        } = patch;
        // The parts the job changed, as it left them (only when nothing changed meanwhile, `rebase`).
        let mut edited = false;
        for part in &parts {
            match part {
                ProjectPart::Name if self.name != name => {
                    self.name.clone_from(&name);
                    edited = true;
                }
                ProjectPart::Settings if self.settings != settings => {
                    self.settings.clone_from(&settings);
                    edited = true;
                }
                ProjectPart::Styles if self.styles != styles => {
                    self.styles.clone_from(&styles);
                    edited = true;
                }
                ProjectPart::Sheets if self.sheets != sheets => {
                    self.sheets.clone_from(&sheets);
                    edited = true;
                }
                ProjectPart::LayerFlags
                    if self.layers.flags() != flags || self.layers.active() != active =>
                {
                    self.layers.restore_flags(&flags);
                    self.layers.restore_active(&active);
                    edited = true;
                }
                _ => {}
            }
        }
        if edited {
            self.mark_edited();
        }
        let call = row.map(|row| self.calls.queue(row, request));
        if ops.is_empty() {
            self.flush_calls(None, false);
            return Ok(call);
        }
        let mut step = Step::named(&label);
        for op in &ops {
            self.apply_op(op);
        }
        step.ops = ops;
        if !unchanged {
            // Another write came between: what follows the objects follows them here.
            self.follow_into(&mut step.ops);
        }
        self.push_step(step);
        Ok(call)
    }

    /// The patch's ops for a document that changed since the copy: only
    /// objects, each it changes or removes as the copy had it; new slots for
    /// what it adds.
    fn rebase(&mut self, patch: &Patch) -> Result<Vec<Op>, PatchRefused> {
        if !patch.parts().is_empty()
            || patch
                .ops
                .iter()
                .any(|op| !matches!(op, Op::Add(_) | Op::Remove(_) | Op::Update { .. }))
        {
            return Err(PatchRefused {
                uids: Vec::new(),
                message: "İş hesaplanırken çizim değişti ve iş katmanlara, bloklara ya da proje ayarlarına da yazıyor; hiçbir şey yazılmadı. İşi yeniden başlatın.".into(),
            });
        }
        let mut stale = Vec::new();
        // Each object as the ops so far leave it (none: removed), by persistent id.
        let mut latest: HashMap<Uuid, Option<Stored>> = HashMap::new();
        // The slots the ops gave or found, by persistent id.
        let mut slots: HashMap<Uuid, Slot> = HashMap::new();
        let mut out = Vec::with_capacity(patch.ops.len());
        for op in &patch.ops {
            match op {
                Op::Update { before, .. } | Op::Remove(before) => {
                    let uid = before.uid;
                    let now = match latest.get(&uid) {
                        Some(state) => state.clone(),
                        None => {
                            let now = self
                                .store
                                .slot_of(uid)
                                .and_then(|s| self.store.get(s))
                                .cloned();
                            // The first touch: the object must be as the job saw it.
                            if !now.as_ref().is_some_and(|n| same_object(n, before)) {
                                stale.push(uid);
                                continue;
                            }
                            now
                        }
                    };
                    let Some(now) = now else {
                        stale.push(uid);
                        continue;
                    };
                    slots.insert(uid, now.slot());
                    if let Op::Update { after, .. } = op {
                        let next = at_slot(after, now.slot());
                        latest.insert(uid, Some(next.clone()));
                        out.push(Op::Update {
                            before: now,
                            after: next,
                        });
                    } else {
                        latest.insert(uid, None);
                        out.push(Op::Remove(now));
                    }
                }
                Op::Add(made) => {
                    // An object the ops took out and put back keeps its slot.
                    let slot = match slots.get(&made.uid) {
                        Some(slot) => *slot,
                        None => self.next_slot_given(),
                    };
                    slots.insert(made.uid, slot);
                    let placed = at_slot(made, slot);
                    latest.insert(made.uid, Some(placed.clone()));
                    out.push(Op::Add(placed));
                }
                _ => {}
            }
        }
        if !stale.is_empty() {
            stale.sort_unstable();
            stale.dedup();
            return Err(PatchRefused {
                uids: stale,
                message: "İş hesaplanırken çizim değişti: işin değiştirdiği nesnelerden bazıları artık iş başladığındaki gibi değil; hiçbir şey yazılmadı. İşi yeniden başlatın.".into(),
            });
        }
        self.store.reserve(out.len());
        Ok(out)
    }

    /// A new slot, given now (never again).
    fn next_slot_given(&mut self) -> Slot {
        let slot = Slot(u32::try_from(self.next_slot).unwrap_or(u32::MAX));
        self.next_slot += 1;
        slot
    }
}

/// Whether two states are the same object as it was: the same data, whatever its slot.
fn same_object(a: &Stored, b: &Stored) -> bool {
    a.uid == b.uid && crate::etag::entity_value(&a.entity) == crate::etag::entity_value(&b.entity)
}

/// `stored` placed at `slot`.
fn at_slot(stored: &Stored, slot: Slot) -> Stored {
    let mut entity = (*stored.entity).clone();
    crate::edit::set_slot(&mut entity, slot);
    Stored {
        uid: stored.uid,
        entity: Arc::new(entity),
    }
}
