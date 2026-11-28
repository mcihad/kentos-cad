//! Kapı 3's part that runs no code (plan/eklenti.md ADIM 21, docs/adr/8057 §26): a package bound as KentOS binds it, in a
//! throwaway registry, and taken away again.
//!
//! - **Bound:** installed with everything it asks for granted, its Python contributions too (activation registers them
//!   without starting a process, D6), and activated: what it adds under its lease and what was left out and why; an
//!   install that fails says its problems, an activation that fails (a dependency missing, the host too old) says why.
//! - **Taken away:** removed with its data; whatever of it stays behind (its packages, its data, its entry, a lease) is
//!   residue.
//! - **Nothing of the package runs:** activation reads its manifest and files, as the app's start does. The registry is
//!   a folder of its own under the system's temporary folder, gone when the run ends: the user's extensions are not
//!   touched. Running its code on a synthetic drawing needs the user's consent (an install) and its environment: that
//!   is the app's part.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use kentos_extensions::{Host, Limits, Problem};
use serde::Serialize;

use crate::lease::{Registered, Skipped};
use crate::registry::{Choice, Options, Registry};
use crate::store::{self, Grant, Source, SourceKind};

/// What binding a package showed.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DryRun {
    /// Why it could not be installed (the install's own problems); empty when it was.
    pub problems: Vec<Problem>,
    /// Why it was installed but not activated; none when it was.
    pub activation: Option<String>,
    /// What it adds, each under its id (`ext.<extension>.<local id>`), kind and title.
    pub registered: Vec<Registered>,
    /// What was left out at activation, and why.
    pub skipped: Vec<Skipped>,
    /// What of it stayed after it was removed with its data: none expected.
    pub residue: Vec<String>,
    /// What the run could not show: the extensions it depends on are not in its empty registry, so its contributions
    /// are bound where they are installed.
    pub note: Option<String>,
}

impl DryRun {
    /// Bound whole and taken away without a trace.
    pub fn is_clean(&self) -> bool {
        self.problems.is_empty()
            && self.activation.is_none()
            && self.skipped.is_empty()
            && self.residue.is_empty()
    }

    /// As an agent and `kek verify --json` read it: what it adds by id, kind and title, without the leases.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "clean": self.is_clean(),
            "problems": self.problems,
            "activation": self.activation,
            "registered": self.registered.iter().map(|r| serde_json::json!({
                "id": r.id, "kind": r.kind, "title": r.title, "python": r.needs_python,
            })).collect::<Vec<_>>(),
            "skipped": self.skipped,
            "residue": self.residue,
            "note": self.note,
        })
    }
}

/// A throwaway folder under the system's temporary folder, one per run.
fn scratch() -> PathBuf {
    static RUNS: AtomicU64 = AtomicU64::new(0);
    std::env::temp_dir().join(format!(
        "kentos-kuru-kosu-{}-{}",
        std::process::id(),
        RUNS.fetch_add(1, Ordering::Relaxed)
    ))
}

/// Binds `bytes` (a `.kek`) as this `host` would, with `options` (its policy and tool check), then takes it away; nothing
/// of the package runs.
pub fn dry_run(bytes: &[u8], host: Host, options: Options) -> DryRun {
    let dir = scratch();
    let _ = store::remove_tree(&dir);
    let out = bind(&dir, bytes, host, options).unwrap_or_else(|problem| DryRun {
        problems: vec![problem],
        ..DryRun::default()
    });
    let _ = store::remove_tree(&dir);
    out
}

fn bind(dir: &Path, bytes: &[u8], host: Host, options: Options) -> Result<DryRun, Problem> {
    fs::create_dir_all(dir).map_err(|e| store::io("Geçici klasör açılamadı", dir, &e))?;
    let limits: Limits = options.limits;
    let mut registry = Registry::open(dir, host, options)?;
    let manifest = kentos_extensions::open(bytes, &limits).inspection.manifest;
    let asked = manifest
        .as_ref()
        .map(|m| m.capabilities().into_iter().collect())
        .unwrap_or_default();
    let depends: Vec<String> = manifest
        .iter()
        .flat_map(|m| &m.dependencies.extensions)
        .map(|d| d.id.clone())
        .collect();
    let source = Source {
        kind: SourceKind::File,
        location: "kuru koşu".to_owned(),
    };
    let choice = Choice {
        grant: Grant {
            capabilities: asked,
            python: true,
        },
        enable: true,
        replace_source: false,
    };
    let installed = match registry.install(bytes, &source, &choice) {
        Ok(installed) => installed,
        Err(problems) => {
            return Ok(DryRun {
                problems,
                ..DryRun::default()
            });
        }
    };
    let id = installed.id.clone();
    let mut out = DryRun::default();
    match installed.activation {
        Some(Ok(activation)) => out.skipped = activation.skipped,
        Some(Err(problem)) => out.activation = Some(problem.message),
        None => out.activation = Some("Etkinleştirilmedi.".to_owned()),
    }
    if out.activation.is_some() && !depends.is_empty() {
        out.note = Some(format!(
            "Kuru koşunun kayıt defteri boş: bağımlı olduğu eklentiler ({}) orada kurulu değil; katkıları onlar kurulu \
             bir KentOS'ta bağlanır.",
            depends.join(", ")
        ));
    }
    out.registered = registry
        .contributions()
        .iter()
        .filter(|c| c.lease.extension == id)
        .cloned()
        .collect();
    registry.remove(&id, false)?;
    let layout = store::Layout {
        root: dir.to_path_buf(),
    };
    for (path, what) in [
        (layout.packages_of(&id), "paket klasörü"),
        (layout.data(&id), "veri klasörü"),
    ] {
        if path.exists() {
            out.residue
                .push(format!("{what} kaldı: {}", path.display()));
        }
    }
    if registry.extensions().iter().any(|v| v.id == id) {
        out.residue
            .push("kayıt defterinde girdisi kaldı".to_owned());
    }
    let leases = registry
        .contributions()
        .iter()
        .filter(|c| c.lease.extension == id)
        .count();
    if leases > 0 {
        out.residue
            .push(format!("{leases} katkısı kirasıyla kaldı"));
    }
    if fs::read_dir(layout.staging()).is_ok_and(|mut d| d.next().is_some()) {
        out.residue
            .push("hazırlık klasöründe dosya kaldı".to_owned());
    }
    Ok(out)
}
