//! Konuma göre seç and İfadeyle seç over data layers (docs/adr/9340, 9345). A data layer's rows are its source's, not
//! the drawing's objects (docs/adr/9101), so the tools did not see them: with only data layers on the map they found no
//! object to try. (İfadeyle seç on one data layer, Katman, asks its source the condition instead: data_select.rs.)
//!
//! When the tool's Seçilecek nesneler or Başvuru nesneleri take a data layer (Katman: a data layer; Görünen
//! katmanlar: a visible one; Tümü: every one; Seçili: the rows chosen in the layer's table), its rows are read from the
//! source (data_rows.rs: whole, off the interface's thread, kept for the drawing as it is) and put, in a copy of the
//! drawing, among its objects. The window counts the inputs on that copy and the tool runs there through
//! `cad.processing.run`, as every channel runs it. What it selects comes back by layer and key: the drawing's objects
//! to the drawing's selection, a data layer's rows to its table's selection, the table showing that layer and the map
//! outlining them. While a source is read Çalıştır waits; one that cannot be read is named.
//!
//! The inputs' lines name the data layers they take, a layer whose short name another shares by its group's path; for
//! İfadeyle seç they name the layers without a field the condition reads (there its value is empty). The result is kept
//! by layer (docs/adr/9345): the card under the form offers each layer's matches, and choosing one shows them in the
//! table without running again.

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

/// The tools it answers for.
pub(crate) const TOOL: &str = "selection.byLocation";
pub(crate) const EXPRESSION: &str = "selection.byExpression";

/// The features parameters a tool's run reads over data layers: the selecting tools' named ones, every other tool's
/// features parameters (data_inputs.rs, docs/adr/9353).
fn inputs_of(tool: &kentos_processing::Tool) -> Vec<String> {
    match tool.id.as_str() {
        TOOL => vec!["input".into(), "reference".into()],
        EXPRESSION => vec!["input".into()],
        id if id.starts_with(kentos_processing::model_runner::MODEL_PREFIX) => Vec::new(),
        _ => tool
            .parameters
            .iter()
            .filter(|p| matches!(p.kind, kentos_processing::ParamKind::Features { .. }))
            .map(|p| p.name.clone())
            .collect(),
    }
}

/// One layer's matches of the last run: its id, how it is named, its rows' keys.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LayerResult {
    pub layer: String,
    pub label: String,
    pub keys: BTreeSet<i64>,
}

/// The last run's matches by layer (docs/adr/9345): the card under the form offers them, the table shows one.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LayerResults {
    /// What the tool says of its matches (“ilişkiyi sağladı”, “koşulu sağladı”).
    verb: &'static str,
    /// The drawing's objects among them, selected in the drawing.
    pub drawn: usize,
    pub layers: Vec<LayerResult>,
    /// The layer the table shows.
    pub shown: Option<String>,
}

impl LayerResults {
    /// The status line: how many matched, by layer when more than one, and what the table shows.
    pub(crate) fn said(&self) -> String {
        let rows: usize = self.layers.iter().map(|l| l.keys.len()).sum();
        let mut out = format!("{} nesne {}", self.drawn + rows, self.verb);
        let shown = self
            .shown
            .as_ref()
            .and_then(|s| self.layers.iter().find(|l| &l.layer == s));
        let keys = |l: &LayerResult| {
            let listed: Vec<String> = l.keys.iter().take(10).map(i64::to_string).collect();
            format!(
                "{}{}",
                listed.join(", "),
                if l.keys.len() > 10 { " …" } else { "" }
            )
        };
        if self.layers.len() > 1 {
            let each: Vec<String> = self
                .layers
                .iter()
                .map(|l| format!("“{}” {}", l.label, l.keys.len()))
                .collect();
            out.push_str(&format!(": {}", each.join(", ")));
            if let Some(l) = shown {
                out.push_str(&format!(
                    ". Tabloda “{}”: {} satır seçili (anahtar: {})",
                    l.label,
                    l.keys.len(),
                    keys(l)
                ));
            }
            out.push_str(
                ". Öbür katmanınkini formun başından tabloda açın; işlem yeniden çalışmaz",
            );
        } else if let Some(l) = shown {
            out.push_str(&format!(
                "; “{}” katmanında {} satır tabloda seçildi (anahtar: {})",
                l.label,
                l.keys.len(),
                keys(l)
            ));
        }
        if self.drawn > 0 {
            out.push_str(&format!("; çizimde {} nesne seçildi", self.drawn));
        }
        out.push('.');
        out
    }
}

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
    /// Each data layer read: how it is named (its group's path when another shares its short name), its rows, and its
    /// fields (the layer's own and those its rows hold).
    taken: Vec<(String, String, BTreeSet<String>)>,
}

impl Located {
    /// The rows chosen in the table, as objects of the copy.
    pub(super) fn chosen(&self) -> &[Slot] {
        &self.chosen
    }
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
    /// The data layers the tool's inputs take, in the tree's order; none for another tool, and for İfadeyle seç on one
    /// data layer (its source is asked: data_select.rs).
    fn data_location_layers(&self) -> Vec<LayerNode> {
        let (Some(window), Some(doc)) =
            (self.gis.processing.dialog.as_ref(), self.document.as_ref())
        else {
            return Vec::new();
        };
        let tree = doc.model.layers();
        let data: Vec<&LayerNode> = tree
            .leaves()
            .into_iter()
            .filter(|l| l.data.is_some())
            .collect();
        let table = self.gis.features.data.selected_layer().map(str::to_owned);
        let mut taken: BTreeSet<String> = BTreeSet::new();
        for param in inputs_of(&window.tool) {
            let Some(v) = window.values.get(&param).and_then(FeaturesValue::read) else {
                continue;
            };
            match v.scope {
                Scope::All => taken.extend(data.iter().map(|l| l.id.clone())),
                Scope::Visible => taken.extend(
                    data.iter()
                        .filter(|l| tree.is_visible(&l.id))
                        .map(|l| l.id.clone()),
                ),
                Scope::Layer(id)
                    if window.tool.id != EXPRESSION && data.iter().any(|l| l.id == id) =>
                {
                    taken.insert(id);
                }
                Scope::Selection => taken.extend(
                    table
                        .iter()
                        .filter(|t| data.iter().any(|l| &l.id == *t))
                        .cloned(),
                ),
                _ => {}
            }
        }
        data.into_iter()
            .filter(|l| taken.contains(&l.id))
            .cloned()
            .collect()
    }

    /// The drawing with the data layers' rows among its objects, kept while nothing changes; none when the tool takes
    /// no data layer.
    pub(super) fn data_location(&self) -> Option<Arc<Located>> {
        let layers = self.data_location_layers();
        if layers.is_empty() {
            return None;
        }
        let doc = self.document.as_ref()?;
        let read: Vec<(LayerNode, Option<crate::data_rows::Rows>)> = layers
            .into_iter()
            .map(|l| (l.clone(), self.data_rows_of(&l.id)))
            .collect();
        let data = &self.gis.features.data;
        let chosen_of = data
            .selected_layer()
            .map(str::to_owned)
            .filter(|t| read.iter().any(|(l, _)| &l.id == t));
        let chosen_keys: Vec<i64> = chosen_of
            .as_ref()
            .map(|_| data.selected.iter().copied().collect())
            .unwrap_or_default();
        let stamp: Stamp = (
            doc.session,
            doc.model.generation(),
            doc.model.data_written(),
            read.iter()
                .map(|(l, r)| {
                    (
                        l.id.clone(),
                        matches!(r, Some(crate::data_rows::Rows::Ready(_))),
                    )
                })
                .collect(),
            chosen_keys.clone(),
        );
        if let Some((at, kept)) = self.gis.processing.data_location.kept.borrow().as_ref()
            && *at == stamp
        {
            return Some(Arc::clone(kept));
        }
        let mut copy = doc.model.clone();
        let mut rows = HashMap::new();
        let (mut reading, mut failed, mut taken) = (Vec::new(), Vec::new(), Vec::new());
        for (node, r) in read {
            match r {
                Some(crate::data_rows::Rows::Ready(all)) => {
                    let held = doc.model.data_hidden(&node.id);
                    let (keys, objects): (Vec<i64>, Vec<kentos_commands::Entity>) = all
                        .iter()
                        .filter(|(k, _)| !held.contains(k))
                        .map(|(k, e)| (*k, e.clone()))
                        .unzip();
                    let mut fields: BTreeSet<String> =
                        node.fields.iter().map(|f| f.name.clone()).collect();
                    fields.extend(objects.iter().flat_map(|e| e.base().attrs.keys().cloned()));
                    // What waits in the edit session, said beside the source's rows (docs/adr/9353).
                    let c = doc.model.data_changes(&node.id);
                    let waiting = c.added.len() + c.changed.len() + c.deleted.len();
                    let count = if waiting > 0 {
                        format!(
                            "{} kaynakta, bekleyen {}",
                            objects.len(),
                            crate::data_edits::counts_words(
                                c.added.len(),
                                c.changed.len(),
                                c.deleted.len()
                            )
                        )
                    } else {
                        objects.len().to_string()
                    };
                    taken.push((layer_label(doc.model.layers(), &node.id), count, fields));
                    let Ok(slots) = copy.add_many(objects, "Seçim") else {
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
        let located = Arc::new(Located {
            doc: copy,
            rows,
            chosen,
            reading,
            failed,
            taken,
        });
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
        let scope = scope_words(&located, &window.tool.id, window.values.get("condition"));
        if let Some(scope) = scope {
            for param in inputs_of(&window.tool) {
                let takes_data = window
                    .values
                    .get(&param)
                    .and_then(FeaturesValue::read)
                    .is_some_and(|v| {
                        matches!(
                            v.scope,
                            Scope::All | Scope::Visible | Scope::Selection | Scope::Layer(_)
                        )
                    });
                if takes_data && let Some(input) = window.inputs.get_mut(&param) {
                    input.description = sentence_after(&input.description, &scope);
                }
            }
        }
        let said = if !located.reading.is_empty() {
            Some(format!(
                "Kaynaktan okunuyor: {}. Satırlar okununca sayılır.",
                located.reading.join(", ")
            ))
        } else if !located.failed.is_empty() {
            let named: Vec<String> = located
                .failed
                .iter()
                .map(|(n, why)| format!("“{n}” okunamadı: {why}"))
                .collect();
            Some(format!(
                "{}. O katmanın satırları denenmez.",
                named.join("; ")
            ))
        } else {
            None
        };
        if let Some(said) = said {
            for param in inputs_of(&window.tool) {
                if let Some(input) = window.inputs.get_mut(&param) {
                    if !located.reading.is_empty() {
                        input.count = 0;
                    }
                    input.description = sentence_after(&input.description, &said);
                }
            }
        }
    }

    /// Çalıştır with a data layer among the inputs: the tool run on the copy through `cad.processing.run`, its choice
    /// brought back by layer and key and kept by layer; none when it takes no data layer.
    pub(super) fn processing_data_location_run(&mut self) -> Option<()> {
        // Another tool writes its result into the drawing: data_inputs.rs.
        if !matches!(
            self.gis.processing.dialog.as_ref()?.tool.id.as_str(),
            TOOL | EXPRESSION
        ) {
            return None;
        }
        let located = self.data_location()?;
        let window = self.gis.processing.dialog.as_mut()?;
        let (tool, label) = (window.tool.id.clone(), window.tool.label.clone());
        if !located.reading.is_empty() {
            window.status = RunStatus::Invalid(format!(
                "Kaynaktan okunuyor: {}. Satırlar okununca çalıştırın.",
                located.reading.join(", ")
            ));
            return Some(());
        }
        let parameters: BTreeMap<String, Value> = window
            .values
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let input_layer = window
            .values
            .get("input")
            .and_then(FeaturesValue::read)
            .and_then(|v| match v.scope {
                Scope::Layer(id) => Some(id),
                _ => None,
            });
        let mut copy = located.doc.clone();
        let uid = |s: &Slot| copy.uid(*s).map(|u| u.to_string());
        let mut selection: Vec<String> = self.selection.ids().iter().filter_map(uid).collect();
        selection.extend(located.chosen.iter().filter_map(uid));
        let b = self.viewport.camera.visible_bounds();
        let run = kentos_commands::ProcessingRun {
            tool: tool.clone(),
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
                let why = a["error"]["message"]
                    .as_str()
                    .map_or_else(|| format!("{label} çalışmadı."), str::to_owned);
                if let Some(window) = &mut self.gis.processing.dialog {
                    window.status = RunStatus::Error(why);
                }
                return Some(());
            }
            Err(e) => {
                if let Some(window) = &mut self.gis.processing.dialog {
                    window.status = RunStatus::Error(format!("{label} çalışmadı: {e:?}"));
                }
                return Some(());
            }
        };
        // Back by layer and key: the drawing's objects, and each data layer's rows.
        let mut drawn: Vec<Slot> = Vec::new();
        let mut by_layer: BTreeMap<String, BTreeSet<i64>> = BTreeMap::new();
        for u in answer["output"]["selected"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            let Some(slot) = kentos_domain::Uuid::parse_str(u)
                .ok()
                .and_then(|u| copy.slot_of(u))
            else {
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
        let tree = copy.layers();
        let layers: Vec<LayerResult> = tree
            .leaves()
            .into_iter()
            .filter_map(|l| by_layer.remove(&l.id).map(|keys| (l, keys)))
            .map(|(l, keys)| LayerResult {
                layer: l.id.clone(),
                label: layer_label(tree, &l.id),
                keys,
            })
            .collect();
        // The table shows one layer: the input's, else the one with most rows chosen.
        let shown = input_layer
            .filter(|l| layers.iter().any(|r| &r.layer == l))
            .or_else(|| {
                layers
                    .iter()
                    .max_by_key(|r| r.keys.len())
                    .map(|r| r.layer.clone())
            });
        let results = LayerResults {
            verb: if tool == EXPRESSION {
                "koşulu sağladı"
            } else {
                "ilişkiyi sağladı"
            },
            drawn: drawn.len(),
            layers,
            shown: shown.clone(),
        };
        match &shown {
            Some(layer) => self.show_result_layer(layer, &results),
            None => {
                // Nothing of a data layer: the table's earlier choice goes with a new selection.
                if let Some(layer) = self.gis.features.data.selected_layer().map(str::to_owned) {
                    self.gis.features.data.choose_rows(&layer, BTreeSet::new());
                }
            }
        }
        let nothing = results.drawn == 0 && results.layers.is_empty();
        if let Some(window) = &mut self.gis.processing.dialog {
            window.status = RunStatus::Ok {
                text: results.said(),
                pick: Vec::new(),
                selected: true,
                undo: false,
                table: None,
                warning: nothing,
            };
            window.layer_results = (results.layers.len() > 1).then_some(results);
        }
        Some(())
    }

    /// A layer's matches in the table: its rows chosen, the table on it.
    fn show_result_layer(&mut self, layer: &str, results: &LayerResults) {
        let Some(r) = results.layers.iter().find(|r| r.layer == layer) else {
            return;
        };
        self.gis.features.data.choose_rows(layer, r.keys.clone());
        self.open_feature_table();
        let _ = self.features_event(crate::features::Event::Layer(layer.to_owned()));
    }

    /// A layer chosen on the result card (docs/adr/9345): its matches shown in the table from the kept result, the run
    /// not made again; the status line says the table's layer.
    pub(super) fn processing_result_layer(&mut self, layer: &str) {
        let Some(mut results) = self
            .gis
            .processing
            .dialog
            .as_ref()
            .and_then(|w| w.layer_results.clone())
        else {
            return;
        };
        if !results.layers.iter().any(|r| r.layer == layer) {
            return;
        }
        self.show_result_layer(layer, &results);
        results.shown = Some(layer.to_owned());
        if let Some(window) = &mut self.gis.processing.dialog {
            if let RunStatus::Ok { text, .. } = &mut window.status {
                *text = results.said();
            }
            window.layer_results = Some(results);
        }
    }
}

/// A line with another sentence after it: a full stop between them where the line has none.
fn sentence_after(line: &str, more: &str) -> String {
    let line = line.trim_end();
    if line.is_empty() {
        more.to_owned()
    } else if line.ends_with(['.', '!', '?', ':']) {
        format!("{line} {more}")
    } else {
        format!("{line}. {more}")
    }
}

/// A layer as the results name it: its group's path when another layer shares its short name.
fn layer_label(tree: &kentos_domain::LayerTree, id: &str) -> String {
    let leaves = tree.leaves();
    let Some(node) = leaves.iter().find(|l| l.id == id) else {
        return id.to_owned();
    };
    let shared = leaves
        .iter()
        .filter(|l| l.name.trim().eq_ignore_ascii_case(node.name.trim()))
        .count()
        > 1;
    if shared {
        tree.path(id)
    } else {
        node.name.clone()
    }
}

/// What an input's line adds over data layers: the layers taken with their rows, and for İfadeyle seç the layers
/// without a field the condition reads (there its value is empty).
fn scope_words(located: &Located, tool: &str, condition: Option<&Value>) -> Option<String> {
    if located.taken.is_empty() {
        return None;
    }
    let each: Vec<String> = located
        .taken
        .iter()
        .map(|(name, n, _)| format!("{name} ({n})"))
        .collect();
    let mut out = format!("Kapsamdaki veri katmanları: {}.", each.join(", "));
    if tool == EXPRESSION
        && let Some(expr) = condition
            .and_then(Value::as_str)
            .and_then(|c| kentos_expression::compile(c).ok())
    {
        let lacking: Vec<String> = located
            .taken
            .iter()
            .filter_map(|(name, _, fields)| {
                let missing: Vec<&str> = expr
                    .fields
                    .iter()
                    .filter(|f| !fields.contains(*f))
                    .map(String::as_str)
                    .collect();
                (!missing.is_empty()).then(|| format!("{name} ({})", missing.join(", ")))
            })
            .collect();
        if !lacking.is_empty() {
            out.push_str(&format!(
                " Alanı olmayan katmanda değeri boş sayılır, eşitlik sağlanmaz: {}.",
                lacking.join("; ")
            ));
        }
    }
    Some(out)
}

#[cfg(test)]
#[path = "data_location_tests.rs"]
mod data_location_tests;
