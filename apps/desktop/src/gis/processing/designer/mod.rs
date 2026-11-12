//! Model tasarımcısı on the desktop (docs/specs/model-designer.md,
//! docs/adr/0116): the web's ModelDesigner. A model is built as a flow
//! diagram: the parts on the left (inputs by type, the tools by category),
//! the diagram in the middle, the selected box's settings on the right. The
//! designer edits a draft; Kaydet writes it to the user's library
//! (`islemler.json`). It keeps its own undo, apart from the drawing's.
//!
//! The rules and words are `kentos_processing::designer` and
//! `model_edit`'s, held to fixtures/processing/v1/designer.json on both
//! platforms; here are the window's state and what its events do. How it
//! looks: `view.rs`, `canvas.rs` (the diagram), `palette.rs`,
//! `inspector.rs`.

mod canvas;
mod inspector;
mod palette;
#[cfg(test)]
mod tests;
mod update;
mod view;

use std::collections::BTreeMap;

use iced::Size;
use kentos_processing::ValueSource;
use kentos_processing::designer::{self as plan, View, canvas as sizes};
use kentos_processing::model::{Model, ModelIssue, check_model};
use kentos_processing::model_edit::NodeRef;
use kentos_processing::{Registry, Tool};

pub(crate) use update::Event;

use super::dialog::ToolDialog;

/// A copy of the draft and what was selected, for undo.
#[derive(Clone)]
struct Snapshot {
    model: Model,
    selected: Option<NodeRef>,
}

/// What the window asks before it closes or deletes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Asking {
    /// Kaydedilmemiş değişiklikler: Kaydetmeden kapat, Vazgeç, Kaydet ve kapat.
    Unsaved,
    /// Modeli sil.
    Remove,
}

/// The menu a wire dropped on a step opens, at the canvas point it was dropped.
#[derive(Clone, Debug)]
pub(crate) struct WireMenu {
    pub from: NodeRef,
    pub to: String,
    pub at: iced::Point,
}

/// The designer's state while it is open.
pub(crate) struct Designer {
    pub draft: Model,
    /// The draft as last saved; none while the model is not in the library.
    saved: Option<serde_json::Value>,
    /// What “unsaved” compares with: the draft as last saved or as opened.
    baseline: serde_json::Value,
    /// A copy of a built-in model (it cannot be deleted from here).
    pub builtin_copy: bool,
    pub selected: Option<NodeRef>,
    past: Vec<Snapshot>,
    future: Vec<Snapshot>,
    /// The last keyed change and when (ms): typing into one field joins one undo step.
    last_key: Option<(String, i64)>,
    /// The draft before a box started to move: one undo step when it stops.
    drag_start: Option<Snapshot>,
    /// The draft's problems, as checked after every change.
    pub problems: Vec<ModelIssue>,
    /// The diagram's view and the lowest scale zooming reaches.
    pub view: View,
    pub floor: f64,
    /// The diagram's size, once it is laid out; a fit waits for it.
    pub canvas: Option<Size>,
    fit_pending: bool,
    /// Araç ara's text.
    pub search: String,
    /// A tool carried from the parts onto the diagram.
    pub carrying: Option<String>,
    pub wire_menu: Option<WireMenu>,
    pub asking: Option<Asking>,
    /// The selected step's fixed values, as the tool's window holds them
    /// (the same controls; docs/specs/model-designer.md §6.3).
    pub fields: Option<ToolDialog>,
    /// Gelişmiş (n) opened in the step's settings.
    pub advanced_open: bool,
    /// A number input's field as typed (`<input>.<field>`): "1." stays while
    /// the model holds 1, until another box is selected or a step undone.
    pub typed: BTreeMap<String, String>,
}

impl Designer {
    /// The designer on `draft`; `saved`: it is in the library as it is.
    pub(crate) fn new(mut draft: Model, saved: bool, builtin_copy: bool) -> Self {
        // A box without a place: the whole diagram is laid out.
        let placed = draft.steps.iter().all(|s| s.position.is_some())
            && draft
                .inputs
                .iter()
                .all(|i| draft.input_positions.contains_key(i.name()));
        if !placed {
            kentos_processing::model_edit::auto_layout(&mut draft);
        }
        let json = draft.to_json();
        Self {
            draft,
            saved: saved.then(|| json.clone()),
            baseline: json,
            builtin_copy,
            selected: None,
            past: Vec::new(),
            future: Vec::new(),
            last_key: None,
            drag_start: None,
            problems: Vec::new(),
            view: sizes::EMPTY,
            floor: sizes::ZOOM_MIN,
            canvas: None,
            fit_pending: true,
            search: String::new(),
            carrying: None,
            wire_menu: None,
            asking: None,
            fields: None,
            advanced_open: false,
            typed: BTreeMap::new(),
        }
    }

    /// Whether the draft differs from the saved or opened model.
    pub(crate) fn dirty(&self) -> bool {
        self.draft.to_json() != self.baseline
    }

    /// Whether the model is in the library (Modeli sil is offered).
    pub(crate) fn is_saved(&self, registry: &Registry) -> bool {
        self.saved.is_some() && registry.model(&self.draft.id).is_some()
    }

    /// The window's title (the web's `titleOf`).
    pub(crate) fn title(&self) -> String {
        plan::texts::title_of(&self.draft.label, self.dirty())
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            model: self.draft.clone(),
            selected: self.selected.clone(),
        }
    }

    /// Before a change: one undo step, unless it joins the last (the same
    /// field typed into again within 1.2 s).
    fn before_change(&mut self, key: Option<&str>, now: i64) {
        let last = self.last_key.as_ref().map(|(k, at)| (k.as_str(), *at));
        if !plan::joins(key, last, now) {
            self.past.push(self.snapshot());
            if self.past.len() > plan::HISTORY_DEPTH {
                self.past.remove(0);
            }
            self.future.clear();
        }
        self.last_key = key.map(|k| (k.to_owned(), now));
    }

    /// After every change: the problems again, and a selection that went
    /// away let go of.
    fn after_change(&mut self, lookup: &dyn Fn(&str) -> Option<Tool>) {
        self.problems = check_model(&self.draft, lookup);
        let exists = match &self.selected {
            Some(NodeRef::Input(name)) => self.draft.inputs.iter().any(|i| i.name() == name),
            Some(NodeRef::Step(id)) => self.draft.steps.iter().any(|s| &s.id == id),
            None => true,
        };
        if !exists {
            self.selected = None;
        }
    }

    /// Ctrl+Z (or Ctrl+Y, `redo`): the model and the selection as they were.
    fn undo(&mut self, redo: bool) -> bool {
        let snap = if redo {
            self.future.pop()
        } else {
            self.past.pop()
        };
        let Some(snap) = snap else {
            return false;
        };
        let now = self.snapshot();
        if redo {
            self.past.push(now);
        } else {
            self.future.push(now);
        }
        self.draft = snap.model;
        self.selected = snap.selected;
        self.last_key = None;
        self.typed.clear();
        true
    }

    /// A box moves: live while it is dragged, one undo step when it stops.
    fn move_box(&mut self, node: &NodeRef, at: (f64, f64), done: bool) {
        if self.drag_start.is_none() {
            self.drag_start = Some(self.snapshot());
        }
        match node {
            NodeRef::Input(name) => {
                self.draft.input_positions.insert(name.clone(), at);
            }
            NodeRef::Step(id) => {
                if let Some(s) = self.draft.steps.iter_mut().find(|s| &s.id == id) {
                    s.position = Some(at);
                }
            }
        }
        if done && let Some(start) = self.drag_start.take() {
            self.past.push(start);
            if self.past.len() > plan::HISTORY_DEPTH {
                self.past.remove(0);
            }
            self.future.clear();
            self.last_key = None;
        }
    }

    /// Every box in view (Tümünü göster; at the first layout, after Düzenle).
    fn fit(&mut self) {
        match self.canvas {
            Some(size) => {
                self.view = plan::fit_view(
                    plan::boxes_bounds(&self.draft),
                    f64::from(size.width),
                    f64::from(size.height),
                );
                self.floor = plan::zoom_floor(self.view.k);
                self.fit_pending = false;
            }
            None => self.fit_pending = true,
        }
    }

    /// The first problem of each step (a box's second line, its dashed edge).
    pub(crate) fn first_problem(&self, step: &str) -> Option<&str> {
        self.problems
            .iter()
            .find(|p| p.step.as_deref() == Some(step))
            .map(|p| p.message.as_str())
    }

    /// A step parameter's source.
    pub(crate) fn source(&self, step: &str, param: &str) -> Option<&ValueSource> {
        self.draft
            .steps
            .iter()
            .find(|s| s.id == step)
            .and_then(|s| s.source(param))
    }
}
