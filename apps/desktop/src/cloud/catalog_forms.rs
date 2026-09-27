//! The catalog's project forms (docs/adr/0112; the web's ProjectForms.ts):
//! Proje bilgileri (name, type, description, tags), Kopyasını oluştur (a
//! copy of the account's, in a workspace it may open projects in) and a new
//! project in the other storage mode (PostGIS'e aktar, Dosya projesine
//! çevir; docs/adr/0039). Each is a window over the catalog and one product
//! command; the server checks and normalizes every value, and a refusal is
//! said in its words. What they say and decide is forms_plan.rs's
//! (fixtures/cloud/v1/forms.json).
//!
//! - Proje bilgileri sends only what changed, from the catalog version it
//!   showed (`@catalog`): a newer one on the server refuses and nothing is
//!   written. The open database project's new name goes through its
//!   autosave first, so it cannot conflict with itself; the rest follows
//!   with the version that rename made (the web's `updateMetadata`). An open
//!   file project's name is the catalog's: the drawing takes it quietly.
//! - A copy is shown selected in Projelerim; a converted project is shown
//!   and opened (the web's `openMade`). The open project's unsent edits go
//!   before a conversion (the web's `settle`).

use std::collections::BTreeMap;

use iced::Task;
use iced::task::Handle;
use iced::widget::text_editor;
use kentos_cloud::ApiFailure;
use kentos_cloud::saving::envelope;
use kentos_contracts::{
    CatalogView, PROJECT_CONVERT, PROJECT_CONVERT_VERSION, PROJECT_DUPLICATE,
    PROJECT_DUPLICATE_VERSION, PROJECT_METADATA_UPDATE, PROJECT_METADATA_UPDATE_VERSION,
    ProjectCatalogChange, ProjectConvert, ProjectDuplicate, ProjectDuplicated,
    ProjectMetadataUpdate, ProjectSummary, ProjectType,
};
use kentos_domain::{External, ExternalMeta, Uuid};
use kentos_expression::js::text::trim;
use kentos_interaction::Level;
use serde_json::json;

use crate::app::{App, Message};
use crate::cloud::actions::Settle;
use crate::cloud::catalog::{List, Said, Tab};
use crate::cloud::forms_plan::{self as plan, Metadata, Place, duplicate, metadata};
use crate::cloud::plan::{DetailAction, detail_plan};
use crate::cloud::{Event as CloudEvent, uuid};

/// The forms' first field, focused when they open.
pub const FORM_NAME: &str = "cloud-form-name";

/// Proje bilgileri: the version shown and the fields now.
#[derive(Debug)]
pub struct MetadataForm {
    pub project: ProjectSummary,
    pub name: String,
    pub project_type: ProjectType,
    pub description: text_editor::Content,
    pub tags: String,
    pub busy: bool,
    pub status: Option<Said>,
}

impl MetadataForm {
    /// What the fields say now (the tags split, the description trimmed).
    pub fn now(&self) -> Metadata {
        Metadata {
            name: self.name.clone(),
            project_type: self.project_type,
            description: trim(&self.description.text()).to_owned(),
            tags: plan::parse_tags(&self.tags),
        }
    }

    /// What the catalog showed.
    pub fn shown(&self) -> Metadata {
        Metadata {
            name: self.project.name.clone(),
            project_type: self.project.project_type,
            description: self.project.description.clone(),
            tags: self.project.tags.clone(),
        }
    }

    pub fn patch(&self) -> ProjectMetadataUpdate {
        plan::metadata_patch(&self.shown(), &self.now())
    }

    pub fn savable(&self) -> bool {
        !self.busy && plan::metadata_savable(&self.name, &self.patch())
    }
}

/// Kopyasını oluştur, or a new project in the other storage mode: its name
/// and workspace.
#[derive(Debug)]
pub struct MadeForm {
    pub project: ProjectSummary,
    /// The conversion's words; none for a copy.
    pub convert: Option<plan::ConvertForm>,
    pub name: String,
    pub places: Vec<Place>,
    pub place: usize,
    pub busy: bool,
    pub status: Option<Said>,
}

impl MadeForm {
    pub fn savable(&self) -> bool {
        !self.busy
            && match self.convert {
                None => plan::duplicate_savable(self.places.len(), &self.name),
                Some(_) => !self.places.is_empty(),
            }
    }
}

/// A form over the catalog.
#[derive(Debug)]
pub enum Form {
    Metadata(Box<MetadataForm>),
    Made(Box<MadeForm>),
}

impl Form {
    pub fn busy(&self) -> bool {
        match self {
            Self::Metadata(f) => f.busy,
            Self::Made(f) => f.busy,
        }
    }
}

/// What a form's request answered.
#[derive(Debug, Clone)]
pub enum Done {
    Metadata(Box<ProjectCatalogChange>),
    Made(Box<ProjectDuplicated>),
    /// The catalog version after the open project's rename (then the rest goes).
    Version(String),
}

/// The forms' messages.
#[derive(Debug, Clone)]
pub enum Event {
    /// The pane's buttons.
    Edit,
    Duplicate,
    Convert,
    Name(String),
    Type(ProjectType),
    Description(text_editor::Action),
    Tags(String),
    Place(usize),
    Submit,
    Close,
    Done {
        id: u64,
        result: Result<Done, ApiFailure>,
    },
}

/// The app's message of a form event.
pub fn msg(event: Event) -> Message {
    crate::cloud::msg(CloudEvent::Form(event))
}

/// The request on its way: its id and handle (dropping it stops it).
pub struct Asked(pub u64, #[allow(dead_code)] Handle);

impl App {
    /// The forms' messages.
    pub(crate) fn project_form_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Edit => self.project_form_open(DetailAction::Edit),
            Event::Duplicate => self.project_form_open(DetailAction::Duplicate),
            Event::Convert => self.project_form_open(DetailAction::Convert),
            Event::Close => {
                self.project_form_close();
                Task::none()
            }
            Event::Submit => self.project_form_submit(),
            Event::Done { id, result } => self.project_form_done(id, result),
            edit => {
                let Some(form) = self
                    .cloud
                    .catalog
                    .as_mut()
                    .and_then(|c| c.form.as_mut())
                    .filter(|f| !f.busy())
                else {
                    return Task::none();
                };
                // The fields take no more than the web's (their maxlength), in UTF-16 units.
                let fits = |t: &str, max: usize| t.encode_utf16().count() <= max;
                match (form, edit) {
                    (Form::Metadata(f), Event::Name(t)) if fits(&t, plan::NAME_MAX) => f.name = t,
                    (Form::Metadata(f), Event::Type(t)) => f.project_type = t,
                    (Form::Metadata(f), Event::Description(action)) => {
                        let before = action.is_edit().then(|| f.description.text());
                        f.description.perform(action);
                        if let Some(before) = before
                            && !fits(&f.description.text(), plan::DESCRIPTION_MAX)
                        {
                            f.description = text_editor::Content::with_text(&before);
                        }
                    }
                    (Form::Metadata(f), Event::Tags(t)) => f.tags = t,
                    (Form::Made(f), Event::Name(t)) if fits(&t, plan::NAME_MAX) => f.name = t,
                    (Form::Made(f), Event::Place(i)) if i < f.places.len() => f.place = i,
                    _ => {}
                }
                Task::none()
            }
        }
    }

    /// The pane's Bilgileri düzenle…, Kopyasını oluştur… or the conversion,
    /// when the plan offers it on the selected project.
    fn project_form_open(&mut self, action: DetailAction) -> Task<Message> {
        let busy = self.cloud.opening.is_some();
        let Some(p) = self
            .cloud
            .catalog
            .as_ref()
            .filter(|c| !c.busy() && !busy)
            .and_then(|c| c.picked().cloned())
        else {
            return Task::none();
        };
        let open = self.is_open_project(&p.id);
        let offered = detail_plan(&p, open)
            .actions
            .iter()
            .any(|a| a.id == action && a.why.is_none());
        if !offered {
            return Task::none();
        }
        let places = self.creatable_places(&p.tenant_id);
        let no_place = places.is_empty().then(|| Said::error(plan::NO_PLACE));
        let form = match action {
            DetailAction::Edit => Form::Metadata(Box::new(MetadataForm {
                name: p.name.clone(),
                project_type: p.project_type,
                description: text_editor::Content::with_text(&p.description),
                tags: p.tags.join(", "),
                busy: false,
                status: None,
                project: p,
            })),
            DetailAction::Duplicate => Form::Made(Box::new(MadeForm {
                convert: None,
                name: plan::copy_name(&p.name),
                places,
                place: 0,
                busy: false,
                status: no_place,
                project: p,
            })),
            _ => {
                let dirty = open && self.document.as_ref().is_some_and(|d| d.dirty());
                Form::Made(Box::new(MadeForm {
                    convert: Some(plan::convert_form(&p.name, p.storage, dirty)),
                    name: String::new(),
                    places,
                    place: 0,
                    busy: false,
                    status: no_place,
                    project: p,
                }))
            }
        };
        let select = matches!(&form, Form::Made(f) if f.convert.is_none());
        if let Some(c) = self.cloud.catalog.as_mut() {
            c.form = Some(form);
        }
        let field = iced::widget::Id::new(FORM_NAME);
        if select {
            // The copy's name is offered, ready to be typed over.
            return Task::batch([
                iced::widget::operation::focus(field.clone()),
                iced::widget::operation::select_all(field),
            ]);
        }
        iced::widget::operation::focus(field)
    }

    /// The workspaces the account may open projects in, the source's first.
    pub(crate) fn creatable_places(&self, first: &str) -> Vec<Place> {
        self.cloud.me.as_ref().map_or_else(Vec::new, |me| {
            plan::creatable_places(&me.memberships, first)
        })
    }

    /// Vazgeç: the form goes unless its request is on its way.
    pub(crate) fn project_form_close(&mut self) {
        if let Some(c) = self.cloud.catalog.as_mut()
            && c.form.as_ref().is_some_and(|f| !f.busy())
        {
            c.form = None;
        }
    }

    fn project_form_submit(&mut self) -> Task<Message> {
        let Some(form) = self.cloud.catalog.as_mut().and_then(|c| c.form.as_mut()) else {
            return Task::none();
        };
        match form {
            Form::Metadata(f) => {
                if !f.savable() {
                    return Task::none();
                }
                let patch = f.patch();
                let p = f.project.clone();
                f.busy = true;
                f.status = Some(Said::info(plan::SAVING));
                self.metadata_send(p, patch)
            }
            Form::Made(f) => {
                if !f.savable() {
                    return Task::none();
                }
                let name = trim(&f.name).to_owned();
                let tenant = f.places.get(f.place).map(|p| p.tenant_id.clone());
                let p = f.project.clone();
                f.busy = true;
                match &f.convert {
                    None => {
                        f.status = Some(Said::info(duplicate::RUNNING));
                        let input = ProjectDuplicate {
                            name: (!name.is_empty()).then_some(name),
                            tenant_id: tenant,
                        };
                        self.project_form_command(
                            &p,
                            PROJECT_DUPLICATE,
                            PROJECT_DUPLICATE_VERSION,
                            serde_json::to_value(input).unwrap_or(json!({})),
                            BTreeMap::new(),
                            |made: ProjectDuplicated| Done::Made(Box::new(made)),
                        )
                    }
                    Some(convert) => {
                        f.status = Some(Said::info(convert.running));
                        let input = ProjectConvert {
                            to: convert.to,
                            name: (!name.is_empty()).then_some(name),
                            tenant_id: tenant,
                        };
                        // The open project's unsent edits go first (the web's `settle`).
                        if self.is_open_project(&p.id) && self.cloud.live.is_some() {
                            self.cloud.settling = Some(Settle::Convert(Box::new((p, input))));
                            return self.flush();
                        }
                        self.convert_send(&p, input)
                    }
                }
            }
        }
    }

    /// Converts once the open project's edits went (or at once).
    pub(crate) fn convert_send(
        &mut self,
        p: &ProjectSummary,
        input: ProjectConvert,
    ) -> Task<Message> {
        self.project_form_command(
            p,
            PROJECT_CONVERT,
            PROJECT_CONVERT_VERSION,
            serde_json::to_value(input).unwrap_or(json!({})),
            BTreeMap::new(),
            |made: ProjectDuplicated| Done::Made(Box::new(made)),
        )
    }

    /// Proje bilgileri's patch: the open database project's new name through
    /// its autosave first, the rest after it; otherwise all at once.
    fn metadata_send(
        &mut self,
        p: ProjectSummary,
        mut patch: ProjectMetadataUpdate,
    ) -> Task<Message> {
        let open_database = self.is_open_project(&p.id)
            && self.document.as_ref().is_some_and(|d| d.is_database())
            && self.cloud.live.is_some();
        if open_database && let Some(name) = patch.name.take() {
            let differs = self.document.as_ref().is_some_and(|d| d.name() != name);
            if differs {
                if let Some(doc) = self.document.as_mut() {
                    doc.model.set_name(&name);
                    if let Some(source) = doc.cloud_source_mut() {
                        source.info.name.clone_from(&name);
                    }
                }
                self.cloud.settling = Some(Settle::Metadata(Box::new((p, patch, name))));
                return self.flush();
            }
        }
        let expected = BTreeMap::from([("@catalog".to_owned(), p.catalog_version.clone())]);
        self.project_form_command(
            &p,
            PROJECT_METADATA_UPDATE,
            PROJECT_METADATA_UPDATE_VERSION,
            serde_json::to_value(patch).unwrap_or(json!({})),
            expected,
            |change: ProjectCatalogChange| Done::Metadata(Box::new(change)),
        )
    }

    /// The open database project's new name went (or waits): the rest goes
    /// with the catalog version that rename made, or the form is done.
    pub(crate) fn metadata_renamed(
        &mut self,
        what: (ProjectSummary, ProjectMetadataUpdate, String),
        sent: bool,
    ) -> Task<Message> {
        let (p, rest, name) = what;
        if !sent {
            if let Some(Form::Metadata(f)) =
                self.cloud.catalog.as_mut().and_then(|c| c.form.as_mut())
            {
                f.busy = false;
                f.status = Some(Said::error(
                    "Yeni ad kaydedilemedi; durum çubuğundaki kayıt durumuna bakın (çakışma ya da bağlantı).",
                ));
            }
            return Task::none();
        }
        self.say(Level::Info, plan::rename::saved(&name));
        if !plan::changes(&rest) {
            return self.metadata_saved(&name);
        }
        // The version the rename made, then the rest.
        let (Some(client), Some(tenant), Some(project)) = (
            self.cloud.signed_in().cloned(),
            uuid(&p.tenant_id),
            uuid(&p.id),
        ) else {
            return Task::none();
        };
        if let Some(Form::Metadata(f)) = self.cloud.catalog.as_mut().and_then(|c| c.form.as_mut()) {
            f.project.name.clone_from(&name);
        }
        let id = self.cloud.next_id();
        let (task, handle) = Task::perform(client.details(tenant, project), move |result| {
            msg(Event::Done {
                id,
                result: result.map(|d| Done::Version(d.project.catalog_version)),
            })
        })
        .abortable();
        if let Some(c) = self.cloud.catalog.as_mut() {
            c.form_asked = Some(Asked(id, handle.abort_on_drop()));
        }
        task
    }

    /// One command of a form, its answer carrying the request's id.
    fn project_form_command<T: serde::de::DeserializeOwned + Send + 'static>(
        &mut self,
        p: &ProjectSummary,
        name: &str,
        version: u32,
        input: serde_json::Value,
        expected: BTreeMap<String, String>,
        wrap: fn(T) -> Done,
    ) -> Task<Message> {
        let (Some(client), Some(tenant), Some(project)) = (
            self.cloud.signed_in().cloned(),
            uuid(&p.tenant_id),
            uuid(&p.id),
        ) else {
            return Task::none();
        };
        let id = self.cloud.next_id();
        let request = envelope(
            tenant,
            project,
            name,
            version,
            Uuid::new_v4(),
            expected,
            input,
        );
        let (task, handle) = Task::perform(client.command::<T>(request), move |result| {
            msg(Event::Done {
                id,
                result: result.map(wrap),
            })
        })
        .abortable();
        if let Some(c) = self.cloud.catalog.as_mut() {
            c.form_asked = Some(Asked(id, handle.abort_on_drop()));
        }
        task
    }

    fn project_form_done(&mut self, id: u64, result: Result<Done, ApiFailure>) -> Task<Message> {
        let Some(c) = self
            .cloud
            .catalog
            .as_mut()
            .filter(|c| c.form_asked.as_ref().is_some_and(|a| a.0 == id))
        else {
            return Task::none();
        };
        c.form_asked = None;
        match result {
            Ok(Done::Version(version)) => {
                let Some(Form::Metadata(f)) = c.form.as_mut() else {
                    return Task::none();
                };
                f.project.catalog_version = version;
                let rest = f.patch();
                let rest = ProjectMetadataUpdate { name: None, ..rest };
                let p = f.project.clone();
                let expected = BTreeMap::from([("@catalog".to_owned(), p.catalog_version.clone())]);
                self.project_form_command(
                    &p,
                    PROJECT_METADATA_UPDATE,
                    PROJECT_METADATA_UPDATE_VERSION,
                    serde_json::to_value(rest).unwrap_or(json!({})),
                    expected,
                    |change: ProjectCatalogChange| Done::Metadata(Box::new(change)),
                )
            }
            Ok(Done::Metadata(change)) => {
                let name = change.project.name.clone();
                // An open file project's name is the catalog's: the drawing takes it quietly.
                if self.is_open_project(&change.project.id)
                    && let Some(doc) = self.document.as_mut()
                    && !doc.is_database()
                    && doc.name() != name
                {
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
                self.metadata_saved(&name)
            }
            Ok(Done::Made(made)) => {
                let convert = match c.form.take() {
                    Some(Form::Made(f)) => f.convert,
                    _ => None,
                };
                match convert {
                    None => {
                        self.say(
                            Level::Success,
                            duplicate::done(&made.project.name, &made.objects),
                        );
                        // The copy is the account's own: shown selected in Projelerim.
                        self.project_form_show(&made.project.id, false)
                    }
                    Some(convert) => {
                        self.say(
                            Level::Success,
                            plan::converted_line(&made.project.name, &made.objects, convert.to),
                        );
                        // The new project is shown and opened (the web's `openMade`).
                        self.project_form_show(&made.project.id, true)
                    }
                }
            }
            Err(failure) => {
                let converting = matches!(&c.form, Some(Form::Made(f)) if f.convert.is_some());
                let why = if converting {
                    plan::convert_reason(&failure)
                } else {
                    plan::failure_reason(&failure)
                };
                match c.form.as_mut() {
                    Some(Form::Metadata(f)) => {
                        f.busy = false;
                        f.status = Some(Said::error(why));
                    }
                    Some(Form::Made(f)) => {
                        f.busy = false;
                        f.status = Some(Said::error(why));
                    }
                    None => {}
                }
                Task::none()
            }
        }
    }

    /// Proje bilgileri saved: the window goes, the line, the list again.
    fn metadata_saved(&mut self, name: &str) -> Task<Message> {
        if let Some(c) = self.cloud.catalog.as_mut() {
            c.form = None;
        }
        self.say(Level::Success, metadata::saved(trim(name)));
        self.catalog_load(false)
    }

    /// A project made here: shown selected in Projelerim, and opened as
    /// soon as the list shows it when `open`.
    fn project_form_show(&mut self, id: &str, open: bool) -> Task<Message> {
        let Some(c) = self.cloud.catalog.as_mut() else {
            return Task::none();
        };
        c.wanted = Some((id.to_owned(), open));
        c.tab = Tab::Info;
        // Whatever was being searched: the new project is the one to show.
        c.search.clear();
        c.search_at = None;
        c.kind = None;
        self.catalog_event(CloudEvent::CatalogView(List::View(CatalogView::Mine)))
    }
}
