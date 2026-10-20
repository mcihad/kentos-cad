//! A catalog command by name: the document's commands through the
//! application's registry (`kentos_native_application::registry`, the
//! handlers the tools run), the commands that write files through
//! [`crate::file_commands`]; the answer is the `CommandResult` as the
//! catalog writes it (`status`-tagged), or by the protocol a `CallAnswer`
//! (docs/adr/0253).

use kentos_commands::{CallAnswer, CallEnvelope, CallOp, FILE_COMMANDS};
use kentos_domain::Document;
use kentos_geometry_core::store::Store;
use kentos_native_application::{DESKTOP_COMMANDS, ExecutionContext, call, registry};
use serde_json::Value;

use crate::HeadlessError;
use crate::file_commands;

/// The commands whose input names the open drawing's own file (`drawing`: relative paths are found from its folder,
/// a reference to it is a cycle), from the catalog.
fn naming_their_drawing() -> &'static std::collections::BTreeSet<String> {
    static NAMES: std::sync::OnceLock<std::collections::BTreeSet<String>> =
        std::sync::OnceLock::new();
    NAMES.get_or_init(|| {
        kentos_commands::catalog::catalog()
            .commands
            .into_iter()
            .filter(|c| c.input.pointer("/properties/drawing").is_some())
            .map(|c| c.id)
            .collect()
    })
}

/// `input` with the open drawing's file as its `drawing` when the command takes one and the caller gave none: the
/// host knows where the drawing came from, Python and MCP need not say it (and MCP cannot: its `drawing` is the
/// handle). A drawing with no file leaves it out.
pub fn with_own_file(command: &str, input: &mut Value, drawing: Option<&std::path::Path>) {
    let (Some(path), Some(fields)) = (drawing, input.as_object_mut()) else {
        return;
    };
    if naming_their_drawing().contains(command) && fields.get("drawing").is_none_or(Value::is_null)
    {
        let path = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
        fields.insert("drawing".into(), Value::String(path.display().to_string()));
    }
}

/// The commands this host runs, by name and version: the desktop's document
/// commands (`DESKTOP_COMMANDS`) and the ones that write files (`FILE_COMMANDS`).
pub fn commands() -> Vec<(&'static str, u32)> {
    DESKTOP_COMMANDS
        .iter()
        .chain(FILE_COMMANDS)
        .copied()
        .collect()
}

/// What a call does (TODOS.md CMD-04): `validate` and `plan` write nothing;
/// `execute` writes one change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Validate,
    Plan,
    Execute,
}

impl Op {
    pub fn parse(text: &str) -> Result<Self, HeadlessError> {
        match text {
            "validate" => Ok(Op::Validate),
            "plan" => Ok(Op::Plan),
            "execute" => Ok(Op::Execute),
            other => Err(HeadlessError::new(
                "invalid_input",
                format!("“{other}” bir çağrı biçimi değil: validate, plan ya da execute olmalı."),
            )),
        }
    }

    /// The protocol's form of it.
    pub fn call_op(self) -> CallOp {
        match self {
            Op::Validate => CallOp::Validate,
            Op::Plan => CallOp::Plan,
            Op::Execute => CallOp::Execute,
        }
    }
}

/// Runs `op` of `command` (version `version`, or the host's when absent)
/// on `doc` with `input`; the command's whole answer.
pub fn run(
    doc: &mut Document,
    command: &str,
    version: Option<u32>,
    op: Op,
    input: Value,
) -> Result<Value, HeadlessError> {
    let Some((_, own)) = commands().into_iter().find(|(name, _)| *name == command) else {
        return Err(refusal(command));
    };
    if let Some(v) = version
        && v != own
    {
        return Err(HeadlessError::new(
            "unknown_version",
            format!("{command} için {v}. sürüm yok; bu sürüm {own}. sürümü çalıştırır."),
        ));
    }
    if file_commands::is_file_command(command) {
        return file_commands::run(doc, command, op.call_op(), input)
            .map_err(|message| HeadlessError::new("invalid_input", message));
    }
    let mut cx = ExecutionContext::new(doc);
    registry::run(&mut cx, command, op.call_op(), input).map_err(|unrun| match unrun.code {
        "unknown_command" => refusal(command),
        _ => HeadlessError::new("invalid_input", unrun.message),
    })
}

/// A call by the protocol (docs/adr/0253 §4): never a host's error, every
/// refusal an answer; a server's command is refused with its reason.
pub fn call_envelope(
    doc: &mut Document,
    store: Option<&Store>,
    envelope: CallEnvelope,
) -> CallAnswer {
    if file_commands::is_file_command(&envelope.command) {
        return file_commands::call(doc, envelope);
    }
    if registry::version_of(&envelope.command).is_none() {
        let e = refusal(&envelope.command);
        return CallAnswer::of(kentos_commands::CommandResult::Failed {
            error: kentos_commands::CommandError {
                code: e.code.to_owned(),
                message: e.message,
                path: Some("command".into()),
                ..kentos_commands::CommandError::default()
            },
        });
    }
    call::call(doc, store, envelope)
}

/// Why a command is not run here: the server's, or unknown.
fn refusal(command: &str) -> HeadlessError {
    let listed = kentos_commands::catalog::catalog()
        .commands
        .into_iter()
        .find(|c| c.id == command);
    match listed {
        Some(c) => HeadlessError::new(
            "server_command",
            format!(
                "{command} ({}) sunucuda çalışır, açık çizimde değil. Bir sunucu bağlantısıyla \
                 çağırın (Python'da kentos.cad.Connection).",
                c.title
            ),
        ),
        None => HeadlessError::new(
            "unknown_command",
            format!("“{command}” katalogda yok. Komutların listesi kataloğun kendisindedir."),
        ),
    }
}
