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
//! An instance holding one storey of its module (docs/adr/0811) says which.
//!
//! A module of several storeys (docs/adr/0796) says its storeys and each instance the levels it stands on; Katları
//! tekrarla's Modülsüz nesneler de copies the source level's objects in no module too, the plan saying how many are
//! copied and how many are left. A loose object whose like already stands on a target is not copied again unless
//! Hedefte aynısı olanı da kopyala is on, the plan saying how many are there already (docs/adr/0810).

use crate::arch;
use iced::widget::{Column, button, container, row};
use iced::{Center, Element, Fill, Task};
use kentos_commands::{
    CommandResult, Entity, ModuleLevelTarget, ModuleNumbering, ModuleRow, ModulesEdit,
    ModulesEdited, ModulesOperation, ModulesQuery, Vec2,
};
use kentos_interaction::Level;
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
const PICK: &str = "Çizimden seç";
const UPDATE: &str = "Kaynağı güncelle";
const DETACH: &str = "Bağımsızlaştır";
const REMOVE: &str = "Kaynağı sil";
const ADD_MEMBERS: &str = "Seçimi örneğe ekle";
const REMOVE_MEMBERS: &str = "Seçimi modülden çıkar";
const REPEAT: &str = "Katları tekrarla";

/// The window's two parts (docs/adr/8050 §6): the project's modules, a list and the chosen one's details, and Katları
/// tekrarla, which works on a level's instances rather than on one module.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Part {
    #[default]
    Modules,
    Repeat,
}

impl std::fmt::Display for Part {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Part::Modules => "Modüller",
            Part::Repeat => REPEAT,
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct Window {
    pub part: Part,
    pub rows: Vec<ModuleRow>,
    pub chosen: usize,
    pub instance: usize,
    pub name: String,
    /// What Kaynağı güncelle from the chosen instance would do; its refusal's words when refused.
    pub preview: Option<Result<ModulesEdited, String>>,
    /// The refusal is only that the chosen instance is the source as it is: information, not a warning.
    pub preview_same: bool,
    /// Katları tekrarla: the source level, and each other level chosen with its number prefix.
    pub repeat_from: Option<String>,
    pub repeat_to: Vec<RepeatTarget>,
    /// What Katları tekrarla would do; its refusal's words when refused.
    pub repeat_preview: Option<Result<ModulesEdited, String>>,
    /// Katları tekrarla copies the source level's objects in no module too (docs/adr/0796 §2).
    pub repeat_loose: bool,
    /// Katları tekrarla copies a loose object even where its like already stands on the target (docs/adr/0810).
    pub repeat_again: bool,
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
    Part(Part),
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
    RepeatAgain(bool),
    Repeat,
    Close,
    /// Çizimden seç: the window steps aside for a pick (docs/adr/8006).
    PickFromDrawing,
}

fn msg(e: Event) -> Message {
    Message::Arch(arch::Event::RoomModules(e))
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
    // A sentence's clauses, not a meta line of “A · B · C” (DESIGN.md §13).
    parts.join("; ")
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
            .arch
            .room_modules
            .as_ref()
            .map(|w| w.name.clone())
            .unwrap_or_default();
        // The part the window shows stays as it is read again (after a write).
        let part = self
            .arch
            .room_modules
            .as_ref()
            .map(|w| w.part)
            .unwrap_or_default();
        let (repeat_from, repeat_to, repeat_loose, repeat_again) = self
            .arch
            .room_modules
            .as_ref()
            .map(|w| {
                (
                    w.repeat_from.clone(),
                    w.repeat_to.clone(),
                    w.repeat_loose,
                    w.repeat_again,
                )
            })
            .unwrap_or_default();
        self.arch.room_modules = Some(Window {
            part,
            rows,
            chosen,
            instance: 0,
            name,
            preview: None,
            preview_same: false,
            repeat_from,
            repeat_to,
            repeat_preview: None,
            repeat_loose,
            repeat_again,
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
        let Some(w) = self.arch.room_modules.as_mut() else {
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
        let w = self.arch.room_modules.as_ref()?;
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
            again: w.repeat_loose && w.repeat_again,
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
        if let Some(w) = self.arch.room_modules.as_mut() {
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
        if let (Some((_, inst)), Some(w)) =
            (self.selected_module(), self.arch.room_modules.as_mut())
            && let Some(row) = w.rows.get(w.chosen)
        {
            w.instance = row.instances.iter().position(|i| i.id == inst).unwrap_or(0);
        }
        self.room_modules_preview();
        self.dialog = Some(Dialog::Arch(arch::ArchDialogKind::RoomModules));
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
        let w = self.arch.room_modules.as_ref()?;
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
                        | CommandResult::NeedsInput { error } => {
                            Err((error.code == "nothing_to_set", error.message))
                        }
                        _ => Err((false, "Önizlenemedi.".into())),
                    },
                )
            }
            _ => None,
        };
        if let Some(w) = self.arch.room_modules.as_mut() {
            w.preview_same = matches!(preview, Some(Err((true, _))));
            w.preview = preview.map(|p| p.map_err(|(_, why)| why));
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
        // Without walls (a stair with its slab, columns), the lower left of the other objects' points (docs/adr/8050
        // §1): the drawing's origin, far from them, made every placement land as far off.
        let (mut ox, mut oy) = (f64::INFINITY, f64::INFINITY);
        for s in self.selection.ids() {
            let Some(e) = doc.model.get(*s) else { continue };
            if let Some(u) = doc.model.uid(*s) {
                uids.push(u.to_string());
            }
            let others: Vec<&Vec2> = match e {
                Entity::Wall(w) => {
                    for p in &w.wall.pts {
                        x = x.min(p.x);
                        y = y.min(p.y);
                    }
                    Vec::new()
                }
                Entity::Column(c) => vec![&c.column.at],
                Entity::Stair(st) => st.stair.pts.iter().collect(),
                Entity::Ramp(r) => r.ramp.pts.iter().collect(),
                Entity::Slab(sl) => sl.slab.parts.iter().flat_map(|p| &p.outer.pts).collect(),
                Entity::Space(sp) => sp.space.parts.iter().flat_map(|p| &p.outer.pts).collect(),
                _ => Vec::new(),
            };
            for p in others {
                ox = ox.min(p.x);
                oy = oy.min(p.y);
            }
        }
        let base = if x.is_finite() {
            Vec2 { x, y }
        } else if ox.is_finite() {
            Vec2 { x: ox, y: oy }
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
            Event::Part(p) => {
                if let Some(w) = self.arch.room_modules.as_mut() {
                    w.part = p;
                }
            }
            Event::Choose(i) => {
                if let Some(w) = self.arch.room_modules.as_mut() {
                    w.chosen = i;
                    w.instance = 0;
                }
                self.room_modules_preview();
            }
            Event::Instance(j) => {
                if let Some(w) = self.arch.room_modules.as_mut() {
                    w.instance = j;
                }
                self.room_modules_preview();
            }
            Event::Name(t) => {
                if let Some(w) = self.arch.room_modules.as_mut() {
                    w.name = t;
                }
            }
            Event::Define => {
                let name = self
                    .arch
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
                    if let Some(w) = self.arch.room_modules.as_mut() {
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
                if let (Some(w), Some(l)) = (self.arch.room_modules.as_mut(), levels.get(i)) {
                    w.repeat_from = Some(l.clone());
                }
                self.room_modules_levels();
                self.room_modules_repeat_preview();
            }
            Event::RepeatChoose(i, on) => {
                if let Some(t) = self
                    .arch
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
                    .arch
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
                if let Some(w) = self.arch.room_modules.as_mut() {
                    w.repeat_loose = on;
                }
                self.room_modules_repeat_preview();
            }
            Event::RepeatAgain(on) => {
                if let Some(w) = self.arch.room_modules.as_mut() {
                    w.repeat_again = on;
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
                    if !done.report.matched.is_empty() {
                        said += &format!(
                            "; {} nesnenin aynısı hedefte vardı, yeniden kopyalanmadı",
                            done.report.matched.len()
                        );
                    }
                    self.say(Level::Success, format!("{said}."));
                    if let Some(w) = self.arch.room_modules.as_mut() {
                        w.repeat_to.iter_mut().for_each(|t| t.chosen = false);
                    }
                }
                self.room_modules_read(keep);
            }
            Event::Close => {
                self.arch.room_modules = None;
                self.dialog = None;
            }
            Event::PickFromDrawing => self.pick_for_window(
                crate::app::Dialog::Arch(arch::ArchDialogKind::RoomModules),
                "Oda modülünün nesneleri",
            ),
        }
        Task::none()
    }

    /// The window (docs/adr/8050 §6): Modüller's list and the chosen module's details side by side (the list/details
    /// pattern, `kentos_ui::widget::ListDetails`), each module's and each instance's actions beside what
    /// they act on; Katları tekrarla in a part of its own. The footer keeps the window's own: Çizimden seç and Kapat.
    pub(crate) fn room_modules_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.arch.room_modules else {
            return iced::widget::text("").into();
        };
        let parts = kentos_ui::widget::Segmented::new([Part::Modules, Part::Repeat], w.part, |p| {
            msg(Event::Part(p))
        });
        let body: Element<'_, Message> = match w.part {
            Part::Repeat => container(self.room_modules_repeat_view(w))
                .height(kentos_ui::theme::typography::scaled(420.0))
                .into(),
            Part::Modules => kentos_ui::widget::ListDetails::new(
                self.room_modules_list(w),
                self.room_modules_details(w),
            )
            .list_foot(self.room_modules_new(w))
            .list_width(250.0)
            .height(420.0)
            .into(),
        };
        Frame::new(TITLE)
            .push(parts)
            .push(body)
            .aside(words::ghost(PICK, Some(msg(Event::PickFromDrawing))))
            .action(words::secondary(CLOSE, Some(msg(Event::Close))))
            .width(860.0)
            .modal(msg(Event::Close))
    }

    /// The project's modules: each one's name and how many instances it has.
    fn room_modules_list<'a>(&self, w: &'a Window) -> Element<'a, Message> {
        use kentos_ui::widget::table::{Column as Col, Row, Table};
        let mut table = Table::new([
            Col::new("Modül").width(Fill),
            Col::new("Örnek")
                .width(kentos_ui::theme::typography::scaled(48.0))
                .align_right(),
        ])
        .empty("Projede oda modülü yok.");
        for (i, m) in w.rows.iter().enumerate() {
            let name = if m.removed {
                label::caption(format!("{} (kaynağı silinmiş)", m.name))
            } else {
                label::body(m.name.clone())
            };
            table = table.push(
                Row::new([
                    name.into(),
                    label::body(m.instances.len().to_string()).into(),
                ])
                .selected(i == w.chosen)
                .on_press(msg(Event::Choose(i))),
            );
        }
        table.into()
    }

    /// Seçimden yeni modül, under the list: its name and the button beside it.
    fn room_modules_new<'a>(&self, w: &'a Window) -> Element<'a, Message> {
        let name = kentos_ui::widget::text_field("Yeni modülün adı", &w.name)
            .on_input(|t| msg(Event::Name(t)));
        Column::new()
            .spacing(6)
            .push(label::strong("Seçimden yeni modül"))
            .push(
                row![
                    container(name).width(Fill),
                    words::secondary(DEFINE, Some(msg(Event::Define)))
                ]
                .spacing(8)
                .align_y(Center),
            )
            .push(label::caption(self.room_module_selection_words()))
            .into()
    }

    /// What Seçimden tanımla would take from the current selection: that one is needed, how many objects it holds, and
    /// which of them a module cannot take (the definition is refused whole while any is selected).
    pub(crate) fn room_module_selection_words(&self) -> String {
        let ids = self.selection.ids();
        let Some(doc) = self.document.as_ref().filter(|_| !ids.is_empty()) else {
            return "Önce çizimde odanın nesnelerini seçin.".into();
        };
        let mut unfit: Vec<&'static str> = Vec::new();
        let mut unfit_count = 0usize;
        for s in ids {
            let Some(e) = doc.model.get(*s) else { continue };
            let tied_column = matches!(e, Entity::Column(c) if c.column.grids.is_some());
            if !kentos_commands::module_kind_ok(e) || tied_column {
                unfit_count += 1;
                let name = if tied_column {
                    "aksa bağlı kolon"
                } else {
                    kentos_interaction::data_search::kind_name(e)
                };
                if !unfit.contains(&name) {
                    unfit.push(name);
                }
            }
        }
        let n = ids.len();
        if unfit_count == 0 {
            return format!("Seçimde {n} nesne var; hepsi modüle alınabilir.");
        }
        let names = unfit
            .iter()
            .map(|w| w.to_lowercase())
            .collect::<Vec<_>>()
            .join(", ");
        let fit = n - unfit_count;
        if fit == 0 {
            return format!(
                "Seçimdeki {n} nesne modüle alınamaz ({names}). Duvar, açıklık, mahal, donatı, kolon, merdiven, rampa, döşeme, ölçü, etiket ya da yazı seçin."
            );
        }
        format!(
            "Seçimde {n} nesne var: {fit} tanesi modüle alınabilir, {unfit_count} tanesi alınamaz ({names}). Tanımlamadan önce bunları seçimden çıkarın."
        )
    }

    /// The chosen module's details: what it is (name and values), its own actions, its instances in a table, and the
    /// chosen instance's state and actions.
    fn room_modules_details<'a>(&'a self, w: &'a Window) -> Element<'a, Message> {
        use kentos_ui::widget::table::{Column as Col, Row, Table};
        let Some(m) = w.rows.get(w.chosen) else {
            // No module yet: what one is and how to make the first (DESIGN.md §7.5: no empty pane without a way on).
            let step = |n: &'static str, words: &'static str| {
                row![
                    container(label::strong(n)).width(kentos_ui::theme::typography::scaled(18.0)),
                    container(label::body(words)).width(Fill)
                ]
                .spacing(6)
            };
            return Column::new()
                .spacing(10)
                .padding(iced::Padding { right: 12.0, ..iced::Padding::ZERO })
                .push(label::strong("Projede oda modülü yok"))
                .push(label::caption(
                    "Bir banyoyu, mutfağı ya da daireyi bir kez çizip bağlı örnekler olarak yerleştirirsiniz: kaynak değişince örnekler güncellenir.",
                ))
                .push(step("1", "Çizimde odanın nesnelerini seçin: duvarlar, kapı ve pencereler, mahal, donatı, kolonlar, merdivenler, döşemeler, mutfak dizileri."))
                .push(step("2", "Soldaki Seçimden yeni modül'e ad verin ve Seçimden tanımla'ya basın; seçim modülün ilk örneği olur."))
                .push(step("3", "Mimari › Proje › Modül yerleştir ile yeni örnekler koyun: aynalı, döndürülmüş ya da başka katta."))
                .into();
        };
        let live = !m.removed;
        let mut facts = kentos_ui::widget::pairs::Pairs::new()
            .push(label::caption("Sürüm"), label::body(m.version.to_string()))
            .push(
                label::caption("Nesneler"),
                label::body(m.members.to_string()),
            );
        if let Some(n) = m.runs {
            facts = facts.push(label::caption("Mutfak dizisi"), label::body(n.to_string()));
        }
        if let Some(n) = m.storeys {
            facts = facts.push(label::caption("Katlar"), label::body(format!("{n} katlı")));
        }
        facts = facts.push(
            label::caption("Örnekler"),
            label::body(m.instances.len().to_string()),
        );
        if m.removed {
            facts = facts.push(
                label::caption("Durum"),
                label::body("Kaynağı silinmiş; örnekler son hâlleriyle duruyor."),
            );
        }
        let remove = button(label::body(REMOVE))
            .on_press_maybe(live.then(|| msg(Event::Remove)))
            .padding(kentos_ui::theme::metrics::action())
            .style(style::button::danger_outline);
        let module_actions = row![
            words::secondary(
                ADD_MEMBERS,
                (live && !m.instances.is_empty()).then(|| msg(Event::AddSelection))
            ),
            words::secondary(REMOVE_MEMBERS, live.then(|| msg(Event::RemoveSelection))),
            iced::widget::space::horizontal(),
            remove,
        ]
        .spacing(8)
        .align_y(Center);
        let level_name = |l: &String| {
            self.document
                .as_ref()
                .and_then(|d| d.model.settings().arch.as_ref())
                .and_then(|a| a.levels.iter().find(|x| &x.id == l))
                .map_or_else(|| l.clone(), |x| x.name.clone())
        };
        let mut instances = Table::new([
            Col::new("Örnek").width(kentos_ui::theme::typography::scaled(56.0)),
            Col::new("Kat").width(kentos_ui::theme::typography::scaled(150.0)),
            Col::new("Ayna").width(kentos_ui::theme::typography::scaled(56.0)),
            Col::new("Sürüm")
                .width(kentos_ui::theme::typography::scaled(52.0))
                .align_right(),
            Col::new("Durum").width(Fill),
        ]);
        for (j, inst) in m.instances.iter().enumerate() {
            // The instance's number, its id's last part (“banyo-3” → 3).
            let number = inst.id.rsplit('-').next().unwrap_or(&inst.id).to_owned();
            // A module of several storeys: the levels it stands on, base first (docs/adr/0796 §1); one storey of the
            // module only (docs/adr/0811).
            let mut level = if inst.levels.len() > 1 {
                inst.levels
                    .iter()
                    .map(&level_name)
                    .collect::<Vec<_>>()
                    .join("–")
            } else {
                inst.level
                    .as_ref()
                    .map_or_else(|| "—".to_owned(), &level_name)
            };
            if let Some(n) = inst.storey {
                level = format!("{level} (modülün {}. katı)", n + 1);
            }
            let mut state: Vec<String> = Vec::new();
            if inst.broken.is_some() {
                state.push("bağı kopuk".into());
            } else if inst.stale {
                state.push("eski sürüm".into());
            }
            if !inst.exceptions.is_empty() {
                state.push(format!("{} istisna", inst.exceptions.len()));
            }
            if !inst.drifted.is_empty() {
                state.push(format!("{} elle değişmiş", inst.drifted.len()));
            }
            if !inst.missing.is_empty() {
                state.push(format!("{} silinmiş", inst.missing.len()));
            }
            if let Some(u) = &inst.unit {
                state.push(format!("bölüm {u}"));
            }
            let state = if state.is_empty() {
                "güncel".to_owned()
            } else {
                state.join(", ")
            };
            instances = instances.push(
                Row::new([
                    label::body(number).into(),
                    label::body(level).into(),
                    label::body(if inst.mirror { "aynalı" } else { "—" }).into(),
                    label::body(inst.version.to_string()).into(),
                    label::body(state).into(),
                ])
                .selected(j == w.instance)
                .on_press(msg(Event::Instance(j))),
            );
        }
        let status = match &w.preview {
            Some(Ok(done)) => words::text_line(
                Kind::Ok,
                format!("Bu örnekten güncellenirse: {}.", report_words(done)),
            ),
            // The source as it is: said without the instance's inner id, with what would make an update.
            Some(Err(_)) if w.preview_same => words::text_line(
                Kind::Info,
                "Bu örnek kaynakla aynı; kaynağı güncellemek için önce bu örnekte bir şeyi değiştirin."
                    .to_owned(),
            ),
            Some(Err(why)) => words::text_line(Kind::Warn, why.clone()),
            None => words::text_line(
                Kind::Info,
                "Kaynağı güncellemek için bir örnek seçin.".to_owned(),
            ),
        };
        let can_update = live && matches!(w.preview, Some(Ok(_)));
        let instance_actions = row![
            words::primary(UPDATE, can_update.then(|| msg(Event::Update))),
            words::secondary(
                DETACH,
                (!m.instances.is_empty()).then(|| msg(Event::Detach))
            ),
        ]
        .spacing(8)
        .align_y(Center);
        Column::new()
            .spacing(12)
            .padding(iced::Padding {
                right: 12.0,
                ..iced::Padding::ZERO
            })
            .push(label::strong(m.name.clone()))
            .push(facts)
            .push(module_actions)
            .push(kentos_ui::widget::horizontal_divider())
            .push(label::strong("Örnekler"))
            .push(instances)
            .push(words::summary(vec![status]))
            .push(instance_actions)
            .into()
    }

    pub(crate) fn room_modules_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        if self.arch.room_modules.is_none() {
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
        // The target levels: each one chosen or not and its number prefix (a short field, not the pane's width).
        let mut targets = Column::new().spacing(4).push(
            row![
                container(label::caption("Hedef kat")).width(Fill),
                container(label::caption("Numara öneki"))
                    .width(kentos_ui::theme::typography::scaled(150.0))
            ]
            .spacing(8),
        );
        for (i, t) in w.repeat_to.iter().enumerate() {
            let prefix = kentos_ui::widget::text_field("Yok", &t.prefix)
                .on_input(move |s| msg(Event::RepeatPrefix(i, s)));
            targets = targets.push(
                row![
                    container(words::check(
                        t.chosen,
                        &t.name,
                        Some(msg(Event::RepeatChoose(i, !t.chosen)))
                    ))
                    .width(Fill),
                    container(prefix).width(kentos_ui::theme::typography::scaled(150.0))
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
                    if !done.report.matched.is_empty() {
                        said += &format!(
                            " {} nesnenin aynısı hedef katta zaten var; yeniden kopyalanmaz.",
                            done.report.matched.len()
                        );
                    }
                }
                words::text_line(
                    if done.report.left.is_empty() && done.report.matched.is_empty() {
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
        // Two columns (docs/adr/8050 §6): what is repeated where on the left, how and what it will do on the right,
        // the part's one primary action under its plan.
        let left = Column::new()
            .spacing(12)
            .push(words::field("Kaynak kat", from, None))
            .push(targets);
        let right = Column::new()
            .spacing(10)
            .push(label::strong("Seçenekler"))
            .push(words::check(
                w.repeat_loose,
                "Modülsüz nesneler de (duvar, açıklık, mahal, donatı, ölçü, etiket)",
                Some(msg(Event::RepeatLoose(!w.repeat_loose))),
            ))
            .push(words::check(
                w.repeat_again,
                "Hedefte aynısı olanı da kopyala",
                w.repeat_loose.then(|| msg(Event::RepeatAgain(!w.repeat_again))),
            ))
            .push(words::summary(vec![summary]))
            .push(words::primary(REPEAT, can.then(|| msg(Event::Repeat))));
        row![
            container(left).width(kentos_ui::theme::typography::scaled(330.0)),
            kentos_ui::widget::vertical_divider(),
            container(right).width(Fill)
        ]
        .spacing(kentos_ui::theme::typography::scaled(16.0))
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
#[path = "../tests/room_modules_tests.rs"]
mod tests;
