//! Ağlar (docs/adr/0209 §10; the web's `ui/networks/NetworksDialog.ts`): the
//! project's networks on the left (Yeni ağ, Sil), the chosen one's
//! definition on the right: its name and kind, edge layers with their
//! filters, junction layers with their roles, filters and closed expressions,
//! how ends meet and the tolerance, the direction, the costs and the closed
//! edges. Denetle builds the network as the window says it and lists its
//! counts and problems; a problem's row selects its objects, marks it in the
//! drawing and goes there. The first thing wrong is said under the form and
//! Kaydet waits; Kaydet writes each changed network through
//! `cad.network.define` (a project setting, not an undo step).

use iced::widget::{Column, Space, button, column, container, row, scrollable, text_input};
use iced::{Center, Element, Fill, Length, Task};
use kentos_contracts::{
    JunctionRole, NetworkConnect, NetworkCostKind, NetworkDef, NetworkDefine,
    NetworkDefineOperation, NetworkKind, networks_problem, next_network_id,
};
use kentos_geometry_core::ops::network::{Checked, ProblemKind};
use kentos_interaction::network::form::{
    CostRow, DIRECTION_DEFAULTS, DirectionKind, EdgeRow, JunctionRow, NetworkForm, def_of,
    form_expressions, form_of, new_form, retyped,
};
use kentos_interaction::network::{Answer, NetworkAsk, NetworkQuestion, NetworkReply};
use kentos_interaction::{Format, Level, ViewChange};
use kentos_ui::icon::icon;
use kentos_ui::theme::typography;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Dialog as Frame, overlay};
use kentos_ui::{label, style};

use super::Event as Networks;
use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind as Say};
use crate::icons::from_web;
use crate::topology::ProblemMark;
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Ağlar";
const SAVE: &str = "Kaydet";
const CANCEL: &str = "Vazgeç";
const ADD: &str = "Yeni ağ";
const REMOVE: &str = "Sil";
const CHECK: &str = "Denetle";
/// At most this many problems are listed (Denetle says how many more).
const SHOWN_PROBLEMS: usize = 300;
/// Denetle's tickets start here: the tools' never reach them.
const TICKETS: u64 = 1 << 48;

/// Yön's choices.
const DIRECTIONS: [(DirectionKind, &str); 3] = [
    (DirectionKind::Both, "Yok (iki yön)"),
    (DirectionKind::Digitized, "Çizim yönü"),
    (DirectionKind::Field, "Alandan"),
];
const KINDS: [(NetworkKind, &str); 2] = [
    (NetworkKind::Road, "Yol ağı"),
    (NetworkKind::Utility, "Şebeke"),
];
const CONNECTS: [(NetworkConnect, &str); 2] = [
    (NetworkConnect::Ends, "Uçlarda"),
    (NetworkConnect::Vertices, "Köşelerde"),
];
const ROLES: [(JunctionRole, &str); 3] = [
    (JunctionRole::Junction, "Bağlantı"),
    (JunctionRole::Source, "Kaynak"),
    (JunctionRole::Valve, "Vana"),
];
const COST_KINDS: [(NetworkCostKind, &str); 2] = [
    (NetworkCostKind::Speed, "Süre (hızdan)"),
    (NetworkCostKind::Field, "Alandan"),
];

/// A problem kind in Denetle's words (docs/adr/0209 §9).
pub fn problem_label(k: ProblemKind) -> &'static str {
    match k {
        ProblemKind::Detached => "Kopuk parça",
        ProblemKind::NearMiss => "Yakın ama bağlı değil",
        ProblemKind::Crossing => "Bağlanmayan kesişim",
        ProblemKind::OffNetwork => "Ağın dışında düğüm",
        ProblemKind::Short => "Sıfır uzunluklu parça",
        ProblemKind::Unread => "Okunamayan değer",
    }
}

/// An expression field of the form the builder (ε) writes back to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    EdgeFilter(usize),
    JunctionFilter(usize),
    JunctionClosed(usize),
    Closed,
}

/// Denetle as last asked: its ticket while it waits, its answer or its words.
#[derive(Clone, Debug)]
struct Check {
    ticket: Option<u64>,
    value: Option<Box<Checked>>,
    words: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Window {
    forms: Vec<NetworkForm>,
    saved: Vec<NetworkDef>,
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

    /// Denetle's answer, once it came.
    #[cfg(test)]
    pub fn checked(&self) -> Option<&Checked> {
        self.check.as_ref().and_then(|c| c.value.as_deref())
    }

    /// The networks' names, as the list shows them.
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
    Kind(usize),
    EdgeLayer(usize, usize),
    EdgeAdd,
    EdgeRemove(usize),
    JunctionLayer(usize, usize),
    JunctionRole(usize, usize),
    JunctionAdd,
    JunctionRemove(usize),
    Text(Field, String),
    Connect(usize),
    Tolerance(String),
    Direction(usize),
    DirectionField(String),
    Forward(String),
    Backward(String),
    Shut(String),
    CostName(usize, String),
    CostKind(usize, usize),
    CostField(usize, String),
    CostUnit(usize, String),
    CostSpeed(usize, String),
    CostAdd,
    CostRemove(usize),
    Builder(Field),
    Check,
    Problem(usize),
    Save,
    Cancel,
}

fn msg(e: Event) -> Message {
    Message::Networks(Networks::Window(e))
}

impl App {
    /// The layers the window lists, by id with their paths.
    fn network_layers(&self) -> Vec<(String, String)> {
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

    /// `network.manage`: the window.
    pub(crate) fn open_networks(&mut self) -> Task<Message> {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return Task::none();
        };
        let format = Format::of(doc.settings());
        let saved = doc.model.settings().networks.clone();
        let forms: Vec<NetworkForm> = saved
            .iter()
            .map(|n| form_of(n, |m| format.from_metres(m)))
            .collect();
        self.networks.window = Some(Window {
            chosen: (!forms.is_empty()).then_some(0),
            forms,
            saved,
            check: None,
            tickets: TICKETS,
            said: None,
        });
        self.dialog = Some(Dialog::Networks);
        Task::none()
    }

    /// The networks the window writes, or the first thing wrong.
    fn window_defs(&self, w: &Window) -> Result<Vec<NetworkDef>, String> {
        let format = self.format();
        let mut out = Vec::new();
        for f in &w.forms {
            let d = def_of(f, |v| format.to_metres(v))?;
            for (what, text) in form_expressions(f) {
                if let Err(e) = kentos_expression::compile(&text) {
                    return Err(format!(
                        "“{}” ağının {}: {}",
                        d.name,
                        kentos_interaction::lower_tr(&what),
                        e.text()
                    ));
                }
            }
            out.push(d);
        }
        match networks_problem(&out) {
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

    fn window_changed(&self, w: &Window) -> bool {
        match self.window_defs(w) {
            Ok(now) => now != w.saved,
            Err(_) => true,
        }
    }

    pub(crate) fn networks_window_event(&mut self, e: Event) -> Task<Message> {
        let active = self
            .document
            .as_ref()
            .map(|d| d.model.layers().active().to_owned())
            .unwrap_or_default();
        let layers = self.network_layers();
        let format = self.format();
        let Some(w) = self.networks.window.as_mut() else {
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
                let ids: Vec<NetworkDef> = w.forms.iter().map(|f| def_shell(&f.id)).collect();
                let mut name = "Yeni ağ".to_owned();
                let mut k = 2;
                while w.forms.iter().any(|f| f.name.trim() == name) {
                    name = format!("Yeni ağ {k}");
                    k += 1;
                }
                let layer = if layers.iter().any(|(id, _)| *id == active) {
                    active
                } else {
                    String::new()
                };
                w.forms.push(new_form(
                    &next_network_id(&ids),
                    &name,
                    NetworkKind::Road,
                    &layer,
                    |m| format.from_metres(m),
                ));
                w.chosen = Some(w.forms.len() - 1);
                w.check = None;
            }
            Event::Remove => {
                if let Some(i) = w.chosen.filter(|&i| i < w.forms.len()) {
                    w.forms.remove(i);
                    w.chosen = (!w.forms.is_empty()).then(|| i.min(w.forms.len() - 1));
                    w.check = None;
                }
            }
            Event::Save => return self.save_networks(),
            Event::Cancel => {
                self.networks.window = None;
                self.dialog = None;
            }
            Event::Check => return self.check_network(),
            Event::Problem(i) => return self.show_network_problem(i),
            Event::Builder(field) => return self.open_builder_for_network(field),
            other => {
                let Some(f) = f else {
                    return Task::none();
                };
                edit(f, other, &layers);
            }
        }
        Task::none()
    }

    /// Kaydet: each network taken away is removed and each new or changed one written, in the list's order.
    fn save_networks(&mut self) -> Task<Message> {
        let Some(w) = self.networks.window.clone() else {
            return Task::none();
        };
        let now = match self.window_defs(&w) {
            Ok(now) => now,
            Err(p) => {
                if let Some(w) = self.networks.window.as_mut() {
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
            |input: NetworkDefine, doc: &mut kentos_domain::Document| -> Result<(), String> {
                let mut cx = kentos_native_application::ExecutionContext::new(doc);
                match kentos_native_application::network_define::execute(&mut cx, input) {
                    kentos_contracts::CommandResult::Completed { warnings, .. } => {
                        said.extend(warnings.into_iter().map(|w| w.message));
                        Ok(())
                    }
                    kentos_contracts::CommandResult::Failed { error }
                    | kentos_contracts::CommandResult::Conflict { error }
                    | kentos_contracts::CommandResult::NeedsInput { error } => Err(error.message),
                    _ => Err("Ağ yazılamadı.".into()),
                }
            };
        for old in &w.saved {
            if !now.iter().any(|n| n.id == old.id)
                && let Err(why) = run(
                    NetworkDefine {
                        operation: NetworkDefineOperation::Remove,
                        network: None,
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
                NetworkDefine {
                    operation: NetworkDefineOperation::Set,
                    network: Some(n.clone()),
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
        self.networks.window = None;
        self.dialog = None;
        self.networks_follow();
        self.say(Level::Info, format!("Ağlar kaydedildi: {} ağ.", now.len()));
        Task::none()
    }

    /// Denetle: the chosen network built as the window says it, checked on the network thread.
    fn check_network(&mut self) -> Task<Message> {
        let format = self.format();
        let Some(w) = self.networks.window.as_mut() else {
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
                });
                return Task::none();
            }
        };
        w.tickets += 1;
        let ticket = w.tickets;
        w.check = Some(Check {
            ticket: Some(ticket),
            value: None,
            words: Some("Ağ kuruluyor ve denetleniyor…".into()),
        });
        // Under a key of its own: the project's network (the tools') is not built again for a definition not saved.
        self.network_ask(NetworkAsk {
            ticket,
            network: format!("#{}", def.id),
            def: Some(def),
            question: NetworkQuestion::Check,
        })
    }

    pub(crate) fn networks_window_answered(&mut self, reply: NetworkReply) -> Task<Message> {
        let Some(c) = self.networks.window.as_mut().and_then(|w| w.check.as_mut()) else {
            return Task::none();
        };
        c.ticket = None;
        match reply.answer {
            Answer::Check(checked) => {
                c.value = Some(checked);
                c.words = None;
            }
            Answer::Failed(why) => c.words = Some(why),
            _ => {}
        }
        Task::none()
    }

    /// A problem's objects selected, the problem marked and the view on it.
    fn show_network_problem(&mut self, i: usize) -> Task<Message> {
        let Some(p) = self
            .networks
            .window
            .as_ref()
            .and_then(|w| w.check.as_ref())
            .and_then(|c| c.value.as_ref())
            .and_then(|v| v.problems.get(i))
            .cloned()
        else {
            return Task::none();
        };
        let slots: Vec<kentos_domain::Slot> = p
            .ids
            .iter()
            .filter_map(|&id| kentos_interaction::network::base::slot_of(id))
            .filter(|s| {
                self.document
                    .as_ref()
                    .is_some_and(|d| d.model.get(*s).is_some())
            })
            .collect();
        self.selection.set(slots);
        self.networks.mark = Some(ProblemMark {
            at: p.at,
            label: problem_label(p.kind).to_owned(),
            regions: Vec::new(),
            edges: Vec::new(),
        });
        let r = 25.0;
        let bounds = kentos_geometry_core::geometry::Bounds {
            min_x: p.at.x - r,
            min_y: p.at.y - r,
            max_x: p.at.x + r,
            max_y: p.at.y + r,
        };
        self.navigating(|app| {
            app.viewport.change(ViewChange::Fit {
                bounds,
                padding: 96.0,
            });
        });
        Task::none()
    }

    /// The text of a field the builder edits.
    pub(crate) fn network_field_text(&self, field: Field) -> String {
        let Some(f) = self
            .networks
            .window
            .as_ref()
            .and_then(|w| w.chosen.and_then(|i| w.forms.get(i)))
        else {
            return String::new();
        };
        match field {
            Field::EdgeFilter(i) => f.edges.get(i).map(|e| e.filter.clone()),
            Field::JunctionFilter(i) => f.junctions.get(i).map(|j| j.filter.clone()),
            Field::JunctionClosed(i) => f.junctions.get(i).map(|j| j.closed.clone()),
            Field::Closed => Some(f.closed.clone()),
        }
        .unwrap_or_default()
    }

    /// The layer a field's expression runs on (its row's; the first edge layer's for the closed edges).
    pub(crate) fn network_field_layer(&self, field: Field) -> String {
        let Some(f) = self
            .networks
            .window
            .as_ref()
            .and_then(|w| w.chosen.and_then(|i| w.forms.get(i)))
        else {
            return String::new();
        };
        match field {
            Field::EdgeFilter(i) => f.edges.get(i).map(|e| e.layer.clone()),
            Field::JunctionFilter(i) | Field::JunctionClosed(i) => {
                f.junctions.get(i).map(|j| j.layer.clone())
            }
            Field::Closed => f.edges.first().map(|e| e.layer.clone()),
        }
        .unwrap_or_default()
    }

    pub(crate) fn networks_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.networks.window else {
            return iced::widget::text("").into();
        };
        let mut list = Column::new().spacing(2);
        for (i, f) in w.forms.iter().enumerate() {
            let kind = KINDS
                .iter()
                .find(|(k, _)| *k == f.kind)
                .map_or("", |(_, n)| n);
            let name = if f.name.trim().is_empty() {
                "Adsız ağ".to_owned()
            } else {
                f.name.clone()
            };
            let line = column![
                label::body(name),
                label::caption(format!("{kind}, {} kenar katmanı", f.edges.len()))
            ]
            .spacing(1);
            list = list.push(
                button(line)
                    .width(Fill)
                    .padding([4, 8])
                    .on_press(msg(Event::Choose(i)))
                    .style(style::button::table_row(w.chosen == Some(i), false)),
            );
        }
        if w.forms.is_empty() {
            list = list.push(
                container(label::muted(
                    "Ağ yok. Yeni ağ ile yol ya da şebeke ağı tanımlayın.",
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
                    (w.forms.len() < kentos_contracts::NETWORK_LIMIT).then(|| msg(Event::Add))
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
                "Soldan bir ağ seçin ya da Yeni ağ ile ekleyin.",
            ))
            .padding(8)
            .into(),
            Some(f) => container(scrollable(self.network_form(w, f)).height(Fill))
                .height(Fill)
                .width(Fill)
                .into(),
        };
        let problem = self.window_defs(w).err();
        let missing: Vec<String> = w
            .forms
            .iter()
            .flat_map(|f| {
                f.edges
                    .iter()
                    .map(|e| e.layer.clone())
                    .chain(f.junctions.iter().map(|j| j.layer.clone()))
            })
            .filter(|l| {
                !l.is_empty()
                    && self
                        .document
                        .as_ref()
                        .is_some_and(|d| d.model.layers().get(l).is_none())
            })
            .collect();
        let line = match (&w.said, &problem) {
            (Some((kind, t)), _) => words::text_line(*kind, t.clone()),
            (None, Some(p)) => words::text_line(Say::Error, p.clone()),
            (None, None) if !missing.is_empty() => {
                let mut seen: Vec<String> = Vec::new();
                for m in missing {
                    if !seen.contains(&m) {
                        seen.push(m);
                    }
                }
                words::text_line(
                    Say::Warn,
                    format!(
                        "Çizimde olmayan katman: {}; ağ kurulurken atlanır.",
                        seen.join(", ")
                    ),
                )
            }
            (None, None) if w.forms.is_empty() => words::text_line(Say::Ok, "Ağ yok."),
            (None, None) => words::text_line(
                Say::Ok,
                format!(
                    "{} ağ. En kısa yol, Hizmet alanı ve Şebeke izleme bu ağları kullanır.",
                    w.forms.len()
                ),
            ),
        };
        let ready = problem.is_none() && self.window_changed(w);
        overlay::modal(
            // The panes take what is left of the window's height and scroll themselves; the buttons stay in view.
            Frame::new(TITLE)
                .fill(row![left, right].spacing(16).height(Fill))
                .push(words::summary(vec![line]))
                .action(words::secondary(CANCEL, Some(msg(Event::Cancel))))
                .action(words::primary(SAVE, ready.then(|| msg(Event::Save))))
                .width(1000.0)
                .max_height(660.0),
            msg(Event::Cancel),
        )
    }

    fn network_form<'a>(&'a self, w: &'a Window, f: &'a NetworkForm) -> Element<'a, Message> {
        let format = self.format();
        let layers = self.network_layers();
        let input = |hint: &'a str,
                     value: &'a str,
                     on: Box<dyn Fn(String) -> Event + 'a>|
         -> Element<'a, Message> {
            text_input(hint, value)
                .size(typography::body())
                .padding([4, 8])
                .on_input(move |t| msg(on(t)))
                .style(style::field::input)
                .into()
        };
        let number =
            |value: &'a str, on: Box<dyn Fn(String) -> Event + 'a>| -> Element<'a, Message> {
                container(
                    text_input("", value)
                        .size(typography::body())
                        .padding([4, 8])
                        .align_x(iced::alignment::Horizontal::Right)
                        .on_input(move |t| msg(on(t)))
                        .style(style::field::input),
                )
                .width(Length::Fixed(typography::scaled(96.0)))
                .into()
            };
        let labelled = |name: &'a str, body: Element<'a, Message>| -> Element<'a, Message> {
            row![
                container(label::body(name)).width(Length::Fixed(typography::scaled(96.0))),
                body
            ]
            .spacing(8)
            .align_y(Center)
            .into()
        };
        let head = |name: &'a str, action: Option<Element<'a, Message>>| -> Element<'a, Message> {
            let mut r = row![label::caption(name), Space::new().width(Fill)].align_y(Center);
            if let Some(a) = action {
                r = r.push(a);
            }
            column![r, kentos_ui::widget::horizontal_divider()]
                .spacing(4)
                .into()
        };
        let small =
            |glyph: &str, words: &'static str, on: Option<Message>| -> Element<'a, Message> {
                button(
                    row![
                        icon(from_web(Some(glyph))).size(12.0),
                        label::caption(words)
                    ]
                    .spacing(4)
                    .align_y(Center),
                )
                .style(style::button::secondary)
                .padding([2, 8])
                .on_press_maybe(on)
                .into()
            };
        let remove = |on: Message| -> Element<'a, Message> {
            button(icon(from_web(Some("close"))).size(12.0))
                .style(style::button::ghost)
                .padding(4)
                .on_press(on)
                .into()
        };
        let builder = |field: Field| -> Element<'a, Message> {
            button(icon(from_web(Some("expression"))).size(14.0))
                .style(style::button::ghost)
                .padding(4)
                .on_press(msg(Event::Builder(field)))
                .into()
        };
        let layer_select =
            |current: &str, on: fn(usize, usize) -> Event, i: usize| -> Element<'a, Message> {
                let mut choices: Vec<Choice> =
                    layers.iter().map(|(_, p)| Choice::new(p.clone())).collect();
                let mut at = layers.iter().position(|(id, _)| id == current);
                if at.is_none() && !current.is_empty() {
                    choices.push(Choice::new(format!("(çizimde yok: {current})")));
                    at = Some(choices.len() - 1);
                }
                Select::new(choices, at, move |k| msg(on(i, k))).into()
            };
        let mut form = Column::new().spacing(8).padding([0, 8]);
        form = form.push(labelled(
            "Ad",
            input("Ağın adı", &f.name, Box::new(Event::Name)),
        ));
        form = form.push(labelled(
            "Tür",
            Select::new(
                KINDS.iter().map(|(_, n)| Choice::new(*n)),
                KINDS.iter().position(|(k, _)| *k == f.kind),
                |k| msg(Event::Kind(k)),
            )
            .into(),
        ));
        // Edge layers.
        form = form.push(head(
            "Kenar katmanları",
            Some(small(
                "plus",
                "Katman ekle",
                (f.edges.len() < kentos_contracts::NETWORK_LAYER_LIMIT)
                    .then(|| msg(Event::EdgeAdd)),
            )),
        ));
        for (i, e) in f.edges.iter().enumerate() {
            form = form.push(
                row![
                    container(layer_select(&e.layer, Event::EdgeLayer, i))
                        .width(Length::FillPortion(2)),
                    container(input(
                        "süzgeç (boş: hepsi)",
                        &e.filter,
                        Box::new(move |t| Event::Text(Field::EdgeFilter(i), t))
                    ))
                    .width(Length::FillPortion(3)),
                    builder(Field::EdgeFilter(i)),
                    remove(msg(Event::EdgeRemove(i))),
                ]
                .spacing(6)
                .align_y(Center),
            );
        }
        if f.edges.is_empty() {
            form = form.push(label::muted("Kenar katmanı yok: ağın çizgileri, çoklu çizgileri ve yayları bir katmanda olmalı."));
        }
        // Junction layers.
        form = form.push(head(
            "Düğüm katmanları",
            Some(small(
                "plus",
                "Katman ekle",
                (f.junctions.len() < kentos_contracts::NETWORK_LAYER_LIMIT)
                    .then(|| msg(Event::JunctionAdd)),
            )),
        ));
        for (i, j) in f.junctions.iter().enumerate() {
            form = form.push(
                row![
                    container(layer_select(&j.layer, Event::JunctionLayer, i))
                        .width(Length::FillPortion(2)),
                    container(Select::new(
                        ROLES.iter().map(|(_, n)| Choice::new(*n)),
                        ROLES.iter().position(|(r, _)| *r == j.role),
                        move |k| msg(Event::JunctionRole(i, k))
                    ))
                    .width(Length::Fixed(typography::scaled(100.0))),
                    container(input(
                        "süzgeç",
                        &j.filter,
                        Box::new(move |t| Event::Text(Field::JunctionFilter(i), t))
                    ))
                    .width(Length::FillPortion(2)),
                    builder(Field::JunctionFilter(i)),
                    container(input(
                        "kapalı ifadesi",
                        &j.closed,
                        Box::new(move |t| Event::Text(Field::JunctionClosed(i), t))
                    ))
                    .width(Length::FillPortion(2)),
                    builder(Field::JunctionClosed(i)),
                    remove(msg(Event::JunctionRemove(i))),
                ]
                .spacing(6)
                .align_y(Center),
            );
        }
        if f.junctions.is_empty() {
            form = form.push(label::muted("Düğüm katmanı yok: bağlantılar çizgilerin uçlarındadır. Vanalar ve kaynaklar için nokta katmanı ekleyin."));
        }
        // Connecting.
        form = form.push(head("Bağlanma", None));
        form = form.push(labelled(
            "Uçlar",
            row![
                container(Select::new(
                    CONNECTS.iter().map(|(_, n)| Choice::new(*n)),
                    CONNECTS.iter().position(|(c, _)| *c == f.connect),
                    |k| msg(Event::Connect(k))
                ))
                .width(Length::Fixed(typography::scaled(150.0))),
                label::caption("Tolerans"),
                number(&f.tolerance, Box::new(Event::Tolerance)),
                label::caption(format.length_unit_label()),
            ]
            .spacing(8)
            .align_y(Center)
            .into(),
        ));
        // Direction.
        form = form.push(head("Yön", None));
        form = form.push(labelled(
            "Yön",
            Select::new(
                DIRECTIONS.iter().map(|(_, n)| Choice::new(*n)),
                DIRECTIONS.iter().position(|(d, _)| *d == f.direction),
                |k| msg(Event::Direction(k)),
            )
            .into(),
        ));
        if f.direction == DirectionKind::Field {
            form = form.push(labelled(
                "Alan",
                input("yön alanı", &f.field, Box::new(Event::DirectionField)),
            ));
            form = form.push(labelled(
                "İleri",
                input("FT, ileri", &f.forward, Box::new(Event::Forward)),
            ));
            form = form.push(labelled(
                "Geri",
                input("TF, geri", &f.backward, Box::new(Event::Backward)),
            ));
            form = form.push(labelled(
                "Kapalı",
                input("N, kapalı", &f.shut, Box::new(Event::Shut)),
            ));
        }
        // Costs.
        form = form.push(head(
            "Maliyetler (Uzunluk her ağda vardır)",
            Some(small(
                "plus",
                "Maliyet ekle",
                (f.costs.len() < kentos_contracts::NETWORK_COST_LIMIT).then(|| msg(Event::CostAdd)),
            )),
        ));
        for (i, c) in f.costs.iter().enumerate() {
            let last: Element<'a, Message> = if c.kind == NetworkCostKind::Speed {
                row![
                    number(&c.speed, Box::new(move |t| Event::CostSpeed(i, t))),
                    label::caption("km/sa")
                ]
                .spacing(4)
                .align_y(Center)
                .into()
            } else {
                container(input(
                    "birim",
                    &c.unit,
                    Box::new(move |t| Event::CostUnit(i, t)),
                ))
                .width(Length::Fixed(typography::scaled(120.0)))
                .into()
            };
            form = form.push(
                row![
                    container(input(
                        "Maliyetin adı",
                        &c.name,
                        Box::new(move |t| Event::CostName(i, t))
                    ))
                    .width(Length::FillPortion(2)),
                    container(Select::new(
                        COST_KINDS.iter().map(|(_, n)| Choice::new(*n)),
                        COST_KINDS.iter().position(|(k, _)| *k == c.kind),
                        move |k| msg(Event::CostKind(i, k))
                    ))
                    .width(Length::Fixed(typography::scaled(130.0))),
                    container(input(
                        if c.kind == NetworkCostKind::Speed {
                            "hız alanı"
                        } else {
                            "maliyet alanı"
                        },
                        &c.field,
                        Box::new(move |t| Event::CostField(i, t))
                    ))
                    .width(Length::FillPortion(2)),
                    last,
                    remove(msg(Event::CostRemove(i))),
                ]
                .spacing(6)
                .align_y(Center),
            );
        }
        // Closed edges.
        form = form.push(head("Kapalı kenarlar", None));
        form = form.push(labelled(
            "İfade",
            row![
                container(input(
                    "ifade (boş: hiçbiri)",
                    &f.closed,
                    Box::new(|t| Event::Text(Field::Closed, t))
                ))
                .width(Fill),
                builder(Field::Closed)
            ]
            .spacing(6)
            .align_y(Center)
            .into(),
        ));
        // Denetle.
        form = form.push(head(
            CHECK,
            Some(small("networks", CHECK, Some(msg(Event::Check)))),
        ));
        form = form.push(self.check_view(w, &format));
        form.into()
    }

    /// Denetle's answer: its counts, its parts and its problems, each a row that shows it in the drawing.
    fn check_view<'a>(&'a self, w: &'a Window, format: &Format) -> Element<'a, Message> {
        let Some(c) = &w.check else {
            return label::muted("Denetle ağı bu tanımla kurar: düğüm, parça ve uzunluğunu, kopuk parçaları ve sorunları gösterir.").into();
        };
        let mut col = Column::new().spacing(6);
        if let Some(words) = &c.words {
            col = col.push(words::text_line(
                if c.value.is_some() || c.ticket.is_some() {
                    Say::Info
                } else {
                    Say::Error
                },
                words.clone(),
            ));
        }
        let Some(v) = &c.value else {
            return col.into();
        };
        let detached: f64 = v.parts.iter().skip(1).map(|p| p.1).sum();
        let parts = if v.parts.len() > 1 {
            format!(
                " (kopuk {}: {})",
                v.parts.len() - 1,
                format.length(detached)
            )
        } else {
            String::new()
        };
        col = col.push(words::text_line(
            if v.problems.is_empty() {
                Say::Ok
            } else {
                Say::Warn
            },
            format!(
                "{} düğüm, {} parça, {}; {} bağlı parça{parts}; {} çıkmaz uç.",
                v.nodes,
                v.pieces,
                format.length(v.length),
                v.parts.len(),
                v.dead_ends
            ),
        ));
        if !v.counts.is_empty() {
            col = col.push(label::caption(
                v.counts
                    .iter()
                    .map(|(k, n)| format!("{}: {n}", problem_label(*k)))
                    .collect::<Vec<_>>()
                    .join(", "),
            ));
        }
        if v.problems.is_empty() {
            return col.into();
        }
        let mut rows = Column::new().spacing(1);
        for (i, p) in v.problems.iter().take(SHOWN_PROBLEMS).enumerate() {
            let ids = if p.ids.is_empty() {
                String::new()
            } else {
                format!("{} nesne", p.ids.len())
            };
            let detail = match (p.kind, p.value) {
                (ProblemKind::NearMiss, Some(d)) => format!("{ids}; {} arayla", format.length(d)),
                (ProblemKind::Short, Some(d)) => format!("{ids}; {}", format.length(d)),
                (ProblemKind::Detached, Some(d)) => format.length(d),
                (ProblemKind::Unread, _) => {
                    let what = match p.cost {
                        None => "yön".to_owned(),
                        Some(c) => w
                            .chosen
                            .and_then(|f| w.forms.get(f))
                            .and_then(|f| f.costs.get(c.saturating_sub(1)))
                            .map_or("maliyet".to_owned(), |c| c.name.clone()),
                    };
                    format!("{ids}; {what} okunamadı")
                }
                _ => ids,
            };
            let line = row![
                container(label::body(problem_label(p.kind)))
                    .width(Length::Fixed(typography::scaled(170.0))),
                container(label::caption(format.point(p.at))).width(Length::FillPortion(1)),
                container(label::caption(detail)).width(Length::FillPortion(1)),
            ]
            .spacing(8)
            .align_y(Center);
            rows = rows.push(
                button(line)
                    .width(Fill)
                    .padding([3, 8])
                    .on_press(msg(Event::Problem(i)))
                    .style(style::button::table_row(false, false)),
            );
        }
        col = col.push(
            container(scrollable(rows).height(Length::Fixed(typography::scaled(200.0))))
                .style(style::container::field_box)
                .width(Fill),
        );
        if v.problems.len() > SHOWN_PROBLEMS {
            col = col.push(label::muted(format!(
                "İlk {SHOWN_PROBLEMS} sorun gösteriliyor; {} sorun daha var.",
                v.problems.len() - SHOWN_PROBLEMS
            )));
        }
        col.into()
    }

    /// The window's controls by their words (a trace's `dialog` step): the chosen network's name, tolerance and
    /// closed expression, a network by its number, the buttons.
    pub(crate) fn networks_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        let Some(w) = &self.networks.window else {
            return Err(format!("{TITLE} penceresi açık değil"));
        };
        Ok(match control {
            Control::Fill("Ad", t) => Some(msg(Event::Name(t.to_owned()))),
            Control::Fill("Tolerans", t) => Some(msg(Event::Tolerance(t.to_owned()))),
            Control::Fill("Kapalı kenarlar", t) => {
                Some(msg(Event::Text(Field::Closed, t.to_owned())))
            }
            Control::Row(n) if n >= 1 && n <= w.forms.len() => Some(msg(Event::Choose(n - 1))),
            Control::Press(SAVE) => self.window_changed(w).then(|| msg(Event::Save)),
            Control::Press(CANCEL) => Some(msg(Event::Cancel)),
            Control::Press(ADD) => Some(msg(Event::Add)),
            Control::Press(REMOVE) => w.chosen.map(|_| msg(Event::Remove)),
            Control::Press(CHECK) => w.chosen.map(|_| msg(Event::Check)),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

/// A network with only its id (`next_network_id` reads the ids).
fn def_shell(id: &str) -> NetworkDef {
    NetworkDef {
        id: id.to_owned(),
        name: String::new(),
        kind: NetworkKind::Road,
        edges: Vec::new(),
        junctions: Vec::new(),
        connect: NetworkConnect::Ends,
        tolerance: kentos_contracts::NETWORK_TOLERANCE,
        direction: kentos_contracts::NetworkDirection::Both,
        costs: Vec::new(),
        closed: None,
    }
}

/// An edit of the chosen network's form.
fn edit(f: &mut NetworkForm, e: Event, layers: &[(String, String)]) {
    let layer = |k: usize| layers.get(k).map(|(id, _)| id.clone());
    match e {
        Event::Name(t) => f.name = t,
        Event::Kind(k) => {
            if let Some(&(kind, _)) = KINDS.get(k) {
                *f = retyped(f, kind);
            }
        }
        Event::EdgeLayer(i, k) => {
            if let (Some(e), Some(id)) = (f.edges.get_mut(i), layer(k)) {
                e.layer = id;
            }
        }
        Event::EdgeAdd => f.edges.push(EdgeRow {
            layer: layers.first().map(|(id, _)| id.clone()).unwrap_or_default(),
            filter: String::new(),
        }),
        Event::EdgeRemove(i) => {
            if i < f.edges.len() {
                f.edges.remove(i);
            }
        }
        Event::JunctionLayer(i, k) => {
            if let (Some(j), Some(id)) = (f.junctions.get_mut(i), layer(k)) {
                j.layer = id;
            }
        }
        Event::JunctionRole(i, k) => {
            if let (Some(j), Some(&(role, _))) = (f.junctions.get_mut(i), ROLES.get(k)) {
                j.role = role;
            }
        }
        Event::JunctionAdd => f.junctions.push(JunctionRow {
            layer: String::new(),
            role: if f.kind == NetworkKind::Utility {
                JunctionRole::Valve
            } else {
                JunctionRole::Junction
            },
            filter: String::new(),
            closed: String::new(),
        }),
        Event::JunctionRemove(i) => {
            if i < f.junctions.len() {
                f.junctions.remove(i);
            }
        }
        Event::Text(field, t) => match field {
            Field::EdgeFilter(i) => {
                if let Some(e) = f.edges.get_mut(i) {
                    e.filter = t;
                }
            }
            Field::JunctionFilter(i) => {
                if let Some(j) = f.junctions.get_mut(i) {
                    j.filter = t;
                }
            }
            Field::JunctionClosed(i) => {
                if let Some(j) = f.junctions.get_mut(i) {
                    j.closed = t;
                }
            }
            Field::Closed => f.closed = t,
        },
        Event::Connect(k) => {
            if let Some(&(c, _)) = CONNECTS.get(k) {
                f.connect = c;
            }
        }
        Event::Tolerance(t) => f.tolerance = t,
        Event::Direction(k) => {
            if let Some(&(d, _)) = DIRECTIONS.get(k) {
                f.direction = d;
                if d == DirectionKind::Field
                    && f.field.is_empty()
                    && f.forward.is_empty()
                    && f.backward.is_empty()
                    && f.shut.is_empty()
                {
                    let [field, forward, backward, shut] = DIRECTION_DEFAULTS;
                    f.field = field.into();
                    f.forward = forward.into();
                    f.backward = backward.into();
                    f.shut = shut.into();
                }
            }
        }
        Event::DirectionField(t) => f.field = t,
        Event::Forward(t) => f.forward = t,
        Event::Backward(t) => f.backward = t,
        Event::Shut(t) => f.shut = t,
        Event::CostName(i, t) => {
            if let Some(c) = f.costs.get_mut(i) {
                c.name = t;
            }
        }
        Event::CostKind(i, k) => {
            if let (Some(c), Some(&(kind, _))) = (f.costs.get_mut(i), COST_KINDS.get(k)) {
                c.kind = kind;
            }
        }
        Event::CostField(i, t) => {
            if let Some(c) = f.costs.get_mut(i) {
                c.field = t;
            }
        }
        Event::CostUnit(i, t) => {
            if let Some(c) = f.costs.get_mut(i) {
                c.unit = t;
            }
        }
        Event::CostSpeed(i, t) => {
            if let Some(c) = f.costs.get_mut(i) {
                c.speed = t;
            }
        }
        Event::CostAdd => {
            let n = f.costs.len();
            f.costs.push(CostRow {
                name: if n == 0 {
                    "Süre".into()
                } else {
                    format!("Maliyet {}", n + 1)
                },
                kind: if n == 0 {
                    NetworkCostKind::Speed
                } else {
                    NetworkCostKind::Field
                },
                field: String::new(),
                unit: String::new(),
                speed: String::new(),
            });
        }
        Event::CostRemove(i) if i < f.costs.len() => {
            f.costs.remove(i);
        }
        _ => {}
    }
}
