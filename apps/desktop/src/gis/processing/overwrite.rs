//! A chosen output file that is already there (gis-usage-report GIS-035, docs/adr/1092): the objects of the open
//! drawing whose linked file it is — a point cloud's member, a raster — said under Çıktı dosyası before the run writes
//! over it, so a result that would silently change under another is seen. The automatic name never takes a file that
//! is there (`kentos_processing::files::free_output`); only a path the user picked can.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use kentos_commands::Entity;
use kentos_domain::Document;
use kentos_processing::{ParamKind, Tool, Values};

/// A chosen output that is there: whether the file is, and the objects linked to it by their names.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Overwrite {
    pub bound: Vec<String>,
}

impl Overwrite {
    /// The line under the path.
    pub fn line(&self) -> String {
        if self.bound.is_empty() {
            return "Bu dosya var: çalıştırınca üzerine yazılır.".to_owned();
        }
        let shown: Vec<String> = self
            .bound
            .iter()
            .take(3)
            .map(|n| format!("“{n}”"))
            .collect();
        let more = self.bound.len().saturating_sub(3);
        let tail = if more > 0 {
            format!(" ve {more} nesne daha")
        } else {
            String::new()
        };
        format!(
            "Bu dosya var ve çizimde {} nesne ona bağlı ({}{tail}): çalıştırınca üzerine yazılır, onlar da değişir. \
             Ayrı bir sonuç için başka bir ad seçin ya da alanı boşaltın.",
            self.bound.len(),
            shown.join(", ")
        )
    }
}

/// Each save-file parameter whose chosen path is a file there, with the drawing's objects linked to it (relative
/// links read beside the drawing in `folder`).
pub fn overwrites(
    doc: &Document,
    folder: Option<&Path>,
    tool: &Tool,
    values: &Values,
) -> BTreeMap<String, Overwrite> {
    let mut out = BTreeMap::new();
    for p in &tool.parameters {
        if !matches!(p.kind, ParamKind::SaveFile { .. }) {
            continue;
        }
        let asked = values
            .get(&p.name)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        if asked.is_empty() || !Path::new(asked).is_file() {
            continue;
        }
        out.insert(
            p.name.clone(),
            Overwrite {
                bound: bound_to(doc, folder, Path::new(asked)),
            },
        );
    }
    out
}

/// The objects whose linked file `path` is: each by its label, else its layer's name.
pub fn bound_to(doc: &Document, folder: Option<&Path>, path: &Path) -> Vec<String> {
    let target = same(path);
    let mut out = Vec::new();
    for e in doc.entities_with_uids().map(|(_, e)| e) {
        let files: Vec<&str> = match e {
            Entity::PointCloud(c) => c
                .cloud
                .sources
                .iter()
                .filter_map(|s| s.file.as_deref())
                .collect(),
            Entity::Raster(r) => r.raster.file.as_deref().into_iter().collect(),
            _ => continue,
        };
        if files
            .iter()
            .any(|f| same(&crate::pictures::resolve(f, folder)) == target)
        {
            let base = e.base();
            let label = base
                .label
                .as_deref()
                .map(str::trim)
                .filter(|l| !l.is_empty());
            let name = label.map(str::to_owned).unwrap_or_else(|| {
                doc.layers()
                    .get(&base.layer_id)
                    .map_or_else(|| "adsız".to_owned(), |l| l.name.clone())
            });
            out.push(name);
        }
    }
    out
}

/// A path as the disk names it, so `a/../b.laz` and `b.laz` are one file.
fn same(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}
