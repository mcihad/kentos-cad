//! Oda modülleri (docs/adr/0775; `arch.modules`): the project's room modules, each with its instances (level, mirror,
//! version against the source's, local exceptions, objects gone). Seçimden tanımla makes the selected walls, openings,
//! spaces, fixtures, dimensions and tags a module, its base the lower left corner of their walls; Kaynağı güncelle
//! takes the chosen instance's objects into the source and gives them to the other linked instances, its plan said
//! first (which instances update, which are skipped, which objects keep their own values or are gone); Bağımsızlaştır
//! leaves the chosen instance following nothing; Kaynağı sil keeps the instances as they are, their link broken. Each
//! write is one undo step (“Oda modülü”); Kapat writes nothing.
//!
//! After the first slice (docs/adr/0779): each instance says how many of its objects were changed by hand away from the
//! source; Seçimi örneğe ekle adds the selection's free objects to the chosen instance's definition (and so to every
//! linked instance), Seçimi modülden çıkar takes the selection's members out; Katları tekrarla places the source
//! level's linked instances on the chosen levels, each with its own number prefix, its plan said before it is written.
//!
//! A module of several storeys (docs/adr/0796) says its storeys and each instance the levels it stands on; Katları
//! tekrarla's Modülsüz nesneler de copies the source level's objects in no module too, the plan saying how many are
//! copied and how many are left.

use iced::widget::{Column, button, container, row};
use iced::{Center, Element, Fill, Task};
use kentos_commands::{
    CommandResult, Entity, ModuleLevelTarget, ModuleNumbering, ModuleRow, ModulesEdit,
    ModulesEdited, ModulesOperation, ModulesQuery, Vec2,
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
pub const TITLE: &str = "Oda modülleri";
const CLOSE: &str = "Kapat";
const DEFINE: &str = "Seçimden tanımla";
const UPDATE: &str = "Kaynağı güncelle";
const DETACH: &str = "Bağımsızlaştır";
const REMOVE: &str = "Kaynağı sil";
const ADD_MEMBERS: &str = "Seçimi örneğe ekle";
const REMOVE_MEMBERS: &str = "Seçimi modülden çıkar";
const REPEAT: &str = "Katları tekrarla";

#[derive(Clone, Debug, Default)]
pub struct Window {
    pub rows: Vec<ModuleRow>,
    pub chosen: usize,
    pub instance: usize,
    pub name: String,
    /// What Kaynağı güncelle from the chosen instance would do; its refusal's words when refused.
    pub preview: Option<Result<ModulesEdited, String>>,
    /// Katları tekrarla: the source level, and each other level chosen with its number prefix.
    pub repeat_from: Option<String>,
    pub repeat_to: Vec<RepeatTarget>,
    /// What Katları tekrarla would do; its refusal's words when refused.
    pub repeat_preview: Option<Result<ModulesEdited, String>>,
    /// Katları tekrarla copies the source level's objects in no module too (docs/adr/0796 §2).
    pub repeat_loose: bool,
}

/// A level Katları tekrarla may place on: its id and name, whether chosen, its number prefix.
#[derive(Clone, Debug, Default)]
pub struct RepeatTarget {
    pub level: String,
    pub name: String,
    pub chosen: bool,
    pub prefix: String,
}

#[derive(Clone, Debug)]
pub enum Event {
    Choose(usize),
    Instance(usize),
    Name(String),
    Define,
    Update,
    Detach,
    Remove,
    AddSelection,
    RemoveSelection,
    RepeatFrom(usize),
    RepeatChoose(usize, bool),
    RepeatPrefix(usize, String),
    RepeatLoose(bool),
    Repeat,
    Close,
}

fn msg(e: Event) -> Message {
    Message::RoomModules(e)
}

/// What a source update says: the instances it reaches and what it passes by.
pub(crate) fn report_words(done: &ModulesEdited) -> String {
    let r = &done.report;
    let mut parts = vec![format!("{} örnek güncellenir", r.updated.len())];
    if !r.skipped.is_empty() {
        parts.push(format!(
            "{} örnek kilitli, eski sürümde kalır",
            r.skipped.len()
        ));
    }
    let kept = r
        .conflicts
        .iter()
        .filter(|c| c.why == kentos_commands::ModuleConflictWhy::Exception)
        .count();
    if kept > 0 {
        parts.push(format!("{kept} yerel istisna korunur"));
    }
    let drifted = r
        .conflicts
        .iter()
        .filter(|c| c.why == kentos_commands::ModuleConflictWhy::Drifted)
        .count();
    if drifted > 0 {
        parts.push(format!("elle değişmiş {drifted} nesne kaynağa döner"));
    }
    let gone = r
        .conflicts
        .iter()
        .filter(|c| c.why == kentos_commands::ModuleConflictWhy::Gone)
        .count();
    if gone > 0 {
        parts.push(format!("{gone} silinmiş nesne atlanır"));
    }
    if !r.added_keys.is_empty() {
        parts.push(format!("{} eksik nesne eklenir", r.added_keys.len()));
    }
    parts.push(format!("{} kaynak nesnesi değişir", r.changed_keys.len()));
    parts.join(" · ")
}

impl App {
    fn room_modules_rows(&self) -> Vec<ModuleRow> {
        let Some(doc) = self.document.as_ref() else {
            return Vec::new();
        };
        match kentos_native_application::modules_query::query(&doc.model, &ModulesQuery::default())
        {
            CommandResult::Completed { output, .. } => output.modules,
            _ => Vec::new(),
        }
    }

    fn room_modules_read(&mut self, keep: Option<String>) {
        let rows = self.room_modules_rows();
        let chosen = keep
            .and_then(|id| rows.iter().position(|r| r.id == id))
            .unwrap_or(0);
        let name = self
            .room_modules
            .as_ref()
            .map(|w| w.name.clone())
            .unwrap_or_default();
        let (repeat_from, repeat_to, repeat_loose) = self
            .room_modules
            .as_ref()
            .map(|w| (w.repeat_from.clone(), w.repeat_to.clone(), w.repeat_loose))
            .unwrap_or_default();
        self.room_modules = Some(Window {
            rows,
            chosen,
            instance: 0,
            name,
            preview: None,
            repeat_from,
            repeat_to,
            repeat_preview: None,
            repeat_loose,
        });
        self.room_modules_levels();
        self.room_modules_preview();
    }

    /// The project's levels for Katları tekrarla: the source the first level with a linked instance (else the first),
    /// the others its targets, their choices and prefixes kept.
    fn room_modules_levels(&mut self) {
        let levels: Vec<(String, String)> = self
            .document
            .as_ref()
            .and_then(|d| d.model.settings().arch.as_ref())
            .map(|a| {
                a.levels
                    .iter()
                    .map(|l| (l.id.clone(), l.name.clone()))
                    .collect()
            })
            .unwrap_or_default();
        let Some(w) = self.room_modules.as_mut() else {
            return;
        };
        let used: Vec<String> = w
            .rows
            .iter()
            .filter(|m| !m.removed)
            .flat_map(|m| m.instances.iter().filter_map(|i| i.level.clone()))
            .collect();
        if w.repeat_from
            .as_ref()
            .is_none_or(|f| !levels.iter().any(|(id, _)| id == f))
        {
            w.repeat_from = levels
                .iter()
                .find(|(id, _)| used.contains(id))
                .or(levels.first())
                .map(|(id, _)| id.clone());
        }
        let old = std::mem::take(&mut w.repeat_to);
        w.repeat_to = levels
            .iter()
            .filter(|(id, _)| Some(id) != w.repeat_from.as_ref())
            .map(|(id, name)| {
                old.iter()
                    .find(|t| &t.level == id)
                    .cloned()
                    .unwrap_or(RepeatTarget {
                        level: id.clone(),
                        name: name.clone(),
                        chosen: false,
                        prefix: String::new(),
                    })
            })
            .collect();
    }

    /// The input Katları tekrarla writes; none without a chosen level.
    fn room_modules_repeat_input(&self) -> Option<ModulesOperation> {
        let w = self.room_modules.as_ref()?;
        let from = w.repeat_from.clone()?;
        let to: Vec<ModuleLevelTarget> = w
            .repeat_to
            .iter()
            .filter(|t| t.chosen)
            .map(|t| ModuleLevelTarget {
                level: t.level.clone(),
                numbering: (!t.prefix.trim().is_empty()).then(|| ModuleNumbering {
                    prefix: Some(t.prefix.trim().to_owned()),
                    suffix: None,
                }),
                unit: None,
                instances: Vec::new(),
            })
            .collect();
        (!to.is_empty()).then_some(ModulesOperation::Repeat {
            from,
            modules: Vec::new(),
            to,
            loose: w.repeat_loose,
        })
    }

    /// The plan of Katları tekrarla.
    fn room_modules_repeat_preview(&mut self) {
        let preview = match (self.room_modules_repeat_input(), self.document.as_ref()) {
            (Some(operation), Some(doc)) => Some(
                match kentos_native_application::modules_edit::plan_in(
                    &doc.model,
                    &ModulesEdit {
                        operation,
                        expected_revision: None,
                    },
                ) {
                    CommandResult::Completed { output, .. } => Ok(output),
                    CommandResult::Failed { error }
                    | CommandResult::Conflict { error }
                    | CommandResult::NeedsInput { error } => Err(error.message),
                    _ => Err("Önizlenemedi.".into()),
                },
            ),
            _ => None,
        };
        if let Some(w) = self.room_modules.as_mut() {
            w.repeat_preview = preview;
        }
    }

    /// Oda modülleri (`arch.modules`): the window, on the module of a selected object when it is in one.
    pub(crate) fn open_room_modules(&mut self) {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return;
        }
        let keep = self.selected_module().map(|(m, _)| m);
        self.room_modules_read(keep);
        if let (Some((_, inst)), Some(w)) = (self.selected_module(), self.room_modules.as_mut())
            && let Some(row) = w.rows.get(w.chosen)
        {
            w.instance = row.instances.iter().position(|i| i.id == inst).unwrap_or(0);
        }
        self.room_modules_preview();
        self.dialog = Some(Dialog::RoomModules);
    }

    /// The module and instance the first selected object is in.
    fn selected_module(&self) -> Option<(String, String)> {
        let doc = self.document.as_ref()?;
        let arch = doc.model.settings().arch.as_ref()?;
        self.selection.ids().iter().find_map(|s| {
            let uid = doc.model.uid(*s)?.to_string();
            arch.modules.iter().find_map(|m| {
                m.instances
                    .iter()
                    .find(|i| i.members.iter().any(|o| o.uid == uid))
                    .map(|i| (m.id.clone(), i.id.clone()))
            })
        })
    }

    fn room_modules_chosen(&self) -> Option<(String, Option<String>)> {
        let w = self.room_modules.as_ref()?;
        let row = w.rows.get(w.chosen)?;
        Some((
            row.id.clone(),
            row.instances.get(w.instance).map(|i| i.id.clone()),
        ))
    }

    /// The plan of Kaynağı güncelle from the chosen instance.
    fn room_modules_preview(&mut self) {
        let preview = match (self.room_modules_chosen(), self.document.as_ref()) {
            (Some((module, Some(from))), Some(doc)) => {
                let input = ModulesEdit {
                    operation: ModulesOperation::Update {
                        module,
                        from,
                        instances: Vec::new(),
                    },
                    expected_revision: None,
                };
                Some(
                    match kentos_native_application::modules_edit::plan_in(&doc.model, &input) {
                        CommandResult::Completed { output, .. } => Ok(output),
                        CommandResult::Failed { error }
                        | CommandResult::Conflict { error }
                        | CommandResult::NeedsInput { error } => Err(error.message),
                        _ => Err("Önizlenemedi.".into()),
                    },
                )
            }
            _ => None,
        };
        if let Some(w) = self.room_modules.as_mut() {
            w.preview = preview;
        }
        self.room_modules_repeat_preview();
    }

    fn room_modules_command(&mut self, operation: ModulesOperation) -> Option<ModulesEdited> {
        let doc = self.document.as_mut()?;
        let result = kentos_native_application::modules_edit::execute(
            &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
            ModulesEdit {
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

    /// The selection's objects and the lower left corner of their walls (else of the first point found).
    fn room_module_selection(&self) -> (Vec<String>, Vec2) {
        let Some(doc) = self.document.as_ref() else {
            return (Vec::new(), Vec2 { x: 0.0, y: 0.0 });
        };
        let mut uids = Vec::new();
        let (mut x, mut y) = (f64::INFINITY, f64::INFINITY);
        for s in self.selection.ids() {
            let Some(e) = doc.model.get(*s) else { continue };
            if let Some(u) = doc.model.uid(*s) {
                uids.push(u.to_string());
            }
            if let Entity::Wall(w) = e {
                for p in &w.wall.pts {
                    x = x.min(p.x);
                    y = y.min(p.y);
                }
            }
        }
        let base = if x.is_finite() {
            Vec2 { x, y }
        } else {
            Vec2 { x: 0.0, y: 0.0 }
        };
        (uids, base)
    }

    /// The selection's objects in no module, and those in an instance of `module`.
    fn room_module_split(&self, module: &str) -> (Vec<String>, Vec<String>) {
        let Some(doc) = self.document.as_ref() else {
            return (Vec::new(), Vec::new());
        };
        let modules = doc.model.arch_modules();
        let (mut free, mut members) = (Vec::new(), Vec::new());
        for s in self.selection.ids() {
            let Some(uid) = doc.model.uid(*s).map(|u| u.to_string()) else {
                continue;
            };
            let owner = modules.iter().find(|m| {
                m.instances
                    .iter()
                    .any(|i| i.members.iter().any(|o| o.uid == uid))
            });
            match owner {
                None => free.push(uid),
                Some(m) if m.id == module => members.push(uid),
                Some(_) => {}
            }
        }
        (free, members)
    }

    /// Seçimi örneğe ekle: the selection's free objects join the instance of the selection's members (or the window's
    /// chosen instance).
    pub(crate) fn room_module_add_selection(&mut self, chosen: Option<(String, String)>) {
        let Some((module, instance)) = chosen.or_else(|| self.selected_module()) else {
            self.warn("Seçimde bir modül örneğinin nesnesi yok: eklenecek nesnelerle birlikte örneğin bir nesnesini de seçin.");
            return;
        };
        let (free, _) = self.room_module_split(&module);
        if free.is_empty() {
            self.warn("Seçimde modüle eklenecek, hiçbir modülde olmayan nesne yok.");
            return;
        }
        let n = free.len();
        if let Some(done) = self.room_modules_command(ModulesOperation::AddMembers {
            module,
            instance: instance.clone(),
            uids: free,
        }) {
            self.say(
                Level::Success,
                format!(
                    "{n} nesne {instance} örneğinden modüle eklendi; {} örnek aldı.",
                    done.report.updated.len()
                ),
            );
        }
    }

    /// Seçimi modülden çıkar: the selection's members leave the definition; the other instances' copies go.
    pub(crate) fn room_module_remove_selection(&mut self, module: Option<String>) {
        let Some(module) = module.or_else(|| self.selected_module().map(|(m, _)| m)) else {
            self.warn("Seçimde bir modül örneğinin nesnesi yok.");
            return;
        };
        let (_, members) = self.room_module_split(&module);
        if members.is_empty() {
            self.warn("Seçimde bu modülün nesnesi yok.");
            return;
        }
        if let Some(done) = self.room_modules_command(ModulesOperation::RemoveMembers {
            module,
            uids: members,
        }) {
            let r = &done.report;
            self.say(
                Level::Success,
                format!(
                    "{} nesne modülden çıktı: {} kopya silindi, {} nesne sıradan nesne olarak kaldı.",
                    r.removed_keys.len(),
                    r.deleted.len(),
                    r.released.len()
                ),
            );
        }
    }

    pub(crate) fn room_modules_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Choose(i) => {
                if let Some(w) = self.room_modules.as_mut() {
                    w.chosen = i;
                    w.instance = 0;
                }
                self.room_modules_preview();
            }
            Event::Instance(j) => {
                if let Some(w) = self.room_modules.as_mut() {
                    w.instance = j;
                }
                self.room_modules_preview();
            }
            Event::Name(t) => {
                if let Some(w) = self.room_modules.as_mut() {
                    w.name = t;
                }
            }
            Event::Define => {
                let name = self
                    .room_modules
                    .as_ref()
                    .map(|w| w.name.trim().to_owned())
                    .unwrap_or_default();
                let name = if name.is_empty() {
                    "Banyo".to_owned()
                } else {
                    name
                };
                let (uids, base) = self.room_module_selection();
                if uids.is_empty() {
                    self.warn("Önce modül olacak duvarları, kapıları, mahali ve donatıları seçin.");
                    return Task::none();
                }
                if let Some(done) = self.room_modules_command(ModulesOperation::Define {
                    name: name.clone(),
                    id: None,
                    uids,
                    base,
                }) {
                    self.say(
                        Level::Success,
                        format!("“{name}” modülü tanımlandı; seçim ilk örneği oldu."),
                    );
                    if let Some(w) = self.room_modules.as_mut() {
                        w.name.clear();
                    }
                    self.room_modules_read(done.module.map(|m| m.id));
                }
            }
            Event::Update => {
                let Some((module, Some(from))) = self.room_modules_chosen() else {
                    return Task::none();
                };
                if let Some(done) = self.room_modules_command(ModulesOperation::Update {
                    module: module.clone(),
                    from: from.clone(),
                    instances: Vec::new(),
                }) {
                    self.say(
                        Level::Success,
                        format!(
                            "Kaynak {from} örneğinden güncellendi: {}.",
                            report_words(&done)
                        ),
                    );
                }
                self.room_modules_read(Some(module));
            }
            Event::Detach => {
                let Some((module, Some(instance))) = self.room_modules_chosen() else {
                    return Task::none();
                };
                if self
                    .room_modules_command(ModulesOperation::Detach {
                        module: module.clone(),
                        instance: instance.clone(),
                    })
                    .is_some()
                {
                    self.say(
                        Level::Success,
                        format!("{instance} bağımsızlaştı; nesneleri kaldı."),
                    );
                }
                self.room_modules_read(Some(module));
            }
            Event::Remove => {
                let Some((module, _)) = self.room_modules_chosen() else {
                    return Task::none();
                };
                if self
                    .room_modules_command(ModulesOperation::Remove {
                        module: module.clone(),
                    })
                    .is_some()
                {
                    self.say(
                        Level::Success,
                        "Modülün kaynağı silindi; örnekler son hâliyle kaldı.",
                    );
                }
                self.room_modules_read(Some(module));
            }
            Event::AddSelection => {
                let chosen = self.room_modules_chosen();
                let Some((module, Some(instance))) = chosen else {
                    return Task::none();
                };
                self.room_module_add_selection(Some((module.clone(), instance)));
                self.room_modules_read(Some(module));
            }
            Event::RemoveSelection => {
                let Some((module, _)) = self.room_modules_chosen() else {
                    return Task::none();
                };
                self.room_module_remove_selection(Some(module.clone()));
                self.room_modules_read(Some(module));
            }
            Event::RepeatFrom(i) => {
                let levels: Vec<String> = self
                    .document
                    .as_ref()
                    .and_then(|d| d.model.settings().arch.as_ref())
                    .map(|a| a.levels.iter().map(|l| l.id.clone()).collect())
                    .unwrap_or_default();
                if let (Some(w), Some(l)) = (self.room_modules.as_mut(), levels.get(i)) {
                    w.repeat_from = Some(l.clone());
                }
                self.room_modules_levels();
                self.room_modules_repeat_preview();
            }
            Event::RepeatChoose(i, on) => {
                if let Some(t) = self
                    .room_modules
                    .as_mut()
                    .and_then(|w| w.repeat_to.get_mut(i))
                {
                    t.chosen = on;
                }
                self.room_modules_repeat_preview();
            }
            Event::RepeatPrefix(i, text) => {
                if let Some(t) = self
                    .room_modules
                    .as_mut()
                    .and_then(|w| w.repeat_to.get_mut(i))
                {
                    t.prefix = text;
                    t.chosen = true;
                }
                self.room_modules_repeat_preview();
            }
            Event::RepeatLoose(on) => {
                if let Some(w) = self.room_modules.as_mut() {
                    w.repeat_loose = on;
                }
                self.room_modules_repeat_preview();
            }
            Event::Repeat => {
                let Some(operation) = self.room_modules_repeat_input() else {
                    self.warn("Tekrarlanacak hedef katı seçin.");
                    return Task::none();
                };
                let keep = self.room_modules_chosen().map(|(m, _)| m);
                if let Some(done) = self.room_modules_command(operation) {
                    let mut said = format!(
                        "Katlar tekrarlandı: {} örnek, {} nesne yerleşti",
                        done.placed.len(),
                        done.created.len()
                    );
                    if !done.copied.is_empty() {
                        said += &format!("; {} modülsüz nesne kopyalandı", done.copied.len());
                    }
                    if !done.report.left.is_empty() {
                        said += &format!("; {} nesne kopyalanmadı", done.report.left.len());
                    }
                    self.say(Level::Success, format!("{said}."));
                    if let Some(w) = self.room_modules.as_mut() {
                        w.repeat_to.iter_mut().for_each(|t| t.chosen = false);
                    }
                }
                self.room_modules_read(keep);
            }
            Event::Close => {
                self.room_modules = None;
                self.dialog = None;
            }
        }
        Task::none()
    }

    pub(crate) fn room_modules_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.room_modules else {
            return iced::widget::text("").into();
        };
        let mut list = Column::new().spacing(4);
        for (i, m) in w.rows.iter().enumerate() {
            let facts = if m.removed {
                format!("kaynağı silinmiş · {} örnek", m.instances.len())
            } else {
                let storeys = m
                    .storeys
                    .map_or_else(String::new, |n| format!(" · {n} katlı"));
                format!(
                    "sürüm {} · {} nesne · {} örnek{storeys}",
                    m.version,
                    m.members,
                    m.instances.len()
                )
            };
            let text = Column::new()
                .spacing(2)
                .push(label::body(m.name.clone()))
                .push(label::caption(facts));
            let entry = row![
                icon(crate::icons::from_web(Some("roomModules"))).size(18.0),
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
                "Projede oda modülü yok. Bir banyonun duvarlarını, kapısını, mahalini ve donatılarını seçip aşağıdan tanımlayın.",
            ));
        }
        let mut right = Column::new().spacing(6);
        if let Some(m) = w.rows.get(w.chosen) {
            right = right.push(label::strong(format!("{} örnekleri", m.name)));
            for (j, inst) in m.instances.iter().enumerate() {
                let mut facts = vec![format!("sürüm {}", inst.version)];
                if inst.stale {
                    facts.push("eski".into());
                }
                let level_name = |l: &String| {
                    self.document
                        .as_ref()
                        .and_then(|d| d.model.settings().arch.as_ref())
                        .and_then(|a| a.levels.iter().find(|x| &x.id == l))
                        .map_or_else(|| l.clone(), |x| x.name.clone())
                };
                // A module of several storeys: the levels it stands on, base first (docs/adr/0796 §1).
                if inst.levels.len() > 1 {
                    facts.push(
                        inst.levels
                            .iter()
                            .map(level_name)
                            .collect::<Vec<_>>()
                            .join("–"),
                    );
                } else if let Some(l) = &inst.level {
                    facts.push(level_name(l));
                }
                if inst.mirror {
                    facts.push("aynalı".into());
                }
                if !inst.exceptions.is_empty() {
                    facts.push(format!("{} istisna", inst.exceptions.len()));
                }
                if !inst.drifted.is_empty() {
                    facts.push(format!("{} nesne elle değişmiş", inst.drifted.len()));
                }
                if !inst.missing.is_empty() {
                    facts.push(format!("{} nesne silinmiş", inst.missing.len()));
                }
                if let Some(u) = &inst.unit {
                    facts.push(format!("bölüm {u}"));
                }
                if inst.broken.is_some() {
                    facts.push("bağ kopuk".into());
                }
                let line = row![
                    label::body(inst.id.clone()),
                    label::caption(facts.join(" · "))
                ]
                .spacing(12)
                .align_y(Center);
                right = right.push(
                    button(line)
                        .on_press(msg(Event::Instance(j)))
                        .padding([5, 8])
                        .width(Fill)
                        .style(style::button::row(j == w.instance)),
                );
            }
            let summary = match &w.preview {
                Some(Ok(done)) => words::text_line(
                    Kind::Ok,
                    format!("Bu örnekten {UPDATE}: {}.", report_words(done)),
                ),
                Some(Err(why)) => words::text_line(Kind::Warn, why.clone()),
                None => words::text_line(
                    Kind::Ok,
                    "Kaynağı güncellemek için bir örnek seçin.".to_owned(),
                ),
            };
            right = right.push(words::summary(vec![summary]));
            let live = !m.removed;
            right = right.push(
                row![
                    words::secondary(
                        ADD_MEMBERS,
                        (live && !m.instances.is_empty()).then(|| msg(Event::AddSelection))
                    ),
                    words::secondary(REMOVE_MEMBERS, live.then(|| msg(Event::RemoveSelection))),
                ]
                .spacing(8),
            );
        }
        let name = kentos_ui::widget::text_field("Yeni modülün adı (Banyo)", &w.name)
            .on_input(|t| msg(Event::Name(t)));
        let define = words::field("Seçimden yeni modül", name, Some("Bir ya da birkaç kattaki düz duvarlar, açıklıklar, mahaller, donatı, ölçü ve etiketler (bir oda, bir daire, bir dubleks); taban noktası duvarların sol alt köşesi, taban katı en alçak kat.".into()));
        let body = Column::new()
            .spacing(14)
            .push(
                row![
                    container(iced::widget::scrollable(list).height(Fill)).width(260.0),
                    container(iced::widget::scrollable(right).height(Fill)).width(Fill)
                ]
                .spacing(16)
                .height(230.0),
            )
            .push(
                row![
                    container(define).width(Fill),
                    container(self.room_modules_repeat_view(w)).width(Fill)
                ]
                .spacing(16),
            );
        let chosen = w.rows.get(w.chosen);
        let live = chosen.is_some_and(|m| !m.removed);
        let can_update = live && matches!(w.preview, Some(Ok(_)));
        let has_instance = chosen.is_some_and(|m| !m.instances.is_empty());
        Frame::new(TITLE)
            .push(body)
            .action(words::secondary(DEFINE, Some(msg(Event::Define))))
            .action(words::secondary(
                DETACH,
                has_instance.then(|| msg(Event::Detach)),
            ))
            .action(words::secondary(REMOVE, live.then(|| msg(Event::Remove))))
            .action(words::secondary(CLOSE, Some(msg(Event::Close))))
            .action(words::primary(
                UPDATE,
                can_update.then(|| msg(Event::Update)),
            ))
            .width(860.0)
            .modal(msg(Event::Close))
    }

    pub(crate) fn room_modules_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        if self.room_modules.is_none() {
            return Err(format!("{TITLE} penceresi açık değil"));
        }
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(DEFINE) => Some(msg(Event::Define)),
            Control::Press(UPDATE) => Some(msg(Event::Update)),
            Control::Press(DETACH) => Some(msg(Event::Detach)),
            Control::Press(REMOVE) => Some(msg(Event::Remove)),
            Control::Press(ADD_MEMBERS) => Some(msg(Event::AddSelection)),
            Control::Press(REMOVE_MEMBERS) => Some(msg(Event::RemoveSelection)),
            Control::Press(REPEAT) => Some(msg(Event::Repeat)),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }

    /// Katları tekrarla: the source level, the target levels with their prefixes, the plan's words, the button.
    fn room_modules_repeat_view<'a>(&self, w: &'a Window) -> Element<'a, Message> {
        let levels: Vec<(String, String)> = self
            .document
            .as_ref()
            .and_then(|d| d.model.settings().arch.as_ref())
            .map(|a| {
                a.levels
                    .iter()
                    .map(|l| (l.id.clone(), l.name.clone()))
                    .collect()
            })
            .unwrap_or_default();
        if levels.len() < 2 {
            return words::field(
                REPEAT,
                label::caption(
                    "Projede en az iki kat olunca bir katın modülleri öbür katlara tekrarlanır.",
                ),
                None,
            );
        }
        let from = Select::new(
            levels.iter().map(|(_, n)| Choice::new(n.clone())),
            w.repeat_from
                .as_ref()
                .and_then(|f| levels.iter().position(|(id, _)| id == f)),
            |i| msg(Event::RepeatFrom(i)),
        )
        .searchable(false);
        let mut targets = Column::new().spacing(4);
        for (i, t) in w.repeat_to.iter().enumerate() {
            let prefix = kentos_ui::widget::text_field("Numara öneki", &t.prefix)
                .on_input(move |s| msg(Event::RepeatPrefix(i, s)));
            targets = targets.push(
                row![
                    container(words::check(
                        t.chosen,
                        &t.name,
                        Some(msg(Event::RepeatChoose(i, !t.chosen)))
                    ))
                    .width(130.0),
                    container(prefix).width(Fill)
                ]
                .spacing(8)
                .align_y(Center),
            );
        }
        let summary = match &w.repeat_preview {
            Some(Ok(done)) => {
                let mut said = format!(
                    "{} örnek, {} nesne yerleşir; her biri kaynağına bağlıdır.",
                    done.placed.len(),
                    done.created.len() - done.copied.len()
                );
                if w.repeat_loose {
                    said += &format!(
                        " {} modülsüz nesne düz kopya olur",
                        done.copied.len()
                    );
                    said += &if done.report.left.is_empty() {
                        ".".to_owned()
                    } else {
                        format!(
                            "; {} nesne kalır (kilitli, başka tür, başka örneğin, tasarım seçeneğinin ya da kat dışına bağlı).",
                            done.report.left.len()
                        )
                    };
                }
                words::text_line(
                    if done.report.left.is_empty() {
                        Kind::Ok
                    } else {
                        Kind::Warn
                    },
                    said,
                )
            }
            Some(Err(why)) => words::text_line(Kind::Warn, why.clone()),
            None => words::text_line(Kind::Ok, "Hedef katları seçin.".to_owned()),
        };
        let can = matches!(w.repeat_preview, Some(Ok(_)));
        Column::new()
            .spacing(6)
            .push(words::field("Katları tekrarla: kaynak kat", from, None))
            .push(targets)
            .push(words::check(
                w.repeat_loose,
                "Modülsüz nesneler de (duvar, açıklık, mahal, donatı, ölçü, etiket)",
                Some(msg(Event::RepeatLoose(!w.repeat_loose))),
            ))
            .push(words::summary(vec![summary]))
            .push(words::secondary(REPEAT, can.then(|| msg(Event::Repeat))))
            .into()
    }

    /// Yerel istisna yap and Kaynağa döndür on the selection's objects, through `cad.modules.edit`.
    pub(crate) fn room_module_selection_action(&mut self, restore: bool) {
        let Some((module, _)) = self.selected_module() else {
            self.warn("Seçimde bir modül örneğinin nesnesi yok.");
            return;
        };
        let (uids, _) = self.room_module_selection();
        let op = if restore {
            ModulesOperation::Restore { module, uids }
        } else {
            ModulesOperation::Except { module, uids }
        };
        if self.room_modules_command(op).is_some() {
            self.say(
                Level::Success,
                if restore {
                    "Seçilen nesneler kaynağa döndü."
                } else {
                    "Seçilen nesneler yerel istisna oldu."
                },
            );
        }
    }
}

#[cfg(test)]
#[path = "room_modules_tests.rs"]
mod tests;
