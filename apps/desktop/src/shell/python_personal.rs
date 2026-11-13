//! Kişisel ortam (plan/eklenti.md ADIM 18A, P04; docs/adr/8057 §5): the user's own Python environment, in Python ve
//! eklentiler under KentOS's Python. Made offline with KentOS's library from the installation. The Python console runs
//! in it (python/host.rs).
//!
//! Packages, as the owner asked (13 November 2026: "Paketler de kaldırılabilmeli listelenebilmeli aranabilmeli
//! listeden seçilerek kurulabilmeli … arayüz ile kolaylaştır"):
//! - **Listed:** the ones the user asked for, each with its line from the recommended list, Güncelle and Kaldır; their
//!   dependencies under them; the whole as uv draws it (Bağımlılık ağacı).
//! - **Searched:** one field filters what is installed and the recommended packages (in Turkish, `sayisal` finds
//!   `Sayısal`); a name that is not among them can still be asked for: the index is searched by uv when it is previewed.
//! - **Picked from the list:** recommended packages are ticked, any number, and previewed together.
//! - **Updated:** Güncellemeleri denetle asks uv which have newer versions; one or all are previewed and made.
//! - **Every change previewed:** what comes (asked for, or as a dependency), what goes, what changes version; nothing
//!   changes before the user agrees. Then a bar while it downloads, Durdur, and a notice for the outcome.
//! - **Removed from the same place:** a package (previewed too), the environment (asked first). uv links packages from
//!   its cache into the environment, so the cache is shown and cleaned apart: removing the environment does not free
//!   the room its packages take there.

use std::collections::{BTreeMap, BTreeSet};
use std::panic::AssertUnwindSafe;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use iced::widget::{Column, button, row};
use iced::{Element, Task};
use kentos_python_env::catalog;
use kentos_python_env::personal::{self, Change, Outdated, Personal, Preview};
use kentos_python_env::python::{InstallError, Interpreter, Progress};
use kentos_python_env::sdk;
use kentos_ui::widget::{Toast, progress};
use kentos_ui::{label, style};

use crate::app::{App, Message};
use crate::exchange::words::{self, Kind as Line};
use crate::python_setup::{OwnPython, PythonEdit, in_background, progress_words, size_words};
use crate::settings_view::Edit;

/// The program's version: the KentOS library's wheel must be of it.
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// How many recommended packages show before Tümünü göster, while nothing is searched.
const FIRST: usize = 8;

/// The personal environment, as looking for it found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OwnEnv {
    Looking,
    Missing,
    Ready(Personal),
    /// uv could not say.
    Unknown(String),
}

/// What the personal environment is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Busy {
    Creating,
    Previewing,
    Applying,
    Removing,
    Cleaning,
    /// Asking the index for newer versions.
    Checking,
}

/// The personal environment as the program holds it.
#[derive(Debug, Default)]
pub(crate) struct PersonalSetup {
    /// None while KentOS's Python is not ready.
    pub(crate) env: Option<OwnEnv>,
    /// uv's cache's bytes (KentOS's own).
    pub(crate) cache: u64,
    /// The search field: a word to filter by, or a package to ask for (`shapely`, `pandas>=2`).
    pub(crate) typed: String,
    /// Recommended packages ticked.
    pub(crate) picked: BTreeSet<String>,
    /// All the recommended packages shown, not the first few.
    pub(crate) show_all: bool,
    /// Newer versions, by name, once asked for; dropped when the environment changes.
    pub(crate) outdated: Option<BTreeMap<String, Outdated>>,
    /// uv's drawing of the dependencies, while shown.
    pub(crate) tree: Option<String>,
    pub(crate) preview: Option<Preview>,
    pub(crate) busy: Option<Busy>,
    /// How far applying a change is.
    pub(crate) progress: Option<Progress>,
    /// While applying: set, it stops.
    pub(crate) stop: Option<Arc<AtomicBool>>,
    /// Ortamı kaldır pressed: asking first.
    pub(crate) confirm_remove: bool,
    /// What went wrong last, said in the part.
    pub(crate) note: Option<String>,
}

/// What the part's controls ask.
#[derive(Debug, Clone)]
pub enum PersonalEdit {
    /// What looking found, and the cache's bytes.
    Found(Result<(Option<Personal>, u64), String>),
    Create,
    Created(Result<(), String>),
    /// The search field.
    Typed(String),
    /// What adding the typed package would do (it need not be a recommended one).
    PreviewAdd,
    /// A recommended package ticked or not.
    Pick(String, bool),
    /// What adding the ticked packages would do, together.
    PreviewPicked,
    ShowAll(bool),
    /// What removing a package the user asked for would do.
    PreviewRemove(String),
    /// What updating one package (or all, none named) would do.
    Upgrade(Option<String>),
    Previewed(Result<Preview, String>),
    /// Vazgeç on a preview.
    Dismiss,
    /// Kur / Kaldır / Güncelle on a preview: the user agrees.
    Apply,
    Progressed(Progress),
    Applied(Result<(), InstallError>),
    /// Durdur.
    Stop,
    /// Güncellemeleri denetle.
    CheckUpdates,
    Checked(Result<Vec<Outdated>, String>),
    /// Bağımlılık ağacı: shown or hidden.
    ToggleTree,
    Tree(Result<String, String>),
    /// Ortamı kaldır: asks first.
    Remove,
    RemoveConfirmed,
    RemoveCancelled,
    Removed(Result<(), String>),
    CleanCache,
    Cleaned(Result<(), String>),
}

fn msg(edit: PersonalEdit) -> Message {
    Message::Settings(Edit::Python(PythonEdit::Personal(edit)))
}

/// What a change that did not finish means, for packages; the environment went back to how it was.
pub(crate) fn apply_words(error: &InstallError) -> String {
    use kentos_python_env::python::Cause;
    let back = "ortam önceki hâline döndü";
    let (sentence, said) = match error {
        InstallError::Cancelled => return format!("Paket kurulumu durduruldu; {back}."),
        InstallError::TimedOut => {
            return format!("Paket kurulumu 30 dakikada bitmedi ve durduruldu; {back}.");
        }
        InstallError::NotRun(why) => return format!("uv çalıştırılamadı: {why}."),
        InstallError::Failed { cause, said } => (
            match cause {
                Cause::Network => format!(
                    "Paketler indirilemedi: internete ulaşılamadı. Bağlantıyı ya da vekil sunucu ayarını (HTTPS_PROXY) \
                     denetleyin; {back}."
                ),
                Cause::Certificate => format!(
                    "Paketler indirilemedi: paket dizininin sertifikası bu bilgisayarda doğrulanamadı; kurumun \
                     sertifikası yüklü olmalı. {back}."
                ),
                Cause::Disk => format!("Paketler kurulamadı: diskte yer yok; {back}."),
                Cause::Folder => {
                    format!("Paketler kurulamadı: kişisel ortamın klasörüne yazılamadı; {back}.")
                }
                Cause::Unavailable | Cause::Other => format!("Paketler kurulamadı; {back}."),
            },
            said,
        ),
    };
    if said.is_empty() {
        sentence
    } else {
        format!("{sentence} uv: “{said}”")
    }
}

/// A preview in sentences: what comes as asked, with its dependencies; what changes version; what goes.
pub(crate) fn preview_words(preview: &Preview) -> Vec<String> {
    let named = |p: &personal::Package| format!("{} {}", p.name, p.version);
    let mut lines = Vec::new();
    let asked: Vec<String> = preview
        .added
        .iter()
        .filter(|p| p.direct)
        .map(named)
        .collect();
    let brought: Vec<String> = preview
        .added
        .iter()
        .filter(|p| !p.direct)
        .map(named)
        .collect();
    if !asked.is_empty() {
        lines.push(format!("Kurulacak: {}.", asked.join(", ")));
    }
    if !brought.is_empty() {
        lines.push(format!(
            "Bağımlılık olarak gelecek ({}): {}.",
            brought.len(),
            brought.join(", ")
        ));
    }
    if !preview.changed.is_empty() {
        lines.push(format!(
            "Sürümü değişecek: {}.",
            preview
                .changed
                .iter()
                .map(|(name, from, to)| format!("{name} {from} → {to}"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if !preview.removed.is_empty() {
        lines.push(format!(
            "Kaldırılacak: {}.",
            preview
                .removed
                .iter()
                .map(named)
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if let Change::Remove(names) = &preview.change
        && preview.removed.is_empty()
    {
        lines.push(format!(
            "{} istediğiniz paketlerden çıkar; başka bir paket istediği için ortamda kalır.",
            names.join(", ")
        ));
    }
    if lines.is_empty() {
        lines.push(match preview.change {
            Change::Upgrade { .. } => {
                "Daha yeni bir sürüm kurulamıyor; ortam değişmeyecek.".to_owned()
            }
            _ => "Ortamda bir şey değişmeyecek.".to_owned(),
        });
    }
    lines
}

/// The notice for a change made.
fn applied_toast(preview: Option<&Preview>) -> Toast<Message> {
    let Some(p) = preview else {
        return Toast::success("Kişisel ortam güncellendi");
    };
    match &p.change {
        Change::Remove(names) => {
            let toast = Toast::success(format!("{} kaldırıldı", names.join(", ")));
            if p.removed.len() > 1 {
                toast.body(format!(
                    "Onunla gelen {} paket de kaldırıldı.",
                    p.removed.len() - 1
                ))
            } else {
                toast
            }
        }
        Change::Upgrade { .. } => {
            let changed: Vec<String> = p
                .changed
                .iter()
                .map(|(name, from, to)| format!("{name} {from} → {to}"))
                .collect();
            let toast = Toast::success("Paketler güncellendi");
            if changed.is_empty() {
                toast
            } else {
                toast.body(changed.join(", "))
            }
        }
        Change::Add(_) => {
            let asked: Vec<String> = p
                .added
                .iter()
                .filter(|a| a.direct)
                .map(|a| format!("{} {}", a.name, a.version))
                .collect();
            let brought = p.added.iter().filter(|a| !a.direct).count();
            let toast = Toast::success(if asked.is_empty() {
                "Paketler kuruldu".to_owned()
            } else {
                format!("{} kuruldu", asked.join(", "))
            });
            if brought > 0 {
                toast.body(format!("{brought} bağımlılığıyla birlikte."))
            } else {
                toast
            }
        }
    }
}

impl App {
    /// uv, the Python folder and KentOS's Python, while all three are ready.
    fn personal_ready(&self) -> Option<(PathBuf, PathBuf, Interpreter)> {
        let p = &self.python_setup;
        match (p.ready_uv(), p.root.clone(), &p.python) {
            (Some(uv), Some(root), Some(OwnPython::Ready(python, _))) => {
                Some((uv, root, python.clone()))
            }
            _ => None,
        }
    }

    /// Looks for the personal environment in the background, while KentOS's Python is ready.
    pub(crate) fn personal_look(&mut self) -> Task<Message> {
        let Some((uv, root, _)) = self.personal_ready() else {
            self.python_setup.personal.env = None;
            return Task::none();
        };
        self.python_setup.personal.env = Some(OwnEnv::Looking);
        let tree = self.python_setup.personal.tree.is_some();
        let looked = in_background(
            {
                let (uv, root) = (uv.clone(), root.clone());
                move || {
                    let found = personal::read(&uv, &root)?;
                    Ok((found, personal::cache_size(&root)))
                }
            },
            Err("arama yarıda kaldı".to_owned()),
            |found| msg(PersonalEdit::Found(found)),
        );
        // A tree shown follows the environment.
        if tree {
            Task::batch([looked, self.personal_tree(uv, root)])
        } else {
            looked
        }
    }

    fn personal_tree(&self, uv: PathBuf, root: PathBuf) -> Task<Message> {
        in_background(
            move || personal::tree(&uv, &root),
            Err("ağaç okunamadı".to_owned()),
            |drawn| msg(PersonalEdit::Tree(drawn)),
        )
    }

    /// Previews a change in the background.
    fn personal_preview(&mut self, change: Change) -> Task<Message> {
        let Some((uv, root, python)) = self.personal_ready() else {
            return Task::none();
        };
        let me = &mut self.python_setup.personal;
        if me.busy.is_some() {
            return Task::none();
        }
        me.busy = Some(Busy::Previewing);
        me.preview = None;
        me.note = None;
        in_background(
            move || personal::preview(&uv, &root, &python, &change),
            Err("önizleme yarıda kaldı".to_owned()),
            |seen| msg(PersonalEdit::Previewed(seen)),
        )
    }

    /// Applies the previewed change on a thread of its own, saying how far it is as it goes.
    fn personal_apply(&mut self) -> Task<Message> {
        let Some((uv, root, python)) = self.personal_ready() else {
            return Task::none();
        };
        let me = &mut self.python_setup.personal;
        if me.busy.is_some() || me.preview.is_none() {
            return Task::none();
        }
        let stop = Arc::new(AtomicBool::new(false));
        me.stop = Some(stop.clone());
        me.busy = Some(Busy::Applying);
        me.progress = Some(Progress::Starting);
        me.note = None;
        let (out, replies) = iced::futures::channel::mpsc::unbounded();
        let spawned = std::thread::Builder::new()
            .name("kentos-python-paket".into())
            .spawn(move || {
                let told = out.clone();
                let ended = std::panic::catch_unwind(AssertUnwindSafe(|| {
                    let wheel = sdk::find(&sdk::dirs(), VERSION).map_err(InstallError::NotRun)?;
                    personal::apply(&uv, &root, &python, &wheel, &stop, &mut |at| {
                        let _ = told.unbounded_send(PersonalEdit::Progressed(at));
                    })
                }))
                .unwrap_or_else(|_| {
                    Err(InstallError::NotRun(
                        "kurulum beklenmedik biçimde durdu".to_owned(),
                    ))
                });
                let _ = out.unbounded_send(PersonalEdit::Applied(ended));
            });
        match spawned {
            Ok(_) => Task::run(replies, msg),
            Err(e) => Task::done(msg(PersonalEdit::Applied(Err(InstallError::NotRun(
                e.to_string(),
            ))))),
        }
    }

    /// The change that brings `name` (or every package, none named) to its newer version.
    fn upgrade_of(&self, name: Option<&str>) -> Option<Change> {
        let me = &self.python_setup.personal;
        let outdated = me.outdated.as_ref()?;
        let Some(OwnEnv::Ready(env)) = &me.env else {
            return None;
        };
        let direct: BTreeSet<&str> = env
            .packages
            .iter()
            .filter(|p| p.direct)
            .map(|p| p.name.as_str())
            .collect();
        let mut raise = Vec::new();
        let mut others = Vec::new();
        for row in outdated.values() {
            if name.is_some_and(|n| n != row.name) {
                continue;
            }
            if direct.contains(row.name.as_str()) {
                raise.push((row.name.clone(), row.latest.clone()));
            } else {
                others.push(row.name.clone());
            }
        }
        (!raise.is_empty() || !others.is_empty()).then_some(Change::Upgrade { raise, others })
    }

    pub(crate) fn personal_edit(&mut self, edit: PersonalEdit) -> Task<Message> {
        match edit {
            PersonalEdit::Found(found) => {
                if self.personal_ready().is_some() {
                    let me = &mut self.python_setup.personal;
                    me.env = Some(match found {
                        Ok((Some(env), cache)) => {
                            me.cache = cache;
                            OwnEnv::Ready(env)
                        }
                        Ok((None, cache)) => {
                            me.cache = cache;
                            OwnEnv::Missing
                        }
                        Err(why) => OwnEnv::Unknown(why),
                    });
                }
            }
            PersonalEdit::Create => {
                let Some((uv, root, python)) = self.personal_ready() else {
                    return Task::none();
                };
                let me = &mut self.python_setup.personal;
                if me.busy.is_some() {
                    return Task::none();
                }
                me.busy = Some(Busy::Creating);
                me.note = None;
                return in_background(
                    move || {
                        let wheel = sdk::find(&sdk::dirs(), VERSION)?;
                        personal::create(&uv, &root, &python, &wheel)
                    },
                    Err("oluşturma yarıda kaldı".to_owned()),
                    |made| msg(PersonalEdit::Created(made)),
                );
            }
            PersonalEdit::Created(made) => {
                self.python_setup.personal.busy = None;
                match made {
                    Ok(()) => self.toast(Toast::success("Kişisel ortam oluşturuldu").body(
                        "Python konsolu artık bu ortamda çalışır; paketleri buradan ekleyin.",
                    )),
                    Err(why) => {
                        self.python_setup.personal.note = Some(why.clone());
                        self.toast(Toast::error("Kişisel ortam oluşturulamadı").body(why));
                    }
                }
                return self.personal_look();
            }
            PersonalEdit::Typed(text) => {
                let me = &mut self.python_setup.personal;
                me.typed = text;
                me.note = None;
            }
            PersonalEdit::PreviewAdd => {
                let typed = self.python_setup.personal.typed.trim().to_owned();
                if let Err(why) = personal::check(&typed) {
                    self.python_setup.personal.note = Some(why);
                    return Task::none();
                }
                return self.personal_preview(Change::Add(vec![typed]));
            }
            PersonalEdit::Pick(name, on) => {
                let picked = &mut self.python_setup.personal.picked;
                if on {
                    picked.insert(name);
                } else {
                    picked.remove(&name);
                }
            }
            PersonalEdit::PreviewPicked => {
                let picked: Vec<String> =
                    self.python_setup.personal.picked.iter().cloned().collect();
                if picked.is_empty() {
                    return Task::none();
                }
                return self.personal_preview(Change::Add(picked));
            }
            PersonalEdit::ShowAll(on) => self.python_setup.personal.show_all = on,
            PersonalEdit::PreviewRemove(name) => {
                return self.personal_preview(Change::Remove(vec![name]));
            }
            PersonalEdit::Upgrade(name) => {
                let Some(change) = self.upgrade_of(name.as_deref()) else {
                    return Task::none();
                };
                return self.personal_preview(change);
            }
            PersonalEdit::Previewed(seen) => {
                let me = &mut self.python_setup.personal;
                me.busy = None;
                match seen {
                    Ok(preview) => me.preview = Some(preview),
                    Err(why) => me.note = Some(why),
                }
            }
            PersonalEdit::Dismiss => self.python_setup.personal.preview = None,
            PersonalEdit::Apply => return self.personal_apply(),
            PersonalEdit::Progressed(at) => {
                let me = &mut self.python_setup.personal;
                if me.busy == Some(Busy::Applying) {
                    me.progress = Some(at);
                }
            }
            PersonalEdit::Applied(ended) => {
                let me = &mut self.python_setup.personal;
                me.busy = None;
                me.stop = None;
                me.progress = None;
                let preview = me.preview.take();
                match ended {
                    Ok(()) => {
                        me.typed.clear();
                        me.picked.clear();
                        // What was newer may be installed now: asked again when wanted.
                        me.outdated = None;
                        let toast = applied_toast(preview.as_ref());
                        self.toast(toast);
                    }
                    Err(InstallError::Cancelled) => self.toast(
                        Toast::info("Paket kurulumu durduruldu").body("Ortam önceki hâline döndü."),
                    ),
                    Err(error) => {
                        let words = apply_words(&error);
                        self.python_setup.personal.note = Some(words.clone());
                        self.toast(Toast::error("Paketler kurulamadı").body(words));
                    }
                }
                return self.personal_look();
            }
            PersonalEdit::Stop => {
                if let Some(stop) = &self.python_setup.personal.stop {
                    stop.store(true, Ordering::Relaxed);
                }
            }
            PersonalEdit::CheckUpdates => {
                let Some((uv, root, _)) = self.personal_ready() else {
                    return Task::none();
                };
                let me = &mut self.python_setup.personal;
                if me.busy.is_some() {
                    return Task::none();
                }
                me.busy = Some(Busy::Checking);
                me.note = None;
                return in_background(
                    move || personal::outdated(&uv, &root),
                    Err("denetim yarıda kaldı".to_owned()),
                    |rows| msg(PersonalEdit::Checked(rows)),
                );
            }
            PersonalEdit::Checked(rows) => {
                let me = &mut self.python_setup.personal;
                me.busy = None;
                match rows {
                    Ok(rows) => {
                        let count = rows.len();
                        me.outdated = Some(rows.into_iter().map(|r| (r.name.clone(), r)).collect());
                        self.toast(if count == 0 {
                            Toast::success("Paketler güncel")
                        } else {
                            Toast::info(format!("{count} paketin yeni sürümü var")).body(
                                "Güncelle ya da Tümünü güncelle önce ne değişeceğini gösterir.",
                            )
                        });
                    }
                    Err(why) => me.note = Some(why),
                }
            }
            PersonalEdit::ToggleTree => {
                if self.python_setup.personal.tree.take().is_some() {
                    return Task::none();
                }
                let Some((uv, root, _)) = self.personal_ready() else {
                    return Task::none();
                };
                self.python_setup.personal.tree = Some(String::new());
                return self.personal_tree(uv, root);
            }
            PersonalEdit::Tree(drawn) => {
                let me = &mut self.python_setup.personal;
                // Hidden meanwhile: not shown again.
                if me.tree.is_some() {
                    me.tree = Some(drawn.unwrap_or_else(|why| format!("Ağaç okunamadı: {why}")));
                }
            }
            PersonalEdit::Remove => {
                let me = &mut self.python_setup.personal;
                if me.busy.is_none() {
                    me.confirm_remove = true;
                }
            }
            PersonalEdit::RemoveCancelled => self.python_setup.personal.confirm_remove = false,
            PersonalEdit::RemoveConfirmed => {
                self.python_setup.personal.confirm_remove = false;
                let Some(root) = self.python_setup.root.clone() else {
                    return Task::none();
                };
                let me = &mut self.python_setup.personal;
                if me.busy.is_some() {
                    return Task::none();
                }
                me.busy = Some(Busy::Removing);
                me.preview = None;
                me.note = None;
                me.tree = None;
                me.outdated = None;
                return in_background(
                    move || personal::remove(&root),
                    Err("kaldırma yarıda kaldı".to_owned()),
                    |ended| msg(PersonalEdit::Removed(ended)),
                );
            }
            PersonalEdit::Removed(ended) => {
                self.python_setup.personal.busy = None;
                match ended {
                    Ok(()) => {
                        let cache = size_words(self.python_setup.personal.cache);
                        self.toast(Toast::success("Kişisel ortam kaldırıldı").body(format!(
                            "Paket önbelleği ({cache}) duruyor; aynı yerden temizleyebilirsiniz."
                        )));
                    }
                    Err(why) => {
                        self.python_setup.personal.note = Some(why.clone());
                        self.toast(Toast::error("Kişisel ortam kaldırılamadı").body(why));
                    }
                }
                return self.personal_look();
            }
            PersonalEdit::CleanCache => {
                let (Some(uv), Some(root)) =
                    (self.python_setup.ready_uv(), self.python_setup.root.clone())
                else {
                    return Task::none();
                };
                let me = &mut self.python_setup.personal;
                if me.busy.is_some() {
                    return Task::none();
                }
                me.busy = Some(Busy::Cleaning);
                return in_background(
                    move || personal::clean_cache(&uv, &root),
                    Err("temizleme yarıda kaldı".to_owned()),
                    |ended| msg(PersonalEdit::Cleaned(ended)),
                );
            }
            PersonalEdit::Cleaned(ended) => {
                self.python_setup.personal.busy = None;
                match ended {
                    Ok(()) => self.toast(Toast::success("Paket önbelleği temizlendi").body(
                        "Ortamdaki paketler çalışmaya devam eder; gerektiğinde yeniden indirilir.",
                    )),
                    Err(why) => self.toast(Toast::error("Önbellek temizlenemedi").body(why)),
                }
                return self.personal_look();
            }
        }
        Task::none()
    }

    /// The personal environment's part: making it; once made, its packages (listed, searched, picked from the list,
    /// updated, removed), each change previewed and then made with how far it is; the cache apart; removing it, asked
    /// first.
    pub(crate) fn personal_part(&self) -> Element<'_, Message> {
        let me = &self.python_setup.personal;
        let mut part = Column::new()
            .spacing(8)
            .push(label::strong("Kişisel ortam"));
        let busy = me.busy;
        match (&me.env, busy) {
            (_, Some(Busy::Creating)) => {
                part = part
                    .push(words::summary(vec![words::text_line(
                        Line::Info,
                        "Kişisel ortam oluşturuluyor…",
                    )]))
                    .push(progress::bar(None).height(6.0));
            }
            (_, Some(Busy::Removing)) => {
                part = part
                    .push(words::summary(vec![words::text_line(
                        Line::Info,
                        "Kişisel ortam kaldırılıyor…",
                    )]))
                    .push(progress::bar(None).height(6.0));
            }
            (None | Some(OwnEnv::Looking), _) => {
                part = part.push(words::summary(vec![words::text_line(
                    Line::Info,
                    "Kişisel ortam aranıyor…",
                )]));
            }
            (Some(OwnEnv::Unknown(why)), _) => {
                part = part.push(words::summary(vec![words::text_line(
                    Line::Error,
                    format!("Kişisel ortam okunamadı: {why}."),
                )]));
            }
            (Some(OwnEnv::Missing), _) => {
                part = part
                    .push(words::summary(vec![words::text_line(
                        Line::Warn,
                        "Kişisel ortam henüz yok.",
                    )]))
                    .push(label::body(
                        "Python konsolu ve kendi betikleriniz bu ortamda çalışır; NumPy gibi paketleri buraya \
                         kurarsınız. Eklentilerin ortamları ayrıdır: buradaki paketler onları etkilemez. Oluşturmak \
                         internet istemez; paket eklerken her şey önce önizlenir.",
                    ))
                    .push(words::primary(
                        "Kişisel ortamı oluştur",
                        busy.is_none().then(|| msg(PersonalEdit::Create)),
                    ));
            }
            (Some(OwnEnv::Ready(env)), _) => {
                part = part.push(self.personal_ready_part(env));
            }
        }
        if let Some(note) = &me.note {
            part = part.push(words::text_line(Line::Error, note.clone()));
        }
        part.into()
    }

    fn personal_ready_part<'a>(&'a self, env: &'a Personal) -> Element<'a, Message> {
        let me = &self.python_setup.personal;
        let busy = me.busy;
        let idle = busy.is_none();
        let library = env
            .sdk
            .as_deref()
            .map(|v| format!("KentOS kitaplığı {v}"))
            .unwrap_or_else(|| "KentOS kitaplığı eksik".to_owned());
        let mut part = Column::new()
            .spacing(10)
            .push(words::summary(vec![words::text_line(
                if env.sdk.is_some() {
                    Line::Ok
                } else {
                    Line::Warn
                },
                format!(
                    "Kişisel ortam hazır · {} · {library}. Python konsolu bu ortamda çalışır.",
                    size_words(env.size)
                ),
            )]));
        // A change under way, or one to agree to, takes the place of the lists.
        if busy == Some(Busy::Applying) {
            let at = me.progress.unwrap_or(Progress::Starting);
            let what = me
                .preview
                .as_ref()
                .map(|p| preview_words(p).join(" "))
                .unwrap_or_default();
            return part
                .push(label::caption(what))
                .push(label::caption(progress_words(at)))
                .push(progress::bar(at.share()).height(6.0))
                .push(words::secondary("Durdur", Some(msg(PersonalEdit::Stop))))
                .into();
        }
        if let Some(preview) = &me.preview {
            let lines = preview_words(preview)
                .into_iter()
                .map(|line| words::text_line(Line::Info, line))
                .collect();
            let agree = match preview.change {
                Change::Remove(_) => "Kaldır",
                Change::Upgrade { .. } => "Güncelle",
                Change::Add(_) => "Kur",
            };
            return part
                .push(label::strong("Önizleme: onaylamadan ortam değişmez"))
                .push(words::summary(lines))
                .push(
                    row![
                        words::primary(
                            agree,
                            (!preview.is_empty()).then(|| msg(PersonalEdit::Apply))
                        ),
                        words::secondary("Vazgeç", Some(msg(PersonalEdit::Dismiss))),
                    ]
                    .spacing(8),
                )
                .into();
        }
        let list = catalog::builtin();
        let query = me.typed.trim();
        let folded = catalog::fold(query);
        let matches = |name: &str, line: &str| {
            folded.is_empty() || catalog::fold(&format!("{name} {line}")).contains(&folded)
        };
        // The search field, and what to do with the whole.
        part = part.push(
            row![
                kentos_ui::widget::text_field(
                    "Paket ara ya da adını yazın (ör. shapely, pandas>=2)",
                    &me.typed
                )
                .on_input(|t| msg(PersonalEdit::Typed(t)))
                .on_submit(msg(if me.picked.is_empty() {
                    PersonalEdit::PreviewAdd
                } else {
                    PersonalEdit::PreviewPicked
                })),
                words::secondary(
                    if busy == Some(Busy::Checking) {
                        "Denetleniyor…"
                    } else {
                        "Güncellemeleri denetle"
                    },
                    idle.then(|| msg(PersonalEdit::CheckUpdates)),
                ),
                words::secondary(
                    if me.tree.is_some() {
                        "Ağacı gizle"
                    } else {
                        "Bağımlılık ağacı"
                    },
                    Some(msg(PersonalEdit::ToggleTree)),
                ),
            ]
            .spacing(8)
            .align_y(iced::Center),
        );
        // Installed: asked for, each with Güncelle and Kaldır; dependencies under them.
        let asked: Vec<&personal::Package> = env.packages.iter().filter(|p| p.direct).collect();
        let newer = |name: &str| me.outdated.as_ref().and_then(|o| o.get(name));
        part = part.push(label::strong(format!("Kurulu paketler ({})", asked.len())));
        if asked.is_empty() {
            part = part.push(label::caption(
                "Henüz paket eklenmedi: aşağıdaki listeden seçin ya da adını yazın.",
            ));
        }
        let mut shown_installed = 0;
        for package in &asked {
            let line = list
                .find(&package.name)
                .map(|e| e.title.as_str())
                .unwrap_or("");
            if !matches(&package.name, line) {
                continue;
            }
            shown_installed += 1;
            let mut entry = row![
                label::body(format!("{} {}", package.name, package.version)),
                label::caption(line.to_owned()),
                iced::widget::space::horizontal().width(iced::Fill),
            ]
            .spacing(10)
            .align_y(iced::Center);
            if let Some(row) = newer(&package.name) {
                entry = entry.push(words::secondary(
                    format!("Güncelle → {}", row.latest),
                    idle.then(|| msg(PersonalEdit::Upgrade(Some(package.name.clone())))),
                ));
            }
            entry = entry.push(words::ghost(
                "Kaldır",
                idle.then(|| msg(PersonalEdit::PreviewRemove(package.name.clone()))),
            ));
            part = part.push(entry);
        }
        if !asked.is_empty() && shown_installed == 0 {
            part = part.push(label::caption("Aramayla eşleşen kurulu paket yok."));
        }
        let brought: Vec<String> = env
            .packages
            .iter()
            .filter(|p| !p.direct)
            .map(|p| match newer(&p.name) {
                Some(row) => format!("{} {} ({} var)", p.name, p.version, row.latest),
                None => format!("{} {}", p.name, p.version),
            })
            .collect();
        if !brought.is_empty() {
            part = part.push(label::caption(format!(
                "Bağımlılık olarak: {}.",
                brought.join(", ")
            )));
        }
        if let Some(outdated) = &me.outdated {
            part = part.push(if outdated.is_empty() {
                Element::from(label::caption("Hepsi güncel."))
            } else {
                row![
                    label::caption(format!("{} paketin yeni sürümü var.", outdated.len())),
                    words::secondary(
                        "Tümünü güncelle",
                        idle.then(|| msg(PersonalEdit::Upgrade(None))),
                    ),
                ]
                .spacing(10)
                .align_y(iced::Center)
                .into()
            });
        }
        if let Some(tree) = &me.tree {
            part = part.push(
                iced::widget::container(label::mono(if tree.is_empty() {
                    "Ağaç okunuyor…".to_owned()
                } else {
                    tree.trim_end().to_owned()
                }))
                .padding(10)
                .width(iced::Fill)
                .style(style::container::bordered),
            );
        }
        // Recommended: ticked and previewed together; a name not among them, asked for as it is.
        let installed: BTreeSet<&str> = env.packages.iter().map(|p| p.name.as_str()).collect();
        let found = list.search(query);
        let offered: Vec<&catalog::Entry> = found
            .into_iter()
            .filter(|e| !asked.iter().any(|p| p.name == e.name))
            .collect();
        part = part.push(label::strong(format!(
            "Önerilen paketler ({})",
            offered.len()
        )));
        let shown = if query.is_empty() && !me.show_all {
            FIRST
        } else {
            usize::MAX
        };
        for entry in offered.iter().take(shown) {
            let ticked = me.picked.contains(&entry.name);
            let mut text = format!("{} · {}", entry.name, entry.title);
            if installed.contains(entry.name.as_str()) {
                text.push_str(" (bağımlılık olarak kurulu)");
            }
            part = part.push(
                row![
                    words::check(
                        ticked,
                        text,
                        idle.then(|| msg(PersonalEdit::Pick(entry.name.clone(), !ticked))),
                    ),
                    iced::widget::space::horizontal().width(iced::Fill),
                    label::caption(entry.group.clone()),
                ]
                .spacing(10)
                .align_y(iced::Center),
            );
        }
        if offered.len() > shown {
            part = part.push(words::ghost(
                format!("Tüm önerilenleri göster ({})", offered.len()),
                Some(msg(PersonalEdit::ShowAll(true))),
            ));
        } else if me.show_all && query.is_empty() {
            part = part.push(words::ghost(
                "Daha az göster",
                Some(msg(PersonalEdit::ShowAll(false))),
            ));
        }
        // Offered as a package of the index when it reads as one: a version written, or nothing here found by it (a word
        // that finds recommended packages is a search, not a name).
        let typed_name = personal::check(query).ok();
        let asks = query.contains(['<', '>', '=', '!', '~', '[']);
        if let Some(name) = typed_name.as_ref().filter(|name| {
            list.find(name).is_none()
                && !asked.iter().any(|p| &p.name == *name)
                && (asks || (offered.is_empty() && shown_installed == 0))
        }) {
            part = part.push(
                row![
                    label::caption(format!(
                        "“{name}” önerilenlerde yok; paket dizininde aranır."
                    )),
                    words::secondary(
                        format!("“{query}” paketini önizle"),
                        idle.then(|| msg(PersonalEdit::PreviewAdd)),
                    ),
                ]
                .spacing(10)
                .align_y(iced::Center),
            );
        } else if offered.is_empty() && !query.is_empty() {
            part = part.push(label::caption("Aramayla eşleşen paket yok."));
        }
        part = part.push(
            row![
                words::primary(
                    if busy == Some(Busy::Previewing) {
                        "Önizleniyor…".to_owned()
                    } else {
                        format!("Seçilenleri önizle ({})", me.picked.len())
                    },
                    (idle && !me.picked.is_empty()).then(|| msg(PersonalEdit::PreviewPicked)),
                ),
                label::caption("Önizleme kurulacakları ve bağımlılıklarını gösterir; onaylamadan ortam değişmez."),
            ]
            .spacing(10)
            .align_y(iced::Center),
        );
        part = part.push(
            row![
                label::caption(format!("Paket önbelleği: {}", size_words(me.cache))),
                words::ghost(
                    if busy == Some(Busy::Cleaning) {
                        "Temizleniyor…"
                    } else {
                        "Önbelleği temizle"
                    },
                    idle.then(|| msg(PersonalEdit::CleanCache)),
                ),
            ]
            .spacing(10)
            .align_y(iced::Center),
        );
        if me.confirm_remove {
            part = part
                .push(words::summary(vec![words::text_line(
                    Line::Warn,
                    format!(
                        "Kişisel ortam kaldırılsın mı? Python konsolu çalışmaz, eklediğiniz {} paket silinir. Paket \
                         önbelleği durur.",
                        asked.len()
                    ),
                )]))
                .push(
                    row![
                        button(label::body("Evet, kaldır"))
                            .on_press(msg(PersonalEdit::RemoveConfirmed))
                            .padding(kentos_ui::theme::metrics::action())
                            .style(style::button::danger),
                        words::secondary("Vazgeç", Some(msg(PersonalEdit::RemoveCancelled))),
                    ]
                    .spacing(8),
                );
        } else {
            part = part.push(words::secondary(
                "Kişisel ortamı kaldır",
                idle.then(|| msg(PersonalEdit::Remove)),
            ));
        }
        part.into()
    }
}
