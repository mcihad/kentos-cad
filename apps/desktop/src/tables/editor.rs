//! Tabloyu düzenle (docs/adr/0184 §5; the web's `ui/table/TableEditor.ts`):
//! the table's cells in a grid as a spreadsheet's (column letters, row
//! numbers, the heading row bold, merged ranges as one cell), the active
//! cell's words in the bar above. Rows and columns are added and deleted,
//! cells merged and unmerged (the core's `ops::table_edit`, refusals said in
//! the status line); a column's alignment, the heading row, Çizgiler and
//! Kalın çerçeve, a column's width and a row's height, Yazıya sığdır;
//! Kaynaktan ayır. A column widens as its words ask. Ctrl+Z and Ctrl+Y undo
//! and redo inside the window; Kaydet writes the table in one step
//! (`cad.entities.edit` `table`, “Tablo”).

use iced::keyboard::key::Named;
use iced::widget::{
    Id, Space, button, column, container, mouse_area, pin, row, scrollable, stack, text_input,
};
use iced::{Center, Element, Fill, Length, Task, Theme};
use kentos_contracts::{
    CommandResult, EditOperation, EntitiesEdit, Entity, EntityEdit, TableAlign, TableEntity,
    TableGrid,
};
use kentos_domain::{Slot, Uuid};
use kentos_geometry_core::entity::CellRange;
use kentos_geometry_core::ops::table::sizes;
use kentos_geometry_core::ops::table_edit::{self, Edit};
use kentos_interaction::Level;
use kentos_native_application::geometry::{drawing_font, edit_geometry, entity_of, shape};
use kentos_native_application::{ExecutionContext, edit};
use kentos_ui::icon::icon;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::{Dialog as Frame, Tip, focus_ring, overlay, tip};
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
    past: Vec<TableEntity>,
    future: Vec<TableEntity>,
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
}

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

/// A table's core shape.
fn core(t: &TableEntity) -> kentos_geometry_core::entity::Shape {
    shape(&Entity::Table(t.clone()))
}

/// The table the core gives back, with the draft's object fields.
fn back(t: &TableEntity, s: kentos_geometry_core::entity::Shape) -> Option<TableEntity> {
    let g = edit_geometry(s)?;
    match entity_of(&g, t.base.clone()) {
        Entity::Table(out) => Some(out),
        _ => None,
    }
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

    fn words_at(&self, (r, c): (usize, usize)) -> String {
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

    /// Keeps a step for Ctrl+Z, then makes `next` the draft.
    fn take(&mut self, next: TableEntity) {
        self.version += 1;
        self.past.push(std::mem::replace(&mut self.draft, next));
        if self.past.len() > 200 {
            self.past.remove(0);
        }
        self.future.clear();
        self.problem = None;
        self.clamp();
    }

    /// Each column at least as wide as its words ask (a cell never spills).
    fn widen(t: &mut TableEntity, font: kentos_geometry_core::text::Font) {
        if let Some(fit) = sizes(&core(t), font) {
            for (w, f) in t.columns.iter_mut().zip(fit.columns) {
                *w = w.max(f);
            }
        }
    }

    /// One of the core's edits on the draft; a refusal is said in the status line.
    fn edit(&mut self, e: Edit, grow: bool, font: kentos_geometry_core::text::Font) -> bool {
        let out = table_edit::edit(&core(&self.draft), &e);
        let Some(next) = out.table.and_then(|s| back(&self.draft, s)) else {
            self.problem = Some(out.problem.unwrap_or_else(|| "Yapılamadı.".to_owned()));
            return false;
        };
        let mut next = next;
        if grow {
            Self::widen(&mut next, font);
        }
        self.take(next);
        true
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
    fn reveal(&mut self) -> Option<f32> {
        let top = self.active.0 as f32 * typography::scaled(ROW_PX);
        let row = typography::scaled(ROW_PX);
        let view = typography::scaled(GRID_PX) - row;
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
            past: Vec::new(),
            future: Vec::new(),
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

    fn table_font(&self) -> kentos_geometry_core::text::Font {
        drawing_font(
            self.document
                .as_ref()
                .and_then(|d| d.model.settings().drawing_font),
        )
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
        let font = self.table_font();
        let format = self.format();
        let Some(e) = self.table_editor.as_mut() else {
            return Task::none();
        };
        let mut task = Task::none();
        let before = (e.active, e.anchor, e.version);
        match event {
            Event::Cell(r, c) => {
                commit_words(e, font);
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
                commit_words(e, font);
                e.step(1, 0, false);
                task = iced::widget::operation::focus(Id::new("table-grid"));
            }
            Event::Tool(t) => tool(e, t, font),
            Event::Width(t) => e.width = t,
            Event::Height(t) => e.height = t,
            Event::FrameWidth(t) => e.frame = t,
            Event::CommitSizes => sizes_typed(e, &format),
            Event::Detach => {
                let mut next = e.draft.clone();
                next.source = None;
                e.take(next);
            }
            Event::Scrolled(y) => e.scroll = y,
            Event::Key(key, shift, bar) => {
                if bar {
                    match key {
                        Named::Escape => {
                            e.words = e.words_at(e.owner(e.active));
                            task = iced::widget::operation::focus(Id::new("table-grid"));
                        }
                        Named::Tab => {
                            commit_words(e, font);
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
                            commit_words(e, font);
                            e.step(1, 0, false);
                        }
                        Named::Enter | Named::F2 => {
                            e.words = e.words_at(e.owner(e.active));
                            task = Task::batch([
                                iced::widget::operation::focus(Id::new(BAR)),
                                iced::widget::operation::move_cursor_to_end(Id::new(BAR)),
                            ]);
                        }
                        Named::Delete | Named::Backspace => clear_selected(e),
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
                if e.draft != e.saved {
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
        let Some(e) = self.table_editor.as_mut() else {
            return task;
        };
        // The bar and the sizes follow the selection and the draft; what is being typed stays.
        if (e.active, e.anchor, e.version) == before {
            return task;
        }
        e.fields(&format);
        if let Some(y) = e.reveal() {
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

    /// Kaydet: the table written in one step “Tablo” (`cad.entities.edit` `table`).
    fn save_table_editor(&mut self) {
        let Some(e) = self.table_editor.as_ref() else {
            return;
        };
        if e.draft == e.saved {
            self.table_editor = None;
            self.dialog = None;
            return;
        }
        let Some(geometry) = edit_geometry(core(&e.draft)) else {
            return;
        };
        let input = EntitiesEdit {
            operation: EditOperation::Table,
            changes: vec![EntityEdit::Update {
                uid: e.uid.to_string(),
                geometry,
            }],
            expected_revision: None,
        };
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        match edit::execute(&mut ExecutionContext::new(&mut doc.model), input) {
            CommandResult::Completed { warnings, .. } => {
                for w in warnings {
                    self.warn(w.message);
                }
                self.say(Level::Success, "Tablo kaydedildi. Ctrl+Z geri alır.");
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
                    .padding(5)
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
                !e.past.is_empty()
            ),
            tool_button(
                "redo",
                "Yinele (Ctrl+Y)".into(),
                Tool::Redo,
                None,
                !e.future.is_empty()
            ),
        ]
        .spacing(2)
        .align_y(Center);
        let toolbar = container(tools)
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
            focus_ring(
                text_input("", &e.words)
                    .id(Id::new(BAR))
                    .on_input(|t| msg(Event::Words(t)))
                    .on_submit(msg(Event::Commit))
                    .padding([5, 8])
                    .width(Fill)
                    .style(style::field::input)
            ),
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
        let view_h = typography::scaled(GRID_PX) - row_px;
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
                let words = e.words_at(top);
                let heading = e.draft.header && top.0 == 0;
                let alignment = if heading {
                    TableAlign::Center
                } else {
                    e.draft
                        .aligns
                        .as_ref()
                        .and_then(|a| a.get(top.1).copied())
                        .unwrap_or(TableAlign::Left)
                };
                let w: f32 = widths[j..(j + span_cols).min(m)].iter().sum();
                let h = span_rows as f32 * row_px;
                let active = top == e.active;
                let chosen = in_range(i, j);
                let mut text = label::body(words).wrapping(iced::widget::text::Wrapping::None);
                if heading {
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
                    .style(move |theme: &Theme| cell_style(theme, chosen, active));
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
        .height(Length::Fixed(typography::scaled(GRID_PX) + 12.0))
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
                    focus_ring(
                        text_input("", value)
                            .on_input(move |t| msg(on(t)))
                            .on_submit(msg(Event::CommitSizes))
                            .padding([5, 8])
                            .width(96)
                            .style(style::field::input)
                    ),
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
                    kentos_contracts::TableSource::File { .. } => "tableFile",
                    kentos_contracts::TableSource::Coordinates { .. } => "tableCoordinates",
                    kentos_contracts::TableSource::Areas { .. } => "tableAreas",
                    kentos_contracts::TableSource::Attributes { .. } => "tableAttributes",
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
        let status: Element<'_, Message> = match &e.problem {
            Some(p) => words::text_line(words::Kind::Error, p.clone()),
            None => label::caption(format!(
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
                overlay::modal(frame, msg(Event::Close)),
                overlay::modal(question, msg(Event::Answer(Answer::Stay))),
            ]
            .into();
        }
        overlay::modal(frame, msg(Event::Close))
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
        _ if counted(" satırı sil") => Tool::DeleteRows,
        _ if counted(" sütunu sil") => Tool::DeleteColumns,
        _ => return None,
    })
}

/// Writes the active cell's words from the bar, the column widened as they ask.
fn commit_words(e: &mut Editor, font: kentos_geometry_core::text::Font) {
    let at = e.owner(e.active);
    if e.words == e.words_at(at) {
        return;
    }
    let words = e.words.clone();
    e.edit(
        Edit::SetCell {
            row: at.0,
            col: at.1,
            words,
        },
        true,
        font,
    );
}

/// Clears the selected cells' words, one step.
fn clear_selected(e: &mut Editor) {
    let r = e.selected();
    let mut next = e.draft.clone();
    let mut any = false;
    for i in r.row..r.row + r.rows {
        for j in r.col..r.col + r.cols {
            if next
                .cells
                .get(i)
                .and_then(|row| row.get(j))
                .is_some_and(|w| !w.is_empty())
            {
                let out = table_edit::edit(
                    &core(&next),
                    &Edit::SetCell {
                        row: i,
                        col: j,
                        words: String::new(),
                    },
                );
                if let Some(t) = out.table.and_then(|s| back(&next, s)) {
                    next = t;
                    any = true;
                }
            }
        }
    }
    if any {
        e.take(next);
    }
}

/// A toolbar button's work.
fn tool(e: &mut Editor, t: Tool, font: kentos_geometry_core::text::Font) {
    let r = e.selected();
    match t {
        Tool::RowAbove => {
            e.edit(
                Edit::InsertRows {
                    at: r.row,
                    count: r.rows,
                },
                false,
                font,
            );
        }
        Tool::RowBelow => {
            e.edit(
                Edit::InsertRows {
                    at: r.row + r.rows,
                    count: r.rows,
                },
                false,
                font,
            );
        }
        Tool::ColumnLeft => {
            e.edit(
                Edit::InsertColumns {
                    at: r.col,
                    count: r.cols,
                },
                true,
                font,
            );
        }
        Tool::ColumnRight => {
            e.edit(
                Edit::InsertColumns {
                    at: r.col + r.cols,
                    count: r.cols,
                },
                true,
                font,
            );
        }
        Tool::DeleteRows => {
            e.edit(
                Edit::DeleteRows {
                    from: r.row,
                    count: r.rows,
                },
                false,
                font,
            );
        }
        Tool::DeleteColumns => {
            e.edit(
                Edit::DeleteColumns {
                    from: r.col,
                    count: r.cols,
                },
                false,
                font,
            );
        }
        Tool::Merge => {
            e.edit(Edit::Merge(r), false, font);
        }
        Tool::Unmerge => {
            e.edit(
                Edit::Unmerge {
                    row: e.active.0,
                    col: e.active.1,
                },
                false,
                font,
            );
        }
        Tool::Align(a) => {
            let mut next = e.draft.clone();
            let mut aligns = next
                .aligns
                .take()
                .unwrap_or_else(|| vec![TableAlign::Left; e.m()]);
            for j in r.col..r.col + r.cols {
                if let Some(x) = aligns.get_mut(j) {
                    *x = a;
                }
            }
            next.aligns = (!aligns.iter().all(|x| *x == TableAlign::Left)).then_some(aligns);
            e.take(next);
        }
        Tool::Header => {
            let mut next = e.draft.clone();
            next.header = !next.header;
            e.take(next);
        }
        Tool::Grid(g) => {
            let mut next = e.draft.clone();
            next.grid = g;
            e.take(next);
        }
        Tool::Frame => {
            let mut next = e.draft.clone();
            // 0,7 mm on paper, as Tablo ekle's (the web's).
            next.frame = match next.frame {
                Some(_) => None,
                None => Some(0.7 / 1000.0 * e.scale),
            };
            e.take(next);
        }
        Tool::Fit => {
            if let Some(s) = sizes(&core(&e.draft), font) {
                let mut next = e.draft.clone();
                next.rows = s.rows;
                next.columns = s.columns;
                e.take(next);
            }
        }
        Tool::Undo => {
            if let Some(prev) = e.past.pop() {
                e.future.push(std::mem::replace(&mut e.draft, prev));
                e.version += 1;
                e.problem = None;
                e.clamp();
            }
        }
        Tool::Redo => {
            if let Some(next) = e.future.pop() {
                e.past.push(std::mem::replace(&mut e.draft, next));
                e.version += 1;
                e.problem = None;
                e.clamp();
            }
        }
    }
}

/// The sizes row typed: the selected columns' width, rows' height and the frame, in the project's unit.
fn sizes_typed(e: &mut Editor, f: &kentos_interaction::Format) {
    let number = |t: &str| {
        t.trim()
            .replace(',', ".")
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite() && *v > 0.0)
    };
    let r = e.selected();
    let mut next = e.draft.clone();
    if let Some(w) = number(&e.width) {
        for j in r.col..r.col + r.cols {
            next.columns[j] = f.to_metres(w);
        }
    }
    if let Some(h) = number(&e.height) {
        for i in r.row..r.row + r.rows {
            next.rows[i] = f.to_metres(h);
        }
    }
    if next.frame.is_some()
        && let Some(w) = number(&e.frame)
    {
        next.frame = Some(w / 1000.0 * e.scale);
    }
    if next != e.draft {
        e.take(next);
    }
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
fn cell_style(theme: &Theme, chosen: bool, active: bool) -> container::Style {
    let t = Tokens::of(theme);
    container::Style {
        background: Some(iced::Background::Color(if chosen {
            t.selection()
        } else {
            t.field
        })),
        text_color: Some(t.text),
        border: iced::Border {
            color: if active { t.accent } else { t.border },
            width: if active { 2.0 } else { 0.5 },
            radius: 0.0.into(),
        },
        ..container::Style::default()
    }
}
