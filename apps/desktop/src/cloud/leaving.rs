//! Leaving the drawing on screen (docs/adr/0041): before the window closes,
//! another drawing or project takes its place, the account signs out or the
//! drawing goes up as a new project. What would be lost is kept first when
//! it can be: a database project's unsent work goes to its device draft
//! (docs/adr/0040) and a notice says so. Otherwise the question says what
//! would be lost, with the count: a local drawing's or a file project's
//! unsaved changes, a viewer's edits (never kept), or unsent work whose
//! draft could not be written.

use iced::{Task, window};
use kentos_cloud::ApiFailure;

use crate::app::{App, Dialog, Message, Then};
use crate::cloud::Event;

/// Where the app goes once the drawing is left.
pub type Leave = Then;

/// What the question says, and its confirm button.
pub struct Question {
    pub title: &'static str,
    pub message: String,
    pub detail: String,
    pub confirm: &'static str,
}

impl Then {
    /// Whether the drawing on screen goes (another one takes its place, or the window closes).
    fn replaces(self) -> bool {
        matches!(
            self,
            Then::Open | Then::Close(_) | Then::OpenCloud { .. } | Then::Reopen | Then::NewProject
        )
    }
}

impl App {
    /// Leaves for `then`: a database project's unsent work to its draft first,
    /// else the question when something would be lost, else at once.
    pub(crate) fn leave(&mut self, then: Then) -> Task<Message> {
        if self.cloud.leaving.is_some() {
            return Task::none();
        }
        self.cloud.leave_failure = None;
        // The signed-in account, or the one kept for work without a connection.
        let user = self.cloud_identity().map(|(_, user)| user);
        let Some(doc) = &self.document else {
            return self.proceed(then);
        };
        let session = doc.session;
        if let Some(live) = self.cloud.live.as_mut().filter(|l| l.session == session)
            && !live.sync.all_sent()
        {
            let unsent = live.sync.pending().max(1);
            if let (Some(store), Some(user), Some(key)) = (&self.cloud.drafts, user, &live.key)
                && let Some(draft) = live.sync.draft(&doc.model, &user)
            {
                self.cloud.leaving = Some(then);
                return Task::perform(store.save_later(key.clone(), draft), move |result| {
                    crate::cloud::msg(Event::Left {
                        leave: then,
                        result: result.map(|()| unsent),
                    })
                });
            }
            // A viewer's edits are never kept; without a draft folder nothing is.
            self.dialog = Some(Dialog::Unsaved(then));
            return Task::none();
        }
        if then.replaces() && doc.dirty() && !doc.is_database() {
            self.dialog = Some(Dialog::Unsaved(then));
            return Task::none();
        }
        self.proceed(then)
    }

    /// The draft of a leaving was written, or could not be.
    pub(crate) fn left(&mut self, then: Then, result: Result<usize, ApiFailure>) -> Task<Message> {
        if self.cloud.leaving.take() != Some(then) {
            return Task::none();
        }
        match result {
            Ok(unsent) => {
                self.output(format!(
                    "Gönderilmemiş {unsent} değişiklik bu cihazda saklandı; proje yeniden açılınca gönderilecek."
                ));
                self.proceed(then)
            }
            Err(failure) => {
                self.warn(failure.message.clone());
                self.cloud.leave_failure = Some(failure.message.clone());
                self.dialog = Some(Dialog::Unsaved(then));
                Task::none()
            }
        }
    }

    /// Goes on to `then`, the drawing left.
    pub(crate) fn proceed(&mut self, then: Then) -> Task<Message> {
        match then {
            Then::Open => self.open(),
            Then::Close(window) => {
                // The project's copy is written whole before the program ends.
                self.close_cloud_project();
                self.recovery.finish();
                window::close(window)
            }
            Then::SignOut => self.sign_out(),
            Then::OpenCloud { tenant, project } => self.start_cloud_open(tenant, project),
            Then::Upload => {
                self.open_upload();
                Task::none()
            }
            Then::Reopen => self.reopen(),
            Then::NewProject => self.new_project_ready(),
        }
    }

    /// Vazgeç on the question: the window it came from comes back.
    pub(crate) fn unsaved_declined(&mut self, then: Then) {
        self.cloud.leave_failure = None;
        match then {
            Then::OpenCloud { .. } if self.cloud.catalog.is_some() => {
                self.dialog = Some(Dialog::Catalog);
            }
            Then::Reopen if self.cloud.file_conflict.is_some() => {
                self.dialog = Some(Dialog::FileConflict);
            }
            Then::NewProject => self.new_project_declined(),
            _ => {}
        }
    }

    /// What the question asks before `then` (the drawing on screen decides).
    pub(crate) fn unsaved_question(&self, then: Then) -> Question {
        let name = self.document.as_ref().map_or("", |d| d.name()).to_owned();
        let action = match then {
            Then::Close(_) => "çıkarsanız",
            Then::SignOut => "oturumu kapatırsanız",
            Then::Upload => "yüklerseniz bu projeye",
            Then::NewProject => "yeni proje açarsanız",
            _ => "başka bir çizim açarsanız",
        };
        let unsent = self
            .cloud
            .live
            .as_ref()
            .filter(|l| {
                self.document
                    .as_ref()
                    .is_some_and(|d| d.session == l.session)
            })
            .filter(|l| !l.sync.all_sent())
            .map(|l| l.sync.pending().max(1));
        if let Some(n) = unsent {
            let detail = match &self.cloud.leave_failure {
                Some(why) => format!(
                    "Bu cihaza da yedeklenemediler ({why}). {} kaybolurlar. Vazgeçip bağlantıyı bekleyin ya da Farklı kaydet ile yerel bir dosyaya kaydedin.",
                    capital(action)
                ),
                None if self.cloud.drafts.is_none() => format!(
                    "Bu bilgisayarda taslak klasörü yok; {action} kaybolurlar. Vazgeçip gönderilmelerini bekleyin ya da Farklı kaydet ile yerel bir dosyaya kaydedin."
                ),
                None => format!(
                    "Bu projede yalnız görüntüleme yetkiniz var: değişiklikleriniz buluta gönderilmez, bu cihazda da saklanmaz; {action} kaybolurlar. Saklamak için Farklı kaydet ile yerel bir dosyaya kaydedin."
                ),
            };
            return Question {
                title: "Gönderilmemiş değişiklikler var",
                message: format!("“{name}” projesinde buluta gönderilmemiş {n} değişiklik var."),
                detail,
                confirm: match then {
                    Then::Close(_) => "Göndermeden çık",
                    Then::SignOut => "Göndermeden oturumu kapat",
                    Then::Upload => "Göndermeden yükle",
                    Then::Reopen => "Göndermeden yeniden aç",
                    Then::Open | Then::OpenCloud { .. } => "Göndermeden aç",
                    Then::NewProject => "Göndermeden yeni proje aç",
                },
            };
        }
        let cloud = self.document.as_ref().and_then(|d| d.cloud_source());
        let (message, detail) = match (cloud, then) {
            (Some(_), Then::Reopen) => (
                format!("“{name}” projesinde kaydedilmemiş değişiklikleriniz var."),
                "Sunucudaki son revizyon açılınca bu değişiklikler bırakılır. Saklamak için Vazgeç'e basıp Ayrı proje olarak kaydet'i seçin.".to_owned(),
            ),
            (Some(_), _) => (
                format!("“{name}” bulut projesinde kaydedilmemiş değişiklikler var."),
                "Kaydet (Ctrl+S) onları projenin yeni revizyonu olarak kaydeder; önce kaydetmek için Vazgeç'e basın.".to_owned(),
            ),
            (None, _) => (
                format!("“{name}” çiziminde kaydedilmemiş değişiklikler var."),
                "Önce kaydetmek için Vazgeç'e basıp Ctrl+S kullanın.".to_owned(),
            ),
        };
        Question {
            title: "Kaydedilmemiş değişiklikler var",
            message,
            detail,
            confirm: match then {
                Then::Close(_) => "Kaydetmeden çık",
                Then::Reopen => "Değişiklikleri bırak ve aç",
                Then::NewProject => "Kaydetmeden yeni proje aç",
                _ => "Kaydetmeden aç",
            },
        }
    }
}

/// The first letter in upper case (Turkish: i → İ).
fn capital(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some('i') => format!("İ{}", chars.as_str()),
        Some(c) => format!("{}{}", c.to_uppercase(), chars.as_str()),
        None => String::new(),
    }
}
