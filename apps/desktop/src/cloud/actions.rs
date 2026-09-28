//! The open cloud project's own actions (the web's `ui/cloud/ProjectActions.ts`
//! and `FileConflict.ts`'s `offerNewest`, docs/adr/0073): Yeniden adlandır,
//! Çöpe taşı and Son revizyonu aç, in the web's words. The server decides
//! every request; a refusal is said in its own words.
//!
//! - A database project's new name goes through its autosave with whatever
//!   else waits, so it cannot conflict with itself; a file project's through
//!   the catalog's `project.rename`, the drawing taking it quietly.
//! - Before the open project goes to the trash, what waits is sent first
//!   (the web's `settle`); then the drawing stays on screen as a local one.
//! - Son revizyonu aç asks first, and over unsaved work it is the question
//!   before the drawing is left (leaving.rs), since the work would be lost.

use std::collections::BTreeMap;
use std::time::Instant;

use iced::widget::{text, text_input};
use iced::{Element, Task};
use kentos_cloud::saving::envelope;
use kentos_cloud::{ApiFailure, SaveState};
use kentos_contracts::{
    EmptyInput, PROJECT_RENAME, PROJECT_RENAME_VERSION, PROJECT_TRASH, PROJECT_TRASH_VERSION,
    ProjectCatalogChange, ProjectPermission, ProjectRename, ProjectStorage,
};
use kentos_domain::{External, ExternalMeta, Uuid};
use kentos_interaction::js_trim;
use kentos_ui::widget::{Banner, Dialog as Window, Form, overlay};
use kentos_ui::{label, style};
use serde_json::json;

use crate::app::{App, Dialog, Message};
use crate::cloud::copy::Link;
use crate::cloud::view::{primary, secondary};
use crate::cloud::{Event, forms_plan as forms, words};

fn cloud(event: Event) -> Message {
    crate::cloud::msg(event)
}

/// The rename window's field, focused when it opens.
const RENAME_FIELD: &str = "cloud-rename";

/// Yeniden adlandır: what is typed and where the request stands.
#[derive(Debug, Clone)]
pub struct Rename {
    pub text: String,
    /// The name it had when the window opened.
    pub was: String,
    /// The request on its way; the field and the button wait.
    pub busy: bool,
    pub status: Option<Status>,
    id: u64,
}

impl Rename {
    /// The request's id, which its answer names.
    #[cfg(test)]
    pub fn id_for_tests(&self) -> u64 {
        self.id
    }

    /// The web's button: a name, not empty, not the same (forms_plan.rs).
    fn can_save(&self) -> bool {
        !self.busy && forms::rename_savable(&self.text, &self.was)
    }
}

/// The line under the field (the web's `cloud-status`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    Info(String),
    Error(String),
}

/// What waits for the open database project's autosave to send everything
/// first (the web's `flush`).
#[derive(Debug, Clone, PartialEq)]
pub enum Settle {
    /// The new name, said once the server has it (or once sending stops).
    Rename(String),
    /// The trash, sent after (whether everything went or not, as the web's `settle`).
    Trash,
    /// The catalog's archive or trash of the open project (catalog_actions.rs).
    Catalog(
        crate::cloud::catalog_actions::Act,
        Box<kentos_contracts::ProjectSummary>,
    ),
    /// A checkpoint of the open project, named once its edits went (catalog_history.rs).
    Checkpoint(
        Box<(
            kentos_contracts::ProjectSummary,
            kentos_contracts::CheckpointCreate,
        )>,
    ),
    /// Proje bilgileri's new name of the open database project; the rest of
    /// the patch goes once it went (catalog_forms.rs).
    Metadata(
        Box<(
            kentos_contracts::ProjectSummary,
            kentos_contracts::ProjectMetadataUpdate,
            String,
        )>,
    ),
    /// The open project in the other storage mode, once its edits went (catalog_forms.rs).
    Convert(
        Box<(
            kentos_contracts::ProjectSummary,
            kentos_contracts::ProjectConvert,
        )>,
    ),
}

/// A failed request as a sentence: the server's words, or what to do when it
/// gave none (the web's `reason`).
pub(super) fn reason(failure: &ApiFailure) -> String {
    match failure.code.as_str() {
        "conflict" => {
            return "Proje bilgileri bu arada başka biri tarafından değiştirildi. Listeyi yenileyip yeniden deneyin.".to_owned();
        }
        "network" => {
            return "Sunucuya ulaşılamadı; bağlantınızı denetleyip yeniden deneyin.".to_owned();
        }
        _ => {}
    }
    if failure.message.is_empty() {
        "İşlem tamamlanamadı.".to_owned()
    } else {
        failure.message.clone()
    }
}

impl App {
    /// Whether the open cloud project exists for this account and it may do
    /// `permission` there (`writes`: it changes the project, which an archived
    /// one refuses; the web's `openMay`).
    pub(crate) fn open_may(&self, permission: ProjectPermission, writes: bool) -> bool {
        let Some(c) = self.document.as_ref().and_then(|d| d.cloud_source()) else {
            return false;
        };
        let ended = self
            .cloud
            .live
            .as_ref()
            .is_some_and(|l| l.sync.state().ended());
        // An archived project refuses what changes it.
        let refused = ended || (writes && c.archived());
        !refused
            && c.info.access.permissions.contains(&permission)
            && self.cloud.me.is_some()
            && self.cloud.link != Link::Offline
    }

    /// `cloud.rename`: the window, the open project's name in its field.
    pub(crate) fn open_rename(&mut self) -> Task<Message> {
        let Some(doc) = &self.document else {
            return Task::none();
        };
        let name = doc.name().to_owned();
        let id = self.cloud.next_id();
        self.cloud.rename = Some(Rename {
            text: name.clone(),
            was: name,
            busy: false,
            status: None,
            id,
        });
        self.dialog = Some(Dialog::CloudRename);
        let field = iced::widget::Id::new(RENAME_FIELD);
        Task::batch([
            iced::widget::operation::focus(field.clone()),
            iced::widget::operation::select_all(field),
        ])
    }

    /// `cloud.delete`: the question first (the web's `askRemove`).
    pub(crate) fn ask_trash(&mut self) {
        if self
            .document
            .as_ref()
            .and_then(|d| d.cloud_source())
            .is_some()
        {
            self.dialog = Some(Dialog::CloudTrash);
        }
    }

    /// `cloud.openNewest` (the web's `offerNewest`, revisions.rs `offer`):
    /// a line while a Kaydet is on its way or where the project cannot be
    /// read any more; over unsaved work the conflict's question when a newer
    /// revision is known, else the unsaved question; over a clean drawing a
    /// plain question.
    pub(crate) fn offer_newest(&mut self) -> Task<Message> {
        self.ask_file(crate::cloud::revisions::Via::Newest);
        Task::none()
    }

    pub(crate) fn actions_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::RenameInput(t) => {
                if let Some(r) = self.cloud.rename.as_mut().filter(|r| !r.busy) {
                    r.text = t;
                }
                Task::none()
            }
            Event::RenameSubmit => self.rename_submit(),
            Event::Renamed { id, result } => self.renamed(id, result),
            Event::TrashConfirm => {
                self.dialog = None;
                self.trash_open_project()
            }
            Event::Trashed { id, result } => self.trashed(id, result),
            _ => Task::none(),
        }
    }

    fn rename_submit(&mut self) -> Task<Message> {
        let Some(r) = self.cloud.rename.as_mut().filter(|r| r.can_save()) else {
            return Task::none();
        };
        let name = js_trim(&r.text).to_owned();
        // The field takes 200 at most on the web (its maxlength), in UTF-16 units.
        if name.encode_utf16().count() > forms::NAME_MAX {
            r.status = Some(Status::Error(
                "Proje adı boş olamaz ve en çok 200 karakter olabilir.".to_owned(),
            ));
            return Task::none();
        }
        r.busy = true;
        r.status = Some(Status::Info(forms::SAVING.to_owned()));
        let id = r.id;
        let Some(doc) = self.document.as_mut() else {
            return Task::none();
        };
        let Some(source) = doc.cloud_source() else {
            return Task::none();
        };
        if source.storage() == ProjectStorage::Database {
            // Through the autosave, with whatever else waits: it cannot conflict with itself.
            doc.model.set_name(&name);
            if let Some(source) = doc.cloud_source_mut() {
                source.info.name.clone_from(&name);
            }
            self.cloud.settling = Some(Settle::Rename(name));
            return self.flush();
        }
        // A file project's name is the catalog's; the drawing takes it quietly after.
        let (Some(client), tenant, project) = (
            self.cloud.signed_in().cloned(),
            source.tenant,
            source.project,
        ) else {
            return Task::none();
        };
        let request = envelope(
            tenant,
            project,
            PROJECT_RENAME,
            PROJECT_RENAME_VERSION,
            Uuid::new_v4(),
            BTreeMap::new(),
            serde_json::to_value(ProjectRename { name }).unwrap_or(json!({})),
        );
        Task::perform(
            client.command::<ProjectCatalogChange>(request),
            move |result| cloud(Event::Renamed { id, result }),
        )
    }

    fn renamed(
        &mut self,
        id: u64,
        result: Result<ProjectCatalogChange, ApiFailure>,
    ) -> Task<Message> {
        let Some(r) = self.cloud.rename.as_mut().filter(|r| r.id == id) else {
            return Task::none();
        };
        match result {
            Ok(done) => {
                let name = done.project.name;
                if let Some(doc) = self.document.as_mut() {
                    let _ = doc.model.apply_external(External {
                        meta: Some(ExternalMeta {
                            name: Some(name.clone()),
                            ..ExternalMeta::default()
                        }),
                        ..External::default()
                    });
                    if let Some(source) = doc.cloud_source_mut() {
                        source.info.name.clone_from(&name);
                    }
                }
                self.cloud.rename = None;
                self.dialog = None;
                self.say_success(forms::rename::saved(&name));
            }
            Err(failure) => {
                r.busy = false;
                r.status = Some(Status::Error(reason(&failure)));
            }
        }
        Task::none()
    }

    /// What waits goes now (the web's `flush`); `settled` says when it went.
    pub(super) fn flush(&mut self) -> Task<Message> {
        // The autosave looks at the drawing first: a name just given differs now.
        self.live_observe(Instant::now());
        let Some(live) = self.cloud.live.as_mut() else {
            return self.settled(true);
        };
        if live.sync.all_sent() {
            return self.settled(true);
        }
        if !live.sync.wants_to_send() {
            return self.settled(false);
        }
        live.send_soon();
        self.live_tick(Instant::now())
    }

    /// The autosave answered or failed while something waited on it: the
    /// waiting one goes on once everything went, or once sending stopped.
    pub(crate) fn settling_step(&mut self, failed: bool) -> Task<Message> {
        if self.cloud.settling.is_none() {
            return Task::none();
        }
        let Some(live) = self.cloud.live.as_ref() else {
            return self.settled(false);
        };
        if live.sync.all_sent() {
            return self.settled(true);
        }
        if failed || !live.sync.wants_to_send() || live.sync.state() == SaveState::Offline {
            return self.settled(false);
        }
        // More batches go at once; the next answer says.
        Task::none()
    }

    fn settled(&mut self, sent: bool) -> Task<Message> {
        match self.cloud.settling.take() {
            Some(Settle::Rename(name)) => {
                self.cloud.rename = None;
                if self.dialog == Some(Dialog::CloudRename) {
                    self.dialog = None;
                }
                if sent {
                    self.say_success(forms::rename::saved(&name));
                } else {
                    self.warn(forms::rename::waiting(&name));
                }
                Task::none()
            }
            Some(Settle::Trash) => self.send_trash(),
            Some(Settle::Catalog(act, p)) => self.catalog_send(act, *p),
            Some(Settle::Checkpoint(what)) => {
                let (p, input) = *what;
                self.checkpoint_send(&p, input)
            }
            Some(Settle::Metadata(what)) => self.metadata_renamed(*what, sent),
            Some(Settle::Convert(what)) => {
                let (p, input) = *what;
                self.convert_send(&p, input)
            }
            None => Task::none(),
        }
    }

    /// Çöpe taşı on the open project: what waits goes first (the web's `settle`).
    fn trash_open_project(&mut self) -> Task<Message> {
        if self.cloud.live.is_some() {
            self.cloud.settling = Some(Settle::Trash);
            return self.flush();
        }
        self.send_trash()
    }

    fn send_trash(&mut self) -> Task<Message> {
        let Some(source) = self.document.as_ref().and_then(|d| d.cloud_source()) else {
            return Task::none();
        };
        let Some(client) = self.cloud.signed_in().cloned() else {
            return Task::none();
        };
        let request = envelope(
            source.tenant,
            source.project,
            PROJECT_TRASH,
            PROJECT_TRASH_VERSION,
            Uuid::new_v4(),
            BTreeMap::new(),
            serde_json::to_value(EmptyInput {}).unwrap_or(json!({})),
        );
        let id = self.cloud.next_id();
        self.cloud.trashing = Some(id);
        Task::perform(
            client.command::<ProjectCatalogChange>(request),
            move |result| cloud(Event::Trashed { id, result }),
        )
    }

    fn trashed(
        &mut self,
        id: u64,
        result: Result<ProjectCatalogChange, ApiFailure>,
    ) -> Task<Message> {
        if self.cloud.trashing != Some(id) {
            return Task::none();
        }
        self.cloud.trashing = None;
        let name = self
            .document
            .as_ref()
            .and_then(|d| d.cloud_source())
            .map_or_else(String::new, |c| c.info.name.clone());
        match result {
            Ok(done) => {
                // The open project was moved to the trash from here: it is left, the drawing stays.
                self.detach_cloud();
                let until = done
                    .project
                    .purge_after
                    .as_deref()
                    .and_then(words::day)
                    .map_or_else(String::new, |day| {
                        format!(" ({day} tarihine kadar geri yüklenebilir)")
                    });
                self.output(format!(
                    "“{name}” bulut projesi çöp kutusuna taşındı{until}. Çizim ekranda kaldı; saklamak için Dosya → Farklı kaydet ile yerel bir dosyaya kaydedin."
                ));
            }
            Err(failure) => self.error(format!(
                "“{name}” çöp kutusuna taşınamadı: {}",
                reason(&failure)
            )),
        }
        Task::none()
    }

    fn say_success(&mut self, text: String) {
        self.say(kentos_interaction::Level::Success, text);
    }

    /// Yeniden adlandır (the web's `openRenameDialog`).
    pub(crate) fn rename_view(&self) -> Element<'_, Message> {
        let Some(r) = &self.cloud.rename else {
            return text("").into();
        };
        let field = text_input("", &r.text)
            .id(iced::widget::Id::new(RENAME_FIELD))
            .on_input_maybe((!r.busy).then_some(|t| cloud(Event::RenameInput(t))))
            .on_submit(cloud(Event::RenameSubmit))
            .padding([5, 8])
            .style(style::field::input);
        let field = kentos_ui::widget::focus_ring(field);
        let mut form = Form::new()
            .label_width(80.0)
            .field(forms::rename::NAME, field)
            .row(label::caption(forms::rename::HINT));
        match &r.status {
            Some(Status::Info(t)) => form = form.row(label::caption(t.clone())),
            Some(Status::Error(t)) => form = form.row(Banner::error(t.clone())),
            None => {}
        }
        overlay::modal(
            Window::new(forms::rename::TITLE)
                .push(form)
                .action(secondary(forms::CANCEL, Some(cloud(Event::Close))))
                .action(primary(
                    forms::rename::SAVE,
                    r.can_save().then(|| cloud(Event::RenameSubmit)),
                ))
                .width(460.0),
            cloud(Event::Close),
        )
    }

    /// Çöp kutusuna taşı (the web's `trashProject` question).
    pub(crate) fn trash_view(&self) -> Element<'_, Message> {
        let Some(source) = self.document.as_ref().and_then(|d| d.cloud_source()) else {
            return text("").into();
        };
        let details = [
            "Proje listelerden kalkar; kimse açamaz ve değiştiremez.",
            "Projeyi şu anda açık tutanların kaydı durur; gönderilmemiş değişiklikleri kendi cihazlarında kalır.",
            "Hiçbir şey silinmez: proje sahibi ya da kurum yöneticisi saklama süresi dolana kadar içinde Çöp kutusu’ndan geri yükleyebilir; sonra proje kalıcı olarak silinir.",
            "Proje şu anda sizde açık: çizim ekranda kalır, dilerseniz yerel bir dosyaya kaydedin.",
        ]
        .join("\n");
        overlay::modal(
            kentos_ui::widget::Confirm::new(
                "Çöp kutusuna taşı",
                cloud(Event::TrashConfirm),
                cloud(Event::Close),
            )
            .message(format!(
                "“{}” projesi ({}) erişimi olan herkes için çöp kutusuna taşınsın mı?",
                source.info.name, source.workspace
            ))
            .detail(details)
            .confirm("Çöpe taşı")
            .destructive(),
            cloud(Event::Close),
        )
    }
}
