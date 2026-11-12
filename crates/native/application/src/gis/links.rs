//! Where a linked file is (docs/adr/0349 §1): its path as kept when absolute, else its relative path from the open
//! drawing's folder, else (an external reference's) in the project's search paths, first its relative path there,
//! then its file name. The places looked in are kept, so a missing file says where it was looked for.

use std::path::{Path, PathBuf};

use kentos_commands::AssetVia;

use crate::source_places::resolve;

/// A linked file as looked for.
#[derive(Clone, Debug, PartialEq)]
pub struct Found {
    pub at: Option<PathBuf>,
    pub via: AssetVia,
    /// The places looked in, in order (each once).
    pub tried: Vec<String>,
}

/// `path` (absolute, or relative to the drawing's folder) and its `relative` path, from the open drawing's file
/// `drawing`, then in `search` (absolute folders, or relative to the drawing's folder).
pub fn find(path: &str, relative: Option<&str>, drawing: Option<&str>, search: &[String]) -> Found {
    let folder = drawing.and_then(|d| Path::new(d).parent());
    find_in(path, relative, folder, search)
}

/// [`find`] from the drawing's folder itself (Paketle's `from`, docs/adr/0350 §4).
pub fn find_in(path: &str, relative: Option<&str>, folder: Option<&Path>, search: &[String]) -> Found {
    let mut tried: Vec<String> = Vec::new();
    let look = |p: PathBuf, tried: &mut Vec<String>| -> Option<PathBuf> {
        let text = p.display().to_string();
        if !tried.contains(&text) {
            tried.push(text);
        }
        p.is_file().then_some(p)
    };
    let kept = Path::new(path);
    if kept.is_absolute()
        && let Some(at) = look(kept.to_path_buf(), &mut tried)
    {
        return Found { at: Some(at), via: AssetVia::Path, tried: Vec::new() };
    }
    let folder: Option<PathBuf> = folder.map(Path::to_path_buf);
    // The relative path: the one kept beside, else the path itself when it is relative.
    let rel: Option<String> = relative
        .map(str::to_owned)
        .or_else(|| (!kept.is_absolute() && !path.trim().is_empty()).then(|| path.replace('\\', "/")));
    if let (Some(folder), Some(rel)) = (&folder, &rel)
        && let Some(at) = look(resolve(folder, rel), &mut tried)
    {
        return Found { at: Some(at), via: AssetVia::Relative, tried: Vec::new() };
    }
    let name = kept.file_name().map(|n| n.to_string_lossy().into_owned());
    for s in search {
        let base = if Path::new(s).is_absolute() {
            PathBuf::from(s)
        } else if let Some(folder) = &folder {
            resolve(folder, s)
        } else {
            continue;
        };
        if let Some(rel) = &rel
            && let Some(at) = look(resolve(&base, rel), &mut tried)
        {
            return Found { at: Some(at), via: AssetVia::SearchPath, tried: Vec::new() };
        }
        if let Some(name) = &name
            && let Some(at) = look(base.join(name), &mut tried)
        {
            return Found { at: Some(at), via: AssetVia::SearchPath, tried: Vec::new() };
        }
    }
    Found { at: None, via: AssetVia::None, tried }
}
