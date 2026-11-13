//! The extension registry of one user (docs/adr/8057 §2; plan/eklenti.md ADIM 17).
//!
//! Installing, preparing, enabling, running, disabling, updating, going back and removing are separate operations and
//! one's success never stands for another's (PY-RUN-03). An install inspects the bytes again (a review is never
//! trusted), writes and re-reads the files in a staging folder, moves them into place and switches the registry in one
//! write: a failure anywhere before that leaves the version in use as it was. The files of a version are checked
//! against its content digest before each activation. Activation order follows the dependencies; stopping an extension
//! stops the ones that depend on it.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use kentos_extensions::integrity::{self, Integrity};
use kentos_extensions::manifest::{ExtensionDependency, Manifest, dependency_cycle};
use kentos_extensions::version::{Version, VersionReq};
use kentos_extensions::{Host, Limits, Problem, compatible};

use crate::journal::{self, Journal, Record};
use crate::lease::{self, ExtensionLease, Registered, Skipped};
use crate::policy::{Policy, Revocations};
use crate::store::{
    self, Channel, Entry, Failure, Grant, InstalledVersion, LOG_LINES, Layout, Metrics,
    RegistryFile, Source, SourceKind, problem,
};
use crate::view::{
    DependencyStatus, ExtensionView, NeedStatus, ProjectNeed, Review, State, Trust,
    capability_lines,
};
use crate::watchdog::{Bisection, CRASHES_TO_FAIL, Finding};

/// How the registry runs.
#[derive(Clone, Debug)]
pub struct Options {
    /// Start without extensions: nothing is activated; the states say why (D6).
    pub safe_mode: bool,
    /// The organisation's policy, read by the host from the system folder.
    pub policy: Option<Policy>,
    /// How long a change waits for another process's.
    pub lock_wait: Duration,
    pub limits: Limits,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            safe_mode: false,
            policy: None,
            lock_wait: Duration::from_secs(5),
            limits: Limits::default(),
        }
    }
}

/// What the user chose when installing.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Choice {
    pub grant: Grant,
    /// Start it after installing (an update keeps an enabled extension enabled).
    pub enable: bool,
    /// Installed from another source: replace it (the user's explicit choice, never silent).
    pub replace_source: bool,
}

/// An activation: its lease, what was left out, how long it took.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Activation {
    pub lease: ExtensionLease,
    pub skipped: Vec<Skipped>,
    pub micros: u64,
}

/// What an install did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Installed {
    pub id: String,
    pub version: String,
    /// The version in use before.
    pub previous: Option<String>,
    /// The same version with the same content was installed already: nothing changed.
    pub already: bool,
    pub activation: Option<Result<Activation, Problem>>,
}

/// What a crash report did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Crash {
    /// Counted: crashes in a row so far.
    Counted(u32),
    /// The extension is stopped for good, until the user retries.
    Failed,
    /// The lease is not the current one: a late report of a stopped activation.
    Ignored,
}

pub struct Registry {
    layout: Layout,
    host: Host,
    options: Options,
    file: RegistryFile,
    revocations: Revocations,
    /// The version in use's manifest of every installed extension.
    manifests: BTreeMap<String, Manifest>,
    leases: BTreeMap<String, ExtensionLease>,
    registered: Vec<Registered>,
    generation: u64,
    journal: Journal,
    /// The newest records of each installed extension's log, as read at opening and as written since: the manager
    /// shows them without reading files at every frame.
    logs: BTreeMap<String, Vec<Record>>,
    /// "Sorunlu eklentiyi bul" going on, as read at opening and as answered since (`sorun-bulma.json`).
    bisect: Option<Bisection>,
}

/// `sorun-bulma.json`: a bisection going on.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BisectFile {
    format: String,
    schema_version: u32,
    bisection: Bisection,
}

const BISECT_FORMAT: &str = "kentos.extension-bisect";

/// The bisection the registry's folder keeps; none when there is none or it cannot be read (the window then runs
/// every enabled extension, as without one).
fn read_bisect(layout: &Layout) -> Option<Bisection> {
    let text = fs::read_to_string(layout.bisect()).ok()?;
    let file: BisectFile = serde_json::from_str(&text).ok()?;
    (file.format == BISECT_FORMAT && file.schema_version == 1).then_some(file.bisection)
}

fn not_installed(id: &str) -> Problem {
    problem("registry_unknown", format!("“{id}” kurulu değil."))
}

/// Same id from another kind of source, or another working folder: replacing it is the user's choice.
fn conflicts(installed: &Source, new: &Source) -> bool {
    installed.kind != new.kind
        || (installed.kind == SourceKind::DeveloperFolder && installed.location != new.location)
}

/// The short history `registry.json` kept before the log (ADIM 17): read into the log once (when the extension has no
/// log yet) and taken out of the file, so that it is no longer written.
fn carry_history(journal: &Journal, file: &mut RegistryFile) {
    for (id, entry) in file.extensions.iter_mut() {
        let earlier = std::mem::take(&mut entry.log);
        if !earlier.is_empty() && !journal.file(Some(id)).exists() {
            for event in earlier {
                let mut record = Record::info("earlier", event.message).of(id, Some(&entry.active));
                record.at = event.at.saturating_mul(1000);
                journal.write(record);
            }
        }
    }
}

/// The installed files against the content digest; when they differ, which files and how, read against the package's
/// own list as long as that list is still the one installed.
fn verify_files(dir: &Path, digest: &str) -> Result<(), String> {
    let files = store::read_files(dir).map_err(|p| p.message)?;
    let list = Integrity::of(
        files
            .iter()
            .filter(|(p, _)| p.as_str() != integrity::FILE)
            .map(|(p, d)| (p.as_str(), d.as_slice())),
    );
    if list.digest() == digest {
        return Ok(());
    }
    let kept = files
        .get(integrity::FILE)
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
        .and_then(|text| Integrity::read(text).ok())
        .filter(|kept| kept.digest() == digest);
    let which = match kept {
        Some(kept) => {
            let found: Vec<String> = kept
                .verify(&files)
                .into_iter()
                .filter_map(|p| {
                    let how = match p.code {
                        "integrity_mismatch" => "değişmiş",
                        "integrity_missing" => "eksik",
                        "integrity_unlisted" => "sonradan eklenmiş",
                        _ => return None,
                    };
                    Some(format!("{} {how}", p.file.unwrap_or_default()))
                })
                .collect();
            if found.is_empty() {
                String::new()
            } else {
                format!(": {}", found.join(", "))
            }
        }
        None => ": paketin dosya listesi (integrity.json) de değişmiş".to_owned(),
    };
    Err(format!("Kurulu dosyalar kurulduğu hâlinden farklı{which}."))
}

/// What to do when the files are not as installed.
const MEND: &str = "Eklentiyi yeniden kurun ya da önceki sürüme dönün.";

fn read_manifest(layout: &Layout, id: &str, version: &str) -> Result<Manifest, Problem> {
    let path = layout.package(id, version).join("manifest.json");
    let text = fs::read_to_string(&path).map_err(|e| store::io("Manifest okunamadı", &path, &e))?;
    Manifest::read(&text)
}

impl Registry {
    /// Opens the registry under `root` (the user's extension folder). A version left half-prepared by a crash is
    /// removed; nothing is activated until [`Registry::startup`] and nothing else is written.
    pub fn open(
        root: impl Into<PathBuf>,
        host: Host,
        options: Options,
    ) -> Result<Registry, Problem> {
        let layout = Layout { root: root.into() };
        let layout_for_bisect = layout.clone();
        let journal = Journal::new(&layout.root);
        let mut file = match layout.read() {
            Ok(file) => file,
            Err(problem) => {
                // Said once until it changes: the program tries at every start.
                journal.write_changed(
                    Record::error(problem.code, problem.message.clone()).part("kayıt defteri"),
                );
                return Err(problem);
            }
        };
        carry_history(&journal, &mut file);
        let logs = file
            .extensions
            .keys()
            .map(|id| (id.clone(), journal.tail(Some(id), LOG_LINES)))
            .collect();
        let revocations = fs::read_to_string(layout.revocations())
            .ok()
            .and_then(|text| Revocations::read(&text).ok())
            .unwrap_or_default();
        // A version left half-prepared by a crash: an hour old at least, so another process's install in progress
        // (seconds) is never taken from under it.
        if let Ok(entries) = fs::read_dir(layout.staging()) {
            for entry in entries.flatten() {
                let stale = entry
                    .metadata()
                    .and_then(|m| m.modified())
                    .ok()
                    .and_then(|t| t.elapsed().ok())
                    .is_none_or(|age| age > Duration::from_secs(3600));
                if stale {
                    let _ = store::remove_tree(&entry.path());
                }
            }
        }
        let mut registry = Registry {
            layout,
            host,
            options,
            file,
            revocations,
            manifests: BTreeMap::new(),
            leases: BTreeMap::new(),
            registered: Vec::new(),
            generation: 0,
            bisect: read_bisect(&layout_for_bisect),
            journal,
            logs,
        };
        let ids: Vec<String> = registry.file.extensions.keys().cloned().collect();
        for id in ids {
            registry.reload_manifest(&id);
        }
        Ok(registry)
    }

    fn reload_manifest(&mut self, id: &str) {
        match self.file.extensions.get(id) {
            Some(entry) => match read_manifest(&self.layout, id, &entry.active) {
                Ok(manifest) => {
                    self.manifests.insert(id.to_owned(), manifest);
                }
                Err(_) => {
                    self.manifests.remove(id);
                }
            },
            None => {
                self.manifests.remove(id);
            }
        }
    }

    /// A change of the registry: under the lock, the file read again (another process may have changed it), the
    /// change applied and the file written whole.
    fn change<T>(
        &mut self,
        change: impl FnOnce(&mut RegistryFile) -> Result<T, Problem>,
    ) -> Result<T, Problem> {
        let layout = self.layout.clone();
        let journal = self.journal.clone();
        let (file, out) = layout.locked(self.options.lock_wait, || {
            let mut file = layout.read()?;
            carry_history(&journal, &mut file);
            let out = change(&mut file)?;
            layout.write(&file)?;
            Ok((file, out))
        })?;
        self.file = file;
        Ok(out)
    }

    /// The logs (`<kayıt>/logs`).
    pub fn journal(&self) -> &Journal {
        &self.journal
    }

    /// Writes a record to its extension's log (the registry's when it names none), with the version in use; what the
    /// manager and the dump show follows. Returns it as written (masked).
    pub fn write_log(&mut self, mut record: Record) -> Record {
        if let Some(id) = &record.extension
            && record.version.is_none()
        {
            record.version = self.file.extensions.get(id).map(|e| e.active.clone());
        }
        let record = self.journal.write(record);
        self.keep_log(&record);
        record
    }

    fn log(&mut self, id: &str, record: Record) {
        self.write_log(record.of(id, None));
    }

    /// Writes a record unless the extension's newest is the same event (a state said at every start).
    fn log_changed(&mut self, id: &str, record: Record) {
        let version = self.file.extensions.get(id).map(|e| e.active.clone());
        let record = self.journal.prepare(record.of(id, version.as_deref()));
        if self
            .logs
            .get(id)
            .and_then(|l| l.last())
            .is_some_and(|last| last.same_event(&record))
        {
            return;
        }
        self.journal.append(&record);
        self.keep_log(&record);
    }

    fn keep_log(&mut self, record: &Record) {
        if let Some(id) = &record.extension
            && self.file.extensions.contains_key(id)
        {
            let kept = self.logs.entry(id.clone()).or_default();
            kept.push(record.clone());
            let extra = kept.len().saturating_sub(LOG_LINES);
            kept.drain(..extra);
        }
    }

    /// The newest records of an installed extension's log, oldest first.
    pub fn log_of(&self, id: &str) -> &[Record] {
        self.logs.get(id).map_or(&[], Vec::as_slice)
    }

    // ---- What is shown ----

    /// The state of an installed extension.
    pub fn state(&self, id: &str) -> Option<State> {
        self.file
            .extensions
            .get(id)
            .map(|entry| self.state_of(id, entry))
    }

    fn state_of(&self, id: &str, entry: &Entry) -> State {
        if self.leases.contains_key(id) {
            return State::Active;
        }
        match &entry.failure {
            Some(Failure::Crashed { reason }) | Some(Failure::Integrity { reason }) => {
                return State::Failed {
                    reason: reason.clone(),
                };
            }
            None => {}
        }
        if let Some(reason) = self.blocked(id, entry) {
            return State::Blocked { reason };
        }
        let Some(manifest) = self.manifests.get(id) else {
            return State::Failed {
                reason: "Kurulu sürümün manifesti okunamadı; eklentiyi yeniden kurun.".to_owned(),
            };
        };
        if let Err(p) = compatible(&manifest.compatibility, &self.host) {
            return State::Incompatible { reason: p.message };
        }
        if !entry.enabled {
            return State::Disabled;
        }
        // A round of "Sorunlu eklentiyi bul" runs only its own (the first, none).
        if self
            .bisect
            .as_ref()
            .is_some_and(|b| !b.testing().iter().any(|t| t == id))
        {
            return State::Held;
        }
        if let Some(reason) = self.dependency_problem(manifest) {
            return State::Waiting { reason };
        }
        State::Ready
    }

    /// Why the version in use may not run: withdrawn, or the policy refuses it.
    fn blocked(&self, id: &str, entry: &Entry) -> Option<String> {
        if let Some(reason) = self.revocations.reason(id, &entry.active) {
            return Some(format!("{} sürümü geri çekildi: {reason}.", entry.active));
        }
        let manifest = self.manifests.get(id)?;
        self.options
            .policy
            .as_ref()?
            .refusal(manifest, entry.source.kind)
    }

    fn dependency_problem(&self, manifest: &Manifest) -> Option<String> {
        manifest.dependencies.extensions.iter().find_map(|d| {
            let status = self.dependency_status(d);
            (!status.met).then_some(status.note)
        })
    }

    fn dependency_status(&self, dependency: &ExtensionDependency) -> DependencyStatus {
        let id = &dependency.id;
        let range = VersionReq::parse(&dependency.version);
        let (met, note) = match self.file.extensions.get(id) {
            None => (
                false,
                format!("“{id}” kurulu değil ({} gerekli).", dependency.version),
            ),
            Some(entry) => {
                let in_range = Version::parse(&entry.active)
                    .is_some_and(|v| range.as_ref().is_some_and(|r| r.matches(&v)));
                if !in_range {
                    (
                        false,
                        format!(
                            "“{id}” {} kurulu; {} gerekli.",
                            entry.active, dependency.version
                        ),
                    )
                } else if !self.leases.contains_key(id) {
                    (false, format!("“{id}” etkin değil."))
                } else {
                    (true, format!("“{id}” {} etkin.", entry.active))
                }
            }
        };
        DependencyStatus {
            id: id.clone(),
            versions: dependency.version.clone(),
            met,
            note,
        }
    }

    fn trust(&self, id: &str, entry: &Entry) -> Trust {
        if matches!(entry.failure, Some(Failure::Integrity { .. })) {
            Trust::IntegrityBroken
        } else if self.blocked(id, entry).is_some() {
            Trust::Blocked
        } else {
            match entry.source.kind {
                SourceKind::File | SourceKind::DeveloperFolder => Trust::Local,
                SourceKind::Organization => Trust::Organization,
                SourceKind::Public => Trust::Approved,
            }
        }
    }

    /// Every installed extension, as the manager lists it.
    pub fn extensions(&self) -> Vec<ExtensionView> {
        self.file
            .extensions
            .iter()
            .map(|(id, entry)| {
                let state = self.state_of(id, entry);
                let trust = self.trust(id, entry);
                ExtensionView {
                    id: id.clone(),
                    name: self
                        .manifests
                        .get(id)
                        .map_or_else(|| id.clone(), |m| m.name.clone()),
                    version: entry.active.clone(),
                    state_label: state.label(),
                    help: state.help(),
                    state,
                    trust,
                    trust_label: trust.label(),
                    source: entry.source.clone(),
                    enabled: entry.enabled,
                    versions: std::iter::once(entry.active.clone())
                        .chain(
                            entry
                                .installed
                                .iter()
                                .map(|v| v.version.clone())
                                .filter(|v| *v != entry.active),
                        )
                        .collect(),
                    channel: entry.channel,
                    pinned: entry.pinned.clone(),
                    metrics: entry.metrics,
                    problem: journal::problem_in(self.log_of(id)).cloned(),
                    log: self.log_of(id).to_vec(),
                }
            })
            .collect()
    }

    /// What the active extensions add, each under its lease.
    pub fn contributions(&self) -> &[Registered] {
        &self.registered
    }

    /// What every running or ready extension adds or would add, worked out without activating anything or writing
    /// the registry (`kentos kek dump`, the MCP resource): a ready one's under a lease of generation 0. With what was
    /// left out and why.
    pub fn planned_contributions(&self) -> (Vec<Registered>, Vec<Skipped>) {
        let mut registered = self.registered.clone();
        let mut skipped = Vec::new();
        for (id, entry) in &self.file.extensions {
            if self.leases.contains_key(id) || self.state_of(id, entry) != State::Ready {
                continue;
            }
            let Some(manifest) = self.manifests.get(id) else {
                continue;
            };
            let lease = ExtensionLease {
                extension: id.clone(),
                version: entry.active.clone(),
                generation: 0,
            };
            let (more, left) = lease::contributions(manifest, &entry.grant, &lease);
            registered.extend(more);
            skipped.extend(left);
        }
        (registered, skipped)
    }

    /// Every installed extension with its state and what the running or ready ones add, as `kentos kek dump` prints it
    /// and the MCP server's `kentos://extensions/installed` gives it (plan/eklenti.md ADIM 18, Ö·38): the evidence for
    /// "where did my tab go?" in one shape for both. Activates nothing.
    pub fn dump(&self) -> serde_json::Value {
        let (contributions, skipped) = self.planned_contributions();
        serde_json::json!({
            "root": self.layout.root.display().to_string(),
            "extensions": self.extensions(),
            "contributions": contributions,
            "skipped": skipped,
        })
    }

    /// What the user granted an extension at install: its capabilities and whether its Python contributions are
    /// wanted (an environment is prepared only then, PY-KEK-17).
    pub fn grant(&self, id: &str) -> Option<&crate::store::Grant> {
        self.file.extensions.get(id).map(|entry| &entry.grant)
    }

    /// The version in use's manifest.
    pub fn manifest(&self, id: &str) -> Option<&Manifest> {
        self.manifests.get(id)
    }

    /// The folder of the version in use, read-only: where a form's or a GeoPackage's file is read from.
    pub fn package_dir(&self, id: &str) -> Option<PathBuf> {
        let entry = self.file.extensions.get(id)?;
        Some(self.layout.package(id, &entry.active))
    }

    /// The content digest of the version in use (`sha256:…`), the one its files are checked against: what a runner
    /// tells the extension's process it runs (plan/eklenti.md ADIM 19, PY-RUN-04).
    pub fn digest(&self, id: &str) -> Option<String> {
        let entry = self.file.extensions.get(id)?;
        entry.version(&entry.active).map(|v| v.digest.clone())
    }

    /// A file of the version in use, by its path in the package; none when it has no such file.
    pub fn package_file(&self, id: &str, path: &str) -> Option<Vec<u8>> {
        if kentos_extensions::manifest::is_extension_id(id)
            && !path.split('/').any(|p| p.is_empty() || p == "..")
        {
            let full = path
                .split('/')
                .fold(self.package_dir(id)?, |dir, part| dir.join(part));
            return fs::read(full).ok();
        }
        None
    }

    /// The lease of a running extension.
    pub fn lease(&self, id: &str) -> Option<&ExtensionLease> {
        self.leases.get(id)
    }

    /// Whether a lease is the running one: a callback with another is not applied (PY-RUN-10).
    pub fn is_current(&self, lease: &ExtensionLease) -> bool {
        self.leases.get(&lease.extension) == Some(lease)
    }

    // ---- Running ----

    /// Activates every enabled extension that can run, its dependencies first; nothing in safe mode.
    pub fn startup(&mut self) -> Vec<(String, Result<Activation, Problem>)> {
        if self.options.safe_mode {
            return Vec::new();
        }
        let mut out = Vec::new();
        for id in self.order() {
            match self.state(&id) {
                Some(State::Ready) => {
                    let result = self.activate(&id);
                    out.push((id, result));
                }
                // Enabled but not started: why, once until it changes.
                Some(
                    state @ (State::Waiting { .. }
                    | State::Blocked { .. }
                    | State::Incompatible { .. }
                    | State::Failed { .. }),
                ) => {
                    let code = match state {
                        State::Failed { .. } => "not_started_failed",
                        State::Blocked { .. } => "not_started_blocked",
                        State::Incompatible { .. } => "not_started_incompatible",
                        _ => "not_started_waiting",
                    };
                    let record = Record::warning(
                        code,
                        format!(
                            "Başlangıçta çalıştırılmadı — {}: {}",
                            state.label(),
                            state.help()
                        ),
                    )
                    .part("başlangıç");
                    self.log_changed(&id, record);
                }
                _ => {}
            }
        }
        out
    }

    /// The installed extensions, every dependency before its dependents.
    fn order(&self) -> Vec<String> {
        fn visit(
            id: &str,
            manifests: &BTreeMap<String, Manifest>,
            seen: &mut BTreeSet<String>,
            out: &mut Vec<String>,
        ) {
            if !seen.insert(id.to_owned()) {
                return;
            }
            if let Some(manifest) = manifests.get(id) {
                for dependency in &manifest.dependencies.extensions {
                    visit(&dependency.id, manifests, seen, out);
                }
            }
            out.push(id.to_owned());
        }
        let mut seen = BTreeSet::new();
        let mut out = Vec::new();
        for id in self.file.extensions.keys() {
            visit(id, &self.manifests, &mut seen, &mut out);
        }
        out.retain(|id| self.file.extensions.contains_key(id));
        out
    }

    /// The installed extensions that depend on `id`, directly or not.
    fn dependents(&self, id: &str) -> Vec<String> {
        let mut found: Vec<String> = Vec::new();
        let mut frontier = vec![id.to_owned()];
        while let Some(at) = frontier.pop() {
            for (other, manifest) in &self.manifests {
                if !found.contains(other)
                    && other != id
                    && manifest.dependencies.extensions.iter().any(|d| d.id == at)
                {
                    found.push(other.clone());
                    frontier.push(other.clone());
                }
            }
        }
        found
    }

    /// The active extensions that depend on `id`, directly or not, the farthest first.
    fn active_dependents(&self, id: &str) -> Vec<String> {
        let mut found: Vec<String> = Vec::new();
        let mut frontier = vec![id.to_owned()];
        while let Some(at) = frontier.pop() {
            for (other, manifest) in &self.manifests {
                if self.leases.contains_key(other)
                    && !found.contains(other)
                    && other != id
                    && manifest.dependencies.extensions.iter().any(|d| d.id == at)
                {
                    found.push(other.clone());
                    frontier.push(other.clone());
                }
            }
        }
        found.reverse();
        found
    }

    /// Activates an installed, enabled extension whose state is ready: its files checked against the digest, then
    /// its contributions registered under a new lease.
    pub fn activate(&mut self, id: &str) -> Result<Activation, Problem> {
        let entry = self
            .file
            .extensions
            .get(id)
            .cloned()
            .ok_or_else(|| not_installed(id))?;
        let state = self.state_of(id, &entry);
        if state != State::Ready {
            return Err(problem(
                "registry_not_ready",
                format!(
                    "“{id}” etkinleştirilemiyor: {}. {}",
                    state.label(),
                    state.help()
                ),
            ));
        }
        let started = Instant::now();
        let digest = entry
            .version(&entry.active)
            .map(|v| v.digest.clone())
            .unwrap_or_default();
        if let Err(reason) = verify_files(&self.layout.package(id, &entry.active), &digest) {
            let failed = format!("{reason} {MEND}");
            let said = failed.clone();
            let owner = id.to_owned();
            self.change(move |file| {
                if let Some(entry) = file.extensions.get_mut(&owner) {
                    entry.failure = Some(Failure::Integrity { reason: failed });
                }
                Ok(())
            })?;
            self.log(
                id,
                Record::error(
                    "integrity_mismatch",
                    "Dosyaları kurulduğu gibi değil; çalıştırılmadı. Eklentiyi yeniden kurun ya da önceki sürüme dönün.",
                )
                .part("etkinleştirme")
                .detail(reason),
            );
            return Err(problem("registry_integrity", said));
        }
        let manifest = match read_manifest(&self.layout, id, &entry.active) {
            Ok(manifest) => manifest,
            Err(p) => {
                self.log(
                    id,
                    Record::error(
                        "manifest_unreadable",
                        "Kurulu sürümün manifesti okunamadı; çalıştırılmadı. Eklentiyi yeniden kurun.",
                    )
                    .part("etkinleştirme")
                    .detail(p.to_string()),
                );
                return Err(p);
            }
        };
        self.generation += 1;
        let lease = ExtensionLease {
            extension: id.to_owned(),
            version: entry.active.clone(),
            generation: self.generation,
        };
        let (registered, skipped) = lease::contributions(&manifest, &entry.grant, &lease);
        self.registered.extend(registered);
        self.leases.insert(id.to_owned(), lease.clone());
        self.manifests.insert(id.to_owned(), manifest);
        let micros = u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX);
        let owner = id.to_owned();
        self.change(move |file| {
            if let Some(entry) = file.extensions.get_mut(&owner) {
                let before = entry.metrics.unwrap_or_default();
                entry.metrics = Some(Metrics {
                    activation_micros: micros,
                    ..before
                });
            }
            Ok(())
        })?;
        let record = Record::info(
            "activated",
            format!("Etkinleştirildi ({:.1} ms).", micros as f64 / 1000.0),
        )
        .part("etkinleştirme");
        self.log(id, record);
        Ok(Activation {
            lease,
            skipped,
            micros,
        })
    }

    /// Revokes a lease: every contribution registered under it goes at once. Returns how many.
    pub fn revoke_lease(&mut self, lease: &ExtensionLease) -> usize {
        let before = self.registered.len();
        self.registered.retain(|r| &r.lease != lease);
        if self.leases.get(&lease.extension) == Some(lease) {
            self.leases.remove(&lease.extension);
        }
        before - self.registered.len()
    }

    /// Stops an extension and every active one that depends on it; returns the dependents stopped.
    fn stop(&mut self, id: &str) -> Vec<String> {
        let dependents = self.active_dependents(id);
        for other in dependents.iter().map(String::as_str).chain([id]) {
            if let Some(lease) = self.leases.get(other).cloned() {
                self.revoke_lease(&lease);
            }
        }
        dependents
    }

    /// Activates those of `ids` that can run now, dependencies first.
    fn restart(&mut self, ids: &[String]) {
        for id in self.order() {
            if ids.contains(&id) && self.state(&id) == Some(State::Ready) {
                let _ = self.activate(&id);
            }
        }
    }

    /// The user enables an extension: it runs when it can, and so do the ones that waited for it.
    pub fn enable(&mut self, id: &str) -> Result<Option<Activation>, Problem> {
        if !self.file.extensions.contains_key(id) {
            return Err(not_installed(id));
        }
        let owner = id.to_owned();
        let was = self.file.extensions.get(id).is_some_and(|e| e.enabled);
        self.change(move |file| {
            if let Some(entry) = file.extensions.get_mut(&owner) {
                entry.enabled = true;
            }
            Ok(())
        })?;
        if !was {
            self.log(id, Record::info("enabled", "Kullanıcı etkinleştirdi."));
        }
        if self.options.safe_mode || self.leases.contains_key(id) {
            return Ok(None);
        }
        let activation = match self.state(id) {
            Some(State::Ready) => Some(self.activate(id)?),
            _ => None,
        };
        let waiting = self.dependents(id);
        self.restart(&waiting);
        Ok(activation)
    }

    /// The user disables an extension: it stops, and so do the ones that depend on it (they wait for it).
    pub fn disable(&mut self, id: &str) -> Result<Vec<String>, Problem> {
        if !self.file.extensions.contains_key(id) {
            return Err(not_installed(id));
        }
        let dependents = self.stop(id);
        let owner = id.to_owned();
        self.change(move |file| {
            if let Some(entry) = file.extensions.get_mut(&owner) {
                entry.enabled = false;
            }
            Ok(())
        })?;
        self.log(
            id,
            Record::info("disabled", "Kullanıcı devre dışı bıraktı."),
        );
        for other in &dependents {
            self.log(
                other,
                Record::warning(
                    "stopped_dependency",
                    format!("Durduruldu: bağımlı olduğu “{id}” devre dışı bırakıldı."),
                ),
            );
        }
        Ok(dependents)
    }

    // ---- Installing ----

    /// The package's id and the dependency cycle it would close, when it would.
    fn cycle_with(&self, manifest: &Manifest) -> Option<Vec<String>> {
        let mut graph: Vec<(&str, Vec<&str>)> = self
            .manifests
            .iter()
            .filter(|(id, _)| **id != manifest.id)
            .map(|(id, m)| {
                (
                    id.as_str(),
                    m.dependencies
                        .extensions
                        .iter()
                        .map(|d| d.id.as_str())
                        .collect(),
                )
            })
            .collect();
        graph.push((
            manifest.id.as_str(),
            manifest
                .dependencies
                .extensions
                .iter()
                .map(|d| d.id.as_str())
                .collect(),
        ));
        dependency_cycle(&graph)
    }

    /// Everything to show before the user decides; installs nothing.
    pub fn review(&self, bytes: &[u8], source: &Source) -> Review {
        let inspection = kentos_extensions::inspect(bytes, &self.options.limits);
        let manifest = inspection.manifest.clone();
        let mut review = Review {
            inspection,
            incompatible: None,
            policy: None,
            revoked: None,
            dependencies: Vec::new(),
            cycle: None,
            installed: None,
            source_conflict: false,
            capabilities: Vec::new(),
            installable: false,
        };
        let Some(manifest) = manifest else {
            return review;
        };
        let installed = self.manifests.get(&manifest.id);
        review.capabilities = capability_lines(&manifest, installed);
        if !review.inspection.is_valid() {
            return review;
        }
        review.incompatible = compatible(&manifest.compatibility, &self.host)
            .err()
            .map(|p| p.message);
        review.policy = self
            .options
            .policy
            .as_ref()
            .and_then(|p| p.refusal(&manifest, source.kind));
        review.revoked = self
            .revocations
            .reason(&manifest.id, &manifest.version)
            .map(|r| format!("{} sürümü geri çekildi: {r}.", manifest.version));
        review.dependencies = manifest
            .dependencies
            .extensions
            .iter()
            .map(|d| self.dependency_status(d))
            .collect();
        review.cycle = self.cycle_with(&manifest);
        if let Some(entry) = self.file.extensions.get(&manifest.id) {
            review.source_conflict = conflicts(&entry.source, source);
            review.installed = Some((entry.active.clone(), entry.source.clone()));
        }
        review.installable = review.incompatible.is_none()
            && review.policy.is_none()
            && review.revoked.is_none()
            && review.cycle.is_none();
        review
    }

    /// Installs (or updates to) a package with the user's choice. Every check runs again on these bytes; the files
    /// are prepared beside the installed ones and the registry switches in one write; the version in use stays as it
    /// was until then. A version is kept to go back to.
    pub fn install(
        &mut self,
        bytes: &[u8],
        source: &Source,
        choice: &Choice,
    ) -> Result<Installed, Vec<Problem>> {
        let result = self.install_checked(bytes, source, choice);
        if let Err(problems) = &result {
            // Whose and which version, when the package's manifest could be read; the registry's log otherwise.
            let manifest = kentos_extensions::open(bytes, &self.options.limits)
                .inspection
                .manifest;
            let first = problems
                .first()
                .map(|p| p.message.clone())
                .unwrap_or_default();
            let detail: Vec<String> = problems.iter().map(ToString::to_string).collect();
            let what = manifest.as_ref().map_or_else(
                || "Paket".to_owned(),
                |m| format!("{} {}", m.name, m.version),
            );
            let mut record = Record::warning(
                "install_refused",
                format!("{what} kurulmadı ({}): {first}", source.kind.words()),
            )
            .part("kurulum")
            .detail(detail.join("\n"));
            if let Some(m) = &manifest {
                record = record.of(&m.id, None);
            }
            self.write_log(record);
        }
        result
    }

    fn install_checked(
        &mut self,
        bytes: &[u8],
        source: &Source,
        choice: &Choice,
    ) -> Result<Installed, Vec<Problem>> {
        let opened = kentos_extensions::open(bytes, &self.options.limits);
        if !opened.inspection.is_valid() {
            return Err(opened.inspection.problems);
        }
        let one = |p: Problem| vec![p];
        let (Some(manifest), Some(digest)) = (
            opened.inspection.manifest.clone(),
            opened.inspection.digest.clone(),
        ) else {
            return Err(one(problem("registry_invalid", "Paket okunamadı.")));
        };
        let id = manifest.id.clone();
        let version = manifest.version.clone();
        compatible(&manifest.compatibility, &self.host).map_err(one)?;
        if let Some(reason) = self
            .options
            .policy
            .as_ref()
            .and_then(|p| p.refusal(&manifest, source.kind))
        {
            return Err(one(problem("registry_policy", reason)));
        }
        if let Some(reason) = self.revocations.reason(&id, &version) {
            return Err(one(problem(
                "registry_revoked",
                format!("{id} {version} geri çekildi: {reason}. Kurulmaz."),
            )));
        }
        if let Some(cycle) = self.cycle_with(&manifest) {
            return Err(one(problem(
                "registry_dependency_cycle",
                format!(
                    "Bağımlılık döngüsü: {}. Eklentiler birbirine döngüyle bağlanamaz.",
                    cycle.join(" → ")
                ),
            )));
        }
        let requested = manifest.capabilities();
        if let Some(c) = choice
            .grant
            .capabilities
            .iter()
            .find(|c| !requested.contains(c))
        {
            return Err(one(problem(
                "registry_grant",
                format!(
                    "“{}” yeteneği paketin istemediği bir yetenek; verilemez.",
                    c.id()
                ),
            )));
        }
        let existing = self.file.extensions.get(&id).cloned();
        if let Some(entry) = &existing {
            if conflicts(&entry.source, source) && !choice.replace_source {
                return Err(one(problem(
                    "registry_source_conflict",
                    format!(
                        "“{id}” {} kaynağından kurulu; bu paket {} kaynağından. Hangisini kullanacağınızı seçin.",
                        entry.source.kind.words(),
                        source.kind.words()
                    ),
                )));
            }
            if let Some(same) = entry.version(&version) {
                let intact = !matches!(entry.failure, Some(Failure::Integrity { .. }))
                    && verify_files(&self.layout.package(&id, &version), &digest).is_ok();
                if same.digest == digest
                    && entry.source == *source
                    && entry.active == version
                    && intact
                {
                    return self.regrant(&id, &version, choice).map_err(one);
                }
                if same.digest != digest && source.kind != SourceKind::DeveloperFolder {
                    return Err(one(problem(
                        "registry_same_version",
                        format!(
                            "{id} {version} kurulu ama bu paketin içeriği farklı (içerik özeti tutmuyor). Aynı sürüm \
                             iki farklı paket olamaz; yazarından yeni bir sürüm isteyin."
                        ),
                    )));
                }
            }
        }
        // Prepared beside the installed versions: written, read back, checked.
        self.generation += 1;
        let staging = self.layout.staging().join(format!(
            "{id}@{version}.{}.{}",
            std::process::id(),
            self.generation
        ));
        let prepared = store::write_files(&staging, &opened.files).and_then(|()| {
            verify_files(&staging, &digest).map_err(|m| problem("registry_integrity", m))
        });
        if let Err(p) = prepared {
            let _ = store::remove_tree(&staging);
            return Err(one(p));
        }
        let target = self.layout.package(&id, &version);
        let placed = (|| -> Result<(), Problem> {
            if target.exists() {
                // A working folder's same version again: the old files out of the way first.
                let old = staging.with_extension("eski");
                fs::rename(&target, &old)
                    .map_err(|e| store::io("Eski sürüm kaldırılamadı", &target, &e))?;
                fs::rename(&staging, &target)
                    .map_err(|e| store::io("Paket yerine konamadı", &target, &e))?;
                store::remove_tree(&old)
            } else {
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent)
                        .map_err(|e| store::io("Klasör oluşturulamadı", parent, &e))?;
                }
                fs::rename(&staging, &target)
                    .map_err(|e| store::io("Paket yerine konamadı", &target, &e))
            }
        })();
        if let Err(p) = placed {
            let _ = store::remove_tree(&staging);
            return Err(one(p));
        }
        let previous = existing.as_ref().map(|e| e.active.clone());
        let preview = manifest.is_preview();
        let new_version = version.clone();
        let owner = id.to_owned();
        let source_now = source.clone();
        let grant = choice.grant.clone();
        let enable = choice.enable;
        let before = previous.clone();
        let removed = self
            .change(move |file| {
                let fresh = !file.extensions.contains_key(&owner);
                let entry = file.extensions.entry(owner).or_insert_with(|| Entry {
                    source: source_now.clone(),
                    installed: Vec::new(),
                    active: new_version.clone(),
                    enabled: enable,
                    grant: grant.clone(),
                    channel: if preview {
                        Channel::Preview
                    } else {
                        Channel::Stable
                    },
                    pinned: None,
                    failure: None,
                    crashes: 0,
                    metrics: None,
                    log: Vec::new(),
                });
                entry.source = source_now.clone();
                entry.installed.retain(|v| v.version != new_version);
                entry.installed.insert(
                    0,
                    InstalledVersion {
                        version: new_version.clone(),
                        digest,
                        installed_at: store::now(),
                    },
                );
                let keep: Vec<String> = [new_version.clone()]
                    .into_iter()
                    .chain(before.clone())
                    .collect();
                let removed: Vec<String> = entry
                    .installed
                    .iter()
                    .filter(|v| !keep.contains(&v.version))
                    .map(|v| v.version.clone())
                    .collect();
                entry.installed.retain(|v| keep.contains(&v.version));
                entry.active = new_version.clone();
                entry.grant = grant;
                if enable {
                    entry.enabled = true;
                }
                entry.failure = None;
                entry.crashes = 0;
                let said = match &before {
                    _ if fresh => Record::info(
                        "installed",
                        format!("{new_version} kuruldu ({}).", source_now.kind.words()),
                    ),
                    Some(old) if *old != new_version => Record::info(
                        "updated",
                        format!(
                            "{old} → {new_version} güncellendi ({}).",
                            source_now.kind.words()
                        ),
                    ),
                    _ => Record::info(
                        "reinstalled",
                        format!(
                            "{new_version} yeniden kuruldu ({}).",
                            source_now.kind.words()
                        ),
                    ),
                };
                Ok((removed, said))
            })
            .map_err(one)?;
        let (removed, said) = removed;
        if !self.logs.contains_key(&id) {
            // Installed again after a removal: its history goes on.
            let earlier = self.journal.tail(Some(&id), LOG_LINES);
            self.logs.insert(id.clone(), earlier);
        }
        self.log(&id, said.part("kurulum"));
        for old in removed {
            let _ = store::remove_tree(&self.layout.package(&id, &old));
        }
        // The old lease goes (a running job keeps its own copy of it); the new version starts when enabled.
        let dependents = self.stop(&id);
        self.reload_manifest(&id);
        let enabled = self.file.extensions.get(&id).is_some_and(|e| e.enabled);
        let activation = (enabled && !self.options.safe_mode).then(|| self.activate(&id));
        // The ones that were running on the old version, and the ones that waited for this one.
        let mut waiting = self.dependents(&id);
        waiting.extend(dependents);
        self.restart(&waiting);
        Ok(Installed {
            id,
            version,
            previous,
            already: false,
            activation,
        })
    }

    /// The same package installed again with another choice: the grant changes and the extension starts again under
    /// a new lease; the same choice changes nothing.
    fn regrant(&mut self, id: &str, version: &str, choice: &Choice) -> Result<Installed, Problem> {
        let entry = self
            .file
            .extensions
            .get(id)
            .cloned()
            .ok_or_else(|| not_installed(id))?;
        let unchanged = entry.grant == choice.grant && (entry.enabled || !choice.enable);
        let mut installed = Installed {
            id: id.to_owned(),
            version: version.to_owned(),
            previous: Some(entry.active.clone()),
            already: true,
            activation: None,
        };
        if unchanged {
            return Ok(installed);
        }
        let owner = id.to_owned();
        let grant = choice.grant.clone();
        let enable = choice.enable;
        self.change(move |file| {
            if let Some(entry) = file.extensions.get_mut(&owner) {
                entry.grant = grant;
                if enable {
                    entry.enabled = true;
                }
            }
            Ok(())
        })?;
        self.log(
            id,
            Record::info("grants_changed", "Kullanıcı izinleri değiştirdi."),
        );
        let mut waiting = self.stop(id);
        if self.state(id) == Some(State::Ready) && !self.options.safe_mode {
            installed.activation = Some(self.activate(id));
        }
        waiting.extend(self.dependents(id));
        self.restart(&waiting);
        Ok(installed)
    }

    /// Packs an author's working folder and installs it from there (D2); its later reloads replace the same version.
    pub fn install_folder(
        &mut self,
        folder: &Path,
        choice: &Choice,
    ) -> Result<Installed, Vec<Problem>> {
        let files = store::read_files(folder).map_err(|p| vec![p])?;
        let packed = kentos_extensions::pack(&files, &self.options.limits)?;
        let source = Source {
            kind: SourceKind::DeveloperFolder,
            location: folder.display().to_string(),
        };
        self.install(&packed.bytes, &source, choice)
    }

    /// Reloads a working folder's extension: packed again, installed over itself under a new lease; nothing of the
    /// old lease stays (PY-RUN-09).
    pub fn reload(&mut self, id: &str) -> Result<Installed, Vec<Problem>> {
        let entry = self
            .file
            .extensions
            .get(id)
            .cloned()
            .ok_or_else(|| vec![not_installed(id)])?;
        if entry.source.kind != SourceKind::DeveloperFolder {
            return Err(vec![problem(
                "registry_not_developer",
                format!(
                    "“{id}” bir geliştirici klasöründen kurulmadı; yeniden yükleme yalnız onlar için."
                ),
            )]);
        }
        let choice = Choice {
            grant: entry.grant.clone(),
            enable: entry.enabled,
            replace_source: false,
        };
        self.install_folder(Path::new(&entry.source.location), &choice)
    }

    /// Goes to another installed version (back to the one before, or forward again): its files checked first.
    pub fn switch_version(
        &mut self,
        id: &str,
        version: &str,
    ) -> Result<Option<Activation>, Problem> {
        let entry = self
            .file
            .extensions
            .get(id)
            .cloned()
            .ok_or_else(|| not_installed(id))?;
        let Some(target) = entry.version(version) else {
            return Err(problem(
                "registry_version_missing",
                format!("“{id}” için {version} sürümü bu bilgisayarda yok."),
            ));
        };
        if let Err(reason) = verify_files(&self.layout.package(id, version), &target.digest) {
            self.log(
                id,
                Record::error(
                    "integrity_mismatch",
                    format!("{version} sürümüne geçilemedi: dosyaları kurulduğu gibi değil."),
                )
                .part("sürüm değiştirme")
                .detail(reason.clone()),
            );
            return Err(problem("registry_integrity", format!("{reason} {MEND}")));
        }
        let owner = id.to_owned();
        let to = version.to_owned();
        let from = entry.active.clone();
        self.change(move |file| {
            if let Some(entry) = file.extensions.get_mut(&owner) {
                entry.active = to.clone();
                entry.failure = None;
                entry.crashes = 0;
            }
            Ok(())
        })?;
        self.log(
            id,
            Record::info("switched", format!("{from} → {version} sürümüne geçildi."))
                .part("sürüm değiştirme"),
        );
        let dependents = self.stop(id);
        self.reload_manifest(id);
        let activation = match self.state(id) {
            Some(State::Ready) if !self.options.safe_mode => Some(self.activate(id)?),
            _ => None,
        };
        let mut waiting = self.dependents(id);
        waiting.extend(dependents);
        self.restart(&waiting);
        Ok(activation)
    }

    /// Removes an extension: its lease, files and registry entry; its own data folder only when asked (PY-KEK-16:
    /// the user's outputs and independent form releases are not the extension's to delete).
    pub fn remove(&mut self, id: &str, keep_data: bool) -> Result<Vec<String>, Problem> {
        if !self.file.extensions.contains_key(id) {
            return Err(not_installed(id));
        }
        let dependents = self.stop(id);
        let owner = id.to_owned();
        let version = self.file.extensions.get(id).map(|e| e.active.clone());
        self.change(move |file| {
            file.extensions.remove(&owner);
            Ok(())
        })?;
        // The log stays: installed again, its history goes on.
        self.journal.write(
            Record::info(
                "removed",
                if keep_data {
                    "Kaldırıldı; verisi duruyor."
                } else {
                    "Kaldırıldı; verisi de silindi."
                },
            )
            .of(id, version.as_deref()),
        );
        self.logs.remove(id);
        self.manifests.remove(id);
        store::remove_tree(&self.layout.packages_of(id))?;
        if !keep_data {
            store::remove_tree(&self.layout.data(id))?;
        }
        for other in &dependents {
            self.log(
                other,
                Record::warning(
                    "stopped_dependency",
                    format!("Durduruldu: bağımlı olduğu “{id}” kaldırıldı."),
                ),
            );
        }
        Ok(dependents)
    }

    // ---- The watchdog ----

    /// An activation or a run crashed. At [`CRASHES_TO_FAIL`] in a row the extension stops for good, with the reason;
    /// a report of a lease no longer running is ignored.
    pub fn report_crash(&mut self, lease: &ExtensionLease, reason: &str) -> Result<Crash, Problem> {
        if !self.is_current(lease) {
            return Ok(Crash::Ignored);
        }
        let owner = lease.extension.clone();
        let reason = reason.to_owned();
        let reason_now = reason.clone();
        let (count, failed) = self.change(move |file| {
            let Some(entry) = file.extensions.get_mut(&owner) else {
                return Ok((0, false));
            };
            entry.crashes += 1;
            let count = entry.crashes;
            let failed = count >= CRASHES_TO_FAIL;
            if failed {
                entry.failure = Some(Failure::Crashed {
                    reason: format!("Art arda {count} kez çöktü; son neden: {reason}."),
                });
            }
            Ok((count, failed))
        })?;
        let id = lease.extension.clone();
        self.log(
            &id,
            Record::error("crashed", format!("Çöktü ({count}. kez art arda)."))
                .part("çalışma")
                .detail(reason_now),
        );
        if failed {
            self.stop(&id);
            self.log(
                &id,
                Record::error(
                    "failed",
                    format!(
                        "Art arda {count} kez çöktüğü için durduruldu; açılışta denenmeyecek. Yayımcısına bildirin, \
                         sonra Yeniden dene."
                    ),
                )
                .part("bekçi"),
            );
            Ok(Crash::Failed)
        } else {
            Ok(Crash::Counted(count))
        }
    }

    /// An activation or a run went well: the crash count starts again.
    pub fn report_healthy(&mut self, lease: &ExtensionLease) -> Result<(), Problem> {
        if !self.is_current(lease)
            || self
                .file
                .extensions
                .get(&lease.extension)
                .is_none_or(|e| e.crashes == 0)
        {
            return Ok(());
        }
        let owner = lease.extension.clone();
        self.change(move |file| {
            if let Some(entry) = file.extensions.get_mut(&owner) {
                entry.crashes = 0;
            }
            Ok(())
        })
    }

    /// Records a run's measures (the Python process's memory and count, ADIM 19).
    pub fn record_metrics(
        &mut self,
        lease: &ExtensionLease,
        memory_bytes: u64,
        processes: u32,
    ) -> Result<(), Problem> {
        if !self.is_current(lease) {
            return Ok(());
        }
        let owner = lease.extension.clone();
        self.change(move |file| {
            if let Some(entry) = file.extensions.get_mut(&owner) {
                let mut metrics = entry.metrics.unwrap_or_default();
                metrics.memory_bytes = memory_bytes;
                metrics.processes = processes;
                entry.metrics = Some(metrics);
            }
            Ok(())
        })
    }

    /// The user retries a failed extension: the error and the count are cleared and it starts once more.
    pub fn retry(&mut self, id: &str) -> Result<Option<Activation>, Problem> {
        let owner = id.to_owned();
        let found = self.change(move |file| {
            Ok(file.extensions.get_mut(&owner).map(|entry| {
                entry.failure = None;
                entry.crashes = 0;
            }))
        })?;
        if found.is_none() {
            return Err(not_installed(id));
        }
        self.log(id, Record::info("retried", "Kullanıcı yeniden denedi."));
        match self.state(id) {
            Some(State::Ready) if !self.options.safe_mode => Ok(Some(self.activate(id)?)),
            _ => Ok(None),
        }
    }

    // ---- Sorunlu eklentiyi bul ----

    /// The bisection going on, if one.
    pub fn bisection(&self) -> Option<&Bisection> {
        self.bisect.as_ref()
    }

    /// Starts finding the extension behind a problem the user sees: the running extensions are the suspects; the
    /// first round runs none of them. The user's choices stay as they are: a round only holds extensions off
    /// ([`State::Held`]), the end brings every one back.
    pub fn start_bisection(&mut self) -> Result<Finding, Problem> {
        let suspects: Vec<String> = self
            .order()
            .into_iter()
            .filter(|id| self.leases.contains_key(id))
            .collect();
        if suspects.is_empty() {
            return Err(problem(
                "bisect_nothing",
                "Çalışan eklenti yok: aranacak bir eklenti de yok.",
            ));
        }
        let b = Bisection::new(suspects.clone());
        self.save_bisection(Some(&b))?;
        self.journal.write(
            Record::info(
                "bisect_started",
                format!(
                    "Sorunlu eklentiyi bulma başladı: {} eklenti ({}), en çok {} tur.",
                    suspects.len(),
                    suspects.join(", "),
                    Bisection::most_rounds(suspects.len())
                ),
            )
            .part("sorun bulma"),
        );
        let finding = b.finding();
        self.bisect = Some(b);
        self.apply_round();
        Ok(finding)
    }

    /// The user's answer for the round: whether the problem was still there. The next round runs, or the extension is
    /// named (in its log, as an error), or the problem is no extension's; at the end every extension is back.
    pub fn answer_bisection(&mut self, problem_seen: bool) -> Result<Finding, Problem> {
        let Some(mut b) = self.bisect.clone() else {
            return Err(problem("bisect_none", "Süren bir sorun bulma yok."));
        };
        b.answer(problem_seen);
        let finding = b.finding();
        let rounds = b.rounds();
        match &finding {
            Finding::Testing(_) => {
                self.save_bisection(Some(&b))?;
                self.bisect = Some(b);
                self.apply_round();
            }
            Finding::Found(id) => {
                self.finish_bisection()?;
                self.log(
                    id,
                    Record::error(
                        "bisect_found",
                        format!(
                            "Sorunlu eklentiyi bulma bu eklentiyi gösterdi ({rounds} turda): sorun yalnız o açıkken \
                             görüldü. Devre dışı bırakın ya da güncelleyin; sürerse yayımcısına bildirin."
                        ),
                    )
                    .part("sorun bulma"),
                );
            }
            Finding::NotAnExtension => {
                self.finish_bisection()?;
                self.journal.write(
                    Record::info(
                        "bisect_not_extension",
                        "Sorun hiçbir eklenti açık değilken de görüldü: eklentilerden değil.",
                    )
                    .part("sorun bulma"),
                );
            }
        }
        Ok(finding)
    }

    /// The user leaves the bisection: every extension back as it was.
    pub fn stop_bisection(&mut self) -> Result<(), Problem> {
        if self.bisect.is_none() {
            return Ok(());
        }
        self.finish_bisection()?;
        self.journal.write(
            Record::info(
                "bisect_stopped",
                "Sorunlu eklentiyi bulma bırakıldı; eklentiler eskisi gibi.",
            )
            .part("sorun bulma"),
        );
        Ok(())
    }

    fn finish_bisection(&mut self) -> Result<(), Problem> {
        self.save_bisection(None)?;
        self.bisect = None;
        self.apply_round();
        Ok(())
    }

    /// Runs what the round runs: the held extensions' leases go, the others that are ready start in order (every
    /// enabled one when no bisection goes on).
    fn apply_round(&mut self) {
        let held: Vec<ExtensionLease> = self
            .leases
            .iter()
            .filter(|(id, _)| self.state_of_entry(id) == Some(State::Held))
            .map(|(_, lease)| lease.clone())
            .collect();
        for lease in held {
            self.revoke_lease(&lease);
        }
        if self.options.safe_mode {
            return;
        }
        for id in self.order() {
            if self.state(&id) == Some(State::Ready) {
                let _ = self.activate(&id);
            }
        }
    }

    /// The state an extension would have without its lease: what a round decides by.
    fn state_of_entry(&self, id: &str) -> Option<State> {
        let entry = self.file.extensions.get(id)?;
        if !entry.enabled {
            return Some(State::Disabled);
        }
        self.bisect
            .as_ref()
            .is_some_and(|b| !b.testing().iter().any(|t| t == id))
            .then_some(State::Held)
    }

    fn save_bisection(&self, b: Option<&Bisection>) -> Result<(), Problem> {
        let path = self.layout.bisect();
        match b {
            Some(b) => {
                let file = BisectFile {
                    format: BISECT_FORMAT.to_owned(),
                    schema_version: 1,
                    bisection: b.clone(),
                };
                let text = serde_json::to_string_pretty(&file).unwrap_or_default();
                store::write_atomic(&path, text.as_bytes())
            }
            None => match fs::remove_file(&path) {
                Ok(()) => Ok(()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(e) => Err(store::io("Sorun bulma dosyası silinemedi", &path, &e)),
            },
        }
    }

    // ---- Trust ----

    /// Applies a revocation list (verified by its source): the withdrawn versions in use stop, their data stays.
    /// Returns the extensions stopped.
    pub fn apply_revocations(&mut self, list: Revocations) -> Result<Vec<String>, Problem> {
        let text = serde_json::to_string_pretty(&list).unwrap_or_default();
        store::write_atomic(&self.layout.revocations(), text.as_bytes())?;
        self.revocations = list;
        let hit: Vec<String> = self
            .file
            .extensions
            .iter()
            .filter(|(id, entry)| self.revocations.reason(id, &entry.active).is_some())
            .map(|(id, _)| id.clone())
            .collect();
        let mut stopped = Vec::new();
        for id in &hit {
            if self.leases.contains_key(id) {
                stopped.extend(self.stop(id));
                stopped.push(id.clone());
            }
            let reason = self
                .file
                .extensions
                .get(id)
                .and_then(|e| self.revocations.reason(id, &e.active))
                .unwrap_or_default()
                .to_owned();
            self.log(
                id,
                Record::warning(
                    "revoked",
                    format!("Bu sürüm geri çekildi: {reason}. Durduruldu; verisi duruyor."),
                ),
            );
        }
        Ok(stopped)
    }

    /// Sets an extension's update channel.
    pub fn set_channel(&mut self, id: &str, channel: Channel) -> Result<(), Problem> {
        let owner = id.to_owned();
        let found = self
            .change(move |file| Ok(file.extensions.get_mut(&owner).map(|e| e.channel = channel)))?;
        found.ok_or_else(|| not_installed(id))
    }

    /// Keeps an extension on a version (none: updates offered again).
    pub fn pin(&mut self, id: &str, version: Option<String>) -> Result<(), Problem> {
        let owner = id.to_owned();
        let found = self
            .change(move |file| Ok(file.extensions.get_mut(&owner).map(|e| e.pinned = version)))?;
        found.ok_or_else(|| not_installed(id))
    }

    // ---- Data and projects ----

    /// The extension's own folder for its settings and data, made when first asked: per user and per extension.
    pub fn data_dir(&self, id: &str) -> Result<PathBuf, Problem> {
        if !self.file.extensions.contains_key(id) {
            return Err(not_installed(id));
        }
        let dir = self.layout.data(id);
        fs::create_dir_all(&dir)
            .map_err(|e| store::io("Eklenti veri klasörü oluşturulamadı", &dir, &e))?;
        Ok(dir)
    }

    /// Empties the extension's data folder (Sıfırla).
    pub fn reset_data(&mut self, id: &str) -> Result<(), Problem> {
        let dir = self.data_dir(id)?;
        store::remove_tree(&dir)?;
        fs::create_dir_all(&dir)
            .map_err(|e| store::io("Eklenti veri klasörü oluşturulamadı", &dir, &e))?;
        self.log(
            id,
            Record::info("data_reset", "Kullanıcı verisini sıfırladı."),
        );
        Ok(())
    }

    /// The extensions a project names (id, version range), each with its status here. Opening the project runs no
    /// code and installs nothing; what is not ready stays restricted until the user decides (D16).
    pub fn project_needs(&self, needs: &[(String, String)]) -> Vec<ProjectNeed> {
        needs
            .iter()
            .map(|(id, versions)| {
                let status = match self.file.extensions.get(id) {
                    None => NeedStatus::Missing,
                    Some(entry) => {
                        let fits = VersionReq::parse(versions)
                            .zip(Version::parse(&entry.active))
                            .is_some_and(|(r, v)| r.matches(&v));
                        if !fits {
                            NeedStatus::NotRunning {
                                reason: format!(
                                    "{} kurulu; proje {versions} istiyor.",
                                    entry.active
                                ),
                            }
                        } else if self.leases.contains_key(id) {
                            NeedStatus::Ready
                        } else {
                            NeedStatus::NotRunning {
                                reason: self.state_of(id, entry).help(),
                            }
                        }
                    }
                };
                ProjectNeed {
                    id: id.clone(),
                    versions: versions.clone(),
                    status,
                }
            })
            .collect()
    }
}
