//! A server's way to an extension version's environment (docs/adr/8057 §66): the desktop's key, place, library, check
//! and record ([`super::prepare`]), but the packages the lock names as wheels only, nothing built, each checked
//! against the lock's hash.
//!
//! - **The lock's pinned and hashed list:** `uv export --locked` of the package's project (no development
//!   dependencies, not the project itself), offline: what the author locked, with each file's SHA-256.
//! - **Wheels only, hashes required:** `uv pip install --require-hashes --no-build --no-deps` of that list into a venv
//!   made from KentOS's Python: no source distribution is built (its build backend would run code on the server), a
//!   file whose hash is not the lock's is refused, and a requirement without a hash is refused.
//! - **Where from:** a folder of wheels the administrator fills (`--find-links`), offline (`--offline --no-index`);
//!   an index (`--index-url`) only where the administrator allows the network, and only in this step.
//! - **Said in Turkish, the package named:** a dependency with no wheel for this system, one missing from the folder,
//!   one whose file is not the lock's, one locked without a hash ([`wheel_words`]).

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use super::{
    CHECK_LIMIT, InstallError, Interpreter, Key, Progress, Ready, Wheel, check_ran, env_for,
    finish, other, place, ready, refs, start,
};
use crate::{personal, python, uv};

/// The lock's list, written beside the project's copy.
const REQUIREMENTS: &str = "gereksinimler.txt";

/// Where a server takes an environment's wheels from.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Wheels {
    /// A folder of wheels the administrator fills (`--find-links`).
    pub folder: Option<PathBuf>,
    /// An index (`--index-url`): the network, where the administrator allows it; none: offline.
    pub index: Option<String>,
    /// A desktop's, from the organisation's wheel folder (docs/adr/8057 §123): its words name this computer, not a
    /// server.
    pub computer: bool,
}

/// Prepares `key`'s environment from the package's lock as a server does: the lock's packages as wheels from `wheels`,
/// each checked against its hash, nothing built; then KentOS's library, the check and the record, as the desktop's
/// [`super::prepare`]. A ready one is used as it is, nothing run; whatever ends it before the record, the half-made
/// folder goes.
#[allow(clippy::too_many_arguments)]
pub fn prepare_wheels(
    uv_path: &Path,
    root: &Path,
    interpreter: &Interpreter,
    sdk: &Wheel,
    package: &Path,
    files: (&str, &str),
    key: &Key,
    wheels: &Wheels,
    cancel: &AtomicBool,
    told: &mut dyn FnMut(Progress),
) -> Result<Ready, InstallError> {
    if let Some(made) = ready(root, key) {
        return Ok(made);
    }
    let dir = place(root, key);
    let made = make(
        uv_path,
        root,
        interpreter,
        sdk,
        package,
        files,
        key,
        wheels,
        &dir,
        cancel,
        told,
    );
    if made.is_err() {
        let _ = std::fs::remove_dir_all(&dir);
    }
    made
}

#[allow(clippy::too_many_arguments)]
fn make(
    uv_path: &Path,
    root: &Path,
    interpreter: &Interpreter,
    sdk: &Wheel,
    package: &Path,
    files: (&str, &str),
    key: &Key,
    wheels: &Wheels,
    dir: &Path,
    cancel: &AtomicBool,
    told: &mut dyn FnMut(Progress),
) -> Result<Ready, InstallError> {
    if wheels.folder.is_none() && wheels.index.is_none() {
        return Err(other(
            "Tekerleklerin nereden alınacağı söylenmedi: ne tekerlek klasörü ne de paket dizini var.".to_owned(),
        ));
    }
    let project = start(package, files, dir)?;
    let venv = dir.join(".venv");
    let env = env_for(root, &venv);
    told(Progress::Starting);
    if cancel.load(std::sync::atomic::Ordering::Relaxed) {
        return Err(InstallError::Cancelled);
    }
    // The lock's pinned and hashed list, offline: the author's lock, not a new resolution.
    let list = project.join(REQUIREMENTS);
    let mut export: Vec<OsString> = [
        "export",
        "--locked",
        "--no-dev",
        "--no-emit-project",
        "--format",
        "requirements-txt",
        "--no-config",
        "--offline",
    ]
    .iter()
    .map(OsString::from)
    .collect();
    for extra in &key.extras {
        export.push(OsString::from("--extra"));
        export.push(OsString::from(extra));
    }
    // Its cooling period, which `--no-config` would leave out (docs/adr/8057 §83).
    let lock = std::fs::read_to_string(project.join("uv.lock")).unwrap_or_default();
    export.extend(super::cooling_args(&lock));
    export.push(OsString::from("--output-file"));
    export.push(list.clone().into_os_string());
    check_ran(uv::run(
        uv_path,
        &uv::os_args(&export),
        Some(&project),
        &refs(&env),
        CHECK_LIMIT,
    ))
    .map_err(|why| other(wheel_words(&why, wheels)))?;
    // A venv of KentOS's Python, nothing downloaded.
    let mut made: Vec<OsString> = [
        "venv",
        "--no-config",
        "--no-python-downloads",
        "--offline",
        "--python",
    ]
    .iter()
    .map(OsString::from)
    .collect();
    made.push(interpreter.path.clone().into_os_string());
    made.push(venv.clone().into_os_string());
    check_ran(uv::run(
        uv_path,
        &uv::os_args(&made),
        Some(dir),
        &refs(&env),
        CHECK_LIMIT,
    ))
    .map_err(|why| other(format!("Ortamın sanal ortamı açılamadı: {why}")))?;
    // The list's wheels, each against its hash, nothing built.
    let python = personal::python_in(&venv, std::env::consts::OS);
    let ran = uv::run(
        uv_path,
        &uv::os_args(&install_args(&python, &list, wheels)),
        Some(dir),
        &refs(&env),
        personal::SYNC_LIMIT,
    )
    .map_err(InstallError::NotRun)?;
    if ran.timed_out {
        return Err(InstallError::TimedOut);
    }
    if ran.status != Some(0) {
        let said = python::said(&ran.stderr).unwrap_or_else(|| ran.stderr.trim().to_owned());
        return Err(match python::cause_of(&ran.stderr) {
            python::Cause::Other => other(wheel_words(&ran.stderr, wheels)),
            cause => InstallError::Failed { cause, said },
        });
    }
    told(Progress::Unpacking);
    finish(uv_path, sdk, key, dir, &venv, &env)
}

/// `uv pip install`'s words for the lock's list: wheels only, hashes required, no dependency beyond the list (the
/// lock's list is whole); offline from the folder, or from the index where there is one.
pub(super) fn install_args(python: &Path, list: &Path, wheels: &Wheels) -> Vec<OsString> {
    let mut args: Vec<OsString> = [
        "pip",
        "install",
        "--no-config",
        "--require-hashes",
        "--no-build",
        "--no-deps",
        "--system-certs",
    ]
    .iter()
    .map(OsString::from)
    .collect();
    match &wheels.index {
        Some(index) => {
            args.push(OsString::from("--index-url"));
            args.push(OsString::from(index));
        }
        None => {
            args.push(OsString::from("--offline"));
            args.push(OsString::from("--no-index"));
        }
    }
    if let Some(folder) = &wheels.folder {
        args.push(OsString::from("--find-links"));
        args.push(folder.clone().into_os_string());
    }
    args.push(OsString::from("--python"));
    args.push(python.as_os_str().to_owned());
    args.push(OsString::from("-r"));
    args.push(list.as_os_str().to_owned());
    args
}

/// Why uv could not put the lock's wheels in, in Turkish, the package named: no wheel for this system (the server
/// builds nothing), not in the folder, not the lock's file, locked without a hash; else uv's own words.
pub fn wheel_words(said: &str, wheels: &Wheels) -> String {
    let flat = said.split_whitespace().collect::<Vec<_>>().join(" ");
    let here = super::platform();
    let quoted = |after: &str| {
        flat.split_once(after)
            .and_then(|(_, rest)| rest.strip_prefix('`'))
            .and_then(|rest| rest.split_once('`'))
            .map(|(name, _)| name.to_owned())
    };
    let before = |marker: &str| {
        flat.split_once(marker)
            .and_then(|(head, _)| head.rsplit_once("Because "))
            .and_then(|(_, after)| named(after))
    };
    let name_of = |pinned: String| match pinned.split_once("==") {
        Some((name, version)) => format!("“{name}” {version}"),
        None => format!("“{pinned}”"),
    };
    if let Some(pinned) = before(" has no usable wheels")
        .or_else(|| before(" has no wheels with a matching"))
        .or_else(|| quoted("Wheels are required for "))
        .or_else(|| quoted("only found wheels for "))
    {
        let (machine, builds) = if wheels.computer {
            (
                "bu bilgisayarın",
                "tekerlek klasöründen kaynak paketi derlenmez",
            )
        } else {
            ("bu sunucunun", "sunucu kaynak paketinden derlemez")
        };
        return format!(
            "{}: {machine} sistemi ({here}) için tekerleği yok ve {builds}. Yayımcı bu sistemin tekerleği olan bir \
             sürümü kilitlemeli ya da yönetici uygun tekerleği tekerlek klasörüne koymalı.",
            name_of(pinned)
        );
    }
    if let Some(name) = before(" was not found in the provided package locations")
        .or_else(|| before(" was not found in the package registry"))
    {
        let place = match (&wheels.folder, &wheels.index) {
            (Some(folder), None) => format!("tekerlek klasöründe ({})", folder.display()),
            (_, Some(index)) => format!("paket dizininde ({index})"),
            (None, None) => "tekerleklerin yerinde".to_owned(),
        };
        return format!(
            "{} {place} yok: kilitteki sürümün tekerleğini oraya koyun.",
            name_of(name)
        );
    }
    if let Some(pinned) = quoted("Hash mismatch for ") {
        return format!(
            "{}: bulunan dosya kilitteki özetle (SHA-256) tutmuyor, başka bir dosya; kilitteki dosyayı koyun.",
            name_of(pinned)
        );
    }
    if let Some((_, names)) = flat.split_once("none were provided for: ") {
        return format!(
            "Eklentinin kilidi {} için özet (SHA-256) taşımıyor; sunucu özeti olmayan paketi kurmaz. Yayımcı paketi \
             bir paket dizininden kilitlemeli.",
            names
                .split(", ")
                .filter_map(|n| named(n.trim_end_matches('.')))
                .map(name_of)
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    let words = python::said(said).unwrap_or_else(|| said.trim().to_owned());
    super::lock_words(&words)
}

/// The requirement uv names at the end of `text`: its environment markers left out (uv writes a universal lock's as
/// `tasli{(sys_platform == 'linux') or …}==1.0`), after the last ` and ` (`only kucuk==1.0 is available and
/// kucuk==1.0`): `tasli==1.0`.
fn named(text: &str) -> Option<String> {
    let mut bare = String::new();
    let mut depth = 0usize;
    for c in text.chars() {
        match c {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            _ if depth == 0 => bare.push(c),
            _ => {}
        }
    }
    let last = bare.rsplit(" and ").next()?.trim();
    last.split_whitespace()
        .last()
        .map(|n| n.trim_matches('`').to_owned())
        .filter(|n| !n.is_empty())
}

#[cfg(test)]
mod tests;
