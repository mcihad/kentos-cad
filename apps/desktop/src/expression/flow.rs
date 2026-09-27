//! The builder's Akış view on the desktop (DESIGN.md §7.16, docs/adr/0101):
//! the web's FlowMode (apps/web/src/ui/expression/FlowMode.ts). The text
//! stays the builder's: a change here goes to the core
//! (`kentos_expression::editor::flow`), which answers with the new text
//! (canonical), and the builder's editor takes it; its check, preview and
//! Tamam work on it as ever. Nodes put down and not connected yet are kept
//! here, with their places, while the builder is open.

use std::collections::HashMap;

use iced::Task;
use kentos_expression::editor::flow::{self as core, Edit, Flow, FlowNode, Tree};

use super::{Builder, ViewMode};
use crate::app::{App, Message};

/// How many changes back Ctrl+Z goes.
const UNDO: usize = 100;
/// The smallest scale the flow first opens at: node text stays readable.
pub(crate) const READABLE: f32 = 0.72;

/// Where the world is on the canvas: a world point `p` is at `p * k + (x, y)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct View {
    pub x: f32,
    pub y: f32,
    pub k: f32,
}

impl Default for View {
    fn default() -> Self {
        View {
            x: 24.0,
            y: 24.0,
            k: 1.0,
        }
    }
}

/// The flow while the builder is open.
#[derive(Default)]
pub(crate) struct FlowState {
    /// The trees not connected to the result (tree 0 is the builder's text).
    pub apart: Vec<Tree>,
    pub flow: Option<Flow>,
    pub selected: Option<String>,
    /// Each whole node's value on the previewed object, by id.
    pub values: HashMap<String, String>,
    pub view: View,
    /// Whether the view has framed the flow since it was shown.
    pub fitted: bool,
    /// The canvas' size (for framing).
    pub room: Option<(f32, f32)>,
    undo: Vec<Vec<Tree>>,
    redo: Vec<Vec<Tree>>,
    /// A palette entry carried out of the tree: it goes where the pointer lets it go.
    pub carrying: Option<String>,
    /// A value being written in the inspector, the node it is for, and what is wrong with it.
    pub draft: String,
    pub draft_for: Option<String>,
    pub draft_error: Option<String>,
    /// A change the core refused, in its words (until the next change).
    pub failed: Option<String>,
}

/// What the flow's canvas and inspector ask for.
#[derive(Clone, Debug)]
pub(crate) enum FlowEvent {
    Select(Option<String>),
    Connect {
        from: String,
        to: String,
        port: usize,
    },
    Disconnect {
        to: String,
        port: usize,
    },
    /// A tree not connected to the result, moved to a world point.
    Move {
        tree: usize,
        at: (f64, f64),
    },
    Remove(String),
    AddPort(String),
    RemovePort(String, usize),
    /// The carried palette entry let go on the canvas: its top-left, into an input when given.
    Drop {
        at: (f64, f64),
        to: Option<(String, usize)>,
    },
    /// The carried entry let go elsewhere.
    Uncarry,
    /// The canvas panned or zoomed.
    View(View),
    /// The canvas has this size.
    Room(f32, f32),
    /// Frame everything; zoom by a factor round the middle.
    Fit,
    Zoom(f32),
    /// A double click on a node: its value to edit.
    Open(String),
    /// The inspector's value: typed, then written (Enter).
    Draft(String),
    Commit,
    SetBool(bool),
    SetField(String),
    SetVariable(String),
    SetOperator(&'static str),
    SetFunction(String),
    SetNegated(bool),
    SetFold(bool),
    Undo,
    Redo,
}

impl FlowState {
    /// The trees: the builder's text first.
    pub fn trees(&self, text: &str) -> Vec<Tree> {
        std::iter::once(Tree::new(text, None))
            .chain(self.apart.iter().cloned())
            .collect()
    }

    pub fn node(&self, id: &str) -> Option<&FlowNode> {
        self.flow.as_ref()?.nodes.iter().find(|n| n.id == id)
    }

    /// The input a new node goes into: the selected node's first empty one,
    /// else the result's when the expression is empty.
    fn target(&self, text: &str) -> Option<(String, usize)> {
        if let Some(sel) = self.selected.as_ref().and_then(|s| self.node(s))
            && let Some(k) = sel.ports.iter().position(|p| p.from.is_none())
        {
            return Some((sel.id.clone(), k));
        }
        text.trim().is_empty().then(|| ("r".to_owned(), 0))
    }

    /// Where a new node goes when nothing says: left of the input it goes
    /// into, else under everything, below the middle of the view.
    fn free(&self, text: &str) -> (f64, f64) {
        let Some(f) = &self.flow else {
            return (-180.0, 0.0);
        };
        if let Some((node, port)) = self.target(text)
            && let Some(n) = self.node(&node)
        {
            return (n.x - core::COLUMN, n.y + n.ports[port].y - core::HEAD / 2.0);
        }
        let (w, _) = self.room.unwrap_or((600.0, 400.0));
        let centre = f64::from((w / 2.0 - self.view.x) / self.view.k);
        (centre - core::NODE_W / 2.0, f.bounds.3 + 24.0)
    }

    /// Frames the flow: not larger than its size; readable when `readable`
    /// (the result's side in view when it does not all fit).
    pub fn fit(&mut self, readable: bool) {
        let (Some(f), Some((w, h))) = (&self.flow, self.room) else {
            return;
        };
        let (l, t, r, b) = f.bounds;
        let pad = 28.0;
        let bw = ((r - l) as f32).max(1.0);
        let bh = ((b - t) as f32).max(1.0);
        let fits = ((w - pad * 2.0) / bw).min((h - pad * 2.0) / bh);
        let floor = if readable { READABLE } else { 0.3 };
        let k = fits.clamp(floor, 1.0);
        let x = if bw * k <= w - pad * 2.0 {
            (w - bw * k) / 2.0 - l as f32 * k
        } else {
            w - pad - r as f32 * k
        };
        let y = if bh * k <= h - pad * 2.0 {
            (h - bh * k) / 2.0 - t as f32 * k
        } else {
            pad - t as f32 * k
        };
        self.view = View { x, y, k };
    }

    /// Pans as little as needed for a node to be in view.
    fn reveal(&mut self, id: &str) {
        let (Some(n), Some((w, h))) = (self.node(id), self.room) else {
            return;
        };
        let v = self.view;
        let pad = 16.0;
        let left = n.x as f32 * v.k + v.x;
        let top = n.y as f32 * v.k + v.y;
        let (nw, nh) = (n.w as f32 * v.k, n.h as f32 * v.k);
        let dx = if left < pad {
            pad - left
        } else if left + nw > w - pad {
            w - pad - (left + nw)
        } else {
            0.0
        };
        let dy = if top < pad {
            pad - top
        } else if top + nh > h - pad {
            h - pad - (top + nh)
        } else {
            0.0
        };
        self.view.x += dx;
        self.view.y += dy;
    }
}

impl Builder {
    /// Replaces the whole text (the flow wrote it): read and checked, the cursor at its end.
    pub(crate) fn set_text(&mut self, text: &str) {
        if text == self.source {
            return;
        }
        self.content = iced::widget::text_editor::Content::with_text(text);
        self.content
            .perform(iced::widget::text_editor::Action::Move(
                iced::widget::text_editor::Motion::DocumentEnd,
            ));
        self.read();
        self.completion = None;
        self.moved(false);
    }

    /// Reads the text and the trees apart and lays out the flow.
    pub(crate) fn refresh_flow(&mut self) {
        let trees = self.flow.trees(&self.source);
        let flow = core::flow(&trees, &self.schema);
        self.flow.flow = Some(flow);
        if let Some(sel) = self.flow.selected.clone() {
            if self.flow.node(&sel).is_none() {
                self.flow.selected = None;
            } else {
                self.flow.reveal(&sel);
            }
        }
        if !self.flow.fitted && self.flow.room.is_some() {
            self.flow.fitted = true;
            self.flow.fit(true);
        }
        self.sync_draft();
    }

    /// The inspector's value field follows the selected value node.
    fn sync_draft(&mut self) {
        let sel = self.flow.selected.clone();
        if sel == self.flow.draft_for {
            return;
        }
        self.flow.draft_error = None;
        self.flow.draft = match sel.as_deref().and_then(|s| self.flow.node(s)) {
            Some(n) if n.kind == core::NodeKind::Number => n.title.clone(),
            Some(n) if n.kind == core::NodeKind::Text => unquote(&n.title),
            _ => String::new(),
        };
        self.flow.draft_for = sel;
    }

    /// Applies a change: the new text goes into the editor, the trees apart stay here.
    fn flow_edit(&mut self, change: Edit) {
        let before = self.flow.trees(&self.source);
        match core::edit(&before, &change, &self.schema) {
            Ok((trees, focus)) => {
                let focus = if matches!(change, Edit::Remove { .. }) {
                    None
                } else {
                    focus
                };
                self.flow_commit(before, trees, focus);
            }
            Err(message) => self.flow.failed = Some(message),
        }
    }

    /// Puts a node down and connects it into `to` (or where `target` says).
    fn flow_put(&mut self, add: Edit, to: Option<(String, usize)>) {
        let into = to.or_else(|| self.flow.target(&self.source));
        let before = self.flow.trees(&self.source);
        let added = core::edit(&before, &add, &self.schema).and_then(|(trees, focus)| {
            match (into, focus) {
                (Some((node, port)), Some(focus)) => core::edit(
                    &trees,
                    &Edit::Connect {
                        from: focus,
                        to: node,
                        port,
                    },
                    &self.schema,
                ),
                (_, focus) => Ok((trees, focus)),
            }
        });
        match added {
            Ok((trees, focus)) => self.flow_commit(before, trees, focus),
            Err(message) => self.flow.failed = Some(message),
        }
    }

    fn flow_commit(&mut self, before: Vec<Tree>, after: Vec<Tree>, focus: Option<String>) {
        self.flow.undo.push(before);
        if self.flow.undo.len() > UNDO {
            self.flow.undo.remove(0);
        }
        self.flow.redo.clear();
        self.flow_restore(after, focus);
    }

    fn flow_restore(&mut self, trees: Vec<Tree>, focus: Option<String>) {
        let mut trees = trees.into_iter();
        let text = trees.next().map(|t| t.text).unwrap_or_default();
        self.flow.apart = trees.collect();
        self.flow.selected = focus;
        self.flow.failed = None;
        self.set_text(&text);
        self.refresh_flow();
        let key = self
            .flow
            .selected
            .as_deref()
            .and_then(|s| self.flow.node(s))
            .and_then(|n| n.key.clone());
        match key {
            Some(key) => self.help_of(&key),
            None => self.help = None,
        }
    }

    /// A palette entry (a double click in the tree, an operator button).
    pub(crate) fn flow_add(&mut self, key: &str) {
        let at = self.flow.free(&self.source);
        self.flow_put(
            Edit::Add {
                key: key.to_owned(),
                at,
            },
            None,
        );
    }

    /// An expression as the language writes it (a field's value from the help).
    pub(crate) fn flow_add_text(&mut self, text: &str) {
        let at = self.flow.free(&self.source);
        self.flow_put(
            Edit::AddText {
                text: text.to_owned(),
                at,
            },
            None,
        );
    }

    fn flow_select(&mut self, id: Option<String>) {
        self.flow.selected = id;
        self.sync_draft();
        let key = self
            .flow
            .selected
            .as_deref()
            .and_then(|s| self.flow.node(s))
            .and_then(|n| n.key.clone());
        match key {
            Some(key) => self.help_of(&key),
            None => self.help = None,
        }
    }

    /// Each whole node's value on the previewed object.
    pub(crate) fn flow_values(
        &mut self,
        doc: Option<&kentos_domain::Document>,
        store: &kentos_geometry_core::store::Store,
    ) {
        self.flow.values.clear();
        let (Some(doc), Some(flow)) = (doc, &self.flow.flow) else {
            return;
        };
        if self.objects.is_empty() || flow.error.is_some() {
            return;
        }
        for n in &flow.nodes {
            if !n.whole || n.text.is_empty() {
                continue;
            }
            if let Ok(v) = self
                .objects
                .value(&n.text, &self.schema, self.index, doc, store)
            {
                self.flow
                    .values
                    .insert(n.id.clone(), kentos_expression::editor::preview(&v));
            }
        }
    }
}

/// 'it''s' → it's
fn unquote(s: &str) -> String {
    let inner = s
        .strip_prefix('\'')
        .and_then(|s| s.strip_suffix('\''))
        .unwrap_or(s);
    inner.replace("''", "'")
}

impl App {
    pub(crate) fn flow_event(&mut self, e: FlowEvent) -> Task<Message> {
        let Some(b) = self.builder.as_mut() else {
            return Task::none();
        };
        if b.view_mode != ViewMode::Flow {
            return Task::none();
        }
        let mut focus_value = false;
        match e {
            FlowEvent::Select(id) => b.flow_select(id),
            FlowEvent::Connect { from, to, port } => b.flow_edit(Edit::Connect { from, to, port }),
            FlowEvent::Disconnect { to, port } => b.flow_edit(Edit::Disconnect { to, port }),
            FlowEvent::Move { tree, at } => b.flow_edit(Edit::Move { tree, at }),
            FlowEvent::Remove(node) => b.flow_edit(Edit::Remove { node }),
            FlowEvent::AddPort(node) => b.flow_edit(Edit::AddPort { node }),
            FlowEvent::RemovePort(node, port) => b.flow_edit(Edit::RemovePort { node, port }),
            FlowEvent::Drop { at, to } => {
                if let Some(key) = b.flow.carrying.take() {
                    b.flow_put(Edit::Add { key, at }, to);
                }
            }
            FlowEvent::Uncarry => {
                b.flow.carrying = None;
                return Task::none();
            }
            FlowEvent::View(v) => {
                b.flow.view = v;
                return Task::none();
            }
            FlowEvent::Room(w, h) => {
                let first = b.flow.room.is_none();
                b.flow.room = Some((w, h));
                if first || !b.flow.fitted {
                    b.flow.fitted = true;
                    b.flow.fit(true);
                }
                return Task::none();
            }
            FlowEvent::Fit => {
                b.flow.fit(false);
                return Task::none();
            }
            FlowEvent::Zoom(f) => {
                if let Some((w, h)) = b.flow.room {
                    let v = b.flow.view;
                    let k = (v.k * f).clamp(0.3, 2.0);
                    let (cx, cy) = (w / 2.0, h / 2.0);
                    let (wx, wy) = ((cx - v.x) / v.k, (cy - v.y) / v.k);
                    b.flow.view = View {
                        k,
                        x: cx - wx * k,
                        y: cy - wy * k,
                    };
                }
                return Task::none();
            }
            FlowEvent::Open(id) => {
                b.flow_select(Some(id));
                focus_value = true;
            }
            FlowEvent::Draft(text) => {
                b.flow.draft = text;
                b.flow.draft_error = None;
                return Task::none();
            }
            FlowEvent::Commit => {
                let Some(node) = b.flow.selected.clone() else {
                    return Task::none();
                };
                let kind = b.flow.node(&node).map(|n| n.kind);
                let draft = b.flow.draft.clone();
                match kind {
                    Some(core::NodeKind::Number) => match draft.trim().parse::<f64>() {
                        Ok(value) if value.is_finite() => {
                            b.flow_edit(Edit::SetNumber { node, value })
                        }
                        _ => {
                            b.flow.draft_error =
                                Some("Bir sayı yazın; ondalık ayırıcı noktadır (12.5).".into());
                            return Task::none();
                        }
                    },
                    Some(core::NodeKind::Text) => b.flow_edit(Edit::SetText { node, value: draft }),
                    _ => return Task::none(),
                }
            }
            FlowEvent::SetBool(value) => {
                if let Some(node) = b.flow.selected.clone() {
                    b.flow_edit(Edit::SetBool { node, value });
                }
            }
            FlowEvent::SetField(name) => {
                if let Some(node) = b.flow.selected.clone() {
                    b.flow_edit(Edit::SetField { node, name });
                }
            }
            FlowEvent::SetVariable(name) => {
                if let Some(node) = b.flow.selected.clone() {
                    b.flow_edit(Edit::SetVariable { node, name });
                }
            }
            FlowEvent::SetOperator(symbol) => {
                if let Some(node) = b.flow.selected.clone() {
                    b.flow_edit(Edit::SetOperator {
                        node,
                        symbol: symbol.to_owned(),
                    });
                }
            }
            FlowEvent::SetFunction(name) => {
                if let Some(node) = b.flow.selected.clone() {
                    b.flow_edit(Edit::SetFunction { node, name });
                }
            }
            FlowEvent::SetNegated(value) => {
                if let Some(node) = b.flow.selected.clone() {
                    b.flow_edit(Edit::SetNegated { node, value });
                }
            }
            FlowEvent::SetFold(value) => {
                if let Some(node) = b.flow.selected.clone() {
                    b.flow_edit(Edit::SetFold { node, value });
                }
            }
            FlowEvent::Undo => {
                if let Some(t) = b.flow.undo.pop() {
                    let now = b.flow.trees(&b.source);
                    b.flow.redo.push(now);
                    b.flow_restore(t, None);
                }
            }
            FlowEvent::Redo => {
                if let Some(t) = b.flow.redo.pop() {
                    let now = b.flow.trees(&b.source);
                    b.flow.undo.push(now);
                    b.flow_restore(t, None);
                }
            }
        }
        let doc = self.document.as_ref().map(|d| &d.model);
        let store = self.spatial.store();
        if let Some(b) = self.builder.as_mut() {
            b.preview_on(doc, store);
        }
        if focus_value {
            return iced::widget::operation::focus(super::flow_view::VALUE);
        }
        Task::none()
    }
}
