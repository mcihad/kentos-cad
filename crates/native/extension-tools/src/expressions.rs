//! The extensions' expression functions in the headless hosts (plan/eklenti.md ADIM 19; docs/adr/8057 §80): a
//! label's text, a style's rule, a layer's filter, a form's calculation and an İşlemler expression that call
//! `ext.<extension>.<function>(…)` in MCP and in the Python library get the extension's answers as on the desktop, a
//! batch of rows a call. The desktop answers while drawing and never waits (`LabelAnswers`); a headless host has no
//! frame to keep, so it waits for the batch.
//!
//! - **Offered by the extension:** only an extension whose manifest says it runs headless (`runtimes` holds
//!   `headless`) offers its functions here, as a tool's descriptor does for a tool (§70); a test runner's host and the
//!   desktop run every one. Another's function is known by name and refused where it is called, saying why: not an
//!   unknown function, nor an empty answer nobody explains.
//! - **The same process:** a batch runs in the version's process of the headless host's pool (`kind: expression`, one
//!   row of arguments each), the files checked against their digest before it starts; a failure empties the batch and
//!   is noted (the extension's log, the host's words).

use std::collections::HashMap;

use kentos_expression::external::Arg;
use serde_json::{Value, json};

use crate::plan::Plan;

/// The expression functions a registry's extensions offer, by their whole name (`ext.<extension>.<local>`): what
/// running each needs and its line for the builder; a function refused here has a line and no plan.
#[derive(Clone, Debug, Default)]
pub struct Offered {
    pub plans: HashMap<String, Plan>,
    pub defs: Vec<kentos_expression::external::ExternalDef>,
}

/// The expression functions of `contributions` (a registry's ready ones): those of an extension that runs headless
/// with their plans, or every one's when `every` (a test runner's host, the desktop); another's refused with its words.
pub fn read_functions(
    registry: &kentos_extension_host::Registry,
    contributions: &[kentos_extension_host::Registered],
    every: bool,
) -> Offered {
    use kentos_extension_host::ContributionKind;
    use kentos_extensions::manifest::Runtime;
    let mut out = Offered::default();
    for c in contributions
        .iter()
        .filter(|c| c.kind == ContributionKind::Expression)
    {
        let ext = c.lease.extension.as_str();
        let Some(manifest) = registry.manifest(ext) else {
            continue;
        };
        let local =
            c.id.strip_prefix(&format!("ext.{ext}."))
                .unwrap_or_default();
        let Some(entry) = manifest
            .entrypoints
            .expressions
            .iter()
            .find(|e| e.id == local)
        else {
            continue;
        };
        let Some(plan) = Plan::of(registry, ext, local, &entry.handler) else {
            continue;
        };
        let name = format!("ext.{ext}.{local}");
        let runs_here = every || manifest.runtimes.contains(&Runtime::Headless);
        out.defs.push(kentos_expression::external::ExternalDef {
            name: name.clone(),
            description: format!(
                "{} — {} eklentisinin ifade işlevi; kendi Python ortamında çalışır.",
                entry.title.as_deref().unwrap_or(local),
                manifest.name
            ),
            refused: (!runs_here).then(|| {
                format!(
                    "“{}” eklentisi başsız çalıştığını söylemiyor (manifestinin `runtimes` listesinde `headless` \
                     yok); işlevlerini yalnız KentOS masaüstü çalıştırır.",
                    manifest.name
                )
            }),
        });
        if runs_here {
            out.plans.insert(name, plan);
        }
    }
    out
}

/// A row of arguments as the extension's code takes them.
pub fn row_json(row: &[Arg]) -> Value {
    Value::Array(
        row.iter()
            .map(|a| match a {
                Arg::Null => Value::Null,
                Arg::Num(n) => json!(n),
                Arg::Text(t) => json!(t),
                Arg::Bool(b) => json!(b),
            })
            .collect(),
    )
}

/// The code's answer (`done`) read into one value a row, `rows` of them: a row it did not answer, or a batch it failed
/// (its words, second), empty.
pub fn values_of(done: &Value, rows: usize) -> (Vec<Arg>, Option<String>) {
    let empty = || vec![Arg::Null; rows];
    if done["ok"] != true {
        let message = done["message"].as_str().unwrap_or("cevap vermedi");
        let words = match done["exception"].as_str() {
            Some(kind) => format!("{kind}: {message}"),
            None => message.to_owned(),
        };
        return (empty(), Some(words));
    }
    let result = &done["result"];
    let mut values: Vec<Arg> = result["values"]
        .as_array()
        .map(|l| {
            l.iter()
                .map(|v| match v {
                    Value::Null => Arg::Null,
                    Value::Bool(b) => Arg::Bool(*b),
                    Value::Number(n) => n.as_f64().map_or(Arg::Null, Arg::Num),
                    Value::String(s) => Arg::Text(s.clone()),
                    other => Arg::Text(other.to_string()),
                })
                .collect()
        })
        .unwrap_or_default();
    values.resize(rows, Arg::Null);
    let failed = result["failed"].as_u64().unwrap_or(0);
    let words = (failed > 0).then(|| {
        format!(
            "{failed} satır hata verdi: {}",
            result["error"].as_str().unwrap_or_default()
        )
    });
    (values, words)
}
