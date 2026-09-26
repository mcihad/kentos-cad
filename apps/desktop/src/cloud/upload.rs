//! “Buluta yükle” (docs/adr/0041; the web's UploadDialog.ts): the drawing on
//! screen becomes a new cloud project in a workspace where the account may
//! open projects (`project.create`), under a name, kept as a file (every
//! save a revision) or in the database (PostGIS). The drawing is written
//! into verified `.kcad` v2 bytes off the UI thread, kentos-cloud creates the
//! project and fills it (a refused drawing leaves no project behind), and
//! the new project is then opened from the server, so the drawing on screen
//! is the cloud project. A refusal names the object and selects it. A retry
//! of the same upload keeps its idempotency key: the server answers with the
//! same project instead of making a second one.

use iced::Task;
use iced::futures::channel::mpsc;
use iced::task::Handle;
use kentos_cloud::{ApiFailure, Uploaded, project_create, upload_new};
use kentos_contracts::{MembershipView, ProjectInfo, ProjectStorage};
use kentos_domain::{Slot, Uuid};

use crate::app::{App, Dialog, Message};
use crate::cloud::{Event, Once, uuid, words};

/// The upload window.
pub struct Upload {
    /// The workspaces where the account may open projects.
    pub tenants: Vec<MembershipView>,
    pub tenant: usize,
    pub name: String,
    pub storage: ProjectStorage,
    /// One per upload the user asked for; a retry after a failure keeps it.
    key: Uuid,
    /// What it is doing now, and how far.
    pub stage: Option<String>,
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
    /// Opens the window on the drawing on screen.
    pub(crate) fn open_upload(&mut self) {
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
            storage: ProjectStorage::Database,
            key: Uuid::new_v4(),
            stage: None,
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
        u.stage = Some(format!("Çizim hazırlanıyor: {} nesne", doc.entity_count()));
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

    /// Vazgeç while it runs: the request is dropped.
    pub(crate) fn upload_stop(&mut self) {
        if let Some(u) = self.cloud.upload.as_mut()
            && u.work.take().is_some()
        {
            u.stage = None;
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
                u.error = Some(format!("Çizim yazılamadı: {why}"));
                return Task::none();
            }
        };
        let Some(client) = client else {
            u.work = None;
            u.stage = None;
            u.error = Some("Bulut oturumu kapandı; yeniden giriş yapın.".to_owned());
            return Task::none();
        };
        let Some(tenant) = u.tenants.get(u.tenant).and_then(|m| uuid(&m.tenant_id)) else {
            u.work = None;
            return Task::none();
        };
        let create = project_create(&doc.model, &u.name, u.storage);
        u.stage = Some(format!(
            "Buluta gönderiliyor: {} nesne, {:.1} MB",
            doc.entity_count(),
            bytes.len() as f64 / 1e6
        ));
        let (task, handle) = Task::perform(
            upload_new(&client, tenant, create, bytes, u.key),
            move |result| crate::cloud::msg(Event::Uploaded { id, result }),
        )
        .abortable();
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
        match result {
            Err(failure) => {
                let drawing = u.drawing;
                let place = refused_place(&failure);
                let named = place.and_then(|i| self.name_object(i, drawing));
                if let Some(u) = self.cloud.upload.as_mut() {
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
