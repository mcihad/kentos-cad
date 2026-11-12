//! The product command catalog (docs/adr/0013): the typed, versioned
//! operations every host reaches the same way (the web app, the desktop app,
//! the server, the CLI, Python and AI). The web app's interface commands
//! (`core/commands.ts`: open a window, pick a tool) are not here; one may end
//! in a product command.
//!
//! Input and output schemas are JSON Schema derived from the contract types
//! themselves (`schemars`, the `schema` feature), so a type and its schema
//! cannot drift. A command is listed only once a host handles it.
//!
//! The descriptor types live here, below every host; `kentos-commands` builds the catalog over the commands' own
//! types (`kentos_commands::catalog()`), so a new command does not rebuild the crates below them.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const CATALOG_FORMAT: &str = "kentos.commands";
/// Each schema carries the definitions it names in its own `$defs`: the catalog as `catalog()` builds it, every
/// schema whole by itself (an MCP tool's, a Python command's).
pub const CATALOG_VERSION: u32 = 1;
/// The schemas' definitions are the catalog's, once each, in its top-level `$defs` (docs/adr/0731): the file
/// `commandCatalog.json`. A schema's `#/$defs/Name` names one there, as JSON Schema reads a reference in a schema
/// embedded in a document without an `$id` of its own.
pub const CATALOG_SHARED_VERSION: u32 = 2;

/// Every product command, as `catalog()` builds it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CommandCatalog {
    #[cfg_attr(feature = "ts", ts(type = "\"kentos.commands\""))]
    pub format: String,
    /// 1: every schema with its own `$defs`; 2: the schemas' definitions in the catalog's `$defs` (docs/adr/0731).
    #[cfg_attr(feature = "ts", ts(type = "1 | 2"))]
    pub version: u32,
    pub commands: Vec<CommandDescriptor>,
    /// What every code of the commands' answers means (docs/adr/0253 §7).
    #[serde(default)]
    pub codes: Vec<CatalogCode>,
    /// The codes any call may answer with, whatever its command (the protocol's).
    #[serde(default)]
    pub protocol: Vec<String>,
    /// What the schemas' `x-unit`s mean (docs/adr/0256 §3): every number of
    /// every command's input, output and plan names one.
    #[serde(default)]
    pub units: Vec<CatalogUnit>,
    /// What a change of looks and styles does: its undo, whether it makes the drawing unsaved, the project's
    /// parts it writes and how the cloud carries them (docs/adr/0314 §2).
    #[serde(default)]
    pub effects: Vec<crate::change_effects::CatalogEffect>,
    /// Version 2 (docs/adr/0731): every definition a schema names by `#/$defs/Name`, once; empty in version 1.
    #[serde(default, rename = "$defs", skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, type = "Record<string, unknown>"))]
    pub defs: std::collections::BTreeMap<String, serde_json::Value>,
}

/// The schemas of a command: its input's, its output's and its plan's.
fn schemas_mut(d: &mut CommandDescriptor) -> impl Iterator<Item = &mut serde_json::Value> {
    [Some(&mut d.input), Some(&mut d.output), d.plan.as_mut()]
        .into_iter()
        .flatten()
}

/// The definitions `schema` names by `#/$defs/Name`, and those they name, from `defs`, in their names' order: the
/// names it reaches that `defs` lacks too, so a reader can say which.
fn reached<'a>(
    schema: &'a serde_json::Value,
    defs: &'a std::collections::BTreeMap<String, serde_json::Value>,
) -> std::collections::BTreeMap<&'a str, Option<&'a serde_json::Value>> {
    use serde_json::Value;
    let mut out = std::collections::BTreeMap::new();
    let mut stack = vec![schema];
    while let Some(node) = stack.pop() {
        match node {
            Value::Object(map) => {
                for (key, value) in map {
                    match (key.as_str(), value) {
                        ("$defs", _) => {}
                        ("$ref", Value::String(r)) => {
                            if let Some(name) = r.strip_prefix("#/$defs/")
                                && !out.contains_key(name)
                            {
                                let def = defs.get(name);
                                out.insert(name, def);
                                stack.extend(def);
                            }
                        }
                        _ => stack.push(value),
                    }
                }
            }
            Value::Array(items) => stack.extend(items),
            _ => {}
        }
    }
    out
}

impl CommandCatalog {
    /// The catalog with its schemas' definitions hoisted into its own `$defs`, each once (version 2,
    /// docs/adr/0731): what `commandCatalog.json` holds. Err names a definition two schemas give different
    /// bodies under one name (two types of one name: the file could not say both).
    pub fn shared(&self) -> Result<CommandCatalog, String> {
        let mut out = self.clone();
        for d in &mut out.commands {
            let id = d.id.clone();
            for schema in schemas_mut(d) {
                let Some(serde_json::Value::Object(defs)) =
                    schema.as_object_mut().and_then(|s| s.remove("$defs"))
                else {
                    continue;
                };
                for (name, body) in defs {
                    match out.defs.get(&name) {
                        Some(known) if *known != body => {
                            return Err(format!(
                                "{id}: “{name}” is defined twice with different bodies; give one of the types another name"
                            ));
                        }
                        Some(_) => {}
                        None => {
                            out.defs.insert(name, body);
                        }
                    }
                }
            }
        }
        out.version = CATALOG_SHARED_VERSION;
        Ok(out)
    }

    /// `schema`, one of this catalog's commands' schemas, whole by itself: with the definitions of the catalog's
    /// `$defs` it reaches, as its own `$defs` (none when it reaches none), the inverse of [`Self::shared`]. Err
    /// names a definition the catalog lacks.
    pub fn standalone(&self, schema: &serde_json::Value) -> Result<serde_json::Value, String> {
        let reached = reached(schema, &self.defs);
        let mut defs = serde_json::Map::new();
        for (name, def) in reached {
            let def = def.ok_or_else(|| format!("“{name}” is not among the catalog's definitions"))?;
            defs.insert(name.to_owned(), def.clone());
        }
        let mut out = schema.clone();
        if let Some(map) = out.as_object_mut() {
            map.remove("$defs");
            if !defs.is_empty() {
                map.insert("$defs".into(), serde_json::Value::Object(defs));
            }
        }
        Ok(out)
    }

    /// The catalog with every schema whole by itself (version 1), from either version: each schema's definitions
    /// back in its own `$defs`.
    pub fn standalone_all(&self) -> Result<CommandCatalog, String> {
        if self.defs.is_empty() {
            return Ok(self.clone());
        }
        let mut out = self.clone();
        for d in &mut out.commands {
            for schema in schemas_mut(d) {
                *schema = self.standalone(schema)?;
            }
        }
        out.defs.clear();
        out.version = CATALOG_VERSION;
        Ok(out)
    }
}

/// A unit the schemas' `x-unit` names and what it means.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CatalogUnit {
    pub unit: String,
    /// Turkish, for people and for an agent reading a number.
    pub meaning: String,
}

/// A code of the commands' answers and what it means (`codes::CODES`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CatalogCode {
    pub code: String,
    /// `failed`, `conflict`, `needsInput`, `warning`, `failedOrWarning`, or
    /// `host`: the host's own failure, not a command's answer.
    pub kind: String,
    /// Whether calling again may succeed (a conflict's).
    pub retryable: bool,
    /// Turkish.
    pub meaning: String,
}

/// One product command (TODOS.md CMD-01).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CommandDescriptor {
    /// Stable name `alan.nesne.eylem`, lowercase ASCII (`cad.polygon.create`); never reused.
    pub id: String,
    /// Version of the input and output schema; an incompatible change is a new version.
    pub version: u32,
    /// Turkish name (interface, help, AI tool title).
    pub title: String,
    /// One or two Turkish sentences: what it does and what it needs.
    pub summary: String,
    /// Command line and search names.
    pub aliases: Vec<String>,
    pub effect: CommandEffect,
    /// Where a handler exists.
    pub hosts: Vec<CommandHost>,
    /// Runs from explicit input alone, without an interface or its implicit state (CMD-07).
    pub headless: bool,
    pub requires: Vec<CommandRequirement>,
    /// Project permissions it needs (docs/adr/0015); `summary` says which applies when.
    pub permissions: Vec<String>,
    pub undo: CommandUndo,
    pub cost: CommandCost,
    /// JSON Schema of the input.
    #[cfg_attr(feature = "ts", ts(type = "unknown"))]
    pub input: serde_json::Value,
    /// JSON Schema of the result.
    #[cfg_attr(feature = "ts", ts(type = "unknown"))]
    pub output: serde_json::Value,
    /// JSON Schema of what `plan` answers: the change it would make, with
    /// nothing written (TODOS.md CMD-04). Absent for a command only the
    /// server runs: the server executes, it does not plan.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "unknown"))]
    pub plan: Option<serde_json::Value>,
    /// The object kinds it takes, the codes it refuses with and warns with
    /// (docs/adr/0253 §7); absent for a command only the server runs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub capabilities: Option<CommandCapabilities>,
    pub examples: Vec<CommandExample>,
}

/// What a command changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CommandEffect {
    /// Nothing.
    Query,
    /// Only the local session: camera, selection, active tool.
    View,
    /// The open document; undoable.
    Document,
    /// Project state on the server: saving, sharing, cloud commits.
    Project,
    /// Tenants and users.
    Admin,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CommandHost {
    Web,
    Desktop,
    Server,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CommandRequirement {
    /// An open document.
    Document,
    /// An open cloud project.
    CloudProject,
    /// A signed-in account.
    SignedIn,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CommandUndo {
    None,
    /// One local undo step.
    Step,
    /// Undone in the cloud by an inverse command, checked like any other (TX-03).
    Inverse,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CommandCost {
    Instant,
    /// The user waits for it.
    Interactive,
    /// Queued; the answer is a job id (202).
    Job,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CommandExample {
    pub title: String,
    #[cfg_attr(feature = "ts", ts(type = "unknown"))]
    pub input: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "unknown"))]
    pub output: Option<serde_json::Value>,
}

// What a command takes and how it answers (docs/adr/0253 §7); `kentos_commands::capabilities_of` fills them.
/// What a command takes and how it answers.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CommandCapabilities {
    /// CAD exchange's explicit format/version/feature profile, shared by UI, Python and AI.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub exchange: Option<crate::CadFormatCapabilities>,
    /// The object kinds its operations apply to; none when it takes no objects
    /// by kind (it names layers, styles, settings) or takes every kind.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<CommandTarget>>", optional))]
    pub targets: Vec<CommandTarget>,
    /// The codes it refuses with (`failed`, `conflict`), beside the
    /// protocol's own (`CommandCatalog.protocol`); `codes` says what each means.
    pub refusals: Vec<String>,
    /// The codes of its warnings.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub warnings: Vec<String>,
}

/// The object kinds an operation of a command applies to.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CommandTarget {
    /// The operation or method (a transform's `type`, a corner's `kind`);
    /// none for the command as a whole.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub operation: Option<String>,
    /// The kinds (`Entity`'s `kind`: `line`, `polyline`, `polygon` …);
    /// `*` every kind but the ones `except` names.
    pub kinds: Vec<String>,
    /// With `*`: the kinds it does not take.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub except: Vec<String>,
    /// The code an object of a kind it does not take is refused with.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub refused_with: Option<String>,
    /// Turkish: what else decides.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub note: Option<String>,
}
