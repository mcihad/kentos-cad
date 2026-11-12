//! `cad.linearref.define` v1 (docs/adr/0569 §2): an event link written into the project's settings (`set`: in place
//! of the one of its id, else last) or taken away (`remove`). A link is a project setting, as a network is: the drawing
//! becomes dirty and no undo step is written, and the events stay where they are until a later write touches them
//! ([`crate::event_follow`]). The desktop's handler over the native document; the shared cases are in
//! `fixtures/commands/v1/cad.linearref.define.json`.
//!
//! The checks, in the contract's order: what the operation needs (`link`, `id`); the link's and the list's rules; the
//! expected revision; the link removed is the project's. A layer the drawing does not have is a warning.

use kentos_commands::{
    CommandResult, CommandWarning, EventLink, LinearRefDefine, LinearRefDefineOperation,
    LinearRefDefined, ProjectSettings, event_links_problem,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;

struct Checked {
    links: Vec<EventLink>,
    warnings: Vec<CommandWarning>,
}

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

fn check(doc: &Document, input: &LinearRefDefine) -> Result<Checked, Stop> {
    let own = &doc.settings().linear_events;
    if input.operation == LinearRefDefineOperation::Set && input.link.is_none() {
        return Err(fail(
            codes::NO_EVENT_LINK,
            "Yazılacak olay bağı (link) verilmedi; bağın tanımını verin.".into(),
            "link",
        ));
    }
    if input.operation == LinearRefDefineOperation::Remove
        && input.id.as_deref().is_none_or(str::is_empty)
    {
        return Err(fail(
            codes::NO_ID,
            "Silinecek olay bağının kimliğini (id) verin.".into(),
            "id",
        ));
    }
    let mut warnings = Vec::new();
    let links: Vec<EventLink> = match (&input.operation, &input.link) {
        (LinearRefDefineOperation::Set, Some(l)) => {
            let mut list = own.clone();
            match list.iter().position(|o| o.id == l.id) {
                Some(at) => list[at] = l.clone(),
                None => list.push(l.clone()),
            }
            if let Some(wrong) = event_links_problem(&list) {
                return Err(fail(codes::INVALID_EVENT_LINK, wrong, "link"));
            }
            for layer in l.layers() {
                if doc.layers().get(layer).is_none() {
                    warnings.push(CommandWarning {
                        code: codes::UNKNOWN_LAYER.into(),
                        message: format!("“{layer}” kimlikli katman çizimde yok; bağ bu katman gelene dek bir şey yapmaz."),
                        path: Some("link".into()),
                    });
                }
            }
            list
        }
        _ => {
            let id = input.id.as_deref().unwrap_or_default();
            own.iter().filter(|o| o.id != id).cloned().collect()
        }
    };
    checks::revision(doc, input.expected_revision.as_deref())?;
    if input.operation == LinearRefDefineOperation::Remove && links.len() == own.len() {
        let id = input.id.as_deref().unwrap_or_default();
        return Err(fail(
            codes::UNKNOWN_EVENT_LINK,
            format!("“{id}” kimlikli olay bağı projede yok. Var olan bir bağın kimliğini verin."),
            "id",
        ));
    }
    Ok(Checked { links, warnings })
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &LinearRefDefine) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed {
            output: (),
            warnings: c.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &LinearRefDefine) -> CommandResult<LinearRefDefined> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed {
            output: LinearRefDefined {
                links: c.links,
                revision: cx.doc.revision().to_string(),
            },
            warnings: c.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// Writes the project's links; a setting, not an undo step.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: LinearRefDefine,
) -> CommandResult<LinearRefDefined> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_LINEARREF_DEFINE,
        kentos_commands::CAD_LINEARREF_DEFINE_VERSION,
        input,
        write_execute,
    )
}

/// [`execute`]'s write; `call::recorded` runs it as one step of the document, all or nothing, and journals it.
fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: LinearRefDefine,
) -> CommandResult<LinearRefDefined> {
    let checked = match check(cx.doc, &input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let doc = &mut *cx.doc;
    doc.set_settings(ProjectSettings {
        linear_events: checked.links.clone(),
        ..doc.settings().clone()
    });
    CommandResult::Completed {
        output: LinearRefDefined {
            links: checked.links,
            revision: doc.revision().to_string(),
        },
        warnings: checked.warnings,
    }
}
