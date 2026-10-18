//! Where a bound layer's file is (docs/adr/0506, plan/gis.md CBS-36): a project moved with its data, or packed with
//! Paketle, finds its GeoPackages again. Saving writes each file's path from the project file's folder (`relative`,
//! on the snapshot written, the drawing unchanged); opening, a link whose path is not there is moved to the file its
//! relative path names when that file has the content the layer last matched (its SHA-256 is the link's revision).
//! A file that is there with another content is not taken: the link waits for the user (Kaynak ▸ Yeniden bağla).
//! A database's link has no relative path.

use std::path::{Component, Path, PathBuf};

use kentos_contracts::encoding::sha256_hex;
use kentos_contracts::{ProjectSettings, SourceKind};
use kentos_domain::Document;

/// `file`'s path from `folder`, `/` between its parts; none when the two are not on the same root (another drive)
/// or either is not absolute. Lexical: `.` and `..` are resolved by their names, links are not followed.
pub fn relative_path(folder: &Path, file: &Path) -> Option<String> {
    if !folder.is_absolute() || !file.is_absolute() {
        return None;
    }
    let a = lexical(folder);
    let b = lexical(file);
    if a.first() != b.first() {
        return None;
    }
    let common = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    if common == b.len() {
        return None;
    }
    let mut parts: Vec<String> = vec!["..".into(); a.len() - common];
    for c in &b[common..] {
        match c {
            Component::Normal(n) => parts.push(n.to_string_lossy().into_owned()),
            _ => return None,
        }
    }
    Some(parts.join("/"))
}

/// A path's components with `.` dropped and `..` taking back the one before it.
fn lexical(p: &Path) -> Vec<Component<'_>> {
    let mut out: Vec<Component<'_>> = Vec::new();
    for c in p.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                if matches!(out.last(), Some(Component::Normal(_))) {
                    out.pop();
                }
            }
            other => out.push(other),
        }
    }
    out
}

/// `relative` (`/` between its parts) from `folder`, resolved lexically.
pub fn resolve(folder: &Path, relative: &str) -> PathBuf {
    let mut p = folder.to_path_buf();
    for part in relative.split('/') {
        match part {
            ".." => {
                p.pop();
            }
            "" | "." => {}
            name => p.push(name),
        }
    }
    p
}

/// The project's settings as a save to `project_file` writes them: each GeoPackage link with its path from the
/// file's folder, none where there is no such path (another drive).
pub fn with_relative(settings: &mut ProjectSettings, project_file: &Path) {
    let Some(folder) = project_file.parent() else {
        return;
    };
    for s in &mut settings.sources {
        if s.kind == SourceKind::Geopackage {
            s.relative = relative_path(folder, Path::new(&s.path));
        }
    }
}

/// What opening found of the project's bound files.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Located {
    /// Links moved to their relative path: id, layer, the old path, the new one.
    pub moved: Vec<(String, String, String, String)>,
    /// Links whose file is at neither place: id, layer, path.
    pub missing: Vec<(String, String, String)>,
    /// Links whose relative path names a file with another content: id, layer, that file. Not moved.
    pub differs: Vec<(String, String, String)>,
}

impl Located {
    pub fn is_empty(&self) -> bool {
        self.moved.is_empty() && self.missing.is_empty() && self.differs.is_empty()
    }
}

/// Opening `project_file`: each GeoPackage link whose path is not there is moved to the file its relative path
/// names, when that file has the content the layer last matched. A project setting changed, never an undo step.
pub fn relocate(doc: &mut Document, project_file: &Path) -> Located {
    let mut found = Located::default();
    let Some(folder) = project_file.parent() else {
        return found;
    };
    let mut settings = doc.settings().clone();
    for s in &mut settings.sources {
        if s.kind != SourceKind::Geopackage || Path::new(&s.path).is_file() {
            continue;
        }
        let Some(rel) = &s.relative else {
            found
                .missing
                .push((s.id.clone(), s.layer.clone(), s.path.clone()));
            continue;
        };
        let at = resolve(folder, rel);
        if !at.is_file() {
            found
                .missing
                .push((s.id.clone(), s.layer.clone(), s.path.clone()));
            continue;
        }
        let same = std::fs::read(&at).is_ok_and(|b| sha256_hex(&b) == s.revision);
        let to = at.display().to_string();
        if same {
            found.moved.push((
                s.id.clone(),
                s.layer.clone(),
                std::mem::replace(&mut s.path, to.clone()),
                to,
            ));
        } else {
            found.differs.push((s.id.clone(), s.layer.clone(), to));
        }
    }
    if !found.moved.is_empty() {
        doc.set_settings(settings);
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_are_written_from_the_projects_folder() {
        let r = |a: &str, b: &str| relative_path(Path::new(a), Path::new(b));
        assert_eq!(
            r("/is/proje", "/is/proje/veri/k.gpkg").as_deref(),
            Some("veri/k.gpkg")
        );
        assert_eq!(
            r("/is/proje", "/is/ortak/k.gpkg").as_deref(),
            Some("../ortak/k.gpkg")
        );
        assert_eq!(r("/is/proje", "/k.gpkg").as_deref(), Some("../../k.gpkg"));
        assert_eq!(
            r("/is/./proje", "/is/x/../proje/k.gpkg").as_deref(),
            Some("k.gpkg")
        );
        assert_eq!(r("/is/proje", "/is/proje"), None);
        assert_eq!(r("is/proje", "/is/proje/k.gpkg"), None);
        assert_eq!(
            resolve(Path::new("/is/proje"), "../ortak/k.gpkg"),
            PathBuf::from("/is/ortak/k.gpkg")
        );
        for case in ["veri/k.gpkg", "../ortak/k.gpkg", "../../k.gpkg"] {
            assert_eq!(kentos_contracts::relative_problem(case), None, "{case}");
        }
    }
}
