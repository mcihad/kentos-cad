//! Senaryo oluştur and Senaryoyu uygula (docs/adr/0210 §9, §10; the web's
//! `ui/time/ScenarioDialog.ts` and `app/scenarios.ts`).
//!
//! - Senaryo oluştur: Ad, Not, the base layers to copy (the active one
//!   checked; none, an empty scenario), Nesneleri kopyala. A locked layer's
//!   objects are not copied: it can be checked only with Nesneleri kopyala
//!   off. Oluştur writes through `cad.scenarios.edit` (one step “Senaryo
//!   oluştur”), then the scenario is shown.
//! - Senaryoyu uygula asks first, then writes through `cad.scenarios.edit`
//!   (one step “Senaryoyu uygula”) and shows Mevcut durum, the group it
//!   leaves shown.

use std::collections::BTreeSet;

use iced::widget::{Column, button, container, row, text, text_input};
use iced::{Center, Element, Fill, Task};
use kentos_contracts::{CommandResult, SCENARIO_NAME_MAX, ScenarioOperation, ScenariosEdit};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, scenarios_edit};
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::tree_view::{Check, check_box};
use kentos_ui::widget::{Confirm, Dialog as Frame, Elided, VirtualList, overlay};
use kentos_ui::{label, style};

use super::{Event as TimeEvent, msg as time_msg, pairs_of, scenario_of};
use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Senaryo oluştur";
/// The question's title.
pub const APPLY_TITLE: &str = "Senaryoyu uygula";
const CREATE: &str = "Oluştur";
const CANCEL: &str = "Vazgeç";
const APPLY: &str = "Uygula";
const COPY: &str = "Nesneleri kopyala";

/// A row's height in the list of base layers, logical pixels.
const ROW: f32 = 26.0;

/// Senaryo oluştur's fields, or the scenario Senaryoyu uygula asks about.
#[derive(Debug, Clone)]
pub enum Window {
    Create {
        name: String,
        note: String,
        checked: BTreeSet<String>,
        copy: bool,
    },
    Apply(String),
}

#[derive(Debug, Clone)]
pub enum Event {
    Name(String),
    Note(String),
    Check(String, bool),
    Copy,
    Create,
    Apply,
    Cancel,
}

fn msg(event: Event) -> Message {
    time_msg(TimeEvent::Scenario(event))
}

impl App {
    /// The base layers: those holding objects, in no scenario.
    fn base_layers(&self) -> Vec<String> {
        let Some(doc) = &self.document else {
            return Vec::new();
        };
        let layers = doc.model.layers();
        layers
            .leaves()
            .into_iter()
            .filter(|l| l.service.is_none() && scenario_of(layers, &l.id).is_none())
            .map(|l| l.id.clone())
            .collect()
    }

    /// Senaryo oluştur (`scenario.create`): the window, the active layer checked when it is a base layer.
    pub(crate) fn open_scenario_create(&mut self) {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        let layers = doc.model.layers();
        let active = layers.active().to_owned();
        let bases = self.base_layers();
        let mut n = 1;
        let taken: Vec<String> = {
            fn names(nodes: &[kentos_contracts::LayerNode], out: &mut Vec<String>) {
                for x in nodes {
                    out.push(x.name.clone());
                    names(&x.children, out);
                }
            }
            let mut out = Vec::new();
            names(layers.nodes(), &mut out);
            out
        };
        while taken.contains(&format!("Senaryo {n}")) {
            n += 1;
        }
        self.time.scenario = Some(Window::Create {
            name: format!("Senaryo {n}"),
            note: String::new(),
            checked: bases.iter().filter(|id| **id == active).cloned().collect(),
            copy: true,
        });
        self.dialog = Some(Dialog::Scenario);
    }

    /// Senaryoyu uygula (`scenario.apply`): asks about `id`, else the scenario at hand.
    pub(crate) fn ask_scenario_apply(&mut self, id: Option<String>) {
        let Some(id) = id.or_else(|| self.scenario_at_hand()) else {
            self.warn("Projede senaryo yok; önce Senaryo oluştur ile bir senaryo yapın.");
            return;
        };
        self.time.scenario = Some(Window::Apply(id));
        self.dialog = Some(Dialog::Scenario);
    }

    pub(crate) fn scenario_event(&mut self, event: Event) -> Task<Message> {
        let Some(w) = self.time.scenario.as_mut() else {
            return Task::none();
        };
        match (w, event) {
            (Window::Create { name, .. }, Event::Name(t)) => *name = t,
            (Window::Create { note, .. }, Event::Note(t)) => *note = t,
            (Window::Create { checked, .. }, Event::Check(id, on)) => {
                if on {
                    checked.insert(id);
                } else {
                    checked.remove(&id);
                }
            }
            (Window::Create { copy, checked, .. }, Event::Copy) => {
                *copy = !*copy;
                // A locked layer checked while copying is left out (its objects would not be copied).
                if *copy && let Some(doc) = &self.document {
                    checked.retain(|id| !doc.model.layers().is_locked(id));
                }
            }
            (Window::Create { .. }, Event::Create) => {
                if self.create_scenario().is_some() {
                    self.time.scenario = None;
                    self.dialog = None;
                }
            }
            (Window::Apply(id), Event::Apply) => {
                let id = id.clone();
                self.time.scenario = None;
                self.dialog = None;
                self.apply_scenario(&id);
            }
            (_, Event::Cancel) => {
                self.time.scenario = None;
                self.dialog = None;
            }
            _ => {}
        }
        Task::none()
    }

    /// Senaryo oluştur through `cad.scenarios.edit`, then the scenario shown. Its group's id, or none when refused (said).
    fn create_scenario(&mut self) -> Option<String> {
        let bases = self.base_layers();
        let Some(Window::Create {
            name,
            note,
            checked,
            copy,
        }) = self.time.scenario.clone()
        else {
            return None;
        };
        let doc = self.document.as_mut()?;
        let layers: Vec<String> = bases
            .into_iter()
            .filter(|id| checked.contains(id))
            .collect();
        let count = layers.len();
        let note = note.trim().to_owned();
        let input = ScenariosEdit {
            operation: ScenarioOperation::Create,
            name: Some(name.clone()),
            layers: Some(layers),
            copy_objects: Some(copy),
            note: (!note.is_empty()).then_some(note),
            scenario: None,
            expected_revision: None,
        };
        match scenarios_edit::execute(&mut ExecutionContext::new(&mut doc.model), input) {
            CommandResult::Completed { output, .. } => {
                let id = output.scenario.clone();
                self.show_scenario(&id);
                self.say(
                    Level::Success,
                    format!(
                        "Senaryo oluşturuldu: {} ({count} katman, {} nesne kopyalandı).",
                        name.trim(),
                        output.objects
                    ),
                );
                Some(id)
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => {
                self.warn(error.message);
                None
            }
            _ => None,
        }
    }

    /// Senaryoyu uygula through `cad.scenarios.edit`, then Mevcut durum with the group it leaves shown.
    pub(crate) fn apply_scenario(&mut self, id: &str) -> bool {
        let Some(doc) = self.document.as_mut() else {
            return false;
        };
        let name = doc
            .model
            .layers()
            .get(id)
            .map_or_else(|| id.to_owned(), |n| n.name.clone());
        let input = ScenariosEdit {
            operation: ScenarioOperation::Apply,
            name: None,
            layers: None,
            copy_objects: None,
            note: None,
            scenario: Some(id.to_owned()),
            expected_revision: None,
        };
        match scenarios_edit::execute(&mut ExecutionContext::new(&mut doc.model), input) {
            CommandResult::Completed { output, .. } => {
                self.show_base();
                if let Some(doc) = self.document.as_mut() {
                    let model = &mut doc.model;
                    // The layers it kept are base layers now: shown, with their group.
                    if model.layers().get(id).is_some() {
                        model.set_layer_visible(id, true);
                    }
                    for p in &output.layers {
                        model.set_layer_visible(&p.base, true);
                    }
                }
                self.say(
                    Level::Success,
                    format!(
                        "Senaryo uygulandı: {name} ({} katman; {} nesne taşındı, {} nesne silindi).",
                        output.layers.len(),
                        output.objects,
                        output.removed
                    ),
                );
                true
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => {
                self.warn(error.message);
                false
            }
            _ => false,
        }
    }

    pub(crate) fn scenario_view(&self) -> Element<'_, Message> {
        let (Some(w), Some(doc)) = (&self.time.scenario, &self.document) else {
            return text("").into();
        };
        let layers = doc.model.layers();
        match w {
            Window::Apply(id) => {
                let model = &doc.model;
                let group = layers.get(id).map_or(id.as_str(), |n| n.name.as_str());
                let pairs = pairs_of(layers, id);
                let mut details: Vec<String> = pairs
                    .iter()
                    .map(|(b, l)| {
                        format!(
                            "“{}”: {} nesne silinir, “{}” katmanının {} nesnesi gelir.",
                            self.layer_path(b),
                            model.by_layer(b).count(),
                            layers.get(l).map_or(l.as_str(), |n| n.name.as_str()),
                            model.by_layer(l).count()
                        )
                    })
                    .collect();
                if pairs.is_empty() {
                    details.push("Senaryoda bir ana katmanın yerine geçen katman yok; grubu sıradan gruba döner.".to_owned());
                }
                let removed: usize = pairs.iter().map(|(b, _)| model.by_layer(b).count()).sum();
                details.push(format!(
                    "Toplam {removed} nesne silinir; tek adımda geri alınır."
                ));
                overlay::modal(
                    Confirm::new(APPLY_TITLE, msg(Event::Apply), msg(Event::Cancel))
                        .message(format!("“{group}” senaryosu Mevcut durum’a yazılsın mı?"))
                        .detail(details.join("\n"))
                        .confirm(APPLY),
                    msg(Event::Cancel),
                )
            }
            Window::Create {
                name,
                note,
                checked,
                copy,
            } => {
                let bases = self.base_layers();
                let counts: Vec<usize> = bases
                    .iter()
                    .map(|id| doc.model.by_layer(id).count())
                    .collect();
                let objects: usize = if *copy {
                    bases
                        .iter()
                        .zip(&counts)
                        .filter(|(id, _)| checked.contains(*id))
                        .map(|(_, n)| n)
                        .sum()
                } else {
                    0
                };
                let words_now = name.trim();
                let said = if words_now.is_empty() {
                    (Kind::Warn, "Senaryonun adını yazın.".to_owned())
                } else if checked.is_empty() {
                    (
                        Kind::Info,
                        "Katman seçilmedi: boş bir senaryo oluşur; önerinin katmanlarını sonra ekleyebilirsiniz.".to_owned(),
                    )
                } else {
                    (
                        Kind::Ok,
                        format!(
                            "{} ana katman kopyalanır{}; senaryo ağacın en üstüne gelir ve gösterilir.",
                            checked.len(),
                            if *copy {
                                format!(", {objects} nesneyle")
                            } else {
                                ", nesnesiz".to_owned()
                            }
                        ),
                    )
                };
                let name_field = text_input("", name)
                    .on_input(|t| msg(Event::Name(t)))
                    .size(typography::body())
                    .padding([5, 8])
                    .style(style::field::input);
                let note_field = text_input("Önerinin kısa açıklaması (isteğe bağlı)", note)
                    .on_input(|t| msg(Event::Note(t)))
                    .size(typography::body())
                    .padding([5, 8])
                    .style(style::field::input);
                let head = row![
                    container(text("")).width(typography::scaled(28.0)),
                    container(label::caption("Ana katman")).width(Fill),
                    container(label::caption("Nesne")).width(typography::scaled(56.0)),
                ]
                .spacing(8)
                .padding([4, 8]);
                let rows: Vec<(String, String, bool, usize)> = bases
                    .iter()
                    .zip(&counts)
                    .map(|(id, n)| (id.clone(), self.layer_path(id), layers.is_locked(id), *n))
                    .collect();
                let checked_now = checked.clone();
                let copying = *copy;
                let list = VirtualList::new(rows.len(), typography::scaled(ROW), move |i| {
                    let (id, path, locked, n) = &rows[i];
                    let blocked = *locked && copying;
                    let on = checked_now.contains(id);
                    let boxed = check_box(
                        if on { Check::Checked } else { Check::Unchecked },
                        (!blocked).then(|| msg(Event::Check(id.clone(), !on))),
                    );
                    let quiet = blocked;
                    let name = Elided::new(path.clone())
                        .size(typography::caption())
                        .font(typography::ui())
                        .style(move |theme: &iced::Theme| {
                            let t = Tokens::of(theme);
                            iced::widget::text::Style {
                                color: Some(if quiet { t.muted } else { t.text }),
                            }
                        })
                        .width(Fill);
                    let name = if *locked {
                        row![icon(Icon::Lock).size(12.0).tone(Tone::Muted), name]
                            .spacing(4)
                            .align_y(Center)
                            .width(Fill)
                    } else {
                        row![name].width(Fill)
                    };
                    let line = button(row![name].align_y(Center))
                        .on_press_maybe((!blocked).then(|| msg(Event::Check(id.clone(), !on))))
                        .padding(0)
                        .style(style::button::ghost)
                        .width(Fill);
                    row![
                        container(boxed)
                            .width(typography::scaled(28.0))
                            .center_x(typography::scaled(28.0)),
                        line,
                        container(label::caption(n.to_string())).width(typography::scaled(56.0)),
                    ]
                    .spacing(8)
                    .padding([0, 8])
                    .height(typography::scaled(ROW))
                    .align_y(Center)
                    .into()
                })
                .height(typography::scaled(200.0));
                let table = container(Column::new().push(head).push(list))
                    .style(style::container::field_box)
                    .width(Fill);
                let body = Column::new()
                    .spacing(12)
                    .push(words::field("Ad", name_field, None))
                    .push(words::field("Not", note_field, None))
                    .push(words::field("Kopyalanacak ana katmanlar", table, None))
                    .push(words::field(
                        "Nesneler",
                        words::check(*copy, COPY, Some(msg(Event::Copy))),
                        None,
                    ))
                    .push(words::summary(vec![words::text_line(said.0, said.1)]));
                let ready = !words_now.is_empty() && words_now.chars().count() <= SCENARIO_NAME_MAX;
                overlay::modal(
                    Frame::new(TITLE)
                        .push(body)
                        .action(words::secondary(CANCEL, Some(msg(Event::Cancel))))
                        .action(words::primary(CREATE, ready.then(|| msg(Event::Create))))
                        .width(600.0),
                    msg(Event::Cancel),
                )
            }
        }
    }

    /// The window's title (a trace's `dialog` step): Senaryo oluştur, or the question's.
    pub(crate) fn scenario_title(&self) -> String {
        match &self.time.scenario {
            Some(Window::Apply(_)) => APPLY_TITLE.to_owned(),
            _ => TITLE.to_owned(),
        }
    }

    /// The window's controls by their words: Ad and Not (`fill`), a layer's box by its path, Nesneleri kopyala,
    /// Oluştur, Vazgeç; the question's Uygula and Vazgeç.
    pub(crate) fn scenario_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        let Some(w) = &self.time.scenario else {
            return Err(format!("{TITLE} penceresi açık değil"));
        };
        Ok(match (w, control) {
            (Window::Create { .. }, Control::Fill("Ad", t)) => Some(msg(Event::Name(t.to_owned()))),
            (Window::Create { .. }, Control::Fill("Not", t)) => {
                Some(msg(Event::Note(t.to_owned())))
            }
            (Window::Create { checked, .. }, Control::Check(words, on)) if words != COPY => {
                let Some(id) = self
                    .base_layers()
                    .into_iter()
                    .find(|id| self.layer_path(id) == words)
                else {
                    return Err(format!("“{TITLE}” penceresinde “{words}” kutusu yok"));
                };
                (checked.contains(&id) != on).then(|| msg(Event::Check(id, on)))
            }
            (Window::Create { copy, .. }, Control::Check(COPY, on)) => {
                (*copy != on).then(|| msg(Event::Copy))
            }
            (Window::Create { name, .. }, Control::Press(CREATE)) => {
                (!name.trim().is_empty()).then(|| msg(Event::Create))
            }
            (Window::Apply(_), Control::Press(APPLY)) => Some(msg(Event::Apply)),
            (_, Control::Press(CANCEL)) => Some(msg(Event::Cancel)),
            (_, other) => {
                return Err(format!(
                    "“{}” penceresinde {other} yok",
                    self.scenario_title()
                ));
            }
        })
    }
}
