//! `cad.review.notes.edit` v1 and `cad.review.notes.list` v1 (docs/adr/1000 §2–§4): review notes bound to objects, a
//! sheet or a named view, kept apart from the design in the project's settings. The edit writes one change as one
//! undo step “İnceleme notu”; a note never changes an object, so one may be bound to an object on a locked or hidden
//! layer. The list says each note's anchors' state against the drawing now: whether the object changed, its block
//! definition (or one nested in it) changed, or it is gone. The shared cases are
//! `fixtures/commands/v1/cad.review.notes.edit.json` and `….list.json` (`scripts/fixtures/review_cases.py`).

use std::collections::HashSet;

use kentos_commands::{
    BlockId, CommandResult, Entity, MAX_REVIEW_ANCHORS, MAX_REVIEW_AUTHOR, MAX_REVIEW_LABEL, MAX_REVIEW_NOTES,
    MAX_REVIEW_REPLIES, REVIEW_NOTE_STEP, ReviewAnchor, ReviewAnchorCheck, ReviewAnchorState, ReviewNote, ReviewNoteOp,
    ReviewNoteState, ReviewNotesEdit, ReviewNotesEdited, ReviewNotesList, ReviewNotesListed, ReviewReply, ReviewStatus,
    review_label_ok, review_text_ok,
};
use kentos_commands::canonical::stamp_of;
use kentos_domain::{Document, Uuid};
use serde_json::{Value, json};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// The stamp of definition `id` and of every definition nested in it, depth first, each once; none when it is not
/// in the drawing.
pub fn block_chain_stamp(doc: &Document, id: BlockId) -> Option<String> {
    doc.block(id)?;
    let mut seen = HashSet::new();
    let mut stamps = Vec::new();
    let mut stack = vec![id];
    while let Some(at) = stack.pop() {
        if !seen.insert(at) {
            continue;
        }
        let Some(def) = doc.block(at) else {
            stamps.push(Value::Null);
            continue;
        };
        stamps.push(Value::String(kentos_domain::etag::block_stamp(def)));
        for e in def.entities.iter().rev() {
            if let Entity::Insert(i) = e {
                stack.push(i.block);
            }
        }
    }
    Some(stamp_of(&json!(stamps)))
}

/// The anchor of object `uid` as the drawing has it now; none when it has no such object.
fn anchor_now(doc: &Document, uid: &str) -> Option<ReviewAnchor> {
    let id = Uuid::parse_str(uid).ok()?;
    let slot = doc.slot_of(id)?;
    let stamp = doc.stamp(slot)?;
    let block_stamp = match doc.get(slot)? {
        Entity::Insert(i) => block_chain_stamp(doc, i.block),
        _ => None,
    };
    Some(ReviewAnchor { uid: uid.to_owned(), stamp, block_stamp })
}

/// How `anchor` stands against the drawing now.
pub fn anchor_state(doc: &Document, anchor: &ReviewAnchor) -> ReviewAnchorState {
    match anchor_now(doc, &anchor.uid) {
        None => ReviewAnchorState::Removed,
        Some(now) if now.stamp != anchor.stamp => ReviewAnchorState::Changed,
        Some(now) if now.block_stamp != anchor.block_stamp => ReviewAnchorState::BlockChanged,
        Some(_) => ReviewAnchorState::Current,
    }
}

fn sheet_known(doc: &Document, id: &str) -> bool {
    doc.sheets()
        .and_then(|s| s.book.get("sheets").and_then(Value::as_array))
        .is_some_and(|list| list.iter().any(|s| s.get("id").and_then(Value::as_str) == Some(id)))
}

fn view_known(doc: &Document, id: &str) -> bool {
    doc.settings().named_views.iter().any(|v| v.id == id)
}

fn text(t: &str, path: &str) -> Result<(), Stop> {
    if review_text_ok(t) {
        return Ok(());
    }
    Err(fail(
        codes::INVALID_REVIEW_NOTE,
        format!(
            "Notun metni boş olmamalı, kenarında boşluk ve satır sonu ile sekme dışında denetim karakteri bulunmamalı, en çok {} karakter olmalı.",
            kentos_commands::MAX_REVIEW_TEXT
        ),
        path,
    ))
}

fn label(t: &Option<String>, most: usize, what: &str, path: &str) -> Result<(), Stop> {
    match t {
        Some(t) if !review_label_ok(t, most) => Err(fail(
            codes::INVALID_REVIEW_NOTE,
            format!("Notun {what} boş olmamalı, kenarında boşluk ve denetim karakteri bulunmamalı, en çok {most} karakter olmalı."),
            path,
        )),
        _ => Ok(()),
    }
}

fn id_text(id: &str, path: &str) -> Result<(), Stop> {
    if checks::is_uid_text(id) {
        return Ok(());
    }
    Err(fail(
        codes::INVALID_UID,
        format!("“{id}” geçerli bir not kimliği değil; kimlik küçük harfli, tireli bir UUID'dir. Kimliği notlar listesinden alın."),
        path,
    ))
}

fn place(notes: &[ReviewNote], id: &str, path: &str) -> Result<usize, Stop> {
    notes.iter().position(|n| n.id == id).ok_or_else(|| {
        fail(
            codes::REVIEW_NOTE_NOT_FOUND,
            format!("“{id}” kimlikli inceleme notu çizimde yok: silinmiş olabilir. Notlar listesini yenileyin."),
            path,
        )
    })
}

/// The anchors `uids` name, stamped from the drawing now: at most [`MAX_REVIEW_ANCHORS`], none twice, each an
/// object the drawing has.
fn anchors(doc: &Document, uids: &[String]) -> Result<Vec<ReviewAnchor>, Stop> {
    if uids.len() > MAX_REVIEW_ANCHORS {
        return Err(fail(
            codes::INVALID_REVIEW_ANCHORS,
            format!("Bir not en çok {MAX_REVIEW_ANCHORS} nesneye bağlanabilir; {} verildi.", uids.len()),
            "change.anchors",
        ));
    }
    let mut seen = HashSet::new();
    for (i, uid) in uids.iter().enumerate() {
        if !seen.insert(uid.as_str()) {
            return Err(fail(
                codes::INVALID_REVIEW_ANCHORS,
                format!("“{uid}” nesnesi iki kez verildi; her nesneyi bir kez bağlayın."),
                &format!("change.anchors[{i}]"),
            ));
        }
    }
    let mut out = Vec::with_capacity(uids.len());
    for (i, uid) in uids.iter().enumerate() {
        let path = format!("change.anchors[{i}]");
        if !checks::is_uid_text(uid) {
            return Err(fail(
                codes::INVALID_UID,
                format!("“{uid}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir."),
                &path,
            ));
        }
        let Some(a) = anchor_now(doc, uid) else {
            return Err(fail(
                codes::ENTITY_NOT_FOUND,
                format!("“{uid}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesneyi seçin."),
                &path,
            ));
        };
        out.push(a);
    }
    Ok(out)
}

fn replies_room(note: &ReviewNote) -> Result<(), Stop> {
    if note.replies.len() >= MAX_REVIEW_REPLIES {
        return Err(fail(
            codes::TOO_MANY_REVIEW_REPLIES,
            format!("{}. not {MAX_REVIEW_REPLIES} yanıtı doldurdu; yeni bir not açın.", note.number),
            "change",
        ));
    }
    Ok(())
}

/// The notes after the change and the note it was about (its number); why not.
fn apply(doc: &Document, input: &ReviewNotesEdit) -> Result<(Vec<ReviewNote>, String, u32), Stop> {
    let mut notes = doc.settings().review_notes.clone();
    let done = match &input.change {
        ReviewNoteOp::Add { id, text: t, anchors: uids, at, sheet, view, revision, origin, author, created_at } => {
            text(t, "change.text")?;
            if let Some(id) = id {
                id_text(id, "change.id")?;
                if notes.iter().any(|n| &n.id == id) {
                    return Err(fail(
                        codes::DUPLICATE_REVIEW_NOTE,
                        format!("“{id}” kimlikli bir inceleme notu çizimde zaten var; başka bir kimlik verin ya da kimliği boş bırakın."),
                        "change.id",
                    ));
                }
            }
            label(author, MAX_REVIEW_AUTHOR, "yazarı", "change.author")?;
            label(created_at, MAX_REVIEW_LABEL, "tarihi", "change.createdAt")?;
            label(revision, MAX_REVIEW_LABEL, "revizyonu", "change.revision")?;
            label(sheet, MAX_REVIEW_LABEL, "paftası", "change.sheet")?;
            label(view, MAX_REVIEW_LABEL, "görünüşü", "change.view")?;
            if let Some(p) = at {
                checks::point(*p, "Notun noktası", "change.at")?;
            }
            let bound = anchors(doc, uids)?;
            if let Some(s) = sheet
                && !sheet_known(doc, s)
            {
                return Err(fail(codes::UNKNOWN_SHEET, format!("“{s}” kimlikli pafta çizimde yok. Var olan bir paftayı seçin."), "change.sheet"));
            }
            if let Some(v) = view
                && !view_known(doc, v)
            {
                return Err(fail(codes::UNKNOWN_VIEW, format!("“{v}” kimlikli adlı görünüş çizimde yok. Var olan bir görünüşü seçin."), "change.view"));
            }
            if notes.len() >= MAX_REVIEW_NOTES {
                return Err(fail(
                    codes::TOO_MANY_REVIEW_NOTES,
                    format!("Çizim {MAX_REVIEW_NOTES} inceleme notunu doldurdu; çözülmüş notları silin."),
                    "change",
                ));
            }
            let number = notes.iter().map(|n| n.number).max().unwrap_or(0) + 1;
            let id = id.clone().unwrap_or_else(|| Uuid::now_v7().to_string());
            notes.push(ReviewNote {
                id: id.clone(),
                number,
                text: t.clone(),
                status: ReviewStatus::Open,
                origin: origin.unwrap_or_default(),
                author: author.clone(),
                created_at: created_at.clone(),
                revision: revision.clone(),
                at: *at,
                sheet: sheet.clone(),
                view: view.clone(),
                anchors: bound,
                replies: Vec::new(),
            });
            (id, number)
        }
        ReviewNoteOp::Update { id, text: t, status, revision } => {
            if let Some(t) = t {
                text(t, "change.text")?;
            }
            id_text(id, "change.id")?;
            let at = place(&notes, id, "change.id")?;
            if t.is_none() && status.is_none() && revision.is_none() {
                return Err(fail(
                    codes::INVALID_REVIEW_NOTE,
                    "Değişiklik boş: metin, durum ya da revizyondan en az birini verin.".into(),
                    "change",
                ));
            }
            label(revision, MAX_REVIEW_LABEL, "revizyonu", "change.revision")?;
            let n = &mut notes[at];
            if let Some(t) = t {
                n.text = t.clone();
            }
            if let Some(s) = status {
                n.status = *s;
            }
            if let Some(r) = revision {
                n.revision = Some(r.clone());
            }
            (n.id.clone(), n.number)
        }
        ReviewNoteOp::Reply { id, text: t, author, created_at, status } => {
            text(t, "change.text")?;
            id_text(id, "change.id")?;
            let at = place(&notes, id, "change.id")?;
            label(author, MAX_REVIEW_AUTHOR, "yazarı", "change.author")?;
            label(created_at, MAX_REVIEW_LABEL, "tarihi", "change.createdAt")?;
            replies_room(&notes[at])?;
            let n = &mut notes[at];
            n.replies.push(ReviewReply { text: t.clone(), author: author.clone(), created_at: created_at.clone() });
            if let Some(s) = status {
                n.status = *s;
            }
            (n.id.clone(), n.number)
        }
        ReviewNoteOp::Rebind { id, anchors: uids } => {
            id_text(id, "change.id")?;
            let at = place(&notes, id, "change.id")?;
            let uids: Vec<String> = match uids {
                Some(u) => u.clone(),
                None => notes[at].anchors.iter().map(|a| a.uid.clone()).collect(),
            };
            let bound = anchors(doc, &uids)?;
            let n = &mut notes[at];
            n.anchors = bound;
            (n.id.clone(), n.number)
        }
        ReviewNoteOp::Remove { id } => {
            id_text(id, "change.id")?;
            let at = place(&notes, id, "change.id")?;
            let n = notes.remove(at);
            (n.id, n.number)
        }
    };
    checks::revision(doc, input.expected_revision.as_deref())?;
    Ok((notes, done.0, done.1))
}

/// Checks the change, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &ReviewNotesEdit) -> CommandResult<()> {
    match apply(cx.doc, input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// What execute would answer now, writing nothing; a new note without an id is told with the id the plan made.
pub fn plan(cx: &ExecutionContext<'_>, input: &ReviewNotesEdit) -> CommandResult<ReviewNotesEdited> {
    match apply(cx.doc, input) {
        Ok((notes, id, number)) => CommandResult::Completed {
            output: ReviewNotesEdited {
                id,
                number,
                notes: notes.len() as u32,
                revision: cx.doc.revision().to_string(),
            },
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// Writes the change as one undo step “İnceleme notu”.
pub fn execute(cx: &mut ExecutionContext<'_>, input: ReviewNotesEdit) -> CommandResult<ReviewNotesEdited> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_REVIEW_NOTES_EDIT,
        kentos_commands::CAD_REVIEW_NOTES_EDIT_VERSION,
        input,
        write,
    )
}

fn write(cx: &mut ExecutionContext<'_>, input: ReviewNotesEdit) -> CommandResult<ReviewNotesEdited> {
    let (notes, id, number) = match apply(cx.doc, &input) {
        Ok(done) => done,
        Err(stop) => return stop.into(),
    };
    cx.doc.name_step(REVIEW_NOTE_STEP);
    let count = notes.len() as u32;
    cx.doc.set_review_notes(notes, REVIEW_NOTE_STEP);
    CommandResult::Completed {
        output: ReviewNotesEdited { id, number, notes: count, revision: cx.doc.revision().to_string() },
        warnings: Vec::new(),
    }
}

/// Every note's state against the drawing now.
pub fn states(doc: &Document) -> Vec<ReviewNoteState> {
    let mut out: Vec<ReviewNoteState> = doc
        .settings()
        .review_notes
        .iter()
        .map(|n| {
            let anchor_states: Vec<ReviewAnchorCheck> = n
                .anchors
                .iter()
                .map(|a| ReviewAnchorCheck { uid: a.uid.clone(), state: anchor_state(doc, a) })
                .collect();
            let sheet_missing = n.sheet.as_deref().is_some_and(|s| !sheet_known(doc, s));
            let view_missing = n.view.as_deref().is_some_and(|v| !view_known(doc, v));
            let stale = sheet_missing
                || view_missing
                || anchor_states.iter().any(|a| a.state != ReviewAnchorState::Current);
            ReviewNoteState { note: n.clone(), stale, anchor_states, sheet_missing, view_missing }
        })
        .collect();
    out.sort_by_key(|s| s.note.number);
    out
}

/// The notes as the input filters them; the counts of all.
pub fn list(doc: &Document, input: &ReviewNotesList) -> ReviewNotesListed {
    let all = states(doc);
    let count = |s: ReviewStatus| all.iter().filter(|n| n.note.status == s).count() as u32;
    let mut listed = ReviewNotesListed {
        open: count(ReviewStatus::Open),
        answered: count(ReviewStatus::Answered),
        resolved: count(ReviewStatus::Resolved),
        stale: all.iter().filter(|n| n.stale).count() as u32,
        revision: doc.revision().to_string(),
        notes: Vec::new(),
    };
    listed.notes = all
        .into_iter()
        .filter(|n| input.status.as_ref().is_none_or(|s| s.contains(&n.note.status)))
        .filter(|n| !input.stale_only || n.stale)
        .collect();
    listed
}

pub mod list_cmd {
    //! `cad.review.notes.list` v1's handlers: a query, the same answer for each stage.
    use super::*;

    pub fn validate(_cx: &ExecutionContext<'_>, _input: &ReviewNotesList) -> CommandResult<()> {
        CommandResult::Completed { output: (), warnings: Vec::new() }
    }

    pub fn plan(cx: &ExecutionContext<'_>, input: &ReviewNotesList) -> CommandResult<ReviewNotesListed> {
        CommandResult::Completed { output: list(cx.doc, input), warnings: Vec::new() }
    }

    pub fn execute(cx: &mut ExecutionContext<'_>, input: ReviewNotesList) -> CommandResult<ReviewNotesListed> {
        CommandResult::Completed { output: list(cx.doc, &input), warnings: Vec::new() }
    }
}
