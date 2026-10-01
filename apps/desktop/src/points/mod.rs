//! Noktalar, the bottom panel's point editor (docs/adr/0153 §1–§2; the web's
//! `ui/bottom/PointTable.ts`): every point of the drawing in a table. Its
//! search, layer and selection filters and its column sort are the shared
//! core's (`point_table`), so both platforms show the same order.
//!
//! - A click on a row selects the point in the drawing; Ctrl turns one over,
//!   Shift takes the run from the last click without Shift.
//! - The drawing's selection shows in the rows; when its first row changes
//!   it is scrolled into view (once: scrolling away stays).
//! - Göster, or a double click on a row's number, zooms to the selected points.
//! - The query (search, layer, only the selected, sort) is kept for as long
//!   as the app lives.
//!
//! The rows are worked out once for the drawing, the selection and the query
//! as they are, not at every frame (docs/adr/0120).

use std::cell::RefCell;

use iced::widget::{button, container, mouse_area, row, space};
use iced::{Center, Element, Fill, Length};
use kentos_contracts::Entity;
use kentos_domain::Slot;
use kentos_geometry_core::ops::point_editor::{TableQuery, TableRow, point_table};
use kentos_interaction::Format;
use kentos_ui::icon::icon;
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::widget::search_box::SearchBox;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::switch::Switch;
use kentos_ui::widget::table::{Column as TableColumn, Row as TableLine, SortOrder, Table};

use crate::app::{App, Message};
use crate::document::Document;
use crate::icons::from_web;

#[cfg(test)]
mod tests;

/// The columns: their header, the core's sort key (none: Sıra, the drawing's
/// order) and whether they hold numbers (the web's `POINT_COLUMNS`).
pub const COLUMNS: [(&str, Option<&str>, bool); 7] = [
    ("Sıra", None, true),
    ("Ad", Some("name"), false),
    ("Y (sağa)", Some("east"), true),
    ("X (yukarı)", Some("north"), true),
    ("Z (kot)", Some("z"), true),
    ("Kod", Some("code"), false),
    ("Katman", Some("layer"), false),
];

/// The tab's words (the web's `POINT_TEXTS`).
pub mod texts {
    pub const SEARCH: &str = "Ad ya da kod ara";
    pub const ALL_LAYERS: &str = "Bütün katmanlar";
    pub const ONLY_SELECTED: &str = "Yalnız seçililer";
    pub const SHOW: &str = "Göster";
    pub const NONE: &str =
        "Çizimde nokta yok. Nokta aracıyla ya da Nokta listesi içe aktar ile ekleyin.";
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

/// The tab's messages.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Search(String),
    /// A layer by name; none: all.
    Layer(Option<String>),
    OnlySelected(bool),
    /// A header pressed, by its column.
    Sort(usize),
    /// A row pressed, by its place in the order shown.
    Press(usize),
    /// A row's number double-clicked: that point, zoomed to.
    Zoom(usize),
    /// Göster: the selected points zoomed to.
    Show,
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
#[derive(Default)]
pub(crate) struct PointsPanel {
    search: String,
    layer: Option<String>,
    only_selected: bool,
    sort: Option<&'static str>,
    descending: bool,
    /// The last click without Shift, in the order shown.
    anchor: Option<usize>,
    /// Bumped at every change of the query.
    version: u64,
    cache: RefCell<Option<(Key, Rows)>>,
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
}

fn layer_name(doc: &kentos_domain::Document, id: &str) -> String {
    doc.layers()
        .get(id)
        .map_or_else(|| id.to_owned(), |n| n.name.clone())
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

    pub(crate) fn points_event(&mut self, event: Event) {
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
            Event::Sort(column) => {
                let p = &mut self.points;
                (p.sort, p.descending) = next_sort(p.sort, p.descending, COLUMNS[column].1);
                p.changed();
            }
            Event::Press(at) => {
                let Some(doc) = &self.document else {
                    return;
                };
                let rows = self.point_rows(doc);
                if at >= rows.shown.len() {
                    return;
                }
                let (ctrl, shift) = (self.modifiers.control(), self.modifiers.shift());
                let ids = click_pick(
                    self.selection.ids(),
                    &rows.shown,
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
                let Some(doc) = &self.document else {
                    return;
                };
                if let Some(&slot) = self.point_rows(doc).shown.get(at) {
                    self.selection.set([slot]);
                    self.navigating(Self::zoom_selection);
                }
            }
            Event::Show => self.navigating(Self::zoom_selection),
        }
    }

    /// The Noktalar tab: its bar (search, layer, only the selected, the
    /// count, Göster) over the table.
    pub(crate) fn points_tab(&self) -> Element<'_, Message> {
        let Some(doc) = &self.document else {
            return container(label::muted(texts::NONE)).padding(12).into();
        };
        let rows = self.point_rows(doc);
        let format = Format::of(doc.model.settings());
        let panel = &self.points;
        let msg = |e: Event| Message::Points(e);

        let search = SearchBox::new(panel.search.clone(), texts::SEARCH, move |t| {
            Message::Points(Event::Search(t))
        })
        .height(28.0);
        let mut choices = vec![Choice::new(texts::ALL_LAYERS)];
        choices.extend(
            rows.layers
                .iter()
                .map(|(name, n)| Choice::new(format!("{name} ({n})"))),
        );
        let chosen = match &panel.layer {
            None => Some(0),
            Some(l) => rows
                .layers
                .iter()
                .position(|(name, _)| name == l)
                .map(|i| i + 1),
        };
        let names: Vec<String> = rows.layers.iter().map(|(name, _)| name.clone()).collect();
        let layer = Select::new(choices, chosen.or(Some(0)), move |i| {
            Message::Points(Event::Layer(if i == 0 {
                None
            } else {
                names.get(i - 1).cloned()
            }))
        });
        let only = Switch::new(panel.only_selected, move |on| {
            Message::Points(Event::OnlySelected(on))
        })
        .label(texts::ONLY_SELECTED);
        let count = label::muted(format!(
            "{} / {} nokta",
            rows.shown.len(),
            rows.points.len()
        ));
        let show = button(
            row![
                icon(from_web(Some("zoomSelection"))).size(14.0),
                label::body(texts::SHOW)
            ]
            .spacing(6)
            .align_y(Center),
        )
        .style(style::button::secondary)
        .padding([4, 10])
        .on_press_maybe((!self.selection.is_empty()).then(|| msg(Event::Show)));
        let bar = row![
            container(search).width(Length::Fixed(220.0)),
            container(layer).width(Length::Fixed(190.0)),
            only,
            space::horizontal(),
            count,
            show,
        ]
        .spacing(10)
        .padding([6, 10])
        .align_y(Center);

        let columns = COLUMNS
            .iter()
            .enumerate()
            .map(|(i, (title, key, numeric))| {
                let order = key.filter(|k| panel.sort == Some(k)).map(|_| {
                    if panel.descending {
                        SortOrder::Descending
                    } else {
                        SortOrder::Ascending
                    }
                });
                let width = match i {
                    0 => Length::Fixed(56.0),
                    1 | 5 => Length::FillPortion(2),
                    6 => Length::FillPortion(2),
                    _ => Length::FillPortion(3),
                };
                let column = TableColumn::new(*title)
                    .width(width)
                    .sortable(order, msg(Event::Sort(i)));
                if *numeric {
                    column.align_right()
                } else {
                    column
                }
            });
        let model = &doc.model;
        let selection = &self.selection;
        let shown = rows.shown.clone();
        let table = Table::new(columns)
            .virtualized(shown.len(), move |i| {
                let slot = shown[i];
                let Some(Entity::Point(p)) = model.get(slot) else {
                    return TableLine::new([label::muted("").into()]);
                };
                let number = mouse_area(label::muted((i + 1).to_string()))
                    .on_double_click(Message::Points(Event::Zoom(i)));
                TableLine::new([
                    number.into(),
                    label::strong(p.base.label.clone().unwrap_or_default()).into(),
                    label::mono(format.coord(p.p.x)).into(),
                    label::mono(format.coord(p.p.y)).into(),
                    label::mono(p.z.map(|z| format.length_bare(z)).unwrap_or_default()).into(),
                    label::body(p.base.attrs.get("Kod").cloned().unwrap_or_default()).into(),
                    label::body(layer_name(model, &p.base.layer_id)).into(),
                ])
                .selected(selection.contains(slot))
                // Every selected row alike, as the web shows them: no primary one.
                .current(false)
                .on_press(Message::Points(Event::Press(i)))
            })
            .reveal(rows.first_selected)
            .empty(if rows.points.is_empty() {
                texts::NONE
            } else {
                texts::NO_MATCH
            });
        iced::widget::column![bar, kentos_ui::widget::horizontal_divider(), table]
            .width(Fill)
            .height(Fill)
            .into()
    }
}
