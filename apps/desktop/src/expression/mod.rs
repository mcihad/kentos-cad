//! İfade oluşturucu on the desktop (DESIGN.md §7.16, docs/adr/0100 §5): the
//! web's builder (apps/web/src/ui/expression/) in KentOS UI. It opens over
//! the window whose expression field asked for it (the processing window's
//! ε button) and edits that field's text:
//!
//! - the editor paints the core's token classes, the error and the warnings,
//!   the parenthesis at the cursor and its pair; completion opens as a name
//!   is typed and on Ctrl+Space; the call's signature shows under it;
//! - the expression is previewed on the field's objects one at a time, or
//!   on one picked on the drawing (Sahneden seç, docs/adr/0088);
//! - the tree lists the fields and their values, the variables, functions
//!   and operators; the help explains what is chosen, highlighted in the
//!   list or under the cursor.
//!
//! Tamam writes the text back; Vazgeç, Esc and × leave the field as it was.
//! Every meaning is the core's (`kentos_expression::editor`), the same the
//! web's builder asks through WASM (fixtures/expression/v2/builder.json).

mod editor;
mod flow;
mod flow_canvas;
mod flow_view;
mod highlight;
mod objects;
#[cfg(test)]
mod tests;
mod view;

use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::{Duration, Instant};

use iced::Task;
use iced::widget::scrollable;
use iced::widget::text_editor::{self, Action, Cursor, Edit, Position};
use kentos_domain::Slot;
use kentos_expression::editor::{
    self as core, Bracket, Check, Completion, Help, Item, Kind, Signature, ValueItem, units,
};
use kentos_expression::{FieldDef, FieldSource, FieldType, Schema};
use kentos_interaction::Level;
use kentos_processing::ParamKind;
use kentos_processing::features;
use kentos_processing::values::FeaturesValue;
use serde_json::{Value, json};

use crate::app::{App, Dialog, Message};
pub(crate) use flow::FlowEvent;
use flow::FlowState;
use highlight::{Lines, Paint};
pub(crate) use objects::Objects;

/// The editor's widget id: the builder gives it the keyboard when it opens.
pub(crate) const EDITOR: &str = "expression-builder";
/// The completion list's scrollable: its highlighted entry is kept in view.
pub(crate) const LIST: &str = "expression-builder-list";

/// A second press on the same row within this is a double click.
const DOUBLE: Duration = Duration::from_millis(450);

/// Values listed at most in the help; the rest are counted.
const SHOWN: usize = 2000;

/// The builder's two views of one text (docs/adr/0101).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ViewMode {
    /// The editor.
    Text,
    /// The same expression as nodes.
    Flow,
}

/// Where the builder's text goes back.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Target {
    /// The processing window's expression parameter of this name.
    Processing(String),
    /// Öznitelik tablosu's İfade süzgeci (features/, docs/adr/0199 §4).
    FeatureFilter,
}

/// A field's values listed in the help.
#[derive(Clone, Debug, Default)]
pub(crate) struct Listed {
    pub field: String,
    pub items: Vec<ValueItem>,
    /// How many distinct values there are (more than listed when many).
    pub total: usize,
    pub chosen: Option<usize>,
}

/// The builder while it is open.
pub(crate) struct Builder {
    pub target: Target,
    /// The field's label, beside the title.
    pub context: String,
    pub content: text_editor::Content,
    pub schema: Schema,
    pub objects: Objects,
    /// The text as last read, its tokens, problems and the highlighter's table.
    pub source: String,
    pub check: Check,
    pub lines: Lines,
    /// The cursor, in UTF-16 units, and what it sits in.
    pub cursor: usize,
    pub signature: Option<Signature>,
    pub bracket: Option<Bracket>,
    /// The completion list and its highlighted entry.
    pub completion: Option<Completion>,
    pub active: usize,
    /// The help in the right column.
    pub help: Option<Help>,
    /// The tree: its search, the groups opened by hand, the chosen entry.
    pub query: String,
    pub open: BTreeSet<String>,
    pub chosen: Option<String>,
    last_press: Option<(String, Instant)>,
    /// The field's values listed in the help.
    pub listed: Option<Listed>,
    last_value: Option<(usize, Instant)>,
    /// The object the preview is on, its name (“Kapalı alan 12”), and what the preview shows.
    pub index: usize,
    pub described: String,
    pub preview: Preview,
    /// The editor's scroll (the list stands under the word being written) and its box.
    pub scroll: (f32, f32),
    room: Option<(f32, f32)>,
    /// The dialog under the builder while an object is picked on the drawing.
    picking: Option<Picking>,
    /// Metin or Akış, and the flow's own state.
    pub view_mode: ViewMode,
    pub flow: FlowState,
}

/// What the preview shows.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Preview {
    /// The value, as the core writes it ('text', 12.5, doğru, boş).
    Value(String),
    /// Why there is none (no objects, an error): in the muted tone.
    Note(String),
}

/// The builder stepped aside while an object is picked on the drawing.
struct Picking {
    dialog: Option<Dialog>,
    before: Vec<Slot>,
}

/// What the builder asks for.
#[derive(Clone, Debug)]
pub(crate) enum Event {
    /// The processing window's ε beside the expression parameter of this name.
    OpenProcessing(String),
    /// The editor's action: typing, moving, selecting, scrolling.
    Edit(Action),
    /// Ctrl+Space: every entry that can stand at the cursor.
    Complete,
    /// ↑ ↓, Page Up and Page Down in the list.
    Step(i32),
    /// A click on an entry of the list.
    Accept(usize),
    /// Enter or Tab in the list: its highlighted entry.
    AcceptActive,
    /// The editor's scrollable moved: its offset.
    Scrolled(f32, f32),
    /// The editor's box has this size (it came into view or was resized).
    Room(f32, f32),
    /// Esc with the list open.
    CloseList,
    /// An operator button: its label, what it inserts and how it stands.
    Operator(&'static str, &'static str, Kind),
    /// Metin or Akış.
    Mode(ViewMode),
    /// Something of the flow (docs/adr/0101).
    Flow(FlowEvent),
    /// A tree row carried out of the tree (Akış): its entry, by its place in the tree.
    Carry(usize),
    /// The tree's search.
    Query(String),
    /// Enter in the search (or with the tree's entry chosen by the keys):
    /// the chosen entry goes in, else the first one found.
    QuerySubmit,
    /// ↑ or ↓ outside the editor: the tree's entry before or after the chosen one.
    TreeStep(i32),
    /// A group opened or closed.
    Toggle(String),
    /// A row of the tree pressed; twice soon, its entry goes in.
    Row(String),
    /// Örnek değerler (10), or Tüm değerler.
    Values(bool),
    /// A listed value pressed; twice soon, it goes in.
    Value(usize),
    /// The preview's object before or after.
    Object(i32),
    /// Sahneden seç: the preview's object picked on the drawing.
    Pick,
    Ok,
    Cancel,
}

fn ev(e: Event) -> Message {
    Message::Builder(e)
}

/// Today's text attributes as the builder's fields, with how many objects have each.
fn attribute_fields(fields: &[(String, usize)]) -> Schema {
    Schema {
        fields: fields
            .iter()
            .map(|(name, count)| FieldDef {
                name: name.clone(),
                ty: FieldType::Text,
                source: FieldSource::Attribute,
                description: format!("{count} nesnede var."),
            })
            .collect(),
    }
}

/// The values Akış' palette offers besides the tree's entries: a number and a text to write.
pub(crate) fn values_section(query: &str) -> Vec<Item> {
    let q = kentos_expression::js::text::fold_turkish(query.trim());
    [
        ("Sayı", "Yazılacak bir sayı: 0", "lit:number"),
        ("Metin", "Yazılacak bir metin: ''", "lit:text"),
    ]
    .into_iter()
    .filter(|(label, _, _)| {
        q.is_empty() || kentos_expression::js::text::fold_turkish(label).contains(&q)
    })
    .map(|(label, detail, key)| Item {
        kind: Kind::Operator,
        label: label.to_owned(),
        detail: detail.to_owned(),
        insert: String::new(),
        caret: 0,
        key: key.to_owned(),
        alias: None,
    })
    .collect()
}

/// The name characters that open (or keep) the list as they are typed.
fn names(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '$' | '[')
}

impl Builder {
    pub fn new(
        target: Target,
        context: String,
        value: &str,
        schema: Schema,
        objects: Objects,
    ) -> Self {
        let mut content = text_editor::Content::with_text(value);
        content.perform(Action::Move(text_editor::Motion::DocumentEnd));
        let mut b = Builder {
            target,
            context,
            content,
            schema,
            objects,
            source: String::new(),
            check: Check::default(),
            lines: Lines::default(),
            cursor: 0,
            signature: None,
            bracket: None,
            completion: None,
            active: 0,
            help: None,
            query: String::new(),
            open: BTreeSet::from(["fields".to_owned()]),
            chosen: None,
            last_press: None,
            listed: None,
            last_value: None,
            index: 0,
            described: String::new(),
            preview: Preview::Note(String::new()),
            scroll: (0.0, 0.0),
            room: None,
            picking: None,
            view_mode: ViewMode::Text,
            flow: FlowState::default(),
        };
        b.read();
        b.moved(true);
        b
    }

    /// Shows the text or the flow of the same expression.
    pub(crate) fn set_mode(&mut self, mode: ViewMode) {
        self.view_mode = mode;
        self.completion = None;
        self.help = None;
        self.chosen = None;
        if mode == ViewMode::Flow {
            self.flow.fitted = false;
            self.refresh_flow();
        }
    }

    /// The tree's entries in Akış' order, with the values to write first:
    /// what a carried row's place in the tree names.
    pub(crate) fn palette(&self) -> Vec<String> {
        let mut keys: Vec<String> = Vec::new();
        if self.view_mode == ViewMode::Flow {
            keys.extend(values_section(&self.query).into_iter().map(|i| i.key));
        }
        keys.extend(self.shown().into_iter().map(|i| i.key));
        keys
    }

    /// The text changed: its tokens, problems and colours again.
    pub(crate) fn read(&mut self) {
        self.source = self.content.text();
        self.check = core::check(&self.source, &self.schema);
        self.bracket = None;
        self.repaint();
    }

    /// The highlighter's table: the tokens, then the error, the warnings and the parentheses.
    fn repaint(&mut self) {
        let tokens = core::tokens(&self.source);
        let mut marks: Vec<(usize, usize, Paint)> = self
            .check
            .warnings
            .iter()
            .map(|w| (w.start, w.end, Paint::Warning))
            .collect();
        if let Some(e) = &self.check.error {
            marks.push((e.start, e.end, Paint::Error));
        }
        if let Some(b) = self.bracket {
            let paint = if b.partner.is_some() {
                Paint::Match
            } else {
                Paint::Unmatched
            };
            marks.push((b.at, b.at + 1, paint));
            if let Some(p) = b.partner {
                marks.push((p, p + 1, paint));
            }
        }
        self.lines = Lines::of(&self.source, &tokens, &marks);
    }

    /// The cursor in UTF-16 units, and the selection's other end.
    fn cursor_units(&self) -> (usize, usize) {
        let c = self.content.cursor();
        let at = |p: Position| units::from_line_byte(&self.source, p.line, p.column);
        let head = at(c.position);
        (head, c.selection.map_or(head, at))
    }

    /// The cursor moved (or the text under it changed): the signature, the
    /// parentheses and, while one writes, the help of what is under it.
    pub(crate) fn moved(&mut self, help: bool) {
        let (head, anchor) = self.cursor_units();
        self.cursor = head;
        self.signature = core::signature(&self.source, head);
        let bracket = (head == anchor)
            .then(|| core::bracket(&self.source, head))
            .flatten();
        if bracket != self.bracket {
            self.bracket = bracket;
            self.repaint();
        }
        if help
            && self.completion.is_none()
            && let Some(h) = core::help_at(&self.source, head, &self.schema)
        {
            self.show_help(h);
        }
    }

    fn show_help(&mut self, h: Help) {
        if self.help.as_ref().is_some_and(|old| old.key == h.key) {
            return;
        }
        // Another field's values are not this one's.
        if self
            .listed
            .as_ref()
            .is_some_and(|l| format!("field:{}", l.field) != h.key)
        {
            self.listed = None;
        }
        self.help = Some(h);
    }

    /// Replaces `start..end` (UTF-16 units) with `text`, the cursor at `caret` in it.
    fn replace(&mut self, start: usize, end: usize, text: &str, caret: usize) {
        let src = &self.source;
        let (l0, c0) = units::to_line_byte(src, start);
        let (l1, c1) = units::to_line_byte(src, end.max(start));
        self.content.move_to(Cursor {
            position: Position {
                line: l1,
                column: c1,
            },
            selection: (start != end).then_some(Position {
                line: l0,
                column: c0,
            }),
        });
        self.content
            .perform(Action::Edit(Edit::Paste(Arc::new(text.to_owned()))));
        self.read();
        let at = start + caret;
        let (line, column) = units::to_line_byte(&self.source, at);
        self.content.move_to(Cursor {
            position: Position { line, column },
            selection: None,
        });
        self.moved(true);
    }

    /// Places an entry over the selection as the core says (a function wraps
    /// it, an operator keeps single spaces).
    fn place(&mut self, kind: Kind, insert: &str, caret: usize) {
        let (head, anchor) = self.cursor_units();
        let (start, end) = (head.min(anchor), head.max(anchor));
        let (text, at) = core::place(&self.source, start, end, kind, insert, caret);
        let before = start;
        let after = units::from_byte(&self.source, self.source.len()) - end;
        let total = units::from_byte(&text, text.len());
        let middle_end = units::to_byte(&text, total - after);
        let middle = &text[units::to_byte(&text, before)..middle_end];
        self.replace(start, end, middle, at - start);
    }

    fn complete(&mut self, explicit: bool) {
        match core::complete(&self.source, self.cursor, &self.schema, explicit) {
            Some(c) if !c.items.is_empty() => {
                self.active = 0;
                if let Some(first) = c.items.first() {
                    let key = first.key.clone();
                    self.completion = Some(c);
                    self.help_of(&key);
                }
            }
            _ => self.completion = None,
        }
    }

    pub(crate) fn help_of(&mut self, key: &str) {
        if let Some(h) = core::help(key, &self.schema) {
            self.show_help(h);
        }
    }

    fn close_list(&mut self) {
        if self.completion.take().is_some() {
            self.moved(true);
        }
    }

    fn accept(&mut self, i: usize) {
        let Some(c) = self.completion.take() else {
            return;
        };
        if let Some(item) = c.items.get(i) {
            let (insert, caret) = (item.insert.clone(), item.caret);
            self.replace(c.start, c.end, &insert, caret);
        }
    }

    /// The tree's entries as they show now: those of the open groups (every
    /// group while searching), in order.
    fn shown(&self) -> Vec<Item> {
        let searching = !self.query.trim().is_empty();
        core::catalog(&self.schema, &self.query)
            .into_iter()
            .filter(|s| searching || self.open.contains(s.group.id()))
            .flat_map(|s| s.items)
            .collect()
    }

    /// The tree's entry of a key, in the tree as it is searched now.
    fn entry(&self, key: &str) -> Option<Item> {
        core::catalog(&self.schema, &self.query)
            .into_iter()
            .flat_map(|s| s.items)
            .find(|i| i.key == key)
    }

    /// The value the preview shows, on the object it is on.
    fn preview_on(
        &mut self,
        doc: Option<&kentos_domain::Document>,
        store: &kentos_geometry_core::store::Store,
    ) {
        let n = self.objects.len();
        // The stepper names its object whatever the text is.
        if let (Some(doc), Some(last)) = (doc, n.checked_sub(1)) {
            self.index = self.index.min(last);
            self.described = self.objects.describe(self.index, doc);
        }
        if self.view_mode == ViewMode::Flow {
            self.flow_values(doc, store);
        }
        self.preview = match doc {
            _ if n == 0 => Preview::Note("Önizleme için nesne yok.".into()),
            _ if self.source.trim().is_empty() => Preview::Note("—".into()),
            _ if self.check.error.is_some() => Preview::Note("—".into()),
            None => Preview::Note("Açık çizim yok.".into()),
            Some(doc) => {
                match self
                    .objects
                    .value(&self.source, &self.schema, self.index, doc, store)
                {
                    Ok(v) => Preview::Value(core::preview(&v)),
                    Err(e) => Preview::Note(e),
                }
            }
        };
    }

    /// Whether the builder is asking the drawing for an object.
    pub fn is_picking(&self) -> bool {
        self.picking.is_some()
    }

    /// The scroll that brings the cursor into view, when it is out of it.
    fn follow(&self) -> Option<Task<Message>> {
        let (width, height) = self.room?;
        let m = editor::Metrics::now();
        let (line, column) = editor::line_column(&self.source, self.cursor);
        let cell = m.cell(line, column);
        let (x0, y0) = self.scroll;
        let margin = 2.0 * m.advance;
        let x = if cell.x - margin < x0 {
            (cell.x - margin).max(0.0)
        } else if cell.x + margin > x0 + width {
            cell.x + margin - width
        } else {
            x0
        };
        let y = if cell.y < y0 {
            cell.y.max(0.0)
        } else if cell.y + m.line * 1.5 > y0 + height {
            cell.y + m.line * 1.5 - height
        } else {
            y0
        };
        ((x - x0).abs() > 0.5 || (y - y0).abs() > 0.5).then(|| {
            iced::widget::operation::scroll_to(
                editor::SCROLLER,
                scrollable::AbsoluteOffset { x, y },
            )
        })
    }
}

impl App {
    /// The processing window's ε beside an expression field: the builder on
    /// that field's text, fields and objects.
    pub(crate) fn open_builder_for_processing(&mut self, name: &str) -> Task<Message> {
        let (Some(doc), Some(window)) = (&self.document, &self.processing.dialog) else {
            return Task::none();
        };
        let Some(def) = window.tool.parameters.iter().find(|p| p.name == name) else {
            return Task::none();
        };
        let ParamKind::Expression { of, .. } = &def.kind else {
            return Task::none();
        };
        let value = window
            .values
            .get(name)
            .and_then(Value::as_str)
            .unwrap_or("");
        // The objects the parameter's input resolves to now, in run order, and their attributes.
        let look = crate::processing::Look {
            doc: &doc.model,
            selection: &self.selection,
            store: self.spatial.store(),
            view: Some(self.viewport.camera.visible_bounds()),
        };
        let input = of
            .as_ref()
            .and_then(|of| window.tool.parameters.iter().find(|p| &p.name == of));
        let (fields, slots): (Vec<(String, usize)>, Vec<Slot>) = match input.map(|p| {
            (
                &p.kind,
                window.values.get(&p.name).and_then(FeaturesValue::read),
            )
        }) {
            Some((ParamKind::Features { kinds, .. }, Some(fv))) => {
                let kinds = kinds.as_deref();
                let summary = features::summarize_features(&fv, kinds, &look);
                let set = features::resolve_features(&fv, kinds, &look);
                (
                    summary.fields,
                    set.entities.iter().map(|e| Slot(e.base().id)).collect(),
                )
            }
            _ => (Vec::new(), Vec::new()),
        };
        let mut builder = Builder::new(
            Target::Processing(name.to_owned()),
            def.label.clone(),
            value,
            attribute_fields(&fields),
            Objects { slots },
        );
        builder.preview_on(Some(&doc.model), self.spatial.store());
        // It opens in the view it was left in last (Metin or Akış).
        if self.builder_flow {
            builder.set_mode(ViewMode::Flow);
        }
        self.builder = Some(builder);
        iced::widget::operation::focus(EDITOR)
    }

    /// Öznitelik tablosu's ε: the builder on its filter, the layer's keys and objects.
    pub(crate) fn open_builder_for_features(&mut self) -> Task<Message> {
        let Some(doc) = &self.document else {
            return Task::none();
        };
        let rows = self.feature_rows(doc);
        let fields: Vec<(String, usize)> = rows
            .table
            .columns
            .iter()
            .filter_map(|c| c.key.clone())
            .map(|k| {
                let n = rows
                    .slots
                    .iter()
                    .filter(|&&s| doc.model.get(s).is_some_and(|e| e.base().attrs.contains_key(&k)))
                    .count();
                (k, n)
            })
            .collect();
        let mut builder = Builder::new(
            Target::FeatureFilter,
            crate::features::texts::FILTER.to_owned(),
            &self.features.filter,
            attribute_fields(&fields),
            Objects {
                slots: rows.slots.clone(),
            },
        );
        builder.preview_on(Some(&doc.model), self.spatial.store());
        if self.builder_flow {
            builder.set_mode(ViewMode::Flow);
        }
        self.builder = Some(builder);
        iced::widget::operation::focus(EDITOR)
    }

    pub(crate) fn builder_event(&mut self, e: Event) -> Task<Message> {
        if let Event::OpenProcessing(name) = &e {
            return self.open_builder_for_processing(name);
        }
        if let Event::Mode(mode) = &e {
            self.builder_flow = *mode == ViewMode::Flow;
        }
        let Some(b) = self.builder.as_mut() else {
            return Task::none();
        };
        let mut focus = false;
        match e {
            Event::Edit(Action::Scroll { lines }) => {
                // The editor never scrolls inside (the canvas over it stays true): the wheel
                // over it scrolls the scrollable round it.
                let by = lines as f32 * editor::Metrics::now().line;
                return iced::widget::operation::scroll_by(
                    editor::SCROLLER,
                    scrollable::AbsoluteOffset { x: 0.0, y: by },
                );
            }
            Event::Edit(action) => {
                let typed = match &action {
                    Action::Edit(Edit::Insert(c)) => Some(*c),
                    _ => None,
                };
                let deleting = matches!(action, Action::Edit(Edit::Backspace | Edit::Delete));
                let edit = action.is_edit();
                b.content.perform(action);
                if edit {
                    b.read();
                }
                b.moved(true);
                if typed.is_some_and(names) || (deleting && b.completion.is_some()) {
                    b.complete(false);
                } else if edit {
                    b.completion = None;
                }
                if !edit {
                    return b.follow().unwrap_or_else(Task::none);
                }
            }
            Event::Complete => b.complete(true),
            Event::Step(by) => {
                if let Some(c) = &b.completion {
                    let n = c.items.len() as i32;
                    let next = if by.abs() == 1 {
                        (b.active as i32 + by).rem_euclid(n)
                    } else {
                        (b.active as i32 + by).clamp(0, n - 1)
                    };
                    b.active = next as usize;
                    let key = c.items[b.active].key.clone();
                    b.help_of(&key);
                }
                return Task::none();
            }
            Event::Accept(i) => {
                b.accept(i);
                focus = true;
            }
            Event::AcceptActive => {
                let i = b.active;
                b.accept(i);
            }
            Event::Scrolled(x, y) => {
                b.scroll = (x, y);
                return Task::none();
            }
            Event::Room(w, h) => {
                b.room = Some((w, h));
                return Task::none();
            }
            Event::CloseList => {
                b.close_list();
                return Task::none();
            }
            Event::Operator(face, insert, kind) => {
                if b.view_mode == ViewMode::Flow {
                    b.flow_add(&format!("op:{face}"));
                } else {
                    b.place(kind, insert, insert.encode_utf16().count());
                    focus = true;
                }
            }
            Event::Mode(mode) => {
                b.set_mode(mode);
                focus = mode == ViewMode::Text;
            }
            Event::Flow(e) => return self.flow_event(e),
            Event::Carry(i) => {
                if b.view_mode == ViewMode::Flow
                    && let Some(key) = b.palette().get(i).cloned()
                {
                    b.chosen = Some(key.clone());
                    b.help_of(&key);
                    b.flow.carrying = Some(key);
                }
                return Task::none();
            }
            Event::Query(q) => {
                b.query = q;
                return Task::none();
            }
            Event::QuerySubmit => {
                let shown = b.shown();
                let chosen = b
                    .chosen
                    .as_ref()
                    .and_then(|k| shown.iter().find(|i| &i.key == k))
                    .or_else(|| shown.first())
                    .cloned();
                if let Some(item) = chosen {
                    if b.view_mode == ViewMode::Flow {
                        b.flow_add(&item.key);
                    } else {
                        b.place(item.kind, &item.insert, item.caret);
                        focus = true;
                    }
                }
            }
            Event::TreeStep(by) => {
                let shown = b.shown();
                if shown.is_empty() {
                    return Task::none();
                }
                let at = b
                    .chosen
                    .as_ref()
                    .and_then(|k| shown.iter().position(|i| &i.key == k));
                let next = match at {
                    Some(i) => (i as i64 + i64::from(by)).clamp(0, shown.len() as i64 - 1) as usize,
                    None if by > 0 => 0,
                    None => shown.len() - 1,
                };
                let key = shown[next].key.clone();
                b.chosen = Some(key.clone());
                b.help_of(&key);
                return Task::none();
            }
            Event::Toggle(group) => {
                if !b.query.trim().is_empty() {
                    return Task::none();
                }
                if !b.open.remove(&group) {
                    b.open.insert(group);
                }
                return Task::none();
            }
            Event::Row(key) => {
                let now = Instant::now();
                let twice = b
                    .last_press
                    .as_ref()
                    .is_some_and(|(k, at)| *k == key && now.duration_since(*at) < DOUBLE);
                b.last_press = Some((key.clone(), now));
                b.chosen = Some(key.clone());
                b.help_of(&key);
                if !twice {
                    return Task::none();
                }
                b.last_press = None;
                if b.view_mode == ViewMode::Flow {
                    b.flow_add(&key);
                } else if let Some(item) = b.entry(&key) {
                    b.place(item.kind, &item.insert, item.caret);
                    focus = true;
                }
            }
            Event::Values(all) => {
                let (Some(doc), Some(help)) = (&self.document, &b.help) else {
                    return Task::none();
                };
                let field = help.title.clone();
                let ty = help.field.map_or(FieldType::Text, |(ty, _)| ty);
                let raw = b.objects.values(&field, (!all).then_some(10), &doc.model);
                let mut items = core::values(&raw, ty);
                let total = items.len();
                items.truncate(SHOWN);
                b.listed = Some(Listed {
                    field,
                    items,
                    total,
                    chosen: None,
                });
                return Task::none();
            }
            Event::Value(i) => {
                let now = Instant::now();
                let twice = b
                    .last_value
                    .is_some_and(|(k, at)| k == i && now.duration_since(at) < DOUBLE);
                b.last_value = Some((i, now));
                let Some(listed) = b.listed.as_mut() else {
                    return Task::none();
                };
                listed.chosen = Some(i);
                if !twice {
                    return Task::none();
                }
                b.last_value = None;
                if let Some(v) = listed.items.get(i) {
                    let insert = v.insert.clone();
                    if b.view_mode == ViewMode::Flow {
                        b.flow_add_text(&insert);
                    } else {
                        let n = insert.encode_utf16().count();
                        b.place(Kind::Field, &insert, n);
                        focus = true;
                    }
                }
            }
            Event::Object(by) => {
                let n = b.objects.len();
                if n > 0 {
                    b.index = (b.index as i64 + i64::from(by)).rem_euclid(n as i64) as usize;
                }
            }
            Event::OpenProcessing(_) => {}
            Event::Pick => return self.builder_pick(),
            Event::Ok => return self.builder_ok(),
            Event::Cancel => {
                self.builder = None;
                return Task::none();
            }
        }
        let doc = self.document.as_ref().map(|d| &d.model);
        let store = self.spatial.store();
        let mut tasks = Vec::new();
        if let Some(b) = self.builder.as_mut() {
            b.preview_on(doc, store);
            tasks.extend(b.follow());
        }
        if focus
            && self
                .builder
                .as_ref()
                .is_some_and(|b| b.view_mode == ViewMode::Text)
        {
            tasks.push(iced::widget::operation::focus(EDITOR));
        }
        Task::batch(tasks)
    }

    /// Tamam: the text goes back to its field; with an error it stays.
    fn builder_ok(&mut self) -> Task<Message> {
        let Some(b) = &self.builder else {
            return Task::none();
        };
        if b.check.error.is_some() && !b.source.trim().is_empty() {
            return Task::none();
        }
        let text = b.source.clone();
        let target = b.target.clone();
        self.builder = None;
        match target {
            Target::Processing(name) => {
                if let Some(window) = &mut self.processing.dialog {
                    window.values.insert(name.clone(), json!(text));
                    window.touch(&name);
                }
                self.refresh_processing();
            }
            Target::FeatureFilter => {
                return self.features_event(crate::features::Event::Filter(text));
            }
        }
        Task::none()
    }

    /// Sahneden seç: the builder and the window under it step aside, one
    /// object is picked on the drawing, and the preview goes to it.
    fn builder_pick(&mut self) -> Task<Message> {
        let Some(b) = self.builder.as_mut() else {
            return Task::none();
        };
        if self.document.is_none() || b.objects.is_empty() {
            return Task::none();
        }
        b.picking = Some(Picking {
            dialog: self.dialog.take(),
            before: self.selection.ids().to_vec(),
        });
        self.selection.clear();
        self.field = None;
        self.snap = None;
        self.say(Level::Command, "İfade oluşturucu: önizleme nesnesi");
        self.session.run(Box::new(
            kentos_interaction::pick_objects::PickObjects::one("Önizleme nesnesi", None),
        ));
        self.with_tool(|s, cx| s.activate(cx));
        Task::none()
    }

    /// The object picked for the preview (or none, Esc): the builder and its
    /// window come back. Returns false when the builder was not picking.
    pub(crate) fn builder_picked(&mut self, keep: bool) -> bool {
        let Some(b) = self.builder.as_mut() else {
            return false;
        };
        let Some(picking) = b.picking.take() else {
            return false;
        };
        let picked = keep
            .then(|| self.selection.ids().first().copied())
            .flatten();
        match picked.map(|s| (s, b.objects.position(s))) {
            Some((_, Some(i))) => b.index = i,
            Some((_, None)) => self
                .warn("Bu nesne ifadenin çalışacağı nesneler arasında değil; önizleme değişmedi."),
            None => {}
        }
        self.selection.set(picking.before);
        self.dialog = picking.dialog;
        let doc = self.document.as_ref().map(|d| &d.model);
        let store = self.spatial.store();
        if let Some(b) = self.builder.as_mut() {
            b.preview_on(doc, store);
        }
        true
    }

    /// Keys that reach the app while the builder is open (the editor takes
    /// its own): Esc is Vazgeç, Ctrl+Enter Tamam.
    pub(crate) fn builder_key(&mut self, press: &crate::keys::KeyPress) -> Option<Task<Message>> {
        let b = self.builder.as_ref()?;
        if b.is_picking() {
            return None;
        }
        use iced::keyboard::key::Named;
        // Akış: Delete removes the selected node, Ctrl+Z and Ctrl+Y undo and redo, Esc lets the node go.
        if b.view_mode == ViewMode::Flow {
            let selected = b.flow.selected.clone();
            let ctrl = press.modifiers.control();
            let letter = match &press.key {
                iced::keyboard::Key::Character(c) => Some(c.to_lowercase()),
                _ => None,
            };
            let flow = match (press.named(), letter.as_deref()) {
                (Some(Named::Delete | Named::Backspace), _) => {
                    selected.filter(|s| s != "r").map(FlowEvent::Remove)
                }
                (Some(Named::Escape), _) if selected.is_some() => Some(FlowEvent::Select(None)),
                (_, Some("z")) if ctrl && press.modifiers.shift() => Some(FlowEvent::Redo),
                (_, Some("z")) if ctrl => Some(FlowEvent::Undo),
                (_, Some("y")) if ctrl => Some(FlowEvent::Redo),
                _ => None,
            };
            if let Some(e) = flow {
                return Some(self.flow_event(e));
            }
        }
        Some(match press.named() {
            Some(Named::Escape) => self.builder_event(Event::Cancel),
            Some(Named::Enter) if press.modifiers.control() => self.builder_event(Event::Ok),
            // Outside the editor (the search, or nothing with the keyboard) the keys choose in the tree.
            Some(Named::ArrowDown) => self.builder_event(Event::TreeStep(1)),
            Some(Named::ArrowUp) => self.builder_event(Event::TreeStep(-1)),
            Some(Named::Enter) => self.builder_event(Event::QuerySubmit),
            _ => Task::none(),
        })
    }
}

/// The message of an editor action.
pub(crate) fn edit(action: Action) -> Message {
    ev(Event::Edit(action))
}
