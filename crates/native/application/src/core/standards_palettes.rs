//! `cad.standards.palettes` v1 (docs/adr/1014): the office tool palettes of the drawing's bound standards, read only.
//! A palette is read from its package where it is bound, never installed in the drawing; a palette id comes once,
//! from the first standard that has it (the standards' own order, as every resource). Each tool says its favourite
//! key, the digest of its recipe and whether its command, at its version, takes its input on this drawing
//! (the command's own `validate`, on a copy: nothing is written); when not, why, in the command's words.

use kentos_commands::{
    CallOp, CommandResult, PaletteState, PaletteToolProblem, PaletteToolState, StandardsPalettes,
    StandardsPalettesRead,
};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::ExecutionContext;
use crate::checks::Stop;

/// A value as canonical JSON: object keys sorted, no spaces.
fn canonical(v: &Value) -> String {
    match v {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let parts: Vec<String> =
                keys.iter().map(|k| format!("{}:{}", Value::String((*k).clone()), canonical(&map[*k]))).collect();
            format!("{{{}}}", parts.join(","))
        }
        Value::Array(items) => format!("[{}]", items.iter().map(canonical).collect::<Vec<_>>().join(",")),
        other => other.to_string(),
    }
}

/// The recipe's digest: SHA-256 of `{"command":…,"commandVersion":…,"input":…}` as canonical JSON.
pub fn digest(command: &str, version: u32, input: &Value) -> String {
    let recipe = serde_json::json!({ "command": command, "commandVersion": version, "input": input });
    Sha256::digest(canonical(&recipe).as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
}

fn read(cx: &ExecutionContext<'_>) -> Result<StandardsPalettesRead, Stop> {
    let resolved = match crate::standards::resolve_project(cx.doc) {
        CommandResult::Completed { output, .. } => output,
        CommandResult::Failed { error } if error.code == crate::codes::NO_STANDARD => {
            return Ok(StandardsPalettesRead::default());
        }
        CommandResult::Failed { error } => return Err(Stop::failed(error)),
        CommandResult::Conflict { error } => return Err(Stop::conflict(error)),
        _ => return Err(Stop::Cancelled),
    };
    let mut palettes = Vec::new();
    for p in &resolved.resources.tool_palettes {
        let origin = resolved.provenance.tool_palettes.get(&p.id).cloned().unwrap_or_else(|| resolved.provenance.rules.clone());
        let mut tools = Vec::new();
        for t in &p.tools {
            let problem = match crate::registry::version_of(&t.command) {
                None => Some(PaletteToolProblem {
                    code: crate::codes::UNKNOWN_COMMAND.into(),
                    message: format!("“{}” bu çizimde çalışan bir komut değil; araç çalışmaz.", t.command),
                    path: Some("command".into()),
                }),
                Some(v) if v != t.command_version => Some(PaletteToolProblem {
                    code: crate::codes::UNKNOWN_COMMAND.into(),
                    message: format!(
                        "Araç “{}” komutunun {}. sürümü için yazılmış; burada {}. sürümü çalışıyor. Paletin yeni revizyonunu kullanın.",
                        t.command, t.command_version, v
                    ),
                    path: Some("commandVersion".into()),
                }),
                Some(_) => {
                    // The command's own check, on a copy of the drawing: `validate` writes nothing, the copy makes
                    // sure of it.
                    let mut copy = cx.doc.clone();
                    let mut inner = ExecutionContext::new(&mut copy);
                    match crate::registry::run(&mut inner, &t.command, CallOp::Validate, t.input.clone()) {
                        Err(unrun) => Some(PaletteToolProblem {
                            code: unrun.code.into(),
                            message: unrun.message,
                            path: Some("input".into()),
                        }),
                        Ok(answer) => match answer["status"].as_str() {
                            Some("completed") => None,
                            _ => Some(PaletteToolProblem {
                                code: answer["error"]["code"].as_str().unwrap_or("invalid_input").into(),
                                message: answer["error"]["message"].as_str().unwrap_or_default().into(),
                                path: answer["error"]["path"].as_str().map(|p| format!("input.{p}")),
                            }),
                        },
                    }
                }
            };
            tools.push(PaletteToolState {
                key: format!("standart:{}:{}:{}", origin.id, p.id, t.id),
                digest: digest(&t.command, t.command_version, &t.input),
                tool: t.clone(),
                ready: problem.is_none(),
                problem,
            });
        }
        palettes.push(PaletteState {
            id: p.id.clone(),
            name: p.name.clone(),
            description: p.description.clone(),
            standard: origin.id,
            revision: origin.revision,
            tools,
        });
    }
    Ok(StandardsPalettesRead { palettes })
}

pub fn validate(cx: &ExecutionContext<'_>, _input: &StandardsPalettes) -> CommandResult<()> {
    match read(cx) {
        Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, _input: &StandardsPalettes) -> CommandResult<StandardsPalettesRead> {
    match read(cx) {
        Ok(output) => CommandResult::Completed { output, warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: StandardsPalettes) -> CommandResult<StandardsPalettesRead> {
    plan(cx, &input)
}
