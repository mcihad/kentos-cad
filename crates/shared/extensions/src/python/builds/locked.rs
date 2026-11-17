//! The wheels a lock names for one computer (docs/adr/8057 §123, PY-UV-27): what an organisation's wheel folder must
//! hold for an extension's environment to be prepared there without the internet, each file with the SHA-256 the lock
//! gives it. A package the lock has no fitting wheel of (a source distribution only, a repository) is listed without
//! files: offline and from a folder nothing is built.

use std::collections::BTreeMap;

use super::{
    Env, Target, Toml, fits, installed_with, is_local, name_of, normalized, parse_toml, project_of,
    supports, version_of, wheel_file,
};
use crate::manifest::PythonProject;

/// A package the lock installs on a computer, with the lock's wheels of it that fit there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LockedWheels {
    pub name: String,
    pub version: String,
    /// Each fitting wheel's file name and SHA-256 (`sha256:…`); none when the lock has no wheel of it for the computer.
    pub files: Vec<(String, String)>,
}

/// What the lock of `project` (its files in `files`) installs on `target` with the optional features `extras`, and the
/// wheels of each that fit there, by name; the project and what it installs from itself left out. None when the files
/// do not read or the lock does not support `target`.
pub fn locked_wheels(
    project: &PythonProject,
    files: &BTreeMap<String, Vec<u8>>,
    target: &Target,
    extras: &[String],
) -> Option<Vec<LockedWheels>> {
    let (name, _, lock) = project_of(project, files)?;
    let lock = parse_toml(lock).ok()?;
    if !supports(&lock, target) {
        return None;
    }
    let entries: Vec<&Toml> = lock
        .get(&["package"])
        .and_then(Toml::array)
        .unwrap_or(&[])
        .iter()
        .collect();
    let own = normalized(&name);
    let root = entries.iter().copied().find(|e| name_of(e) == own)?;
    let env = Env::of(target);
    let mut out: Vec<LockedWheels> = installed_with(&entries, root, &env, Some(extras))
        .into_iter()
        .filter(|entry| !is_local(entry))
        .map(|entry| LockedWheels {
            name: name_of(entry),
            version: version_of(entry),
            files: entry
                .get(&["wheels"])
                .and_then(Toml::array)
                .unwrap_or(&[])
                .iter()
                .filter_map(|wheel| {
                    let file = wheel_file(wheel)?;
                    let hash = wheel.get(&["hash"]).and_then(Toml::str)?;
                    fits(file, target).then(|| (file.to_owned(), hash.to_owned()))
                })
                .collect(),
        })
        .collect();
    out.sort_by(|a, b| (&a.name, &a.version).cmp(&(&b.name, &b.version)));
    out.dedup_by(|a, b| a.name == b.name && a.version == b.version);
    Some(out)
}
