//! Öznitelikler: the web's properties panel (`ui/properties/PropertiesPanel.ts`
//! on `ui/widgets/PropertyGrid.ts`), editable as the web's (docs/adr/0063).
//!
//! - Nothing selected: “Seçili nesne yok”, how to select, and the drawing's
//!   summary. One object: its kind, label and layer over the grid, “#id” in
//!   the header; Genel, Geometri and its attributes. Several: how many of
//!   which kinds, “N nesne”; what they share and their totals.
//! - A text or number cell looks like text until clicked: Enter or leaving
//!   it commits, Esc keeps the value (KentOS UI's `EditCell`). A value the
//!   panel does not take (an empty text, not a number, a height not above
//!   zero) leaves the drawing as it was, and the cell shows its value again.
//! - Katman ▾, Renk ▾ and Sembol ▾ are drop-downs; an object on a locked
//!   layer, or a selection holding one, is not edited, its values still named.
//! - Every edit is one undo step with the web's name: “Katman değiştir”,
//!   “Renk değiştir”, else “Değiştir”. Nothing is said, but for objects
//!   moved to a hidden layer.
//! - Sections close on their header and stay closed while the app runs.
//!
//! The rows are data (`rows`), drawn by KentOS UI's `PropertySheet`.

mod rows;
#[cfg(test)]
mod tests;

use iced::widget::{Column, column, container, row, space};
use iced::{Center, Color, Element, Fill, Theme};
use kentos_contracts::{Entity, HatchPatternType};
use kentos_domain::Slot;
use kentos_ui::label;
use kentos_ui::widget::property_grid::{self, PropertySheet};
use kentos_ui::widget::{EditCell, Menu, swatch};

pub(crate) use rows::{Choice, Editor, Panel, Row, Summary};

use crate::app::{App, Message};
use crate::document::Document;

/// A change asked for in the panel.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// A section's header: closes or opens it.
    Toggle(&'static str),
    /// What a text or number cell committed.
    Commit(Field, String),
    /// Katman ▾: the objects to the layer.
    Layer(Vec<Slot>, String),
    /// Renk ▾: the objects' colour; none, the layer's.
    Color(Vec<Slot>, Option<String>),
    /// Desen ▾ of a hatch.
    Pattern(Slot, HatchPatternType),
}

/// The value a cell edits.
#[derive(Debug, Clone, PartialEq)]
pub enum Field {
    /// A point's Y (east) and X (north).
    PointX(Slot),
    PointY(Slot),
    DimensionOffset(Slot),
    DimensionHeight(Slot),
    /// A dimension's own text; empty, the measured value again.
    DimensionText(Slot),
    HatchAngle(Slot),
    HatchSpacing(Slot),
    /// A text object's text, height and angle.
    Text(Slot),
    TextHeight(Slot),
    TextAngle(Slot),
    /// An attribute, by its key.
    Attribute(Slot, String),
}

/// JavaScript's `parseFloat(text.replace(',', '.'))`, as the web reads the
/// panel's numbers: the first comma is the decimal point, the longest number
/// at the start is taken (“12abc” is 12), none is NaN.
pub(crate) fn web_number(text: &str) -> f64 {
    let text = kentos_interaction::js_trim(text).replacen(',', ".", 1);
    let bytes = text.as_bytes();
    let mut end = 0;
    if matches!(bytes.first(), Some(b'+' | b'-')) {
        end = 1;
    }
    if text[end..].starts_with("Infinity") {
        return text[..end + "Infinity".len()]
            .replace("Infinity", "inf")
            .parse()
            .unwrap_or(f64::NAN);
    }
    let digits = |from: usize| {
        bytes[from..]
            .iter()
            .take_while(|b| b.is_ascii_digit())
            .count()
    };
    let whole = digits(end);
    end += whole;
    let mut fraction = 0;
    if bytes.get(end) == Some(&b'.') {
        fraction = digits(end + 1);
        if whole > 0 || fraction > 0 {
            end += 1 + fraction;
        }
    }
    if whole == 0 && fraction == 0 {
        return f64::NAN;
    }
    if matches!(bytes.get(end), Some(b'e' | b'E')) {
        let sign = usize::from(matches!(bytes.get(end + 1), Some(b'+' | b'-')));
        let exponent = digits(end + 1 + sign);
        if exponent > 0 {
            end += 1 + sign + exponent;
        }
    }
    text[..end].parse().unwrap_or(f64::NAN)
}

impl App {
    /// The panel's content for the selection now.
    pub(crate) fn properties_panel(&self, doc: &Document) -> Panel {
        let selected: Vec<Slot> = self.selection.ids().to_vec();
        rows::panel(doc, &selected, |ids| {
            // Summed by the geometry store, as the web sums them: a selection can be large.
            let ids: Vec<f64> = ids.iter().map(|s| f64::from(s.0)).collect();
            self.spatial.store().measure(&ids)
        })
    }

    /// The header's meta: “#12”, “3 nesne”; none with nothing selected.
    pub(crate) fn properties_meta(&self, doc: &Document) -> Option<String> {
        self.properties_panel(doc).meta
    }

    /// The panel's body: the summary or the empty state, then the grid.
    pub(crate) fn properties_view<'a>(&'a self, doc: &'a Document) -> Element<'a, Message> {
        let panel = self.properties_panel(doc);
        let head: Element<'a, Message> = match panel.summary {
            Summary::Empty => boxed(
                column![
                    label::strong("Seçili nesne yok"),
                    label::muted(
                        "Özelliklerini görmek için çizimde bir nesneye tıklayın. Birden fazla nesne için sürükleyerek seçin."
                    ),
                ]
                .spacing(4),
                [14, 14],
            ),
            Summary::One { kind, label, layer } => {
                let mut title = row![label::strong(kind)].spacing(8).align_y(Center);
                if let Some(text) = label {
                    title = title.push(label::body(text).style(accent_text));
                }
                let mut lines = column![title].spacing(3);
                if let Some((color, path)) = layer {
                    lines = lines.push(
                        row![swatch(self.drawing_color(&color)), label::caption(path)]
                            .spacing(6)
                            .align_y(Center),
                    );
                }
                boxed(lines, [9, 12])
            }
            Summary::Many { count, kinds } => boxed(
                column![
                    label::strong(format!("{count} nesne seçili")),
                    label::caption(kinds),
                ]
                .spacing(3),
                [9, 12],
            ),
        };
        let mut sheet = PropertySheet::new();
        for section in panel.sections {
            let open = !self.props_closed.contains(section.id);
            sheet = sheet.section(
                section.title,
                open,
                Message::Properties(Event::Toggle(section.id)),
            );
            if !open {
                continue;
            }
            for row in section.rows {
                let label = row.label.clone();
                sheet = sheet.row(label, self.cell(row));
            }
        }
        Column::new().push(head).push(sheet).width(Fill).into()
    }

    /// A row's value: text, an editable cell or a drop-down.
    fn cell<'a>(&'a self, row: Row) -> Element<'a, Message> {
        let unit = row.unit.clone();
        match row.editor {
            None => property_grid::value(row.value, row.numeric, unit.map(|u| u.into_owned())),
            Some(Editor::Text(field) | Editor::Number(field)) => {
                EditCell::new(row.value, move |text| {
                    Message::Properties(Event::Commit(field.clone(), text))
                })
                .numeric(row.numeric)
                .unit(unit.map(|u| u.into_owned()).unwrap_or_default())
                .into()
            }
            Some(Editor::Select {
                text,
                swatch,
                items,
            }) => {
                // Colours resolved now: the menu is built again at every opening.
                let items: Vec<(Choice, Option<Color>)> = items
                    .into_iter()
                    .map(|choice| {
                        let color = match &choice {
                            Choice::Pick {
                                swatch: Some(value),
                                ..
                            } => Some(self.drawing_color(value)),
                            _ => None,
                        };
                        (choice, color)
                    })
                    .collect();
                property_grid::choice(
                    text,
                    swatch.map(|value| self.drawing_color(&value)),
                    move || menu(&items),
                )
            }
        }
    }

    /// A drawing colour as the panel shows it (a layer's, one of the
    /// colours): the theme's ink colours by the drawing's palette, else the hex.
    pub(crate) fn drawing_color(&self, value: &str) -> Color {
        let palette = crate::viewport::palette(self.canvas());
        palette
            .resolve(value)
            .map_or_else(|| crate::view::hex_color(value), crate::view::rgba_color)
    }

    /// What the panel asked for: an edit of the drawing, or a section toggled.
    pub(crate) fn properties_event(&mut self, event: Event) {
        if let Event::Toggle(id) = event {
            if !self.props_closed.remove(id) {
                self.props_closed.insert(id);
            }
            return;
        }
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let model = &mut doc.model;
        match event {
            Event::Toggle(_) => {}
            Event::Layer(ids, layer) => {
                let changes: Vec<(Slot, Entity)> = ids
                    .iter()
                    .filter_map(|slot| {
                        let mut e = model.get(*slot)?.clone();
                        e.base_mut().layer_id = layer.clone();
                        Some((*slot, e))
                    })
                    .collect();
                let moved = model.update_many(changes, "Katman değiştir");
                // On a hidden layer they vanish from the drawing, still selected:
                // said, as the tools say it when they draw there.
                if moved > 0 && !model.layers().is_visible(&layer) {
                    let name = model
                        .layers()
                        .get(&layer)
                        .map_or(layer.clone(), |n| n.name.clone());
                    self.warn(format!(
                        "“{name}” katmanı gizli; taşınan nesneler görünmeyecek."
                    ));
                }
            }
            Event::Color(ids, color) => {
                let changes: Vec<(Slot, Entity)> = ids
                    .iter()
                    .filter_map(|slot| {
                        let mut e = model.get(*slot)?.clone();
                        e.base_mut().color = color.clone();
                        Some((*slot, e))
                    })
                    .collect();
                model.update_many(changes, "Renk değiştir");
            }
            Event::Pattern(slot, kind) => {
                if let Some(Entity::Hatch(h)) = model.get(slot) {
                    let mut h = h.clone();
                    h.pattern.kind = kind;
                    model.update(slot, Entity::Hatch(h));
                }
            }
            Event::Commit(field, text) => commit(model, &field, &text),
        }
    }
}

/// A cell's text into the drawing, as the web's editors take it; what they
/// do not take changes nothing.
fn commit(model: &mut kentos_domain::Document, field: &Field, text: &str) {
    let n = web_number(text);
    let finite = n.is_finite();
    let slot = match field {
        Field::PointX(s)
        | Field::PointY(s)
        | Field::DimensionOffset(s)
        | Field::DimensionHeight(s)
        | Field::DimensionText(s)
        | Field::HatchAngle(s)
        | Field::HatchSpacing(s)
        | Field::Text(s)
        | Field::TextHeight(s)
        | Field::TextAngle(s)
        | Field::Attribute(s, _) => *s,
    };
    let Some(mut e) = model.get(slot).cloned() else {
        return;
    };
    let taken = match (field, &mut e) {
        (Field::PointX(_), Entity::Point(p)) if finite => {
            p.p.x = n;
            true
        }
        (Field::PointY(_), Entity::Point(p)) if finite => {
            p.p.y = n;
            true
        }
        (Field::DimensionOffset(_), Entity::Dimension(d)) if finite => {
            d.offset = n;
            true
        }
        (Field::DimensionHeight(_), Entity::Dimension(d)) if finite && n > 0.0 => {
            d.height = n;
            true
        }
        (Field::DimensionText(_), Entity::Dimension(d)) => {
            let own = kentos_interaction::js_trim(text);
            d.text = (!own.is_empty()).then(|| own.to_owned());
            true
        }
        (Field::HatchAngle(_), Entity::Hatch(h)) if finite => {
            h.pattern.angle = n;
            true
        }
        (Field::HatchSpacing(_), Entity::Hatch(h)) if finite && n > 0.0 => {
            h.pattern.spacing = n;
            true
        }
        // Trimmed, as the in-place editor stores it; an empty text is not taken.
        (Field::Text(_), Entity::Text(t)) => {
            let body = kentos_interaction::js_trim(text);
            if body.is_empty() {
                false
            } else {
                t.text = body.to_owned();
                true
            }
        }
        (Field::TextHeight(_), Entity::Text(t)) if finite && n > 0.0 => {
            t.height = n;
            true
        }
        (Field::TextAngle(_), Entity::Text(t)) if finite => {
            t.rotation = ((n % 360.0) + 360.0) % 360.0;
            true
        }
        (Field::Attribute(_, key), e) => {
            let base = e.base_mut();
            let old = base.attrs.get(key).cloned();
            // Keep the drawn number in step with the cadastral attribute.
            if (key == "Parsel" || key == "Ada") && base.label.is_some() && base.label == old {
                base.label = Some(text.to_owned());
            }
            base.attrs.insert(key.clone(), text.to_owned());
            true
        }
        _ => false,
    };
    if taken {
        model.update(slot, e);
    }
}

/// The drop-down's menu: its choices, their swatches, the commands.
fn menu(items: &[(Choice, Option<Color>)]) -> Menu<Message> {
    items
        .iter()
        .fold(Menu::new(), |menu, (choice, color)| match choice {
            Choice::Pick {
                label,
                chosen,
                enabled,
                message,
                ..
            } => {
                let menu = menu.radio(label.clone(), *chosen, enabled.then(|| message.clone()));
                match color {
                    Some(color) => menu.swatch(*color),
                    None => menu,
                }
            }
            Choice::Separator => menu.separator(),
            Choice::Command { label, icon, id } => menu.item(*label, Message::Run(id)).icon(*icon),
        })
}

/// The summary's box: its padding, a line under it.
fn boxed<'a>(content: impl Into<Element<'a, Message>>, padding: [u16; 2]) -> Element<'a, Message> {
    column![
        container(content).padding(padding).width(Fill),
        container(space::horizontal())
            .width(Fill)
            .height(1)
            .style(|theme: &Theme| container::Style {
                background: Some(kentos_ui::theme::Tokens::of(theme).border.into()),
                ..container::Style::default()
            }),
    ]
    .into()
}

/// The object's label in the summary: the accent's text colour (a parcel's number).
fn accent_text(theme: &Theme) -> iced::widget::text::Style {
    iced::widget::text::Style {
        color: Some(kentos_ui::theme::Tokens::of(theme).accent),
    }
}
