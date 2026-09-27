//! “Buluta yükle” (docs/adr/0041; the web's UploadDialog.ts): the drawing on
//! screen becomes a new cloud project in a workspace where the account may
//! open projects (`project.create`), under a name, kept as a file (every
//! save a revision) or in the database (PostGIS). The drawing is written
//! into verified `.kcad` v2 bytes off the UI thread, kentos-cloud creates the
//! project and fills it (a refused drawing leaves no project behind), and
//! the new project is then opened from the server, so the drawing on screen
//! is the cloud project. A drawing that changed on its way (other people's
//! changes to an open database project) is not replaced: the project holds
//! it as it went up, the window closes and the log says so (the web's
//! f7616e4). A refusal names the object and selects it. A retry of an
//! upload that got no answer keeps its idempotency key: the server answers
//! with the same project instead of making a second one. A refusal for good
//! trashed the empty project, so the next try is a new upload with a new key.

use std::sync::Arc;

use iced::Task;
use iced::futures::channel::mpsc;
use iced::futures::stream;
use iced::task::Handle;
use iced::widget::text_editor;
use kentos_cloud::{ApiFailure, Uploaded, project_create, upload_new_watched};
use kentos_contracts::{MembershipView, ProjectInfo, ProjectStorage, ProjectType};
use kentos_domain::{Slot, Uuid};
use kentos_expression::js::text::trim;

use crate::app::{App, Dialog, Message};
use crate::cloud::forms_plan::parse_tags;
use crate::cloud::{Event, Once, uuid, words};

/// The stages the window says (the web's UploadDialog `STAGE_TEXT`).
pub(crate) mod stages {
    pub(crate) const CREATING: &str = "Proje oluşturuluyor…";
    pub(crate) const ENCODING: &str = "Çizim KCAD v2 olarak hazırlanıyor…";
    pub(crate) const VERIFYING: &str = "Sunucu dosyayı doğruluyor…";
    pub(crate) const IMPORTING: &str = "Çizim veritabanına aktarılıyor (tek işlemde)…";
}

/// The upload window.
pub struct Upload {
    /// The workspaces where the account may open projects.
    pub tenants: Vec<MembershipView>,
    pub tenant: usize,
    pub name: String,
    pub storage: ProjectStorage,
    /// The catalog's fields of the new project (the web's `catalogFields`).
    pub project_type: ProjectType,
    pub description: text_editor::Content,
    pub tags: String,
    /// One per upload the user asked for; a retry after a failure keeps it.
    key: Uuid,
    /// What it is doing now, and how far (0…1, when known).
    pub stage: Option<String>,
    pub fraction: Option<f32>,
    pub error: Option<String>,
    /// The step on its way: its id and request.
    work: Option<(u64, Option<Handle>)>,
    /// The drawing being uploaded: which one, and its revision then.
    drawing: Option<(u64, u64)>,
    /// A file conflict's “Ayrı proje olarak kaydet”: the save kept on this
    /// device for the old project is done with once this one is up.
    pub from_conflict: bool,
}

impl Upload {
    pub fn working(&self) -> bool {
        self.work.is_some()
    }

    /// The step on its way (tests answer it), and the upload's key.
    #[cfg(test)]
    pub(super) fn request(&self) -> Option<(u64, Uuid)> {
        self.work.as_ref().map(|(id, _)| (*id, self.key))
    }

    /// The upload's idempotency key (tests).
    #[cfg(test)]
    pub(super) fn key_for_tests(&self) -> Uuid {
        self.key
    }

    /// Chooses the workspace, name and storage (Ayrı proje olarak kaydet);
    /// false when the account may not open projects in that workspace.
    pub fn choose(&mut self, tenant: &str, name: &str, storage: ProjectStorage) -> bool {
        self.name = name.to_owned();
        self.storage = storage;
        match self.tenants.iter().position(|m| m.tenant_id == tenant) {
            Some(i) => {
                self.tenant = i;
                true
            }
            None => false,
        }
    }

    /// Something the upload is made of changed: a new upload, with a new key.
    fn changed(&mut self) {
        self.error = None;
        self.key = Uuid::new_v4();
    }
}

/// The object a refusal names (`entities[13]`): its place in the file.
fn refused_place(failure: &ApiFailure) -> Option<usize> {
    let path = failure.path.as_deref()?;
    path.strip_prefix("entities[")?
        .strip_suffix(']')?
        .parse()
        .ok()
}

impl App {
    /// Opens the window on the drawing on screen, starting on `storage`
    /// (Buluta dosya olarak kaydet: file).
    pub(crate) fn open_upload(&mut self, storage: ProjectStorage) {
        let Some(doc) = &self.document else {
            return;
        };
        let tenants: Vec<MembershipView> = self.cloud.me.as_ref().map_or_else(Vec::new, |me| {
            me.memberships
                .iter()
                .filter(|m| {
                    m.active && m.seat && m.capabilities.iter().any(|c| c == "project.create")
                })
                .cloned()
                .collect()
        });
        // The open project's workspace first, when the account may open projects there.
        let open = doc.cloud_source().map(|c| c.info.tenant_id.clone());
        let tenant = open
            .and_then(|t| tenants.iter().position(|m| m.tenant_id == t))
            .unwrap_or(0);
        self.cloud.upload = Some(Upload {
            tenants,
            tenant,
            name: doc.name().to_owned(),
            storage,
            project_type: ProjectType::Cad,
            description: text_editor::Content::new(),
            tags: String::new(),
            key: Uuid::new_v4(),
            stage: None,
            fraction: None,
            error: None,
            work: None,
            drawing: None,
            from_conflict: false,
        });
        self.dialog = Some(Dialog::Upload);
    }

    pub(crate) fn upload_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::UploadTenant(i) => {
                if let Some(u) = self.cloud.upload.as_mut().filter(|u| !u.working())
                    && i < u.tenants.len()
                {
                    u.tenant = i;
                    u.changed();
                }
            }
            Event::UploadName(name) => {
                if let Some(u) = self.cloud.upload.as_mut().filter(|u| !u.working()) {
                    u.name = name;
                    u.changed();
                }
            }
            Event::UploadStorage(storage) => {
                if let Some(u) = self.cloud.upload.as_mut().filter(|u| !u.working()) {
                    u.storage = storage;
                    u.changed();
                }
            }
            // The catalog's fields are part of what the upload asks for: a change is a new upload.
            Event::UploadType(kind) => {
                if let Some(u) = self.cloud.upload.as_mut().filter(|u| !u.working()) {
                    u.project_type = kind;
                    u.changed();
                }
            }
            Event::UploadDescription(action) => {
                if let Some(u) = self.cloud.upload.as_mut().filter(|u| !u.working()) {
                    let edits = action.is_edit();
                    u.description.perform(action);
                    if edits {
                        u.changed();
                    }
                }
            }
            Event::UploadTags(tags) => {
                if let Some(u) = self.cloud.upload.as_mut().filter(|u| !u.working()) {
                    u.tags = tags;
                    u.changed();
                }
            }
            Event::UploadProgress { id, done, total } => self.upload_progress(id, done, total),
            Event::UploadSubmit => return self.upload_submit(),
            Event::UploadStop => self.upload_stop(),
            Event::UploadEncoded { id, result } => return self.upload_encoded(id, result),
            Event::Uploaded { id, result } => return self.uploaded(id, result),
            _ => {}
        }
        Task::none()
    }

    /// Yükle: the drawing into bytes off the UI thread first.
    pub(crate) fn upload_submit(&mut self) -> Task<Message> {
        let id = self.cloud.next_id();
        let (Some(u), Some(doc)) = (self.cloud.upload.as_mut(), self.document.as_ref()) else {
            return Task::none();
        };
        if u.working() {
            return Task::none();
        }
        if u.tenants.is_empty() {
            u.error = Some("Proje açma yetkiniz olan bir çalışma alanı yok (project.create); kurum yöneticinize başvurun.".to_owned());
            return Task::none();
        }
        let name = u.name.trim();
        if name.is_empty() || name.chars().count() > 200 {
            u.error = Some("Proje adı boş olamaz ve en çok 200 karakter olabilir.".to_owned());
            return Task::none();
        }
        u.error = None;
        u.stage = Some(stages::ENCODING.to_owned());
        u.fraction = None;
        u.drawing = Some((doc.session, doc.model.revision()));
        u.work = Some((id, None));
        let model = doc.model.clone();
        // The file is the project: it carries the project's name (a file project's drawing is named by it).
        let name = name.to_owned();
        iced_runtime::task::blocking(move |mut out: mpsc::Sender<Message>| {
            let mut snapshot = model.to_snapshot_v2();
            drop(model);
            snapshot.name = name;
            let result = kentos_kcad::encode_verified(&snapshot)
                .map(Once::new)
                .map_err(|e| e.message);
            let _ = iced::futures::executor::block_on(iced::futures::SinkExt::send(
                &mut out,
                crate::cloud::msg(Event::UploadEncoded { id, result }),
            ));
        })
    }

    /// How far the file is: a percentage and the sizes, then what the server
    /// does with it (the web's `onProgress` and `STAGE_TEXT`).
    fn upload_progress(&mut self, id: u64, done: u64, total: u64) {
        let Some(u) = self
            .cloud
            .upload
            .as_mut()
            .filter(|u| u.work.as_ref().is_some_and(|(w, _)| *w == id))
        else {
            return;
        };
        if total > 0 && done < total {
            #[allow(clippy::cast_precision_loss)]
            let fraction = done as f64 / total as f64;
            // JavaScript's Math.round: halves up.
            let percent = (fraction * 100.0 + 0.5).floor();
            u.fraction = Some(fraction as f32);
            u.stage = Some(format!(
                "Yükleniyor: %{percent} ({} / {})",
                words::size_text(usize::try_from(done).unwrap_or(usize::MAX)),
                words::size_text(usize::try_from(total).unwrap_or(usize::MAX))
            ));
        } else {
            u.fraction = Some(1.0);
            u.stage = Some(
                match u.storage {
                    ProjectStorage::File => stages::VERIFYING,
                    ProjectStorage::Database => stages::IMPORTING,
                }
                .to_owned(),
            );
        }
    }

    /// Vazgeç while it runs: the request is dropped.
    pub(crate) fn upload_stop(&mut self) {
        if let Some(u) = self.cloud.upload.as_mut()
            && u.work.take().is_some()
        {
            u.stage = None;
            u.fraction = None;
            u.error = Some("Yükleme durduruldu. Sunucuda yarım bir proje kaldıysa Projelerim'de görünür; yeniden yükleyince aynı proje kullanılır.".to_owned());
        }
    }

    fn upload_encoded(&mut self, id: u64, result: Result<Once<Vec<u8>>, String>) -> Task<Message> {
        let client = self.cloud.signed_in().cloned();
        let (Some(u), Some(doc)) = (
            self.cloud
                .upload
                .as_mut()
                .filter(|u| u.work.as_ref().is_some_and(|(w, _)| *w == id)),
            self.document.as_ref(),
        ) else {
            return Task::none();
        };
        let bytes = match result.map(|b| b.take()) {
            Ok(Some(bytes)) => bytes,
            Ok(None) => return Task::none(),
            Err(why) => {
                u.work = None;
                u.stage = None;
                u.fraction = None;
                u.error = Some(format!("Çizim yazılamadı: {why}"));
                return Task::none();
            }
        };
        let Some(client) = client else {
            u.work = None;
            u.stage = None;
            u.fraction = None;
            u.error = Some("Bulut oturumu kapandı; yeniden giriş yapın.".to_owned());
            return Task::none();
        };
        let Some(tenant) = u.tenants.get(u.tenant).and_then(|m| uuid(&m.tenant_id)) else {
            u.work = None;
            return Task::none();
        };
        // The catalog's fields go with the project (the web's `createInput`).
        let mut create = project_create(&doc.model, &u.name, u.storage);
        create.project_type = Some(u.project_type);
        let description = trim(&u.description.text()).to_owned();
        create.description = (!description.is_empty()).then_some(description);
        let tags = parse_tags(&u.tags);
        create.tags = (!tags.is_empty()).then_some(tags);
        u.stage = Some(stages::CREATING.to_owned());
        u.fraction = None;
        // How far the file is: the project is made by the first part.
        let (tell, told) = mpsc::unbounded();
        let progress: kentos_cloud::Progress = Arc::new(move |done, total| {
            let _ =
                tell.unbounded_send(crate::cloud::msg(Event::UploadProgress { id, done, total }));
        });
        let upload = upload_new_watched(&client, tenant, create, bytes, u.key, Some(progress));
        let done = stream::once(async move {
            crate::cloud::msg(Event::Uploaded {
                id,
                result: upload.await,
            })
        });
        let (task, handle) = Task::stream(stream::select(told, done)).abortable();
        u.work = Some((id, Some(handle.abort_on_drop())));
        task
    }

    fn uploaded(
        &mut self,
        id: u64,
        result: Result<(ProjectInfo, Uploaded), ApiFailure>,
    ) -> Task<Message> {
        let Some(u) = self
            .cloud
            .upload
            .as_mut()
            .filter(|u| u.work.as_ref().is_some_and(|(w, _)| *w == id))
        else {
            return Task::none();
        };
        u.work = None;
        u.stage = None;
        u.fraction = None;
        let drawing = u.drawing;
        match result {
            Err(failure) => {
                let place = refused_place(&failure);
                let named = place.and_then(|i| self.name_object(i, drawing));
                if let Some(u) = self.cloud.upload.as_mut() {
                    // Refused for good, the empty project went to the trash
                    // (kentos-cloud's upload_new): the next try is a new upload.
                    // Without an answer the project stays, and the same key finds it.
                    if !failure.transient() {
                        u.key = Uuid::new_v4();
                    }
                    u.error = Some(match named {
                        Some(what) => format!(
                            "{} Reddedilen nesne: {what}; çizimde seçildi.",
                            failure.message
                        ),
                        None => failure.message.clone(),
                    });
                }
                Task::none()
            }
            Ok((info, uploaded)) => {
                let (Some(tenant), Some(project)) = (uuid(&info.tenant_id), uuid(&info.id)) else {
                    return Task::none();
                };
                let objects = match &uploaded {
                    Uploaded::File(c) => c.objects.clone().unwrap_or_else(|| "?".to_owned()),
                    Uploaded::Database(i) => i.objects.clone(),
                };
                let own = self.cloud.membership(&info.tenant_id).is_some();
                let place = words::workspace(info.tenant_kind, &info.tenant_name, own);
                // Changed on its way: opening the project would replace the
                // change, so the drawing stays local, with it (and a conflict's
                // kept save stays too: the project does not hold the newest).
                let now = self
                    .document
                    .as_ref()
                    .map(|d| (d.session, d.model.revision()));
                if now != drawing {
                    self.cloud.upload = None;
                    if self.dialog == Some(Dialog::Upload) {
                        self.dialog = None;
                    }
                    self.warn(format!(
                        "“{}” bulut projesi oluşturuldu ve çizim içe aktarıldı ({objects} nesne), ama çizim yükleme sürerken değişti; ekrandaki çizim projeye bağlanmadı ve değişiklikleri yerinde duruyor. Projeyi Bulut projesi aç ile açın.",
                        info.name
                    ));
                    return Task::none();
                }
                // Both works are kept: the old project's newest stays, this one is up now.
                if self.cloud.upload.as_ref().is_some_and(|u| u.from_conflict)
                    && let Some(held) = self.cloud.held.as_mut()
                    && held.replica.clear_save().is_ok()
                {
                    held.kept_save = false;
                }
                self.cloud.upload = None;
                if self.dialog == Some(Dialog::Upload) {
                    self.dialog = None;
                }
                self.say(
                    kentos_interaction::Level::Success,
                    format!(
                        "“{}” buluta yüklendi ({place}, {}): {objects} nesne. Proje sunucudan açılıyor.",
                        info.name,
                        words::storage_title(info.storage)
                    ),
                );
                // The drawing on screen becomes the cloud project: read back from the server.
                self.cloud.open_hint = Some((info.name.clone(), info.storage));
                self.start_cloud_open(tenant, project)
            }
        }
    }

    /// The object at a file's place `i` (the drawing unchanged since the
    /// upload), named and selected: “14. nesne: Nokta · Parseller”.
    fn name_object(&mut self, i: usize, drawing: Option<(u64, u64)>) -> Option<String> {
        let doc = self.document.as_ref()?;
        if drawing != Some((doc.session, doc.model.revision())) {
            return None;
        }
        let entity = doc.model.entities().nth(i)?;
        let base = entity.base();
        let layer = doc
            .model
            .layers()
            .get(&base.layer_id)
            .map_or(base.layer_id.clone(), |l| l.name.clone());
        let label = base
            .label
            .as_deref()
            .filter(|l| !l.is_empty())
            .map(|l| format!(" · {l}"))
            .unwrap_or_default();
        let what = format!(
            "{}. nesne: {} · {layer}{label}",
            i + 1,
            words::kind(entity.kind())
        );
        let slot = Slot(base.id);
        self.selection.set([slot]);
        Some(what)
    }
}
