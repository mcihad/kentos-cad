//! The review of external references (docs/adr/1060; plan/cad-advanced.md `CAD-ADV-14`): what `cad.xrefs.list`'s
//! `review` says of each reference (its revision, its depth in the network, what the drawing shows of it, what it
//! brought, the layer states naming its layers, the files that may be its moved source), what a reload's or a
//! repath's `diff` says it changes (its pieces and the objects tied into them), the layers' looks set back to the
//! source's (`resetLayers`) and the nested references found from the source's own folder.
//!
//! Nothing here binds a file: a candidate is a file to be reviewed, bound only by Yolu değiştir.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use kentos_commands::encoding::sha256_reader_hex;
use kentos_commands::{
    BlockDefinition, BlockId, Entity, LayerNode, MAX_CANDIDATE_BYTES, XrefCandidate, XrefContent,
    XrefDiff, XrefFollowEffect, XrefFollower, XrefFollowerKind, XrefMatch, XrefSource, XrefState,
};
use kentos_domain::{Document, Slot};
use serde_json::Value;

use crate::source_places::{relative_path, resolve};

/// What a review adds to a listed reference.
pub(crate) struct Review {
    pub depth: u32,
    pub content: XrefContent,
    pub brings: Vec<String>,
    pub layer_states: Vec<String>,
    pub candidates: Vec<XrefCandidate>,
}

/// The review of the reference `b` (its source `x`, its state, where its file was found) from the open drawing's
/// file `drawing`; `budget` the files the candidates may still read.
pub(crate) fn review(
    doc: &Document,
    b: &BlockDefinition,
    x: &XrefSource,
    state: XrefState,
    found: Option<&Path>,
    drawing: Option<&str>,
    budget: &mut usize,
) -> Review {
    let mut depth = 0;
    let mut up = x.parent;
    while let Some(p) = up {
        depth += 1;
        if depth >= 64 {
            break;
        }
        up = doc
            .block(p)
            .and_then(|d| d.xref.as_ref())
            .and_then(|x| x.parent);
    }
    let content = match state {
        XrefState::Loaded => XrefContent::Source,
        XrefState::Unloaded => XrefContent::None,
        XrefState::Missing | XrefState::Changed => XrefContent::Cached,
    };
    let brings = doc
        .blocks()
        .iter()
        .filter(|d| d.xref.as_ref().and_then(|x| x.parent) == Some(b.id))
        .map(|d| d.name.clone())
        .collect();
    let candidates = if matches!(state, XrefState::Missing | XrefState::Changed) {
        candidates(
            &x.path,
            x.relative.as_deref(),
            &x.digest,
            drawing,
            &doc.settings().xref_paths,
            found,
            budget,
        )
    } else {
        Vec::new()
    };
    Review {
        depth,
        content,
        brings,
        layer_states: layer_states(doc, x.group.as_deref()),
        candidates,
    }
}

/// The project's layer states naming a node under `group` (the group too), by name in the project's order.
fn layer_states(doc: &Document, group: Option<&str>) -> Vec<String> {
    let Some(node) = group.and_then(|g| doc.layers().get(g)) else {
        return Vec::new();
    };
    fn walk(n: &LayerNode, out: &mut HashSet<String>) {
        out.insert(n.id.clone());
        for c in &n.children {
            walk(c, out);
        }
    }
    let mut ids = HashSet::new();
    walk(node, &mut ids);
    doc.settings()
        .layer_states
        .iter()
        .filter(|s| s.nodes.iter().any(|n| ids.contains(&n.node)))
        .map(|s| s.name.clone())
        .collect()
}

/// A path as the file system names it, else as given.
fn canonical(p: &Path) -> PathBuf {
    std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
}

/// The files that may be the source kept at `path` (docs/adr/1060 §2), from the allowed places in this order: the
/// kept path's folder, the folder of its relative path from the drawing's, the drawing's folder, the project's search
/// paths; each once, not looked into further. In each, in the order of the files' names, those with the kept file's
/// extension (letter case aside), at most [`MAX_CANDIDATE_BYTES`] long and neither `found` (the file the reference
/// is bound to now) nor the open drawing: one whose SHA-256 is `digest` matches by content; another of the kept
/// file's name, by name. The content matches first, then the name matches, each in the order found. `budget` is the
/// files that may still be read; when it runs out the rest are not looked at.
pub fn candidates(
    path: &str,
    relative: Option<&str>,
    digest: &str,
    drawing: Option<&str>,
    search: &[String],
    found: Option<&Path>,
    budget: &mut usize,
) -> Vec<XrefCandidate> {
    let kept = Path::new(path);
    let drawing_folder = drawing.and_then(|d| Path::new(d).parent());
    let mut folders: Vec<PathBuf> = Vec::new();
    if kept.is_absolute()
        && let Some(p) = kept.parent()
    {
        folders.push(p.to_path_buf());
    }
    if let (Some(folder), Some(rel)) = (drawing_folder, relative)
        && let Some(p) = resolve(folder, rel).parent()
    {
        folders.push(p.to_path_buf());
    }
    if let Some(folder) = drawing_folder {
        folders.push(folder.to_path_buf());
    }
    for root in search {
        if Path::new(root).is_absolute() {
            folders.push(PathBuf::from(root));
        } else if let Some(folder) = drawing_folder {
            folders.push(resolve(folder, root));
        }
    }
    let mut seen: HashSet<PathBuf> = HashSet::new();
    folders.retain(|f| seen.insert(canonical(f)));
    let extension = kept.extension().map(|e| e.to_string_lossy().to_lowercase());
    let name = kept.file_name().map(|n| n.to_os_string());
    let skip: Vec<PathBuf> = found
        .into_iter()
        .map(canonical)
        .chain(drawing.map(|d| canonical(Path::new(d))))
        .collect();
    let (mut by_content, mut by_name): (Vec<XrefCandidate>, Vec<XrefCandidate>) =
        (Vec::new(), Vec::new());
    let mut looked: HashSet<PathBuf> = HashSet::new();
    'folders: for folder in folders {
        let Ok(entries) = std::fs::read_dir(&folder) else {
            continue;
        };
        let mut files: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
        files.sort();
        for file in files {
            if file.extension().map(|e| e.to_string_lossy().to_lowercase()) != extension {
                continue;
            }
            let Ok(meta) = std::fs::metadata(&file) else {
                continue;
            };
            if !meta.is_file() || meta.len() > MAX_CANDIDATE_BYTES {
                continue;
            }
            let real = canonical(&file);
            if skip.contains(&real) || !looked.insert(real) {
                continue;
            }
            if *budget == 0 {
                break 'folders;
            }
            *budget -= 1;
            let Ok((_, sha)) = std::fs::File::open(&file).and_then(sha256_reader_hex) else {
                continue;
            };
            let at = file.display().to_string();
            if sha == digest {
                by_content.push(XrefCandidate {
                    path: at,
                    matched: XrefMatch::Content,
                    digest: sha,
                });
            } else if file.file_name().map(|n| n.to_os_string()) == name {
                by_name.push(XrefCandidate {
                    path: at,
                    matched: XrefMatch::Name,
                    digest: sha,
                });
            }
        }
    }
    by_content.extend(by_name);
    by_content
}

/// The references `id` brought (`blocks`, as loaded from `source_file`) found from the source's own folder, as the
/// source found them (docs/adr/1060 §6): one found elsewhere than its kept path is bound there, its relative path
/// written from the open drawing's folder. One not found keeps what the source said.
pub(crate) fn relocate_nested(
    blocks: &mut [BlockDefinition],
    id: BlockId,
    source_file: &str,
    drawing: Option<&str>,
    search: &[String],
) {
    let Some(source_folder) = Path::new(source_file).parent() else {
        return;
    };
    let ids: HashSet<BlockId> = std::iter::once(id)
        .chain(blocks.iter().map(|b| b.id))
        .collect();
    for b in blocks.iter_mut() {
        let Some(x) = b.xref.as_mut() else { continue };
        if b.id == id || !x.parent.is_some_and(|p| ids.contains(&p)) {
            continue;
        }
        let f = crate::xrefs::found_source_in(
            &x.path,
            x.relative.as_deref(),
            &x.digest,
            Some(source_folder),
            search,
        );
        let Some(at) = f.at else { continue };
        if at == Path::new(&x.path) {
            continue;
        }
        x.path = at.to_string_lossy().into_owned();
        x.relative = drawing
            .and_then(|d| Path::new(d).parent())
            .and_then(|folder| relative_path(folder, &at));
    }
}

/// What can follow a reference's pieces: a dimension, a leader, an associative hatch, a text's field, a table's cell.
fn follower_kind(e: &Entity) -> Option<XrefFollowerKind> {
    Some(match e {
        Entity::Dimension(_) => XrefFollowerKind::Dimension,
        Entity::Leader(_) => XrefFollowerKind::Leader,
        Entity::Hatch(_) => XrefFollowerKind::Hatch,
        Entity::Text(_) => XrefFollowerKind::Text,
        Entity::Table(_) => XrefFollowerKind::Table,
        _ => return None,
    })
}

/// How many ties a value says are broken: its `broken` fields that hold something.
fn broken_count(v: &Value) -> usize {
    match v {
        Value::Object(o) => o
            .iter()
            .map(|(k, v)| {
                if k == "broken" && !v.is_null() {
                    1
                } else {
                    broken_count(v)
                }
            })
            .sum(),
        Value::Array(a) => a.iter().map(broken_count).sum(),
        _ => 0,
    }
}

/// A reference's state before a reload or a repath, for its `diff`.
pub(crate) struct Before {
    pieces: Vec<(u32, Value)>,
    nested: Vec<String>,
    tied: Vec<(Slot, XrefFollowerKind, Value)>,
}

fn pieces(def: Option<&BlockDefinition>) -> Vec<(u32, Value)> {
    def.map(|d| {
        d.entities
            .iter()
            .filter_map(|e| {
                Some((
                    e.base().id,
                    kentos_domain::xref::content_of(&serde_json::to_value(e).ok()?),
                ))
            })
            .collect()
    })
    .unwrap_or_default()
}

/// The definitions `id` brought, by name, in the drawing's block order.
fn nested_names(doc: &Document, id: BlockId) -> Vec<String> {
    let brought: HashSet<BlockId> = crate::xrefs::brought(doc, id).into_iter().collect();
    doc.blocks()
        .iter()
        .filter(|b| brought.contains(&b.id))
        .map(|b| b.name.clone())
        .collect()
}

impl Before {
    pub(crate) fn of(doc: &Document, id: BlockId) -> Self {
        let tied = doc
            .entities()
            .filter_map(|e| {
                let kind = follower_kind(e)?;
                Some((Slot(e.base().id), kind, serde_json::to_value(e).ok()?))
            })
            .collect();
        Self {
            pieces: pieces(doc.block(id)),
            nested: nested_names(doc, id),
            tied,
        }
    }

    /// What the step changed, now that it is written.
    pub(crate) fn diff(self, doc: &Document, id: BlockId) -> XrefDiff {
        let now = pieces(doc.block(id));
        let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
        let mut d = XrefDiff::default();
        for (k, c) in &now {
            match self.pieces.iter().find(|(o, _)| o == k) {
                Some((_, old)) if old == c => d.same += 1,
                Some(_) => d.changed += 1,
                None => d.added += 1,
            }
        }
        d.removed = count(
            self.pieces
                .iter()
                .filter(|(o, _)| !now.iter().any(|(k, _)| k == o))
                .count(),
        );
        let nested = nested_names(doc, id);
        d.nested_added = nested
            .iter()
            .filter(|n| !self.nested.contains(n))
            .cloned()
            .collect();
        d.nested_removed = self
            .nested
            .iter()
            .filter(|n| !nested.contains(n))
            .cloned()
            .collect();
        for (slot, kind, was) in self.tied {
            let Some(e) = doc.get(slot) else { continue };
            let Ok(is) = serde_json::to_value(e) else {
                continue;
            };
            if is == was {
                continue;
            }
            let effect = if broken_count(&is) > broken_count(&was) {
                XrefFollowEffect::Breaks
            } else if kind == XrefFollowerKind::Hatch
                && was.get("assoc").is_some_and(|a| !a.is_null())
                && is.get("assoc").is_none_or(Value::is_null)
            {
                XrefFollowEffect::Unties
            } else {
                XrefFollowEffect::Follows
            };
            let Some(uid) = doc.uid(slot) else { continue };
            d.followers.push(XrefFollower {
                uid: uid.to_string(),
                kind,
                effect,
            });
        }
        d
    }
}

/// The warning a diff gives when ties break, in its words; none when none breaks.
pub(crate) fn breaking_words(d: &XrefDiff, name: &str) -> Option<String> {
    let broken = d
        .followers
        .iter()
        .filter(|f| f.effect == XrefFollowEffect::Breaks)
        .count();
    let untied = d
        .followers
        .iter()
        .filter(|f| f.effect == XrefFollowEffect::Unties)
        .count();
    if broken + untied == 0 {
        return None;
    }
    let mut parts = Vec::new();
    if broken > 0 {
        parts.push(format!("{broken} nesnenin bağı kopuyor"));
    }
    if untied > 0 {
        parts.push(format!("{untied} tarama sınırını bırakıyor"));
    }
    Some(format!(
        "“{name}” kaynağında {} parça artık yok ya da değişti: {}. Kopan bağ yerinde kalır ve nedenini söyler; başka nesneye geçmez.",
        d.removed + d.changed,
        parts.join(", ")
    ))
}

/// Each of our nodes given its source's look (`looks`, docs/adr/1060 §5): its style (an undo step), shown, locked,
/// frozen and printed (the layer tree's own flags, as every layer's, docs/adr/0248 §2). The names of those that
/// changed, in the source's tree order.
pub(crate) fn reset_layers(doc: &mut Document, looks: &[(String, LayerNode)]) -> Vec<String> {
    let mut out = Vec::new();
    for (ours, theirs) in looks {
        let Some(now) = doc.layers().get(ours).cloned() else {
            continue;
        };
        let mut changed = false;
        if now.style != theirs.style {
            changed |= doc.set_layer_style(ours, theirs.style.clone(), "Katmanları kaynağa döndür");
        }
        if now.visible != theirs.visible {
            doc.set_layer_visible(ours, theirs.visible);
            changed = true;
        }
        if now.locked != theirs.locked {
            changed |= doc.set_layer_locked(ours, theirs.locked);
        }
        let frozen = theirs.frozen.unwrap_or(false);
        if now.frozen.unwrap_or(false) != frozen {
            changed |= doc.set_layer_frozen(ours, frozen);
        }
        let plot = theirs.plot.unwrap_or(true);
        if now.plot.unwrap_or(true) != plot {
            changed |= doc.set_layer_plot(ours, plot);
        }
        if changed {
            out.push(now.name.clone());
        }
    }
    out
}
