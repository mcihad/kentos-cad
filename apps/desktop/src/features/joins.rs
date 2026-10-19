//! Öznitelik tablosu's join views (docs/adr/0527): Katman ▾ lists the project's views after the layers; one
//! chosen shows its rows as `cad.joins.query` reads them from the drawing now (`joins_query::read`, worked out
//! again only when the drawing changed), each a record of the layer beside one of its matches.
//!
//! - The bar: Katman ▾, the report (rows, matched and unmatched records, records without a key, the joined
//!   layer's records met, the cardinality), Birleşik görünümler… (the window).
//! - A double click on a cell edits it; Enter writes it to the record the cell belongs to (`cad.entities.set`
//!   on the row's left or right record), Esc gives up. A key's cell, a locked layer's and a row without its
//!   record cannot be written: the cell is muted and says why on hover.
//! - Under the rows the sums of the numeric columns, each record once however many rows it is in.
//! - A view whose layer or key is gone shows why instead of rows.

use std::cell::RefCell;
use std::rc::Rc;

use iced::widget::text::Wrapping;
use iced::widget::{button, column, container, mouse_area, row, space};
use iced::{Center, Element, Fill, Length, Task};
use kentos_contracts::{EntitiesSetProperties, JoinSide, JoinView, PropertiesOperation, js_trim};
use kentos_domain::{Slot, Uuid};
use kentos_native_application::joins_query::{JoinRead, read};
use kentos_ui::icon::icon;
use kentos_ui::widget::table::{Column as TableColumn, Row as TableLine, Table};
use kentos_ui::widget::{Tip, tip};
use kentos_ui::{label, style};

use super::{Event, texts};
use crate::app::{App, Message};
use crate::document::Document;
use crate::icons::from_web;
use crate::points::{cell, focus_field};

/// The words of the view.
pub mod words {
    pub const VIEWS: &str = "Birleşik görünümler…";
    pub const VIEWS_HINT: &str = "Projenin birleşik görünümleri: bir katmanın kayıtları başka bir katmanın kayıtlarıyla anahtarlarıyla eşlenir; bir ilişkiden de kurulur.";
    pub const NO_ROWS: &str = "Görünümde satır yok.";
    pub const NO_RECORD: &str = "Bu satırda birleşen kayıt yok; yazılacak kayıt yok.";
    pub const TOTALS: &str = "Toplam (her kayıt bir kez)";
    /// A view's mark in Katman ▾.
    pub const MARK: &str = "⋈";
}

/// The view read, by the drawing (session, changes), the tab's version and the view.
type Key = (u64, u64, u64, String);

/// The join view's state in the tab.
#[derive(Default)]
pub(crate) struct JoinPane {
    /// The view shown instead of a layer.
    pub view: Option<String>,
    /// The cell edited (the row's id, the column) and its text.
    editing: Option<(String, usize)>,
    text: String,
    cache: RefCell<Option<(Key, Rc<JoinRead>)>>,
}

fn msg(e: Event) -> Message {
    Message::Features(e)
}

/// A persistent id as text read back.
fn uid_of(text: &str) -> Option<Uuid> {
    text.parse().ok()
}

impl JoinPane {
    /// The cell being edited, if one is.
    #[cfg(test)]
    pub(crate) fn editing_cell(&self) -> Option<&(String, usize)> {
        self.editing.as_ref()
    }
}

impl App {
    /// The view chosen, while the project has it.
    pub(crate) fn join_view(&self, doc: &Document) -> Option<JoinView> {
        let id = self.features.join.view.as_deref()?;
        doc.model
            .settings()
            .join_views
            .iter()
            .find(|v| v.id == id)
            .cloned()
    }

    /// The chosen view read from the drawing as it is.
    pub(crate) fn join_read(&self, doc: &Document, v: &JoinView) -> Rc<JoinRead> {
        let key = (
            doc.session,
            doc.model.generation(),
            self.features.version,
            v.id.clone(),
        );
        if let Some((known, read)) = self.features.join.cache.borrow().as_ref()
            && *known == key
        {
            return read.clone();
        }
        let r = Rc::new(read(&doc.model, v));
        *self.features.join.cache.borrow_mut() = Some((key, r.clone()));
        r
    }

    /// The report as one line.
    fn join_report(r: &JoinRead, with: &str) -> String {
        let p = &r.report;
        let mut out = format!(
            "{} satır · {} kayıt: {} eşleşen, {} eşleşmeyen",
            p.rows, p.records, p.matched, p.unmatched
        );
        if p.null_keys > 0 {
            out.push_str(&format!(" ({} anahtarsız)", p.null_keys));
        }
        out.push_str(&format!(
            " · {with}: {} kayıttan {}'i eşleşti",
            p.with_records, p.with_used
        ));
        if !p.cardinality.is_empty() {
            out.push_str(&format!(" · {}", p.cardinality));
        }
        out
    }

    /// The Tablo tab on a join view: the bar, the rows and the sums.
    pub(crate) fn joins_tab<'a>(
        &'a self,
        doc: &'a Document,
        v: JoinView,
        bar: Element<'a, Message>,
    ) -> Element<'a, Message> {
        let r = self.join_read(doc, &v);
        let with = doc
            .model
            .layers()
            .get(&v.with)
            .map_or_else(|| v.with.clone(), |n| n.name.clone());
        let views = tip(
            button(
                row![
                    icon(from_web(Some("joinViews"))).size(14.0),
                    label::body(words::VIEWS)
                ]
                .spacing(6)
                .align_y(Center),
            )
            .style(style::button::secondary)
            .padding([4, 10])
            .on_press(Message::Run("data.joinViews")),
            Tip::new(words::VIEWS).body(words::VIEWS_HINT),
            iced::widget::tooltip::Position::Top,
        );
        let sums: Vec<String> = r
            .totals
            .iter()
            .filter_map(|t| {
                let name = &r.columns.get(t.column as usize)?.name;
                let sum = t.sum.clone()?;
                // A column whose values are mostly not numbers is not summed for the eye.
                (t.skipped == 0).then(|| format!("{name} {sum}"))
            })
            .collect();
        let info: Element<'_, Message> = match &r.broken {
            Some(why) => label::body(why.clone()).style(style::text::warning).into(),
            None if sums.is_empty() => label::muted(Self::join_report(&r, &with)).into(),
            None => tip(
                label::muted(format!(
                    "{} · {}: {}",
                    Self::join_report(&r, &with),
                    words::TOTALS,
                    sums.join(" · ")
                )),
                Tip::new(words::TOTALS).body(
                    "Bir kaydı birden çok satırda olan görünümde her kayıt toplama bir kez girer.",
                ),
                iced::widget::tooltip::Position::Top,
            ),
        };
        let head = container(
            column![row![bar, space::horizontal(), views].align_y(Center), info].spacing(6),
        )
        .padding([6, 10])
        .width(Fill);
        if r.broken.is_some() {
            return column![head, kentos_ui::widget::horizontal_divider()]
                .width(Fill)
                .height(Fill)
                .into();
        }
        let mut columns = vec![
            TableColumn::new("Sıra")
                .width(Length::Fixed(56.0))
                .align_right(),
        ];
        for c in &r.columns {
            columns.push(TableColumn::new(c.name.clone()).width(Length::Fixed(140.0)));
        }
        let min_width = 56.0 + 148.0 * r.columns.len() as f32;
        let read_rows = r.clone();
        let editing = self.features.join.editing.clone();
        let text = self.features.join.text.clone();
        let count = r.rows.len();
        let table = Table::new(columns)
            .horizontal()
            .min_width(min_width)
            .virtualized(count, move |i| {
                let line = &read_rows.rows[i];
                let mut cells: Vec<Element<'_, Message>> =
                    vec![label::muted((i + 1).to_string()).into()];
                for (j, c) in read_rows.columns.iter().enumerate() {
                    if editing
                        .as_ref()
                        .is_some_and(|(id, k)| *id == line.id && *k == j)
                    {
                        cells.push(cell::edit_box(
                            &text,
                            false,
                            |t| msg(Event::JoinInput(t)),
                            msg(Event::JoinEnter),
                            msg(Event::JoinCancel),
                            msg(Event::JoinCancel),
                        ));
                        continue;
                    }
                    let value = line.cells[j]
                        .clone()
                        .unwrap_or_else(|| texts::EMPTY.to_owned());
                    let reason = c.reason.clone().or_else(|| {
                        (c.side == JoinSide::Right && line.right.is_none())
                            .then(|| words::NO_RECORD.to_owned())
                    });
                    let face: Element<'_, Message> = match &reason {
                        Some(_) => label::muted(value).wrapping(Wrapping::None).into(),
                        None => label::body(value).wrapping(Wrapping::None).into(),
                    };
                    cells.push(match reason {
                        Some(why) => tip(
                            face,
                            Tip::new(c.name.clone()).body(why),
                            iced::widget::tooltip::Position::Top,
                        ),
                        None => mouse_area(face)
                            .on_double_click(msg(Event::JoinEdit(i, j)))
                            .into(),
                    });
                }
                TableLine::new(cells).on_press(msg(Event::JoinPress(i)))
            })
            .empty(words::NO_ROWS);
        column![head, kentos_ui::widget::horizontal_divider(), table]
            .width(Fill)
            .height(Fill)
            .into()
    }

    /// A join view's events.
    pub(crate) fn join_event(&mut self, e: Event) -> Task<Message> {
        let Some(doc) = &self.document else {
            return Task::none();
        };
        let Some(v) = self.join_view(doc) else {
            return Task::none();
        };
        let r = self.join_read(doc, &v);
        match e {
            Event::JoinPress(i) => {
                let Some(line) = r.rows.get(i) else {
                    return Task::none();
                };
                let slots: Vec<Slot> = [Some(line.left.as_str()), line.right.as_deref()]
                    .into_iter()
                    .flatten()
                    .filter_map(uid_of)
                    .filter_map(|u| doc.model.slot_of(u))
                    .collect();
                self.selection.set(slots);
            }
            Event::JoinEdit(i, j) => {
                let (Some(line), Some(c)) = (r.rows.get(i), r.columns.get(j)) else {
                    return Task::none();
                };
                if !c.editable || (c.side == JoinSide::Right && line.right.is_none()) {
                    return Task::none();
                }
                self.features.join.text = line.cells[j].clone().unwrap_or_default();
                self.features.join.editing = Some((line.id.clone(), j));
                return focus_field();
            }
            Event::JoinInput(t) => self.features.join.text = t,
            Event::JoinCancel => self.features.join.editing = None,
            Event::JoinEnter => {
                let Some((id, j)) = self.features.join.editing.take() else {
                    return Task::none();
                };
                let text = std::mem::take(&mut self.features.join.text);
                let (Some(line), Some(c)) = (r.rows.iter().find(|l| l.id == id), r.columns.get(j))
                else {
                    return Task::none();
                };
                // The record the cell belongs to: the row's left or right.
                let owner = match c.side {
                    JoinSide::Left => Some(line.left.clone()),
                    JoinSide::Right => line.right.clone(),
                };
                let Some(owner) = owner else {
                    return Task::none();
                };
                let value = (!js_trim(&text).is_empty()).then(|| text.clone());
                let input = EntitiesSetProperties {
                    line_pattern: None,
                    line_type: None,
                    line_type_scale: None,
                    transparency: None,
                    wipeout: None,
                    uids: vec![owner],
                    layer_id: None,
                    color: None,
                    line_weight: None,
                    symbol: None,
                    attrs: Some([(c.field.clone(), value)].into_iter().collect()),
                    label: None,
                    unlink: false,
                    operation: PropertiesOperation::Attributes,
                    expected_revision: None,
                    attachments: None,
                };
                let Some(doc) = self.document.as_mut() else {
                    return Task::none();
                };
                if let Err(message) = super::write_attribute(&mut doc.model, input) {
                    self.warn(message);
                    self.features.join.editing = Some((id, j));
                    self.features.join.text = text;
                    return focus_field();
                }
                self.features.changed();
            }
            _ => {}
        }
        Task::none()
    }
}
