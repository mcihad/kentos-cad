//! `road.pay.edit` v1 (plan/road.md Y11.4, docs/adr/0909 §4): the project's pay item book written (new, or the given
//! fields replacing its) or removed; one undo step “Poz defteri”. The shared cases are
//! `fixtures/commands/v1/road.pay.edit.json`.

use kentos_commands::{CommandResult, PayBookDef, PayEdit, PayWritten, RoadEditOperation};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::road_pay as common;
use crate::road_sections::judged;
use crate::road_terrains::fail;

pub(crate) const STEP: &str = "Poz defteri";

fn check(doc: &kentos_domain::Document, input: &PayEdit) -> Result<Option<PayBookDef>, Stop> {
    common::road_project(doc)?;
    let old = doc.road_pay();
    let next = match input.operation {
        RoadEditOperation::Remove => {
            common::book(doc)?;
            None
        }
        RoadEditOperation::Rebind => {
            return Err(fail(codes::INVALID_PAY_BOOK, "Poz defteri bir koridora bağlı değil; yeniden bağlanmaz.".into(), "operation"));
        }
        RoadEditOperation::Set => match old {
            Some(old) => {
                let given = input.source.is_some()
                    || input.version.is_some()
                    || input.currency.is_some()
                    || input.items.is_some()
                    || input.rules.is_some();
                if !given {
                    return Err(fail(codes::INVALID_PAY_BOOK, "Değiştirilecek bir şey yok: defterin bir alanını verin.".into(), ""));
                }
                let mut d = old.clone();
                if let Some(v) = &input.source {
                    d.source = v.clone();
                }
                if let Some(v) = &input.version {
                    d.version = v.clone();
                }
                if let Some(v) = &input.currency {
                    d.currency = v.clone();
                }
                if let Some(v) = &input.items {
                    d.items = v.clone();
                }
                if let Some(v) = &input.rules {
                    d.rules = v.clone();
                }
                if d != *old {
                    d.revision = old.revision.saturating_add(1);
                }
                Some(d)
            }
            None => {
                let missing = [("source", input.source.is_none()), ("version", input.version.is_none()), ("items", input.items.is_none())]
                    .into_iter()
                    .find(|(_, m)| *m);
                if let Some((field, _)) = missing {
                    return Err(fail(codes::INVALID_PAY_BOOK, "Yeni poz defteri için source, version ve items verilmeli.".into(), field));
                }
                let (Some(source), Some(version), Some(items)) = (&input.source, &input.version, &input.items) else {
                    return Err(Stop::Cancelled);
                };
                Some(PayBookDef {
                    source: source.clone(),
                    version: version.clone(),
                    revision: 1,
                    currency: input.currency.clone().flatten(),
                    items: items.clone(),
                    rules: input.rules.clone().unwrap_or_default(),
                })
            }
        },
    };
    let mut road = doc.settings().road.clone().unwrap_or_default();
    road.pay = next.clone();
    judged(&road, codes::INVALID_PAY_BOOK, "Poz defteri")?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    Ok(next)
}

fn write(cx: &mut ExecutionContext<'_>, next: Option<PayBookDef>) -> CommandResult<PayWritten> {
    let doc = &mut *cx.doc;
    if let Err(r) = doc.set_road_pay(next.clone(), STEP) {
        return fail(codes::INVALID_PAY_BOOK, format!("Poz defteri kurallarına uymuyor: {}.", r.0), "").into();
    }
    CommandResult::Completed { output: PayWritten { book: next, revision: doc.revision().to_string() }, warnings: vec![] }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &PayEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: vec![] },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &PayEdit) -> CommandResult<PayWritten> {
    match check(cx.doc, input) {
        Ok(next) => CommandResult::Completed {
            output: PayWritten { book: next, revision: cx.doc.revision().to_string() },
            warnings: vec![],
        },
        Err(stop) => stop.into(),
    }
}

/// Writes the book: one undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: PayEdit) -> CommandResult<PayWritten> {
    crate::call::recorded(cx, kentos_commands::ROAD_PAY_EDIT, kentos_commands::ROAD_PAY_EDIT_VERSION, input, |cx, input| {
        match check(cx.doc, &input) {
            Ok(next) => write(cx, next),
            Err(stop) => stop.into(),
        }
    })
}

/// The book `input` would write and its quantities, for a window (no step written), or why not.
pub fn preview(doc: &kentos_domain::Document, input: &PayEdit) -> Result<(kentos_commands::PayQueried, Vec<kentos_commands::CommandWarning>), String> {
    let next = check(doc, input).map_err(crate::road_corridors::words)?;
    let mut copy = doc.reading_copy();
    copy.set_road_pay(next, STEP).map_err(|r| r.0)?;
    common::answer(&copy, None).map_err(crate::road_corridors::words)
}
