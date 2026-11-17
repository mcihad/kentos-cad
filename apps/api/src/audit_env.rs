//! The Python environments the server prepares for its audit rules (docs/adr/8057 §66), and the interpreter map the
//! worker's audit source reads. Made by the administrator's command (`kentosd audit-env prepare`), later by the
//! catalog when a release is published ([`prepare_release`] is that hook), with the desktop's machinery
//! (`kentos_python_env::extension`), wheels only.
//!
//! - **The release:** its package's bytes opened and checked as the audit job checks them (the inspection, the
//!   content digest asked, the id and the version), refused when it asks for the network, has no Python project or no
//!   audit; its Python project's two files taken out.
//! - **The environment:** keyed by the extension, its version, its content digest, its lock, KentOS's Python and this
//!   system, under the server's folder ([`EnvSettings::dir`]); made from the lock's wheels alone, each against its
//!   hash, nothing built; offline from the wheel folder the administrator fills, or from an index where the
//!   administrator allows it; KentOS's library from the installation's own wheel; marked ready only when complete. A
//!   ready one is
//!   used as it is, nothing run.
//! - **The map** ([`AuditMap`]): [`MAP`] in that folder, `{"extension@version": "/…/bin/python"}`, written whole and
//!   renamed into place; the setting's under it, the file winning; read again at every job. A version's environment
//!   removed takes its entry with it.

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

use kentos_application::audit_catalog::AuditPythons;
use kentos_extensions::manifest::Capability;
use kentos_python_env::extension::{self, Listed, Wheels};
use kentos_python_env::sdk;
use kentos_python_env::uv;

// By their paths: the server job tests take this module in by its file too (`tests/job_run.rs`).
#[path = "audit_env/map.rs"]
mod map;
#[path = "audit_env/python.rs"]
mod python;

pub use map::AuditMap;
use python::{asked, installed, words};

/// The interpreter map's file, in the environments' folder.
pub const MAP: &str = "yorumlayicilar.json";

/// The wheel folder's name in the environments' folder, when the administrator names no other.
pub const WHEELS: &str = "tekerlekler";

/// Where a package's project files are taken out to while its environment is made.
const OPENED: &str = "acilan";

/// The folder's lock: one preparation or removal at a time on this server, whoever asks.
const LOCK: &str = "hazirlama.kilit";

/// The server's settings for its audit environments (`config.rs` reads them).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EnvSettings {
    /// The environments, uv's cache, the wheel folder and the map (`KENTOS_AUDIT_ENV_DIR`).
    pub dir: PathBuf,
    /// KentOS's Python the environments are made from (`KENTOS_AUDIT_BASE_PYTHON`); none: the one uv installed in
    /// [`Self::dir`].
    pub base_python: Option<PathBuf>,
    /// The folder of wheels the administrator fills (`KENTOS_AUDIT_WHEELS`); none: [`WHEELS`] in [`Self::dir`].
    pub wheels: Option<PathBuf>,
    /// An index, where the administrator allows the network while preparing (`KENTOS_AUDIT_INDEX_URL`).
    pub index_url: Option<String>,
    /// uv (`KENTOS_AUDIT_UV`); none: the installation's own, else `PATH`.
    pub uv: Option<PathBuf>,
    /// Where KentOS's library's wheel is (`KENTOS_AUDIT_SDK_DIR`, else the installation's beside the program).
    pub sdk_dirs: Vec<PathBuf>,
    /// The setting's mapping (`KENTOS_AUDIT_PYTHONS`), under the file's.
    pub pythons: AuditPythons,
}

impl EnvSettings {
    /// The wheel folder.
    pub fn wheels_dir(&self) -> PathBuf {
        self.wheels.clone().unwrap_or_else(|| self.dir.join(WHEELS))
    }

    /// The interpreter map: the file in [`Self::dir`] over the setting's.
    pub fn map(&self) -> AuditMap {
        AuditMap {
            file: self.dir.join(MAP),
            setting: self.pythons.clone(),
        }
    }
}

/// A release's environment, prepared.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Prepared {
    /// `extension@version`.
    pub release: String,
    /// The package's content digest (`sha256:…`).
    pub digest: String,
    /// The environment's Python, as the map has it.
    pub python: PathBuf,
    /// What it holds besides KentOS's library.
    pub packages: Vec<Listed>,
    /// Made now; false when it was ready and nothing ran.
    pub made: bool,
    pub took: Duration,
}

/// `extension@version`, split; refused when either side is empty.
pub fn release_of(text: &str) -> Result<(String, String), String> {
    match text.split_once('@') {
        Some((e, v)) if !e.is_empty() && !v.is_empty() => Ok((e.to_owned(), v.to_owned())),
        _ => Err(format!(
            "“{text}”: sürüm eklenti@sürüm olmalı (tr.ornek.denetim@1.0.0)."
        )),
    }
}

/// Prepares the environment of `extension` at `version` from its package's bytes (`digest`: the content the catalog
/// approved, when it is known), and maps its Python. The catalog's publishing calls it; so does the administrator's
/// command. Preparing a ready one again runs nothing and maps it again.
pub fn prepare_release(
    settings: &EnvSettings,
    extension: &str,
    version: &str,
    digest: Option<&str>,
    bytes: &[u8],
) -> Result<Prepared, String> {
    let began = Instant::now();
    let opened = kentos_extensions::open(bytes, &kentos_extensions::Limits::default());
    if let Some(p) = opened.inspection.problems.first() {
        return Err(format!("Eklentinin paketi açılamadı: {}", p.message));
    }
    let Some(computed) = opened.inspection.digest.clone() else {
        return Err("Paketin içerik özeti okunamadı.".to_owned());
    };
    if let Some(asked) = digest
        && asked != computed
    {
        return Err(format!(
            "Paketin içerik özeti istenen değil ({asked}): başka bir sürüm ya da bozulmuş bir paket."
        ));
    }
    let Some(manifest) = opened.inspection.manifest.as_ref() else {
        return Err("Paketin manifesti okunamadı.".to_owned());
    };
    let packed = manifest.version.to_string();
    if manifest.id != extension || packed != version {
        return Err(format!(
            "Paket {} {}; istenen {extension} {version}.",
            manifest.id, manifest.version
        ));
    }
    if manifest
        .requested_capabilities
        .iter()
        .any(|c| c.id == Capability::Network)
    {
        return Err(
            "Bulut denetim kutusu ağa çıkmaz: “network” yeteneğini isteyen eklentinin ortamı hazırlanmaz.".to_owned(),
        );
    }
    if manifest.entrypoints.audits.is_empty() {
        return Err(
            "Eklentinin bulut işinde çalışacak bir denetimi yok; ortamı hazırlanmaz.".to_owned(),
        );
    }
    let Some(project) = manifest.python.as_ref() else {
        return Err("Eklentinin Python projesi yok; hazırlanacak ortamı yok.".to_owned());
    };
    let _held = hold(&settings.dir)?;
    // The project's two files, taken out where the environment's machinery reads them.
    let short: String = computed
        .trim_start_matches("sha256:")
        .chars()
        .take(16)
        .collect();
    let package = settings
        .dir
        .join(OPENED)
        .join(format!("{short}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&package);
    let taken = take_out(
        &package,
        &opened.files,
        [&project.project_file, &project.lock_file],
    );
    let made = taken.and_then(|()| {
        prepare_in(
            settings,
            extension,
            version,
            &computed,
            &package,
            (&project.project_file, &project.lock_file),
        )
    });
    let _ = std::fs::remove_dir_all(&package);
    let (ready, made) = made?;
    settings.map().set(extension, version, &ready.python)?;
    Ok(Prepared {
        release: format!("{extension}@{version}"),
        digest: computed,
        python: ready.python,
        packages: ready.packages,
        made,
        took: began.elapsed(),
    })
}

/// The folder's lock, held until the returned file is dropped: the administrator's command and the catalog's
/// publishing never make or remove an environment, or write the map, at the same time.
fn hold(dir: &Path) -> Result<std::fs::File, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{} açılamadı: {e}", dir.display()))?;
    let path = dir.join(LOCK);
    let file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .open(&path)
        .map_err(|e| format!("{} açılamadı: {e}", path.display()))?;
    file.lock()
        .map_err(|e| format!("{} kilitlenemedi: {e}", path.display()))?;
    Ok(file)
}

/// The files `paths` of a package, written under `dir` at their paths.
fn take_out<'a>(
    dir: &Path,
    files: &std::collections::BTreeMap<String, Vec<u8>>,
    paths: impl IntoIterator<Item = &'a String>,
) -> Result<(), String> {
    for path in paths {
        if kentos_extensions::path_problem(path).is_some() {
            return Err(format!("Paketteki “{path}” yolu kullanılamaz."));
        }
        let data = files
            .get(path)
            .ok_or_else(|| format!("Paketin Python projesinde “{path}” yok."))?;
        let to = dir.join(path);
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("{} açılamadı: {e}", parent.display()))?;
        }
        std::fs::write(&to, data).map_err(|e| format!("{} yazılamadı: {e}", to.display()))?;
    }
    Ok(())
}

/// The environment of a package whose project files are in `package`: ready, and whether it was made now.
fn prepare_in(
    settings: &EnvSettings,
    extension: &str,
    version: &str,
    digest: &str,
    package: &Path,
    files: (&str, &str),
) -> Result<(extension::Ready, bool), String> {
    std::fs::create_dir_all(&settings.dir)
        .map_err(|e| format!("{} açılamadı: {e}", settings.dir.display()))?;
    let find_uv = || {
        let found = uv::detect(settings.uv.as_deref());
        found
            .path()
            .filter(|_| found.ok())
            .map(Path::to_path_buf)
            .ok_or_else(|| found.words())
    };
    // KentOS's Python first: with the lock, it says the key, and a ready environment needs nothing else.
    let (interpreter, mut uv_path) = match &settings.base_python {
        Some(path) => (asked(path)?, None),
        None => {
            let uv_path = find_uv()?;
            (installed(settings, &uv_path)?, Some(uv_path))
        }
    };
    let mut key = extension::key_for(package, files.1, extension, version, &interpreter, &[])?;
    key.digest = digest.to_owned();
    if let Some(ready) = extension::ready(&settings.dir, &key) {
        return Ok((ready, false));
    }
    let uv_path = match uv_path.take() {
        Some(found) => found,
        None => find_uv()?,
    };
    let wheel = sdk::find(&settings.sdk_dirs, env!("CARGO_PKG_VERSION"))?;
    let folder = settings.wheels_dir();
    let wheels = Wheels {
        folder: folder.is_dir().then(|| folder.clone()),
        index: settings.index_url.clone(),
        computer: false,
    };
    if wheels.folder.is_none() && wheels.index.is_none() {
        return Err(format!(
            "Tekerlek klasörü yok: {}. Kilitteki paketlerin tekerleklerini oraya koyun (ya da KENTOS_AUDIT_WHEELS ile \
             başka bir klasör, ağa izin verilecekse KENTOS_AUDIT_INDEX_URL ile bir paket dizini gösterin).",
            folder.display()
        ));
    }
    extension::prepare_wheels(
        &uv_path,
        &settings.dir,
        &interpreter,
        &wheel,
        package,
        files,
        &key,
        &wheels,
        &AtomicBool::new(false),
        &mut |_| {},
    )
    .map(|ready| (ready, true))
    .map_err(|e| words(&e))
}

/// Removes the environments of `extension` at `version` and its entry in the map; says how many folders went.
pub fn remove_release(
    settings: &EnvSettings,
    extension: &str,
    version: &str,
) -> Result<usize, String> {
    let _held = hold(&settings.dir)?;
    let gone = extension::remove_version(&settings.dir, extension, version);
    settings.map().unset(extension, version)?;
    Ok(gone.len())
}

/// The map's mappings and whether each interpreter is there.
pub fn listing(settings: &EnvSettings) -> Result<Vec<(String, String, bool)>, String> {
    Ok(settings
        .map()
        .current()?
        .entries()
        .map(|(release, python)| {
            (
                release.to_owned(),
                python.to_owned(),
                Path::new(python).is_file(),
            )
        })
        .collect())
}

// By its path: the server job tests take this module in by its file too (`tests/job_run.rs`).
#[cfg(test)]
#[path = "audit_env/tests.rs"]
mod tests;
