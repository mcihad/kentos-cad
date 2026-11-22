//! İfadeyle seç on a data layer (docs/adr/9133). A data layer's rows are its source's, not the drawing's objects
//! (docs/adr/9101), so the tool's condition is asked of the source by the rows' reader the table's filter asks through
//! (`layers_rows::read`, off the interface's thread). The keys it keeps make the table's selection of the layer as Seçim
//! biçimi says: a new selection, added to it, taken out of it, or only the ones kept within it. The table comes to the
//! layer with its rows selected; the map outlines them and Seçime yakınlaştır frames them. The preview counts the rows
//! İstatistik reads (data_rows.rs). Durdur leaves the selection as it was. CAD özellik süzgeci asks of the drawing's
//! objects alone: a data layer's rows have no CAD colour or text height, so it is refused there.

use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use iced::Task;
use kentos_commands::{LayerNode, LayersRows};
use serde_json::Value;

use super::Event;
use super::dialog::RunStatus;
use crate::app::{App, Message};
use crate::gis;

/// The tool it answers for.
const TOOL: &str = "selection.byExpression";

/// The words.
pub(crate) mod words {
    pub const ASKING: &str = "Koşul kaynakta soruluyor…";
    pub const STOPPED: &str = "Durduruldu; seçim değişmedi.";
    pub const CAD: &str = "CAD özellik süzgeci veri katmanında kullanılamaz: kaynağın satırlarının CAD rengi ve yazı \
                           yüksekliği yoktur. Gelişmiş ayarlarda kapatın.";
    pub const READING: &str = "Kaynağın satırları okunuyor; sayı birazdan.";
}

/// A run asking the source: its number, its stop switch, the layer and Seçim biçimi.
struct Run {
    id: u64,
    stop: Arc<AtomicBool>,
    layer: String,
    mode: String,
}

/// The run going, and the runs' count.
#[derive(Default)]
pub(crate) struct DataSelect {
    running: Option<Run>,
    runs: u64,
}

/// What the source answered: the keys the condition keeps and the rows there are, or why not.
#[derive(Debug, Clone)]
pub enum SelectEvent {
    Ran(u64, Result<(Vec<i64>, u64), String>),
}

fn msg(e: SelectEvent) -> Message {
    Message::Gis(gis::Event::Processing(Event::DataSelect(e)))
}

/// What a run on a data layer asks.
struct Asked {
    node: LayerNode,
    condition: String,
    mode: String,
}

/// The selection Seçim biçimi makes of the one there is and the keys the condition kept.
fn combine(mode: &str, current: &BTreeSet<i64>, hits: &BTreeSet<i64>) -> BTreeSet<i64> {
    match mode {
        "add" => current.union(hits).copied().collect(),
        "remove" => current.difference(hits).copied().collect(),
        "within" => current.intersection(hits).copied().collect(),
        _ => hits.clone(),
    }
}

/// The keys of the rows the condition keeps and how many rows there are, read from the source.
fn keys_kept(
    node: &LayerNode,
    settings: &kentos_commands::ProjectSettings,
    condition: &str,
    stop: &AtomicBool,
) -> Result<(Vec<i64>, u64), String> {
    let stopped = || stop.load(Ordering::Relaxed);
    let read = |filter: Option<String>, keys: bool| {
        kentos_native_application::layers_rows::read(
            node,
            settings,
            &LayersRows {
                layer: node.id.clone(),
                filter,
                fields: Some(Vec::new()),
                count: Some(true),
                all_keys: keys.then_some(true),
                limit: Some(0),
                ..LayersRows::default()
            },
            &stopped,
        )
        .map_err(|e| e.message)
    };
    let kept = read(Some(condition.to_owned()), true)?;
    let total = read(None, false)?.total.unwrap_or(0);
    Ok((kept.keys.unwrap_or_default(), total))
}

impl App {
    /// The data layer İfadeyle seç's input names (Katman: a data layer); none for another tool or input.
    fn data_select_layer(&self) -> Option<LayerNode> {
        let window = self.gis.processing.dialog.as_ref()?;
        if window.tool.id != TOOL {
            return None;
        }
        let doc = self.document.as_ref()?;
        let input = window.values.get("input").and_then(kentos_processing::FeaturesValue::read)?;
        let kentos_processing::Scope::Layer(id) = input.scope else {
            return None;
        };
        let node = doc.model.layers().get(&id)?;
        node.data.as_ref()?;
        Some(node.clone())
    }

    /// The input's line on a data layer: its rows in the source, not “no object” in the drawing; its fields offered to
    /// the condition. While the rows are read it says so without a warning; a source that cannot be read warns why.
    pub(super) fn data_select_input(&self) -> Option<kentos_processing::InputSummary> {
        let node = self.data_select_layer()?;
        let (count, description) = match self.data_layer_rows(&node.id)? {
            Ok(Some(rows)) => (
                rows.len(),
                format!(
                    "“{}” veri katmanı: {} satır kaynağında; koşul kaynakta sorulur.",
                    node.name,
                    crate::view::thousands(rows.len() as f64)
                ),
            ),
            Ok(None) => (1, format!("“{}” veri katmanı: satırları kaynaktan okunuyor.", node.name)),
            Err(why) => (0, format!("“{}” veri katmanı okunamadı: {why}", node.name)),
        };
        Some(kentos_processing::InputSummary {
            count,
            description,
            by_kind: Vec::new(),
            fields: node.fields.iter().map(|f| (f.name.clone(), count)).collect(),
            rows: true,
            rasters: Vec::new(),
        })
    }

    /// The data layer İfadeyle seç's input names (Katman: a data layer), with the condition and Seçim biçimi; why not
    /// when it cannot run there; none for another tool or input.
    fn data_select_asked(&self) -> Option<Result<Asked, String>> {
        let node = self.data_select_layer()?;
        let window = self.gis.processing.dialog.as_ref()?;
        if window.values.get("cadFilter").and_then(Value::as_bool) == Some(true) {
            return Some(Err(words::CAD.to_owned()));
        }
        let text = |name: &str| window.values.get(name).and_then(Value::as_str).unwrap_or_default().trim().to_owned();
        let mode = Some(text("mode")).filter(|m| !m.is_empty()).unwrap_or_else(|| "new".to_owned());
        Some(Ok(Asked {
            node,
            condition: text("condition"),
            mode,
        }))
    }

    /// The condition's preview on a data layer: how many of the rows İstatistik reads it keeps, said while they are read
    /// and when a field it names is not the layer's.
    pub(super) fn data_select_preview(&self) -> Option<String> {
        let asked = self.data_select_asked()?.ok()?;
        if asked.condition.is_empty() {
            return None;
        }
        let doc = self.document.as_ref()?;
        let unit = kentos_native_application::expression_unit::reading_unit(doc.model.settings());
        let filter = kentos_commands::LayerFilter {
            expression: Some(asked.condition.clone()),
            ..kentos_commands::LayerFilter::default()
        };
        let compiled = match kentos_native_application::layer_filter::compile_filter(&filter, unit) {
            Ok(c) => c,
            Err(why) => return Some(why),
        };
        let fields: Vec<String> = asked.node.fields.iter().map(|f| f.name.clone()).collect();
        if let Some(why) = compiled
            .expr()
            .and_then(|e| kentos_native_application::layers_rows::missing_fields(&e.fields, &fields, &asked.node.name))
        {
            return Some(why);
        }
        Some(match self.data_layer_rows(&asked.node.id)? {
            Ok(None) => words::READING.to_owned(),
            Err(why) => why,
            Ok(Some(rows)) => {
                let (kept, total) =
                    kentos_native_application::layers_filter::count_rows(&doc.model, &rows, Some(&compiled));
                format!(
                    "{} / {} satır koşulu sağlıyor.",
                    crate::view::thousands(f64::from(kept)),
                    crate::view::thousands(f64::from(total))
                )
            }
        })
    }

    /// Çalıştır on a data layer: the condition asked of the source, off the interface's thread; none for another tool
    /// or input (the drawing's run goes on).
    pub(super) fn processing_data_select_run(&mut self) -> Option<Task<Message>> {
        let asked = self.data_select_asked()?;
        let settings = self.document.as_ref()?.model.settings().clone();
        let window = self.gis.processing.dialog.as_mut()?;
        let asked = match asked {
            Ok(a) => a,
            Err(why) => {
                window.status = RunStatus::Invalid(why);
                return Some(Task::none());
            }
        };
        let stop = Arc::new(AtomicBool::new(false));
        let select = &mut self.gis.processing.data_select;
        select.runs += 1;
        let id = select.runs;
        select.running = Some(Run {
            id,
            stop: Arc::clone(&stop),
            layer: asked.node.id.clone(),
            mode: asked.mode.clone(),
        });
        window.status = RunStatus::Running {
            fraction: 0.0,
            label: words::ASKING.to_owned(),
            background: true,
        };
        let Asked { node, condition, .. } = asked;
        Some(crate::db_connections::off_thread(
            move || keys_kept(&node, &settings, &condition, &stop),
            move |answer| msg(SelectEvent::Ran(id, answer)),
        ))
    }

    /// Durdur for a run asking the source: asked to stop; the selection stays as it was. False when none runs.
    pub(super) fn processing_data_select_stop(&mut self) -> bool {
        let Some(run) = &self.gis.processing.data_select.running else {
            return false;
        };
        run.stop.store(true, Ordering::Relaxed);
        true
    }

    pub(super) fn data_select_event(&mut self, event: SelectEvent) -> Task<Message> {
        let SelectEvent::Ran(id, answer) = event;
        let select = &mut self.gis.processing.data_select;
        let Some(run) = select.running.take_if(|r| r.id == id) else {
            return Task::none();
        };
        let status = if run.stop.load(Ordering::Relaxed) {
            RunStatus::Error(words::STOPPED.to_owned())
        } else {
            match answer {
                Err(why) => RunStatus::Error(why),
                Ok((kept, total)) => {
                    let data = &self.gis.features.data;
                    let current: BTreeSet<i64> =
                        if data.any_selected_of(&run.layer) { data.selected.clone() } else { BTreeSet::new() };
                    let hits: BTreeSet<i64> = kept.into_iter().collect();
                    let chosen = combine(&run.mode, &current, &hits);
                    let count = chosen.len();
                    self.gis.features.data.choose_rows(&run.layer, chosen);
                    // The table comes to the layer: its rows selected there, outlined on the map.
                    self.open_feature_table();
                    let _ = self.features_event(crate::features::Event::Layer(run.layer.clone()));
                    let n = |v: usize| crate::view::thousands(v as f64);
                    RunStatus::Ok {
                        text: format!(
                            "{} / {} satır koşulu sağladı; seçimde {} satır var.",
                            n(hits.len()),
                            crate::view::thousands(total as f64),
                            n(count)
                        ),
                        pick: Vec::new(),
                        selected: true,
                        undo: false,
                        table: None,
                        warning: hits.is_empty(),
                    }
                }
            }
        };
        if let Some(window) = &mut self.gis.processing.dialog {
            window.status = status;
        }
        Task::none()
    }
}

#[cfg(test)]
#[path = "data_select_tests.rs"]
mod data_select_tests;
