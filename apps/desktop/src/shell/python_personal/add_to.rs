//! Eklentiye ekle (plan/eklenti.md ADIM 18A, the last step of PY-OPT-06's example: "betik/eklenti bağımlılığına ekle";
//! docs/adr/8057 §95): a package the user asked for in the personal environment and tried in the console, added to the
//! Python project of an extension they develop and its lock renewed, the extension loaded again with it.
//!
//! - **Which extensions:** the installed ones whose source is a working folder (Asistan's workshop, an agent's on the
//!   link) and which have a Python project. A packed `.kek` or a catalog's version is not edited in place.
//! - **What is written:** the version tried here and its series (`numpy>=2.5.4,<3`), by uv with the project's own
//!   settings (its cooling period too); the project's files change only when the lock resolved
//!   (`kentos_python_env::extension::add_dependency`). One already among its dependencies is said, not changed.
//! - **Then:** the extension is loaded again from its folder with the capabilities the user granted, so the installed
//!   one and its folder agree; its environment is made from the new lock as any other, after its preview.

use std::collections::BTreeMap;
use std::path::PathBuf;

use iced::widget::row;
use iced::{Center, Element, Task};
use kentos_extension_host::SourceKind;
use kentos_extensions::manifest::PythonProject;
use kentos_python_env::{extension, personal};
use kentos_ui::label;
use kentos_ui::widget::{Choice, Select, Toast, progress};

use super::{Busy, PersonalEdit, msg};
use crate::app::{App, Message};
use crate::exchange::words;
use crate::python_setup::in_background;

/// The chooser open under a package's row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AddTo {
    pub(crate) package: String,
    pub(crate) version: String,
    /// Its place in [`App::developed_extensions`].
    pub(crate) choice: usize,
}

/// An extension being developed: installed from a working folder, with a Python project.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Developed {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) folder: PathBuf,
    pub(crate) python: PythonProject,
}

impl Developed {
    /// The folder of its `pyproject.toml`.
    fn project(&self) -> PathBuf {
        let file = self.folder.join(&self.python.project_file);
        file.parent()
            .map_or_else(|| self.folder.clone(), PathBuf::from)
    }

    /// Its lock's packages and their versions, read from its folder; empty when they do not read.
    fn locked(&self) -> BTreeMap<String, String> {
        let mut files = BTreeMap::new();
        for path in [&self.python.project_file, &self.python.lock_file] {
            if let Ok(bytes) = std::fs::read(self.folder.join(path)) {
                files.insert(path.clone(), bytes);
            }
        }
        kentos_extensions::release::locked_packages(&self.python, &files)
            .unwrap_or_default()
            .into_iter()
            .map(|(name, p)| (name, p.version))
            .collect()
    }
}

/// What adding did, for the notice.
#[derive(Clone, Debug)]
pub(crate) struct Added {
    id: String,
    name: String,
    folder: PathBuf,
    requirement: String,
    /// The packages the lock now installs that it did not, with their versions.
    new: Vec<String>,
}

impl App {
    /// The extensions a package can be added to, by name.
    pub(crate) fn developed_extensions(&self) -> Vec<Developed> {
        let Some(registry) = self.extensions.registry.as_ref() else {
            return Vec::new();
        };
        let mut out: Vec<Developed> = registry
            .extensions()
            .into_iter()
            .filter(|v| v.source.kind == SourceKind::DeveloperFolder)
            .filter_map(|v| {
                let python = registry.manifest(&v.id)?.python.clone()?;
                Some(Developed {
                    name: self.extension_name(&v.id),
                    folder: PathBuf::from(&v.source.location),
                    id: v.id,
                    python,
                })
            })
            .collect();
        out.sort_by(|a, b| a.name.cmp(&b.name));
        out
    }

    /// Under the package's row while its chooser is open: Eklenti [the extensions] Ekle Vazgeç, and what is written.
    pub(crate) fn add_to_part(&self, add: &AddTo) -> Element<'_, Message> {
        let list = self.developed_extensions();
        let requirement = extension::requirement_for(&add.package, &add.version);
        let mut part = iced::widget::Column::new().spacing(6);
        if self.python_setup.personal.busy == Some(Busy::Adding) {
            return part
                .push(label::caption(format!(
                    "“{requirement}” ekleniyor ve kilit yenileniyor (uv)…"
                )))
                .push(progress::bar(None).height(6.0))
                .into();
        }
        let select = Select::new(
            list.iter().map(|d| Choice::new(d.name.clone())),
            Some(add.choice.min(list.len().saturating_sub(1))),
            |i| msg(PersonalEdit::AddToPick(i)),
        );
        part = part
            .push(
                row![
                    label::body("Eklenti"),
                    select,
                    words::primary("Ekle", Some(msg(PersonalEdit::AddToConfirm))),
                    words::ghost("Vazgeç", Some(msg(PersonalEdit::AddToCancel))),
                ]
                .spacing(10)
                .align_y(Center),
            )
            .push(label::caption(format!(
                "Eklentinin pyproject.toml dosyasına “{requirement}” yazılır ve kilidi yenilenir (uv add; eklentinin \
                 kendi ayarlarıyla). Kilit çözülmezse eklentinin dosyaları değişmez. Sonra eklenti klasöründen yeniden \
                 yüklenir; Python ortamı yeni kilitle hazırlanır."
            )));
        part.into()
    }

    /// Ekle: the requirement written to the chosen extension's project and locked, in the background; one already among
    /// its dependencies is said and nothing is run.
    pub(crate) fn add_to_start(&mut self) -> Task<Message> {
        let Some((uv, root, python)) = self.personal_ready() else {
            return Task::none();
        };
        let me = &self.python_setup.personal;
        let (Some(add), None) = (me.add_to.clone(), me.busy) else {
            return Task::none();
        };
        let Some(ext) = self.developed_extensions().into_iter().nth(add.choice) else {
            return Task::none();
        };
        let project = ext.project();
        let declared = std::fs::read_to_string(ext.folder.join(&ext.python.project_file))
            .map(|text| personal::direct_names(&text))
            .unwrap_or_default();
        if declared.contains(&personal::normalize(&add.package)) {
            let locked = ext
                .locked()
                .get(&personal::normalize(&add.package))
                .map(|v| format!(" (kilitte {v})"))
                .unwrap_or_default();
            let me = &mut self.python_setup.personal;
            me.add_to = None;
            me.note = Some(format!(
                "“{}” zaten “{}” eklentisinin bağımlılığı{locked}; değiştirilmedi.",
                add.package, ext.name
            ));
            return Task::none();
        }
        let requirement = extension::requirement_for(&add.package, &add.version);
        let me = &mut self.python_setup.personal;
        me.busy = Some(Busy::Adding);
        me.note = None;
        in_background(
            move || {
                let before = ext.locked();
                extension::add_dependency(&uv, &root, &python, &project, &requirement)?;
                let new = ext
                    .locked()
                    .into_iter()
                    .filter(|(name, _)| !before.contains_key(name))
                    .map(|(name, version)| format!("{name} {version}"))
                    .collect();
                Ok(Added {
                    id: ext.id,
                    name: ext.name,
                    folder: ext.folder,
                    requirement,
                    new,
                })
            },
            Err("ekleme yarıda kaldı".to_owned()),
            |ended| msg(PersonalEdit::AddedTo(ended)),
        )
    }

    /// What adding did: on success the extension loaded again from its folder and a notice; else why, in the part.
    pub(crate) fn add_to_done(&mut self, ended: Result<Added, String>) {
        let me = &mut self.python_setup.personal;
        me.busy = None;
        let added = match ended {
            Ok(added) => added,
            Err(why) => {
                me.note = Some(why);
                return;
            }
        };
        me.add_to = None;
        let locked = if added.new.is_empty() {
            "Kilit yenilendi.".to_owned()
        } else {
            format!("Kilit yenilendi; yeni: {}.", added.new.join(", "))
        };
        let reloaded = match self.ext_reload_folder(&added.id, &added.folder) {
            Ok(_) => "Eklenti klasöründen yeniden yüklendi; Python ortamını Eklentiler'den yeni kilitle hazırlayın."
                .to_owned(),
            Err(why) => format!("Eklenti yeniden yüklenemedi: {why}"),
        };
        self.extension_worked(
            &added.id,
            "python",
            "dependency_added",
            &format!(
                "Kişisel ortamdan bağımlılık eklendi: {}. {locked}",
                added.requirement
            ),
        );
        self.toast(
            Toast::success(format!("{}: bağımlılık eklendi", added.name)).body(format!(
                "pyproject.toml'a “{}” yazıldı. {locked} {reloaded}",
                added.requirement
            )),
        );
    }
}
