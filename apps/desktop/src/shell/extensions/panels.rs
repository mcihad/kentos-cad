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
//! - **A running button shows how far it is:** its command's `ctx.progress` as a bar and words under the form, the
//!   button named, with Durdur (its caption stays as it is); a command that answers a message, not rows, leaves the
//!   table as it was.
//! - **A tree groups a command's rows** by the columns its definition names, each group with its count and open
//!   unless the user closed it; a leaf with a `uid` is an object.
//! - **A card writes a row's columns one under the other** (KentOS's info card: its title column heading it, its tag
//!   column beside, the others by name), where a table's columns do not fit; a card with a `uid` is pressed to show
//!   its object.
//! - **Enter presses the primary button** in a text or number field (else the panel's first button).
//! - **A value changed can run a command:** an input's `onChange` runs it with the panel's values (its rows to the
//!   first table); a change while a command runs waits for it, one rerun with the values then.
//! - **Pickers follow their layer picker:** a field picker lists the fields of the layer chosen there, a feature picker
//!   takes the selected objects of that layer when the user asks ("Seçimi al"); choosing another layer clears both.
//! - **Several panels stay open:** each in its own tab at the top of the Eklenti tab (of any extensions), what was
//!   typed in each kept and where it was scrolled; a panel opened again is shown, not opened twice. A panel's run
//!   answers it while another is shown; its tab marks it (running, or words not yet seen). Closing the shown panel
//!   shows the next. The panels scroll under their tabs, which stay in sight.

use std::collections::BTreeMap;

use iced::Task;
use kentos_extensions::manifest::PanelSide;
use kentos_extensions::ui_panel::{Component, PanelDefinition};
use serde_json::{Map, Value, json};

use crate::app::{App, Message};
use crate::exchange::words;

/// The least share of its side's open stacks the stack showing an opened panel has.
const PANEL_SHARE: f32 = 0.6;
/// The panels' scroller: one, showing the panel shown.
pub(super) const SCROLLER: &str = "extension-panel";

/// The panels' scroller to `y`: the panel shown as it was left (the scroller was the last panel's).
fn scroll_back(y: f32) -> Task<Message> {
    iced::widget::operation::scroll_to(
        iced::widget::Id::new(SCROLLER),
        iced::widget::scrollable::AbsoluteOffset {
            x: None,
            y: Some(y),
        },
    )
}

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
    /// Each table's (and tree's) columns and rows, from its button's command.
    pub(crate) tables: BTreeMap<String, (Vec<String>, Vec<Value>)>,
    /// A tree's closed groups: the tree's id and the group's values, joined by `/`.
    pub(crate) folded: std::collections::BTreeSet<String>,
    /// The panel's last words and their mark: done, stopped by the user, failed.
    pub(crate) said: Option<(words::Kind, String)>,
    /// The button (or the changed input) whose command runs now.
    pub(crate) running: Option<String>,
    /// An input with `onChange` changed while a command ran: its command runs again when that one ends.
    pub(crate) rerun: Option<String>,
    /// Its run answered while another panel was shown: its tab marks it until it is shown.
    pub(crate) unseen: bool,
    /// How far down it was scrolled: given back when it is shown again.
    pub(crate) scroll: f32,
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
    /// A date field's day chosen from the calendar (its words, GG.AA.YYYY; empty: cleared).
    Picked(String, String),
    Press(String),
    /// A table's row with a `uid`: its object selected and zoomed to.
    Show(String),
    /// A tree's group opened or closed (its key: the tree's id and the group's values).
    Fold(String),
    /// The open panel at this place in the tabs shown.
    Activate(usize),
    /// Its tab moved: from, to (where it goes once taken out).
    Move(usize, usize),
    /// The open panel at this place closed.
    Close(usize),
    /// The panel shown scrolled to here.
    Scrolled(f32),
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
            folded: std::collections::BTreeSet::new(),
            said: None,
            running: None,
            rerun: None,
            unseen: false,
            scroll: 0.0,
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
                Component::DateField { id, value, .. } => {
                    panel.texts.insert(
                        id.clone(),
                        value.as_deref().map(date_words).unwrap_or_default(),
                    );
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
                // An ISO date; none when empty; a text that is no day, or a day out of its bounds, said by its label.
                Component::DateField {
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
                    let Some(iso) = kentos_extensions::ui_panel::typed_date(text) else {
                        return Err(format!(
                            "“{label}”: “{text}” bir tarih değil (GG.AA.YYYY ya da YYYY-AA-GG yazın)."
                        ));
                    };
                    let early = min.as_deref().is_some_and(|m| iso.as_str() < m);
                    let late = max.as_deref().is_some_and(|m| iso.as_str() > m);
                    if early || late {
                        return Err(match (min, max) {
                            (Some(a), Some(b)) => format!(
                                "“{label}” {} ile {} arasında olmalı.",
                                date_words(a),
                                date_words(b)
                            ),
                            (Some(a), None) => {
                                format!("“{label}” en erken {} olabilir.", date_words(a))
                            }
                            (_, Some(b)) => format!("“{label}” en geç {} olabilir.", date_words(b)),
                            (None, None) => unreachable!(),
                        });
                    }
                    out.insert(id.clone(), json!(iso));
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
    pub(super) fn layer_of(&self, picker: Option<&str>) -> Option<&str> {
        picker
            .and_then(|l| self.choices.get(l))
            .map(String::as_str)
            .filter(|l| !l.is_empty())
    }

    /// What an output (a table, a tree, a card) says before its first rows, by what fills it: a button, else a change
    /// (a panel without a button said "Düğmeye basınca", real window, 15 November 2026).
    pub(super) fn waiting_words(&self, output: &str, done: &str) -> String {
        let first = self.table_of(None);
        let by_button = self.definition.body.iter().any(|c| {
            matches!(c, Component::Button { table, .. }
                if table.as_deref().map_or(first.as_deref() == Some(output), |t| t == output))
        });
        let by = if by_button {
            "Düğmeye basınca"
        } else {
            "Değer değişince"
        };
        format!("{by} komutun satırları burada {done}.")
    }

    /// The output (a table, a tree, a card) a button's rows go to: its own, else the panel's first.
    pub(super) fn table_of(&self, table: Option<&String>) -> Option<String> {
        table.cloned().or_else(|| {
            self.definition.body.iter().find_map(|c| match c {
                Component::Table { id, .. }
                | Component::Tree { id, .. }
                | Component::Card { id, .. } => Some(id.clone()),
                _ => None,
            })
        })
    }
}

/// An ISO date as a field shows it, GG.AA.YYYY; another text as it is.
fn date_words(iso: &str) -> String {
    match kentos_extensions::ui_panel::iso_date(iso) {
        Some((y, m, d)) => format!("{d:02}.{m:02}.{y:04}"),
        None => iso.to_owned(),
    }
}

/// A number as a field shows it: a whole one without decimals, a decimal with a comma.
pub(super) fn number_words(n: f64) -> String {
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

impl super::Extensions {
    /// The panel the Eklenti tab shows.
    pub(crate) fn panel(&self) -> Option<&OpenPanel> {
        self.panels.get(self.active_panel)
    }

    #[cfg(test)]
    pub(crate) fn panel_mut(&mut self) -> Option<&mut OpenPanel> {
        self.panels.get_mut(self.active_panel)
    }

    /// The open panel a run started from; its tab marks the answer when another panel is shown.
    fn answered(&mut self, origin: &PanelOrigin) -> Option<&mut OpenPanel> {
        let shown = self.active_panel;
        let (at, panel) = self
            .panels
            .iter_mut()
            .enumerate()
            .find(|(_, p)| p.ext == origin.ext && p.local == origin.panel)?;
        panel.unseen = at != shown;
        Some(panel)
    }
}

impl App {
    /// An extension's panel from its tab: its definition read from the package (checked when it was installed) and
    /// shown in the dock's Eklenti tab, in a tab of its own beside the panels open there. A panel open already is
    /// shown, what was typed in it kept.
    pub(crate) fn ext_panel_open(&mut self, ext: &str, local: &str) -> Task<Message> {
        if let Some(at) = self
            .extensions
            .panels
            .iter()
            .position(|p| p.ext == ext && p.local == local)
        {
            let back = self.ext_panel_activate(at);
            return Task::batch([back, self.show_extension_panel()]);
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
        self.extensions.panels.push(panel);
        self.extensions.active_panel = self.extensions.panels.len() - 1;
        Task::batch([scroll_back(0.0), self.show_extension_panel()])
    }

    /// The open panel at `at` shown, scrolled as it was left; its answer seen.
    fn ext_panel_activate(&mut self, at: usize) -> Task<Message> {
        let Some(panel) = self.extensions.panels.get_mut(at) else {
            return Task::none();
        };
        panel.unseen = false;
        let y = panel.scroll;
        self.extensions.active_panel = at;
        scroll_back(y)
    }

    /// A panel's tab moved among the others; the panel shown stays shown.
    fn ext_panel_move(&mut self, from: usize, to: usize) {
        let x = &mut self.extensions;
        if from >= x.panels.len() || to >= x.panels.len() {
            return;
        }
        let panel = x.panels.remove(from);
        x.panels.insert(to, panel);
        let shown = x.active_panel;
        x.active_panel = if shown == from {
            to
        } else if from < shown && shown <= to {
            shown - 1
        } else if to <= shown && shown < from {
            shown + 1
        } else {
            shown
        };
    }

    /// The open panel at `at` closed (a run it started answers nowhere); the next one shown, else the one before,
    /// scrolled as it was left.
    fn ext_panel_close(&mut self, at: usize) -> Task<Message> {
        let x = &mut self.extensions;
        if at >= x.panels.len() {
            return Task::none();
        }
        x.panels.remove(at);
        if at < x.active_panel || (x.active_panel >= x.panels.len() && x.active_panel > 0) {
            x.active_panel -= 1;
        }
        match x.panels.get_mut(x.active_panel) {
            Some(panel) => {
                panel.unseen = false;
                scroll_back(panel.scroll)
            }
            None => Task::none(),
        }
    }

    /// The panels of extensions stopped since (disabled, removed, the registry gone) closed, with the calls their
    /// buttons left; the dock's tab too when none is left.
    pub(crate) fn ext_panels_of_stopped_go(&mut self) {
        let x = &mut self.extensions;
        let before = x.panels.len();
        let shown = x.panel().map(|p| (p.ext.clone(), p.local.clone()));
        let registry = x.registry.as_ref();
        let live = |ext: &str| registry.is_some_and(|r| r.lease(ext).is_some());
        x.panels.retain(|p| live(&p.ext));
        x.panel_calls.retain(|c| live(&c.ext));
        if x.panels.len() == before {
            return;
        }
        x.active_panel = shown
            .and_then(|(e, l)| x.panels.iter().position(|p| p.ext == e && p.local == l))
            .unwrap_or_else(|| x.active_panel.min(x.panels.len().saturating_sub(1)));
        if x.panels.is_empty() {
            x.panel_placed = None;
            self.docks.close(crate::app::Panel::Extension);
        }
    }

    /// The dock shows the Eklenti tab on the side the panel shown asks for in its manifest, the one opened last (right:
    /// beside Katmanlar, the side panels shown if F4 hid them; left: beside the map, Katmanlar staying in sight;
    /// bottom: under the map), its stack at least [`PANEL_SHARE`] of the side. A tab KentOS docked moves when a panel
    /// opened asks for another side; one the user moved stays where it is. Choosing among the open panels' tabs moves
    /// nothing.
    pub(crate) fn show_extension_panel(&mut self) -> Task<Message> {
        use crate::app::Panel;
        use kentos_ui::widget::docking::{Event as Dock, Side, Slot, Target};
        if !self.open_panel(Panel::Extension) {
            return Task::none();
        }
        let wanted = match self.extensions.panel().map_or(PanelSide::Right, |p| p.side) {
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
        // Room to work in: the stack showing the panel takes at least three fifths of its side (with a form and its
        // cards it had a third under Öznitelikler and was used by scrolling, real window, 15 November 2026). Only more:
        // a stack the user made taller keeps its share.
        if let Some(Slot::Docked(side, index)) = self.docks.slot(Panel::Extension) {
            let stacks = self.docks.stacks(side);
            let others: f32 = stacks
                .iter()
                .enumerate()
                .filter(|(i, s)| *i != index && !s.collapsed)
                .map(|(_, s)| s.weight)
                .sum();
            let wanted = PANEL_SHARE / (1.0 - PANEL_SHARE) * others;
            if others > 0.0 && stacks.get(index).is_some_and(|s| s.weight < wanted) {
                let mut weights: Vec<f32> = stacks.iter().map(|s| s.weight).collect();
                weights[index] = wanted;
                self.docks.update(Dock::Shared(side, weights));
            }
        }
        Task::none()
    }

    /// What the user does in the panel shown, or with the open panels' tabs.
    pub(crate) fn ext_panel_event(&mut self, event: PanelEvent) -> Task<Message> {
        let at = self.extensions.active_panel;
        let Some(panel) = self.extensions.panels.get_mut(at) else {
            return Task::none();
        };
        match event {
            PanelEvent::Text(id, text) => {
                panel.texts.insert(id, text);
            }
            PanelEvent::Check(id, on) => {
                panel.checks.insert(id.clone(), on);
                return self.ext_panel_changed(at, &id);
            }
            PanelEvent::Choose(id, value) => {
                panel.choose(id.clone(), value);
                return self.ext_panel_changed(at, &id);
            }
            PanelEvent::Unpick(id) => {
                panel.picks.insert(id.clone(), Vec::new());
                return self.ext_panel_changed(at, &id);
            }
            // A day chosen from the calendar is a change (typing is not: a process each key, as a text field's).
            PanelEvent::Picked(id, words) => {
                panel.texts.insert(id.clone(), words);
                return self.ext_panel_changed(at, &id);
            }
            PanelEvent::Pick(id) => return self.ext_panel_pick(at, &id),
            PanelEvent::Activate(to) => return self.ext_panel_activate(to),
            PanelEvent::Move(from, to) => self.ext_panel_move(from, to),
            PanelEvent::Close(which) => return self.ext_panel_close(which),
            PanelEvent::Scrolled(y) => panel.scroll = y,
            PanelEvent::Press(button) => return self.ext_panel_press(at, &button),
            PanelEvent::Fold(key) => {
                if !panel.folded.remove(&key) {
                    panel.folded.insert(key);
                }
            }
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
    fn ext_panel_pick(&mut self, at: usize, picker: &str) -> Task<Message> {
        let Some(panel) = self.extensions.panels.get(at) else {
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
        let Some(panel) = self.extensions.panels.get_mut(at) else {
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
        self.ext_panel_changed(at, picker)
    }

    /// A button: its command run with the panel's values, its rows to its table.
    fn ext_panel_press(&mut self, at: usize, button: &str) -> Task<Message> {
        let found = self
            .extensions
            .panels
            .get(at)
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
            Some((command, table)) => self.ext_panel_start(at, button, command, table),
            None => Task::none(),
        }
    }

    /// An input changed: its `onChange` command runs with the panel's values, its rows to the panel's first table. A
    /// change while a command runs waits for it: one rerun, with the values then.
    fn ext_panel_changed(&mut self, at: usize, input: &str) -> Task<Message> {
        let Some(panel) = self.extensions.panels.get_mut(at) else {
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
        self.ext_panel_start(at, input, command, None)
    }

    /// The changes that came while their panels' commands ran: each command now, with the values now (runner.rs, when
    /// a run ends).
    pub(crate) fn ext_panel_rerun(&mut self) -> Task<Message> {
        let waiting: Vec<(usize, String)> = self
            .extensions
            .panels
            .iter_mut()
            .enumerate()
            .filter(|(_, p)| p.running.is_none())
            .filter_map(|(at, p)| p.rerun.take().map(|input| (at, input)))
            .collect();
        let mut tasks = Vec::new();
        for (at, input) in waiting {
            tasks.push(self.ext_panel_changed(at, &input));
        }
        Task::batch(tasks)
    }

    /// The panel's `command` started by `trigger` (a button, a changed input) with the panel's values; a number that is
    /// not one, or Python not ready, said in the panel and nothing run.
    fn ext_panel_start(
        &mut self,
        at: usize,
        trigger: &str,
        command: String,
        table: Option<String>,
    ) -> Task<Message> {
        let ext = match self.extensions.panels.get(at) {
            Some(p) if p.running.is_none() => p.ext.clone(),
            _ => return Task::none(),
        };
        let why = self
            .extensions
            .python_reasons
            .get(&ext)
            .and_then(Option::as_deref)
            .map(str::to_owned);
        let Some(panel) = self.extensions.panels.get_mut(at) else {
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
        self.extensions.panel_calls.push(PanelCall {
            ext: ext.clone(),
            command: command.clone(),
            params,
            origin,
        });
        self.ext_run(&ext, "command", &command)
    }

    /// The first panel call of `ext`'s `command`, taken for its launch (runner.rs); none for another command's. A
    /// call carries its values and its panel, so two panels' calls of one command are answered right whichever launch
    /// takes which.
    pub(crate) fn ext_panel_take(&mut self, ext: &str, command: &str) -> Option<PanelCall> {
        let at = self
            .extensions
            .panel_calls
            .iter()
            .position(|c| c.ext == ext && c.command == command)?;
        Some(self.extensions.panel_calls.remove(at))
    }

    /// A panel call that did not start: the panel says why and its button can be pressed again.
    pub(crate) fn ext_panel_dropped(&mut self, ext: &str, command: &str, why: &str) {
        if let Some(call) = self.ext_panel_take(ext, command) {
            self.ext_panel_answer(&call.origin, Err(why.to_owned()));
        }
    }

    /// A run started from a panel ended (runner.rs): its rows into its table and its words under the panel, or why
    /// it failed, whether it is shown or not; nothing when the panel it started from is closed.
    pub(crate) fn ext_panel_answer(
        &mut self,
        origin: &PanelOrigin,
        outcome: Result<Value, String>,
    ) {
        let Some(panel) = self.extensions.answered(origin) else {
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
        if let Some(panel) = self.extensions.answered(origin) {
            panel.running = None;
            // A change that waited for it does not start it again: the user stopped it.
            panel.rerun = None;
            panel.said = Some((
                words::Kind::Info,
                "Durduruldu; çizime yazdıkları geri alındı.".to_owned(),
            ));
        }
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

    /// A date field's value is the ISO date of what was typed, either way it was written; a text that is no day, and
    /// a day out of its bounds, are said by its label; empty is none.
    #[test]
    fn a_date_field_gives_the_iso_date_of_what_was_typed() {
        let value = json!({
            "format": "kentos.ui.panel", "schemaVersion": 1, "title": "Tarih",
            "body": [{ "type": "dateField", "id": "tarih", "label": "Kart tarihi", "value": "2026-11-15",
                       "min": "2026-01-01", "max": "2026-12-31" }]
        });
        let mut p = OpenPanel::new(
            "tr.ornek.vitrin",
            "tarih",
            kentos_extensions::ui_panel::read(&value).unwrap(),
        );
        assert_eq!(
            p.texts["tarih"], "15.11.2026",
            "its value as the field shows it"
        );
        assert_eq!(p.params().unwrap(), json!({ "tarih": "2026-11-15" }));
        p.texts.insert("tarih".into(), "3.2.2026".into());
        assert_eq!(p.params().unwrap(), json!({ "tarih": "2026-02-03" }));
        p.texts.insert("tarih".into(), "30.02.2026".into());
        assert_eq!(
            p.params().unwrap_err(),
            "“Kart tarihi”: “30.02.2026” bir tarih değil (GG.AA.YYYY ya da YYYY-AA-GG yazın)."
        );
        p.texts.insert("tarih".into(), "2025-12-31".into());
        assert_eq!(
            p.params().unwrap_err(),
            "“Kart tarihi” 01.01.2026 ile 31.12.2026 arasında olmalı."
        );
        p.texts.insert("tarih".into(), " ".into());
        assert_eq!(p.params().unwrap(), json!({ "tarih": null }));
    }

    #[test]
    fn a_card_takes_a_commands_rows_where_a_table_would() {
        let value = json!({
            "format": "kentos.ui.panel", "schemaVersion": 1, "title": "Parsel kartı",
            "body": [
                { "type": "featurePicker", "id": "parseller", "label": "Parseller", "onChange": "parsel-karti" },
                { "type": "card", "id": "kartlar", "title": "parsel", "tag": "durum" }
            ]
        });
        let p = OpenPanel::new(
            "tr.ornek.vitrin",
            "parsel-paneli",
            kentos_extensions::ui_panel::read(&value).unwrap(),
        );
        assert_eq!(
            p.table_of(None).as_deref(),
            Some("kartlar"),
            "the panel's first output"
        );
        assert_eq!(panel().table_of(None).as_deref(), Some("sonuc"));
        // Before its first rows it says what fills it: no button here, a change.
        assert_eq!(
            p.waiting_words("kartlar", "kart kart listelenir"),
            "Değer değişince komutun satırları burada kart kart listelenir."
        );
        assert_eq!(
            panel().waiting_words("sonuc", "listelenir"),
            "Düğmeye basınca komutun satırları burada listelenir."
        );
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
