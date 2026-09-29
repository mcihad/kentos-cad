//! Blok oluştur's window (the web's `ui/blocks/BlockDefineDialog.ts`,
//! docs/adr/0144 §6): the new block's name (the first free “Blok n”
//! offered), what it is, and whether the objects give way to an insert of it
//! on the active layer; the tool (`kentos_interaction::block_define`) has
//! picked the objects and the base point. Every change asks
//! `cad.blocks.define` and shows its refusal (or its warning) under the
//! fields; Oluştur waits until it passes and writes through it, one undo
//! step “Blok tanımla”. The new block is the one Blok ekle places next; the
//! insert that took the objects' place is selected.

use iced::widget::{Column, Id, text, text_input};
use iced::{Element, Task};
use kentos_contracts::BlocksDefine;
use kentos_domain::Slot;
use kentos_interaction::{Format, Level, Vec2, block_define};
use kentos_ui::style;
use kentos_ui::widget::{Dialog as Window, focus_ring, overlay};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const DEFINE_TITLE: &str = "Blok oluştur";
const NAME: &str = "Ad";
const DESCRIPTION: &str = "Açıklama";
const REPLACE: &str = "Seçilenleri blokla değiştir";
const CREATE: &str = "Oluştur";
const CANCEL: &str = "Vazgeç";

/// Blok oluştur's name field, which takes the keyboard when the window opens
/// (the trace player follows it: traces/command_line.rs).
pub(crate) const NAME_FIELD: &str = "block-define-name";

/// What the app keeps for the block windows.
#[derive(Debug)]
pub struct Blocks {
    /// Blok oluştur's window, while it is open.
    define: Option<Define>,
    /// “Seçilenleri blokla değiştir”: the window's last choice, for as long as the app lives.
    replace: bool,
    /// The window opened: its name field takes the keyboard, its text chosen.
    focus: bool,
}

impl Default for Blocks {
    fn default() -> Self {
        Self {
            define: None,
            replace: true,
            focus: false,
        }
    }
}

/// Blok oluştur's window: what it names, and what `cad.blocks.define` answers for it.
#[derive(Debug, Clone)]
struct Define {
    base: Vec2,
    uids: Vec<String>,
    name: String,
    description: String,
    replace: bool,
    /// The active layer, which the insert goes on, and its name.
    layer: String,
    layer_name: String,
    /// The command's answer for the window as it is: its warnings' words, or its refusal's.
    answer: Result<Vec<String>, String>,
}

impl Define {
    /// The command's input (the web's `input()`): the name and the
    /// description trimmed, an empty description left out.
    fn input(&self) -> BlocksDefine {
        let about = self.description.trim();
        BlocksDefine {
            name: self.name.trim().to_owned(),
            base: kentos_contracts::Vec2 {
                x: self.base.x,
                y: self.base.y,
            },
            uids: self.uids.clone(),
            description: (!about.is_empty()).then(|| about.to_owned()),
            replace: self.replace.then_some(true),
            layer_id: self.replace.then(|| self.layer.clone()),
            expected_revision: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    Name(String),
    Description(String),
    Replace,
    Create,
    Close,
}

fn msg(event: Event) -> Message {
    Message::Blocks(event)
}

impl App {
    /// Blok oluştur's base point (input.rs): the window opens for the selected objects.
    pub(crate) fn open_block_define(&mut self, base: Vec2) {
        let Some(doc) = &self.document else {
            return;
        };
        let model = &doc.model;
        let uids = self
            .selection
            .ids()
            .iter()
            .filter_map(|slot| model.uid(*slot))
            .map(|uid| uid.to_string())
            .collect();
        let layer = model.layers().active().to_owned();
        let layer_name = model
            .layers()
            .get(&layer)
            .map_or_else(|| layer.clone(), |n| n.name.clone());
        let name =
            kentos_contracts::blocks::free_name(model.blocks().iter().map(|b| b.name.as_str()));
        self.blocks.define = Some(Define {
            base,
            uids,
            name,
            description: String::new(),
            replace: self.blocks.replace,
            layer,
            layer_name,
            answer: Ok(Vec::new()),
        });
        self.blocks.focus = true;
        self.dialog = Some(Dialog::BlockDefine);
        self.block_define_check();
    }

    /// What the command answers for the window as it is now.
    fn block_define_check(&mut self) {
        let (Some(d), Some(doc)) = (self.blocks.define.as_mut(), self.document.as_mut()) else {
            return;
        };
        d.answer = block_define::check(&mut doc.model, &d.input());
    }

    pub(crate) fn blocks_event(&mut self, event: Event) {
        let Some(d) = self.blocks.define.as_mut() else {
            return;
        };
        match event {
            Event::Name(text) => d.name = text,
            Event::Description(text) => d.description = text,
            Event::Replace => d.replace = !d.replace,
            Event::Create => return self.block_define_create(),
            Event::Close => {
                self.dialog = None;
                return self.block_define_closed();
            }
        }
        self.block_define_check();
    }

    /// The window went (Vazgeç, Esc, a click beside it): nothing is written.
    pub(crate) fn block_define_closed(&mut self) {
        self.blocks.define = None;
    }

    /// Oluştur: the block through `cad.blocks.define`, when the command lets it.
    fn block_define_create(&mut self) {
        let (Some(d), Some(doc)) = (self.blocks.define.as_mut(), self.document.as_mut()) else {
            return;
        };
        if d.answer.is_err() {
            return;
        }
        let input = d.input();
        let name = input.name.clone();
        match block_define::define(&mut doc.model, input) {
            Err(why) => d.answer = Err(why),
            Ok((out, warnings)) => {
                let (count, replace) = (d.uids.len(), d.replace);
                self.blocks.replace = replace;
                self.blocks.define = None;
                self.dialog = None;
                self.memory.block_insert = Some(out.block);
                if let Some(id) = out.id {
                    self.selection.set([Slot(id)]);
                }
                for w in warnings {
                    self.warn(w);
                }
                let replaced = if out.insert.is_some() {
                    " Nesneler yerleştirmeyle değiştirildi."
                } else {
                    ""
                };
                self.say(
                    Level::Success,
                    format!("“{name}” bloğu tanımlandı: {count} nesne.{replaced}"),
                );
            }
        }
    }

    /// The window opened: the keyboard to its name field, the name chosen.
    pub(crate) fn blocks_tasks(&mut self) -> Task<Message> {
        if !std::mem::take(&mut self.blocks.focus) || self.blocks.define.is_none() {
            return Task::none();
        }
        let field = Id::new(NAME_FIELD);
        Task::batch([
            iced::widget::operation::focus(field.clone()),
            iced::widget::operation::select_all(field),
        ])
    }

    /// Blok oluştur (the web's `openBlockDefineDialog`).
    pub(crate) fn block_define_view(&self) -> Element<'_, Message> {
        let Some(d) = &self.blocks.define else {
            return text("").into();
        };
        let base = self
            .document
            .as_ref()
            .map_or_else(String::new, |doc| Format::of(doc.settings()).point(d.base));
        let summary = words::summary(vec![
            words::text_line(Kind::Info, format!("{} nesne bloğa alınır.", d.uids.len())),
            words::text_line(Kind::Info, format!("Taban noktası: {base}")),
        ]);
        let mut body = Column::new()
            .spacing(12)
            .push(summary)
            .push(words::field(
                NAME,
                field(&d.name, "", Event::Name, Some(NAME_FIELD)),
                None,
            ))
            .push(words::field(
                DESCRIPTION,
                field(
                    &d.description,
                    "İsteğe bağlı: bloğun ne olduğu",
                    Event::Description,
                    None,
                ),
                None,
            ))
            .push(
                Column::new()
                    .spacing(4)
                    .push(words::check(d.replace, REPLACE, Some(msg(Event::Replace))))
                    .push(kentos_ui::label::caption(replace_hint(
                        d.replace,
                        &d.layer_name,
                    ))),
            );
        match &d.answer {
            Err(why) => body = body.push(words::text_line(Kind::Error, why.clone())),
            Ok(warnings) if !warnings.is_empty() => {
                body = body.push(words::text_line(Kind::Warn, warnings.join(" ")));
            }
            Ok(_) => {}
        }
        overlay::modal(
            Window::new(DEFINE_TITLE)
                .push(body)
                .action(words::secondary(CANCEL, Some(msg(Event::Close))))
                .action(words::primary(
                    CREATE,
                    d.answer.is_ok().then(|| msg(Event::Create)),
                ))
                .width(460.0),
            msg(Event::Close),
        )
    }

    /// The window's controls by their words (a trace's `dialog` step).
    pub(crate) fn block_define_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        let Some(d) = &self.blocks.define else {
            return Err("Blok oluştur penceresi açık değil".to_owned());
        };
        Ok(match control {
            Control::Fill(NAME, t) => Some(msg(Event::Name(t.to_owned()))),
            Control::Fill(DESCRIPTION, t) => Some(msg(Event::Description(t.to_owned()))),
            Control::Check(REPLACE, on) => (d.replace != on).then(|| msg(Event::Replace)),
            Control::Press(CREATE) => d.answer.is_ok().then(|| msg(Event::Create)),
            Control::Press(CANCEL) => Some(msg(Event::Close)),
            other => return Err(format!("“{DEFINE_TITLE}” penceresinde {other} yok")),
        })
    }
}

/// What “Seçilenleri blokla değiştir” does, as it is set (the web's `replaceHint`).
fn replace_hint(replace: bool, layer: &str) -> String {
    if replace {
        format!("Nesneler silinir; yerlerine bloğun bir yerleştirmesi “{layer}” katmanına konur.")
    } else {
        "Nesneler yerlerinde kalır; blok yalnız tanımlanır.".to_owned()
    }
}

/// A text field of the window: Enter is Oluştur, as on the web.
fn field<'a>(
    value: &'a str,
    placeholder: &'a str,
    on: fn(String) -> Event,
    id: Option<&'static str>,
) -> Element<'a, Message> {
    let mut input = text_input(placeholder, value)
        .on_input(move |t| msg(on(t)))
        .on_submit(msg(Event::Create))
        .padding([5, 8])
        .style(style::field::input);
    if let Some(id) = id {
        input = input.id(Id::new(id));
    }
    focus_ring(input).into()
}

#[cfg(test)]
mod tests {
    use kentos_domain::Slot;
    use kentos_interaction::Vec2;

    use super::{DEFINE_TITLE, Event};
    use crate::app::{App, Dialog, Message};
    use crate::files_testing::app_with_drawing;
    use crate::traces::Control;

    /// The sample drawing with its first two objects on layers that are not
    /// locked selected, and the window open for them at the origin.
    fn opened() -> (App, Vec<Slot>) {
        let mut app = app_with_drawing();
        let model = &app.document.as_ref().expect("open").model;
        let slots: Vec<Slot> = model
            .entities()
            .filter(|e| !model.layers().is_locked(&e.base().layer_id))
            .take(2)
            .map(|e| Slot(e.base().id))
            .collect();
        app.selection.set(slots.clone());
        app.open_block_define(Vec2::new(0.0, 0.0));
        (app, slots)
    }

    fn count(app: &App) -> usize {
        app.document.as_ref().map_or(0, |d| d.entity_count())
    }

    fn send(app: &mut App, event: Event) {
        let _ = app.update(Message::Blocks(event));
    }

    #[test]
    fn the_first_free_name_is_offered_and_the_objects_stay_when_asked() {
        let (mut app, slots) = opened();
        assert_eq!(app.dialog, Some(Dialog::BlockDefine));
        assert_eq!(app.dialog_title().as_deref(), Some(DEFINE_TITLE));
        let window = app.blocks.define.as_ref().expect("open");
        assert_eq!((window.name.as_str(), window.replace), ("Blok 1", true));
        assert_eq!(window.uids.len(), 2);
        let before = count(&app);
        send(&mut app, Event::Replace);
        send(&mut app, Event::Description("  Deneme  ".into()));
        send(&mut app, Event::Create);
        assert_eq!(app.dialog, None);
        assert_eq!(count(&app), before, "the objects stay where they are");
        let model = &app.document.as_ref().expect("open").model;
        let block = model.blocks().last().expect("defined");
        assert_eq!(block.name, "Blok 1");
        assert_eq!(block.description.as_deref(), Some("Deneme"));
        assert_eq!(
            (block.base.x, block.base.y, block.entities.len()),
            (0.0, 0.0, 2)
        );
        assert_eq!(
            app.memory.block_insert,
            Some(block.id),
            "Blok ekle places it next"
        );
        assert_eq!(app.selection.ids(), &slots[..], "the selection stays");
        assert!(
            !app.blocks.replace,
            "the choice is kept for the next window"
        );
        // The next window offers the next name, with the box as it was left.
        app.open_block_define(Vec2::new(0.0, 0.0));
        let window = app.blocks.define.as_ref().expect("open");
        assert_eq!((window.name.as_str(), window.replace), ("Blok 2", false));
    }

    #[test]
    fn a_blank_name_is_refused_in_the_window_and_esc_writes_nothing() {
        let (mut app, _) = opened();
        let blocks = |app: &App| app.document.as_ref().map_or(0, |d| d.model.blocks().len());
        let before = blocks(&app);
        send(&mut app, Event::Name(" \t".into()));
        let answer = app.blocks.define.as_ref().map(|d| d.answer.clone());
        assert_eq!(
            answer,
            Some(Err("Blok adı boş olamaz; bir ad yazın.".to_owned()))
        );
        // Oluştur is off: pressed, it sends nothing; the message itself does nothing.
        assert!(matches!(
            app.dialog_control(Control::Press("Oluştur")),
            Ok(None)
        ));
        send(&mut app, Event::Create);
        assert_eq!(
            (app.dialog, blocks(&app)),
            (Some(Dialog::BlockDefine), before)
        );
        assert!(app.dialog_control(Control::Press("Kaydet")).is_err());
        // Esc (the app's close) takes the window away; nothing is written.
        app.close_dialog();
        assert_eq!((app.dialog, blocks(&app)), (None, before));
        assert!(app.blocks.define.is_none());
        assert!(app.dialog_control(Control::Press("Oluştur")).is_err());
    }
}
