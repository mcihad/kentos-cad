//! Köşe tablosu's view (docs/adr/0172 §2–§4; the web's `VertexTable`): the
//! bar (what the table does, the count, Göster) over the table, the object's
//! measures below; a value cell double-clicked holds the editor's field
//! (`points::cell`).

use iced::widget::{button, column, container, mouse_area, row, space};
use iced::{Center, Element, Fill, Length};
use kentos_contracts::Entity;
use kentos_interaction::{Format, bearing_grad, measures};
use kentos_ui::icon::icon;
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::widget::table::{Column as TableColumn, Row as TableLine, Table};
use kentos_ui::widget::{Tip, tip};

use super::edit::{At, Column, cell_editable, kind_of};
use super::{Cell, Event, Shown, Target, Walk, ring_names, texts};
use crate::app::{App, Message};
use crate::icons::from_web;
use crate::points::cell;

fn msg(e: Event) -> Message {
    Message::Vertices(e)
}

/// What a column shows.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Key {
    No,
    Ring,
    Value(Column),
    Chord,
    Bearing,
}

/// A column: what it shows, its header, whether it holds numbers.
#[derive(Clone)]
struct Col {
    key: Key,
    title: String,
    numeric: bool,
    width: Length,
}

impl Col {
    /// The cell it edits.
    fn edit(&self) -> Option<Column> {
        match self.key {
            Key::Value(c) => Some(c),
            _ => None,
        }
    }
}

/// The object's name and its own measures, as the coordinate list says
/// them (arcs and holes counted), and the note when the table does not write.
fn footer(e: &Entity, format: &Format, note: &str) -> String {
    let base = e.base();
    let title = match &base.label {
        Some(label) => match base.attrs.get("Ada") {
            Some(ada) if !ada.is_empty() => format!("{ada} ada {label}"),
            _ => label.clone(),
        },
        None => format!("#{}", base.id),
    };
    let text = match measures(e) {
        (Some(area), length) => format!(
            "{title}   Alan {}{}",
            format.area(area),
            length.map_or_else(String::new, |l| format!("   Çevre {}", format.length(l)))
        ),
        (None, Some(length)) => format!("{title}   Uzunluk {}", format.length(length)),
        (None, None) => title,
    };
    if note.is_empty() {
        text
    } else {
        format!("{text}   {note}")
    }
}

impl App {
    /// Köşe tablosu in the Koordinat listesi tab: its bar over the table,
    /// the object's measures below.
    pub(crate) fn vertex_table(&self, target: Target) -> Element<'_, Message> {
        let Some(doc) = &self.document else {
            return space::horizontal().into();
        };
        let Some(e) = doc.model.get(target.slot) else {
            return space::horizontal().into();
        };
        let Some(kind) = kind_of(e) else {
            return space::horizontal().into();
        };
        let format = Format::of(doc.settings());
        let rows = self.vertex_rows(target.slot);
        let panel = &self.vertices;
        let selected = panel
            .selection_on(target.slot, rows.len())
            .cloned()
            .unwrap_or_default();
        let paths = kentos_native_application::elevation::paths(e);
        let rings = ring_names(e);
        let unit = format.length_unit_label();
        // East and north as the project's type names them (docs/adr/0165 §4).
        let col = |key, title: String, numeric, width| Col {
            key,
            title,
            numeric,
            width,
        };
        let mut cols = vec![col(Key::No, "Köşe".into(), true, Length::Fixed(56.0))];
        if rings.len() > 1 {
            cols.push(col(
                Key::Ring,
                "Halka".into(),
                false,
                Length::FillPortion(2),
            ));
        }
        cols.extend([
            col(
                Key::Value(Column::East),
                format.axes_text("Y (sağa)"),
                true,
                Length::FillPortion(3),
            ),
            col(
                Key::Value(Column::North),
                format.axes_text("X (yukarı)"),
                true,
                Length::FillPortion(3),
            ),
            col(
                Key::Value(Column::Z),
                "Z (kot)".into(),
                true,
                Length::FillPortion(2),
            ),
        ]);
        // A line's edge takes no arc: its column would stay empty.
        if kind != kentos_geometry_core::ops::vertex_table::Kind::Line {
            cols.push(col(
                Key::Value(Column::Radius),
                format!("Yarıçap ({unit})"),
                true,
                Length::FillPortion(2),
            ));
        }
        cols.extend([
            col(
                Key::Chord,
                format!("Kenar ({unit})"),
                true,
                Length::FillPortion(2),
            ),
            col(
                Key::Bearing,
                format!(
                    "{} ({})",
                    format.direction_name(),
                    format.angle_unit_label()
                ),
                true,
                Length::FillPortion(2),
            ),
        ]);
        let columns = cols.iter().map(|c| {
            let column = TableColumn::new(c.title.clone()).width(c.width);
            if c.numeric {
                column.align_right()
            } else {
                column
            }
        });
        let editing = panel.editing;
        let any_selected = !selected.is_empty();
        let text = panel.text.clone();
        let writes = target.writes;
        let count = rows.len();
        let draft_at = panel.draft_at(&rows);
        let draft = panel.draft.clone();
        let shown_rows = rows.clone();
        // The draft's row stands under its vertex's row.
        let which = move |i: usize| match draft_at {
            Some(d) if i == d => Shown::Draft,
            Some(d) if i > d => Shown::Row(i - 1),
            _ => Shown::Row(i),
        };
        let table = Table::new(columns)
            .virtualized(count + usize::from(draft_at.is_some()), move |i| {
                let field = |numeric: bool| {
                    cell::edit_box(
                        &text,
                        numeric,
                        |t| msg(Event::Input(t)),
                        msg(Event::Finish(Some(Walk::Down))),
                        msg(Event::Finish(None)),
                        msg(Event::Cancel),
                    )
                };
                let Shown::Row(k) = which(i) else {
                    // Satır ekle's row: Yeni, its ring, its cells as typed, the one edited open.
                    let (after, d) = draft
                        .clone()
                        .unwrap_or((At { path: 0, index: 0 }, super::edit::Draft::default()));
                    let cells = cols.iter().map(|c| -> Element<'_, Message> {
                        if let (Some(col), Some((Cell::Draft, ec))) = (c.edit(), editing)
                            && ec == col
                        {
                            return field(c.numeric);
                        }
                        let value = match c.key {
                            Key::No => {
                                return label::strong(texts::DRAFT)
                                    .style(style::text::accent)
                                    .into();
                            }
                            Key::Ring => rings.get(after.path).cloned().unwrap_or_default(),
                            Key::Value(Column::East) => d.east.clone(),
                            Key::Value(Column::North) => d.north.clone(),
                            Key::Value(Column::Z) => d.z.clone(),
                            _ => String::new(),
                        };
                        let shown: Element<'_, Message> = if c.numeric {
                            label::mono(value).style(style::text::muted).into()
                        } else {
                            label::muted(value).into()
                        };
                        match c.edit() {
                            Some(col) if col != Column::Radius => mouse_area(shown)
                                .on_double_click(msg(Event::Edit(i, col)))
                                .into(),
                            _ => shown,
                        }
                    });
                    return TableLine::new(cells).selected(true).current(false);
                };
                let r = &shown_rows[k];
                let at = At {
                    path: r.path,
                    index: r.index,
                };
                let next = paths.get(r.path).and_then(|p| {
                    let n = if r.index + 1 < p.pts.len() {
                        Some(r.index + 1)
                    } else if p.closed {
                        Some(0)
                    } else {
                        None
                    };
                    n.map(|n| p.pts[n])
                });
                let cells = cols.iter().map(|c| -> Element<'_, Message> {
                    if let (Some(col), Some((Cell::Vertex(a), ec))) = (c.edit(), editing)
                        && a == at
                        && ec == col
                    {
                        return field(c.numeric);
                    }
                    let value = match c.key {
                        Key::No => (k + 1).to_string(),
                        Key::Ring => rings.get(r.path).cloned().unwrap_or_default(),
                        Key::Value(Column::East) => format.coord(r.p.x),
                        Key::Value(Column::North) => format.coord(r.p.y),
                        Key::Value(Column::Z) => {
                            r.z.map(|z| format.length_bare(z)).unwrap_or_default()
                        }
                        Key::Value(Column::Radius) => {
                            r.radius.map(|v| format.length_bare(v)).unwrap_or_default()
                        }
                        Key::Chord => r.chord.map(|v| format.length_bare(v)).unwrap_or_default(),
                        Key::Bearing => next
                            .map(|n| format.direction_bare(bearing_grad(r.p, n)))
                            .unwrap_or_default(),
                    };
                    if c.key == Key::No {
                        return mouse_area(label::muted(value))
                            .on_double_click(msg(Event::Zoom(i)))
                            .into();
                    }
                    let shown: Element<'_, Message> = if c.numeric {
                        label::mono(value).into()
                    } else {
                        label::body(value).into()
                    };
                    match c.edit() {
                        Some(col) if writes && cell_editable(kind, r, col) => mouse_area(shown)
                            .on_double_click(msg(Event::Edit(i, col)))
                            .into(),
                        _ => shown,
                    }
                });
                TableLine::new(cells)
                    .selected(selected.contains(&at))
                    // Every selected row alike, as the web shows them: no primary one.
                    .current(false)
                    .on_press(msg(Event::Press(i)))
            })
            .reveal(match editing {
                Some((Cell::Draft, _)) => draft_at,
                Some((Cell::Vertex(at), _)) => rows
                    .iter()
                    .position(|r| r.path == at.path && r.index == at.index)
                    .map(|k| match draft_at {
                        Some(d) if k >= d => k + 1,
                        _ => k,
                    }),
                None => None,
            });
        let hint = if writes {
            texts::HINT
        } else if target.note == texts::LOCKED_NOTE {
            texts::LOCKED
        } else {
            texts::MANY
        };
        let bar_button =
            |glyph: &str, words: &'static str, hint: &'static str, on: Option<Message>| {
                tip(
                    button(
                        row![icon(from_web(Some(glyph))).size(14.0), label::body(words)]
                            .spacing(6)
                            .align_y(Center),
                    )
                    .style(style::button::secondary)
                    .padding([4, 10])
                    .on_press_maybe(on),
                    Tip::new(words).body(hint),
                    iced::widget::tooltip::Position::Top,
                )
            };
        let bar = container(
            row![
                label::muted(hint),
                space::horizontal(),
                label::muted(format!("{count} köşe")),
                bar_button(
                    "plus",
                    texts::ADD,
                    texts::ADD_HINT,
                    (writes && count > 0).then(|| msg(Event::AddRow)),
                ),
                bar_button(
                    "erase",
                    texts::REMOVE,
                    texts::REMOVE_HINT,
                    (writes && any_selected).then(|| msg(Event::Remove)),
                ),
                bar_button(
                    "zoomSelection",
                    texts::SHOW,
                    texts::SHOW_HINT,
                    (count > 0).then(|| msg(Event::Show)),
                ),
            ]
            .spacing(10)
            .align_y(Center),
        )
        .padding([6, 10])
        .width(Fill);
        column![
            bar,
            kentos_ui::widget::horizontal_divider(),
            container(table).height(Fill),
            container(label::mono_caption(footer(e, &format, target.note))).padding([4, 12]),
        ]
        .width(Fill)
        .height(Fill)
        .into()
    }
}
