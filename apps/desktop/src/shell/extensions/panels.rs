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
/// A split panel's outputs' scroller, under its sash (docs/adr/8057 §69).
pub(super) const OUTPUTS: &str = "extension-panel-outputs";

/// The panels' scrollers to `y` and `outputs`: the panel shown as it was left (the scrollers were the last panel's).
fn scroll_back(y: f32, outputs: f32) -> Task<Message> {
    let to = |id: &'static str, y: f32| {
        iced::widget::operation::scroll_to(
            iced::widget::Id::new(id),
            iced::widget::scrollable::AbsoluteOffset {
                x: None,
                y: Some(y),
            },
        )
    };
    Task::batch([to(SCROLLER, y), to(OUTPUTS, outputs)])
}

/// A panel open in the dock.
#[derive(Clone, Debug)]
pub(crate) struct OpenPanel {
    /// Its own number, new each time it opens: a run's answer reaches the panel it started from, not one opened again
    /// after it closed (PY-UI-17, docs/adr/8057 §117).
    pub(crate) instance: u64,
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
    /// Each multiple choice's chosen values.
    pub(crate) multi: BTreeMap<String, Vec<String>>,
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
    /// The input that was wrong when a command was asked for, and why: said under it until it is changed.
    pub(crate) invalid: Option<(String, String)>,
    /// What its running code changed of it (panel_session.rs, docs/adr/8057 §32): a choice's or a multiple choice's
    /// options, a text's words, the components it hid or turned off.
    pub(crate) options: BTreeMap<String, Vec<kentos_extensions::ui_panel::Choice>>,
    pub(crate) words: BTreeMap<String, String>,
    pub(crate) hidden: std::collections::BTreeSet<String>,
    pub(crate) disabled: std::collections::BTreeSet<String>,
    /// The inputs the user changed while its command ran: a value the code sets there does not go over the user's.
    pub(crate) touched: std::collections::BTreeSet<String>,
    /// Its running code said something under it: kept when the run ends without words of its own.
    pub(crate) spoke: bool,
    /// A running code of its extension opened it (`ctx.open_panel`) and may change it until the run ends: the inputs
    /// the user changes meanwhile are the user's, as while its own command runs.
    pub(crate) held: bool,
    /// Each table's and card component's page shown (panel_view.rs `pager`); its first when its rows come again.
    pub(crate) pages: BTreeMap<String, usize>,
    /// Each row component's keyboard cursor (docs/adr/8057 §71): the row ↑ and ↓ reached, from the first of all its
    /// rows; Enter shows its object. Gone with the rows it was on.
    pub(crate) cursor: BTreeMap<String, usize>,
    /// Each component's rules compiled (`visibleWhen`, `requiredWhen`, `enabledWhen`; docs/adr/8057 §33), in the
    /// body's order: worked out from the panel's values as they change, no Python.
    pub(crate) rules: Vec<CompiledRules>,
    /// The calculated components' places in the order they are worked out, each after those it reads
    /// (panel_values.rs, docs/adr/8057 §35).
    pub(crate) calc_order: Vec<usize>,
    /// What each calculated text shows now, by its place (none: its `text`).
    pub(crate) calculated: BTreeMap<usize, String>,
    /// The sections folded now, by place (`section`: its `collapsed` at first, then the user's; panel_view.rs).
    pub(crate) closed: std::collections::BTreeSet<usize>,
    /// The tab the user chose (panel_tabs.rs, docs/adr/8057 §43); none: the first.
    pub(crate) tab: Option<String>,
    /// Each `image` component's picture its code gave (panel_image.rs), checked and decoded.
    pub(crate) images: BTreeMap<String, iced::widget::image::Handle>,
    /// A dialog shown as a wizard (`ctx.dialog(…, steps=True)`, dialogs.rs): its tabs are the steps, no tab bar.
    pub(crate) steps: bool,
    /// Each shape picker's drawn shape, as GeoJSON (panel_points.rs, docs/adr/8057 §62).
    pub(crate) geoms: BTreeMap<String, Value>,
    /// A split panel's form height as the user dragged its sash; none: the default (docs/adr/8057 §69).
    pub(crate) split: Option<f32>,
    /// How far down a split panel's outputs were scrolled: given back when it is shown again.
    pub(crate) out_scroll: f32,
}

/// A component's rules, compiled against the panel's inputs (Kapı 1 passed them; one that does not compile is none).
#[derive(Clone, Debug, Default)]
pub(crate) struct CompiledRules {
    pub(crate) visible: Option<kentos_expression::Expr>,
    pub(crate) required: Option<kentos_expression::Expr>,
    pub(crate) enabled: Option<kentos_expression::Expr>,
    /// Its value worked out from the inputs (`calculate`, panel_values.rs).
    pub(crate) calculate: Option<kentos_expression::Expr>,
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
    /// The open panel's own number ([`OpenPanel::instance`]): a panel closed while its command ran and opened again is
    /// another, which the run's answer does not reach (docs/adr/8057 §117).
    pub(crate) instance: u64,
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
    /// A date, time or date-time field's value chosen from its picker, or typed and applied with Enter (its words,
    /// as the field shows them; empty: cleared).
    Picked(String, String),
    /// A multiple choice's option chosen or let go: (its id, the option's value).
    Toggle(String, String),
    Press(String),
    /// A table's row with a `uid`: its object selected and zoomed to.
    Show(String),
    /// A tree's group opened or closed (its key: the tree's id and the group's values).
    Fold(String),
    /// A table's or a card component's page of rows shown (its id, the page from 0).
    Page(String, usize),
    /// A row component's keyboard cursor moved to a row (its id, the row from the first of all its rows, and the page
    /// that holds it, shown).
    Cursor(String, usize, usize),
    /// A section opened or folded, by its place.
    Section(usize),
    /// A tab's page shown, by its words.
    Tab(String),
    /// A file picker's Seç…: the computer's file window, its choice as `Picked`.
    PickFile(String),
    /// A point picker's Haritadan seç: a point picked on the map, the field's words then (panel_points.rs).
    PickPoint(String),
    /// A shape picker's Haritada çiz: a shape drawn on the map (panel_points.rs).
    DrawShape(String),
    /// A shape picker's ✕: its shape let go.
    ClearShape(String),
    /// A number field's − (−1) or + (1): its value by its step.
    Step(String, i8),
    /// The open panel at this place in the tabs shown.
    Activate(usize),
    /// Its tab moved: from, to (where it goes once taken out).
    Move(usize, usize),
    /// The open panel at this place closed.
    Close(usize),
    /// The panel shown scrolled to here.
    Scrolled(f32),
    /// A split panel's outputs scrolled to here (docs/adr/8057 §69).
    ScrolledOutputs(f32),
    /// A split panel's sash dragged: the form's height.
    Split(f32),
    /// A split panel's sash double-clicked: the form's default height.
    SplitReset,
}

impl OpenPanel {
    /// The panel as its definition starts it.
    pub(crate) fn new(ext: &str, local: &str, definition: PanelDefinition) -> OpenPanel {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let mut panel = OpenPanel {
            instance: NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            ext: ext.to_owned(),
            local: local.to_owned(),
            definition,
            side: PanelSide::Right,
            texts: BTreeMap::new(),
            checks: BTreeMap::new(),
            choices: BTreeMap::new(),
            picks: BTreeMap::new(),
            multi: BTreeMap::new(),
            tables: BTreeMap::new(),
            folded: std::collections::BTreeSet::new(),
            said: None,
            running: None,
            rerun: None,
            unseen: false,
            scroll: 0.0,
            invalid: None,
            options: BTreeMap::new(),
            words: BTreeMap::new(),
            hidden: std::collections::BTreeSet::new(),
            disabled: std::collections::BTreeSet::new(),
            touched: std::collections::BTreeSet::new(),
            spoke: false,
            held: false,
            pages: BTreeMap::new(),
            cursor: BTreeMap::new(),
            rules: Vec::new(),
            calc_order: Vec::new(),
            calculated: BTreeMap::new(),
            closed: std::collections::BTreeSet::new(),
            tab: None,
            images: BTreeMap::new(),
            steps: false,
            geoms: BTreeMap::new(),
            split: None,
            out_scroll: 0.0,
        };
        // The sections folded when it opens.
        for (at, c) in panel.definition.body.iter().enumerate() {
            if let Component::Section {
                collapsed: true, ..
            } = c
            {
                panel.closed.insert(at);
            }
        }
        // Its rules, compiled once against its inputs (ui_panel::rules).
        let body: Vec<Value> = panel
            .definition
            .body
            .iter()
            .filter_map(|c| serde_json::to_value(c).ok())
            .collect();
        let inputs = kentos_extensions::ui_panel::rules::inputs_of(&body);
        let compile = |source: &Option<String>| {
            source
                .as_deref()
                .and_then(|s| kentos_extensions::ui_panel::rules::compile(s, &inputs).ok())
        };
        // Its worked-out values read the moment it opened (`bugün()`, `şimdi()`).
        let moment = super::panel_values::moment_now();
        let worked = |source: &Option<String>| {
            source.as_deref().and_then(|s| {
                kentos_extensions::ui_panel::values::compile(s, &inputs, &moment).ok()
            })
        };
        panel.rules = panel
            .definition
            .rules
            .iter()
            .map(|r| CompiledRules {
                visible: compile(&r.visible_when),
                required: compile(&r.required_when),
                enabled: compile(&r.enabled_when),
                calculate: worked(&r.calculate),
            })
            .collect();
        let reads: Vec<(Option<String>, Vec<String>)> = panel
            .rules
            .iter()
            .zip(&panel.definition.body)
            .filter_map(|(r, c)| {
                Some((
                    c.id().map(str::to_owned),
                    r.calculate.as_ref()?.fields.clone(),
                ))
            })
            .collect();
        let places: Vec<usize> = (0..panel.rules.len())
            .filter(|at| panel.rules[*at].calculate.is_some())
            .collect();
        panel.calc_order = kentos_extensions::ui_panel::values::calc_order(&reads)
            .map(|order| order.into_iter().map(|i| places[i]).collect())
            .unwrap_or_default();
        for c in &panel.definition.body {
            match c {
                Component::TextField { id, value, .. } => {
                    panel
                        .texts
                        .insert(id.clone(), value.clone().unwrap_or_default());
                }
                // A file's path, typed or chosen: none at first.
                Component::FilePicker { id, .. } => {
                    panel.texts.insert(id.clone(), String::new());
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
                Component::TimeField { id, value, .. } => {
                    panel
                        .texts
                        .insert(id.clone(), value.clone().unwrap_or_default());
                }
                Component::DateTimeField { id, value, .. } => {
                    panel.texts.insert(
                        id.clone(),
                        value.as_deref().map(date_time_words).unwrap_or_default(),
                    );
                }
                Component::MultiSelect { id, value, .. } => {
                    panel.multi.insert(id.clone(), value.clone());
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
                // Its own value; none waits for the drawing's system (panel_crs.rs `default_crs`).
                Component::CrsPicker { id, value, .. } => {
                    panel
                        .choices
                        .insert(id.clone(), value.clone().unwrap_or_default());
                }
                // A point as typed, to the millimetre (panel_points.rs, docs/adr/8057 §57).
                Component::PointPicker { id, value, .. } => {
                    panel.texts.insert(
                        id.clone(),
                        value
                            .map(kentos_extensions::ui_panel::point_words)
                            .unwrap_or_default(),
                    );
                }
                _ => {}
            }
        }
        // Its defaults on the values it starts with, then its calculations on those.
        panel.apply_defaults(&moment);
        panel.recalculate();
        panel
    }

    /// The command's parameters: each input's value by its id (a text trimmed, a number read with a decimal comma or
    /// point, none when empty); or which input is wrong and why, said by its label: one the user must give left empty
    /// (`required`), a number that is not one, a value out of bounds. The field's id goes with the words: the form
    /// says them under it.
    pub(crate) fn check_params(&self) -> Result<Value, (String, String)> {
        let mut out = Map::new();
        for (at, c) in self.definition.body.iter().enumerate() {
            let id = c.id().unwrap_or_default().to_owned();
            // One its code or its rule hid is not asked of the user (its value still goes).
            if self.shown(at, c)
                && self.asked(at, c)
                && let Some(why) = self.missing(c)
            {
                return Err((id, why));
            }
            self.read_into(c, &mut out).map_err(|why| (id, why))?;
        }
        Ok(Value::Object(out))
    }

    /// Like [`OpenPanel::check_params`], for the inputs on page `page` alone (a wizard's step and the components on
    /// every page): which is wrong and why, said by its label; none when they are right.
    pub(crate) fn check_page(&self, page: &str) -> Result<(), (String, String)> {
        let mut out = Map::new();
        for (at, c) in self.definition.body.iter().enumerate() {
            if !self.on_page(at, Some(page)) {
                continue;
            }
            let id = c.id().unwrap_or_default().to_owned();
            if self.shown(at, c)
                && self.asked(at, c)
                && let Some(why) = self.missing(c)
            {
                return Err((id, why));
            }
            self.read_into(c, &mut out).map_err(|why| (id, why))?;
        }
        Ok(())
    }

    /// The wrong input changed by `event`: its words under it go (the next ask checks it again). While its command
    /// runs, the input is the user's: the code's value does not go over it (panel_session.rs).
    pub(crate) fn forget_invalid(&mut self, event: &PanelEvent) {
        let changed = match event {
            PanelEvent::Text(id, _)
            | PanelEvent::Check(id, _)
            | PanelEvent::Choose(id, _)
            | PanelEvent::Picked(id, _)
            | PanelEvent::Toggle(id, _)
            | PanelEvent::Pick(id)
            | PanelEvent::Unpick(id)
            | PanelEvent::ClearShape(id) => Some(id),
            _ => None,
        };
        if changed.is_some_and(|id| self.invalid.as_ref().is_some_and(|(at, _)| at == id)) {
            // The words under the form were that field's: they go with it, not left saying what is no longer so
            // (a dialog's "dosya verilmedi" stayed under a typed path, real window, 15 November 2026).
            if let (Some((_, why)), Some((words::Kind::Error, said))) = (&self.invalid, &self.said)
                && said == why
            {
                self.said = None;
            }
            self.invalid = None;
        }
        if let Some(id) = changed
            && (self.running.is_some() || self.held)
        {
            self.touched.insert(id.clone());
        }
    }

    /// Whether component `at` is shown: its `visibleWhen` holds (or it has none), its code did not hide it, and the
    /// section it is in is shown (a section hidden hides what it gathers). A folded section's components are still
    /// shown, only not drawn ([`OpenPanel::drawn`]): folding is a view, not a rule, and what they hold is still asked.
    pub(crate) fn shown(&self, at: usize, c: &Component) -> bool {
        let own = self.holds(at, |r| r.visible.as_ref()).unwrap_or(true)
            && c.id().is_none_or(|id| !self.hidden.contains(id));
        own && self
            .section_of(at)
            .is_none_or(|s| self.shown(s, &self.definition.body[s]))
    }

    /// Whether component `at` can be used (a button pressed, a field typed in): its `enabledWhen` holds (or it has
    /// none) and its code did not turn it off.
    pub(crate) fn usable(&self, at: usize, c: &Component) -> bool {
        self.holds(at, |r| r.enabled.as_ref()).unwrap_or(true)
            && c.id().is_none_or(|id| !self.disabled.contains(id))
            && !self.calculated_at(at)
    }

    /// Whether input `at` is asked of the user: `required`, or its `requiredWhen` holding.
    pub(crate) fn asked(&self, at: usize, c: &Component) -> bool {
        c.required() || self.holds(at, |r| r.required.as_ref()) == Some(true)
    }

    /// Whether a shown input it asks for has no value yet: a change's command waits for it (docs/adr/8057 §62).
    pub(crate) fn waits_for_value(&self) -> bool {
        self.definition
            .body
            .iter()
            .enumerate()
            .any(|(at, c)| self.shown(at, c) && self.asked(at, c) && self.missing(c).is_some())
    }

    /// A rule of component `at` worked out from the panel's values; none when it has no such rule.
    fn holds(
        &self,
        at: usize,
        which: impl Fn(&CompiledRules) -> Option<&kentos_expression::Expr>,
    ) -> Option<bool> {
        let expr = self.rules.get(at).and_then(which)?;
        Some(kentos_extensions::ui_panel::rules::holds(expr, |id| {
            self.rule_value(id)
        }))
    }

    /// Input `id`'s value as a rule reads it (ui_panel::rules): a number as `4.5`, a check `true` or `false`, a date
    /// or moment ISO, a multiple choice's chosen values joined; none when empty.
    pub(super) fn rule_value(&self, id: &str) -> Option<String> {
        let typed = || {
            self.texts
                .get(id)
                .map(|t| t.trim())
                .filter(|t| !t.is_empty())
        };
        let joined =
            |list: Option<&Vec<String>>| list.filter(|l| !l.is_empty()).map(|l| l.join(", "));
        match self.definition.body.iter().find(|c| c.id() == Some(id))? {
            Component::TextField { .. } | Component::FilePicker { .. } => {
                typed().map(str::to_owned)
            }
            Component::PointPicker { .. } => typed()
                .and_then(kentos_extensions::ui_panel::typed_point)
                .map(kentos_extensions::ui_panel::point_words),
            Component::ShapePicker { .. } => {
                self.geoms.get(id).map(super::panel_points::shape_words)
            }
            Component::RelationPicker { .. } => {
                self.choices.get(id).filter(|c| !c.is_empty()).cloned()
            }
            Component::NumberField { .. } => typed()
                .and_then(|t| self.typed_number(id, t))
                .map(|n| n.to_string()),
            Component::DateField { .. } => {
                typed().and_then(kentos_extensions::ui_panel::typed_date)
            }
            Component::TimeField { .. } => {
                typed().and_then(kentos_extensions::ui_panel::typed_time)
            }
            Component::DateTimeField { .. } => {
                typed().and_then(kentos_extensions::ui_panel::typed_date_time)
            }
            Component::Checkbox { .. } => {
                Some(self.checks.get(id).copied().unwrap_or(false).to_string())
            }
            Component::Select { .. }
            | Component::LayerPicker { .. }
            | Component::FieldPicker { .. }
            | Component::CrsPicker { .. } => {
                self.choices.get(id).filter(|v| !v.is_empty()).cloned()
            }
            Component::MultiSelect { .. } => joined(self.multi.get(id)),
            Component::FeaturePicker { .. } => joined(self.picks.get(id)),
            _ => None,
        }
    }

    /// A choice's or a multiple choice's options now: its code's, else its definition's.
    pub(crate) fn options_of<'a>(
        &'a self,
        id: &str,
        defined: &'a [kentos_extensions::ui_panel::Choice],
    ) -> &'a [kentos_extensions::ui_panel::Choice] {
        self.options.get(id).map_or(defined, Vec::as_slice)
    }

    /// The command's parameters, or why not in words ([`OpenPanel::check_params`]).
    #[cfg(test)]
    pub(crate) fn params(&self) -> Result<Value, String> {
        self.check_params().map_err(|(_, why)| why)
    }

    /// Why an input asked for (`required`, or its `requiredWhen` holding) is not given, in its label's words; none
    /// when it is.
    fn missing(&self, c: &Component) -> Option<String> {
        let id = c.id()?;
        let label = match c {
            Component::TextField { label, .. }
            | Component::NumberField { label, .. }
            | Component::LayerPicker { label, .. }
            | Component::FieldPicker { label, .. }
            | Component::FeaturePicker { label, .. }
            | Component::MultiSelect { label, .. }
            | Component::DateField { label, .. }
            | Component::TimeField { label, .. }
            | Component::DateTimeField { label, .. }
            | Component::FilePicker { label, .. }
            | Component::CrsPicker { label, .. }
            | Component::PointPicker { label, .. }
            | Component::ShapePicker { label, .. }
            | Component::RelationPicker { label, .. } => label,
            _ => return None,
        };
        match c {
            Component::LayerPicker { .. }
            | Component::FieldPicker { .. }
            | Component::CrsPicker { .. } => self
                .choices
                .get(id)
                .is_none_or(String::is_empty)
                .then(|| format!("“{label}” seçilmedi.")),
            Component::FeaturePicker { .. } => {
                self.picks.get(id).is_none_or(Vec::is_empty).then(|| {
                    format!("“{label}”: nesne alınmadı (çizimde seçip Seçimi al'a basın).")
                })
            }
            Component::MultiSelect { .. } => self
                .multi
                .get(id)
                .is_none_or(Vec::is_empty)
                .then(|| format!("“{label}”: en az biri seçilmeli.")),
            Component::FilePicker { .. } => self
                .texts
                .get(id)
                .is_none_or(|t| t.trim().is_empty())
                .then(|| format!("“{label}”: dosya verilmedi (yolunu yazın ya da Seç…'e basın).")),
            Component::PointPicker { .. } => self
                .texts
                .get(id)
                .is_none_or(|t| t.trim().is_empty())
                .then(|| {
                    format!("“{label}”: nokta verilmedi (x, y yazın ya da Haritadan seç'e basın).")
                }),
            Component::ShapePicker { .. } => (!self.geoms.contains_key(id))
                .then(|| format!("“{label}”: çizilmedi (Haritada çiz'e basın).")),
            Component::RelationPicker { .. } => self
                .choices
                .get(id)
                .is_none_or(|c| c.is_empty())
                .then(|| format!("“{label}”: kayıt seçilmedi.")),
            _ => self
                .texts
                .get(id)
                .is_none_or(|t| t.trim().is_empty())
                .then(|| format!("“{label}” boş bırakılamaz.")),
        }
    }

    /// Input `c`'s value into `out` by its id, or why it is wrong.
    fn read_into(&self, c: &Component, out: &mut Map<String, Value>) -> Result<(), String> {
        match c {
            Component::TextField { id, .. } => {
                let text = self.texts.get(id).map_or("", |t| t.trim());
                out.insert(id.clone(), json!(text));
            }
            // A relation's parent record: its persistent id, none when none is chosen (docs/adr/8057 §67).
            Component::RelationPicker { id, .. } => {
                let chosen = self.choices.get(id).filter(|c| !c.is_empty());
                out.insert(id.clone(), chosen.map_or(Value::Null, |c| json!(c)));
            }
            // A drawn shape: GeoJSON, none when empty (docs/adr/8057 §62).
            Component::ShapePicker { id, .. } => {
                out.insert(
                    id.clone(),
                    self.geoms.get(id).cloned().unwrap_or(Value::Null),
                );
            }
            // A point: `[x, y]`, none when empty (docs/adr/8057 §57).
            Component::PointPicker { id, label, .. } => {
                let typed = self.texts.get(id).map_or("", |t| t.trim());
                let value = if typed.is_empty() {
                    Value::Null
                } else {
                    let [x, y] =
                        kentos_extensions::ui_panel::typed_point(typed).ok_or_else(|| {
                            format!("“{label}” bir nokta olmalı: x, y (ör. 500123.45, 4401234.50).")
                        })?;
                    json!([x, y])
                };
                out.insert(id.clone(), value);
            }
            // A file to be read: its absolute path, checked to be a file of the kinds offered (panel_files.rs).
            Component::FilePicker {
                id,
                label,
                filters,
                save,
                ..
            } => {
                let typed = self.texts.get(id).map_or("", String::as_str);
                out.insert(
                    id.clone(),
                    super::panel_files::file_value(label, filters, *save, typed)?,
                );
            }
            Component::NumberField {
                id,
                label,
                min,
                max,
                unit,
                ..
            } => {
                let text = self.texts.get(id).map_or("", |t| t.trim());
                if text.is_empty() {
                    out.insert(id.clone(), Value::Null);
                    return Ok(());
                }
                // In its own unit, whichever of its units it was typed in (panel_units.rs, docs/adr/8057 §64).
                let units = self.units_words(id);
                let Some(n) = self.typed_number(id, text) else {
                    return Err(match units {
                        Some(units) => format!(
                            "“{label}”: “{text}” okunamadı: bir sayı ya da birimiyle yazılmış bir sayı olmalı \
                             ({units})."
                        ),
                        None => format!("“{label}”: “{text}” bir sayı değil."),
                    });
                };
                if min.is_some_and(|m| n < m) || max.is_some_and(|m| n > m) {
                    // With its unit when it takes others: a number typed in km is checked in m.
                    let words = |b: f64| match unit.as_deref().filter(|_| units.is_some()) {
                        Some(unit) => format!("{}\u{a0}{unit}", number_words(b)),
                        None => number_words(b),
                    };
                    return Err(format!(
                        "“{label}” {} ile {} arasında olmalı.",
                        min.map_or_else(|| "−∞".to_owned(), words),
                        max.map_or_else(|| "∞".to_owned(), words)
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
            // A coordinate system's `EPSG:<code>`, none when none is chosen (panel_crs.rs).
            Component::CrsPicker { id, .. } => {
                let chosen = self.choices.get(id).filter(|v| !v.is_empty());
                out.insert(id.clone(), chosen.map_or(Value::Null, |v| json!(v)));
            }
            // The chosen values, in the options' order.
            Component::MultiSelect { id, options, .. } => {
                let chosen = self.multi.get(id);
                let values: Vec<&str> = self
                    .options_of(id, options)
                    .iter()
                    .filter(|o| chosen.is_some_and(|c| c.contains(&o.value)))
                    .map(|o| o.value.as_str())
                    .collect();
                out.insert(id.clone(), json!(values));
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
                    return Ok(());
                }
                let Some(iso) = kentos_extensions::ui_panel::typed_date(text) else {
                    return Err(format!(
                        "“{label}”: “{text}” bir tarih değil (GG.AA.YYYY ya da YYYY-AA-GG yazın)."
                    ));
                };
                bounded(label, &iso, min.as_deref(), max.as_deref(), date_words)?;
                out.insert(id.clone(), json!(iso));
            }
            // A time of day, HH:MM; as a date's.
            Component::TimeField {
                id,
                label,
                min,
                max,
                ..
            } => {
                let text = self.texts.get(id).map_or("", |t| t.trim());
                if text.is_empty() {
                    out.insert(id.clone(), Value::Null);
                    return Ok(());
                }
                let Some(iso) = kentos_extensions::ui_panel::typed_time(text) else {
                    return Err(format!(
                        "“{label}”: “{text}” bir saat değil (SS:DD yazın, ör. 14:30)."
                    ));
                };
                bounded(label, &iso, min.as_deref(), max.as_deref(), str::to_owned)?;
                out.insert(id.clone(), json!(iso));
            }
            // A local date and time, 2026-11-15T14:30; as a date's.
            Component::DateTimeField {
                id,
                label,
                min,
                max,
                ..
            } => {
                let text = self.texts.get(id).map_or("", |t| t.trim());
                if text.is_empty() {
                    out.insert(id.clone(), Value::Null);
                    return Ok(());
                }
                let Some(iso) = kentos_extensions::ui_panel::typed_date_time(text) else {
                    return Err(format!(
                        "“{label}”: “{text}” bir tarih ve saat değil (GG.AA.YYYY SS:DD yazın, ör. 15.11.2026 \
                             14:30)."
                    ));
                };
                bounded(label, &iso, min.as_deref(), max.as_deref(), date_time_words)?;
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
        Ok(())
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

    /// A button's command answered `result`: its rows to `table` (the button's output) when it gives rows, its words
    /// (or the rows counted) under the form; the table filled, if one was.
    pub(super) fn take_answer(&mut self, table: Option<&String>, result: &Value) -> Option<String> {
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
        // A message answers no rows: the table keeps what it showed (a button that writes, as Yapıları numarala, does
        // not empty the search's results).
        let filled = table.filter(|_| result.get("rows").is_some()).cloned();
        if let Some(table) = &filled {
            self.tables.insert(table.clone(), (columns, rows.clone()));
            // New rows from their first page.
            self.pages.remove(table);
            self.cursor.remove(table);
        }
        // Words its code said as it ran stay when its answer says none of its own (panel_session.rs).
        match result.get("message").and_then(Value::as_str) {
            Some(said) => self.said = Some((words::Kind::Ok, said.to_owned())),
            None if self.spoke && self.said.is_some() => {}
            None => self.said = Some((words::Kind::Ok, format!("{} satır.", rows.len()))),
        }
        filled
    }

    /// Number field `id`'s − (`sign` −1) or + (1): its value (else its first, else its lower bound, else 0) moved by
    /// its step and kept within its bounds, to the step's decimals (0.1 steps do not drift into 0.30000000000000004).
    pub(crate) fn step(&mut self, id: &str, sign: i8) {
        let Some(Component::NumberField {
            step: Some(step),
            min,
            max,
            value,
            ..
        }) = self.definition.body.iter().find(|c| c.id() == Some(id))
        else {
            return;
        };
        let (step, min, max) = (*step, *min, *max);
        let now = self
            .texts
            .get(id)
            .and_then(|t| self.typed_number(id, t))
            .or(*value)
            .or(min)
            .unwrap_or(0.0);
        let decimals = step.to_string().split_once('.').map_or(0, |(_, d)| d.len()) as i32;
        let scale = 10f64.powi(decimals);
        let mut next = ((now + f64::from(sign) * step) * scale).round() / scale;
        if let Some(min) = min {
            next = next.max(min);
        }
        if let Some(max) = max {
            next = next.min(max);
        }
        self.texts.insert(id.to_owned(), number_words(next));
    }

    /// The output (a table, a tree, a card) a button's rows go to: its own, else the panel's first.
    pub(super) fn table_of(&self, table: Option<&String>) -> Option<String> {
        table.cloned().or_else(|| {
            self.definition.body.iter().find_map(|c| match c {
                Component::Table { id, .. }
                | Component::Tree { id, .. }
                | Component::Card { id, .. }
                | Component::Stats { id, .. }
                | Component::Chart { id, .. } => Some(id.clone()),
                _ => None,
            })
        })
    }
}

/// An ISO date as a field shows it, GG.AA.YYYY; another text as it is.
pub(super) fn date_words(iso: &str) -> String {
    match kentos_extensions::ui_panel::iso_date(iso) {
        Some((y, m, d)) => format!("{d:02}.{m:02}.{y:04}"),
        None => iso.to_owned(),
    }
}

/// A local date and time as a field shows it, GG.AA.YYYY SS:DD; another text as it is.
pub(super) fn date_time_words(iso: &str) -> String {
    match kentos_extensions::ui_panel::iso_date_time(iso) {
        Some(((y, mo, d), (h, mi))) => format!("{d:02}.{mo:02}.{y:04} {h:02}:{mi:02}"),
        None => iso.to_owned(),
    }
}

/// A value inside its field's bounds, or why not by its label: written alike (ISO), text order is time order; the
/// bounds said in the field's own words.
fn bounded(
    label: &str,
    iso: &str,
    min: Option<&str>,
    max: Option<&str>,
    words: fn(&str) -> String,
) -> Result<(), String> {
    let early = min.is_some_and(|m| iso < m);
    let late = max.is_some_and(|m| iso > m);
    if !(early || late) {
        return Ok(());
    }
    Err(match (min, max) {
        (Some(a), Some(b)) => format!("“{label}” {} ile {} arasında olmalı.", words(a), words(b)),
        (Some(a), None) => format!("“{label}” en erken {} olabilir.", words(a)),
        (_, Some(b)) => format!("“{label}” en geç {} olabilir.", words(b)),
        (None, None) => unreachable!(),
    })
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
pub(super) fn number_of(text: &str) -> Option<f64> {
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

    /// The number of `ext`'s open panel `local` (0 when it is not open), for a test's run origin.
    #[cfg(test)]
    pub(crate) fn instance_of(&self, ext: &str, local: &str) -> u64 {
        self.panels
            .iter()
            .find(|p| p.ext == ext && p.local == local)
            .map_or(0, |p| p.instance)
    }

    /// The open panel a run started from; its tab marks the answer when another panel is shown.
    fn answered(&mut self, origin: &PanelOrigin) -> Option<&mut OpenPanel> {
        let shown = self.active_panel;
        let (at, panel) = self.panels.iter_mut().enumerate().find(|(_, p)| {
            p.instance == origin.instance && p.ext == origin.ext && p.local == origin.panel
        })?;
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
        // In the interface's language when the package translates it (docs/adr/8057 §106).
        let definition = registry
            .localized_file(ext, &entry.definition, &format!("ui.{}", entry.id))
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
        panel.default_crs(self.document.as_ref().map(|d| d.model.settings().srid));
        panel.side = entry.side;
        self.extensions.panels.push(panel);
        self.extensions.active_panel = self.extensions.panels.len() - 1;
        Task::batch([scroll_back(0.0, 0.0), self.show_extension_panel()])
    }

    /// The open panel at `at` shown, scrolled as it was left; its answer seen.
    fn ext_panel_activate(&mut self, at: usize) -> Task<Message> {
        let Some(panel) = self.extensions.panels.get_mut(at) else {
            return Task::none();
        };
        panel.unseen = false;
        let (y, outputs) = (panel.scroll, panel.out_scroll);
        self.extensions.active_panel = at;
        scroll_back(y, outputs)
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
    pub(super) fn ext_panel_close(&mut self, at: usize) -> Task<Message> {
        let x = &mut self.extensions;
        if at >= x.panels.len() {
            return Task::none();
        }
        let closed = x.panels.remove(at);
        x.panel_charts
            .retain(|(ext, local, _), _| !(*ext == closed.ext && *local == closed.local));
        if at < x.active_panel || (x.active_panel >= x.panels.len() && x.active_panel > 0) {
            x.active_panel -= 1;
        }
        match x.panels.get_mut(x.active_panel) {
            Some(panel) => {
                panel.unseen = false;
                scroll_back(panel.scroll, panel.out_scroll)
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
        panel.forget_invalid(&event);
        match event {
            // Each value changed works the calculations out again (panel_values.rs), before a command reads them.
            PanelEvent::Text(id, text) => {
                panel.texts.insert(id, text);
                panel.recalculate();
            }
            PanelEvent::Check(id, on) => {
                panel.checks.insert(id.clone(), on);
                panel.recalculate();
                return self.ext_panel_changed(at, &id);
            }
            PanelEvent::Choose(id, value) => {
                panel.choose(id.clone(), value);
                panel.recalculate();
                return self.ext_panel_changed(at, &id);
            }
            PanelEvent::Unpick(id) => {
                panel.picks.insert(id.clone(), Vec::new());
                panel.recalculate();
                return self.ext_panel_changed(at, &id);
            }
            // A day chosen from the calendar is a change (typing is not: a process each key, as a text field's).
            PanelEvent::Picked(id, words) => {
                panel.texts.insert(id.clone(), words);
                panel.recalculate();
                return self.ext_panel_changed(at, &id);
            }
            PanelEvent::Toggle(id, value) => {
                let chosen = panel.multi.entry(id.clone()).or_default();
                match chosen.iter().position(|v| *v == value) {
                    Some(i) => {
                        chosen.remove(i);
                    }
                    None => chosen.push(value),
                }
                panel.recalculate();
                return self.ext_panel_changed(at, &id);
            }
            PanelEvent::Pick(id) => return self.ext_panel_pick(at, &id),
            PanelEvent::PickPoint(id) => return self.ext_panel_pick_point(at, &id),
            PanelEvent::DrawShape(id) => return self.ext_panel_draw_shape(at, &id),
            PanelEvent::ClearShape(id) => {
                panel.geoms.remove(&id);
                panel.recalculate();
                return self.ext_panel_changed(at, &id);
            }
            PanelEvent::Step(id, sign) => {
                panel.step(&id, sign);
                panel.recalculate();
            }
            PanelEvent::Activate(to) => return self.ext_panel_activate(to),
            PanelEvent::Move(from, to) => self.ext_panel_move(from, to),
            PanelEvent::Close(which) => return self.ext_panel_close(which),
            PanelEvent::Scrolled(y) => panel.scroll = y,
            PanelEvent::ScrolledOutputs(y) => panel.out_scroll = y,
            PanelEvent::Split(height) => panel.split = Some(height),
            PanelEvent::SplitReset => panel.split = None,
            PanelEvent::Press(button) => return self.ext_panel_press(at, &button),
            PanelEvent::Page(id, page) => {
                panel.pages.insert(id, page);
            }
            PanelEvent::Cursor(id, row, page) => {
                panel.pages.insert(id.clone(), page);
                panel.cursor.insert(id, row);
            }
            PanelEvent::Section(at) => panel.toggle_section(at),
            PanelEvent::Tab(name) => panel.show_tab(&name),
            PanelEvent::PickFile(id) => {
                let Some(picker) = super::panel_files::picker_of(panel, &id) else {
                    return Task::none();
                };
                return super::panel_files::pick(&picker, move |path| {
                    super::msg(super::Event::Panel(PanelEvent::Picked(id.clone(), path)))
                });
            }
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
        panel.recalculate();
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
    /// change while a command runs waits for it: one rerun, with the values then. While an input it asks for has no
    /// value yet, the command waits saying nothing, and the rows given before go.
    pub(super) fn ext_panel_changed(&mut self, at: usize, input: &str) -> Task<Message> {
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
        // A change runs its command only with the values it asks for. One still to be given (an area let go with ✕,
        // a box ticked before the area is drawn) is not a mistake to say, as a button's press says it: said in red
        // twice, real window. The answer to the values before goes: it is not the answer to these.
        if panel.waits_for_value() {
            panel.said = None;
            if let Some(table) = panel.table_of(None) {
                panel.tables.remove(&table);
            }
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
        // Objects taken before and deleted since are not given: the code would not find them. Nor a relation's record
        // chosen before and deleted since (docs/adr/8057 §67).
        if let Some(doc) = &self.document {
            let gone = |u: &String| {
                kentos_domain::Uuid::parse_str(u)
                    .ok()
                    .and_then(|u| doc.model.slot_of(u))
                    .is_none()
            };
            for uids in panel.picks.values_mut() {
                uids.retain(|u| !gone(u));
            }
            let records: Vec<String> = panel
                .definition
                .body
                .iter()
                .filter_map(|c| match c {
                    Component::RelationPicker { id, .. } => Some(id.clone()),
                    _ => None,
                })
                .collect();
            for id in records {
                if panel
                    .choices
                    .get(&id)
                    .is_some_and(|u| !u.is_empty() && gone(u))
                {
                    panel.choices.insert(id, String::new());
                }
            }
        }
        let params = match panel.check_params() {
            Ok(params) => params,
            Err((input, why)) => {
                panel.said = Some((words::Kind::Error, why.clone()));
                panel.reveal(&input);
                panel.invalid = Some((input, why));
                return Task::none();
            }
        };
        panel.invalid = None;
        panel.show_in_own_units(&params);
        let origin = PanelOrigin {
            ext: ext.clone(),
            panel: panel.local.clone(),
            button: trigger.to_owned(),
            table: panel.table_of(table.as_ref()),
            instance: panel.instance,
        };
        panel.running = Some(trigger.to_owned());
        panel.said = None;
        panel.touched.clear();
        panel.spoke = false;
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
        // What a right-click menu's run was opened on goes with it: a later run from the ribbon is not given it.
        self.extensions
            .target_params
            .take_if(|(e, l, _)| e == ext && l == command);
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
        panel.touched.clear();
        match outcome {
            Ok(result) => {
                // The charts drawing those rows, worked out again (panel_chart.rs).
                if let Some(table) = panel.take_answer(origin.table.as_ref(), &result) {
                    self.ext_panel_charts(&origin.ext, &origin.panel, &table);
                }
            }
            Err(why) => panel.said = Some((words::Kind::Error, why)),
        }
    }

    /// A run started from a panel stopped with Durdur: said as the user's choice, not a failure (it was shown in red,
    /// real window, 14 November 2026); its table keeps what it showed.
    pub(crate) fn ext_panel_stopped(&mut self, origin: &PanelOrigin) {
        if let Some(panel) = self.extensions.answered(origin) {
            panel.running = None;
            panel.touched.clear();
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
mod tests;
