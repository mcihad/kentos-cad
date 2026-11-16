//! The Python packages a package installs, as the install review shows them (docs/adr/8057 §81, PY-CLOUD-14): its
//! lock's packages, the extension itself left out; on an update, against the installed version's: a new one, another
//! version or source, the same version with other files, one no longer installed. The packages run with the user's
//! rights in the extension's environment, so they are shown before Kur as the capabilities are; a name a letter from a
//! well-known package's is marked where it is listed.

use std::collections::BTreeMap;
use std::path::Path;

use kentos_extensions::manifest::Manifest;
use kentos_extensions::release::{LockedPackage, locked_packages};
use serde::Serialize;

use crate::view::Change;

/// A Python package the lock installs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageLine {
    /// Its normalised name.
    pub name: String,
    /// The version installed with it; the installed version's when it is no longer installed.
    pub version: String,
    /// The installed version's version or source, when it changed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<String>,
    /// Where it comes from when not the Python Package Index (`git https://…`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// `Changed` with the same version: its files are not the ones installed.
    pub change: Change,
    /// The well-known package its name is a letter from.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resembles: Option<&'static str>,
}

/// The installed version's packages: its project and lock from its package folder.
pub(crate) fn installed(
    manifest: &Manifest,
    package: &Path,
) -> Option<BTreeMap<String, LockedPackage>> {
    let project = manifest.python.as_ref()?;
    let mut files = BTreeMap::new();
    for path in [&project.project_file, &project.lock_file] {
        files.insert(path.clone(), std::fs::read(package.join(path)).ok()?);
    }
    locked_packages(project, &files)
}

/// `new`'s packages against `old`'s (none on a first install): the changed first, then the kept, then the dropped,
/// each by name.
pub fn lines(
    new: &BTreeMap<String, LockedPackage>,
    old: Option<&BTreeMap<String, LockedPackage>>,
) -> Vec<PackageLine> {
    let source = |p: &LockedPackage| (!p.from_the_index()).then(|| p.source.clone());
    let mut out: Vec<PackageLine> = new
        .iter()
        .map(|(name, p)| {
            let (change, before) = match old.map(|o| o.get(name)) {
                None => (Change::Asked, None),
                Some(None) => (Change::Added, None),
                Some(Some(b)) if b.version != p.version => {
                    (Change::Changed, Some(b.version.clone()))
                }
                Some(Some(b)) if b.source != p.source => (Change::Changed, Some(b.source.clone())),
                Some(Some(b)) if b.files != p.files => (Change::Changed, Some(b.version.clone())),
                Some(Some(_)) => (Change::Same, None),
            };
            PackageLine {
                name: name.clone(),
                version: p.version.clone(),
                before,
                source: source(p),
                change,
                resembles: kentos_extensions::python_names::resembles_known(name),
            }
        })
        .collect();
    if let Some(old) = old {
        out.extend(
            old.iter()
                .filter(|(n, _)| !new.contains_key(*n))
                .map(|(name, p)| PackageLine {
                    name: name.clone(),
                    version: p.version.clone(),
                    before: None,
                    source: source(p),
                    change: Change::Removed,
                    resembles: None,
                }),
        );
    }
    let order = |c: Change| match c {
        Change::Added | Change::Changed => 0,
        Change::Asked | Change::Same => 1,
        Change::Removed => 2,
    };
    out.sort_by(|a, b| (order(a.change), &a.name).cmp(&(order(b.change), &b.name)));
    out
}
