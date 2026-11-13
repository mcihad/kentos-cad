//! Bir eklentinin Python ortamı (plan/eklenti.md ADIM 18A, P05, P07; docs/adr/8057 §5): in the extension's details,
//! for an extension whose Python contributions were chosen at install. What preparing it would install is read from
//! its lock first; its optional features are ticked; it is made on the user's press with a bar and Durdur (the owner's
//! request of 13 November 2026, for whatever is installed), each outcome a notice; ready, it runs offline; it is
//! removed from the same place, and with the extension. Each extension version has its own environment
//! (kentos-python-env's `extension`): two asking for different versions of a package never share one.

use std::collections::{BTreeMap, BTreeSet};
use std::panic::AssertUnwindSafe;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use iced::Task;
use iced::widget::row;
use kentos_extensions::manifest::OptionalFeature;
use kentos_python_env::extension::{self, Listed, Ready};
use kentos_python_env::python::{InstallError, Interpreter, Progress};
use kentos_python_env::sdk;
use kentos_ui::label;
use kentos_ui::widget::{Banner, Form, Toast, progress};

use super::{Event, msg};
use crate::app::{App, Message};
use crate::exchange::words;
use crate::python_setup::{OwnPython, in_background, progress_words};

/// The program's version: the KentOS library's wheel must be of it.
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// An extension's environment, as looking for it found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum EnvStatus {
    Looking,
    Missing,
    Ready(Box<Ready>),
    /// It could not be read.
    Unknown(String),
}

/// What it is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EnvBusy {
    Previewing,
    Preparing,
    Removing,
}

/// One extension's environment as the program holds it.
#[derive(Debug, Default)]
pub(crate) struct EnvState {
    pub(crate) status: Option<EnvStatus>,
    /// The optional features ticked (their extras).
    pub(crate) extras: BTreeSet<String>,
    /// What preparing would install, once previewed.
    pub(crate) preview: Option<Vec<Listed>>,
    pub(crate) busy: Option<EnvBusy>,
    pub(crate) progress: Option<Progress>,
    /// While preparing: set, it stops.
    pub(crate) stop: Option<Arc<AtomicBool>>,
    /// What went wrong last.
    pub(crate) note: Option<String>,
}

/// What the section's controls ask, each for an extension (its id).
#[derive(Debug, Clone)]
pub(crate) enum EnvEvent {
    Found(String, Result<Option<Ready>, String>),
    /// An optional feature (its extra) ticked or not.
    Extra(String, String, bool),
    Preview(String),
    Previewed(String, Result<Vec<Listed>, String>),
    Dismiss(String),
    Prepare(String),
    Progressed(String, Progress),
    Prepared(String, Result<Box<Ready>, InstallError>),
    Stop(String),
    Remove(String),
    Removed(String, Result<(), String>),
}

fn env_msg(event: EnvEvent) -> Message {
    msg(Event::Env(event))
}

/// What working on an extension's environment needs.
struct Context {
    uv: PathBuf,
    root: PathBuf,
    python: Interpreter,
    package: PathBuf,
    project_file: String,
    lock_file: String,
    version: String,
}

/// What preparing it would install, in a line: what the lock gives on this computer.
pub(crate) fn listed_words(listed: &[Listed]) -> String {
    if listed.is_empty() {
        return "Kurulacak paket yok: eklentinin kilidi bu bilgisayar için bir şey istemiyor."
            .to_owned();
    }
    format!(
        "Kurulacak paketler ({}): {}.",
        listed.len(),
        listed
            .iter()
            .map(|l| format!("{} {}", l.name, l.version))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

/// What a prepare that did not finish means.
pub(crate) fn prepare_words(error: &InstallError) -> String {
    use kentos_python_env::python::Cause;
    let (sentence, said) = match error {
        InstallError::Cancelled => {
            return "Ortam hazırlama durduruldu; yarım kalan ortam silindi.".to_owned();
        }
        InstallError::TimedOut => {
            return "Ortam hazırlama 30 dakikada bitmedi ve durduruldu; yeniden deneyin."
                .to_owned();
        }
        InstallError::NotRun(why) => return format!("uv çalıştırılamadı: {why}."),
        InstallError::Failed { cause, said } => (
            match cause {
                Cause::Network => {
                    "Paketler indirilemedi: internete ulaşılamadı. Bağlantıyı ya da vekil sunucu \
                                   ayarını (HTTPS_PROXY) denetleyin."
                        .to_owned()
                }
                Cause::Certificate => {
                    "Paketler indirilemedi: paket dizininin sertifikası bu bilgisayarda \
                                       doğrulanamadı; kurumun sertifikası yüklü olmalı."
                        .to_owned()
                }
                Cause::Disk => "Ortam hazırlanamadı: diskte yer yok.".to_owned(),
                Cause::Folder => {
                    "Ortam hazırlanamadı: KentOS'un Python klasörüne yazılamadı.".to_owned()
                }
                Cause::Unavailable | Cause::Other => String::new(),
            },
            said,
        ),
    };
    match (sentence.is_empty(), said.is_empty()) {
        (true, _) => said.clone(),
        (false, true) => sentence,
        (false, false) => format!("{sentence} uv: “{said}”"),
    }
}

impl App {
    /// Whether an extension asks for an environment: Python contributions in its manifest, chosen at install.
    fn wants_env(&self, id: &str) -> bool {
        let Some(registry) = self.extensions.registry.as_ref() else {
            return false;
        };
        registry.manifest(id).is_some_and(|m| m.python.is_some())
            && registry.grant(id).is_some_and(|g| g.python)
    }

    /// uv, the Python folder, KentOS's Python and the extension's package, while all are there; else why not.
    fn env_context(&self, id: &str) -> Result<Context, &'static str> {
        let p = &self.python_setup;
        let python = match &p.python {
            Some(OwnPython::Ready(python, _)) => python.clone(),
            _ => {
                return Err(
                    "Önce Uygulama ayarları › Python ve eklentiler'den KentOS'un Python'unu kurun.",
                );
            }
        };
        let (Some(uv), Some(root)) = (p.ready_uv(), p.root.clone()) else {
            return Err(
                "uv hazır değil: Uygulama ayarları › Python ve eklentiler nedenini söylüyor.",
            );
        };
        let registry = self
            .extensions
            .registry
            .as_ref()
            .ok_or("Eklenti kayıt defteri açık değil.")?;
        let manifest = registry.manifest(id).ok_or("Eklenti kurulu değil.")?;
        let project = manifest
            .python
            .as_ref()
            .ok_or("Eklentinin Python işlevi yok.")?;
        let package = registry.package_dir(id).ok_or("Eklentinin paketi yok.")?;
        Ok(Context {
            uv,
            root,
            python,
            package,
            project_file: project.project_file.clone(),
            lock_file: project.lock_file.clone(),
            version: manifest.version.to_string(),
        })
    }

    /// An extension's environment is ready now: files only, for the ribbon's words.
    pub(crate) fn env_ready_now(&self, id: &str) -> bool {
        let Ok(c) = self.env_context(id) else {
            return false;
        };
        extension::lock_digest(&c.package, &c.lock_file)
            .ok()
            .and_then(|lock| extension::find(&c.root, id, &c.version, &lock, &c.python.version))
            .is_some()
    }

    /// Why an extension's Python button does not run now, for that extension: Python off, uv or KentOS's Python not
    /// ready, its own environment not prepared, or the runner still to come.
    pub(crate) fn python_reason(&self, id: &str) -> String {
        // Python itself first: off, looking, being installed or removed.
        let p = &self.python_setup;
        if !matches!(p.python, Some(OwnPython::Ready(..))) || p.installing.is_some() || p.removing {
            return p.why_not().to_owned();
        }
        if self.env_ready_now(id) {
            "Python ortamı hazır; eklentilerin Python işlevlerini çalıştıran yürütücü KentOS'un sonraki sürümünde."
                .to_owned()
        } else {
            format!(
                "Bu eklentinin Python ortamı hazırlanmadı: Eklentiler › {} › Python ortamı.",
                self.extension_name(id)
            )
        }
    }

    /// Looks for an extension's environment in the background.
    pub(crate) fn env_look(&mut self, id: &str) -> Task<Message> {
        if !self.wants_env(id) {
            return Task::none();
        }
        let Ok(c) = self.env_context(id) else {
            self.extensions.envs.remove(id);
            return Task::none();
        };
        let state = self.extensions.envs.entry(id.to_owned()).or_default();
        if state.busy.is_some() {
            return Task::none();
        }
        state.status = Some(EnvStatus::Looking);
        let id = id.to_owned();
        let asked = id.clone();
        in_background(
            move || {
                let lock = extension::lock_digest(&c.package, &c.lock_file)?;
                Ok(extension::find(
                    &c.root,
                    &asked,
                    &c.version,
                    &lock,
                    &c.python.version,
                ))
            },
            Err("arama yarıda kaldı".to_owned()),
            move |found| env_msg(EnvEvent::Found(id.clone(), found)),
        )
    }

    /// Prepares an extension's environment on a thread of its own, saying how far it is as it goes.
    fn env_prepare(&mut self, id: &str) -> Task<Message> {
        let Ok(c) = self.env_context(id) else {
            return Task::none();
        };
        let state = self.extensions.envs.entry(id.to_owned()).or_default();
        if state.busy.is_some() {
            return Task::none();
        }
        let extras: Vec<String> = state.extras.iter().cloned().collect();
        let stop = Arc::new(AtomicBool::new(false));
        state.stop = Some(stop.clone());
        state.busy = Some(EnvBusy::Preparing);
        state.progress = Some(Progress::Starting);
        state.note = None;
        let (out, replies) = iced::futures::channel::mpsc::unbounded();
        let ext = id.to_owned();
        let spawned = std::thread::Builder::new()
            .name("kentos-eklenti-ortami".into())
            .spawn(move || {
                let told = out.clone();
                let told_id = ext.clone();
                let ended = std::panic::catch_unwind(AssertUnwindSafe(|| {
                    let key = extension::key_for(
                        &c.package,
                        &c.lock_file,
                        &ext,
                        &c.version,
                        &c.python,
                        &extras,
                    )
                    .map_err(InstallError::NotRun)?;
                    let wheel = sdk::find(&sdk::dirs(), VERSION).map_err(InstallError::NotRun)?;
                    let made = extension::prepare(
                        &c.uv,
                        &c.root,
                        &c.python,
                        &wheel,
                        &c.package,
                        (&c.project_file, &c.lock_file),
                        &key,
                        &stop,
                        &mut |at| {
                            let _ = told.unbounded_send(EnvEvent::Progressed(told_id.clone(), at));
                        },
                    )?;
                    // Another set of features made before: this one takes its place.
                    extension::clean_for(&c.root, &ext, Some(&key));
                    Ok(Box::new(made))
                }))
                .unwrap_or_else(|_| {
                    Err(InstallError::NotRun(
                        "hazırlama beklenmedik biçimde durdu".to_owned(),
                    ))
                });
                let _ = out.unbounded_send(EnvEvent::Prepared(ext, ended));
            });
        match spawned {
            Ok(_) => Task::run(replies, env_msg),
            Err(e) => Task::done(env_msg(EnvEvent::Prepared(
                id.to_owned(),
                Err(InstallError::NotRun(e.to_string())),
            ))),
        }
    }

    pub(crate) fn env_event(&mut self, event: EnvEvent) -> Task<Message> {
        match event {
            EnvEvent::Found(id, found) => {
                let state = self.extensions.envs.entry(id).or_default();
                state.status = Some(match found {
                    Ok(Some(ready)) => {
                        // The features it was made with are the user's choice.
                        state.extras = ready.key.extras.iter().cloned().collect();
                        EnvStatus::Ready(Box::new(ready))
                    }
                    Ok(None) => EnvStatus::Missing,
                    Err(why) => EnvStatus::Unknown(why),
                });
            }
            EnvEvent::Extra(id, extra, on) => {
                let state = self.extensions.envs.entry(id).or_default();
                if on {
                    state.extras.insert(extra);
                } else {
                    state.extras.remove(&extra);
                }
                // What it would install changes with them.
                state.preview = None;
            }
            EnvEvent::Preview(id) => {
                let Ok(c) = self.env_context(&id) else {
                    return Task::none();
                };
                let state = self.extensions.envs.entry(id.clone()).or_default();
                if state.busy.is_some() {
                    return Task::none();
                }
                state.busy = Some(EnvBusy::Previewing);
                state.note = None;
                let extras: Vec<String> = state.extras.iter().cloned().collect();
                let asked = id.clone();
                return in_background(
                    move || {
                        let key = extension::key_for(
                            &c.package,
                            &c.lock_file,
                            &asked,
                            &c.version,
                            &c.python,
                            &extras,
                        )?;
                        extension::preview(
                            &c.uv,
                            &c.root,
                            &c.python,
                            &c.package,
                            (&c.project_file, &c.lock_file),
                            &key,
                        )
                    },
                    Err("önizleme yarıda kaldı".to_owned()),
                    move |seen| env_msg(EnvEvent::Previewed(id.clone(), seen)),
                );
            }
            EnvEvent::Previewed(id, seen) => {
                let state = self.extensions.envs.entry(id).or_default();
                state.busy = None;
                match seen {
                    Ok(listed) => state.preview = Some(listed),
                    Err(why) => state.note = Some(why),
                }
            }
            EnvEvent::Dismiss(id) => {
                if let Some(state) = self.extensions.envs.get_mut(&id) {
                    state.preview = None;
                }
            }
            EnvEvent::Prepare(id) => return self.env_prepare(&id),
            EnvEvent::Progressed(id, at) => {
                if let Some(state) = self.extensions.envs.get_mut(&id)
                    && state.busy == Some(EnvBusy::Preparing)
                {
                    state.progress = Some(at);
                }
            }
            EnvEvent::Prepared(id, ended) => {
                let name = self.extension_name(&id);
                let state = self.extensions.envs.entry(id.clone()).or_default();
                state.busy = None;
                state.stop = None;
                state.progress = None;
                state.preview = None;
                match ended {
                    Ok(ready) => {
                        let count = ready.packages.len();
                        state.status = Some(EnvStatus::Ready(ready));
                        self.toast(
                            Toast::success(format!("{name}: Python ortamı hazır"))
                                .body(format!("{count} paket; internetsiz çalışır.")),
                        );
                    }
                    Err(InstallError::Cancelled) => {
                        self.toast(
                            Toast::info(format!("{name}: ortam hazırlama durduruldu"))
                                .body("Yarım kalan ortam silindi."),
                        );
                    }
                    Err(error) => {
                        let words = prepare_words(&error);
                        state.note = Some(words.clone());
                        self.toast(
                            Toast::error(format!("{name}: Python ortamı hazırlanamadı"))
                                .body(words),
                        );
                    }
                }
                self.extensions_changed();
                return self.env_look(&id);
            }
            EnvEvent::Stop(id) => {
                if let Some(stop) = self.extensions.envs.get(&id).and_then(|s| s.stop.as_ref()) {
                    stop.store(true, Ordering::Relaxed);
                }
            }
            EnvEvent::Remove(id) => {
                let Some(root) = self.python_setup.root.clone() else {
                    return Task::none();
                };
                let state = self.extensions.envs.entry(id.clone()).or_default();
                if state.busy.is_some() {
                    return Task::none();
                }
                state.busy = Some(EnvBusy::Removing);
                let asked = id.clone();
                return in_background(
                    move || {
                        extension::clean_for(&root, &asked, None);
                        Ok(())
                    },
                    Err("kaldırma yarıda kaldı".to_owned()),
                    move |ended| env_msg(EnvEvent::Removed(id.clone(), ended)),
                );
            }
            EnvEvent::Removed(id, ended) => {
                let name = self.extension_name(&id);
                if let Some(state) = self.extensions.envs.get_mut(&id) {
                    state.busy = None;
                }
                match ended {
                    Ok(()) => self.toast(
                        Toast::success(format!("{name}: Python ortamı kaldırıldı"))
                            .body("Gerektiğinde yeniden hazırlanır."),
                    ),
                    Err(why) => {
                        self.toast(Toast::error(format!("{name}: ortam kaldırılamadı")).body(why))
                    }
                }
                self.extensions_changed();
                return self.env_look(&id);
            }
        }
        Task::none()
    }

    /// The details' Python ortamı section, for an extension with Python contributions.
    pub(crate) fn env_section<'a>(
        &'a self,
        mut form: Form<'a, Message>,
        id: &str,
    ) -> Form<'a, Message> {
        let Some(registry) = self.extensions.registry.as_ref() else {
            return form;
        };
        let Some(project) = registry.manifest(id).and_then(|m| m.python.as_ref()) else {
            return form;
        };
        form = form.section("Python ortamı");
        if !registry.grant(id).is_some_and(|g| g.python) {
            return form.wide(label::caption(
                "Python işlevleri kurulumda seçilmedi; bu eklenti için ortam hazırlanmaz.",
            ));
        }
        if let Err(why) = self.env_context(id) {
            return form.wide(Banner::warning(why));
        }
        let state = self.extensions.envs.get(id);
        let busy = state.and_then(|s| s.busy);
        let idle = busy.is_none();
        let ask = |e: EnvEvent| Some(env_msg(e));
        let features: &BTreeMap<String, OptionalFeature> = &project.optional_features;
        let ticked = |extra: &str| state.is_some_and(|s| s.extras.contains(extra));
        let status = state.and_then(|s| s.status.as_ref());
        if busy == Some(EnvBusy::Preparing) {
            let at = state.and_then(|s| s.progress).unwrap_or(Progress::Starting);
            return form
                .wide(Banner::info(
                    "Ortam hazırlanıyor: eklentinin kilidindeki paketler kuruluyor.",
                ))
                .wide(label::caption(progress_words(at)))
                .wide(progress::bar(at.share()).height(6.0))
                .wide(words::secondary(
                    "Durdur",
                    ask(EnvEvent::Stop(id.to_owned())),
                ));
        }
        match status {
            None | Some(EnvStatus::Looking) => {
                form = form.wide(label::caption("Ortam aranıyor…"));
            }
            Some(EnvStatus::Unknown(why)) => {
                form = form.wide(Banner::error(format!("Ortam okunamadı: {why}")));
            }
            Some(EnvStatus::Ready(ready)) => {
                form = form.wide(Banner::info(format!(
                    "Ortam hazır · Python {} · {} paket · internetsiz çalışır.",
                    ready.key.python,
                    ready.packages.len()
                )));
                if !ready.packages.is_empty() {
                    form = form.wide(label::caption(
                        ready
                            .packages
                            .iter()
                            .map(|p| format!("{} {}", p.name, p.version))
                            .collect::<Vec<_>>()
                            .join(", "),
                    ));
                }
                form = form.field(
                    "Klasör",
                    label::caption(
                        ready
                            .python
                            .parent()
                            .and_then(|bin| bin.parent())
                            .map(|venv| venv.display().to_string())
                            .unwrap_or_default(),
                    ),
                );
            }
            Some(EnvStatus::Missing) => {
                form = form.wide(Banner::warning(
                    "Ortam hazır değil: eklentinin Python işlevleri hazırlanınca çalışır. Hazırlamadan önce neyin \
                     kurulacağını görürsünüz.",
                ));
            }
        }
        // Its optional features: part of what the environment holds.
        for feature in features.values() {
            let extra = feature.extra.clone();
            let on = ticked(&extra);
            form = form.wide(words::check(
                on,
                format!("{} (isteğe bağlı)", feature.label),
                idle.then(|| env_msg(EnvEvent::Extra(id.to_owned(), extra.clone(), !on))),
            ));
        }
        let made_with = match status {
            Some(EnvStatus::Ready(ready)) => Some(ready.key.extras.clone()),
            _ => None,
        };
        let chosen: Vec<String> = state
            .map(|s| s.extras.iter().cloned().collect())
            .unwrap_or_default();
        let current = made_with.as_ref() == Some(&chosen);
        if let Some(listed) = state.and_then(|s| s.preview.as_ref()) {
            form = form.wide(Banner::info(listed_words(listed))).wide(
                row![
                    words::primary(
                        "Hazırla",
                        idle.then(|| env_msg(EnvEvent::Prepare(id.to_owned())))
                    ),
                    words::secondary("Vazgeç", ask(EnvEvent::Dismiss(id.to_owned()))),
                ]
                .spacing(8),
            );
        } else if !current {
            let caption = if made_with.is_some() {
                "Seçilen özelliklerle yeniden hazırla"
            } else {
                "Ortamı hazırla"
            };
            form = form.wide(
                row![
                    words::primary(
                        if busy == Some(EnvBusy::Previewing) {
                            "Önizleniyor…"
                        } else {
                            caption
                        },
                        idle.then(|| env_msg(EnvEvent::Preview(id.to_owned()))),
                    ),
                    label::caption(
                        "Önce kurulacak paketler gösterilir; onaylamadan bir şey indirilmez."
                    ),
                ]
                .spacing(10)
                .align_y(iced::Center),
            );
        }
        if made_with.is_some() {
            form = form.wide(words::secondary(
                if busy == Some(EnvBusy::Removing) {
                    "Kaldırılıyor…"
                } else {
                    "Ortamı kaldır"
                },
                idle.then(|| env_msg(EnvEvent::Remove(id.to_owned()))),
            ));
        }
        if let Some(note) = state.and_then(|s| s.note.as_ref()) {
            form = form.wide(Banner::error(note.clone()));
        }
        form
    }
}
