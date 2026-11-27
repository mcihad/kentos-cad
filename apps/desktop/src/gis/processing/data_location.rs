//! Konuma göre seç over data layers (docs/adr/9340). A data layer's rows are its source's, not the drawing's objects
//! (docs/adr/9101), so the tool did not see them: with only data layers on the map it found no object to try.
//!
//! When the tool's Seçilecek nesneler or Başvuru nesneleri take a data layer (Katman: a data layer; Görünen
//! katmanlar: a visible one; Tümü: every one; Seçili: the rows chosen in the layer's table), its rows are read from the
//! source (data_rows.rs: whole, off the interface's thread, kept for the drawing as it is) and put, in a copy of the
//! drawing, among its objects. The window counts the inputs on that copy and the tool runs there through
//! `cad.processing.run`, as every channel runs it. What it selects comes back by layer and key: the drawing's objects
//! to the drawing's selection, a data layer's rows to its table's selection, the table showing that layer and the map
//! outlining them. While a source is read Çalıştır waits; one that cannot be read is named.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

use kentos_commands::{CallOp, LayerNode};
use kentos_domain::Slot;
use kentos_expression::Variable;
use kentos_processing::{Bounds, FeaturesValue, Scene, Scope};
use serde_json::Value;

use super::dialog::RunStatus;
use crate::app::App;

/// The tool it answers for.
pub(crate) const TOOL: &str = "selection.byLocation";

/// The drawing with the data layers' rows among its objects, and which object is which row.
pub(crate) struct Located {
    pub doc: kentos_domain::Document,
    /// A row's object in the copy (its slot) → its layer and key.
    rows: HashMap<u32, (String, i64)>,
    /// The rows chosen in the table, as objects of the copy (a run's Seçili and Seçim biçimi start from them).
    chosen: Vec<Slot>,
    /// The layers still read, and those whose source could not be read (with why), by name.
    pub reading: Vec<String>,
    pub failed: Vec<(String, String)>,
}

/// What the kept copy is of: the drawing's session, its generation and its writes to the sources; each layer and
/// whether its rows were read; the table's chosen rows.
type Stamp = (u64, u64, u64, Vec<(String, bool)>, Vec<i64>);

/// The copy kept for the drawing as it is.
#[derive(Default)]
pub(crate) struct DataLocation {
    kept: RefCell<Option<(Stamp, Arc<Located>)>>,
}

/// The copy as the window and the run read it.
struct LocatedScene<'a> {
    located: &'a Located,
    selected: Vec<Slot>,
    view: Option<Bounds>,
    variables: Vec<Variable>,
}

impl Scene for LocatedScene<'_> {
    fn doc(&self) -> &kentos_domain::Document {
        &self.located.doc
    }

    fn selected(&self) -> Vec<Slot> {
        self.selected.clone()
    }

    fn visible_bounds(&self) -> Option<Bounds> {
        self.view
    }

    fn variables(&self) -> Vec<Variable> {
        self.variables.clone()
    }
}

impl App {
    /// The data layers Konuma göre seç's inputs take, in the tree's order; none for another tool.
    fn data_location_layers(&self) -> Vec<LayerNode> {
        let (Some(window), Some(doc)) = (self.gis.processing.dialog.as_ref(), self.document.as_ref()) else {
            return Vec::new();
        };
        if window.tool.id != TOOL {
            return Vec::new();
        }
        let tree = doc.model.layers();
        let data: Vec<&LayerNode> = tree.leaves().into_iter().filter(|l| l.data.is_some()).collect();
        let table = self.gis.features.data.selected_layer().map(str::to_owned);
        let mut taken: BTreeSet<String> = BTreeSet::new();
        for param in ["input", "reference"] {
            let Some(v) = window.values.get(param).and_then(FeaturesValue::read) else {
                continue;
            };
            match v.scope {
                Scope::All => taken.extend(data.iter().map(|l| l.id.clone())),
                Scope::Visible => taken.extend(data.iter().filter(|l| tree.is_visible(&l.id)).map(|l| l.id.clone())),
                Scope::Layer(id) if data.iter().any(|l| l.id == id) => {
                    taken.insert(id);
                }
                Scope::Selection => taken.extend(table.iter().filter(|t| data.iter().any(|l| &l.id == *t)).cloned()),
                _ => {}
            }
        }
        data.into_iter().filter(|l| taken.contains(&l.id)).cloned().collect()
    }

    /// The drawing with the data layers' rows among its objects, kept while nothing changes; none when the tool takes
    /// no data layer.
    pub(super) fn data_location(&self) -> Option<Arc<Located>> {
        let layers = self.data_location_layers();
        if layers.is_empty() {
            return None;
        }
        let doc = self.document.as_ref()?;
        let read: Vec<(LayerNode, Option<crate::data_rows::Rows>)> =
            layers.into_iter().map(|l| (l.clone(), self.data_rows_of(&l.id))).collect();
        let data = &self.gis.features.data;
        let chosen_of = data.selected_layer().map(str::to_owned).filter(|t| read.iter().any(|(l, _)| &l.id == t));
        let chosen_keys: Vec<i64> = chosen_of.as_ref().map(|_| data.selected.iter().copied().collect()).unwrap_or_default();
        let stamp: Stamp = (
            doc.session,
            doc.model.generation(),
            doc.model.data_written(),
            read.iter().map(|(l, r)| (l.id.clone(), matches!(r, Some(crate::data_rows::Rows::Ready(_))))).collect(),
            chosen_keys.clone(),
        );
        if let Some((at, kept)) = self.gis.processing.data_location.kept.borrow().as_ref()
            && *at == stamp
        {
            return Some(Arc::clone(kept));
        }
        let mut copy = doc.model.clone();
        let mut rows = HashMap::new();
        let (mut reading, mut failed) = (Vec::new(), Vec::new());
        for (node, r) in read {
            match r {
                Some(crate::data_rows::Rows::Ready(all)) => {
                    let held = doc.model.data_hidden(&node.id);
                    let (keys, objects): (Vec<i64>, Vec<kentos_commands::Entity>) =
                        all.iter().filter(|(k, _)| !held.contains(k)).map(|(k, e)| (*k, e.clone())).unzip();
                    let Ok(slots) = copy.add_many(objects, "Konuma göre seç") else {
                        failed.push((node.name.clone(), "çizimde yer kalmadı".to_owned()));
                        continue;
                    };
                    for (slot, key) in slots.into_iter().zip(keys) {
                        rows.insert(slot.0, (node.id.clone(), key));
                    }
                }
                Some(crate::data_rows::Rows::Failed(why)) => failed.push((node.name.clone(), why)),
                _ => reading.push(node.name.clone()),
            }
        }
        let chosen = match &chosen_of {
            Some(layer) => rows
                .iter()
                .filter(|(_, (l, k))| l == layer && chosen_keys.contains(k))
                .map(|(s, _)| Slot(*s))
                .collect(),
            None => Vec::new(),
        };
        let located = Arc::new(Located { doc: copy, rows, chosen, reading, failed });
        *self.gis.processing.data_location.kept.borrow_mut() = Some((stamp, Arc::clone(&located)));
        Some(located)
    }

    /// The window's inputs counted on the copy with the data layers' rows; a source still read or that cannot be read
    /// said on the inputs.
    pub(super) fn data_location_refresh(&mut self) {
        let Some(located) = self.data_location() else {
            return;
        };
        let mut selected = self.selection.ids().to_vec();
        selected.extend(located.chosen.iter().copied());
        let scene = LocatedScene {
            located: &located,
            selected,
            view: Some(self.viewport.camera.visible_bounds()),
            variables: self.expression_variables(),
        };
        let Some(window) = &mut self.gis.processing.dialog else {
            return;
        };
        window.refresh(&self.gis.processing.runner, &scene);
        let said = if !located.reading.is_empty() {
            Some(format!("Kaynaktan okunuyor: {}. Satırlar okununca sayılır.", located.reading.join(", ")))
        } else if !located.failed.is_empty() {
            let named: Vec<String> = located.failed.iter().map(|(n, why)| format!("“{n}” okunamadı: {why}")).collect();
            Some(format!("{}. O katmanın satırları denenmez.", named.join("; ")))
        } else {
            None
        };
        if let Some(said) = said {
            for param in ["input", "reference"] {
                if let Some(input) = window.inputs.get_mut(param) {
                    if !located.reading.is_empty() {
                        input.count = 0;
                    }
                    input.description = format!("{} {said}", input.description);
                }
            }
        }
    }

    /// Çalıştır with a data layer among the inputs: the tool run on the copy through `cad.processing.run`, its choice
    /// brought back by layer and key; none when it takes no data layer.
    pub(super) fn processing_data_location_run(&mut self) -> Option<()> {
        let located = self.data_location()?;
        let window = self.gis.processing.dialog.as_mut()?;
        if !located.reading.is_empty() {
            window.status = RunStatus::Invalid(format!(
                "Kaynaktan okunuyor: {}. Satırlar okununca çalıştırın.",
                located.reading.join(", ")
            ));
            return Some(());
        }
        let parameters: BTreeMap<String, Value> =
            window.values.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        let input_layer = window.values.get("input").and_then(FeaturesValue::read).and_then(|v| match v.scope {
            Scope::Layer(id) => Some(id),
            _ => None,
        });
        let mut copy = located.doc.clone();
        let uid = |s: &Slot| copy.uid(*s).map(|u| u.to_string());
        let mut selection: Vec<String> = self.selection.ids().iter().filter_map(uid).collect();
        selection.extend(located.chosen.iter().filter_map(uid));
        let b = self.viewport.camera.visible_bounds();
        let run = kentos_commands::ProcessingRun {
            tool: TOOL.to_owned(),
            parameters,
            selection,
            view: Some([b.min_x, b.min_y, b.max_x, b.max_y]),
            sink: None,
            drawing: None,
            environment: None,
            model: None,
        };
        let answer = kentos_native_application::registry::run(
            &mut kentos_native_application::ExecutionContext::new(&mut copy),
            kentos_commands::CAD_PROCESSING_RUN,
            CallOp::Execute,
            serde_json::to_value(&run).unwrap_or_default(),
        );
        let answer = match answer {
            Ok(a) if a["status"] == "completed" => a,
            Ok(a) => {
                let why = a["error"]["message"].as_str().unwrap_or("Konuma göre seç çalışmadı.").to_owned();
                if let Some(window) = &mut self.gis.processing.dialog {
                    window.status = RunStatus::Error(why);
                }
                return Some(());
            }
            Err(e) => {
                if let Some(window) = &mut self.gis.processing.dialog {
                    window.status = RunStatus::Error(format!("Konuma göre seç çalışmadı: {e:?}"));
                }
                return Some(());
            }
        };
        // Back by layer and key: the drawing's objects, and each data layer's rows.
        let mut drawn: Vec<Slot> = Vec::new();
        let mut by_layer: BTreeMap<String, BTreeSet<i64>> = BTreeMap::new();
        for u in answer["output"]["selected"].as_array().into_iter().flatten().filter_map(Value::as_str) {
            let Some(slot) = kentos_domain::Uuid::parse_str(u).ok().and_then(|u| copy.slot_of(u)) else {
                continue;
            };
            match located.rows.get(&slot.0) {
                Some((layer, key)) => {
                    by_layer.entry(layer.clone()).or_default().insert(*key);
                }
                None => drawn.push(slot),
            }
        }
        self.selection.set(drawn.clone());
        let name = |id: &str| copy.layers().get(id).map_or_else(|| id.to_owned(), |l| l.name.clone());
        // The table shows one layer: the input's, else the one with most rows chosen.
        let shown = input_layer
            .filter(|l| by_layer.contains_key(l))
            .or_else(|| by_layer.iter().max_by_key(|(_, k)| k.len()).map(|(l, _)| l.clone()));
        let rows: usize = by_layer.values().map(BTreeSet::len).sum();
        let mut said = vec![format!("{} nesne ilişkiyi sağladı", drawn.len() + rows)];
        match &shown {
            Some(layer) => {
                let keys = by_layer.get(layer).cloned().unwrap_or_default();
                let listed: Vec<String> = keys.iter().take(10).map(i64::to_string).collect();
                said.push(format!(
                    "“{}” katmanında {} satır tabloda seçildi (anahtar: {}{})",
                    name(layer),
                    keys.len(),
                    listed.join(", "),
                    if keys.len() > 10 { " …" } else { "" }
                ));
                self.gis.features.data.choose_rows(layer, keys);
                self.open_feature_table();
                let _ = self.features_event(crate::features::Event::Layer(layer.clone()));
            }
            None => {
                // Nothing of a data layer: the table's earlier choice goes with a new selection.
                if let Some(layer) = self.gis.features.data.selected_layer().map(str::to_owned) {
                    self.gis.features.data.choose_rows(&layer, BTreeSet::new());
                }
            }
        }
        if !drawn.is_empty() {
            said.push(format!("çizimde {} nesne seçildi", drawn.len()));
        }
        for (layer, keys) in by_layer.iter().filter(|(l, _)| Some(*l) != shown.as_ref()) {
            said.push(format!(
                "“{}” katmanında {} satır da sağladı (tabloda görmek için Seçilecek nesneler'de o katmanı seçin)",
                name(layer),
                keys.len()
            ));
        }
        if let Some(window) = &mut self.gis.processing.dialog {
            window.status = RunStatus::Ok {
                text: format!("{}.", said.join("; ")),
                pick: Vec::new(),
                selected: true,
                undo: false,
                table: None,
                warning: drawn.is_empty() && rows == 0,
            };
        }
        Some(())
    }
}

#[cfg(test)]
#[path = "data_location_tests.rs"]
mod data_location_tests;
