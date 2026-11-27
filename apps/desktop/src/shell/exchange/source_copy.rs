//! The drawing as the file exports write it (docs/adr/9344): every data layer's rows, as the tables read them
//! (data_rows.rs: whole, off the interface's thread, kept for the drawing as it is), put among the drawing's objects on
//! their layers, the layers' links taken off. GeoPackage, Shapefile, CSV, FlatGeobuf, GeoParquet and GML dışa aktar
//! count and write on it, so a project of data layers writes its rows and not empty tables. The rows the drawing holds
//! (taken or new in the edit session) are its objects already and are not put in twice. While a source is read the
//! windows say so; a source that cannot be read is named and its layer written empty.
//!
//! The copy is made again only when the drawing changes or a source's rows come; the objects keep their slots and
//! lasting ids, so the drawing's selection names the same objects in it, and a chosen row is found by its layer and key.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use kentos_commands::Entity;
use kentos_domain::Slot;

use crate::app::App;

/// The drawing with its data layers' rows.
pub(crate) struct SourceCopy {
    pub doc: kentos_domain::Document,
    /// A row's object in the copy (its slot) → its layer and key.
    pub rows: HashMap<u32, (String, i64)>,
    /// The data layers still read, and those whose source could not be read (with why), by name.
    pub reading: Vec<String>,
    pub failed: Vec<(String, String)>,
}

impl SourceCopy {
    /// The lasting ids of a layer's rows by their keys.
    pub(crate) fn uids_of(&self, layer: &str, keys: &[i64]) -> Vec<String> {
        self.rows
            .iter()
            .filter(|(_, (l, k))| l == layer && keys.contains(k))
            .filter_map(|(slot, _)| self.doc.uid(Slot(*slot)).map(|u| u.to_string()))
            .collect()
    }
}

/// What the kept copy is of: the drawing's session, generation and writes to the sources, and each data layer's
/// source with whether its rows were read.
type Stamp = (u64, u64, u64, Vec<(String, String, bool)>);

thread_local! {
    static KEPT: RefCell<Option<(Stamp, Rc<SourceCopy>)>> = const { RefCell::new(None) };
}

impl App {
    /// The drawing with every data layer's rows as objects of their layers, links taken off; none without a data layer.
    pub(crate) fn source_copy(&self) -> Option<Rc<SourceCopy>> {
        let doc = self.document.as_ref()?;
        let nodes: Vec<kentos_commands::LayerNode> =
            doc.model.layers().leaves().into_iter().filter(|l| l.data.is_some()).cloned().collect();
        if nodes.is_empty() {
            return None;
        }
        let read: Vec<Option<crate::data_rows::Rows>> = nodes.iter().map(|n| self.data_rows_of(&n.id)).collect();
        let stamp: Stamp = (
            doc.session,
            doc.model.generation(),
            doc.model.data_written(),
            nodes
                .iter()
                .zip(&read)
                .map(|(n, r)| {
                    let address = n.data.as_ref().map(|d| format!("{}\u{0}{}", d.address, d.table)).unwrap_or_default();
                    (n.id.clone(), address, matches!(r, Some(crate::data_rows::Rows::Ready(_))))
                })
                .collect(),
        );
        if let Some(kept) = KEPT.with(|k| k.borrow().as_ref().filter(|(at, _)| *at == stamp).map(|(_, c)| Rc::clone(c))) {
            return Some(kept);
        }
        // The links taken off first: the copy's layers are its own, as an exported file's are.
        let mut snapshot = doc.model.to_snapshot_v2();
        unlink(&mut snapshot.layers);
        let mut copy = kentos_domain::Document::from_snapshot_v2(snapshot).ok()?;
        let mut rows = HashMap::new();
        let (mut reading, mut failed) = (Vec::new(), Vec::new());
        for (node, r) in nodes.iter().zip(read) {
            match r {
                Some(crate::data_rows::Rows::Ready(all)) => {
                    let held = doc.model.data_hidden(&node.id);
                    let (keys, objects): (Vec<i64>, Vec<Entity>) =
                        all.iter().filter(|(k, _)| !held.contains(k)).map(|(k, e)| (*k, e.clone())).unzip();
                    match copy.add_many(objects, "Dışa aktar") {
                        Ok(slots) => {
                            for (slot, key) in slots.into_iter().zip(keys) {
                                rows.insert(slot.0, (node.id.clone(), key));
                            }
                        }
                        Err(_) => failed.push((node.name.clone(), "çizimde yer kalmadı".to_owned())),
                    }
                }
                Some(crate::data_rows::Rows::Failed(why)) => failed.push((node.name.clone(), why)),
                _ => reading.push(node.name.clone()),
            }
        }
        let made = Rc::new(SourceCopy { doc: copy, rows, reading, failed });
        KEPT.with(|k| *k.borrow_mut() = Some((stamp, Rc::clone(&made))));
        Some(made)
    }

    /// An export's refusal while a source is still read: said as waiting, not as nothing to write.
    pub(crate) fn while_read(&self, why: String) -> String {
        match self.source_copy() {
            Some(c) if !c.reading.is_empty() => {
                format!("Kaynaktan okunuyor: {}. Satırları okununca sayılır.", c.reading.join(", "))
            }
            _ => why,
        }
    }

    /// What an export window says of the sources: read ones nothing, one still read or that cannot be read by name.
    pub(crate) fn source_copy_words(&self) -> Vec<(bool, String)> {
        let Some(c) = self.source_copy() else {
            return Vec::new();
        };
        let mut out = Vec::new();
        if !c.reading.is_empty() {
            out.push((false, format!("Kaynaktan okunuyor: {}. Satırları okununca sayılır.", c.reading.join(", "))));
        }
        for (name, why) in &c.failed {
            out.push((true, format!("“{name}” katmanının kaynağı okunamadı: {why}. Bu katmanın satırları yazılmaz.")));
        }
        out
    }
}

/// The layers' links to their sources taken off, groups' children too.
fn unlink(layers: &mut [kentos_commands::LayerNode]) {
    for l in layers {
        l.data = None;
        unlink(&mut l.children);
    }
}
