//! Noktalar, the bottom panel's point editor (docs/adr/0153 §1–§4; the web's
//! `ui/bottom/PointTable.ts`): every point of the drawing in a table. Its
//! search, layer and selection filters and its column sort are the shared
//! core's (`point_table`), so both platforms show the same order.
//!
//! - A click on a row selects the point in the drawing; Ctrl turns one over,
//!   Shift takes the run from the last click without Shift.
//! - The drawing's selection shows in the rows; when its first row changes
//!   it is scrolled into view (once: scrolling away stays).
//! - Göster, or a double click on a row's number, zooms to the selected points.
//! - A double click on Ad, Y, X, Z or Kod edits it in place (`edit.rs` writes
//!   it): Enter writes and goes down the column, Tab right, Shift+Tab left, a
//!   press elsewhere writes and stops, Esc gives up. Satır ekle opens a draft
//!   row at the bottom; Enter writes it and opens the next, its name one more.
//!   Sil deletes the selected points.
//! - The query (search, layer, only the selected, sort) and Bağlı çizgiler
//!   izler are kept for as long as the app lives.
//!
//! The rows are worked out once for the drawing, the selection and the query
//! as they are, not at every frame (docs/adr/0120). The view is `view.rs`.

use std::cell::RefCell;

use iced::Task;
use kentos_contracts::Entity;
use kentos_domain::Slot;
use kentos_geometry_core::ops::point_editor::{TableQuery, TableRow, point_table};

use crate::app::{App, Message};
use crate::document::Document;

mod cell;
pub mod edit;
#[cfg(test)]
mod tests;
mod view;

use edit::{Draft, EditColumn};

/// The columns: their header, the core's sort key (none: Sıra, the drawing's
/// order), whether they hold numbers and the cell edited (the web's `POINT_COLUMNS`).
pub const COLUMNS: [(&str, Option<&str>, bool, Option<EditColumn>); 7] = [
    ("Sıra", None, true, None),
    ("Ad", Some("name"), false, Some(EditColumn::Name)),
    ("Y (sağa)", Some("east"), true, Some(EditColumn::East)),
    ("X (yukarı)", Some("north"), true, Some(EditColumn::North)),
    ("Z (kot)", Some("z"), true, Some(EditColumn::Z)),
    ("Kod", Some("code"), false, Some(EditColumn::Code)),
    ("Katman", Some("layer"), false, None),
];

/// The cells the editor walks, in the order Tab takes them.
pub const EDIT_COLUMNS: [EditColumn; 5] = [
    EditColumn::Name,
    EditColumn::East,
    EditColumn::North,
    EditColumn::Z,
    EditColumn::Code,
];

/// The tab's words (the web's `POINT_TEXTS`).
pub mod texts {
    pub const SEARCH: &str = "Ad ya da kod ara";
    pub const ALL_LAYERS: &str = "Bütün katmanlar";
    pub const ONLY_SELECTED: &str = "Yalnız seçililer";
    pub const FOLLOW: &str = "Bağlı çizgiler izler";
    pub const ADD: &str = "Satır ekle";
    pub const REMOVE: &str = "Sil";
    pub const SHOW: &str = "Göster";
    pub const DRAFT: &str = "Yeni";
    pub const NONE: &str = "Çizimde nokta yok. Nokta aracıyla, Satır ekle ile ya da Nokta listesi içe aktar ile ekleyin.";
    pub const NO_MATCH: &str = "Süzgece uyan nokta yok.";
}

/// A header click (docs/adr/0153 §2): ascending, then descending, then the
/// drawing's order; Sıra is the drawing's order (the web's `nextSort`).
pub fn next_sort(
    sort: Option<&'static str>,
    descending: bool,
    column: Option<&'static str>,
) -> (Option<&'static str>, bool) {
    match column {
        None => (None, false),
        Some(c) if sort == Some(c) && descending => (None, false),
        Some(c) => (Some(c), sort == Some(c)),
    }
}

/// What a click on the row at `at` (in the order shown) selects: that point
/// alone; with Ctrl the selection with it turned over; with Shift the run
/// from `anchor` (the last click without Shift) to it (the web's `clickPick`).
pub fn click_pick(
    selected: &[Slot],
    shown: &[Slot],
    at: usize,
    anchor: Option<usize>,
    ctrl: bool,
    shift: bool,
) -> Vec<Slot> {
    let id = shown[at];
    if shift && let Some(anchor) = anchor.filter(|&a| a < shown.len()) {
        let (a, b) = if anchor <= at {
            (anchor, at)
        } else {
            (at, anchor)
        };
        return shown[a..=b].to_vec();
    }
    if ctrl {
        return if selected.contains(&id) {
            selected.iter().copied().filter(|&s| s != id).collect()
        } else {
            selected.iter().copied().chain([id]).collect()
        };
    }
    vec![id]
}

/// Where the editor goes after a cell of the row `id` (docs/adr/0153 §3; the
/// web's `nextCell`), in `ids`, the rows' order before the write: down the
/// column, right (Kod to the next row's Ad), left (Ad to the row above's
/// Kod); none past the ends.
pub fn next_cell(ids: &[Slot], id: Slot, col: EditColumn, how: Walk) -> Option<(Slot, EditColumn)> {
    let at = ids.iter().position(|&s| s == id)?;
    if how == Walk::Down {
        return ids.get(at + 1).map(|&s| (s, col));
    }
    let c = EDIT_COLUMNS.iter().position(|&c| c == col)?;
    let right = how == Walk::Right;
    let next = if right { c + 1 } else { c.wrapping_sub(1) };
    if next < EDIT_COLUMNS.len() {
        return Some((id, EDIT_COLUMNS[next]));
    }
    let row = if right { at + 1 } else { at.checked_sub(1)? };
    let col = if right {
        EDIT_COLUMNS[0]
    } else {
        EDIT_COLUMNS[EDIT_COLUMNS.len() - 1]
    };
    ids.get(row).map(|&s| (s, col))
}

/// A point as the table reads it (the core's `TableRow`; the web's `rowOf`).
pub fn row_of(p: &kentos_contracts::PointEntity, layer: &str, selected: bool) -> TableRow {
    TableRow {
        name: p.base.label.clone(),
        east: p.p.x,
        north: p.p.y,
        z: p.z,
        code: p.base.attrs.get("Kod").cloned(),
        layer: layer.to_owned(),
        selected,
    }
}

/// Which way an edit ends: Enter, Tab, Shift+Tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Walk {
    Down,
    Right,
    Left,
}

/// The cell edited: a point's, or the draft's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Point(Slot),
    Draft,
}

/// The tab's messages.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Search(String),
    /// A layer by name; none: all.
    Layer(Option<String>),
    OnlySelected(bool),
    Follow(bool),
    /// A header pressed, by its column.
    Sort(usize),
    /// A row pressed, by its place in the order shown.
    Press(usize),
    /// A row's number double-clicked: that point, zoomed to.
    Zoom(usize),
    /// Göster: the selected points zoomed to.
    Show,
    /// A value cell double-clicked: the row (the draft after the points) and the column.
    Edit(usize, usize),
    /// The editor's text typed.
    Input(String),
    /// The edit ends: written and gone a way (Enter, Tab, Shift+Tab), or
    /// stopped (a press elsewhere).
    Finish(Option<Walk>),
    /// Esc: the edit given up (the draft with it).
    Cancel,
    /// Satır ekle.
    AddRow,
    /// Sil: the selected points deleted.
    Remove,
}

/// The rows as worked out for a drawing, a selection and a query.
#[derive(Clone, Debug, Default)]
pub(crate) struct Rows {
    /// The points in the drawing's order.
    points: Vec<Slot>,
    /// The points shown, in the order shown.
    shown: Vec<Slot>,
    /// The first selected row in the order shown.
    first_selected: Option<usize>,
    /// The layers holding points, in the layer list's order: name and count.
    layers: Vec<(String, usize)>,
}

/// The open drawing (its session), its changes, the selection's version and the query's.
type Key = (u64, u64, u64, u64);

/// The tab's state, kept for as long as the app lives.
pub(crate) struct PointsPanel {
    search: String,
    layer: Option<String>,
    only_selected: bool,
    sort: Option<&'static str>,
    descending: bool,
    /// Bağlı çizgiler izler.
    follow: bool,
    /// The last click without Shift, in the order shown.
    anchor: Option<usize>,
    /// The cell edited and the editor's text.
    editing: Option<(Target, EditColumn)>,
    text: String,
    draft: Option<Draft>,
    /// Bumped at every change of the query.
    version: u64,
    cache: RefCell<Option<(Key, Rows)>>,
}

impl Default for PointsPanel {
    fn default() -> Self {
        Self {
            search: String::new(),
            layer: None,
            only_selected: false,
            sort: None,
            descending: false,
            follow: true,
            anchor: None,
            editing: None,
            text: String::new(),
            draft: None,
            version: 0,
            cache: RefCell::default(),
        }
    }
}

impl PointsPanel {
    fn query(&self) -> TableQuery {
        TableQuery {
            search: self.search.clone(),
            layer: self.layer.clone(),
            only_selected: self.only_selected,
            sort: self.sort.map(str::to_owned),
            descending: self.descending,
        }
    }

    fn changed(&mut self) {
        self.version += 1;
    }

    /// Whether a cell is being edited: Tab is its key then.
    pub(crate) fn editing(&self) -> bool {
        self.editing.is_some()
    }
}

fn layer_name(doc: &kentos_domain::Document, id: &str) -> String {
    doc.layers()
        .get(id)
        .map_or_else(|| id.to_owned(), |n| n.name.clone())
}

/// The keyboard to the editor's field, its text chosen.
fn focus_field() -> Task<Message> {
    let id = iced::widget::Id::new(cell::FIELD);
    Task::batch([
        iced::widget::operation::focus(id.clone()),
        iced::widget::operation::select_all(id),
    ])
}

impl App {
    /// The rows for the drawing, the selection and the query as they are,
    /// worked out again only when one of them changed.
    pub(crate) fn point_rows(&self, doc: &Document) -> Rows {
        let panel = &self.points;
        let key = (
            doc.session,
            doc.model.generation(),
            self.selection.version(),
            panel.version,
        );
        if let Some((known, rows)) = panel.cache.borrow().as_ref()
            && *known == key
        {
            return rows.clone();
        }
        let model = &doc.model;
        let mut points = Vec::new();
        let mut data = Vec::new();
        let mut counts: Vec<(String, usize)> = Vec::new();
        for e in model.entities() {
            let Entity::Point(p) = e else {
                continue;
            };
            let slot = Slot(p.base.id);
            let name = layer_name(model, &p.base.layer_id);
            match counts.iter_mut().find(|(id, _)| *id == p.base.layer_id) {
                Some((_, n)) => *n += 1,
                None => counts.push((p.base.layer_id.clone(), 1)),
            }
            data.push(row_of(p, &name, self.selection.contains(slot)));
            points.push(slot);
        }
        // In the layer list's order.
        let order: Vec<&str> = model
            .layers()
            .leaves()
            .iter()
            .map(|n| n.id.as_str())
            .collect();
        counts.sort_by_key(|(id, _)| order.iter().position(|o| o == id).unwrap_or(usize::MAX));
        let layers: Vec<(String, usize)> = counts
            .into_iter()
            .map(|(id, n)| (layer_name(model, &id), n))
            .collect();
        let mut query = panel.query();
        // A layer chosen that no longer holds points shows all.
        if query
            .layer
            .as_ref()
            .is_some_and(|l| !layers.iter().any(|(name, _)| name == l))
        {
            query.layer = None;
        }
        let shown: Vec<Slot> = point_table(&data, &query)
            .into_iter()
            .map(|i| points[i as usize])
            .collect();
        let first_selected = shown.iter().position(|&s| self.selection.contains(s));
        let rows = Rows {
            points,
            shown,
            first_selected,
            layers,
        };
        *panel.cache.borrow_mut() = Some((key, rows.clone()));
        rows
    }

    /// The rows shown now (none without a drawing).
    fn shown_points(&self) -> Vec<Slot> {
        self.document
            .as_ref()
            .map(|doc| self.point_rows(doc).shown)
            .unwrap_or_default()
    }

    pub(crate) fn points_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Search(text) => {
                self.points.search = text;
                self.points.changed();
            }
            Event::Layer(layer) => {
                self.points.layer = layer;
                self.points.changed();
            }
            Event::OnlySelected(on) => {
                self.points.only_selected = on;
                self.points.changed();
            }
            Event::Follow(on) => self.points.follow = on,
            Event::Sort(column) => {
                let p = &mut self.points;
                (p.sort, p.descending) = next_sort(p.sort, p.descending, COLUMNS[column].1);
                p.changed();
            }
            Event::Press(at) => {
                let shown = self.shown_points();
                if at >= shown.len() {
                    return Task::none();
                }
                let (ctrl, shift) = (self.modifiers.control(), self.modifiers.shift());
                let ids = click_pick(
                    self.selection.ids(),
                    &shown,
                    at,
                    self.points.anchor,
                    ctrl,
                    shift,
                );
                if !shift {
                    self.points.anchor = Some(at);
                }
                self.selection.set(ids);
            }
            Event::Zoom(at) => {
                if let Some(&slot) = self.shown_points().get(at) {
                    self.selection.set([slot]);
                    self.navigating(Self::zoom_selection);
                }
            }
            Event::Show => self.navigating(Self::zoom_selection),
            Event::Edit(at, column) => {
                let Some(col) = COLUMNS.get(column).and_then(|c| c.3) else {
                    return Task::none();
                };
                let shown = self.shown_points();
                let target = match shown.get(at) {
                    Some(&slot) => Target::Point(slot),
                    None if self.points.draft.is_some() => Target::Draft,
                    None => return Task::none(),
                };
                return self.edit_cell(Some((target, col)));
            }
            Event::Input(text) => self.points.text = text,
            Event::Finish(walk) => return self.finish_cell(walk),
            Event::Cancel => {
                if matches!(self.points.editing, Some((Target::Draft, _))) {
                    self.points.draft = None;
                }
                self.points.editing = None;
            }
            Event::AddRow => {
                if self.points.draft.is_none() {
                    self.points.draft = Some(Draft::default());
                }
                return self.edit_cell(Some((Target::Draft, EditColumn::Name)));
            }
            Event::Remove => return self.update(Message::Run("tool.erase")),
        }
        Task::none()
    }

    /// The cell edited from now (none: no editor), its text the value's.
    fn edit_cell(&mut self, to: Option<(Target, EditColumn)>) -> Task<Message> {
        self.points.editing = to;
        let Some((target, col)) = to else {
            return Task::none();
        };
        self.points.text = match target {
            Target::Draft => {
                let d = self.points.draft.clone().unwrap_or_default();
                match col {
                    EditColumn::Name => d.name,
                    EditColumn::East => d.east,
                    EditColumn::North => d.north,
                    EditColumn::Z => d.z,
                    EditColumn::Code => d.code,
                }
            }
            Target::Point(slot) => match self.document.as_ref().and_then(|d| d.model.get(slot)) {
                Some(Entity::Point(p)) => edit::cell_text(p, col),
                _ => String::new(),
            },
        };
        focus_field()
    }

    /// The edit ends with the editor's text: written, then the editor goes
    /// `walk`'s way (none: it stops). A value refused keeps the cell open
    /// with what was typed when a key ended it.
    fn finish_cell(&mut self, walk: Option<Walk>) -> Task<Message> {
        let Some((target, col)) = self.points.editing else {
            return Task::none();
        };
        let text = std::mem::take(&mut self.points.text);
        match target {
            Target::Draft => {
                let mut d = self.points.draft.clone().unwrap_or_default();
                match col {
                    EditColumn::Name => d.name = text,
                    EditColumn::East => d.east = text,
                    EditColumn::North => d.north = text,
                    EditColumn::Z => d.z = text,
                    EditColumn::Code => d.code = text,
                }
                self.points.draft = Some(d.clone());
                match walk {
                    None => {
                        self.points.editing = None;
                        Task::none()
                    }
                    Some(Walk::Down) => {
                        let layer = self
                            .document
                            .as_ref()
                            .map(|doc| doc.model.layers().active().to_owned())
                            .unwrap_or_default();
                        let color = self.draft.color.map(str::to_owned);
                        let Some(doc) = self.document.as_mut() else {
                            return Task::none();
                        };
                        let out = edit::write_draft(&mut doc.model, &d, &layer, color.as_deref());
                        for line in out.outcome.said {
                            self.warn(line);
                        }
                        match out.next {
                            // Written: the next row, its name one more, its Y open.
                            Some(next) => {
                                self.points.draft = Some(Draft {
                                    name: next,
                                    ..Draft::default()
                                });
                                self.edit_cell(Some((Target::Draft, EditColumn::East)))
                            }
                            None => self.edit_cell(Some((target, col))),
                        }
                    }
                    Some(walk) => {
                        let c = EDIT_COLUMNS.iter().position(|&c| c == col).unwrap_or(0);
                        let n = EDIT_COLUMNS.len();
                        let next = if walk == Walk::Right {
                            (c + 1) % n
                        } else {
                            (c + n - 1) % n
                        };
                        self.edit_cell(Some((Target::Draft, EDIT_COLUMNS[next])))
                    }
                }
            }
            Target::Point(slot) => {
                // Where to go next, by the rows' order before the write (a sort may move the row).
                let next = walk.and_then(|w| next_cell(&self.shown_points(), slot, col, w));
                let follow = self.points.follow;
                let Some(doc) = self.document.as_mut() else {
                    return Task::none();
                };
                let out = edit::write_cell(&mut doc.model, slot, col, &text, follow);
                for line in out.said {
                    self.warn(line);
                }
                if out.stay && walk.is_some() {
                    self.points.editing = Some((target, col));
                    self.points.text = text;
                    return focus_field();
                }
                self.edit_cell(next.map(|(s, c)| (Target::Point(s), c)))
            }
        }
    }
}
