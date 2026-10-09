//! `cad.network.define` v1 (docs/adr/0209 §11): a network's definition
//! written into the project's settings (`set`: in place of the one of its
//! id, else last) or taken away (`remove`). A network is a project setting,
//! as the topology rules are: the drawing becomes dirty and no undo step is
//! written. The desktop's handler over the native document; the web's is
//! `apps/web/src/product/networkDefine.ts`. Both pass the shared cases in
//! `fixtures/commands/v1/cad.network.define.json`.
//!
//! The checks, in the contract's order: what the operation needs
//! (`network`, `id`); the network's and the list's rules; the expected
//! revision; the network removed is the project's. A layer the drawing does
//! not have is a warning.

use kentos_contracts::{
    CommandResult, CommandWarning, NetworkDef, NetworkDefine, NetworkDefineOperation,
    NetworkDefined, ProjectSettings, networks_problem,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;

struct Checked {
    networks: Vec<NetworkDef>,
    warnings: Vec<CommandWarning>,
}

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::Failed(error(code, message, Some(path.to_owned())))
}

fn check(doc: &Document, input: &NetworkDefine) -> Result<Checked, Stop> {
    let own = &doc.settings().networks;
    if input.operation == NetworkDefineOperation::Set && input.network.is_none() {
        return Err(fail(
            codes::NO_NETWORK,
            "Yazılacak ağ (network) verilmedi; ağın tanımını verin.".into(),
            "network",
        ));
    }
    if input.operation == NetworkDefineOperation::Remove
        && input.id.as_deref().is_none_or(str::is_empty)
    {
        return Err(fail(
            codes::NO_ID,
            "Silinecek ağın kimliğini (id) verin.".into(),
            "id",
        ));
    }
    let mut warnings = Vec::new();
    let networks: Vec<NetworkDef> = match (&input.operation, &input.network) {
        (NetworkDefineOperation::Set, Some(n)) => {
            let mut list = own.clone();
            match list.iter().position(|o| o.id == n.id) {
                Some(at) => list[at] = n.clone(),
                None => list.push(n.clone()),
            }
            if let Some(wrong) = n.problem().or_else(|| networks_problem(&list)) {
                return Err(fail(codes::INVALID_NETWORK, wrong, "network"));
            }
            for layer in n.layers() {
                if doc.layers().get(layer).is_none() {
                    warnings.push(CommandWarning {
                        code: codes::UNKNOWN_LAYER.into(),
                        message: format!("“{layer}” kimlikli katman çizimde yok; ağ kurulurken bu katman atlanır."),
                        path: Some("network".into()),
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
    if input.operation == NetworkDefineOperation::Remove && networks.len() == own.len() {
        let id = input.id.as_deref().unwrap_or_default();
        return Err(fail(
            codes::UNKNOWN_NETWORK,
            format!("“{id}” kimlikli ağ projede yok. Var olan bir ağın kimliğini verin."),
            "id",
        ));
    }
    Ok(Checked { networks, warnings })
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &NetworkDefine) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed {
            output: (),
            warnings: c.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &NetworkDefine) -> CommandResult<NetworkDefined> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed {
            output: NetworkDefined {
                networks: c.networks,
                revision: cx.doc.revision().to_string(),
            },
            warnings: c.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// Writes the project's networks; a setting, not an undo step.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: NetworkDefine,
) -> CommandResult<NetworkDefined> {
    let checked = match check(cx.doc, &input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let doc = &mut *cx.doc;
    doc.set_settings(ProjectSettings {
        networks: checked.networks.clone(),
        ..doc.settings().clone()
    });
    CommandResult::Completed {
        output: NetworkDefined {
            networks: checked.networks,
            revision: doc.revision().to_string(),
        },
        warnings: checked.warnings,
    }
}
