//! The rollover card (the web's `ui/shell/HoverCard.ts`, docs/adr/0068):
//! resting the pointer on an object while no command runs shows what it is
//! (kind, layer, length or area, parcel data) without selecting it.
//!
//! - It waits half a second on the same object, then follows the pointer
//!   18 px right and 20 px down. Near the drawing's right edge it goes left
//!   of the pointer, near its bottom above it, and it stays 8 px inside the
//!   drawing (`kentos_ui::widget::beside`, the web's `besidePointer`).
//! - It is 160 to 280 px × the type scale wide, and 16 px narrower than the
//!   drawing. Long names and texts wrap, inside a word when they must. The
//!   layer goes under the kind when both do not fit on one line and wraps
//!   there, its swatch on the first line. Row labels stay on one line; values
//!   wrap, right-aligned (DESIGN.md §7.4.2; the web's 5c1cf5a).
//! - It goes when the pointer leaves the object or the drawing, when a
//!   command starts or a grip is taken, and never shows while the
//!   preference `drafting.hoverInfo` is off.
//! - Its rows are the web's, in its order: Ada, Mahalle, Nitelik; the deed
//!   area as the deed says it beside the computed one (“Hesaplanan alan”),
//!   since their difference is what a surveyor checks; holes; perimeter or
//!   length; radius; a text's text; a point's elevation.

use std::time::Duration;

use iced::widget::text::Wrapping;
use iced::widget::{container, row};
use iced::{Center, Color, Element, Padding, Point, Right, Vector};
use kentos_contracts::Entity;
use kentos_domain::Document as Model;
use kentos_interaction::{Format, measures};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::typography;
use kentos_ui::widget::{InfoCard, Pairs, beside, swatch};

use crate::app::{App, Message};
use crate::selecting::kind_title;

/// How long the pointer rests on an object before the card shows (the web's `DELAY_MS`).
pub(crate) const DELAY: Duration = Duration::from_millis(500);

/// The card's place from the pointer where there is room: right and down.
const GAP: Vector = Vector::new(18.0, 20.0);
/// The card's least and most width with its padding, at the default text size.
const MIN_WIDTH: f32 = 160.0;
const MAX_WIDTH: f32 = 280.0;
/// Its padding left and right.
const PAD_X: f32 = 10.0;

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

/// The card as the web draws it: the title with the layer right of it or
/// under it, then the rows, labels muted on the left, values right-aligned,
/// on a popover (see the module comment for its widths).
pub(crate) fn view<'a>(card: Card) -> Element<'a, Message> {
    let size = typography::caption();
    let title = label::strong(card.title)
        .size(size)
        .wrapping(Wrapping::WordOrGlyph);
    let mut info =
        InfoCard::new(title).min_width(typography::from_default(MIN_WIDTH) - 2.0 * PAD_X);
    if let Some((name, color)) = card.layer {
        // The swatch on the name's first line, however many lines it takes.
        let first_line = (typography::caption() * 1.3).round();
        info = info.tag(
            row![
                container(swatch(color)).height(first_line).align_y(Center),
                label::caption(name).wrapping(Wrapping::WordOrGlyph),
            ]
            .spacing(5),
        );
    }
    if !card.rows.is_empty() {
        // The web's grid (`auto 1fr`): a long value wraps, even inside a
        // word, and its row grows; the names stay beside their values.
        let rows = card.rows.into_iter().fold(
            Pairs::new()
                .spacing_x(12.0)
                .spacing_y(2.0)
                .align_values(Right),
            |rows, (name, value, numeric)| {
                let text = if numeric {
                    label::mono(value)
                } else {
                    label::body(value)
                };
                let text = text
                    .size(size)
                    .align_x(Right)
                    .wrapping(Wrapping::WordOrGlyph);
                rows.push(label::caption(name).wrapping(Wrapping::None), text)
            },
        );
        info = info.body(rows);
    }
    container(info)
        .max_width(typography::from_default(MAX_WIDTH))
        .padding(Padding {
            top: 7.0,
            right: PAD_X,
            bottom: 8.0,
            left: PAD_X,
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

    /// The card over the drawing, when one shows: beside the pointer, on
    /// its other side near the drawing's edges.
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
        let pointer = Point::new(x.round() as f32, y.round() as f32);
        Some(beside(view(card), pointer, GAP))
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
        // On by default; turned off in Uygulama ayarları (Nesne bilgi kartı), no card.
        assert!(app.hover_info);
        let _ = app
            .settings
            .choose(&[("drafting.hoverInfo", serde_json::Value::from(false))]);
        app.apply_settings();
        assert!(!app.hover_info);
        app.hover_card_due(app.selection.hover_version());
        assert_eq!(app.hover_card, None);
        let _ = app
            .settings
            .choose(&[("drafting.hoverInfo", serde_json::Value::from(true))]);
        app.apply_settings();
        assert!(app.hover_info);
        // A command running: no card either.
        let _ = app.update(Message::Run("tool.line"));
        app.hover_card_due(app.selection.hover_version());
        assert_eq!(app.hover_card, None);
    }
}

/// The grips of a selected parcel with one being moved, and the rollover
/// card over it: as it is, with a layer named as long as the MPYY showcase's
/// (the web's `hover-card-long`), and that card at the drawing's bottom
/// right corner (`hover-card-corner`), the last two with large text too
/// (`-buyuk`); and the value field beside the cursor, in the middle and at
/// the drawing's top right corner (`deger-alani`). Not run by default:
/// `cargo test -p kentos-desktop hover_card::screens -- --ignored --nocapture`.
#[cfg(test)]
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::{Point, Size};
    use kentos_domain::Slot;
    use kentos_ui::snapshot::Snapshot;

    use crate::viewport::Event;

    // Large text is set for some: no other test lays out meanwhile.
    let _typography = crate::appearance::tests::TYPOGRAPHY.lock();
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    const LONG: &str = "Ortak gösterimler · Korunacak alanlar · Bugünkü arazi kullanımı devam ettirilerek korunacak alanlar";
    let cases = [
        ("tutamac", 13),
        ("uzerine-gelme", 13),
        ("uzerine-gelme-uzun", 13),
        ("uzerine-gelme-kose", 13),
        ("uzerine-gelme-uzun-buyuk", 16),
        ("uzerine-gelme-kose-buyuk", 16),
        ("deger-alani", 13),
        ("deger-alani-kose", 13),
    ];
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for (name, text_size) in cases {
                let mut app = crate::files_testing::app_with_drawing();
                let _ = app.settings.choose(&[
                    ("appearance.theme", serde_json::Value::from(mode)),
                    ("appearance.textSize", serde_json::Value::from(text_size)),
                ]);
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
                } else if name.starts_with("deger-alani") {
                    // Çizgi running, a value typed with the pointer resting.
                    let _ = app.update(Message::Run("tool.line"));
                    let camera = app.viewport.camera;
                    let (x, y) = if name == "deger-alani-kose" {
                        (camera.width - 20.0, 24.0)
                    } else {
                        (camera.width / 2.0, camera.height / 2.0)
                    };
                    let at = Point::new(x as f32, y as f32);
                    let _ = app.update(Message::Viewport(Event::Moved(at)));
                    app.field = Some(crate::input::Field {
                        text: "@25.50<45".into(),
                        at: camera.screen_to_world(x, y),
                    });
                } else {
                    if name != "uzerine-gelme" {
                        let doc = app.document.as_mut().expect("open");
                        doc.model.rename_layer("parsel", LONG);
                    }
                    // The pointer resting inside the parcel.
                    let doc = app.document.as_ref().expect("open");
                    let Some(kentos_contracts::Entity::Polygon(p)) = doc.model.get(parcel) else {
                        panic!("the parcel");
                    };
                    let inside = kentos_interaction::Vec2::new(
                        p.pts.iter().map(|q| q.x).sum::<f64>() / p.pts.len() as f64 + 6.0,
                        p.pts.iter().map(|q| q.y).sum::<f64>() / p.pts.len() as f64 + 8.0,
                    );
                    if name.starts_with("uzerine-gelme-kose") {
                        // The drawing moved so that point is 24 px from its bottom right corner.
                        let camera = &mut app.viewport.camera;
                        let (x, y) = (camera.width - 24.0, camera.height - 24.0);
                        camera.center = kentos_interaction::Vec2::new(
                            inside.x - (x - camera.width / 2.0) / camera.scale,
                            inside.y + (y - camera.height / 2.0) / camera.scale,
                        );
                    }
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
