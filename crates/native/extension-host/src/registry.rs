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

use crate::lease::{self, ExtensionLease, Registered, Skipped};
use crate::policy::{Policy, Revocations};
use crate::store::{
    self, Channel, Entry, Failure, Grant, InstalledVersion, Layout, Metrics, RegistryFile, Source,
    SourceKind, problem,
};
use crate::view::{
    DependencyStatus, ExtensionView, NeedStatus, ProjectNeed, Review, State, Trust,
    capability_lines,
};
use crate::watchdog::CRASHES_TO_FAIL;

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
}

fn not_installed(id: &str) -> Problem {
    problem("registry_unknown", format!("“{id}” kurulu değil."))
}

/// Same id from another kind of source, or another working folder: replacing it is the user's choice.
fn conflicts(installed: &Source, new: &Source) -> bool {
    installed.kind != new.kind
        || (installed.kind == SourceKind::DeveloperFolder && installed.location != new.location)
}

/// The installed files against the content digest.
fn verify_files(dir: &Path, digest: &str) -> Result<(), String> {
    let files = store::read_files(dir).map_err(|p| p.message)?;
    let list = Integrity::of(
        files
            .iter()
            .filter(|(p, _)| p.as_str() != integrity::FILE)
            .map(|(p, d)| (p.as_str(), d.as_slice())),
    );
    if list.digest() == digest {
        Ok(())
    } else {
        Err("Kurulu dosyalar kurulduğu hâlinden farklı (içerik özeti tutmuyor); eklentiyi yeniden kurun.".to_owned())
    }
}

fn read_manifest(layout: &Layout, id: &str, version: &str) -> Result<Manifest, Problem> {
    let path = layout.package(id, version).join("manifest.json");
    let text = fs::read_to_string(&path).map_err(|e| store::io("Manifest okunamadı", &path, &e))?;
    Manifest::read(&text)
}

impl Registry {
    /// Opens the registry under `root` (the user's extension folder). A version left half-prepared by a crash is
    /// removed; nothing is activated until [`Registry::startup`].
    pub fn open(
        root: impl Into<PathBuf>,
        host: Host,
        options: Options,
    ) -> Result<Registry, Problem> {
        let layout = Layout { root: root.into() };
        let file = layout.read()?;
        let revocations = fs::read_to_string(layout.revocations())
            .ok()
            .and_then(|text| Revocations::read(&text).ok())
            .unwrap_or_default();
        let _ = store::remove_tree(&layout.staging());
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
        let (file, out) = layout.locked(self.options.lock_wait, || {
            let mut file = layout.read()?;
            let out = change(&mut file)?;
            layout.write(&file)?;
            Ok((file, out))
        })?;
        self.file = file;
        Ok(out)
    }

    /// Notes a line in an extension's log.
    fn note(&mut self, id: &str, message: String) -> Result<(), Problem> {
        let id = id.to_owned();
        self.change(move |file| {
            if let Some(entry) = file.extensions.get_mut(&id) {
                entry.note(message);
            }
            Ok(())
        })
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
                    log: entry.log.clone(),
                }
            })
            .collect()
    }

    /// What the active extensions add, each under its lease.
    pub fn contributions(&self) -> &[Registered] {
        &self.registered
    }

    /// The version in use's manifest.
    pub fn manifest(&self, id: &str) -> Option<&Manifest> {
        self.manifests.get(id)
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
            if self.state(&id) == Some(State::Ready) {
                let result = self.activate(&id);
                out.push((id, result));
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
            let failed = reason.clone();
            let owner = id.to_owned();
            self.change(move |file| {
                if let Some(entry) = file.extensions.get_mut(&owner) {
                    entry.note(format!("Bütünlük denetimi düştü: {failed}"));
                    entry.failure = Some(Failure::Integrity { reason: failed });
                }
                Ok(())
            })?;
            return Err(problem("registry_integrity", reason));
        }
        let manifest = read_manifest(&self.layout, id, &entry.active)?;
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
        let version = entry.active.clone();
        self.change(move |file| {
            if let Some(entry) = file.extensions.get_mut(&owner) {
                let before = entry.metrics.unwrap_or_default();
                entry.metrics = Some(Metrics {
                    activation_micros: micros,
                    ..before
                });
                entry.note(format!("{version} etkinleştirildi ({} ms).", micros / 1000));
            }
            Ok(())
        })?;
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
        self.change(move |file| {
            if let Some(entry) = file.extensions.get_mut(&owner) {
                if !entry.enabled {
                    entry.note("Etkinleştirildi (kullanıcı).");
                }
                entry.enabled = true;
            }
            Ok(())
        })?;
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
        let stopped = dependents.clone();
        self.change(move |file| {
            for other in &stopped {
                if let Some(entry) = file.extensions.get_mut(other) {
                    entry.note(format!("Durduruldu: bağımlı olduğu “{owner}” devre dışı."));
                }
            }
            if let Some(entry) = file.extensions.get_mut(&owner) {
                entry.enabled = false;
                entry.note("Devre dışı bırakıldı (kullanıcı).");
            }
            Ok(())
        })?;
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
                entry.note(match &before {
                    _ if fresh => format!("{new_version} kuruldu ({}).", source_now.kind.words()),
                    Some(old) if *old != new_version => {
                        format!(
                            "{old} → {new_version} güncellendi ({}).",
                            source_now.kind.words()
                        )
                    }
                    _ => format!(
                        "{new_version} yeniden kuruldu ({}).",
                        source_now.kind.words()
                    ),
                });
                Ok(removed)
            })
            .map_err(one)?;
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
                entry.note("İzinler değişti (kullanıcı).");
            }
            Ok(())
        })?;
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
        verify_files(&self.layout.package(id, version), &target.digest)
            .map_err(|m| problem("registry_integrity", m))?;
        let owner = id.to_owned();
        let to = version.to_owned();
        let from = entry.active.clone();
        self.change(move |file| {
            if let Some(entry) = file.extensions.get_mut(&owner) {
                entry.active = to.clone();
                entry.failure = None;
                entry.crashes = 0;
                entry.note(format!("{from} → {to} sürümüne geçildi."));
            }
            Ok(())
        })?;
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
        self.change(move |file| {
            file.extensions.remove(&owner);
            Ok(())
        })?;
        self.manifests.remove(id);
        store::remove_tree(&self.layout.packages_of(id))?;
        if !keep_data {
            store::remove_tree(&self.layout.data(id))?;
        }
        for other in &dependents {
            let _ = self.note(
                other,
                format!("Durduruldu: bağımlı olduğu “{id}” kaldırıldı."),
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
        let (count, failed) = self.change(move |file| {
            let Some(entry) = file.extensions.get_mut(&owner) else {
                return Ok((0, false));
            };
            entry.crashes += 1;
            let count = entry.crashes;
            entry.note(format!("Çöktü ({count}. kez art arda): {reason}"));
            let failed = count >= CRASHES_TO_FAIL;
            if failed {
                entry.failure = Some(Failure::Crashed {
                    reason: format!("Art arda {count} kez çöktü; son neden: {reason}."),
                });
            }
            Ok((count, failed))
        })?;
        if failed {
            self.stop(&lease.extension);
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
                entry.note("Yeniden deneniyor (kullanıcı).");
            }))
        })?;
        if found.is_none() {
            return Err(not_installed(id));
        }
        match self.state(id) {
            Some(State::Ready) if !self.options.safe_mode => Ok(Some(self.activate(id)?)),
            _ => Ok(None),
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
            self.note(id, format!("Geri çekildi: {reason}. Verisi duruyor."))?;
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
        self.note(id, "Verisi sıfırlandı (kullanıcı).".to_owned())
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
