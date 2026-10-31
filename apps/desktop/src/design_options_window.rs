//! Tasarım seçenekleri (docs/adr/0788; `arch.options`): the project's design option sets, each with its options (which
//! one is primary, how many objects of which kinds, its spaces' area and its walls' length, objects gone). Seçimden
//! tanımla makes the selection a new set's first option; Seçeneği kopyala adds an independent copy of the chosen
//! option; Birincil yap makes the drawing and the sheets show it; Karşılaştır reads two options side by side (counts,
//! areas and the plan difference); Kabul et keeps the chosen option as the main model and deletes the others, its plan
//! said first; Seçeneği sil removes a non-primary option with its objects. Each write is one undo step (“Tasarım
//! seçeneği”); Kapat writes nothing.

use iced::widget::{Column, button, container, row};
use iced::{Center, Element, Fill, Task};
use kentos_commands::{
    CommandResult, OptionSetRow, OptionsCompare, OptionsCompared, OptionsEdit, OptionsEdited,
    OptionsOperation, OptionsQuery,
};
use kentos_interaction::Level;
use kentos_ui::icon::icon;
use kentos_ui::widget::Dialog as Frame;
use kentos_ui::widget::{Choice, Select};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Tasarım seçenekleri";
const CLOSE: &str = "Kapat";
const DEFINE: &str = "Seçimden tanımla";
const COPY: &str = "Seçeneği kopyala";
const PRIMARY: &str = "Birincil yap";
const ACCEPT: &str = "Kabul et";
const DELETE: &str = "Seçeneği sil";
const ADD: &str = "Seçimi ekle";
const REMOVE: &str = "Seçimi çıkar";

#[derive(Clone, Debug, Default)]
pub struct Window {
    pub rows: Vec<OptionSetRow>,
    pub chosen: usize,
    pub option: usize,
    pub name: String,
    /// The two options compared (indices into the chosen set's), and what the comparison says.
    pub compare: (usize, usize),
    pub compared: Option<Result<OptionsCompared, String>>,
    /// What Kabul et with the chosen option would do; its refusal's words when refused.
    pub accept: Option<Result<OptionsEdited, String>>,
}

#[derive(Clone, Debug)]
pub enum Event {
    Choose(usize),
    Option(usize),
    Name(String),
    CompareA(usize),
    CompareB(usize),
    Define,
    Copy,
    Primary,
    Add,
    Remove,
    Accept,
    Delete,
    Close,
}

fn msg(e: Event) -> Message {
    Message::DesignOptions(e)
}

/// A number with two decimals and a decimal comma, a sign when asked.
fn num(v: f64, signed: bool) -> String {
    let t = format!("{:.2}", v.abs()).replace('.', ",");
    if signed && v > 0.0005 {
        format!("+{t}")
    } else if v < -0.0005 {
        format!("−{t}")
    } else {
        t
    }
}

/// A kind's name as the window says it.
fn kind_words(kind: &str) -> &str {
    match kind {
        "wall" => "duvar",
        "opening" => "açıklık",
        "space" => "mahal",
        "insert" => "donatı",
        "dimension" => "ölçü",
        "tag" => "etiket",
        "text" => "yazı",
        "line" => "çizgi",
        _ => kind,
    }
}

/// What a comparison says, in one line.
pub(crate) fn compared_words(c: &OptionsCompared) -> String {
    let mut parts = vec![format!("{} aynı", c.same)];
    if !c.changed.is_empty() {
        parts.push(format!("{} değişmiş", c.changed.len()));
    }
    if !c.removed.is_empty() {
        parts.push(format!("{} yalnız {}'da", c.removed.len(), c.a.to_uppercase()));
    }
    if !c.added.is_empty() {
        parts.push(format!("{} yalnız {}'de", c.added.len(), c.b.to_uppercase()));
    }
    parts.push(format!("mahal alanı {} m²", num(c.space_area, true)));
    parts.push(format!("duvar {} m", num(c.wall_length, true)));
    for k in &c.kinds {
        parts.push(format!("{} {} → {}", kind_words(&k.kind), k.a, k.b));
    }
    parts.join(" · ")
}

impl App {
    fn design_options_rows(&self) -> Vec<OptionSetRow> {
        let Some(doc) = self.document.as_ref() else {
            return Vec::new();
        };
        match kentos_native_application::options_query::query(&doc.model, &OptionsQuery::default()) {
            CommandResult::Completed { output, .. } => output.sets,
            _ => Vec::new(),
        }
    }

    fn design_options_read(&mut self, keep: Option<String>) {
        let rows = self.design_options_rows();
        let chosen = keep
            .and_then(|id| rows.iter().position(|r| r.id == id))
            .unwrap_or(0);
        let name = self
            .design_options
            .as_ref()
            .map(|w| w.name.clone())
            .unwrap_or_default();
        let option = rows
            .get(chosen)
            .and_then(|s| s.options.iter().position(|o| o.primary))
            .unwrap_or(0);
        let n = rows.get(chosen).map_or(0, |s| s.options.len());
        self.design_options = Some(Window {
            rows,
            chosen,
            option,
            name,
            compare: (0, usize::from(n > 1)),
            compared: None,
            accept: None,
        });
        self.design_options_preview();
    }

    /// Tasarım seçenekleri (`arch.options`): the window, on the set of a selected object when it is in one.
    pub(crate) fn open_design_options(&mut self) {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return;
        }
        let keep = self.selected_option().map(|(s, _)| s);
        self.design_options_read(keep);
        if let (Some((_, option)), Some(w)) = (self.selected_option(), self.design_options.as_mut())
            && let Some(set) = w.rows.get(w.chosen)
        {
            w.option = set.options.iter().position(|o| o.id == option).unwrap_or(w.option);
        }
        self.design_options_preview();
        self.dialog = Some(Dialog::DesignOptions);
    }

    /// The set and option the first selected object is in.
    fn selected_option(&self) -> Option<(String, String)> {
        let doc = self.document.as_ref()?;
        let sets = doc.model.arch_options();
        self.selection.ids().iter().find_map(|s| {
            let uid = doc.model.uid(*s)?.to_string();
            kentos_commands::option_of(sets, &uid).map(|(s, o)| (s.id.clone(), o.id.clone()))
        })
    }

    fn design_options_chosen(&self) -> Option<(String, Option<String>)> {
        let w = self.design_options.as_ref()?;
        let set = w.rows.get(w.chosen)?;
        Some((set.id.clone(), set.options.get(w.option).map(|o| o.id.clone())))
    }

    /// The comparison of the two options picked, and the plan of Kabul et with the chosen one.
    fn design_options_preview(&mut self) {
        let Some(doc) = self.document.as_ref() else {
            return;
        };
        let Some(w) = self.design_options.as_ref() else {
            return;
        };
        let Some(set) = w.rows.get(w.chosen) else {
            return;
        };
        let (a, b) = w.compare;
        let compared = match (set.options.get(a), set.options.get(b)) {
            (Some(x), Some(y)) if a != b => Some(
                match kentos_native_application::options_query::query(
                    &doc.model,
                    &OptionsQuery {
                        compare: Some(OptionsCompare {
                            set: set.id.clone(),
                            a: x.id.clone(),
                            b: y.id.clone(),
                            search: None,
                            tolerance: None,
                        }),
                        ..Default::default()
                    },
                ) {
                    CommandResult::Completed { output, .. } => output.compared.ok_or_else(String::new),
                    CommandResult::Failed { error } | CommandResult::Conflict { error } => Err(error.message),
                    _ => Err("Karşılaştırılamadı.".into()),
                },
            ),
            _ => None,
        };
        let accept = set.options.get(w.option).map(|o| {
            match kentos_native_application::options_edit::plan_in(
                &doc.model,
                &OptionsEdit {
                    operation: OptionsOperation::Accept {
                        set: set.id.clone(),
                        option: Some(o.id.clone()),
                    },
                    expected_revision: None,
                },
            ) {
                CommandResult::Completed { output, .. } => Ok(output),
                CommandResult::Failed { error } | CommandResult::Conflict { error } => Err(error.message),
                _ => Err("Önizlenemedi.".into()),
            }
        });
        if let Some(w) = self.design_options.as_mut() {
            w.compared = compared;
            w.accept = accept;
        }
    }

    fn design_options_command(&mut self, operation: OptionsOperation) -> Option<OptionsEdited> {
        let doc = self.document.as_mut()?;
        let result = kentos_native_application::options_edit::execute(
            &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
            OptionsEdit {
                operation,
                expected_revision: None,
            },
        );
        match result {
            CommandResult::Completed { output, warnings } => {
                for w in warnings {
                    self.warn(w.message);
                }
                Some(output)
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

    /// The selection's objects: those in no option, and those of `set`'s options.
    fn design_options_selection(&self, set: Option<&str>) -> (Vec<String>, Vec<String>) {
        let Some(doc) = self.document.as_ref() else {
            return (Vec::new(), Vec::new());
        };
        let sets = doc.model.arch_options();
        let (mut free, mut members) = (Vec::new(), Vec::new());
        for s in self.selection.ids() {
            let Some(uid) = doc.model.uid(*s).map(|u| u.to_string()) else {
                continue;
            };
            match kentos_commands::option_of(sets, &uid) {
                None => free.push(uid),
                Some((x, _)) if Some(x.id.as_str()) == set => members.push(uid),
                Some(_) => {}
            }
        }
        (free, members)
    }

    pub(crate) fn design_options_event(&mut self, event: Event) -> Task<Message> {
        let chosen = self.design_options_chosen();
        match event {
            Event::Choose(i) => {
                if let Some(w) = self.design_options.as_mut() {
                    w.chosen = i;
                    w.option = w
                        .rows
                        .get(i)
                        .and_then(|s| s.options.iter().position(|o| o.primary))
                        .unwrap_or(0);
                    let n = w.rows.get(i).map_or(0, |s| s.options.len());
                    w.compare = (0, usize::from(n > 1));
                }
                self.design_options_preview();
            }
            Event::Option(j) => {
                if let Some(w) = self.design_options.as_mut() {
                    w.option = j;
                }
                self.design_options_preview();
            }
            Event::Name(t) => {
                if let Some(w) = self.design_options.as_mut() {
                    w.name = t;
                }
            }
            Event::CompareA(i) => {
                if let Some(w) = self.design_options.as_mut() {
                    w.compare.0 = i;
                }
                self.design_options_preview();
            }
            Event::CompareB(i) => {
                if let Some(w) = self.design_options.as_mut() {
                    w.compare.1 = i;
                }
                self.design_options_preview();
            }
            Event::Define => {
                let name = self
                    .design_options
                    .as_ref()
                    .map(|w| w.name.trim().to_owned())
                    .unwrap_or_default();
                let name = if name.is_empty() { "Mutfak".to_owned() } else { name };
                let (uids, _) = self.design_options_selection(None);
                if uids.is_empty() {
                    self.warn("Önce seçenek olacak duvarları, kapıları, mahali ve donatıları seçin.");
                    return Task::none();
                }
                if let Some(done) = self.design_options_command(OptionsOperation::Define {
                    name: name.clone(),
                    id: None,
                    option: None,
                    uids,
                }) {
                    self.say(Level::Success, format!("“{name}” seçenek kümesi tanımlandı; seçim A seçeneği oldu."));
                    if let Some(w) = self.design_options.as_mut() {
                        w.name.clear();
                    }
                    self.design_options_read(done.set.map(|s| s.id));
                }
            }
            Event::Copy => {
                let Some((set, Some(option))) = chosen else {
                    return Task::none();
                };
                if let Some(done) = self.design_options_command(OptionsOperation::Copy {
                    set: set.clone(),
                    option,
                    name: None,
                    primary: false,
                }) {
                    self.say(
                        Level::Success,
                        format!(
                            "{} seçeneği eklendi: {} nesnenin bağımsız kopyası.",
                            done.option.unwrap_or_default().to_uppercase(),
                            done.created.len()
                        ),
                    );
                }
                self.design_options_read(Some(set));
            }
            Event::Primary => {
                let Some((set, Some(option))) = chosen else {
                    return Task::none();
                };
                if self
                    .design_options_command(OptionsOperation::Primary {
                        set: set.clone(),
                        option,
                    })
                    .is_some()
                {
                    self.say(Level::Success, "Birincil seçenek değişti; çizim ve paftalar onu gösteriyor.");
                }
                self.design_options_read(Some(set));
            }
            Event::Add => {
                let Some((set, Some(option))) = chosen else {
                    return Task::none();
                };
                let (free, _) = self.design_options_selection(Some(&set));
                if free.is_empty() {
                    self.warn("Seçimde seçeneğe eklenecek, hiçbir seçenekte olmayan nesne yok.");
                    return Task::none();
                }
                let n = free.len();
                if self
                    .design_options_command(OptionsOperation::AddMembers {
                        set: set.clone(),
                        option,
                        uids: free,
                    })
                    .is_some()
                {
                    self.say(Level::Success, format!("{n} nesne seçeneğe eklendi."));
                }
                self.design_options_read(Some(set));
            }
            Event::Remove => {
                let Some((set, _)) = chosen else {
                    return Task::none();
                };
                let (_, members) = self.design_options_selection(Some(&set));
                if members.is_empty() {
                    self.warn("Seçimde bu kümenin nesnesi yok.");
                    return Task::none();
                }
                let n = members.len();
                if self
                    .design_options_command(OptionsOperation::RemoveMembers {
                        set: set.clone(),
                        uids: members,
                    })
                    .is_some()
                {
                    self.say(Level::Success, format!("{n} nesne ana modele döndü."));
                }
                self.design_options_read(Some(set));
            }
            Event::Accept => {
                let Some((set, Some(option))) = chosen else {
                    return Task::none();
                };
                if let Some(done) = self.design_options_command(OptionsOperation::Accept {
                    set: set.clone(),
                    option: Some(option),
                }) {
                    self.say(
                        Level::Success,
                        format!(
                            "Seçenek kabul edildi: {} nesne ana model oldu, {} nesne silindi.",
                            done.released.len(),
                            done.deleted.len()
                        ),
                    );
                }
                self.design_options_read(None);
            }
            Event::Delete => {
                let Some((set, Some(option))) = chosen else {
                    return Task::none();
                };
                if let Some(done) = self.design_options_command(OptionsOperation::Delete {
                    set: set.clone(),
                    option,
                }) {
                    self.say(Level::Success, format!("Seçenek {} nesnesiyle silindi.", done.deleted.len()));
                }
                self.design_options_read(Some(set));
            }
            Event::Close => {
                self.design_options = None;
                self.dialog = None;
            }
        }
        Task::none()
    }

    pub(crate) fn design_options_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.design_options else {
            return iced::widget::text("").into();
        };
        let mut list = Column::new().spacing(4);
        for (i, s) in w.rows.iter().enumerate() {
            let primary = s
                .options
                .iter()
                .find(|o| o.primary)
                .map_or_else(String::new, |o| o.name.clone());
            let text = Column::new()
                .spacing(2)
                .push(label::body(s.name.clone()))
                .push(label::caption(format!("{} seçenek · birincil {primary}", s.options.len())));
            let entry = row![
                icon(crate::icons::from_web(Some("designOptions"))).size(18.0),
                container(text).width(Fill)
            ]
            .spacing(8)
            .align_y(Center);
            list = list.push(
                button(entry)
                    .on_press(msg(Event::Choose(i)))
                    .padding([6, 8])
                    .width(Fill)
                    .style(style::button::row(i == w.chosen)),
            );
        }
        if w.rows.is_empty() {
            list = list.push(label::caption(
                "Projede tasarım seçeneği yok. Mutfağın ya da bir odanın duvarlarını, kapısını, mahalini ve donatılarını seçip aşağıdan tanımlayın.",
            ));
        }
        let mut right = Column::new().spacing(6);
        if let Some(s) = w.rows.get(w.chosen) {
            right = right.push(label::strong(format!("{} seçenekleri", s.name)));
            for (j, o) in s.options.iter().enumerate() {
                let mut facts = Vec::new();
                if o.primary {
                    facts.push("birincil".to_owned());
                }
                facts.push(format!("{} nesne", o.objects));
                if o.space_area > 0.0 {
                    facts.push(format!("mahal {} m²", num(o.space_area, false)));
                }
                if o.wall_length > 0.0 {
                    facts.push(format!("duvar {} m", num(o.wall_length, false)));
                }
                if o.missing > 0 {
                    facts.push(format!("{} nesne silinmiş", o.missing));
                }
                let line = row![
                    container(label::body(o.name.clone())).width(90.0),
                    label::caption(facts.join(" · "))
                ]
                .spacing(12)
                .align_y(Center);
                right = right.push(
                    button(line)
                        .on_press(msg(Event::Option(j)))
                        .padding([5, 8])
                        .width(Fill)
                        .style(style::button::row(j == w.option)),
                );
            }
            if s.options.len() > 1 {
                let names = || s.options.iter().map(|o| Choice::new(o.name.clone()));
                let a = Select::new(names(), Some(w.compare.0), |i| msg(Event::CompareA(i))).searchable(false);
                let b = Select::new(names(), Some(w.compare.1), |i| msg(Event::CompareB(i))).searchable(false);
                right = right.push(words::field(
                    "Karşılaştır",
                    row![container(a).width(Fill), label::caption("ile"), container(b).width(Fill)]
                        .spacing(8)
                        .align_y(Center),
                    None,
                ));
                let line = match &w.compared {
                    Some(Ok(c)) => words::text_line(Kind::Ok, compared_words(c)),
                    Some(Err(why)) => words::text_line(Kind::Warn, why.clone()),
                    None => words::text_line(Kind::Ok, "İki ayrı seçenek seçin.".to_owned()),
                };
                right = right.push(words::summary(vec![line]));
            }
            let accept = match &w.accept {
                Some(Ok(done)) => words::text_line(
                    Kind::Ok,
                    format!(
                        "{ACCEPT}: {} nesne ana model olur, {} nesne silinir; küme kalkar.",
                        done.released.len(),
                        done.deleted.len()
                    ),
                ),
                Some(Err(why)) => words::text_line(Kind::Warn, why.clone()),
                None => words::text_line(Kind::Ok, "Bir seçenek seçin.".to_owned()),
            };
            right = right.push(words::summary(vec![accept]));
            right = right.push(
                row![
                    words::secondary(ADD, Some(msg(Event::Add))),
                    words::secondary(REMOVE, Some(msg(Event::Remove))),
                ]
                .spacing(8),
            );
        }
        let name = kentos_ui::widget::text_field("Yeni kümenin adı (Mutfak)", &w.name)
            .on_input(|t| msg(Event::Name(t)));
        let define = words::field(
            "Seçimden yeni küme",
            name,
            Some("Seçilen nesneler kümenin birincil A seçeneği olur; ana modelden onlara bağlı nesne (kapının duvarı, etiketin mahali) varsa onları da seçin.".into()),
        );
        let body = Column::new()
            .spacing(14)
            .push(
                row![
                    container(iced::widget::scrollable(list).height(Fill)).width(240.0),
                    container(iced::widget::scrollable(right).height(Fill)).width(Fill)
                ]
                .spacing(16)
                .height(300.0),
            )
            .push(define);
        let set = w.rows.get(w.chosen);
        let option = set.and_then(|s| s.options.get(w.option));
        let is_primary = option.is_some_and(|o| o.primary);
        Frame::new(TITLE)
            .push(body)
            .action(words::secondary(DEFINE, Some(msg(Event::Define))))
            .action(words::secondary(COPY, option.map(|_| msg(Event::Copy))))
            .action(words::secondary(
                PRIMARY,
                (option.is_some() && !is_primary).then(|| msg(Event::Primary)),
            ))
            .action(words::secondary(
                DELETE,
                (option.is_some() && !is_primary).then(|| msg(Event::Delete)),
            ))
            .action(words::secondary(CLOSE, Some(msg(Event::Close))))
            .action(words::primary(
                ACCEPT,
                matches!(w.accept, Some(Ok(_))).then(|| msg(Event::Accept)),
            ))
            .width(860.0)
            .modal(msg(Event::Close))
    }

    pub(crate) fn design_options_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        if self.design_options.is_none() {
            return Err(format!("{TITLE} penceresi açık değil"));
        }
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(DEFINE) => Some(msg(Event::Define)),
            Control::Press(COPY) => Some(msg(Event::Copy)),
            Control::Press(PRIMARY) => Some(msg(Event::Primary)),
            Control::Press(ACCEPT) => Some(msg(Event::Accept)),
            Control::Press(DELETE) => Some(msg(Event::Delete)),
            Control::Press(ADD) => Some(msg(Event::Add)),
            Control::Press(REMOVE) => Some(msg(Event::Remove)),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}
