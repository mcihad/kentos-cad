//! A program's cloud projects (docs/adr/0802): a module's program lists, offers and opens only its own module's
//! catalog types (kentos-launch's `Module::project_types`): KentOS Mimari a building's (`architecture`), KentOS
//! Harita every CAD type that is not a building's (general CAD, land readjustment, zoning plan, subdivision, road),
//! KentOS GIS a CBS project's; kentos-cad every type. A project of another module is not opened: the program it
//! belongs to is said and the drawing on screen stays as it is (handing it over by its id is a later step,
//! docs/adr/0801 §8). Its type is the catalog's: a row's, or asked of the server (its catalog entry) when only its id
//! is known (the application menu's recent projects). This device's copies, whose type is not kept, are listed by
//! their drawing's module (`program::module_of`).

use iced::Task;
use kentos_cloud::ApiFailure;
use kentos_commands::{ProjectInfo, ProjectStorage, ProjectType};
use kentos_domain::Uuid;
use kentos_launch::Module;

use crate::app::{App, Dialog, Message, Then};
use crate::cloud::{Event, plan};
use crate::program::{in_program, module_of, of_program};
pub(crate) use crate::program::{
    module_of_project_type as module_of_type, project_types_of as types_of,
};

/// What the server says of a project about to be opened: its name, catalog type and how it is kept.
pub type Entry = (String, ProjectType, ProjectStorage);

/// What choosing a type means where one is chosen (the upload, Proje bilgileri): the web's words, and in a
/// module's program that it offers its own types alone.
pub(crate) fn type_hint(program: Option<Module>) -> String {
    match program {
        None => plan::TYPE_HINT.to_owned(),
        Some(m) => format!(
            "{} Burada yalnız {} türleri seçilir; başka türdeki projeler kendi programlarında açılır.",
            plan::TYPE_HINT,
            of_program(m)
        ),
    }
}

/// What a module's program says of a cloud project of `owner`'s it does not open.
pub(crate) fn refusal(name: &str, owner: Module) -> String {
    format!(
        "“{name}” {} projesi; onu {} açın.",
        of_program(owner),
        in_program(owner)
    )
}

impl App {
    /// The catalog types this program lists and offers, in its order: a module's own; every one in kentos-cad.
    pub(crate) fn cloud_types(&self) -> Vec<ProjectType> {
        match self.program {
            None => plan::PROJECT_TYPES.to_vec(),
            Some(m) => types_of(m),
        }
    }

    /// Why this program does not open the cloud project `name` of catalog type `t`: the program it belongs to.
    pub(crate) fn cloud_refusal(&self, name: &str, t: ProjectType) -> Option<String> {
        let here = self.program?;
        let owner = module_of_type(t).filter(|m| *m != here)?;
        Some(refusal(name, owner))
    }

    /// Whether this program lists one of this device's copies: one whose drawing is its module's or no module's.
    pub(crate) fn lists_copy(&self, info: &ProjectInfo) -> bool {
        self.program
            .is_none_or(|here| module_of(&info.settings).is_none_or(|m| m == here))
    }

    /// Before a cloud project is opened by its id (the catalog, the application menu's recent projects): in a
    /// module's program one of another module's is refused with the program it belongs to, the drawing on screen
    /// left as it is; one whose type is not known yet is asked of the server first. None: go on (kentos-cad, the
    /// drawing on screen's own project, a type of this program's, no connection to ask on).
    pub(crate) fn cloud_open_check(
        &mut self,
        tenant: Uuid,
        project: Uuid,
    ) -> Option<Task<Message>> {
        self.program?;
        // The drawing on screen is that project: it opened here.
        if self
            .document
            .as_ref()
            .and_then(|d| d.cloud_source())
            .is_some_and(|s| s.project == project)
        {
            return None;
        }
        let known = self
            .cloud
            .open_kind
            .filter(|(p, _)| *p == project)
            .map(|(_, kind)| kind);
        match known {
            Some(Some(kind)) => {
                let name = self
                    .cloud
                    .open_hint
                    .as_ref()
                    .map_or("Bulut projesi", |(n, _)| n.as_str())
                    .to_owned();
                let why = self.cloud_refusal(&name, kind)?;
                self.cloud.open_kind = None;
                self.cloud.open_hint = None;
                self.refuse_cloud_open(why);
                Some(Task::none())
            }
            // Asked, and the server did not say: it opens as it did before (asked again the next time).
            Some(None) => {
                self.cloud.open_kind = None;
                None
            }
            None => {
                let client = self.cloud.signed_in().cloned()?;
                let id = self.cloud.next_id();
                let was = self
                    .document
                    .as_ref()
                    .map(|d| (d.session, d.model.revision()));
                self.cloud.open_check = Some((id, was));
                let asked = client.details(tenant, project);
                Some(Task::perform(asked, move |result| {
                    crate::cloud::msg(Event::OpenChecked {
                        id,
                        tenant,
                        project,
                        result: result
                            .map(|d| (d.project.name, d.project.project_type, d.project.storage)),
                    })
                }))
            }
        }
    }

    /// The server's answer about a project to open by its id: with its type known it opens or is refused; without
    /// an answer it opens as it did before. A later open, or another drawing on screen meanwhile, drops it.
    pub(crate) fn cloud_open_checked(
        &mut self,
        id: u64,
        tenant: Uuid,
        project: Uuid,
        result: Result<Entry, ApiFailure>,
    ) -> Task<Message> {
        let Some((_, was)) = self.cloud.open_check.filter(|(asked, _)| *asked == id) else {
            return Task::none();
        };
        self.cloud.open_check = None;
        let now = self
            .document
            .as_ref()
            .map(|d| (d.session, d.model.revision()));
        let kind = match result {
            Ok((name, kind, storage)) => {
                if self.cloud.open_hint.is_none() {
                    self.cloud.open_hint = Some((name, storage));
                }
                Some(kind)
            }
            Err(_) => None,
        };
        if now != was {
            let name = self
                .cloud
                .open_hint
                .take()
                .map_or_else(|| "Bulut projesi".to_owned(), |(n, _)| n);
            self.warn(format!(
                "“{name}” açılmadı: açılış sürerken ekrandaki çizim değişti ya da başka bir çizim açıldı; o çizim olduğu gibi duruyor. Projeyi yeniden açın."
            ));
            return Task::none();
        }
        self.cloud.open_kind = Some((project, kind));
        self.leave(Then::OpenCloud { tenant, project })
    }

    /// A cloud project not opened here: said in the catalog when it is on screen, else in the log.
    fn refuse_cloud_open(&mut self, why: String) {
        match self.cloud.catalog.as_mut() {
            Some(c) if self.dialog == Some(Dialog::Catalog) => {
                c.status = Some(crate::cloud::catalog::Said::error(why));
            }
            _ => self.error(why),
        }
    }
}
