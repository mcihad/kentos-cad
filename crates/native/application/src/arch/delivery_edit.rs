//! `cad.delivery.edit` v1 (“Teslim kaydı”, docs/adr/8099; plan/bim-plan.md BIM-15): the drawing's delivery record
//! (`settings.arch.deliveries`) as one undo step: a package's delivery added, the authority's answer given (accepted, or
//! returned with its reason and the review notes binding the reasons to objects), a mistaken record taken away. The
//! shared cases are `fixtures/commands/v1/cad.delivery.edit.json` (`scripts/fixtures/delivery_cases.py`).
//!
//! The checks, in order (the first that fails answers): the ids' form (`invalid_uid`); the record named exists
//! (`delivery_not_found`), an added id does not (`duplicate_delivery`); a response's notes exist
//! (`review_note_not_found`), each once (`invalid_delivery`); the record's own checks (`invalid_delivery`, its field's
//! path); the count (`too_many_deliveries`); the expected revision.

use kentos_commands::{
    CommandResult, Delivery, DeliveryChange, DeliveryEdit, DeliveryEdited, DeliveryResponse, MAX_DELIVERIES,
    is_delivery_id,
};
use kentos_domain::{Document, Uuid};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;

/// The undo step's name.
pub const STEP: &str = "Teslim kaydı";

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.into())))
}

/// A text as given: its edges trimmed, an empty one none.
fn said(t: &Option<String>) -> Option<String> {
    t.as_deref().map(str::trim).filter(|t| !t.is_empty()).map(str::to_owned)
}

/// The record the change leaves and the changed record's id and number; refused with the first check that fails.
fn check(doc: &Document, input: &DeliveryEdit) -> Result<(Vec<Delivery>, String, u32), Stop> {
    let mut next = doc.deliveries().to_vec();
    let find = |list: &[Delivery], id: &str| -> Result<usize, Stop> {
        if !is_delivery_id(id) {
            return Err(fail(codes::INVALID_UID, format!("“{id}” küçük harfli, tireli bir UUID değil."), "change.id"));
        }
        list.iter().position(|d| d.id == id).ok_or_else(|| {
            fail(codes::DELIVERY_NOT_FOUND, format!("“{id}” kimlikli teslim kaydı çizimde yok."), "change.id")
        })
    };
    let (id, number) = match &input.change {
        DeliveryChange::Add { id, revision, package, to, channel, at, by, receipt } => {
            if let Some(id) = id {
                if !is_delivery_id(id) {
                    return Err(fail(
                        codes::INVALID_UID,
                        format!("“{id}” küçük harfli, tireli bir UUID değil."),
                        "change.id",
                    ));
                }
                if next.iter().any(|d| &d.id == id) {
                    return Err(fail(
                        codes::DUPLICATE_DELIVERY,
                        format!("“{id}” kimlikli bir teslim kaydı çizimde zaten var."),
                        "change.id",
                    ));
                }
            }
            let id = id.clone().unwrap_or_else(|| Uuid::now_v7().to_string());
            let number = next.iter().map(|d| d.number).max().unwrap_or(0) + 1;
            let mut package = package.clone();
            package.path = said(&package.path);
            package.source = said(&package.source);
            next.push(Delivery {
                id: id.clone(),
                number,
                revision: revision.trim().to_owned(),
                package,
                to: to.trim().to_owned(),
                channel: *channel,
                at: at.trim().to_owned(),
                by: said(by),
                receipt: said(receipt),
                response: None,
            });
            (id, number)
        }
        DeliveryChange::Respond { id, outcome, at, reason, notes, receipt } => {
            let i = find(&next, id)?;
            for (j, note) in notes.iter().enumerate() {
                let path = format!("change.notes[{j}]");
                if !is_delivery_id(note) {
                    return Err(fail(codes::INVALID_UID, format!("“{note}” küçük harfli, tireli bir UUID değil."), &path));
                }
                if !doc.settings().review_notes.iter().any(|n| &n.id == note) {
                    return Err(fail(
                        codes::REVIEW_NOTE_NOT_FOUND,
                        format!("“{note}” kimlikli inceleme notu çizimde yok; gerekçeyi önce inceleme notu olarak ekleyin."),
                        &path,
                    ));
                }
            }
            if let Some(j) = (1..notes.len()).find(|&j| notes[..j].contains(&notes[j])) {
                return Err(fail(
                    codes::INVALID_DELIVERY,
                    format!("“{}” inceleme notu cevapta iki kez verildi; her notu bir kez verin.", notes[j]),
                    &format!("change.notes[{j}]"),
                ));
            }
            next[i].response = Some(DeliveryResponse {
                outcome: *outcome,
                at: at.trim().to_owned(),
                reason: said(reason),
                notes: notes.clone(),
                receipt: said(receipt),
            });
            (next[i].id.clone(), next[i].number)
        }
        DeliveryChange::Remove { id } => {
            let i = find(&next, id)?;
            let gone = next.remove(i);
            (gone.id, gone.number)
        }
    };
    if let Some((path, why)) = Delivery::list_problem(&next) {
        // The record's field, as the change names it.
        let i = next.iter().position(|d| d.id == id);
        let field = path.splitn(3, '.').nth(2).unwrap_or("");
        let path = match (i, field) {
            (Some(_), f) if f.starts_with("response.") => format!("change.{}", &f["response.".len()..]),
            (Some(_), f) if !f.is_empty() => format!("change.{f}"),
            _ => "change".to_owned(),
        };
        let code = if next.len() > MAX_DELIVERIES { codes::TOO_MANY_DELIVERIES } else { codes::INVALID_DELIVERY };
        return Err(fail(code, why, &path));
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    Ok((next, id, number))
}

pub fn validate(cx: &ExecutionContext<'_>, input: &DeliveryEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// What it would write, nothing written.
pub fn plan(cx: &ExecutionContext<'_>, input: &DeliveryEdit) -> CommandResult<DeliveryEdited> {
    match check(cx.doc, input) {
        Ok((deliveries, id, number)) => CommandResult::Completed {
            output: DeliveryEdited { id, number, deliveries, revision: cx.doc.revision().to_string() },
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: DeliveryEdit) -> CommandResult<DeliveryEdited> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_DELIVERY_EDIT,
        kentos_commands::CAD_DELIVERY_EDIT_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: DeliveryEdit) -> CommandResult<DeliveryEdited> {
    let (deliveries, id, number) = match check(cx.doc, &input) {
        Ok(next) => next,
        Err(stop) => return stop.into(),
    };
    let doc = &mut *cx.doc;
    if let Err(e) = doc.set_deliveries(deliveries.clone(), STEP) {
        return CommandResult::Failed { error: error(codes::INVALID_DELIVERY, e.0, Some("change".into())) };
    }
    CommandResult::Completed {
        output: DeliveryEdited { id, number, deliveries, revision: doc.revision().to_string() },
        warnings: Vec::new(),
    }
}
