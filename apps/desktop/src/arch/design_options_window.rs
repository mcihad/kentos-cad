//! Tasarım seçenekleri (docs/adr/0788; `arch.options`): the project's design option sets, each with its options (which
//! one is primary, how many objects of which kinds, its spaces' area and its walls' length, objects gone). Seçimden
//! tanımla makes the selection a new set's first option; Seçeneği kopyala adds an independent copy of the chosen
//! option; Birincil yap makes the drawing and the sheets show it; Karşılaştır reads two options side by side (counts,
//! areas and the plan difference); Kabul et keeps the chosen option as the main model and deletes the others, its plan
//! said first; Seçeneği sil removes a non-primary option with its objects. Bileşimler (docs/adr/0808) keeps named
//! combinations of the sets' options: Birincillerden kaydet records what the drawing shows now, Uygula makes a
//! combination's options the primaries, Sil removes one no view shows; the picked one is compared with another set by
//! set (docs/adr/0809). Each write is one undo step (“Tasarım
//! seçeneği”); Kapat writes nothing.

use crate::arch;
use iced::widget::{Column, button, container, row};
use iced::{Center, Element, Fill, Task};
use kentos_commands::{
    CombosCompared, CommandResult, OptionComboRow, OptionSetRow, OptionsCompare, OptionsCompared, OptionsEdit, OptionsEdited,
    OptionsCompareCombos, OptionsOperation, OptionsQuery,
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
const PICK: &str = "Çizimden seç";
const COPY: &str = "Seçeneği kopyala";
const PRIMARY: &str = "Birincil yap";
const ACCEPT: &str = "Kabul et";
const DELETE: &str = "Seçeneği sil";
const ADD: &str = "Seçimi ekle";
const REMOVE: &str = "Seçimi çıkar";
const COMBO_SAVE: &str = "Birincillerden kaydet";
const COMBO_APPLY: &str = "Bileşimi uygula";
const COMBO_DELETE: &str = "Bileşimi sil";

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
    /// The project's combinations (docs/adr/0808), the one picked and the name a new one is saved under.
    pub combos: Vec<OptionComboRow>,
    pub combo: usize,
    pub combo_name: String,
    /// The combination the picked one is compared with, and what that comparison says (docs/adr/0809).
    pub versus: usize,
    pub combos_compared: Option<Result<CombosCompared, String>>,
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
    Combo(usize),
    ComboName(String),
    ComboSave,
    ComboApply,
    ComboDelete,
    Versus(usize),
    Close,
    /// Çizimden seç: the window steps aside for a pick (docs/adr/8006).
    PickFromDrawing,
}

fn msg(e: Event) -> Message {
    Message::Arch(arch::Event::DesignOptions(e))
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
/// What a comparison of two combinations says, in one line: each set differing, the sets alike, the plans' totals.
pub(crate) fn combos_words(c: &CombosCompared, rows: &[OptionSetRow]) -> String {
    let name = |id: &str| rows.iter().find(|s| s.id == id).map_or(id.to_owned(), |s| s.name.clone());
    let option = |set: &str, id: &str| {
        rows.iter()
            .find(|s| s.id == set)
            .and_then(|s| s.options.iter().find(|o| o.id == id))
            .map_or(id.to_uppercase(), |o| o.name.clone())
    };
    let mut parts: Vec<String> = c
        .sets
        .iter()
        .map(|s| {
            let mut ws = vec![format!("{} aynı", s.same)];
            if !s.changed.is_empty() {
                ws.push(format!("{} değişmiş", s.changed.len()));
            }
            if !s.removed.is_empty() {
                ws.push(format!("{} yalnız {}'da", s.removed.len(), option(&s.set, &s.a)));
            }
            if !s.added.is_empty() {
                ws.push(format!("{} yalnız {}'de", s.added.len(), option(&s.set, &s.b)));
            }
            format!("{} {} → {}: {}", name(&s.set), option(&s.set, &s.a), option(&s.set, &s.b), ws.join(", "))
        })
        .collect();
    if !c.alike.is_empty() {
        parts.push(format!("aynı: {}", c.alike.iter().map(|s| name(s)).collect::<Vec<_>>().join(", ")));
    }
    parts.push(format!(
        "plan {} → {} nesne, mahal {} m², duvar {} m",
        c.objects_a,
        c.objects_b,
        num(c.space_area, true),
        num(c.wall_length, true)
    ));
    parts.join(" · ")
}

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
    fn design_options_rows(&self) -> (Vec<OptionSetRow>, Vec<OptionComboRow>) {
        let Some(doc) = self.document.as_ref() else {
            return (Vec::new(), Vec::new());
        };
        match kentos_native_application::options_query::query(&doc.model, &OptionsQuery::default()) {
            CommandResult::Completed { output, .. } => (output.sets, output.combos),
            _ => (Vec::new(), Vec::new()),
        }
    }

    fn design_options_read(&mut self, keep: Option<String>) {
        let (rows, combos) = self.design_options_rows();
        let (combo, combo_name, versus) = self
            .arch
            .design_options
            .as_ref()
            .map(|w| {
                let last = combos.len().saturating_sub(1);
                (w.combo.min(last), w.combo_name.clone(), w.versus.min(last))
            })
            .unwrap_or_default();
        let versus = if versus == combo { usize::from(combo == 0 && combos.len() > 1) } else { versus };
        let chosen = keep
            .and_then(|id| rows.iter().position(|r| r.id == id))
            .unwrap_or(0);
        let name = self
            .arch
            .design_options
            .as_ref()
            .map(|w| w.name.clone())
            .unwrap_or_default();
        let option = rows
            .get(chosen)
            .and_then(|s| s.options.iter().position(|o| o.primary))
            .unwrap_or(0);
        let n = rows.get(chosen).map_or(0, |s| s.options.len());
        self.arch.design_options = Some(Window {
            rows,
            chosen,
            option,
            name,
            compare: (0, usize::from(n > 1)),
            compared: None,
            accept: None,
            combos,
            combo,
            combo_name,
            versus,
            combos_compared: None,
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
        if let (Some((_, option)), Some(w)) =
            (self.selected_option(), self.arch.design_options.as_mut())
            && let Some(set) = w.rows.get(w.chosen)
        {
            w.option = set
                .options
                .iter()
                .position(|o| o.id == option)
                .unwrap_or(w.option);
        }
        self.design_options_preview();
        self.dialog = Some(Dialog::Arch(arch::ArchDialogKind::DesignOptions));
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
        let w = self.arch.design_options.as_ref()?;
        let set = w.rows.get(w.chosen)?;
        Some((
            set.id.clone(),
            set.options.get(w.option).map(|o| o.id.clone()),
        ))
    }

    /// The comparison of the two options picked, and the plan of Kabul et with the chosen one.
    fn design_options_preview(&mut self) {
        let Some(doc) = self.document.as_ref() else {
            return;
        };
        let Some(w) = self.arch.design_options.as_ref() else {
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
        let combos_compared = match (w.combos.get(w.combo), w.combos.get(w.versus)) {
            (Some(x), Some(y)) if x.id != y.id => Some(
                match kentos_native_application::options_query::query(
                    &doc.model,
                    &OptionsQuery {
                        compare_combos: Some(OptionsCompareCombos {
                            a: x.id.clone(),
                            b: y.id.clone(),
                            search: None,
                            tolerance: None,
                        }),
                        ..Default::default()
                    },
                ) {
                    CommandResult::Completed { output, .. } => output.combos_compared.ok_or_else(String::new),
                    CommandResult::Failed { error } | CommandResult::Conflict { error } => Err(error.message),
                    _ => Err("Karşılaştırılamadı.".into()),
                },
            ),
            _ => None,
        };
        if let Some(w) = self.arch.design_options.as_mut() {
            w.compared = compared;
            w.accept = accept;
            w.combos_compared = combos_compared;
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
                if let Some(w) = self.arch.design_options.as_mut() {
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
                if let Some(w) = self.arch.design_options.as_mut() {
                    w.option = j;
                }
                self.design_options_preview();
            }
            Event::Name(t) => {
                if let Some(w) = self.arch.design_options.as_mut() {
                    w.name = t;
                }
            }
            Event::CompareA(i) => {
                if let Some(w) = self.arch.design_options.as_mut() {
                    w.compare.0 = i;
                }
                self.design_options_preview();
            }
            Event::CompareB(i) => {
                if let Some(w) = self.arch.design_options.as_mut() {
                    w.compare.1 = i;
                }
                self.design_options_preview();
            }
            Event::Define => {
                let name = self
                    .arch
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
                    self.say(
                        Level::Success,
                        format!("“{name}” seçenek kümesi tanımlandı; seçim A seçeneği oldu."),
                    );
                    if let Some(w) = self.arch.design_options.as_mut() {
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
            Event::Combo(i) => {
                if let Some(w) = self.arch.design_options.as_mut() {
                    if w.versus == i {
                        w.versus = w.combo;
                    }
                    w.combo = i;
                }
                self.design_options_preview();
            }
            Event::Versus(i) => {
                if let Some(w) = self.arch.design_options.as_mut() {
                    w.versus = i;
                }
                self.design_options_preview();
            }
            Event::ComboName(t) => {
                if let Some(w) = self.arch.design_options.as_mut() {
                    w.combo_name = t;
                }
            }
            Event::ComboSave => {
                let keep = chosen.map(|(s, _)| s);
                let name = self
                    .arch
                    .design_options
                    .as_ref()
                    .map(|w| w.combo_name.trim().to_owned())
                    .filter(|n| !n.is_empty())
                    .unwrap_or_else(|| {
                        format!(
                            "Teklif {}",
                            self.arch
                                .design_options
                                .as_ref()
                                .map_or(0, |w| w.combos.len())
                                + 1
                        )
                    });
                if let Some(done) = self.design_options_command(OptionsOperation::SaveCombo {
                    name: name.clone(),
                    id: None,
                    choices: None,
                }) {
                    self.say(Level::Success, format!("“{name}” bileşimi kümelerin şimdiki birincilleriyle kaydedildi."));
                    let id = done.combo.map(|c| c.id);
                    self.design_options_read(keep);
                    if let Some(w) = self.arch.design_options.as_mut() {
                        w.combo_name.clear();
                        w.combo = id
                            .and_then(|id| w.combos.iter().position(|c| c.id == id))
                            .unwrap_or(w.combo);
                    }
                    self.design_options_preview();
                }
            }
            Event::ComboApply | Event::ComboDelete => {
                let keep = chosen.map(|(s, _)| s);
                let Some(c) = self
                    .arch
                    .design_options
                    .as_ref()
                    .and_then(|w| w.combos.get(w.combo).cloned())
                else {
                    return Task::none();
                };
                let apply = matches!(event, Event::ComboApply);
                let operation = if apply {
                    OptionsOperation::ApplyCombo { combo: c.id.clone() }
                } else {
                    OptionsOperation::DeleteCombo { combo: c.id.clone() }
                };
                if self.design_options_command(operation).is_some() {
                    self.say(
                        Level::Success,
                        if apply {
                            format!("“{}” uygulandı; çizim ve paftalar onun seçeneklerini gösteriyor.", c.name)
                        } else {
                            format!("“{}” bileşimi silindi.", c.name)
                        },
                    );
                }
                self.design_options_read(keep);
            }
            Event::Close => {
                self.arch.design_options = None;
                self.dialog = None;
            }
            Event::PickFromDrawing => self.pick_for_window(
                crate::app::Dialog::Arch(arch::ArchDialogKind::DesignOptions),
                "Tasarım seçeneğinin nesneleri",
            ),
        }
        Task::none()
    }

    pub(crate) fn design_options_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.arch.design_options else {
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
        let combos = self.design_options_combos(w);
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
            .push(define)
            .push(combos);
        let set = w.rows.get(w.chosen);
        let option = set.and_then(|s| s.options.get(w.option));
        let is_primary = option.is_some_and(|o| o.primary);
        Frame::new(TITLE)
            .scroll_fill(body)
            .aside(words::ghost(PICK, Some(msg(Event::PickFromDrawing))))
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

    /// Bileşimler: the project's combinations, each with what it shows, the one picked applied or deleted, a new one
    /// saved from the primaries.
    fn design_options_combos<'a>(&'a self, w: &'a Window) -> Element<'a, Message> {
        let mut list = Column::new().spacing(2);
        for (i, c) in w.combos.iter().enumerate() {
            let shown: Vec<String> = c
                .choices
                .iter()
                .filter_map(|ch| {
                    let s = w.rows.iter().find(|s| s.id == ch.set)?;
                    let o = s.options.iter().find(|o| o.id == ch.option)?;
                    Some(format!("{} {}", s.name, o.name))
                })
                .collect();
            let mut facts = vec![shown.join(", "), format!("{} nesne", c.objects)];
            if c.space_area > 0.0 {
                facts.push(format!("mahal {} m²", num(c.space_area, false)));
            }
            if c.current {
                facts.insert(0, "şimdiki".into());
            }
            let line = row![
                container(label::body(c.name.clone())).width(140.0),
                label::caption(facts.join(" · "))
            ]
            .spacing(12)
            .align_y(Center);
            list = list.push(
                button(line)
                    .on_press(msg(Event::Combo(i)))
                    .padding([4, 8])
                    .width(Fill)
                    .style(style::button::row(i == w.combo)),
            );
        }
        if w.combos.is_empty() {
            list = list.push(label::caption(
                "Bileşim yok. Kümelerde göstermek istediğiniz seçenekleri birincil yapıp bir adla kaydedin; teklifler arasında tek tıkla geçilir.",
            ));
        }
        let picked = w.combos.get(w.combo);
        let name = kentos_ui::widget::text_field("Bileşimin adı (Teklif 1)", &w.combo_name)
            .on_input(|t| msg(Event::ComboName(t)));
        let has_sets = !w.rows.is_empty();
        let mut out = Column::new()
            .spacing(6)
            .push(label::strong("Bileşimler"))
            .push(container(iced::widget::scrollable(list)).max_height(120.0));
        if let (Some(c), true) = (picked, w.combos.len() > 1) {
            let others = Select::new(
                w.combos.iter().map(|c| Choice::new(c.name.clone())),
                Some(w.versus),
                |i| msg(Event::Versus(i)),
            )
            .searchable(false);
            out = out.push(words::field(
                "Karşılaştır",
                row![label::caption(c.name.clone()), label::caption("ile"), container(others).width(220.0)]
                    .spacing(8)
                    .align_y(Center),
                None,
            ));
            let line = match &w.combos_compared {
                Some(Ok(cc)) => words::text_line(Kind::Ok, combos_words(cc, &w.rows)),
                Some(Err(why)) => words::text_line(Kind::Warn, why.clone()),
                None => words::text_line(Kind::Ok, "İki ayrı bileşim seçin.".to_owned()),
            };
            out = out.push(words::summary(vec![line]));
        }
        out.push(
                row![
                    container(name).width(Fill),
                    words::secondary(COMBO_SAVE, has_sets.then(|| msg(Event::ComboSave))),
                    words::secondary(
                        COMBO_APPLY,
                        picked.filter(|c| !c.current).map(|_| msg(Event::ComboApply))
                    ),
                    words::secondary(COMBO_DELETE, picked.map(|_| msg(Event::ComboDelete))),
                ]
                .spacing(8)
                .align_y(Center),
            )
            .into()
    }

    pub(crate) fn design_options_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        if self.arch.design_options.is_none() {
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
            Control::Press(COMBO_SAVE) => Some(msg(Event::ComboSave)),
            Control::Press(COMBO_APPLY) => Some(msg(Event::ComboApply)),
            Control::Press(COMBO_DELETE) => Some(msg(Event::ComboDelete)),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}
