//! The rollover card (the web's `ui/shell/HoverCard.ts`, docs/adr/0068):
//! resting the pointer on an object while no command runs shows what it is
//! (kind, layer, length or area, parcel data) without selecting it.
//!
//! - It waits half a second on the same object, then follows the pointer
//!   18 px right and 20 px down.
//! - It goes when the pointer leaves the object or the drawing, when a
//!   command starts or a grip is taken, and never shows while the
//!   preference `drafting.hoverInfo` is off.
//! - Its rows are the web's, in its order: Ada, Mahalle, Nitelik; the deed
//!   area as the deed says it beside the computed one (“Hesaplanan alan”),
//!   since their difference is what a surveyor checks; holes; perimeter or
//!   length; radius; a text's text; a point's elevation.

use std::time::Duration;

use iced::widget::{column, container, row, space};
use iced::{Center, Color, Element, Length, Padding, Right};
use kentos_contracts::Entity;
use kentos_domain::Document as Model;
use kentos_interaction::{Format, measures};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::typography;
use kentos_ui::widget::{horizontal_divider, swatch};

use crate::app::{App, Message};
use crate::selecting::kind_title;

/// How long the pointer rests on an object before the card shows (the web's `DELAY_MS`).
pub(crate) const DELAY: Duration = Duration::from_millis(500);

/// `message` after `delay`, from a thread of its own (as the recovery ticks run).
pub(crate) fn after(delay: Duration, message: Message) -> iced::Task<Message> {
    let (done, wait) = iced::futures::channel::oneshot::channel();
    std::thread::spawn(move || {
        std::thread::sleep(delay);
        let _ = done.send(message);
    });
    iced::Task::perform(wait, |message| message.ok()).and_then(iced::Task::done)
}

/// The registered (tapu) area as the card shows it (the web's
/// `deedAreaText`): the attribute as written, not parsed, rounded or
/// converted, since it is the deed's value (CLAUDE.md §7, §23.1), with “m²”
/// after a plain decimal number (a point or a comma); none when empty.
pub(crate) fn deed_area_text(text: Option<&str>) -> Option<(String, bool)> {
    let text = kentos_interaction::js_trim(text.unwrap_or(""));
    if text.is_empty() {
        return None;
    }
    let plain = plain_decimal(text);
    Some((
        if plain {
            format!("{text} m²")
        } else {
            text.to_owned()
        },
        plain,
    ))
}

/// Digits, then at most one point or comma and more digits (the web's `^\d+([.,]\d+)?$`).
fn plain_decimal(text: &str) -> bool {
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    match text.split_once(['.', ',']) {
        Some((whole, fraction)) => digits(whole) && digits(fraction),
        None => digits(text),
    }
}

/// What a card shows: its title, the object's layer and colour, its rows
/// (label, value, whether the value is a number).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Card {
    pub title: String,
    pub layer: Option<(String, Color)>,
    pub rows: Vec<(&'static str, String, bool)>,
}

/// The web's `content`: the card for `e`.
pub(crate) fn card(
    e: &Entity,
    model: &Model,
    format: &Format,
    color: impl Fn(&str) -> Color,
) -> Card {
    let base = e.base();
    let attr = |key: &str| {
        base.attrs
            .get(key)
            .filter(|v| !v.is_empty())
            .map(String::as_str)
    };
    let kind = kind_title(e.kind());
    let title = match (attr("Parsel"), base.label.as_deref().filter(|l| !l.is_empty())) {
        (Some(number), _) => format!("Parsel {number}"),
        (None, Some(label)) => format!("{kind} {label}"),
        (None, None) => kind.to_owned(),
    };
    let layer = model.layers().get(&base.layer_id).map(|node| {
        let own = base.color.as_deref().unwrap_or(&node.style.color);
        (node.name.clone(), color(own))
    });
    let mut rows: Vec<(&'static str, String, bool)> = Vec::new();
    for key in ["Ada", "Mahalle", "Nitelik"] {
        if let Some(value) = attr(key) {
            rows.push((key, value.to_owned(), false));
        }
    }
    // The registered (deed) area as written, beside the computed one.
    let deed = deed_area_text(base.attrs.get("Tapu alanı (m²)").map(String::as_str));
    let has_deed = deed.is_some();
    if let Some((text, plain)) = deed {
        rows.push(("Tapu alanı", text, plain));
    }
    let (area, length) = measures(e);
    if let Some(area) = area {
        let name = if has_deed {
            "Hesaplanan alan"
        } else {
            "Alan"
        };
        rows.push((name, format.area(area), true));
    }
    if let Entity::Polygon(p) = e
        && let Some(holes) = p.holes.as_ref().filter(|h| !h.is_empty())
    {
        rows.push(("Ada (delik)", holes.len().to_string(), true));
    }
    if let Some(length) = length {
        let name = if matches!(e, Entity::Polygon(_) | Entity::Circle(_)) {
            "Çevre"
        } else {
            "Uzunluk"
        };
        rows.push((name, format.length(length), true));
    }
    match e {
        Entity::Circle(c) => rows.push(("Yarıçap", format.length(c.r), true)),
        Entity::Arc(a) => rows.push(("Yarıçap", format.length(a.r), true)),
        Entity::Text(t) => rows.push(("Metin", t.text.clone(), false)),
        Entity::Point(p) => {
            if let Some(z) = p.z {
                rows.push(("Kot", format.length(z), true));
            }
        }
        _ => {}
    }
    Card { title, layer, rows }
}

/// The card as the web draws it: the title and the layer over the rows,
/// labels muted on the left, values right-aligned, on a popover. Its rows
/// keep their own width (a row that fills would collapse in a card that
/// shrinks to its content); 160 px at least, 280 at most, as on the web.
pub(crate) fn view<'a>(card: Card) -> Element<'a, Message> {
    let size = typography::caption();
    let mut head = row![label::strong(card.title).size(size)]
        .spacing(12)
        .align_y(Center);
    if let Some((name, color)) = card.layer {
        head = head.push(
            row![swatch(color), label::caption(name)]
                .spacing(5)
                .align_y(Center),
        );
    }
    // The least width, less the padding. A space of no height would be left out.
    let mut body = column![head, space().width(140)];
    if !card.rows.is_empty() {
        let names = column(
            card.rows
                .iter()
                .map(|(name, _, _)| label::caption(*name).into()),
        )
        .spacing(2);
        let values = column(card.rows.into_iter().map(|(_, value, numeric)| {
            let text = if numeric {
                label::mono(value)
            } else {
                label::body(value)
            };
            text.size(size).into()
        }))
        .spacing(2)
        .align_x(Right);
        // A line between the head and the rows, 6 px from each (the web's border-top).
        body = body
            .push(space().height(6))
            .push(horizontal_divider())
            .push(space().height(6))
            .push(row![names, values].spacing(12));
    }
    container(body.width(Length::Shrink).max_width(260))
        .padding(Padding {
            top: 7.0,
            right: 10.0,
            bottom: 8.0,
            left: 10.0,
        })
        .style(style::container::popover)
        .into()
}

impl App {
    /// After every message: a new hover starts the card's wait, and the card
    /// it showed goes (the web's `watchAll` on the hover).
    pub(crate) fn follow_hover(&mut self) -> iced::Task<Message> {
        let version = self.selection.hover_version();
        if version == self.hover_seen {
            return iced::Task::none();
        }
        self.hover_seen = version;
        self.hover_card = None;
        let idle = !self.session.is_running() && !self.session.grip_active();
        if self.hover_info && idle && self.selection.hover().is_some() {
            return after(DELAY, Message::HoverCard(version));
        }
        iced::Task::none()
    }

    /// The wait is over: the card shows if the pointer still rests on the
    /// same object and nothing runs.
    pub(crate) fn hover_card_due(&mut self, version: u64) {
        let idle = !self.session.is_running() && !self.session.grip_active();
        if version == self.selection.hover_version() && self.hover_info && idle {
            self.hover_card = self.selection.hover();
        }
    }

    /// The card over the drawing, when one shows: beside the pointer.
    pub(crate) fn hover_card_view(&self) -> Option<Element<'_, Message>> {
        let slot = self.hover_card?;
        if self.session.is_running() || self.session.grip_active() {
            return None;
        }
        let doc = self.document.as_ref()?;
        let e = doc.model.get(slot)?;
        let at = self.viewport.cursor?;
        let [x, y] = self.viewport.camera.world_to_screen(at);
        let format = Format::of(doc.settings());
        let card = card(e, &doc.model, &format, |value| self.drawing_color(value));
        Some(
            iced::widget::pin(view(card))
                .x(x.round() as f32 + 18.0)
                .y(y.round() as f32 + 20.0)
                .width(Length::Fill)
                .height(Length::Fill)
                .into(),
        )
    }
}

#[cfg(test)]
mod tests {
    use iced::Color;
    use kentos_contracts::Entity;
    use kentos_domain::Slot;
    use kentos_interaction::Format;

    use super::card;
    use crate::app::Message;
    use crate::files_testing::app_with_drawing;

    /// The demo drawing's parcel 4 (Ada 101, Parsel 7, one hole, on Parsel)
    /// and circle 5 (radius 3.25 m, on Bina).
    #[test]
    fn the_rows_are_the_webs_in_its_order() {
        let app = app_with_drawing();
        let doc = app.document.as_ref().expect("open");
        let format = Format::of(doc.settings());
        let black = |_: &str| Color::BLACK;
        let parcel = doc.model.get(Slot(4)).expect("the parcel");
        let c = card(parcel, &doc.model, &format, black);
        assert_eq!(c.title, "Parsel 7");
        assert_eq!(c.layer.as_ref().map(|(n, _)| n.as_str()), Some("Parsel"));
        let names: Vec<&str> = c.rows.iter().map(|r| r.0).collect();
        assert_eq!(names, ["Ada", "Alan", "Ada (delik)", "Çevre"]);
        assert_eq!(c.rows[0].1, "101");
        // The deed area as the deed says it (“118,5”, a decimal comma kept), beside the computed one.
        let mut deed = parcel.clone();
        deed.base_mut()
            .attrs
            .insert("Tapu alanı (m²)".into(), " 118,5 ".into());
        let c = card(&deed, &doc.model, &format, black);
        let rows: Vec<(&str, &str)> = c
            .rows
            .iter()
            .map(|r| (r.0, r.1.as_str()))
            .take(3)
            .collect();
        assert_eq!(rows[1], ("Tapu alanı", "118,5 m²"));
        assert_eq!(rows[2].0, "Hesaplanan alan");
        // Text that is not a plain number is shown as it is.
        assert_eq!(
            super::deed_area_text(Some("tapuda yok")),
            Some(("tapuda yok".to_owned(), false))
        );
        assert_eq!(super::deed_area_text(Some("723.525")), Some(("723.525 m²".to_owned(), true)));
        assert_eq!(super::deed_area_text(Some("723abc")), Some(("723abc".to_owned(), false)));
        assert_eq!(super::deed_area_text(Some("  ")), None);
        let circle = doc.model.get(Slot(5)).expect("the circle");
        let c = card(circle, &doc.model, &format, black);
        assert_eq!(c.title, "Daire");
        let rows: Vec<(&str, &str)> = c.rows.iter().map(|r| (r.0, r.1.as_str())).collect();
        assert_eq!(rows[1].0, "Çevre");
        assert_eq!(rows[2], ("Yarıçap", "3.250 m"));
        let Some(Entity::Circle(_)) = doc.model.get(Slot(5)) else {
            panic!("a circle");
        };
    }

    #[test]
    fn the_card_waits_for_the_same_hover_and_goes_with_it() {
        let mut app = app_with_drawing();
        app.selection.set_hover(Some(Slot(5)));
        let _ = app.follow_hover();
        assert_eq!(app.hover_card, None, "it waits");
        let first = app.selection.hover_version();
        app.hover_card_due(first);
        assert_eq!(app.hover_card, Some(Slot(5)));
        // Another object: the card goes; the first wait no longer counts.
        app.selection.set_hover(Some(Slot(6)));
        let _ = app.follow_hover();
        assert_eq!(app.hover_card, None);
        app.hover_card_due(first);
        assert_eq!(app.hover_card, None);
        // The preference off, or a command running: no card.
        app.hover_info = false;
        app.hover_card_due(app.selection.hover_version());
        assert_eq!(app.hover_card, None);
        app.hover_info = true;
        let _ = app.update(Message::Run("tool.line"));
        app.hover_card_due(app.selection.hover_version());
        assert_eq!(app.hover_card, None);
    }
}

/// The grips of a selected parcel with one being moved, and the rollover
/// card over it. Not run by default:
/// `cargo test -p kentos-desktop hover_card::screens -- --ignored --nocapture`.
#[cfg(test)]
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::{Point, Size};
    use kentos_domain::Slot;
    use kentos_ui::snapshot::Snapshot;

    use crate::viewport::Event;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for name in ["tutamac", "uzerine-gelme"] {
                let mut app = crate::files_testing::app_with_drawing();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                let screen = |app: &App, p: kentos_interaction::Vec2| {
                    let [x, y] = app.viewport.camera.world_to_screen(p);
                    Point::new(x as f32, y as f32)
                };
                let parcel = Slot(4);
                if name == "tutamac" {
                    // The parcel selected, its third corner taken and moved 40 px right, 30 up.
                    app.selection.set([parcel]);
                    let doc = app.document.as_ref().expect("open");
                    app.spatial.sync(&doc.model);
                    let corner = app.spatial.grips(&[parcel])[0].points[2];
                    let at = screen(&app, corner);
                    for event in [Event::Moved(at), Event::Pressed(at), Event::Released(at)] {
                        let _ = app.update(Message::Viewport(event));
                    }
                    let to = Point::new(at.x + 40.0, at.y - 30.0);
                    let _ = app.update(Message::Viewport(Event::Moved(to)));
                } else {
                    // The pointer resting inside the parcel.
                    let doc = app.document.as_ref().expect("open");
                    let Some(kentos_contracts::Entity::Polygon(p)) = doc.model.get(parcel) else {
                        panic!("the parcel");
                    };
                    let inside = kentos_interaction::Vec2::new(
                        p.pts.iter().map(|q| q.x).sum::<f64>() / p.pts.len() as f64 + 6.0,
                        p.pts.iter().map(|q| q.y).sum::<f64>() / p.pts.len() as f64 + 8.0,
                    );
                    let at = screen(&app, inside);
                    let _ = app.update(Message::Viewport(Event::Moved(at)));
                    app.hover_card_due(app.selection.hover_version());
                    assert_eq!(app.hover_card, Some(parcel), "{name}: the card shows");
                }
                snapshot.settle(&mut app, App::view, &mut update);
                let file = out.join(format!("{name}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}
