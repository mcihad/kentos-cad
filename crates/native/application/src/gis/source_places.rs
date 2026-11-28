//! Where a bound layer's file is (docs/adr/0506, plan/gis.md CBS-36): a project moved with its data, or packed with
//! Paketle, finds its GeoPackages again. Saving writes each file's path from the project file's folder (`relative`,
//! on the snapshot written, the drawing unchanged); opening, a link whose path is not there is moved to the file its
//! relative path names when that file has the content the layer last matched (its SHA-256 is the link's revision).
//! A file that is there with another content is not taken: the link waits for the user (Kaynak ▸ Yeniden bağla).
//! A database's link has no relative path. A GeoPackage data layer (docs/adr/9107 §3) is written the same way, its
//! `relative` beside its `address`; opened, a layer whose file is not at its address is moved to its relative path when
//! a file is there (a data layer is live, so its content is not compared). A project kept inside a GeoPackage takes
//! that file as its own place: its layers in the same file name it by its name and travel with it.

use std::path::{Component, Path, PathBuf};

use kentos_commands::encoding::sha256_hex;
use kentos_commands::{Entity, LayerNode, ProjectSettings, SourceKind};
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
    // A map work item's file, the same way (docs/adr/1201 §1).
    if let Some(work) = &mut settings.map_work {
        for f in work.items.iter_mut().filter_map(|i| i.file.as_mut()) {
            f.relative = relative_path(folder, Path::new(&f.path));
        }
    }
}

/// The data layers as a save to `project_file` writes them (docs/adr/9107 §3): each GeoPackage data layer with its
/// file's path from the project's folder, none where there is no such path (another drive) or for a database's. A
/// project kept in a GeoPackage passes that file: its own layers are named by its name.
pub fn with_relative_layers(layers: &mut [LayerNode], project_file: &Path) {
    let Some(folder) = project_file.parent() else {
        return;
    };
    for node in layers {
        if let Some(d) = node.data.as_mut() {
            d.relative = (d.driver == "gpkg")
                .then(|| {
                    relative_path(
                        folder,
                        Path::new(d.address.strip_prefix("gpkg:").unwrap_or(&d.address)),
                    )
                })
                .flatten();
        }
        with_relative_layers(&mut node.children, project_file);
    }
}

/// The file a linked raster or picture shows; none for an embedded one or another object.
fn linked_file(e: &mut Entity) -> Option<&mut String> {
    match e {
        Entity::Raster(r) => r.raster.file.as_mut(),
        Entity::Image(i) => i.image.file.as_mut(),
        _ => None,
    }
}

/// Linked rasters' and pictures' files as a project kept in a GeoPackage writes them (docs/adr/9117 §3): each one's
/// path from that file's folder, `/` between its parts, so the project and the files beside it move together (a tile
/// table of the same file is named by the file's name); a file on another drive stays absolute. Read back with
/// [`with_absolute_files`].
pub fn with_relative_files(entities: &mut [Entity], project_file: &Path) {
    let Some(folder) = project_file.parent() else {
        return;
    };
    for e in entities {
        if let Some(file) = linked_file(e)
            && let Some(rel) = relative_path(folder, Path::new(file.as_str()))
        {
            *file = rel;
        }
    }
}

/// The other way (docs/adr/9117 §3): a relative raster or picture file from the project file's folder, absolute again,
/// so what reads the opened project finds it whatever its own folder is.
pub fn with_absolute_files(entities: &mut [Entity], project_file: &Path) {
    let Some(folder) = project_file.parent() else {
        return;
    };
    for e in entities {
        if let Some(file) = linked_file(e)
            && !file.trim().is_empty()
            && Path::new(file.as_str()).is_relative()
        {
            *file = resolve(folder, &file.replace('\\', "/"))
                .display()
                .to_string();
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
    /// Map work items' files (docs/adr/1201 §1), the same three ways: item id, item name, the paths.
    pub work_moved: Vec<(String, String, String, String)>,
    pub work_missing: Vec<(String, String, String)>,
    pub work_differs: Vec<(String, String, String)>,
    /// GeoPackage data layers (docs/adr/9107 §3) found at their relative path: layer id, layer name, the paths.
    pub layers_moved: Vec<(String, String, String, String)>,
    /// GeoPackage data layers whose file is at neither place: layer id, layer name, path.
    pub layers_missing: Vec<(String, String, String)>,
}

impl Located {
    pub fn is_empty(&self) -> bool {
        self.moved.is_empty()
            && self.missing.is_empty()
            && self.differs.is_empty()
            && self.work_moved.is_empty()
            && self.work_missing.is_empty()
            && self.work_differs.is_empty()
            && self.layers_moved.is_empty()
            && self.layers_missing.is_empty()
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
    // A map work item's file moved with the project: taken at its relative path when its content is the item's
    // revision (docs/adr/1201 §1); another content there waits for the user (Yeniden bağla, Yenile).
    if let Some(work) = &mut settings.map_work {
        for item in &mut work.items {
            let Some(f) = item.file.as_mut().filter(|f| !Path::new(&f.path).is_file()) else {
                continue;
            };
            let at = f
                .relative
                .as_deref()
                .map(|rel| resolve(folder, rel))
                .filter(|p| p.is_file());
            let Some(at) = at else {
                found
                    .work_missing
                    .push((item.id.clone(), item.name.clone(), f.path.clone()));
                continue;
            };
            let to = at.display().to_string();
            // Streamed: an orthophoto may be large.
            if crate::file_digest::sha256(&at).is_ok_and(|(_, sha)| sha == f.revision) {
                found.work_moved.push((
                    item.id.clone(),
                    item.name.clone(),
                    std::mem::replace(&mut f.path, to.clone()),
                    to,
                ));
            } else {
                found
                    .work_differs
                    .push((item.id.clone(), item.name.clone(), to));
            }
        }
    }
    if !found.moved.is_empty() || !found.work_moved.is_empty() {
        doc.set_settings(settings);
    }
    relocate_layers(doc, folder, &mut found);
    found
}

/// The GeoPackage data layers whose file is not at its address, moved to their relative path when a file is there.
fn relocate_layers(doc: &mut Document, folder: &Path, found: &mut Located) {
    let mut moves: Vec<(String, String, String, String)> = Vec::new();
    for node in doc.layers().leaves() {
        let Some(d) = node.data.as_ref().filter(|d| d.driver == "gpkg") else {
            continue;
        };
        let path = d.address.strip_prefix("gpkg:").unwrap_or(&d.address);
        if Path::new(path).is_file() {
            continue;
        }
        match d
            .relative
            .as_deref()
            .map(|rel| resolve(folder, rel))
            .filter(|p| p.is_file())
        {
            Some(at) => moves.push((
                node.id.clone(),
                node.name.clone(),
                d.address.clone(),
                at.display().to_string(),
            )),
            None => {
                found
                    .layers_missing
                    .push((node.id.clone(), node.name.clone(), d.address.clone()))
            }
        }
    }
    for (id, name, from, to) in moves {
        doc.set_layer_data_address(&id, &to);
        found.layers_moved.push((id, name, from, to));
    }
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
            assert_eq!(kentos_commands::relative_problem(case), None, "{case}");
        }
    }
}
