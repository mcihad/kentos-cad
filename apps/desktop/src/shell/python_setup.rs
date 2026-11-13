//! Python ve eklentiler (plan/eklenti.md ADIM 18A; docs/adr/8057 §5): the user's choice to use Python and Python
//! extensions, off by default, uv found for it, and KentOS's own Python installed when the user asks.
//!
//! - **Off** (P01, P02): nothing looks for uv or Python and nothing is downloaded; CAD, GIS and KentOS's own tools work;
//!   an extension's Python button stays shown, not pressable, and says why.
//! - **On:** uv is looked for in the background (the window never waits for it): the chosen path; else the uv KentOS's
//!   installation carries (the owner's decision, 13 November 2026), `PATH`, the user folders a session started from
//!   the menu lacks; its version is checked against the compatibility manifest (kentos-python-env). Not found (the
//!   installation's own removed) or too old, the window says so and shows uv's official way in for the user to run
//!   themselves: the running program installs nothing on its own (PY-UV-03…06). A path can be given by hand;
//!   Yeniden algıla looks again.
//! - **KentOS's Python:** with uv ready, KentOS's own Python is looked for in the user's KentOS folder. None there, the
//!   window says what installing it downloads, from where, how big, where it goes and what it leaves alone, and installs
//!   it only when the user presses the button: that press is the consent (P03). It runs in the background with a bar
//!   that shows the share downloaded (uv's own size and the growing archive; unpacking has no measure and sweeps), can
//!   be stopped (Durdur) and says why when it fails (PY-UV-07); the program keeps working meanwhile.
//! - **Removed from the same place** (the owner, 13 November 2026: "Kaldırılabilmelidir"): Kaldır asks first, with the
//!   room it frees, then uv uninstalls it and its folder goes.
//! - **Toasts** (the owner: "toast ile geri bildirim"): installed, stopped, failed, removed: each a notice in the
//!   window's corner, so the outcome is seen even with the settings closed; a failure also stays in the section.
//!
//! The choice is the user's own (`<veri>/kentos-cad/python/ayarlar.json`), written at once, apart from the settings'
//! draft, as Yapay zekâ's agents are.

use std::panic::AssertUnwindSafe;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use iced::widget::{Column, button, row};
use iced::{Element, Task};
use kentos_python_env::compat;
use kentos_python_env::python::{InstallError, Interpreter, Progress};
use kentos_python_env::settings::PythonSettings;
use kentos_python_env::uv::UvStatus;
use kentos_ui::widget::Toast;
use kentos_ui::widget::progress;
use kentos_ui::{label, style};

use crate::app::{App, Message};
use crate::exchange::words::{self, Kind as Line};
use crate::settings_view::Edit;

/// KentOS's own Python, as looking for it found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OwnPython {
    Looking,
    /// None installed in KentOS's folder.
    Missing,
    /// Installed, and its folder's bytes.
    Ready(Interpreter, u64),
    /// uv could not say.
    Unknown(String),
}

/// The user's Python as the program holds it.
#[derive(Debug, Default)]
pub(crate) struct PythonSetup {
    /// The user's Python folder; none on a computer without a data folder.
    pub(crate) root: Option<PathBuf>,
    pub(crate) settings: PythonSettings,
    /// What looking for uv found; none until it is looked for, and never while off.
    pub(crate) uv: Option<UvStatus>,
    /// Looking now.
    pub(crate) looking: bool,
    /// KentOS's own Python; none until uv is ready.
    pub(crate) python: Option<OwnPython>,
    /// While installing: set, it stops the install.
    pub(crate) installing: Option<Arc<AtomicBool>>,
    /// How far installing is.
    pub(crate) progress: Option<Progress>,
    /// Kaldır pressed: asking before removing.
    pub(crate) confirm_remove: bool,
    /// Removing now.
    pub(crate) removing: bool,
    /// The uv path typed, before Uygula.
    pub(crate) uv_typed: String,
    /// The last action's outcome and its words.
    pub(crate) note: Option<(Line, String)>,
    /// The personal environment (python_personal.rs), while KentOS's Python is ready.
    pub(crate) personal: crate::python_personal::PersonalSetup,
}

impl PythonSetup {
    /// The device's: the user's folder and choice (`KENTOS_PYTHON_DIR` points elsewhere).
    pub(crate) fn device() -> PythonSetup {
        let root = kentos_python_env::places::root();
        let settings = root
            .as_deref()
            .map(PythonSettings::load)
            .unwrap_or_default();
        PythonSetup {
            uv_typed: settings.uv_path.clone().unwrap_or_default(),
            root,
            settings,
            ..PythonSetup::default()
        }
    }

    /// uv's path while it is ready to use.
    pub(crate) fn ready_uv(&self) -> Option<PathBuf> {
        match &self.uv {
            Some(UvStatus::Found { path, .. }) if self.settings.enabled => Some(path.clone()),
            _ => None,
        }
    }

    /// Why an extension's Python button does not run now, in a sentence.
    pub(crate) fn why_not(&self) -> &'static str {
        if !self.settings.enabled {
            return "Python kapalı: Uygulama ayarları › Python ve eklentiler'den açabilirsiniz.";
        }
        match &self.uv {
            None => "Python açık; uv aranıyor.",
            Some(status) if !status.ok() => {
                "uv hazır değil: Uygulama ayarları › Python ve eklentiler nedenini söylüyor."
            }
            Some(_) if self.installing.is_some() => "KentOS'un Python'u kuruluyor.",
            Some(_) if self.removing => "KentOS'un Python'u kaldırılıyor.",
            Some(_) => match &self.python {
                None | Some(OwnPython::Looking) => "Python açık; KentOS'un Python'u aranıyor.",
                Some(OwnPython::Missing | OwnPython::Unknown(_)) => {
                    "KentOS'un Python'u kurulu değil: Uygulama ayarları › Python ve eklentiler'den kurabilirsiniz."
                }
                Some(OwnPython::Ready(..)) => {
                    "Python hazır; eklentilerin Python işlevlerini çalıştıran yürütücü KentOS'un sonraki sürümünde."
                }
            },
        }
    }
}

/// What the section's controls ask.
#[derive(Debug, Clone)]
pub enum PythonEdit {
    Enable(bool),
    /// Yeniden algıla.
    Detect,
    /// What the background look found.
    Detected(UvStatus),
    UvTyped(String),
    /// Uygula: the typed path (empty: found on its own).
    UvApply,
    /// One of uv's official ways in onto the clipboard, by its place in the manifest's list for this system.
    CopyInstall(usize),
    /// What looking for KentOS's own Python found, and its folder's bytes.
    Found(Result<(Option<Interpreter>, u64), String>),
    /// The user's consent: install KentOS's Python.
    Install,
    /// How far installing is.
    Progressed(Progress),
    /// How installing ended.
    Installed(Result<(), InstallError>),
    /// Durdur.
    StopInstall,
    /// Kaldır: asks first.
    Remove,
    /// Evet, kaldır.
    RemoveConfirmed,
    /// Vazgeç.
    RemoveCancelled,
    /// How removing ended: the bytes freed.
    Removed(Result<u64, String>),
    /// The personal environment's part (python_personal.rs).
    Personal(crate::python_personal::PersonalEdit),
}

/// A size as the window says it: `98 MB`, `32,6 MB`, `512 KB`.
pub(crate) fn size_words(bytes: u64) -> String {
    let mb = bytes as f64 / (1024.0 * 1024.0);
    if mb >= 10.0 {
        format!("{} MB", mb.round() as u64)
    } else if mb >= 0.1 {
        format!("{mb:.1} MB").replace('.', ",")
    } else {
        format!("{} KB", (bytes as f64 / 1024.0).ceil() as u64)
    }
}

/// A size to a tenth, for a download under way: `12,4 MB`.
fn size_tenths(bytes: u64) -> String {
    format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0)).replace('.', ",")
}

/// How far installing is, as the bar's caption says it.
pub(crate) fn progress_words(progress: Progress) -> String {
    match progress {
        Progress::Starting => "Hazırlanıyor…".to_owned(),
        Progress::Downloading {
            done,
            total: Some(total),
        } => format!(
            "İndiriliyor: {} / {} · %{}",
            size_tenths(done.min(total)),
            size_tenths(total),
            (progress.share().unwrap_or(0.0) * 100.0).floor() as u32
        ),
        Progress::Downloading { done, total: None } => {
            format!("İndiriliyor: {}", size_tenths(done))
        }
        Progress::Unpacking => "Açılıyor ve yerine konuyor…".to_owned(),
    }
}

fn msg(edit: PythonEdit) -> Message {
    Message::Settings(Edit::Python(edit))
}

/// `work` on a thread of its own, its answer as a message: the window never waits for uv.
pub(crate) fn in_background<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
    lost: T,
    then: impl Fn(T) -> Message + Send + 'static,
) -> Task<Message> {
    Task::perform(
        async move {
            let (sent, got) = iced::futures::channel::oneshot::channel();
            std::thread::spawn(move || {
                let _ = sent.send(work());
            });
            got.await.unwrap_or(lost)
        },
        then,
    )
}

impl App {
    /// Looks for uv in the background, while Python is on; nothing while off.
    pub(crate) fn python_detect(&mut self) -> Task<Message> {
        let p = &mut self.python_setup;
        if !p.settings.enabled {
            return Task::none();
        }
        p.looking = true;
        let chosen = p.settings.uv_path.clone().map(PathBuf::from);
        in_background(
            move || kentos_python_env::uv::detect(chosen.as_deref()),
            UvStatus::NotFound {
                looked: Vec::new(),
                own: false,
            },
            |status| msg(PythonEdit::Detected(status)),
        )
    }

    /// Looks for KentOS's own Python in the background, while uv is ready.
    fn python_look(&mut self) -> Task<Message> {
        let p = &mut self.python_setup;
        let (Some(uv), Some(root)) = (p.ready_uv(), p.root.clone()) else {
            p.python = None;
            return Task::none();
        };
        p.python = Some(OwnPython::Looking);
        let range = compat::compat().python.range.clone();
        in_background(
            move || {
                let found = kentos_python_env::python::installed(&uv, &root, &range)?;
                let size = kentos_python_env::python::folder_size(
                    &kentos_python_env::places::interpreters(&root),
                );
                Ok((found, size))
            },
            Err("arama yarıda kaldı".to_owned()),
            |found| msg(PythonEdit::Found(found)),
        )
    }

    /// Installs KentOS's Python on a thread of its own, saying how far it is as it goes.
    fn python_install(&mut self) -> Task<Message> {
        let p = &mut self.python_setup;
        let (Some(uv), Some(root)) = (p.ready_uv(), p.root.clone()) else {
            return Task::none();
        };
        if p.installing.is_some() || p.removing {
            return Task::none();
        }
        let stop = Arc::new(AtomicBool::new(false));
        p.installing = Some(stop.clone());
        p.progress = Some(Progress::Starting);
        p.note = None;
        p.confirm_remove = false;
        self.extensions_changed();
        let version = compat::compat().python.install.clone();
        let (out, replies) = iced::futures::channel::mpsc::unbounded();
        let spawned = std::thread::Builder::new()
            .name("kentos-python-kur".into())
            .spawn(move || {
                let told = out.clone();
                let ended = std::panic::catch_unwind(AssertUnwindSafe(|| {
                    kentos_python_env::python::install(&uv, &root, &version, &stop, &mut |at| {
                        let _ = told.unbounded_send(PythonEdit::Progressed(at));
                    })
                }))
                .unwrap_or_else(|_| {
                    Err(InstallError::NotRun(
                        "kurulum beklenmedik biçimde durdu".to_owned(),
                    ))
                });
                let _ = out.unbounded_send(PythonEdit::Installed(ended));
            });
        match spawned {
            Ok(_) => Task::run(replies, msg),
            Err(e) => Task::done(msg(PythonEdit::Installed(Err(InstallError::NotRun(
                e.to_string(),
            ))))),
        }
    }

    fn python_save(&mut self) {
        let p = &mut self.python_setup;
        let saved = match &p.root {
            Some(root) => p.settings.save(root),
            None => Err(
                "Bu bilgisayarda kullanıcı veri klasörü yok; seçim bu oturumla sınırlı.".to_owned(),
            ),
        };
        if let Err(why) = saved {
            p.note = Some((Line::Error, why));
        }
    }

    pub(crate) fn python_edit(&mut self, edit: PythonEdit) -> Task<Message> {
        match edit {
            PythonEdit::Enable(on) => {
                self.python_setup.settings.enabled = on;
                self.python_setup.note = None;
                self.python_save();
                if !on {
                    let p = &mut self.python_setup;
                    p.uv = None;
                    p.looking = false;
                    p.python = None;
                    p.personal.env = None;
                    p.personal.preview = None;
                    // Turned off while installing: the install stops; so does a change of packages.
                    if let Some(stop) = &p.installing {
                        stop.store(true, Ordering::Relaxed);
                    }
                    if let Some(stop) = &p.personal.stop {
                        stop.store(true, Ordering::Relaxed);
                    }
                }
                self.extensions_changed();
                return self.python_detect();
            }
            PythonEdit::Detect => return self.python_detect(),
            PythonEdit::Detected(status) => {
                let p = &mut self.python_setup;
                p.looking = false;
                // Turned off while looking: what was found is not kept.
                if p.settings.enabled {
                    p.uv = Some(status);
                }
                self.extensions_changed();
                return self.python_look();
            }
            PythonEdit::UvTyped(text) => self.python_setup.uv_typed = text,
            PythonEdit::UvApply => {
                let typed = self.python_setup.uv_typed.trim().to_owned();
                self.python_setup.settings.uv_path = (!typed.is_empty()).then_some(typed);
                self.python_save();
                return self.python_detect();
            }
            PythonEdit::CopyInstall(n) => {
                let Some(command) = compat::compat().install_commands().get(n).cloned() else {
                    return Task::none();
                };
                self.python_setup.note = Some((
                    Line::Ok,
                    "Kurulum komutu panoya kopyalandı; bir uçbirimde kendiniz çalıştırın, sonra Yeniden algıla."
                        .to_owned(),
                ));
                return iced::clipboard::write(command);
            }
            PythonEdit::Found(found) => {
                let p = &mut self.python_setup;
                // uv turned off or changed while looking: what was found is not kept.
                if p.ready_uv().is_some() {
                    p.python = Some(match found {
                        Ok((Some(python), size)) => OwnPython::Ready(python, size),
                        Ok((None, _)) => OwnPython::Missing,
                        Err(why) => OwnPython::Unknown(why),
                    });
                }
                self.extensions_changed();
                // With KentOS's Python ready, the personal environment on it; without it, none.
                return self.personal_look();
            }
            PythonEdit::Personal(edit) => return self.personal_edit(edit),
            PythonEdit::Install => return self.python_install(),
            PythonEdit::Progressed(at) => {
                if self.python_setup.installing.is_some() {
                    self.python_setup.progress = Some(at);
                }
            }
            PythonEdit::Installed(ended) => {
                let p = &mut self.python_setup;
                p.installing = None;
                p.progress = None;
                let version = compat::compat().python.install.clone();
                match ended {
                    Ok(()) => {
                        p.note = None;
                        self.toast(
                            Toast::success(format!("Python {version} kuruldu")).body(
                                "KentOS'un kendi klasöründe; sistemdeki Python'a dokunulmadı.",
                            ),
                        );
                    }
                    Err(InstallError::Cancelled) => {
                        p.note = None;
                        self.toast(
                            Toast::info("Python kurulumu durduruldu")
                                .body("İstediğinizde yeniden başlatabilirsiniz."),
                        );
                    }
                    Err(error) => {
                        let words = error.words(&version);
                        p.note = Some((Line::Error, words.clone()));
                        self.toast(Toast::error("Python kurulamadı").body(words));
                    }
                }
                self.extensions_changed();
                return self.python_look();
            }
            PythonEdit::StopInstall => {
                if let Some(stop) = &self.python_setup.installing {
                    stop.store(true, Ordering::Relaxed);
                }
            }
            PythonEdit::Remove => {
                let p = &mut self.python_setup;
                if p.installing.is_none() && !p.removing {
                    p.confirm_remove = true;
                }
            }
            PythonEdit::RemoveCancelled => self.python_setup.confirm_remove = false,
            PythonEdit::RemoveConfirmed => {
                let p = &mut self.python_setup;
                p.confirm_remove = false;
                let Some(root) = p.root.clone() else {
                    return Task::none();
                };
                if p.installing.is_some() || p.removing {
                    return Task::none();
                }
                p.removing = true;
                p.note = None;
                let uv = p.ready_uv();
                self.extensions_changed();
                return in_background(
                    move || kentos_python_env::python::remove(uv.as_deref(), &root),
                    Err("kaldırma yarıda kaldı".to_owned()),
                    |ended| msg(PythonEdit::Removed(ended)),
                );
            }
            PythonEdit::Removed(ended) => {
                self.python_setup.removing = false;
                match ended {
                    Ok(freed) => self.toast(
                        Toast::success("Python kaldırıldı")
                            .body(format!("{} yer açıldı.", size_words(freed))),
                    ),
                    Err(why) => {
                        self.python_setup.note = Some((Line::Error, why.clone()));
                        self.toast(Toast::error("Python kaldırılamadı").body(why));
                    }
                }
                self.extensions_changed();
                return self.python_look();
            }
        }
        Task::none()
    }

    /// The section: the choice, then while on, what looking for uv found, its path and uv's way in, and KentOS's own
    /// Python.
    pub(crate) fn python_section(&self) -> Element<'_, Message> {
        let p = &self.python_setup;
        let c = compat::compat();
        let mut page = Column::new().spacing(12);
        page = page.push(words::check(
            p.settings.enabled,
            "Python ve Python'lu eklentileri kullan",
            Some(msg(PythonEdit::Enable(!p.settings.enabled))),
        ));
        page = page.push(label::caption(
            "Kapalıyken Python ve uv aranmaz, hiçbir şey indirilmez; CAD, CBS ve KentOS'un kendi araçları her zaman \
             çalışır. Python isteyen eklentilerin o düğmeleri nedenini söyleyerek pasif kalır.",
        ));
        if p.settings.enabled {
            let line = match (&p.uv, p.looking) {
                (_, true) | (None, _) => words::text_line(Line::Info, "uv aranıyor…"),
                (Some(status), _) if status.ok() => words::text_line(Line::Ok, status.words()),
                (Some(UvStatus::NotFound { .. }), _) => words::text_line(
                    Line::Warn,
                    p.uv.as_ref().map(UvStatus::words).unwrap_or_default(),
                ),
                (Some(status), _) => words::text_line(Line::Error, status.words()),
            };
            page = page.push(words::summary(vec![line]));
            page = page.push(
                row![
                    kentos_ui::widget::text_field(
                        "uv'nin yolu (boş: kendiliğinden bulunur)",
                        &p.uv_typed
                    )
                    .on_input(|t| msg(PythonEdit::UvTyped(t))),
                    words::secondary("Uygula", Some(msg(PythonEdit::UvApply))),
                    words::secondary(
                        "Yeniden algıla",
                        (!p.looking).then(|| msg(PythonEdit::Detect)),
                    ),
                ]
                .spacing(8)
                .align_y(iced::Center),
            );
            if p.uv.as_ref().is_some_and(UvStatus::offers_install) {
                // uv's official ways in for this system, each with its own copy button; on Windows WinGet after the
                // PowerShell installer, for computers that refuse `-ExecutionPolicy ByPass`.
                let mut ways = Column::new().spacing(6).push(label::body(
                    "uv'yi kendiniz kurun: KentOS çalışırken bilgisayarınıza program kurmaz.",
                ));
                for (n, command) in c.install_commands().iter().enumerate() {
                    ways = ways.push(
                        row![
                            label::mono(command.clone()),
                            words::secondary("Kopyala", Some(msg(PythonEdit::CopyInstall(n)))),
                        ]
                        .spacing(10)
                        .align_y(iced::Center),
                    );
                }
                ways = ways.push(label::caption(format!(
                    "Kurduktan sonra Yeniden algıla. Ayrıntı: {}",
                    c.install.page
                )));
                page = page.push(ways);
            }
            if p.ready_uv().is_some() {
                page = page.push(self.own_python_part());
            }
            if matches!(p.python, Some(OwnPython::Ready(..)))
                && p.installing.is_none()
                && !p.removing
            {
                page = page.push(self.personal_part());
            }
            page = page.push(label::caption(format!(
                "KentOS'un Python'u {} · uv en az {}.",
                c.python.range, c.uv.min
            )));
        }
        if let Some((kind, said)) = &p.note {
            page = page.push(words::text_line(*kind, said.clone()));
        }
        page.into()
    }

    /// KentOS's own Python: what was found; when there is none, what installing it means and the button that is the
    /// user's consent; while installing, how far it is and Durdur; installed, Kaldır, which asks first.
    fn own_python_part(&self) -> Element<'_, Message> {
        let p = &self.python_setup;
        let c = compat::compat();
        let version = &c.python.install;
        let mut part = Column::new()
            .spacing(8)
            .push(label::strong("KentOS'un Python'u"));
        if p.installing.is_some() {
            let at = p.progress.unwrap_or(Progress::Starting);
            part = part
                .push(words::summary(vec![words::text_line(
                    Line::Info,
                    format!(
                        "Python {version} kuruluyor. Bağlantıya göre birkaç dakika sürebilir; KentOS'u kullanmaya \
                         devam edebilirsiniz."
                    ),
                )]))
                .push(label::caption(progress_words(at)))
                .push(progress::bar(at.share()).height(6.0))
                .push(words::secondary("Durdur", Some(msg(PythonEdit::StopInstall))));
            return part.into();
        }
        if p.removing {
            return part
                .push(words::summary(vec![words::text_line(
                    Line::Info,
                    "KentOS'un Python'u kaldırılıyor…",
                )]))
                .push(progress::bar(None).height(6.0))
                .into();
        }
        match &p.python {
            None | Some(OwnPython::Looking) => {
                part = part.push(words::summary(vec![words::text_line(
                    Line::Info,
                    "KentOS'un Python'u aranıyor…",
                )]));
            }
            Some(OwnPython::Ready(python, size)) if p.confirm_remove => {
                part = part
                    .push(words::summary(vec![words::text_line(
                        Line::Warn,
                        format!(
                            "Python {} kaldırılsın mı? {} yer açılır.{} Python isteyen eklentiler yeniden kurulana \
                             dek çalışmaz; yeniden kurmak için yeniden indirilir.",
                            python.version,
                            size_words(*size),
                            if matches!(
                                p.personal.env,
                                Some(crate::python_personal::OwnEnv::Ready(_))
                            ) {
                                " Üzerinde çalışan kişisel ortam da silinir."
                            } else {
                                ""
                            }
                        ),
                    )]))
                    .push(
                        row![
                            button(label::body("Evet, kaldır"))
                                .on_press(msg(PythonEdit::RemoveConfirmed))
                                .padding(kentos_ui::theme::metrics::action())
                                .style(style::button::danger),
                            words::secondary("Vazgeç", Some(msg(PythonEdit::RemoveCancelled))),
                        ]
                        .spacing(8),
                    );
            }
            Some(OwnPython::Ready(python, size)) => {
                part = part
                    .push(words::summary(vec![words::text_line(
                        Line::Ok,
                        format!(
                            "Python {} hazır · {} ({}).",
                            python.version,
                            size_words(*size),
                            python.path.display()
                        ),
                    )]))
                    .push(words::secondary("Kaldır", Some(msg(PythonEdit::Remove))));
            }
            Some(OwnPython::Missing) => {
                let folder = p
                    .root
                    .as_deref()
                    .map(kentos_python_env::places::interpreters)
                    .unwrap_or_default();
                part = part
                    .push(words::summary(vec![words::text_line(
                        Line::Warn,
                        "KentOS'un Python'u henüz kurulmadı.",
                    )]))
                    .push(label::body(format!(
                        "uv, Python {version}'yi Astral'ın python-build-standalone derlemelerinden (GitHub) indirir: \
                         {}. Yalnız sizin hesabınıza, şu klasöre kurulur: {}. Sistemdeki Python'a, PATH'e ve Windows \
                         kayıt defterine dokunulmaz; istediğinizde buradan kaldırabilirsiniz.",
                        c.python.size,
                        folder.display()
                    )))
                    .push(label::caption(
                        "İnternet gerekir. Vekil sunucu ve sertifikalar bilgisayarın ayarlarından alınır. Bu düğmeye \
                         basmadıkça hiçbir şey indirilmez.",
                    ))
                    .push(words::primary(
                        format!("Python {version}'yi indir ve kur"),
                        Some(msg(PythonEdit::Install)),
                    ));
            }
            Some(OwnPython::Unknown(why)) => {
                part = part.push(words::summary(vec![words::text_line(
                    Line::Error,
                    format!(
                        "KentOS'un Python'u okunamadı: {why}. Yeniden algılayın; sürerse KentOS'un Python klasörünü \
                         denetleyin."
                    ),
                )]));
            }
        }
        part.into()
    }
}
