//! What running an extension's İşlemler tool needs, found before the run (docs/adr/8057 §7): the tools a registry's
//! extensions add, each read from its descriptor without its code; each one's plan (its extension version, handler,
//! package and lock); how the version's process starts in the environment prepared for it.

use std::collections::HashMap;
use std::path::PathBuf;

use kentos_extension_host::{ContributionKind, Registered, Registry};
use kentos_processing::external::{self, ExternalTool};
use kentos_python_env::extension::{self, BYTECODE};
use kentos_python_env::process::{Start, interpreter_args};
use kentos_python_env::python::Interpreter;
use serde_json::{Value, json};

/// What running one of an extension's contributions needs, found when the registry changes: no disk is read for it at
/// the run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    pub ext: String,
    pub local: String,
    pub handler: String,
    pub version: String,
    /// The installed version's package folder.
    pub source: PathBuf,
    /// Its content digest as installed (`sha256:…`).
    pub digest: String,
    /// Its lock in the package (`uv.lock`): its environment is found by it.
    pub lock_file: String,
    /// Installed from a working folder: its environment is a development one (docs/adr/8057 §97).
    pub development: bool,
}

impl Plan {
    /// `local`'s plan (its `handler`) in `ext` as installed in `registry`; none when it is not installed.
    pub fn of(registry: &Registry, ext: &str, local: &str, handler: &str) -> Option<Plan> {
        let manifest = registry.manifest(ext)?;
        Some(Plan {
            ext: ext.to_owned(),
            local: local.to_owned(),
            handler: handler.to_owned(),
            version: manifest.version.to_string(),
            source: registry.package_dir(ext)?,
            digest: registry.digest(ext).unwrap_or_default(),
            lock_file: manifest
                .python
                .as_ref()
                .map(|p| p.lock_file.clone())
                .unwrap_or_default(),
            development: registry.developed(ext),
        })
    }

    /// Its version's process in a pool: `<eklenti>@<sürüm>`.
    pub fn key(&self) -> String {
        format!("{}@{}", self.ext, self.version)
    }
}

/// KentOS's own Python, when ready: what finds an extension's environment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Python {
    pub interpreter: Interpreter,
    /// The user's Python folder (`KENTOS_PYTHON_DIR` points elsewhere).
    pub root: PathBuf,
    pub uv: PathBuf,
}

/// Where the extensions' code runs on `interpreter` (`Python 3.12.13 · linux-x86_64`): their packages' binaries depend
/// on both, and a layer a tool makes records it (docs/adr/8057 §15).
pub fn environment_of(interpreter: &Interpreter) -> String {
    format!(
        "Python {} · {}-{}",
        interpreter.version,
        std::env::consts::OS,
        std::env::consts::ARCH
    )
}

/// How a host puts this KentOS's library into an environment that holds another build (docs/adr/8057 §6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Library {
    /// The installation's wheel must be there (the desktop ships with it): without it nothing runs.
    Required,
    /// Put in when the installation's wheel is found beside the program; else the environment runs with the library
    /// it was prepared with (a headless host started where no installation is, the Python library's interpreter).
    IfFound,
}

/// The environment of `plan`'s version found, this KentOS's library in it (`library`), and how its process starts.
pub fn start_of(plan: &Plan, python: &Python, library: Library) -> Result<Start, String> {
    let lock = extension::lock_digest(&plan.source, &plan.lock_file)?;
    let mut ready = extension::find(
        &python.root,
        &plan.ext,
        &plan.version,
        &lock,
        &python.interpreter.version,
        plan.development,
    )
    .ok_or_else(|| {
        "Bu eklentinin Python ortamı hazırlanmadı: Eklentiler › Python ortamı.".to_owned()
    })?;
    let dirs = kentos_python_env::sdk::dirs();
    match kentos_python_env::sdk::find(&dirs, env!("CARGO_PKG_VERSION")) {
        Ok(wheel) => {
            ready = extension::refresh_library(&python.uv, &python.root, &ready, &wheel)?;
        }
        Err(why) if library == Library::Required => return Err(why),
        Err(_) => {}
    }
    let pycache = extension::place(&python.root, &ready.key).join(BYTECODE);
    Ok(Start {
        python: ready.python,
        args: interpreter_args(&pycache),
        hello: json!({
            "type": "hello",
            "protocol": 1,
            "extension": plan.ext,
            "version": plan.version,
            "digest": plan.digest,
            "source": plan.source.display().to_string(),
            "paths": ["src"],
        }),
    })
}

/// The İşlemler tools of a registry's processing contributions, read from their descriptors without their code.
#[derive(Default)]
pub struct Listed {
    pub tools: Vec<ExternalTool>,
    /// What running each needs, by its id.
    pub plans: HashMap<String, Plan>,
    /// Each one's descriptor, by its id: its path in the package (`tools/taks.tool.json`).
    pub descriptors: HashMap<String, String>,
    /// The descriptors that do not fit: the extension, the tool's local id and why.
    pub refused: Vec<(String, String, String)>,
}

/// The tools `contributions` add (the registry's running ones on the desktop, its ready ones in a headless host), each
/// with its package's digest (a model's step pins it, docs/adr/8057 §13) and `environment`, where its code runs (empty
/// until KentOS's Python is known).
pub fn read_tools(registry: &Registry, contributions: &[Registered], environment: &str) -> Listed {
    let mut listed = Listed::default();
    for c in contributions
        .iter()
        .filter(|c| c.kind == ContributionKind::Processing)
    {
        let ext = c.lease.extension.clone();
        let Some(manifest) = registry.manifest(&ext) else {
            continue;
        };
        let local =
            c.id.strip_prefix(&format!("ext.{ext}."))
                .unwrap_or_default()
                .to_owned();
        let Some(entry) = manifest
            .entrypoints
            .processing
            .iter()
            .find(|p| p.id == local)
        else {
            continue;
        };
        let Some(plan) = Plan::of(registry, &ext, &local, &entry.handler) else {
            continue;
        };
        let read = std::fs::read_to_string(plan.source.join(&entry.descriptor))
            .map_err(|e| format!("{} okunamadı: {e}", entry.descriptor))
            .and_then(|text| {
                serde_json::from_str::<Value>(&text)
                    .map_err(|e| format!("{} JSON değil: {e}", entry.descriptor))
            })
            .and_then(|descriptor| {
                external::from_descriptor(&ext, &manifest.name, &plan.version, &local, &descriptor)
            });
        match read {
            Ok(mut tool) => {
                tool.digest = plan.digest.clone();
                tool.environment = environment.to_owned();
                let id = tool.tool.id.clone();
                listed
                    .descriptors
                    .insert(id.clone(), entry.descriptor.clone());
                listed.plans.insert(id, plan);
                listed.tools.push(tool);
            }
            Err(why) => listed.refused.push((ext, local, why)),
        }
    }
    listed
}
