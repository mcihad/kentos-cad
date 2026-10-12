//! Adres verisi (`address.data`, docs/adr/0229 §2, §13): the project's
//! address data on the left (Yeni, Sil), the chosen one's definition on the
//! right: its name; Kapı noktaları, Yollar and Bölgeler (mahalle, ilçe, il),
//! each on or off with its layer and the attributes that say what (Alanları
//! tanı fills them from the layers' names); the lowest score and Konumdan
//! adres's reach. Alanları tanı and Denetle are at the buttons' left:
//! Denetle builds the window's definition on the address thread and the
//! summary says what it holds and what it left out until the definition
//! changes. The first thing wrong is said there too and Kaydet waits; Kaydet writes each changed
//! definition through `cad.addresses.define` (a project setting, not an
//! undo step). The form's rules are `kentos_interaction::address_form`'s.

use iced::widget::{Column, Space, button, column, container, row, scrollable, text_input};
use iced::{Center, Element, Fill, Length, Task};
use kentos_contracts::{
    AddressLocator, AddressesDefine, AddressesDefineOperation, address_locators_problem,
    next_address_locator_id,
};
use kentos_geometry_core::ops::geocode::check::Summary;
use kentos_interaction::address_form::{
    AddressForm, DOOR_FIELDS, LEVELS, STREET_FIELDS, def_of, form_of, guess_all, guess_area,
    guess_doors, guess_streets, new_form, parts_line,
};
use kentos_interaction::{Format, Level};
use kentos_ui::icon::icon;
use kentos_ui::theme::typography;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Dialog as Frame, overlay};
use kentos_ui::{label, style};

use super::Event as Address;
use super::engine::{Answer, Built, Job, Reply};
use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind as Say};
use crate::icons::from_web;
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Adres verisi";
const SAVE: &str = "Kaydet";
const CANCEL: &str = "Vazgeç";
const ADD: &str = "Yeni";
const REMOVE: &str = "Sil";
const CHECK: &str = "Denetle";
const GUESS: &str = "Alanları tanı";
/// A field not chosen.
const NONE: &str = "—";
/// Denetle's tickets start here: the tab's never reach them.
const TICKETS: u64 = 1 << 48;
/// The most repeated doors Denetle lists.
const SHOWN_REPEATS: usize = 8;

#[derive(Clone, Debug)]
struct Check {
    ticket: Option<u64>,
    value: Option<Box<Summary>>,
    words: Option<String>,
    millis: Option<f64>,
}

#[derive(Clone, Debug)]
pub struct Window {
    forms: Vec<AddressForm>,
    saved: Vec<AddressLocator>,
    chosen: Option<usize>,
    check: Option<Check>,
    tickets: u64,
    said: Option<(Say, String)>,
}

impl Window {
    /// Whether Denetle waits for this ticket.
    pub fn waits_for(&self, ticket: u64) -> bool {
        self.check
            .as_ref()
            .is_some_and(|c| c.ticket == Some(ticket))
    }

    /// Denetle's answer.
    pub fn answered(&mut self, reply: Reply) {
        let Some(c) = self.check.as_mut() else {
            return;
        };
        c.ticket = None;
        match reply.answer {
            Answer::Checked(s) => {
                c.value = Some(s);
                c.words = None;
            }
            Answer::Failed(why) => c.words = Some(why),
            _ => {}
        }
    }

    /// A build's time, for Denetle's line.
    pub fn built(&mut self, b: &Built) {
        let chosen = self
            .chosen
            .and_then(|i| self.forms.get(i))
            .map(|f| format!("#{}", f.id));
        if let Some(c) = self.check.as_mut()
            && chosen.as_deref() == Some(b.key.as_str())
        {
            c.millis = Some(b.millis);
        }
    }

    /// Denetle's summary, once it came.
    #[cfg(test)]
    pub fn checked(&self) -> Option<&Summary> {
        self.check.as_ref().and_then(|c| c.value.as_deref())
    }

    /// The definitions' names, as the list shows them.
    #[cfg(test)]
    pub fn names(&self) -> Vec<String> {
        self.forms.iter().map(|f| f.name.clone()).collect()
    }
}

#[derive(Clone, Debug)]
pub enum Event {
    Choose(usize),
    Add,
    Remove,
    Name(String),
    DoorsOn(bool),
    DoorsLayer(usize),
    DoorField(usize, usize),
    StreetsOn(bool),
    StreetsLayer(usize),
    StreetField(usize, usize),
    AreaOn(usize, bool),
    AreaLayer(usize, usize),
    AreaName(usize, usize),
    AreaDistrict(usize, usize),
    Guess,
    MinScore(String),
    Reach(String),
    Check,
    Save,
    Cancel,
}

fn msg(e: Event) -> Message {
    Message::Address(Address::Data(e))
}

/// A field list's choices: none, then the layer's names; a name it does not have kept, marked.
fn field_choices(names: &[String], current: &str) -> (Vec<Choice>, Option<usize>) {
    let mut choices = vec![Choice::new(NONE)];
    choices.extend(names.iter().map(Choice::new));
    let at = if current.is_empty() {
        Some(0)
    } else if let Some(i) = names.iter().position(|n| n == current) {
        Some(i + 1)
    } else {
        choices.push(Choice::new(format!("(katmanda yok: {current})")));
        Some(choices.len() - 1)
    };
    (choices, at)
}

/// The name a field list's choice `k` gives; the current one for the mark of a missing name.
fn field_of(names: &[String], k: usize, current: &str) -> String {
    match k {
        0 => String::new(),
        k => names
            .get(k - 1)
            .cloned()
            .unwrap_or_else(|| current.to_owned()),
    }
}

impl App {
    /// The layers the window lists, by id with their paths.
    fn address_layers(&self) -> Vec<(String, String)> {
        let Some(doc) = &self.document else {
            return Vec::new();
        };
        let layers = doc.model.layers();
        layers
            .leaves()
            .iter()
            .map(|n| (n.id.clone(), layers.path(&n.id)))
            .collect()
    }

    /// A layer's attribute names.
    fn address_fields(&self, layer: &str) -> Vec<String> {
        self.document
            .as_ref()
            .map(|d| kentos_native_application::addresses::field_names(&d.model, layer))
            .unwrap_or_default()
    }

    /// `address.data`: the window.
    pub(crate) fn open_address_data(&mut self) -> Task<Message> {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return Task::none();
        };
        let format = Format::of(doc.settings());
        let saved = doc.model.settings().address_locators.clone();
        let forms: Vec<AddressForm> = saved
            .iter()
            .map(|l| form_of(l, |m| format.from_metres(m)))
            .collect();
        self.address.data = Some(Window {
            chosen: (!forms.is_empty()).then_some(0),
            forms,
            saved,
            check: None,
            tickets: TICKETS,
            said: None,
        });
        self.dialog = Some(Dialog::AddressData);
        Task::none()
    }

    /// The definitions the window writes, or the first thing wrong.
    fn data_defs(&self, w: &Window) -> Result<Vec<AddressLocator>, String> {
        let format = self.format();
        let out = w
            .forms
            .iter()
            .map(|f| def_of(f, |v| format.to_metres(v)))
            .collect::<Result<Vec<_>, _>>()?;
        match address_locators_problem(&out) {
            Some(p) => {
                let mut c = p.chars();
                let first = c
                    .next()
                    .map(|x| kentos_interaction::upper_tr(&x.to_string()))
                    .unwrap_or_default();
                Err(format!("{first}{}.", c.as_str()))
            }
            None => Ok(out),
        }
    }

    pub(crate) fn address_data_event(&mut self, e: Event) -> Task<Message> {
        let layers = self.address_layers();
        let format = self.format();
        let names_of = |app: &App, layer: &str| app.address_fields(layer);
        let names = {
            let Some(w) = self.address.data.as_ref() else {
                return Task::none();
            };
            let f = w.chosen.and_then(|i| w.forms.get(i));
            let layer_of = |e: &Event| -> Option<String> {
                let f = f?;
                match e {
                    Event::DoorField(..) | Event::DoorsLayer(_) => Some(f.doors.layer.clone()),
                    Event::StreetField(..) | Event::StreetsLayer(_) => {
                        Some(f.streets.layer.clone())
                    }
                    Event::AreaName(i, _) | Event::AreaDistrict(i, _) | Event::AreaLayer(i, _) => {
                        f.areas.get(*i).map(|a| a.layer.clone())
                    }
                    _ => None,
                }
            };
            // A layer chosen: its names, for the fields Alanları tanı fills.
            let chosen_layer = match &e {
                Event::DoorsLayer(k) | Event::StreetsLayer(k) | Event::AreaLayer(_, k) => {
                    layers.get(*k).map(|(id, _)| id.clone())
                }
                other => layer_of(other),
            };
            chosen_layer.map(|l| names_of(self, &l)).unwrap_or_default()
        };
        let all_names: std::collections::HashMap<String, Vec<String>> = if matches!(e, Event::Guess)
        {
            layers
                .iter()
                .map(|(id, _)| (id.clone(), self.address_fields(id)))
                .collect()
        } else {
            Default::default()
        };
        let Some(w) = self.address.data.as_mut() else {
            return Task::none();
        };
        w.said = None;
        let chosen = w.chosen;
        let f = chosen.and_then(|i| w.forms.get_mut(i));
        match e {
            Event::Choose(i) => {
                w.chosen = Some(i);
                w.check = None;
            }
            Event::Add => {
                let defs: Vec<AddressLocator> = w
                    .forms
                    .iter()
                    .map(|f| AddressLocator {
                        id: f.id.clone(),
                        name: f.name.clone(),
                        doors: None,
                        streets: None,
                        areas: Vec::new(),
                        min_score: kentos_contracts::ADDRESS_MIN_SCORE,
                        reach: kentos_contracts::ADDRESS_REACH,
                    })
                    .collect();
                let mut name = "Numarataj".to_owned();
                let mut k = 2;
                while w.forms.iter().any(|f| f.name.trim() == name) {
                    name = format!("Numarataj {k}");
                    k += 1;
                }
                w.forms
                    .push(new_form(&next_address_locator_id(&defs), &name, |m| {
                        format.from_metres(m)
                    }));
                w.chosen = Some(w.forms.len() - 1);
                w.check = None;
            }
            Event::Remove => {
                if let Some(i) = chosen {
                    w.forms.remove(i);
                    w.chosen = if w.forms.is_empty() {
                        None
                    } else {
                        Some(i.min(w.forms.len() - 1))
                    };
                    w.check = None;
                }
            }
            Event::Cancel => {
                self.address.data = None;
                self.dialog = None;
            }
            Event::Save => return self.save_address_data(),
            Event::Check => return self.check_address_data(),
            Event::Guess => {
                if let Some(f) = f {
                    guess_all(f, |l| all_names.get(l).cloned().unwrap_or_default());
                }
                w.check = None;
            }
            other => {
                let Some(f) = f else {
                    return Task::none();
                };
                w.check = None;
                match other {
                    Event::Name(t) => f.name = t,
                    Event::DoorsOn(on) => f.doors.on = on,
                    Event::DoorsLayer(k) => {
                        if let Some((id, _)) = layers.get(k) {
                            f.doors.layer = id.clone();
                            guess_doors(&mut f.doors, &names);
                        }
                    }
                    Event::DoorField(i, k) => {
                        if let Some(slot) = f.doors.fields.get_mut(i) {
                            *slot = field_of(&names, k, slot);
                        }
                    }
                    Event::StreetsOn(on) => f.streets.on = on,
                    Event::StreetsLayer(k) => {
                        if let Some((id, _)) = layers.get(k) {
                            f.streets.layer = id.clone();
                            guess_streets(&mut f.streets, &names);
                        }
                    }
                    Event::StreetField(i, k) => {
                        if let Some(slot) = f.streets.fields.get_mut(i) {
                            *slot = field_of(&names, k, slot);
                        }
                    }
                    Event::AreaOn(i, on) => {
                        if let Some(a) = f.areas.get_mut(i) {
                            a.on = on;
                        }
                    }
                    Event::AreaLayer(i, k) => {
                        if let (Some(a), Some((id, _))) = (f.areas.get_mut(i), layers.get(k)) {
                            a.layer = id.clone();
                            guess_area(a, LEVELS[i].0, &names);
                        }
                    }
                    Event::AreaName(i, k) => {
                        if let Some(a) = f.areas.get_mut(i) {
                            a.name = field_of(&names, k, &a.name);
                        }
                    }
                    Event::AreaDistrict(i, k) => {
                        if let Some(a) = f.areas.get_mut(i) {
                            a.district = field_of(&names, k, &a.district);
                        }
                    }
                    Event::MinScore(t) => f.min_score = t,
                    Event::Reach(t) => f.reach = t,
                    _ => {}
                }
            }
        }
        Task::none()
    }

    fn save_address_data(&mut self) -> Task<Message> {
        let Some(w) = self.address.data.clone() else {
            return Task::none();
        };
        let now = match self.data_defs(&w) {
            Ok(now) => now,
            Err(p) => {
                if let Some(w) = self.address.data.as_mut() {
                    w.said = Some((Say::Error, p));
                }
                return Task::none();
            }
        };
        let Some(doc) = self.document.as_mut() else {
            return Task::none();
        };
        let mut said: Vec<String> = Vec::new();
        let mut run =
            |input: AddressesDefine, doc: &mut kentos_domain::Document| -> Result<(), String> {
                let mut cx = kentos_native_application::ExecutionContext::new(doc);
                match kentos_native_application::address_define::execute(&mut cx, input) {
                    kentos_contracts::CommandResult::Completed { warnings, .. } => {
                        said.extend(warnings.into_iter().map(|w| w.message));
                        Ok(())
                    }
                    kentos_contracts::CommandResult::Failed { error }
                    | kentos_contracts::CommandResult::Conflict { error }
                    | kentos_contracts::CommandResult::NeedsInput { error } => Err(error.message),
                    _ => Err("Adres verisi yazılamadı.".into()),
                }
            };
        for old in &w.saved {
            if !now.iter().any(|n| n.id == old.id)
                && let Err(why) = run(
                    AddressesDefine {
                        operation: AddressesDefineOperation::Remove,
                        locator: None,
                        id: Some(old.id.clone()),
                        expected_revision: None,
                    },
                    &mut doc.model,
                )
            {
                self.say(Level::Warn, why);
                return Task::none();
            }
        }
        for n in &now {
            if w.saved.iter().any(|o| o == n) {
                continue;
            }
            if let Err(why) = run(
                AddressesDefine {
                    operation: AddressesDefineOperation::Set,
                    locator: Some(n.clone()),
                    id: None,
                    expected_revision: None,
                },
                &mut doc.model,
            ) {
                self.say(Level::Warn, why);
                return Task::none();
            }
        }
        for s in said {
            self.say(Level::Warn, s);
        }
        self.address.data = None;
        self.dialog = None;
        self.say(
            Level::Info,
            format!("Adres verisi kaydedildi: {} tanım.", now.len()),
        );
        Task::none()
    }

    /// Denetle: the chosen definition built as the window says it, checked on the address thread.
    fn check_address_data(&mut self) -> Task<Message> {
        let format = self.format();
        let Some(w) = self.address.data.as_mut() else {
            return Task::none();
        };
        let Some(f) = w.chosen.and_then(|i| w.forms.get(i)) else {
            return Task::none();
        };
        let def = match def_of(f, |v| format.to_metres(v)) {
            Ok(d) => d,
            Err(p) => {
                w.check = Some(Check {
                    ticket: None,
                    value: None,
                    words: Some(p),
                    millis: None,
                });
                return Task::none();
            }
        };
        w.tickets += 1;
        let ticket = w.tickets;
        w.check = Some(Check {
            ticket: Some(ticket),
            value: None,
            words: Some("Dizin kuruluyor ve denetleniyor…".into()),
            millis: None,
        });
        // Under a key of its own: the project's locator (the tab's) is not built again for a definition not saved.
        let key = format!("#{}", def.id);
        let job_key = key.clone();
        self.address_ask(
            &key,
            &def,
            move |done| Job::Check {
                key: job_key,
                ticket,
                done,
            },
            ticket,
        )
    }

    pub(crate) fn address_data_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.address.data else {
            return iced::widget::text("").into();
        };
        let mut list = Column::new().spacing(2);
        for (i, f) in w.forms.iter().enumerate() {
            let name = if f.name.trim().is_empty() {
                "Adsız adres verisi".to_owned()
            } else {
                f.name.clone()
            };
            list = list.push(
                button(column![label::body(name), label::caption(parts_line(f))].spacing(1))
                    .width(Fill)
                    .padding([4, 8])
                    .on_press(msg(Event::Choose(i)))
                    .style(style::button::table_row(w.chosen == Some(i), false)),
            );
        }
        if w.forms.is_empty() {
            list = list.push(
                container(label::muted(
                    "Adres verisi yok. Yeni ile kapı noktaları, yollar ve mahalle alanlarından bir tanım ekleyin.",
                ))
                .padding(8),
            );
        }
        let tool = |glyph: &str, words: &'static str, on: Option<Message>| {
            button(
                row![icon(from_web(Some(glyph))).size(14.0), label::body(words)]
                    .spacing(6)
                    .align_y(Center),
            )
            .style(style::button::secondary)
            .padding([4, 10])
            .on_press_maybe(on)
        };
        let left = column![
            container(scrollable(list).height(Fill))
                .height(Fill)
                .style(style::container::field_box)
                .width(Fill),
            row![
                tool(
                    "plus",
                    ADD,
                    (w.forms.len() < kentos_contracts::ADDRESS_LOCATOR_LIMIT)
                        .then(|| msg(Event::Add))
                ),
                tool("erase", REMOVE, w.chosen.map(|_| msg(Event::Remove)))
            ]
            .spacing(8),
        ]
        .spacing(8)
        .height(Fill)
        .width(Length::Fixed(typography::scaled(210.0)));
        let right: Element<'_, Message> = match w.chosen.and_then(|i| w.forms.get(i)) {
            None => container(label::muted(
                "Soldan bir tanım seçin ya da Yeni ile ekleyin.",
            ))
            .padding(8)
            .into(),
            Some(f) => container(scrollable(self.address_form_view(f)).height(Fill))
                .height(Fill)
                .width(Fill)
                .into(),
        };
        let problem = self.data_defs(w).err();
        // Denetle's answer stays in the summary until the definition changes; a refusal or a problem otherwise.
        let checked = w
            .check
            .as_ref()
            .map(|c| self.address_check_lines(c))
            .unwrap_or_default();
        let lines = match (&w.said, &problem) {
            (Some((kind, t)), _) => vec![words::text_line(*kind, t.clone())],
            (None, _) if !checked.is_empty() => checked,
            (None, Some(p)) => vec![words::text_line(Say::Error, p.clone())],
            (None, None) if w.forms.is_empty() => {
                vec![words::text_line(Say::Ok, "Adres verisi yok.")]
            }
            (None, None) => vec![words::text_line(
                Say::Ok,
                format!(
                    "{} tanım. Adres ara, Konumdan adres ve İşlemler'in adres araçları bunları kullanır; veri cihazdan çıkmaz.",
                    w.forms.len()
                ),
            )],
        };
        let ready = problem.is_none() && self.data_defs(w).is_ok_and(|now| now != w.saved);
        let one = w.chosen.is_some();
        overlay::modal(
            Frame::new(TITLE)
                .fill(row![left, right].spacing(16).height(Fill))
                .push(words::summary(lines))
                .aside(words::secondary(GUESS, one.then(|| msg(Event::Guess))))
                .aside(words::secondary(CHECK, one.then(|| msg(Event::Check))))
                .action(words::secondary(CANCEL, Some(msg(Event::Cancel))))
                .action(words::primary(SAVE, ready.then(|| msg(Event::Save))))
                .width(1000.0)
                .max_height(660.0),
            msg(Event::Cancel),
        )
    }

    fn address_form_view<'a>(&'a self, f: &'a AddressForm) -> Element<'a, Message> {
        let format = self.format();
        let layers = self.address_layers();
        let labelled = |name: &'a str, body: Element<'a, Message>| -> Element<'a, Message> {
            row![
                container(label::body(name)).width(Length::Fixed(typography::scaled(150.0))),
                body
            ]
            .spacing(8)
            .align_y(Center)
            .into()
        };
        let head = |on: bool, name: &'a str, toggle: Message| -> Element<'a, Message> {
            column![
                row![
                    words::check(on, name, Some(toggle)),
                    Space::new().width(Fill)
                ]
                .align_y(Center),
                kentos_ui::widget::horizontal_divider()
            ]
            .spacing(4)
            .into()
        };
        let layer_select =
            |current: &str, on: Box<dyn Fn(usize) -> Event + 'a>| -> Element<'a, Message> {
                let mut choices: Vec<Choice> =
                    layers.iter().map(|(_, p)| Choice::new(p.clone())).collect();
                let mut at = layers.iter().position(|(id, _)| id == current);
                if at.is_none() && !current.is_empty() {
                    choices.push(Choice::new(format!("(çizimde yok: {current})")));
                    at = Some(choices.len() - 1);
                }
                Select::new(choices, at, move |k| msg(on(k)))
                    .placeholder("Katman seçin")
                    .into()
            };
        let field_select = |layer: &str,
                            current: &str,
                            on: Box<dyn Fn(usize) -> Event + 'a>|
         -> Element<'a, Message> {
            let names = self.address_fields(layer);
            let (choices, at) = field_choices(&names, current);
            container(Select::new(choices, at, move |k| msg(on(k))))
                .width(Length::Fixed(typography::scaled(220.0)))
                .into()
        };
        let input = |value: &'a str, on: fn(String) -> Event| -> Element<'a, Message> {
            container(
                text_input("", value)
                    .size(typography::body())
                    .padding([4, 8])
                    .on_input(move |t| msg(on(t)))
                    .style(style::field::input),
            )
            .width(Length::Fixed(typography::scaled(120.0)))
            .into()
        };
        let mut form = Column::new().spacing(8).padding([0, 8]);
        form = form.push(labelled(
            "Ad",
            text_input("Tanımın adı", &f.name)
                .size(typography::body())
                .padding([4, 8])
                .on_input(|t| msg(Event::Name(t)))
                .style(style::field::input)
                .into(),
        ));
        // Kapı noktaları.
        form = form.push(head(
            f.doors.on,
            "Kapı noktaları",
            msg(Event::DoorsOn(!f.doors.on)),
        ));
        if f.doors.on {
            form = form.push(labelled(
                "Katman",
                layer_select(&f.doors.layer, Box::new(Event::DoorsLayer)),
            ));
            for (i, (name, needed)) in DOOR_FIELDS.iter().enumerate() {
                let label_words: &'a str = if *needed {
                    match i {
                        0 => "Yol *",
                        _ => "Kapı no *",
                    }
                } else {
                    name
                };
                form = form.push(labelled(
                    label_words,
                    field_select(
                        &f.doors.layer,
                        &f.doors.fields[i],
                        Box::new(move |k| Event::DoorField(i, k)),
                    ),
                ));
            }
        }
        // Yollar.
        form = form.push(head(
            f.streets.on,
            "Yollar",
            msg(Event::StreetsOn(!f.streets.on)),
        ));
        if f.streets.on {
            form = form.push(labelled(
                "Katman",
                layer_select(&f.streets.layer, Box::new(Event::StreetsLayer)),
            ));
            for (i, (name, needed)) in STREET_FIELDS.iter().enumerate() {
                let label_words: &'a str = if *needed { "Ad *" } else { name };
                form = form.push(labelled(
                    label_words,
                    field_select(
                        &f.streets.layer,
                        &f.streets.fields[i],
                        Box::new(move |k| Event::StreetField(i, k)),
                    ),
                ));
            }
        }
        // Bölgeler.
        form = form.push(
            column![
                label::caption("Bölgeler"),
                kentos_ui::widget::horizontal_divider()
            ]
            .spacing(4),
        );
        for (i, ((_, word), a)) in LEVELS.iter().zip(&f.areas).enumerate() {
            let mut line = row![
                container(words::check(
                    a.on,
                    *word,
                    Some(msg(Event::AreaOn(i, !a.on)))
                ))
                .width(Length::Fixed(typography::scaled(150.0))),
            ]
            .spacing(8)
            .align_y(Center);
            if a.on {
                line = line
                    .push(
                        container(layer_select(
                            &a.layer,
                            Box::new(move |k| Event::AreaLayer(i, k)),
                        ))
                        .width(Length::FillPortion(3)),
                    )
                    .push(label::caption("Ad"))
                    .push(field_select(
                        &a.layer,
                        &a.name,
                        Box::new(move |k| Event::AreaName(i, k)),
                    ));
                if i == 0 {
                    line = line.push(label::caption("İlçe")).push(field_select(
                        &a.layer,
                        &a.district,
                        Box::new(move |k| Event::AreaDistrict(i, k)),
                    ));
                }
            }
            form = form.push(line);
        }
        // Eşleşme.
        form = form.push(
            column![
                label::caption("Eşleşme"),
                kentos_ui::widget::horizontal_divider()
            ]
            .spacing(4),
        );
        form = form.push(labelled(
            "En düşük puan",
            input(&f.min_score, Event::MinScore),
        ));
        let unit = format.unit.mark();
        form = form.push(
            row![
                container(label::body("Konumdan adres uzaklığı"))
                    .width(Length::Fixed(typography::scaled(150.0))),
                input(&f.reach, Event::Reach),
                label::muted(unit),
            ]
            .spacing(8)
            .align_y(Center),
        );
        form.into()
    }

    /// Denetle's lines, for the window's summary: seen wherever the form is scrolled to.
    fn address_check_lines<'a>(&self, c: &'a Check) -> Vec<Element<'a, Message>> {
        let mut lines: Vec<Element<'a, Message>> = Vec::new();
        if let Some(t) = &c.words {
            lines.push(words::text_line(
                if c.ticket.is_some() {
                    Say::Info
                } else {
                    Say::Error
                },
                t.clone(),
            ));
        }
        if let Some(s) = &c.value {
            let k = &s.counts;
            let time = c
                .millis
                .map(|m| format!(" ({m:.0} ms)"))
                .unwrap_or_default();
            lines.push(words::text_line(
                Say::Ok,
                format!(
                    "{} kapı, {} yol, {} yol çizgisi, {} mahalle, {} bölge alanı{time}.",
                    k.doors, k.streets, k.lines, k.neighbourhoods, k.areas
                ),
            ));
            let mut out: Vec<String> = Vec::new();
            for (n, words) in [
                (s.not_doors, "nesne kapı katmanında nokta değil"),
                (s.empty_streets, "kapının yolu yok"),
                (s.unread_doors, "kapının numarası okunamadı"),
                (s.not_streets, "nesne yol katmanında çizgi değil"),
                (s.unnamed_lines, "yolun adı yok"),
                (s.not_areas, "nesne bölge katmanında alan değil"),
                (s.unnamed_areas, "alanın adı yok"),
            ] {
                if n > 0 {
                    out.push(format!("{n} {words}"));
                }
            }
            if !out.is_empty() {
                lines.push(words::text_line(
                    Say::Warn,
                    format!("Atlananlar: {}.", out.join("; ")),
                ));
            }
            if s.repeated > 0 {
                let shown: Vec<String> = s
                    .repeats
                    .iter()
                    .take(SHOWN_REPEATS)
                    .map(|r| {
                        let hood = if r.neighbourhood.is_empty() {
                            String::new()
                        } else {
                            format!(" ({})", r.neighbourhood)
                        };
                        format!("{} No:{}{hood}: {} kez", r.street, r.door, r.orders.len())
                    })
                    .collect();
                let more = s.repeated.saturating_sub(shown.len());
                let tail = if more > 0 {
                    format!("; {more} tane daha")
                } else {
                    String::new()
                };
                lines.push(words::text_line(
                    Say::Warn,
                    format!(
                        "Aynı yolda aynı kapı: {} ({}{tail}).",
                        s.repeated,
                        shown.join("; ")
                    ),
                ));
            }
        }
        lines
    }

    /// The window's controls by their words (a trace's `dialog` step).
    pub(crate) fn address_data_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        let w = self
            .address
            .data
            .as_ref()
            .ok_or("Adres verisi açık değil")?;
        Ok(match control {
            Control::Press(SAVE) => Some(msg(Event::Save)),
            Control::Press(CANCEL) => Some(msg(Event::Cancel)),
            Control::Press(ADD) => Some(msg(Event::Add)),
            Control::Press(REMOVE) => w.chosen.map(|_| msg(Event::Remove)),
            Control::Press(CHECK) => Some(msg(Event::Check)),
            Control::Press(GUESS) => Some(msg(Event::Guess)),
            Control::Fill("Ad", t) => Some(msg(Event::Name(t.to_owned()))),
            Control::Fill("En düşük puan", t) => Some(msg(Event::MinScore(t.to_owned()))),
            Control::Fill("Konumdan adres uzaklığı", t) => Some(msg(Event::Reach(t.to_owned()))),
            Control::Check("Kapı noktaları", on) => Some(msg(Event::DoorsOn(on))),
            Control::Check("Yollar", on) => Some(msg(Event::StreetsOn(on))),
            Control::Check(word, on) => {
                let i = LEVELS
                    .iter()
                    .position(|(_, w)| *w == word)
                    .ok_or_else(|| format!("“{word}” kutusu yok"))?;
                Some(msg(Event::AreaOn(i, on)))
            }
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}
