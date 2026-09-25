//! The open drawing: a native document (`kentos_domain::Document`,
//! docs/adr/0020) and the file it came from.
//!
//! Opening reads a `.kcad` v1 snapshot (`DocumentSnapshotV1`, ADR 0002)
//! through the shared contracts into the document; saving writes the
//! document's snapshot as the web writes it, so the desktop reads what the
//! web saves and the other way round (ADR 0011 keeps v1 readable when the
//! binary format comes). Every change goes through the document, with the
//! web's rules: undo, the dirty flag and the saved revision.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use kentos_contracts::{DocumentSnapshotV1, LayerNode, LayerNodeType, ProjectSettings};

/// A drawing and where it came from.
#[derive(Debug, Clone)]
pub struct Document {
    /// What the drawing holds and every change to it.
    pub model: kentos_domain::Document,
    pub path: Option<PathBuf>,
}

impl Document {
    /// A drawing from a snapshot; refused when the document cannot hold it
    /// (a repeated object id, an object on a layer the file does not have).
    pub fn new(snapshot: DocumentSnapshotV1, path: Option<PathBuf>) -> Result<Self, String> {
        Ok(Self {
            model: kentos_domain::Document::from_snapshot(snapshot)?,
            path,
        })
    }

    /// Reads a `.kcad` file; the reader refuses other formats and versions.
    pub fn read(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("{} okunamadı: {e}", path.display()))?;
        let snapshot =
            DocumentSnapshotV1::from_json(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        Self::new(snapshot, Some(path.to_path_buf()))
            .map_err(|e| format!("{}: {e}", path.display()))
    }

    /// A save of `revision` to `path` finished: the drawing is clean unless it
    /// changed meanwhile (CLAUDE.md §4.8).
    pub fn saved(&mut self, path: PathBuf, revision: u64) {
        self.path = Some(path);
        self.model.mark_saved(revision);
    }

    pub fn name(&self) -> &str {
        self.model.name()
    }

    /// Whether the drawing has changes no save has written.
    pub fn dirty(&self) -> bool {
        self.model.is_dirty()
    }

    pub fn entity_count(&self) -> usize {
        self.model.len()
    }

    /// The top of the layer tree.
    pub fn layers(&self) -> &[LayerNode] {
        self.model.layers().nodes()
    }

    pub fn settings(&self) -> &ProjectSettings {
        self.model.settings()
    }

    pub fn count(&self, layer: &str) -> usize {
        self.model.count(layer)
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
        for entity in self.model.entities() {
            *kinds.entry(entity.kind()).or_insert(0) += 1;
        }
        kinds
    }

    pub fn layer_count(&self) -> usize {
        self.model.layers().leaves().len()
    }

    pub fn find(&self, id: &str) -> Option<&LayerNode> {
        self.model.layers().get(id)
    }
}

/// Writes a drawing as the web does: one line of JSON and a newline. Written
/// to a temporary file first and renamed over the old one, so a failed save
/// never leaves a half-written drawing (TODOS.md FILE-16).
pub fn write(snapshot: &DocumentSnapshotV1, path: &Path) -> Result<(), String> {
    let text = serde_json::to_string(snapshot).map_err(|e| format!("çizim yazılamadı: {e}"))?;
    let temporary = path.with_extension("kcad.yaziliyor");
    std::fs::write(&temporary, format!("{text}\n"))
        .and_then(|()| std::fs::rename(&temporary, path))
        .map_err(|e| {
            let _ = std::fs::remove_file(&temporary);
            format!("{} yazılamadı: {e}", path.display())
        })
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
        let doc = Document::new(snapshot.clone(), None).expect("the document holds it");
        assert!(doc.layer_count() > 0);
        assert_eq!(doc.kinds().values().sum::<usize>(), doc.entity_count());
        assert_eq!(doc.entity_count(), snapshot.entities.len());

        let dir = std::env::temp_dir().join(format!("kentos-desktop-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temporary directory");
        let path = dir.join("cizim.kcad");
        write(&doc.model.to_snapshot(), &path).expect("writes");
        // Byte for byte what writing the file's own snapshot gives: nothing lost or reordered.
        assert_eq!(
            std::fs::read_to_string(&path).expect("written"),
            format!(
                "{}\n",
                serde_json::to_string(&snapshot).expect("serializes")
            )
        );
        let again = Document::read(&path).expect("reads back");
        assert_eq!(again.model.to_snapshot(), snapshot);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// As on the web (fixtures/document-ops/v1/layers.json): showing every
    /// layer is an edit even when all were shown, and not an undo step.
    #[test]
    fn showing_all_layers_is_an_edit_but_not_undoable() {
        let snapshot = DocumentSnapshotV1::from_json(DEMO).expect("reads");
        let mut doc = Document::new(snapshot, None).expect("opens");
        let hidden = doc
            .layers()
            .iter()
            .find(|n| !n.visible)
            .map(|n| n.id.clone())
            .expect("the demo has a hidden layer");
        doc.model.show_all_layers();
        assert!(doc.dirty());
        assert!(doc.find(&hidden).is_some_and(|n| n.visible));
        assert!(!doc.model.can_undo());

        let revision = doc.model.revision();
        doc.saved(PathBuf::from("cizim.kcad"), revision);
        assert!(!doc.dirty());
        doc.model.show_all_layers();
        assert!(doc.dirty(), "nothing was hidden, still an edit (web)");
    }

    #[test]
    fn a_save_of_an_older_revision_leaves_the_drawing_unsaved() {
        let snapshot = DocumentSnapshotV1::from_json(DEMO).expect("reads");
        let mut doc = Document::new(snapshot, None).expect("opens");
        let layer = doc.layers()[0].id.clone();
        doc.model.toggle_layer_locked(&layer);
        let writing = doc.model.revision();
        doc.model.toggle_layer_locked(&layer);
        doc.saved(PathBuf::from("cizim.kcad"), writing);
        assert!(
            doc.dirty(),
            "the change made while saving is not in the file"
        );
        assert_eq!(doc.path.as_deref(), Some(Path::new("cizim.kcad")));
        let current = doc.model.revision();
        doc.saved(PathBuf::from("cizim.kcad"), current);
        assert!(!doc.dirty());
    }

    #[test]
    fn the_default_system_has_a_name() {
        assert_eq!(crs_name(5256), Some("TUREF / TM36"));
    }
}
