//! What a version's package says, as the release flow reads it (plan/eklenti.md ADIM 22A; PY-CLOUD-04): its
//! manifest, its integrity list and its Python project, taken from the inspector's reading of the bytes. The release's
//! identity ([`ReleaseFacts::content`]) is computed from these, so it is what the bytes say, never what an upload
//! claims; the review compares two versions' facts ([`super::review`]).
//!
//! The Python project and lock are read twice: as they are (the lock's digest is part of the identity) and without
//! what every version changes (the project's own version in both files, the lock format's revision), so a version
//! that only bumps its number is not taken for a changed project or lock.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{Refusal, ReleaseContent};
use crate::archive::Opened;
use crate::integrity::{self, FileDigest, Integrity, sha256_hex};
use crate::manifest::{Manifest, PythonProject};
use crate::python::{Toml, normalized, parse_toml};

/// The index most packages come from: a package from it is named by its version alone.
const DEFAULT_INDEX: &str = "registry https://pypi.org/simple";

/// What a version's package says, read by [`ReleaseFacts::of`]; its [`content`](Self::content) is the release's
/// identity.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReleaseFacts {
    pub manifest: Manifest,
    /// Every content file's path, size and SHA-256: the integrity list.
    pub files: Vec<FileDigest>,
    /// The Python project and its lock, when the package has them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub python: Option<PythonFacts>,
    /// The SBOM's SHA-256, once the catalog made one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sbom: Option<String>,
}

/// A package's Python project as the review compares it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PythonFacts {
    /// The lock file's SHA-256 as it is: part of the release's identity.
    pub lock: String,
    /// The lock's SHA-256 read without the project's own version and the lock format's revision.
    pub lock_compared: String,
    /// The project file's SHA-256 read without the project's version.
    pub project_compared: String,
    /// What the lock installs, the project itself left out, by normalised name.
    pub packages: BTreeMap<String, LockedPackage>,
}

/// A package the lock installs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LockedPackage {
    pub version: String,
    /// Where it comes from, as the lock says: `registry https://pypi.org/simple`, `git https://…`.
    pub source: String,
    /// The SHA-256 of its distribution files' hashes, sorted: the same version with other files differs.
    pub files: String,
}

impl LockedPackage {
    /// From the Python Package Index, as most packages are.
    pub fn from_the_index(&self) -> bool {
        self.source == DEFAULT_INDEX
    }

    pub(super) fn describe(&self) -> String {
        if self.source == DEFAULT_INDEX {
            self.version.clone()
        } else {
            format!("{} · {}", self.version, self.source)
        }
    }
}

impl ReleaseFacts {
    /// What a person sees of it in a catalog: its name, and its icon's SHA-256 from the integrity list.
    pub fn listing(&self) -> super::Listing {
        super::Listing {
            name: self.manifest.name.clone(),
            icon: self.manifest.icon.as_ref().and_then(|icon| {
                self.files
                    .iter()
                    .find(|f| f.path == *icon)
                    .map(|f| f.sha256.clone())
            }),
        }
    }

    /// The facts of an opened package, as the inspector read it; a package that did not pass has none.
    pub fn of(opened: &Opened) -> Result<ReleaseFacts, Refusal> {
        let inspection = &opened.inspection;
        if let Some(problem) = inspection.problems.first() {
            return Err(Refusal::new(
                "package_invalid",
                format!("Paket denetimden geçmedi: {problem}"),
            ));
        }
        let (Some(manifest), Some(digest)) = (&inspection.manifest, &inspection.digest) else {
            return Err(Refusal::new(
                "package_invalid",
                "Paketin manifesti ya da bütünlük listesi okunamadı.",
            ));
        };
        let python = match &manifest.python {
            Some(project) => Some(python_facts(project, &opened.files)?),
            None => None,
        };
        let facts = ReleaseFacts {
            manifest: manifest.clone(),
            files: inspection.files.clone(),
            python,
            sbom: None,
        };
        if facts.content().package != *digest {
            return Err(Refusal::new(
                "package_invalid",
                "Paketin içerik özeti bütünlük listesiyle tutmuyor.",
            ));
        }
        Ok(facts)
    }

    /// What the release is (PY-CLOUD-04).
    pub fn content(&self) -> ReleaseContent {
        ReleaseContent {
            package: Integrity {
                format: integrity::FORMAT.to_owned(),
                schema_version: integrity::SCHEMA_VERSION,
                files: self.files.clone(),
            }
            .digest(),
            lock: self.python.as_ref().map(|p| p.lock.clone()),
            sbom: self.sbom.clone(),
            capabilities: self.manifest.capabilities(),
        }
    }
}

/// What a Python project's lock installs, the project itself left out, by normalised name: the install review's list
/// and an update's difference (docs/adr/8057 §81). None when its files are missing or not read.
pub fn locked_packages(
    project: &PythonProject,
    files: &BTreeMap<String, Vec<u8>>,
) -> Option<BTreeMap<String, LockedPackage>> {
    python_facts(project, files).ok().map(|p| p.packages)
}

fn python_facts(
    project: &PythonProject,
    files: &BTreeMap<String, Vec<u8>>,
) -> Result<PythonFacts, Refusal> {
    let lock_text = text(files, &project.lock_file)?;
    let mut lock = toml(files, &project.lock_file)?;
    let mut pyproject = toml(files, &project.project_file)?;
    // The project's version goes from both files: every version changes it.
    let own = match &mut pyproject {
        Toml::Table(root) => match root.get_mut("project") {
            Some(Toml::Table(p)) => {
                p.remove("version");
                match p.get("name") {
                    Some(Toml::Str(name)) => Some(normalized(name)),
                    _ => None,
                }
            }
            _ => None,
        },
        _ => None,
    };
    let mut packages = BTreeMap::new();
    if let Toml::Table(root) = &mut lock {
        root.remove("revision");
        if let Some(Toml::Array(list)) = root.get_mut("package") {
            for entry in list.iter_mut() {
                let Toml::Table(p) = entry else { continue };
                let Some(Toml::Str(name)) = p.get("name") else {
                    continue;
                };
                let name = normalized(name);
                if own.as_deref() == Some(name.as_str()) {
                    p.remove("version");
                    continue;
                }
                let version = match p.get("version") {
                    Some(Toml::Str(v)) => v.clone(),
                    _ => String::new(),
                };
                let package = LockedPackage {
                    version,
                    source: source(p.get("source")),
                    files: distribution_hashes(p),
                };
                packages.insert(name, package);
            }
        }
    }
    Ok(PythonFacts {
        lock: sha256_hex(lock_text.as_bytes()),
        lock_compared: sha256_hex(canonical(&lock).as_bytes()),
        project_compared: sha256_hex(canonical(&pyproject).as_bytes()),
        packages,
    })
}

fn text<'a>(files: &'a BTreeMap<String, Vec<u8>>, path: &str) -> Result<&'a str, Refusal> {
    files
        .get(path)
        .and_then(|b| std::str::from_utf8(b).ok())
        .ok_or_else(|| {
            Refusal::new(
                "package_invalid",
                format!("{path} pakette yok ya da UTF-8 değil."),
            )
        })
}

fn toml(files: &BTreeMap<String, Vec<u8>>, path: &str) -> Result<Toml, Refusal> {
    parse_toml(text(files, path)?).map_err(|(line, message)| {
        Refusal::new(
            "package_invalid",
            format!("{path}, satır {line}: TOML okunamadı ({message})."),
        )
    })
}

/// A lock entry's `source` table as one line: `registry https://pypi.org/simple`.
fn source(value: Option<&Toml>) -> String {
    match value {
        Some(Toml::Table(table)) => table
            .iter()
            .map(|(kind, v)| match v {
                Toml::Str(s) => format!("{kind} {s}"),
                other => format!("{kind} {}", canonical(other)),
            })
            .collect::<Vec<_>>()
            .join(", "),
        Some(other) => canonical(other),
        None => String::new(),
    }
}

fn distribution_hashes(package: &BTreeMap<String, Toml>) -> String {
    let wheels: &[Toml] = match package.get("wheels") {
        Some(Toml::Array(wheels)) => wheels,
        _ => &[],
    };
    let mut hashes: Vec<&str> = package
        .get("sdist")
        .into_iter()
        .chain(wheels)
        .filter_map(|file| match file {
            Toml::Table(f) => match f.get("hash") {
                Some(Toml::Str(hash)) => Some(hash.as_str()),
                _ => None,
            },
            _ => None,
        })
        .collect();
    hashes.sort_unstable();
    sha256_hex(hashes.join("\n").as_bytes())
}

/// A TOML value written one way, tables in key order: what two files' contents are compared by.
fn canonical(value: &Toml) -> String {
    fn write(value: &Toml, out: &mut String) {
        let quoted = |s: &str| serde_json::to_string(s).unwrap_or_default();
        match value {
            Toml::Str(s) => out.push_str(&quoted(s)),
            Toml::Int(i) => out.push_str(&i.to_string()),
            Toml::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Toml::Other(s) => {
                out.push('~');
                out.push_str(&quoted(s));
            }
            Toml::Array(items) => {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    write(item, out);
                }
                out.push(']');
            }
            Toml::Table(table) => {
                out.push('{');
                for (i, (key, item)) in table.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    out.push_str(&quoted(key));
                    out.push(':');
                    write(item, out);
                }
                out.push('}');
            }
        }
    }
    let mut out = String::new();
    write(value, &mut out);
    out
}
