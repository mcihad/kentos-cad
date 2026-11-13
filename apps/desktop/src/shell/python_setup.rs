//! Python ve eklentiler (plan/eklenti.md ADIM 18A; docs/adr/8057 §5): the user's choice to use Python and Python
//! extensions, off by default, and uv found for it.
//!
//! - **Off** (P01, P02): nothing looks for uv or Python and nothing is downloaded; CAD, GIS and KentOS's own tools work;
//!   an extension's Python button stays shown, not pressable, and says why.
//! - **On:** uv is looked for in the background (the window never waits for it): the chosen path; else the uv KentOS's
//!   installation carries (the owner's decision, 13 November 2026), `PATH`, the user folders a session started from
//!   the menu lacks; its version is checked against the compatibility manifest (kentos-python-env). Not found (the
//!   installation's own removed) or too old, the window says so and shows uv's official way in for the user to run
//!   themselves: the running program installs nothing on its own (PY-UV-03…06). A path can be given by hand;
//!   Yeniden algıla looks again.
//!
//! The choice is the user's own (`<veri>/kentos-cad/python/ayarlar.json`), written at once, apart from the settings'
//! draft, as Yapay zekâ's agents are.

use std::path::PathBuf;

use iced::widget::{Column, row};
use iced::{Element, Task};
use kentos_python_env::compat;
use kentos_python_env::settings::PythonSettings;
use kentos_python_env::uv::UvStatus;
use kentos_ui::label;

use crate::app::{App, Message};
use crate::exchange::words::{self, Kind as Line};
use crate::settings_view::Edit;

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
    /// The uv path typed, before Uygula.
    pub(crate) uv_typed: String,
    /// The last action's outcome: an error or not, and its words.
    pub(crate) note: Option<(bool, String)>,
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
            Some(_) => {
                "Python hazır; eklentilerin Python işlevlerini çalıştıran yürütücü KentOS'un sonraki sürümünde."
            }
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
}

fn msg(edit: PythonEdit) -> Message {
    Message::Settings(Edit::Python(edit))
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
        Task::perform(
            async move {
                let (sent, got) = iced::futures::channel::oneshot::channel();
                std::thread::spawn(move || {
                    let _ = sent.send(kentos_python_env::uv::detect(chosen.as_deref()));
                });
                got.await.unwrap_or(UvStatus::NotFound {
                    looked: Vec::new(),
                    own: false,
                })
            },
            |status| msg(PythonEdit::Detected(status)),
        )
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
            p.note = Some((true, why));
        }
    }

    pub(crate) fn python_edit(&mut self, edit: PythonEdit) -> Task<Message> {
        match edit {
            PythonEdit::Enable(on) => {
                self.python_setup.settings.enabled = on;
                self.python_setup.note = None;
                self.python_save();
                if !on {
                    self.python_setup.uv = None;
                    self.python_setup.looking = false;
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
                    false,
                    "Kurulum komutu panoya kopyalandı; bir uçbirimde kendiniz çalıştırın, sonra Yeniden algıla."
                        .to_owned(),
                ));
                return iced::clipboard::write(command);
            }
        }
        Task::none()
    }

    /// The section: the choice, then while on, what looking for uv found, its path and uv's way in.
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
            page = page.push(label::caption(format!(
                "KentOS'un Python'u {} · uv en az {}.",
                c.python.range, c.uv.min
            )));
        }
        if let Some((error, said)) = &p.note {
            page = page.push(words::text_line(
                if *error { Line::Error } else { Line::Ok },
                said.clone(),
            ));
        }
        page.into()
    }
}
