//! An extension's panel (docs/adr/8057 §24; plan/eklenti.md ADIM 22): its `kentos.ui.panel` drawn in the dock's Eklenti
//! tab with KentOS's own components. A button runs one of the extension's commands with the panel's values as its
//! parameters (`ctx.params`, by each input's id); the rows the command answers fill the panel's table, its words show
//! under it. The extension's code never gets a widget: the panel is data, its buttons are commands, and a command runs
//! as any other (its process, the broker, one undo step, Durdur, its log).
//!
//! - **Opening needs no Python:** the panel is declared; its buttons say why they do not run while the extension's
//!   environment is not ready, as its ribbon buttons do.
//! - **What is typed is kept as typed:** a number's text too, so a half-typed `12,` is not lost; it is read when a
//!   button is pressed, and a text that is not a number is said there, nothing run.
//! - **A late answer goes nowhere:** a run's rows are written only to the panel it started from, while it is open.
//! - **A row with a `uid` is an object:** clicking it selects and zooms to it, as an audit's Göster; the `uid` is not
//!   shown as a column.
//! - **A running button shows how far it is:** its command's `ctx.progress` as a bar and words under the form, with
//!   Durdur; a command that answers a message, not rows, leaves the table as it was.
//! - **Enter presses the primary button** in a text or number field (else the panel's first button).
//! - **A value changed can run a command:** an input's `onChange` runs it with the panel's values (its rows to the
//!   first table); a change while a command runs waits for it, one rerun with the values then.
//! - **Pickers follow their layer picker:** a field picker lists the fields of the layer chosen there, a feature picker
//!   takes the selected objects of that layer when the user asks ("Seçimi al"); choosing another layer clears both.

use std::collections::BTreeMap;

use iced::widget::{column, container, row, scrollable};
use iced::{Element, Fill, Task};
use kentos_extensions::manifest::PanelSide;
use kentos_extensions::ui_panel::{Component, PanelDefinition};
use kentos_ui::label;
use kentos_ui::widget::Form;
use kentos_ui::widget::select::{Choice, Select};
use serde_json::{Map, Value, json};

use super::{Event, msg};
use crate::app::{App, Message};
use crate::exchange::words;

/// Rows a table shows; the rest are counted.
const SHOWN_ROWS: usize = 500;

/// A panel open in the dock.
#[derive(Clone, Debug)]
pub(crate) struct OpenPanel {
    pub(crate) ext: String,
    pub(crate) local: String,
    pub(crate) definition: PanelDefinition,
    /// The side of the window its manifest asks for (`entrypoints.panels[].side`).
    pub(crate) side: PanelSide,
    /// What each text and number field holds, as typed.
    pub(crate) texts: BTreeMap<String, String>,
    pub(crate) checks: BTreeMap<String, bool>,
    pub(crate) choices: BTreeMap<String, String>,
    /// Each feature picker's objects, their persistent ids.
    pub(crate) picks: BTreeMap<String, Vec<String>>,
    /// Each table's columns and rows, from its button's command.
    pub(crate) tables: BTreeMap<String, (Vec<String>, Vec<Value>)>,
    /// The panel's last words and their mark: done, stopped by the user, failed.
    pub(crate) said: Option<(words::Kind, String)>,
    /// The button (or the changed input) whose command runs now.
    pub(crate) running: Option<String>,
    /// An input with `onChange` changed while a command ran: its command runs again when that one ends.
    pub(crate) rerun: Option<String>,
}

/// A panel's button pressed: what the next launch of its command gives its code, and where its answer goes.
#[derive(Clone, Debug)]
pub(crate) struct PanelCall {
    pub(crate) ext: String,
    pub(crate) command: String,
    pub(crate) params: Value,
    pub(crate) origin: PanelOrigin,
}

/// Where a run started from a panel answers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PanelOrigin {
    pub(crate) ext: String,
    pub(crate) panel: String,
    pub(crate) button: String,
    pub(crate) table: Option<String>,
}

/// What a panel is told by the user.
#[derive(Clone, Debug)]
pub(crate) enum PanelEvent {
    Text(String, String),
    Check(String, bool),
    Choose(String, String),
    /// A feature picker takes the drawing's selection.
    Pick(String),
    /// A feature picker lets its objects go.
    Unpick(String),
    Press(String),
    /// A table's row with a `uid`: its object selected and zoomed to.
    Show(String),
    Close,
}

impl OpenPanel {
    /// The panel as its definition starts it.
    pub(crate) fn new(ext: &str, local: &str, definition: PanelDefinition) -> OpenPanel {
        let mut panel = OpenPanel {
            ext: ext.to_owned(),
            local: local.to_owned(),
            definition,
            side: PanelSide::Right,
            texts: BTreeMap::new(),
            checks: BTreeMap::new(),
            choices: BTreeMap::new(),
            picks: BTreeMap::new(),
            tables: BTreeMap::new(),
            said: None,
            running: None,
            rerun: None,
        };
        for c in &panel.definition.body {
            match c {
                Component::TextField { id, value, .. } => {
                    panel
                        .texts
                        .insert(id.clone(), value.clone().unwrap_or_default());
                }
                Component::NumberField { id, value, .. } => {
                    panel
                        .texts
                        .insert(id.clone(), value.map(number_words).unwrap_or_default());
                }
                Component::Checkbox { id, value, .. } => {
                    panel.checks.insert(id.clone(), *value);
                }
                // A layer, a field and objects are chosen by the user: none at first.
                Component::LayerPicker { id, .. } | Component::FieldPicker { id, .. } => {
                    panel.choices.insert(id.clone(), String::new());
                }
                Component::FeaturePicker { id, .. } => {
                    panel.picks.insert(id.clone(), Vec::new());
                }
                Component::Select {
                    id, options, value, ..
                } => {
                    let first = options.first().map(|o| o.value.clone());
                    panel
                        .choices
                        .insert(id.clone(), value.clone().or(first).unwrap_or_default());
                }
                _ => {}
            }
        }
        panel
    }

    /// The command's parameters: each input's value by its id (a text trimmed, a number read with a decimal comma or
    /// point, none when empty); or which number is not one, said in its label.
    pub(crate) fn params(&self) -> Result<Value, String> {
        let mut out = Map::new();
        for c in &self.definition.body {
            match c {
                Component::TextField { id, .. } => {
                    let text = self.texts.get(id).map_or("", |t| t.trim());
                    out.insert(id.clone(), json!(text));
                }
                Component::NumberField {
                    id,
                    label,
                    min,
                    max,
                    ..
                } => {
                    let text = self.texts.get(id).map_or("", |t| t.trim());
                    if text.is_empty() {
                        out.insert(id.clone(), Value::Null);
                        continue;
                    }
                    let Some(n) = number_of(text) else {
                        return Err(format!("“{label}”: “{text}” bir sayı değil."));
                    };
                    if min.is_some_and(|m| n < m) || max.is_some_and(|m| n > m) {
                        return Err(format!(
                            "“{label}” {} ile {} arasında olmalı.",
                            min.map_or_else(|| "−∞".to_owned(), number_words),
                            max.map_or_else(|| "∞".to_owned(), number_words)
                        ));
                    }
                    out.insert(id.clone(), json!(n));
                }
                Component::Select { id, .. } => {
                    out.insert(
                        id.clone(),
                        json!(self.choices.get(id).cloned().unwrap_or_default()),
                    );
                }
                // A layer's id, a field's name; none when none is chosen.
                Component::LayerPicker { id, .. } | Component::FieldPicker { id, .. } => {
                    let chosen = self.choices.get(id).filter(|l| !l.is_empty());
                    out.insert(id.clone(), chosen.map_or(Value::Null, |l| json!(l)));
                }
                // The objects' persistent ids; one or none when it takes one.
                Component::FeaturePicker { id, multiple, .. } => {
                    let picked = self.picks.get(id).cloned().unwrap_or_default();
                    let value = if *multiple {
                        json!(picked)
                    } else {
                        picked.first().map_or(Value::Null, |u| json!(u))
                    };
                    out.insert(id.clone(), value);
                }
                Component::Checkbox { id, .. } => {
                    out.insert(
                        id.clone(),
                        json!(self.checks.get(id).copied().unwrap_or(false)),
                    );
                }
                _ => {}
            }
        }
        Ok(Value::Object(out))
    }

    /// A choice made: another layer in a layer picker clears the field and the objects chosen from the one before.
    pub(crate) fn choose(&mut self, id: String, value: String) {
        let layer = self
            .definition
            .body
            .iter()
            .any(|c| matches!(c, Component::LayerPicker { id: l, .. } if *l == id));
        if layer && self.choices.get(&id) != Some(&value) {
            for c in &self.definition.body {
                match c {
                    Component::FieldPicker { id: f, layer, .. } if *layer == id => {
                        self.choices.insert(f.clone(), String::new());
                    }
                    Component::FeaturePicker {
                        id: f,
                        layer: Some(layer),
                        ..
                    } if *layer == id => {
                        self.picks.insert(f.clone(), Vec::new());
                    }
                    _ => {}
                }
            }
        }
        self.choices.insert(id, value);
    }

    /// The button Enter presses in a text or number field: the primary one, else the first; none without buttons.
    pub(crate) fn default_button(&self) -> Option<&str> {
        let buttons: Vec<(&str, bool)> = self
            .definition
            .body
            .iter()
            .filter_map(|c| match c {
                Component::Button { id, primary, .. } => Some((id.as_str(), *primary)),
                _ => None,
            })
            .collect();
        buttons
            .iter()
            .find(|(_, primary)| *primary)
            .or(buttons.first())
            .map(|(id, _)| *id)
    }

    /// The layer a picker's layer picker holds now; none when it has none or none is chosen.
    fn layer_of(&self, picker: Option<&str>) -> Option<&str> {
        picker
            .and_then(|l| self.choices.get(l))
            .map(String::as_str)
            .filter(|l| !l.is_empty())
    }

    /// The table a button's rows go to: its own, else the panel's first.
    fn table_of(&self, table: Option<&String>) -> Option<String> {
        table.cloned().or_else(|| {
            self.definition.body.iter().find_map(|c| match c {
                Component::Table { id, .. } => Some(id.clone()),
                _ => None,
            })
        })
    }
}

/// A number as a field shows it: a whole one without decimals, a decimal with a comma.
fn number_words(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{n:.0}")
    } else {
        n.to_string().replace('.', ",")
    }
}

/// A typed number, its decimal sign a comma or a point.
fn number_of(text: &str) -> Option<f64> {
    text.replace(' ', "")
        .replace(',', ".")
        .parse::<f64>()
        .ok()
        .filter(|n| n.is_finite())
}

/// A cell's words.
fn cell_words(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => "—".to_owned(),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Bool(b)) => if *b { "evet" } else { "hayır" }.to_owned(),
        Some(Value::Number(n)) => n.as_f64().map_or_else(|| n.to_string(), number_words),
        Some(other) => other.to_string(),
    }
}

impl App {
    /// An extension's panel from its tab: its definition read from the package (checked when it was installed) and
    /// shown in the dock's Eklenti tab. The same panel again keeps what was typed in it.
    pub(crate) fn ext_panel_open(&mut self, ext: &str, local: &str) -> Task<Message> {
        if self
            .extensions
            .panel
            .as_ref()
            .is_some_and(|p| p.ext == ext && p.local == local)
        {
            return self.show_extension_panel();
        }
        let Some(registry) = self.extensions.registry.as_ref() else {
            return Task::none();
        };
        let part = format!("panel “{local}”");
        let Some(entry) = registry
            .manifest(ext)
            .and_then(|m| m.entrypoints.panels.iter().find(|p| p.id == local).cloned())
        else {
            self.extension_failed(
                ext,
                &part,
                "panel_missing",
                "Bu panel eklentinin kurulu sürümünde yok. Eklentiyi güncelleyin ya da yayımcısına bildirin.",
                None,
            );
            return Task::none();
        };
        let definition = registry
            .package_file(ext, &entry.definition)
            .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
            .and_then(|value| kentos_extensions::ui_panel::read(&value));
        let Some(definition) = definition else {
            self.extension_failed(
                ext,
                &part,
                "panel_invalid",
                &format!(
                    "Panel dosyası ({}) okunamadı ya da bir KentOS paneli değil. Yayımcısına bildirin.",
                    entry.definition
                ),
                None,
            );
            return Task::none();
        };
        let mut panel = OpenPanel::new(ext, local, definition);
        panel.side = entry.side;
        self.extensions.panel = Some(panel);
        self.show_extension_panel()
    }

    /// The dock shows the Eklenti tab on the side the open panel's manifest asks for (right: beside Katmanlar, the side
    /// panels shown if F4 hid them; left: beside the map, Katmanlar staying in sight; bottom: under the map). A tab
    /// KentOS docked moves when another panel asks for another side; one the user moved stays where it is.
    pub(crate) fn show_extension_panel(&mut self) -> Task<Message> {
        use crate::app::Panel;
        use kentos_ui::widget::docking::{Event as Dock, Side, Slot, Target};
        if !self.open_panel(Panel::Extension) {
            return Task::none();
        }
        let wanted = match self
            .extensions
            .panel
            .as_ref()
            .map_or(PanelSide::Right, |p| p.side)
        {
            PanelSide::Left => Side::Left,
            PanelSide::Right => Side::Right,
            PanelSide::Bottom => Side::Bottom,
        };
        if wanted == Side::Right && !self.right_panel_shown() {
            self.toggle_right_panel();
        }
        if let Some(Slot::Docked(side, _)) = self.docks.slot(Panel::Extension)
            && side != wanted
            && self.extensions.panel_placed == Some(side)
        {
            self.docks.close(Panel::Extension);
        }
        if !self.docks.contains(Panel::Extension) {
            self.docks.show(Panel::Extension, wanted);
            // Right: beside Katmanlar, in the side's tall stack: `show` puts a new panel at the end of the side's last
            // stack, Öznitelikler's, where a panel of a form and a table had a third of the height (measured in the
            // real window, 14 November 2026).
            if wanted == Side::Right
                && let Some(Slot::Docked(side, index)) = self.docks.slot(Panel::Layers)
                && self.docks.slot(Panel::Extension) != Some(Slot::Docked(side, index))
            {
                let end = self
                    .docks
                    .stacks(side)
                    .get(index)
                    .map_or(0, |s| s.tabs.len());
                self.docks.update(Dock::Moved(
                    Panel::Extension,
                    Target::Tab(Slot::Docked(side, index), end),
                ));
            }
            self.extensions.panel_placed = Some(wanted);
        }
        self.docks.update(Dock::Selected(Panel::Extension));
        Task::none()
    }

    pub(crate) fn ext_panel_event(&mut self, event: PanelEvent) -> Task<Message> {
        let Some(panel) = self.extensions.panel.as_mut() else {
            return Task::none();
        };
        match event {
            PanelEvent::Text(id, text) => {
                panel.texts.insert(id, text);
            }
            PanelEvent::Check(id, on) => {
                panel.checks.insert(id.clone(), on);
                return self.ext_panel_changed(&id);
            }
            PanelEvent::Choose(id, value) => {
                panel.choose(id.clone(), value);
                return self.ext_panel_changed(&id);
            }
            PanelEvent::Unpick(id) => {
                panel.picks.insert(id.clone(), Vec::new());
                return self.ext_panel_changed(&id);
            }
            PanelEvent::Pick(id) => return self.ext_panel_pick(&id),
            PanelEvent::Close => self.extensions.panel = None,
            PanelEvent::Press(button) => return self.ext_panel_press(&button),
            PanelEvent::Show(uid) => {
                return self.ext_show_object(
                    &uid,
                    "Satırın nesnesi çizimde yok: silinmiş ya da başka bir çizim açık.",
                );
            }
        }
        Task::none()
    }

    /// A feature picker takes the drawing's selection: the selected objects of the layer its layer picker holds and of
    /// its type (the first only when it takes one); or the panel says why none fit, and what it held stays.
    fn ext_panel_pick(&mut self, picker: &str) -> Task<Message> {
        let Some(panel) = self.extensions.panel.as_ref() else {
            return Task::none();
        };
        let Some(Component::FeaturePicker {
            label: words,
            layer,
            geometry,
            multiple,
            ..
        }) = panel
            .definition
            .body
            .iter()
            .find(|c| c.id() == Some(picker))
            .cloned()
        else {
            return Task::none();
        };
        let layer = panel.layer_of(layer.as_deref()).map(str::to_owned);
        let kind = geometry
            .as_deref()
            .and_then(|g| serde_json::from_value::<kentos_commands::LayerGeometry>(json!(g)).ok());
        let selected = self.selection.ids();
        let mut picked: Vec<String> = Vec::new();
        let mut layer_name = None;
        if let Some(doc) = &self.document {
            picked = selected
                .iter()
                .filter_map(|&slot| {
                    let e = doc.model.get(slot)?;
                    let fits = layer.as_ref().is_none_or(|l| e.base().layer_id == *l)
                        && kind.is_none_or(|k| k.admits(e.kind()));
                    fits.then(|| doc.model.uid(slot)).flatten()
                })
                .map(|u| u.to_string())
                .collect();
            layer_name = layer
                .as_deref()
                .and_then(|l| doc.model.layers().get(l))
                .map(|n| n.name.clone());
        }
        let count = selected.len();
        let Some(panel) = self.extensions.panel.as_mut() else {
            return Task::none();
        };
        if picked.is_empty() {
            let mut wanted: Vec<String> = Vec::new();
            if let Some(name) = layer_name {
                wanted.push(format!("“{name}” katmanından"));
            }
            if let Some(k) = kind {
                wanted.push(format!("{} türünde", k.label().to_lowercase()));
            }
            let why = if count == 0 {
                format!("“{words}”: önce çizimde nesne seçin.")
            } else if wanted.is_empty() {
                format!("“{words}”: seçili nesnelerin kalıcı kimliği yok.")
            } else {
                format!(
                    "“{words}”: seçili {count} nesneden hiçbiri {} değil.",
                    wanted.join(" ve ")
                )
            };
            panel.said = Some((words::Kind::Error, why));
            return Task::none();
        }
        if !multiple {
            picked.truncate(1);
        }
        panel.picks.insert(picker.to_owned(), picked);
        panel.said = None;
        self.ext_panel_changed(picker)
    }

    /// A button: its command run with the panel's values, its rows to its table.
    fn ext_panel_press(&mut self, button: &str) -> Task<Message> {
        let found = self
            .extensions
            .panel
            .as_ref()
            .filter(|p| p.running.is_none())
            .and_then(
                |p| match p.definition.body.iter().find(|c| c.id() == Some(button)) {
                    Some(Component::Button { command, table, .. }) => {
                        Some((command.clone(), table.clone()))
                    }
                    _ => None,
                },
            );
        match found {
            Some((command, table)) => self.ext_panel_start(button, command, table),
            None => Task::none(),
        }
    }

    /// An input changed: its `onChange` command runs with the panel's values, its rows to the panel's first table. A
    /// change while a command runs waits for it: one rerun, with the values then.
    fn ext_panel_changed(&mut self, input: &str) -> Task<Message> {
        let Some(panel) = self.extensions.panel.as_mut() else {
            return Task::none();
        };
        let Some(command) = panel
            .definition
            .body
            .iter()
            .find(|c| c.id() == Some(input))
            .and_then(Component::on_change)
            .map(str::to_owned)
        else {
            return Task::none();
        };
        if panel.running.is_some() {
            panel.rerun = Some(input.to_owned());
            return Task::none();
        }
        self.ext_panel_start(input, command, None)
    }

    /// A change that came while the panel's command ran: its command now, with the values now (runner.rs, when a run
    /// ends).
    pub(crate) fn ext_panel_rerun(&mut self) -> Task<Message> {
        let input = match self.extensions.panel.as_mut() {
            Some(p) if p.running.is_none() => p.rerun.take(),
            _ => None,
        };
        match input {
            Some(input) => self.ext_panel_changed(&input),
            None => Task::none(),
        }
    }

    /// The panel's `command` started by `trigger` (a button, a changed input) with the panel's values; a number that is
    /// not one, or Python not ready, said in the panel and nothing run.
    fn ext_panel_start(
        &mut self,
        trigger: &str,
        command: String,
        table: Option<String>,
    ) -> Task<Message> {
        let ext = match self.extensions.panel.as_ref() {
            Some(p) if p.running.is_none() => p.ext.clone(),
            _ => return Task::none(),
        };
        let why = self
            .extensions
            .python_reasons
            .get(&ext)
            .and_then(Option::as_deref)
            .map(str::to_owned);
        let Some(panel) = self.extensions.panel.as_mut() else {
            return Task::none();
        };
        if let Some(why) = why {
            panel.said = Some((words::Kind::Error, why));
            return Task::none();
        }
        // Objects taken before and deleted since are not given: the code would not find them.
        if let Some(doc) = &self.document {
            for uids in panel.picks.values_mut() {
                uids.retain(|u| {
                    kentos_domain::Uuid::parse_str(u)
                        .ok()
                        .and_then(|u| doc.model.slot_of(u))
                        .is_some()
                });
            }
        }
        let params = match panel.params() {
            Ok(params) => params,
            Err(why) => {
                panel.said = Some((words::Kind::Error, why));
                return Task::none();
            }
        };
        let origin = PanelOrigin {
            ext: ext.clone(),
            panel: panel.local.clone(),
            button: trigger.to_owned(),
            table: panel.table_of(table.as_ref()),
        };
        panel.running = Some(trigger.to_owned());
        panel.said = None;
        self.extensions.panel_call = Some(PanelCall {
            ext: ext.clone(),
            command: command.clone(),
            params,
            origin,
        });
        self.ext_run(&ext, "command", &command)
    }

    /// The panel call of `ext`'s `command`, taken for its launch (runner.rs); none for another command's.
    pub(crate) fn ext_panel_take(&mut self, ext: &str, command: &str) -> Option<PanelCall> {
        self.extensions
            .panel_call
            .take_if(|c| c.ext == ext && c.command == command)
    }

    /// A panel call that did not start: the panel says why and its button can be pressed again.
    pub(crate) fn ext_panel_dropped(&mut self, ext: &str, command: &str, why: &str) {
        if let Some(call) = self.ext_panel_take(ext, command) {
            self.ext_panel_answer(&call.origin, Err(why.to_owned()));
        }
    }

    /// A run started from a panel ended (runner.rs): its rows into its table and its words under the panel, or why
    /// it failed; nothing when the panel it started from is closed or another.
    pub(crate) fn ext_panel_answer(
        &mut self,
        origin: &PanelOrigin,
        outcome: Result<Value, String>,
    ) {
        let Some(panel) = self
            .extensions
            .panel
            .as_mut()
            .filter(|p| p.ext == origin.ext && p.local == origin.panel)
        else {
            return;
        };
        panel.running = None;
        match outcome {
            Ok(result) => {
                let rows = result
                    .get("rows")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                let mut columns: Vec<String> = result
                    .get("columns")
                    .and_then(Value::as_array)
                    .map(|c| {
                        c.iter()
                            .filter_map(Value::as_str)
                            .map(str::to_owned)
                            .collect()
                    })
                    .unwrap_or_default();
                if columns.is_empty() {
                    for row in &rows {
                        for key in row.as_object().into_iter().flat_map(|o| o.keys()) {
                            if !columns.contains(key) {
                                columns.push(key.clone());
                            }
                        }
                    }
                }
                // A message answers no rows: the table keeps what it showed (a button that writes, as Yapıları
                // numarala, does not empty the search's results).
                if let Some(table) = origin
                    .table
                    .as_ref()
                    .filter(|_| result.get("rows").is_some())
                {
                    panel.tables.insert(table.clone(), (columns, rows.clone()));
                }
                let words = result
                    .get("message")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
                    .unwrap_or_else(|| format!("{} satır.", rows.len()));
                panel.said = Some((words::Kind::Ok, words));
            }
            Err(why) => panel.said = Some((words::Kind::Error, why)),
        }
    }

    /// A run started from a panel stopped with Durdur: said as the user's choice, not a failure (it was shown in red,
    /// real window, 14 November 2026); its table keeps what it showed.
    pub(crate) fn ext_panel_stopped(&mut self, origin: &PanelOrigin) {
        if let Some(panel) = self
            .extensions
            .panel
            .as_mut()
            .filter(|p| p.ext == origin.ext && p.local == origin.panel)
        {
            panel.running = None;
            // A change that waited for it does not start it again: the user stopped it.
            panel.rerun = None;
            panel.said = Some((
                words::Kind::Info,
                "Durduruldu; çizime yazdıkları geri alındı.".to_owned(),
            ));
        }
    }

    /// The dock's Eklenti tab: the open panel, its components as KentOS draws them.
    pub(crate) fn extension_panel_view(&self) -> Element<'_, Message> {
        let Some(p) = &self.extensions.panel else {
            return container(label::muted(
                "Açık eklenti paneli yok: bir eklentinin sekmesindeki panel düğmesiyle açılır.",
            ))
            .padding(12)
            .into();
        };
        let name = self.extension_name(&p.ext);
        let why = self
            .extensions
            .python_reasons
            .get(&p.ext)
            .and_then(Option::as_deref);
        let event = |e: PanelEvent| msg(Event::Panel(e));
        // Enter in a text or number field presses the panel's primary button, as a form's.
        let enter = p
            .default_button()
            .filter(|_| p.running.is_none() && why.is_none())
            .map(|b| event(PanelEvent::Press(b.to_owned())));
        let mut form = Form::new().label_width(kentos_ui::theme::typography::scaled(110.0));
        let mut tables: Vec<Element<'_, Message>> = Vec::new();
        for c in &p.definition.body {
            match c {
                // A text the panel's whole width: in the field column it wrapped every few words in a narrow dock.
                Component::Text { text, .. } => form = form.wide(label::caption(text.clone())),
                Component::TextField {
                    id,
                    label: words,
                    placeholder,
                    ..
                } => {
                    let id = id.clone();
                    let value = p.texts.get(&id).map_or("", String::as_str);
                    let mut field =
                        kentos_ui::widget::text_field(placeholder.as_deref().unwrap_or(""), value)
                            .on_input(move |t| event(PanelEvent::Text(id.clone(), t)));
                    if let Some(press) = enter.clone() {
                        field = field.on_submit(press);
                    }
                    form = form.field(words.clone(), field);
                }
                Component::NumberField {
                    id,
                    label: words,
                    unit,
                    ..
                } => {
                    let id = id.clone();
                    let value = p.texts.get(&id).map_or("", String::as_str);
                    let mut field = kentos_ui::widget::input_field::InputField::new("", value)
                        .on_input(move |t| event(PanelEvent::Text(id.clone(), t)));
                    if let Some(unit) = unit {
                        field = field.suffix(unit.clone());
                    }
                    if let Some(press) = enter.clone() {
                        field = field.on_submit(press);
                    }
                    form = form.field(words.clone(), field);
                }
                Component::Select {
                    id,
                    label: words,
                    options,
                    ..
                } => {
                    let at = options
                        .iter()
                        .position(|o| p.choices.get(id) == Some(&o.value));
                    let (id, values): (String, Vec<String>) = (
                        id.clone(),
                        options.iter().map(|o| o.value.clone()).collect(),
                    );
                    let pick = Select::new(
                        options.iter().map(|o| Choice::new(o.label.clone())),
                        at,
                        move |i| event(PanelEvent::Choose(id.clone(), values[i].clone())),
                    );
                    form = form.field(words.clone(), pick);
                }
                Component::LayerPicker {
                    id,
                    label: words,
                    geometry,
                    ..
                } => {
                    let layers = self.panel_layers(geometry.as_deref());
                    let at = layers
                        .iter()
                        .position(|(l, _)| p.choices.get(id) == Some(l));
                    let (id, ids): (String, Vec<String>) =
                        (id.clone(), layers.iter().map(|(l, _)| l.clone()).collect());
                    let pick = Select::new(
                        layers.into_iter().map(|(_, name)| Choice::new(name)),
                        at,
                        move |i| event(PanelEvent::Choose(id.clone(), ids[i].clone())),
                    );
                    form = form.field(words.clone(), pick);
                }
                Component::FieldPicker {
                    id,
                    label: words,
                    layer,
                    kinds,
                    ..
                } => {
                    let fields = self.panel_fields(p.layer_of(Some(layer)), kinds);
                    let at = fields
                        .iter()
                        .position(|(f, _)| p.choices.get(id) == Some(f));
                    let (id, names): (String, Vec<String>) =
                        (id.clone(), fields.iter().map(|(f, _)| f.clone()).collect());
                    let pick = Select::new(
                        fields.into_iter().map(|(_, words)| Choice::new(words)),
                        at,
                        move |i| event(PanelEvent::Choose(id.clone(), names[i].clone())),
                    );
                    form = form.field(words.clone(), pick);
                }
                Component::FeaturePicker {
                    id, label: words, ..
                } => {
                    // Two lines: beside both buttons the count had no room in a narrow dock and Temizle ran past its
                    // edge (real window, 14 November 2026).
                    let picked = p.picks.get(id).map_or(0, Vec::len);
                    let count: Element<'_, Message> = if picked == 0 {
                        label::muted("Seçilmedi").into()
                    } else {
                        label::caption(format!("{picked} nesne")).into()
                    };
                    let mut line = row![count].spacing(8).align_y(iced::Center);
                    if picked > 0 {
                        line = line.push(words::ghost(
                            "Temizle",
                            Some(event(PanelEvent::Unpick(id.clone()))),
                        ));
                    }
                    let cell = column![
                        line,
                        words::secondary("Seçimi al", Some(event(PanelEvent::Pick(id.clone())))),
                    ]
                    .spacing(4);
                    form = form.field(words.clone(), cell);
                }
                Component::Checkbox {
                    id, label: words, ..
                } => {
                    let on = p.checks.get(id).copied().unwrap_or(false);
                    form = form.row(words::check(
                        on,
                        words.clone(),
                        Some(event(PanelEvent::Check(id.clone(), !on))),
                    ));
                }
                Component::Button {
                    id,
                    label: words,
                    primary,
                    ..
                } => {
                    let running = p.running.as_deref() == Some(id.as_str());
                    let caption = if running {
                        format!("{words}…")
                    } else {
                        words.clone()
                    };
                    let on = (p.running.is_none() && why.is_none())
                        .then(|| event(PanelEvent::Press(id.clone())));
                    form = form.row(if *primary {
                        words::primary(caption, on)
                    } else {
                        words::secondary(caption, on)
                    });
                }
                Component::Table {
                    id, label: words, ..
                } => {
                    tables.push(self.panel_table(p, id, words.as_deref()));
                }
            }
        }
        let header = row![
            column![
                label::strong(p.definition.title.clone()),
                label::caption(name)
            ]
            .width(Fill),
            words::ghost("Kapat", Some(event(PanelEvent::Close))),
        ]
        .spacing(8);
        let mut body = column![header, form].spacing(12).padding(12).width(Fill);
        // Its button's command running: how far it is, from its ctx.progress (moving until it says), and Durdur.
        if let Some(run) = self
            .extensions
            .runners
            .get(&p.ext)
            .and_then(|r| r.run.as_ref())
            .filter(|run| run.origin.as_ref().is_some_and(|o| o.panel == p.local))
        {
            let (share, text) = run.progress.as_ref().map_or(
                (None, "Çalışıyor…".to_owned()),
                super::runner::progress_words,
            );
            let stop: Element<'_, Message> = if run.stopping.is_some() {
                label::muted("Durduruluyor…").into()
            } else {
                words::ghost("Durdur", Some(msg(Event::StopRun(p.ext.clone()))))
            };
            body = body.push(
                column![
                    kentos_ui::widget::progress::bar(share).width(Fill),
                    row![container(label::caption(text)).width(Fill), stop]
                        .spacing(8)
                        .align_y(iced::Center),
                ]
                .spacing(6),
            );
        }
        if let Some(why) = why {
            body = body.push(words::line(
                words::Kind::Warn,
                label::caption(format!("Düğmeler komut çalıştırır: {why}")),
            ));
        }
        if let Some((kind, said)) = &p.said {
            body = body.push(words::line(*kind, label::caption(said.clone())));
        }
        for table in tables {
            body = body.push(table);
        }
        scrollable(body).height(Fill).into()
    }

    /// The open drawing's layers a layer picker offers: none ("—") first, then each layer (groups give their names to
    /// the path) of `geometry`'s type or untyped.
    fn panel_layers(&self, geometry: Option<&str>) -> Vec<(String, String)> {
        fn walk(
            nodes: &[kentos_commands::LayerNode],
            path: &str,
            geometry: Option<&str>,
            out: &mut Vec<(String, String)>,
        ) {
            for n in nodes {
                let name = if path.is_empty() {
                    n.name.clone()
                } else {
                    format!("{path} › {}", n.name)
                };
                if n.kind == kentos_commands::LayerNodeType::Group {
                    walk(&n.children, &name, geometry, out);
                    continue;
                }
                let fits = match (geometry, n.geometry) {
                    (None, _) | (_, None) => true,
                    (Some(want), Some(kind)) => serde_json::to_value(kind)
                        .ok()
                        .and_then(|v| v.as_str().map(|k| k == want))
                        .unwrap_or(false),
                };
                if fits {
                    out.push((n.id.clone(), name));
                }
            }
        }
        let mut out = vec![(String::new(), "—".to_owned())];
        if let Some(doc) = &self.document {
            walk(doc.model.layers().nodes(), "", geometry, &mut out);
        }
        out
    }

    /// The fields a field picker offers: none ("—") first, then the fields of `layer` (of `kinds` when given), each by
    /// its alias and name; only none while no layer is chosen.
    pub(crate) fn panel_fields(
        &self,
        layer: Option<&str>,
        kinds: &[String],
    ) -> Vec<(String, String)> {
        let mut out = vec![(String::new(), "—".to_owned())];
        let node = layer.and_then(|l| self.document.as_ref()?.model.layers().get(l));
        for f in node.into_iter().flat_map(|n| &n.fields) {
            let kind = serde_json::to_value(f.kind).ok();
            if !kinds.is_empty()
                && !kinds
                    .iter()
                    .any(|k| kind.as_ref().and_then(Value::as_str) == Some(k))
            {
                continue;
            }
            let words = match f
                .alias
                .as_deref()
                .filter(|a| !a.trim().is_empty() && *a != f.name)
            {
                Some(alias) => format!("{alias} ({})", f.name),
                None => f.name.clone(),
            };
            out.push((f.name.clone(), words));
        }
        out
    }

    /// A table's rows, the first [`SHOWN_ROWS`]; empty until its button's command answered.
    fn panel_table<'a>(
        &self,
        p: &'a OpenPanel,
        id: &str,
        words: Option<&'a str>,
    ) -> Element<'a, Message> {
        use kentos_ui::widget::table::{Column as Col, Row, Table};
        let Some((columns, rows)) = p.tables.get(id) else {
            return column![
                label::caption(words.unwrap_or("Sonuçlar").to_owned()),
                label::muted("Düğmeye basınca komutun satırları burada listelenir."),
            ]
            .spacing(4)
            .into();
        };
        // No rows: no columns either (a command's rows name them), so no empty header bar.
        if rows.is_empty() {
            return column![
                label::caption(words.unwrap_or("Sonuçlar").to_owned()),
                label::muted("Satır yok."),
            ]
            .spacing(4)
            .into();
        }
        // A row's `uid` names its object: a click shows it; it is not a column.
        let shown: Vec<&String> = columns.iter().filter(|c| c.as_str() != "uid").collect();
        let mut table = Table::new(shown.iter().map(|c| Col::new((*c).clone()).width(Fill)))
            .empty("Satır yok.");
        for row in rows.iter().take(SHOWN_ROWS) {
            let mut line = Row::new(
                shown
                    .iter()
                    .map(|c| label::caption(cell_words(row.get(c.as_str()))).into()),
            );
            if let Some(uid) = row.get("uid").and_then(Value::as_str) {
                line = line.on_press(msg(Event::Panel(PanelEvent::Show(uid.to_owned()))));
            }
            table = table.push(line);
        }
        let mut out = column![
            label::caption(words.unwrap_or("Sonuçlar").to_owned()),
            table
        ]
        .spacing(4);
        if rows.len() > SHOWN_ROWS {
            out = out.push(label::muted(format!(
                "… {} satır daha",
                rows.len() - SHOWN_ROWS
            )));
        }
        out.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn panel() -> OpenPanel {
        let value = json!({
            "format": "kentos.ui.panel", "schemaVersion": 1, "title": "Yapı ara",
            "body": [
                { "type": "textField", "id": "sorgu", "label": "Yapı no", "value": " Y-0 " },
                { "type": "numberField", "id": "en-az", "label": "En az alan", "min": 0 },
                { "type": "select", "id": "durum", "label": "Durum",
                  "options": [{ "value": "hepsi", "label": "Hepsi" }, { "value": "numarasiz", "label": "Numarasız" }] },
                { "type": "checkbox", "id": "secili", "label": "Yalnız seçililer" },
                { "type": "button", "id": "ara", "label": "Ara", "command": "yapi-ara" },
                { "type": "table", "id": "sonuc" }
            ]
        });
        OpenPanel::new(
            "tr.ornek.vitrin",
            "yapi-ara",
            kentos_extensions::ui_panel::read(&value).unwrap(),
        )
    }

    #[test]
    fn pickers_follow_their_layer_picker_and_give_names_and_ids() {
        let value = json!({
            "format": "kentos.ui.panel", "schemaVersion": 1, "title": "Değer say",
            "body": [
                { "type": "layerPicker", "id": "katman", "label": "Katman" },
                { "type": "fieldPicker", "id": "alan", "label": "Alan", "layer": "katman" },
                { "type": "featurePicker", "id": "nesneler", "label": "Nesneler", "layer": "katman" },
                { "type": "featurePicker", "id": "bir", "label": "Bir nesne", "multiple": false },
                { "type": "button", "id": "say", "label": "Say", "command": "deger-say" }
            ]
        });
        let mut p = OpenPanel::new(
            "tr.ornek.vitrin",
            "deger-paneli",
            kentos_extensions::ui_panel::read(&value).unwrap(),
        );
        assert_eq!(
            p.params().unwrap(),
            json!({ "katman": null, "alan": null, "nesneler": [], "bir": null }),
            "nothing chosen: none, no objects"
        );
        p.choose("katman".into(), "parsel".into());
        p.choose("alan".into(), "ada".into());
        p.picks
            .insert("nesneler".into(), vec!["a".into(), "b".into()]);
        p.picks.insert("bir".into(), vec!["c".into()]);
        assert_eq!(
            p.params().unwrap(),
            json!({ "katman": "parsel", "alan": "ada", "nesneler": ["a", "b"], "bir": "c" })
        );
        // The same layer again keeps them; another clears what was chosen from the one before, not the free picker.
        p.choose("katman".into(), "parsel".into());
        assert_eq!(p.params().unwrap()["alan"], json!("ada"));
        p.choose("katman".into(), "yapi".into());
        assert_eq!(
            p.params().unwrap(),
            json!({ "katman": "yapi", "alan": null, "nesneler": [], "bir": "c" })
        );
    }

    #[test]
    fn enter_presses_the_primary_button_else_the_first() {
        let p = panel();
        assert_eq!(p.default_button(), Some("ara"), "the only button");
        let value = json!({
            "format": "kentos.ui.panel", "schemaVersion": 1, "title": "İki düğme",
            "body": [
                { "type": "textField", "id": "sorgu", "label": "Aranan" },
                { "type": "button", "id": "numarala", "label": "Numarala", "command": "a" },
                { "type": "button", "id": "ara", "label": "Ara", "command": "b", "primary": true }
            ]
        });
        let two = OpenPanel::new("x", "y", kentos_extensions::ui_panel::read(&value).unwrap());
        assert_eq!(
            two.default_button(),
            Some("ara"),
            "the primary one, wherever it is"
        );
    }

    #[test]
    fn the_values_are_the_commands_parameters_a_number_read_as_typed() {
        let mut p = panel();
        assert_eq!(
            p.params().unwrap(),
            json!({ "sorgu": "Y-0", "en-az": null, "durum": "hepsi", "secili": false })
        );
        p.texts.insert("en-az".into(), "12,5".into());
        assert_eq!(p.params().unwrap()["en-az"], json!(12.5));
        p.texts.insert("en-az".into(), "on iki".into());
        assert_eq!(
            p.params().unwrap_err(),
            "“En az alan”: “on iki” bir sayı değil."
        );
        p.texts.insert("en-az".into(), "-3".into());
        assert!(p.params().unwrap_err().contains("arasında olmalı"));
        assert_eq!(
            p.table_of(None).as_deref(),
            Some("sonuc"),
            "the panel's first table"
        );
    }
}
