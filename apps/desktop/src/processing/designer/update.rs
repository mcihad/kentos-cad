//! What Model tasarımcısı's events do (the web's ModelDesigner,
//! ModelCanvas, modelPalette and modelInspector): every edit goes through
//! one path that keeps the undo step, changes the draft and checks it.

use iced::Task;
use kentos_interaction::Level;
use kentos_interaction::pick::PickPoint;
use kentos_processing::designer::{self as plan, View, texts};
use kentos_processing::model_edit::{self as edit, INPUT_TYPES, NodeRef};
use kentos_processing::parameters::default_value;
use kentos_processing::{Defaults, Tool, ValueSource};
use serde_json::{Value, json};

use super::{Asking, Designer, WireMenu};
use crate::app::{App, Dialog, Message};
use crate::processing::dialog::ToolDialog;

fn now_ms() -> i64 {
    crate::cloud::now_ms()
}

/// Where a step parameter's list is set to.
#[derive(Clone, Debug)]
pub(crate) enum SourceChoice {
    ToolDefault,
    Fixed,
    From(ValueSource),
    /// Yeni model girdisi yap.
    AsInput,
}

/// A model input's field in the settings.
#[derive(Clone, Debug)]
pub(crate) enum InputField {
    Label(String),
    Description(String),
    Optional(bool),
    /// Nesneler: the default scope.
    Scope(String),
    /// Nesneler: a kind chip pressed.
    Kind(String),
    /// Sayı: Varsayılan, En az, En çok as typed.
    Number(&'static str, String),
    Integer(bool),
    /// Metin: the default text.
    Text(String),
    AllowEmpty(bool),
    /// Evet / hayır: the default.
    Flag(bool),
    /// Katman: the default new layer's name.
    NewLayer(String),
}

/// What the window asks for.
#[derive(Clone, Debug)]
pub(crate) enum Event {
    // The diagram.
    Select(Option<NodeRef>),
    Move {
        node: NodeRef,
        at: (f64, f64),
        done: bool,
    },
    /// A wire from `from`'s port let go over a step, at a point of the diagram.
    Wire {
        from: NodeRef,
        to: String,
        at: iced::Point,
    },
    /// A box double-clicked: selected, its settings to hand.
    Open(NodeRef),
    View(View),
    /// The diagram's size, as laid out.
    Resized(iced::Size),
    Fit,
    /// Uzaklaş and Yakınlaş (about the diagram's middle).
    Zoom(f64),
    // The parts.
    AddInput(usize),
    /// A tool clicked (next to the selected box) or let go on the diagram (a world point).
    AddTool {
        tool: String,
        at: Option<(f64, f64)>,
    },
    Search(String),
    /// A tool row carried out of the parts (its place in the list).
    Carry(usize),
    /// A carried tool let go away from the diagram.
    Uncarry,
    // The wire's menu.
    Connect {
        step: String,
        param: String,
        src: ValueSource,
    },
    MenuClose,
    // The settings: the model.
    Label(String),
    Description(String),
    Category(String),
    RemoveOutput(usize),
    AddOutput {
        step: String,
        output: String,
    },
    // An input.
    Input(String, InputField),
    RemoveInput(String),
    // A step.
    Caption(String),
    Source {
        param: String,
        choice: SourceChoice,
    },
    /// A fixed value's control (the tool window's own).
    Field(crate::processing::Event),
    Advanced,
    RemoveStep,
    // The model.
    Delete,
    // The foot.
    Layout,
    Close,
    SaveRun,
    Save,
    /// The question's answer: `Some(true)` save and close, `Some(false)` close
    /// without saving (or delete, for Modeli sil), `None` stay.
    Answer(Option<bool>),
    Undo,
    Redo,
}

impl App {
    /// `processing.newModel`, with a model's id to edit it: a built-in one
    /// opens as its copy (the web's `openModelDesigner`).
    pub(crate) fn open_model_designer(&mut self, model: Option<&str>) {
        let designer = match model {
            None => Designer::new(edit::new_model(new_model_id()), false, false),
            Some(id) => {
                let Some(source) = self.processing.registry.model(id).cloned() else {
                    self.error(texts::not_found(id));
                    return;
                };
                if self.processing.registry.is_builtin_model(id) {
                    Designer::new(edit::copy_model(&source, new_model_id()), false, true)
                } else {
                    Designer::new(source, true, false)
                }
            }
        };
        self.processing.designer = Some(Box::new(designer));
        self.dialog = Some(Dialog::ModelDesigner);
        self.designer_checked();
    }

    /// The draft's problems again, and the selected step's controls.
    fn designer_checked(&mut self) {
        let registry = self.processing.registry.clone();
        let lookup = |id: &str| registry.tool(id);
        let Some(d) = self.processing.designer.as_deref_mut() else {
            return;
        };
        d.after_change(&lookup);
        self.designer_fields(false);
    }

    /// The selected step's fixed values as the tool window's controls hold
    /// them; `keep`: the same step's controls stay (a number being typed).
    fn designer_fields(&mut self, keep: bool) {
        let Some(doc) = &self.document else {
            if let Some(d) = self.processing.designer.as_deref_mut() {
                d.fields = None;
            }
            return;
        };
        let defaults = Defaults::of(&doc.model);
        let registry = &self.processing.registry;
        let Some(d) = self.processing.designer.as_deref_mut() else {
            return;
        };
        let Some(NodeRef::Step(id)) = &d.selected else {
            d.fields = None;
            return;
        };
        let Some(step) = d.draft.steps.iter().find(|s| &s.id == id) else {
            d.fields = None;
            return;
        };
        let Some(tool) = registry.tool(&step.tool) else {
            d.fields = None;
            return;
        };
        // The fixed values and the tool's defaults: what the controls and visibility read.
        let values: kentos_processing::Values = tool
            .parameters
            .iter()
            .map(|p| {
                let v = match step.source(&p.name) {
                    Some(ValueSource::Value(v)) => v.clone(),
                    _ => default_value(p, &defaults),
                };
                (p.name.clone(), v)
            })
            .collect();
        let same = d
            .fields
            .as_ref()
            .is_some_and(|f| f.tool.id == tool.id && f.values == values);
        if !(keep && same) {
            d.fields = Some(ToolDialog::new(tool, Some(&values), false, &defaults));
        }
        let look = crate::processing::Look {
            doc: &doc.model,
            selection: &self.selection,
            store: self.spatial.store(),
            view: Some(self.viewport.camera.visible_bounds()),
        };
        if let Some(fields) = d.fields.as_mut() {
            fields.refresh(&self.processing.runner, &look);
        }
    }

    /// One edit of the draft: its undo step, the change, the check.
    fn designer_change(
        &mut self,
        key: Option<&str>,
        change: impl FnOnce(&mut Designer, &dyn Fn(&str) -> Option<Tool>),
    ) {
        let registry = self.processing.registry.clone();
        let lookup = |id: &str| registry.tool(id);
        let Some(d) = self.processing.designer.as_deref_mut() else {
            return;
        };
        d.before_change(key, now_ms());
        change(d, &lookup);
        d.after_change(&lookup);
        self.designer_fields(key.is_some());
    }

    pub(crate) fn model_designer_event(&mut self, e: Event) -> Task<Message> {
        if self.processing.designer.is_none() {
            return Task::none();
        }
        match e {
            Event::Select(node) => {
                if let Some(d) = self.processing.designer.as_deref_mut() {
                    if d.selected != node {
                        d.advanced_open = false;
                        d.typed.clear();
                    }
                    d.selected = node;
                    d.wire_menu = None;
                }
                self.designer_fields(false);
            }
            Event::Open(node) => {
                if let Some(d) = self.processing.designer.as_deref_mut() {
                    if d.selected.as_ref() != Some(&node) {
                        d.advanced_open = false;
                        d.typed.clear();
                    }
                    d.selected = Some(node);
                }
                self.designer_fields(false);
            }
            Event::Move { node, at, done } => {
                if let Some(d) = self.processing.designer.as_deref_mut() {
                    d.move_box(&node, at, done);
                }
            }
            Event::Wire { from, to, at } => {
                if let Some(d) = self.processing.designer.as_deref_mut() {
                    d.wire_menu = Some(WireMenu { from, to, at });
                }
            }
            Event::MenuClose => {
                if let Some(d) = self.processing.designer.as_deref_mut() {
                    d.wire_menu = None;
                }
            }
            Event::Connect { step, param, src } => {
                self.designer_change(None, |d, _| {
                    edit::set_source(&mut d.draft, &step, &param, Some(src));
                    d.selected = Some(NodeRef::Step(step.clone()));
                    d.wire_menu = None;
                });
            }
            Event::View(view) => {
                if let Some(d) = self.processing.designer.as_deref_mut() {
                    d.view = view;
                }
            }
            Event::Resized(size) => {
                if let Some(d) = self.processing.designer.as_deref_mut() {
                    d.canvas = Some(size);
                    if d.fit_pending {
                        d.fit();
                    }
                }
            }
            Event::Fit => {
                if let Some(d) = self.processing.designer.as_deref_mut() {
                    d.fit();
                }
            }
            Event::Zoom(f) => {
                if let Some(d) = self.processing.designer.as_deref_mut()
                    && let Some(size) = d.canvas
                {
                    let middle =
                        plan::Pt::new(f64::from(size.width) / 2.0, f64::from(size.height) / 2.0);
                    d.view = plan::zoom_at(d.view, middle, f, d.floor);
                }
            }
            Event::AddInput(i) => {
                if let Some(kind) = INPUT_TYPES.get(i) {
                    self.designer_change(None, |d, _| {
                        let at = plan::spot_near(&d.draft, d.selected.as_ref());
                        let name = edit::add_input(
                            &mut d.draft,
                            kind.type_name,
                            kind.label,
                            Some((at.x, at.y)),
                        );
                        d.selected = Some(NodeRef::Input(name));
                    });
                }
            }
            Event::AddTool { tool, at } => {
                self.designer_change(None, |d, lookup| {
                    d.carrying = None;
                    let place = match at {
                        Some((x, y)) => (plan::snap(x), plan::snap(y)),
                        None => {
                            let p = plan::spot_near(&d.draft, d.selected.as_ref());
                            (p.x, p.y)
                        }
                    };
                    let from = d.selected.clone();
                    let id =
                        edit::add_step(&mut d.draft, &tool, lookup, Some(place), from.as_ref());
                    d.selected = Some(NodeRef::Step(id));
                });
            }
            Event::Search(text) => {
                if let Some(d) = self.processing.designer.as_deref_mut() {
                    d.search = text;
                }
            }
            Event::Carry(place) => {
                let tool = super::palette::tool_at(
                    &self.processing.registry,
                    self.processing
                        .designer
                        .as_deref()
                        .map_or("", |d| d.search.as_str()),
                    place,
                );
                if let (Some(d), Some(tool)) = (self.processing.designer.as_deref_mut(), tool) {
                    d.carrying = Some(tool);
                }
            }
            Event::Uncarry => {
                if let Some(d) = self.processing.designer.as_deref_mut() {
                    d.carrying = None;
                }
            }
            Event::Label(text) => self.designer_change(Some("label"), |d, _| d.draft.label = text),
            Event::Description(text) => {
                self.designer_change(Some("description"), |d, _| d.draft.description = text);
            }
            Event::Category(id) => self.designer_change(None, |d, _| d.draft.category = id),
            Event::RemoveOutput(i) => self.designer_change(None, |d, _| {
                if i < d.draft.outputs.len() {
                    d.draft.outputs.remove(i);
                }
            }),
            Event::AddOutput { step, output } => self.designer_change(None, |d, lookup| {
                let _ = edit::add_output(&mut d.draft, &step, &output, lookup);
            }),
            Event::Input(name, field) => {
                let key = format!("{name}.{}", input_key(&field));
                let typed = match &field {
                    InputField::Number(_, text) => Some(text.clone()),
                    _ => None,
                };
                self.designer_change(Some(&key), |d, _| {
                    set_input(&mut d.draft, &name, field);
                    if let Some(text) = typed {
                        d.typed.insert(key.clone(), text);
                    }
                });
            }
            Event::RemoveInput(name) => self.designer_change(None, |d, _| {
                edit::remove_input(&mut d.draft, &name);
                d.selected = None;
            }),
            Event::Caption(text) => {
                let Some(NodeRef::Step(step)) = self.designer_selected() else {
                    return Task::none();
                };
                let key = format!("{step}.caption");
                self.designer_change(Some(&key), |d, _| {
                    edit::set_caption(&mut d.draft, &step, &text)
                });
            }
            Event::Source { param, choice } => {
                let Some(NodeRef::Step(step)) = self.designer_selected() else {
                    return Task::none();
                };
                let defaults = self.document.as_ref().map(|doc| Defaults::of(&doc.model));
                self.designer_change(None, |d, lookup| {
                    let def = d
                        .draft
                        .steps
                        .iter()
                        .find(|s| s.id == step)
                        .and_then(|s| lookup(&s.tool))
                        .and_then(|t| t.parameters.into_iter().find(|p| p.name == param));
                    match choice {
                        SourceChoice::ToolDefault => {
                            edit::set_source(&mut d.draft, &step, &param, None)
                        }
                        SourceChoice::Fixed => {
                            let value = match d.source(&step, &param) {
                                Some(ValueSource::Value(v)) => v.clone(),
                                _ => match (&def, &defaults) {
                                    (Some(p), Some(defaults)) => default_value(p, defaults),
                                    _ => Value::Null,
                                },
                            };
                            edit::set_source(
                                &mut d.draft,
                                &step,
                                &param,
                                Some(ValueSource::Value(value)),
                            );
                        }
                        SourceChoice::From(src) => {
                            edit::set_source(&mut d.draft, &step, &param, Some(src))
                        }
                        SourceChoice::AsInput => {
                            if let (Some(p), Some(defaults)) = (&def, &defaults) {
                                let _ = edit::input_from_param(&mut d.draft, &step, p, defaults);
                            }
                        }
                    }
                });
            }
            Event::Field(field) => return self.designer_field(field),
            Event::Advanced => {
                if let Some(d) = self.processing.designer.as_deref_mut() {
                    d.advanced_open = !d.advanced_open;
                }
            }
            Event::RemoveStep => {
                let Some(NodeRef::Step(step)) = self.designer_selected() else {
                    return Task::none();
                };
                self.designer_change(None, |d, _| {
                    edit::remove_step(&mut d.draft, &step);
                    d.selected = None;
                });
            }
            Event::Delete => {
                if let Some(d) = self.processing.designer.as_deref_mut() {
                    d.asking = Some(Asking::Remove);
                }
            }
            Event::Layout => {
                self.designer_change(None, |d, _| edit::auto_layout(&mut d.draft));
                if let Some(d) = self.processing.designer.as_deref_mut() {
                    d.fit();
                }
            }
            Event::Close => self.designer_close(),
            Event::Save => {
                self.designer_save();
            }
            Event::SaveRun => {
                if !self.designer_save() {
                    return Task::none();
                }
                let id = self
                    .processing
                    .designer
                    .as_deref()
                    .map(|d| d.draft.id.clone());
                self.processing.designer = None;
                self.dialog = None;
                if let Some(id) = id {
                    return self.processing_command(&format!("processing.model.{id}"));
                }
            }
            Event::Answer(answer) => return self.designer_answer(answer),
            Event::Undo | Event::Redo => {
                let redo = matches!(e, Event::Redo);
                let undone = self
                    .processing
                    .designer
                    .as_deref_mut()
                    .is_some_and(|d| d.undo(redo));
                if undone {
                    self.designer_checked();
                }
            }
        }
        Task::none()
    }

    fn designer_selected(&self) -> Option<NodeRef> {
        self.processing
            .designer
            .as_deref()
            .and_then(|d| d.selected.clone())
    }

    /// A fixed value's control changed: the step's value, joined as typing.
    fn designer_field(&mut self, field: crate::processing::Event) -> Task<Message> {
        use crate::processing::Event as F;
        let Some(NodeRef::Step(step)) = self.designer_selected() else {
            return Task::none();
        };
        match field {
            F::Pick(name) => {
                self.designer_pick(step, name, None);
                return Task::none();
            }
            F::PickChoice(name) => {
                let picks = self.processing.designer.as_deref().and_then(|d| {
                    d.fields.as_ref().and_then(|f| {
                        f.tool
                            .parameters
                            .iter()
                            .find(|p| p.name == name)
                            .and_then(|p| p.picks.clone())
                    })
                });
                if let Some((option, point)) = picks {
                    self.designer_pick(step, point, Some((name, option)));
                }
                return Task::none();
            }
            F::Run
            | F::Close
            | F::Reset
            | F::Results
            | F::Undo
            | F::Target(_)
            | F::Panel(_)
            | F::PickObjects(_) => {
                return Task::none();
            }
            F::Advanced => {
                if let Some(d) = self.processing.designer.as_deref_mut() {
                    d.advanced_open = !d.advanced_open;
                }
                return Task::none();
            }
            _ => {}
        }
        let active = self
            .document
            .as_ref()
            .map(|d| d.model.layers().active().to_owned())
            .unwrap_or_default();
        // The control's window takes the edit as the tool window does, then
        // the values it changed go to the step, fixed.
        let Some(before) = self
            .processing
            .designer
            .as_deref()
            .and_then(|d| d.fields.as_ref())
            .map(|f| f.values.clone())
        else {
            return Task::none();
        };
        let name = field_name(&field);
        if let Some(fields) = self
            .processing
            .designer
            .as_deref_mut()
            .and_then(|d| d.fields.as_mut())
        {
            fields.edit(field, &active);
        }
        let after = self
            .processing
            .designer
            .as_deref()
            .and_then(|d| d.fields.as_ref())
            .map(|f| f.values.clone())
            .unwrap_or_default();
        let changed: Vec<(String, Value)> = after
            .iter()
            .filter(|(k, v)| before.get(*k) != Some(*v))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        if changed.is_empty() {
            return Task::none();
        }
        let key = name.map(|n| format!("{step}.{n}"));
        self.designer_change(key.as_deref(), |d, _| {
            for (param, value) in changed {
                // Only a parameter set to a fixed value shows a control.
                if matches!(d.source(&step, &param), Some(ValueSource::Value(_))) {
                    edit::set_source(&mut d.draft, &step, &param, Some(ValueSource::Value(value)));
                }
            }
        });
        Task::none()
    }

    /// Sahneden seç in a step's settings: the designer steps aside, the point
    /// is shown on the drawing and the designer opens again on the same draft
    /// (docs/adr/0088; the web's `pickPoint`).
    fn designer_pick(&mut self, step: String, name: String, then: Option<(String, String)>) {
        let Some(mut designer) = self.processing.designer.take() else {
            return;
        };
        let label = designer
            .fields
            .as_ref()
            .and_then(|f| f.tool.parameters.iter().find(|p| p.name == name))
            .map(|p| p.label.clone());
        designer.selected = Some(NodeRef::Step(step.clone()));
        self.say(
            Level::Command,
            texts::pick::command(label.as_deref().unwrap_or(texts::pick::UNNAMED)),
        );
        self.processing.picking = Some(crate::processing::Picking::Designer {
            designer,
            step,
            name,
            then,
        });
        self.dialog = None;
        self.field = None;
        self.snap = None;
        self.session.run(Box::new(PickPoint::new(
            "",
            label.unwrap_or_else(|| texts::pick::POINT.to_owned()),
        )));
        self.with_tool(|s, cx| s.activate(cx));
    }

    /// The point shown for the designer (or none, Esc): it opens again, the
    /// point a fixed value, the choice too.
    pub(crate) fn designer_picked(
        &mut self,
        mut designer: Box<Designer>,
        step: String,
        name: String,
        then: Option<(String, String)>,
        p: Option<kentos_interaction::Vec2>,
    ) {
        if let Some(p) = p {
            designer.before_change(None, now_ms());
            edit::set_source(
                &mut designer.draft,
                &step,
                &name,
                Some(ValueSource::Value(json!({ "x": p.x, "y": p.y }))),
            );
            if let Some((choice, option)) = then {
                edit::set_source(
                    &mut designer.draft,
                    &step,
                    &choice,
                    Some(ValueSource::Value(json!(option))),
                );
            }
        }
        self.processing.designer = Some(designer);
        self.dialog = Some(Dialog::ModelDesigner);
        self.designer_checked();
    }

    /// Kaydet (Ctrl+S): a blank name is “Adsız model”; the model goes to the
    /// user's library, problems or not, and becomes what “unsaved” compares with.
    fn designer_save(&mut self) -> bool {
        let Some(d) = self.processing.designer.as_deref_mut() else {
            return false;
        };
        d.draft.label = plan::saved_label(&d.draft.label);
        let model = d.draft.clone();
        let problems = d.problems.len();
        if let Err(why) = self.processing.registry.save_model(model.clone()) {
            self.error(why);
            return false;
        }
        self.processing.memory.save_model(&model);
        if let Some(d) = self.processing.designer.as_deref_mut() {
            let json = d.draft.to_json();
            d.saved = Some(json.clone());
            d.baseline = json;
            d.builtin_copy = false;
        }
        self.say(Level::Success, texts::save::saved(&model.label, problems));
        self.designer_checked();
        true
    }

    /// Kapat, ×, Esc: unsaved changes are asked about first.
    pub(crate) fn designer_close(&mut self) {
        let Some(d) = self.processing.designer.as_deref_mut() else {
            return;
        };
        if d.wire_menu.take().is_some() {
            return;
        }
        if d.carrying.take().is_some() {
            return;
        }
        if d.asking.is_some() {
            d.asking = None;
            return;
        }
        if d.dirty() {
            d.asking = Some(Asking::Unsaved);
            return;
        }
        self.processing.designer = None;
        self.dialog = None;
    }

    fn designer_answer(&mut self, answer: Option<bool>) -> Task<Message> {
        let asking = self.processing.designer.as_deref().and_then(|d| d.asking);
        if let Some(d) = self.processing.designer.as_deref_mut() {
            d.asking = None;
        }
        match (asking, answer) {
            (Some(Asking::Unsaved), Some(true)) => {
                if self.designer_save() {
                    self.processing.designer = None;
                    self.dialog = None;
                }
            }
            (Some(Asking::Unsaved), Some(false)) => {
                self.processing.designer = None;
                self.dialog = None;
            }
            (Some(Asking::Remove), Some(_)) => {
                if let Some(d) = self.processing.designer.take() {
                    let label = d.draft.label.clone();
                    self.processing.registry.remove_model(&d.draft.id);
                    self.processing.memory.remove_model(&d.draft.id);
                    self.say(Level::Info, texts::remove::done(&label));
                }
                self.dialog = None;
            }
            _ => {}
        }
        Task::none()
    }

    /// The designer's keys: Ctrl+S, Ctrl+Z / Ctrl+Y, Delete.
    pub(crate) fn model_designer_key(
        &mut self,
        press: &crate::keys::KeyPress,
    ) -> Option<Task<Message>> {
        use iced::keyboard::key::Named;
        let d = self.processing.designer.as_deref()?;
        if d.asking.is_some() {
            return None;
        }
        match crate::keys::chord(press).as_deref() {
            Some("Ctrl+S") => return Some(self.model_designer_event(Event::Save)),
            Some("Ctrl+Z") => return Some(self.model_designer_event(Event::Undo)),
            Some("Ctrl+Y" | "Ctrl+Shift+Z") => return Some(self.model_designer_event(Event::Redo)),
            _ => {}
        }
        match (press.named(), &d.selected) {
            (Some(Named::Delete | Named::Backspace), Some(NodeRef::Input(name))) => {
                let name = name.clone();
                Some(self.model_designer_event(Event::RemoveInput(name)))
            }
            (Some(Named::Delete | Named::Backspace), Some(NodeRef::Step(_))) => {
                Some(self.model_designer_event(Event::RemoveStep))
            }
            _ => None,
        }
    }
}

/// A model id: time and a random tail, so two made in the same millisecond
/// differ (the web's `modelId`).
fn new_model_id() -> String {
    let ms = crate::cloud::now_ms().max(0) as u64;
    let tail = std::collections::hash_map::RandomState::new();
    let r = std::hash::BuildHasher::hash_one(&tail, ms) % 36u64.pow(5);
    format!("m-{}{}", radix36(ms), radix36(r))
}

fn radix36(mut n: u64) -> String {
    const DIGITS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    if n == 0 {
        return "0".into();
    }
    let mut out = Vec::new();
    while n > 0 {
        out.push(DIGITS[(n % 36) as usize]);
        n /= 36;
    }
    out.reverse();
    String::from_utf8(out).unwrap_or_default()
}

/// The field's key for joining typing into one undo step (`<input>.<field>`).
fn input_key(field: &InputField) -> &'static str {
    match field {
        InputField::Label(_) => "label",
        InputField::Description(_) => "description",
        InputField::Optional(_) => "optional",
        InputField::Scope(_)
        | InputField::Flag(_)
        | InputField::Text(_)
        | InputField::NewLayer(_) => "default",
        InputField::Kind(_) => "kinds",
        InputField::Number(key, _) => key,
        InputField::Integer(_) => "integer",
        InputField::AllowEmpty(_) => "allowEmpty",
    }
}

/// The parameter a control's event is about.
fn field_name(e: &crate::processing::Event) -> Option<String> {
    use crate::processing::Event as F;
    match e {
        F::Value(n, _)
        | F::Number(n, _)
        | F::Text(n, _)
        | F::LayerName(n, _)
        | F::Scope(n, _)
        | F::Kind(n, _)
        | F::Insert(n, _) => Some(n.clone()),
        _ => None,
    }
}

/// An input's field written as the web writes it (`Object.assign(def, …)`;
/// a value made empty is taken out).
fn set_input(model: &mut kentos_processing::Model, name: &str, field: InputField) {
    let Some(input) = model.inputs.iter_mut().find(|i| i.name() == name) else {
        return;
    };
    let def = &mut input.0;
    fn put(def: &mut serde_json::Map<String, Value>, key: &str, value: Option<Value>) {
        match value {
            Some(v) => {
                def.insert(key.to_owned(), v);
            }
            None => {
                def.remove(key);
            }
        }
    }
    match field {
        InputField::Label(text) => put(def, "label", Some(json!(text))),
        InputField::Description(text) => {
            put(
                def,
                "description",
                (!text.is_empty()).then_some(json!(text)),
            );
        }
        InputField::Optional(on) => put(def, "optional", on.then_some(json!(true))),
        InputField::Scope(scope) => put(def, "default", Some(json!({ "scope": scope }))),
        InputField::Kind(kind) => {
            const ORDER: [&str; 7] = [
                "polygon", "polyline", "line", "point", "circle", "arc", "text",
            ];
            let mut kinds: Vec<String> = def
                .get("kinds")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|k| k.as_str().map(str::to_owned))
                .collect();
            match kinds.iter().position(|k| *k == kind) {
                Some(i) => {
                    kinds.remove(i);
                }
                None => kinds.push(kind),
            }
            let ordered: Vec<&str> = ORDER
                .iter()
                .copied()
                .filter(|k| kinds.iter().any(|x| x == k))
                .collect();
            put(def, "kinds", (!ordered.is_empty()).then(|| json!(ordered)));
        }
        InputField::Number(key, text) => {
            let t = text.trim().replace(',', ".");
            let optional = key != "default";
            if t.is_empty() && optional {
                put(def, key, None);
            } else if let Ok(n) = t.parse::<f64>()
                && n.is_finite()
            {
                put(
                    def,
                    key,
                    Some(kentos_processing::model::js_numbers(&json!(n))),
                );
            }
        }
        InputField::Integer(on) => put(def, "integer", on.then_some(json!(true))),
        InputField::Text(text) => put(def, "default", Some(json!(text))),
        InputField::AllowEmpty(on) => put(def, "allowEmpty", on.then_some(json!(true))),
        InputField::Flag(on) => put(def, "default", Some(json!(on))),
        InputField::NewLayer(text) => put(def, "default", Some(json!({ "newName": text }))),
    }
}
