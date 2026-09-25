//! The open drawing: a `.kcad` v1 snapshot (`DocumentSnapshotV1`, ADR 0002)
//! read and written through the shared contracts, so the desktop saves
//! exactly what the web writes and reads what it saves (ADR 0011 keeps v1
//! readable when the binary format comes).
//!
//! This first shell does not edit geometry. What it may change is part of the
//! file (layer visibility, lock, fold) and marks the drawing unsaved.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use kentos_contracts::{DocumentSnapshotV1, LayerNode, LayerNodeType};

/// A drawing, where it came from, and whether it changed since.
#[derive(Debug, Clone)]
pub struct Document {
    pub snapshot: DocumentSnapshotV1,
    pub path: Option<PathBuf>,
    pub dirty: bool,
    /// Grows with every change. A save clears `dirty` only for the revision
    /// it wrote: a change made while saving stays unsaved (CLAUDE.md §4.8).
    pub revision: u64,
    /// Object count per layer id.
    counts: HashMap<String, usize>,
}

impl Document {
    pub fn new(snapshot: DocumentSnapshotV1, path: Option<PathBuf>) -> Self {
        let mut counts = HashMap::new();
        for entity in &snapshot.entities {
            *counts.entry(entity.base().layer_id.clone()).or_insert(0) += 1;
        }
        Self {
            snapshot,
            path,
            dirty: false,
            revision: 0,
            counts,
        }
    }

    /// Reads a `.kcad` file; the reader refuses other formats and versions.
    pub fn read(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("{} okunamadı: {e}", path.display()))?;
        let snapshot =
            DocumentSnapshotV1::from_json(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(Self::new(snapshot, Some(path.to_path_buf())))
    }

    /// Writes the snapshot as the web does: one line of JSON and a newline.
    /// Written to a temporary file first and renamed over the old one, so a
    /// failed save never leaves a half-written drawing (TODOS.md FILE-16).
    pub fn write(&self, path: &Path) -> Result<(), String> {
        let text =
            serde_json::to_string(&self.snapshot).map_err(|e| format!("çizim yazılamadı: {e}"))?;
        let temporary = path.with_extension("kcad.yaziliyor");
        std::fs::write(&temporary, format!("{text}\n"))
            .and_then(|()| std::fs::rename(&temporary, path))
            .map_err(|e| {
                let _ = std::fs::remove_file(&temporary);
                format!("{} yazılamadı: {e}", path.display())
            })
    }

    /// A save of `revision` to `path` finished.
    pub fn saved(&mut self, path: PathBuf, revision: u64) {
        self.path = Some(path);
        if revision == self.revision {
            self.dirty = false;
        }
    }

    pub fn name(&self) -> &str {
        &self.snapshot.name
    }

    pub fn count(&self, layer: &str) -> usize {
        self.counts.get(layer).copied().unwrap_or(0)
    }

    /// Objects under a layer or group, its children included.
    pub fn count_below(&self, node: &LayerNode) -> usize {
        match node.kind {
            LayerNodeType::Layer => self.count(&node.id),
            LayerNodeType::Group => node.children.iter().map(|c| self.count_below(c)).sum(),
        }
    }

    /// Object count per kind (`polygon`, `text` …), sorted by kind.
    pub fn kinds(&self) -> BTreeMap<&'static str, usize> {
        let mut kinds = BTreeMap::new();
        for entity in &self.snapshot.entities {
            *kinds.entry(entity.kind()).or_insert(0) += 1;
        }
        kinds
    }

    pub fn layer_count(&self) -> usize {
        fn leaves(nodes: &[LayerNode]) -> usize {
            nodes
                .iter()
                .map(|n| match n.kind {
                    LayerNodeType::Layer => 1,
                    LayerNodeType::Group => leaves(&n.children),
                })
                .sum()
        }
        leaves(&self.snapshot.layers)
    }

    pub fn find(&self, id: &str) -> Option<&LayerNode> {
        find(&self.snapshot.layers, id)
    }

    /// Applies `change` to the layer or group `id`; the drawing becomes unsaved.
    pub fn change_layer(&mut self, id: &str, change: impl FnOnce(&mut LayerNode)) {
        if let Some(node) = find_mut(&mut self.snapshot.layers, id) {
            change(node);
            self.dirty = true;
            self.revision += 1;
        }
    }

    /// Shows every layer and group (web: Tüm katmanları göster).
    pub fn show_all(&mut self) {
        fn show(nodes: &mut [LayerNode]) -> bool {
            let mut changed = false;
            for node in nodes {
                changed |= !node.visible;
                node.visible = true;
                changed |= show(&mut node.children);
            }
            changed
        }
        if show(&mut self.snapshot.layers) {
            self.dirty = true;
            self.revision += 1;
        }
    }
}

fn find<'a>(nodes: &'a [LayerNode], id: &str) -> Option<&'a LayerNode> {
    nodes.iter().find_map(|n| {
        if n.id == id {
            Some(n)
        } else {
            find(&n.children, id)
        }
    })
}

fn find_mut<'a>(nodes: &'a mut [LayerNode], id: &str) -> Option<&'a mut LayerNode> {
    for node in nodes {
        if node.id == id {
            return Some(node);
        }
        if let Some(found) = find_mut(&mut node.children, id) {
            return Some(found);
        }
    }
    None
}

/// The name of an EPSG code, from the shared CRS registry the web also reads
/// (fixtures/crs/v1/registry.json).
pub fn crs_name(srid: u32) -> Option<&'static str> {
    use std::sync::OnceLock;
    static NAMES: OnceLock<HashMap<u32, &'static str>> = OnceLock::new();
    NAMES
        .get_or_init(|| {
            #[derive(serde::Deserialize)]
            struct Registry {
                systems: Vec<System>,
            }
            #[derive(serde::Deserialize)]
            struct System {
                srid: u32,
                name: String,
            }
            serde_json::from_str::<Registry>(include_str!("../../../fixtures/crs/v1/registry.json"))
                .map(|r| {
                    r.systems
                        .into_iter()
                        .map(|s| (s.srid, &*Box::leak(s.name.into_boxed_str())))
                        .collect()
                })
                .unwrap_or_default()
        })
        .get(&srid)
        .copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEMO: &str = include_str!("../../../fixtures/document/v1/sample.json");

    #[test]
    fn a_web_file_is_read_counted_and_written_back_unchanged() {
        let snapshot = DocumentSnapshotV1::from_json(DEMO).expect("the web's demo file reads");
        let doc = Document::new(snapshot, None);
        assert!(doc.layer_count() > 0);
        assert_eq!(
            doc.kinds().values().sum::<usize>(),
            doc.snapshot.entities.len()
        );

        let dir = std::env::temp_dir().join(format!("kentos-desktop-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temporary directory");
        let path = dir.join("cizim.kcad");
        doc.write(&path).expect("writes");
        let again = Document::read(&path).expect("reads back");
        assert_eq!(again.snapshot, doc.snapshot);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn showing_all_layers_marks_the_drawing_unsaved_only_when_something_was_hidden() {
        let snapshot = DocumentSnapshotV1::from_json(DEMO).expect("reads");
        let mut doc = Document::new(snapshot, None);
        doc.show_all();
        let was_hidden = doc.dirty;
        doc.dirty = false;
        doc.show_all();
        assert!(!doc.dirty, "nothing was hidden the second time");
        let first = doc
            .snapshot
            .layers
            .first()
            .map(|n| n.id.clone())
            .expect("a layer");
        doc.change_layer(&first, |n| n.visible = false);
        assert!(doc.dirty);
        let _ = was_hidden;
    }

    #[test]
    fn the_default_system_has_a_name() {
        assert_eq!(crs_name(5256), Some("TUREF / TM36"));
    }
}
