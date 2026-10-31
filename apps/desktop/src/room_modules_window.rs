//! Oda modülleri (docs/adr/0775; `arch.modules`): the project's room modules, each with its instances (level, mirror,
//! version against the source's, local exceptions, objects gone). Seçimden tanımla makes the selected walls, openings,
//! spaces, fixtures, dimensions and tags a module, its base the lower left corner of their walls; Kaynağı güncelle
//! takes the chosen instance's objects into the source and gives them to the other linked instances, its plan said
//! first (which instances update, which are skipped, which objects keep their own values or are gone); Bağımsızlaştır
//! leaves the chosen instance following nothing; Kaynağı sil keeps the instances as they are, their link broken. Each
//! write is one undo step (“Oda modülü”); Kapat writes nothing.

use iced::widget::{Column, button, container, row};
use iced::{Center, Element, Fill, Task};
use kentos_commands::{
    CommandResult, Entity, ModuleRow, ModulesEdit, ModulesEdited, ModulesOperation, ModulesQuery,
    Vec2,
};
use kentos_interaction::Level;
use kentos_ui::icon::icon;
use kentos_ui::widget::Dialog as Frame;
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

#[derive(Clone, Debug, Default)]
pub struct Window {
    pub rows: Vec<ModuleRow>,
    pub chosen: usize,
    pub instance: usize,
    pub name: String,
    /// What Kaynağı güncelle from the chosen instance would do; its refusal's words when refused.
    pub preview: Option<Result<ModulesEdited, String>>,
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
    let gone = r.conflicts.len() - kept;
    if gone > 0 {
        parts.push(format!("{gone} silinmiş nesne atlanır"));
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
        self.room_modules = Some(Window {
            rows,
            chosen,
            instance: 0,
            name,
            preview: None,
        });
        self.room_modules_preview();
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
                format!(
                    "sürüm {} · {} nesne · {} örnek",
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
                if let Some(l) = &inst.level {
                    let name = self
                        .document
                        .as_ref()
                        .and_then(|d| d.model.settings().arch.as_ref())
                        .and_then(|a| a.levels.iter().find(|x| &x.id == l))
                        .map_or_else(|| l.clone(), |x| x.name.clone());
                    facts.push(name);
                }
                if inst.mirror {
                    facts.push("aynalı".into());
                }
                if !inst.exceptions.is_empty() {
                    facts.push(format!("{} istisna", inst.exceptions.len()));
                }
                if !inst.missing.is_empty() {
                    facts.push(format!("{} nesne silinmiş", inst.missing.len()));
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
        }
        let name = kentos_ui::widget::text_field("Yeni modülün adı (Banyo)", &w.name)
            .on_input(|t| msg(Event::Name(t)));
        let define = words::field("Seçimden yeni modül", name, Some("Tek kattaki düz duvarlar, açıklıklar, mahal, donatı, ölçü ve etiketler; taban noktası duvarların sol alt köşesi.".into()));
        let body = Column::new()
            .spacing(14)
            .push(
                row![
                    container(iced::widget::scrollable(list).height(Fill)).width(260.0),
                    container(iced::widget::scrollable(right).height(Fill)).width(Fill)
                ]
                .spacing(16)
                .height(320.0),
            )
            .push(define);
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
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
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
