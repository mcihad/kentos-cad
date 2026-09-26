//! The open drawing: a native document (`kentos_domain::Document`,
//! docs/adr/0020) and the file it came from.
//!
//! Opening tells the file's kind by its content (docs/specs/kcad-v2.md §8):
//! a `.kcad` v2 (binary, `kentos-kcad`) opens with every object's persistent
//! id; a v1 (JSON, `DocumentSnapshotV1`) opens with ids derived from its
//! content (docs/adr/0014) and is kept read-only: Save asks where to write the
//! v2 file and never replaces the v1 original by itself (docs/adr/0025).
//! Saving writes v2 as the web does, so the desktop reads what the web saves
//! and the other way round. Every change goes through the document, with the
//! web's rules: undo, the dirty flag and the saved revision.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use kentos_contracts::{
    DocumentSnapshotV1, DocumentSnapshotV2, LayerNode, LayerNodeType, ProjectSettings,
};

/// A drawing and where it came from.
#[derive(Debug, Clone)]
pub struct Document {
    /// What the drawing holds and every change to it.
    pub model: kentos_domain::Document,
    pub path: Option<PathBuf>,
    /// Which opened drawing this is, unique while the program runs: a save
    /// that finishes after another drawing was opened applies to its own
    /// drawing only (CLAUDE.md §21.2).
    pub session: u64,
    /// Opened from a `.kcad` v1 file: Save asks where to write the v2 file
    /// instead of replacing the old one (docs/adr/0025).
    pub legacy: bool,
}

fn session() -> u64 {
    static OPENED: AtomicU64 = AtomicU64::new(0);
    OPENED.fetch_add(1, Ordering::Relaxed) + 1
}

impl Document {
    /// A drawing from a v1 snapshot; refused when the document cannot hold it
    /// (a repeated object id, an object on a layer the file does not have).
    /// With a path it came from a v1 file, which Save does not write over.
    pub fn new(snapshot: DocumentSnapshotV1, path: Option<PathBuf>) -> Result<Self, String> {
        Ok(Self {
            model: kentos_domain::Document::from_snapshot(snapshot)?,
            legacy: path.is_some(),
            path,
            session: session(),
        })
    }

    /// A drawing from a v2 snapshot, every object with the id the file gave it.
    pub fn from_v2(snapshot: DocumentSnapshotV2, path: Option<PathBuf>) -> Result<Self, String> {
        Ok(Self {
            model: kentos_domain::Document::from_snapshot_v2(snapshot)?,
            legacy: false,
            path,
            session: session(),
        })
    }

    /// Reads a `.kcad` file, v2 or v1, whatever its name says, at once (the
    /// app opens in stages, opening.rs); anything else is refused with the reason.
    pub fn read(path: &Path) -> Result<Self, String> {
        crate::opening::read(path, &AtomicBool::new(false), &mut |_| {})?
            .ok_or_else(|| format!("{}: açılış durduruldu.", path.display()))
    }

    /// A save of `revision` to `path` finished: the drawing is clean unless it
    /// changed meanwhile (CLAUDE.md §4.8), and it lives in a v2 file now.
    pub fn saved(&mut self, path: PathBuf, revision: u64) {
        self.path = Some(path);
        self.legacy = false;
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

/// Writes a drawing as a `.kcad` v2 file, as the web does (TODOS.md FILE-16):
/// the bytes are read back to the same drawing before anything touches the
/// disk (`encode_verified`); they go to a new temporary file beside the
/// target, which is flushed to the disk and read back, and only then renamed
/// over the target in one step. A save that fails anywhere leaves the
/// previous file as it was and removes the temporary one. At once; the app
/// saves off its UI thread with a panel and a stop (saving.rs); tests and
/// measurements write with this.
#[cfg(test)]
pub fn write(snapshot: &DocumentSnapshotV2, path: &Path) -> Result<(), String> {
    use crate::saving::{self, SaveError};
    saving::write_watched(
        snapshot,
        path,
        &AtomicBool::new(false),
        &mut |_| {},
        &saving::Faults::NONE,
    )
    .map_err(|e| match e {
        SaveError::Failed(why) => why,
        SaveError::Stopped => format!("{}: kayıt durduruldu.", path.display()),
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
    const KCAD: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/kcad/v2/");

    /// A fresh directory under the system's temporary one; never the user's files.
    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("kentos-desktop-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temporary directory");
        dir
    }

    fn uids(doc: &Document) -> Vec<kentos_domain::Uuid> {
        doc.model
            .entities()
            .map(|e| {
                doc.model
                    .uid(kentos_domain::Slot(e.base().id))
                    .expect("an id")
            })
            .collect()
    }

    // ── TODOS.md §9 acceptance: the desktop's part of the exchange with the web ──
    // (apps/web/src/app/kcadExchange.test.ts has the chain and the web's part).

    /// The persistent id the web gave its new point.
    const NEW_POINT: &str = "0192f5a0-7c3e-7000-8000-00000000e001";

    fn decode_fixture(name: &str) -> DocumentSnapshotV2 {
        let bytes = std::fs::read(format!("{KCAD}{name}")).expect("fixture");
        kentos_kcad::decode(&bytes).expect("reads")
    }

    /// A drawing as JSON without the objects' slots, which a file does not keep: equal means bit for bit.
    fn without_slots(doc: &DocumentSnapshotV2) -> serde_json::Value {
        let mut v = serde_json::to_value(doc).expect("serializes");
        for e in v["entities"].as_array_mut().expect("entities") {
            e.as_object_mut().expect("an object").remove("id");
        }
        v
    }

    fn index_of(s: &DocumentSnapshotV2, kind: &str) -> usize {
        s.entities
            .iter()
            .position(|e| e.kind() == kind)
            .expect("the kind")
    }

    fn layer_mut<'a>(nodes: &'a mut [LayerNode], id: &str) -> Option<&'a mut LayerNode> {
        for n in nodes {
            if n.id == id {
                return Some(n);
            }
            if let Some(found) = layer_mut(&mut n.children, id) {
                return Some(found);
            }
        }
        None
    }

    fn remove_at(s: &mut DocumentSnapshotV2, i: usize) {
        s.entities.remove(i);
        s.uids.remove(i);
    }

    /// The web's edits, applied by hand to the drawing before them: what the web must have saved.
    fn web_edits(mut s: DocumentSnapshotV2) -> DocumentSnapshotV2 {
        use kentos_contracts::{Entity, EntityBase, EntityId, PointEntity, Vec2};
        let i = index_of(&s, "point");
        if let Entity::Point(p) = &mut s.entities[i] {
            p.base.attrs = [("Ad".to_owned(), "P1-web".to_owned())].into();
        }
        let i = index_of(&s, "line");
        if let Entity::Line(l) = &mut s.entities[i] {
            l.b.x += 1.5;
        }
        let i = index_of(&s, "ellipse");
        remove_at(&mut s, i);
        s.entities.push(Entity::Point(PointEntity {
            base: EntityBase {
                id: 0,
                layer_id: "cizim".into(),
                color: None,
                attrs: [("Ad".to_owned(), "P2-web".to_owned())].into(),
                label: None,
                symbol: None,
            },
            p: Vec2 {
                x: 486_520.125,
                y: 4_420_195.75,
            },
            z: None,
        }));
        s.uids.push(EntityId::parse(NEW_POINT).expect("a UUID"));
        if let Some(n) = layer_mut(&mut s.layers, "bina") {
            n.locked = false;
        }
        s
    }

    /// The desktop's edits, applied by hand: what the desktop must have saved.
    fn desktop_edits(mut s: DocumentSnapshotV2) -> DocumentSnapshotV2 {
        use kentos_contracts::Entity;
        let i = index_of(&s, "polygon");
        if let Entity::Polygon(p) = &mut s.entities[i] {
            p.base.attrs.insert("Nitelik".into(), "Arsa".into());
        }
        let i = index_of(&s, "text");
        remove_at(&mut s, i);
        if let Some(n) = layer_mut(&mut s.layers, "cizim") {
            n.visible = true;
        }
        s
    }

    /// The desktop opens what the web saved of the desktop's drawing
    /// (`exchange/web-edited.kcad`), finds the web's edits and nothing else
    /// changed, edits it and saves it: `exchange/desktop-edited.kcad`, which the
    /// web reads back. `KENTOS_WRITE_EXCHANGE=1` writes that file instead of
    /// comparing it.
    #[test]
    fn the_desktop_reads_the_webs_edits_without_loss_and_saves_its_own() {
        use kentos_contracts::Entity;
        let dir = scratch("exchange");
        let from_web = dir.join("web-edited.kcad");
        std::fs::copy(format!("{KCAD}exchange/web-edited.kcad"), &from_web).expect("copies");
        let mut doc = Document::read(&from_web).expect("the web's file reads");
        assert!(!doc.legacy);
        let want = web_edits(decode_fixture("migrated.kcad"));
        assert_eq!(
            without_slots(&doc.model.to_snapshot_v2()),
            without_slots(&want)
        );

        let snapshot = doc.model.to_snapshot_v2();
        let slot = |kind: &str| {
            kentos_domain::Slot(snapshot.entities[index_of(&snapshot, kind)].base().id)
        };
        let parcel = slot("polygon");
        let mut polygon = doc.model.get(parcel).cloned().expect("the parcel");
        if let Entity::Polygon(p) = &mut polygon {
            p.base.attrs.insert("Nitelik".into(), "Arsa".into());
        }
        assert!(doc.model.update(parcel, polygon));
        assert_eq!(doc.model.remove(&[slot("text")]), 1);
        doc.model.toggle_layer_visible("cizim");

        let out = dir.join("desktop-edited.kcad");
        write(&doc.model.to_snapshot_v2(), &out).expect("writes");
        let bytes = std::fs::read(&out).expect("written");
        let back = kentos_kcad::decode(&bytes).expect("reads back");
        assert_eq!(
            without_slots(&back),
            without_slots(&desktop_edits(decode_fixture("exchange/web-edited.kcad")))
        );
        let committed = format!("{KCAD}exchange/desktop-edited.kcad");
        if std::env::var_os("KENTOS_WRITE_EXCHANGE").is_some() {
            std::fs::write(&committed, &bytes).expect("writes the fixture");
        }
        assert!(bytes == std::fs::read(&committed).expect("fixture"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_web_v1_file_opens_read_only_and_saves_as_the_reference_v2() {
        let dir = scratch("v1");
        let old = dir.join("örnek.kcad");
        std::fs::write(&old, DEMO).expect("a v1 file");
        let doc = Document::read(&old).expect("the web's v1 file reads");
        assert!(doc.legacy, "a v1 file is not written over");
        assert!(doc.layer_count() > 0);
        assert_eq!(doc.kinds().values().sum::<usize>(), doc.entity_count());
        assert_eq!(doc.entity_count(), 13);

        // Saved as v2: exactly the bytes the independent writer made of the same file
        // (fixtures/kcad/v2/migrated.kcad): ids, project id and source record included.
        let new = dir.join("örnek-v2.kcad");
        write(&doc.model.to_snapshot_v2(), &new).expect("writes");
        let bytes = std::fs::read(&new).expect("written");
        assert!(bytes == std::fs::read(format!("{KCAD}migrated.kcad")).expect("fixture"));
        assert_eq!(std::fs::read_to_string(&old).expect("still there"), DEMO);

        let again = Document::read(&new).expect("reads back");
        assert!(!again.legacy);
        assert_eq!(uids(&again), uids(&doc));
        assert_eq!(again.model.project_id(), doc.model.project_id());
        assert_eq!(again.model.migrated_from(), doc.model.migrated_from());
        assert_eq!(again.model.to_snapshot(), doc.model.to_snapshot());
        // No temporary file is left beside the saved one.
        let names: Vec<String> = std::fs::read_dir(&dir)
            .expect("lists")
            .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names.len(), 2, "{names:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_failed_save_leaves_the_previous_file_as_it_was() {
        let dir = scratch("failed");
        let path = dir.join("cizim.kcad");
        let doc = Document::new(DocumentSnapshotV1::from_json(DEMO).expect("reads"), None)
            .expect("opens");
        write(&doc.model.to_snapshot_v2(), &path).expect("writes");
        let good = std::fs::read(&path).expect("written");

        // The drawing cannot be encoded: nothing is written.
        let mut broken = doc.model.to_snapshot_v2();
        broken.origin.x = f64::NAN;
        let error = write(&broken, &path).expect_err("refused");
        assert!(error.contains("NaN"), "{error}");
        assert!(std::fs::read(&path).expect("still there") == good);

        // The disk refuses the last step (the target is a directory now): the temporary file
        // goes, the target stays.
        let blocked = dir.join("dolu.kcad");
        std::fs::create_dir_all(blocked.join("içerik")).expect("a directory in the way");
        let error = write(&doc.model.to_snapshot_v2(), &blocked).expect_err("refused");
        assert!(
            error.contains("Önceki dosya olduğu gibi duruyor"),
            "{error}"
        );
        assert!(blocked.join("içerik").is_dir());
        let left: Vec<String> = std::fs::read_dir(&dir)
            .expect("lists")
            .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(".yaziliyor"))
            .collect();
        assert!(left.is_empty(), "{left:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn what_is_not_a_drawing_is_refused_with_the_reason() {
        let dir = scratch("foreign");
        let cases = [
            ("bos.kcad", Vec::new(), "dosya boş"),
            (
                "cizim.dxf.kcad",
                b"  0\nSECTION\n".to_vec(),
                "İçe aktar ile açılır",
            ),
            (
                "bozuk.kcad",
                std::fs::read(format!("{KCAD}broken/bad-hash.kcad")).expect("fixture"),
                "SHA-256",
            ),
            (
                "stil.kcad",
                br#"{"format":"kentos-style","version":1}"#.to_vec(),
                "KentOS çizim dosyası değil",
            ),
        ];
        for (name, data, says) in cases {
            let path = dir.join(name);
            std::fs::write(&path, data).expect("written");
            let error = Document::read(&path).expect_err(name);
            assert!(error.contains(says), "{name}: {error}");
        }
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
        assert!(
            !doc.dirty(),
            "nothing was hidden: not an edit (docs/adr/0020)"
        );
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
