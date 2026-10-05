//! Köşe tablosu (docs/adr/0172; the web's `ui/bottom/VertexTable.ts`): the
//! bottom panel's Koordinat listesi for one line, polyline or area, its
//! vertices in the core's order (the outer ring, its holes, then each part's
//! ring and holes) with Halka (when there is more than one), Y, X, Z, the
//! signed radius of the edge leaving each, its chord and bearing; the
//! object's measures below.
//!
//! - A click selects a row (Ctrl turns one over, Shift takes the run from
//!   the last click); the selected rows' vertices are ringed in the drawing
//!   (marks.rs). The selection is the table's, not the drawing's; another
//!   object, or another vertex count, clears it.
//! - A double click on Köşe, or Göster, brings the vertex to the view's
//!   middle, its scale kept.
//! - While the table writes (one object selected, its layer not locked), a
//!   double click on Y, X, Z or Yarıçap edits it in place (`edit.rs` writes
//!   it): Enter writes and goes down the column, Tab right, Shift+Tab left,
//!   a press elsewhere writes and stops, Esc gives up.
//! - Satır ekle opens a draft row under the last selected row (the last row
//!   when none is): Y, X and Z typed, Enter adds the vertex after that row's
//!   and opens the next draft under it. Sil, or Delete once a row was
//!   pressed, removes the selected rows' vertices.
//!
//! The rows are worked out once for the drawing as it is, not at every
//! frame (docs/adr/0120). The view is `view.rs`.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;

use iced::Task;
use kentos_contracts::Entity;
use kentos_domain::Slot;
use kentos_geometry_core::ops::vertex_table::{Kind, Row};

use crate::app::{App, Message};

pub mod edit;
#[cfg(test)]
mod tests;
mod view;

use edit::{At, Column, cell_editable, cell_text, kind_of, rows_of};

/// The tab's words (the web's `VERTEX_TEXTS`).
pub mod texts {
    pub const SHOW: &str = "Göster";
    pub const SHOW_HINT: &str = "Seçili satırın köşesini görünümün ortasına getirir";
    pub const ADD: &str = "Satır ekle";
    pub const ADD_HINT: &str = "Seçili satırın altında yeni satır: Y, X ve Z yazılır, Enter köşeyi o satırın köşesinden sonra ekler ve sonrakini açar.";
    pub const REMOVE: &str = "Sil";
    pub const REMOVE_HINT: &str = "Seçili satırların köşelerini siler (Delete)";
    pub const DRAFT: &str = "Yeni";
    pub const HINT: &str =
        "Değeri değiştirmek için hücreye çift tıklayın; Enter yazar, Tab sağa geçer.";
    pub const LOCKED: &str = "Katman kilitli; köşeler düzenlenmez.";
    pub const MANY: &str = "Birden çok nesne seçili; düzenlemek için tek nesne seçin.";
    pub const LOCKED_NOTE: &str = "(katman kilitli)";
    pub const MANY_NOTE: &str = "(ilk nesne gösteriliyor; düzenlemek için tek nesne seçin)";
}

/// Which way an edit ends: Enter, Tab, Shift+Tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Walk {
    Down,
    Right,
    Left,
}

/// The table's messages. A row is named by its place among the rows shown,
/// the draft's row among them.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// A row pressed.
    Press(usize),
    /// A row's number double-clicked: its vertex to the view's middle.
    Zoom(usize),
    /// Göster: the first selected vertex to the view's middle.
    Show,
    /// A value cell double-clicked: the row and the column.
    Edit(usize, Column),
    /// The editor's text typed.
    Input(String),
    /// The edit ends: written and gone a way (Enter, Tab, Shift+Tab), or
    /// stopped (a press elsewhere).
    Finish(Option<Walk>),
    /// Esc: the edit given up (the draft with it).
    Cancel,
    /// Satır ekle: a draft row under the last selected row.
    AddRow,
    /// Sil, or Delete in the table: the selected rows' vertices removed.
    Remove,
}

/// What the table shows: the object, whether the table writes it, and the
/// note when it does not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Target {
    pub slot: Slot,
    pub writes: bool,
    pub note: &'static str,
}

/// A cell's row: a vertex's, or the draft's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cell {
    Vertex(At),
    Draft,
}

/// A row shown: a vertex's row (its index in the rows), or the draft's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shown {
    Row(usize),
    Draft,
}

/// The open drawing (its session), its changes and the object.
type Key = (u64, u64, Slot);

/// Topoloji's state for a write: on or off, and Noktalar da.
#[derive(Clone, Copy)]
struct TopologyOn {
    on: bool,
    points: bool,
}

impl TopologyOn {
    /// The topology a write takes, with the store (in step with the drawing).
    fn of(self, spatial: &kentos_interaction::spatial::Spatial) -> Option<edit::Topology<'_>> {
        self.on.then_some(edit::Topology {
            spatial,
            points: self.points,
        })
    }
}

/// The tab's state, kept for as long as the app lives.
#[derive(Default)]
pub(crate) struct VertexPanel {
    /// The object the row selection was made on, and its vertex count then.
    object: Option<(Slot, usize)>,
    selected: BTreeSet<At>,
    /// The last click without Shift, by the row's index.
    anchor: Option<usize>,
    /// The cell edited and the editor's text.
    editing: Option<(Cell, Column)>,
    text: String,
    /// Satır ekle's row: the vertex it goes after and its cells as typed.
    draft: Option<(At, edit::Draft)>,
    /// Whether the table has the keyboard: a row was pressed last (Delete
    /// is then the table's, not the drawing's Sil).
    pub(crate) keyboard: bool,
    cache: RefCell<Option<(Key, Rc<Vec<Row>>)>>,
}

impl VertexPanel {
    /// Whether a cell is being edited: Tab is its key then.
    pub(crate) fn editing(&self) -> bool {
        self.editing.is_some()
    }

    /// The row selection, when it was made on this object as it is.
    fn selection_on(&self, slot: Slot, count: usize) -> Option<&BTreeSet<At>> {
        (self.object == Some((slot, count))).then_some(&self.selected)
    }

    /// The selection follows the object: another object, or another vertex
    /// count (a vertex added or removed, an undo), clears it.
    fn follow(&mut self, slot: Slot, count: usize) {
        if self.object.is_some_and(|(s, _)| s != slot) {
            self.draft = None;
            self.editing = None;
        }
        if self.object != Some((slot, count)) {
            self.object = Some((slot, count));
            self.selected.clear();
            self.anchor = None;
        }
    }

    /// Where the draft's row stands among the rows shown (under its
    /// vertex's row), when there is a draft and its vertex is there.
    fn draft_at(&self, rows: &[Row]) -> Option<usize> {
        let (after, _) = self.draft.as_ref()?;
        rows.iter()
            .position(|r| r.path == after.path && r.index == after.index)
            .map(|i| i + 1)
    }

    /// What the row shown at `i` is.
    fn shown(&self, rows: &[Row], i: usize) -> Shown {
        match self.draft_at(rows) {
            Some(d) if i == d => Shown::Draft,
            Some(d) if i > d => Shown::Row(i - 1),
            _ => Shown::Row(i),
        }
    }
}

/// What a click on the row at `at` selects: it alone; with Ctrl the
/// selection with it turned over; with Shift the run from `anchor` (the
/// web's `clickRows`).
pub fn click_rows(
    selected: &BTreeSet<At>,
    rows: &[At],
    at: usize,
    anchor: Option<usize>,
    ctrl: bool,
    shift: bool,
) -> BTreeSet<At> {
    let key = rows[at];
    if shift && let Some(anchor) = anchor.filter(|&a| a < rows.len()) {
        let (a, b) = if anchor <= at {
            (anchor, at)
        } else {
            (at, anchor)
        };
        return rows[a..=b].iter().copied().collect();
    }
    if ctrl {
        let mut out = selected.clone();
        if !out.remove(&key) {
            out.insert(key);
        }
        return out;
    }
    BTreeSet::from([key])
}

/// Where the editor goes after a cell (docs/adr/0172 §4; the web's
/// `nextVertexCell`), by the rows' order: Enter down the column (to the next
/// row whose cell is edited), Tab right (Yarıçap to the next row's Y),
/// Shift+Tab left; none past the ends.
pub fn next_cell(
    kind: Kind,
    rows: &[Row],
    at: usize,
    col: Column,
    walk: Walk,
) -> Option<(usize, Column)> {
    if walk == Walk::Down {
        return (at + 1..rows.len())
            .find(|&r| cell_editable(kind, &rows[r], col))
            .map(|r| (r, col));
    }
    let right = walk == Walk::Right;
    let n = Column::ALL.len();
    let (mut r, mut c) = (at, Column::ALL.iter().position(|&x| x == col)?);
    loop {
        if right {
            c += 1;
            if c == n {
                r += 1;
                c = 0;
            }
        } else if c == 0 {
            r = r.checked_sub(1)?;
            c = n - 1;
        } else {
            c -= 1;
        }
        if r >= rows.len() {
            return None;
        }
        if cell_editable(kind, &rows[r], Column::ALL[c]) {
            return Some((r, Column::ALL[c]));
        }
    }
}

/// The rings' names, path by path (the elevations' order): Dış, Delik 1,
/// Parça 2, Parça 2, delik 1; a multi-part polyline's Parça 1, Parça 2
/// (docs/adr/0174; the web's `ringNames`).
pub fn ring_names(e: &Entity) -> Vec<String> {
    if let Entity::Polyline(l) = e
        && let Some(parts) = &l.parts
    {
        return (1..=parts.len() + 1)
            .map(|k| format!("Parça {k}"))
            .collect();
    }
    let Entity::Polygon(area) = e else {
        return vec![String::new()];
    };
    let mut out = vec!["Dış".to_owned()];
    out.extend((1..=area.holes.as_ref().map_or(0, Vec::len)).map(|i| format!("Delik {i}")));
    for (k, part) in area.parts.iter().flatten().enumerate() {
        out.push(format!("Parça {}", k + 2));
        out.extend(
            (1..=part.holes.as_ref().map_or(0, Vec::len))
                .map(|i| format!("Parça {}, delik {i}", k + 2)),
        );
    }
    out
}

/// The keyboard to the editor's field, its text chosen.
fn focus_field() -> Task<Message> {
    crate::points::focus_field()
}

impl App {
    /// The object Köşe tablosu shows, when the coordinate list shows line
    /// work (docs/adr/0172 §1): the first object of the selection that is
    /// not a point or a text, when it is a line, a polyline or an area; the
    /// table writes it when it alone is selected and its layer is not locked.
    pub(crate) fn vertex_target(&self) -> Option<Target> {
        let doc = self.document.as_ref()?;
        let model = &doc.model;
        let entities: Vec<&Entity> = self
            .selection
            .ids()
            .iter()
            .filter_map(|s| model.get(*s))
            .collect();
        if entities.is_empty() || entities.iter().all(|e| matches!(e, Entity::Point(_))) {
            return None;
        }
        let e = entities
            .iter()
            .find(|e| !matches!(e, Entity::Point(_) | Entity::Text(_)))
            .unwrap_or(&entities[0]);
        kind_of(e)?;
        let locked = model.layers().is_locked(&e.base().layer_id);
        let many = entities.len() > 1;
        Some(Target {
            slot: Slot(e.base().id),
            writes: !many && !locked,
            note: if many {
                texts::MANY_NOTE
            } else if locked {
                texts::LOCKED_NOTE
            } else {
                ""
            },
        })
    }

    /// The object's rows, worked out again only when the drawing changed.
    pub(crate) fn vertex_rows(&self, slot: Slot) -> Rc<Vec<Row>> {
        let Some(doc) = &self.document else {
            return Rc::default();
        };
        let key = (doc.session, doc.model.generation(), slot);
        if let Some((known, rows)) = self.vertices.cache.borrow().as_ref()
            && *known == key
        {
            return rows.clone();
        }
        let rows = Rc::new(doc.model.get(slot).map(rows_of).unwrap_or_default());
        *self.vertices.cache.borrow_mut() = Some((key, rows.clone()));
        rows
    }

    /// The vertices the table's selected rows name, to ring in the drawing:
    /// none while the panel does not show the table.
    pub(crate) fn vertex_marks(&self) -> Vec<kentos_interaction::Vec2> {
        if !self.command_expanded || self.bottom_tab != crate::bottom::BottomTab::Coords {
            return Vec::new();
        }
        let Some(target) = self.vertex_target() else {
            return Vec::new();
        };
        let rows = self.vertex_rows(target.slot);
        let Some(selected) = self.vertices.selection_on(target.slot, rows.len()) else {
            return Vec::new();
        };
        rows.iter()
            .filter(|r| {
                selected.contains(&At {
                    path: r.path,
                    index: r.index,
                })
            })
            .map(|r| r.p)
            .collect()
    }

    pub(crate) fn vertices_event(&mut self, event: Event) -> Task<Message> {
        let Some(target) = self.vertex_target() else {
            self.vertices.editing = None;
            self.vertices.draft = None;
            return Task::none();
        };
        let rows = self.vertex_rows(target.slot);
        self.vertices.follow(target.slot, rows.len());
        let at_of = |i: usize| {
            rows.get(i).map(|r| At {
                path: r.path,
                index: r.index,
            })
        };
        match event {
            Event::Press(i) => {
                let Shown::Row(i) = self.vertices.shown(&rows, i) else {
                    return Task::none();
                };
                let keys: Vec<At> = (0..rows.len()).filter_map(at_of).collect();
                if i >= keys.len() {
                    return Task::none();
                }
                // The table takes the keyboard: Delete is its own then.
                self.vertices.keyboard = true;
                self.layers_keyboard = false;
                self.blocks_panel.keyboard = false;
                self.templates_panel.keyboard = false;
                let (ctrl, shift) = (self.modifiers.control(), self.modifiers.shift());
                let panel = &mut self.vertices;
                panel.selected = click_rows(&panel.selected, &keys, i, panel.anchor, ctrl, shift);
                if !shift {
                    panel.anchor = Some(i);
                }
            }
            Event::Zoom(i) => {
                let Shown::Row(i) = self.vertices.shown(&rows, i) else {
                    return Task::none();
                };
                if let (Some(at), Some(row)) = (at_of(i), rows.get(i)) {
                    self.vertices.selected = BTreeSet::from([at]);
                    self.vertices.anchor = Some(i);
                    let p = row.p;
                    self.navigating(|app| app.viewport.camera.center_on(p));
                }
            }
            Event::Show => {
                let selected = &self.vertices.selected;
                let row = rows
                    .iter()
                    .find(|r| {
                        selected.contains(&At {
                            path: r.path,
                            index: r.index,
                        })
                    })
                    .or_else(|| rows.first());
                if let Some(p) = row.map(|r| r.p) {
                    self.navigating(|app| app.viewport.camera.center_on(p));
                }
            }
            Event::Edit(i, col) => {
                if !target.writes {
                    return Task::none();
                }
                return match self.vertices.shown(&rows, i) {
                    Shown::Draft if col != Column::Radius => self.edit_draft(col),
                    Shown::Draft => Task::none(),
                    Shown::Row(i) => {
                        let (Some(at), Some(row)) = (at_of(i), rows.get(i)) else {
                            return Task::none();
                        };
                        let kind = self
                            .document
                            .as_ref()
                            .and_then(|d| d.model.get(target.slot))
                            .and_then(kind_of);
                        if !kind.is_some_and(|k| cell_editable(k, row, col)) {
                            return Task::none();
                        }
                        self.edit_vertex(Some((at, col)), row)
                    }
                };
            }
            Event::Input(text) => self.vertices.text = text,
            Event::Finish(walk) => return self.finish_vertex(target.slot, &rows, walk),
            Event::Cancel => {
                if matches!(self.vertices.editing, Some((Cell::Draft, _))) {
                    self.vertices.draft = None;
                }
                self.vertices.editing = None;
            }
            Event::AddRow => {
                if !target.writes {
                    return Task::none();
                }
                let selected = &self.vertices.selected;
                let after = rows
                    .iter()
                    .rev()
                    .find(|r| {
                        selected.contains(&At {
                            path: r.path,
                            index: r.index,
                        })
                    })
                    .or_else(|| rows.last());
                let Some(after) = after.map(|r| At {
                    path: r.path,
                    index: r.index,
                }) else {
                    return Task::none();
                };
                self.vertices.draft = Some((after, edit::Draft::default()));
                return self.edit_draft(Column::East);
            }
            Event::Remove => {
                let at: Vec<At> = rows
                    .iter()
                    .map(|r| At {
                        path: r.path,
                        index: r.index,
                    })
                    .filter(|a| self.vertices.selected.contains(a))
                    .collect();
                if !target.writes || at.is_empty() {
                    return Task::none();
                }
                let Some(topology) = self.vertex_topology() else {
                    return Task::none();
                };
                let Some(doc) = self.document.as_mut() else {
                    return Task::none();
                };
                let out =
                    edit::remove(&mut doc.model, target.slot, &at, topology.of(&self.spatial));
                self.say_vertex(out);
            }
        }
        Task::none()
    }

    /// Whether the table's writes take the neighbours along (Topoloji, docs/adr/0172 §6), the
    /// geometry store brought in step with the drawing first; none without a drawing.
    fn vertex_topology(&mut self) -> Option<TopologyOn> {
        let doc = self.document.as_ref()?;
        if self.draft.topology {
            self.spatial.sync(&doc.model);
        }
        Some(TopologyOn {
            on: self.draft.topology,
            points: self.draft.topology_points,
        })
    }

    /// Says what came of a write: its warnings, and the neighbours that changed with it.
    fn say_vertex(&mut self, out: edit::Outcome) {
        for line in out.told {
            self.output(line);
        }
        for line in out.said {
            self.warn(line);
        }
    }

    /// The cell edited from now (none: no editor), its text the value's.
    fn edit_vertex(&mut self, to: Option<(At, Column)>, row: &Row) -> Task<Message> {
        self.vertices.editing = to.map(|(at, col)| (Cell::Vertex(at), col));
        let Some((_, col)) = to else {
            return Task::none();
        };
        // In a local project's unit (docs/adr/0165 §2).
        let per_metre = self
            .document
            .as_ref()
            .map_or(1.0, |d| d.settings().unit().per_metre());
        self.vertices.text = cell_text(row, col, per_metre);
        focus_field()
    }

    /// The draft's cell `col` edited, its text as typed.
    fn edit_draft(&mut self, col: Column) -> Task<Message> {
        let Some((_, d)) = &self.vertices.draft else {
            return Task::none();
        };
        self.vertices.text = match col {
            Column::East => d.east.clone(),
            Column::North => d.north.clone(),
            Column::Z => d.z.clone(),
            Column::Radius => return Task::none(),
        };
        self.vertices.editing = Some((Cell::Draft, col));
        focus_field()
    }

    /// The edit ends with the editor's text: written, then the editor goes
    /// `walk`'s way (none: it stops). A value refused keeps the cell open
    /// with what was typed when a key ended it.
    fn finish_vertex(&mut self, slot: Slot, rows: &[Row], walk: Option<Walk>) -> Task<Message> {
        let Some((cell, col)) = self.vertices.editing else {
            return Task::none();
        };
        let text = std::mem::take(&mut self.vertices.text);
        let at = match cell {
            Cell::Draft => return self.finish_draft(slot, col, text, walk),
            Cell::Vertex(at) => at,
        };
        let Some(kind) = self
            .document
            .as_ref()
            .and_then(|d| d.model.get(slot))
            .and_then(kind_of)
        else {
            self.vertices.editing = None;
            return Task::none();
        };
        // Where to go next, by the rows before the write (a vertex's place does not change its row).
        let place = rows
            .iter()
            .position(|r| r.path == at.path && r.index == at.index);
        let next = walk.and_then(|w| place.and_then(|i| next_cell(kind, rows, i, col, w)));
        let Some(topology) = self.vertex_topology() else {
            return Task::none();
        };
        let Some(doc) = self.document.as_mut() else {
            return Task::none();
        };
        let out = edit::write_cell(
            &mut doc.model,
            slot,
            at,
            col,
            &text,
            topology.of(&self.spatial),
        );
        let stay = out.stay;
        self.say_vertex(out);
        if stay && walk.is_some() {
            self.vertices.editing = Some((cell, col));
            self.vertices.text = text;
            return focus_field();
        }
        // The rows after the write: the next cell opens with its value as it is now.
        let after = self.vertex_rows(slot);
        self.vertices.follow(slot, after.len());
        match next.and_then(|(i, c)| after.get(i).map(|r| (r.clone(), c))) {
            Some((row, c)) => {
                let at = At {
                    path: row.path,
                    index: row.index,
                };
                self.edit_vertex(Some((at, c)), &row)
            }
            None => {
                self.vertices.editing = None;
                Task::none()
            }
        }
    }

    /// The draft's cell ends with `text`: Enter writes the vertex (the next
    /// draft opens under it, its Y open; a value refused keeps the cell
    /// open), Tab and Shift+Tab walk its cells, a press elsewhere keeps what
    /// was typed.
    fn finish_draft(
        &mut self,
        slot: Slot,
        col: Column,
        text: String,
        walk: Option<Walk>,
    ) -> Task<Message> {
        let Some((after, mut d)) = self.vertices.draft.clone() else {
            self.vertices.editing = None;
            return Task::none();
        };
        match col {
            Column::East => d.east = text,
            Column::North => d.north = text,
            Column::Z => d.z = text,
            Column::Radius => {}
        }
        self.vertices.draft = Some((after, d.clone()));
        let cells = [Column::East, Column::North, Column::Z];
        let c = cells.iter().position(|&x| x == col).unwrap_or(0);
        match walk {
            None => {
                self.vertices.editing = None;
                Task::none()
            }
            Some(Walk::Right) => self.edit_draft(cells[(c + 1) % cells.len()]),
            Some(Walk::Left) => self.edit_draft(cells[(c + cells.len() - 1) % cells.len()]),
            Some(Walk::Down) => {
                let Some(topology) = self.vertex_topology() else {
                    return Task::none();
                };
                let Some(doc) = self.document.as_mut() else {
                    return Task::none();
                };
                let (out, next) =
                    edit::write_draft(&mut doc.model, slot, after, &d, topology.of(&self.spatial));
                self.say_vertex(out);
                match next {
                    Some(next) => {
                        let rows = self.vertex_rows(slot);
                        self.vertices.follow(slot, rows.len());
                        self.vertices.draft = Some((next, edit::Draft::default()));
                        self.edit_draft(Column::East)
                    }
                    None => self.edit_draft(col),
                }
            }
        }
    }
}
