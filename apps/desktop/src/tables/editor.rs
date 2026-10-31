//! Tabloyu düzenle (docs/adr/0184 §5; the web's `ui/table/TableEditor.ts`):
//! the table's cells in a grid as a spreadsheet's (column letters, row
//! numbers, the heading row bold, merged ranges as one cell), the active
//! cell's words in the bar above. Rows and columns are added and deleted,
//! cells merged and unmerged (the core's `ops::table_edit`, refusals said in
//! the status line); a column's alignment, the heading row, Çizgiler and
//! Kalın çerçeve, a column's width and a row's height, Yazıya sığdır;
//! Kaynaktan ayır. A column widens as its words ask. Cells' styles (Kalın,
//! Yazı rengi, Dolgu, Basamak, Biçimi temizle), `=` formulas worked out as
//! they are typed, fields (Blok sayısı) (docs/adr/0327). Every change is one
//! of `cad.tables.edit`'s edits: the draft is the command's plan of them, Ctrl+Z
//! and Ctrl+Y take them back and again inside the window, Kaydet runs them in
//! one step (“Tabloyu düzenle”).

use iced::keyboard::key::Named;
use iced::widget::{Id, Space, button, column, container, mouse_area, pin, row, scrollable, stack};
use iced::{Center, Element, Fill, Length, Task, Theme};
use kentos_commands::{
    CommandResult, Entity, FieldSource, ProjectValue, TableAlign, TableEdit, TableEntity,
    TableGrid, TablesEdit, TablesOperation,
};
use kentos_domain::{Slot, Uuid};
use kentos_geometry_core::entity::CellRange;
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, tables_edit};
use kentos_ui::icon::icon;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Dialog as Frame, Tip, tip};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words;
use crate::keys::KeyPress;
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const EDITOR_TITLE: &str = "Tabloyu düzenle";
const SAVE: &str = "Kaydet";
const CANCEL: &str = "Vazgeç";
const DETACH: &str = "Kaynaktan ayır";
/// The cell bar's id: what typing goes to.
pub(crate) const BAR: &str = "table-bar";
/// The grid's rows, logical pixels at the interface's size.
const ROW_PX: f32 = 26.0;
/// The grid's height, logical pixels.
const GRID_PX: f32 = 330.0;

/// The grid's height in a window `window_h` tall: at most `GRID_PX`, less in a low window so the dialog's buttons
/// stay in it (the rest of the dialog is about 420 px), never under five rows.
fn grid_px(window_h: f32) -> f32 {
    let most = typography::scaled(GRID_PX);
    (window_h - typography::scaled(420.0)).clamp(typography::scaled(ROW_PX) * 5.0, most)
}
/// The row numbers' column.
const GUTTER_PX: f32 = 44.0;
/// The rows drawn past the window either way.
const OVERSCAN: usize = 6;

/// A column's name as a spreadsheet's: A … Z, AA, AB …
pub fn column_name(j: usize) -> String {
    let mut s = Vec::new();
    let mut n = j + 1;
    while n > 0 {
        s.push(b'A' + ((n - 1) % 26) as u8);
        n = (n - 1) / 26;
    }
    s.reverse();
    String::from_utf8(s).unwrap_or_default()
}

/// The window while it is open.
#[derive(Clone, Debug)]
pub struct Editor {
    uid: Uuid,
    /// The plot scale: Kalın çerçeve's width is typed on paper (mm).
    scale: f64,
    /// Bumped by every change of the draft: the bar and the sizes follow it.
    version: u64,
    pub(crate) draft: TableEntity,
    saved: TableEntity,
    /// The edits made, in order: the draft is `cad.tables.edit`'s plan of them.
    edits: Vec<TableEdit>,
    /// How many edits there were before each step (Ctrl+Z goes back to it).
    steps: Vec<usize>,
    /// The steps taken back (Ctrl+Y makes them again).
    redo: Vec<Vec<TableEdit>>,
    /// The drawing's blocks' names, for Blok sayısı.
    blocks: Vec<String>,
    active: (usize, usize),
    anchor: (usize, usize),
    /// The bar's words, as typed.
    words: String,
    problem: Option<String>,
    /// The grid's vertical scroll, logical pixels.
    scroll: f32,
    dragging: bool,
    /// The sizes row's fields, as typed.
    width: String,
    height: String,
    frame: String,
    /// Closing with changes: the question is up.
    asking: bool,
}

/// What the toolbar does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    RowAbove,
    RowBelow,
    ColumnLeft,
    ColumnRight,
    DeleteRows,
    DeleteColumns,
    Merge,
    Unmerge,
    Align(TableAlign),
    Header,
    Grid(Option<TableGrid>),
    Frame,
    Fit,
    Undo,
    Redo,
    /// The selection's cells' styles (docs/adr/0327 §1).
    Bold,
    Color(usize),
    Fill(usize),
    Decimals(u32),
    ClearStyle,
    /// The active cell bound to the count of `blocks[i]`'s inserts, or let go of.
    Field(usize),
    Unfield,
}

/// Yazı rengi's colours: name and `#rrggbb`.
pub const TEXT_COLORS: [(&str, &str); 5] = [
    ("Siyah", "#1b1f24"),
    ("Kırmızı", "#c0392b"),
    ("Mavi", "#1f5fa8"),
    ("Yeşil", "#2e7d32"),
    ("Turuncu", "#c46a00"),
];
/// Dolgu's colours, light enough to read through.
pub const FILLS: [(&str, &str); 5] = [
    ("Gri", "#eceff3"),
    ("Sarı", "#fff3c4"),
    ("Mavi", "#e2edfb"),
    ("Yeşil", "#e3f3e6"),
    ("Pembe", "#fbe4e6"),
];

#[derive(Debug, Clone)]
pub enum Event {
    /// A press on a cell (its top left when merged).
    Cell(usize, usize),
    Over(usize, usize),
    Release,
    /// A double click on a cell: its words in the bar.
    Write,
    Words(String),
    /// Enter in the bar: the words written, the cell below.
    Commit,
    Tool(Tool),
    Width(String),
    Height(String),
    FrameWidth(String),
    CommitSizes,
    Detach,
    Scrolled(f32),
    /// A key the grid takes, and whether the bar had the keyboard.
    Key(Named, bool, bool),
    /// A letter typed while the grid has the keyboard: the bar starts with it.
    Typed(String, bool),
    Save,
    Close,
    /// The question's answers: save, leave without saving, stay.
    Answer(Answer),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Answer {
    Save,
    Discard,
    Stay,
}

fn msg(event: Event) -> Message {
    Message::TableEditor(event)
}

/// The merged range holding a cell, if any.
fn range_at(t: &TableEntity, r: usize, c: usize) -> Option<CellRange> {
    t.merges.iter().find(|m| m.holds(r, c)).map(|m| CellRange {
        row: m.row as usize,
        col: m.col as usize,
        rows: m.rows as usize,
        cols: m.cols as usize,
    })
}

impl Editor {
    fn n(&self) -> usize {
        self.draft.rows.len()
    }

    fn m(&self) -> usize {
        self.draft.columns.len()
    }

    /// The cell holding a cell's words: a merged range's top left.
    fn owner(&self, (r, c): (usize, usize)) -> (usize, usize) {
        range_at(&self.draft, r, c).map_or((r, c), |g| (g.row, g.col))
    }

    /// What the bar shows for a cell: its formula, else its words.
    fn words_at(&self, (r, c): (usize, usize)) -> String {
        if let Some(f) = self.draft.formula_at(r, c) {
            return f.formula.clone();
        }
        self.draft
            .cells
            .get(r)
            .and_then(|row| row.get(c))
            .cloned()
            .unwrap_or_default()
    }

    /// The selected range: from the anchor to the active cell, grown to the merged ranges it touches.
    fn selected(&self) -> CellRange {
        let (mut r0, mut c0) = (
            self.anchor.0.min(self.active.0),
            self.anchor.1.min(self.active.1),
        );
        let (mut r1, mut c1) = (
            self.anchor.0.max(self.active.0),
            self.anchor.1.max(self.active.1),
        );
        loop {
            let mut grown = false;
            for g in &self.draft.merges {
                let (gr, gc) = (g.row as usize, g.col as usize);
                let (gr1, gc1) = (gr + g.rows as usize - 1, gc + g.cols as usize - 1);
                if gr > r1 || gr1 < r0 || gc > c1 || gc1 < c0 {
                    continue;
                }
                let next = (r0.min(gr), c0.min(gc), r1.max(gr1), c1.max(gc1));
                if next != (r0, c0, r1, c1) {
                    (r0, c0, r1, c1) = next;
                    grown = true;
                }
            }
            if !grown {
                break;
            }
        }
        CellRange {
            row: r0,
            col: c0,
            rows: r1 - r0 + 1,
            cols: c1 - c0 + 1,
        }
    }

    fn clamp(&mut self) {
        let (n, m) = (self.n().max(1), self.m().max(1));
        self.active = self.owner((self.active.0.min(n - 1), self.active.1.min(m - 1)));
        self.anchor = (self.anchor.0.min(n - 1), self.anchor.1.min(m - 1));
    }

    /// The sizes row and the bar from the selection.
    fn fields(&mut self, f: &kentos_interaction::Format) {
        let r = self.selected();
        self.width =
            kentos_geometry_core::display::fixed(f.from_metres(self.draft.columns[r.col]), 2);
        self.height =
            kentos_geometry_core::display::fixed(f.from_metres(self.draft.rows[r.row]), 2);
        self.frame = self.draft.frame.map_or_else(String::new, |w| {
            kentos_geometry_core::display::fixed(w / self.scale * 1000.0, 2)
        });
        self.words = self.words_at(self.owner(self.active));
    }

    /// Moves the active cell by rows and columns, over merged ranges; with `extend` the anchor stays.
    fn step(&mut self, dr: isize, dc: isize, extend: bool) {
        let g = range_at(&self.draft, self.active.0, self.active.1);
        let mut row = self.active.0 as isize + dr;
        let mut col = self.active.1 as isize + dc;
        if let Some(g) = g {
            if dr > 0 {
                row = (g.row + g.rows) as isize;
            }
            if dc > 0 {
                col = (g.col + g.cols) as isize;
            }
        }
        let row = row.clamp(0, self.n() as isize - 1) as usize;
        let col = col.clamp(0, self.m() as isize - 1) as usize;
        self.active = if extend {
            (row, col)
        } else {
            self.owner((row, col))
        };
        if !extend {
            self.anchor = self.active;
        }
        self.problem = None;
    }

    /// The scroll that shows the active cell.
    fn reveal(&mut self, grid: f32) -> Option<f32> {
        let top = self.active.0 as f32 * typography::scaled(ROW_PX);
        let row = typography::scaled(ROW_PX);
        let view = grid - row;
        if top < self.scroll {
            Some(top)
        } else if top + row > self.scroll + view {
            Some(top + row - view)
        } else {
            None
        }
    }
}

impl App {
    /// Tabloyu düzenle for the table at `slot` (the selection's, or a double
    /// click's); the keyboard goes to the grid (from the command line too).
    pub(crate) fn open_table_editor(&mut self, slot: Slot) -> Task<Message> {
        let Some(doc) = &self.document else {
            return Task::none();
        };
        let Some(Entity::Table(t)) = doc.model.get(slot) else {
            return Task::none();
        };
        if doc.model.layers().is_locked(&t.base.layer_id) {
            self.warn("Kilitli katmandaki tablo düzenlenemez.");
            return Task::none();
        }
        let Some(uid) = doc.model.uid(slot) else {
            return Task::none();
        };
        let mut e = Editor {
            uid,
            scale: doc.model.settings().plot_scale,
            version: 0,
            draft: t.clone(),
            saved: t.clone(),
            edits: Vec::new(),
            steps: Vec::new(),
            redo: Vec::new(),
            blocks: doc.model.blocks().iter().map(|b| b.name.clone()).collect(),
            active: (0, 0),
            anchor: (0, 0),
            words: String::new(),
            problem: None,
            scroll: 0.0,
            dragging: false,
            width: String::new(),
            height: String::new(),
            frame: String::new(),
            asking: false,
        };
        e.fields(&self.format());
        self.table_editor = Some(e);
        self.dialog = Some(Dialog::TableEditor);
        iced::widget::operation::focus(Id::new("table-grid"))
    }

    /// The draft as `cad.tables.edit` would write `edits`: the saved table when there are none, the refusal's words
    /// when it refuses them.
    fn table_planned(
        &self,
        uid: Uuid,
        edits: &[TableEdit],
        saved: &TableEntity,
    ) -> Result<TableEntity, String> {
        if edits.is_empty() {
            return Ok(saved.clone());
        }
        let Some(doc) = &self.document else {
            return Err("Çizim yok.".to_owned());
        };
        let input = TablesEdit {
            operation: TablesOperation::Edit {
                uid: uid.to_string(),
                edits: edits.to_vec(),
            },
            expected_revision: None,
        };
        match tables_edit::plan_in(&doc.model, &input) {
            CommandResult::Completed { output, .. } => Ok(match output.tables.into_iter().next() {
                Some(Entity::Table(t)) => t,
                _ => saved.clone(),
            }),
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => Err(error.message),
            _ => Err("Yapılamadı.".to_owned()),
        }
    }

    /// One step of edits: the draft their plan, or the refusal said in the window.
    fn table_push(&mut self, group: Vec<TableEdit>) {
        let Some(e) = self.table_editor.as_ref() else {
            return;
        };
        if group.is_empty() {
            return;
        }
        let mut all = e.edits.clone();
        all.extend(group);
        let planned = self.table_planned(e.uid, &all, &e.saved);
        let Some(e) = self.table_editor.as_mut() else {
            return;
        };
        match planned {
            Ok(t) => {
                e.steps.push(e.edits.len());
                e.edits = all;
                e.redo.clear();
                e.draft = t;
                e.version += 1;
                e.problem = None;
                e.clamp();
            }
            Err(why) => e.problem = Some(why),
        }
    }

    /// Ctrl+Z (`back`) or Ctrl+Y: a step taken back or made again.
    fn table_step(&mut self, back: bool) {
        let Some(e) = self.table_editor.as_mut() else {
            return;
        };
        if back {
            let Some(n) = e.steps.pop() else {
                return;
            };
            let tail = e.edits.split_off(n);
            e.redo.push(tail);
        } else {
            let Some(group) = e.redo.pop() else {
                return;
            };
            e.steps.push(e.edits.len());
            e.edits.extend(group);
        }
        let (uid, edits, saved) = (e.uid, e.edits.clone(), e.saved.clone());
        let planned = self.table_planned(uid, &edits, &saved);
        if let Some(e) = self.table_editor.as_mut() {
            if let Ok(t) = planned {
                e.draft = t;
            }
            e.version += 1;
            e.problem = None;
            e.clamp();
        }
    }

    /// The grid's keys while the window is open: ↑ ↓ ← → (Shift: a range),
    /// Tab, Enter and F2 (the bar), Delete; Ctrl+Z and Ctrl+Y; a letter
    /// starts the bar with it. Esc: the bar's words back, else the window
    /// asks or closes. None for a key the window leaves.
    pub(crate) fn table_editor_key(&mut self, press: &KeyPress) -> Option<Task<Message>> {
        let e = self.table_editor.as_ref()?;
        if e.asking {
            return None;
        }
        let ctrl = press.modifiers.control() || press.modifiers.command();
        match crate::keys::chord(press).as_deref() {
            Some("Ctrl+Z") => return Some(self.table_editor_event(Event::Tool(Tool::Undo))),
            Some("Ctrl+Y" | "Ctrl+Shift+Z") => {
                return Some(self.table_editor_event(Event::Tool(Tool::Redo)));
            }
            _ => {}
        }
        let shift = press.modifiers.shift();
        if let Some(
            key @ (Named::ArrowUp
            | Named::ArrowDown
            | Named::ArrowLeft
            | Named::ArrowRight
            | Named::Tab
            | Named::Enter
            | Named::F2
            | Named::Delete
            | Named::Backspace
            | Named::Escape),
        ) = press.named()
        {
            return Some(
                iced::widget::operation::is_focused(Id::new(BAR))
                    .map(move |bar| msg(Event::Key(key, shift, bar))),
            );
        }
        if !ctrl
            && !press.modifiers.alt()
            && let Some(ch) = press.character()
        {
            let typed = ch.to_string();
            return Some(
                iced::widget::operation::is_focused(Id::new(BAR))
                    .map(move |bar| msg(Event::Typed(typed.clone(), bar))),
            );
        }
        None
    }

    pub(crate) fn table_editor_event(&mut self, event: Event) -> Task<Message> {
        let format = self.format();
        let Some(e) = self.table_editor.as_mut() else {
            return Task::none();
        };
        let mut task = Task::none();
        let before = (e.active, e.anchor, e.version);
        // The step this event makes, and a step taken back or made again.
        let mut group: Vec<TableEdit> = Vec::new();
        let mut step: Option<bool> = None;
        match event {
            Event::Cell(r, c) => {
                group.extend(commit_edits(e));
                e.active = (r, c);
                if !self.modifiers.shift() {
                    e.anchor = (r, c);
                }
                e.dragging = true;
                e.problem = None;
                // The keyboard goes to the grid: the bar is left.
                task = iced::widget::operation::focus(Id::new("table-grid"));
            }
            Event::Over(r, c) => {
                if e.dragging && e.active != (r, c) {
                    e.active = (r, c);
                }
            }
            Event::Release => e.dragging = false,
            Event::Write => {
                e.words = e.words_at(e.owner(e.active));
                task = Task::batch([
                    iced::widget::operation::focus(Id::new(BAR)),
                    iced::widget::operation::move_cursor_to_end(Id::new(BAR)),
                ]);
            }
            Event::Words(t) => e.words = t,
            Event::Commit => {
                // A schedule's cell that is not its objects' own field is read only (docs/adr/0733 §6).
                if let Some(Err(why)) = self
                    .document
                    .as_ref()
                    .and_then(|d| schedule_cell(&d.model, &e.saved, e.active))
                {
                    e.problem = Some(why);
                    e.words = e.words_at(e.active);
                    return Task::none();
                }
                group.extend(commit_edits(e));
                e.step(1, 0, false);
                task = iced::widget::operation::focus(Id::new("table-grid"));
            }
            Event::Tool(Tool::Undo) => step = Some(true),
            Event::Tool(Tool::Redo) => step = Some(false),
            Event::Tool(t) => group.extend(tool_edits(e, t)),
            Event::Width(t) => e.width = t,
            Event::Height(t) => e.height = t,
            Event::FrameWidth(t) => e.frame = t,
            Event::CommitSizes => group.extend(sizes_edits(e, &format)),
            Event::Detach => group.push(TableEdit::Unlink),
            Event::Scrolled(y) => e.scroll = y,
            Event::Key(key, shift, bar) => {
                if bar {
                    match key {
                        Named::Escape => {
                            e.words = e.words_at(e.owner(e.active));
                            task = iced::widget::operation::focus(Id::new("table-grid"));
                        }
                        Named::Tab => {
                            group.extend(commit_edits(e));
                            e.step(0, if shift { -1 } else { 1 }, false);
                            task = iced::widget::operation::focus(Id::new("table-grid"));
                        }
                        _ => {}
                    }
                } else {
                    match key {
                        Named::ArrowUp => e.step(-1, 0, shift),
                        Named::ArrowDown => e.step(1, 0, shift),
                        Named::ArrowLeft => e.step(0, -1, shift),
                        Named::ArrowRight => e.step(0, 1, shift),
                        Named::Tab => e.step(0, if shift { -1 } else { 1 }, false),
                        // Words given the bar without its keyboard (a trace's fill) are written as its Enter would.
                        Named::Enter if e.words != e.words_at(e.owner(e.active)) => {
                            group.extend(commit_edits(e));
                            e.step(1, 0, false);
                        }
                        Named::Enter | Named::F2 => {
                            e.words = e.words_at(e.owner(e.active));
                            task = Task::batch([
                                iced::widget::operation::focus(Id::new(BAR)),
                                iced::widget::operation::move_cursor_to_end(Id::new(BAR)),
                            ]);
                        }
                        Named::Delete | Named::Backspace => group.extend(clear_edits(e)),
                        Named::Escape => return self.table_editor_event(Event::Close),
                        _ => {}
                    }
                }
            }
            Event::Typed(t, bar) => {
                if !bar {
                    e.words = t;
                    task = Task::batch([
                        iced::widget::operation::focus(Id::new(BAR)),
                        iced::widget::operation::move_cursor_to_end(Id::new(BAR)),
                    ]);
                }
            }
            Event::Save => {
                self.save_table_editor();
                return Task::none();
            }
            Event::Close => {
                if !e.edits.is_empty() && e.draft != e.saved {
                    e.asking = true;
                } else {
                    self.table_editor = None;
                    self.dialog = None;
                }
                return Task::none();
            }
            Event::Answer(Answer::Save) => {
                self.save_table_editor();
                return Task::none();
            }
            Event::Answer(Answer::Discard) => {
                self.table_editor = None;
                self.dialog = None;
                return Task::none();
            }
            Event::Answer(Answer::Stay) => {
                e.asking = false;
                return Task::none();
            }
        }
        if let Some(back) = step {
            self.table_step(back);
        }
        self.table_push(group);
        let grid = grid_px(self.window_size.height);
        let Some(e) = self.table_editor.as_mut() else {
            return task;
        };
        // The bar and the sizes follow the selection and the draft; what is being typed stays.
        if (e.active, e.anchor, e.version) == before {
            return task;
        }
        e.fields(&format);
        if let Some(y) = e.reveal(grid) {
            e.scroll = y;
            task = Task::batch([
                task,
                iced::widget::operation::scroll_to(
                    Id::new("table-rows"),
                    scrollable::AbsoluteOffset {
                        x: None,
                        y: Some(y),
                    },
                ),
            ]);
        }
        task
    }

    /// Kaydet: the edits run in one step “Tabloyu düzenle” (`cad.tables.edit`); a schedule's cells are written to its
    /// objects instead (`cad.schedules.edit`, docs/adr/0344 §3), its sizes as a table's, in one step.
    fn save_table_editor(&mut self) {
        let Some(e) = self.table_editor.as_ref() else {
            return;
        };
        if e.edits.is_empty() || e.draft == e.saved {
            self.table_editor = None;
            self.dialog = None;
            return;
        }
        if matches!(
            e.saved.source,
            Some(kentos_commands::TableSource::Schedule { .. })
        ) {
            self.save_schedule_editor();
            return;
        }
        let input = TablesEdit {
            operation: TablesOperation::Edit {
                uid: e.uid.to_string(),
                edits: e.edits.clone(),
            },
            expected_revision: None,
        };
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        match tables_edit::execute(&mut ExecutionContext::new(&mut doc.model), input) {
            CommandResult::Completed { warnings, .. } => {
                for w in warnings {
                    self.warn(w.message);
                }
                self.announce(Level::Success, "Tablo kaydedildi. Ctrl+Z geri alır.");
                self.table_editor = None;
                self.dialog = None;
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => {
                if let Some(e) = self.table_editor.as_mut() {
                    e.problem = Some(error.message);
                    e.asking = false;
                }
            }
            _ => {}
        }
    }

    /// Kaydet on a schedule: each cell written to its row's objects, the sizes as a table's; one step, all or none.
    fn save_schedule_editor(&mut self) {
        use kentos_commands::{ScheduleOperation, SchedulesEdit};
        let Some(e) = self.table_editor.as_ref() else {
            return;
        };
        let uid = e.uid.to_string();
        let edits = e.edits.clone();
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let group = doc
            .model
            .begin_group(kentos_native_application::schedules::EDIT_STEP);
        let mut problem = None;
        let mut sizes = Vec::new();
        for edit in edits {
            let result = match edit {
                TableEdit::SetCell { row, col, words } => {
                    kentos_native_application::schedules_edit::execute(
                        &mut ExecutionContext::new(&mut doc.model),
                        SchedulesEdit {
                            uid: uid.clone(),
                            operation: ScheduleOperation::Cell {
                                row,
                                column: col,
                                value: words,
                            },
                            expected_revision: None,
                        },
                    )
                }
                size @ (TableEdit::RowHeight { .. }
                | TableEdit::ColumnWidth { .. }
                | TableEdit::Fit { .. }) => {
                    sizes.push(size);
                    continue;
                }
                _ => {
                    problem = Some(
                        "Çizelgenin satırları, sütunları ve biçimi belgenin yazdığıdır; yalnız hücreler, boylar ve genişlikler düzenlenir."
                            .to_owned(),
                    );
                    break;
                }
            };
            if let CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } = result
            {
                problem = Some(error.message);
                break;
            }
        }
        if problem.is_none() && !sizes.is_empty() {
            let input = TablesEdit {
                operation: TablesOperation::Edit { uid, edits: sizes },
                expected_revision: None,
            };
            if let CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } =
                tables_edit::execute(&mut ExecutionContext::new(&mut doc.model), input)
            {
                problem = Some(error.message);
            }
        }
        match problem {
            Some(why) => {
                doc.model.cancel_group(group);
                if let Some(e) = self.table_editor.as_mut() {
                    e.problem = Some(why);
                    e.asking = false;
                }
            }
            None => {
                doc.model.end_group(group);
                self.announce(
                    Level::Success,
                    "Çizelge kaydedildi: hücreler nesnelere yazıldı. Ctrl+Z geri alır.",
                );
                self.table_editor = None;
                self.dialog = None;
            }
        }
    }

    pub(crate) fn table_editor_view(&self) -> Element<'_, Message> {
        let Some(e) = &self.table_editor else {
            return label::body("").into();
        };
        let format = self.format();
        let r = e.selected();
        let n = e.n();
        let m = e.m();
        // The toolbar.
        let tool_button = |name: &'static str,
                           says: String,
                           t: Tool,
                           on: Option<bool>,
                           enabled: bool|
         -> Element<'_, Message> {
            tip(
                button(icon(crate::icons::from_web(Some(name))).size(18.0))
                    .padding(kentos_ui::theme::metrics::icon(18.0))
                    .style(style::button::toggle(on == Some(true)))
                    .on_press_maybe(enabled.then(|| msg(Event::Tool(t)))),
                Tip::new(says),
                iced::widget::tooltip::Position::Top,
            )
        };
        let gap = || -> Element<'_, Message> {
            container(Space::new().width(1).height(20))
                .padding([0, 5])
                .style(|theme: &Theme| container::Style {
                    background: Some(iced::Background::Color(Tokens::of(theme).border)),
                    ..container::Style::default()
                })
                .width(Length::Fixed(1.0))
                .into()
        };
        let merged = range_at(&e.draft, e.active.0, e.active.1);
        let align = e
            .draft
            .aligns
            .as_ref()
            .and_then(|a| a.get(e.active.1).copied())
            .unwrap_or(TableAlign::Left);
        let grid = e.draft.grid;
        let rows_word = if r.rows > 1 {
            format!("{} satırı sil", r.rows)
        } else {
            "Satırı sil".to_owned()
        };
        let cols_word = if r.cols > 1 {
            format!("{} sütunu sil", r.cols)
        } else {
            "Sütunu sil".to_owned()
        };
        let tools = row![
            tool_button(
                "tableRowAbove",
                "Üstüne satır ekle".into(),
                Tool::RowAbove,
                None,
                true
            ),
            tool_button(
                "tableRowBelow",
                "Altına satır ekle".into(),
                Tool::RowBelow,
                None,
                true
            ),
            tool_button(
                "tableColumnLeft",
                "Soluna sütun ekle".into(),
                Tool::ColumnLeft,
                None,
                true
            ),
            tool_button(
                "tableColumnRight",
                "Sağına sütun ekle".into(),
                Tool::ColumnRight,
                None,
                true
            ),
            tool_button(
                "tableRowDelete",
                rows_word,
                Tool::DeleteRows,
                None,
                r.rows < n
            ),
            tool_button(
                "tableColumnDelete",
                cols_word,
                Tool::DeleteColumns,
                None,
                r.cols < m
            ),
            gap(),
            tool_button(
                "tableMerge",
                "Hücreleri birleştir".into(),
                Tool::Merge,
                None,
                r.rows * r.cols >= 2 && merged.is_none_or(|g| g.rows != r.rows || g.cols != r.cols)
            ),
            tool_button(
                "tableUnmerge",
                "Birleşimi ayır".into(),
                Tool::Unmerge,
                None,
                merged.is_some()
            ),
            gap(),
            tool_button(
                "cellLeft",
                "Sütunu sola hizala".into(),
                Tool::Align(TableAlign::Left),
                Some(align == TableAlign::Left),
                true
            ),
            tool_button(
                "cellCenter",
                "Sütunu ortala".into(),
                Tool::Align(TableAlign::Center),
                Some(align == TableAlign::Center),
                true
            ),
            tool_button(
                "cellRight",
                "Sütunu sağa hizala".into(),
                Tool::Align(TableAlign::Right),
                Some(align == TableAlign::Right),
                true
            ),
            gap(),
            tool_button(
                "tableHeader",
                "Başlık satırı: ilk satır kalın ve ortalı".into(),
                Tool::Header,
                Some(e.draft.header),
                true
            ),
            gap(),
            tool_button(
                "tableGridAll",
                "Çizgiler: tümü".into(),
                Tool::Grid(None),
                Some(grid.is_none()),
                true
            ),
            tool_button(
                "tableGridOuter",
                "Çizgiler: yalnız dış çizgi".into(),
                Tool::Grid(Some(TableGrid::Outer)),
                Some(grid == Some(TableGrid::Outer)),
                true
            ),
            tool_button(
                "tableGridRows",
                "Çizgiler: dış çizgi ve satırlar".into(),
                Tool::Grid(Some(TableGrid::Rows)),
                Some(grid == Some(TableGrid::Rows)),
                true
            ),
            tool_button(
                "tableGridNone",
                "Çizgiler: yok".into(),
                Tool::Grid(Some(TableGrid::None)),
                Some(grid == Some(TableGrid::None)),
                true
            ),
            tool_button(
                "tableFrame",
                "Kalın çerçeve".into(),
                Tool::Frame,
                Some(e.draft.frame.is_some()),
                grid != Some(TableGrid::None)
            ),
            gap(),
            tool_button(
                "tableFit",
                "Yazıya sığdır: satır ve sütunlar yazıları kadar".into(),
                Tool::Fit,
                None,
                true
            ),
            gap(),
            tool_button(
                "undo",
                "Geri al (Ctrl+Z)".into(),
                Tool::Undo,
                None,
                !e.steps.is_empty()
            ),
            tool_button(
                "redo",
                "Yinele (Ctrl+Y)".into(),
                Tool::Redo,
                None,
                !e.redo.is_empty()
            ),
        ]
        .spacing(2)
        .align_y(Center);
        // The cells' styles and fields (docs/adr/0327 §1, §3): a row of their own.
        let style = e.draft.cell_style(at_cell(e).0, at_cell(e).1);
        let bold_on = style.bold.unwrap_or(e.draft.header && at_cell(e).0 == 0);
        let swatches = |list: &[(&str, &str)]| -> Vec<Choice> {
            list.iter()
                .map(|(name, hex)| Choice::new(*name).color(hex_color(hex)))
                .collect()
        };
        let picked = |list: &[(&str, &str)], now: &Option<String>| {
            now.as_deref()
                .and_then(|c| list.iter().position(|(_, h)| *h == c))
        };
        let decimals: Vec<Choice> = (0..=4)
            .map(|d| Choice::new(format!("{d} basamak")))
            .collect();
        let blocks: Vec<Choice> = e.blocks.iter().map(Choice::new).collect();
        let field_now =
            e.draft
                .field_at(at_cell(e).0, at_cell(e).1)
                .and_then(|f| match &f.source {
                    FieldSource::Project {
                        value: ProjectValue::BlockCount,
                        name: Some(n),
                    } => e.blocks.iter().position(|b| b == n),
                    _ => None,
                });
        let narrow = |w: f32, el: Element<'static, Message>| -> Element<'static, Message> {
            container(el)
                .width(Length::Fixed(typography::scaled(w)))
                .into()
        };
        let format_tools = row![
            tool_button("tableBold", "Kalın: seçili hücreler".into(), Tool::Bold, Some(bold_on), true),
            tip(
                narrow(
                    118.0,
                    Select::new(swatches(&TEXT_COLORS), picked(&TEXT_COLORS, &style.color), |i| {
                        msg(Event::Tool(Tool::Color(i)))
                    })
                    .placeholder("Yazı rengi")
                    .into()
                ),
                Tip::new("Yazı rengi: seçili hücreler"),
                iced::widget::tooltip::Position::Top,
            ),
            tip(
                narrow(
                    104.0,
                    Select::new(swatches(&FILLS), picked(&FILLS, &style.fill), |i| {
                        msg(Event::Tool(Tool::Fill(i)))
                    })
                    .placeholder("Dolgu")
                    .into()
                ),
                Tip::new("Dolgu: seçili hücrelerin zemini"),
                iced::widget::tooltip::Position::Top,
            ),
            tip(
                narrow(
                    112.0,
                    Select::new(decimals, style.decimals.map(|d| d as usize), |i| {
                        msg(Event::Tool(Tool::Decimals(i as u32)))
                    })
                    .placeholder("Basamak")
                    .into()
                ),
                Tip::new("Basamak: formül ve alan değerlerinin ondalığı"),
                iced::widget::tooltip::Position::Top,
            ),
            tool_button("tableClearStyle", "Biçimi temizle: seçili hücrelere değen biçimler".into(), Tool::ClearStyle, None, !e.draft.styles.is_empty()),
            gap(),
            tip(
                narrow(
                    150.0,
                    Select::new(blocks, field_now, |i| msg(Event::Tool(Tool::Field(i))))
                        .placeholder("Blok sayısı")
                        .into()
                ),
                Tip::new("Alan: etkin hücre seçilen bloğun yerleştirme sayısını yazar, çizim değişince güncellenir"),
                iced::widget::tooltip::Position::Top,
            ),
            tool_button("tableUnfield", "Alanı kaldır: yazısı kalır".into(), Tool::Unfield, None, field_now.is_some() || e.draft.field_at(at_cell(e).0, at_cell(e).1).is_some()),
            gap(),
            label::caption(cell_note(e))
                .wrapping(iced::widget::text::Wrapping::None),
        ]
        .spacing(4)
        .align_y(Center);
        let toolbar = container(column![tools, format_tools].spacing(4))
            .padding(4)
            .width(Fill)
            .style(style::container::header);
        // The bar: the active cell's address and words.
        let at = e.owner(e.active);
        let address = match range_at(&e.draft, at.0, at.1) {
            Some(g) => format!(
                "{}{}:{}{}",
                column_name(g.col),
                g.row + 1,
                column_name(g.col + g.cols - 1),
                g.row + g.rows
            ),
            None => format!("{}{}", column_name(at.1), at.0 + 1),
        };
        let bar = row![
            container(label::body(address).font(typography::mono()))
                .padding([5, 8])
                .width(Length::Fixed(typography::scaled(72.0)))
                .center_x(Length::Fixed(typography::scaled(72.0)))
                .style(style::container::header),
            kentos_ui::widget::text_field("", &e.words)
                .id(Id::new(BAR))
                .on_input(|t| msg(Event::Words(t)))
                .on_submit(msg(Event::Commit))
                .width(Fill),
        ]
        .spacing(8)
        .align_y(Center);
        // The grid: its columns as wide as their words, its rows in view.
        let row_px = typography::scaled(ROW_PX);
        let gutter = typography::scaled(GUTTER_PX);
        let widths: Vec<f32> = (0..m)
            .map(|j| {
                let letters = e
                    .draft
                    .cells
                    .iter()
                    .take(2000)
                    .filter_map(|row| row.get(j))
                    .map(|w| w.chars().count())
                    .max()
                    .unwrap_or(0);
                typography::scaled((letters as f32 * 7.4 + 22.0).clamp(72.0, 260.0))
            })
            .collect();
        let lefts: Vec<f32> = widths
            .iter()
            .scan(gutter, |x, w| {
                let at = *x;
                *x += w;
                Some(at)
            })
            .collect();
        let total_w = gutter + widths.iter().sum::<f32>();
        let grid = grid_px(self.window_size.height);
        let view_h = grid - row_px;
        let first = ((e.scroll / row_px) as usize).saturating_sub(OVERSCAN);
        let last = (((e.scroll + view_h) / row_px).ceil() as usize + OVERSCAN).min(n);
        let in_range = |i: usize, j: usize| {
            i >= r.row && i < r.row + r.rows && j >= r.col && j < r.col + r.cols
        };
        let mut cells: Vec<Element<'_, Message>> = vec![
            Space::new()
                .width(Length::Fixed(total_w))
                .height(Length::Fixed(n as f32 * row_px))
                .into(),
        ];
        for i in first..last {
            let on_row = i >= r.row && i < r.row + r.rows;
            cells.push(
                pin(container(label::caption((i + 1).to_string()))
                    .padding([0, 8])
                    .width(Length::Fixed(gutter))
                    .height(Length::Fixed(row_px))
                    .align_right(Length::Fixed(gutter))
                    .align_y(Center)
                    .style(move |theme: &Theme| gutter_style(theme, on_row)))
                .x(0.0)
                .y(i as f32 * row_px)
                .into(),
            );
            for j in 0..m {
                let g = range_at(&e.draft, i, j);
                let (top, span_rows, span_cols) = match g {
                    Some(g) => {
                        let top = g.row.max(first);
                        if i != top || j != g.col {
                            continue;
                        }
                        ((g.row, g.col), (g.row + g.rows).min(last) - top, g.cols)
                    }
                    None => ((i, j), 1, 1),
                };
                // The cell as the table shows it: a formula's worked out value, its style's look (docs/adr/0327 §1).
                let words = e
                    .draft
                    .cells
                    .get(top.0)
                    .and_then(|row| row.get(top.1))
                    .cloned()
                    .unwrap_or_default();
                let heading = e.draft.header && top.0 == 0;
                let look = e.draft.cell_style(top.0, top.1);
                let alignment = look.align.unwrap_or(if heading {
                    TableAlign::Center
                } else {
                    e.draft
                        .aligns
                        .as_ref()
                        .and_then(|a| a.get(top.1).copied())
                        .unwrap_or(TableAlign::Left)
                });
                let fill = look.fill.as_deref().map(hex_color);
                let ink = look.color.as_deref().map(hex_color);
                let w: f32 = widths[j..(j + span_cols).min(m)].iter().sum();
                let h = span_rows as f32 * row_px;
                let active = top == e.active;
                let chosen = in_range(i, j);
                let mut text = label::body(words).wrapping(iced::widget::text::Wrapping::None);
                if look.bold.unwrap_or(heading) {
                    text = text.font(typography::ui_strong());
                }
                let place = match alignment {
                    TableAlign::Left => iced::alignment::Horizontal::Left,
                    TableAlign::Center => iced::alignment::Horizontal::Center,
                    TableAlign::Right => iced::alignment::Horizontal::Right,
                };
                let face = container(text)
                    .padding([0, 9])
                    .width(Length::Fixed(w))
                    .height(Length::Fixed(h))
                    .align_x(place)
                    .align_y(Center)
                    .clip(true)
                    .style(move |theme: &Theme| cell_style(theme, chosen, active, fill, ink));
                cells.push(
                    pin(mouse_area(face)
                        .on_press(msg(Event::Cell(top.0, top.1)))
                        .on_enter(msg(Event::Over(top.0, top.1)))
                        .on_release(msg(Event::Release))
                        .on_double_click(msg(Event::Write)))
                    .x(lefts[j])
                    .y(i as f32 * row_px)
                    .into(),
                );
            }
        }
        let body = scrollable(stack(cells))
            .id(Id::new("table-rows"))
            .direction(scrollable::Direction::Vertical(scrollable::Scrollbar::new()))
            .height(Length::Fixed(view_h))
            .on_scroll(|v| msg(Event::Scrolled(v.absolute_offset().y)));
        let mut heads = row![
            container(Space::new())
                .width(Length::Fixed(gutter))
                .height(Length::Fixed(row_px))
                .style(|theme: &Theme| gutter_style(theme, false))
        ];
        for (j, w) in widths.iter().enumerate() {
            let on = j >= r.col && j < r.col + r.cols;
            heads = heads.push(
                container(label::caption(column_name(j)))
                    .width(Length::Fixed(*w))
                    .height(Length::Fixed(row_px))
                    .center_x(Length::Fixed(*w))
                    .align_y(Center)
                    .style(move |theme: &Theme| gutter_style(theme, on)),
            );
        }
        let grid = container(
            scrollable(column![heads, body].width(Length::Fixed(total_w)))
                .id(Id::new("table-grid"))
                .direction(scrollable::Direction::Horizontal(
                    scrollable::Scrollbar::new(),
                )),
        )
        .width(Fill)
        .height(Length::Fixed(grid + 12.0))
        .style(style::container::field_box);
        // The sizes row.
        let unit = format.length_unit_label();
        let size_field = |title: String,
                          value: &str,
                          on: fn(String) -> Event,
                          unit: &'static str|
         -> Element<'_, Message> {
            column![
                label::caption(title),
                row![
                    kentos_ui::widget::text_field("", value)
                        .on_input(move |t| msg(on(t)))
                        .on_submit(msg(Event::CommitSizes))
                        .width(96),
                    label::muted(unit),
                ]
                .spacing(6)
                .align_y(Center),
            ]
            .spacing(4)
            .into()
        };
        let col_word = if r.cols > 1 {
            format!(
                "{}–{} genişliği",
                column_name(r.col),
                column_name(r.col + r.cols - 1)
            )
        } else {
            format!("{} sütununun genişliği", column_name(r.col))
        };
        let row_word = if r.rows > 1 {
            format!("{}–{} yüksekliği", r.row + 1, r.row + r.rows)
        } else {
            format!("{}. satırın yüksekliği", r.row + 1)
        };
        let mut sizes_row = row![
            size_field(col_word, &e.width, Event::Width, unit),
            size_field(row_word, &e.height, Event::Height, unit),
        ]
        .spacing(18)
        .align_y(iced::Alignment::End);
        if e.draft.frame.is_some() {
            sizes_row = sizes_row.push(size_field(
                "Çerçeve".to_owned(),
                &e.frame,
                Event::FrameWidth,
                "mm",
            ));
        }
        // The source.
        let source: Element<'_, Message> = match &e.draft.source {
            None => row![
                icon(crate::icons::from_web(Some("table"))).size(16.0),
                label::body("Elle yazılmış tablo").font(typography::ui_strong())
            ]
            .spacing(6)
            .align_y(Center)
            .into(),
            Some(s) => {
                let name = match s {
                    kentos_commands::TableSource::File { .. } => "tableFile",
                    kentos_commands::TableSource::Coordinates { .. } => "tableCoordinates",
                    kentos_commands::TableSource::Areas { .. } => "tableAreas",
                    kentos_commands::TableSource::Attributes { .. } => "tableAttributes",
                    kentos_commands::TableSource::Blocks { .. } => "tableBlocks",
                    kentos_commands::TableSource::Schedule { of, .. } => match of {
                        kentos_commands::ScheduleOf::Spaces => "scheduleSpaces",
                        kentos_commands::ScheduleOf::Units => "scheduleUnits",
                        _ => "scheduleOpenings",
                    },
                };
                row![
                    icon(crate::icons::from_web(Some(name))).size(16.0),
                    label::body(format!("Kaynak: {}", super::source_words(&e.draft)))
                        .font(typography::ui_strong()),
                    label::caption("Tabloyu güncelle elle yazılanların üstüne yazar.")
                        .wrapping(iced::widget::text::Wrapping::None)
                        .width(Fill),
                    tip(
                        words::secondary(DETACH, Some(msg(Event::Detach))),
                        Tip::new("Tablo kaynağını bırakır: Tabloyu güncelle ona dokunmaz."),
                        iced::widget::tooltip::Position::Top,
                    ),
                ]
                .spacing(8)
                .align_y(Center)
                .into()
            }
        };
        // The status line.
        let width: f64 = e.draft.columns.iter().sum();
        let depth: f64 = e.draft.rows.iter().sum();
        let cell = self
            .document
            .as_ref()
            .and_then(|d| schedule_cell(&d.model, &e.saved, e.active));
        let status: Element<'_, Message> = match (&e.problem, cell) {
            (Some(p), _) => words::text_line(words::Kind::Error, p.clone()),
            (None, Some(Ok(says))) => words::text_line(words::Kind::Ok, says),
            (None, Some(Err(why))) => words::text_line(words::Kind::Warn, why),
            (None, None) => label::caption(format!(
                "{n} satır × {m} sütun; {} × {}. Çift tık, Enter ya da F2 hücreyi yazar, Shift ile alan seçilir.",
                format.length(width),
                format.length(depth)
            ))
            .into(),
        };
        let content = column![toolbar, bar, grid, sizes_row, source, status].spacing(10);
        let changed = e.draft != e.saved;
        let frame = Frame::new(EDITOR_TITLE)
            .push(content)
            .action(words::secondary(CANCEL, Some(msg(Event::Close))))
            .action(words::primary(SAVE, changed.then(|| msg(Event::Save))))
            .width(920.0);
        if e.asking {
            let question = Frame::new("Kaydedilmemiş değişiklikler")
                .push(label::body(format!(
                    "“{EDITOR_TITLE}” içinde kaydedilmemiş değişiklikler var. Pencere kapanırsa bu değişiklikler kaybolur."
                )))
                .aside(words::ghost("Kaydetmeden kapat", Some(msg(Event::Answer(Answer::Discard)))))
                .action(words::secondary(CANCEL, Some(msg(Event::Answer(Answer::Stay)))))
                .action(words::primary("Kaydet ve kapat", Some(msg(Event::Answer(Answer::Save)))))
                .width(460.0);
            return stack![
                frame.modal(msg(Event::Close)),
                question.modal(msg(Event::Answer(Answer::Stay))),
            ]
            .into();
        }
        frame.modal(msg(Event::Close))
    }

    /// The window's controls by their words (a trace's `dialog` step).
    pub(crate) fn table_editor_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        let Some(e) = &self.table_editor else {
            return Err(format!("{EDITOR_TITLE} penceresi açık değil"));
        };
        Ok(match control {
            Control::Press(SAVE) => (e.draft != e.saved).then(|| msg(Event::Save)),
            Control::Press(CANCEL) => Some(msg(Event::Close)),
            Control::Press(DETACH) => e.draft.source.is_some().then(|| msg(Event::Detach)),
            Control::Press("Kaydet ve kapat") => Some(msg(Event::Answer(Answer::Save))),
            Control::Press("Kaydetmeden kapat") => Some(msg(Event::Answer(Answer::Discard))),
            Control::Fill("Hücrenin yazısı", t) => Some(msg(Event::Words(t.to_owned()))),
            Control::Press(words) => match tool_by_words(words) {
                Some(t) => Some(msg(Event::Tool(t))),
                None => return Err(format!("“{EDITOR_TITLE}” penceresinde “{words}” yok")),
            },
            other => return Err(format!("“{EDITOR_TITLE}” penceresinde {other} yok")),
        })
    }
}

/// The active cell's owner (a merged range's top left).
fn at_cell(e: &Editor) -> (usize, usize) {
    e.owner(e.active)
}

/// What the active cell is beyond its words: its formula or its field.
fn cell_note(e: &Editor) -> String {
    let (r, c) = at_cell(e);
    if let Some(f) = e.draft.formula_at(r, c) {
        return format!("Formül: {}", f.formula);
    }
    if let Some(f) = e.draft.field_at(r, c) {
        let what = match &f.source {
            FieldSource::Project { value, name } => match name {
                Some(n) => format!("{} ({n})", value.label()),
                None => value.label().to_owned(),
            },
            FieldSource::Object { value, .. } => {
                format!("Nesnenin {}", value.label().to_lowercase())
            }
        };
        return match f.broken {
            Some(b) => format!("Alan: {what}; okunamadı: {}", b.words()),
            None => format!("Alan: {what}"),
        };
    }
    "Formül için = ile başlayın: =TOPLA(B2:B5), =B2*C2".to_owned()
}

/// `#rrggbb` as a colour (a swatch).
fn hex_color(hex: &str) -> iced::Color {
    let n = u32::from_str_radix(hex.trim_start_matches('#'), 16).unwrap_or(0);
    iced::Color::from_rgb8((n >> 16) as u8, (n >> 8) as u8, n as u8)
}

/// A toolbar button by its words (its tip, as the web's `aria-label`): a trace's `press`.
fn tool_by_words(words: &str) -> Option<Tool> {
    let counted = |suffix: &str| {
        words
            .strip_suffix(suffix)
            .is_some_and(|n| n.parse::<usize>().is_ok())
    };
    Some(match words {
        "Üstüne satır ekle" => Tool::RowAbove,
        "Altına satır ekle" => Tool::RowBelow,
        "Soluna sütun ekle" => Tool::ColumnLeft,
        "Sağına sütun ekle" => Tool::ColumnRight,
        "Hücreleri birleştir" => Tool::Merge,
        "Birleşimi ayır" => Tool::Unmerge,
        "Sütunu sola hizala" => Tool::Align(TableAlign::Left),
        "Sütunu ortala" => Tool::Align(TableAlign::Center),
        "Sütunu sağa hizala" => Tool::Align(TableAlign::Right),
        "Başlık satırı: ilk satır kalın ve ortalı" => Tool::Header,
        "Çizgiler: tümü" => Tool::Grid(None),
        "Çizgiler: yalnız dış çizgi" => Tool::Grid(Some(TableGrid::Outer)),
        "Çizgiler: dış çizgi ve satırlar" => Tool::Grid(Some(TableGrid::Rows)),
        "Çizgiler: yok" => Tool::Grid(Some(TableGrid::None)),
        "Kalın çerçeve" => Tool::Frame,
        "Yazıya sığdır: satır ve sütunlar yazıları kadar" => Tool::Fit,
        "Geri al (Ctrl+Z)" => Tool::Undo,
        "Yinele (Ctrl+Y)" => Tool::Redo,
        "Satırı sil" => Tool::DeleteRows,
        "Sütunu sil" => Tool::DeleteColumns,
        "Kalın: seçili hücreler" => Tool::Bold,
        "Biçimi temizle: seçili hücrelere değen biçimler" => Tool::ClearStyle,
        "Alanı kaldır: yazısı kalır" => Tool::Unfield,
        _ if counted(" satırı sil") => Tool::DeleteRows,
        _ if counted(" sütunu sil") => Tool::DeleteColumns,
        _ => return None,
    })
}

/// The bar's words written to the active cell (a formula when they begin with `=`), the column widened as they ask.
/// What a schedule's cell at `at` is (docs/adr/0733 §6): written to its row's objects (the words say which), or read
/// only and why; none for another table.
pub(crate) fn schedule_cell(
    doc: &kentos_domain::Document,
    t: &kentos_commands::TableEntity,
    at: (usize, usize),
) -> Option<Result<String, String>> {
    let Some(kentos_commands::TableSource::Schedule {
        of,
        columns,
        total,
        members,
        units,
        ..
    }) = &t.source
    else {
        return None;
    };
    let (r, c) = at;
    // A units schedule's row is its program groups (docs/adr/0795).
    let groups = (*of == kentos_commands::ScheduleOf::Units)
        .then(|| units.get(r.wrapping_sub(1)).filter(|g| !g.is_empty()));
    let column = columns.get(c)?;
    if r == 0 {
        return Some(Err(
            "Başlık satırı: sütunun adı belgenin yazdığıdır; düzenlenmez.".into(),
        ));
    }
    let objects = members.get(r - 1).filter(|m| !m.is_empty());
    if let Some(Some(groups)) = groups {
        if !kentos_native_application::schedules::editable(*of, column.field) {
            return Some(Err(format!(
                "Hesaplanır: “{}” bina programından ve mahallerden okunur; düzenlenmez.",
                doc.schedule_heading_in(*of, column)
            )));
        }
        let program = doc.program();
        let names: Vec<String> = groups
            .iter()
            .filter_map(|g| program.unit(g))
            .map(|u| match &u.name {
                Some(n) => format!("{} {n}", u.number),
                None => u.number.clone(),
            })
            .collect();
        return Some(Ok(format!(
            "Bina programına yazılır: {}. Kaydet tek adımda yazar; çizelge ve etiketler izler.",
            names.join(", ")
        )));
    }
    let Some(objects) = objects.filter(|_| groups.is_none()) else {
        let what = if *total && r + 1 == t.cells.len() {
            "Genel toplam"
        } else {
            "Grup başlığı ya da ara toplam"
        };
        return Some(Err(format!(
            "{what}: nesnelerin satırı değil; düzenlenmez."
        )));
    };
    if !kentos_native_application::schedules::editable(*of, column.field) {
        return Some(Err(format!(
            "Hesaplanır: “{}” çizimden okunur; düzenlenmez. Kaynağın kendi alanını değiştirin.",
            doc.schedule_heading(column)
        )));
    }
    let names: Vec<String> = objects
        .iter()
        .filter_map(|u| {
            Uuid::parse_str(u)
                .ok()
                .and_then(|u| doc.slot_of(u))
                .and_then(|s| doc.get(s))
        })
        .filter_map(|e| {
            ["number", "tag", "name", "type"]
                .iter()
                .find_map(|p| {
                    doc.arch_value(e, p, None)
                        .map(|v| doc.arch_value_text(&v, false))
                })
                .filter(|w| !w.is_empty())
        })
        .take(6)
        .collect();
    let more = objects.len().saturating_sub(names.len());
    let mut words = format!("Yazılır: {} nesne", objects.len());
    if !names.is_empty() {
        words.push_str(&format!(" — {}", names.join(", ")));
        if more > 0 {
            words.push_str(&format!(" ve {more} nesne daha"));
        }
    }
    Some(Ok(format!("{words}. Kaydet hepsine tek adımda yazar.")))
}

fn commit_edits(e: &Editor) -> Vec<TableEdit> {
    let at = e.owner(e.active);
    if e.words == e.words_at(at) {
        return Vec::new();
    }
    vec![
        TableEdit::SetCell {
            row: at.0 as u32,
            col: at.1 as u32,
            words: e.words.clone(),
        },
        TableEdit::Fit {
            from: at.1 as u32,
            count: 1,
            grow: true,
            rows: false,
        },
    ]
}

/// The selected cells' words cleared, one step.
fn clear_edits(e: &Editor) -> Vec<TableEdit> {
    let r = e.selected();
    let mut out = Vec::new();
    for i in r.row..r.row + r.rows {
        for j in r.col..r.col + r.cols {
            let hidden = range_at(&e.draft, i, j).is_some_and(|g| (g.row, g.col) != (i, j));
            if !hidden && !e.words_at((i, j)).is_empty() {
                out.push(TableEdit::SetCell {
                    row: i as u32,
                    col: j as u32,
                    words: String::new(),
                });
            }
        }
    }
    out
}

/// A style over the selection.
fn style_edit(r: CellRange, f: impl FnOnce(&mut TableEdit)) -> TableEdit {
    let mut s = TableEdit::Style {
        row: r.row as u32,
        col: r.col as u32,
        rows: r.rows as u32,
        cols: r.cols as u32,
        align: None,
        bold: None,
        color: None,
        fill: None,
        decimals: None,
    };
    f(&mut s);
    s
}

/// A toolbar button's edits (Ctrl+Z and Ctrl+Y are the window's).
fn tool_edits(e: &Editor, t: Tool) -> Vec<TableEdit> {
    let r = e.selected();
    let (row, col, rows, cols) = (r.row as u32, r.col as u32, r.rows as u32, r.cols as u32);
    let fit_new = |at: u32, count: u32| TableEdit::Fit {
        from: at,
        count,
        grow: true,
        rows: false,
    };
    match t {
        Tool::RowAbove => vec![TableEdit::InsertRows {
            at: row,
            count: rows,
        }],
        Tool::RowBelow => vec![TableEdit::InsertRows {
            at: row + rows,
            count: rows,
        }],
        Tool::ColumnLeft => vec![
            TableEdit::InsertColumns {
                at: col,
                count: cols,
            },
            fit_new(col, cols),
        ],
        Tool::ColumnRight => vec![
            TableEdit::InsertColumns {
                at: col + cols,
                count: cols,
            },
            fit_new(col + cols, cols),
        ],
        Tool::DeleteRows => vec![TableEdit::DeleteRows {
            from: row,
            count: rows,
        }],
        Tool::DeleteColumns => vec![TableEdit::DeleteColumns {
            from: col,
            count: cols,
        }],
        Tool::Merge => vec![TableEdit::Merge {
            row,
            col,
            rows,
            cols,
        }],
        Tool::Unmerge => vec![TableEdit::Unmerge {
            row: e.active.0 as u32,
            col: e.active.1 as u32,
        }],
        Tool::Align(a) => vec![TableEdit::ColumnAlign {
            from: col,
            count: cols,
            align: a,
        }],
        Tool::Header => vec![TableEdit::Header {
            on: !e.draft.header,
        }],
        Tool::Grid(g) => vec![TableEdit::Grid { grid: g }],
        // 0,7 mm on paper, as Tablo ekle's (the web's).
        Tool::Frame => vec![TableEdit::Frame {
            width: match e.draft.frame {
                Some(_) => None,
                None => Some(0.7 / 1000.0 * e.scale),
            },
        }],
        Tool::Fit => vec![TableEdit::Fit {
            from: 0,
            count: e.m() as u32,
            grow: false,
            rows: true,
        }],
        Tool::Bold => {
            let on = e
                .draft
                .cell_style(r.row, r.col)
                .bold
                .unwrap_or(e.draft.header && r.row == 0);
            vec![style_edit(r, |s| {
                if let TableEdit::Style { bold, .. } = s {
                    *bold = Some(!on);
                }
            })]
        }
        Tool::Color(i) => vec![style_edit(r, |s| {
            if let TableEdit::Style { color, .. } = s {
                *color = TEXT_COLORS.get(i).map(|(_, c)| (*c).to_owned());
            }
        })],
        Tool::Fill(i) => vec![style_edit(r, |s| {
            if let TableEdit::Style { fill, .. } = s {
                *fill = FILLS.get(i).map(|(_, c)| (*c).to_owned());
            }
        })],
        Tool::Decimals(d) => vec![style_edit(r, |s| {
            if let TableEdit::Style { decimals, .. } = s {
                *decimals = Some(d);
            }
        })],
        Tool::ClearStyle => vec![TableEdit::ClearStyle {
            row,
            col,
            rows,
            cols,
        }],
        Tool::Field(i) => match e.blocks.get(i) {
            Some(name) => {
                let at = e.owner(e.active);
                vec![TableEdit::Field {
                    row: at.0 as u32,
                    col: at.1 as u32,
                    source: FieldSource::Project {
                        value: ProjectValue::BlockCount,
                        name: Some(name.clone()),
                    },
                    format: None,
                }]
            }
            None => Vec::new(),
        },
        Tool::Unfield => {
            let at = e.owner(e.active);
            if e.draft.field_at(at.0, at.1).is_some() {
                vec![TableEdit::Unfield {
                    row: at.0 as u32,
                    col: at.1 as u32,
                }]
            } else {
                Vec::new()
            }
        }
        Tool::Undo | Tool::Redo => Vec::new(),
    }
}

/// The sizes row typed: the selected columns' width, rows' height and the frame, in the project's unit.
fn sizes_edits(e: &Editor, f: &kentos_interaction::Format) -> Vec<TableEdit> {
    let number = |t: &str| {
        t.trim()
            .replace(',', ".")
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite() && *v > 0.0)
    };
    let r = e.selected();
    let mut out = Vec::new();
    if let Some(w) = number(&e.width)
        && (r.col..r.col + r.cols).any(|j| e.draft.columns[j] != f.to_metres(w))
    {
        out.push(TableEdit::ColumnWidth {
            from: r.col as u32,
            count: r.cols as u32,
            width: f.to_metres(w),
        });
    }
    if let Some(h) = number(&e.height)
        && (r.row..r.row + r.rows).any(|i| e.draft.rows[i] != f.to_metres(h))
    {
        out.push(TableEdit::RowHeight {
            from: r.row as u32,
            count: r.rows as u32,
            height: f.to_metres(h),
        });
    }
    if let Some(fr) = e.draft.frame
        && let Some(w) = number(&e.frame)
        && fr != w / 1000.0 * e.scale
    {
        out.push(TableEdit::Frame {
            width: Some(w / 1000.0 * e.scale),
        });
    }
    out
}

/// A row number's or a column letter's cell: the header's look, the accent on the selection's.
fn gutter_style(theme: &Theme, on: bool) -> container::Style {
    let t = Tokens::of(theme);
    container::Style {
        background: Some(iced::Background::Color(if on {
            t.selection()
        } else {
            t.header
        })),
        text_color: Some(if on { t.accent } else { t.muted }),
        border: iced::Border {
            color: t.border,
            width: 0.5,
            radius: 0.0.into(),
        },
        ..container::Style::default()
    }
}

/// A cell: lines between, the selection's fill, the active cell's accent ring.
/// A grid cell: the selection's colour when chosen, else its fill; its style's colour, on a fill a dark ink, else
/// the theme's text (a dark colour lightened on a dark theme, to be read).
fn cell_style(
    theme: &Theme,
    chosen: bool,
    active: bool,
    fill: Option<iced::Color>,
    ink: Option<iced::Color>,
) -> container::Style {
    let t = Tokens::of(theme);
    let dark = t.is_dark;
    let filled = fill.filter(|_| !chosen);
    let text = match (ink, filled) {
        (Some(c), Some(_)) => c,
        (Some(c), None) if dark => iced::Color::from_rgb(
            c.r + (1.0 - c.r) * 0.45,
            c.g + (1.0 - c.g) * 0.45,
            c.b + (1.0 - c.b) * 0.45,
        ),
        (Some(c), None) => c,
        (None, Some(_)) => iced::Color::from_rgb8(0x1b, 0x1f, 0x24),
        (None, None) => t.text,
    };
    container::Style {
        background: Some(iced::Background::Color(if chosen {
            t.selection()
        } else {
            filled.unwrap_or(t.field)
        })),
        text_color: Some(text),
        border: iced::Border {
            color: if active { t.accent } else { t.border },
            width: if active { 2.0 } else { 0.5 },
            radius: 0.0.into(),
        },
        ..container::Style::default()
    }
}
