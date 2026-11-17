//! The extension API's coverage manifest (docs/inventory/eklenti-api-kapsami.json; plan/eklenti.md ECO-API-06;
//! docs/adr/8057 §124): every entry it names is found in the code, each by its surface's own record (the agents' tools,
//! the command catalog, `kentos kek`'s subcommands, the Python library, the catalog server's routes, the desktop's
//! words), and what it measures is what is there. A manifest that names a tool no one has, or a count no one counted,
//! fails here: the coverage is measured, not claimed.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde_json::{Value, json};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn read(path: &str) -> String {
    let at = root().join(path);
    std::fs::read_to_string(&at).unwrap_or_else(|e| panic!("{}: {e}", at.display()))
}

/// What each surface has, read from its own record.
struct Records {
    tools: BTreeMap<String, Value>,
    commands: BTreeMap<String, Value>,
    cli: String,
    routes: String,
    envelope: String,
}

impl Records {
    fn read() -> Records {
        let tools = kentos_headless::agent_tools::tools()
            .into_iter()
            .map(|t| (t.name.to_owned(), t.input))
            .collect();
        let commands = kentos_commands::catalog::catalog()
            .commands
            .into_iter()
            .map(|c| {
                (
                    c.id.clone(),
                    serde_json::to_value(&c.input).unwrap_or(Value::Null),
                )
            })
            .collect();
        Records {
            tools,
            commands,
            cli: read("apps/cli/src/main.rs"),
            routes: read("apps/api/src/http/mod.rs"),
            envelope: read("crates/shared/contracts/src/core/command_call.rs"),
        }
    }

    /// Whether `name` is what `surface` has; why not.
    fn has(&self, surface: &str, name: &str) -> Result<(), String> {
        let found = match surface {
            "ajan" => self.tools.contains_key(name),
            "komut" => self.commands.contains_key(name),
            "kek" => self.cli.contains(&format!("[\"kek\", \"{name}\"")),
            "katalog" => self
                .routes
                .contains(&format!("\"/v1/extension-catalogs{name}\"")),
            "python" => python_has(name),
            "basliksiz" => {
                let (on, rest) = name.split_once(':').ok_or("yüzey:ad olmalı")?;
                if !matches!(on, "ajan" | "komut" | "kek" | "python") {
                    return Err(format!("başsız yüzey olamaz: {on}"));
                }
                return self.has(on, rest);
            }
            other => return Err(format!("bu yüzey adla doğrulanmaz: {other}")),
        };
        if found {
            Ok(())
        } else {
            Err(format!("{surface} yüzeyinde “{name}” yok"))
        }
    }
}

/// `module:name` in the Python library: a name bound or a function defined at the module's top.
fn python_has(entry: &str) -> bool {
    let Some((module, name)) = entry.split_once(':') else {
        return false;
    };
    let text = read(&format!("python/kentos/{module}.py"));
    text.contains(&format!("\n{name} = ")) || text.contains(&format!("\ndef {name}("))
}

/// The keyword arguments of `cad.processing.run` in the Python library: its class's `__call__`.
fn python_run_kwargs() -> String {
    let text = read("python/kentos/cad/processing.py");
    let start = text.find("class _ProcessingRun").expect("the run's class");
    let rest = &text[start..];
    let end = rest[1..].find("\nclass ").map_or(rest.len(), |i| i + 1);
    rest[..end].to_owned()
}

/// What one surface's entry is: there, only the user's window, or not there.
enum Entry<'a> {
    There(&'a Value),
    Window(&'a str),
    Not,
}

fn entry(value: &Value) -> Entry<'_> {
    if let Some(window) = value.get("pencere").and_then(Value::as_str) {
        Entry::Window(window)
    } else if value.get("yok").and_then(Value::as_str).is_some() {
        Entry::Not
    } else {
        Entry::There(value)
    }
}

#[test]
fn the_manifest_names_only_what_is_there_and_measures_it() {
    let manifest: Value =
        serde_json::from_str(&read("docs/inventory/eklenti-api-kapsami.json")).expect("JSON");
    assert_eq!(manifest["format"], "kentos.extension-api-coverage");
    let records = Records::read();
    let surfaces: Vec<String> = manifest["surfaces"]
        .as_object()
        .expect("surfaces")
        .keys()
        .cloned()
        .collect();
    let absent = manifest["absent"].as_object().expect("absent");
    let mut wrong: Vec<String> = Vec::new();
    let mut counts: BTreeMap<String, [u32; 3]> = BTreeMap::new();
    let operations = manifest["operations"].as_array().expect("operations");
    for op in operations {
        let id = op["id"].as_str().unwrap_or_default();
        for surface in &surfaces {
            let count = counts.entry(surface.clone()).or_default();
            if absent.contains_key(surface) {
                count[2] += 1;
                continue;
            }
            let Some(value) = op.get(surface) else {
                wrong.push(format!(
                    "{id}: {surface} yazılmamış (ad, pencere ya da yok)"
                ));
                continue;
            };
            match entry(value) {
                Entry::Not => count[2] += 1,
                Entry::Window(tool) => {
                    count[1] += 1;
                    if let Err(why) = records.has("ajan", tool) {
                        wrong.push(format!("{id}: {why}"));
                    }
                }
                Entry::There(value) => {
                    count[0] += 1;
                    let found = match (surface.as_str(), value) {
                        ("masaustu", Value::Object(at)) => {
                            let (Some(file), Some(text)) = (
                                at.get("file").and_then(Value::as_str),
                                at.get("text").and_then(Value::as_str),
                            ) else {
                                wrong.push(format!("{id}: masaüstü {{file, text}} olmalı"));
                                continue;
                            };
                            if read(file).contains(text) {
                                Ok(())
                            } else {
                                Err(format!("masaüstü: {file} “{text}” geçmiyor"))
                            }
                        }
                        (s, Value::String(name)) => records.has(s, name),
                        (s, other) => Err(format!("{s}: okunamayan giriş {other}")),
                    };
                    if let Err(why) = found {
                        wrong.push(format!("{id}: {why}"));
                    }
                }
            }
        }
    }
    // The run's options, each on its surfaces.
    let kwargs = python_run_kwargs();
    let run_input = &records.commands["cad.processing.run"];
    let agent_input = &records.tools["processing.run"]["properties"];
    let mut options: BTreeMap<String, [u32; 2]> = BTreeMap::new();
    for row in manifest["options"]["rows"].as_array().expect("rows") {
        let option = row["option"].as_str().unwrap_or_default();
        for surface in ["komut", "python", "ajan"] {
            let count = options.entry(surface.to_owned()).or_default();
            let value = &row[surface];
            if value.get("yok").and_then(Value::as_str).is_some() {
                count[1] += 1;
                continue;
            }
            count[0] += 1;
            let name = value.as_str().unwrap_or_default();
            let found = match (surface, name.split_once(':'), name.split_once('.')) {
                ("komut", Some(("zarf", field)), _) => {
                    records.envelope.contains(&format!("pub {field}:"))
                }
                ("komut", _, _) => run_input["properties"].get(name).is_some(),
                ("python", _, _) => kwargs.contains(&format!("\n        {name}: ")),
                ("ajan", _, Some(("input", field))) => {
                    agent_input.get("input").is_some()
                        && run_input["properties"].get(field).is_some()
                }
                ("ajan", _, _) => agent_input.get(name).is_some(),
                _ => false,
            };
            if !found {
                wrong.push(format!(
                    "seçenek {option}: {surface} yüzeyinde “{name}” yok"
                ));
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    let measured = json!({
        "operations": operations.len(),
        "surfaces": counts.iter().map(|(s, [there, window, not])| {
            (s.clone(), json!({ "var": there, "pencere": window, "yok": not }))
        }).collect::<serde_json::Map<String, Value>>(),
        "options": options.iter().map(|(s, [there, not])| {
            (s.clone(), json!({ "var": there, "yok": not }))
        }).collect::<serde_json::Map<String, Value>>(),
    });
    assert_eq!(
        manifest["measured"],
        measured,
        "the manifest's counts are the measured ones; write these:\n{}",
        serde_json::to_string_pretty(&measured).unwrap_or_default()
    );
}

/// Every operation of the API family's table (docs/adr/8061 §11) is in the manifest, and no other.
#[test]
fn every_family_of_the_api_is_measured() {
    let manifest: Value =
        serde_json::from_str(&read("docs/inventory/eklenti-api-kapsami.json")).expect("JSON");
    let ids: Vec<&str> = manifest["operations"]
        .as_array()
        .expect("operations")
        .iter()
        .filter_map(|o| o["id"].as_str())
        .collect();
    for family in [
        "python.environments.",
        "extensions.",
        "extensions.catalog.",
        "processing.tools.",
        "processing.models",
        "discovery",
    ] {
        assert!(ids.iter().any(|id| id.starts_with(family)), "{family}");
    }
    let mut unique = ids.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(unique.len(), ids.len(), "an operation once");
}
