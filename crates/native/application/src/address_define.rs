//! `cad.addresses.define` v1 (docs/adr/0229 §14): an address locator
//! written into the project's settings (`set`: in place of the one of its
//! id, else last) or taken away (`remove`). A locator is a project setting,
//! as a network is: the drawing becomes dirty and no undo step is written.
//! The shared cases are `fixtures/commands/v1/cad.addresses.define.json`.
//!
//! The checks, in the contract's order: what the operation needs
//! (`locator`, `id`); the locator's and the list's rules; the expected
//! revision; the locator removed is the project's. A layer the drawing does
//! not have is a warning.

use kentos_commands::{
    AddressLocator, AddressesDefine, AddressesDefineOperation, AddressesDefined, CommandResult,
    CommandWarning, ProjectSettings, address_locators_problem,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;

struct Checked {
    locators: Vec<AddressLocator>,
    warnings: Vec<CommandWarning>,
}

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

fn check(doc: &Document, input: &AddressesDefine) -> Result<Checked, Stop> {
    let own = &doc.settings().address_locators;
    if input.operation == AddressesDefineOperation::Set && input.locator.is_none() {
        return Err(fail(
            codes::NO_LOCATOR,
            "Yazılacak adres verisi (locator) verilmedi; tanımını verin.".into(),
            "locator",
        ));
    }
    if input.operation == AddressesDefineOperation::Remove
        && input.id.as_deref().is_none_or(str::is_empty)
    {
        return Err(fail(
            codes::NO_ID,
            "Silinecek adres verisinin kimliğini (id) verin.".into(),
            "id",
        ));
    }
    let mut warnings = Vec::new();
    let locators: Vec<AddressLocator> = match (&input.operation, &input.locator) {
        (AddressesDefineOperation::Set, Some(l)) => {
            let mut list = own.clone();
            match list.iter().position(|o| o.id == l.id) {
                Some(at) => list[at] = l.clone(),
                None => list.push(l.clone()),
            }
            if let Some(wrong) = l.problem().or_else(|| address_locators_problem(&list)) {
                return Err(fail(codes::INVALID_LOCATOR, wrong, "locator"));
            }
            for layer in l.layers() {
                if doc.layers().get(layer).is_none() {
                    warnings.push(CommandWarning {
                        code: codes::UNKNOWN_LAYER.into(),
                        message: format!(
                            "“{layer}” kimlikli katman çizimde yok; adres dizini kurulurken bu katman atlanır."
                        ),
                        path: Some("locator".into()),
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
    if input.operation == AddressesDefineOperation::Remove && locators.len() == own.len() {
        let id = input.id.as_deref().unwrap_or_default();
        return Err(fail(
            codes::UNKNOWN_LOCATOR,
            format!(
                "“{id}” kimlikli adres verisi projede yok. Var olan bir tanımın kimliğini verin."
            ),
            "id",
        ));
    }
    Ok(Checked { locators, warnings })
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &AddressesDefine) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed {
            output: (),
            warnings: c.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &AddressesDefine) -> CommandResult<AddressesDefined> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed {
            output: AddressesDefined {
                locators: c.locators,
                revision: cx.doc.revision().to_string(),
            },
            warnings: c.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// Writes the project's locators; a setting, not an undo step.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: AddressesDefine,
) -> CommandResult<AddressesDefined> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_ADDRESSES_DEFINE,
        kentos_commands::CAD_ADDRESSES_DEFINE_VERSION,
        input,
        write_execute,
    )
}

/// [`execute`]'s write; `call::recorded` runs it as one step of the
/// document, all or nothing, and journals it (docs/adr/0253 §1, §3).
fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: AddressesDefine,
) -> CommandResult<AddressesDefined> {
    let checked = match check(cx.doc, &input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let doc = &mut *cx.doc;
    doc.set_settings(ProjectSettings {
        address_locators: checked.locators.clone(),
        ..doc.settings().clone()
    });
    CommandResult::Completed {
        output: AddressesDefined {
            locators: checked.locators,
            revision: doc.revision().to_string(),
        },
        warnings: checked.warnings,
    }
}
